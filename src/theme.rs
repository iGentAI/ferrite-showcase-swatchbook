//! The design as data.
//!
//! A [`Theme`] is everything that makes one variant look different from
//! another: layout, colours, fonts, corner radius, type scale, and an
//! optional block of custom CSS. It is stored in the `data` store (so a fork
//! copies it) and compiled to CSS custom properties on every page render.

use std::collections::BTreeMap;

use ferrite_app::serde::{Deserialize, Serialize};
use ferrite_app::serde_json::{json, Value};

use crate::model::{clean_text, ink_for, normalize_hex};

pub const MAX_THEME_NAME_CHARS: usize = 40;
pub const MAX_CUSTOM_CSS_BYTES: usize = 12_000;
pub const RADIUS_MAX: u32 = 40;
pub const SCALE_MIN: u32 = 80;
pub const SCALE_MAX: u32 = 140;

/// One complete design.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "ferrite_app::serde")]
pub struct Theme {
    /// Display name of the design, shown in the vote bar ("Poster").
    pub name: String,
    /// One of [`LAYOUTS`]: decides how palettes are arranged.
    pub layout: String,
    pub bg: String,
    pub surface: String,
    pub ink: String,
    pub muted: String,
    pub accent: String,
    /// Keys into [`FONTS`].
    pub font_display: String,
    pub font_body: String,
    /// Corner radius in px, `0..=RADIUS_MAX`.
    pub radius: u32,
    /// Root type scale in percent, `SCALE_MIN..=SCALE_MAX`.
    pub scale: u32,
    /// Extra CSS appended after the theme. May not contain `<`.
    #[serde(default)]
    pub custom_css: String,
}

/// Layout keys and their labels.
pub const LAYOUTS: &[(&str, &str)] = &[
    ("gallery", "Gallery grid"),
    ("poster", "Poster bands"),
    ("soft", "Soft cards"),
];

/// A font the Studio offers. `google` is the Google Fonts `family=` value,
/// loaded by the browser; `None` means a locally available stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(crate = "ferrite_app::serde")]
pub struct Font {
    pub key: &'static str,
    pub label: &'static str,
    pub stack: &'static str,
    pub google: Option<&'static str>,
}

pub const FONTS: &[Font] = &[
    Font {
        key: "system",
        label: "System UI",
        stack: "system-ui, -apple-system, \"Segoe UI\", Roboto, sans-serif",
        google: None,
    },
    Font {
        key: "inter",
        label: "Inter",
        stack: "\"Inter\", system-ui, sans-serif",
        google: Some("Inter:wght@400;600;800"),
    },
    Font {
        key: "space-grotesk",
        label: "Space Grotesk",
        stack: "\"Space Grotesk\", system-ui, sans-serif",
        google: Some("Space+Grotesk:wght@400;600;700"),
    },
    Font {
        key: "dm-sans",
        label: "DM Sans",
        stack: "\"DM Sans\", system-ui, sans-serif",
        google: Some("DM+Sans:wght@400;600;700"),
    },
    Font {
        key: "archivo-black",
        label: "Archivo Black",
        stack: "\"Archivo Black\", Impact, sans-serif",
        google: Some("Archivo+Black"),
    },
    Font {
        key: "fraunces",
        label: "Fraunces",
        stack: "\"Fraunces\", Georgia, serif",
        google: Some("Fraunces:ital,wght@0,400;0,700;1,700"),
    },
    Font {
        key: "dm-serif",
        label: "DM Serif Display",
        stack: "\"DM Serif Display\", Georgia, serif",
        google: Some("DM+Serif+Display"),
    },
    Font {
        key: "playfair",
        label: "Playfair Display",
        stack: "\"Playfair Display\", Georgia, serif",
        google: Some("Playfair+Display:wght@400;800"),
    },
    Font {
        key: "ibm-plex-mono",
        label: "IBM Plex Mono",
        stack: "\"IBM Plex Mono\", ui-monospace, monospace",
        google: Some("IBM+Plex+Mono:wght@400;600"),
    },
    Font {
        key: "georgia",
        label: "Georgia",
        stack: "Georgia, \"Times New Roman\", serif",
        google: None,
    },
];

