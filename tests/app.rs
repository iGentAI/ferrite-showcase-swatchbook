//! End-to-end tests: the real app, driven through the `ferrite-app-test`
//! harness exactly as the Ferrite gateway drives it (requests arrive on the
//! `requests` queue, replies go back on `replies`, data lives in `data`).
//!
//! The fault tests write directly into the app's store to recreate what an
//! interrupted request leaves behind, then check that a retry finishes the
//! work exactly once.

use ferrite_app::serde_json::{self, json, Value};
use ferrite_app_test::{ingress, IngressReply, IngressRequest, TestEnv, TestHarness};

const ADMIN_KEY: &str = "correct-horse-battery";

async fn start() -> TestEnv {
    let env = TestHarness::new()
        .with_queue("requests")
        .with_queue("replies")
        .with_store("data")
        .build()
        .await;
    env.run_app(|ctx| swatchbook::run(ctx));
    env
}

async fn send(env: &TestEnv, req: IngressRequest) -> IngressReply {
    env.handle(req).await.expect("the app replied")
}

fn header(reply: &IngressReply, name: &str) -> Option<String> {
    reply
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

fn body_json(reply: &IngressReply) -> Value {
    serde_json::from_slice(&reply.body).unwrap_or(Value::Null)
}

async fn page(env: &TestEnv, path: &str) -> String {
    let reply = send(env, ingress::get(path)).await;
    assert_eq!(reply.status, 200, "GET {path}");
    assert_eq!(
        header(&reply, "content-type").as_deref(),
        Some("text/html; charset=utf-8")
    );
    String::from_utf8(reply.body).expect("utf-8 page")
}

async fn get_json(env: &TestEnv, path: &str) -> (u16, Value) {
    let reply = send(env, ingress::get(path)).await;
    (reply.status, body_json(&reply))
}

async fn post(env: &TestEnv, path: &str, body: Value) -> (u16, Value) {
    let reply = send(env, ingress::post(path, body.to_string())).await;
    (reply.status, body_json(&reply))
}

async fn post_keyed(env: &TestEnv, path: &str, key: &str, body: Value) -> (u16, Value) {
    let reply = send(env, ingress::post_keyed(path, key, body.to_string())).await;
    (reply.status, body_json(&reply))
}

async fn claim(env: &TestEnv) {
    let (status, body) = post(env, "/api/admin/claim", json!({ "admin_key": ADMIN_KEY })).await;
    assert_eq!(status, 201, "{body}");
}

fn voter(n: u32) -> String {
    format!("visitor-{n:04}")
}

fn poster_theme() -> Value {
    json!({
        "name": "Poster", "layout": "poster",
        "bg": "#0e0e0e", "surface": "#1a1a1a", "ink": "#f4f1ea", "muted": "#8d8a83", "accent": "#d7ff3a",
        "font_display": "archivo-black", "font_body": "ibm-plex-mono",
        "radius": 0, "scale": 105, "custom_css": ".pname{letter-spacing:.02em}"
    })
}

fn new_palette(name: &str) -> Value {
    json!({ "name": name, "author": "Ana", "colors": ["#F4A261", "#e76f51", "#2a9d8f", "#264653"] })
}

async fn palette_names(env: &TestEnv) -> Vec<String> {
    let (status, list) = get_json(env, "/api/palettes").await;
    assert_eq!(status, 200);
    list.as_array()
        .expect("array")
        .iter()
        .map(|p| p["name"].as_str().unwrap_or_default().to_string())
        .collect()
}

#[tokio::test]
async fn home_page_serves_the_seed_library_in_the_default_design() {
    let env = start().await;
    let reply = send(&env, ingress::get("/")).await;
    assert_eq!(reply.status, 200);
    assert_eq!(header(&reply, "cache-control").as_deref(), Some("no-store"));
    let html = String::from_utf8(reply.body).expect("utf-8");
    assert!(html.contains("<body class=\"layout-gallery\">"));
    assert!(html.contains("--accent:#ff4f00"));
    assert!(html.contains("family=Inter"));
    assert!(html.contains("<b>12</b> palettes"));
    assert!(html.contains("Design A"));
    assert!(html.contains("Concrete &amp; Copper"));
    let newest = html.find("Bauhaus Primaries").expect("first seed");
    let oldest = html.find("Ocean Editorial").expect("last seed");
    assert!(newest < oldest, "newest palettes come first");

    let embedded = page(&env, "/?embed=1").await;
    assert!(embedded.contains("<body class=\"layout-gallery embed\">"));

    let health = send(&env, ingress::get("/healthz")).await;
    assert_eq!((health.status, health.body.as_slice()), (200, b"ok".as_slice()));

    let names = palette_names(&env).await;
    assert_eq!(names.len(), 12);
    assert_eq!(names[0], "Bauhaus Primaries");
    env.shutdown().await;
}

#[tokio::test]
async fn submissions_are_validated() {
    let env = start().await;
    let cases = [
        (json!({ "name": "", "colors": ["#fff", "#000", "#123"] }), "name"),
        (json!({ "name": "Few", "colors": ["#fff"] }), "between 3 and 6"),
        (json!({ "name": "Bad", "colors": ["#fff", "#000", "tomato"] }), "tomato"),
        (json!({ "name": "x".repeat(41), "colors": ["#fff", "#000", "#123"] }), "40 characters"),
    ];
    for (body, needle) in cases {
        let (status, err) = post(&env, "/api/palettes", body).await;
        assert_eq!(status, 400);
        let message = err["error"].as_str().unwrap_or_default();
        assert!(message.contains(needle), "{message}");
    }
    let reply = send(&env, ingress::post("/api/palettes", "not json")).await;
    assert_eq!(reply.status, 400);
    assert_eq!(palette_names(&env).await.len(), 12);
    env.shutdown().await;
}

#[tokio::test]
async fn a_retried_submission_creates_one_palette() {
    let env = start().await;
    let (status, first) = post_keyed(&env, "/api/palettes", "submit-1", new_palette("Late summer")).await;
    assert_eq!(status, 201, "{first}");
    assert_eq!(first["colors"][0], "#f4a261");
    assert_eq!(first["author"], "Ana");
    assert_eq!(first["round"], 0);

    let (status, again) = post_keyed(&env, "/api/palettes", "submit-1", new_palette("Late summer")).await;
    assert_eq!(status, 200);
    assert_eq!(again["id"], first["id"]);

    let names = palette_names(&env).await;
    assert_eq!(names.len(), 13);
    assert_eq!(names[0], "Late summer");
    let (_, stats) = get_json(&env, "/api/stats").await;
    assert_eq!((stats["adds"].as_i64(), stats["palettes"].as_i64()), (Some(1), Some(13)));
    assert!(page(&env, "/").await.contains("<b>13</b> palettes"));
    env.shutdown().await;
}

#[tokio::test]
async fn concurrent_attempts_of_one_submission_create_one_palette() {
    let env = start().await;
    let attempts = (0..4).map(|_| {
        ingress::post_keyed("/api/palettes", "same-key", new_palette("Raced").to_string())
    });
    let replies = env.handle_all(attempts).await;
    let mut ids = Vec::new();
    for reply in replies {
        let reply = reply.expect("reply");
        assert!(matches!(reply.status, 200 | 201), "status {}", reply.status);
        ids.push(body_json(&reply)["id"].as_str().unwrap_or_default().to_string());
    }
    ids.dedup();
    assert_eq!(ids.len(), 1, "every attempt resolved to the same palette");
    let names = palette_names(&env).await;
    assert_eq!(names.iter().filter(|n| *n == "Raced").count(), 1);
    let (_, stats) = get_json(&env, "/api/stats").await;
    assert_eq!((stats["adds"].as_i64(), stats["palettes"].as_i64()), (Some(1), Some(13)));
    env.shutdown().await;
}

#[tokio::test]
async fn a_submission_interrupted_after_binding_its_key_is_finished_by_the_retry() {
    let env = start().await;
    // What a crash right after binding the key leaves: the binding, no palette.
    let digest = swatchbook::model::digest_hex("interrupted", 16);
    let mut store = env.store("data");
    store
        .put(format!("ik/{digest}").as_bytes(), b"abc123def456|1800000000000|0")
        .await
        .expect("seed the binding");

    let (status, palette) = post_keyed(&env, "/api/palettes", "interrupted", new_palette("Recovered")).await;
    assert_eq!(status, 201, "{palette}");
    assert_eq!(palette["id"], "abc123def456");
    assert_eq!(palette["created_ms"], 1_800_000_000_000u64);

    let (status, again) = post_keyed(&env, "/api/palettes", "interrupted", new_palette("Recovered")).await;
    assert_eq!((status, again["id"].clone()), (200, json!("abc123def456")));
    let names = palette_names(&env).await;
    assert_eq!(names.iter().filter(|n| *n == "Recovered").count(), 1);
    env.shutdown().await;
}

#[tokio::test]
async fn likes_count_once_per_visitor() {
    let env = start().await;
    let path = "/api/palettes/seed-01/like";
    let (status, first) = post(&env, path, json!({ "voter": voter(1) })).await;
    assert_eq!(status, 200);
    assert_eq!((first["likes"].as_i64(), first["counted"].as_bool()), (Some(1), Some(true)));
    let (_, repeat) = post(&env, path, json!({ "voter": voter(1) })).await;
    assert_eq!((repeat["likes"].as_i64(), repeat["counted"].as_bool()), (Some(1), Some(false)));
    let (_, second) = post(&env, path, json!({ "voter": voter(2) })).await;
    assert_eq!(second["likes"].as_i64(), Some(2));

    assert_eq!(post(&env, "/api/palettes/nope/like", json!({ "voter": voter(1) })).await.0, 404);
    assert_eq!(post(&env, path, json!({ "voter": "x" })).await.0, 400);

    let (_, list) = get_json(&env, "/api/palettes").await;
    assert_eq!(list[0]["likes"].as_i64(), Some(2));
    assert!(page(&env, "/").await.contains("<b>2</b> likes"));
    let (_, stats) = get_json(&env, "/api/stats").await;
    assert_eq!(stats["likes"].as_i64(), Some(2));
    env.shutdown().await;
}

#[tokio::test]
async fn a_like_whose_reply_was_lost_is_not_counted_twice() {
    let env = start().await;
    // The first attempt stored the like, then its reply was lost.
    let mut store = env.store("data");
    store
        .put(format!("lk/seed-02/{}", voter(7)).as_bytes(), b"0")
        .await
        .expect("seed the like");
    let (status, retry) = post(&env, "/api/palettes/seed-02/like", json!({ "voter": voter(7) })).await;
    assert_eq!(status, 200);
    assert_eq!((retry["likes"].as_i64(), retry["counted"].as_bool()), (Some(1), Some(false)));
    let (_, stats) = get_json(&env, "/api/stats").await;
    assert_eq!(stats["likes"].as_i64(), Some(1));
    env.shutdown().await;
}

#[tokio::test]
async fn design_votes_count_once_per_visitor_per_round() {
    let env = start().await;
    claim(&env).await;
    let (_, a) = post(&env, "/api/vote", json!({ "voter": voter(1) })).await;
    assert_eq!((a["votes"].as_i64(), a["counted"].as_bool()), (Some(1), Some(true)));
    let (_, again) = post(&env, "/api/vote", json!({ "voter": voter(1) })).await;
    assert_eq!((again["votes"].as_i64(), again["counted"].as_bool()), (Some(1), Some(false)));
    let (_, b) = post(&env, "/api/vote", json!({ "voter": voter(2) })).await;
    assert_eq!(b["votes"].as_i64(), Some(2));

    let (_, first_visit) = post(&env, "/api/visit", json!({ "voter": voter(1) })).await;
    let (_, second_visit) = post(&env, "/api/visit", json!({ "voter": voter(1) })).await;
    assert_eq!((first_visit["counted"].as_bool(), second_visit["counted"].as_bool()), (Some(true), Some(false)));

    let (_, stats) = get_json(&env, "/api/stats").await;
    assert_eq!((stats["votes"].as_i64(), stats["visitors"].as_i64(), stats["round"].as_i64()), (Some(2), Some(1), Some(0)));
    assert!(page(&env, "/").await.contains("<b id=\"votes\">2</b> votes"));

    assert_eq!(post(&env, "/api/admin/reset-scores", json!({ "admin_key": "wrong-key-123" })).await.0, 403);
    let (status, reset) = post(&env, "/api/admin/reset-scores", json!({ "admin_key": ADMIN_KEY })).await;
    assert_eq!(status, 200);
    assert_eq!((reset["round"].as_i64(), reset["stats"]["votes"].as_i64()), (Some(1), Some(0)));

    // After a reset the same visitor can vote again, in the new round.
    let (_, new_round) = post(&env, "/api/vote", json!({ "voter": voter(1) })).await;
    assert_eq!((new_round["votes"].as_i64(), new_round["counted"].as_bool()), (Some(1), Some(true)));
    let html = page(&env, "/").await;
    assert!(html.contains("data-round=\"1\""));
    assert!(html.contains("<b id=\"votes\">1</b> vote<"));
    env.shutdown().await;
}

#[tokio::test]
async fn the_admin_key_is_claimed_once_and_guards_changes() {
    let env = start().await;
    assert!(page(&env, "/studio").await.contains("id=\"t-claim\""));
    assert_eq!(post(&env, "/api/admin/claim", json!({ "admin_key": "short" })).await.0, 400);
    claim(&env).await;
    assert_eq!(post(&env, "/api/admin/claim", json!({ "admin_key": "another-key-456" })).await.0, 409);
    assert_eq!(post(&env, "/api/admin/check", json!({ "admin_key": ADMIN_KEY })).await.0, 200);
    assert_eq!(post(&env, "/api/admin/check", json!({ "admin_key": "another-key-456" })).await.0, 403);
    assert!(!page(&env, "/studio").await.contains("id=\"t-claim\""));

    let denied = post(&env, "/api/theme", json!({ "admin_key": "another-key-456", "theme": poster_theme() })).await;
    assert_eq!(denied.0, 403);
    let site = json!({ "title": "X", "tagline": "Y", "label": "Z" });
    assert_eq!(post(&env, "/api/site", json!({ "site": site })).await.0, 403);
    assert!(page(&env, "/").await.contains("layout-gallery"));
    env.shutdown().await;
}

#[tokio::test]
async fn publishing_a_theme_restyles_the_same_data() {
    let env = start().await;
    claim(&env).await;
    let (status, published) = post(&env, "/api/theme", json!({ "admin_key": ADMIN_KEY, "theme": poster_theme() })).await;
    assert_eq!(status, 200, "{published}");
    assert_eq!(published["theme"]["name"], "Poster");

    let html = page(&env, "/").await;
    assert!(html.contains("<body class=\"layout-poster\">"));
    assert!(html.contains("--bg:#0e0e0e"));
    assert!(html.contains("family=Archivo+Black"));
    assert!(html.contains(".pname{letter-spacing:.02em}"));
    assert!(html.contains("Bauhaus Primaries"), "same palettes, new design");
    assert!(html.contains("<b>12</b> palettes"));

    let (_, current) = get_json(&env, "/api/theme").await;
    assert_eq!(current["theme"]["layout"], "poster");
    for key in ["gallery", "poster", "soft"] {
        assert_eq!(current["catalog"]["presets"][key]["layout"], key);
    }
    assert!(current["catalog"]["fonts"]["inter"]["stack"].is_string());

    let mut unsafe_css = poster_theme();
    unsafe_css["custom_css"] = json!("</style><script>alert(1)</script>");
    assert_eq!(post(&env, "/api/theme", json!({ "admin_key": ADMIN_KEY, "theme": unsafe_css })).await.0, 400);
    let mut bad_colour = poster_theme();
    bad_colour["accent"] = json!("red");
    assert_eq!(post(&env, "/api/theme", json!({ "admin_key": ADMIN_KEY, "theme": bad_colour })).await.0, 400);
    let incomplete = json!({ "admin_key": ADMIN_KEY, "theme": { "name": "Half" } });
    assert_eq!(post(&env, "/api/theme", incomplete).await.0, 400);
    assert!(!page(&env, "/").await.contains("<script>alert(1)"));
    env.shutdown().await;
}

#[tokio::test]
async fn variant_settings_link_the_two_designs() {
    let env = start().await;
    claim(&env).await;
    assert!(page(&env, "/compare").await.contains("No second design is linked yet"));

    let site = json!({
        "title": "Swatchbook", "tagline": "Same palettes, new look.", "label": "Design B",
        "sibling_url": "https://abc123.ferrite.run/"
    });
    let (status, saved) = post(&env, "/api/site", json!({ "admin_key": ADMIN_KEY, "site": site })).await;
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["site"]["sibling_url"], "https://abc123.ferrite.run");

    let html = page(&env, "/").await;
    assert!(html.contains("Design B"));
    assert!(html.contains("Same palettes, new look."));
    assert!(html.contains("<a href=\"https://abc123.ferrite.run\" target=\"_top\">See the other design</a>"));
    let compare = page(&env, "/compare").await;
    assert!(compare.contains("src=\"/?embed=1\""));
    assert!(compare.contains("src=\"https://abc123.ferrite.run/?embed=1\""));

    let hostile = json!({ "title": "T", "tagline": "T", "label": "L", "sibling_url": "javascript:alert(1)" });
    assert_eq!(post(&env, "/api/site", json!({ "admin_key": ADMIN_KEY, "site": hostile })).await.0, 400);
    let (_, current) = get_json(&env, "/api/site").await;
    assert_eq!(current["site"]["label"], "Design B");
    env.shutdown().await;
}

