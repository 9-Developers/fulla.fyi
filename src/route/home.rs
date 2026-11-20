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
    use axum::{
        body::to_bytes,
        http::{Uri, header},
    };

    use super::home;
    use crate::route::static_asset;
    use crate::static_assets;
    use nine_dev_site_lib::{assets, switcher};

    #[tokio::test]
    async fn bundled_template_assets_use_served_fingerprinted_urls()
    -> Result<(), Box<dyn std::error::Error>> {
        let Ok(page) = home().await else {
            panic!("home page should render");
        };

        let script_pos = page
            .0
            .find(switcher::JS.url)
            .ok_or("missing theme script")?;
        assert!(script_pos < page.0.find("rel=\"stylesheet\"").ok_or("missing CSS")?);
        for asset in [
            &assets::htmx::JS,
            &assets::theme::PALETTE,
            &switcher::JS,
            &switcher::CSS,
            &assets::icons::DISPLAY,
            &assets::icons::ANGLE_DOWN,
        ] {
            assert!(page.0.contains(asset.url));
            let response = static_asset(Uri::from_static(asset.url)).await;
            assert_eq!(response.status(), 200);
            assert_eq!(response.headers()[header::CONTENT_TYPE], asset.content_type);
            assert_eq!(
                response.headers()[header::CACHE_CONTROL],
                "public, max-age=31536000, immutable"
            );
            assert_eq!(
                to_bytes(response.into_body(), 64 * 1024).await?.as_ref(),
                asset.bytes
            );
        }

        for source_path in [
            "main.css",
            "images/icons/codeberg.svg",
            "images/icons/discord.svg",
        ] {
            let url = static_assets::static_url(source_path);

            assert!(
                page.0.contains(url),
                "rendered page should reference {source_path} as {url}"
            );

            let response = static_asset(Uri::from_static(url)).await;
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
            if source_path.starts_with("images/icons/") {
                assert!(page.0.contains(&format!(r#"href="{url}#icon""#)));
                assert!(std::str::from_utf8(&bytes)?.contains(r#"id="icon""#));
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn shared_flavor_icons_keep_their_public_urls() -> Result<(), Box<dyn std::error::Error>>
    {
        use nine_dev_site_lib::assets::theme;

        let Ok(page) = home().await else {
            panic!("home page should render");
        };
        for (url, icon) in [
            (
                "/static/images/icons/latte.86af2ea9588a23fd.svg",
                &theme::LATTE,
            ),
            (
                "/static/images/icons/frappe.0014b909d18c2099.svg",
                &theme::FRAPPE,
            ),
            (
                "/static/images/icons/macchiato.fa94051b39df5e2c.svg",
                &theme::MACCHIATO,
            ),
            (
                "/static/images/icons/mocha.58d995f68329eec0.svg",
                &theme::MOCHA,
            ),
        ] {
            assert!(page.0.contains(&format!(r#"href="{url}#icon""#)));
            let response = static_asset(Uri::from_static(url)).await;
            assert_eq!(response.status(), 200, "{url}");
            assert_eq!(response.headers()[header::CONTENT_TYPE], "image/svg+xml");
            assert_eq!(
                response.headers()[header::CACHE_CONTROL],
                "public, max-age=31536000, immutable"
            );
            assert_eq!(
                to_bytes(response.into_body(), 64 * 1024).await?.as_ref(),
                icon.bytes
            );
        }
        for url in [
            "/static/images/icons/latte.svg",
            "/static/images/icons/latte.0000000000000000.svg",
        ] {
            let response = static_asset(Uri::from_static(url)).await;
            assert_eq!(response.status(), 404);
            assert!(!response.headers().contains_key(header::CACHE_CONTROL));
        }
        Ok(())
    }
}
