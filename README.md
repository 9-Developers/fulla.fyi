# Fulla

[![Discord](https://img.shields.io/discord/452233740408717313?color=b4befe&label=discord&labelColor=1e1e2e&logo=discord&style=flat)](https://discord.gg/KTgshSbEUU)
[![License](https://img.shields.io/badge/License-BSD_3--Clause-89b4fa?labelColor=1e1e2e&logo=bsd&logoColor=white&style=flat)](https://opensource.org/license/bsd-3-clause)
[![Deployment](https://git.9dev.tech/9-Developers/fulla.fyi/badges/workflows/staging.yml/badge.svg?branch=main&event=push&label=Deployment&labelColor=1e1e2e&style=flat&logo=forgejo&logoColor=white)](https://git.9dev.tech/9-Developers/fulla.fyi/actions/workflows/staging.yml)

Fulla is a wishlist organizer in development for families and friends who want
to share gift ideas from any retailer. You can create an account, sign in, and
visit your private account page. Creating wishlists, sharing them, and recording
purchases are planned features.

[Visit the beta](https://beta.fulla.fyi) · [Join the Discord](https://discord.gg/KTgshSbEUU)

## Run locally

From a checkout of this repository, you'll need:

- Until `nine_dev_site_lib` 0.12.0 is published, a sibling checkout of
  `9dev-site-lib` with that version and `nine_dev_site_build` 0.2.4.
  Use `cargo --config dev/local-site-lib.toml` in place of `cargo` below.
- [Rust via rustup](https://rustup.rs/). The checkout selects the version in
  [`rust-toolchain.toml`](rust-toolchain.toml).
- [Tailwind CLI 4.3.5](https://github.com/tailwindlabs/tailwindcss/releases/tag/v4.3.5).
  Put the executable for your platform on `PATH` as `tailwindcss`. Verify that
  `tailwindcss --help` prints `v4.3.5` before building; CI and image releases use the same version.
- A PostgreSQL 18 database. If you have Docker, you can start one locally:

  ```sh
  docker run --name fulla-postgres -e POSTGRES_PASSWORD=fulla_local \
    -p 127.0.0.1:5432:5432 -d postgres:18
  docker exec fulla-postgres pg_isready -h 127.0.0.1 -U postgres
  ```

Once `pg_isready` reports “accepting connections,” apply the migrations and
start the server. If using an existing database, substitute its URL below.

```sh
export DATABASE_URL=postgres://postgres:fulla_local@127.0.0.1:5432/postgres
cargo run --locked -- migrate
SESSION_SECURE=false cargo run --locked
```

## Try an account

Fonts are bundled in the executable. Loading a page does not contact Google Fonts.

1. Open [registration](http://localhost:8080/register). Choose a username,
   provide an email address, and use a strong password or passphrase (12–128
   UTF-8 bytes). Registration signs you in and opens your account page.
2. [Sign out](http://localhost:8080/logout) and confirm. Opening
   [your account](http://localhost:8080/account) now takes you to sign-in.
3. Sign back in with either your username or your email address. Account forms
   work without JavaScript. Email verification and password recovery are not yet
   available.

## Configuration

Set these variables in your shell or in `./.env` in the directory where you
start Fulla. Exported values take precedence; parent directories are not searched.

| Variable         | Default  | Use                                                        |
| ---------------- | -------- | ---------------------------------------------------------- |
| `DATABASE_URL`   | Required | PostgreSQL connection URL for migrations and the server.   |
| `SESSION_SECURE` | `true`   | Set to `false` for local HTTP only. Keep `true` for HTTPS. |
| `HOST`           | `0.0.0.0` | HTTP listen address (IPv4 or unbracketed IPv6). Server only. |
| `PORT`           | `8080`   | HTTP listen port (1–65535). Railway sets `3000`. Server only. |
| `RUST_LOG`       | `info`   | Logging filter.                                            |

After updating Fulla, rerun the migration command before starting the server.
Startup checks migration history. To check whether the running app can reach
PostgreSQL, request `http://localhost:8080/health`: it returns HTTP 200 when
ready or 503 when the database is unavailable.

## Railway releases and rollback

Staging builds a Linux/amd64 image. After Railway reports that deployment
healthy, Forgejo records its commit and image digest in a staging receipt.
Production tags promote that digest without rebuilding. Keep the successful
run, its receipt, and the registry image for every supported release.
Commits staged only by the old source workflow need a new image staging run.

Before relying on image rollback, use Docker and the two retained digest
references from the deployment logs to rehearse the upgrade on disposable
PostgreSQL:

```sh
bash scripts/rehearse-rollback.sh "$PREVIOUS_IMAGE" "$STAGED_IMAGE"
```

The rehearsal starts each image, checks `/health`, then retries the previous
image's migration command and startup against the upgraded database. A newer
migration prevents an image-only rollback, even for additive SQL. In that
case roll forward with a fix built for the migrated schema. Restoring a
database backup is a separate recovery procedure and can lose later writes.
Keep migrations compatible with the running binary during deployment.

## Grafana Cloud

The server writes JSON logs to stdout. To also send application logs and HTTP
request counts and durations to Grafana Cloud, set `OTEL_EXPORTER_OTLP_ENDPOINT`
and `OTEL_EXPORTER_OTLP_HEADERS` to the base endpoint and authorization header
provided by your Grafana stack. Use a token with `logs:write` and `metrics:write`.
On Railway, set these on the `fulla.fyi` service in each environment where you
want telemetry. Railway configuration applies preserve both values and the
optional `OTEL_SERVICE_NAME` override. Leave the endpoint unset to run locally
without export.

Import the [shared **9 Developers · Site HTTP** dashboard](https://git.9dev.tech/9-Developers/9dev-site-lib/src/branch/main/observability/grafana-dashboard.json)
and select `fulla.fyi` (or
your `OTEL_SERVICE_NAME` override). Railway sets `DEPLOYMENT_ENVIRONMENT` to
`staging` or `production`; local runs default to `development`. Request `/` or
`/licenses` to generate traffic; `/health` is excluded. The `migrate` command
logs locally and needs no Grafana configuration.

## License

Fulla uses the [BSD 3-Clause License](LICENSE). When redistributing it, include
the license, [frontend notices](THIRD_PARTY_NOTICES.md), and
[Rust dependency notices](licenses/rust.txt). The running app also makes the
notices available at `/licenses`.

[badge-discord]: https://img.shields.io/discord/452233740408717313?color=b4befe&label=discord&labelColor=1e1e2e&logo=discord&style=flat
[link-discord]: https://discord.gg/KTgshSbEUU
[badge-bsd-3]: https://img.shields.io/badge/License-BSD_3--Clause-89b4fa?labelColor=1e1e2e&logo=bsd&logoColor=white&style=flat
[link-bsd-3]: https://opensource.org/license/bsd-3-clause
[badge-workflow-build]: https://git.9dev.tech/9-Developers/fulla.fyi/badges/workflows/build.yml/badge.svg?branch=main&event=push&label=Build&labelColor=1e1e2e&style=flat&logo=forgejo&logoColor=white
[link-workflow-build]: https://git.9dev.tech/9-Developers/fulla.fyi/actions/workflows/build.yml
[badge-workflow-security]: https://git.9dev.tech/9-Developers/fulla.fyi/badges/workflows/checks.yml/badge.svg?branch=main&event=push&label=Security&labelColor=1e1e2e&style=flat&logo=trivy&logoColor=white
[link-workflow-security]: https://git.9dev.tech/9-Developers/fulla.fyi/actions/workflows/checks.yml
[badge-releases]: https://git.9dev.tech/9-Developers/fulla.fyi/badges/release.svg?color=cba6f7&labelColor=1e1e2e&style=flat&logo=codeberg&logoColor=white
[link-releases]: https://git.9dev.tech/9-Developers/fulla.fyi/releases
