use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{generate_token, hash_password, verify_password, AuthUser, OptionalAuthUser};
use crate::error::{AppError, Result};
use crate::models::{CardStatus, CardVisibility};
use crate::state::AppState;

// Template structs
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    user: Option<String>,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: Option<String>,
}

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {
    error: Option<String>,
}

#[derive(Template)]
#[template(path = "boards.html")]
struct BoardsTemplate {
    user: String,
    boards: Vec<BoardView>,
}

#[derive(Template)]
#[template(path = "board_new.html")]
struct NewBoardTemplate {
    user: String,
}

#[derive(Template)]
#[template(path = "board_detail.html")]
struct BoardDetailTemplate {
    user: String,
    board: BoardView,
    columns: Vec<ColumnView>,
    tags: Vec<TagView>,
    filter_tags: Vec<FilterTagView>,
    has_active_filters: bool,
}

#[derive(Template)]
#[template(path = "board_settings.html")]
struct BoardSettingsTemplate {
    user: String,
    board: BoardView,
    tags: Vec<TagView>,
}

#[derive(Template)]
#[template(path = "user_settings.html")]
struct UserSettingsTemplate {
    user: String,
    chat_message_count: i64,
    llm_context: Option<String>,
}

#[derive(Template)]
#[template(path = "inbox.html")]
struct InboxTemplate {
    user: String,
    cards: Vec<InboxCardView>,
    current_status: Option<String>,
}

#[derive(Template)]
#[template(path = "card_detail.html")]
struct CardDetailTemplate {
    user: String,
    card: CardDetailView,
    comments: Vec<CommentView>,
    current_user_id: String,
}

// View structs for templates
#[allow(dead_code)]
struct InboxCardView {
    id: String,
    title: String,
    body: Option<String>,
    status: String,
    due_date: Option<String>,
    boards: Vec<BoardView>,
}

#[allow(dead_code)]
struct CardDetailView {
    id: String,
    title: String,
    body: Option<String>,
    status: String,
    due_date: Option<String>,
    created_at: String,
    boards: Vec<BoardView>,
    tags: Vec<TagView>,
}

#[allow(dead_code)]
struct CommentView {
    id: String,
    user_id: String,
    author_name: String,
    body: String,
    created_at: String,
    updated_at: String,
}

#[allow(dead_code)]
struct BoardView {
    id: String,
    name: String,
    description: Option<String>,
    role: String,
}

#[allow(dead_code)]
struct ColumnView {
    id: String,
    name: String,
    position: i32,
    cards: Vec<CardView>,
}

#[allow(dead_code)]
struct CardView {
    id: String,
    title: String,
    body: Option<String>,
    position: i32,
    visibility: String,
    tags: Vec<TagView>,
}

#[derive(Clone)]
#[allow(dead_code)]
struct TagView {
    id: String,
    name: String,
    color: String,
}

#[derive(Clone)]
#[allow(dead_code)]
struct FilterTagView {
    id: String,
    name: String,
    color: String,
    is_active: bool,
    toggle_url: String,
}

