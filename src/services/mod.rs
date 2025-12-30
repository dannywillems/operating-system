pub mod ollama;
pub mod web_search;

pub use ollama::OllamaClient;
pub use web_search::{format_search_results, WebSearchClient};
