//! Beautiful response formatting for AI outputs
//!
//! Renders AI responses with Syncable's brand colors (purple/magenta theme)
//! and nice markdown-like formatting.

// Note: colored crate is used in other modules, here we use custom ANSI codes

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
    /// Reset
    pub const RESET: &str = "\x1b[0m";
    /// Bold
    pub const BOLD: &str = "\x1b[1m";
    /// Italic
    pub const ITALIC: &str = "\x1b[3m";
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

        // Parse and format the markdown content
        Self::format_markdown(text);

        // Print footer separator
        println!();
        Self::print_separator();
    }

    /// Print the response header with Syncable styling
    fn print_header() {
        print!(
            "{}{}╭─ {} Syncable AI {}{}",
            brand::PURPLE,
            brand::BOLD,
            "🤖",
            brand::RESET,
            brand::DIM
        );
        println!("─────────────────────────────────────────────────────╮{}", brand::RESET);
    }

    /// Print a separator line
    fn print_separator() {
        println!(
            "{}╰───────────────────────────────────────────────────────────────────╯{}",
            brand::DIM,
            brand::RESET
        );
    }

    /// Format and print markdown content with nice styling
    fn format_markdown(text: &str) {
        let mut in_code_block = false;
        let mut code_lang = String::new();
        let mut list_depth = 0;

        for line in text.lines() {
            let trimmed = line.trim();

            // Handle code blocks
            if trimmed.starts_with("```") {
                if in_code_block {
                    // End code block
                    println!(
                        "{}  └────────────────────────────────────────────────────────────┘{}",
                        brand::DIM,
                        brand::RESET
                    );
                    in_code_block = false;
                    code_lang.clear();
                } else {
                    // Start code block
                    code_lang = trimmed.strip_prefix("```").unwrap_or("").to_string();
                    let lang_display = if code_lang.is_empty() {
                        "code".to_string()
                    } else {
                        code_lang.clone()
                    };
                    println!(
                        "{}  ┌─ {}{}{} ──────────────────────────────────────────────────────┐{}",
                        brand::DIM,
                        brand::CYAN,
                        lang_display,
                        brand::DIM,
                        brand::RESET
                    );
                    in_code_block = true;
                }
                continue;
            }

            if in_code_block {
                // Code content with syntax highlighting hint
                println!("{}  │ {}{}{}  │", brand::DIM, brand::CYAN, line, brand::RESET);
                continue;
            }

            // Handle headers
            if let Some(header) = Self::parse_header(trimmed) {
                Self::print_formatted_header(header.0, header.1);
                continue;
            }

            // Handle bullet points
            if let Some(bullet) = Self::parse_bullet(trimmed) {
                Self::print_bullet(bullet.0, bullet.1, &mut list_depth);
                continue;
            }

            // Handle bold and inline code in regular text
            Self::print_formatted_text(line);
        }
    }

    /// Parse header level and content
    fn parse_header(line: &str) -> Option<(usize, &str)> {
        if line.starts_with("### ") {
            Some((3, line.strip_prefix("### ").unwrap()))
        } else if line.starts_with("## ") {
            Some((2, line.strip_prefix("## ").unwrap()))
        } else if line.starts_with("# ") {
            Some((1, line.strip_prefix("# ").unwrap()))
        } else {
            None
        }
    }

    /// Print a formatted header
    fn print_formatted_header(level: usize, content: &str) {
        match level {
            1 => {
                println!();
                println!(
                    "{}{}  ▓▓ {} {}",
                    brand::PURPLE,
                    brand::BOLD,
                    content.to_uppercase(),
                    brand::RESET
                );
                println!(
                    "{}  ════════════════════════════════════════════════════════{}",
                    brand::PURPLE,
                    brand::RESET
                );
            }
            2 => {
                println!();
                println!(
                    "{}{}  ▸ {} {}",
                    brand::LIGHT_PURPLE,
                    brand::BOLD,
                    content,
                    brand::RESET
                );
                println!(
                    "{}  ────────────────────────────────────────────────────────{}",
                    brand::DIM,
                    brand::RESET
                );
            }
            _ => {
                println!();
                println!(
                    "{}{}  ◦ {} {}",
                    brand::MAGENTA,
                    brand::BOLD,
                    content,
                    brand::RESET
                );
            }
        }
    }

    /// Parse bullet point
    fn parse_bullet(line: &str) -> Option<(usize, &str)> {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let depth = indent / 2;

        if trimmed.starts_with("- ") {
            Some((depth, trimmed.strip_prefix("- ").unwrap()))
        } else if trimmed.starts_with("* ") {
            Some((depth, trimmed.strip_prefix("* ").unwrap()))
        } else if trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) 
            && trimmed.chars().nth(1) == Some('.') 
        {
            Some((depth, trimmed.split_once(". ").map(|(_, rest)| rest).unwrap_or(trimmed)))
        } else {
            None
        }
    }

    /// Print a bullet point with proper indentation
    fn print_bullet(depth: usize, content: &str, _list_depth: &mut usize) {
        let indent = "  ".repeat(depth + 1);
        let bullet_char = match depth {
            0 => "●",
            1 => "○",
            _ => "◦",
        };
        let bullet_color = match depth {
            0 => brand::PURPLE,
            1 => brand::MAGENTA,
            _ => brand::DIM,
        };

        // Format the content with inline styles
        let formatted = Self::format_inline(content);
        println!("{}{}{} {}{}", indent, bullet_color, bullet_char, brand::TEXT, formatted);
        print!("{}", brand::RESET);
    }

    /// Print formatted text with inline styles
    fn print_formatted_text(line: &str) {
        if line.trim().is_empty() {
            println!();
            return;
        }

        let formatted = Self::format_inline(line);
        println!("{}  {}{}", brand::TEXT, formatted, brand::RESET);
    }

    /// Format inline markdown (bold, italic, code)
    fn format_inline(text: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Handle **bold**
            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
                if let Some(end) = Self::find_closing(&chars, i + 2, "**") {
                    let bold_text: String = chars[i + 2..end].iter().collect();
                    result.push_str(brand::BOLD);
                    result.push_str(brand::LIGHT_PURPLE);
                    result.push_str(&bold_text);
                    result.push_str(brand::RESET);
                    result.push_str(brand::TEXT);
                    i = end + 2;
                    continue;
                }
            }

            // Handle `code`
            if chars[i] == '`' && (i + 1 >= chars.len() || chars[i + 1] != '`') {
                if let Some(end) = chars[i + 1..].iter().position(|&c| c == '`') {
                    let code_text: String = chars[i + 1..i + 1 + end].iter().collect();
                    result.push_str(brand::CYAN);
                    result.push_str("`");
                    result.push_str(&code_text);
                    result.push_str("`");
                    result.push_str(brand::RESET);
                    result.push_str(brand::TEXT);
                    i = i + 2 + end;
                    continue;
                }
            }

            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// Find closing marker
    fn find_closing(chars: &[char], start: usize, marker: &str) -> Option<usize> {
        let marker_chars: Vec<char> = marker.chars().collect();
        let marker_len = marker_chars.len();

        for i in start..=chars.len() - marker_len {
            let matches = (0..marker_len).all(|j| chars[i + j] == marker_chars[j]);
            if matches {
                return Some(i);
            }
        }
        None
    }
}