// Form structs
#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct RegisterForm {
    name: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct CreateBoardForm {
    name: String,
    description: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateColumnForm {
    name: String,
}

#[derive(Deserialize)]
pub struct CreateCardForm {
    column_id: Uuid,
    title: String,
    body: Option<String>,
}

#[derive(Deserialize)]
pub struct MoveCardForm {
    column_id: Uuid,
    position: i32,
}

#[derive(Deserialize)]
pub struct CreateTagForm {
    name: String,
    color: String,
}

#[derive(Deserialize)]
pub struct AddTagToCardForm {
    tag_id: Uuid,
}

#[derive(Deserialize, Default)]
pub struct BoardFilterQuery {
    #[serde(default)]
    tags: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct InboxQuery {
    #[serde(default)]
    status: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateInboxCardForm {
    title: String,
    body: Option<String>,
    due_date: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateCardStatusForm {
    status: String,
}

#[derive(Deserialize)]
pub struct CommentForm {
    body: String,
}

// Handlers
pub async fn index(auth: OptionalAuthUser) -> impl IntoResponse {
    let template = IndexTemplate {
        user: auth.0.map(|a| a.user.name),
    };
    Html(template.render().unwrap())
}

pub async fn login_page() -> impl IntoResponse {
    let template = LoginTemplate { error: None };
    Html(template.render().unwrap())
}

pub async fn login_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(input): Form<LoginForm>,
) -> Result<Response> {
    let user = match state.users.find_by_email(&input.email).await? {
        Some(u) => u,
        None => {
            let template = LoginTemplate {
                error: Some("Invalid email or password".to_string()),
            };
            return Ok(Html(template.render().unwrap()).into_response());
        }
    };

    if !verify_password(&input.password, &user.password_hash)? {
        let template = LoginTemplate {
            error: Some("Invalid email or password".to_string()),
        };
        return Ok(Html(template.render().unwrap()).into_response());
    }

    let token = generate_token();
    state.sessions.create(user.id, &token).await?;

    let cookie = Cookie::build(("session", token))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .build();

    Ok((jar.add(cookie), Redirect::to("/boards")).into_response())
}

pub async fn register_page() -> impl IntoResponse {
    let template = RegisterTemplate { error: None };
    Html(template.render().unwrap())
}

pub async fn register_submit(
    State(state): State<AppState>,
    Form(input): Form<RegisterForm>,
) -> Result<Response> {
    if input.name.is_empty() || input.email.is_empty() || input.password.is_empty() {
        let template = RegisterTemplate {
            error: Some("All fields are required".to_string()),
        };
        return Ok(Html(template.render().unwrap()).into_response());
    }

    if input.password.len() < 8 {
        let template = RegisterTemplate {
            error: Some("Password must be at least 8 characters".to_string()),
        };
        return Ok(Html(template.render().unwrap()).into_response());
    }

    if state.users.email_exists(&input.email).await? {
        let template = RegisterTemplate {
            error: Some("Email already registered".to_string()),
        };
        return Ok(Html(template.render().unwrap()).into_response());
    }

    let password_hash = hash_password(&input.password)?;
    let id = Uuid::new_v4();
    state
        .users
        .create(id, &input.email, &password_hash, &input.name)
        .await?;

    Ok(Redirect::to("/login").into_response())
}

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
    auth: AuthUser,
) -> Result<Response> {
    if let Some(token) = auth.session_token {
        state.sessions.delete_by_token(&token).await?;
    }

    let cookie = Cookie::build(("session", ""))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::seconds(0))
        .build();

    Ok((jar.add(cookie), Redirect::to("/")).into_response())
}

pub async fn boards_page(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse> {
    let boards = state.boards.list_for_user(auth.user.id).await?;

    let board_views: Vec<BoardView> = boards
        .into_iter()
        .map(|(b, role)| BoardView {
            id: b.id.to_string(),
            name: b.name,
            description: b.description,
            role,
        })
        .collect();

    let template = BoardsTemplate {
        user: auth.user.name,
        boards: board_views,
    };

    Ok(Html(template.render().unwrap()))
}

pub async fn new_board_page(auth: AuthUser) -> impl IntoResponse {
    let template = NewBoardTemplate {
        user: auth.user.name,
    };
    Html(template.render().unwrap())
}

pub async fn create_board_submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(input): Form<CreateBoardForm>,
) -> Result<Response> {
    let board = state
        .boards
        .create(&input.name, input.description.as_deref(), auth.user.id)
        .await?;

    Ok(Redirect::to(&format!("/boards/{}", board.id)).into_response())
}

pub async fn board_detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
    Query(filter): Query<BoardFilterQuery>,
) -> Result<impl IntoResponse> {
    let board = state.boards.get_by_id(board_id).await?;
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    let columns = state.columns.list_by_board(board_id).await?;
    let tags = state.tags.list_by_board(board_id).await?;

    // Parse active tag filters from query string (comma-separated UUIDs)
    let active_tag_ids: Vec<String> = filter
        .tags
        .as_ref()
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let active_tag_uuids: Vec<Uuid> = active_tag_ids
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .collect();

    let tag_views: Vec<TagView> = tags
        .iter()
        .map(|t| TagView {
            id: t.id.to_string(),
            name: t.name.clone(),
            color: t.color.clone(),
        })
        .collect();

    // Build filter tags with pre-computed toggle URLs
    let filter_tags: Vec<FilterTagView> = tags
        .iter()
        .map(|t| {
            let tag_id_str = t.id.to_string();
            let is_active = active_tag_ids.contains(&tag_id_str);

            let toggle_url = if is_active {
                // Remove this tag from filter
                let other_tags: Vec<&String> = active_tag_ids
                    .iter()
                    .filter(|id| *id != &tag_id_str)
                    .collect();
                if other_tags.is_empty() {
                    format!("/boards/{}", board_id)
                } else {
                    format!(
                        "/boards/{}?tags={}",
                        board_id,
                        other_tags
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                }
            } else {
                // Add this tag to filter
                if active_tag_ids.is_empty() {
                    format!("/boards/{}?tags={}", board_id, tag_id_str)
                } else {
                    format!(
                        "/boards/{}?tags={},{}",
                        board_id,
                        active_tag_ids.join(","),
                        tag_id_str
                    )
                }
            };

            FilterTagView {
                id: tag_id_str,
                name: t.name.clone(),
                color: t.color.clone(),
                is_active,
                toggle_url,
            }
        })
        .collect();

    let has_active_filters = !active_tag_ids.is_empty();

    let mut column_views = Vec::new();
    for col in columns {
        let cards = state.cards.list_by_column(col.id).await?;
        let mut card_views = Vec::new();
        for card in cards {
            let card_tags = state.tags.list_for_card(card.id).await?;

            // Filter: if active tags are set, only show cards that have ALL of them
            if !active_tag_uuids.is_empty() {
                let card_tag_ids: Vec<Uuid> = card_tags.iter().map(|t| t.id).collect();
                let has_all_tags = active_tag_uuids.iter().all(|t| card_tag_ids.contains(t));
                if !has_all_tags {
                    continue;
                }
            }

            card_views.push(CardView {
                id: card.id.to_string(),
                title: card.title,
                body: card.body,
                position: card.position,
                visibility: card.visibility,
                tags: card_tags
                    .into_iter()
                    .map(|t| TagView {
                        id: t.id.to_string(),
                        name: t.name,
                        color: t.color,
                    })
                    .collect(),
            });
        }
        column_views.push(ColumnView {
            id: col.id.to_string(),
            name: col.name,
            position: col.position,
            cards: card_views,
        });
    }

    let template = BoardDetailTemplate {
        user: auth.user.name,
        board: BoardView {
            id: board.id.to_string(),
            name: board.name,
            description: board.description,
            role: role.to_string(),
        },
        columns: column_views,
        tags: tag_views,
        filter_tags,
        has_active_filters,
    };

    Ok(Html(template.render().unwrap()))
}

pub async fn board_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let board = state.boards.get_by_id(board_id).await?;
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    let tags = state.tags.list_by_board(board_id).await?;
    let tag_views: Vec<TagView> = tags
        .into_iter()
        .map(|t| TagView {
            id: t.id.to_string(),
            name: t.name,
            color: t.color,
        })
        .collect();

    let template = BoardSettingsTemplate {
        user: auth.user.name,
        board: BoardView {
            id: board.id.to_string(),
            name: board.name,
            description: board.description,
            role: role.to_string(),
        },
        tags: tag_views,
    };

    Ok(Html(template.render().unwrap()))
}

