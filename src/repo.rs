//! Data access for the `data` store.
//!
//! Everything lives in one store service, namespaced by key prefix:
//!
//! | Key                    | Value                                             |
//! |------------------------|---------------------------------------------------|
//! | `p/<rev-ts>/<id>`      | palette JSON; `rev-ts` = `u64::MAX - created_ms` as 16 hex digits, so a prefix scan lists newest first |
//! | `pi/<id>`              | the palette's `p/...` key (lookup and delete)     |
//! | `lk/<id>/<voter>`      | a like (one per visitor per palette); value = the round it was cast in |
//! | `ik/<digest>`          | `<id>\|<created_ms>\|<round>` for a submission's idempotency key (24 h TTL) |
//! | `sv/<round>/<voter>`   | a unique visitor in a scoreboard round            |
//! | `sb/<round>/<voter>`   | a design vote in a scoreboard round               |
//! | `c/theme`, `c/site`    | the design and the variant settings (JSON)        |
//! | `c/admin`              | hash of the admin key                             |
//! | `c/epoch`              | current scoreboard round                          |
//! | `c/seeded`             | set once the seed library is written              |
//!
//! Every visitor action is ONE atomic create-if-absent write of its marker,
//! and every count shown (likes, votes, visitors, additions, library size)
//! is derived by counting records rather than kept in a separate counter.
//! So a retried request, a crash between two steps, or two workers racing
//! can never count an action twice or lose one that was acknowledged.
//!
//! Because a Ferrite fork copies the whole store, a fork inherits all of it:
//! palettes, likes, the design, the admin key hash and the scoreboard.

use std::collections::HashMap;
use std::io;

use ferrite_app::client::StoreClusterClient;
use ferrite_app::serde::de::DeserializeOwned;
use ferrite_app::serde::Serialize;
use ferrite_app::serde_json;

use crate::model::{self, Palette, Site};
use crate::theme::{self, Theme};

/// Rows per scan call (the platform's maximum page).
const SCAN_PAGE: u32 = 1024;
/// Most palettes a page lists (the cap plus headroom for racing submissions).
pub const MAX_LISTED: usize = model::MAX_PALETTES as usize + 50;

const KEY_THEME: &str = "c/theme";
const KEY_SITE: &str = "c/site";
const KEY_ADMIN: &str = "c/admin";
const KEY_EPOCH: &str = "c/epoch";
const KEY_SEEDED: &str = "c/seeded";

/// The scoreboard for the current round, plus what identifies the variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(crate = "ferrite_app::serde")]
pub struct Stats {
    pub label: String,
    pub design: String,
    pub round: i64,
    pub visitors: i64,
    pub votes: i64,
    pub likes: i64,
    pub adds: i64,
    pub palettes: i64,
}

/// Like counts derived from the like markers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LikeTally {
    /// All-time likes per palette id.
    pub per_palette: HashMap<String, i64>,
    /// Likes cast in the current round, per palette id.
    pub this_round: HashMap<String, i64>,
}

impl LikeTally {
    /// Likes in the current round on palettes that still exist.
    pub fn round_total_for(&self, palettes: &[Palette]) -> i64 {
        palettes
            .iter()
            .map(|p| self.this_round.get(&p.id).copied().unwrap_or(0))
            .sum()
    }
}

/// Everything a page render needs, read in one pass.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub theme: Theme,
    pub site: Site,
    pub palettes: Vec<Palette>,
    /// All-time likes per palette id.
    pub likes: HashMap<String, i64>,
    pub stats: Stats,
    pub admin_set: bool,
}

/// The identity a submission's idempotency key is bound to. Every attempt of
/// the same submission writes the palette under these values, so they all
/// land on the same storage keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submission {
    pub id: String,
    pub created_ms: u64,
    pub round: i64,
}

impl Submission {
    fn encode(&self) -> String {
        format!("{}|{}|{}", self.id, self.created_ms, self.round)
    }

