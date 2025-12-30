use sqlx::SqlitePool;
use std::sync::Arc;

use crate::repo::{
    board::BoardRepository, card::CardRepository, column::ColumnRepository,
    session::SessionRepository, tag::TagRepository, token::ApiTokenRepository,
    user::UserRepository,
};
use crate::services::OllamaClient;

#[derive(Clone)]
pub struct AppState {
    pub users: UserRepository,
    pub sessions: SessionRepository,
    pub tokens: ApiTokenRepository,
    pub boards: BoardRepository,
    pub columns: ColumnRepository,
    pub cards: CardRepository,
    pub tags: TagRepository,
    pub ollama: OllamaClient,
    pub pool: Arc<SqlitePool>,
}

impl AppState {
    pub fn new(pool: SqlitePool) -> Self {
        let pool = Arc::new(pool);
        Self {
            users: UserRepository::new(pool.clone()),
            sessions: SessionRepository::new(pool.clone()),
            tokens: ApiTokenRepository::new(pool.clone()),
            boards: BoardRepository::new(pool.clone()),
            columns: ColumnRepository::new(pool.clone()),
            cards: CardRepository::new(pool.clone()),
            tags: TagRepository::new(pool.clone()),
            ollama: OllamaClient::from_env(),
            pool,
        }
    }
}