pub async fn create_column_submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
    Form(input): Form<CreateColumnForm>,
) -> Result<Response> {
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    state.columns.create(board_id, &input.name, None).await?;

    Ok(Redirect::to(&format!("/boards/{}", board_id)).into_response())
}

pub async fn create_card_submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
    Form(input): Form<CreateCardForm>,
) -> Result<Response> {
    let column = state.columns.get_by_id(input.column_id).await?;

    let role = state
        .boards
        .get_user_role(column.board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    state
        .cards
        .create(
            input.column_id,
            &input.title,
            input.body.as_deref(),
            None,
            CardVisibility::Restricted,
            CardStatus::Open,
            None,
            None,
            None,
            auth.user.id,
        )
        .await?;

    Ok(Redirect::to(&format!("/boards/{}", board_id)).into_response())
}

pub async fn move_card_submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(card_id): Path<Uuid>,
    Form(input): Form<MoveCardForm>,
) -> Result<Response> {
    let board_id = state.cards.get_board_id_for_card(card_id).await?;

    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    state
        .cards
        .move_card(card_id, input.column_id, input.position)
        .await?;

    Ok(Redirect::to(&format!("/boards/{}", board_id)).into_response())
}

pub async fn create_tag_submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
    Form(input): Form<CreateTagForm>,
) -> Result<Response> {
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    state
        .tags
        .create(board_id, &input.name, &input.color)
        .await?;

    Ok(Redirect::to(&format!("/boards/{}/settings", board_id)).into_response())
}

