use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{generate_token, hash_password, verify_password, AuthUser, OptionalAuthUser};
use crate::error::{AppError, Result};
use crate::models::CardVisibility;
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
}

#[derive(Template)]
#[template(path = "board_settings.html")]
struct BoardSettingsTemplate {
    user: String,
    board: BoardView,
}

// View structs for templates
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
) -> Result<impl IntoResponse> {
    let board = state.boards.get_by_id(board_id).await?;
    let role = state
        .boards
        .get_user_role(board_id, auth.user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    let columns = state.columns.list_by_board(board_id).await?;
    let tags = state.tags.list_by_board(board_id).await?;

    let tag_views: Vec<TagView> = tags
        .iter()
        .map(|t| TagView {
            id: t.id.to_string(),
            name: t.name.clone(),
            color: t.color.clone(),
        })
        .collect();

    let mut column_views = Vec::new();
    for col in columns {
        let cards = state.cards.list_by_column(col.id).await?;
        let mut card_views = Vec::new();
        for card in cards {
            let card_tags = state.tags.list_for_card(card.id).await?;
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

    let template = BoardSettingsTemplate {
        user: auth.user.name,
        board: BoardView {
            id: board.id.to_string(),
            name: board.name,
            description: board.description,
            role: role.to_string(),
        },
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
