use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    response::Response,
};
use axum_login::tower_sessions::{
    SessionStore,
    session::{Id, Record},
};
use sqlx::PgPool;
use tower::ServiceExt;

use super::{Backend, password_policy, store::Store};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const PASSWORD: &str = "orchid telescope velvet glacier 47";

async fn app(pool: &PgPool) -> Result<Router, super::AuthError> {
    crate::route::get_routes(crate::database::AppState { pool: pool.clone() }, false).await
}

async fn request(
    app: &Router,
    path: &str,
    cookie: &str,
    form: Option<&[(&str, &str)]>,
    htmx: bool,
) -> Result<Response, Box<dyn std::error::Error>> {
    let mut builder = Request::builder().uri(path).header(header::COOKIE, cookie);
    if htmx {
        builder = builder.header("hx-request", "true");
    }
    let body = if let Some(form) = form {
        builder = builder
            .method("POST")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
        Body::from(serde_urlencoded::to_string(form)?)
    } else {
        Body::empty()
    };
    Ok(app.clone().oneshot(builder.body(body)?).await?)
}

async fn body(response: Response) -> Result<String, Box<dyn std::error::Error>> {
    Ok(String::from_utf8(
        to_bytes(response.into_body(), 1024 * 1024).await?.to_vec(),
    )?)
}

fn cookie(response: &Response) -> Result<String, Box<dyn std::error::Error>> {
    Ok(response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("missing cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("empty cookie")?
        .to_owned())
}

fn csrf(html: &str) -> Result<&str, Box<dyn std::error::Error>> {
    Ok(html
        .split("name=\"csrf\" value=\"")
        .nth(1)
        .ok_or("missing CSRF token")?
        .split('"')
        .next()
        .ok_or("missing token value")?)
}

async fn anonymous(
    app: &Router,
    path: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let response = request(app, path, "", None, false).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let cookie = cookie(&response)?;
    let html = body(response).await?;
    Ok((cookie, csrf(&html)?.to_owned()))
}

#[sqlx::test]
async fn browser_policy_covers_pages_assets_errors_and_accounts(pool: PgPool) -> TestResult {
    let app = app(&pool).await?;
    for (path, status) in [
        ("/", 200),
        ("/site.webmanifest", 200),
        ("/missing", 404),
        ("/login", 200),
    ] {
        let response = request(&app, path, "", None, false).await?;
        assert_eq!(response.status(), status);
        let headers = response.headers();
        assert!(
            headers[header::CONTENT_SECURITY_POLICY]
                .to_str()?
                .contains("frame-ancestors 'none'")
        );
        assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert_eq!(headers[header::X_FRAME_OPTIONS], "DENY");
        let policy = if path == "/login" {
            "same-origin"
        } else {
            "strict-origin-when-cross-origin"
        };
        assert_eq!(headers[header::REFERRER_POLICY], policy);
    }
    Ok(())
}

fn registration(token: &str) -> [(&str, &str); 4] {
    [
        ("csrf", token),
        ("username", "Alice"),
        ("email", "alice@example.test"),
        ("password", PASSWORD),
    ]
}

#[sqlx::test]
async fn registration_persistence_login_rotation_and_logout(pool: PgPool) -> TestResult {
    let router = app(&pool).await?;
    let response = request(&router, "/account", "", None, false).await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers()[header::LOCATION], "/login");
    let response = request(&router, "/account", "", None, true).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["hx-redirect"], "/login");

    let (anonymous_cookie, token) = anonymous(&router, "/register").await?;
    let response = request(
        &router,
        "/register",
        &anonymous_cookie,
        Some(&registration(&token)),
        false,
    )
    .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let signed_in_cookie = cookie(&response)?;
    assert_ne!(signed_in_cookie, anonymous_cookie);
    let cookie_header = response.headers()[header::SET_COOKIE].to_str()?;
    assert!(cookie_header.contains("HttpOnly"));
    assert!(cookie_header.contains("SameSite=Lax"));
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users")
        .fetch_one(&pool)
        .await?;
    assert!(hash.starts_with("$argon2id$"));
    assert!(!hash.contains(PASSWORD));

    // A fresh router/backend simulates a process restart using the same database.
    let router = app(&pool).await?;
    let response = request(&router, "/account", &signed_in_cookie, None, false).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = body(response).await?;
    assert!(html.contains("Welcome, Alice"));
    let new_token = csrf(&html)?.to_owned();
    assert_ne!(token, new_token);
    assert_eq!(
        request(&router, "/account", &anonymous_cookie, None, false)
            .await?
            .status(),
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        request(
            &router,
            "/logout",
            &signed_in_cookie,
            Some(&[("csrf", &token)]),
            false
        )
        .await?
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(&router, "/logout", &signed_in_cookie, None, false)
            .await?
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        request(&router, "/account", &signed_in_cookie, None, false)
            .await?
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        request(
            &router,
            "/logout",
            &signed_in_cookie,
            Some(&[("csrf", &new_token)]),
            false
        )
        .await?
        .status(),
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        request(&router, "/account", &signed_in_cookie, None, false)
            .await?
            .status(),
        StatusCode::SEE_OTHER
    );

    let (login_cookie, token) = anonymous(&router, "/login").await?;
    for identifier in ["nobody", "ALICE"] {
        let response = request(
            &router,
            "/login",
            &login_cookie,
            Some(&[
                ("csrf", &token),
                ("identifier", identifier),
                ("password", "wrong password"),
            ]),
            false,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let html = body(response).await?;
        assert!(html.contains("Invalid username/email or password."));
        assert!(!html.contains("wrong password"));
    }
    let response = request(
        &router,
        "/login",
        &login_cookie,
        Some(&[
            ("csrf", &token),
            ("identifier", " ALICE@EXAMPLE.TEST "),
            ("password", PASSWORD),
        ]),
        false,
    )
    .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let login_cookie = cookie(&response)?;
    // A password change invalidates existing sessions via axum-login's auth hash.
    sqlx::query("UPDATE users SET password_hash = 'changed'")
        .execute(&pool)
        .await?;
    assert_eq!(
        request(&router, "/account", &login_cookie, None, false)
            .await?
            .status(),
        StatusCode::SEE_OTHER
    );
    Ok(())
}

