use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, Result};
use crate::models::{CreateTag, TagResponse};
use crate::state::AppState;

pub async fn create_tag(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
    Json(input): Json<CreateTag>,
) -> Result<Json<TagResponse>> {
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    if input.name.is_empty() {
        return Err(AppError::Validation("Tag name is required".to_string()));
    }

    let color = input.color.unwrap_or_else(|| "#6c757d".to_string());

    let tag = state.tags.create(board_id, &input.name, &color).await?;
    Ok(Json(tag.into()))
}

pub async fn list_tags(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
) -> Result<Json<Vec<TagResponse>>> {
    let _role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    let tags = state.tags.list_by_board(board_id).await?;
    Ok(Json(tags.into_iter().map(|t| t.into()).collect()))
}

pub async fn update_tag(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(tag_id): Path<Uuid>,
) -> Result<Json<TagResponse>> {
    // We need the board_id to check permissions, get it from the tag
    let tag = state.tags.get_by_id(tag_id).await?;

    let role = state
        .boards
        .get_user_role(tag.board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    Ok(Json(tag.into()))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(tag_id): Path<Uuid>,
) -> Result<()> {
    let tag = state.tags.get_by_id(tag_id).await?;

    let role = state
        .boards
        .get_user_role(tag.board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    state.tags.delete(tag_id).await?;
    Ok(())
}

pub async fn add_tag_to_card(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((card_id, tag_id)): Path<(Uuid, Uuid)>,
) -> Result<()> {
    let board_id = state.cards.get_board_id_for_card(card_id).await?;

    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    // Verify tag belongs to the same board
    let tag = state.tags.get_by_id(tag_id).await?;
    if tag.board_id != board_id {
        return Err(AppError::BadRequest(
            "Tag does not belong to this board".to_string(),
        ));
    }

    state.tags.add_to_card(card_id, tag_id).await?;
    Ok(())
}

pub async fn remove_tag_from_card(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((card_id, tag_id)): Path<(Uuid, Uuid)>,
) -> Result<()> {
    let board_id = state.cards.get_board_id_for_card(card_id).await?;

    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    state.tags.remove_from_card(card_id, tag_id).await?;
    Ok(())
}
