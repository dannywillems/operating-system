use chrono::NaiveDate;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::{Card, CardFilter, CardVisibility};

#[derive(Clone)]
pub struct CardRepository {
    pool: Arc<SqlitePool>,
}

impl CardRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        column_id: Uuid,
        title: &str,
        body: Option<&str>,
        position: Option<i32>,
        visibility: CardVisibility,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        due_date: Option<NaiveDate>,
        created_by: Uuid,
    ) -> Result<Card> {
        let id = Uuid::new_v4();

        let pos = match position {
            Some(p) => p,
            None => {
                let max_pos = sqlx::query_scalar::<_, Option<i32>>(
                    "SELECT MAX(position) FROM cards WHERE column_id = $1",
                )
                .bind(column_id)
                .fetch_one(self.pool.as_ref())
                .await?;
                max_pos.unwrap_or(-1) + 1
            }
        };

        let card = sqlx::query_as::<_, Card>(
            r#"
            INSERT INTO cards (id, column_id, title, body, position, visibility, start_date, end_date, due_date, created_by, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, datetime('now'), datetime('now'))
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(column_id)
        .bind(title)
        .bind(body)
        .bind(pos)
        .bind(visibility.to_string())
        .bind(start_date)
        .bind(end_date)
        .bind(due_date)
        .bind(created_by)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(card)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Card>> {
        let card = sqlx::query_as::<_, Card>("SELECT * FROM cards WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool.as_ref())
            .await?;

        Ok(card)
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Card> {
        self.find_by_id(id).await?.ok_or(AppError::NotFound)
    }

    pub async fn list_by_column(&self, column_id: Uuid) -> Result<Vec<Card>> {
        let cards = sqlx::query_as::<_, Card>(
            "SELECT * FROM cards WHERE column_id = $1 ORDER BY position ASC",
        )
        .bind(column_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(cards)
    }

    pub async fn list_by_board_with_filter(
        &self,
        board_id: Uuid,
        _user_id: Uuid,
        user_role: Option<&str>,
        filter: &CardFilter,
    ) -> Result<Vec<Card>> {
        let mut query = String::from(
            r#"
            SELECT c.* FROM cards c
            INNER JOIN columns col ON c.column_id = col.id
            WHERE col.board_id = $1
            "#,
        );

        // Filter by visibility based on user role
        if let Some(role) = user_role {
            match role {
                "owner" | "editor" => {
                    // Can see all cards
                }
                "reader" => {
                    query.push_str(" AND (c.visibility != 'private')");
                }
                _ => {
                    query.push_str(" AND c.visibility = 'public'");
                }
            }
        } else {
            query.push_str(" AND c.visibility = 'public'");
        }

        // Full-text query on title/body
        if let Some(ref q) = filter.query {
            query.push_str(&format!(
                " AND (c.title LIKE '%{}%' OR c.body LIKE '%{}%')",
                q.replace('\'', "''"),
                q.replace('\'', "''")
            ));
        }

        // Date filters
        if let Some(date) = filter.start_date_from {
            query.push_str(&format!(" AND c.start_date >= '{}'", date));
        }
        if let Some(date) = filter.start_date_to {
            query.push_str(&format!(" AND c.start_date <= '{}'", date));
        }
        if let Some(date) = filter.end_date_from {
            query.push_str(&format!(" AND c.end_date >= '{}'", date));
        }
        if let Some(date) = filter.end_date_to {
            query.push_str(&format!(" AND c.end_date <= '{}'", date));
        }
        if let Some(date) = filter.due_date_from {
            query.push_str(&format!(" AND c.due_date >= '{}'", date));
        }
        if let Some(date) = filter.due_date_to {
            query.push_str(&format!(" AND c.due_date <= '{}'", date));
        }
        if let Some(date) = filter.updated_from {
            query.push_str(&format!(" AND c.updated_at >= '{}'", date));
        }
        if let Some(date) = filter.updated_to {
            query.push_str(&format!(" AND c.updated_at <= '{}'", date));
        }

        // Tag filter
        if let Some(ref tags) = filter.tags {
            if !tags.is_empty() {
                let tag_ids: Vec<String> = tags.iter().map(|t| format!("'{}'", t)).collect();
                query.push_str(&format!(
                    " AND c.id IN (SELECT card_id FROM card_tags WHERE tag_id IN ({}))",
                    tag_ids.join(",")
                ));
            }
        }

        query.push_str(" ORDER BY col.position ASC, c.position ASC");

        let cards = sqlx::query_as::<_, Card>(&query)
            .bind(board_id)
            .fetch_all(self.pool.as_ref())
            .await?;

        Ok(cards)
    }

    pub async fn update(
        &self,
        id: Uuid,
        title: Option<&str>,
        body: Option<&str>,
        visibility: Option<CardVisibility>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        due_date: Option<NaiveDate>,
    ) -> Result<Card> {
        let card = sqlx::query_as::<_, Card>(
            r#"
            UPDATE cards
            SET title = COALESCE($2, title),
                body = COALESCE($3, body),
                visibility = COALESCE($4, visibility),
                start_date = COALESCE($5, start_date),
                end_date = COALESCE($6, end_date),
                due_date = COALESCE($7, due_date),
                updated_at = datetime('now')
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(title)
        .bind(body)
        .bind(visibility.map(|v| v.to_string()))
        .bind(start_date)
        .bind(end_date)
        .bind(due_date)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(card)
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM cards WHERE id = $1")
            .bind(id)
            .execute(self.pool.as_ref())
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    pub async fn move_card(&self, id: Uuid, new_column_id: Uuid, new_position: i32) -> Result<Card> {
        let card = self.get_by_id(id).await?;

        // If moving within the same column
        if card.column_id == new_column_id {
            if new_position > card.position {
                sqlx::query(
                    r#"
                    UPDATE cards
                    SET position = position - 1
                    WHERE column_id = $1 AND position > $2 AND position <= $3
                    "#,
                )
                .bind(card.column_id)
                .bind(card.position)
                .bind(new_position)
                .execute(self.pool.as_ref())
                .await?;
            } else if new_position < card.position {
                sqlx::query(
                    r#"
                    UPDATE cards
                    SET position = position + 1
                    WHERE column_id = $1 AND position >= $2 AND position < $3
                    "#,
                )
                .bind(card.column_id)
                .bind(new_position)
                .bind(card.position)
                .execute(self.pool.as_ref())
                .await?;
            }
        } else {
            // Moving to a different column
            // Decrease positions in old column
            sqlx::query(
                r#"
                UPDATE cards
                SET position = position - 1
                WHERE column_id = $1 AND position > $2
                "#,
            )
            .bind(card.column_id)
            .bind(card.position)
            .execute(self.pool.as_ref())
            .await?;

            // Increase positions in new column
            sqlx::query(
                r#"
                UPDATE cards
                SET position = position + 1
                WHERE column_id = $1 AND position >= $2
                "#,
            )
            .bind(new_column_id)
            .bind(new_position)
            .execute(self.pool.as_ref())
            .await?;
        }

        // Update the card
        let updated = sqlx::query_as::<_, Card>(
            r#"
            UPDATE cards
            SET column_id = $2, position = $3, updated_at = datetime('now')
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(new_column_id)
        .bind(new_position)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(updated)
    }

    pub async fn get_board_id_for_card(&self, card_id: Uuid) -> Result<Uuid> {
        let board_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT col.board_id FROM cards c
            INNER JOIN columns col ON c.column_id = col.id
            WHERE c.id = $1
            "#,
        )
        .bind(card_id)
        .fetch_optional(self.pool.as_ref())
        .await?
        .ok_or(AppError::NotFound)?;

        Ok(board_id)
    }
}
