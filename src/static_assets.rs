//! Explicit public assets with hashes supplied by the build script after CSS compilation.

use include_dir::{Dir, File, include_dir};

static PUBLIC_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/src/public");

// Add an entry when adding an asset. Tests check full coverage.
const STATIC_ASSETS: &[(&str, &str)] = &[
    (
        "get_theme.js",
        concat!("/static/get_theme.", env!("ASSET_HASH_get_theme.js"), ".js"),
    ),
    (
        "htmx.min.js",
        concat!("/static/htmx.min.", env!("ASSET_HASH_htmx.min.js"), ".js"),
    ),
    (
        "images/icons/codeberg.svg",
        concat!(
            "/static/images/icons/codeberg.",
            env!("ASSET_HASH_images/icons/codeberg.svg"),
            ".svg"
        ),
    ),
    (
        "images/icons/discord.svg",
        concat!(
            "/static/images/icons/discord.",
            env!("ASSET_HASH_images/icons/discord.svg"),
            ".svg"
        ),
    ),
    (
        "images/icons/expand.svg",
        concat!(
            "/static/images/icons/expand.",
            env!("ASSET_HASH_images/icons/expand.svg"),
            ".svg"
        ),
    ),
    (
        "images/icons/frappe.svg",
        concat!(
            "/static/images/icons/frappe.",
            env!("ASSET_HASH_images/icons/frappe.svg"),
            ".svg"
        ),
    ),
    (
        "images/icons/latte.svg",
        concat!(
            "/static/images/icons/latte.",
            env!("ASSET_HASH_images/icons/latte.svg"),
            ".svg"
        ),
    ),
    (
        "images/icons/macchiato.svg",
        concat!(
            "/static/images/icons/macchiato.",
            env!("ASSET_HASH_images/icons/macchiato.svg"),
            ".svg"
        ),
    ),
    (
        "images/icons/mocha.svg",
        concat!(
            "/static/images/icons/mocha.",
            env!("ASSET_HASH_images/icons/mocha.svg"),
            ".svg"
        ),
    ),
    (
        "images/icons/system.svg",
        concat!(
            "/static/images/icons/system.",
            env!("ASSET_HASH_images/icons/system.svg"),
            ".svg"
        ),
    ),
    (
        "main.css",
        concat!("/static/main.", env!("ASSET_HASH_main.css"), ".css"),
    ),
    (
        "theme.js",
        concat!("/static/theme.", env!("ASSET_HASH_theme.js"), ".js"),
    ),
];

/// Return the content-addressed URL of a declared asset.
#[must_use]
pub fn static_url(source_path: &str) -> &'static str {
    STATIC_ASSETS
        .iter()
        .find(|(source, _)| *source == source_path)
        .map_or("/static/missing", |(_, url)| *url)
}

/// Resolve only current, exact content-addressed request paths.
#[must_use]
pub fn source_path_for_request(request_path: &str) -> Option<&'static str> {
    STATIC_ASSETS.iter().find_map(|(source, url)| {
        (url.strip_prefix("/static/") == Some(request_path)).then_some(*source)
    })
}

pub(crate) fn get_file(source_path: &str) -> Option<&'static File<'static>> {
    PUBLIC_DIR.get_file(source_path)
}

#[cfg(test)]
mod tests {
    use super::{PUBLIC_DIR, STATIC_ASSETS, source_path_for_request, static_url};
    use include_dir::Dir;
    use sha2::{Digest, Sha256};
    use std::collections::BTreeSet;

    const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

    fn paths(dir: &Dir<'_>, output: &mut BTreeSet<String>) {
        for file in dir.files() {
            output.insert(file.path().to_string_lossy().replace('\\', "/"));
        }
        for child in dir.dirs() {
            paths(child, output);
        }
    }

    #[test]
    fn declarations_cover_embedded_files_once() {
        let mut embedded = BTreeSet::new();
        paths(&PUBLIC_DIR, &mut embedded);
        let declared: BTreeSet<_> = STATIC_ASSETS
            .iter()
            .map(|(path, _)| (*path).to_owned())
            .collect();
        assert_eq!(declared, embedded);
        assert_eq!(declared.len(), STATIC_ASSETS.len());
    }

    #[test]
    fn urls_match_bytes_and_reject_stale_hashes() -> Result<(), Box<dyn std::error::Error>> {
        for &(source, url) in STATIC_ASSETS {
            let file = PUBLIC_DIR.get_file(source).ok_or("asset missing")?;
            let digest = Sha256::digest(file.contents());
            let mut hash = String::with_capacity(16);
            for byte in &digest[..8] {
                hash.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
                hash.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
            }
            assert!(url.contains(&format!(".{hash}.")));
            assert_eq!(static_url(source), url);
            let request = url.strip_prefix("/static/").ok_or("bad URL")?;
            assert_eq!(source_path_for_request(request), Some(source));
            assert_eq!(
                source_path_for_request(&request.replace(&hash, "stale")),
                None
            );
            assert_eq!(source_path_for_request(source), None);
        }
        assert_eq!(static_url("missing.css"), "/static/missing");
        Ok(())
    }
}