    fn decode(bytes: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(bytes).ok()?;
        let mut parts = text.split('|');
        let id = parts.next()?.to_string();
        let created_ms = parts.next()?.parse().ok()?;
        let round = parts.next().unwrap_or("0").parse().ok()?;
        (model::valid_id(&id) && parts.next().is_none()).then_some(Submission {
            id,
            created_ms,
            round,
        })
    }
}

/// Read a number stored as decimal text (an 8-byte little-endian encoding is
/// accepted as a fallback). Absent or unreadable values read as 0.
pub fn decode_counter(bytes: &[u8]) -> i64 {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|s| s.trim().parse::<i64>().ok())
        .or_else(|| <[u8; 8]>::try_from(bytes).ok().map(i64::from_le_bytes))
        .unwrap_or(0)
}

fn palette_key(p: &Palette) -> String {
    format!("p/{:016x}/{}", u64::MAX - p.created_ms, p.id)
}
fn index_key(id: &str) -> String {
    format!("pi/{id}")
}
fn like_marker(id: &str, voter: &str) -> String {
    format!("lk/{id}/{voter}")
}
fn like_prefix(id: &str) -> String {
    format!("lk/{id}/")
}
fn vote_marker(round: i64, voter: &str) -> String {
    format!("sb/{round}/{voter}")
}
fn vote_prefix(round: i64) -> String {
    format!("sb/{round}/")
}
fn visit_marker(round: i64, voter: &str) -> String {
    format!("sv/{round}/{voter}")
}
fn visit_prefix(round: i64) -> String {
    format!("sv/{round}/")
}
fn submission_key(digest: &str) -> String {
    format!("ik/{digest}")
}

/// The palette id inside a like-marker key `lk/<id>/<voter>`.
fn like_key_palette(key: &[u8]) -> Option<&str> {
    let rest = std::str::from_utf8(key.strip_prefix(b"lk/")?).ok()?;
    rest.split_once('/').map(|(id, _)| id)
}

/// Typed access to the `data` store.
pub struct Repo {
    store: StoreClusterClient,
}

impl Repo {
    pub fn new(store: StoreClusterClient) -> Self {
        Self { store }
    }

    async fn get_json<T: DeserializeOwned>(&mut self, key: &str) -> io::Result<Option<T>> {
        match self.store.get(key.as_bytes()).await? {
            Some(bytes) => Ok(serde_json::from_slice(&bytes).ok()),
            None => Ok(None),
        }
    }

    async fn put_json<T: Serialize>(&mut self, key: &str, value: &T) -> io::Result<()> {
        let bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
        self.store.put(key.as_bytes(), &bytes).await?;
        Ok(())
    }

    /// Call `visit` for every row under `prefix`, a page at a time.
    async fn scan_each(
        &mut self,
        prefix: &[u8],
        mut visit: impl FnMut(&[u8], &[u8]) + Send,
    ) -> io::Result<()> {
        let mut after: Option<Vec<u8>> = None;
        loop {
            let (rows, more) = self
                .store
                .scan_prefix(prefix, after.as_deref(), SCAN_PAGE)
                .await?;
            for (key, value, _) in &rows {
                visit(key, value);
            }
            match rows.last() {
                Some((key, _, _)) if more => after = Some(key.clone()),
                _ => return Ok(()),
            }
        }
    }

    async fn count_prefix(&mut self, prefix: &str) -> io::Result<i64> {
        let mut n = 0i64;
        self.scan_each(prefix.as_bytes(), |_, _| n += 1).await?;
        Ok(n)
    }

    /// Create `key` if absent. True only for the call that created it.
    async fn create_marker(&mut self, key: &str, value: &[u8]) -> io::Result<bool> {
        Ok(self
            .store
            .put_if_version(key.as_bytes(), value, None)
            .await?
            .is_ok())
    }

