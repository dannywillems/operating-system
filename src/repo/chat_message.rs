use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::Result;
use crate::models::ChatMessage;

#[derive(Clone)]
pub struct ChatMessageRepository {
    pool: Arc<SqlitePool>,
}

impl ChatMessageRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        board_id: Uuid,
        user_id: Uuid,
        message: &str,
        response: &str,
        actions_taken: Option<&str>,
    ) -> Result<ChatMessage> {
        let id = Uuid::new_v4();

        let chat_message = sqlx::query_as::<_, ChatMessage>(
            r#"
            INSERT INTO chat_messages (id, board_id, user_id, message, response, actions_taken, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, datetime('now'))
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(board_id)
        .bind(user_id)
        .bind(message)
        .bind(response)
        .bind(actions_taken)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(chat_message)
    }

    pub async fn list_by_board(&self, board_id: Uuid, limit: i64) -> Result<Vec<ChatMessage>> {
        let messages = sqlx::query_as::<_, ChatMessage>(
            r#"
            SELECT * FROM chat_messages
            WHERE board_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(board_id)
        .bind(limit)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(messages)
    }

    pub async fn delete_by_board(&self, board_id: Uuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM chat_messages WHERE board_id = $1")
            .bind(board_id)
            .execute(self.pool.as_ref())
            .await?;

        Ok(result.rows_affected())
    }

    pub async fn delete_all_by_user(&self, user_id: Uuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM chat_messages WHERE user_id = $1")
            .bind(user_id)
            .execute(self.pool.as_ref())
            .await?;

        Ok(result.rows_affected())
    }

    pub async fn count_by_user(&self, user_id: Uuid) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM chat_messages WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(self.pool.as_ref())
            .await?;

        Ok(count.0)
    }
}
