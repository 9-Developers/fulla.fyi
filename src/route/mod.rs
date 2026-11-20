//! Route handlers.

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::get,
};

use crate::database::AppState;
use crate::static_assets;
use axum_login::tower_sessions;

pub use error::*;
pub use home::*;
pub use licenses::*;

mod auth;
mod error;
mod home;
mod licenses;

/// Get app routes.
///
/// # Errors
/// Returns an error if the authentication backend cannot initialize.
pub async fn get_routes(
    state: AppState,
    secure_cookies: bool,
) -> Result<Router, crate::auth::AuthError> {
    let backend = crate::auth::Backend::new(state.pool.clone()).await?;
    let sessions =
        tower_sessions::SessionManagerLayer::new(crate::auth::store::Store(state.pool.clone()))
            .with_name(if secure_cookies {
                "__Host-fulla_session"
            } else {
                "fulla_session"
            })
            .with_secure(secure_cookies)
            .with_http_only(true)
            .with_same_site(tower_sessions::cookie::SameSite::Lax)
            .with_expiry(tower_sessions::Expiry::OnInactivity(
                time::Duration::minutes(30),
            ));
    let accounts = Router::new()
        .route("/register", get(auth::register_page).post(auth::register))
        .route("/login", get(auth::login_page).post(auth::login))
        .route("/logout", get(auth::logout_page).post(auth::logout))
        .route("/account", get(auth::account))
        .layer(axum::extract::DefaultBodyLimit::max(16 * 1024))
        .layer(axum_login::AuthManagerLayerBuilder::new(backend, sessions).build())
        .layer(axum::middleware::from_fn(no_store));
    Ok(Router::new()
        .merge(accounts)
        .route("/.well-known/{*path}", get(well_known_asset))
        .route("/static/{*path}", get(static_asset))
        .route("/", get(home))
        .route("/licenses", get(licenses))
        .route("/health", get(health))
        .fallback(not_found)
        .with_state(state))
}

async fn no_store(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("same-origin"),
    );
    response
        .headers_mut()
        .insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    response
}

async fn health(State(state): State<AppState>) -> StatusCode {
    match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        sqlx::query("SELECT 1").execute(&state.pool),
    )
    .await
    {
        Ok(Ok(_)) => StatusCode::OK,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
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
        content_type(&source_path),
        "public, max-age=3600",
    )
}

async fn static_asset(uri: Uri) -> Response {
    const IMMUTABLE: &str = "public, max-age=31536000, immutable";
    if let Some(asset) = nine_dev_site_lib::assets::lookup(uri.path()) {
        return asset_response(
            asset.bytes,
            HeaderValue::from_static(asset.content_type),
            IMMUTABLE,
        );
    }
    let Some(source_path) = static_assets::source_path_for_request(uri.path()) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(file) = static_assets::get_file(source_path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    asset_response(file.contents(), content_type(source_path), IMMUTABLE)
}

fn content_type(source_path: &str) -> HeaderValue {
    let mime = mime_guess::from_path(source_path).first_or_octet_stream();
    HeaderValue::from_str(mime.as_ref())
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"))
}

fn asset_response(
    contents: &'static [u8],
    content_type: HeaderValue,
    cache_control: &'static str,
) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static(cache_control),
            ),
        ],
        Body::from(contents),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use axum::{
        body::to_bytes,
        extract::Path,
        http::{HeaderValue, Uri, header},
        response::IntoResponse,
    };

    use super::{PageError, static_asset, well_known_asset};
    use crate::static_assets;

    #[sqlx::test]
    async fn readiness_tracks_database_availability(pool: sqlx::PgPool) {
        let state = crate::database::AppState { pool };
        assert_eq!(
            super::health(axum::extract::State(state.clone())).await,
            200
        );
        state.pool.close().await;
        assert_eq!(super::health(axum::extract::State(state)).await, 503);
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
        let response = static_asset(Uri::from_static(static_assets::static_url("main.css"))).await;

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
        let response = static_asset(Uri::from_static("/static/theme.invalid.js")).await;

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
