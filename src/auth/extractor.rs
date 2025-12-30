use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::extract::CookieJar;

use crate::auth::hash_token;
use crate::error::AppError;
use crate::models::User;
use crate::state::AppState;

pub struct AuthUser {
    pub user: User,
    pub session_token: Option<String>,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);
        let cookies = CookieJar::from_request_parts(parts, &state)
            .await
            .map_err(|_| AppError::Unauthorized)?;

        // Check session cookie first
        if let Some(session_cookie) = cookies.get("session") {
            let token = session_cookie.value();
            if let Some(session) = state.sessions.find_by_token(token).await? {
                if let Some(user) = state.users.find_by_id(session.user_id).await? {
                    return Ok(AuthUser {
                        user,
                        session_token: Some(token.to_string()),
                    });
                }
            }
        }

        // Check Authorization header for API token
        if let Some(auth_header) = parts.headers.get("Authorization") {
            let auth_str = auth_header.to_str().map_err(|_| AppError::Unauthorized)?;

            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                let token_hash = hash_token(token);
                if let Some(api_token) = state.tokens.find_by_hash(&token_hash).await? {
                    state.tokens.update_last_used(api_token.id).await?;
                    if let Some(user) = state.users.find_by_id(api_token.user_id).await? {
                        return Ok(AuthUser {
                            user,
                            session_token: None,
                        });
                    }
                }
            }
        }

        Err(AppError::Unauthorized)
    }
}

pub struct OptionalAuthUser(pub Option<AuthUser>);

#[axum::async_trait]
impl<S> FromRequestParts<S> for OptionalAuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match AuthUser::from_request_parts(parts, state).await {
            Ok(user) => Ok(OptionalAuthUser(Some(user))),
            Err(_) => Ok(OptionalAuthUser(None)),
        }
    }
}
