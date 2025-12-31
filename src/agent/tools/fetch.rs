//! Web fetch tool for retrieving online content
//!
//! Provides the agent with the ability to fetch content from URLs and convert
//! HTML to readable markdown. Inspired by Forge's NetFetch tool.
//!
//! Features:
//! - Fetches HTTP/HTTPS URLs
//! - Converts HTML to markdown for readability
//! - Respects robots.txt (basic check)
//! - Truncates large responses to prevent context overflow
//! - Returns raw content when requested

use reqwest::{Client, Url};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Maximum content length to return (characters)
const MAX_CONTENT_LENGTH: usize = 40_000;

// ============================================================================
// Web Fetch Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct WebFetchArgs {
    /// URL to fetch
    pub url: String,
    /// If true, return raw content without markdown conversion (default: false)
    pub raw: Option<bool>,
}

#[derive(Debug, thiserror::Error)]
#[error("Web fetch error: {0}")]
pub struct WebFetchError(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchTool {
    #[serde(skip)]
    client: Option<Client>,
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WebFetchTool {
    pub fn new() -> Self {
        Self {
            client: Some(
                Client::builder()
                    .user_agent("Mozilla/5.0 (compatible; SyncableCLI/0.1; +https://syncable.dev)")
                    .timeout(std::time::Duration::from_secs(30))
                    .build()
                    .unwrap_or_default(),
            ),
        }
    }

    fn client(&self) -> Client {
        self.client.clone().unwrap_or_default()
    }

    /// Check robots.txt for disallowed paths (basic check)
    async fn check_robots_txt(&self, url: &Url) -> Result<(), WebFetchError> {
        let robots_url = format!("{}://{}/robots.txt", url.scheme(), url.authority());

        // Try to fetch robots.txt (ignore errors - many sites don't have one)
        if let Ok(response) = self.client().get(&robots_url).send().await
            && response.status().is_success()
                && let Ok(robots_content) = response.text().await {
                    let path = url.path();
                    for line in robots_content.lines() {
                        if let Some(disallowed) = line.strip_prefix("Disallow: ") {
                            let disallowed = disallowed.trim();
                            if !disallowed.is_empty() {
                                let disallowed = if !disallowed.starts_with('/') {
                                    format!("/{}", disallowed)
                                } else {
                                    disallowed.to_string()
                                };
                                let check_path = if !path.starts_with('/') {
                                    format!("/{}", path)
                                } else {
                                    path.to_string()
                                };
                                if check_path.starts_with(&disallowed) {
                                    return Err(WebFetchError(format!(
                                        "URL {} cannot be fetched due to robots.txt restrictions",
                                        url
                                    )));
                                }
                            }
                        }
                    }
                }
        Ok(())
    }

    /// Fetch URL content and optionally convert HTML to markdown
    async fn fetch_url(&self, url: &Url, force_raw: bool) -> Result<FetchResult, WebFetchError> {
        // Check robots.txt first
        self.check_robots_txt(url).await?;

        let response = self
            .client()
            .get(url.as_str())
            .send()
            .await
            .map_err(|e| WebFetchError(format!("Failed to fetch URL {}: {}", url, e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(WebFetchError(format!(
                "Failed to fetch {} - status code {}",
                url, status
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let raw_content = response
            .text()
            .await
            .map_err(|e| WebFetchError(format!("Failed to read response from {}: {}", url, e)))?;

        // Determine if content is HTML
        let is_html = raw_content[..100.min(raw_content.len())].contains("<html")
            || raw_content[..100.min(raw_content.len())].contains("<!DOCTYPE")
            || raw_content[..100.min(raw_content.len())].contains("<!doctype")
            || content_type.contains("text/html")
            || (content_type.is_empty() && raw_content.contains("<body"));

        // Convert HTML to markdown unless raw is requested
        let content = if is_html && !force_raw {
            html_to_markdown(&raw_content)
        } else {
            raw_content
        };

        // Truncate if too long
        let (content, was_truncated) = if content.len() > MAX_CONTENT_LENGTH {
            (
                content[..MAX_CONTENT_LENGTH].to_string() + "\n\n[Content truncated...]",
                true,
            )
        } else {
            (content, false)
        };

        Ok(FetchResult {
            content,
            content_type,
            status_code: status.as_u16(),
            was_truncated,
            was_html: is_html && !force_raw,
        })
    }
}

#[derive(Debug)]
struct FetchResult {
    content: String,
    content_type: String,
    status_code: u16,
    was_truncated: bool,
    was_html: bool,
}

impl Tool for WebFetchTool {
    const NAME: &'static str = "web_fetch";

    type Error = WebFetchError;
    type Args = WebFetchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Fetch content from a URL and return it as text or markdown.

Use this tool to:
- Look up documentation for libraries, frameworks, or APIs
- Check official guides and tutorials
- Verify information from authoritative sources
- Research best practices and patterns
- Access API reference documentation
- Get current information beyond training data

The tool automatically converts HTML pages to readable markdown format.
For API endpoints returning JSON/XML, use raw=true to get the unprocessed response.

Limitations:
- Cannot access pages requiring authentication
- Respects robots.txt restrictions
- Large pages are truncated to ~40,000 characters
- Some sites may block automated requests"#
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to fetch (must be http:// or https://)"
                    },
                    "raw": {
                        "type": "boolean",
                        "description": "If true, return raw content without HTML-to-markdown conversion. Default: false"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Parse and validate URL
        let url = Url::parse(&args.url)
            .map_err(|e| WebFetchError(format!("Invalid URL '{}': {}", args.url, e)))?;

        // Only allow http/https
        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(WebFetchError(format!(
                "Unsupported URL scheme '{}'. Only http and https are supported.",
                url.scheme()
            )));
        }

        let force_raw = args.raw.unwrap_or(false);
        let result = self.fetch_url(&url, force_raw).await?;

        let output = json!({
            "url": args.url,
            "status_code": result.status_code,
            "content_type": result.content_type,
            "converted_to_markdown": result.was_html,
            "truncated": result.was_truncated,
            "content": result.content
        });

        serde_json::to_string_pretty(&output)
            .map_err(|e| WebFetchError(format!("Failed to serialize response: {}", e)))
    }
}

/// Convert HTML content to Markdown
///
/// Uses a simple regex-based approach for common HTML elements.
/// For more complex HTML, consider using a proper HTML parser.
fn html_to_markdown(html: &str) -> String {
    use regex::Regex;

    let mut content = html.to_string();

    // Remove script and style tags entirely
    let script_re = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
    content = script_re.replace_all(&content, "").to_string();

    let style_re = Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
    content = style_re.replace_all(&content, "").to_string();

    // Remove comments
    let comment_re = Regex::new(r"(?is)<!--.*?-->").unwrap();
    content = comment_re.replace_all(&content, "").to_string();

    // Convert headers
    let h1_re = Regex::new(r"(?is)<h1[^>]*>(.*?)</h1>").unwrap();
    content = h1_re.replace_all(&content, "\n# $1\n").to_string();

    let h2_re = Regex::new(r"(?is)<h2[^>]*>(.*?)</h2>").unwrap();
    content = h2_re.replace_all(&content, "\n## $1\n").to_string();

    let h3_re = Regex::new(r"(?is)<h3[^>]*>(.*?)</h3>").unwrap();
    content = h3_re.replace_all(&content, "\n### $1\n").to_string();

    let h4_re = Regex::new(r"(?is)<h4[^>]*>(.*?)</h4>").unwrap();
    content = h4_re.replace_all(&content, "\n#### $1\n").to_string();

    let h5_re = Regex::new(r"(?is)<h5[^>]*>(.*?)</h5>").unwrap();
    content = h5_re.replace_all(&content, "\n##### $1\n").to_string();

    let h6_re = Regex::new(r"(?is)<h6[^>]*>(.*?)</h6>").unwrap();
    content = h6_re.replace_all(&content, "\n###### $1\n").to_string();

    // Convert paragraphs
    let p_re = Regex::new(r"(?is)<p[^>]*>(.*?)</p>").unwrap();
    content = p_re.replace_all(&content, "\n$1\n").to_string();

    // Convert links
    let a_re = Regex::new(r#"(?is)<a[^>]*href="([^"]*)"[^>]*>(.*?)</a>"#).unwrap();
    content = a_re.replace_all(&content, "[$2]($1)").to_string();

    // Convert bold/strong
    let strong_re = Regex::new(r"(?is)<(?:strong|b)[^>]*>(.*?)</(?:strong|b)>").unwrap();
    content = strong_re.replace_all(&content, "**$1**").to_string();

    // Convert italic/em
    let em_re = Regex::new(r"(?is)<(?:em|i)[^>]*>(.*?)</(?:em|i)>").unwrap();
    content = em_re.replace_all(&content, "*$1*").to_string();

    // Convert code blocks
    let pre_re = Regex::new(r"(?is)<pre[^>]*><code[^>]*>(.*?)</code></pre>").unwrap();
    content = pre_re.replace_all(&content, "\n```\n$1\n```\n").to_string();

    let pre_only_re = Regex::new(r"(?is)<pre[^>]*>(.*?)</pre>").unwrap();
    content = pre_only_re
        .replace_all(&content, "\n```\n$1\n```\n")
        .to_string();

    // Convert inline code
    let code_re = Regex::new(r"(?is)<code[^>]*>(.*?)</code>").unwrap();
    content = code_re.replace_all(&content, "`$1`").to_string();

    // Convert lists
    let ul_re = Regex::new(r"(?is)<ul[^>]*>(.*?)</ul>").unwrap();
    content = ul_re.replace_all(&content, "\n$1\n").to_string();

    let ol_re = Regex::new(r"(?is)<ol[^>]*>(.*?)</ol>").unwrap();
    content = ol_re.replace_all(&content, "\n$1\n").to_string();

    let li_re = Regex::new(r"(?is)<li[^>]*>(.*?)</li>").unwrap();
    content = li_re.replace_all(&content, "- $1\n").to_string();

    // Convert blockquotes
    let bq_re = Regex::new(r"(?is)<blockquote[^>]*>(.*?)</blockquote>").unwrap();
    content = bq_re.replace_all(&content, "\n> $1\n").to_string();

    // Convert line breaks
    let br_re = Regex::new(r"(?i)<br\s*/?>").unwrap();
    content = br_re.replace_all(&content, "\n").to_string();

    // Convert horizontal rules
    let hr_re = Regex::new(r"(?i)<hr\s*/?>").unwrap();
    content = hr_re.replace_all(&content, "\n---\n").to_string();

    // Remove remaining HTML tags
    let tag_re = Regex::new(r"<[^>]+>").unwrap();
    content = tag_re.replace_all(&content, "").to_string();

    // Decode common HTML entities
    content = content
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&copy;", "©")
        .replace("&reg;", "®")
        .replace("&trade;", "™")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
        .replace("&hellip;", "…");

    // Clean up excessive whitespace
    let multiline_re = Regex::new(r"\n{3,}").unwrap();
    content = multiline_re.replace_all(&content, "\n\n").to_string();

    let space_re = Regex::new(r" {2,}").unwrap();
    content = space_re.replace_all(&content, " ").to_string();

    content.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_to_markdown_headers() {
        let html = "<h1>Title</h1><h2>Subtitle</h2><h3>Section</h3>";
        let md = html_to_markdown(html);
        assert!(md.contains("# Title"));
        assert!(md.contains("## Subtitle"));
        assert!(md.contains("### Section"));
    }

    #[test]
    fn test_html_to_markdown_links() {
        let html = r#"<a href="https://example.com">Example</a>"#;
        let md = html_to_markdown(html);
        assert!(md.contains("[Example](https://example.com)"));
    }

    #[test]
    fn test_html_to_markdown_formatting() {
        let html = "<strong>bold</strong> and <em>italic</em>";
        let md = html_to_markdown(html);
        assert!(md.contains("**bold**"));
        assert!(md.contains("*italic*"));
    }

    #[test]
    fn test_html_to_markdown_code() {
        let html = "<code>inline</code> and <pre><code>block</code></pre>";
        let md = html_to_markdown(html);
        assert!(md.contains("`inline`"));
        assert!(md.contains("```"));
    }

    #[test]
    fn test_html_to_markdown_lists() {
        let html = "<ul><li>Item 1</li><li>Item 2</li></ul>";
        let md = html_to_markdown(html);
        assert!(md.contains("- Item 1"));
        assert!(md.contains("- Item 2"));
    }

    #[test]
    fn test_html_to_markdown_removes_scripts() {
        let html = "<p>Content</p><script>alert('xss')</script><p>More</p>";
        let md = html_to_markdown(html);
        assert!(!md.contains("script"));
        assert!(!md.contains("alert"));
        assert!(md.contains("Content"));
        assert!(md.contains("More"));
    }
}
