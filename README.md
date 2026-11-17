# Fulla

[![Discord][badge-discord]][link-discord]
[![License][badge-bsd-3]][link-bsd-3]
[![Build][badge-workflow-build]][link-workflow-build]
[![Security][badge-workflow-security]][link-workflow-security]
[![Release][badge-releases]][link-releases]

Share wishlists with your friends.

The current Rust draft serves the home page and bundled assets. A local `.env` is
loaded when present; `PORT` defaults to `3000` and `RUST_LOG` to `info`.
Invalid supplied values fail startup. `/health` checks this stateless draft only:
the database pool and product schema are not yet connected to the server.

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

The application uses Railpack with Tailwind 4.3.3 from `railpack.json`, builds with
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

The server does not consume `DATABASE_URL` yet. Connecting SQLx, running
migrations, persistent sessions, and database-aware readiness checks are future
application work. There is currently no pre-deploy migration command.

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
