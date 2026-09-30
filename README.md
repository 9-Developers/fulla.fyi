# fulla.fyi

[![Discord](https://img.shields.io/discord/452233740408717313?color=738ADB&label=discord&logo=discord)][3]
[![License](https://img.shields.io/badge/License-BSD_3--Clause-blue.svg)][4]
[![Build](https://codeberg.org/9-Developers/fulla.fyi/badges/workflows/build.yml/badge.svg?branch=main&event=push&label=build)][5]
[![Security](https://codeberg.org/9-Developers/fulla.fyi/badges/workflows/security-trusted.yml/badge.svg?branch=main&event=push&label=security)][6]

Share wishlists with your friends.

The current Rust draft serves the home page and bundled assets. A local `.env` is
loaded when present; `PORT` defaults to `3000` and `RUST_LOG` to `info`.
Invalid supplied values fail startup. `/health` checks this stateless draft only:
the database pool and product schema are not yet connected to the server.

## Links

- [Production][1]
- [Beta][2]

## Tech stack

| Requirement     | Dependency           |
|-----------------|----------------------|
| Database        | [SQLx][7]            |
| HTTP handler    | [Axum][8]            |
| HTML templates  | [Askama][9]          |
| Markdown parser | [pulldown_cmark][10] |
| Javascript      | [HTMX][11]           |

[1]: https://fulla.fyi
[2]: https://beta.fulla.fyi
[3]: https://discord.gg/KTgshSbEUU
[4]: https://opensource.org/license/bsd-3-clause
[5]: https://codeberg.org/9-Developers/fulla.fyi/actions/workflows/build.yml
[6]: https://codeberg.org/9-Developers/fulla.fyi/actions/workflows/security-trusted.yml
[7]: https://docs.rs/sqlx
[8]: https://docs.rs/axum
[9]: https://docs.rs/askama/
[10]: https://docs.rs/pulldown-cmark
[11]: https://htmx.org
