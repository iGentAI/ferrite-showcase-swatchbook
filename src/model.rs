//! Domain model: palettes, per-variant site settings, input validation, and
//! the seed library a fresh deployment starts with.

// Only `sha256` is taken from the platform: it is the one crypto function
// whose name is the same in the local SDK and the hosted one.
use ferrite_app::crypto::sha256;
use ferrite_app::serde::{Deserialize, Serialize};

pub const MIN_COLORS: usize = 3;
pub const MAX_COLORS: usize = 6;
pub const MAX_NAME_CHARS: usize = 40;
pub const MAX_AUTHOR_CHARS: usize = 30;
/// Upper bound on the library size, so an open submission form cannot grow
/// the store (or the rendered page) without limit.
pub const MAX_PALETTES: i64 = 500;
pub const MIN_ADMIN_KEY_CHARS: usize = 12;
pub const MAX_SITE_TITLE_CHARS: usize = 40;
pub const MAX_TAGLINE_CHARS: usize = 90;
pub const MAX_LABEL_CHARS: usize = 24;
pub const MAX_URL_CHARS: usize = 200;
/// Author shown on the seed palettes.
pub const SEED_AUTHOR: &str = "Swatchbook";

/// A published palette.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "ferrite_app::serde")]
pub struct Palette {
    pub id: String,
    pub name: String,
    pub author: String,
    /// Normalized lowercase `#rrggbb` colours, in display order.
    pub colors: Vec<String>,
    pub created_ms: u64,
    /// Scoreboard round the palette was added in (`None` for the seeds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub round: Option<i64>,
}

/// A palette submission as it arrives from the browser.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(crate = "ferrite_app::serde", default)]
pub struct NewPalette {
    pub name: String,
    pub author: String,
    pub colors: Vec<String>,
}

/// A submission that passed validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidPalette {
    pub name: String,
    pub author: String,
    pub colors: Vec<String>,
}

/// Settings that belong to one variant rather than to its design: its label
/// ("Design A"), the copy at the top of the page, and the URL of the other
/// variant it is being compared with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "ferrite_app::serde")]
pub struct Site {
    pub title: String,
    pub tagline: String,
    pub label: String,
    #[serde(default)]
    pub sibling_url: String,
}

impl Default for Site {
    fn default() -> Self {
        Site {
            title: "Swatchbook".to_string(),
            tagline: "Colour palettes, picked by designers.".to_string(),
            label: "Design A".to_string(),
            sibling_url: String::new(),
        }
    }
}

/// Normalize `#abc`, `abc`, `#aabbcc` or `aabbcc` to lowercase `#aabbcc`.
pub fn normalize_hex(input: &str) -> Option<String> {
    let trimmed = input.trim();
    let digits = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match digits.len() {
        6 => Some(format!("#{}", digits.to_ascii_lowercase())),
        3 => {
            let mut out = String::from("#");
            for c in digits.chars() {
                let c = c.to_ascii_lowercase();
                out.push(c);
                out.push(c);
            }
            Some(out)
        }
        _ => None,
    }
}

/// Collapse runs of whitespace and drop control characters.
pub fn clean_text(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| !c.is_control())
        .collect()
}

fn snippet(input: &str) -> String {
    input.chars().take(12).collect()
}

/// Validate and normalize a palette submission.
pub fn validate_palette(input: &NewPalette) -> Result<ValidPalette, String> {
    let name = clean_text(&input.name);
    if name.is_empty() {
        return Err("Give the palette a name.".to_string());
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(format!(
            "Keep the name to {MAX_NAME_CHARS} characters or fewer."
        ));
    }
    let mut author = clean_text(&input.author);
    if author.chars().count() > MAX_AUTHOR_CHARS {
        return Err(format!(
            "Keep your name to {MAX_AUTHOR_CHARS} characters or fewer."
        ));
    }
    if author.is_empty() {
        author = "anonymous".to_string();
    }
    if input.colors.len() < MIN_COLORS || input.colors.len() > MAX_COLORS {
        return Err(format!(
            "A palette needs between {MIN_COLORS} and {MAX_COLORS} colours."
        ));
    }
    let mut colors = Vec::with_capacity(input.colors.len());
    for raw in &input.colors {
        match normalize_hex(raw) {
            Some(c) => colors.push(c),
            None => {
                return Err(format!(
                    "\"{}\" is not a hex colour such as #1a2b3c.",
                    snippet(raw)
                ))
            }
        }
    }
    Ok(ValidPalette {
        name,
        author,
        colors,
    })
}

