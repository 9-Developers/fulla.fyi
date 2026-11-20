use std::path::{Path, PathBuf};
use std::process::Command;

use nine_dev_site_build::assets::fingerprint;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let compiled_css = PathBuf::from(std::env::var("OUT_DIR")?).join("main.css");
    let public_css = manifest_dir.join("src/public/main.css");

    println!("cargo:rerun-if-changed=src/style");
    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rerun-if-changed=src/public");
    println!("cargo:rerun-if-changed=migrations");

    let output = Command::new("tailwindcss")
        .args(["-i", "src/style/main.css", "-o"])
        .arg(&compiled_css)
        .arg("--minify")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        return Err(format!("tailwindcss build failed:\n{stdout}\n{stderr}").into());
    }

    // Avoid retriggering this build merely because Tailwind rewrote identical CSS.
    let css = std::fs::read(compiled_css)?;
    if std::fs::read(&public_css).ok().as_deref() != Some(css.as_slice()) {
        std::fs::write(public_css, css)?;
    }

    emit_asset_urls(
        &manifest_dir.join("src/public"),
        &manifest_dir.join("src/public"),
    )?;

    Ok(())
}

fn emit_asset_urls(root: &Path, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            emit_asset_urls(root, &path)?;
            continue;
        }
        let source = path
            .strip_prefix(root)?
            .to_str()
            .ok_or("asset path must be UTF-8")?
            .replace('\\', "/");
        let asset = fingerprint(&source, &std::fs::read(&path)?);
        println!("cargo:rustc-env=ASSET_URL_{source}={}", asset.url);
    }
    Ok(())
}
