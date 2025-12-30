use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::Result;
use crate::models::Session;

#[derive(Clone)]
pub struct SessionRepository {
    pool: Arc<SqlitePool>,
}

impl SessionRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub async fn create(&self, user_id: Uuid, token: &str) -> Result<Session> {
        let id = Uuid::new_v4();
        let expires_at = Utc::now() + Duration::days(7);

        let session = sqlx::query_as::<_, Session>(
            r#"
            INSERT INTO sessions (id, user_id, token, expires_at, created_at)
            VALUES ($1, $2, $3, $4, datetime('now'))
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(token)
        .bind(expires_at)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(session)
    }

    pub async fn find_by_token(&self, token: &str) -> Result<Option<Session>> {
        let session = sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE token = $1 AND expires_at > datetime('now')",
        )
        .bind(token)
        .fetch_optional(self.pool.as_ref())
        .await?;

        Ok(session)
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(id)
            .execute(self.pool.as_ref())
            .await?;

        Ok(())
    }

    pub async fn delete_by_token(&self, token: &str) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE token = $1")
            .bind(token)
            .execute(self.pool.as_ref())
            .await?;

        Ok(())
    }

    pub async fn delete_expired(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM sessions WHERE expires_at <= datetime('now')")
            .execute(self.pool.as_ref())
            .await?;

        Ok(result.rows_affected())
    }
}
