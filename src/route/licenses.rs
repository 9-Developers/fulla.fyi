//! Third-party license notices bundled with the executable.

use askama::Template;
use axum::response::Html;
use tracing::error;

use crate::PageError;

#[derive(Template)]
#[template(path = "licenses.html")]
struct LicensesPage {
    title: &'static str,
    frontend_notices: &'static str,
    rust_notices: &'static str,
}

/// Renders bundled dependency licenses.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn licenses() -> Result<Html<String>, PageError> {
    let page = LicensesPage {
        title: "Open-source licenses",
        frontend_notices: include_str!("../../THIRD_PARTY_NOTICES.md"),
        rust_notices: include_str!("../../licenses/rust.txt"),
    };

    page.render().map(Html).map_err(|err| {
        error!(error = %err, "licenses: template render failed");
        PageError::internal_server_error()
    })
}
