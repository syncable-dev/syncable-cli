//! Beautiful response formatting for AI outputs
//!
//! Renders AI responses with proper markdown support using termimad
//! and syntax highlighting using syntect.

use std::sync::Arc;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::as_24_bit_terminal_escaped;
use termimad::crossterm::style::{Attribute, Color};
use termimad::{CompoundStyle, LineStyle, MadSkin};

/// Syncable brand colors using ANSI 256-color codes
pub mod brand {
    /// Primary purple (like the S in logo)
    pub const PURPLE: &str = "\x1b[38;5;141m";
    /// Accent magenta
    pub const MAGENTA: &str = "\x1b[38;5;207m";
    /// Light purple for headers
    pub const LIGHT_PURPLE: &str = "\x1b[38;5;183m";
    /// Cyan for code/technical
    pub const CYAN: &str = "\x1b[38;5;51m";
    /// Soft white for body text
    pub const TEXT: &str = "\x1b[38;5;252m";
    /// Dim gray for secondary info
    pub const DIM: &str = "\x1b[38;5;245m";
    /// Green for success
    pub const SUCCESS: &str = "\x1b[38;5;114m";
    /// Yellow for warnings
    pub const YELLOW: &str = "\x1b[38;5;221m";
    /// Peach/light orange for thinking (like N in logo)
    pub const PEACH: &str = "\x1b[38;5;216m";
    /// Lighter peach for thinking secondary
    pub const LIGHT_PEACH: &str = "\x1b[38;5;223m";
    /// Coral/salmon for thinking accents
    pub const CORAL: &str = "\x1b[38;5;209m";
    /// Reset
    pub const RESET: &str = "\x1b[0m";
    /// Bold
    pub const BOLD: &str = "\x1b[1m";
    /// Italic
    pub const ITALIC: &str = "\x1b[3m";
}

/// Syntax highlighter with cached resources
#[derive(Clone)]
pub struct SyntaxHighlighter {
    syntax_set: Arc<SyntaxSet>,
    theme_set: Arc<ThemeSet>,
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self {
            syntax_set: Arc::new(SyntaxSet::load_defaults_newlines()),
            theme_set: Arc::new(ThemeSet::load_defaults()),
        }
    }
}

