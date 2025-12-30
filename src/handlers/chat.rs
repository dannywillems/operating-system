use axum::{
    extract::{Path, State},
    Json,
};
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, Result};
use crate::models::{
    ActionTaken, CardVisibility, ChatMessageResponse, ChatResponse, LlmAction, SendChatRequest,
};
use crate::state::AppState;

/// Build the system prompt with board context and user context
async fn build_system_prompt(
    state: &AppState,
    board_id: Uuid,
    user_context: Option<&str>,
) -> Result<String> {
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

6. list_tags - List all tags on the board
   {"action": "list_tags", "params": {}, "message": "Here are the tags..."}

7. delete_column - Delete a column (and all its cards)
   {"action": "delete_column", "params": {"column": "column name"}, "message": "Deleted column..."}

8. delete_tag - Delete a tag from the board
   {"action": "delete_tag", "params": {"tag": "tag name"}, "message": "Deleted tag..."}

9. delete_card - Delete a card
   {"action": "delete_card", "params": {"card": "card title"}, "message": "Deleted card..."}

10. no_action - Just respond without taking action
   {"action": "no_action", "params": {}, "message": "Your response here..."}
"##;

    let tags_str = if tag_names.is_empty() {
        "none".to_string()
    } else {
        tag_names.join(", ")
    };

    let user_context_section = match user_context {
        Some(ctx) if !ctx.is_empty() => format!("\nUser context:\n{}\n", ctx),
        _ => String::new(),
    };

    Ok(format!(
        r#"You are a Kanban board assistant for the board "{board_name}".
{user_context}
You can execute these actions by responding with JSON:
{examples}
Current board state:
- Board: {board_name}
- Columns: {columns}
- Tags: {tags}

IMPORTANT: Always respond with valid JSON in the format shown above. Use "no_action" if the user is just asking a question or chatting.
"#,
        board_name = board.name,
        user_context = user_context_section,
        examples = examples,
        columns = column_info.join(", "),
        tags = tags_str
    ))
}

/// Parse the LLM response to extract actions (handles multiple JSON objects)
fn parse_llm_response(response: &str) -> Vec<LlmAction> {
    let response = response.trim();
    let mut actions = Vec::new();

    // Try direct JSON parse first (single action)
    if let Ok(action) = serde_json::from_str::<LlmAction>(response) {
        return vec![action];
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
                return vec![action];
            }
        }
    }

    // Try to find multiple JSON objects in the response
    let mut search_start = 0;
    while let Some(start) = response[search_start..].find('{') {
        let abs_start = search_start + start;
        // Find matching closing brace
        let mut depth = 0;
        let mut end_pos = None;
        for (i, c) in response[abs_start..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end_pos = Some(abs_start + i);
                        break;
                    }
                }
                _ => {}
            }
        }

        if let Some(end) = end_pos {
            let json_str = &response[abs_start..=end];
            if let Ok(action) = serde_json::from_str::<LlmAction>(json_str) {
                actions.push(action);
            }
            search_start = end + 1;
        } else {
            break;
        }
    }

    actions
}

/// Extract a readable message from the LLM response, cleaning up raw JSON
fn extract_readable_message(response: &str, actions: &[LlmAction]) -> String {
    // If we have actions with messages, combine them
    if !actions.is_empty() {
        let messages: Vec<&str> = actions
            .iter()
            .filter(|a| !a.message.is_empty())
            .map(|a| a.message.as_str())
            .collect();

        if !messages.is_empty() {
            return messages.join(" ");
        }
    }

    // If response looks like raw JSON, provide a generic message
    let trimmed = response.trim();
    if trimmed.starts_with('{') || trimmed.contains("\"action\"") {
        return "Processing your request...".to_string();
    }

    response.to_string()
}

