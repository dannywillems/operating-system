use axum::{
    extract::{Path, State},
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{generate_token, hash_password, hash_token, verify_password, AuthUser};
use crate::error::{AppError, Result};
use crate::models::{ApiTokenCreatedResponse, ApiTokenResponse, CreateApiToken, CreateUser, UserResponse};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub token: Option<String>,
}

pub async fn register(
    State(state): State<AppState>,
    Json(input): Json<CreateUser>,
) -> Result<Json<AuthResponse>> {
    // Validate input
    if input.email.is_empty() || input.password.is_empty() || input.name.is_empty() {
        return Err(AppError::Validation("All fields are required".to_string()));
    }

    if input.password.len() < 8 {
        return Err(AppError::Validation(
            "Password must be at least 8 characters".to_string(),
        ));
    }

    // Check if email already exists
    if state.users.email_exists(&input.email).await? {
        return Err(AppError::BadRequest("Email already registered".to_string()));
    }

    // Hash password and create user
    let password_hash = hash_password(&input.password)?;
    let id = Uuid::new_v4();
    let user = state
        .users
        .create(id, &input.email, &password_hash, &input.name)
        .await?;

    Ok(Json(AuthResponse {
        user: user.into(),
        token: None,
    }))
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<LoginRequest>,
) -> Result<(CookieJar, Json<AuthResponse>)> {
    let user = state
        .users
        .find_by_email(&input.email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !verify_password(&input.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    // Create session
    let token = generate_token();
    state.sessions.create(user.id, &token).await?;

    let cookie = Cookie::build(("session", token))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .build();

    Ok((
        jar.add(cookie),
        Json(AuthResponse {
            user: user.into(),
            token: None,
        }),
    ))
}

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
    auth: AuthUser,
) -> Result<CookieJar> {
    if let Some(token) = auth.session_token {
        state.sessions.delete_by_token(&token).await?;
    }

    let cookie = Cookie::build(("session", ""))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::seconds(0))
        .build();

    Ok(jar.add(cookie))
}

pub async fn create_api_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateApiToken>,
) -> Result<Json<ApiTokenCreatedResponse>> {
    if input.name.is_empty() {
        return Err(AppError::Validation("Token name is required".to_string()));
    }

    let token = generate_token();
    let token_hash = hash_token(&token);

    let api_token = state
        .tokens
        .create(
            auth.user.id,
            &input.name,
            &token_hash,
            input.scope,
            input.expires_in_days,
        )
        .await?;

    Ok(Json(ApiTokenCreatedResponse {
        id: api_token.id,
        token,
        name: api_token.name,
        scope: api_token.scope,
        expires_at: api_token.expires_at,
    }))
}

pub async fn list_api_tokens(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ApiTokenResponse>>> {
    let tokens = state.tokens.list_by_user(auth.user.id).await?;
    Ok(Json(tokens.into_iter().map(|t| t.into()).collect()))
}

pub async fn revoke_api_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(token_id): Path<Uuid>,
) -> Result<()> {
    state.tokens.delete(token_id, auth.user.id).await?;
    Ok(())
}
