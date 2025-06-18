//! Box drawing utilities for creating formatted text boxes in the terminal

use colored::*;
use crate::analyzer::display::utils::{visual_width, truncate_to_width};

/// Content line for measuring and drawing
#[derive(Debug, Clone)]
struct ContentLine {
    label: String,
    value: String,
    label_colored: bool,
}

impl ContentLine {
    fn new(label: &str, value: &str, label_colored: bool) -> Self {
        Self {
            label: label.to_string(),
            value: value.to_string(),
            label_colored,
        }
    }
    
    fn separator() -> Self {
        Self {
            label: "SEPARATOR".to_string(),
            value: String::new(),
            label_colored: false,
        }
    }
}

/// Box drawer that pre-calculates optimal dimensions
pub struct BoxDrawer {
    title: String,
    lines: Vec<ContentLine>,
    min_width: usize,
    max_width: usize,
}

impl BoxDrawer {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            lines: Vec::new(),
            min_width: 60,
            max_width: 120, // Reduced from 150 for better terminal compatibility
        }
    }
    
    pub fn add_line(&mut self, label: &str, value: &str, label_colored: bool) {
        self.lines.push(ContentLine::new(label, value, label_colored));
    }
    
    pub fn add_value_only(&mut self, value: &str) {
        self.lines.push(ContentLine::new("", value, false));
    }
    
    pub fn add_separator(&mut self) {
        self.lines.push(ContentLine::separator());
    }
    
    /// Calculate optimal box width based on content
    fn calculate_optimal_width(&self) -> usize {
        let title_width = visual_width(&self.title) + 6; // "┌─ " + title + " " + extra padding
        let mut max_content_width = 0;
        
        // Calculate the actual rendered width for each line
        for line in &self.lines {
            if line.label == "SEPARATOR" {
                continue;
            }
            
            let rendered_width = self.calculate_rendered_line_width(line);
            max_content_width = max_content_width.max(rendered_width);
        }
        
        // Add reasonable buffer for content
        let content_width_with_buffer = max_content_width + 4; // More buffer for safety
        
        // Box needs padding: "│ " + content + " │" = content + 4
        let needed_width = content_width_with_buffer + 4;
        
        // Use the maximum of title width and content width
        let optimal_width = title_width.max(needed_width).max(self.min_width);
        optimal_width.min(self.max_width)
    }
    
    /// Calculate the actual rendered width of a line as it will appear
    fn calculate_rendered_line_width(&self, line: &ContentLine) -> usize {
        let label_width = visual_width(&line.label);
        let value_width = visual_width(&line.value);
        
        if !line.label.is_empty() && !line.value.is_empty() {
            // Label + value: need space between them
            // For colored labels, ensure minimum spacing
            let min_label_space = if line.label_colored { 25 } else { label_width };
            min_label_space + 2 + value_width // 2 spaces minimum between label and value
        } else if !line.value.is_empty() {
            // Value only
            value_width
        } else if !line.label.is_empty() {
            // Label only
            label_width
        } else {
            // Empty line
            0
        }
    }
    
    /// Draw the complete box
    pub fn draw(&self) -> String {
        let box_width = self.calculate_optimal_width();
        let content_width = box_width - 4; // Available space for content
        
        let mut output = Vec::new();
        
        // Top border
        output.push(self.draw_top(box_width));
        
        // Content lines
        for line in &self.lines {
            if line.label == "SEPARATOR" {
                output.push(self.draw_separator(box_width));
            } else if line.label.is_empty() && line.value.is_empty() {
                output.push(self.draw_empty_line(box_width));
            } else {
                output.push(self.draw_content_line(line, content_width));
            }
        }
        
        // Bottom border
        output.push(self.draw_bottom(box_width));
        
        output.join("\n")
    }
    
    fn draw_top(&self, width: usize) -> String {
        let title_colored = self.title.bright_cyan();
        let title_len = visual_width(&self.title);
        
        // "┌─ " + title + " " + remaining dashes + "┐"
        let prefix_len = 3; // "┌─ "
        let suffix_len = 1; // "┐"
        let title_space = 1; // space after title
        
        let remaining_space = width - prefix_len - title_len - title_space - suffix_len;
        
        format!("┌─ {} {}┐", 
            title_colored,
            "─".repeat(remaining_space)
        )
    }
    
    fn draw_bottom(&self, width: usize) -> String {
        format!("└{}┘", "─".repeat(width - 2))
    }
    
    fn draw_separator(&self, width: usize) -> String {
        format!("│ {} │", "─".repeat(width - 4).dimmed())
    }
    
    fn draw_empty_line(&self, width: usize) -> String {
        format!("│ {} │", " ".repeat(width - 4))
    }
    
    fn draw_content_line(&self, line: &ContentLine, content_width: usize) -> String {
        // Format the label with color if needed
        let formatted_label = if line.label_colored && !line.label.is_empty() {
            line.label.bright_white().to_string()
        } else {
            line.label.clone()
        };
        
        // Calculate actual display widths (use original label for width)
        let label_display_width = visual_width(&line.label);
        let value_display_width = visual_width(&line.value);
        
        // Build the content
        let content = if !line.label.is_empty() && !line.value.is_empty() {
            // Both label and value - ensure proper spacing
            let min_label_space = if line.label_colored { 25 } else { label_display_width };
            let label_padding = min_label_space.saturating_sub(label_display_width);
            let remaining_space = content_width.saturating_sub(min_label_space + 2); // 2 for spacing
            
            if value_display_width <= remaining_space {
                // Value fits - right align it
                let value_padding = remaining_space.saturating_sub(value_display_width);
                format!("{}{:<width$}  {}{}", 
                    formatted_label, 
                    "",
                    " ".repeat(value_padding),
                    line.value,
                    width = label_padding
                )
            } else {
                // Value too long - truncate it
                let truncated_value = truncate_to_width(&line.value, remaining_space.saturating_sub(3));
                format!("{}{:<width$}  {}", 
                    formatted_label, 
                    "",
                    truncated_value,
                    width = label_padding
                )
            }
        } else if !line.value.is_empty() {
            // Value only - left align
            if value_display_width <= content_width {
                format!("{:<width$}", line.value, width = content_width)
            } else {
                truncate_to_width(&line.value, content_width)
            }
        } else if !line.label.is_empty() {
            // Label only - left align
            if label_display_width <= content_width {
                format!("{:<width$}", formatted_label, width = content_width)
            } else {
                truncate_to_width(&formatted_label, content_width)
            }
        } else {
            // Empty line
            " ".repeat(content_width)
        };
        
        // Ensure final content is exactly the right width
        let actual_width = visual_width(&content);
        let final_content = if actual_width < content_width {
            format!("{}{}", content, " ".repeat(content_width - actual_width))
        } else if actual_width > content_width {
            truncate_to_width(&content, content_width)
        } else {
            content
        };
        
        format!("│ {} │", final_content)
    }
} 