    /// Startup work: adopt the admin secret (if configured) and seed the
    /// library on a fresh store. Both steps are idempotent.
    pub async fn bootstrap(&mut self, admin_secret: Option<&str>) -> io::Result<()> {
        if let Some(key) = admin_secret
            .map(str::trim)
            .filter(|k| k.chars().count() >= model::MIN_ADMIN_KEY_CHARS)
        {
            self.store
                .put(KEY_ADMIN.as_bytes(), model::admin_key_hash(key).as_bytes())
                .await?;
        }
        self.seed_if_needed().await?;
        Ok(())
    }

    /// Write the seed library unless it was written before. Seeds use fixed
    /// ids and timestamps, so an interrupted seeding is safely redone.
    pub async fn seed_if_needed(&mut self) -> io::Result<bool> {
        if self.store.get(KEY_SEEDED.as_bytes()).await?.is_some() {
            return Ok(false);
        }
        for p in &model::seed_palettes() {
            self.insert_palette(p).await?;
        }
        self.store.put(KEY_SEEDED.as_bytes(), b"1").await?;
        Ok(true)
    }

    pub async fn theme(&mut self) -> io::Result<Theme> {
        Ok(self
            .get_json(KEY_THEME)
            .await?
            .unwrap_or_else(theme::default_theme))
    }

    pub async fn set_theme(&mut self, t: &Theme) -> io::Result<()> {
        self.put_json(KEY_THEME, t).await
    }

    pub async fn site(&mut self) -> io::Result<Site> {
        Ok(self.get_json(KEY_SITE).await?.unwrap_or_default())
    }

    pub async fn set_site(&mut self, s: &Site) -> io::Result<()> {
        self.put_json(KEY_SITE, s).await
    }

    /// The current scoreboard round (0 until the first reset).
    pub async fn round(&mut self) -> io::Result<i64> {
        Ok(self
            .store
            .get(KEY_EPOCH.as_bytes())
            .await?
            .map(|b| decode_counter(&b))
            .unwrap_or(0))
    }

    /// Start a new scoreboard round; earlier markers stay stored but unused.
    /// A single atomic increment, which the client retries safely.
    pub async fn next_round(&mut self) -> io::Result<i64> {
        Ok(self.store.add(KEY_EPOCH.as_bytes(), 1).await?.0)
    }

    /// Number of palettes in the library.
    pub async fn palette_count(&mut self) -> io::Result<i64> {
        self.count_prefix("p/").await
    }

    /// Palettes, newest first, at most `max`.
    pub async fn list_palettes(&mut self, max: usize) -> io::Result<Vec<Palette>> {
        let mut out = Vec::new();
        let mut after: Option<Vec<u8>> = None;
        loop {
            let (rows, more) = self
                .store
                .scan_prefix(b"p/", after.as_deref(), SCAN_PAGE)
                .await?;
            for (_, value, _) in &rows {
                if let Ok(p) = serde_json::from_slice::<Palette>(value) {
                    out.push(p);
                    if out.len() >= max {
                        return Ok(out);
                    }
                }
            }
            match rows.last() {
                Some((key, _, _)) if more => after = Some(key.clone()),
                _ => return Ok(out),
            }
        }
    }

    /// Count the like markers, all-time and for `round`.
    pub async fn like_tally(&mut self, round: i64) -> io::Result<LikeTally> {
        let mut tally = LikeTally::default();
        self.scan_each(b"lk/", |key, value| {
            if let Some(id) = like_key_palette(key) {
                *tally.per_palette.entry(id.to_string()).or_insert(0) += 1;
                if decode_counter(value) == round {
                    *tally.this_round.entry(id.to_string()).or_insert(0) += 1;
                }
            }
        })
        .await?;
        Ok(tally)
    }

