use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::User;

#[derive(Clone)]
pub struct UserRepository {
    pool: Arc<SqlitePool>,
}

impl UserRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        id: Uuid,
        email: &str,
        password_hash: &str,
        name: &str,
    ) -> Result<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, email, password_hash, name, created_at, updated_at)
            VALUES ($1, $2, $3, $4, datetime('now'), datetime('now'))
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(email)
        .bind(password_hash)
        .bind(name)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(user)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool.as_ref())
            .await?;

        Ok(user)
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(self.pool.as_ref())
            .await?;

        Ok(user)
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<User> {
        self.find_by_id(id).await?.ok_or(AppError::NotFound)
    }

    pub async fn email_exists(&self, email: &str) -> Result<bool> {
        let result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE email = $1")
            .bind(email)
            .fetch_one(self.pool.as_ref())
            .await?;

        Ok(result > 0)
    }

    pub async fn update_llm_context(&self, id: Uuid, llm_context: Option<&str>) -> Result<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET llm_context = $2, updated_at = datetime('now')
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(llm_context)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(user)
    }
}
