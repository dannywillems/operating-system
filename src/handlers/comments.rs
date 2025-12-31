use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, Result};
use crate::models::{CommentResponse, CreateComment, UpdateComment};
use crate::state::AppState;

/// Check if user has view access to a card
async fn can_view_card(state: &AppState, card_id: Uuid, user_id: Uuid) -> Result<bool> {
    let card = state.cards.get_by_id(card_id).await?;

    // Owner or creator always has access
    if card.owner_id == Some(user_id) || card.created_by == user_id {
        return Ok(true);
    }

    // Check if card is on any board the user has access to
    let boards = state.card_boards.list_boards_for_card(card_id).await?;
    for board in boards {
        if state
            .boards
            .get_user_role(board.id, user_id)
            .await?
            .is_some()
        {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Check if user has edit access to a card (required for adding comments)
async fn can_edit_card(state: &AppState, card_id: Uuid, user_id: Uuid) -> Result<bool> {
    let card = state.cards.get_by_id(card_id).await?;

    // Owner or creator always has edit access
    if card.owner_id == Some(user_id) || card.created_by == user_id {
        return Ok(true);
    }

    // Check if card is on any board the user has edit access to
    let boards = state.card_boards.list_boards_for_card(card_id).await?;
    for board in boards {
        if let Some(role) = state.boards.get_user_role(board.id, user_id).await? {
            if role.can_edit() {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// List all comments for a card
pub async fn list_comments(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(card_id): Path<Uuid>,
) -> Result<Json<Vec<CommentResponse>>> {
    // Verify card exists and user has access
    if !can_view_card(&state, card_id, auth.user.id).await? {
        return Err(AppError::Forbidden);
    }

    let comments = state.comments.list_by_card(card_id).await?;
    Ok(Json(comments.into_iter().map(|c| c.into()).collect()))
}

/// Add a comment to a card
pub async fn create_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(card_id): Path<Uuid>,
    Json(input): Json<CreateComment>,
) -> Result<Json<CommentResponse>> {
    if input.body.trim().is_empty() {
        return Err(AppError::Validation("Comment body is required".to_string()));
    }

    // Verify card exists and user has edit access
    if !can_edit_card(&state, card_id, auth.user.id).await? {
        return Err(AppError::Forbidden);
    }

    let comment = state
        .comments
        .create(card_id, auth.user.id, &input.body)
        .await?;

    // Get the author name for the response
    let user = state.users.get_by_id(auth.user.id).await?;

    Ok(Json(CommentResponse {
        id: comment.id,
        card_id: comment.card_id,
        user_id: comment.user_id,
        author_name: user.name,
        body: comment.body,
        created_at: comment.created_at,
        updated_at: comment.updated_at,
    }))
}

/// Update a comment (only the author can update)
pub async fn update_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(comment_id): Path<Uuid>,
    Json(input): Json<UpdateComment>,
) -> Result<Json<CommentResponse>> {
    if input.body.trim().is_empty() {
        return Err(AppError::Validation("Comment body is required".to_string()));
    }

    let comment = state.comments.get_by_id(comment_id).await?;

    // Only the author can edit their comment
    if comment.user_id != auth.user.id {
        return Err(AppError::Forbidden);
    }

    let updated = state.comments.update(comment_id, &input.body).await?;

    let user = state.users.get_by_id(auth.user.id).await?;

    Ok(Json(CommentResponse {
        id: updated.id,
        card_id: updated.card_id,
        user_id: updated.user_id,
        author_name: user.name,
        body: updated.body,
        created_at: updated.created_at,
        updated_at: updated.updated_at,
    }))
}

/// Delete a comment (only the author can delete)
pub async fn delete_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(comment_id): Path<Uuid>,
) -> Result<()> {
    let comment = state.comments.get_by_id(comment_id).await?;

    // Only the author can delete their comment
    if comment.user_id != auth.user.id {
        return Err(AppError::Forbidden);
    }

    state.comments.delete(comment_id).await?;
    Ok(())
}
