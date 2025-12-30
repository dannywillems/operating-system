use axum_test::TestServer;
use cookie::Cookie;
use personal_os::{create_router, state::AppState, test_utils};
use serde_json::{json, Value};

async fn setup_server() -> TestServer {
    let state = test_utils::create_test_state().await;
    let app = create_router(state);
    TestServer::new(app).unwrap()
}

#[allow(dead_code)]
async fn setup_server_with_state() -> (TestServer, AppState) {
    let state = test_utils::create_test_state().await;
    let app = create_router(state.clone());
    (TestServer::new(app).unwrap(), state)
}

async fn register_and_login(server: &TestServer) -> String {
    let email = format!("test_{}@example.com", uuid::Uuid::new_v4());

    server
        .post("/api/auth/register")
        .json(&json!({
            "email": email,
            "password": "testpassword123",
            "name": "Test User"
        }))
        .await;

    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "email": email,
            "password": "testpassword123"
        }))
        .await;

    login_response.cookie("session").value().to_string()
}

fn session_cookie(token: &str) -> Cookie<'static> {
    Cookie::new("session", token.to_string())
}

// ============================================================================
// Auth Tests
// ============================================================================

mod auth_tests {
    use super::*;

    #[tokio::test]
    async fn test_register_success() {
        let server = setup_server().await;

        let response = server
            .post("/api/auth/register")
            .json(&json!({
                "email": "newuser@example.com",
                "password": "securepassword123",
                "name": "New User"
            }))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert!(body["user"]["id"].is_string());
        assert_eq!(body["user"]["email"], "newuser@example.com");
        assert_eq!(body["user"]["name"], "New User");
    }

    #[tokio::test]
    async fn test_register_missing_fields() {
        let server = setup_server().await;

        let response = server
            .post("/api/auth/register")
            .json(&json!({
                "email": "",
                "password": "password123",
                "name": "User"
            }))
            .await;

        response.assert_status_unprocessable_entity();
    }

    #[tokio::test]
    async fn test_register_short_password() {
        let server = setup_server().await;

        let response = server
            .post("/api/auth/register")
            .json(&json!({
                "email": "user@example.com",
                "password": "short",
                "name": "User"
            }))
            .await;

        response.assert_status_unprocessable_entity();
    }

    #[tokio::test]
    async fn test_register_duplicate_email() {
        let server = setup_server().await;

        server
            .post("/api/auth/register")
            .json(&json!({
                "email": "duplicate@example.com",
                "password": "password123",
                "name": "First User"
            }))
            .await
            .assert_status_ok();

        let response = server
            .post("/api/auth/register")
            .json(&json!({
                "email": "duplicate@example.com",
                "password": "password456",
                "name": "Second User"
            }))
            .await;

        response.assert_status_bad_request();
    }

    #[tokio::test]
    async fn test_login_success() {
        let server = setup_server().await;

        server
            .post("/api/auth/register")
            .json(&json!({
                "email": "login@example.com",
                "password": "password123",
                "name": "Login User"
            }))
            .await;

        let response = server
            .post("/api/auth/login")
            .json(&json!({
                "email": "login@example.com",
                "password": "password123"
            }))
            .await;

        response.assert_status_ok();
        let session_cookie = response.maybe_cookie("session");
        assert!(session_cookie.is_some());
    }

    #[tokio::test]
    async fn test_login_invalid_password() {
        let server = setup_server().await;

        server
            .post("/api/auth/register")
            .json(&json!({
                "email": "loginbad@example.com",
                "password": "password123",
                "name": "User"
            }))
            .await;

        let response = server
            .post("/api/auth/login")
            .json(&json!({
                "email": "loginbad@example.com",
                "password": "wrongpassword"
            }))
            .await;

        response.assert_status_unauthorized();
    }

    #[tokio::test]
    async fn test_login_nonexistent_user() {
        let server = setup_server().await;

        let response = server
            .post("/api/auth/login")
            .json(&json!({
                "email": "nonexistent@example.com",
                "password": "password123"
            }))
            .await;

        response.assert_status_unauthorized();
    }

    #[tokio::test]
    async fn test_logout() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;

        let response = server
            .post("/api/auth/logout")
            .add_cookie(session_cookie(&session))
            .await;

        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_create_api_token() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;

