# Swatchbook: same data, two designs

A community colour-palette gallery built to show designers one thing Ferrite
Cloud does that other hosting does not: **fork a live product, content and
all, in seconds, and restyle the copy.**

The look of the site is stored as data (a theme record) next to the palettes.
When the running app is forked on Ferrite, the fork gets the code, the
palettes, the likes *and* the design, copied at one instant. Restyle the fork
in its built-in Studio and you have two live URLs showing the same real content
in two different designs, each counting its own design votes.

## What visitors can do

- Browse the palettes. Each colour shows its hex code on hover; in the Poster
  design the codes are always printed.
- Like palettes (once per browser) and add their own (3 to 6 colours).
- Press **I prefer this design** (once per browser per scoreboard round).
- Open **Compare side by side** to see both designs at once.

Anyone can browse, like, vote and add palettes. Publishing a design, changing
a variant's settings, removing a palette and resetting the scoreboard need the
admin key. Both variants share one key, because the fork copies its hash.

## Presenter script (about 5 minutes)

1. **Show the real product.** Open Design A. Add a palette from the audience,
   like a few. This is a working app with real data, not a mock-up.
2. **Fork it.** Ask your coding agent ("fork swatchbook into
   swatchbook-c"), run `ferrite fork swatchbook-c`, or call the `fork` tool.
   About ten seconds later there is a new URL: an exact twin with every
   palette and like, including the one just added.
3. **Restyle the copy live.** Open the fork's `/studio`. Pick a preset
   (Gallery, Poster, Soft) or change colours, fonts, layout, corner radius,
   type scale or custom CSS. Every change previews instantly on the real
   palettes. Press **Publish design**.
4. **Let the room decide.** In the Studio, set *Other design's URL* on each
   side, then open `/compare`. People vote on their phones; each design
   counts its own visitors, votes, likes and additions (shown in the Studio
   scoreboard). **Start a new scoreboard round** resets the counts for a fair
   start.
5. **The point.** No lorem ipsum, no staging copy to build, no data export.
   The design is data, so a fork carries it, and the fork is a real, separate
   app that can be changed freely without touching the original. Ferrite can
   also fork an app *as it was* at any moment in the last 14 days.

## How it works

- **Ferrite app**: a Rust crate (`src/`) built and run by Ferrite. HTTP
  requests arrive on the `requests` queue; all state lives in the `data`
  store (see `ferrite.json`).
- **Design as data** (`src/theme.rs`): a validated theme record holding the
  layout (`gallery`, `poster`, `soft`), five colours, display and body fonts
  (Google Fonts or system stacks), corner radius, type scale and optional
  custom CSS. Pages are server-rendered with the theme as CSS custom
  properties and the layout as a body class, so every design uses the same
  markup.
- **Studio** (`/studio`): the live gallery with an editor panel. Controls
  apply instantly in the browser; Publish stores the theme. Custom CSS may
  not contain `<`, so it cannot break out of its `<style>` element.
- **Counting that survives retries** (`src/repo.rs`): every visitor action
  (like, vote, visit, submission) is one atomic create-if-absent write, and
  every count is derived by counting those records. A retried request, a
  crash between steps or two workers racing can never count an action twice.
- **Admin key**: only its SHA-256 hash is stored, in the data store, so a fork
  inherits it. Set it by claiming a fresh app in the Studio, or with an
  `ADMIN_KEY` secret (applied at startup).

### HTTP API

| Method | Path | Notes |
|---|---|---|
| GET | `/`, `/?embed=1` | gallery (embed: trimmed for the compare frames) |
| GET | `/studio`, `/compare`, `/healthz` | |
| GET | `/api/palettes` | palettes with like counts |
| POST | `/api/palettes` | `{name, author, colors}`; honours the `idempotency-key` header |
| POST | `/api/palettes/<id>/like` | `{voter}` |
| POST | `/api/palettes/<id>/delete` | admin |
| POST | `/api/vote`, `/api/visit` | `{voter}` |
| GET | `/api/stats` | this round's scoreboard |
| GET / POST | `/api/theme` | the design (POST: admin, `{admin_key, theme}`) |
| GET / POST | `/api/site` | label, title, headline, other design's URL (POST: admin) |
| POST | `/api/admin/claim`, `/api/admin/check`, `/api/admin/reset-scores` | |

## Develop and test

The app is a Rust crate with one runtime dependency, `ferrite-app`, taken from
the public SDK ([iGentAI/ferrite-sdk](https://github.com/iGentAI/ferrite-sdk)).
Clone the SDK beside this repository and install the `ferrite` CLI:

```sh
git clone https://github.com/iGentAI/ferrite-sdk
git clone https://github.com/iGentAI/ferrite-showcase-swatchbook
curl -fsSL https://downloads.ferrite.cloud/install.sh | sh
cd ferrite-showcase-swatchbook
ferrite check     # code gate, ferrite.json statics and cargo check
ferrite test      # the unit and end-to-end suites
ferrite dev       # serve the app at http://localhost:8787
```

The tests drive the real app through Ferrite's test harness
(`ferrite-app-test`), the same way the gateway does: 18 unit tests and 16
end-to-end tests, including concurrent and interrupted submissions, lost
replies and interrupted deletes. `ferrite test --remote` compiles the same
source and runs the same suite on the platform's engines.

## Deploy your own pair

```sh
ferrite login --ephemeral --endpoint https://mcp.ferrite.cloud/mcp   # a trial account; skip it if `ferrite account` already shows yours
ferrite deploy                    # builds and starts the app; `ferrite host` prints its address
python3 scripts/link_variants.py claim https://<host-a> --key-file /path/to/admin_key
ferrite fork swatchbook-b         # the copy, with every palette and like, at its own address
python3 scripts/link_variants.py link --a https://<host-a> --b https://<host-b> \
    --key-file /path/to/admin_key --b-preset poster
```

Set the admin key on Design A before forking, because the fork copies its
hash. The key is read from a file you choose and is never printed.

`ferrite cost --instances 1` prints the deployment fee and the projected
monthly running cost of one variant (64 MB, 10% CPU, one queue, one store)
before you deploy.

`scripts/ferrite_mcp.py` is a small MCP client (Python standard library only)
for the same steps without the CLI: it calls `deploy_source`, `build_status`
and `fork` on `https://mcp.ferrite.cloud/mcp` with a token you hold. Run it
with `--help` for its commands.

## Support, security and licence

Questions and support: info@igent.ai. Report a security issue privately to
security@ferrite.cloud.

Swatchbook is released under the MIT licence; see `LICENSE`. The hosted Ferrite
platform is proprietary and not open source.
