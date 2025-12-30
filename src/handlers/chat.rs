use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, Result};
use crate::models::{
    ActionTaken, CardVisibility, ChatMessageResponse, ChatResponse, LlmAction, SendChatRequest,
};
use crate::state::AppState;

/// Build the system prompt with board context
async fn build_system_prompt(state: &AppState, board_id: Uuid) -> Result<String> {
    let board = state.boards.get_by_id(board_id).await?;
    let columns = state.columns.list_by_board(board_id).await?;
    let tags = state.tags.list_by_board(board_id).await?;

    let tag_names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();

    // Count cards per column
    let mut column_info = Vec::new();
    for col in &columns {
        let cards = state.cards.list_by_column(col.id).await?;
        column_info.push(format!("{} ({} cards)", col.name, cards.len()));
    }

    let examples = r##"
1. create_card - Create a new card
   {"action": "create_card", "params": {"column": "column_name", "title": "card title", "body": "optional description"}, "message": "Created card..."}

2. move_card - Move a card to another column
   {"action": "move_card", "params": {"card_title": "card to move", "target_column": "destination column"}, "message": "Moved card..."}

3. create_tag - Create a new tag
   {"action": "create_tag", "params": {"name": "tag name", "color": "#hex_color"}, "message": "Created tag..."}

4. add_tag - Add a tag to a card
   {"action": "add_tag", "params": {"card_title": "card title", "tag_name": "tag to add"}, "message": "Added tag..."}

5. list_cards - List cards (optionally filtered by column)
   {"action": "list_cards", "params": {"column": "optional column name"}, "message": "Here are the cards..."}

6. no_action - Just respond without taking action
   {"action": "no_action", "params": {}, "message": "Your response here..."}
"##;

    let tags_str = if tag_names.is_empty() {
        "none".to_string()
    } else {
        tag_names.join(", ")
    };

    Ok(format!(
        r#"You are a Kanban board assistant for the board "{board_name}".

You can execute these actions by responding with JSON:
{examples}
Current board state:
- Board: {board_name}
- Columns: {columns}
- Tags: {tags}

IMPORTANT: Always respond with valid JSON in the format shown above. Use "no_action" if the user is just asking a question or chatting.
"#,
        board_name = board.name,
        examples = examples,
        columns = column_info.join(", "),
        tags = tags_str
    ))
}

/// Parse the LLM response to extract action
fn parse_llm_response(response: &str) -> Option<LlmAction> {
    // Try to find JSON in the response
    let response = response.trim();

    // Try direct JSON parse first
    if let Ok(action) = serde_json::from_str::<LlmAction>(response) {
        return Some(action);
    }

    // Try to find JSON block in markdown code blocks
    if let Some(start) = response.find("```json") {
        if let Some(end) = response[start..]
            .find("```\n")
            .or(response[start..].rfind("```"))
        {
            let json_start = start + 7; // Skip "```json"
            let json_str = &response[json_start..start + end].trim();
            if let Ok(action) = serde_json::from_str::<LlmAction>(json_str) {
                return Some(action);
            }
        }
    }

    // Try to find JSON object anywhere in response
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            let json_str = &response[start..=end];
            if let Ok(action) = serde_json::from_str::<LlmAction>(json_str) {
                return Some(action);
            }
        }
    }

    None
}