pub async fn delete_tag_submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((board_id, tag_id)): Path<(Uuid, Uuid)>,
) -> Result<Response> {
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    state.tags.delete(tag_id).await?;

    Ok(Redirect::to(&format!("/boards/{}/settings", board_id)).into_response())
}

pub async fn delete_column_submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(column_id): Path<Uuid>,
) -> Result<Response> {
    let column = state.columns.get_by_id(column_id).await?;
    let board_id = column.board_id;

    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    state.columns.delete(column_id).await?;

    Ok(Redirect::to(&format!("/boards/{}", board_id)).into_response())
}

pub async fn delete_board_submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
) -> Result<Response> {
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_delete() {
        return Err(AppError::Forbidden);
    }

    state.boards.delete(board_id).await?;

    Ok(Redirect::to("/boards").into_response())
}

pub async fn add_tag_to_card_submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(card_id): Path<Uuid>,
    Form(input): Form<AddTagToCardForm>,
) -> Result<Response> {
    let board_id = state.cards.get_board_id_for_card(card_id).await?;

    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !role.can_edit() {
        return Err(AppError::Forbidden);
    }

    state.tags.add_to_card(card_id, input.tag_id).await?;

    Ok(Redirect::to(&format!("/boards/{}", board_id)).into_response())
}

pub async fn remove_tag_from_card_submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((card_id, tag_id)): Path<(Uuid, Uuid)>,
) -> Result<Response> {
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

    Ok(Redirect::to(&format!("/boards/{}", board_id)).into_response())
}

// User settings handlers
pub async fn user_settings(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse> {
    let chat_message_count = state.chat_messages.count_by_user(auth.user.id).await?;

    let template = UserSettingsTemplate {
        user: auth.user.name.clone(),
        chat_message_count,
        llm_context: auth.user.llm_context,
    };

    Ok(Html(template.render().unwrap()))
}

#[derive(Deserialize)]
pub struct UpdateLlmContextForm {
    llm_context: Option<String>,
}

pub async fn update_llm_context_submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(input): Form<UpdateLlmContextForm>,
) -> Result<Response> {
    // Trim and convert empty string to None
    let context = input
        .llm_context
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    state
        .users
        .update_llm_context(auth.user.id, context.as_deref())
        .await?;

    Ok(Redirect::to("/settings").into_response())
}

pub async fn delete_chat_history_submit(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Response> {
    state.chat_messages.delete_all_by_user(auth.user.id).await?;

    Ok(Redirect::to("/settings").into_response())
}

// Inbox handlers
pub async fn inbox_page(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<InboxQuery>,
) -> Result<Response> {
    let status = query
        .status
        .as_ref()
        .and_then(|s| s.parse::<CardStatus>().ok());

    let cards = state
        .cards
        .list_by_owner_with_status(auth.user.id, status)
        .await?;

    let mut card_views = Vec::new();
    for card in cards {
        // Get boards this card is assigned to
        let boards = state.card_boards.list_boards_for_card(card.id).await?;
        let board_views: Vec<BoardView> = boards
            .into_iter()
            .map(|b| BoardView {
                id: b.id.to_string(),
                name: b.name,
                description: b.description,
                role: "owner".to_string(), // Simplified for inbox view
            })
            .collect();

        card_views.push(InboxCardView {
            id: card.id.to_string(),
            title: card.title,
            body: card.body,
            status: card.status,
            due_date: card.due_date.map(|d| d.to_string()),
            boards: board_views,
        });
    }

    let template = InboxTemplate {
        user: auth.user.name,
        cards: card_views,
        current_status: query.status,
    };

    Ok(Html(template.render().unwrap()).into_response())
}

pub async fn create_inbox_card_submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(input): Form<CreateInboxCardForm>,
) -> Result<Response> {
    use chrono::NaiveDate;

    if input.title.trim().is_empty() {
        return Err(AppError::Validation("Card title is required".to_string()));
    }

    let due_date = input.due_date.and_then(|s| {
        if s.is_empty() {
            None
        } else {
            NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()
        }
    });

    state
        .cards
        .create_standalone(
            input.title.trim(),
            input.body.as_deref(),
            CardVisibility::Private,
            CardStatus::Open,
            None,
            None,
            due_date,
            auth.user.id,
        )
        .await?;

    Ok(Redirect::to("/inbox").into_response())
}

