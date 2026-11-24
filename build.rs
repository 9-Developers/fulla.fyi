use std::path::{Path, PathBuf};

use nine_dev_site_build::{
    catalog::{
        self,
        Registration::{Directory, File, Revalidate},
    },
    tailwind,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for input in ["src/style", "templates", "public", "migrations"] {
        println!("cargo:rerun-if-changed={input}");
    }
    let out = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?);
    tailwind::write_catppuccin_aliases(Path::new("src/style/catppuccin.aliases.css"))?;
    tailwind::compile(
        Path::new("src/style/main.css"),
        Path::new("public/main.css"),
        &out.join("main.css"),
    )?;

    catalog::write(
        Path::new("public"),
        &out.join("public_assets.rs"),
        &[
            File("main.css"),
            Directory("images"),
            Revalidate("site.webmanifest", "/site.webmanifest"),
            Revalidate("favicon.ico", "/favicon.ico"),
            Revalidate("icon-180.png", "/icon-180.png"),
            Revalidate("icon-192.png", "/icon-192.png"),
            Revalidate("icon-512.png", "/icon-512.png"),
        ],
    )?;
    Ok(())
}