impl SyntaxHighlighter {
    /// Highlight code with the given language
    pub fn highlight(&self, code: &str, lang: &str) -> String {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut hl = HighlightLines::new(syntax, theme);

        code.lines()
            .filter_map(|line| hl.highlight_line(line, &self.syntax_set).ok())
            .map(|ranges| format!("{}\x1b[0m", as_24_bit_terminal_escaped(&ranges, false)))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A code block extracted from markdown
#[derive(Clone, Debug)]
struct CodeBlock {
    code: String,
    lang: String,
}

/// Parses markdown and extracts code blocks for separate highlighting
struct CodeBlockParser {
    markdown: String,
    blocks: Vec<CodeBlock>,
}

impl CodeBlockParser {
    /// Extract code blocks from markdown content
    fn parse(content: &str) -> Self {
        let mut blocks = Vec::new();
        let mut result = String::new();
        let mut in_code_block = false;
        let mut code_lines: Vec<&str> = Vec::new();
        let mut current_lang = String::new();

        for line in content.lines() {
            if line.trim_start().starts_with("```") {
                if in_code_block {
                    // End of code block - store and add placeholder
                    result.push_str(&format!("\x00{}\x00\n", blocks.len()));
                    blocks.push(CodeBlock {
                        code: code_lines.join("\n"),
                        lang: current_lang.clone(),
                    });
                    code_lines.clear();
                    current_lang.clear();
                    in_code_block = false;
                } else {
                    // Start of code block
                    current_lang = line
                        .trim_start()
                        .strip_prefix("```")
                        .unwrap_or("")
                        .to_string();
                    in_code_block = true;
                }
            } else if in_code_block {
                code_lines.push(line);
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }

        // Handle unclosed code block
        if in_code_block && !code_lines.is_empty() {
            result.push_str(&format!("\x00{}\x00\n", blocks.len()));
            blocks.push(CodeBlock {
                code: code_lines.join("\n"),
                lang: current_lang,
            });
        }

        Self {
            markdown: result,
            blocks,
        }
    }

    /// Get the processed markdown with placeholders
    fn markdown(&self) -> &str {
        &self.markdown
    }

    /// Replace placeholders with highlighted code blocks
    fn restore(&self, highlighter: &SyntaxHighlighter, mut rendered: String) -> String {
        for (i, block) in self.blocks.iter().enumerate() {
            // Just show syntax-highlighted code without language header
            let highlighted = highlighter.highlight(&block.code, &block.lang);
            let code_block = format!("\n{}\n", highlighted);
            rendered = rendered.replace(&format!("\x00{i}\x00"), &code_block);
        }
        rendered
    }
}

/// Markdown formatter with Syncable branding
pub struct MarkdownFormat {
    skin: MadSkin,
    highlighter: SyntaxHighlighter,
}

impl Default for MarkdownFormat {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownFormat {
    /// Create a new MarkdownFormat with Syncable brand colors
    #[allow(clippy::field_reassign_with_default)]
    pub fn new() -> Self {
        let mut skin = MadSkin::default();

        // Inline code - cyan
        skin.inline_code = CompoundStyle::new(Some(Color::Cyan), None, Default::default());

        // Code blocks - will be replaced with syntax highlighted version
        skin.code_block = LineStyle::new(
            CompoundStyle::new(None, None, Default::default()),
            Default::default(),
        );

        // Headers - purple theme with bold
        let mut h1_style = CompoundStyle::new(Some(Color::Magenta), None, Default::default());
        h1_style.add_attr(Attribute::Bold);
        skin.headers[0] = LineStyle::new(h1_style.clone(), Default::default());
        skin.headers[1] = LineStyle::new(h1_style.clone(), Default::default());

        let h3_style = CompoundStyle::new(Some(Color::Magenta), None, Default::default());
        skin.headers[2] = LineStyle::new(h3_style, Default::default());

        // Bold - light purple with bold attribute
        let mut bold_style = CompoundStyle::new(Some(Color::Magenta), None, Default::default());
        bold_style.add_attr(Attribute::Bold);
        skin.bold = bold_style;

        // Italic
        skin.italic = CompoundStyle::with_attr(Attribute::Italic);

        // Strikethrough
        let mut strikethrough = CompoundStyle::with_attr(Attribute::CrossedOut);
        strikethrough.add_attr(Attribute::Dim);
        skin.strikeout = strikethrough;

        Self {
            skin,
            highlighter: SyntaxHighlighter::default(),
        }
    }

    /// Render markdown content to a styled string for terminal display
    pub fn render(&self, content: impl Into<String>) -> String {
        let content = content.into();
        let content = content.trim();

        if content.is_empty() {
            return String::new();
        }

        // Extract code blocks for separate highlighting
        let parsed = CodeBlockParser::parse(content);

        // Render with termimad
        let rendered = self.skin.term_text(parsed.markdown()).to_string();

        // Restore highlighted code blocks
        parsed
            .restore(&self.highlighter, rendered)
            .trim()
            .to_string()
    }
}

/// Response formatter with beautiful rendering
pub struct ResponseFormatter;

impl ResponseFormatter {
    /// Format and print a complete AI response with nice styling
    pub fn print_response(text: &str) {
        // Print the response header
        println!();
        Self::print_header();
        println!();

        // Render markdown with proper formatting (tables, code blocks, etc.)
        let formatter = MarkdownFormat::new();
        let rendered = formatter.render(text);

        // Add indentation for all lines to fit within box
        for line in rendered.lines() {
            println!("  {}", line);
        }

        // Print footer separator
        println!();
        Self::print_separator();
    }

    /// Print the response header with Syncable styling
    fn print_header() {
        print!("{}{}╭─ 🤖 Syncable AI ", brand::PURPLE, brand::BOLD);
        println!(
            "{}─────────────────────────────────────────────────────╮{}",
            brand::DIM,
            brand::RESET
        );
    }

    /// Print a separator line
    fn print_separator() {
        println!(
            "{}╰───────────────────────────────────────────────────────────────────╯{}",
            brand::DIM,
            brand::RESET
        );
    }
}

/// Simple response printer for when we just want colored output
pub struct SimpleResponse;

impl SimpleResponse {
    /// Print a simple AI response with minimal formatting
    pub fn print(text: &str) {
        println!();
        println!(
            "{}{} Syncable AI:{}",
            brand::PURPLE,
            brand::BOLD,
            brand::RESET
        );
        let formatter = MarkdownFormat::new();
        println!("{}", formatter.render(text));
        println!();
    }
}

/// Tool execution display during processing
pub struct ToolProgress {
    tools_executed: Vec<ToolExecution>,
}

#[derive(Clone)]
struct ToolExecution {
    name: String,
    description: String,
    status: ToolStatus,
}

#[derive(Clone, Copy)]
enum ToolStatus {
    Running,
    Success,
    Error,
}

impl ToolProgress {
    pub fn new() -> Self {
        Self {
            tools_executed: Vec::new(),
        }
    }

    /// Mark a tool as starting execution
    pub fn tool_start(&mut self, name: &str, description: &str) {
        self.tools_executed.push(ToolExecution {
            name: name.to_string(),
            description: description.to_string(),
            status: ToolStatus::Running,
        });
        self.redraw();
    }

    /// Mark the last tool as complete
    pub fn tool_complete(&mut self, success: bool) {
        if let Some(tool) = self.tools_executed.last_mut() {
            tool.status = if success {
                ToolStatus::Success
            } else {
                ToolStatus::Error
            };
        }
        self.redraw();
    }

    /// Redraw the tool progress display
    fn redraw(&self) {
        for tool in &self.tools_executed {
            let (icon, color) = match tool.status {
                ToolStatus::Running => ("", brand::YELLOW),
                ToolStatus::Success => ("", brand::SUCCESS),
                ToolStatus::Error => ("", "\x1b[38;5;196m"),
            };
            println!(
                "  {} {}{}{} {}{}{}",
                icon,
                color,
                tool.name,
                brand::RESET,
                brand::DIM,
                tool.description,
                brand::RESET
            );
        }
    }

    /// Print final summary after all tools complete
    pub fn print_summary(&self) {
        if !self.tools_executed.is_empty() {
            let success_count = self
                .tools_executed
                .iter()
                .filter(|t| matches!(t.status, ToolStatus::Success))
                .count();
            println!(
                "\n{}  {} tools executed successfully{}",
                brand::DIM,
                success_count,
                brand::RESET
            );
        }
    }
}

impl Default for ToolProgress {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_render_empty() {
        let formatter = MarkdownFormat::new();
        assert!(formatter.render("").is_empty());
    }

    #[test]
    fn test_markdown_render_simple() {
        let formatter = MarkdownFormat::new();
        let result = formatter.render("Hello world");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_code_block_extraction() {
        let parsed = CodeBlockParser::parse("Hello\n```rust\nfn main() {}\n```\nWorld");
        assert_eq!(parsed.blocks.len(), 1);
        assert_eq!(parsed.blocks[0].lang, "rust");
        assert_eq!(parsed.blocks[0].code, "fn main() {}");
    }

    #[test]
    fn test_syntax_highlighter() {
        let hl = SyntaxHighlighter::default();
        let result = hl.highlight("fn main() {}", "rust");
        // Should contain ANSI codes
        assert!(result.contains("\x1b["));
    }
}