/// Look up a font by key.
pub fn font(key: &str) -> Option<&'static Font> {
    FONTS.iter().find(|f| f.key == key)
}

/// The built-in starting points offered in the Studio.
pub const PRESET_KEYS: &[&str] = &["gallery", "poster", "soft"];

/// The design a fresh deployment starts with.
pub fn default_theme() -> Theme {
    gallery_preset()
}

fn gallery_preset() -> Theme {
    Theme {
        name: "Gallery".to_string(),
        layout: "gallery".to_string(),
        bg: "#f6f5f1".to_string(),
        surface: "#ffffff".to_string(),
        ink: "#141414".to_string(),
        muted: "#6b6b6b".to_string(),
        accent: "#ff4f00".to_string(),
        font_display: "inter".to_string(),
        font_body: "inter".to_string(),
        radius: 8,
        scale: 100,
        custom_css: String::new(),
    }
}

/// A built-in preset by key.
pub fn preset(key: &str) -> Option<Theme> {
    match key {
        "gallery" => Some(gallery_preset()),
        "poster" => Some(Theme {
            name: "Poster".to_string(),
            layout: "poster".to_string(),
            bg: "#0e0e0e".to_string(),
            surface: "#1a1a1a".to_string(),
            ink: "#f4f1ea".to_string(),
            muted: "#8d8a83".to_string(),
            accent: "#d7ff3a".to_string(),
            font_display: "archivo-black".to_string(),
            font_body: "ibm-plex-mono".to_string(),
            radius: 0,
            scale: 105,
            custom_css: String::new(),
        }),
        "soft" => Some(Theme {
            name: "Soft".to_string(),
            layout: "soft".to_string(),
            bg: "#f4f0ff".to_string(),
            surface: "#ffffff".to_string(),
            ink: "#2a2140".to_string(),
            muted: "#7a7091".to_string(),
            accent: "#7c5cff".to_string(),
            font_display: "fraunces".to_string(),
            font_body: "dm-sans".to_string(),
            radius: 22,
            scale: 100,
            custom_css: String::new(),
        }),
        _ => None,
    }
}

/// Validate and normalize a theme submitted from the Studio.
pub fn validate(input: &Theme) -> Result<Theme, String> {
    let name = clean_text(&input.name);
    if name.is_empty() || name.chars().count() > MAX_THEME_NAME_CHARS {
        return Err(format!(
            "Name the design in 1 to {MAX_THEME_NAME_CHARS} characters."
        ));
    }
    if !LAYOUTS.iter().any(|(key, _)| *key == input.layout) {
        return Err("Choose one of the available layouts.".to_string());
    }
    let colour = |label: &str, value: &str| {
        normalize_hex(value).ok_or_else(|| format!("{label} must be a hex colour such as #1a2b3c."))
    };
    let bg = colour("Background", &input.bg)?;
    let surface = colour("Surface", &input.surface)?;
    let ink = colour("Text", &input.ink)?;
    let muted = colour("Muted text", &input.muted)?;
    let accent = colour("Accent", &input.accent)?;
    if font(&input.font_display).is_none() || font(&input.font_body).is_none() {
        return Err("Choose fonts from the list.".to_string());
    }
    if input.radius > RADIUS_MAX {
        return Err(format!("Corner radius goes from 0 to {RADIUS_MAX} px."));
    }
    if !(SCALE_MIN..=SCALE_MAX).contains(&input.scale) {
        return Err(format!(
            "Type scale goes from {SCALE_MIN}% to {SCALE_MAX}%."
        ));
    }
    if input.custom_css.len() > MAX_CUSTOM_CSS_BYTES {
        return Err(format!(
            "Custom CSS is limited to {MAX_CUSTOM_CSS_BYTES} bytes."
        ));
    }
    // The CSS is placed inside a <style> element; without '<' it cannot
    // close that element or open another one.
    if input.custom_css.contains('<') {
        return Err("Custom CSS cannot contain the \"<\" character.".to_string());
    }
    Ok(Theme {
        name,
        layout: input.layout.clone(),
        bg,
        surface,
        ink,
        muted,
        accent,
        font_display: input.font_display.clone(),
        font_body: input.font_body.clone(),
        radius: input.radius,
        scale: input.scale,
        custom_css: input.custom_css.clone(),
    })
}