/// Simple response printer for when we just want colored output
pub struct SimpleResponse;

impl SimpleResponse {
    /// Print a simple AI response with minimal formatting
    pub fn print(text: &str) {
        println!();
        println!("{}{}🤖 Syncable AI:{}", brand::PURPLE, brand::BOLD, brand::RESET);
        println!("{}{}{}", brand::TEXT, text, brand::RESET);
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
            tool.status = if success { ToolStatus::Success } else { ToolStatus::Error };
        }
        self.redraw();
    }

    /// Redraw the tool progress display
    fn redraw(&self) {
        // Clear previous lines and redraw
        for tool in &self.tools_executed {
            let (icon, color) = match tool.status {
                ToolStatus::Running => ("◐", brand::YELLOW),
                ToolStatus::Success => ("✓", brand::SUCCESS),
                ToolStatus::Error => ("✗", "\x1b[38;5;196m"),
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
            let success_count = self.tools_executed
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
    fn test_parse_header() {
        assert_eq!(ResponseFormatter::parse_header("# Hello"), Some((1, "Hello")));
        assert_eq!(ResponseFormatter::parse_header("## World"), Some((2, "World")));
        assert_eq!(ResponseFormatter::parse_header("### Test"), Some((3, "Test")));
        assert_eq!(ResponseFormatter::parse_header("Not a header"), None);
    }

    #[test]
    fn test_format_inline_bold() {
        let result = ResponseFormatter::format_inline("This is **bold** text");
        assert!(result.contains("bold"));
    }
}
