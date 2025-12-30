use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum BoardRole {
    #[sqlx(rename = "owner")]
    Owner,
    #[sqlx(rename = "editor")]
    Editor,
    #[sqlx(rename = "reader")]
    Reader,
}

impl std::fmt::Display for BoardRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoardRole::Owner => write!(f, "owner"),
            BoardRole::Editor => write!(f, "editor"),
            BoardRole::Reader => write!(f, "reader"),
        }
    }
}

impl std::str::FromStr for BoardRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "owner" => Ok(BoardRole::Owner),
            "editor" => Ok(BoardRole::Editor),
            "reader" => Ok(BoardRole::Reader),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }
}

impl BoardRole {
    pub fn can_edit(&self) -> bool {
        matches!(self, BoardRole::Owner | BoardRole::Editor)
    }

    pub fn can_delete(&self) -> bool {
        matches!(self, BoardRole::Owner)
    }

    pub fn can_manage_permissions(&self) -> bool {
        matches!(self, BoardRole::Owner)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Board {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BoardPermission {
    pub id: Uuid,
    pub board_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBoard {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBoard {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddBoardPermission {
    pub user_id: Uuid,
    pub role: BoardRole,
}

#[derive(Debug, Serialize)]
pub struct BoardResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BoardWithDetails {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub role: String,
    pub columns: Vec<super::column::ColumnResponse>,
    pub tags: Vec<super::tag::TagResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
