//! Web search via DuckDuckGo HTML (no API key needed).
//! Used as a tool call when the LLM detects it needs external information.

use log::{info, warn};
use reqwest::Client;
use regex::Regex;

/// Search result from DuckDuckGo
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub snippet: String,
}

/// Determine if a question likely needs web search based on keywords.
pub fn should_search(text: &str) -> bool {
    let lower = text.to_lowercase();
    let triggers = [
        "latest", "recent", "current", "today", "news", "weather",
        "price", "stock", "who won", "score", "working on",
        "search", "look up", "find out", "what happened",
    ];
    triggers.iter().any(|t| lower.contains(t))
}

/// Extract a search query from the user's question.
pub fn make_search_query(question: &str) -> String {
    let q = question.trim().trim_end_matches('?');
    // Strip common question prefixes for cleaner search
    let re = Regex::new(r"(?i)^(what|who|where|when|how|why)\s+(is|are|was|were|do|does|did)\s+").unwrap();
    let cleaned = re.replace(q, "").to_string();
    if cleaned.len() > 3 { cleaned } else { question.to_string() }
}

/// Perform a web search via DuckDuckGo HTML and return top results.
pub async fn search(query: &str, max_results: usize) -> Result<Vec<SearchResult>, String> {
    info!("[WebSearch] Searching: {}", query);

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let encoded = urlencoding::encode(query);
    let url = format!("https://html.duckduckgo.com/html/?q={}", encoded);

    let response = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
        .send()
        .await
        .map_err(|e| format!("Search request failed: {}", e))?;

    let html = response
        .text()
        .await
        .map_err(|e| format!("Failed to read search response: {}", e))?;

    let results = parse_results(&html, max_results);
    info!("[WebSearch] Got {} results for: {}", results.len(), query);
    Ok(results)
}

/// Format search results as a text block for LLM context.
pub async fn search_and_format(query: &str, max_results: usize) -> String {
    match search(query, max_results).await {
        Ok(results) if !results.is_empty() => {
            results
                .iter()
                .map(|r| format!("{}: {}", r.title, r.snippet))
                .collect::<Vec<_>>()
                .join("\n\n")
        }
        Ok(_) => String::new(),
        Err(e) => {
            warn!("[WebSearch] Search failed: {}", e);
            String::new()
        }
    }
}

/// Enrich a prompt with web search results if the question needs external info.
/// Returns the enriched prompt and whether search was used.
pub async fn enrich_prompt_if_needed(prompt: &str, question: &str) -> (String, bool) {
    if !should_search(question) {
        return (prompt.to_string(), false);
    }

    let query = make_search_query(question);
    let results = search_and_format(&query, 3).await;

    if results.is_empty() {
        return (prompt.to_string(), false);
    }

    let enriched = format!(
        "{}\n\nWeb search results for '{}':\n---\n{}\n---\nUse these search results to provide an accurate, up-to-date answer.",
        prompt, query, results
    );

    (enriched, true)
}

fn parse_results(html: &str, max_results: usize) -> Vec<SearchResult> {
    let snippet_re = Regex::new(r"result__snippet[^>]*>(.*?)</a>").unwrap();
    let title_re = Regex::new(r"result__a[^>]*>(.*?)</a>").unwrap();
    let tag_re = Regex::new(r"<[^>]+>").unwrap();

    let snippets: Vec<String> = snippet_re
        .captures_iter(html)
        .take(max_results)
        .filter_map(|c| c.get(1))
        .map(|m| strip_html(&tag_re, m.as_str()))
        .collect();

    let titles: Vec<String> = title_re
        .captures_iter(html)
        .take(max_results)
        .filter_map(|c| c.get(1))
        .map(|m| strip_html(&tag_re, m.as_str()))
        .collect();

    snippets
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.is_empty())
        .map(|(i, snippet)| SearchResult {
            title: titles.get(i).cloned().unwrap_or_default(),
            snippet: snippet.clone(),
        })
        .collect()
}

fn strip_html(tag_re: &Regex, html: &str) -> String {
    tag_re
        .replace_all(html, "")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}
