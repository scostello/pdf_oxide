//! High-level PDF builder and document type.
//!
//! Provides `Pdf` for simple operations and `PdfBuilder` for customized creation.

use crate::editor::DocumentEditor;
use crate::error::Result;
use crate::writer::{DocumentBuilder, DocumentMetadata, PageSize};
use std::fs;
use std::path::Path;

/// Column alignment for GFM tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GfmAlign {
    Left,
    Center,
    Right,
}

/// A parsed GFM table.
#[derive(Debug)]
struct GfmTable {
    /// Header row cells
    headers: Vec<String>,
    /// Data rows
    rows: Vec<Vec<String>>,
    /// Column alignments
    alignments: Vec<GfmAlign>,
}

impl GfmTable {
    /// Parse a GFM table from lines.
    fn parse(lines: &[&str]) -> Option<Self> {
        if lines.len() < 2 {
            return None;
        }

        // Parse header row
        let headers = Self::parse_row(lines[0])?;
        if headers.is_empty() {
            return None;
        }

        // Parse separator row and extract alignments
        let alignments = Self::parse_separator(lines[1], headers.len())?;

        // Parse data rows
        let mut rows = Vec::new();
        for line in &lines[2..] {
            if let Some(row) = Self::parse_row(line) {
                // Pad or truncate row to match header column count
                let mut padded_row = row;
                padded_row.resize(headers.len(), String::new());
                rows.push(padded_row);
            }
        }

        Some(Self {
            headers,
            rows,
            alignments,
        })
    }

    /// Parse a table row (header or data).
    fn parse_row(line: &str) -> Option<Vec<String>> {
        let trimmed = line.trim();
        if !trimmed.contains('|') {
            return None;
        }

        // Remove leading/trailing pipes and split
        let content = trimmed.trim_start_matches('|').trim_end_matches('|');
        let cells: Vec<String> = content.split('|').map(|s| s.trim().to_string()).collect();

        if cells.is_empty() {
            None
        } else {
            Some(cells)
        }
    }

    /// Parse the separator row and extract alignments.
    fn parse_separator(line: &str, expected_cols: usize) -> Option<Vec<GfmAlign>> {
        let trimmed = line.trim();
        if !trimmed.contains('|') || !trimmed.contains('-') {
            return None;
        }

        let content = trimmed.trim_start_matches('|').trim_end_matches('|');
        let parts: Vec<&str> = content.split('|').map(|s| s.trim()).collect();

        // Validate it looks like a separator
        if parts.iter().any(|p| !Self::is_separator_cell(p)) {
            return None;
        }

        let mut alignments: Vec<GfmAlign> = parts
            .iter()
            .map(|p| {
                let has_left_colon = p.starts_with(':');
                let has_right_colon = p.ends_with(':');
                match (has_left_colon, has_right_colon) {
                    (true, true) => GfmAlign::Center,
                    (false, true) => GfmAlign::Right,
                    _ => GfmAlign::Left,
                }
            })
            .collect();

        // Pad with default alignment if needed
        alignments.resize(expected_cols, GfmAlign::Left);

        Some(alignments)
    }

