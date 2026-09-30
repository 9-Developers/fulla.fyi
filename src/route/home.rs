//! Home route handler.

use askama::Template;
use axum::response::Html;
use tracing::error;

use crate::PageError;

#[derive(Template)]
#[template(path = "home.html")]
struct HomePage {
    title: &'static str,
}

/// Renders the home page.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn home() -> Result<Html<String>, PageError> {
    let page = HomePage { title: "Home" };

    match page.render() {
        Ok(html) => Ok(Html(html)),
        Err(err) => {
            error!(error = %err, "home: template render failed");

            Err(PageError::internal_server_error())
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::to_bytes, extract::Path, http::header};

    use super::home;
    use crate::route::static_asset;
    use crate::static_assets;

    #[tokio::test]
    async fn bundled_template_assets_use_served_fingerprinted_urls()
    -> Result<(), Box<dyn std::error::Error>> {
        let Ok(page) = home().await else {
            panic!("home page should render");
        };

        for source_path in [
            "main.css",
            "htmx.min.js",
            "get_theme.js",
            "theme.js",
            "images/icons/system.svg",
            "images/icons/expand.svg",
            "images/icons/latte.svg",
            "images/icons/frappe.svg",
            "images/icons/macchiato.svg",
            "images/icons/mocha.svg",
            "images/icons/codeberg.svg",
            "images/icons/discord.svg",
        ] {
            let url = static_assets::static_url(source_path);

            assert!(
                page.0.contains(url),
                "rendered page should reference {source_path} as {url}"
            );

            let path = url.trim_start_matches("/static/").to_owned();
            let response = static_asset(Path(path)).await;
            assert_eq!(
                response.status(),
                200,
                "asset {source_path} should be served"
            );
            assert_eq!(
                response.headers()[header::CACHE_CONTROL],
                "public, max-age=31536000, immutable"
            );
            let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024).await?;
            assert!(!bytes.is_empty(), "asset {source_path} should not be empty");
        }
        Ok(())
    }
}