#[tokio::test]
async fn user_text_is_escaped_everywhere() {
    let env = start().await;
    let body = json!({ "name": "<img src=x onerror=alert(1)>", "author": "\"quoted\"", "colors": ["#111", "#222", "#333"] });
    assert_eq!(post(&env, "/api/palettes", body).await.0, 201);
    let html = page(&env, "/").await;
    assert!(!html.contains("<img src=x"));
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    assert!(html.contains("&quot;quoted&quot;"));

    claim(&env).await;
    let site = json!({ "title": "Swatchbook", "tagline": "Hi", "label": "</script><b>B</b>" });
    assert_eq!(post(&env, "/api/site", json!({ "admin_key": ADMIN_KEY, "site": site })).await.0, 200);
    let studio = page(&env, "/studio").await;
    assert!(!studio.contains("</script><b>B</b>"));
    assert!(studio.contains("&lt;/script&gt;&lt;b&gt;B&lt;/b&gt;"));
    assert!(studio.contains("\\u003c/script\\u003e\\u003cb\\u003eB"));
    env.shutdown().await;
}

#[tokio::test]
async fn the_studio_boots_with_the_live_design_and_catalog() {
    let env = start().await;
    let html = page(&env, "/studio").await;
    assert!(html.contains("<body class=\"layout-gallery studio\">"));
    assert!(html.contains("Design studio"));
    assert!(html.contains("Scoreboard · round 0"));
    let start_tag = "<script type=\"application/json\" id=\"sb-boot\">";
    let from = html.find(start_tag).expect("boot data") + start_tag.len();
    let len = html[from..].find("</script>").expect("end of boot data");
    let boot: Value = serde_json::from_str(&html[from..from + len]).expect("boot json");
    assert_eq!(boot["theme"]["name"], "Gallery");
    assert_eq!(boot["admin_set"], false);
    assert_eq!(boot["catalog"]["presets"]["poster"]["layout"], "poster");
    assert_eq!(boot["site"]["label"], "Design A");
    env.shutdown().await;
}

