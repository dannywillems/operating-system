use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::{CardTag, Tag};

#[derive(Clone)]
pub struct TagRepository {
    pool: Arc<SqlitePool>,
}

impl TagRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Create a board-scoped tag
    pub async fn create(&self, board_id: Uuid, name: &str, color: &str) -> Result<Tag> {
        let id = Uuid::new_v4();

        let tag = sqlx::query_as::<_, Tag>(
            r#"
            INSERT INTO tags (id, board_id, owner_id, name, color, created_at)
            VALUES ($1, $2, NULL, $3, $4, datetime('now'))
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(board_id)
        .bind(name)
        .bind(color)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(tag)
    }

    /// Create a global (user-scoped) tag
    pub async fn create_global(&self, owner_id: Uuid, name: &str, color: &str) -> Result<Tag> {
        let id = Uuid::new_v4();

        let tag = sqlx::query_as::<_, Tag>(
            r#"
            INSERT INTO tags (id, board_id, owner_id, name, color, created_at)
            VALUES ($1, NULL, $2, $3, $4, datetime('now'))
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(owner_id)
        .bind(name)
        .bind(color)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(tag)
    }

    /// List global tags owned by a user
    pub async fn list_by_owner(&self, owner_id: Uuid) -> Result<Vec<Tag>> {
        let tags = sqlx::query_as::<_, Tag>(
            "SELECT * FROM tags WHERE owner_id = $1 AND board_id IS NULL ORDER BY name ASC",
        )
        .bind(owner_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(tags)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Tag>> {
        let tag = sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool.as_ref())
            .await?;

        Ok(tag)
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Tag> {
        self.find_by_id(id).await?.ok_or(AppError::NotFound)
    }

    pub async fn list_by_board(&self, board_id: Uuid) -> Result<Vec<Tag>> {
        let tags =
            sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE board_id = $1 ORDER BY name ASC")
                .bind(board_id)
                .fetch_all(self.pool.as_ref())
                .await?;

        Ok(tags)
    }

    pub async fn update(&self, id: Uuid, name: Option<&str>, color: Option<&str>) -> Result<Tag> {
        let tag = sqlx::query_as::<_, Tag>(
            r#"
            UPDATE tags
            SET name = COALESCE($2, name),
                color = COALESCE($3, color)
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(color)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(tag)
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM tags WHERE id = $1")
            .bind(id)
            .execute(self.pool.as_ref())
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    pub async fn add_to_card(&self, card_id: Uuid, tag_id: Uuid) -> Result<CardTag> {
        let card_tag = sqlx::query_as::<_, CardTag>(
            r#"
            INSERT INTO card_tags (card_id, tag_id, created_at)
            VALUES ($1, $2, datetime('now'))
            ON CONFLICT(card_id, tag_id) DO UPDATE SET created_at = created_at
            RETURNING *
            "#,
        )
        .bind(card_id)
        .bind(tag_id)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(card_tag)
    }

    pub async fn remove_from_card(&self, card_id: Uuid, tag_id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM card_tags WHERE card_id = $1 AND tag_id = $2")
            .bind(card_id)
            .bind(tag_id)
            .execute(self.pool.as_ref())
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    pub async fn list_for_card(&self, card_id: Uuid) -> Result<Vec<Tag>> {
        let tags = sqlx::query_as::<_, Tag>(
            r#"
            SELECT t.* FROM tags t
            INNER JOIN card_tags ct ON t.id = ct.tag_id
            WHERE ct.card_id = $1
            ORDER BY t.name ASC
            "#,
        )
        .bind(card_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(tags)
    }
}
