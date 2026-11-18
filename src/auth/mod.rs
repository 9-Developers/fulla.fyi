//! Account authentication and shared abuse controls.

use std::{fmt, sync::Arc};

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum_login::{AuthUser, AuthnBackend, UserId};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::sync::Semaphore;

pub mod store;
#[cfg(test)]
mod tests;

pub type AuthSession = axum_login::AuthSession<Backend>;

#[derive(Clone, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    password_hash: String,
}

impl fmt::Debug for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl AuthUser for User {
    type Id = i64;

    fn id(&self) -> i64 {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

pub struct Credentials {
    pub identifier: String,
    pub password: String,
}

#[derive(Clone)]
pub struct Backend {
    pub pool: PgPool,
    work: Arc<Semaphore>,
    dummy_hash: Arc<String>,
}

impl Backend {
    pub async fn new(pool: PgPool) -> Result<Self, AuthError> {
        let dummy_hash = tokio::task::spawn_blocking(|| hash_password(&random_token()))
            .await
            .map_err(|_| AuthError)??;
        Ok(Self {
            pool,
            work: Arc::new(Semaphore::new(4)),
            dummy_hash: Arc::new(dummy_hash),
        })
    }

    pub async fn register(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<Option<User>, AuthError> {
        let permit = self
            .work
            .clone()
            .try_acquire_owned()
            .map_err(|_| AuthError)?;
        let password_hash = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            hash_password(&password)
        })
        .await
        .map_err(|_| AuthError)??;
        // Both case-insensitive unique indexes arbitrate concurrent registrations.
        sqlx::query_as(
            "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING RETURNING id, username, password_hash",
        )
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError)
    }

    pub async fn allow_attempt(&self, key: &str, maximum: i32) -> Result<bool, AuthError> {
        // Avoid storing raw account identifiers in the limiter.
        let key = hex_bytes(&Sha256::digest(key.as_bytes()));
        let attempts: i32 = sqlx::query_scalar(
            "INSERT INTO auth_rate_limits (key, attempts, expires_at)
             VALUES ($1, 1, now() + interval '15 minutes')
             ON CONFLICT (key) DO UPDATE SET
               attempts = CASE WHEN auth_rate_limits.expires_at <= now() THEN 1
                               ELSE least(auth_rate_limits.attempts + 1, 10000) END,
               expires_at = CASE WHEN auth_rate_limits.expires_at <= now()
                                 THEN now() + interval '15 minutes'
                                 ELSE auth_rate_limits.expires_at END
             RETURNING attempts",
        )
        .bind(key)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| AuthError)?;
        Ok(attempts <= maximum)
    }
}

impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = AuthError;

    async fn authenticate(&self, credentials: Credentials) -> Result<Option<User>, AuthError> {
        let user: Option<User> = sqlx::query_as(
            "SELECT id, username, password_hash FROM users
             WHERE lower(username) = $1 OR lower(email) = $1",
        )
        .bind(credentials.identifier)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError)?;
        let hash = user.as_ref().map_or_else(
            || (*self.dummy_hash).clone(),
            |user| user.password_hash.clone(),
        );
        let permit = self
            .work
            .clone()
            .try_acquire_owned()
            .map_err(|_| AuthError)?;
        let valid = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let parsed = PasswordHash::new(&hash).map_err(|_| AuthError)?;
            Ok::<_, AuthError>(
                Argon2::default()
                    .verify_password(credentials.password.as_bytes(), &parsed)
                    .is_ok(),
            )
        })
        .await
        .map_err(|_| AuthError)??;
        Ok(user.filter(|_| valid))
    }

    async fn get_user(&self, id: &UserId<Self>) -> Result<Option<User>, AuthError> {
        sqlx::query_as("SELECT id, username, password_hash FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| AuthError)
    }
}

#[derive(Debug)]
pub struct AuthError;

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("authentication service unavailable")
    }
}

impl std::error::Error for AuthError {}

fn hash_password(password: &str) -> Result<String, AuthError> {
    Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|hash| hash.to_string())
        .map_err(|_| AuthError)
}

pub fn random_token() -> String {
    use argon2::password_hash::rand_core::RngCore;
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex_bytes(&bytes)
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut token, byte| {
            use std::fmt::Write;
            let _ = write!(token, "{byte:02x}");
            token
        })
}

/// Shared policy for password-setting flows; never used to reject an existing login.
pub fn password_policy(password: &str, username: &str, email: &str) -> Result<(), &'static str> {
    if !(12..=128).contains(&password.len()) {
        return Err("Use a password between 12 and 128 UTF-8 bytes. Spaces are welcome.");
    }
    let local = email.split('@').next().unwrap_or_default();
    let estimate = zxcvbn::zxcvbn(password, &[username, email, local]);
    if estimate.score() < zxcvbn::Score::Three {
        return Err(
            "Password strength is below 3 of 4. Try several unrelated words; avoid your username, email, common phrases, and repeated patterns.",
        );
    }
    Ok(())
}

pub async fn cleanup(pool: PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
    loop {
        interval.tick().await;
        for statement in [
            "DELETE FROM auth_sessions WHERE expires_at <= now()",
            "DELETE FROM auth_rate_limits WHERE expires_at <= now()",
        ] {
            if sqlx::query(statement).execute(&pool).await.is_err() {
                tracing::warn!("expired authentication record cleanup failed");
            }
        }
    }
}
