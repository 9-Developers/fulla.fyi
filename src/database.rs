//! Database connections and the embedded schema contract.

use std::{str::FromStr, time::Duration};

use sqlx::{PgPool, Row, migrate::Migrator, postgres::PgConnectOptions, postgres::PgPoolOptions};

pub static MIGRATOR: Migrator = sqlx::migrate!();

/// State shared by HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    /// Bounded `PostgreSQL` connection pool.
    pub pool: PgPool,
}

pub async fn connect() -> Result<PgPool, Box<dyn std::error::Error>> {
    let url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required")?;
    if !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
        return Err("DATABASE_URL must be a PostgreSQL URL".into());
    }
    let options =
        PgConnectOptions::from_str(&url).map_err(|_| "DATABASE_URL must be a PostgreSQL URL")?;
    // Never include connection options or credentials in diagnostics.
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
        .map_err(|_| "could not connect to PostgreSQL".into())
}

/// Reject missing, failed, changed, or unknown migrations without modifying the database.
pub async fn check_schema(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let rows =
        sqlx::query("SELECT version, checksum, success FROM _sqlx_migrations ORDER BY version")
            .fetch_all(pool)
            .await
            .map_err(|_| "migration history unavailable; run fulla_fyi migrate")?;
    let expected: Vec<_> = MIGRATOR
        .iter()
        .filter(|migration| migration.migration_type.is_up_migration())
        .collect();
    if rows.len() != expected.len() {
        return Err("database schema differs from this executable; run matching migrations".into());
    }
    for (row, migration) in rows.iter().zip(expected) {
        if row.try_get::<i64, _>("version")? != migration.version
            || !row.try_get::<bool, _>("success")?
            || row.try_get::<Vec<u8>, _>("checksum")?.as_slice() != migration.checksum.as_ref()
        {
            return Err("database migration history does not match this executable".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MIGRATOR, check_schema};
    use sqlx::PgPool;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    async fn fixture(pool: &PgPool) -> TestResult {
        sqlx::raw_sql(
            "INSERT INTO users (username, email, password_hash) VALUES
                ('owner', 'owner@example.test', 'hash'),
                ('buyer', 'buyer@example.test', 'hash'),
                ('other', 'other@example.test', 'hash');
             INSERT INTO lists (owner_id, name) VALUES (1, 'Birthday');
             INSERT INTO items (list_id, title, retailer_url, desired_quantity)
                VALUES (1, 'Hard drive', 'https://example.test/drive', 4);",
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn rejects(pool: &PgPool, sql: &'static str, code: &str) -> TestResult {
        match sqlx::query(sql).execute(pool).await {
            Err(sqlx::Error::Database(error)) => assert_eq!(error.code().as_deref(), Some(code)),
            result => return Err(format!("expected SQLSTATE {code}, got {result:?}").into()),
        }
        Ok(())
    }

    #[sqlx::test]
    async fn migration_roundtrip_and_history_validation(pool: PgPool) -> TestResult {
        check_schema(&pool).await?;
        MIGRATOR.run(&pool).await?;
        MIGRATOR.undo(&pool, 0).await?;
        assert!(check_schema(&pool).await.is_err());
        MIGRATOR.run(&pool).await?;
        check_schema(&pool).await?;
        sqlx::query("UPDATE _sqlx_migrations SET checksum = decode('00', 'hex')")
            .execute(&pool)
            .await?;
        assert!(check_schema(&pool).await.is_err());
        Ok(())
    }

    #[sqlx::test]
    async fn ownership_membership_and_grant_constraints(pool: PgPool) -> TestResult {
        fixture(&pool).await?;
        sqlx::raw_sql(
            "INSERT INTO groups (owner_id, name) VALUES (1, 'Family'), (2, 'Family');
             INSERT INTO user_groups (group_id, user_id) VALUES (1, 2);
             INSERT INTO list_group_grants (list_id, group_id) VALUES (1, 1);
             INSERT INTO list_user_grants (list_id, user_id) VALUES (1, 3);",
        )
        .execute(&pool)
        .await?;
        rejects(
            &pool,
            "INSERT INTO groups (owner_id, name) VALUES (1, 'family')",
            "23505",
        )
        .await?;
        rejects(
            &pool,
            "INSERT INTO user_groups VALUES (1, 2, now())",
            "23505",
        )
        .await?;
        rejects(
            &pool,
            "INSERT INTO list_user_grants VALUES (1, 999)",
            "23503",
        )
        .await?;
        rejects(
            &pool,
            "INSERT INTO lists (owner_id, name) VALUES (999, 'Invalid')",
            "23503",
        )
        .await?;
        sqlx::query("DELETE FROM groups WHERE id = 1")
            .execute(&pool)
            .await?;
        let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM list_group_grants")
            .fetch_one(&pool)
            .await?;
        assert_eq!(grants, 0);
        Ok(())
    }

    #[sqlx::test]
    async fn media_metadata_and_storage_identity(pool: PgPool) -> TestResult {
        fixture(&pool).await?;
        sqlx::query(
            "INSERT INTO media_assets (owner_id, bucket, object_key)
             VALUES (1, 'sources', 'avatar/one')",
        )
        .execute(&pool)
        .await?;
        for statement in [
            "UPDATE media_assets SET status = 'ready'",
            "UPDATE media_assets SET width = 0",
            "UPDATE media_assets SET height = -1",
            "UPDATE media_assets SET byte_size = 0",
            "UPDATE media_assets SET content_type = 'image/svg+xml'",
            "UPDATE media_assets SET status = 'unknown'",
            "UPDATE media_assets SET object_key = ' '",
        ] {
            rejects(&pool, statement, "23514").await?;
        }
        rejects(
            &pool,
            "INSERT INTO media_assets (owner_id, bucket, object_key)
             VALUES (2, 'sources', 'avatar/one')",
            "23505",
        )
        .await?;
        sqlx::query(
            "UPDATE media_assets SET status = 'ready', content_type = 'image/webp',
             byte_size = 1024, width = 256, height = 256, updated_at = now()",
        )
        .execute(&pool)
        .await?;
        rejects(
            &pool,
            "UPDATE media_assets SET content_type = NULL",
            "23514",
        )
        .await?;
        Ok(())
    }

    #[sqlx::test]
    async fn media_ownership_replacement_and_cleanup(pool: PgPool) -> TestResult {
        fixture(&pool).await?;
        sqlx::raw_sql(
            "INSERT INTO media_assets
                (owner_id, bucket, object_key, status, content_type, byte_size, width, height)
             VALUES
                (1, 'sources', 'one', 'ready', 'image/webp', 1024, 256, 256),
                (1, 'sources', 'two', 'ready', 'image/jpeg', 2048, 1280, 720),
                (2, 'sources', 'three', 'ready', 'image/png', 512, 64, 64);
             INSERT INTO user_avatars (user_id, media_id) VALUES (1, 1);
             INSERT INTO list_media (list_id, owner_id, slot, media_id)
                VALUES (1, 1, 'cover', 2);",
        )
        .execute(&pool)
        .await?;
        for statement in [
            "UPDATE user_avatars SET media_id = 3 WHERE user_id = 1",
            "UPDATE list_media SET media_id = 3",
            "UPDATE list_media SET owner_id = 2, media_id = 3",
            "UPDATE lists SET owner_id = 2 WHERE id = 1",
            "DELETE FROM media_assets WHERE id = 1",
            "DELETE FROM media_assets WHERE id = 2",
        ] {
            rejects(&pool, statement, "23503").await?;
        }
        rejects(
            &pool,
            "INSERT INTO list_media (list_id, owner_id, slot, media_id)
             VALUES (1, 1, 'cover', 1)",
            "23505",
        )
        .await?;
        for statement in [
            "UPDATE user_avatars SET focal_x = -0.1",
            "UPDATE list_media SET focal_y = 1.1",
            "UPDATE list_media SET focal_x = 'NaN'",
            "UPDATE list_media SET slot = ' '",
        ] {
            rejects(&pool, statement, "23514").await?;
        }
        sqlx::raw_sql(
            "UPDATE user_avatars SET media_id = 2 WHERE user_id = 1;
             DELETE FROM media_assets WHERE id = 1;
             DELETE FROM items WHERE list_id = 1;
             DELETE FROM lists WHERE id = 1;",
        )
        .execute(&pool)
        .await?;
        let usages: i64 = sqlx::query_scalar("SELECT count(*) FROM list_media")
            .fetch_one(&pool)
            .await?;
        assert_eq!(usages, 0);
        // Removing a list usage must preserve a source still used by an avatar.
        rejects(&pool, "DELETE FROM media_assets WHERE id = 2", "23503").await?;
        sqlx::raw_sql(
            "DELETE FROM user_avatars WHERE user_id = 1;
             UPDATE media_assets SET status = 'deleting' WHERE id = 2;
             DELETE FROM media_assets WHERE id = 2;",
        )
        .execute(&pool)
        .await?;
        Ok(())
    }

    #[sqlx::test]
    async fn quantities_attribution_and_idempotency(pool: PgPool) -> TestResult {
        fixture(&pool).await?;
        rejects(&pool, "UPDATE items SET desired_quantity = 0", "23514").await?;
        rejects(
            &pool,
            "UPDATE items SET retailer_url = 'javascript:alert(1)'",
            "23514",
        )
        .await?;
        // Independent concurrent purchasers must both retain their records.
        let (first, second) = tokio::join!(
            sqlx::query("INSERT INTO purchases (item_id, purchaser_id, quantity, submission_key) VALUES (1, 2, 2, 'first')").execute(&pool),
            sqlx::query("INSERT INTO purchases (item_id, purchaser_id, quantity, submission_key) VALUES (1, 3, 1, 'second')").execute(&pool),
        );
        first?;
        second?;
        rejects(&pool, "INSERT INTO purchases (item_id, purchaser_id, quantity, submission_key) VALUES (1, 2, 2, 'first')", "23505").await?;
        rejects(&pool, "UPDATE purchases SET quantity = -1", "23514").await?;
        sqlx::raw_sql(
            "UPDATE purchases SET quantity = 1 WHERE purchaser_id = 2;
             UPDATE purchases SET canceled_at = now() WHERE purchaser_id = 3;
             UPDATE items SET desired_quantity = 1, archived_at = now();",
        )
        .execute(&pool)
        .await?;
        let purchased: i64 = sqlx::query_scalar(
            "SELECT sum(quantity)::bigint FROM purchases WHERE item_id = 1 AND canceled_at IS NULL",
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(purchased, 1);
        let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM purchases")
            .fetch_one(&pool)
            .await?;
        assert_eq!(retained, 2);
        rejects(&pool, "DELETE FROM items WHERE id = 1", "23503").await?;
        Ok(())
    }
}
