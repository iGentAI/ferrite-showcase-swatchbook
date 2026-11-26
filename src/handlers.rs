//! HTTP routing and endpoint logic.
//!
//! Pages: `GET /` (gallery, `?embed=1` for the compare frames), `GET /studio`
//! (live design editor), `GET /compare` (both variants side by side).
//!
//! JSON API (all bodies are JSON; admin calls carry `admin_key` in the body):
//! - `GET  /api/palettes`               palettes with like counts
//! - `POST /api/palettes`               add `{name, author, colors}` (honours `idempotency-key`)
//! - `POST /api/palettes/<id>/like`     `{voter}`, once per visitor
//! - `POST /api/palettes/<id>/delete`   admin
//! - `POST /api/vote`                   `{voter}`: vote for this variant's design
//! - `POST /api/visit`                  `{voter}`: count a unique visitor for this round
//! - `GET  /api/stats`                  this round's scoreboard
//! - `GET  /api/theme`, `POST /api/theme` (admin)  the design
//! - `GET  /api/site`,  `POST /api/site`  (admin)  variant label, copy, sibling URL
//! - `POST /api/admin/claim`            set the admin key if none is set
//! - `POST /api/admin/check`            verify an admin key
//! - `POST /api/admin/reset-scores`     admin: start a new scoreboard round

use std::io;

use ferrite_app::serde::de::DeserializeOwned;
use ferrite_app::serde::{Deserialize, Serialize};
use ferrite_app::serde_json::{self, json};
use ferrite_app::{AppContext, IngressReply, IngressRequest};

use crate::html;
use crate::model::{self, NewPalette, Palette, Site};
use crate::repo::{Repo, Submission};
use crate::theme::{self, Theme};

/// How long a submission's idempotency key is remembered.
const SUBMISSION_TTL_MS: u64 = 24 * 60 * 60 * 1000;

/// A response before it is wrapped for the gateway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

impl Response {
    pub fn html(body: String) -> Self {
        Self {
            status: 200,
            content_type: "text/html; charset=utf-8",
            body: body.into_bytes(),
        }
    }

    pub fn text(status: u16, body: &str) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8",
            body: body.as_bytes().to_vec(),
        }
    }

    pub fn json<T: Serialize>(status: u16, value: &T) -> Self {
        match serde_json::to_vec(value) {
            Ok(body) => Self {
                status,
                content_type: "application/json",
                body,
            },
            Err(_) => Self::text(500, "could not encode the response"),
        }
    }

    pub fn error(status: u16, message: &str) -> Self {
        Self::json(status, &json!({ "error": message }))
    }

    /// Wrap for the gateway, with content-type and no-store caching (pages
    /// change whenever the design or the data does).
    pub fn into_reply(self, correlation_id: &str) -> IngressReply {
        let mut reply = IngressReply::new(correlation_id.to_string(), self.status, self.body);
        reply.headers.push(("content-type".to_string(), self.content_type.to_string()));
        reply.headers.push(("cache-control".to_string(), "no-store".to_string()));
        reply.headers.push(("x-content-type-options".to_string(), "nosniff".to_string()));
        reply.headers.push(("referrer-policy".to_string(), "same-origin".to_string()));
        reply
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(crate = "ferrite_app::serde", default)]
struct VoterBody {
    voter: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(crate = "ferrite_app::serde", default)]
struct AdminBody {
    admin_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "ferrite_app::serde")]
struct ThemeUpdate {
    #[serde(default)]
    admin_key: String,
    theme: Theme,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "ferrite_app::serde")]
struct SiteUpdate {
    #[serde(default)]
    admin_key: String,
    site: Site,
}

#[derive(Debug, Serialize)]
#[serde(crate = "ferrite_app::serde")]
struct PaletteView<'a> {
    #[serde(flatten)]
    palette: &'a Palette,
    likes: i64,
}

/// Route one request. Never fails: storage errors become a 500 response.
pub async fn handle(ctx: &AppContext, repo: &mut Repo, req: &IngressRequest) -> Response {
    match route(ctx, repo, req).await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("swatchbook: {} {} failed: {e}", req.method, req.path);
            Response::error(500, "Something went wrong on our side. Please try again.")
        }
    }
}

/// Split `/path?query` into its parts.
fn split_target(target: &str) -> (&str, &str) {
    target.split_once('?').unwrap_or((target, ""))
}

