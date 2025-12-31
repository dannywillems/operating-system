use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// All possible chat actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatAction {
    CreateBoard,
    DeleteBoard,
    CreateColumn,
    CreateCard,
    CreateInboxCard,
    MoveCard,
    MoveCardCrossBoard,
    AssignCard,
    UpdateStatus,
    CreateTag,
    AddTag,
    AddComment,
    ListCards,
    ListTags,
    DeleteColumn,
    DeleteTag,
    DeleteCard,
    WebSearch,
    NoAction,
    Unknown,
}

impl ChatAction {
    /// Returns true if this action only reads data (no modifications)
    pub fn is_read_only(&self) -> bool {
        matches!(
            self,
            ChatAction::ListCards
                | ChatAction::ListTags
                | ChatAction::WebSearch
                | ChatAction::NoAction
        )
    }

    /// Returns true if this action requires an existing board parameter
    pub fn requires_board(&self) -> bool {
        !matches!(
            self,
            ChatAction::CreateBoard
                | ChatAction::CreateInboxCard
                | ChatAction::MoveCardCrossBoard
                | ChatAction::AssignCard
                | ChatAction::UpdateStatus
                | ChatAction::AddComment
                | ChatAction::WebSearch
                | ChatAction::NoAction
                | ChatAction::ListCards
                | ChatAction::ListTags
                | ChatAction::Unknown
        )
    }
}

impl FromStr for ChatAction {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('_', "").as_str() {
            "createboard" => Ok(ChatAction::CreateBoard),
            "deleteboard" => Ok(ChatAction::DeleteBoard),
            "createcolumn" => Ok(ChatAction::CreateColumn),
            "createcard" => Ok(ChatAction::CreateCard),
            "createinboxcard" | "inboxcard" => Ok(ChatAction::CreateInboxCard),
            "movecard" => Ok(ChatAction::MoveCard),
            "movecardcrossboard" => Ok(ChatAction::MoveCardCrossBoard),
            "assigncard" | "assigntoboard" => Ok(ChatAction::AssignCard),
            "updatestatus" | "setstatus" | "changestatus" => Ok(ChatAction::UpdateStatus),
            "createtag" => Ok(ChatAction::CreateTag),
            "addtag" => Ok(ChatAction::AddTag),
            "addcomment" | "comment" => Ok(ChatAction::AddComment),
            "listcards" => Ok(ChatAction::ListCards),
            "listtags" => Ok(ChatAction::ListTags),
            "deletecolumn" => Ok(ChatAction::DeleteColumn),
            "deletetag" => Ok(ChatAction::DeleteTag),
            "deletecard" => Ok(ChatAction::DeleteCard),
            "websearch" | "search" => Ok(ChatAction::WebSearch),
            "noaction" => Ok(ChatAction::NoAction),
            _ => Ok(ChatAction::Unknown),
        }
    }
}

impl fmt::Display for ChatAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChatAction::CreateBoard => write!(f, "create_board"),
            ChatAction::DeleteBoard => write!(f, "delete_board"),
            ChatAction::CreateColumn => write!(f, "create_column"),
            ChatAction::CreateCard => write!(f, "create_card"),
            ChatAction::CreateInboxCard => write!(f, "create_inbox_card"),
            ChatAction::MoveCard => write!(f, "move_card"),
            ChatAction::MoveCardCrossBoard => write!(f, "move_card_cross_board"),
            ChatAction::AssignCard => write!(f, "assign_card"),
            ChatAction::UpdateStatus => write!(f, "update_status"),
            ChatAction::CreateTag => write!(f, "create_tag"),
            ChatAction::AddTag => write!(f, "add_tag"),
            ChatAction::AddComment => write!(f, "add_comment"),
            ChatAction::ListCards => write!(f, "list_cards"),
            ChatAction::ListTags => write!(f, "list_tags"),
            ChatAction::DeleteColumn => write!(f, "delete_column"),
            ChatAction::DeleteTag => write!(f, "delete_tag"),
            ChatAction::DeleteCard => write!(f, "delete_card"),
            ChatAction::WebSearch => write!(f, "web_search"),
            ChatAction::NoAction => write!(f, "no_action"),
            ChatAction::Unknown => write!(f, "unknown"),
        }
    }
}

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
