use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let compiled_css = PathBuf::from(std::env::var("OUT_DIR")?).join("main.css");
    let public_css = manifest_dir.join("src/public/main.css");

    println!("cargo:rerun-if-changed=src/style");
    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rerun-if-changed=src/public");

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

    emit_hashes(
        &manifest_dir.join("src/public"),
        &manifest_dir.join("src/public"),
    )?;

    Ok(())
}

fn emit_hashes(root: &Path, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            emit_hashes(root, &path)?;
            continue;
        }
        let source = path
            .strip_prefix(root)?
            .to_str()
            .ok_or("asset path must be UTF-8")?
            .replace('\\', "/");
        let digest = Sha256::digest(std::fs::read(&path)?);
        let mut hash = String::with_capacity(16);
        for byte in &digest[..8] {
            hash.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
            hash.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
        }
        println!("cargo:rustc-env=ASSET_HASH_{source}={hash}");
    }
    Ok(())
}
