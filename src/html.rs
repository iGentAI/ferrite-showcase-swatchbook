//! Server-rendered pages.
//!
//! Every page is complete HTML on first paint (no client-side rendering), so
//! a fork, a preview and a slow phone all show the real content at once. The
//! theme arrives as CSS custom properties; the layout as a body class.

use ferrite_app::serde::Serialize;
use ferrite_app::serde_json;

use crate::assets;
use crate::model::{self, Palette, Site};
use crate::repo::Snapshot;
use crate::theme::{self, Theme};

/// Escape text for HTML content and double- or single-quoted attributes.
pub fn esc(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// JSON safe to embed in a `<script type="application/json">` element.
fn script_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "{}".to_string())
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

fn plural(n: i64, one: &str, many: &str) -> &'static str {
    // Returns a static word for the two fixed vocabularies used below.
    match (n == 1, one, many) {
        (true, "vote", _) => "vote",
        (false, _, "votes") => "votes",
        (true, "palette", _) => "palette",
        (false, _, "palettes") => "palettes",
        (true, "like", _) => "like",
        _ => "likes",
    }
}

fn document_head(out: &mut String, title: &str, theme: &Theme, extra_css: &str) {
    out.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    out.push_str("<title>");
    out.push_str(&esc(title));
    out.push_str("</title>");
    out.push_str("<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">");
    out.push_str("<link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin>");
    match theme::fonts_href(theme) {
        Some(href) => {
            out.push_str("<link id=\"sb-fonts\" rel=\"stylesheet\" href=\"");
            out.push_str(&esc(&href));
            out.push_str("\">");
        }
        None => out.push_str("<link id=\"sb-fonts\" rel=\"stylesheet\">"),
    }
    out.push_str("<style>");
    out.push_str(assets::BASE_CSS);
    out.push_str(extra_css);
    out.push_str("</style><style id=\"sb-theme\">");
    out.push_str(&theme::css_variables(theme));
    // Validated: custom CSS never contains '<', so it cannot leave <style>.
    out.push_str("</style><style id=\"sb-custom\">");
    out.push_str(&theme.custom_css);
    out.push_str("</style></head>");
}

/// The gallery page. `embed` trims it for the compare frames.
pub fn index(s: &Snapshot, embed: bool) -> String {
    let mut out = String::with_capacity(48 * 1024);
    let title = format!("{} · {}", s.site.title, s.site.label);
    document_head(&mut out, &title, &s.theme, "");
    out.push_str(&format!(
        "<body class=\"layout-{}{}\">",
        esc(&s.theme.layout),
        if embed { " embed" } else { "" }
    ));
    masthead(&mut out, s);
    gallery(&mut out, s);
    votebar(&mut out, s);
    add_dialog(&mut out);
    out.push_str("<script>");
    out.push_str(assets::APP_JS);
    out.push_str("</script></body></html>");
    out
}

/// The gallery with the live design editor docked beside it.
pub fn studio(s: &Snapshot) -> String {
    #[derive(Serialize)]
    #[serde(crate = "ferrite_app::serde")]
    struct Boot<'a> {
        theme: &'a Theme,
        site: &'a Site,
        admin_set: bool,
        catalog: serde_json::Value,
    }

    let mut out = String::with_capacity(64 * 1024);
    let title = format!("Studio · {} · {}", s.site.title, s.site.label);
    document_head(&mut out, &title, &s.theme, assets::STUDIO_CSS);
    out.push_str(&format!(
        "<body class=\"layout-{} studio\">",
        esc(&s.theme.layout)
    ));
    masthead(&mut out, s);
    gallery(&mut out, s);
    votebar(&mut out, s);
    add_dialog(&mut out);
    studio_panel(&mut out, s);
    let boot = Boot {
        theme: &s.theme,
        site: &s.site,
        admin_set: s.admin_set,
        catalog: theme::catalog(),
    };
    out.push_str("<script type=\"application/json\" id=\"sb-boot\">");
    out.push_str(&script_json(&boot));
    out.push_str("</script><script>");
    out.push_str(assets::APP_JS);
    out.push_str("</script><script>");
    out.push_str(assets::STUDIO_JS);
    out.push_str("</script></body></html>");
    out
}

