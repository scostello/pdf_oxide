//! Office document to PDF conversion.
//!
//! This module provides functionality to convert Microsoft Office documents
//! (DOCX, XLSX, PPTX) to PDF format using native Rust parsing.
//!
//! # Overview
//!
//! Office documents are XML-based archives (Open XML / OOXML format) that
//! can be parsed directly. This module converts them to PDF by:
//! - Extracting text, formatting, and structure from Office files
//! - Rendering content using the pdf_oxide DocumentBuilder
//!
//! # Supported Formats
//!
//! - **DOCX**: Word documents (paragraphs, tables, images, lists)
//! - **XLSX**: Excel spreadsheets (cells, sheets, basic formatting)
//! - **PPTX**: PowerPoint presentations (slides, text boxes, shapes)
//!
//! # Example
//!
//! ```ignore
//! use pdf_oxide::converters::office::OfficeConverter;
//!
//! // Convert a DOCX file to PDF
//! let converter = OfficeConverter::new();
//! let pdf_bytes = converter.convert_docx("document.docx")?;
//! std::fs::write("output.pdf", pdf_bytes)?;
//! ```
//!
//! # Feature Flag
//!
//! This module requires the `office` feature to be enabled:
//!
//! ```toml
//! [dependencies]
//! pdf_oxide = { version = "0.3", features = ["office"] }
//! ```

#[cfg(feature = "office")]
mod docx;
#[cfg(feature = "office")]
mod pptx;
#[cfg(feature = "office")]
mod styles;
#[cfg(feature = "office")]
mod xlsx;

#[cfg(feature = "office")]
pub use docx::DocxConverter;
#[cfg(feature = "office")]
pub use pptx::PptxConverter;
#[cfg(feature = "office")]
pub use xlsx::XlsxConverter;

use crate::error::{Error, Result};
use crate::writer::{DocumentBuilder, DocumentMetadata, PageSize};
use std::path::Path;

/// Page margins in points (1 inch = 72 points).
#[derive(Debug, Clone, Copy)]
pub struct Margins {
    /// Top margin in points
    pub top: f32,
    /// Bottom margin in points
    pub bottom: f32,
    /// Left margin in points
    pub left: f32,
    /// Right margin in points
    pub right: f32,
}

impl Default for Margins {
    fn default() -> Self {
        Self {
            top: 72.0,    // 1 inch
            bottom: 72.0, // 1 inch
            left: 72.0,   // 1 inch
            right: 72.0,  // 1 inch
        }
    }
}

impl Margins {
    /// Create margins with equal values on all sides.
    pub fn uniform(margin: f32) -> Self {
        Self {
            top: margin,
            bottom: margin,
            left: margin,
            right: margin,
        }
    }

    /// Create margins with no spacing.
    pub fn none() -> Self {
        Self::uniform(0.0)
    }
}

/// Configuration for Office to PDF conversion.
#[derive(Debug, Clone)]
pub struct OfficeConfig {
    /// Page size for output PDF
    pub page_size: PageSize,
    /// Margins in points
    pub margins: Margins,
    /// Whether to embed fonts (currently uses standard PDF fonts)
    pub embed_fonts: bool,
    /// Default font for text
    pub default_font: String,
    /// Default font size in points
    pub default_font_size: f32,
    /// Line height multiplier
    pub line_height: f32,
    /// Whether to include images
    pub include_images: bool,
}

impl Default for OfficeConfig {
    fn default() -> Self {
        Self {
            page_size: PageSize::Letter,
            margins: Margins::default(),
            embed_fonts: false,
            default_font: "Helvetica".to_string(),
            default_font_size: 11.0,
            line_height: 1.2,
            include_images: true,
        }
    }
}

impl OfficeConfig {
    /// Create config with A4 page size.
    pub fn a4() -> Self {
        Self {
            page_size: PageSize::A4,
            ..Default::default()
        }
    }

    /// Create config with Letter page size.
    pub fn letter() -> Self {
        Self::default()
    }
}