    /// Check if a string looks like a separator cell (dashes with optional colons).
    fn is_separator_cell(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }
        let stripped = s.trim_start_matches(':').trim_end_matches(':');
        !stripped.is_empty() && stripped.chars().all(|c| c == '-')
    }

    /// Calculate column widths based on content.
    fn column_widths(&self) -> Vec<usize> {
        let mut widths: Vec<usize> = self.headers.iter().map(|h| h.len()).collect();

        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.len());
                }
            }
        }

        // Minimum width of 3 for readability
        widths.iter().map(|w| (*w).max(3)).collect()
    }

    /// Render the table as formatted text lines.
    fn render(&self) -> Vec<String> {
        let widths = self.column_widths();
        let mut lines = Vec::new();

        // Render header
        lines.push(self.render_row(&self.headers, &widths, &self.alignments));

        // Render separator
        lines.push(self.render_separator(&widths, &self.alignments));

        // Render data rows
        for row in &self.rows {
            lines.push(self.render_row(row, &widths, &self.alignments));
        }

        lines
    }

    /// Render a single row with proper padding and alignment.
    fn render_row(&self, cells: &[String], widths: &[usize], alignments: &[GfmAlign]) -> String {
        let mut parts = Vec::new();
        for (i, cell) in cells.iter().enumerate() {
            let width = widths.get(i).copied().unwrap_or(3);
            let align = alignments.get(i).copied().unwrap_or(GfmAlign::Left);
            let formatted = match align {
                GfmAlign::Left => format!("{:<width$}", cell, width = width),
                GfmAlign::Center => format!("{:^width$}", cell, width = width),
                GfmAlign::Right => format!("{:>width$}", cell, width = width),
            };
            parts.push(formatted);
        }
        format!("| {} |", parts.join(" | "))
    }

    /// Render the separator row.
    fn render_separator(&self, widths: &[usize], alignments: &[GfmAlign]) -> String {
        let mut parts = Vec::new();
        for (i, width) in widths.iter().enumerate() {
            let align = alignments.get(i).copied().unwrap_or(GfmAlign::Left);
            let dashes = "-".repeat(*width);
            let sep = match align {
                GfmAlign::Left => format!(":{}", dashes),
                GfmAlign::Center => format!(":{}:", &dashes[..dashes.len().saturating_sub(1)]),
                GfmAlign::Right => format!("{}:", dashes),
            };
            parts.push(sep);
        }
        format!("|{}|", parts.join("|"))
    }
}

/// Check if a line looks like a GFM table row.
fn is_table_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() > 2
}

/// Configuration for PDF generation.
#[derive(Debug, Clone)]
pub struct PdfConfig {
    /// Document title
    pub title: Option<String>,
    /// Document author
    pub author: Option<String>,
    /// Document subject
    pub subject: Option<String>,
    /// Document keywords
    pub keywords: Option<String>,
    /// Page size
    pub page_size: PageSize,
    /// Left margin in points
    pub margin_left: f32,
    /// Right margin in points
    pub margin_right: f32,
    /// Top margin in points
    pub margin_top: f32,
    /// Bottom margin in points
    pub margin_bottom: f32,
    /// Default font size
    pub font_size: f32,
    /// Line height multiplier
    pub line_height: f32,
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            title: None,
            author: None,
            subject: None,
            keywords: None,
            page_size: PageSize::Letter,
            margin_left: 72.0,   // 1 inch
            margin_right: 72.0,  // 1 inch
            margin_top: 72.0,    // 1 inch
            margin_bottom: 72.0, // 1 inch
            font_size: 12.0,
            line_height: 1.5,
        }
    }
}

/// A high-level PDF document.
///
/// This type provides a simple API for creating and manipulating PDFs.
/// For more complex operations, use `DocumentEditor` directly.
#[derive(Debug)]
pub struct Pdf {
    /// The underlying PDF bytes
    bytes: Vec<u8>,
    /// Configuration used to create this PDF
    config: PdfConfig,
}

