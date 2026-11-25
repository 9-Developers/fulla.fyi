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

- Rust 2024; `rust-toolchain.toml` pins the compiler version.
- Axum server, Askama templates, vendored HTMX, and embedded public assets.
- Tailwind CSS and Catppuccin themes already exist. CI and release builds pin Tailwind
  4.3.5; `build.rs` requires a `tailwindcss` executable on `PATH`.
- The server currently serves the home page, `/licenses`, error pages, static assets, and
  `/health`. The health endpoint checks PostgreSQL connectivity.
- SQLx/PostgreSQL uses shared application state and a bounded connection pool.
  Startup validates embedded migration history; `fulla_fyi migrate` applies it.
  Registration/login/logout and PostgreSQL-backed sessions are implemented.
  `/account` is protected; wishlist and group authorization remains planned.
- The V0 schema models users, owned groups/lists, memberships, sharing grants,
  items, notes, attributed purchases, media assets, and owned avatar/list-media
  references. R2 uploads and Cloudflare Images transformations remain planned
  (9DEV-89/90), as do product handlers.

Keep this document and README accurate as these capabilities are implemented.

## Commands

| Command                                              | Purpose                           |
| ---------------------------------------------------- | --------------------------------- |
| `cargo run --locked`                                 | Build and start the local server  |
| `cargo run --locked -- migrate`                      | Apply database migrations        |
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
Database tests require `DATABASE_URL` on a disposable PostgreSQL 18 server with
database creation privileges. SQLx creates isolated databases; CI supplies Postgres.
The FreeBSD `ci-rust` job uses the `ci-postgres:18` service. After the build it
runs `target/debug/fulla_fyi migrate` against that database, then runs the
pinned `9dev-site-lib` OTLP smoke check with `--fulla target/debug/fulla_fyi`.
The receiver is local and needs no Grafana credentials.

## Environment

Fulla loads `./.env` through `nine_dev_site_lib::startup` before starting
Tokio. Exported variables take precedence. Only server mode reads `PORT`;
`migrate` needs no listener configuration. Preserve existing `.env` files.

| Variable   | Default | Purpose                  |
| ---------- | ------- | ------------------------ |
| `PORT`     | `3000`  | Listen port on `0.0.0.0` |
| `RUST_LOG` | `info`  | Tracing filter           |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | unset | Enables server-side OTLP logs and metrics |
| `OTEL_EXPORTER_OTLP_HEADERS` | unset | Grafana authorization header, stored as a service secret |
| `DEPLOYMENT_ENVIRONMENT` | `development` | Telemetry environment; set by Railway IaC |
| `DATABASE_URL` | required | PostgreSQL URL      |
| `SESSION_SECURE` | `true` | Secure auth cookies; set `false` only for local HTTP |

Invalid supplied values fail startup. Railway IaC supplies `DATABASE_URL` as a
reference to Postgres. Keep secrets
out of source control and public assets, which are embedded in the executable.

## Architecture and conventions

- `src/main.rs`: startup, environment parsing, logging, and HTTP listener.
- `src/database.rs`: pool, application state, migration checks, and database tests.
- `src/auth/`: axum-login backend, PostgreSQL session store, password policy,
  shared database rate limits, and authentication integration tests.
- `src/route/`: routing and page handlers; `error.rs` renders safe error pages.
- `templates/`: Askama HTML pages; use server-rendered pages and HTMX fragments
  for product interactions.
- `src/style/`: editable Tailwind styles and mappings to the shared palette.
- `public/`: embedded browser assets, including generated `main.css`.
- `build.rs`: CSS compilation and declarations of fingerprinted files, directories, and stable URLs.
- `src/static_assets.rs`: the generated site catalog, embedded bytes, and URL lookup for templates.
- `migrations/`: SQLx PostgreSQL up/down migrations.
- `.forgejo/workflows/`: build, security, and release deployment.

Follow the Rust formatting and lint configuration in the repository. Unsafe code,
`unwrap`, and `expect` are denied by the existing lint configuration. Keep domain
logic and validation in Rust. Use Askama's default escaping for user content.
Preserve accessible labels, keyboard interaction, responsive layouts, and themes.

