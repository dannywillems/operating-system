use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::Column;

#[derive(Clone)]
pub struct ColumnRepository {
    pool: Arc<SqlitePool>,
}

impl ColumnRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub async fn create(&self, board_id: Uuid, name: &str, position: Option<i32>) -> Result<Column> {
        let id = Uuid::new_v4();

        // Get the next position if not specified
        let pos = match position {
            Some(p) => p,
            None => {
                let max_pos = sqlx::query_scalar::<_, Option<i32>>(
                    "SELECT MAX(position) FROM columns WHERE board_id = $1",
                )
                .bind(board_id)
                .fetch_one(self.pool.as_ref())
                .await?;
                max_pos.unwrap_or(-1) + 1
            }
        };

        let column = sqlx::query_as::<_, Column>(
            r#"
            INSERT INTO columns (id, board_id, name, position, created_at, updated_at)
            VALUES ($1, $2, $3, $4, datetime('now'), datetime('now'))
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(board_id)
        .bind(name)
        .bind(pos)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(column)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Column>> {
        let column = sqlx::query_as::<_, Column>("SELECT * FROM columns WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool.as_ref())
            .await?;

        Ok(column)
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Column> {
        self.find_by_id(id).await?.ok_or(AppError::NotFound)
    }

    pub async fn list_by_board(&self, board_id: Uuid) -> Result<Vec<Column>> {
        let columns = sqlx::query_as::<_, Column>(
            "SELECT * FROM columns WHERE board_id = $1 ORDER BY position ASC",
        )
        .bind(board_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(columns)
    }

    pub async fn update(&self, id: Uuid, name: Option<&str>) -> Result<Column> {
        let column = sqlx::query_as::<_, Column>(
            r#"
            UPDATE columns
            SET name = COALESCE($2, name),
                updated_at = datetime('now')
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(column)
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM columns WHERE id = $1")
            .bind(id)
            .execute(self.pool.as_ref())
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    pub async fn move_column(&self, id: Uuid, new_position: i32) -> Result<Column> {
        let column = self.get_by_id(id).await?;

        // Shift other columns
        if new_position > column.position {
            sqlx::query(
                r#"
                UPDATE columns
                SET position = position - 1
                WHERE board_id = $1 AND position > $2 AND position <= $3
                "#,
            )
            .bind(column.board_id)
            .bind(column.position)
            .bind(new_position)
            .execute(self.pool.as_ref())
            .await?;
        } else if new_position < column.position {
            sqlx::query(
                r#"
                UPDATE columns
                SET position = position + 1
                WHERE board_id = $1 AND position >= $2 AND position < $3
                "#,
            )
            .bind(column.board_id)
            .bind(new_position)
            .bind(column.position)
            .execute(self.pool.as_ref())
            .await?;
        }

        // Update the column's position
        let updated = sqlx::query_as::<_, Column>(
            r#"
            UPDATE columns
            SET position = $2, updated_at = datetime('now')
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(new_position)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(updated)
    }
}
