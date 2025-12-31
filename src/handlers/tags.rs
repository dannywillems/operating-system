use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, Result};
use crate::models::{CreateTag, TagResponse, UpdateTag};
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
    Json(input): Json<UpdateTag>,
) -> Result<Json<TagResponse>> {
    // We need the board_id to check permissions, get it from the tag
    let tag = state.tags.get_by_id(tag_id).await?;

    // Check permissions based on tag scope
    if let Some(board_id) = tag.board_id {
        // Board-scoped tag - check board permissions
        let role = state
            .boards
            .get_user_role(board_id, auth.user.id)
            .await?
            .ok_or(AppError::Forbidden)?;

        if !role.can_edit() {
            return Err(AppError::Forbidden);
        }
    } else if let Some(owner_id) = tag.owner_id {
        // Global tag - only owner can edit
        if owner_id != auth.user.id {
            return Err(AppError::Forbidden);
        }
    } else {
        return Err(AppError::Forbidden);
    }

    let updated_tag = state
        .tags
        .update(tag_id, input.name.as_deref(), input.color.as_deref())
        .await?;

    Ok(Json(updated_tag.into()))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(tag_id): Path<Uuid>,
) -> Result<()> {
    let tag = state.tags.get_by_id(tag_id).await?;

    // Check permissions based on tag scope
    if let Some(board_id) = tag.board_id {
        // Board-scoped tag - check board permissions
        let role = state
            .boards
            .get_user_role(board_id, auth.user.id)
            .await?
            .ok_or(AppError::Forbidden)?;

        if !role.can_edit() {
            return Err(AppError::Forbidden);
        }
    } else if let Some(owner_id) = tag.owner_id {
        // Global tag - only owner can delete
        if owner_id != auth.user.id {
            return Err(AppError::Forbidden);
        }
    } else {
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
    // Get the tag first to check its scope
    let tag = state.tags.get_by_id(tag_id).await?;

    // Get the card's board (if any) for permission checking
    let card_board_id = state.cards.get_board_id_for_card(card_id).await.ok();

    // Check if user can add this tag to this card
    if let Some(tag_board_id) = tag.board_id {
        // Board-scoped tag - card must be on the same board
        if let Some(card_bid) = card_board_id {
            if tag_board_id != card_bid {
                return Err(AppError::BadRequest(
                    "Tag does not belong to this board".to_string(),
                ));
            }
            // Check board permissions
            let role = state
                .boards
                .get_user_role(card_bid, auth.user.id)
                .await?
                .ok_or(AppError::Forbidden)?;

            if !role.can_edit() {
                return Err(AppError::Forbidden);
            }
        } else {
            return Err(AppError::BadRequest(
                "Cannot add board tag to a standalone card".to_string(),
            ));
        }
    } else if let Some(tag_owner_id) = tag.owner_id {
        // Global tag - user must own the tag or have edit access to the card's board
        let can_use_tag = tag_owner_id == auth.user.id;
        let can_edit_card = if let Some(bid) = card_board_id {
            state
                .boards
                .get_user_role(bid, auth.user.id)
                .await?
                .map(|r| r.can_edit())
                .unwrap_or(false)
        } else {
            // Standalone card - check if user owns it
            let card = state.cards.get_by_id(card_id).await?;
            card.owner_id == Some(auth.user.id) || card.created_by == auth.user.id
        };

        if !can_use_tag || !can_edit_card {
            return Err(AppError::Forbidden);
        }
    } else {
        return Err(AppError::Forbidden);
    }

    state.tags.add_to_card(card_id, tag_id).await?;
    Ok(())
}

pub async fn remove_tag_from_card(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((card_id, tag_id)): Path<(Uuid, Uuid)>,
) -> Result<()> {
    // Get the card's board (if any) for permission checking
    let card_board_id = state.cards.get_board_id_for_card(card_id).await.ok();

    // Check if user can edit this card
    if let Some(board_id) = card_board_id {
        let role = state
            .boards
            .get_user_role(board_id, auth.user.id)
            .await?
            .ok_or(AppError::Forbidden)?;

        if !role.can_edit() {
            return Err(AppError::Forbidden);
        }
    } else {
        // Standalone card - check if user owns it
        let card = state.cards.get_by_id(card_id).await?;
        if card.owner_id != Some(auth.user.id) && card.created_by != auth.user.id {
            return Err(AppError::Forbidden);
        }
    }

    state.tags.remove_from_card(card_id, tag_id).await?;
    Ok(())
}
