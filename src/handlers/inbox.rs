use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, Result};
use crate::models::{
    AssignCardToBoard, CardResponse, CardStatus, CardVisibility, CreateGlobalCard,
    CreateGlobalTag, MoveCardInBoard, TagResponse, UpdateCard, UpdateCardStatus,
};
use crate::state::AppState;

/// Query parameters for listing cards
#[derive(Debug, Deserialize, Default)]
pub struct ListCardsQuery {
    pub status: Option<String>,
}

/// List all cards owned by the current user
pub async fn list_cards(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListCardsQuery>,
) -> Result<Json<Vec<CardResponse>>> {
    let status = query
        .status
        .and_then(|s| s.parse::<CardStatus>().ok());

    let cards = state
        .cards
        .list_by_owner_with_status(auth.user.id, status)
        .await?;

    let mut responses = Vec::new();
    for card in cards {
        let tags = state.tags.list_for_card(card.id).await?;
        responses.push(card.into_response(tags.into_iter().map(|t| t.into()).collect()));
    }

    Ok(Json(responses))
}

/// Create a standalone (global) card
pub async fn create_card(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateGlobalCard>,
) -> Result<Json<CardResponse>> {
    if input.title.is_empty() {
        return Err(AppError::Validation("Card title is required".to_string()));
    }

    let visibility = input.visibility.unwrap_or(CardVisibility::Private);
    let status = input.status.unwrap_or(CardStatus::Open);

    let card = state
        .cards
        .create_standalone(
            &input.title,
            input.body.as_deref(),
            visibility,
            status,
            input.start_date,
            input.end_date,
            input.due_date,
            auth.user.id,
        )
        .await?;

    Ok(Json(card.into_response(vec![])))
}

/// Get a card by ID (user must own it or have access via board)
pub async fn get_card(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(card_id): Path<Uuid>,
) -> Result<Json<CardResponse>> {
    let card = state.cards.get_by_id(card_id).await?;

    // Check access: either owner or has board access
    let has_access = card.owner_id == Some(auth.user.id)
        || card.created_by == auth.user.id
        || {
            // Check if card is on any board the user has access to
            let boards = state.card_boards.list_boards_for_card(card_id).await?;
            let mut has_board_access = false;
            for board in boards {
                if state
                    .boards
                    .get_user_role(board.id, auth.user.id)
                    .await?
                    .is_some()
                {
                    has_board_access = true;
                    break;
                }
            }
            has_board_access
        };

    if !has_access {
        return Err(AppError::Forbidden);
    }

    let tags = state.tags.list_for_card(card.id).await?;
    Ok(Json(
        card.into_response(tags.into_iter().map(|t| t.into()).collect()),
    ))
}

/// Update a card (user must own it or have edit access via board)
pub async fn update_card(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(card_id): Path<Uuid>,
    Json(input): Json<UpdateCard>,
) -> Result<Json<CardResponse>> {
    let card = state.cards.get_by_id(card_id).await?;

    // Check edit access
    let can_edit = card.owner_id == Some(auth.user.id)
        || card.created_by == auth.user.id
        || {
            let boards = state.card_boards.list_boards_for_card(card_id).await?;
            let mut has_edit_access = false;
            for board in boards {
                if let Some(role) = state.boards.get_user_role(board.id, auth.user.id).await? {
                    if role.can_edit() {
                        has_edit_access = true;
                        break;
                    }
                }
            }
            has_edit_access
        };

    if !can_edit {
        return Err(AppError::Forbidden);
    }

    let updated_card = state
        .cards
        .update(
            card_id,
            input.title.as_deref(),
            input.body.as_deref(),
            input.visibility,
            input.status,
            input.start_date,
            input.end_date,
            input.due_date,
        )
        .await?;

    let tags = state.tags.list_for_card(updated_card.id).await?;
    Ok(Json(
        updated_card.into_response(tags.into_iter().map(|t| t.into()).collect()),
    ))
}

/// Update card status only
pub async fn update_card_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(card_id): Path<Uuid>,
    Json(input): Json<UpdateCardStatus>,
) -> Result<Json<CardResponse>> {
    let card = state.cards.get_by_id(card_id).await?;

    // Check edit access
    let can_edit = card.owner_id == Some(auth.user.id)
        || card.created_by == auth.user.id
        || {
            let boards = state.card_boards.list_boards_for_card(card_id).await?;
            let mut has_edit_access = false;
            for board in boards {
                if let Some(role) = state.boards.get_user_role(board.id, auth.user.id).await? {
                    if role.can_edit() {
                        has_edit_access = true;
                        break;
                    }
                }
            }
            has_edit_access
        };

    if !can_edit {
        return Err(AppError::Forbidden);
    }

    let updated_card = state.cards.update_status(card_id, input.status).await?;

    let tags = state.tags.list_for_card(updated_card.id).await?;
    Ok(Json(
        updated_card.into_response(tags.into_iter().map(|t| t.into()).collect()),
    ))
}