        let response = server
            .post("/api/auth/tokens")
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "name": "My Token",
                "scope": "Read"
            }))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert!(body["id"].is_string());
        assert!(body["token"].is_string());
        assert_eq!(body["name"], "My Token");
    }

    #[tokio::test]
    async fn test_list_api_tokens() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;

        server
            .post("/api/auth/tokens")
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "name": "Token 1",
                "scope": "Read"
            }))
            .await;

        let response = server
            .get("/api/auth/tokens")
            .add_cookie(session_cookie(&session))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert!(body.as_array().unwrap().len() >= 1);
    }

    #[tokio::test]
    async fn test_revoke_api_token() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;

        let create_response = server
            .post("/api/auth/tokens")
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "name": "Token to Delete",
                "scope": "Read"
            }))
            .await;

        let token_id = create_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let response = server
            .delete(&format!("/api/auth/tokens/{}", token_id))
            .add_cookie(session_cookie(&session))
            .await;

        response.assert_status_ok();
    }
}

// ============================================================================
// Board Tests
// ============================================================================

mod board_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_board() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;

        let response = server
            .post("/api/boards")
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "name": "Test Board",
                "description": "A test board"
            }))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert!(body["id"].is_string());
        assert_eq!(body["name"], "Test Board");
        assert_eq!(body["description"], "A test board");
        assert_eq!(body["role"], "owner");
    }

    #[tokio::test]
    async fn test_create_board_without_description() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;

        let response = server
            .post("/api/boards")
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "name": "Minimal Board"
            }))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["name"], "Minimal Board");
    }

    #[tokio::test]
    async fn test_create_board_empty_name() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;

        let response = server
            .post("/api/boards")
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "name": ""
            }))
            .await;

        response.assert_status_unprocessable_entity();
    }

    #[tokio::test]
    async fn test_list_boards() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;

        server
            .post("/api/boards")
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Board 1"}))
            .await;

        server
            .post("/api/boards")
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Board 2"}))
            .await;

        let response = server
            .get("/api/boards")
            .add_cookie(session_cookie(&session))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_get_board() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;

        let create_response = server
            .post("/api/boards")
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Get Test Board"}))
            .await;

        let board_id = create_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let response = server
            .get(&format!("/api/boards/{}", board_id))
            .add_cookie(session_cookie(&session))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["name"], "Get Test Board");
    }

    #[tokio::test]
    async fn test_update_board() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;

        let create_response = server
            .post("/api/boards")
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Original Name"}))
            .await;

        let board_id = create_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let response = server
            .put(&format!("/api/boards/{}", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "name": "Updated Name",
                "description": "New description"
            }))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["name"], "Updated Name");
        assert_eq!(body["description"], "New description");
    }

    #[tokio::test]
    async fn test_delete_board() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;

        let create_response = server
            .post("/api/boards")
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Board to Delete"}))
            .await;

        let board_id = create_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let response = server
            .delete(&format!("/api/boards/{}", board_id))
            .add_cookie(session_cookie(&session))
            .await;

        response.assert_status_ok();

        let get_response = server
            .get(&format!("/api/boards/{}", board_id))
            .add_cookie(session_cookie(&session))
            .await;

        get_response.assert_status_not_found();
    }

    #[tokio::test]
    async fn test_unauthorized_board_access() {
        let server = setup_server().await;

        let response = server.get("/api/boards").await;

        response.assert_status_unauthorized();
    }
}

// ============================================================================
// Column Tests
// ============================================================================

mod column_tests {
    use super::*;

    async fn create_board(server: &TestServer, session: &str) -> String {
        let response = server
            .post("/api/boards")
            .add_cookie(session_cookie(session))
            .json(&json!({"name": "Test Board"}))
            .await;

        response.json::<Value>()["id"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn test_create_column() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let board_id = create_board(&server, &session).await;

        let response = server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "To Do"}))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["name"], "To Do");
        assert_eq!(body["position"], 0);
    }

    #[tokio::test]
    async fn test_create_multiple_columns() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let board_id = create_board(&server, &session).await;

        server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "To Do"}))
            .await;

        let response = server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "In Progress"}))
            .await;

        let body: Value = response.json();
        assert_eq!(body["position"], 1);
    }

    #[tokio::test]
    async fn test_list_columns() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let board_id = create_board(&server, &session).await;

        server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Column 1"}))
            .await;

        server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Column 2"}))
            .await;

        let response = server
            .get(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_update_column() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let board_id = create_board(&server, &session).await;

        let create_response = server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Original"}))
            .await;

        let column_id = create_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let response = server
            .put(&format!("/api/columns/{}", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Updated"}))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["name"], "Updated");
    }

    #[tokio::test]
    async fn test_delete_column() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let board_id = create_board(&server, &session).await;

        let create_response = server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "To Delete"}))
            .await;

        let column_id = create_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let response = server
            .delete(&format!("/api/columns/{}", column_id))
            .add_cookie(session_cookie(&session))
            .await;

        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_move_column() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let board_id = create_board(&server, &session).await;

        let col1_response = server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Column 1"}))
            .await;
        let col1_id = col1_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Column 2"}))
            .await;

        let response = server
            .patch(&format!("/api/columns/{}/move", col1_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"position": 1}))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["position"], 1);
    }
}