    pub async fn palette(&mut self, id: &str) -> io::Result<Option<Palette>> {
        let Some(primary) = self.store.get(index_key(id).as_bytes()).await? else {
            return Ok(None);
        };
        Ok(self
            .store
            .get(&primary)
            .await?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
    }

    /// Write a palette and its index entry in one batch. Idempotent: the
    /// keys are a pure function of the palette's id and timestamp.
    pub async fn insert_palette(&mut self, p: &Palette) -> io::Result<()> {
        let primary = palette_key(p);
        let index = index_key(&p.id);
        let json = serde_json::to_vec(p).map_err(io::Error::other)?;
        self.store
            .put_batch(&[
                (primary.as_bytes(), json.as_slice()),
                (index.as_bytes(), primary.as_bytes()),
            ])
            .await?;
        Ok(())
    }

    /// Remove a palette. The index entry goes last, so a retry after a
    /// partial failure still finds what is left to remove. Its like markers
    /// are then cleared best-effort: counts only include palettes that exist,
    /// so any marker left behind is never counted.
    pub async fn delete_palette(&mut self, id: &str) -> io::Result<bool> {
        let Some(primary) = self.store.get(index_key(id).as_bytes()).await? else {
            return Ok(false);
        };
        self.store.delete(&primary).await?;
        self.store.delete(index_key(id).as_bytes()).await?;

        let mut markers: Vec<Vec<u8>> = Vec::new();
        let cleanup = async {
            self.scan_each(like_prefix(id).as_bytes(), |key, _| markers.push(key.to_vec()))
                .await?;
            for key in &markers {
                self.store.delete(key).await?;
            }
            Ok::<(), io::Error>(())
        };
        if let Err(e) = cleanup.await {
            eprintln!("swatchbook: like cleanup for {id} left markers behind: {e}");
        }
        Ok(true)
    }

    /// The submission an idempotency key is bound to, if any.
    pub async fn submission(&mut self, digest: &str) -> io::Result<Option<Submission>> {
        Ok(self
            .store
            .get(submission_key(digest).as_bytes())
            .await?
            .as_deref()
            .and_then(Submission::decode))
    }

    /// Bind an idempotency key to `candidate` unless it is bound already,
    /// and return the binding that holds (ours, or the earlier attempt's).
    pub async fn claim_submission(
        &mut self,
        digest: &str,
        candidate: &Submission,
        expires_at_ms: u64,
    ) -> io::Result<Submission> {
        let key = submission_key(digest);
        let created = self
            .store
            .put_if_version_expiring(key.as_bytes(), candidate.encode().as_bytes(), None, expires_at_ms)
            .await?;
        if created.is_ok() {
            return Ok(candidate.clone());
        }
        Ok(self
            .submission(digest)
            .await?
            .unwrap_or_else(|| candidate.clone()))
    }

    /// Like a palette once per visitor. Returns (likes on it, counted).
    pub async fn like(&mut self, id: &str, voter: &str) -> io::Result<(i64, bool)> {
        let round = self.round().await?;
        let counted = self
            .create_marker(&like_marker(id, voter), round.to_string().as_bytes())
            .await?;
        Ok((self.count_prefix(&like_prefix(id)).await?, counted))
    }

    /// Vote for this variant's design, once per visitor per round.
    /// Returns (votes this round, counted).
    pub async fn vote(&mut self, voter: &str) -> io::Result<(i64, bool)> {
        let round = self.round().await?;
        let counted = self.create_marker(&vote_marker(round, voter), b"1").await?;
        Ok((self.count_prefix(&vote_prefix(round)).await?, counted))
    }

    /// Record a unique visitor for this round. True for a first visit.
    pub async fn visit(&mut self, voter: &str) -> io::Result<bool> {
        let round = self.round().await?;
        self.create_marker(&visit_marker(round, voter), b"1").await
    }

    /// Derive the current round's scoreboard from the stored records.
    async fn tally(
        &mut self,
        site: &Site,
        theme: &Theme,
        palettes: &[Palette],
        include_visitors: bool,
    ) -> io::Result<(Stats, LikeTally)> {
        let round = self.round().await?;
        let likes = self.like_tally(round).await?;
        let votes = self.count_prefix(&vote_prefix(round)).await?;
        let visitors = if include_visitors {
            self.count_prefix(&visit_prefix(round)).await?
        } else {
            0
        };
        let stats = Stats {
            label: site.label.clone(),
            design: theme.name.clone(),
            round,
            visitors,
            votes,
            likes: likes.round_total_for(palettes),
            adds: palettes.iter().filter(|p| p.round == Some(round)).count() as i64,
            palettes: palettes.len() as i64,
        };
        Ok((stats, likes))
    }

    /// The current round's scoreboard.
    pub async fn stats(&mut self, site: &Site, theme: &Theme) -> io::Result<Stats> {
        let palettes = self.list_palettes(MAX_LISTED).await?;
        Ok(self.tally(site, theme, &palettes, true).await?.0)
    }

    pub async fn admin_hash(&mut self) -> io::Result<Option<String>> {
        Ok(self
            .store
            .get(KEY_ADMIN.as_bytes())
            .await?
            .and_then(|b| String::from_utf8(b).ok()))
    }

    /// Set the admin key hash only if none is set yet.
    pub async fn claim_admin(&mut self, hash: &str) -> io::Result<bool> {
        self.create_marker(KEY_ADMIN, hash.as_bytes()).await
    }

    /// Read everything a page needs. Counting visitors is only needed by
    /// the Studio's scoreboard, so the gallery skips it.
    pub async fn snapshot(&mut self, include_visitors: bool) -> io::Result<Snapshot> {
        let theme = self.theme().await?;
        let site = self.site().await?;
        let palettes = self.list_palettes(MAX_LISTED).await?;
        let (stats, likes) = self.tally(&site, &theme, &palettes, include_visitors).await?;
        let admin_set = self.admin_hash().await?.is_some();
        Ok(Snapshot {
            theme,
            site,
            palettes,
            likes: likes.per_palette,
            stats,
            admin_set,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_decode_text_and_le_bytes() {
        assert_eq!(decode_counter(b"42"), 42);
        assert_eq!(decode_counter(b" -3 "), -3);
        assert_eq!(decode_counter(&7i64.to_le_bytes()), 7);
        assert_eq!(decode_counter(b""), 0);
    }

    #[test]
    fn palette_keys_sort_newest_first() {
        let mut older = model::seed_palettes().remove(0);
        older.created_ms = 1_000;
        let mut newer = older.clone();
        newer.created_ms = 2_000;
        assert!(palette_key(&newer) < palette_key(&older));
        assert_eq!(palette_key(&older).len(), "p/".len() + 16 + 1 + older.id.len());
    }

    #[test]
    fn submissions_round_trip() {
        let s = Submission {
            id: "abc123".into(),
            created_ms: 1700,
            round: 2,
        };
        assert_eq!(Submission::decode(s.encode().as_bytes()), Some(s));
        assert_eq!(Submission::decode(b"garbage"), None);
        assert_eq!(Submission::decode(b"bad id|1|0"), None);
        assert_eq!(Submission::decode(b"abc|1|0|extra"), None);
    }

    #[test]
    fn like_keys_name_their_palette() {
        assert_eq!(like_key_palette(b"lk/seed-01/visitor-1"), Some("seed-01"));
        assert_eq!(like_key_palette(b"lk/nope"), None);
        assert_eq!(like_key_palette(b"sb/0/visitor"), None);
    }

    #[test]
    fn round_likes_ignore_removed_palettes() {
        let palettes = model::seed_palettes();
        let mut tally = LikeTally::default();
        tally.this_round.insert("seed-01".into(), 2);
        tally.this_round.insert("deleted".into(), 5);
        assert_eq!(tally.round_total_for(&palettes), 2);
    }
}
