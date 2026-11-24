//! Route handlers.

use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};

use crate::database::AppState;
use crate::static_assets;
use axum_login::tower_sessions;
use nine_dev_site_lib::serving::{self, content_type};

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
    Ok(accounts
        .route("/", get(home))
        .route("/licenses", get(licenses))
        .route("/health", get(health))
        .fallback(not_found)
        .with_state(state)
        .merge(asset_routes()))
}

fn asset_routes() -> Router {
    serving::asset_router(&static_assets::CATALOG, static_assets::bytes)
        .route("/.well-known/{*path}", get(well_known_asset))
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
    let Some(bytes) = static_assets::bytes(&source_path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    serving::response(bytes, content_type(&source_path), "public, max-age=3600")
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        extract::Path,
        http::{HeaderValue, Request, header},
        response::{IntoResponse, Response},
    };

    use super::{PageError, well_known_asset};
    use crate::static_assets;
    use tower::ServiceExt;

    pub(super) async fn get_asset(path: &str) -> Result<Response, Box<dyn std::error::Error>> {
        Ok(super::asset_routes()
            .oneshot(Request::get(path).body(Body::empty())?)
            .await?)
    }

    #[tokio::test]
    async fn manifest_revalidates_at_the_url_in_the_page() -> Result<(), Box<dyn std::error::Error>>
    {
        let app = super::asset_routes();
        let path = "/site.webmanifest";
        let page = super::home()
            .await
            .map_err(|_| "home page did not render")?;
        assert!(
            page.0
                .contains(r#"<link rel="manifest" href="/site.webmanifest">"#)
        );
        let response = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), 200);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "application/manifest+json"
        );
        let cache = "public, max-age=0, must-revalidate";
        assert_eq!(response.headers()[header::CACHE_CONTROL], cache);
        let etag = response.headers()[header::ETAG].to_str()?.to_owned();
        let file = include_bytes!("../../public/site.webmanifest");
        assert_eq!(to_bytes(response.into_body(), 4096).await?.as_ref(), file);
        for (condition, expected) in [
            (etag.clone(), 304),
            (format!("\"old,tag\", W/{etag}"), 304),
            ("*".into(), 304),
            ("\"stale\"".into(), 200),
        ] {
            let request = Request::get(path)
                .header(header::IF_NONE_MATCH, condition)
                .body(Body::empty())?;
            let response = app.clone().oneshot(request).await?;
            assert_eq!(response.status(), expected);
            assert_eq!(response.headers()[header::ETAG], etag);
            assert_eq!(response.headers()[header::CACHE_CONTROL], cache);
            let body = to_bytes(response.into_body(), 4096).await?;
            assert_eq!(
                body.as_ref(),
                if expected == 304 {
                    &[]
                } else {
                    file.as_slice()
                }
            );
        }
        Ok(())
    }

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

    #[test]
    fn compiled_css_resolves_palette_utilities_from_templates_and_apply() {
        // This utility is used in error.html; @apply alone does not emit its class.
        let css = include_str!("../../public/main.css");
        assert!(css.contains(".text-ctp-subtext1{color:var(--ctp-subtext1)"));
        assert!(css.contains("background-color:var(--ctp-base)"));
    }

    #[tokio::test]
    async fn fingerprinted_static_asset_is_cached_immutably()
    -> Result<(), Box<dyn std::error::Error>> {
        let response = get_asset(static_assets::static_url("main.css")).await?;

        assert_eq!(response.status(), 200);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&HeaderValue::from_static(
                "public, max-age=31536000, immutable"
            ))
        );
        assert!(response.headers().contains_key(header::CONTENT_TYPE));
        Ok(())
    }

    #[tokio::test]
    async fn unknown_static_asset_is_not_cached() -> Result<(), Box<dyn std::error::Error>> {
        let response = get_asset("/static/theme.invalid.js").await?;

        assert_eq!(response.status(), 404);
        assert!(!response.headers().contains_key(header::CACHE_CONTROL));
        Ok(())
    }

    #[tokio::test]
    async fn well_known_asset_rejects_parent_segments() {
        let response = well_known_asset(Path("../theme.js".to_string())).await;

        assert_eq!(response.status(), 404);
        assert!(!response.headers().contains_key(header::CACHE_CONTROL));
    }
}
