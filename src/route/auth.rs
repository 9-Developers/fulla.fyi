//! Server-rendered account forms. No passwords are returned in HTML or diagnostics.

use askama::Template;
use axum::{
    Form,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_login::tower_sessions::{self, Expiry, Session};
use serde::Deserialize;
use subtle::ConstantTimeEq;

use crate::auth::{AuthSession, Credentials, password_policy, random_token};

#[derive(Template)]
#[template(path = "auth.html")]
struct AccountPage {
    title: &'static str,
    mode: &'static str,
    csrf: String,
    error: String,
    username: String,
    email: String,
    identifier: String,
}

#[derive(Deserialize)]
pub struct AccountForm {
    csrf: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    email: String,
    #[serde(default)]
    identifier: String,
    #[serde(default)]
    password: String,
}

pub async fn register_page(auth: AuthSession) -> Response {
    if auth.user.is_some() {
        return Redirect::to("/account").into_response();
    }
    page(&auth.session, "register", "", None, StatusCode::OK).await
}

pub async fn login_page(auth: AuthSession) -> Response {
    if auth.user.is_some() {
        return Redirect::to("/account").into_response();
    }
    page(&auth.session, "login", "", None, StatusCode::OK).await
}

pub async fn account(
    auth: AuthSession,
    axum_htmx::HxRequest(htmx): axum_htmx::HxRequest,
) -> Response {
    let Some(user) = auth.user else {
        // HTMX must navigate rather than swapping a login page into a private view.
        return if htmx {
            (axum_htmx::HxRedirect("/login".to_owned()), StatusCode::OK).into_response()
        } else {
            Redirect::to("/login").into_response()
        };
    };
    let form = AccountForm {
        csrf: String::new(),
        username: user.username,
        email: String::new(),
        identifier: String::new(),
        password: String::new(),
    };
    page(&auth.session, "account", "", Some(&form), StatusCode::OK).await
}

pub async fn logout_page(auth: AuthSession) -> Response {
    if auth.user.is_none() {
        return Redirect::to("/login").into_response();
    }
    page(&auth.session, "logout", "", None, StatusCode::OK).await
}

pub async fn register(mut auth: AuthSession, Form(mut form): Form<AccountForm>) -> Response {
    if !valid_csrf(&auth.session, &form.csrf).await {
        return forbidden();
    }
    if auth.user.is_some() {
        return Redirect::to("/account").into_response();
    }
    match auth.backend.allow_attempt("register:global", 30).await {
        Ok(true) => {}
        Ok(false) => return limited(),
        Err(_) => return unavailable(),
    }
    form.username = form.username.trim().to_owned();
    form.email = form.email.trim().to_ascii_lowercase();
    if let Err(message) = validate_registration(&form) {
        return page(
            &auth.session,
            "register",
            message,
            Some(&form),
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await;
    }
    let password = std::mem::take(&mut form.password);
    match auth
        .backend
        .register(form.username.clone(), form.email.clone(), password)
        .await
    {
        Ok(Some(user)) => {
            if auth.login(&user).await.is_err() || rotate_csrf(&auth.session).await.is_err() {
                return unavailable();
            }
            auth.session.set_expiry(Some(Expiry::AtDateTime(
                time::OffsetDateTime::now_utc() + time::Duration::days(7),
            )));
            Redirect::to("/account").into_response()
        }
        Ok(None) => {
            page(
                &auth.session,
                "register",
                "That username or email is already registered. Choose another or sign in.",
                Some(&form),
                StatusCode::CONFLICT,
            )
            .await
        }
        Err(_) => unavailable(),
    }
}

pub async fn login(mut auth: AuthSession, Form(mut form): Form<AccountForm>) -> Response {
    if !valid_csrf(&auth.session, &form.csrf).await {
        return forbidden();
    }
    if auth.user.is_some() {
        return Redirect::to("/account").into_response();
    }
    form.identifier = form.identifier.trim().to_ascii_lowercase();
    if form.identifier.len() > 254 || form.password.is_empty() || form.password.len() > 128 {
        return invalid_login(&auth.session, &form).await;
    }
    // Shared limits work behind Railway without trusting spoofable forwarded headers.
    for (key, maximum) in [
        ("login:global".to_owned(), 100),
        (format!("login:{}", form.identifier), 10),
    ] {
        match auth.backend.allow_attempt(&key, maximum).await {
            Ok(true) => {}
            Ok(false) => return limited(),
            Err(_) => return unavailable(),
        }
    }
    let credentials = Credentials {
        identifier: form.identifier.clone(),
        password: std::mem::take(&mut form.password),
    };
    match auth.authenticate(credentials).await {
        Ok(Some(user)) => {
            if auth.login(&user).await.is_err() || rotate_csrf(&auth.session).await.is_err() {
                return unavailable();
            }
            auth.session.set_expiry(Some(Expiry::AtDateTime(
                time::OffsetDateTime::now_utc() + time::Duration::days(7),
            )));
            Redirect::to("/account").into_response()
        }
        Ok(None) => invalid_login(&auth.session, &form).await,
        Err(_) => unavailable(),
    }
}

pub async fn logout(mut auth: AuthSession, Form(form): Form<AccountForm>) -> Response {
    if !valid_csrf(&auth.session, &form.csrf).await {
        return forbidden();
    }
    if auth.logout().await.is_err() {
        return unavailable();
    }
    Redirect::to("/login").into_response()
}

fn validate_registration(form: &AccountForm) -> Result<(), &'static str> {
    if !(3..=32).contains(&form.username.len())
        || !form
            .username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err("Use a username of 3–32 letters, numbers, underscores, or hyphens.");
    }
    let Some((local, domain)) = form.email.split_once('@') else {
        return Err("Enter a valid email address.");
    };
    if form.email.len() > 254
        || local.is_empty()
        || local.len() > 64
        || !local
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".!#$%&'*+-/=?^_`{|}~".contains(&b))
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
        || !domain.contains('.')
        || domain.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    {
        return Err("Enter a valid email address.");
    }
    password_policy(
        &form.password,
        &form.username.to_ascii_lowercase(),
        &form.email,
    )
}