/// Main converter for Office documents to PDF.
///
/// Supports DOCX, XLSX, and PPTX formats through native Rust parsing.
#[derive(Debug, Clone, Default)]
pub struct OfficeConverter {
    config: OfficeConfig,
}

impl OfficeConverter {
    /// Create a new converter with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a converter with custom configuration.
    pub fn with_config(config: OfficeConfig) -> Self {
        Self { config }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &OfficeConfig {
        &self.config
    }

    /// Convert a DOCX file to PDF bytes.
    #[cfg(feature = "office")]
    pub fn convert_docx(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let bytes = std::fs::read(path.as_ref())?;
        self.convert_docx_bytes(&bytes)
    }

    /// Convert DOCX bytes to PDF bytes.
    #[cfg(feature = "office")]
    pub fn convert_docx_bytes(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let converter = DocxConverter::new(self.config.clone());
        converter.convert(bytes)
    }

    /// Convert an XLSX file to PDF bytes.
    #[cfg(feature = "office")]
    pub fn convert_xlsx(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let bytes = std::fs::read(path.as_ref())?;
        self.convert_xlsx_bytes(&bytes)
    }

    /// Convert XLSX bytes to PDF bytes.
    #[cfg(feature = "office")]
    pub fn convert_xlsx_bytes(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let converter = XlsxConverter::new(self.config.clone());
        converter.convert(bytes)
    }

    /// Convert a PPTX file to PDF bytes.
    #[cfg(feature = "office")]
    pub fn convert_pptx(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let bytes = std::fs::read(path.as_ref())?;
        self.convert_pptx_bytes(&bytes)
    }

    /// Convert PPTX bytes to PDF bytes.
    #[cfg(feature = "office")]
    pub fn convert_pptx_bytes(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let converter = PptxConverter::new(self.config.clone());
        converter.convert(bytes)
    }

    /// Auto-detect format and convert to PDF.
    pub fn convert(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        match extension.as_str() {
            #[cfg(feature = "office")]
            "docx" => self.convert_docx(path),
            #[cfg(feature = "office")]
            "xlsx" | "xls" => self.convert_xlsx(path),
            #[cfg(feature = "office")]
            "pptx" => self.convert_pptx(path),
            _ => Err(Error::InvalidPdf(format!("Unsupported file format: {}", extension))),
        }
    }

    /// Stub for non-office feature builds
    #[cfg(not(feature = "office"))]
    pub fn convert_docx(&self, _path: impl AsRef<Path>) -> Result<Vec<u8>> {
        Err(Error::InvalidPdf("Office conversion requires the 'office' feature".to_string()))
    }

    /// Stub for non-office feature builds
    #[cfg(not(feature = "office"))]
    pub fn convert_docx_bytes(&self, _bytes: &[u8]) -> Result<Vec<u8>> {
        Err(Error::InvalidPdf("Office conversion requires the 'office' feature".to_string()))
    }

    /// Stub for non-office feature builds
    #[cfg(not(feature = "office"))]
    pub fn convert_xlsx(&self, _path: impl AsRef<Path>) -> Result<Vec<u8>> {
        Err(Error::InvalidPdf("Office conversion requires the 'office' feature".to_string()))
    }

    /// Stub for non-office feature builds
    #[cfg(not(feature = "office"))]
    pub fn convert_xlsx_bytes(&self, _bytes: &[u8]) -> Result<Vec<u8>> {
        Err(Error::InvalidPdf("Office conversion requires the 'office' feature".to_string()))
    }

    /// Stub for non-office feature builds
    #[cfg(not(feature = "office"))]
    pub fn convert_pptx(&self, _path: impl AsRef<Path>) -> Result<Vec<u8>> {
        Err(Error::InvalidPdf("Office conversion requires the 'office' feature".to_string()))
    }

    /// Stub for non-office feature builds
    #[cfg(not(feature = "office"))]
    pub fn convert_pptx_bytes(&self, _bytes: &[u8]) -> Result<Vec<u8>> {
        Err(Error::InvalidPdf("Office conversion requires the 'office' feature".to_string()))
    }
}

/// Helper to create a basic PDF from text content.
#[allow(dead_code)]
pub(crate) fn create_simple_pdf(
    title: &str,
    content: &[String],
    config: &OfficeConfig,
) -> Result<Vec<u8>> {
    let metadata = DocumentMetadata::new().title(title).creator("pdf_oxide");

    let mut builder = DocumentBuilder::new().metadata(metadata);

    let (page_width, page_height) = config.page_size.dimensions();
    let content_width = page_width - config.margins.left - config.margins.right;
    let line_height = config.default_font_size * config.line_height;

    // Process content into lines with page breaks
    let mut all_lines: Vec<(String, bool)> = Vec::new(); // (line, is_new_page)
    let mut current_y = page_height - config.margins.top;

    for line in content {
        if line.trim().is_empty() {
            current_y -= line_height;
            all_lines.push((String::new(), false));
            continue;
        }

        // Simple word wrap
        let words: Vec<&str> = line.split_whitespace().collect();
        let mut current_line = String::new();
        let avg_char_width = config.default_font_size * 0.5;

        for word in words {
            let word_with_space = if current_line.is_empty() {
                word.to_string()
            } else {
                format!(" {}", word)
            };

            let new_width = (current_line.len() + word_with_space.len()) as f32 * avg_char_width;

            if new_width > content_width && !current_line.is_empty() {
                // Check for page break
                let is_new_page = current_y < config.margins.bottom + line_height;
                if is_new_page {
                    current_y = page_height - config.margins.top;
                }
                all_lines.push((current_line, is_new_page));
                current_y -= line_height;
                current_line = word.to_string();
            } else {
                current_line.push_str(&word_with_space);
            }
        }

        if !current_line.is_empty() {
            let is_new_page = current_y < config.margins.bottom + line_height;
            if is_new_page {
                current_y = page_height - config.margins.top;
            }
            all_lines.push((current_line, is_new_page));
            current_y -= line_height;
        }
    }

    // Now render all lines
    current_y = page_height - config.margins.top;
    let mut page_builder = builder.page(config.page_size);
    page_builder = page_builder
        .at(config.margins.left, current_y)
        .font(&config.default_font, config.default_font_size);

    for (line, is_new_page) in &all_lines {
        if *is_new_page {
            page_builder.done();
            current_y = page_height - config.margins.top;
            page_builder = builder.page(config.page_size);
            page_builder = page_builder.font(&config.default_font, config.default_font_size);
        }

        if !line.is_empty() {
            page_builder = page_builder.at(config.margins.left, current_y).text(line);
        }
        current_y -= line_height;
    }

    page_builder.done();
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_margins_default() {
        let margins = Margins::default();
        assert_eq!(margins.top, 72.0);
        assert_eq!(margins.bottom, 72.0);
        assert_eq!(margins.left, 72.0);
        assert_eq!(margins.right, 72.0);
    }

    #[test]
    fn test_margins_uniform() {
        let margins = Margins::uniform(36.0);
        assert_eq!(margins.top, 36.0);
        assert_eq!(margins.bottom, 36.0);
    }

    #[test]
    fn test_config_default() {
        let config = OfficeConfig::default();
        assert_eq!(config.default_font, "Helvetica");
        assert_eq!(config.default_font_size, 11.0);
    }

    #[test]
    fn test_converter_new() {
        let converter = OfficeConverter::new();
        assert_eq!(converter.config().default_font, "Helvetica");
    }

    #[test]
    fn test_create_simple_pdf() {
        let config = OfficeConfig::default();
        let content = vec!["Hello, World!".to_string(), "This is a test.".to_string()];

        let result = create_simple_pdf("Test", &content, &config);
        assert!(result.is_ok());

        let bytes = result.unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);
        assert!(pdf_str.starts_with("%PDF-"));
    }
}