/// True when `name`, `name=1` or `name=true` is in the query string.
fn query_flag(query: &str, name: &str) -> bool {
    query.split('&').any(|pair| {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        key == name && (value.is_empty() || value == "1" || value == "true")
    })
}

/// Parse a JSON body; an empty body reads as `{}`. Handlers match on the
/// result and return the 400 response on error. (Ferrite's code gate admits
/// no app-defined macros, so this is spelled out at each call site.)
fn parse_body<T: DeserializeOwned>(body: &[u8]) -> Result<T, Response> {
    let body: &[u8] = if body.iter().all(u8::is_ascii_whitespace) {
        b"{}"
    } else {
        body
    };
    serde_json::from_slice(body)
        .map_err(|_| Response::error(400, "The request body is not valid JSON for this endpoint."))
}

fn is_known_path(segments: &[&str]) -> bool {
    matches!(
        segments,
        [] | ["studio"]
            | ["compare"]
            | ["healthz"]
            | ["api", "palettes"]
            | ["api", "palettes", _, "like"]
            | ["api", "palettes", _, "delete"]
            | ["api", "vote"]
            | ["api", "visit"]
            | ["api", "stats"]
            | ["api", "theme"]
            | ["api", "site"]
            | ["api", "admin", "claim"]
            | ["api", "admin", "check"]
            | ["api", "admin", "reset-scores"]
    )
}

async fn route(ctx: &AppContext, repo: &mut Repo, req: &IngressRequest) -> io::Result<Response> {
    let (path, query) = split_target(&req.path);
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let method = req.method.to_ascii_uppercase();
    let body = req.body.as_slice();

    match (method.as_str(), segments.as_slice()) {
        ("GET", []) => page_index(repo, query_flag(query, "embed")).await,
        ("GET", ["studio"]) => page_studio(repo).await,
        ("GET", ["compare"]) => page_compare(repo).await,
        ("GET", ["healthz"]) => Ok(Response::text(200, "ok")),
        ("GET", ["api", "palettes"]) => list_palettes(repo).await,
        ("POST", ["api", "palettes"]) => add_palette(ctx, repo, req).await,
        ("POST", ["api", "palettes", id, "like"]) => like_palette(repo, id, body).await,
        ("POST", ["api", "palettes", id, "delete"]) => delete_palette(repo, id, body).await,
        ("POST", ["api", "vote"]) => vote(repo, body).await,
        ("POST", ["api", "visit"]) => visit(repo, body).await,
        ("GET", ["api", "stats"]) => stats(repo).await,
        ("GET", ["api", "theme"]) => get_theme(repo).await,
        ("POST", ["api", "theme"]) => set_theme(repo, body).await,
        ("GET", ["api", "site"]) => get_site(repo).await,
        ("POST", ["api", "site"]) => set_site(repo, body).await,
        ("POST", ["api", "admin", "claim"]) => claim_admin(repo, body).await,
        ("POST", ["api", "admin", "check"]) => check_admin(repo, body).await,
        ("POST", ["api", "admin", "reset-scores"]) => reset_scores(repo, body).await,
        (_, segments) if is_known_path(segments) => {
            Ok(Response::error(405, "That method is not allowed here."))
        }
        _ => Ok(Response::error(404, "Not found.")),
    }
}

/// True when `key` matches this app's admin key.
async fn authorized(repo: &mut Repo, key: &str) -> io::Result<bool> {
    let key = key.trim();
    if key.is_empty() {
        return Ok(false);
    }
    let Some(stored) = repo.admin_hash().await? else {
        return Ok(false);
    };
    Ok(model::constant_time_eq(
        model::admin_key_hash(key).as_bytes(),
        stored.as_bytes(),
    ))
}

fn forbidden() -> Response {
    Response::error(403, "That admin key is not right for this app.")
}

async fn page_index(repo: &mut Repo, embed: bool) -> io::Result<Response> {
    let snapshot = repo.snapshot(false).await?;
    Ok(Response::html(html::index(&snapshot, embed)))
}

async fn page_studio(repo: &mut Repo) -> io::Result<Response> {
    let snapshot = repo.snapshot(true).await?;
    Ok(Response::html(html::studio(&snapshot)))
}

