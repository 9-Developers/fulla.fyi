//! Explicit public assets with fingerprinted URLs supplied after CSS compilation.

use include_dir::{Dir, File, include_dir};

static PUBLIC_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/src/public");

macro_rules! asset {
    ($path:literal) => {
        ($path, env!(concat!("ASSET_URL_", $path)))
    };
}

// Add an entry when adding an asset. Tests check full coverage.
const STATIC_ASSETS: &[(&str, &str)] = &[
    asset!("get_theme.js"),
    asset!("htmx.min.js"),
    asset!("images/icons/codeberg.svg"),
    asset!("images/icons/discord.svg"),
    asset!("images/icons/expand.svg"),
    asset!("images/icons/frappe.svg"),
    asset!("images/icons/latte.svg"),
    asset!("images/icons/macchiato.svg"),
    asset!("images/icons/mocha.svg"),
    asset!("images/icons/system.svg"),
    asset!("main.css"),
    asset!("theme.js"),
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
    STATIC_ASSETS
        .iter()
        .find_map(|(source, url)| (*url == request_path).then_some(*source))
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
            assert_eq!(source_path_for_request(url), Some(source));
            assert_eq!(source_path_for_request(&url.replace(&hash, "stale")), None);
            assert_eq!(source_path_for_request(source), None);
        }
        assert_eq!(static_url("missing.css"), "/static/missing");
        Ok(())
    }
}
