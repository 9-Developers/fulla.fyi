# Fulla

[![Discord][badge-discord]][link-discord]
[![License][badge-bsd-3]][link-bsd-3]
[![Build][badge-workflow-build]][link-workflow-build]
[![Security][badge-workflow-security]][link-workflow-security]
[![Release][badge-releases]][link-releases]

Share wishlists with your friends.

The current Rust draft serves the home page, `/licenses`, and bundled assets. A local `.env` is
loaded when present; `PORT` defaults to `3000` and `RUST_LOG` to `info`.
Invalid supplied values fail startup. `DATABASE_URL` is required. Startup checks
embedded migration history; `/health` checks PostgreSQL connectivity.

## Dependency licenses

The footer links to `/licenses`, which embeds `THIRD_PARTY_NOTICES.md` and
`licenses/rust.txt` in the executable. Include these files and `LICENSE` when
redistributing the application. The frontend inventory includes externally
loaded Sora fonts and the four MIT-licensed Fluent Emoji theme icons.
Provenance for the Discord, expand, and system SVGs is still being confirmed.

Install the pinned tools and regenerate Rust notices after dependency updates:

```sh
cargo install cargo-about --version 0.9.3 --locked --features cli
cargo install cargo-deny --version 0.21.0 --locked
cargo deny --locked check licenses
cargo about generate --locked --fail licenses/rust.hbs -o licenses/rust.txt
sh .forgejo/scripts/check-license-notices.sh
```

Commit the generated file. CI checks the license policy, rejects canonical MIT
copyright placeholders, and compares regenerated notices. `about.toml` includes
checksum-verified license clarifications where automatic discovery loses upstream
attribution. Git clarifications use each published crate's recorded commit.
Review changed upstream texts before updating their checksums; never fix the
generated report by hand. This targeted check does not replace attribution review.
`.gitattributes` preserves the report's upstream line endings for exact CI comparison.
`about.toml` selects permissive alternatives and includes build,
development, and target-specific dependencies. Keep its accepted licenses
aligned with `deny.toml`; review new licenses before extending either list.
Normal application builds use the committed notices and need neither tool.

For frontend additions or upgrades, record the exact upstream source/version,
license, copyright notice, and any required NOTICE text in
`THIRD_PARTY_NOTICES.md`. Preserve license comments during minification.
Review platform libraries (such as system OpenSSL), container/base-image packages,
and deployment tooling separately when distributing them; Cargo's inventory
does not cover those components.

## Railway deployment

Railway infrastructure is defined in `.railway/railway.go`, with Go dependencies
pinned in `.railway/go.mod` and `.railway/go.sum`. The `fulla.fyi` named partial
manages the `fulla.fyi` application and `Postgres` database separately in each
environment. **Staging** currently exists; production is not provisioned yet.
Staging retains its existing 5 GB `postgres-volume` in `us-east4-eqdc4a`.
Production's Postgres 18 database helper provisions a fresh persistent volume.
Register `beta.fulla.fyi` in staging and `fulla.fyi` in production in the Railway
dashboard, both targeting port 3000. The pinned CLI cannot register new custom
domains through IaC, so domains are dashboard-managed. Other environment names
are rejected by the Go configuration.

The application uses Railpack with Tailwind 4.3.5 from `railpack.json`, builds with
`cargo build --release --locked`, and starts `./fulla_fyi`. Railway probes
`/health` with a 300-second timeout and retries failed processes up to 10 times.
Source, template, migration, build configuration, and `.railway/**` changes are
included in the watch patterns.

### Plan and apply

Use Railway CLI **5.74.1** and Go **1.27.4**, matching CI. Authenticate and link
this checkout to the `fulla.fyi` project and `staging` environment:

```sh
railway login
railway link --project 24db0769-cf07-4290-bae1-dd1bceaea653 --environment staging
railway status
railway config plan --environment staging
railway config apply --environment staging
```

IaC evaluation is separate from application deployment. After applying the
configuration, upload the application with:

```sh
railway up --ci --environment staging --service fulla.fyi
```

The initial staging plan was verified to create only the application service,
preserving the database and volume. There was no application service with a
legacy Config File path to clear, so `railway.toml` has been removed. If migrating
another environment that still has a Config File path, run
`railway config migrate cutover` in that linked environment before deploying
this revision. `railway config migrate undo` restores the paths from the local,
git-ignored `.railway/.cac-migration.json` snapshot; it does not restore settings
changed by an IaC apply.

### Staging and production workflows

| Trigger                  | Target                   | Workflow                            |
| ------------------------ | ------------------------ | ----------------------------------- |
| Pull request             | Checks only              | Build checks |
| Push to `main`           | Staging                  | `deploy.yml`                        |
| Tag `vMAJOR.MINOR.PATCH` | Production, once enabled | `release.yml`                       |