async fn page_compare(repo: &mut Repo) -> io::Result<Response> {
    let site = repo.site().await?;
    let theme = repo.theme().await?;
    Ok(Response::html(html::compare(&site, &theme)))
}

async fn list_palettes(repo: &mut Repo) -> io::Result<Response> {
    let palettes = repo.list_palettes(crate::repo::MAX_LISTED).await?;
    let round = repo.round().await?;
    let likes = repo.like_tally(round).await?.per_palette;
    let views: Vec<PaletteView<'_>> = palettes
        .iter()
        .map(|palette| PaletteView {
            palette,
            likes: likes.get(&palette.id).copied().unwrap_or(0),
        })
        .collect();
    Ok(Response::json(200, &views))
}

async fn add_palette(ctx: &AppContext, repo: &mut Repo, req: &IngressRequest) -> io::Result<Response> {
    let input: NewPalette = match parse_body(&req.body) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    let valid = match model::validate_palette(&input) {
        Ok(valid) => valid,
        Err(message) => return Ok(Response::error(400, &message)),
    };

    // The gateway sets idempotency_key from the client's idempotency-key
    // header (else the correlation id). A retry of a submission that already
    // completed is answered first, so it stays answerable even once the
    // library has reached its cap.
    let digest = model::digest_hex(&req.idempotency_key, 16);
    if let Some(prior) = repo.submission(&digest).await? {
        if let Some(existing) = repo.palette(&prior.id).await? {
            return Ok(Response::json(200, &existing));
        }
    }
    if repo.palette_count().await? >= model::MAX_PALETTES {
        return Ok(Response::error(
            409,
            "The library is full for this demo. Ask the host to remove a few palettes.",
        ));
    }

    // Bind the key to an id, timestamp and round. Whichever attempt binds it
    // first decides them, and every attempt (concurrent, or a retry after an
    // interruption) writes the palette under those same values, so they all
    // land on the same storage keys. There is no separate counter to update:
    // the library size and the round's additions are counted from palettes.
    let now = ctx.now_ms();
    let candidate = Submission {
        id: model::new_palette_id(&format!("{now}|{}|{digest}", req.correlation_id)),
        created_ms: now,
        round: repo.round().await?,
    };
    let bound = repo
        .claim_submission(&digest, &candidate, now + SUBMISSION_TTL_MS)
        .await?;
    if bound != candidate {
        if let Some(existing) = repo.palette(&bound.id).await? {
            return Ok(Response::json(200, &existing));
        }
    }

    let palette = Palette {
        id: bound.id,
        name: valid.name,
        author: valid.author,
        colors: valid.colors,
        created_ms: bound.created_ms,
        round: Some(bound.round),
    };
    repo.insert_palette(&palette).await?;
    Ok(Response::json(201, &palette))
}

fn voter_or_400(body: &VoterBody) -> Result<&str, Response> {
    if model::valid_voter(&body.voter) {
        Ok(body.voter.as_str())
    } else {
        Err(Response::error(400, "A visitor id (voter) of 8 to 64 letters, digits or dashes is required."))
    }
}

async fn like_palette(repo: &mut Repo, id: &str, body: &[u8]) -> io::Result<Response> {
    if !model::valid_id(id) {
        return Ok(Response::error(404, "That palette does not exist."));
    }
    let input: VoterBody = match parse_body(body) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    let voter = match voter_or_400(&input) {
        Ok(voter) => voter,
        Err(response) => return Ok(response),
    };
    if repo.palette(id).await?.is_none() {
        return Ok(Response::error(404, "That palette does not exist."));
    }
    let (likes, counted) = repo.like(id, voter).await?;
    Ok(Response::json(200, &json!({ "likes": likes, "counted": counted })))
}

async fn delete_palette(repo: &mut Repo, id: &str, body: &[u8]) -> io::Result<Response> {
    let input: AdminBody = match parse_body(body) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    if !authorized(repo, &input.admin_key).await? {
        return Ok(forbidden());
    }
    if !model::valid_id(id) || !repo.delete_palette(id).await? {
        return Ok(Response::error(404, "That palette does not exist."));
    }
    Ok(Response::json(200, &json!({ "deleted": id })))
}

async fn vote(repo: &mut Repo, body: &[u8]) -> io::Result<Response> {
    let input: VoterBody = match parse_body(body) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    let voter = match voter_or_400(&input) {
        Ok(voter) => voter,
        Err(response) => return Ok(response),
    };
    let (votes, counted) = repo.vote(voter).await?;
    Ok(Response::json(200, &json!({ "votes": votes, "counted": counted })))
}

