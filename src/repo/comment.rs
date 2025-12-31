use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::{Comment, CommentWithAuthor};

#[derive(Clone)]
pub struct CommentRepository {
    pool: Arc<SqlitePool>,
}

impl CommentRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Create a new comment on a card
    pub async fn create(&self, card_id: Uuid, user_id: Uuid, body: &str) -> Result<Comment> {
        let id = Uuid::new_v4();

        let comment = sqlx::query_as::<_, Comment>(
            r#"
            INSERT INTO comments (id, card_id, user_id, body, created_at, updated_at)
            VALUES ($1, $2, $3, $4, datetime('now'), datetime('now'))
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(card_id)
        .bind(user_id)
        .bind(body)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(comment)
    }

    /// Find a comment by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Comment>> {
        let comment = sqlx::query_as::<_, Comment>("SELECT * FROM comments WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool.as_ref())
            .await?;

        Ok(comment)
    }

    /// Get a comment by ID or return NotFound error
    pub async fn get_by_id(&self, id: Uuid) -> Result<Comment> {
        self.find_by_id(id).await?.ok_or(AppError::NotFound)
    }

    /// List all comments for a card, ordered by creation time
    pub async fn list_by_card(&self, card_id: Uuid) -> Result<Vec<CommentWithAuthor>> {
        let comments = sqlx::query_as::<_, CommentWithAuthor>(
            r#"
            SELECT c.id, c.card_id, c.user_id, c.body, c.created_at, c.updated_at,
                   u.name as author_name
            FROM comments c
            INNER JOIN users u ON c.user_id = u.id
            WHERE c.card_id = $1
            ORDER BY c.created_at ASC
            "#,
        )
        .bind(card_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(comments)
    }

    /// Update a comment's body
    pub async fn update(&self, id: Uuid, body: &str) -> Result<Comment> {
        let comment = sqlx::query_as::<_, Comment>(
            r#"
            UPDATE comments
            SET body = $2, updated_at = datetime('now')
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(body)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(comment)
    }

    /// Delete a comment
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM comments WHERE id = $1")
            .bind(id)
            .execute(self.pool.as_ref())
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    /// Count comments for a card
    pub async fn count_by_card(&self, card_id: Uuid) -> Result<i64> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM comments WHERE card_id = $1")
                .bind(card_id)
                .fetch_one(self.pool.as_ref())
                .await?;

        Ok(count.0)
    }
}
