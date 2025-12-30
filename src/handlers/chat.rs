use axum::{
    extract::{Path, State},
    Json,
};
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, Result};
use crate::models::{
    ActionTaken, CardVisibility, ChatAction, ChatMessageResponse, ChatResponse, LlmAction,
    SendChatRequest,
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

/// Build the global system prompt with all accessible boards
async fn build_global_system_prompt(
    state: &AppState,
    user_id: Uuid,
    user_context: Option<&str>,
) -> Result<String> {
    // Fetch all boards the user has access to
    let boards = state.boards.list_for_user(user_id).await?;

    if boards.is_empty() {
        let user_context_section = match user_context {
            Some(ctx) if !ctx.is_empty() => format!("\nUser context:\n{}\n", ctx),
            _ => String::new(),
        };

        return Ok(format!(
            r#"You are a Kanban board assistant.
{user_context}
You don't have access to any boards yet. Suggest the user create a board first.

Respond with JSON:
{{"action": "no_action", "params": {{}}, "message": "Your response here..."}}
"#,
            user_context = user_context_section,
        ));
    }

    // Build board summaries
    let mut board_summaries = Vec::new();
    for (board, role) in &boards {
        let columns = state.columns.list_by_board(board.id).await?;
        let tags = state.tags.list_by_board(board.id).await?;

        let mut card_count = 0;
        let mut column_names = Vec::new();
        for col in &columns {
            let cards = state.cards.list_by_column(col.id).await?;
            card_count += cards.len();
            column_names.push(col.name.as_str());
        }

        let tag_names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();

        board_summaries.push(format!(
            "- {} (role: {}, {} columns: [{}], {} cards, tags: [{}])",
            board.name,
            role,
            columns.len(),
            column_names.join(", "),
            card_count,
            if tag_names.is_empty() {
                "none".to_string()
            } else {
                tag_names.join(", ")
            }
        ));
    }

    let examples = r##"
1. create_board - Create a new board
   {"action": "create_board", "params": {"name": "board name", "description": "optional description"}, "message": "Created board..."}

2. create_card - Create a new card (specify board)
   {"action": "create_card", "params": {"board": "board name", "column": "column_name", "title": "card title", "body": "optional description"}, "message": "Created card..."}

3. move_card - Move a card to another column (within same board)
   {"action": "move_card", "params": {"board": "board name", "card_title": "card to move", "target_column": "destination column"}, "message": "Moved card..."}

4. move_card_cross_board - Move a card between boards
   {"action": "move_card_cross_board", "params": {"from_board": "source board", "to_board": "target board", "card": "card title", "column": "destination column"}, "message": "Moved card..."}

5. create_tag - Create a new tag (specify board)
   {"action": "create_tag", "params": {"board": "board name", "name": "tag name", "color": "#hex_color"}, "message": "Created tag..."}

6. add_tag - Add a tag to a card (specify board)
   {"action": "add_tag", "params": {"board": "board name", "card_title": "card title", "tag_name": "tag to add"}, "message": "Added tag..."}

7. list_cards - List cards from a board
   {"action": "list_cards", "params": {"board": "board name", "column": "optional column name"}, "message": "Here are the cards..."}

8. list_tags - List all tags on a board
   {"action": "list_tags", "params": {"board": "board name"}, "message": "Here are the tags..."}

9. delete_column - Delete a column (specify board)
   {"action": "delete_column", "params": {"board": "board name", "column": "column name"}, "message": "Deleted column..."}

10. delete_tag - Delete a tag (specify board)
   {"action": "delete_tag", "params": {"board": "board name", "tag": "tag name"}, "message": "Deleted tag..."}

11. delete_card - Delete a card (specify board)
   {"action": "delete_card", "params": {"board": "board name", "card": "card title"}, "message": "Deleted card..."}

12. no_action - Just respond without taking action
   {"action": "no_action", "params": {}, "message": "Your response here..."}
"##;

    let user_context_section = match user_context {
        Some(ctx) if !ctx.is_empty() => format!("\nUser context:\n{}\n", ctx),
        _ => String::new(),
    };

    Ok(format!(
        r#"You are a Kanban board assistant with access to multiple boards.
{user_context}
You can execute actions across any of the user's boards. Always specify the "board" parameter.

Available actions:
{examples}
Your boards:
{boards}

IMPORTANT: Always respond with valid JSON. Always include "board" param for board-specific actions. Use "no_action" for questions.
"#,
        user_context = user_context_section,
        examples = examples,
        boards = board_summaries.join("\n")
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
    let chat_action: ChatAction = action.action.parse().unwrap_or(ChatAction::Unknown);

    match chat_action {
        ChatAction::CreateCard => {
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

        ChatAction::MoveCard => {
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

        ChatAction::CreateTag => {
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

        ChatAction::AddTag => {
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

        ChatAction::DeleteColumn => {
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

        ChatAction::DeleteTag => {
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

        ChatAction::DeleteCard => {
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

        ChatAction::ListCards | ChatAction::ListTags | ChatAction::NoAction => Ok(ActionTaken {
            action: chat_action.to_string(),
            description: "No modification made".to_string(),
            success: true,
        }),

        // CreateBoard and MoveCardCrossBoard are handled in global chat only
        ChatAction::CreateBoard | ChatAction::MoveCardCrossBoard => Ok(ActionTaken {
            action: chat_action.to_string(),
            description: "This action is only available in global chat".to_string(),
            success: false,
        }),

        ChatAction::Unknown => Ok(ActionTaken {
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

// =============================================================================
// Global Chat (Cross-Board)
// =============================================================================

/// Helper to find a board by name from the user's accessible boards
/// Returns (Board, role_string) where role_string is "owner", "editor", or "reader"
async fn find_board_by_name(
    state: &AppState,
    user_id: Uuid,
    board_name: &str,
) -> Result<Option<(crate::models::Board, String)>> {
    let boards = state.boards.list_for_user(user_id).await?;
    Ok(boards
        .into_iter()
        .find(|(b, _)| b.name.to_lowercase() == board_name.to_lowercase()))
}

/// Check if a role string allows editing
fn role_can_edit(role: &str) -> bool {
    matches!(role.to_lowercase().as_str(), "owner" | "editor")
}

/// Execute an action in global context (resolves board from params)
#[instrument(skip(state), fields(user_id = %user_id, action = %action.action))]
async fn execute_global_action(
    state: &AppState,
    user_id: Uuid,
    action: &LlmAction,
) -> Result<ActionTaken> {
    info!(params = ?action.params, "Executing global chat action");

    let chat_action: ChatAction = action.action.parse().unwrap_or(ChatAction::Unknown);

    // Handle special actions that don't need an existing board
    match chat_action {
        ChatAction::MoveCardCrossBoard => {
            return execute_cross_board_move(state, user_id, action).await;
        }
        ChatAction::CreateBoard => {
            return execute_create_board(state, user_id, action).await;
        }
        _ => {}
    }

    // For read-only actions, we don't need a board
    if chat_action.is_read_only() {
        info!(action = %chat_action, "Read-only action, no board modification");
        return Ok(ActionTaken {
            action: chat_action.to_string(),
            description: "No modification made".to_string(),
            success: true,
        });
    }

    // Get board name from params
    let board_name = action.params["board"]
        .as_str()
        .or_else(|| action.params["board_name"].as_str())
        .unwrap_or("");

    // Find the board
    if board_name.is_empty() {
        warn!("Missing board name in global action");
        return Ok(ActionTaken {
            action: chat_action.to_string(),
            description: format!(
                "Missing board name. Please specify which board. Params: {:?}",
                action.params
            ),
            success: false,
        });
    }

    let board_result = find_board_by_name(state, user_id, board_name).await?;
    let (board, role) = match board_result {
        Some(br) => br,
        None => {
            warn!(board_name = %board_name, "Board not found");
            return Ok(ActionTaken {
                action: chat_action.to_string(),
                description: format!("Board '{}' not found", board_name),
                success: false,
            });
        }
    };

    // Check permission
    if !role_can_edit(&role) {
        warn!(board = %board.name, role = %role, "Insufficient permission");
        return Ok(ActionTaken {
            action: chat_action.to_string(),
            description: format!("You don't have permission to edit board '{}'", board.name),
            success: false,
        });
    }

    info!(board = %board.name, "Executing action on board");

    // Execute the action using the existing single-board function
    let result = execute_action(state, board.id, user_id, action).await?;

    info!(
        success = result.success,
        description = %result.description,
        "Global action completed"
    );

    Ok(result)
}

/// Execute a cross-board card move
#[instrument(skip(state), fields(user_id = %user_id))]
async fn execute_cross_board_move(
    state: &AppState,
    user_id: Uuid,
    action: &LlmAction,
) -> Result<ActionTaken> {
    let from_board_name = action.params["from_board"]
        .as_str()
        .or_else(|| action.params["source_board"].as_str())
        .or_else(|| action.params["source"].as_str())
        .unwrap_or("");

    let to_board_name = action.params["to_board"]
        .as_str()
        .or_else(|| action.params["target_board"].as_str())
        .or_else(|| action.params["destination"].as_str())
        .unwrap_or("");

    let card_title = action.params["card"]
        .as_str()
        .or_else(|| action.params["card_title"].as_str())
        .or_else(|| action.params["title"].as_str())
        .unwrap_or("");

    let target_column = action.params["column"]
        .as_str()
        .or_else(|| action.params["target_column"].as_str())
        .or_else(|| action.params["to_column"].as_str())
        .unwrap_or("");

    if from_board_name.is_empty()
        || to_board_name.is_empty()
        || card_title.is_empty()
        || target_column.is_empty()
    {
        return Ok(ActionTaken {
            action: "move_card_cross_board".to_string(),
            description: format!(
                "Missing params. Need from_board, to_board, card, column. Got: {:?}",
                action.params
            ),
            success: false,
        });
    }

    info!(
        from_board = %from_board_name,
        to_board = %to_board_name,
        card = %card_title,
        column = %target_column,
        "Cross-board move requested"
    );

    // Find both boards
    let from_board = find_board_by_name(state, user_id, from_board_name).await?;
    let to_board = find_board_by_name(state, user_id, to_board_name).await?;

    let (source_board, source_role) = match from_board {
        Some(b) => b,
        None => {
            return Ok(ActionTaken {
                action: "move_card_cross_board".to_string(),
                description: format!("Source board '{}' not found", from_board_name),
                success: false,
            });
        }
    };

    let (target_board, target_role) = match to_board {
        Some(b) => b,
        None => {
            return Ok(ActionTaken {
                action: "move_card_cross_board".to_string(),
                description: format!("Target board '{}' not found", to_board_name),
                success: false,
            });
        }
    };

    // Check permissions on both boards
    if !role_can_edit(&source_role) {
        return Ok(ActionTaken {
            action: "move_card_cross_board".to_string(),
            description: format!(
                "You don't have permission to edit board '{}'",
                source_board.name
            ),
            success: false,
        });
    }

    if !role_can_edit(&target_role) {
        return Ok(ActionTaken {
            action: "move_card_cross_board".to_string(),
            description: format!(
                "You don't have permission to edit board '{}'",
                target_board.name
            ),
            success: false,
        });
    }

    // Find the card in source board
    let source_columns = state.columns.list_by_board(source_board.id).await?;
    let mut found_card = None;

    for col in &source_columns {
        let cards = state.cards.list_by_column(col.id).await?;
        if let Some(card) = cards
            .iter()
            .find(|c| c.title.to_lowercase() == card_title.to_lowercase())
        {
            found_card = Some(card.clone());
            break;
        }
    }

    let source_card = match found_card {
        Some(c) => c,
        None => {
            return Ok(ActionTaken {
                action: "move_card_cross_board".to_string(),
                description: format!(
                    "Card '{}' not found in board '{}'",
                    card_title, source_board.name
                ),
                success: false,
            });
        }
    };

    // Find target column
    let target_columns = state.columns.list_by_board(target_board.id).await?;
    let target_col = target_columns
        .iter()
        .find(|c| c.name.to_lowercase() == target_column.to_lowercase());

    let target_col = match target_col {
        Some(c) => c,
        None => {
            return Ok(ActionTaken {
                action: "move_card_cross_board".to_string(),
                description: format!(
                    "Column '{}' not found in board '{}'",
                    target_column, target_board.name
                ),
                success: false,
            });
        }
    };

    // Parse visibility from string to enum
    let visibility: CardVisibility = source_card
        .visibility
        .parse()
        .unwrap_or(CardVisibility::Restricted);

    // Create new card in target board
    state
        .cards
        .create(
            target_col.id,
            &source_card.title,
            source_card.body.as_deref(),
            None,
            visibility,
            source_card.start_date,
            source_card.end_date,
            source_card.due_date,
            user_id,
        )
        .await?;

    // Delete source card
    state.cards.delete(source_card.id).await?;

    info!(
        card = %source_card.title,
        from = %source_board.name,
        to = %target_board.name,
        "Cross-board move completed"
    );

    Ok(ActionTaken {
        action: "move_card_cross_board".to_string(),
        description: format!(
            "Moved '{}' from '{}' to '{}' (column '{}')",
            source_card.title, source_board.name, target_board.name, target_col.name
        ),
        success: true,
    })
}

/// Execute create_board action
#[instrument(skip(state), fields(user_id = %user_id))]
async fn execute_create_board(
    state: &AppState,
    user_id: Uuid,
    action: &LlmAction,
) -> Result<ActionTaken> {
    let name = action.params["name"]
        .as_str()
        .or_else(|| action.params["board_name"].as_str())
        .or_else(|| action.params["title"].as_str())
        .unwrap_or("");

    let description = action.params["description"]
        .as_str()
        .or_else(|| action.params["desc"].as_str());

    if name.is_empty() {
        return Ok(ActionTaken {
            action: "create_board".to_string(),
            description: format!("Missing board name. Params: {:?}", action.params),
            success: false,
        });
    }

    info!(name = %name, description = ?description, "Creating new board");

    let board = state.boards.create(name, description, user_id).await?;

    info!(board_id = %board.id, name = %board.name, "Board created successfully");

    Ok(ActionTaken {
        action: "create_board".to_string(),
        description: format!("Created board '{}'", board.name),
        success: true,
    })
}

/// Send a global chat message (cross-board)
#[instrument(skip(state, auth, input), fields(user_id = %auth.user.id))]
pub async fn send_global_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<SendChatRequest>,
) -> Result<Json<ChatResponse>> {
    info!(message = %input.message, "Global chat message received");

    // Store input message before moving
    let user_message = input.message.clone();

    // Build global system prompt with all boards
    let system_prompt =
        build_global_system_prompt(&state, auth.user.id, auth.user.llm_context.as_deref()).await?;

    debug!("Global system prompt built successfully");

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
    info!("Sending global chat request to LLM");
    let llm_response = state.ollama.chat(messages).await?;
    debug!(
        response_length = llm_response.len(),
        "LLM response received"
    );

    // Parse LLM response for actions
    let mut actions_taken = Vec::new();
    let parsed_actions = parse_llm_response(&llm_response);

    if parsed_actions.is_empty() {
        debug!("No actions parsed from LLM response");
    } else {
        info!(
            action_count = parsed_actions.len(),
            "Parsed actions from global LLM response"
        );
    }

    // Execute all parsed actions
    for action in &parsed_actions {
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

        info!(action = %action.action, "Executing global action");
        let action_result = execute_global_action(&state, auth.user.id, action).await?;

        if action_result.success {
            info!(
                action = %action_result.action,
                description = %action_result.description,
                "Global action executed successfully"
            );
        } else {
            warn!(
                action = %action_result.action,
                description = %action_result.description,
                "Global action failed"
            );
        }

        actions_taken.push(action_result);
    }

    // Extract a readable message from the response
    let response_message = extract_readable_message(&llm_response, &parsed_actions);

    // Persist the chat message (global = no board_id)
    let actions_json = if actions_taken.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&actions_taken).unwrap_or_default())
    };

    state
        .chat_messages
        .create_global(
            auth.user.id,
            &user_message,
            &response_message,
            actions_json.as_deref(),
        )
        .await?;

    info!(
        actions_executed = actions_taken.len(),
        successful = actions_taken.iter().filter(|a| a.success).count(),
        "Global chat request completed"
    );

    Ok(Json(ChatResponse {
        response: response_message,
        actions_taken,
    }))
}

/// Get global chat history
pub async fn get_global_history(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ChatMessageResponse>>> {
    // Get last 50 global messages
    let messages = state.chat_messages.list_global(auth.user.id, 50).await?;

    // Convert to response format and reverse for chronological order
    let responses: Vec<ChatMessageResponse> = messages
        .into_iter()
        .map(|m| m.into_response())
        .rev()
        .collect();

    Ok(Json(responses))
}

/// Clear global chat history
pub async fn clear_global_history(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>> {
    let deleted = state.chat_messages.delete_global(auth.user.id).await?;

    Ok(Json(serde_json::json!({
        "deleted": deleted
    })))
}