impl Pdf {
    /// Create a new empty PDF.
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            config: PdfConfig::default(),
        }
    }

    /// Create a PDF from Markdown content.
    ///
    /// Supports common Markdown features:
    /// - Headings (# H1, ## H2, etc.)
    /// - Paragraphs
    /// - Bold and italic text
    /// - Lists (ordered and unordered)
    /// - Code blocks
    /// - Blockquotes
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdf_oxide::api::Pdf;
    ///
    /// let pdf = Pdf::from_markdown("# Hello World\n\nThis is **bold** text.")?;
    /// pdf.save("output.pdf")?;
    /// ```
    pub fn from_markdown(content: &str) -> Result<Self> {
        PdfBuilder::new().from_markdown(content)
    }

    /// Create a PDF from HTML content.
    ///
    /// Supports basic HTML elements:
    /// - `<h1>` through `<h6>` headings
    /// - `<p>` paragraphs
    /// - `<b>`, `<strong>` for bold
    /// - `<i>`, `<em>` for italic
    /// - `<ul>`, `<ol>`, `<li>` for lists
    /// - `<pre>`, `<code>` for code
    /// - `<blockquote>` for quotes
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdf_oxide::api::Pdf;
    ///
    /// let pdf = Pdf::from_html("<h1>Hello</h1><p>World</p>")?;
    /// pdf.save("output.pdf")?;
    /// ```
    pub fn from_html(content: &str) -> Result<Self> {
        PdfBuilder::new().from_html(content)
    }

    /// Create a PDF from plain text.
    ///
    /// The text is rendered as-is with the default font and size.
    /// Line breaks in the input are preserved.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdf_oxide::api::Pdf;
    ///
    /// let pdf = Pdf::from_text("Hello, World!\n\nThis is plain text.")?;
    /// pdf.save("output.pdf")?;
    /// ```
    pub fn from_text(content: &str) -> Result<Self> {
        PdfBuilder::new().from_text(content)
    }

    /// Open an existing PDF file.
    ///
    /// Returns a `DocumentEditor` for modifying the PDF.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdf_oxide::api::Pdf;
    ///
    /// let mut editor = Pdf::open("existing.pdf")?;
    /// editor.set_title("New Title");
    /// editor.save("modified.pdf")?;
    /// ```
    pub fn open(path: impl AsRef<Path>) -> Result<DocumentEditor> {
        DocumentEditor::open(path)
    }

    /// Get the PDF bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Convert to PDF bytes, consuming the Pdf.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Save the PDF to a file.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdf_oxide::api::Pdf;
    ///
    /// let pdf = Pdf::from_markdown("# Hello")?;
    /// pdf.save("output.pdf")?;
    /// ```
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        fs::write(path.as_ref(), &self.bytes)?;
        Ok(())
    }

    /// Get the configuration used to create this PDF.
    pub fn config(&self) -> &PdfConfig {
        &self.config
    }
}

impl Default for Pdf {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating PDFs with custom configuration.
///
/// Use this for more control over the PDF generation process.
///
/// # Example
///
/// ```ignore
/// use pdf_oxide::api::PdfBuilder;
/// use pdf_oxide::writer::PageSize;
///
/// let pdf = PdfBuilder::new()
///     .title("My Document")
///     .author("John Doe")
///     .page_size(PageSize::A4)
///     .margins(50.0, 50.0, 50.0, 50.0)
///     .font_size(11.0)
///     .from_markdown("# Content")?;
/// ```
#[derive(Debug, Clone)]
pub struct PdfBuilder {
    config: PdfConfig,
}

impl PdfBuilder {
    /// Create a new PDF builder with default configuration.
    pub fn new() -> Self {
        Self {
            config: PdfConfig::default(),
        }
    }

    /// Set the document title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.config.title = Some(title.into());
        self
    }