/// Both variants side by side, for presenting.
pub fn compare(site: &Site, theme: &Theme) -> String {
    let mut out = String::with_capacity(8 * 1024);
    out.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    out.push_str(&format!(
        "<title>Same data, two designs · {}</title>",
        esc(&site.title)
    ));
    out.push_str("<style>");
    out.push_str(assets::COMPARE_CSS);
    out.push_str("</style></head><body class=\"compare\">");
    out.push_str(
        "<header class=\"cmp-head\"><b>Same data, two designs</b>\
         <span>Both designs started from the same palettes, copied by a Ferrite fork. \
         Only the design differs. Vote for the one that works better.</span></header>",
    );
    out.push_str("<main class=\"frames\">");
    out.push_str(&format!(
        "<figure><figcaption><span><b>{}</b> · {}</span>\
         <a href=\"/\" target=\"_blank\" rel=\"noopener\">Open</a></figcaption>\
         <iframe src=\"/?embed=1\" title=\"{}\" loading=\"lazy\"></iframe></figure>",
        esc(&site.label),
        esc(&theme.name),
        esc(&site.label)
    ));
    if site.sibling_url.is_empty() {
        out.push_str(
            "<figure class=\"placeholder\"><figcaption><span><b>The other design</b></span></figcaption>\
             <div class=\"empty\"><p>No second design is linked yet.</p>\
             <p>Fork this app on Ferrite, restyle the fork in its Studio, then paste the \
             fork's address into <em>Other design's URL</em> in this app's \
             <a href=\"/studio\">Studio</a>.</p></div></figure>",
        );
    } else {
        let url = esc(&site.sibling_url);
        out.push_str(&format!(
            "<figure><figcaption><span><b>The other design</b></span>\
             <a href=\"{url}\" target=\"_blank\" rel=\"noopener\">Open</a></figcaption>\
             <iframe src=\"{url}/?embed=1\" title=\"The other design\" loading=\"lazy\"></iframe></figure>"
        ));
    }
    out.push_str("</main>");
    out.push_str(
        "<p class=\"note\">Palettes added after the fork live only on the side they were added to. \
         If a frame stays blank, open the two designs in separate tabs.</p>",
    );
    out.push_str("</body></html>");
    out
}

fn masthead(out: &mut String, s: &Snapshot) {
    let total_likes: i64 = s
        .palettes
        .iter()
        .map(|p| s.likes.get(&p.id).copied().unwrap_or(0))
        .sum();
    let count = s.palettes.len() as i64;
    out.push_str(&format!(
        "<header class=\"masthead\">\
         <a class=\"brand\" href=\"/\" target=\"_top\"><span class=\"logo\" aria-hidden=\"true\"></span><span>{title}</span></a>\
         <span class=\"design-tag\"><span class=\"variant-label\">{label}</span> · <span class=\"design-name\">{design}</span></span>\
         <span class=\"spacer\"></span>\
         <button class=\"btn\" type=\"button\" data-open-add>Add a palette</button>\
         </header>\
         <section class=\"intro\"><h1>{tagline}</h1>\
         <p class=\"count\"><b>{count}</b> {palettes_word} · <b>{total_likes}</b> {likes_word}</p></section>",
        title = esc(&s.site.title),
        label = esc(&s.site.label),
        design = esc(&s.theme.name),
        tagline = esc(&s.site.tagline),
        palettes_word = plural(count, "palette", "palettes"),
        likes_word = plural(total_likes, "like", "likes"),
    ));
}

fn gallery(out: &mut String, s: &Snapshot) {
    out.push_str("<main class=\"pals\" id=\"pals\">");
    if s.palettes.is_empty() {
        out.push_str("<p class=\"empty\">No palettes yet. Add the first one.</p>");
    }
    for (i, palette) in s.palettes.iter().enumerate() {
        let likes = s.likes.get(&palette.id).copied().unwrap_or(0);
        palette_card(out, i + 1, palette, likes);
    }
    out.push_str("</main>");
}

fn palette_card(out: &mut String, position: usize, p: &Palette, likes: i64) {
    let id = esc(&p.id);
    let name = esc(&p.name);
    let hexes: Vec<String> = p.colors.iter().map(|c| c.to_uppercase()).collect();
    out.push_str(&format!(
        "<article class=\"pal\" data-id=\"{id}\">\
         <div class=\"sw\" role=\"img\" aria-label=\"Colours: {}\">",
        esc(&hexes.join(", "))
    ));
    for (colour, hex) in p.colors.iter().zip(&hexes) {
        let tone = if model::is_dark(colour) { "on-dark" } else { "on-light" };
        // Colours are validated #rrggbb values, safe inside the style attribute.
        out.push_str(&format!(
            "<span class=\"c {tone}\" style=\"--c:{}\"><i>{}</i></span>",
            esc(colour),
            esc(hex)
        ));
    }
    out.push_str(&format!(
        "</div><div class=\"meta\"><span class=\"idx\">{position:02}</span>\
         <div class=\"text\"><h2 class=\"pname\">{name}</h2><p class=\"by\">by {author}</p></div>\
         <button class=\"like\" type=\"button\" data-id=\"{id}\" aria-label=\"Like {name}\">\
         <span class=\"heart\" aria-hidden=\"true\">&#9829;</span><b>{likes}</b></button>\
         <button class=\"del\" type=\"button\" data-id=\"{id}\" title=\"Remove this palette\" aria-label=\"Remove {name}\">&times;</button>\
         </div></article>",
        author = esc(&p.author),
    ));
}