async fn invalid_login(session: &Session, form: &AccountForm) -> Response {
    page(
        session,
        "login",
        "Invalid username/email or password.",
        Some(form),
        StatusCode::UNAUTHORIZED,
    )
    .await
}

async fn valid_csrf(session: &Session, supplied: &str) -> bool {
    match session.get::<String>("csrf").await {
        Ok(Some(expected)) if supplied.len() == 64 => {
            bool::from(expected.as_bytes().ct_eq(supplied.as_bytes()))
        }
        _ => false,
    }
}

async fn rotate_csrf(session: &Session) -> Result<String, tower_sessions::session::Error> {
    let token = random_token();
    session.insert("csrf", &token).await?;
    Ok(token)
}

async fn page(
    session: &Session,
    mode: &'static str,
    error: &str,
    form: Option<&AccountForm>,
    status: StatusCode,
) -> Response {
    let token = match session.get::<String>("csrf").await {
        Ok(Some(token)) => token,
        Ok(None) => match rotate_csrf(session).await {
            Ok(token) => token,
            Err(_) => return unavailable(),
        },
        Err(_) => return unavailable(),
    };
    let title = match mode {
        "register" => "Create account",
        "login" => "Sign in",
        "logout" => "Sign out",
        _ => "Your account",
    };
    let page = AccountPage {
        title,
        mode,
        csrf: token,
        error: error.to_owned(),
        username: form.map_or_else(String::new, |form| form.username.clone()),
        email: form.map_or_else(String::new, |form| form.email.clone()),
        identifier: form.map_or_else(String::new, |form| form.identifier.clone()),
    };
    page.render().map_or_else(
        |_| unavailable(),
        |html| (status, Html(html)).into_response(),
    )
}

fn forbidden() -> Response {
    (
        StatusCode::FORBIDDEN,
        "This form has expired. Reload the page and try again.",
    )
        .into_response()
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "Account services are temporarily unavailable. Please try again.",
    )
        .into_response()
}

fn limited() -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(header::RETRY_AFTER, "900")],
        "Too many attempts. Please wait 15 minutes and try again.",
    )
        .into_response()
}
