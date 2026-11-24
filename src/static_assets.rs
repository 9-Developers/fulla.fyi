//! Site-owned public files; only images are enumerated automatically.

use include_dir::{Dir, File, include_dir};
use nine_dev_site_lib::catalog::SiteCatalog;

static PUBLIC_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/public");

pub(crate) static CATALOG: SiteCatalog = include!(concat!(env!("OUT_DIR"), "/public_assets.rs"));

/// Return a declared asset's public URL for use in templates.
#[must_use]
pub fn static_url(source: &str) -> &'static str {
    CATALOG
        .by_source(source)
        .map_or("/static/missing", |entry| entry.url)
}

/// Resolve only the current encoded URL for an immutable site asset.
#[must_use]
pub fn source_path_for_request(path: &str) -> Option<&'static str> {
    CATALOG.immutable.by_url(path).map(|entry| entry.source)
}

pub(crate) fn bytes(source: &str) -> Option<&'static [u8]> {
    PUBLIC_DIR.get_file(source).map(File::contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::collections::BTreeSet;

    fn paths(dir: &Dir<'_>, output: &mut BTreeSet<String>) {
        for file in dir.files() {
            let path = file.path().to_string_lossy().replace('\\', "/");
            if !path.split('/').any(|part| part.starts_with('.')) {
                output.insert(path);
            }
        }
        for child in dir.dirs() {
            paths(child, output);
        }
    }

    #[test]
    fn catalog_matches_embedded_bytes_and_resolves_only_current_urls()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut embedded = BTreeSet::new();
        paths(&PUBLIC_DIR, &mut embedded);
        let mut declared = BTreeSet::new();
        let mut urls = BTreeSet::new();
        for entry in CATALOG
            .immutable
            .0
            .iter()
            .chain(CATALOG.aliases.iter().map(|alias| &alias.entry))
        {
            assert!(declared.insert(entry.source.to_owned()));
            assert!(urls.insert(entry.url));
            assert!(
                nine_dev_site_lib::assets::lookup(entry.url).is_none(),
                "URL collides with a shared asset: {}",
                entry.url
            );
            let bytes = bytes(entry.source).ok_or("declared asset missing")?;
            let hash = format!(
                "{:016x}",
                u64::from_be_bytes(Sha256::digest(bytes)[..8].try_into()?)
            );
            assert_eq!(entry.hash, hash);
            assert_eq!(static_url(entry.source), entry.url);
            if CATALOG.alias_by_url(entry.url).is_none() {
                assert!(entry.url.contains(&format!(".{hash}")));
                assert_eq!(source_path_for_request(entry.url), Some(entry.source));
                assert_eq!(
                    source_path_for_request(&entry.url.replace(&hash, "stale")),
                    None
                );
                assert_eq!(
                    source_path_for_request(&format!("/static/{}", entry.source)),
                    None
                );
            } else {
                assert_eq!(source_path_for_request(entry.url), None);
            }
        }
        assert_eq!(declared, embedded);
        assert_eq!(static_url("missing.css"), "/static/missing");
        Ok(())
    }
}