/// Delete a card (user must own it or have edit access via board)
pub async fn delete_card(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(card_id): Path<Uuid>,
) -> Result<()> {
    let card = state.cards.get_by_id(card_id).await?;

    // Check delete access
    let can_delete = card.owner_id == Some(auth.user.id)
        || card.created_by == auth.user.id
        || {
            let boards = state.card_boards.list_boards_for_card(card_id).await?;
            let mut has_delete_access = false;
            for board in boards {
                if let Some(role) = state.boards.get_user_role(board.id, auth.user.id).await? {
                    if role.can_edit() {
                        has_delete_access = true;
                        break;
                    }
                }
            }
            has_delete_access
        };

    if !can_delete {
        return Err(AppError::Forbidden);
    }

    state.cards.delete(card_id).await?;
    Ok(())
}

/// Assign a card to a board
pub async fn assign_card_to_board(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((card_id, board_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<AssignCardToBoard>,
) -> Result<Json<CardResponse>> {
    let card = state.cards.get_by_id(card_id).await?;

    // User must own the card or have edit access to the target board
    let can_assign = card.owner_id == Some(auth.user.id)
        || card.created_by == auth.user.id
        || state
            .boards
            .get_user_role(board_id, auth.user.id)
            .await?
            .map(|r| r.can_edit())
            .unwrap_or(false);

    if !can_assign {
        return Err(AppError::Forbidden);
    }

    // Verify column belongs to the board if specified
    if let Some(column_id) = input.column_id {
        let column = state.columns.get_by_id(column_id).await?;
        if column.board_id != board_id {
            return Err(AppError::BadRequest(
                "Column does not belong to this board".to_string(),
            ));
        }
    }

    // Check if already assigned
    if state
        .card_boards
        .is_card_on_board(card_id, board_id)
        .await?
    {
        return Err(AppError::BadRequest(
            "Card is already assigned to this board".to_string(),
        ));
    }

    state
        .card_boards
        .assign_card_to_board(card_id, board_id, input.column_id, input.position)
        .await?;

    let updated_card = state.cards.get_by_id(card_id).await?;
    let tags = state.tags.list_for_card(card_id).await?;
    Ok(Json(
        updated_card.into_response(tags.into_iter().map(|t| t.into()).collect()),
    ))
}

/// Remove a card from a board (does not delete the card)
pub async fn remove_card_from_board(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((card_id, board_id)): Path<(Uuid, Uuid)>,
) -> Result<()> {
    let card = state.cards.get_by_id(card_id).await?;

    // User must own the card or have edit access to the board
    let can_remove = card.owner_id == Some(auth.user.id)
        || card.created_by == auth.user.id
        || state
            .boards
            .get_user_role(board_id, auth.user.id)
            .await?
            .map(|r| r.can_edit())
            .unwrap_or(false);

    if !can_remove {
        return Err(AppError::Forbidden);
    }

    state
        .card_boards
        .remove_card_from_board(card_id, board_id)
        .await?;
    Ok(())
}

/// Move a card within a board (change column/position)
pub async fn move_card_in_board(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((card_id, board_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<MoveCardInBoard>,
) -> Result<Json<CardResponse>> {
    // User must have edit access to the board
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    // Verify column belongs to the board if specified
    if let Some(column_id) = input.column_id {
        let column = state.columns.get_by_id(column_id).await?;
        if column.board_id != board_id {
            return Err(AppError::BadRequest(
                "Column does not belong to this board".to_string(),
            ));
        }
    }

    state
        .card_boards
        .move_card_in_board(card_id, board_id, input.column_id, input.position)
        .await?;

    let card = state.cards.get_by_id(card_id).await?;
    let tags = state.tags.list_for_card(card_id).await?;
    Ok(Json(
        card.into_response(tags.into_iter().map(|t| t.into()).collect()),
    ))
}

/// List user's global tags
pub async fn list_global_tags(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<TagResponse>>> {
    let tags = state.tags.list_by_owner(auth.user.id).await?;
    Ok(Json(tags.into_iter().map(|t| t.into()).collect()))
}

/// Create a global tag
pub async fn create_global_tag(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateGlobalTag>,
) -> Result<Json<TagResponse>> {
    if input.name.is_empty() {
        return Err(AppError::Validation("Tag name is required".to_string()));
    }

    let color = input.color.unwrap_or_else(|| "#6c757d".to_string());
    let tag = state
        .tags
        .create_global(auth.user.id, &input.name, &color)
        .await?;
    Ok(Json(tag.into()))
}
