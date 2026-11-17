# License audit — 9DEV-88

## Application dependencies

The locked Cargo graph contains 259 packages including Fulla itself.
`cargo deny --locked check licenses` passes with MIT, BSD-3-Clause,
Apache-2.0, Unicode-3.0, and Zlib allowed. Crates offering LGPL as an
alternative also offer MIT or Apache-2.0; no LGPL option is selected.
`cargo-about` includes build, dev, and platform-specific dependencies.
No separate `*NOTICE*` files were found in the downloaded dependency sources.
The generated report retains distinct copyright-bearing license texts.

The manually maintained browser inventory is in `THIRD_PARTY_NOTICES.md`.
HTMX's v4.0.0 license and Tailwind's v4.3.3 license were checked against
their tagged upstream sources. Catppuccin and Google Fonts' Sora notices
were checked against their upstream license files. The original Catppuccin
palette import does not record a revision. Sora is fetched remotely.

## Artwork

The maintainer recalls sourcing most SVGs from SVG Viewer. That site is an
aggregator with multiple licenses, not a blanket license grant.

- `codeberg.svg`: exact path match to Octicons v12.0.0 git-branch-24.
  MIT notice now included.
- `latte.svg`, `frappe.svg`, `macchiato.svg`, `mocha.svg`: replaced with
  Microsoft Fluent Emoji High Contrast Sunflower, Potted plant, Hibiscus,
  and Herb at revision `1ffb34c752ecf5d402f04cfb4b392c77f57c54bc`.
  MIT notice included. Path geometry and fill rules are preserved; SVG
  wrappers use symbols and fills inherit `currentColor`.
  This removes the previous Macchiato artwork from ICOOON MONO, whose
  redistribution restrictions conflicted with the aggregator's PD label.
  Old revisions still contain that artwork.
- `discord.svg`, `expand.svg`, `system.svg`: exact upstream sources and
  licenses remain unverified.
  Obtain original item URLs or replace them with verified assets.

Sources:
- https://github.com/microsoft/fluentui-emoji/tree/1ffb34c752ecf5d402f04cfb4b392c77f57c54bc/assets
- https://github.com/microsoft/fluentui-emoji/blob/1ffb34c752ecf5d402f04cfb4b392c77f57c54bc/LICENSE
- https://www.svgviewer.dev/s/382843/hibiscus-illustration
- https://icooon-mono.com/license/
- https://github.com/primer/octicons/blob/v12.0.0/icons/git-branch-24.svg
- https://github.com/primer/octicons/blob/v12.0.0/LICENSE

The page and CI checks do not resolve these artwork provenance gaps.
Do not mark the overall artwork audit complete until they are resolved.

## Scope

This inventory covers the application Cargo graph and browser assets.
It is not a container or operating-system software bill of materials.
System libraries, Railway's base image, npm build-tool dependencies, and
Go deployment tooling are not covered by cargo-about. Reassess those
components if distributing build environments or container images.