    /// Set the document author.
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.config.author = Some(author.into());
        self
    }

    /// Set the document subject.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.config.subject = Some(subject.into());
        self
    }

    /// Set the document keywords.
    pub fn keywords(mut self, keywords: impl Into<String>) -> Self {
        self.config.keywords = Some(keywords.into());
        self
    }

    /// Set the page size.
    pub fn page_size(mut self, size: PageSize) -> Self {
        self.config.page_size = size;
        self
    }

    /// Set all margins to the same value.
    pub fn margin(mut self, margin: f32) -> Self {
        self.config.margin_left = margin;
        self.config.margin_right = margin;
        self.config.margin_top = margin;
        self.config.margin_bottom = margin;
        self
    }

    /// Set individual margins (left, right, top, bottom).
    pub fn margins(mut self, left: f32, right: f32, top: f32, bottom: f32) -> Self {
        self.config.margin_left = left;
        self.config.margin_right = right;
        self.config.margin_top = top;
        self.config.margin_bottom = bottom;
        self
    }

    /// Set the default font size.
    pub fn font_size(mut self, size: f32) -> Self {
        self.config.font_size = size;
        self
    }

    /// Set the line height multiplier.
    pub fn line_height(mut self, height: f32) -> Self {
        self.config.line_height = height;
        self
    }

    /// Build a PDF from Markdown content.
    pub fn from_markdown(self, content: &str) -> Result<Pdf> {
        let bytes = self.render_markdown(content)?;
        Ok(Pdf {
            bytes,
            config: self.config,
        })
    }

    /// Build a PDF from HTML content.
    pub fn from_html(self, content: &str) -> Result<Pdf> {
        let bytes = self.render_html(content)?;
        Ok(Pdf {
            bytes,
            config: self.config,
        })
    }

    /// Build a PDF from plain text.
    pub fn from_text(self, content: &str) -> Result<Pdf> {
        let bytes = self.render_text(content)?;
        Ok(Pdf {
            bytes,
            config: self.config,
        })
    }

    /// Render Markdown content to PDF bytes.
    #[allow(clippy::manual_strip)]
    fn render_markdown(&self, content: &str) -> Result<Vec<u8>> {
        let mut builder = DocumentBuilder::new();

        // Set metadata
        let mut metadata = DocumentMetadata::new();
        if let Some(ref title) = self.config.title {
            metadata = metadata.title(title);
        }
        if let Some(ref author) = self.config.author {
            metadata = metadata.author(author);
        }
        if let Some(ref subject) = self.config.subject {
            metadata = metadata.subject(subject);
        }
        builder = builder.metadata(metadata);

        // Parse and render Markdown
        let (_page_width, page_height) = self.config.page_size.dimensions();
        let start_y = page_height - self.config.margin_top;

        // Collect text items with positions
        let mut text_items: Vec<(f32, f32, String)> = Vec::new();
        let mut y = start_y;
        let mut in_code = false;

        // Collect all lines for table detection
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Check if this starts a table (need at least header + separator)
            if !in_code && is_table_line(line) && i + 1 < lines.len() {
                // Collect consecutive table lines
                let mut table_lines = vec![line];
                let mut j = i + 1;
                while j < lines.len() && is_table_line(lines[j]) {
                    table_lines.push(lines[j]);
                    j += 1;
                }

                // Try to parse as GFM table
                if let Some(table) = GfmTable::parse(&table_lines) {
                    // Render the table as formatted text
                    let table_font_size = self.config.font_size * 0.9; // Slightly smaller for tables
                    let line_height = table_font_size * self.config.line_height;

                    // Add some space before table
                    y -= line_height * 0.5;

                    for table_line in table.render() {
                        y -= line_height;
                        if y < self.config.margin_bottom {
                            y = start_y - line_height;
                        }
                        text_items.push((self.config.margin_left, y, table_line));
                    }

                    // Add some space after table
                    y -= line_height * 0.5;

                    i = j; // Skip all table lines
                    continue;
                }
            }

            // Handle non-table lines
            if line.starts_with("```") {
                in_code = !in_code;
                y -= self.config.font_size * self.config.line_height;
                i += 1;
                continue;
            }

            let (text, font_size) = if in_code {
                (line.to_string(), self.config.font_size * 0.9)
            } else if line.starts_with("# ") {
                (line[2..].to_string(), self.config.font_size * 2.0)
            } else if line.starts_with("## ") {
                (line[3..].to_string(), self.config.font_size * 1.5)
            } else if line.starts_with("### ") {
                (line[4..].to_string(), self.config.font_size * 1.25)
            } else if line.starts_with("#### ") {
                (line[5..].to_string(), self.config.font_size * 1.1)
            } else if line.starts_with("- ") || line.starts_with("* ") {
                (format!("• {}", &line[2..]), self.config.font_size)
            } else if line.starts_with("> ") {
                (format!("  {}", &line[2..]), self.config.font_size)
            } else if line.trim().is_empty() {
                y -= self.config.font_size * self.config.line_height;
                i += 1;
                continue;
            } else {
                // Strip basic formatting markers
                let text = line
                    .replace("**", "")
                    .replace("__", "")
                    .replace("*", "")
                    .replace("_", "");
                (text, self.config.font_size)
            };

            let line_height = font_size * self.config.line_height;
            y -= line_height;

            if y < self.config.margin_bottom {
                y = start_y - line_height;
            }

            if !text.is_empty() {
                text_items.push((self.config.margin_left, y, text));
            }

            i += 1;
        }

        // Render all items
        {
            let mut page = builder.page(self.config.page_size);
            for (x, y, text) in text_items {
                page = page.at(x, y).text(&text);
            }
            page.done();
        }

        builder.build()
    }

    /// Render HTML content to PDF bytes.
    fn render_html(&self, content: &str) -> Result<Vec<u8>> {
        // Simple HTML to Markdown conversion, then render as Markdown
        let markdown = self.html_to_markdown(content);
        self.render_markdown(&markdown)
    }

    /// Convert basic HTML to Markdown.
    fn html_to_markdown(&self, html: &str) -> String {
        let mut result = html.to_string();

        // Replace common HTML tags with Markdown equivalents
        result = result.replace("<h1>", "# ").replace("</h1>", "\n");
        result = result.replace("<h2>", "## ").replace("</h2>", "\n");
        result = result.replace("<h3>", "### ").replace("</h3>", "\n");
        result = result.replace("<h4>", "#### ").replace("</h4>", "\n");
        result = result.replace("<h5>", "##### ").replace("</h5>", "\n");
        result = result.replace("<h6>", "###### ").replace("</h6>", "\n");

        result = result.replace("<p>", "").replace("</p>", "\n\n");
        result = result
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<br />", "\n");

        result = result.replace("<strong>", "**").replace("</strong>", "**");
        result = result.replace("<b>", "**").replace("</b>", "**");
        result = result.replace("<em>", "*").replace("</em>", "*");
        result = result.replace("<i>", "*").replace("</i>", "*");

        result = result.replace("<code>", "`").replace("</code>", "`");
        result = result.replace("<pre>", "```\n").replace("</pre>", "\n```");

        result = result
            .replace("<blockquote>", "> ")
            .replace("</blockquote>", "\n");

        result = result.replace("<ul>", "").replace("</ul>", "");
        result = result.replace("<ol>", "").replace("</ol>", "");
        result = result.replace("<li>", "- ").replace("</li>", "\n");

        // Remove any remaining HTML tags
        let mut in_tag = false;
        let mut cleaned = String::new();
        for c in result.chars() {
            if c == '<' {
                in_tag = true;
            } else if c == '>' {
                in_tag = false;
            } else if !in_tag {
                cleaned.push(c);
            }
        }

        // Clean up extra whitespace
        let lines: Vec<&str> = cleaned.lines().collect();
        lines.join("\n")
    }

    /// Render plain text to PDF bytes.
    fn render_text(&self, content: &str) -> Result<Vec<u8>> {
        let mut builder = DocumentBuilder::new();

        // Set metadata
        let mut metadata = DocumentMetadata::new();
        if let Some(ref title) = self.config.title {
            metadata = metadata.title(title);
        }
        if let Some(ref author) = self.config.author {
            metadata = metadata.author(author);
        }
        builder = builder.metadata(metadata);

        // Render text
        let (_page_width, page_height) = self.config.page_size.dimensions();
        let start_y = page_height - self.config.margin_top;
        let line_height = self.config.font_size * self.config.line_height;

        // Collect lines with their positions
        let mut text_items: Vec<(f32, f32, String)> = Vec::new();
        let mut y = start_y;

        for line in content.lines() {
            y -= line_height;

            if y < self.config.margin_bottom {
                y = start_y - line_height;
            }

            if !line.is_empty() {
                text_items.push((self.config.margin_left, y, line.to_string()));
            }
        }

        // Now render all items on a single page
        {
            let mut page = builder.page(self.config.page_size);
            for (x, y, text) in text_items {
                page = page.at(x, y).text(&text);
            }
            page.done();
        }

        builder.build()
    }
}