/// Validate and normalize variant settings.
pub fn validate_site(input: &Site) -> Result<Site, String> {
    let title = clean_text(&input.title);
    if title.is_empty() || title.chars().count() > MAX_SITE_TITLE_CHARS {
        return Err(format!(
            "The site title needs 1 to {MAX_SITE_TITLE_CHARS} characters."
        ));
    }
    let tagline = clean_text(&input.tagline);
    if tagline.is_empty() || tagline.chars().count() > MAX_TAGLINE_CHARS {
        return Err(format!(
            "The headline needs 1 to {MAX_TAGLINE_CHARS} characters."
        ));
    }
    let label = clean_text(&input.label);
    if label.is_empty() || label.chars().count() > MAX_LABEL_CHARS {
        return Err(format!(
            "The variant label needs 1 to {MAX_LABEL_CHARS} characters."
        ));
    }
    let sibling_url = input.sibling_url.trim().trim_end_matches('/').to_string();
    if !sibling_url.is_empty() && !is_safe_https_url(&sibling_url) {
        return Err("The other design's URL must be a plain https:// address, \
                    such as https://abc123.ferrite.run."
            .to_string());
    }
    Ok(Site {
        title,
        tagline,
        label,
        sibling_url,
    })
}

/// An `https://` URL made only of host and path characters: no query,
/// fragment, quotes, spaces or credentials. Safe to place in `href`/`src`.
pub fn is_safe_https_url(url: &str) -> bool {
    if url.len() > MAX_URL_CHARS {
        return false;
    }
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or("");
    !host.is_empty()
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b':')
        && rest
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~/:%".contains(&b))
}

/// A browser-generated visitor id: 8 to 64 characters of `[A-Za-z0-9-]`.
pub fn valid_voter(voter: &str) -> bool {
    (8..=64).contains(&voter.len()) && voter.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// A palette id: 1 to 32 characters of `[A-Za-z0-9-]`.
pub fn valid_id(id: &str) -> bool {
    (1..=32).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// Lowercase hexadecimal of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 0x0f)]));
    }
    out
}

/// Compare two byte strings in time that depends only on their lengths, so
/// checking an admin key reveals nothing about how much of it matched.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

/// Hex of the first `bytes` bytes of SHA-256(`input`).
pub fn digest_hex(input: &str, bytes: usize) -> String {
    let digest = sha256(input.as_bytes());
    hex(&digest[..bytes.min(digest.len())])
}

/// A 12-hex-character palette id derived from a unique seed string.
pub fn new_palette_id(seed: &str) -> String {
    digest_hex(seed, 6)
}

/// The stored form of an admin key (the key itself is never stored).
pub fn admin_key_hash(key: &str) -> String {
    digest_hex(&format!("swatchbook-admin-v1:{key}"), 32)
}

/// WCAG relative luminance of a normalized `#rrggbb` colour.
pub fn luminance(hex: &str) -> f64 {
    let value = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
    let channel = |shift: u32| {
        let c = f64::from((value >> shift) & 0xff) / 255.0;
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
}

/// True when white text contrasts better than near-black on this colour.
pub fn is_dark(hex: &str) -> bool {
    let l = luminance(hex);
    1.05 / (l + 0.05) > (l + 0.05) / 0.05
}

/// The text colour to put on top of `hex`.
pub fn ink_for(hex: &str) -> &'static str {
    if is_dark(hex) {
        "#ffffff"
    } else {
        "#111111"
    }
}

/// Fixed creation time for the seeds, so seeding is idempotent and every
/// palette added later sorts above them.
const SEED_BASE_MS: u64 = 1_762_000_000_000;

const SEEDS: &[(&str, &[&str])] = &[
    ("Bauhaus Primaries", &["#d62828", "#f7b801", "#1d4e89", "#111111", "#f2efe6"]),
    ("Memphis Milano", &["#ff6f91", "#ffc75f", "#00c2a8", "#845ec2", "#1b1b1b"]),
    ("Swiss Rail", &["#eb0000", "#ffffff", "#2d2d2d", "#8c8c8c"]),
    ("Risograph Run", &["#ff48b0", "#0078bf", "#ffe800", "#00a95c", "#f4f0e6"]),
    ("Terracotta Studio", &["#c8553d", "#f28f3b", "#ffd5c2", "#588b8b", "#2e2e2e"]),
    ("Nordic Winter", &["#e9eef2", "#a7bcc9", "#5c7a8c", "#2b3a44", "#d9c5a0"]),
    ("Neon Arcade", &["#0b0f2b", "#ff2e88", "#00f0ff", "#f9f871", "#7b2ff7"]),
    ("Matisse Cut-outs", &["#1f3a93", "#f25c54", "#f7b32b", "#2e933c", "#f4ebd9"]),
    ("Darkroom", &["#0a0a0a", "#3b0d0c", "#a4161a", "#e5383b", "#f5f3f4"]),
    ("Pistachio Gelato", &["#93c572", "#d8e9a8", "#f6f1d1", "#e8b4bc", "#6b4e3d"]),
    ("Concrete & Copper", &["#bfbfbf", "#8a8a8a", "#4a4a4a", "#b87333", "#f0ede8"]),
    ("Ocean Editorial", &["#03045e", "#0077b6", "#00b4d8", "#90e0ef", "#caf0f8"]),
];

