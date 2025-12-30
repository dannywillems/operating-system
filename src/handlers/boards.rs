use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, Result};
use crate::models::{
    AddBoardPermission, BoardResponse, BoardRole, BoardWithDetails, CreateBoard, UpdateBoard,
};
use crate::state::AppState;

pub async fn create_board(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateBoard>,
) -> Result<Json<BoardResponse>> {
    if input.name.is_empty() {
        return Err(AppError::Validation("Board name is required".to_string()));
    }

    let board = state
        .boards
        .create(&input.name, input.description.as_deref(), auth.user.id)
        .await?;

    Ok(Json(BoardResponse {
        id: board.id,
        name: board.name,
        description: board.description,
        owner_id: board.owner_id,
        role: "owner".to_string(),
        created_at: board.created_at,
        updated_at: board.updated_at,
    }))
}

pub async fn list_boards(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<BoardResponse>>> {
    let boards = state.boards.list_for_user(auth.user.id).await?;

    Ok(Json(
        boards
            .into_iter()
            .map(|(board, role)| BoardResponse {
                id: board.id,
                name: board.name,
                description: board.description,
                owner_id: board.owner_id,
                role,
                created_at: board.created_at,
                updated_at: board.updated_at,
            })
            .collect(),
    ))
}

pub async fn get_board(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
) -> Result<Json<BoardWithDetails>> {
    let board = state.boards.get_by_id(board_id).await?;
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    let columns = state.columns.list_by_board(board_id).await?;
    let tags = state.tags.list_by_board(board_id).await?;

    let mut column_responses = Vec::new();
    for col in columns {
        let cards = state.cards.list_by_column(col.id).await?;
        let mut card_responses = Vec::new();
        for card in cards {
            let card_tags = state.tags.list_for_card(card.id).await?;
            card_responses
                .push(card.into_response(card_tags.into_iter().map(|t| t.into()).collect()));
        }
        let mut col_response: crate::models::ColumnResponse = col.into();
        col_response.cards = card_responses;
        column_responses.push(col_response);
    }

    Ok(Json(BoardWithDetails {
        id: board.id,
        name: board.name,
        description: board.description,
        owner_id: board.owner_id,
        role: role.to_string(),
        columns: column_responses,
        tags: tags.into_iter().map(|t| t.into()).collect(),
        created_at: board.created_at,
        updated_at: board.updated_at,
    }))
}

pub async fn update_board(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
    Json(input): Json<UpdateBoard>,
) -> Result<Json<BoardResponse>> {
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    let board = state
        .boards
        .update(
            board_id,
            input.name.as_deref(),
            input.description.as_deref(),
        )
        .await?;

    Ok(Json(BoardResponse {
        id: board.id,
        name: board.name,
        description: board.description,
        owner_id: board.owner_id,
        role: role.to_string(),
        created_at: board.created_at,
        updated_at: board.updated_at,
    }))
}

pub async fn delete_board(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
) -> Result<()> {
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_delete() {
        return Err(AppError::Forbidden);
    }

    state.boards.delete(board_id).await?;
    Ok(())
}

pub async fn add_permission(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
    Json(input): Json<AddBoardPermission>,
) -> Result<()> {
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_manage_permissions() {
        return Err(AppError::Forbidden);
    }

    // Cannot add another owner
    if input.role == BoardRole::Owner {
        return Err(AppError::BadRequest("Cannot add another owner".to_string()));
    }

    state
        .boards
        .add_permission(board_id, input.user_id, input.role)
        .await?;

    Ok(())
}

pub async fn remove_permission(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((board_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<()> {
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_manage_permissions() {
        return Err(AppError::Forbidden);
    }

    state.boards.remove_permission(board_id, user_id).await?;
    Ok(())
}