/// The theme as CSS custom properties on `:root`.
pub fn css_variables(t: &Theme) -> String {
    let display = font(&t.font_display).unwrap_or(&FONTS[0]).stack;
    let body = font(&t.font_body).unwrap_or(&FONTS[0]).stack;
    format!(
        ":root{{--bg:{};--surface:{};--ink:{};--muted:{};--accent:{};--accent-ink:{};\
         --radius:{}px;--scale:{}.{:02};--font-display:{};--font-body:{}}}",
        t.bg,
        t.surface,
        t.ink,
        t.muted,
        t.accent,
        ink_for(&t.accent),
        t.radius,
        t.scale / 100,
        t.scale % 100,
        display,
        body
    )
}

/// The Google Fonts stylesheet URL for the theme's fonts, if any are hosted.
pub fn fonts_href(t: &Theme) -> Option<String> {
    let mut families: Vec<&str> = Vec::new();
    for key in [&t.font_display, &t.font_body] {
        if let Some(family) = font(key).and_then(|f| f.google) {
            if !families.contains(&family) {
                families.push(family);
            }
        }
    }
    if families.is_empty() {
        return None;
    }
    let params: Vec<String> = families.iter().map(|f| format!("family={f}")).collect();
    Some(format!(
        "https://fonts.googleapis.com/css2?{}&display=swap",
        params.join("&")
    ))
}

/// Everything the Studio needs to offer choices: presets, fonts, layouts.
pub fn catalog() -> Value {
    let presets: BTreeMap<&str, Theme> = PRESET_KEYS
        .iter()
        .filter_map(|key| preset(key).map(|t| (*key, t)))
        .collect();
    let fonts: BTreeMap<&str, &Font> = FONTS.iter().map(|f| (f.key, f)).collect();
    let layouts: Vec<Value> = LAYOUTS
        .iter()
        .map(|(key, label)| json!({ "key": key, "label": label }))
        .collect();
    json!({ "presets": presets, "fonts": fonts, "layouts": layouts })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_valid() {
        for key in PRESET_KEYS {
            let t = preset(key).expect("preset exists");
            assert_eq!(validate(&t).as_ref(), Ok(&t), "preset {key}");
        }
        assert!(preset("nope").is_none());
    }

    #[test]
    fn validation_rejects_unsafe_or_out_of_range_values() {
        let base = default_theme();
        let mut t = base.clone();
        t.custom_css = "</style><script>alert(1)</script>".into();
        assert!(validate(&t).is_err());
        let mut t = base.clone();
        t.layout = "zigzag".into();
        assert!(validate(&t).is_err());
        let mut t = base.clone();
        t.accent = "red; background:url(x)".into();
        assert!(validate(&t).is_err());
        let mut t = base.clone();
        t.font_body = "comic-sans".into();
        assert!(validate(&t).is_err());
        let mut t = base.clone();
        t.scale = 200;
        assert!(validate(&t).is_err());
        let mut t = base;
        t.accent = "#ABC".into();
        assert_eq!(validate(&t).expect("valid").accent, "#aabbcc");
    }

    #[test]
    fn css_variables_and_fonts() {
        let t = preset("poster").expect("poster");
        let css = css_variables(&t);
        assert!(css.starts_with(":root{--bg:#0e0e0e;"));
        assert!(css.contains("--scale:1.05;"));
        assert!(css.contains("--accent-ink:#111111;"));
        let href = fonts_href(&t).expect("hosted fonts");
        assert_eq!(
            href,
            "https://fonts.googleapis.com/css2?family=Archivo+Black&family=IBM+Plex+Mono:wght@400;600&display=swap"
        );
        let mut system = t;
        system.font_display = "system".into();
        system.font_body = "georgia".into();
        assert_eq!(fonts_href(&system), None);
    }
}