/// Execute an action based on LLM response
async fn execute_action(
    state: &AppState,
    board_id: Uuid,
    user_id: Uuid,
    action: &LlmAction,
) -> Result<ActionTaken> {
    match action.action.as_str() {
        "create_card" => {
            let column_name = action.params["column"].as_str().unwrap_or("");
            let title = action.params["title"].as_str().unwrap_or("");
            let body = action.params["body"].as_str();

            if column_name.is_empty() || title.is_empty() {
                return Ok(ActionTaken {
                    action: "create_card".to_string(),
                    description: "Missing column or title".to_string(),
                    success: false,
                });
            }

            // Find column by name
            let columns = state.columns.list_by_board(board_id).await?;
            let column = columns
                .iter()
                .find(|c| c.name.to_lowercase() == column_name.to_lowercase());

            if let Some(col) = column {
                state
                    .cards
                    .create(
                        col.id,
                        title,
                        body,
                        None,
                        CardVisibility::Restricted,
                        None,
                        None,
                        None,
                        user_id,
                    )
                    .await?;

                Ok(ActionTaken {
                    action: "create_card".to_string(),
                    description: format!("Created card '{}' in column '{}'", title, col.name),
                    success: true,
                })
            } else {
                Ok(ActionTaken {
                    action: "create_card".to_string(),
                    description: format!("Column '{}' not found", column_name),
                    success: false,
                })
            }
        }

        "move_card" => {
            let card_title = action.params["card_title"].as_str().unwrap_or("");
            let target_column = action.params["target_column"].as_str().unwrap_or("");

            if card_title.is_empty() || target_column.is_empty() {
                return Ok(ActionTaken {
                    action: "move_card".to_string(),
                    description: "Missing card_title or target_column".to_string(),
                    success: false,
                });
            }

            // Find card by title
            let columns = state.columns.list_by_board(board_id).await?;
            let mut found_card = None;

            for col in &columns {
                let cards = state.cards.list_by_column(col.id).await?;
                if let Some(card) = cards
                    .iter()
                    .find(|c| c.title.to_lowercase() == card_title.to_lowercase())
                {
                    found_card = Some(card.clone());
                    break;
                }
            }

            // Find target column
            let target_col = columns
                .iter()
                .find(|c| c.name.to_lowercase() == target_column.to_lowercase());

            match (found_card, target_col) {
                (Some(card), Some(col)) => {
                    state.cards.move_card(card.id, col.id, 0).await?;
                    Ok(ActionTaken {
                        action: "move_card".to_string(),
                        description: format!("Moved '{}' to '{}'", card.title, col.name),
                        success: true,
                    })
                }
                (None, _) => Ok(ActionTaken {
                    action: "move_card".to_string(),
                    description: format!("Card '{}' not found", card_title),
                    success: false,
                }),
                (_, None) => Ok(ActionTaken {
                    action: "move_card".to_string(),
                    description: format!("Column '{}' not found", target_column),
                    success: false,
                }),
            }
        }

        "create_tag" => {
            let name = action.params["name"].as_str().unwrap_or("");
            let color = action.params["color"].as_str().unwrap_or("#6c757d");

            if name.is_empty() {
                return Ok(ActionTaken {
                    action: "create_tag".to_string(),
                    description: "Missing tag name".to_string(),
                    success: false,
                });
            }

            state.tags.create(board_id, name, color).await?;

            Ok(ActionTaken {
                action: "create_tag".to_string(),
                description: format!("Created tag '{}'", name),
                success: true,
            })
        }

        "add_tag" => {
            let card_title = action.params["card_title"].as_str().unwrap_or("");
            let tag_name = action.params["tag_name"].as_str().unwrap_or("");

            if card_title.is_empty() || tag_name.is_empty() {
                return Ok(ActionTaken {
                    action: "add_tag".to_string(),
                    description: "Missing card_title or tag_name".to_string(),
                    success: false,
                });
            }

            // Find card
            let columns = state.columns.list_by_board(board_id).await?;
            let mut found_card = None;

            for col in &columns {
                let cards = state.cards.list_by_column(col.id).await?;
                if let Some(card) = cards
                    .iter()
                    .find(|c| c.title.to_lowercase() == card_title.to_lowercase())
                {
                    found_card = Some(card.clone());
                    break;
                }
            }

            // Find tag
            let tags = state.tags.list_by_board(board_id).await?;
            let tag = tags
                .iter()
                .find(|t| t.name.to_lowercase() == tag_name.to_lowercase());

            match (found_card, tag) {
                (Some(card), Some(t)) => {
                    state.tags.add_to_card(card.id, t.id).await?;
                    Ok(ActionTaken {
                        action: "add_tag".to_string(),
                        description: format!("Added tag '{}' to '{}'", t.name, card.title),
                        success: true,
                    })
                }
                (None, _) => Ok(ActionTaken {
                    action: "add_tag".to_string(),
                    description: format!("Card '{}' not found", card_title),
                    success: false,
                }),
                (_, None) => Ok(ActionTaken {
                    action: "add_tag".to_string(),
                    description: format!("Tag '{}' not found", tag_name),
                    success: false,
                }),
            }
        }

        "list_cards" | "no_action" => Ok(ActionTaken {
            action: action.action.clone(),
            description: "No modification made".to_string(),
            success: true,
        }),

        _ => Ok(ActionTaken {
            action: action.action.clone(),
            description: format!("Unknown action: {}", action.action),
            success: false,
        }),
    }
}

/// Send a chat message and get a response
pub async fn send_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
    Json(input): Json<SendChatRequest>,
) -> Result<Json<ChatResponse>> {
    // Verify user has access to board
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    // Store input message before moving
    let user_message = input.message.clone();

    // Build system prompt with board context
    let system_prompt = build_system_prompt(&state, board_id).await?;

    // Create messages for Ollama
    let messages = vec![
        crate::services::ollama::OllamaMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
        crate::services::ollama::OllamaMessage {
            role: "user".to_string(),
            content: input.message,
        },
    ];

    // Send to Ollama
    let llm_response = state.ollama.chat(messages).await?;

    // Parse LLM response for actions
    let mut actions_taken = Vec::new();
    let response_message;

    if let Some(action) = parse_llm_response(&llm_response) {
        // Only execute if user can edit
        if role.can_edit() && action.action != "no_action" && action.action != "list_cards" {
            let action_result = execute_action(&state, board_id, auth.user.id, &action).await?;
            actions_taken.push(action_result);
        }
        response_message = action.message;
    } else {
        // LLM didn't return valid JSON, just use the raw response
        response_message = llm_response;
    }

    // Persist the chat message
    let actions_json = if actions_taken.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&actions_taken).unwrap_or_default())
    };

    state
        .chat_messages
        .create(
            board_id,
            auth.user.id,
            &user_message,
            &response_message,
            actions_json.as_deref(),
        )
        .await?;

    Ok(Json(ChatResponse {
        response: response_message,
        actions_taken,
    }))
}

/// Get chat history for a board
pub async fn get_history(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
) -> Result<Json<Vec<ChatMessageResponse>>> {
    // Verify user has access to board
    state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    // Get last 50 messages, ordered by created_at DESC
    let messages = state.chat_messages.list_by_board(board_id, 50).await?;

    // Convert to response format and reverse for chronological order
    let responses: Vec<ChatMessageResponse> = messages
        .into_iter()
        .map(|m| m.into_response())
        .rev()
        .collect();

    Ok(Json(responses))
}

/// Clear chat history for a board
pub async fn clear_history(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Verify user has owner/editor access to board
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    let deleted = state.chat_messages.delete_by_board(board_id).await?;

    Ok(Json(serde_json::json!({
        "deleted": deleted
    })))
}
