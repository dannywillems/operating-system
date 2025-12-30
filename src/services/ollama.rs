use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument, warn};

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

        info!(
            base_url = %base_url,
            model = %model,
            "Initializing Ollama client"
        );

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

    #[instrument(skip(self, messages), fields(model = %self.model, message_count = messages.len()))]
    pub async fn chat(&self, messages: Vec<OllamaMessage>) -> Result<String> {
        let request = OllamaChatRequest {
            model: self.model.clone(),
            messages,
            stream: false,
        };

        let url = format!("{}/api/chat", self.base_url);

        debug!(url = %url, "Sending chat request to Ollama");

        let start = std::time::Instant::now();
        let response: reqwest::Response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                error!(error = %e, "Ollama request failed");
                AppError::Internal(format!("Ollama request failed: {}", e))
            })?;

        let elapsed = start.elapsed();

        if !response.status().is_success() {
            let status = response.status();
            let body: String = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!(
                status = %status,
                body = %body,
                "Ollama returned error"
            );
            return Err(AppError::Internal(format!(
                "Ollama returned error {}: {}",
                status, body
            )));
        }

        let chat_response: OllamaChatResponse =
            response.json::<OllamaChatResponse>().await.map_err(|e| {
                error!(error = %e, "Failed to parse Ollama response");
                AppError::Internal(format!("Failed to parse Ollama response: {}", e))
            })?;

        info!(
            elapsed_ms = elapsed.as_millis(),
            response_length = chat_response.message.content.len(),
            "Ollama chat completed"
        );

        debug!(response = %chat_response.message.content, "LLM response content");

        Ok(chat_response.message.content)
    }

    pub async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        match self.client.get(&url).send().await {
            Ok(_) => {
                debug!("Ollama is available");
                true
            }
            Err(e) => {
                warn!(error = %e, "Ollama is not available");
                false
            }
        }
    }
}