pub async fn update_card_status_submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(card_id): Path<Uuid>,
    Form(input): Form<UpdateCardStatusForm>,
) -> Result<Response> {
    let card = state.cards.get_by_id(card_id).await?;

    // Verify ownership
    if card.owner_id != Some(auth.user.id) && card.created_by != auth.user.id {
        return Err(AppError::Forbidden);
    }

    let status: CardStatus = input
        .status
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid status".to_string()))?;

    state.cards.update_status(card_id, status).await?;

    Ok(Redirect::to("/inbox").into_response())
}

/// Card detail page with comments
pub async fn card_detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(card_id): Path<Uuid>,
) -> Result<Response> {
    let card = state.cards.get_by_id(card_id).await?;

    // Check access
    let has_access = card.owner_id == Some(auth.user.id)
        || card.created_by == auth.user.id
        || {
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

    // Get boards for this card
    let boards = state.card_boards.list_boards_for_card(card_id).await?;
    let mut board_views = Vec::new();
    for board in boards {
        let role = state
            .boards
            .get_user_role(board.id, auth.user.id)
            .await?
            .map(|r| r.to_string())
            .unwrap_or_else(|| "none".to_string());
        board_views.push(BoardView {
            id: board.id.to_string(),
            name: board.name,
            description: board.description,
            role,
        });
    }

    // Get tags for this card
    let tags = state.tags.list_for_card(card_id).await?;
    let tag_views: Vec<TagView> = tags
        .into_iter()
        .map(|t| TagView {
            id: t.id.to_string(),
            name: t.name,
            color: t.color,
        })
        .collect();

    // Get comments
    let comments = state.comments.list_by_card(card_id).await?;
    let comment_views: Vec<CommentView> = comments
        .into_iter()
        .map(|c| CommentView {
            id: c.id.to_string(),
            user_id: c.user_id.to_string(),
            author_name: c.author_name,
            body: c.body,
            created_at: c.created_at.format("%Y-%m-%d %H:%M").to_string(),
            updated_at: c.updated_at.format("%Y-%m-%d %H:%M").to_string(),
        })
        .collect();

    let card_view = CardDetailView {
        id: card.id.to_string(),
        title: card.title,
        body: card.body,
        status: card.status.to_string(),
        due_date: card.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
        created_at: card.created_at.format("%Y-%m-%d %H:%M").to_string(),
        boards: board_views,
        tags: tag_views,
    };

    let template = CardDetailTemplate {
        user: auth.user.name,
        card: card_view,
        comments: comment_views,
        current_user_id: auth.user.id.to_string(),
    };

    Ok(Html(template.render().unwrap()).into_response())
}

/// Add a comment to a card
pub async fn add_comment_submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(card_id): Path<Uuid>,
    Form(input): Form<CommentForm>,
) -> Result<Response> {
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

    if input.body.trim().is_empty() {
        return Err(AppError::Validation("Comment body is required".to_string()));
    }

    state
        .comments
        .create(card_id, auth.user.id, &input.body)
        .await?;

    Ok(Redirect::to(&format!("/cards/{}", card_id)).into_response())
}

/// Delete a comment
pub async fn delete_comment_submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((card_id, comment_id)): Path<(Uuid, Uuid)>,
) -> Result<Response> {
    let comment = state.comments.get_by_id(comment_id).await?;

    // Only the author can delete their comment
    if comment.user_id != auth.user.id {
        return Err(AppError::Forbidden);
    }

    state.comments.delete(comment_id).await?;

    Ok(Redirect::to(&format!("/cards/{}", card_id)).into_response())
}
