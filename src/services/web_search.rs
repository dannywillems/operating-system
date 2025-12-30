use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::error::{AppError, Result};

/// A single search result from DuckDuckGo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Client for performing web searches via DuckDuckGo
#[derive(Clone)]
pub struct WebSearchClient {
    client: Client,
}

impl Default for WebSearchClient {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSearchClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Search DuckDuckGo and return parsed results
    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>> {
        if query.trim().is_empty() {
            return Err(AppError::BadRequest("Search query cannot be empty".into()));
        }

        info!(query = %query, max_results = %max_results, "Performing web search");

        let encoded_query = urlencoding::encode(query);
        let url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);

        let response = self.client.get(&url).send().await.map_err(|e| {
            error!(error = %e, "Failed to fetch search results");
            AppError::Internal(format!("Search request failed: {}", e))
        })?;

        if !response.status().is_success() {
            warn!(status = %response.status(), "Search returned non-success status");
            return Err(AppError::Internal(format!(
                "Search failed with status: {}",
                response.status()
            )));
        }

        let html = response.text().await.map_err(|e| {
            error!(error = %e, "Failed to read search response");
            AppError::Internal(format!("Failed to read response: {}", e))
        })?;

        let results = parse_duckduckgo_results(&html, max_results);

        info!(count = results.len(), "Search completed");

        Ok(results)
    }
}

/// Parse DuckDuckGo HTML search results
fn parse_duckduckgo_results(html: &str, max_results: usize) -> Vec<SearchResult> {
    let document = Html::parse_document(html);

    // DuckDuckGo HTML search uses these selectors
    let result_selector = Selector::parse(".result").unwrap();
    let title_selector = Selector::parse(".result__a").unwrap();
    let snippet_selector = Selector::parse(".result__snippet").unwrap();
    let url_selector = Selector::parse(".result__url").unwrap();

    let mut results = Vec::new();

    for element in document.select(&result_selector) {
        if results.len() >= max_results {
            break;
        }

        // Extract title
        let title = element
            .select(&title_selector)
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default()
            .trim()
            .to_string();

        // Extract URL - try href attribute first, then fallback to text
        let url = element
            .select(&title_selector)
            .next()
            .and_then(|e| e.value().attr("href"))
            .map(|href| {
                // DuckDuckGo uses redirect URLs, extract the actual URL
                if href.contains("uddg=") {
                    href.split("uddg=")
                        .nth(1)
                        .and_then(|s| s.split('&').next())
                        .map(|s| urlencoding::decode(s).unwrap_or_default().to_string())
                        .unwrap_or_else(|| href.to_string())
                } else {
                    href.to_string()
                }
            })
            .or_else(|| {
                element
                    .select(&url_selector)
                    .next()
                    .map(|e| e.text().collect::<String>().trim().to_string())
            })
            .unwrap_or_default();

        // Extract snippet
        let snippet = element
            .select(&snippet_selector)
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default()
            .trim()
            .to_string();

        // Skip results without title or URL
        if title.is_empty() || url.is_empty() {
            continue;
        }

        results.push(SearchResult {
            title,
            url,
            snippet,
        });
    }

    results
}

/// Format search results as text for LLM consumption
pub fn format_search_results(results: &[SearchResult]) -> String {
    if results.is_empty() {
        return "No search results found.".to_string();
    }

    let mut output = String::new();

    for (i, result) in results.iter().enumerate() {
        output.push_str(&format!(
            "{}. {}\n   URL: {}\n   {}\n\n",
            i + 1,
            result.title,
            result.url,
            result.snippet
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_search_results_empty() {
        let results: Vec<SearchResult> = vec![];
        assert_eq!(format_search_results(&results), "No search results found.");
    }

    #[test]
    fn test_format_search_results() {
        let results = vec![SearchResult {
            title: "Test Title".to_string(),
            url: "https://example.com".to_string(),
            snippet: "This is a test snippet.".to_string(),
        }];

        let formatted = format_search_results(&results);
        assert!(formatted.contains("Test Title"));
        assert!(formatted.contains("https://example.com"));
        assert!(formatted.contains("This is a test snippet"));
    }
}
