//! Content types for AG-UI protocol multimodal messages.
//!
//! This module defines input content types for handling text and binary
//! content in messages, enabling multimodal agent interactions.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

/// Error type for content validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentValidationError {
    message: String,
}

impl ContentValidationError {
    /// Creates a new validation error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ContentValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ContentValidationError {}

/// Text input content for messages.
///
/// Represents plain text content in a message.
///
/// # Example
///
/// ```
/// use ag_ui_core::TextInputContent;
///
/// let content = TextInputContent::new("Hello, world!");
/// assert_eq!(content.text, "Hello, world!");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextInputContent {
    /// The content type discriminator, always "text".
    #[serde(rename = "type")]
    pub type_tag: String,
    /// The text content.
    pub text: String,
}

impl TextInputContent {
    /// Creates a new text input content.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            type_tag: "text".to_string(),
            text: text.into(),
        }
    }
}

/// Binary input content for multimodal messages.
///
/// Represents binary content such as images, files, or other media.
/// At least one of `id`, `url`, or `data` must be provided.
///
/// # Example
///
/// ```
/// use ag_ui_core::BinaryInputContent;
///
/// let content = BinaryInputContent::new("image/png")
///     .with_url("https://example.com/image.png")
///     .with_filename("screenshot.png");
///
/// assert_eq!(content.mime_type, "image/png");
/// assert_eq!(content.url, Some("https://example.com/image.png".to_string()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryInputContent {
    /// The content type discriminator, always "binary".
    #[serde(rename = "type")]
    pub type_tag: String,
    /// The MIME type of the binary content.
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// Optional identifier for the content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Optional URL where the content can be fetched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Optional base64-encoded data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// Optional filename for the content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

impl BinaryInputContent {
    /// Creates a new binary input content with the given MIME type.
    pub fn new(mime_type: impl Into<String>) -> Self {
        Self {
            type_tag: "binary".to_string(),
            mime_type: mime_type.into(),
            id: None,
            url: None,
            data: None,
            filename: None,
        }
    }

    /// Sets the content identifier.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the content URL.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Sets the base64-encoded data.
    pub fn with_data(mut self, data: impl Into<String>) -> Self {
        self.data = Some(data.into());
        self
    }

    /// Sets the filename.
    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    /// Validates that at least one of id, url, or data is present.
    pub fn validate(&self) -> Result<(), ContentValidationError> {
        if self.id.is_none() && self.url.is_none() && self.data.is_none() {
            return Err(ContentValidationError::new(
                "BinaryInputContent requires at least one of: id, url, or data",
            ));
        }
        Ok(())
    }
}

/// Input content union type for multimodal messages.
///
/// This is a discriminated union that can hold either text or binary content.
/// The `type` field in JSON determines which variant is used.
///
/// # Example
///
/// ```
/// use ag_ui_core::InputContent;
///
/// // Create text content
/// let text = InputContent::text("Hello!");
/// assert!(text.is_text());
///
/// // Create binary content with URL
/// let binary = InputContent::binary_with_url("image/jpeg", "https://example.com/img.jpg");
/// assert!(binary.is_binary());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InputContent {
    /// Text content variant.
    Text {
        /// The text content.
        text: String,
    },
    /// Binary content variant for images, files, etc.
    Binary {
        /// The MIME type of the binary content.
        #[serde(rename = "mimeType")]
        mime_type: String,
        /// Optional identifier for the content.
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Optional URL where the content can be fetched.
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        /// Optional base64-encoded data.
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<String>,
        /// Optional filename for the content.
        #[serde(skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
    },
}

impl InputContent {
    /// Creates a text content variant.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Creates a minimal binary content variant.
    pub fn binary(mime_type: impl Into<String>) -> Self {
        Self::Binary {
            mime_type: mime_type.into(),
            id: None,
            url: None,
            data: None,
            filename: None,
        }
    }

    /// Creates a binary content variant with a URL.
    pub fn binary_with_url(mime_type: impl Into<String>, url: impl Into<String>) -> Self {
        Self::Binary {
            mime_type: mime_type.into(),
            id: None,
            url: Some(url.into()),
            data: None,
            filename: None,
        }
    }

    /// Creates a binary content variant with base64-encoded data.
    pub fn binary_with_data(mime_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Binary {
            mime_type: mime_type.into(),
            id: None,
            url: None,
            data: Some(data.into()),
            filename: None,
        }
    }