Both deployment workflows run build and trusted security checks before calling
`railway.yml`. The shared workflow plans and applies IaC, uploads the
checked-out commit, then waits for that deployment's health check to succeed.
A successful build alone is not a successful deployment. Deployments are
serialized per environment; staging and production can run independently.

Production validates the release tag, checks its commit is reachable from
`main`, and queries Forgejo for a successful staging workflow run of the exact
commit. Wait for staging to finish before tagging. If the gate fails, rerun the
release after staging succeeds. Historical staging success counts even after
newer commits have deployed; preserve the relevant Forgejo workflow history.
Each environment rebuilds the same commit rather than promoting a container
image.

The IaC apply uses a saved plan from the same checkout and fails if the
environment or configuration tree changed. Destructive changes require a
separate explicit apply; deployment jobs do not authorize them.

Configure these Forgejo repository secrets:

- `RAILWAY_STAGING_TOKEN`: Railway project token scoped to **staging**.
- `RAILWAY_PRODUCTION_TOKEN`: separate project token scoped to **production**,
  added when production is created.

The former `RAILWAY_TOKEN` repository secret can be renamed to
`RAILWAY_STAGING_TOKEN`. The service is named `fulla.fyi` in each environment.
The production history check uses the built-in Forgejo job token, not a Railway
token. Security checks use preinstalled Gitleaks, Semgrep, and Trivy on
`ci-security`; there is no Socket scan. Go and the checksum-verified Railway CLI
are pinned in the shared deployment workflow on `node-24`. Build checks use
`ci-rust` and `ci-go` runners with their preinstalled toolchains.
CI checks Go formatting, module integrity, vet, environment isolation, and
deployment-gate behavior without Railway credentials.

### Enable production later

1. Create an empty `production` environment in the `fulla.fyi` Railway project.
   Use a fresh database; do not copy staging data or secrets.
2. Link to production and review/apply its infrastructure:
   ```sh
   railway link --project 24db0769-cf07-4290-bae1-dd1bceaea653 --environment production
   railway config plan --environment production
   railway config apply --environment production
   ```
3. Register `fulla.fyi` on the production application in Railway's dashboard,
   targeting port 3000, and configure the supplied DNS records and required
   runtime secrets. Register staging's `beta.fulla.fyi` the same way after its
   first IaC apply.
4. Add `RAILWAY_PRODUCTION_TOKEN` and set the Forgejo repository **variable**
   `PRODUCTION_DEPLOY_ENABLED` to `true`. Until then, production jobs are skipped.
5. Tag a commit with a successful staging deployment as `vX.Y.Z` and push the tag.

The same partial name is scoped independently to each environment. Future
database integration must add schema migrations before the first stateful
production deployment. Set `PRODUCTION_DEPLOY_ENABLED` back to `false` to pause
future releases without affecting staging.

### Runtime configuration and database integration

IaC sets `PORT=3000`, `RUST_LOG=info`, and `DATABASE_URL` as a reference to
`Postgres.DATABASE_URL`; no database credentials are stored in Git. Database
credentials and mounts are provisioned by Railway's database helper.
Application variables are managed by IaC: declare new variables in the file,
using `railway.Preserve()` for existing secret values that should stay in Railway.

The server uses at most 10 connections with a five-second acquisition timeout.
`/health` returns 200 when PostgreSQL responds within two seconds, or 503 otherwise.
Startup rejects missing, failed, unknown, or checksum-mismatched migrations.
Connection errors do not include credentials.

### Local database workflow

Install the pinned Rust toolchain and Tailwind CLI. Start PostgreSQL 18 and set
`DATABASE_URL` in your environment or local `.env`. For a disposable database:

```sh
docker run --name fulla-postgres -e POSTGRES_PASSWORD=fulla_local \
  -p 127.0.0.1:5432:5432 -d postgres:18
ready=false
for attempt in $(seq 1 60); do
  if docker exec fulla-postgres pg_isready -h 127.0.0.1 -U postgres; then
    ready=true
    break
  fi
  sleep 1
done
if [ "$ready" != true ]; then
  echo "PostgreSQL did not become ready within 60 attempts" >&2
  exit 1
fi
export DATABASE_URL=postgres://postgres:fulla_local@localhost:5432/postgres
cargo run --locked -- migrate
cargo run --locked
```

`fulla_fyi migrate` applies embedded SQLx migrations and exits. Railway runs
`./fulla_fyi migrate` before deployment; normal startup only validates history.
Migration changes rebuild the executable. The baseline replaces unapplied draft
migrations. Once deployed, add forward migrations instead of editing existing
ones. Down migrations are destructive and intended for disposable databases.

