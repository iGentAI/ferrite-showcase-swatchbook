#![forbid(unsafe_code)]
//! # Swatchbook
//!
//! A community colour-palette gallery built to demonstrate one Ferrite idea:
//! **same data, two designs**.
//!
//! The look of the site is not code. It is a theme record stored next to the
//! palettes in the app's `data` store. Forking the running app on Ferrite
//! copies the palettes *and* the theme, so the fork starts as an exact twin.
//! Restyling the fork in its Studio (`/studio`) then gives two live URLs that
//! serve the same real content in two different designs, each counting its
//! own design votes.
//!
//! Module map:
//! - [`model`]: palettes, per-variant site settings, validation, seed library.
//! - [`theme`]: the design-as-data record, presets, fonts and CSS output.
//! - [`repo`]: the key layout of the `data` store and every read and write.
//! - [`handlers`]: the HTTP router and endpoint logic.
//! - `html` and `assets`: server-rendered pages and their CSS/JS.

mod assets;
pub mod handlers;
mod html;
pub mod model;
pub mod repo;
pub mod theme;

use std::future::Future;
use std::io;

use ferrite_app::client::{QueueClusterClient, StoreClusterClient};
use ferrite_app::{rt, serde_json, sleep_ms, AppContext, FerriteApp, IngressRequest};

use crate::repo::Repo;

/// Queue binding the gateway publishes HTTP requests onto (spec: `requests`).
pub const REQUESTS: &str = "requests";
/// Store binding holding palettes, counters and the theme (spec: `data`).
pub const DATA: &str = "data";
/// Optional per-app secret whose value becomes the admin key at startup.
pub const ADMIN_SECRET: &str = "ADMIN_KEY";
/// Concurrent request workers per instance. Each owns its own queue consumer
/// and store connection, so one slow request never holds up the others.
pub const WORKERS: usize = 4;
/// Long-poll window for the request queue (the platform caps it at 30 s).
const POLL_WAIT_MS: u64 = 25_000;
/// Startup seeding is retried this many times (500 ms apart) before serving.
const BOOTSTRAP_ATTEMPTS: u32 = 20;

/// The Ferrite application type named by `main`.
pub struct App;

impl FerriteApp for App {
    fn run(ctx: AppContext) -> impl Future<Output = io::Result<()>> + Send {
        run(ctx)
    }
}

/// Run the app: bootstrap the store, then serve HTTP requests from the
/// `requests` queue on [`WORKERS`] concurrent workers until the process ends.
/// Tests call this directly through the `ferrite-app-test` harness.
pub async fn run(ctx: AppContext) -> io::Result<()> {
    let mut repo = Repo::new(ctx.store(DATA)?);
    bootstrap(&ctx, &mut repo).await;

    let mut workers = Vec::with_capacity(WORKERS);
    for _ in 0..WORKERS {
        // Handles are created up front so a wiring mistake fails at startup.
        let requests = ctx.queue(REQUESTS)?;
        let store = ctx.store(DATA)?;
        let ctx = ctx.clone();
        workers.push(rt::spawn(serve(ctx, requests, store)));
    }
    for worker in workers {
        match worker.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(io::Error::other(format!("worker task failed: {e}"))),
        }
    }
    Ok(())
}

/// Sync the admin secret into the store and seed the palette library on a
/// fresh deployment. Transient store errors are retried; if they persist the
/// app still serves (the seed is attempted again on the next start).
async fn bootstrap(ctx: &AppContext, repo: &mut Repo) {
    let admin_secret = ctx.secret(ADMIN_SECRET);
    for attempt in 1..=BOOTSTRAP_ATTEMPTS {
        match repo.bootstrap(admin_secret.as_deref()).await {
            Ok(()) => return,
            Err(e) => {
                eprintln!("swatchbook: bootstrap attempt {attempt} failed: {e}");
                sleep_ms(500).await;
            }
        }
    }
    eprintln!("swatchbook: serving without a completed bootstrap");
}

/// One request worker: lease a request, route it, publish the reply to the
/// gateway's reply queue, then ack. A reply that cannot be published is left
/// unacked so the queue redelivers the request once its lease lapses.
async fn serve(
    ctx: AppContext,
    mut requests: QueueClusterClient,
    store: StoreClusterClient,
) -> io::Result<()> {
    let mut repo = Repo::new(store);
    loop {
        let lease = match requests.poll_wait(POLL_WAIT_MS).await {
            Ok(Some(lease)) => lease,
            Ok(None) => continue,
            Err(e) => {
                eprintln!("swatchbook: poll failed: {e}");
                sleep_ms(250).await;
                continue;
            }
        };
        let req: IngressRequest = match serde_json::from_slice(&lease.payload) {
            Ok(req) => req,
            Err(e) => {
                eprintln!("swatchbook: dropping a malformed request: {e}");
                if let Err(e) = requests.ack(lease.token).await {
                    eprintln!("swatchbook: ack failed: {e}");
                }
                continue;
            }
        };

        let reply = handlers::handle(&ctx, &mut repo, &req)
            .await
            .into_reply(&req.correlation_id);
        let payload = match serde_json::to_vec(&reply) {
            Ok(payload) => payload,
            Err(e) => {
                eprintln!("swatchbook: could not encode a reply: {e}");
                continue;
            }
        };
        match ctx.reply_to_queue(&req.reply_to).await {
            Ok(mut replies) => {
                if let Err(e) = replies.publish(&payload).await {
                    eprintln!("swatchbook: reply publish failed: {e}");
                    continue;
                }
            }
            Err(e) => {
                eprintln!("swatchbook: reply queue unavailable: {e}");
                continue;
            }
        }
        if let Err(e) = requests.ack(lease.token).await {
            eprintln!("swatchbook: ack failed: {e}");
        }
    }
}
