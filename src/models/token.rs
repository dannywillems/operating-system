use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum TokenScope {
    #[sqlx(rename = "read")]
    Read,
    #[sqlx(rename = "write")]
    Write,
    #[sqlx(rename = "admin")]
    Admin,
}

impl std::fmt::Display for TokenScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenScope::Read => write!(f, "read"),
            TokenScope::Write => write!(f, "write"),
            TokenScope::Admin => write!(f, "admin"),
        }
    }
}

impl std::str::FromStr for TokenScope {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "read" => Ok(TokenScope::Read),
            "write" => Ok(TokenScope::Write),
            "admin" => Ok(TokenScope::Admin),
            _ => Err(format!("Invalid scope: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub token_hash: String,
    pub scope: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiToken {
    pub name: String,
    pub scope: TokenScope,
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ApiTokenResponse {
    pub id: Uuid,
    pub name: String,
    pub scope: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ApiTokenCreatedResponse {
    pub id: Uuid,
    pub token: String,
    pub name: String,
    pub scope: String,
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<ApiToken> for ApiTokenResponse {
    fn from(token: ApiToken) -> Self {
        Self {
            id: token.id,
            name: token.name,
            scope: token.scope,
            expires_at: token.expires_at,
            created_at: token.created_at,
            last_used_at: token.last_used_at,
        }
    }
}
