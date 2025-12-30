use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, Result};
use crate::models::{ColumnResponse, CreateColumn, MoveColumn, UpdateColumn};
use crate::state::AppState;

pub async fn create_column(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
    Json(input): Json<CreateColumn>,
) -> Result<Json<ColumnResponse>> {
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    if input.name.is_empty() {
        return Err(AppError::Validation("Column name is required".to_string()));
    }

    let column = state
        .columns
        .create(board_id, &input.name, input.position)
        .await?;

    Ok(Json(column.into()))
}

pub async fn list_columns(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
) -> Result<Json<Vec<ColumnResponse>>> {
    let _role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    let columns = state.columns.list_by_board(board_id).await?;
    Ok(Json(columns.into_iter().map(|c| c.into()).collect()))
}

pub async fn update_column(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(column_id): Path<Uuid>,
    Json(input): Json<UpdateColumn>,
) -> Result<Json<ColumnResponse>> {
    let column = state.columns.get_by_id(column_id).await?;

    let role = state
        .boards
        .get_user_role(column.board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    let updated = state
        .columns
        .update(column_id, input.name.as_deref())
        .await?;
    Ok(Json(updated.into()))
}

pub async fn delete_column(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(column_id): Path<Uuid>,
) -> Result<()> {
    let column = state.columns.get_by_id(column_id).await?;

    let role = state
        .boards
        .get_user_role(column.board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    state.columns.delete(column_id).await?;
    Ok(())
}

pub async fn move_column(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(column_id): Path<Uuid>,
    Json(input): Json<MoveColumn>,
) -> Result<Json<ColumnResponse>> {
    let column = state.columns.get_by_id(column_id).await?;

    let role = state
        .boards
        .get_user_role(column.board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    let updated = state.columns.move_column(column_id, input.position).await?;
    Ok(Json(updated.into()))
}
