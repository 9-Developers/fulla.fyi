# AGENTS.md — Fulla.fyi

## Product

Fulla.fyi is a wishlist organizer and sharing site supporting links to any
retailer. Users have accounts, manage wishlists, assign item priorities, choose
desired quantities, and sort items. Users can create groups such as family or
friends and share lists with groups, specific users, or through a public link.

“Want count” means desired units. For example, a user wants four hard drives;
another user records buying two, leaving two remaining. Model desired quantities
and purchaser-attributed purchase records separately. Account for corrections,
multiple purchasers, and concurrent updates. Gift-surprise visibility and whether
anonymous visitors can record purchases are still product decisions.

Track work in the **Fulla.fyi** Linear project under **9 Developers (9DEV)**:
https://linear.app/ixirsii/project/fullafyi-8960c5ab9048

## Current implementation

- Rust 2024 with Rust 1.97.1 pinned in `rust-toolchain.toml`.
- Axum server, Askama templates, vendored HTMX, and embedded public assets.
- Tailwind CSS and Catppuccin themes already exist. CI and Railpack pin Tailwind
  4.3.3; `build.rs` requires a `tailwindcss` executable on `PATH`.
- The server currently serves the home page, error pages, static assets, and
  `/health`. The health endpoint checks the stateless draft only.
- SQLx/PostgreSQL, Argon2, axum-login, and tower-sessions dependencies are present,
  but database integration and authentication are not implemented.
- Priority, group, and user migrations are started. List, item, and membership
  migrations are placeholders. Product features described above remain planned.

Keep this document and README accurate as these capabilities are implemented.

## Commands

| Command                                              | Purpose                           |
| ---------------------------------------------------- | --------------------------------- |
| `cargo run --locked`                                 | Build and start the local server  |
| `cargo build --release --locked`                     | Build the production executable   |
| `cargo fmt --all`                                    | Format Rust                       |
| `cargo fmt --all -- --check`                         | Check formatting                  |
| `cargo clippy --all-targets --locked -- -D warnings` | Run strict lint checks            |
| `cargo test --locked --all-targets`                  | Run tests                         |
| `cargo build --locked`                               | Run the CI-equivalent debug build |

Install the pinned Tailwind CLI before building. Commit `Cargo.lock`.
For code changes, run formatting, Clippy, and relevant tests; CI also runs the
locked build. Documentation-only changes do not require recompiling the app.
Builds compile CSS and embed assets, so template and asset changes need a rebuild.

## Environment

The current server loads a local `.env` when present. Exported variables take
precedence. Preserve existing `.env` files.

| Variable   | Default | Purpose                  |
| ---------- | ------- | ------------------------ |
| `PORT`     | `3000`  | Listen port on `0.0.0.0` |
| `RUST_LOG` | `info`  | Tracing filter           |

Invalid supplied values fail startup. Railway IaC supplies `DATABASE_URL` as a
reference to Postgres, but the application does not consume it yet. Keep secrets
out of source control and public assets, which are embedded in the executable.

## Architecture and conventions

- `src/main.rs`: startup, environment parsing, logging, and HTTP listener.
- `src/route/`: routing and page handlers; `error.rs` renders safe error pages.
- `templates/`: Askama HTML pages; use server-rendered pages and HTMX fragments
  for product interactions.
- `src/style/`: editable Tailwind and theme sources.
- `src/public/`: current embedded browser assets, including generated `main.css`.
- `build.rs`: CSS compilation and asset hashing.
- `src/static_assets.rs`: explicit asset declarations and fingerprinted URLs.
- `migrations/`: SQLx PostgreSQL up/down migrations.
- `.forgejo/workflows/`: build, security, and release deployment.

Follow the Rust formatting and lint configuration in the repository. Unsafe code,
`unwrap`, and `expect` are denied by the existing lint configuration. Keep domain
logic and validation in Rust. Use Askama's default escaping for user content.
Preserve accessible labels, keyboard interaction, responsive layouts, and themes.

### Assets

Edit `src/style/` rather than generated `src/public/main.css`. The build script
currently writes compiled CSS there only when its contents change.
Add declarations to `STATIC_ASSETS` when adding embedded assets and use
`static_url` from templates. Tests require declarations to cover embedded files
exactly once. Fingerprinted URLs must match their bytes and reject stale hashes.
Keep immutable caching for fingerprinted assets and the separate `.well-known`
handling.

The planned asset update is **9DEV-69**. Reference `../limpdick.io/build.rs` and
`../limpdick.io/src/static_assets.rs` for its root `public/` layout, generated
image index, and stable assets with ETag revalidation. Preserve Fulla-specific
behavior when adapting it. Tailwind foundation work is **9DEV-73**.

### Database and product behavior

Before changing existing migrations, determine whether they have been applied.
Use forward migrations for deployed schemas. Add foreign keys, constraints,
indexes, and transactions appropriate to ownership, memberships, and quantities.

Enforce list ownership and sharing access on the server for both full-page and
HTMX requests. Revoked sharing grants and group membership changes must affect
access consistently. Keep purchase-record permissions separate from list editing.
Validate item URLs and quantities on the server. Store password hashes, use
persistent sessions, and protect state-changing forms when implementing accounts.

Test meaningful domain behavior: quantity changes, partial purchases, concurrent
purchase updates, sorting, ownership, sharing grants, and revocation. Integration
tests should exercise PostgreSQL migrations and authorization boundaries.

## CI and deployment

Forgejo build CI currently runs on `node24`, installs Rust and Tailwind, then
checks formatting, strict Clippy, tests, and a locked build. Preserve the
trusted/untrusted security workflow separation.

Railway is configured through `.railway/railway.go` and `railpack.json`.
The `fulla.fyi` partial manages the application and Postgres independently in
staging and production. Staging retains its existing 5 GB volume; production's
database helper provisions a fresh volume. Register domains `beta.fulla.fyi` and
`fulla.fyi` respectively in the dashboard at port 3000; the pinned CLI cannot
register new domains via IaC. Production is not provisioned yet.
Go module dependencies are pinned in `.railway/`.
CI uses Go 1.27.4 and Railway CLI 5.74.1. Preview with
`railway config plan --environment staging` and apply with
`railway config apply --environment staging`; configuration evaluation is
separate from source deployment.

Pushes to `main` deploy staging after build and trusted security checks.
Releases use validated `vMAJOR.MINOR.PATCH` tags whose commits are on `main`
and have a successful staging deployment recorded in Forgejo run history.
Production jobs require repository variable `PRODUCTION_DEPLOY_ENABLED=true`.
The shared `railway.yml` uses a saved IaC plan/apply, uploads source, and
waits for the specific deployment's health result. Concurrency is per environment.
Use separate `RAILWAY_STAGING_TOKEN` and `RAILWAY_PRODUCTION_TOKEN` secrets,
each scoped to its environment. The service name is `fulla.fyi`.
Build CI includes Go and Node deployment-gate tests. The release executable is
`fulla_fyi`, and Railway probes `/health`. Runtime database integration and a
pre-deploy migration command are not implemented. See README for local commands
and legacy Config File cutover guidance.
