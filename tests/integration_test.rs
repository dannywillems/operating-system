use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

const BASE_URL: &str = "http://localhost:3000";

async fn create_test_client() -> Client {
    Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create client")
}

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_auth_register_and_login() {
    let client = create_test_client().await;

    // Register a new user
    let email = format!("test_{}@example.com", uuid::Uuid::new_v4());
    let register_response = client
        .post(format!("{}/api/auth/register", BASE_URL))
        .json(&json!({
            "email": email,
            "password": "testpassword123",
            "name": "Test User"
        }))
        .send()
        .await
        .expect("Failed to register");

    assert_eq!(register_response.status(), 200);

    let body: Value = register_response.json().await.unwrap();
    assert!(body["user"]["id"].is_string());
    assert_eq!(body["user"]["email"], email);

    // Login with the new user
    let login_response = client
        .post(format!("{}/api/auth/login", BASE_URL))
        .json(&json!({
            "email": email,
            "password": "testpassword123"
        }))
        .send()
        .await
        .expect("Failed to login");

    assert_eq!(login_response.status(), 200);
}

#[tokio::test]
#[ignore]
async fn test_board_crud() {
    let client = create_test_client().await;

    // Register and login
    let email = format!("test_{}@example.com", uuid::Uuid::new_v4());
    client
        .post(format!("{}/api/auth/register", BASE_URL))
        .json(&json!({
            "email": email,
            "password": "testpassword123",
            "name": "Test User"
        }))
        .send()
        .await
        .unwrap();

    client
        .post(format!("{}/api/auth/login", BASE_URL))
        .json(&json!({
            "email": email,
            "password": "testpassword123"
        }))
        .send()
        .await
        .unwrap();

    // Create a board
    let create_response = client
        .post(format!("{}/api/boards", BASE_URL))
        .json(&json!({
            "name": "Test Board",
            "description": "A test board"
        }))
        .send()
        .await
        .expect("Failed to create board");

    assert_eq!(create_response.status(), 200);
    let board: Value = create_response.json().await.unwrap();
    let board_id = board["id"].as_str().unwrap();

    // Get the board
    let get_response = client
        .get(format!("{}/api/boards/{}", BASE_URL, board_id))
        .send()
        .await
        .expect("Failed to get board");

    assert_eq!(get_response.status(), 200);

    // Update the board
    let update_response = client
        .put(format!("{}/api/boards/{}", BASE_URL, board_id))
        .json(&json!({
            "name": "Updated Board"
        }))
        .send()
        .await
        .expect("Failed to update board");

    assert_eq!(update_response.status(), 200);
    let updated: Value = update_response.json().await.unwrap();
    assert_eq!(updated["name"], "Updated Board");

    // Delete the board
    let delete_response = client
        .delete(format!("{}/api/boards/{}", BASE_URL, board_id))
        .send()
        .await
        .expect("Failed to delete board");

    assert_eq!(delete_response.status(), 200);
}

#[tokio::test]
#[ignore]
async fn test_card_move_between_columns() {
    let client = create_test_client().await;

    // Register and login
    let email = format!("test_{}@example.com", uuid::Uuid::new_v4());
    client
        .post(format!("{}/api/auth/register", BASE_URL))
        .json(&json!({
            "email": email,
            "password": "testpassword123",
            "name": "Test User"
        }))
        .send()
        .await
        .unwrap();

    client
        .post(format!("{}/api/auth/login", BASE_URL))
        .json(&json!({
            "email": email,
            "password": "testpassword123"
        }))
        .send()
        .await
        .unwrap();

    // Create a board
    let board: Value = client
        .post(format!("{}/api/boards", BASE_URL))
        .json(&json!({"name": "Kanban Board"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let board_id = board["id"].as_str().unwrap();

    // Create two columns
    let col1: Value = client
        .post(format!("{}/api/boards/{}/columns", BASE_URL, board_id))
        .json(&json!({"name": "To Do"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let col1_id = col1["id"].as_str().unwrap();

    let col2: Value = client
        .post(format!("{}/api/boards/{}/columns", BASE_URL, board_id))
        .json(&json!({"name": "Done"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let col2_id = col2["id"].as_str().unwrap();

    // Create a card in the first column
    let card: Value = client
        .post(format!("{}/api/columns/{}/cards", BASE_URL, col1_id))
        .json(&json!({
            "title": "Task 1",
            "body": "Do something"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let card_id = card["id"].as_str().unwrap();

    assert_eq!(card["column_id"], col1_id);

    // Move the card to the second column
    let move_response = client
        .patch(format!("{}/api/cards/{}/move", BASE_URL, card_id))
        .json(&json!({
            "column_id": col2_id,
            "position": 0
        }))
        .send()
        .await
        .expect("Failed to move card");

    assert_eq!(move_response.status(), 200);
    let moved_card: Value = move_response.json().await.unwrap();
    assert_eq!(moved_card["column_id"], col2_id);
    assert_eq!(moved_card["position"], 0);
}

#[tokio::test]
#[ignore]
async fn test_tags_on_cards() {
    let client = create_test_client().await;

    // Register and login
    let email = format!("test_{}@example.com", uuid::Uuid::new_v4());
    client
        .post(format!("{}/api/auth/register", BASE_URL))
        .json(&json!({
            "email": email,
            "password": "testpassword123",
            "name": "Test User"
        }))
        .send()
        .await
        .unwrap();

    client
        .post(format!("{}/api/auth/login", BASE_URL))
        .json(&json!({
            "email": email,
            "password": "testpassword123"
        }))
        .send()
        .await
        .unwrap();

    // Create a board
    let board: Value = client
        .post(format!("{}/api/boards", BASE_URL))
        .json(&json!({"name": "Tagged Board"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let board_id = board["id"].as_str().unwrap();

    // Create a column
    let col: Value = client
        .post(format!("{}/api/boards/{}/columns", BASE_URL, board_id))
        .json(&json!({"name": "Tasks"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let col_id = col["id"].as_str().unwrap();

    // Create a card
    let card: Value = client
        .post(format!("{}/api/columns/{}/cards", BASE_URL, col_id))
        .json(&json!({"title": "Tagged Task"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let card_id = card["id"].as_str().unwrap();

    // Create a tag
    let tag: Value = client
        .post(format!("{}/api/boards/{}/tags", BASE_URL, board_id))
        .json(&json!({
            "name": "Urgent",
            "color": "#dc3545"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let tag_id = tag["id"].as_str().unwrap();

    // Add tag to card
    let add_tag_response = client
        .post(format!(
            "{}/api/cards/{}/tags/{}",
            BASE_URL, card_id, tag_id
        ))
        .send()
        .await
        .expect("Failed to add tag to card");

    assert_eq!(add_tag_response.status(), 200);

    // Get the card and verify tag is attached
    let card_with_tag: Value = client
        .get(format!("{}/api/cards/{}", BASE_URL, card_id))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(!card_with_tag["tags"].as_array().unwrap().is_empty());
    assert_eq!(card_with_tag["tags"][0]["name"], "Urgent");

    // Remove tag from card
    let remove_tag_response = client
        .delete(format!(
            "{}/api/cards/{}/tags/{}",
            BASE_URL, card_id, tag_id
        ))
        .send()
        .await
        .expect("Failed to remove tag from card");

    assert_eq!(remove_tag_response.status(), 200);
}