#[sqlx::test]
async fn validation_csrf_duplicates_and_limits(pool: PgPool) -> TestResult {
    let router = app(&pool).await?;
    let (cookie, token) = anonymous(&router, "/register").await?;
    assert_eq!(
        request(
            &router,
            "/register",
            &cookie,
            Some(&registration("bad")),
            false
        )
        .await?
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(&router, "/register", "", Some(&registration(&token)), true)
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    for (field, value) in [
        ("username", "<script>"),
        ("email", "not-an-email"),
        ("password", "passwordpassword"),
    ] {
        let mut form = registration(&token);
        for (name, entry) in &mut form {
            if *name == field {
                *entry = value;
            }
        }
        let response = request(&router, "/register", &cookie, Some(&form), false).await?;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let html = body(response).await?;
        assert!(!html.contains("<script>"));
        assert!(!html.contains("passwordpassword"));
    }
    assert_eq!(
        request(
            &router,
            "/register",
            &cookie,
            Some(&registration(&token)),
            false
        )
        .await?
        .status(),
        StatusCode::SEE_OTHER
    );
    let (cookie, token) = anonymous(&router, "/register").await?;
    for (username, email) in [
        ("aLiCe", "other@example.test"),
        ("other", "ALICE@EXAMPLE.TEST"),
    ] {
        let response = request(
            &router,
            "/register",
            &cookie,
            Some(&[
                ("csrf", &token),
                ("username", username),
                ("email", email),
                ("password", PASSWORD),
            ]),
            false,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);
    let oversized = "a".repeat(20_000);
    assert_eq!(
        request(
            &router,
            "/login",
            &cookie,
            Some(&[("password", &oversized)]),
            false
        )
        .await?
        .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let backend = Backend::new(pool.clone()).await?;
    for _ in 0..10 {
        assert!(backend.allow_attempt("login:alice", 10).await?);
    }
    let response = request(
        &router,
        "/login",
        &cookie,
        Some(&[
            ("csrf", &token),
            ("identifier", "Alice"),
            ("password", PASSWORD),
        ]),
        false,
    )
    .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers()[header::RETRY_AFTER], "900");
    sqlx::query("UPDATE auth_rate_limits SET expires_at = now() - interval '1 second'")
        .execute(&pool)
        .await?;
    assert!(backend.allow_attempt("login:alice", 10).await?);
    Ok(())
}

#[sqlx::test]
async fn secure_cookie_and_expired_sessions(pool: PgPool) -> TestResult {
    let router =
        crate::route::get_routes(crate::database::AppState { pool: pool.clone() }, true).await?;
    let response = request(&router, "/login", "", None, false).await?;
    let header = response.headers()[header::SET_COOKIE].to_str()?;
    assert!(header.starts_with("__Host-fulla_session="));
    assert!(header.contains("; Secure"));
    assert!(header.contains("Path=/"));
    assert!(!header.contains("Domain="));
    let router = app(&pool).await?;
    let (cookie, token) = anonymous(&router, "/register").await?;
    let response = request(
        &router,
        "/register",
        &cookie,
        Some(&registration(&token)),
        false,
    )
    .await?;
    let cookie = super::tests::cookie(&response)?;
    sqlx::query("UPDATE auth_sessions SET expires_at = now() - interval '1 second'")
        .execute(&pool)
        .await?;
    assert_eq!(
        request(&router, "/account", &cookie, None, false)
            .await?
            .status(),
        StatusCode::SEE_OTHER
    );
    Ok(())
}

#[sqlx::test]
async fn concurrent_limits_and_stale_session_writes(pool: PgPool) -> TestResult {
    let backend = Backend::new(pool.clone()).await?;
    let (first, second) = tokio::join!(
        backend.allow_attempt("concurrent", 1),
        backend.allow_attempt("concurrent", 1),
    );
    assert_ne!(first?, second?);
    let store = Store(pool);
    let mut record = Record {
        id: Id::default(),
        data: std::collections::HashMap::default(),
        expiry_date: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
    };
    store.create(&mut record).await?;
    let mut collision = record.clone();
    store.create(&mut collision).await?;
    assert_ne!(record.id, collision.id);
    store.delete(&record.id).await?;
    store.save(&record).await?;
    assert!(store.load(&record.id).await?.is_none());
    Ok(())
}

#[test]
fn password_setting_policy_supports_passphrases_and_bounds() {
    assert!(password_policy(PASSWORD, "alice", "alice@example.test").is_ok());
    assert!(password_policy("vQ7!mR2#zT8$", "alice", "alice@example.test").is_ok());
    let boundary = format!("{PASSWORD}{}", "x".repeat(128 - PASSWORD.len()));
    assert!(password_policy(&boundary, "alice", "alice@example.test").is_ok());
    assert!(password_policy("passwordpassword", "alice", "alice@example.test").is_err());
    assert!(password_policy("AliceAliceAlice!", "alice", "alice@example.test").is_err());
    for password in ["short".to_owned(), "x".repeat(129), "🦀".repeat(33)] {
        assert!(password_policy(&password, "alice", "alice@example.test").is_err());
    }
}

#[sqlx::test]
async fn existing_passwords_are_not_subject_to_registration_policy(pool: PgPool) -> TestResult {
    let backend = Backend::new(pool.clone()).await?;
    backend
        .register(
            "legacy".into(),
            "legacy@example.test".into(),
            " old ".into(),
        )
        .await?;
    let router = app(&pool).await?;
    let (cookie, token) = anonymous(&router, "/login").await?;
    for (password, status) in [
        ("old", StatusCode::UNAUTHORIZED),
        (" old ", StatusCode::SEE_OTHER),
    ] {
        assert_eq!(
            request(
                &router,
                "/login",
                &cookie,
                Some(&[
                    ("csrf", &token),
                    ("identifier", "legacy"),
                    ("password", password),
                ]),
                false
            )
            .await?
            .status(),
            status
        );
    }
    Ok(())
}
