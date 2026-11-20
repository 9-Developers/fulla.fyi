use std::{
    fmt::Write,
    path::{Path, PathBuf},
};

use nine_dev_site_build::{assets, tailwind};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for input in ["src/style", "templates", "public", "migrations"] {
        println!("cargo:rerun-if-changed={input}");
    }
    let out = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?);
    tailwind::compile(
        Path::new("src/style/main.css"),
        Path::new("public/main.css"),
        &out.join("main.css"),
    )?;

    let mut images = String::from("&[\n");
    for asset in assets::walk(Path::new("public"))? {
        let source = asset.source;
        if source.split('/').any(|part| part.starts_with('.')) {
            // .well-known is served directly; dotfiles are not catalog assets.
            continue;
        }
        let assets::Fingerprint { hash, url } = asset.fingerprint;
        if source.starts_with("images/") {
            writeln!(
                images,
                "Entry {{ source: {source:?}, url: {url:?}, hash: {hash:?} }},"
            )?;
        } else {
            if source.contains('=') {
                return Err(format!("named asset path cannot contain '=': {source}").into());
            }
            println!("cargo:rustc-env=ASSET_URL_{source}={url}");
            println!("cargo:rustc-env=ASSET_HASH_{source}={hash}");
        }
    }
    images.push_str("]\n");
    std::fs::write(out.join("image_index.rs"), images)?;
    Ok(())
}
