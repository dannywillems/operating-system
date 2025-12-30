use serde::{Deserialize, Serialize};

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
#[derive(Debug, Serialize)]
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
