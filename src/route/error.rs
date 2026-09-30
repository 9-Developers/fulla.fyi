//! Error route handler.

use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorPage<'a> {
    title: &'a str,
    status_code: u16,
    heading: &'a str,
    message: &'a str,
}

/// HTTP error that renders as a site-styled page.
pub struct PageError {
    status: StatusCode,
}

impl PageError {
    /// Creates a `404 Not Found` page error.
    #[must_use]
    pub const fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
        }
    }

    /// Creates a `500 Internal Server Error` page error.
    #[must_use]
    pub const fn internal_server_error() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for PageError {
    fn into_response(self) -> Response {
        let (title, heading, message) = match self.status {
            StatusCode::NOT_FOUND => (
                "404 Not Found",
                "Page not found",
                "The page you were looking for does not exist or has moved.",
            ),
            StatusCode::INTERNAL_SERVER_ERROR => (
                "500 Internal Server Error",
                "Something went wrong",
                "The server ran into a problem while rendering this page.",
            ),
            _ => (
                "Error",
                "Something went wrong",
                "The server could not complete this request.",
            ),
        };
        let page = ErrorPage {
            title,
            status_code: self.status.as_u16(),
            heading,
            message,
        };

        page.render().map_or_else(
            |error| {
                tracing::error!(%error, "error page: template render failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            },
            |html| (self.status, Html(html)).into_response(),
        )
    }
}

/// Renders the fallback page for unmatched routes.
pub async fn not_found() -> PageError {
    PageError::not_found()
}