/// Execute an action based on LLM response
async fn execute_action(
    state: &AppState,
    board_id: Uuid,
    user_id: Uuid,
    action: &LlmAction,
) -> Result<ActionTaken> {
    match action.action.as_str() {
        "create_card" | "createcard" => {
            // Accept alternative param names
            let column_name = action.params["column"]
                .as_str()
                .or_else(|| action.params["column_name"].as_str())
                .or_else(|| action.params["in"].as_str())
                .unwrap_or("");
            let title = action.params["title"]
                .as_str()
                .or_else(|| action.params["name"].as_str())
                .or_else(|| action.params["card_title"].as_str())
                .unwrap_or("");
            let body = action.params["body"]
                .as_str()
                .or_else(|| action.params["description"].as_str())
                .or_else(|| action.params["content"].as_str());

            if column_name.is_empty() || title.is_empty() {
                return Ok(ActionTaken {
                    action: "create_card".to_string(),
                    description: format!(
                        "Missing column or title. Received params: {:?}",
                        action.params
                    ),
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

        "move_card" | "movecard" => {
            // Accept alternative param names the LLM might use
            let card_title = action.params["card_title"]
                .as_str()
                .or_else(|| action.params["card"].as_str())
                .or_else(|| action.params["title"].as_str())
                .or_else(|| action.params["name"].as_str())
                .unwrap_or("");
            let target_column = action.params["target_column"]
                .as_str()
                .or_else(|| action.params["column"].as_str())
                .or_else(|| action.params["to"].as_str())
                .or_else(|| action.params["destination"].as_str())
                .unwrap_or("");

            if card_title.is_empty() || target_column.is_empty() {
                return Ok(ActionTaken {
                    action: "move_card".to_string(),
                    description: format!(
                        "Missing card_title or target_column. Received params: {:?}",
                        action.params
                    ),
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

        "create_tag" | "createtag" => {
            // Accept alternative param names
            let name = action.params["name"]
                .as_str()
                .or_else(|| action.params["tag_name"].as_str())
                .or_else(|| action.params["tag"].as_str())
                .unwrap_or("");
            let color = action.params["color"]
                .as_str()
                .or_else(|| action.params["hex_color"].as_str())
                .unwrap_or("#6c757d");

            if name.is_empty() {
                return Ok(ActionTaken {
                    action: "create_tag".to_string(),
                    description: format!("Missing tag name. Received params: {:?}", action.params),
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

        "add_tag" | "addtag" => {
            // Accept alternative param names
            let card_title = action.params["card_title"]
                .as_str()
                .or_else(|| action.params["card"].as_str())
                .or_else(|| action.params["title"].as_str())
                .unwrap_or("");
            let tag_name = action.params["tag_name"]
                .as_str()
                .or_else(|| action.params["tag"].as_str())
                .or_else(|| action.params["name"].as_str())
                .unwrap_or("");

            if card_title.is_empty() || tag_name.is_empty() {
                return Ok(ActionTaken {
                    action: "add_tag".to_string(),
                    description: format!(
                        "Missing card_title or tag_name. Received params: {:?}",
                        action.params
                    ),
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

        "delete_column" | "deletecolumn" => {
            // Accept alternative param names
            let column_name = action.params["column"]
                .as_str()
                .or_else(|| action.params["column_name"].as_str())
                .or_else(|| action.params["name"].as_str())
                .unwrap_or("");

            if column_name.is_empty() {
                return Ok(ActionTaken {
                    action: "delete_column".to_string(),
                    description: format!(
                        "Missing column name. Received params: {:?}",
                        action.params
                    ),
                    success: false,
                });
            }

            // Find column by name
            let columns = state.columns.list_by_board(board_id).await?;
            let column = columns
                .iter()
                .find(|c| c.name.to_lowercase() == column_name.to_lowercase());

            if let Some(col) = column {
                state.columns.delete(col.id).await?;
                Ok(ActionTaken {
                    action: "delete_column".to_string(),
                    description: format!("Deleted column '{}'", col.name),
                    success: true,
                })
            } else {
                Ok(ActionTaken {
                    action: "delete_column".to_string(),
                    description: format!("Column '{}' not found", column_name),
                    success: false,
                })
            }
        }

        "delete_tag" | "deletetag" => {
            // Accept alternative param names
            let tag_name = action.params["tag"]
                .as_str()
                .or_else(|| action.params["tag_name"].as_str())
                .or_else(|| action.params["name"].as_str())
                .unwrap_or("");

            if tag_name.is_empty() {
                return Ok(ActionTaken {
                    action: "delete_tag".to_string(),
                    description: format!("Missing tag name. Received params: {:?}", action.params),
                    success: false,
                });
            }

            // Find tag by name
            let tags = state.tags.list_by_board(board_id).await?;
            let tag = tags
                .iter()
                .find(|t| t.name.to_lowercase() == tag_name.to_lowercase());

            if let Some(t) = tag {
                state.tags.delete(t.id).await?;
                Ok(ActionTaken {
                    action: "delete_tag".to_string(),
                    description: format!("Deleted tag '{}'", t.name),
                    success: true,
                })
            } else {
                Ok(ActionTaken {
                    action: "delete_tag".to_string(),
                    description: format!("Tag '{}' not found", tag_name),
                    success: false,
                })
            }
        }

        "delete_card" | "deletecard" => {
            // Accept alternative param names
            let card_title = action.params["card"]
                .as_str()
                .or_else(|| action.params["card_title"].as_str())
                .or_else(|| action.params["title"].as_str())
                .or_else(|| action.params["name"].as_str())
                .unwrap_or("");

            if card_title.is_empty() {
                return Ok(ActionTaken {
                    action: "delete_card".to_string(),
                    description: format!(
                        "Missing card title. Received params: {:?}",
                        action.params
                    ),
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

            if let Some(card) = found_card {
                state.cards.delete(card.id).await?;
                Ok(ActionTaken {
                    action: "delete_card".to_string(),
                    description: format!("Deleted card '{}'", card.title),
                    success: true,
                })
            } else {
                Ok(ActionTaken {
                    action: "delete_card".to_string(),
                    description: format!("Card '{}' not found", card_title),
                    success: false,
                })
            }
        }

        "list_cards" | "listcards" | "list_tags" | "listtags" | "no_action" | "noaction" => {
            Ok(ActionTaken {
                action: action.action.clone(),
                description: "No modification made".to_string(),
                success: true,
            })
        }

        _ => Ok(ActionTaken {
            action: action.action.clone(),
            description: format!("Unknown action: {}", action.action),
            success: false,
        }),
    }
}

/// Send a chat message and get a response
#[instrument(skip(state, auth, input), fields(user_id = %auth.user.id, board_id = %board_id))]
pub async fn send_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
    Json(input): Json<SendChatRequest>,
) -> Result<Json<ChatResponse>> {
    info!(message = %input.message, "Chat message received");

    // Verify user has access to board
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    debug!(role = ?role, "User role verified");

    // Store input message before moving
    let user_message = input.message.clone();

    // Build system prompt with board context and user's custom context
    let system_prompt =
        build_system_prompt(&state, board_id, auth.user.llm_context.as_deref()).await?;

    debug!("System prompt built successfully");

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
    info!("Sending request to LLM");
    let llm_response = state.ollama.chat(messages).await?;
    debug!(
        response_length = llm_response.len(),
        "LLM response received"
    );

    // Parse LLM response for actions (now handles multiple actions)
    let mut actions_taken = Vec::new();
    let parsed_actions = parse_llm_response(&llm_response);

    if parsed_actions.is_empty() {
        debug!("No actions parsed from LLM response");
    } else {
        info!(
            action_count = parsed_actions.len(),
            "Parsed actions from LLM response"
        );
        for action in &parsed_actions {
            debug!(action = %action.action, params = ?action.params, "Parsed action");
        }
    }

    // Execute all parsed actions
    for action in &parsed_actions {
        // Skip no-op actions
        let readonly_actions = [
            "no_action",
            "noaction",
            "list_cards",
            "listcards",
            "list_tags",
            "listtags",
        ];
        if readonly_actions.contains(&action.action.as_str()) {
            debug!(action = %action.action, "Skipping read-only action");
            continue;
        }

        // Only execute if user can edit
        if role.can_edit() {
            info!(action = %action.action, "Executing action");
            let action_result = execute_action(&state, board_id, auth.user.id, action).await?;

            if action_result.success {
                info!(
                    action = %action_result.action,
                    description = %action_result.description,
                    "Action executed successfully"
                );
            } else {
                warn!(
                    action = %action_result.action,
                    description = %action_result.description,
                    "Action failed"
                );
            }

            actions_taken.push(action_result);
        } else {
            warn!(action = %action.action, "User lacks permission to execute action");
        }
    }

    // Extract a readable message from the response
    let response_message = extract_readable_message(&llm_response, &parsed_actions);

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

    info!(
        actions_executed = actions_taken.len(),
        successful = actions_taken.iter().filter(|a| a.success).count(),
        "Chat request completed"
    );

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