async fn visit(repo: &mut Repo, body: &[u8]) -> io::Result<Response> {
    let input: VoterBody = match parse_body(body) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    let voter = match voter_or_400(&input) {
        Ok(voter) => voter,
        Err(response) => return Ok(response),
    };
    let counted = repo.visit(voter).await?;
    Ok(Response::json(200, &json!({ "counted": counted })))
}

async fn stats(repo: &mut Repo) -> io::Result<Response> {
    let site = repo.site().await?;
    let theme = repo.theme().await?;
    let stats = repo.stats(&site, &theme).await?;
    Ok(Response::json(200, &stats))
}

async fn get_theme(repo: &mut Repo) -> io::Result<Response> {
    let theme = repo.theme().await?;
    Ok(Response::json(
        200,
        &json!({ "theme": theme, "catalog": theme::catalog() }),
    ))
}

async fn set_theme(repo: &mut Repo, body: &[u8]) -> io::Result<Response> {
    let input: ThemeUpdate = match parse_body(body) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    if !authorized(repo, &input.admin_key).await? {
        return Ok(forbidden());
    }
    let theme = match theme::validate(&input.theme) {
        Ok(theme) => theme,
        Err(message) => return Ok(Response::error(400, &message)),
    };
    repo.set_theme(&theme).await?;
    Ok(Response::json(200, &json!({ "theme": theme })))
}

async fn get_site(repo: &mut Repo) -> io::Result<Response> {
    let site = repo.site().await?;
    Ok(Response::json(200, &json!({ "site": site })))
}

async fn set_site(repo: &mut Repo, body: &[u8]) -> io::Result<Response> {
    let input: SiteUpdate = match parse_body(body) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    if !authorized(repo, &input.admin_key).await? {
        return Ok(forbidden());
    }
    let site = match model::validate_site(&input.site) {
        Ok(site) => site,
        Err(message) => return Ok(Response::error(400, &message)),
    };
    repo.set_site(&site).await?;
    Ok(Response::json(200, &json!({ "site": site })))
}

async fn claim_admin(repo: &mut Repo, body: &[u8]) -> io::Result<Response> {
    let input: AdminBody = match parse_body(body) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    let key = input.admin_key.trim();
    if key.chars().count() < model::MIN_ADMIN_KEY_CHARS {
        return Ok(Response::error(
            400,
            &format!(
                "Choose an admin key of at least {} characters.",
                model::MIN_ADMIN_KEY_CHARS
            ),
        ));
    }
    if repo.claim_admin(&model::admin_key_hash(key)).await? {
        Ok(Response::json(201, &json!({ "claimed": true })))
    } else {
        Ok(Response::error(409, "An admin key is already set for this app."))
    }
}

async fn check_admin(repo: &mut Repo, body: &[u8]) -> io::Result<Response> {
    let input: AdminBody = match parse_body(body) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    if authorized(repo, &input.admin_key).await? {
        Ok(Response::json(200, &json!({ "ok": true })))
    } else {
        Ok(forbidden())
    }
}

async fn reset_scores(repo: &mut Repo, body: &[u8]) -> io::Result<Response> {
    let input: AdminBody = match parse_body(body) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    if !authorized(repo, &input.admin_key).await? {
        return Ok(forbidden());
    }
    let round = repo.next_round().await?;
    let site = repo.site().await?;
    let theme = repo.theme().await?;
    let stats = repo.stats(&site, &theme).await?;
    Ok(Response::json(200, &json!({ "round": round, "stats": stats })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_and_flags() {
        assert_eq!(split_target("/?embed=1"), ("/", "embed=1"));
        assert_eq!(split_target("/studio"), ("/studio", ""));
        assert!(query_flag("a=2&embed=1", "embed"));
        assert!(query_flag("embed", "embed"));
        assert!(!query_flag("embed=0", "embed"));
        assert!(!query_flag("embedded=1", "embed"));
    }

    #[test]
    fn empty_bodies_read_as_empty_objects() {
        let parsed: VoterBody = parse_body(b"").expect("empty body");
        assert_eq!(parsed.voter, "");
        assert!(parse_body::<VoterBody>(b"not json").is_err());
    }
}