fn votebar(out: &mut String, s: &Snapshot) {
    let sibling = if s.site.sibling_url.is_empty() {
        String::new()
    } else {
        format!(
            "<a href=\"{}\" target=\"_top\">See the other design</a>",
            esc(&s.site.sibling_url)
        )
    };
    out.push_str(&format!(
        "<footer class=\"votebar\" data-round=\"{round}\">\
         <span class=\"vb-label\">You are looking at <b class=\"variant-label\">{label}</b>: <span class=\"design-name\">{design}</span></span>\
         <button class=\"btn vote\" id=\"vote\" type=\"button\">I prefer this design</button>\
         <span class=\"score\"><b id=\"votes\">{votes}</b> {votes_word}</span>\
         <span class=\"vb-links\">{sibling}<a href=\"/compare\" target=\"_top\">Compare side by side</a>\
         <a href=\"/studio\" target=\"_top\">Studio</a></span></footer>",
        round = s.stats.round,
        label = esc(&s.site.label),
        design = esc(&s.theme.name),
        votes = s.stats.votes,
        votes_word = plural(s.stats.votes, "vote", "votes"),
    ));
}

fn add_dialog(out: &mut String) {
    out.push_str(
        "<dialog id=\"add-dialog\" aria-labelledby=\"add-title\"><form id=\"add-form\" method=\"dialog\">\
         <h2 id=\"add-title\">Add a palette</h2>\
         <label class=\"field\" for=\"f-name\">Name</label>\
         <input id=\"f-name\" name=\"name\" type=\"text\" maxlength=\"40\" required placeholder=\"Late summer\">\
         <label class=\"field\" for=\"f-author\">Your name</label>\
         <input id=\"f-author\" name=\"author\" type=\"text\" maxlength=\"30\" placeholder=\"Optional\">\
         <span class=\"field\">Colours (3 to 6)</span>\
         <div class=\"colors\" id=\"f-colors\"></div>\
         <div class=\"color-tools\"><button type=\"button\" class=\"btn ghost\" id=\"f-more\">Add colour</button>\
         <button type=\"button\" class=\"btn ghost\" id=\"f-less\">Remove colour</button></div>\
         <p class=\"err\" id=\"f-err\" role=\"alert\"></p>\
         <div class=\"form-actions\"><button type=\"button\" class=\"btn ghost\" id=\"f-cancel\">Cancel</button>\
         <button type=\"submit\" class=\"btn\" id=\"f-submit\">Publish palette</button></div>\
         </form></dialog>",
    );
}

fn select_options(out: &mut String, options: &[(&str, &str)], selected: &str) {
    for (key, label) in options {
        out.push_str(&format!(
            "<option value=\"{}\"{}>{}</option>",
            esc(key),
            if *key == selected { " selected" } else { "" },
            esc(label)
        ));
    }
}

