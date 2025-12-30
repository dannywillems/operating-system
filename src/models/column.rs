use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Column {
    pub id: Uuid,
    pub board_id: Uuid,
    pub name: String,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateColumn {
    pub name: String,
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateColumn {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MoveColumn {
    pub position: i32,
}

#[derive(Debug, Serialize)]
pub struct ColumnResponse {
    pub id: Uuid,
    pub board_id: Uuid,
    pub name: String,
    pub position: i32,
    pub cards: Vec<super::card::CardResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Column> for ColumnResponse {
    fn from(col: Column) -> Self {
        Self {
            id: col.id,
            board_id: col.board_id,
            name: col.name,
            position: col.position,
            cards: vec![],
            created_at: col.created_at,
            updated_at: col.updated_at,
        }
    }
}