impl Default for PdfBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_config_default() {
        let config = PdfConfig::default();
        assert_eq!(config.margin_left, 72.0);
        assert_eq!(config.font_size, 12.0);
        assert!(config.title.is_none());
    }

    #[test]
    fn test_pdf_builder_chain() {
        let builder = PdfBuilder::new()
            .title("Test")
            .author("Author")
            .subject("Subject")
            .keywords("test, pdf")
            .page_size(PageSize::A4)
            .margin(50.0)
            .font_size(11.0)
            .line_height(1.4);

        assert_eq!(builder.config.title, Some("Test".to_string()));
        assert_eq!(builder.config.author, Some("Author".to_string()));
        assert_eq!(builder.config.margin_left, 50.0);
        assert_eq!(builder.config.font_size, 11.0);
    }

    #[test]
    fn test_pdf_builder_margins() {
        let builder = PdfBuilder::new().margins(10.0, 20.0, 30.0, 40.0);

        assert_eq!(builder.config.margin_left, 10.0);
        assert_eq!(builder.config.margin_right, 20.0);
        assert_eq!(builder.config.margin_top, 30.0);
        assert_eq!(builder.config.margin_bottom, 40.0);
    }

    #[test]
    fn test_pdf_from_text() {
        let result = Pdf::from_text("Hello, World!");
        assert!(result.is_ok());

        let pdf = result.unwrap();
        assert!(!pdf.as_bytes().is_empty());
    }

    #[test]
    fn test_pdf_from_markdown() {
        let result = Pdf::from_markdown("# Hello\n\nWorld");
        assert!(result.is_ok());

        let pdf = result.unwrap();
        assert!(!pdf.as_bytes().is_empty());
    }

    #[test]
    fn test_pdf_from_html() {
        let result = Pdf::from_html("<h1>Hello</h1><p>World</p>");
        assert!(result.is_ok());

        let pdf = result.unwrap();
        assert!(!pdf.as_bytes().is_empty());
    }

    #[test]
    fn test_html_to_markdown() {
        let builder = PdfBuilder::new();
        let md = builder.html_to_markdown("<h1>Title</h1><p>Text</p>");

        assert!(md.contains("# Title"));
        assert!(md.contains("Text"));
    }

    #[test]
    fn test_pdf_into_bytes() {
        let pdf = Pdf::from_text("Test").unwrap();
        let bytes = pdf.into_bytes();
        assert!(!bytes.is_empty());
        assert!(bytes.starts_with(b"%PDF"));
    }

    // GFM Table parsing tests

    #[test]
    fn test_gfm_table_parse_simple() {
        let lines = vec![
            "| Name | Age |",
            "|------|-----|",
            "| Alice | 30 |",
            "| Bob | 25 |",
        ];
        let table = super::GfmTable::parse(&lines);
        assert!(table.is_some());

        let table = table.unwrap();
        assert_eq!(table.headers.len(), 2);
        assert_eq!(table.headers[0], "Name");
        assert_eq!(table.headers[1], "Age");
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0][0], "Alice");
        assert_eq!(table.rows[0][1], "30");
    }

    #[test]
    fn test_gfm_table_alignments() {
        let lines = vec![
            "| Left | Center | Right |",
            "|:-----|:------:|------:|",
            "| L | C | R |",
        ];
        let table = super::GfmTable::parse(&lines).unwrap();

        assert_eq!(table.alignments[0], super::GfmAlign::Left);
        assert_eq!(table.alignments[1], super::GfmAlign::Center);
        assert_eq!(table.alignments[2], super::GfmAlign::Right);
    }

    #[test]
    fn test_gfm_table_render() {
        let lines = vec!["| A | B |", "|---|---|", "| 1 | 2 |"];
        let table = super::GfmTable::parse(&lines).unwrap();
        let rendered = table.render();

        assert_eq!(rendered.len(), 3); // header + separator + 1 data row
        assert!(rendered[0].contains("A"));
        assert!(rendered[0].contains("B"));
        assert!(rendered[1].contains("-"));
        assert!(rendered[2].contains("1"));
        assert!(rendered[2].contains("2"));
    }

    #[test]
    fn test_gfm_table_invalid_separator() {
        // Missing separator row
        let lines = vec!["| Name | Age |", "| Alice | 30 |"];
        let table = super::GfmTable::parse(&lines);
        assert!(table.is_none());
    }

    #[test]
    fn test_gfm_table_in_markdown() {
        let markdown = r#"# Table Test

Here is a table:

| Item | Price |
|------|-------|
| Apple | $1 |
| Orange | $2 |

End of table.
"#;
        let result = Pdf::from_markdown(markdown);
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_table_line() {
        assert!(super::is_table_line("| A | B |"));
        assert!(super::is_table_line("|---|---|"));
        assert!(super::is_table_line("| Cell |"));
        assert!(!super::is_table_line("Not a table"));
        assert!(!super::is_table_line("| Only one pipe"));
        assert!(!super::is_table_line(""));
    }
}