fn studio_panel(out: &mut String, s: &Snapshot) {
    let t = &s.theme;
    out.push_str(
        "<aside class=\"studio-panel\" aria-label=\"Design studio\">\
         <h2>Design studio</h2>\
         <p class=\"hint\">Every change previews live on the real palettes. Publish to make it the \
         design everyone sees on this variant.</p>",
    );

    // Scoreboard.
    out.push_str(&format!(
        "<h3>Scoreboard · round {}</h3><dl class=\"scores\">\
         <div><dt>Visitors</dt><dd>{}</dd></div><div><dt>Votes</dt><dd>{}</dd></div>\
         <div><dt>Likes</dt><dd>{}</dd></div><div><dt>Added</dt><dd>{}</dd></div></dl>",
        s.stats.round, s.stats.visitors, s.stats.votes, s.stats.likes, s.stats.adds
    ));

    // Presets.
    out.push_str("<h3>Start from a preset</h3><div class=\"presets\">");
    for key in theme::PRESET_KEYS {
        if let Some(p) = theme::preset(key) {
            out.push_str(&format!(
                "<button type=\"button\" class=\"preset\" data-preset=\"{key}\">\
                 <span style=\"background:linear-gradient(90deg,{bg} 0 34%,{accent} 34% 67%,{ink} 67%)\"></span>{name}</button>",
                key = esc(key),
                bg = esc(&p.bg),
                accent = esc(&p.accent),
                ink = esc(&p.ink),
                name = esc(&p.name),
            ));
        }
    }
    out.push_str("</div>");

    // Identity and layout.
    out.push_str(&format!(
        "<h3>Design</h3>\
         <label class=\"stack\">Design name<input id=\"t-name\" data-theme type=\"text\" maxlength=\"40\" value=\"{}\"></label>\
         <label class=\"stack\">Layout<select id=\"t-layout\" data-theme>",
        esc(&t.name)
    ));
    select_options(out, theme::LAYOUTS, &t.layout);
    out.push_str("</select></label>");

    // Colours.
    out.push_str("<h3>Colour</h3>");
    for (key, label, value) in [
        ("bg", "Background", &t.bg),
        ("surface", "Surface", &t.surface),
        ("ink", "Text", &t.ink),
        ("muted", "Muted text", &t.muted),
        ("accent", "Accent", &t.accent),
    ] {
        out.push_str(&format!(
            "<label>{label}<input type=\"color\" id=\"t-{key}\" data-theme value=\"{}\"></label>",
            esc(value)
        ));
    }

    // Type.
    let fonts: Vec<(&str, &str)> = theme::FONTS.iter().map(|f| (f.key, f.label)).collect();
    out.push_str("<h3>Type</h3><label class=\"stack\">Display font<select id=\"t-fd\" data-theme>");
    select_options(out, &fonts, &t.font_display);
    out.push_str("</select></label><label class=\"stack\">Body font<select id=\"t-fb\" data-theme>");
    select_options(out, &fonts, &t.font_body);
    out.push_str(&format!(
        "</select></label>\
         <label>Scale<input type=\"range\" id=\"t-scale\" data-theme min=\"{}\" max=\"{}\" step=\"5\" value=\"{}\"><output id=\"o-scale\">{}%</output></label>\
         <h3>Shape</h3>\
         <label>Corners<input type=\"range\" id=\"t-radius\" data-theme min=\"0\" max=\"{}\" step=\"1\" value=\"{}\"><output id=\"o-radius\">{}px</output></label>\
         <h3>Custom CSS</h3>\
         <textarea id=\"t-css\" data-theme spellcheck=\"false\" placeholder=\".pname {{ letter-spacing: .02em }}\">{}</textarea>",
        theme::SCALE_MIN,
        theme::SCALE_MAX,
        t.scale,
        t.scale,
        theme::RADIUS_MAX,
        t.radius,
        t.radius,
        esc(&t.custom_css)
    ));

    // Publish.
    out.push_str(
        "<h3>Publish</h3>\
         <label class=\"stack\">Admin key<input id=\"t-key\" type=\"password\" autocomplete=\"off\" placeholder=\"Needed to publish\"></label>",
    );
    if !s.admin_set {
        out.push_str(
            "<p class=\"hint warn\">No admin key is set for this app yet. Type one above \
             (at least 12 characters), keep a copy, and claim the app with it.</p>\
             <button type=\"button\" class=\"sbtn\" id=\"t-claim\">Claim this app with that key</button>",
        );
    }
    out.push_str(
        "<div class=\"publish-bar\"><button type=\"button\" class=\"sbtn primary\" id=\"t-publish\">Publish design</button>\
         <button type=\"button\" class=\"sbtn\" id=\"t-revert\">Revert to the live design</button>\
         <p class=\"status\" id=\"t-status\" role=\"status\"></p></div>",
    );

    // Variant settings.
    out.push_str(&format!(
        "<h3>This variant</h3>\
         <label class=\"stack\">Variant label<input id=\"s-label\" type=\"text\" maxlength=\"24\" value=\"{}\"></label>\
         <label class=\"stack\">Site title<input id=\"s-title\" type=\"text\" maxlength=\"40\" value=\"{}\"></label>\
         <label class=\"stack\">Headline<input id=\"s-tagline\" type=\"text\" maxlength=\"90\" value=\"{}\"></label>\
         <label class=\"stack\">Other design's URL<input id=\"s-sibling\" type=\"url\" placeholder=\"https://...\" value=\"{}\"></label>\
         <button type=\"button\" class=\"sbtn\" id=\"s-save\">Save variant settings</button>\
         <button type=\"button\" class=\"sbtn danger\" id=\"s-reset\">Start a new scoreboard round</button>\
         <p class=\"status\" id=\"s-status\" role=\"status\"></p>\
         </aside>",
        esc(&s.site.label),
        esc(&s.site.title),
        esc(&s.site.tagline),
        esc(&s.site.sibling_url)
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_covers_markup_and_quotes() {
        assert_eq!(esc("<a href='x'>\"&\"</a>"), "&lt;a href=&#39;x&#39;&gt;&quot;&amp;&quot;&lt;/a&gt;");
    }

    #[test]
    fn script_json_cannot_close_the_script_element() {
        let json = script_json(&serde_json::json!({ "x": "</script><b>&" }));
        assert!(!json.contains('<'));
        assert!(!json.contains('>'));
        assert!(json.contains("\\u003c/script\\u003e"));
    }
}
