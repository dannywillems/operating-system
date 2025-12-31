use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, Result};
use crate::models::{
    CardFilter, CardResponse, CardStatus, CardVisibility, CreateCard, MoveCard, UpdateCard,
};
use crate::state::AppState;

pub async fn create_card(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(column_id): Path<Uuid>,
    Json(input): Json<CreateCard>,
) -> Result<Json<CardResponse>> {
    let column = state.columns.get_by_id(column_id).await?;

    let role = state
        .boards
        .get_user_role(column.board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    if input.title.is_empty() {
        return Err(AppError::Validation("Card title is required".to_string()));
    }

    let visibility = input.visibility.unwrap_or(CardVisibility::Restricted);
    let status = input.status.unwrap_or(CardStatus::Open);

    let card = state
        .cards
        .create(
            column_id,
            &input.title,
            input.body.as_deref(),
            input.position,
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

pub async fn list_cards(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
    Query(filter): Query<CardFilter>,
) -> Result<Json<Vec<CardResponse>>> {
    let role = state.boards.get_user_role(board_id, auth.user.id).await?;

    let cards = state
        .cards
        .list_by_board_with_filter(
            board_id,
            auth.user.id,
            role.as_ref().map(|r| r.to_string()).as_deref(),
            &filter,
        )
        .await?;

    let mut responses = Vec::new();
    for card in cards {
        let tags = state.tags.list_for_card(card.id).await?;
        responses.push(card.into_response(tags.into_iter().map(|t| t.into()).collect()));
    }

    Ok(Json(responses))
}

pub async fn get_card(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(card_id): Path<Uuid>,
) -> Result<Json<CardResponse>> {
    let card = state.cards.get_by_id(card_id).await?;
    let board_id = state.cards.get_board_id_for_card(card_id).await?;

    let role = state.boards.get_user_role(board_id, auth.user.id).await?;

    // Check visibility
    let visibility: CardVisibility = card.visibility.parse().unwrap_or(CardVisibility::Private);
    match visibility {
        CardVisibility::Private => {
            if !role.map(|r| r.can_edit()).unwrap_or(false) {
                return Err(AppError::Forbidden);
            }
        }
        CardVisibility::Restricted => {
            if role.is_none() {
                return Err(AppError::Forbidden);
            }
        }
        CardVisibility::Public => {
            // Anyone can view
        }
    }

    let tags = state.tags.list_for_card(card.id).await?;
    Ok(Json(card.into_response(
        tags.into_iter().map(|t| t.into()).collect(),
    )))
}

pub async fn update_card(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(card_id): Path<Uuid>,
    Json(input): Json<UpdateCard>,
) -> Result<Json<CardResponse>> {
    let board_id = state.cards.get_board_id_for_card(card_id).await?;

    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    let card = state
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

    let tags = state.tags.list_for_card(card.id).await?;
    Ok(Json(card.into_response(
        tags.into_iter().map(|t| t.into()).collect(),
    )))
}

pub async fn delete_card(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(card_id): Path<Uuid>,
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

    state.cards.delete(card_id).await?;
    Ok(())
}

pub async fn move_card(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(card_id): Path<Uuid>,
    Json(input): Json<MoveCard>,
) -> Result<Json<CardResponse>> {
    let board_id = state.cards.get_board_id_for_card(card_id).await?;

    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    // Verify target column belongs to the same board
    let target_column = state.columns.get_by_id(input.column_id).await?;
    if target_column.board_id != board_id {
        return Err(AppError::BadRequest(
            "Cannot move card to a different board".to_string(),
        ));
    }

    let card = state
        .cards
        .move_card(card_id, input.column_id, input.position)
        .await?;

    let tags = state.tags.list_for_card(card.id).await?;
    Ok(Json(card.into_response(
        tags.into_iter().map(|t| t.into()).collect(),
    )))
}