// ============================================================================
// Card Tests
// ============================================================================

mod card_tests {
    use super::*;

    async fn create_board_and_column(server: &TestServer, session: &str) -> (String, String) {
        let board_response = server
            .post("/api/boards")
            .add_cookie(session_cookie(session))
            .json(&json!({"name": "Test Board"}))
            .await;
        let board_id = board_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let column_response = server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(session))
            .json(&json!({"name": "To Do"}))
            .await;
        let column_id = column_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        (board_id, column_id)
    }

    #[tokio::test]
    async fn test_create_card() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let (_board_id, column_id) = create_board_and_column(&server, &session).await;

        let response = server
            .post(&format!("/api/columns/{}/cards", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "title": "Test Card",
                "body": "Card description"
            }))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["title"], "Test Card");
        assert_eq!(body["body"], "Card description");
    }

    #[tokio::test]
    async fn test_create_card_with_visibility() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let (_board_id, column_id) = create_board_and_column(&server, &session).await;

        let response = server
            .post(&format!("/api/columns/{}/cards", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "title": "Private Card",
                "visibility": "Private"
            }))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["visibility"], "private");
    }

    #[tokio::test]
    async fn test_list_cards() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let (board_id, column_id) = create_board_and_column(&server, &session).await;

        server
            .post(&format!("/api/columns/{}/cards", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"title": "Card 1"}))
            .await;

        server
            .post(&format!("/api/columns/{}/cards", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"title": "Card 2"}))
            .await;

        let response = server
            .get(&format!("/api/boards/{}/cards", board_id))
            .add_cookie(session_cookie(&session))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_get_card() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let (_board_id, column_id) = create_board_and_column(&server, &session).await;

        let create_response = server
            .post(&format!("/api/columns/{}/cards", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"title": "Get Test Card"}))
            .await;
        let card_id = create_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let response = server
            .get(&format!("/api/cards/{}", card_id))
            .add_cookie(session_cookie(&session))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["title"], "Get Test Card");
    }

    #[tokio::test]
    async fn test_update_card() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let (_board_id, column_id) = create_board_and_column(&server, &session).await;

        let create_response = server
            .post(&format!("/api/columns/{}/cards", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"title": "Original Title"}))
            .await;
        let card_id = create_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let response = server
            .put(&format!("/api/cards/{}", card_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "title": "Updated Title",
                "body": "Updated body"
            }))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["title"], "Updated Title");
        assert_eq!(body["body"], "Updated body");
    }

    #[tokio::test]
    async fn test_delete_card() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let (_board_id, column_id) = create_board_and_column(&server, &session).await;

        let create_response = server
            .post(&format!("/api/columns/{}/cards", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"title": "Card to Delete"}))
            .await;
        let card_id = create_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let response = server
            .delete(&format!("/api/cards/{}", card_id))
            .add_cookie(session_cookie(&session))
            .await;

        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_move_card_between_columns() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;

        let board_response = server
            .post("/api/boards")
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Test Board"}))
            .await;
        let board_id = board_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let col1_response = server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "To Do"}))
            .await;
        let col1_id = col1_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let col2_response = server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Done"}))
            .await;
        let col2_id = col2_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let card_response = server
            .post(&format!("/api/columns/{}/cards", col1_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"title": "Card to Move"}))
            .await;
        let card_id = card_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let response = server
            .patch(&format!("/api/cards/{}/move", card_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "column_id": col2_id,
                "position": 0
            }))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["column_id"], col2_id);
        assert_eq!(body["position"], 0);
    }

    #[tokio::test]
    async fn test_move_card_within_column() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let (_board_id, column_id) = create_board_and_column(&server, &session).await;

        server
            .post(&format!("/api/columns/{}/cards", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"title": "Card 1"}))
            .await;

        let card2_response = server
            .post(&format!("/api/columns/{}/cards", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"title": "Card 2"}))
            .await;
        let card2_id = card2_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let response = server
            .patch(&format!("/api/cards/{}/move", card2_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "column_id": column_id,
                "position": 0
            }))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["position"], 0);
    }
}

