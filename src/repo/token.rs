use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::{ApiToken, TokenScope};

#[derive(Clone)]
pub struct ApiTokenRepository {
    pool: Arc<SqlitePool>,
}

impl ApiTokenRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        name: &str,
        token_hash: &str,
        scope: TokenScope,
        expires_in_days: Option<i64>,
    ) -> Result<ApiToken> {
        let id = Uuid::new_v4();
        let expires_at = expires_in_days.map(|days| Utc::now() + Duration::days(days));

        let token = sqlx::query_as::<_, ApiToken>(
            r#"
            INSERT INTO api_tokens (id, user_id, name, token_hash, scope, expires_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, datetime('now'))
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(name)
        .bind(token_hash)
        .bind(scope.to_string())
        .bind(expires_at)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(token)
    }

    pub async fn find_by_hash(&self, token_hash: &str) -> Result<Option<ApiToken>> {
        let token = sqlx::query_as::<_, ApiToken>(
            "SELECT * FROM api_tokens WHERE token_hash = $1 AND (expires_at IS NULL OR expires_at > datetime('now'))",
        )
        .bind(token_hash)
        .fetch_optional(self.pool.as_ref())
        .await?;

        Ok(token)
    }

    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<ApiToken>> {
        let tokens = sqlx::query_as::<_, ApiToken>(
            "SELECT * FROM api_tokens WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(tokens)
    }

    pub async fn delete(&self, id: Uuid, user_id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM api_tokens WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(self.pool.as_ref())
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    pub async fn update_last_used(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE api_tokens SET last_used_at = datetime('now') WHERE id = $1")
            .bind(id)
            .execute(self.pool.as_ref())
            .await?;

        Ok(())
    }
}