/// The library a fresh deployment starts with, newest first.
pub fn seed_palettes() -> Vec<Palette> {
    SEEDS
        .iter()
        .enumerate()
        .map(|(i, (name, colors))| Palette {
            id: format!("seed-{:02}", i + 1),
            name: (*name).to_string(),
            author: SEED_AUTHOR.to_string(),
            colors: colors.iter().map(|c| (*c).to_string()).collect(),
            created_ms: SEED_BASE_MS - (i as u64) * 60_000,
            round: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colours_normalize() {
        assert_eq!(normalize_hex("#ABC").as_deref(), Some("#aabbcc"));
        assert_eq!(normalize_hex(" 1A2b3C ").as_deref(), Some("#1a2b3c"));
        assert_eq!(normalize_hex("#12345"), None);
        assert_eq!(normalize_hex("red"), None);
        assert_eq!(normalize_hex("#ggg"), None);
    }

    #[test]
    fn palette_validation() {
        let ok = NewPalette {
            name: "  Late   summer ".into(),
            author: String::new(),
            colors: vec!["#fff".into(), "000000".into(), "#123abc".into()],
        };
        let v = validate_palette(&ok).expect("valid");
        assert_eq!(v.name, "Late summer");
        assert_eq!(v.author, "anonymous");
        assert_eq!(v.colors, vec!["#ffffff", "#000000", "#123abc"]);

        let too_few = NewPalette {
            colors: vec!["#fff".into()],
            ..ok.clone()
        };
        assert!(validate_palette(&too_few).is_err());
        let bad_colour = NewPalette {
            colors: vec!["#fff".into(), "#000".into(), "blue".into()],
            ..ok.clone()
        };
        assert!(validate_palette(&bad_colour).unwrap_err().contains("blue"));
        let unnamed = NewPalette {
            name: "   ".into(),
            ..ok
        };
        assert!(validate_palette(&unnamed).is_err());
    }

    #[test]
    fn sibling_urls_must_be_plain_https() {
        assert!(is_safe_https_url("https://abc.ferrite.run"));
        assert!(is_safe_https_url("https://example.com/path/to"));
        assert!(!is_safe_https_url("http://example.com"));
        assert!(!is_safe_https_url("javascript:alert(1)"));
        assert!(!is_safe_https_url("https://example.com/?q=1"));
        assert!(!is_safe_https_url("https://\"onmouseover=x"));
        assert!(!is_safe_https_url("https:///path"));
    }

    #[test]
    fn hex_and_constant_time_comparison() {
        assert_eq!(hex(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
        assert_eq!(hex(&[]), "");
        assert!(constant_time_eq(b"same", b"same"));
        assert!(!constant_time_eq(b"same", b"sane"));
        assert!(!constant_time_eq(b"short", b"longer"));
        // SHA-256("abc"), the FIPS 180-2 test vector.
        assert_eq!(
            digest_hex("abc", 32),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(new_palette_id("abc").len(), 12);
    }

    #[test]
    fn contrast_picks_readable_ink() {
        assert_eq!(ink_for("#000000"), "#ffffff");
        assert_eq!(ink_for("#ffffff"), "#111111");
        assert_eq!(ink_for("#d7ff3a"), "#111111");
        assert_eq!(ink_for("#1d4e89"), "#ffffff");
    }

    #[test]
    fn seeds_are_valid_and_ordered() {
        let seeds = seed_palettes();
        assert_eq!(seeds.len(), 12);
        for pair in seeds.windows(2) {
            assert!(pair[0].created_ms > pair[1].created_ms);
        }
        for p in &seeds {
            assert!(valid_id(&p.id));
            for c in &p.colors {
                assert_eq!(normalize_hex(c).as_deref(), Some(c.as_str()));
            }
        }
    }
}
