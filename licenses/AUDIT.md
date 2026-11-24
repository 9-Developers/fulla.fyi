# License audit — 9DEV-88

## Updating notices

After changing Cargo dependencies, run from the repository root:

```sh
cargo install cargo-about --version 0.9.3 --locked --features cli
cargo install cargo-deny --version 0.21.0 --locked
cargo deny --locked check licenses
cargo about generate --locked --fail licenses/rust.hbs -o licenses/rust.txt
sh .forgejo/scripts/check-license-notices.sh
```

Review the diff before committing the generated file; do not hand-edit the
report. Starting with `nine_dev_site_build` 0.1.1, the published crate includes
the upstream BSD notice. For `nine_dev_site_lib` 0.9.0, `about.toml` verifies
the packaged BSD `LICENSE`, the original Valorant Randomizer `LICENSE-telemetry`,
and the complete `THIRD_PARTY_NOTICES.md`. The latter includes the MIT icon and
palette notices, the 0BSD license for HTMX, and the CC BY 4.0 attribution for
the bundled Font Awesome Patreon symbol. That artwork license is recorded in
the asset notice but not in the published crate's Cargo license expression.

For frontend changes, record the exact source, version, license, copyright
notice, and any required NOTICE text in `THIRD_PARTY_NOTICES.md`.

## Application dependencies

The locked Cargo graph contains 297 packages including Fulla itself.
`cargo deny --locked check licenses` passes with MIT, BSD-3-Clause, 0BSD,
Apache-2.0, Unicode-3.0, and Zlib allowed. Crates offering LGPL as an
alternative also offer MIT or Apache-2.0; no LGPL option is selected.
`cargo-about` includes build, dev, and platform-specific dependencies.
The generated report retains distinct copyright-bearing license texts.

### Canonical MIT fallback review

CodeRabbit identified that the initial report assigned 21 package entries to
canonical MIT text with placeholder copyright holders. Those entries now use
checksum-verified clarifications in `about.toml`:

- Chrono 0.4.45: packaged `LICENSE.txt`, preserving both its introductory
  Kang Seonghoon/contributors attribution and the MIT copyright notice.
- axum-login 0.18.0, include_dir/include_dir_macros 0.7.4, tower-sessions-core
  0.14.0/0.15.0, tower-sessions-memory-store 0.15.0: repository-root license
  files at each published package's `.cargo_vcs_info.json` commit.
- SQLx core, macros, macros-core, and postgres 0.9.0: repository-root
  `LICENSE-MIT` at the published commit. Packaged files contain only a relative
  path, not license text.
- r-efi 5.3.0/6.0.0: identical license/copyright prefix of packaged `AUTHORS`.
  The complete grants and copyright section are retained; MIT is selected.
- whoami 2.1.2: packaged `LICENSE_MIT`, plus the separate copyright notice
  for The WhoAmI Contributors from its packaged `README.md`.
- windows-core, windows-implement, windows-interface, windows-link,
  windows-result, windows-strings, windows-sys: packaged lowercase `license-mit`
  files, preserving Microsoft's copyright notice.

Combined license documents are retained verbatim where needed, even though only
the MIT alternative is selected. CI rejects the specific canonical MIT copyright
placeholders; Apache's unfilled example appendix is legitimate license text.
The guard is a regression check, not a comprehensive legal/completeness audit.

The manually maintained browser inventory is in `THIRD_PARTY_NOTICES.md`.
HTMX's v4.0.0 license and Tailwind's v4.3.5 license were checked against
their tagged upstream sources. The Catppuccin, Mulish, and Source Code Pro
notices were checked against their upstream license files. The original
Catppuccin palette import does not record a revision. Mulish and Source Code Pro
are fetched remotely from Google Fonts without pinned revisions; no font
binaries are bundled.

## Artwork

The maintainer recalls sourcing most SVGs from SVG Viewer. That site is an
aggregator with multiple licenses, not a blanket license grant.

- The home page's GitHub links use the Bootstrap Icons v1.13.1 GitHub symbol.
  The former Codeberg link used the Octicons v12.0.0 git-branch-24 symbol.
  The shared library includes both MIT notices.
- `latte.svg`, `frappe.svg`, `macchiato.svg`, `mocha.svg`: replaced with
  Microsoft Fluent Emoji High Contrast Sunflower, Potted plant, Hibiscus,
  and Herb at revision `1ffb34c752ecf5d402f04cfb4b392c77f57c54bc`.
  Supplied by `nine_dev_site_lib` since 0.1.0; the bytes and fingerprinted
  URLs match Fulla's original files. MIT notice included. Path geometry and fill rules are preserved; SVG
  wrappers use symbols and fills inherit `currentColor`.
  This removes the previous Macchiato artwork from ICOOON MONO, whose
  redistribution restrictions conflicted with the aggregator's PD label.
  Old revisions still contain that artwork.
- `expand.svg` and `system.svg` were retired with the local theme switcher.
- The Discord link now uses the shared Bootstrap Icons v1.13.1 symbol. The
  former local `discord.svg` had unverified provenance and remains in old revisions.

Sources:

- https://github.com/microsoft/fluentui-emoji/tree/1ffb34c752ecf5d402f04cfb4b392c77f57c54bc/assets
- https://github.com/microsoft/fluentui-emoji/blob/1ffb34c752ecf5d402f04cfb4b392c77f57c54bc/LICENSE
- https://www.svgviewer.dev/s/382843/hibiscus-illustration
- https://icooon-mono.com/license/
- https://github.com/primer/octicons/blob/v12.0.0/icons/git-branch-24.svg
- https://github.com/primer/octicons/blob/v12.0.0/LICENSE

## Scope

This inventory covers the application Cargo graph and browser assets.
It is not a container or operating-system software bill of materials.
System libraries, Railway's base image, npm build-tool dependencies, and
Go deployment tooling are not covered by cargo-about. Reassess those
components if distributing build environments or container images.