#[tokio::test]
async fn the_admin_can_remove_a_palette() {
    let env = start().await;
    claim(&env).await;
    let path = "/api/palettes/seed-03/delete";
    assert_eq!(post(&env, path, json!({ "admin_key": "wrong-key-123" })).await.0, 403);
    post(&env, "/api/palettes/seed-03/like", json!({ "voter": voter(1) })).await;
    let (status, deleted) = post(&env, path, json!({ "admin_key": ADMIN_KEY })).await;
    assert_eq!(status, 200, "{deleted}");
    let names = palette_names(&env).await;
    assert_eq!(names.len(), 11);
    assert!(!names.iter().any(|n| n == "Swiss Rail"));
    let (_, stats) = get_json(&env, "/api/stats").await;
    assert_eq!((stats["palettes"].as_i64(), stats["likes"].as_i64()), (Some(11), Some(0)));
    assert_eq!(post(&env, path, json!({ "admin_key": ADMIN_KEY })).await.0, 404);
    assert_eq!(post(&env, "/api/palettes/seed-03/like", json!({ "voter": voter(2) })).await.0, 404);
    env.shutdown().await;
}

#[tokio::test]
async fn a_delete_interrupted_after_removing_the_palette_is_finished_by_the_retry() {
    let env = start().await;
    claim(&env).await;
    // What a crash after the first step leaves: the index, no palette row.
    let mut store = env.store("data");
    let primary = store.get(b"pi/seed-04").await.expect("read").expect("index exists");
    store.delete(&primary).await.expect("remove the palette row");

    let (status, _) = post(&env, "/api/palettes/seed-04/delete", json!({ "admin_key": ADMIN_KEY })).await;
    assert_eq!(status, 200);
    assert!(store.get(b"pi/seed-04").await.expect("read").is_none());
    assert_eq!(palette_names(&env).await.len(), 11);
    env.shutdown().await;
}

#[tokio::test]
async fn unknown_routes_and_methods_are_refused() {
    let env = start().await;
    assert_eq!(get_json(&env, "/nope").await.0, 404);
    assert_eq!(get_json(&env, "/api/vote").await.0, 405);
    let put = send(&env, ingress::request("PUT", "/api/theme", "k", "{}")).await;
    assert_eq!(put.status, 405);
    let malformed = send(&env, ingress::post("/api/vote", "not json")).await;
    assert_eq!(malformed.status, 400);
    env.shutdown().await;
}
