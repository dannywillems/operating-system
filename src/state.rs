use sqlx::SqlitePool;
use std::sync::Arc;

use crate::repo::{
    board::BoardRepository, card::CardRepository, card_board::CardBoardRepository,
    chat_message::ChatMessageRepository, column::ColumnRepository, comment::CommentRepository,
    session::SessionRepository, tag::TagRepository, token::ApiTokenRepository, user::UserRepository,
};
use crate::services::{OllamaClient, WebSearchClient};

#[derive(Clone)]
pub struct AppState {
    pub users: UserRepository,
    pub sessions: SessionRepository,
    pub tokens: ApiTokenRepository,
    pub boards: BoardRepository,
    pub columns: ColumnRepository,
    pub cards: CardRepository,
    pub card_boards: CardBoardRepository,
    pub tags: TagRepository,
    pub comments: CommentRepository,
    pub chat_messages: ChatMessageRepository,
    pub ollama: OllamaClient,
    pub web_search: WebSearchClient,
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
            card_boards: CardBoardRepository::new(pool.clone()),
            tags: TagRepository::new(pool.clone()),
            comments: CommentRepository::new(pool.clone()),
            chat_messages: ChatMessageRepository::new(pool.clone()),
            ollama: OllamaClient::from_env(),
            web_search: WebSearchClient::new(),
            pool,
        }
    }
}
