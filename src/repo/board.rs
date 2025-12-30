use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::{Board, BoardPermission, BoardRole};

#[derive(Debug, sqlx::FromRow)]
struct BoardWithRole {
    id: Uuid,
    name: String,
    description: Option<String>,
    owner_id: Uuid,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    role: String,
}

#[derive(Clone)]
pub struct BoardRepository {
    pool: Arc<SqlitePool>,
}

impl BoardRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub async fn create(&self, name: &str, description: Option<&str>, owner_id: Uuid) -> Result<Board> {
        let id = Uuid::new_v4();

        let board = sqlx::query_as::<_, Board>(
            r#"
            INSERT INTO boards (id, name, description, owner_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, datetime('now'), datetime('now'))
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(owner_id)
        .fetch_one(self.pool.as_ref())
        .await?;

        // Add owner permission
        self.add_permission(board.id, owner_id, BoardRole::Owner).await?;

        Ok(board)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Board>> {
        let board = sqlx::query_as::<_, Board>("SELECT * FROM boards WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool.as_ref())
            .await?;

        Ok(board)
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Board> {
        self.find_by_id(id).await?.ok_or(AppError::NotFound)
    }

    pub async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<(Board, String)>> {
        let rows = sqlx::query_as::<_, BoardWithRole>(
            r#"
            SELECT b.id, b.name, b.description, b.owner_id, b.created_at, b.updated_at, bp.role
            FROM boards b
            INNER JOIN board_permissions bp ON b.id = bp.board_id
            WHERE bp.user_id = $1
            ORDER BY b.updated_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        let boards = rows
            .into_iter()
            .map(|r| {
                (
                    Board {
                        id: r.id,
                        name: r.name,
                        description: r.description,
                        owner_id: r.owner_id,
                        created_at: r.created_at,
                        updated_at: r.updated_at,
                    },
                    r.role,
                )
            })
            .collect();

        Ok(boards)
    }

    pub async fn update(&self, id: Uuid, name: Option<&str>, description: Option<&str>) -> Result<Board> {
        let board = sqlx::query_as::<_, Board>(
            r#"
            UPDATE boards
            SET name = COALESCE($2, name),
                description = COALESCE($3, description),
                updated_at = datetime('now')
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(board)
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM boards WHERE id = $1")
            .bind(id)
            .execute(self.pool.as_ref())
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    pub async fn get_user_role(&self, board_id: Uuid, user_id: Uuid) -> Result<Option<BoardRole>> {
        let role = sqlx::query_scalar::<_, String>(
            "SELECT role FROM board_permissions WHERE board_id = $1 AND user_id = $2",
        )
        .bind(board_id)
        .bind(user_id)
        .fetch_optional(self.pool.as_ref())
        .await?;

        Ok(role.and_then(|r| r.parse().ok()))
    }

    pub async fn add_permission(&self, board_id: Uuid, user_id: Uuid, role: BoardRole) -> Result<BoardPermission> {
        let id = Uuid::new_v4();

        let permission = sqlx::query_as::<_, BoardPermission>(
            r#"
            INSERT INTO board_permissions (id, board_id, user_id, role, created_at)
            VALUES ($1, $2, $3, $4, datetime('now'))
            ON CONFLICT(board_id, user_id) DO UPDATE SET role = $4
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(board_id)
        .bind(user_id)
        .bind(role.to_string())
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(permission)
    }

    pub async fn remove_permission(&self, board_id: Uuid, user_id: Uuid) -> Result<()> {
        let result = sqlx::query(
            "DELETE FROM board_permissions WHERE board_id = $1 AND user_id = $2 AND role != 'owner'",
        )
        .bind(board_id)
        .bind(user_id)
        .execute(self.pool.as_ref())
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::BadRequest(
                "Cannot remove owner permission or permission not found".to_string(),
            ));
        }

        Ok(())
    }

    pub async fn list_permissions(&self, board_id: Uuid) -> Result<Vec<BoardPermission>> {
        let permissions = sqlx::query_as::<_, BoardPermission>(
            "SELECT * FROM board_permissions WHERE board_id = $1",
        )
        .bind(board_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(permissions)
    }
}