    /// Returns true if this is text content.
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text { .. })
    }

    /// Returns true if this is binary content.
    pub fn is_binary(&self) -> bool {
        matches!(self, Self::Binary { .. })
    }

    /// Returns the text content if this is a text variant.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            Self::Binary { .. } => None,
        }
    }

    /// Validates the content.
    ///
    /// For text content, always succeeds.
    /// For binary content, validates that at least one of id, url, or data is present.
    pub fn validate(&self) -> Result<(), ContentValidationError> {
        match self {
            Self::Text { .. } => Ok(()),
            Self::Binary {
                id, url, data, ..
            } => {
                if id.is_none() && url.is_none() && data.is_none() {
                    Err(ContentValidationError::new(
                        "Binary content requires at least one of: id, url, or data",
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: TextInputContent serialization
    #[test]
    fn test_text_input_content_serialization() {
        let content = TextInputContent::new("Hello, world!");
        let json = serde_json::to_string(&content).unwrap();

        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello, world!\""));
    }

    // Test 2: TextInputContent deserialization
    #[test]
    fn test_text_input_content_deserialization() {
        let json = r#"{"type":"text","text":"Hello!"}"#;
        let content: TextInputContent = serde_json::from_str(json).unwrap();

        assert_eq!(content.type_tag, "text");
        assert_eq!(content.text, "Hello!");
    }

    // Test 3: BinaryInputContent serialization
    #[test]
    fn test_binary_input_content_serialization() {
        let content = BinaryInputContent::new("image/png")
            .with_url("https://example.com/img.png")
            .with_filename("test.png");

        let json = serde_json::to_string(&content).unwrap();

        assert!(json.contains("\"type\":\"binary\""));
        assert!(json.contains("\"mimeType\":\"image/png\""));
        assert!(json.contains("\"url\":\"https://example.com/img.png\""));
        assert!(json.contains("\"filename\":\"test.png\""));
        // Optional fields should be omitted when None
        assert!(!json.contains("\"id\""));
        assert!(!json.contains("\"data\""));
    }

    // Test 4: BinaryInputContent builder pattern
    #[test]
    fn test_binary_input_content_builder() {
        let content = BinaryInputContent::new("application/pdf")
            .with_id("file-123")
            .with_url("https://example.com/doc.pdf")
            .with_data("base64data")
            .with_filename("document.pdf");

        assert_eq!(content.mime_type, "application/pdf");
        assert_eq!(content.id, Some("file-123".to_string()));
        assert_eq!(content.url, Some("https://example.com/doc.pdf".to_string()));
        assert_eq!(content.data, Some("base64data".to_string()));
        assert_eq!(content.filename, Some("document.pdf".to_string()));
    }

    // Test 5: InputContent text variant
    #[test]
    fn test_input_content_text_variant() {
        let content = InputContent::text("Hello!");

        assert!(content.is_text());
        assert!(!content.is_binary());
        assert_eq!(content.as_text(), Some("Hello!"));
    }

    // Test 6: InputContent binary variant
    #[test]
    fn test_input_content_binary_variant() {
        let content = InputContent::binary_with_url("image/jpeg", "https://example.com/img.jpg");

        assert!(!content.is_text());
        assert!(content.is_binary());
        assert_eq!(content.as_text(), None);
    }

    // Test 7: InputContent discriminated union serialization
    #[test]
    fn test_input_content_discriminated_union() {
        // Text variant
        let text = InputContent::text("Hello");
        let text_json = serde_json::to_string(&text).unwrap();
        assert!(text_json.contains("\"type\":\"text\""));

        // Binary variant
        let binary = InputContent::binary_with_url("image/png", "https://example.com/img.png");
        let binary_json = serde_json::to_string(&binary).unwrap();
        assert!(binary_json.contains("\"type\":\"binary\""));

        // Deserialize text
        let parsed_text: InputContent = serde_json::from_str(&text_json).unwrap();
        assert!(parsed_text.is_text());

        // Deserialize binary
        let parsed_binary: InputContent = serde_json::from_str(&binary_json).unwrap();
        assert!(parsed_binary.is_binary());
    }

    // Test 8: Binary validation success
    #[test]
    fn test_binary_validation_success() {
        let with_url = BinaryInputContent::new("image/png").with_url("https://example.com/img.png");
        assert!(with_url.validate().is_ok());

        let with_data = BinaryInputContent::new("image/png").with_data("base64data");
        assert!(with_data.validate().is_ok());

        let with_id = BinaryInputContent::new("image/png").with_id("file-123");
        assert!(with_id.validate().is_ok());
    }

    // Test 9: Binary validation failure
    #[test]
    fn test_binary_validation_failure() {
        let empty = BinaryInputContent::new("image/png");
        let result = empty.validate();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("at least one of"));
    }

    // Test 10: InputContent roundtrip
    #[test]
    fn test_input_content_roundtrip() {
        // Text roundtrip
        let text = InputContent::text("Hello, world!");
        let text_json = serde_json::to_string(&text).unwrap();
        let text_parsed: InputContent = serde_json::from_str(&text_json).unwrap();
        assert_eq!(text, text_parsed);

        // Binary roundtrip
        let binary = InputContent::Binary {
            mime_type: "image/png".to_string(),
            id: Some("img-123".to_string()),
            url: Some("https://example.com/img.png".to_string()),
            data: Some("iVBORw0KGgo=".to_string()),
            filename: Some("screenshot.png".to_string()),
        };
        let binary_json = serde_json::to_string(&binary).unwrap();
        let binary_parsed: InputContent = serde_json::from_str(&binary_json).unwrap();
        assert_eq!(binary, binary_parsed);
    }

    // Test 11: InputContent validation
    #[test]
    fn test_input_content_validation() {
        // Text always valid
        let text = InputContent::text("Hello");
        assert!(text.validate().is_ok());

        // Binary with url is valid
        let binary_valid = InputContent::binary_with_url("image/png", "https://example.com/img.png");
        assert!(binary_valid.validate().is_ok());

        // Binary without id/url/data is invalid
        let binary_invalid = InputContent::binary("image/png");
        assert!(binary_invalid.validate().is_err());
    }

    // Test 12: BinaryInputContent deserialization
    #[test]
    fn test_binary_input_content_deserialization() {
        let json = r#"{"type":"binary","mimeType":"image/jpeg","url":"https://example.com/img.jpg"}"#;
        let content: BinaryInputContent = serde_json::from_str(json).unwrap();

        assert_eq!(content.type_tag, "binary");
        assert_eq!(content.mime_type, "image/jpeg");
        assert_eq!(content.url, Some("https://example.com/img.jpg".to_string()));
        assert_eq!(content.id, None);
        assert_eq!(content.data, None);
        assert_eq!(content.filename, None);
    }
}
