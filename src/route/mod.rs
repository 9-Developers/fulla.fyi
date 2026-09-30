//! Route handlers.

use axum::{
    Router,
    body::Body,
    extract::Path,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};

use crate::static_assets;

pub use error::*;
pub use home::*;

mod error;
mod home;

/// Get app routes.
pub fn get_routes() -> Router {
    Router::new()
        .route("/.well-known/{*path}", get(well_known_asset))
        .route("/static/{*path}", get(static_asset))
        .route("/", get(home))
        .route("/health", get(health))
        .fallback(not_found)
}

async fn health() -> StatusCode {
    // This draft has no active database connection yet; the router is its essential dependency.
    StatusCode::OK
}

async fn well_known_asset(Path(path): Path<String>) -> Response {
    if path
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return StatusCode::NOT_FOUND.into_response();
    }

    let source_path = format!(".well-known/{path}");
    let Some(file) = static_assets::get_file(&source_path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    asset_response(
        file.contents(),
        &source_path,
        HeaderValue::from_static("public, max-age=3600"),
    )
}

async fn static_asset(Path(path): Path<String>) -> Response {
    let Some(source_path) = static_assets::source_path_for_request(&path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(file) = static_assets::get_file(source_path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    asset_response(
        file.contents(),
        source_path,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    )
}

fn asset_response(
    contents: &'static [u8],
    source_path: &str,
    cache_control: HeaderValue,
) -> Response {
    let mut response = Body::from(contents).into_response();
    let mime = mime_guess::from_path(source_path).first_or_octet_stream();
    let content_type = HeaderValue::from_str(mime.as_ref())
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));

    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, content_type);
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, cache_control);

    response
}

#[cfg(test)]
mod tests {
    use axum::{
        body::to_bytes,
        extract::Path,
        http::{HeaderValue, header},
        response::IntoResponse,
    };

    use super::{PageError, health, static_asset, well_known_asset};
    use crate::static_assets;

    #[tokio::test]
    async fn current_stateless_draft_is_ready() {
        assert_eq!(health().await, 200);
    }

    #[tokio::test]
    async fn page_errors_render_safe_full_pages() {
        for (error, status, heading) in [
            (PageError::not_found(), 404, "Page not found"),
            (
                PageError::internal_server_error(),
                500,
                "Something went wrong",
            ),
        ] {
            let response = error.into_response();
            assert_eq!(response.status(), status);
            let body = to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap_or_default();
            let html = String::from_utf8_lossy(&body);
            assert!(html.contains(heading));
            assert!(!html.contains("template render failed"));
        }
    }

    #[tokio::test]
    async fn fingerprinted_static_asset_is_cached_immutably() {
        let path = static_assets::static_url("theme.js")
            .trim_start_matches("/static/")
            .to_string();
        let response = static_asset(Path(path)).await;

        assert_eq!(response.status(), 200);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&HeaderValue::from_static(
                "public, max-age=31536000, immutable"
            ))
        );
        assert!(response.headers().contains_key(header::CONTENT_TYPE));
    }

    #[tokio::test]
    async fn unknown_static_asset_is_not_cached() {
        let response = static_asset(Path("theme.invalid.js".to_string())).await;

        assert_eq!(response.status(), 404);
        assert!(!response.headers().contains_key(header::CACHE_CONTROL));
    }

    #[tokio::test]
    async fn well_known_asset_rejects_parent_segments() {
        let response = well_known_asset(Path("../theme.js".to_string())).await;

        assert_eq!(response.status(), 404);
        assert!(!response.headers().contains_key(header::CACHE_CONTROL));
    }
}
