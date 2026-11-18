//! `SQLx` session storage using the application's migration and pool lifecycle.

use async_trait::async_trait;
use axum_login::tower_sessions::{
    SessionStore,
    session::{Id, Record},
    session_store,
};
use sqlx::{PgPool, Row};

#[derive(Clone, Debug)]
pub struct Store(pub PgPool);

fn storage_error(_: impl std::fmt::Display) -> session_store::Error {
    session_store::Error::Backend("session storage unavailable".into())
}

#[async_trait]
impl SessionStore for Store {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        loop {
            let inserted = sqlx::query(
                "INSERT INTO auth_sessions (id, data, expires_at) VALUES ($1, $2, $3)
                 ON CONFLICT (id) DO NOTHING",
            )
            .bind(record.id.to_string())
            .bind(sqlx::types::Json(&record.data))
            .bind(record.expiry_date)
            .execute(&self.0)
            .await
            .map_err(storage_error)?;
            if inserted.rows_affected() == 1 {
                return Ok(());
            }
            record.id = Id::default();
        }
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        // Never recreate a session deleted by concurrent logout or ID rotation.
        sqlx::query(
            "UPDATE auth_sessions SET data = $2, expires_at = $3
             WHERE id = $1 AND expires_at > now()",
        )
        .bind(record.id.to_string())
        .bind(sqlx::types::Json(&record.data))
        .bind(record.expiry_date)
        .execute(&self.0)
        .await
        .map_err(storage_error)?;
        Ok(())
    }

    async fn load(&self, id: &Id) -> session_store::Result<Option<Record>> {
        let row = sqlx::query(
            "SELECT data, expires_at FROM auth_sessions WHERE id = $1 AND expires_at > now()",
        )
        .bind(id.to_string())
        .fetch_optional(&self.0)
        .await
        .map_err(storage_error)?;
        row.map(|row| {
            Ok(Record {
                id: *id,
                data: row
                    .try_get::<sqlx::types::Json<_>, _>("data")
                    .map_err(storage_error)?
                    .0,
                expiry_date: row.try_get("expires_at").map_err(storage_error)?,
            })
        })
        .transpose()
    }

    async fn delete(&self, id: &Id) -> session_store::Result<()> {
        sqlx::query("DELETE FROM auth_sessions WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.0)
            .await
            .map_err(storage_error)?;
        Ok(())
    }
}
