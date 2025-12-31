use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::{Board, Card, CardBoardAssignment};

#[derive(Clone)]
pub struct CardBoardRepository {
    pool: Arc<SqlitePool>,
}

impl CardBoardRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Assign a card to a board, optionally placing it in a column
    pub async fn assign_card_to_board(
        &self,
        card_id: Uuid,
        board_id: Uuid,
        column_id: Option<Uuid>,
        position: Option<i32>,
    ) -> Result<CardBoardAssignment> {
        let id = Uuid::new_v4();

        // Calculate position if not provided
        let pos = match position {
            Some(p) => p,
            None => {
                let max_pos = if let Some(col_id) = column_id {
                    sqlx::query_scalar::<_, Option<i32>>(
                        "SELECT MAX(position) FROM card_boards WHERE board_id = $1 AND column_id = $2",
                    )
                    .bind(board_id)
                    .bind(col_id)
                    .fetch_one(self.pool.as_ref())
                    .await?
                } else {
                    sqlx::query_scalar::<_, Option<i32>>(
                        "SELECT MAX(position) FROM card_boards WHERE board_id = $1 AND column_id IS NULL",
                    )
                    .bind(board_id)
                    .fetch_one(self.pool.as_ref())
                    .await?
                };
                max_pos.unwrap_or(-1) + 1
            }
        };

        let assignment = sqlx::query_as::<_, CardBoardAssignment>(
            r#"
            INSERT INTO card_boards (id, card_id, board_id, column_id, position, created_at)
            VALUES ($1, $2, $3, $4, $5, datetime('now'))
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(card_id)
        .bind(board_id)
        .bind(column_id)
        .bind(pos)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(assignment)
    }

    /// Remove a card from a board
    pub async fn remove_card_from_board(&self, card_id: Uuid, board_id: Uuid) -> Result<()> {
        let result = sqlx::query(
            "DELETE FROM card_boards WHERE card_id = $1 AND board_id = $2",
        )
        .bind(card_id)
        .bind(board_id)
        .execute(self.pool.as_ref())
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    /// Move a card within a board (change column and/or position)
    pub async fn move_card_in_board(
        &self,
        card_id: Uuid,
        board_id: Uuid,
        column_id: Option<Uuid>,
        position: i32,
    ) -> Result<CardBoardAssignment> {
        let assignment = sqlx::query_as::<_, CardBoardAssignment>(
            r#"
            UPDATE card_boards
            SET column_id = $3, position = $4
            WHERE card_id = $1 AND board_id = $2
            RETURNING *
            "#,
        )
        .bind(card_id)
        .bind(board_id)
        .bind(column_id)
        .bind(position)
        .fetch_optional(self.pool.as_ref())
        .await?
        .ok_or(AppError::NotFound)?;

        Ok(assignment)
    }

    /// Get assignment for a card on a specific board
    pub async fn get_assignment(
        &self,
        card_id: Uuid,
        board_id: Uuid,
    ) -> Result<Option<CardBoardAssignment>> {
        let assignment = sqlx::query_as::<_, CardBoardAssignment>(
            "SELECT * FROM card_boards WHERE card_id = $1 AND board_id = $2",
        )
        .bind(card_id)
        .bind(board_id)
        .fetch_optional(self.pool.as_ref())
        .await?;

        Ok(assignment)
    }

    /// List all boards a card is assigned to
    pub async fn list_boards_for_card(&self, card_id: Uuid) -> Result<Vec<Board>> {
        let boards = sqlx::query_as::<_, Board>(
            r#"
            SELECT b.* FROM boards b
            INNER JOIN card_boards cb ON b.id = cb.board_id
            WHERE cb.card_id = $1
            ORDER BY cb.created_at ASC
            "#,
        )
        .bind(card_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(boards)
    }

    /// List all cards assigned to a board
    pub async fn list_cards_for_board(&self, board_id: Uuid) -> Result<Vec<Card>> {
        let cards = sqlx::query_as::<_, Card>(
            r#"
            SELECT c.* FROM cards c
            INNER JOIN card_boards cb ON c.id = cb.card_id
            WHERE cb.board_id = $1
            ORDER BY cb.column_id, cb.position ASC
            "#,
        )
        .bind(board_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(cards)
    }

    /// List all assignments for a board (includes position info)
    pub async fn list_assignments_for_board(&self, board_id: Uuid) -> Result<Vec<CardBoardAssignment>> {
        let assignments = sqlx::query_as::<_, CardBoardAssignment>(
            r#"
            SELECT * FROM card_boards
            WHERE board_id = $1
            ORDER BY column_id, position ASC
            "#,
        )
        .bind(board_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(assignments)
    }

    /// Check if a card is assigned to a specific board
    pub async fn is_card_on_board(&self, card_id: Uuid, board_id: Uuid) -> Result<bool> {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM card_boards WHERE card_id = $1 AND board_id = $2)",
        )
        .bind(card_id)
        .bind(board_id)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(exists)
    }
}
