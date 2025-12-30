use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

#[derive(Clone)]
pub struct OllamaClient {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
}

impl OllamaClient {
    pub fn new(base_url: Option<String>, model: Option<String>) -> Self {
        let base_url = base_url.unwrap_or_else(|| "http://localhost:11434".to_string());
        let model = model.unwrap_or_else(|| "llama3.2".to_string());

        Self {
            client: reqwest::Client::new(),
            base_url,
            model,
        }
    }

    pub fn from_env() -> Self {
        let base_url = std::env::var("OLLAMA_URL").ok();
        let model = std::env::var("OLLAMA_MODEL").ok();
        Self::new(base_url, model)
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub async fn chat(&self, messages: Vec<OllamaMessage>) -> Result<String> {
        let request = OllamaChatRequest {
            model: self.model.clone(),
            messages,
            stream: false,
        };

        let url = format!("{}/api/chat", self.base_url);

        let response: reqwest::Response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Ollama request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body: String = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::Internal(format!(
                "Ollama returned error {}: {}",
                status, body
            )));
        }

        let chat_response: OllamaChatResponse = response
            .json::<OllamaChatResponse>()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse Ollama response: {}", e)))?;

        Ok(chat_response.message.content)
    }

    pub async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        self.client.get(&url).send().await.is_ok()
    }
}