// ============================================================================
// Tag Tests
// ============================================================================

mod tag_tests {
    use super::*;

    async fn create_board_and_column(server: &TestServer, session: &str) -> (String, String) {
        let board_response = server
            .post("/api/boards")
            .add_cookie(session_cookie(session))
            .json(&json!({"name": "Test Board"}))
            .await;
        let board_id = board_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let column_response = server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(session))
            .json(&json!({"name": "To Do"}))
            .await;
        let column_id = column_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        (board_id, column_id)
    }

    #[tokio::test]
    async fn test_create_tag() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let (board_id, _column_id) = create_board_and_column(&server, &session).await;

        let response = server
            .post(&format!("/api/boards/{}/tags", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "name": "Urgent",
                "color": "#dc3545"
            }))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["name"], "Urgent");
        assert_eq!(body["color"], "#dc3545");
    }

    #[tokio::test]
    async fn test_create_tag_default_color() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let (board_id, _column_id) = create_board_and_column(&server, &session).await;

        let response = server
            .post(&format!("/api/boards/{}/tags", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Default Color Tag"}))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["color"], "#6c757d");
    }

    #[tokio::test]
    async fn test_list_tags() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let (board_id, _column_id) = create_board_and_column(&server, &session).await;

        server
            .post(&format!("/api/boards/{}/tags", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Tag 1"}))
            .await;

        server
            .post(&format!("/api/boards/{}/tags", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Tag 2"}))
            .await;

        let response = server
            .get(&format!("/api/boards/{}/tags", board_id))
            .add_cookie(session_cookie(&session))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_update_tag() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let (board_id, _column_id) = create_board_and_column(&server, &session).await;

        let create_response = server
            .post(&format!("/api/boards/{}/tags", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Original", "color": "#000000"}))
            .await;
        let tag_id = create_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let response = server
            .put(&format!("/api/tags/{}", tag_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "name": "Updated",
                "color": "#ffffff"
            }))
            .await;

        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["name"], "Updated");
        assert_eq!(body["color"], "#ffffff");
    }

    #[tokio::test]
    async fn test_delete_tag() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let (board_id, _column_id) = create_board_and_column(&server, &session).await;

        let create_response = server
            .post(&format!("/api/boards/{}/tags", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Tag to Delete"}))
            .await;
        let tag_id = create_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let response = server
            .delete(&format!("/api/tags/{}", tag_id))
            .add_cookie(session_cookie(&session))
            .await;

        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_add_tag_to_card() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let (board_id, column_id) = create_board_and_column(&server, &session).await;

        let card_response = server
            .post(&format!("/api/columns/{}/cards", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"title": "Card with Tag"}))
            .await;
        let card_id = card_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let tag_response = server
            .post(&format!("/api/boards/{}/tags", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Important"}))
            .await;
        let tag_id = tag_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let response = server
            .post(&format!("/api/cards/{}/tags/{}", card_id, tag_id))
            .add_cookie(session_cookie(&session))
            .await;

        response.assert_status_ok();

        let card_detail = server
            .get(&format!("/api/cards/{}", card_id))
            .add_cookie(session_cookie(&session))
            .await;

        let body: Value = card_detail.json();
        let tags = body["tags"].as_array().unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0]["name"], "Important");
    }

    #[tokio::test]
    async fn test_remove_tag_from_card() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;
        let (board_id, column_id) = create_board_and_column(&server, &session).await;

        let card_response = server
            .post(&format!("/api/columns/{}/cards", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"title": "Card with Tag"}))
            .await;
        let card_id = card_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let tag_response = server
            .post(&format!("/api/boards/{}/tags", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Removable"}))
            .await;
        let tag_id = tag_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        server
            .post(&format!("/api/cards/{}/tags/{}", card_id, tag_id))
            .add_cookie(session_cookie(&session))
            .await;

        let response = server
            .delete(&format!("/api/cards/{}/tags/{}", card_id, tag_id))
            .add_cookie(session_cookie(&session))
            .await;

        response.assert_status_ok();

        let card_detail = server
            .get(&format!("/api/cards/{}", card_id))
            .add_cookie(session_cookie(&session))
            .await;

        let body: Value = card_detail.json();
        let tags = body["tags"].as_array().unwrap();
        assert!(tags.is_empty());
    }
}

// ============================================================================
// E2E Tests
// ============================================================================

mod e2e_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_kanban_workflow() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;

        let board_response = server
            .post("/api/boards")
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Project Board", "description": "For tracking project tasks"}))
            .await;
        board_response.assert_status_ok();
        let board_id = board_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let todo_response = server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "To Do"}))
            .await;
        let todo_id = todo_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let in_progress_response = server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "In Progress"}))
            .await;
        let in_progress_id = in_progress_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let done_response = server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Done"}))
            .await;
        let done_id = done_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let bug_tag_response = server
            .post(&format!("/api/boards/{}/tags", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Bug", "color": "#dc3545"}))
            .await;
        let bug_tag_id = bug_tag_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let feature_tag_response = server
            .post(&format!("/api/boards/{}/tags", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Feature", "color": "#28a745"}))
            .await;
        let feature_tag_id = feature_tag_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let card1_response = server
            .post(&format!("/api/columns/{}/cards", todo_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "title": "Fix login bug",
                "body": "Users can't login with special characters in password"
            }))
            .await;
        let card1_id = card1_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let card2_response = server
            .post(&format!("/api/columns/{}/cards", todo_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "title": "Add dark mode",
                "body": "Implement dark mode theme"
            }))
            .await;
        let card2_id = card2_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        server
            .post(&format!("/api/cards/{}/tags/{}", card1_id, bug_tag_id))
            .add_cookie(session_cookie(&session))
            .await
            .assert_status_ok();

        server
            .post(&format!("/api/cards/{}/tags/{}", card2_id, feature_tag_id))
            .add_cookie(session_cookie(&session))
            .await
            .assert_status_ok();

        let move_response = server
            .patch(&format!("/api/cards/{}/move", card1_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"column_id": in_progress_id, "position": 0}))
            .await;
        move_response.assert_status_ok();

        let board_detail = server
            .get(&format!("/api/boards/{}", board_id))
            .add_cookie(session_cookie(&session))
            .await;
        board_detail.assert_status_ok();

        let board_data: Value = board_detail.json();
        assert_eq!(board_data["columns"].as_array().unwrap().len(), 3);
        assert_eq!(board_data["tags"].as_array().unwrap().len(), 2);

        let complete_response = server
            .patch(&format!("/api/cards/{}/move", card1_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"column_id": done_id, "position": 0}))
            .await;
        complete_response.assert_status_ok();
        assert_eq!(complete_response.json::<Value>()["column_id"], done_id);
    }

    #[tokio::test]
    async fn test_cards_list_returns_all_cards() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;

        let board_response = server
            .post("/api/boards")
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Filter Test Board"}))
            .await;
        let board_id = board_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let column_response = server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Tasks"}))
            .await;
        let column_id = column_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let tag_response = server
            .post(&format!("/api/boards/{}/tags", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Priority"}))
            .await;
        let tag_id = tag_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let card1_response = server
            .post(&format!("/api/columns/{}/cards", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"title": "Tagged Card"}))
            .await;
        let card1_id = card1_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        server
            .post(&format!("/api/columns/{}/cards", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"title": "Untagged Card"}))
            .await;

        server
            .post(&format!("/api/cards/{}/tags/{}", card1_id, tag_id))
            .add_cookie(session_cookie(&session))
            .await;

        // Verify cards list returns all cards
        let all_cards_response = server
            .get(&format!("/api/boards/{}/cards", board_id))
            .add_cookie(session_cookie(&session))
            .await;

        all_cards_response.assert_status_ok();
        let cards: Value = all_cards_response.json();
        assert_eq!(cards.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_filter_cards_by_query() {
        let server = setup_server().await;
        let session = register_and_login(&server).await;

        let board_response = server
            .post("/api/boards")
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Query Test Board"}))
            .await;
        let board_id = board_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let column_response = server
            .post(&format!("/api/boards/{}/columns", board_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({"name": "Tasks"}))
            .await;
        let column_id = column_response.json::<Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        server
            .post(&format!("/api/columns/{}/cards", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "title": "Fix authentication bug",
                "body": "Users cannot login"
            }))
            .await;

        server
            .post(&format!("/api/columns/{}/cards", column_id))
            .add_cookie(session_cookie(&session))
            .json(&json!({
                "title": "Add new feature",
                "body": "Implement dashboard"
            }))
            .await;

        let filter_response = server
            .get(&format!(
                "/api/boards/{}/cards?query=authentication",
                board_id
            ))
            .add_cookie(session_cookie(&session))
            .await;

        filter_response.assert_status_ok();
        let cards: Value = filter_response.json();
        assert_eq!(cards.as_array().unwrap().len(), 1);
        assert_eq!(cards[0]["title"], "Fix authentication bug");
    }
}
