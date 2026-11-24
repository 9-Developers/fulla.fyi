# Fulla

[![Discord](https://img.shields.io/discord/452233740408717313?color=b4befe&label=discord&labelColor=1e1e2e&logo=discord&style=flat)](https://discord.gg/KTgshSbEUU)
[![License](https://img.shields.io/badge/License-BSD_3--Clause-89b4fa?labelColor=1e1e2e&logo=bsd&logoColor=white&style=flat)](https://opensource.org/license/bsd-3-clause)
[![Deployment](https://git.9dev.tech/9-Developers/fulla.fyi/badges/workflows/deploy.yml/badge.svg?branch=main&event=push&label=Deployment&labelColor=1e1e2e&style=flat&logo=forgejo&logoColor=white)](https://git.9dev.tech/9-Developers/fulla.fyi/actions/workflows/deploy.yml)

Fulla is a wishlist organizer in development for families and friends who want
to share gift ideas from any retailer. You can create an account, sign in, and
visit your private account page. Creating wishlists, sharing them, and recording
purchases are planned features.

[Visit the beta](https://beta.fulla.fyi) · [Join the Discord](https://discord.gg/KTgshSbEUU)

## Run locally

From a checkout of this repository, you'll need:

- [Rust via rustup](https://rustup.rs/). The checkout selects the version in
  [`rust-toolchain.toml`](rust-toolchain.toml).
- [Tailwind CLI 4.3.5](https://github.com/tailwindlabs/tailwindcss/releases/tag/v4.3.5).
  Put the executable for your platform on `PATH` as `tailwindcss`. Verify that
  `tailwindcss --help` prints `v4.3.5` before building; CI and Railpack use the same version.
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

1. Open [registration](http://localhost:3000/register). Choose a username,
   provide an email address, and use a strong password or passphrase (12–128
   UTF-8 bytes). Registration signs you in and opens your account page.
2. [Sign out](http://localhost:3000/logout) and confirm. Opening
   [your account](http://localhost:3000/account) now takes you to sign-in.
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
| `PORT`           | `3000`   | HTTP listen port (1–65535). Not used by `migrate`.         |
| `RUST_LOG`       | `info`   | Logging filter.                                            |

After updating Fulla, rerun the migration command before starting the server.
Startup checks migration history. To check whether the running app can reach
PostgreSQL, request `http://localhost:3000/health`: it returns HTTP 200 when
ready or 503 when the database is unavailable.

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
[badge-workflow-security]: https://git.9dev.tech/9-Developers/fulla.fyi/badges/workflows/security.yml/badge.svg?branch=main&event=push&label=Security&labelColor=1e1e2e&style=flat&logo=trivy&logoColor=white
[link-workflow-security]: https://git.9dev.tech/9-Developers/fulla.fyi/actions/workflows/security.yml
[badge-releases]: https://git.9dev.tech/9-Developers/fulla.fyi/badges/release.svg?color=cba6f7&labelColor=1e1e2e&style=flat&logo=codeberg&logoColor=white
[link-releases]: https://git.9dev.tech/9-Developers/fulla.fyi/releases
