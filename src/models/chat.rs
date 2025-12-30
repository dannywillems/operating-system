use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Persisted chat message
/// board_id is optional to support global chat (None = global, Some = board-specific)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChatMessage {
    pub id: Uuid,
    pub board_id: Option<Uuid>,
    pub user_id: Uuid,
    pub message: String,
    pub response: String,
    pub actions_taken: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Chat message response for API
#[derive(Debug, Serialize)]
pub struct ChatMessageResponse {
    pub id: Uuid,
    pub message: String,
    pub response: String,
    pub actions_taken: Vec<ActionTaken>,
    pub created_at: DateTime<Utc>,
}

impl ChatMessage {
    pub fn into_response(self) -> ChatMessageResponse {
        let actions = self
            .actions_taken
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        ChatMessageResponse {
            id: self.id,
            message: self.message,
            response: self.response,
            actions_taken: actions,
            created_at: self.created_at,
        }
    }
}

/// Request to send a chat message
#[derive(Debug, Deserialize)]
pub struct SendChatRequest {
    pub message: String,
}

/// Response from chat endpoint
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub response: String,
    pub actions_taken: Vec<ActionTaken>,
}

/// An action that was executed by the chat handler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionTaken {
    pub action: String,
    pub description: String,
    pub success: bool,
}

/// Parsed action from LLM response
#[derive(Debug, Deserialize)]
pub struct LlmAction {
    pub action: String,
    #[serde(default)]
    pub params: serde_json::Value,
    pub message: String,
}

/// Multiple actions response from LLM
#[derive(Debug, Deserialize)]
pub struct LlmResponse {
    #[serde(default)]
    pub actions: Vec<LlmAction>,
    pub message: String,
}
