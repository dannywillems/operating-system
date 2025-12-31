use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: Uuid,
    pub board_id: Option<Uuid>,
    pub owner_id: Option<Uuid>,
    pub name: String,
    pub color: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CardTag {
    pub card_id: Uuid,
    pub tag_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTag {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTag {
    pub name: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TagResponse {
    pub id: Uuid,
    pub board_id: Option<Uuid>,
    pub owner_id: Option<Uuid>,
    pub name: String,
    pub color: String,
    pub created_at: DateTime<Utc>,
}

impl From<Tag> for TagResponse {
    fn from(tag: Tag) -> Self {
        Self {
            id: tag.id,
            board_id: tag.board_id,
            owner_id: tag.owner_id,
            name: tag.name,
            color: tag.color,
            created_at: tag.created_at,
        }
    }
}

/// Request to create a global (user-scoped) tag
#[derive(Debug, Deserialize)]
pub struct CreateGlobalTag {
    pub name: String,
    pub color: Option<String>,
}
