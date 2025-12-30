mod auth;
mod error;
mod handlers;
mod models;
mod repo;
mod state;

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use sqlx::sqlite::SqlitePoolOptions;
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "personal_os=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data.db?mode=rwc".to_string());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let state = AppState::new(pool);

    let app = create_router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

pub fn create_router(state: AppState) -> Router {
    let api_routes = Router::new()
        // Auth routes
        .route("/auth/register", post(handlers::auth::register))
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/logout", post(handlers::auth::logout))
        .route("/auth/tokens", post(handlers::auth::create_api_token))
        .route("/auth/tokens", get(handlers::auth::list_api_tokens))
        .route(
            "/auth/tokens/:token_id",
            delete(handlers::auth::revoke_api_token),
        )
        // Board routes
        .route("/boards", post(handlers::boards::create_board))
        .route("/boards", get(handlers::boards::list_boards))
        .route("/boards/:board_id", get(handlers::boards::get_board))
        .route("/boards/:board_id", put(handlers::boards::update_board))
        .route("/boards/:board_id", delete(handlers::boards::delete_board))
        .route(
            "/boards/:board_id/permissions",
            post(handlers::boards::add_permission),
        )
        .route(
            "/boards/:board_id/permissions/:user_id",
            delete(handlers::boards::remove_permission),
        )
        // Column routes
        .route(
            "/boards/:board_id/columns",
            post(handlers::columns::create_column),
        )
        .route(
            "/boards/:board_id/columns",
            get(handlers::columns::list_columns),
        )
        .route("/columns/:column_id", put(handlers::columns::update_column))
        .route(
            "/columns/:column_id",
            delete(handlers::columns::delete_column),
        )
        .route(
            "/columns/:column_id/move",
            patch(handlers::columns::move_column),
        )
        // Card routes
        .route(
            "/columns/:column_id/cards",
            post(handlers::cards::create_card),
        )
        .route("/boards/:board_id/cards", get(handlers::cards::list_cards))
        .route("/cards/:card_id", get(handlers::cards::get_card))
        .route("/cards/:card_id", put(handlers::cards::update_card))
        .route("/cards/:card_id", delete(handlers::cards::delete_card))
        .route("/cards/:card_id/move", patch(handlers::cards::move_card))
        // Tag routes
        .route("/boards/:board_id/tags", post(handlers::tags::create_tag))
        .route("/boards/:board_id/tags", get(handlers::tags::list_tags))
        .route("/tags/:tag_id", put(handlers::tags::update_tag))
        .route("/tags/:tag_id", delete(handlers::tags::delete_tag))
        .route(
            "/cards/:card_id/tags/:tag_id",
            post(handlers::tags::add_tag_to_card),
        )
        .route(
            "/cards/:card_id/tags/:tag_id",
            delete(handlers::tags::remove_tag_from_card),
        );

    let web_routes = Router::new()
        .route("/", get(handlers::web::index))
        .route("/login", get(handlers::web::login_page))
        .route("/login", post(handlers::web::login_submit))
        .route("/register", get(handlers::web::register_page))
        .route("/register", post(handlers::web::register_submit))
        .route("/logout", post(handlers::web::logout))
        .route("/boards", get(handlers::web::boards_page))
        .route("/boards/new", get(handlers::web::new_board_page))
        .route("/boards/new", post(handlers::web::create_board_submit))
        .route("/boards/:board_id", get(handlers::web::board_detail))
        .route(
            "/boards/:board_id/settings",
            get(handlers::web::board_settings),
        )
        .route(
            "/boards/:board_id/columns/new",
            post(handlers::web::create_column_submit),
        )
        .route(
            "/boards/:board_id/cards/new",
            post(handlers::web::create_card_submit),
        )
        .route(
            "/cards/:card_id/move",
            post(handlers::web::move_card_submit),
        );

    Router::new()
        .nest("/api", api_routes)
        .merge(web_routes)
        .nest_service("/static", ServeDir::new("src/static"))
        .with_state(state)
}
