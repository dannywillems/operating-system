use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Comment {
    pub id: Uuid,
    pub card_id: Uuid,
    pub user_id: Uuid,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateComment {
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateComment {
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub id: Uuid,
    pub card_id: Uuid,
    pub user_id: Uuid,
    pub author_name: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Comment with author info joined from users table
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CommentWithAuthor {
    pub id: Uuid,
    pub card_id: Uuid,
    pub user_id: Uuid,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author_name: String,
}

impl From<CommentWithAuthor> for CommentResponse {
    fn from(c: CommentWithAuthor) -> Self {
        Self {
            id: c.id,
            card_id: c.card_id,
            user_id: c.user_id,
            author_name: c.author_name,
            body: c.body,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}