### Assets

`/licenses` embeds `THIRD_PARTY_NOTICES.md`, the shared asset notices from
`nine_dev_site_lib`, and `licenses/rust.txt`.
Update the frontend inventory and preserve upstream notices when changing assets.
After dependency changes, follow the [notice update procedure](licenses/AUDIT.md#updating-notices).
CI uses cargo-deny 0.21.0 to enforce
`deny.toml` and checks generated notice freshness. Keep `about.toml` and
`deny.toml` license choices aligned. The four flavor icons use MIT-licensed Fluent
Emoji High Contrast SVGs with `currentColor`. The home page uses the shared
`IconLink` component with the library's GitHub and Discord symbols.

Rust license clarifications in `about.toml` preserve upstream copyright notices.
Review source license changes before updating checksums. CI runs
`sh .forgejo/scripts/check-license-notices.sh` to reject canonical MIT placeholders;
generation success and freshness alone do not verify attribution completeness.

Edit `src/style/` rather than generated `public/main.css`. The build script uses
`nine_dev_site_build` to compile CSS, write it only if changed, and fingerprint
the final files. Place site images under `public/images/`; they are indexed at
build time without individual declarations. Register other immutable files with
`File` and stable browser paths with `Revalidate` in `build.rs`. Templates call
`static_url` with a path relative to `public/`. A stable entry such as
`site.webmanifest` also registers its root URL with ETag revalidation.
In the base template, load the shared `nine_dev_site_lib::switcher` script
before CSS and the `assets::theme::PALETTE` stylesheet before `main.css`.
Fulla defaults to Latte without JavaScript. `build.rs` generates the ignored
`src/style/catppuccin.aliases.css` from the build crate before invoking Tailwind.
`src/style/main.css` explicitly scans `templates/` for utility classes; keep
class names literal in Askama. The static handler serves the library assets, including
HTMX (`assets::htmx::JS` from the runtime crate's `htmx` feature).
Tests require declarations to cover local files exactly once. Fingerprinted
URLs must match their bytes and reject stale hashes.
Keep immutable caching for fingerprinted assets. Font faces are generated from
the bundled WOFF2 files before compiling CSS, so changing font bytes updates
both the font URL and the stylesheet URL.

Register each new discovery document with `MaxAge` in `build.rs`, using its
exact `/.well-known/` URL and a one-hour lifetime. To inspect the reusable wishlist card and item layout, run
`cargo run --locked --example wishlist_preview > target/wishlist-preview.html`
and open that file in a browser. The example renders populated and empty cards
without connecting to PostgreSQL.

### Database and product behavior

Before changing existing migrations, determine whether they have been applied.
Use forward migrations for deployed schemas. Add foreign keys, constraints,
indexes, and transactions appropriate to ownership, memberships, and quantities.

Enforce list ownership and sharing access on the server for both full-page and
HTMX requests. Revoked sharing grants and group membership changes must affect
access consistently. Keep purchase-record permissions separate from list editing.
Validate item URLs and quantities on the server. Store password hashes, use
persistent sessions, and protect state-changing forms when implementing accounts.

Account forms use synchronizer CSRF tokens and Argon2id password hashes. Reuse
`auth::password_policy` when implementing password changes/resets (12–128 UTF-8
bytes, zxcvbn score >= 3); never apply strength policy at login. Sessions rotate on
login and are deleted on logout. Session-store updates must not recreate deleted
sessions. Keep auth responses uncached and cookies Secure in deployed environments.
Use axum-login's tower-sessions re-export and `auth.session` in handlers rather
than a direct dependency or separate Session extractor. The custom store uses the
application's SQLx 0.9 pool and forward migrations.

Test meaningful domain behavior: quantity changes, partial purchases, concurrent
purchase updates, sorting, ownership, sharing grants, and revocation. Integration
tests should exercise PostgreSQL migrations and authorization boundaries.

Purchase submission keys are purchaser-scoped. Sum uncanceled purchases;
preserve `recorded_at` on corrections and retain history when items are archived
or desired quantities decrease. Serialize changes and submissions on the item
row once the overpurchase policy is settled. The schema does not cap totals.

Only attach ready media assets owned by the account. Lock the asset row for
attachment, replacement, and cleanup. Check both avatar and list-media usages
before marking an asset `deleting`; retain the row until object deletion
succeeds. Public avatar delivery must not expose private list media when an
asset is reused.

Auth rate limits use shared PostgreSQL counters in 15-minute windows (10 logins
per identifier, 100 logins globally, 30 registrations globally). Successful
attempts count. Review the shared limits before public launch. Signed-in
sessions expire after seven days; anonymous form sessions expire after 30 minutes.

## CI and deployment

Forgejo build CI uses `ci-rust` for Rust checks and `ci-go` for Railway
configuration and Node deployment-gate tests. Rust and Go are supplied by the
runners; the Rust job installs Tailwind 4.3.5 before checking formatting, strict
Clippy, tests, and a locked build.

`security.yml` uses `ci-security` runners with preinstalled Gitleaks, Semgrep,
and Trivy. It runs on pushes to `main` and through `workflow_call`; pull requests
currently run build checks only. There are no separate trusted/untrusted security
workflows or Socket scan. Keep repository secrets out of untrusted PR jobs.

Forgejo reusable workflows accept `inputs` and `outputs` under `workflow_call`,
not `secrets`. Pass secrets through the caller's `jobs.<job_id>.secrets` mapping.

Railway is configured through `.railway/railway.go`.
The `fulla.fyi` partial manages the application and Postgres independently in
staging and production. Staging retains its existing 5 GB volume; production's
database helper provisions a fresh volume. Register domains `beta.fulla.fyi` and
`fulla.fyi` respectively in the dashboard at port 3000; the pinned CLI cannot
register new domains via IaC. Production is not provisioned yet.
Go module dependencies are pinned in `.railway/`.
The shared deployment workflow requires a trusted Bookworm Linux/amd64
`ci-railway` runner with Go, Railway CLI 5.74.1, `crane`, Bash, Node,
the repository's Rust toolchain, native build tools and `libssl-dev`, and
Tailwind 4.3.5. Set `RAILWAY_PROJECT_ID` and package credentials
`REGISTRY_USER` / `REGISTRY_TOKEN` for container and generic packages.
Give Railway separate pull credentials for private images and disable automatic
image updates. Preview with
`railway config plan --environment staging` and apply with
`railway config apply --environment staging`; configuration evaluation is
separate from image promotion.

Pushes to `main` deploy staging after build and security checks.
Releases use validated `vMAJOR.MINOR.PATCH` tags whose commits are on `main`
and have a successful staging deployment recorded in Forgejo run history.
The shared `railway.yml` checks the digest, uses a saved IaC plan/apply,
sets the service instance's image, and waits for that deployment's health result.
Staging records the healthy commit/digest in the `<repository>-staging`
generic package keyed by run ID. Production requires that exact receipt;
retain receipts and images and restrict package writes. Concurrency is per environment.
Use separate `RAILWAY_STAGING_TOKEN` and `RAILWAY_PRODUCTION_TOKEN` secrets,
each scoped to its environment. The service name is `fulla.fyi`.
Build CI includes Go and Node deployment-gate tests. The release executable is
`/app/fulla_fyi`, and Railway probes `/health`. From `/app`, Railway runs `./fulla_fyi migrate`
before deployment; startup validates the schema. See the [local setup](README.md#run-locally).

### Railway operator commands

To preview or apply staging infrastructure:

```sh
railway login
railway link --project 24db0769-cf07-4290-bae1-dd1bceaea653 --environment staging
railway status
railway config plan --environment staging
railway config apply --environment staging
```

If an environment still has a legacy Config File path, run
`railway config migrate cutover` there first. `railway config migrate undo`
restores paths from `.railway/.cac-migration.json`, not settings changed by an
IaC apply.

To enable production, create an empty `production` environment. Link, plan,
and apply as above using `production`, then configure the domain, DNS, and
production-scoped secrets. Push a release tag after the commit's staging
deployment succeeds; retain its Forgejo run history for the release gate.