Run `cargo test --locked --all-targets` with `DATABASE_URL` pointing to a disposable
PostgreSQL server whose role can create databases. SQLx creates isolated test
databases. CI supplies PostgreSQL 18. Tests cover migration rollback/reapplication,
checksums, constraints, attribution, idempotency, and readiness.

### Schema and remaining product work

The V0 schema includes users with password hashes, owned groups/wishlists,
memberships, direct/group read grants, event dates, ordered/prioritized items,
owner notes, and purchaser-attributed quantities. Group names are unique per owner
(case-insensitive); usernames and emails are also case-insensitively unique.
Group owners are implicit members. Lists without grants are private.

Media metadata is stored in `media_assets`: private R2 bucket/object references,
processing status, format, byte size, dimensions, and lifecycle timestamps.
Ready assets require complete image metadata. `user_avatars` and `list_media`
reference uploads owned by the same account and keep focal points on each usage.
An absent avatar row means local initials. List media slots remain extensible
until registry customization is finalized; handlers must validate allowed slots.
Referenced media cannot be deleted. Removing a usage retains the source row for
retryable object cleanup; ownership transfers must explicitly handle media usages.

R2 uploads and Cloudflare Images transformations are planned in 9DEV-89/90.
Delivery URLs are generated, not stored. Handlers must only attach ready assets,
update `updated_at`, and serialize attachment/replacement and cleanup using an
asset-row lock. Cleanup must check both usage tables before deleting an object,
mark the asset `deleting`, and retain its row until object deletion succeeds.
Source objects stay private; avatar delivery is public by URL and list media
delivery follows list access. A source reused across usages must not expose private
media through a public avatar or another public usage; publishing, revocation,
and cache invalidation require application-level enforcement.

Purchases have purchaser-scoped submission keys to prevent duplicate inserts.
Canceled records and archived items retain history; reducing desired quantities
does not erase purchases. Live totals sum uncanceled records. Item/list deletion
must explicitly handle retained history rather than cascading purchases away.
Future purchase handlers must use transactions and a consistent item-row lock
for quantity changes and submissions once the overpurchase policy is settled.
The schema does not cap aggregate purchases at desired units.

Authentication, persistent sessions, authorization, and product handlers remain
future work. Handlers must fully validate HTTP(S) URLs; the database check is only
a baseline constraint. Notes support multiple entries structurally; UI/visibility
remain undecided. Handlers must preserve purchase `recorded_at` on corrections and
update `updated_at`. Event cutoff/timezone, rescheduling, and carryover rules
remain open in 9DEV-72/75. Public tokens and advanced copy/merge are V1 work.

## Links

- [Production][production]
- [Beta][beta]

## Tech stack

| Requirement     | Dependency                       |
| --------------- | -------------------------------- |
| Database        | [SQLx][sqlx]                     |
| HTTP handler    | [Axum][axum]                     |
| HTML templates  | [Askama][askama]                 |
| Markdown parser | [pulldown_cmark][pulldown-cmark] |
| Javascript      | [HTMX][htmx]                     |

[badge-discord]: https://img.shields.io/discord/452233740408717313?color=b4befe&label=discord&labelColor=1e1e2e&logo=discord&style=flat
[link-discord]: https://discord.gg/KTgshSbEUU
[badge-bsd-3]: https://img.shields.io/badge/License-BSD_3--Clause-89b4fa?labelColor=1e1e2e&logo=bsd&logoColor=white&style=flat
[link-bsd-3]: https://opensource.org/license/bsd-3-clause
[badge-workflow-build]: https://git.9dev.tech/9-Developers/fulla.fyi/badges/workflows/build.yml/badge.svg?branch=main&event=push&label=Build&labelColor=1e1e2e&style=flat&logo=forgejo&logoColor=white
[link-workflow-build]: https://git.9dev.tech/9-Developers/fulla.fyi/actions/workflows/build.yml
[badge-workflow-security]: https://git.9dev.tech/9-Developers/fulla.fyi/badges/workflows/security.yml/badge.svg?branch=main&event=push&label=Security&labelColor=1e1e2e&style=flat&logo=trivy&logoColor=white
[link-workflow-security]: https://git.9dev.tech/9-Developers/fulla.fyi/actions/workflows/security.yml
[badge-releases]: https://git.9dev.tech/9-Developers/fulla.fyi/badges/release.svg?color=cba6f7&labelColor=1e1e2e&style=flat&logo=codeberg&logoColor=white
[link-releases]: https://git.9dev.tech/9-Developers/fulla.fyi/releases
[production]: https://fulla.fyi
[beta]: https://beta.fulla.fyi
[sqlx]: https://docs.rs/sqlx
[axum]: https://docs.rs/axum
[askama]: https://docs.rs/askama/
[pulldown-cmark]: https://docs.rs/pulldown-cmark
[htmx]: https://htmx.org
