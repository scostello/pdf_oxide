//! Python bindings via PyO3.
//!
//! This module provides Python bindings for the PDF library, exposing the core functionality
//! through a Python-friendly API with proper error handling and type hints.
//!
//! # Architecture
//!
//! - `PyPdfDocument`: Python wrapper around Rust `PdfDocument`
//! - Error mapping: Rust errors → Python exceptions
//! - Default arguments using `#[pyo3(signature = ...)]`
//!
//! # Example
//!
//! ```python
//! from pdf_oxide import PdfDocument
//!
//! doc = PdfDocument("document.pdf")
//! text = doc.extract_text(0)
//! markdown = doc.to_markdown(0, detect_headings=True)
//! ```

use pyo3::exceptions::{PyIOError, PyRuntimeError};
use pyo3::prelude::*;

use crate::converters::ConversionOptions as RustConversionOptions;
use crate::document::PdfDocument as RustPdfDocument;

/// Python wrapper for PdfDocument.
///
/// Provides PDF parsing, text extraction, and format conversion capabilities.
///
/// # Methods
///
/// - `__init__(path)`: Open a PDF file
/// - `version()`: Get PDF version tuple
/// - `page_count()`: Get number of pages
/// - `extract_text(page)`: Extract text from a page
/// - `to_markdown(page, ...)`: Convert page to Markdown
/// - `to_html(page, ...)`: Convert page to HTML
/// - `to_markdown_all(...)`: Convert all pages to Markdown
/// - `to_html_all(...)`: Convert all pages to HTML
#[pyclass(name = "PdfDocument", unsendable)]
pub struct PyPdfDocument {
    /// Inner Rust document
    inner: RustPdfDocument,
}

#[pymethods]
impl PyPdfDocument {
    /// Open a PDF file.
    ///
    /// Args:
    ///     path (str): Path to the PDF file
    ///
    /// Returns:
    ///     PdfDocument: Opened PDF document
    ///
    /// Raises:
    ///     IOError: If the file cannot be opened or is not a valid PDF
    ///
    /// Example:
    ///     >>> doc = PdfDocument("sample.pdf")
    ///     >>> print(doc.version())
    ///     (1, 7)
    #[new]
    fn new(path: String) -> PyResult<Self> {
        let doc = RustPdfDocument::open(&path)
            .map_err(|e| PyIOError::new_err(format!("Failed to open PDF: {}", e)))?;

        Ok(PyPdfDocument { inner: doc })
    }

    /// Get PDF version.
    ///
    /// Returns:
    ///     tuple[int, int]: PDF version as (major, minor), e.g. (1, 7) for PDF 1.7
    ///
    /// Example:
    ///     >>> doc = PdfDocument("sample.pdf")
    ///     >>> version = doc.version()
    ///     >>> print(f"PDF {version[0]}.{version[1]}")
    ///     PDF 1.7
    fn version(&self) -> (u8, u8) {
        self.inner.version()
    }

    /// Get number of pages in the document.
    ///
    /// Returns:
    ///     int: Number of pages
    ///
    /// Raises:
    ///     RuntimeError: If page count cannot be determined
    ///
    /// Example:
    ///     >>> doc = PdfDocument("sample.pdf")
    ///     >>> print(f"Pages: {doc.page_count()}")
    ///     Pages: 42
    fn page_count(&mut self) -> PyResult<usize> {
        self.inner
            .page_count()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to get page count: {}", e)))
    }

    /// Extract text from a page.
    ///
    /// Args:
    ///     page (int): Page index (0-based)
    ///
    /// Returns:
    ///     str: Extracted text from the page
    ///
    /// Raises:
    ///     RuntimeError: If text extraction fails
    ///
    /// Example:
    ///     >>> doc = PdfDocument("sample.pdf")
    ///     >>> text = doc.extract_text(0)
    ///     >>> print(text[:100])
    fn extract_text(&mut self, page: usize) -> PyResult<String> {
        self.inner
            .extract_text(page)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to extract text: {}", e)))
    }

    /// Check if document has a structure tree (Tagged PDF).
    ///
    /// Tagged PDFs contain explicit document structure that defines reading order,
    /// semantic meaning, and accessibility information. This is the PDF-spec-compliant
    /// way to determine reading order.
    ///
    /// Returns:
    ///     bool: True if document has logical structure (Tagged PDF), False otherwise
    ///
    /// Example:
    ///     >>> doc = PdfDocument("sample.pdf")
    ///     >>> if doc.has_structure_tree():
    ///     ...     print("Tagged PDF with logical structure")
    ///     ... else:
    ///     ...     print("Untagged PDF - uses page content order")
    fn has_structure_tree(&mut self) -> bool {
        match self.inner.structure_tree() {
            Ok(Some(_)) => true,
            _ => false,
        }
    }

    /// Convert page to plain text.
    ///
    /// Args:
    ///     page (int): Page index (0-based)
    ///     preserve_layout (bool): Preserve visual layout (default: False) [currently unused]
    ///     detect_headings (bool): Detect headings (default: True) [currently unused]
    ///     include_images (bool): Include images (default: True) [currently unused]
    ///     image_output_dir (str | None): Directory for images (default: None) [currently unused]
    ///
    /// Returns:
    ///     str: Plain text from the page
    ///
    /// Raises:
    ///     RuntimeError: If conversion fails
    ///
    /// Example:
    ///     >>> doc = PdfDocument("paper.pdf")
    ///     >>> text = doc.to_plain_text(0)
    ///     >>> print(text[:100])
    ///
    /// Note:
    ///     Options parameters are accepted for API consistency but currently unused for plain text.
    #[pyo3(signature = (page, preserve_layout=false, detect_headings=true, include_images=true, image_output_dir=None))]
    fn to_plain_text(
        &mut self,
        page: usize,
        preserve_layout: bool,
        detect_headings: bool,
        include_images: bool,
        image_output_dir: Option<String>,
    ) -> PyResult<String> {
        let options = RustConversionOptions {
            preserve_layout,
            detect_headings,
            extract_tables: false,
            include_images,
            image_output_dir,
            ..Default::default()
        };

        self.inner
            .to_plain_text(page, &options)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert to plain text: {}", e)))
    }

    /// Convert all pages to plain text.
    ///
    /// Args:
    ///     preserve_layout (bool): Preserve visual layout (default: False) [currently unused]
    ///     detect_headings (bool): Detect headings (default: True) [currently unused]
    ///     include_images (bool): Include images (default: True) [currently unused]
    ///     image_output_dir (str | None): Directory for images (default: None) [currently unused]
    ///
    /// Returns:
    ///     str: Plain text from all pages separated by horizontal rules
    ///
    /// Raises:
    ///     RuntimeError: If conversion fails
    ///
    /// Example:
    ///     >>> doc = PdfDocument("book.pdf")
    ///     >>> text = doc.to_plain_text_all()
    ///     >>> with open("book.txt", "w") as f:
    ///     ...     f.write(text)
    ///
    /// Note:
    ///     Options parameters are accepted for API consistency but currently unused for plain text.
    #[pyo3(signature = (preserve_layout=false, detect_headings=true, include_images=true, image_output_dir=None))]
    fn to_plain_text_all(
        &mut self,
        preserve_layout: bool,
        detect_headings: bool,
        include_images: bool,
        image_output_dir: Option<String>,
    ) -> PyResult<String> {
        let options = RustConversionOptions {
            preserve_layout,
            detect_headings,
            extract_tables: false,
            include_images,
            image_output_dir,
            ..Default::default()
        };

        self.inner.to_plain_text_all(&options).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to convert all pages to plain text: {}", e))
        })
    }

    /// Convert page to Markdown.
    ///
    /// Args:
    ///     page (int): Page index (0-based)
    ///     preserve_layout (bool): Preserve visual layout (default: False)
    ///     detect_headings (bool): Detect headings based on font size (default: True)
    ///     include_images (bool): Include images in output (default: True)
    ///     image_output_dir (str | None): Directory to save images (default: None)
    ///
    /// Returns:
    ///     str: Markdown text
    ///
    /// Raises:
    ///     RuntimeError: If conversion fails
    ///
    /// Example:
    ///     >>> doc = PdfDocument("paper.pdf")
    ///     >>> markdown = doc.to_markdown(0, detect_headings=True)
    ///     >>> with open("output.md", "w") as f:
    ///     ...     f.write(markdown)
    #[pyo3(signature = (page, preserve_layout=false, detect_headings=true, include_images=true, image_output_dir=None, embed_images=true))]
    fn to_markdown(
        &mut self,
        page: usize,
        preserve_layout: bool,
        detect_headings: bool,
        include_images: bool,
        image_output_dir: Option<String>,
        embed_images: bool,
    ) -> PyResult<String> {
        let options = RustConversionOptions {
            preserve_layout,
            detect_headings,
            extract_tables: false,
            include_images,
            image_output_dir,
            embed_images,
            ..Default::default()
        };

        self.inner
            .to_markdown(page, &options)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert to Markdown: {}", e)))
    }

    /// Convert page to HTML.
    ///
    /// Args:
    ///     page (int): Page index (0-based)
    ///     preserve_layout (bool): Preserve visual layout with CSS positioning (default: False)
    ///     detect_headings (bool): Detect headings based on font size (default: True)
    ///     include_images (bool): Include images in output (default: True)
    ///     image_output_dir (str | None): Directory to save images (default: None)
    ///
    /// Returns:
    ///     str: HTML text
    ///
    /// Raises:
    ///     RuntimeError: If conversion fails
    ///
    /// Example:
    ///     >>> doc = PdfDocument("paper.pdf")
    ///     >>> html = doc.to_html(0, preserve_layout=False)
    ///     >>> with open("output.html", "w") as f:
    ///     ...     f.write(html)
    #[pyo3(signature = (page, preserve_layout=false, detect_headings=true, include_images=true, image_output_dir=None, embed_images=true))]
    fn to_html(
        &mut self,
        page: usize,
        preserve_layout: bool,
        detect_headings: bool,
        include_images: bool,
        image_output_dir: Option<String>,
        embed_images: bool,
    ) -> PyResult<String> {
        let options = RustConversionOptions {
            preserve_layout,
            detect_headings,
            extract_tables: false,
            include_images,
            image_output_dir,
            embed_images,
            ..Default::default()
        };

        self.inner
            .to_html(page, &options)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert to HTML: {}", e)))
    }

    /// Convert all pages to Markdown.
    ///
    /// Args:
    ///     preserve_layout (bool): Preserve visual layout (default: False)
    ///     detect_headings (bool): Detect headings based on font size (default: True)
    ///     include_images (bool): Include images in output (default: True)
    ///     image_output_dir (str | None): Directory to save images (default: None)
    ///
    /// Returns:
    ///     str: Markdown text with all pages separated by horizontal rules
    ///
    /// Raises:
    ///     RuntimeError: If conversion fails
    ///
    /// Example:
    ///     >>> doc = PdfDocument("book.pdf")
    ///     >>> markdown = doc.to_markdown_all(detect_headings=True)
    ///     >>> with open("book.md", "w") as f:
    ///     ...     f.write(markdown)
    #[pyo3(signature = (preserve_layout=false, detect_headings=true, include_images=true, image_output_dir=None, embed_images=true))]
    fn to_markdown_all(
        &mut self,
        preserve_layout: bool,
        detect_headings: bool,
        include_images: bool,
        image_output_dir: Option<String>,
        embed_images: bool,
    ) -> PyResult<String> {
        let options = RustConversionOptions {
            preserve_layout,
            detect_headings,
            extract_tables: false,
            include_images,
            image_output_dir,
            embed_images,
            ..Default::default()
        };

        self.inner.to_markdown_all(&options).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to convert all pages to Markdown: {}", e))
        })
    }

    /// Convert all pages to HTML.
    ///
    /// Args:
    ///     preserve_layout (bool): Preserve visual layout with CSS positioning (default: False)
    ///     detect_headings (bool): Detect headings based on font size (default: True)
    ///     include_images (bool): Include images in output (default: True)
    ///     image_output_dir (str | None): Directory to save images (default: None)
    ///
    /// Returns:
    ///     str: HTML text with all pages wrapped in div.page elements
    ///
    /// Raises:
    ///     RuntimeError: If conversion fails
    ///
    /// Example:
    ///     >>> doc = PdfDocument("book.pdf")
    ///     >>> html = doc.to_html_all(preserve_layout=True)
    ///     >>> with open("book.html", "w") as f:
    ///     ...     f.write(html)
    #[pyo3(signature = (preserve_layout=false, detect_headings=true, include_images=true, image_output_dir=None, embed_images=true))]
    fn to_html_all(
        &mut self,
        preserve_layout: bool,
        detect_headings: bool,
        include_images: bool,
        image_output_dir: Option<String>,
        embed_images: bool,
    ) -> PyResult<String> {
        let options = RustConversionOptions {
            preserve_layout,
            detect_headings,
            extract_tables: false,
            include_images,
            image_output_dir,
            embed_images,
            ..Default::default()
        };

        self.inner.to_html_all(&options).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to convert all pages to HTML: {}", e))
        })
    }

    /// String representation of the document.
    ///
    /// Returns:
    ///     str: Representation showing PDF version
    fn __repr__(&self) -> String {
        format!("PdfDocument(version={}.{})", self.inner.version().0, self.inner.version().1)
    }
}

// === PDF Creation API ===

use crate::api::PdfBuilder as RustPdfBuilder;

/// Python wrapper for PDF creation.
///
/// Provides simple PDF creation from Markdown, HTML, or plain text.
///
/// # Methods
///
/// - `from_markdown(content)`: Create PDF from Markdown
/// - `from_html(content)`: Create PDF from HTML
/// - `from_text(content)`: Create PDF from plain text
/// - `save(path)`: Save PDF to file
///
/// Example:
///     >>> pdf = Pdf.from_markdown("# Hello World")
///     >>> pdf.save("output.pdf")
#[pyclass(name = "Pdf")]
pub struct PyPdf {
    bytes: Vec<u8>,
}

#[pymethods]
impl PyPdf {
    /// Create a PDF from Markdown content.
    ///
    /// Args:
    ///     content (str): Markdown content
    ///     title (str, optional): Document title
    ///     author (str, optional): Document author
    ///
    /// Returns:
    ///     Pdf: Created PDF document
    ///
    /// Raises:
    ///     RuntimeError: If PDF creation fails
    ///
    /// Example:
    ///     >>> pdf = Pdf.from_markdown("# Hello\\n\\nWorld")
    ///     >>> pdf.save("hello.pdf")
    #[staticmethod]
    #[pyo3(signature = (content, title=None, author=None))]
    fn from_markdown(content: &str, title: Option<&str>, author: Option<&str>) -> PyResult<Self> {
        let mut builder = RustPdfBuilder::new();
        if let Some(t) = title {
            builder = builder.title(t);
        }
        if let Some(a) = author {
            builder = builder.author(a);
        }

        let pdf = builder
            .from_markdown(content)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create PDF: {}", e)))?;

        Ok(PyPdf {
            bytes: pdf.into_bytes(),
        })
    }

    /// Create a PDF from HTML content.
    ///
    /// Args:
    ///     content (str): HTML content
    ///     title (str, optional): Document title
    ///     author (str, optional): Document author
    ///
    /// Returns:
    ///     Pdf: Created PDF document
    ///
    /// Example:
    ///     >>> pdf = Pdf.from_html("<h1>Hello</h1><p>World</p>")
    ///     >>> pdf.save("hello.pdf")
    #[staticmethod]
    #[pyo3(signature = (content, title=None, author=None))]
    fn from_html(content: &str, title: Option<&str>, author: Option<&str>) -> PyResult<Self> {
        let mut builder = RustPdfBuilder::new();
        if let Some(t) = title {
            builder = builder.title(t);
        }
        if let Some(a) = author {
            builder = builder.author(a);
        }

        let pdf = builder
            .from_html(content)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create PDF: {}", e)))?;

        Ok(PyPdf {
            bytes: pdf.into_bytes(),
        })
    }

    /// Create a PDF from plain text.
    ///
    /// Args:
    ///     content (str): Plain text content
    ///     title (str, optional): Document title
    ///     author (str, optional): Document author
    ///
    /// Returns:
    ///     Pdf: Created PDF document
    ///
    /// Example:
    ///     >>> pdf = Pdf.from_text("Hello, World!")
    ///     >>> pdf.save("hello.pdf")
    #[staticmethod]
    #[pyo3(signature = (content, title=None, author=None))]
    fn from_text(content: &str, title: Option<&str>, author: Option<&str>) -> PyResult<Self> {
        let mut builder = RustPdfBuilder::new();
        if let Some(t) = title {
            builder = builder.title(t);
        }
        if let Some(a) = author {
            builder = builder.author(a);
        }

        let pdf = builder
            .from_text(content)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create PDF: {}", e)))?;

        Ok(PyPdf {
            bytes: pdf.into_bytes(),
        })
    }

    /// Save the PDF to a file.
    ///
    /// Args:
    ///     path (str): Output file path
    ///
    /// Raises:
    ///     IOError: If the file cannot be written
    ///
    /// Example:
    ///     >>> pdf = Pdf.from_markdown("# Hello")
    ///     >>> pdf.save("output.pdf")
    fn save(&self, path: &str) -> PyResult<()> {
        std::fs::write(path, &self.bytes)
            .map_err(|e| PyIOError::new_err(format!("Failed to save PDF: {}", e)))
    }

    /// Get the PDF as bytes.
    ///
    /// Returns:
    ///     bytes: Raw PDF data
    ///
    /// Example:
    ///     >>> pdf = Pdf.from_markdown("# Hello")
    ///     >>> data = pdf.to_bytes()
    ///     >>> len(data) > 0
    ///     True
    fn to_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Get the size of the PDF in bytes.
    ///
    /// Returns:
    ///     int: Size in bytes
    fn __len__(&self) -> usize {
        self.bytes.len()
    }

    /// String representation.
    fn __repr__(&self) -> String {
        format!("Pdf({} bytes)", self.bytes.len())
    }
}

// === Advanced Graphics Types ===

use crate::layout::Color as RustColor;
use crate::writer::{
    BlendMode as RustBlendMode, LineCap as RustLineCap, LineJoin as RustLineJoin,
    PatternPresets as RustPatternPresets,
};

/// RGB Color for PDF graphics.
///
/// Example:
///     >>> color = Color(1.0, 0.0, 0.0)  # Red
///     >>> color = Color.red()
///     >>> color = Color.from_hex("#FF0000")
#[pyclass(name = "Color")]
#[derive(Clone)]
pub struct PyColor {
    inner: RustColor,
}

#[pymethods]
impl PyColor {
    /// Create a new RGB color.
    ///
    /// Args:
    ///     r (float): Red component (0.0 to 1.0)
    ///     g (float): Green component (0.0 to 1.0)
    ///     b (float): Blue component (0.0 to 1.0)
    #[new]
    fn new(r: f32, g: f32, b: f32) -> Self {
        PyColor {
            inner: RustColor::new(r, g, b),
        }
    }

    /// Create color from hex string.
    ///
    /// Args:
    ///     hex_str (str): Hex color like "#FF0000" or "FF0000"
    ///
    /// Example:
    ///     >>> red = Color.from_hex("#FF0000")
    #[staticmethod]
    fn from_hex(hex_str: &str) -> PyResult<Self> {
        let hex = hex_str.trim_start_matches('#');
        if hex.len() != 6 {
            return Err(PyRuntimeError::new_err("Invalid hex color format"));
        }
        let r = u8::from_str_radix(&hex[0..2], 16)
            .map_err(|_| PyRuntimeError::new_err("Invalid hex color"))?;
        let g = u8::from_str_radix(&hex[2..4], 16)
            .map_err(|_| PyRuntimeError::new_err("Invalid hex color"))?;
        let b = u8::from_str_radix(&hex[4..6], 16)
            .map_err(|_| PyRuntimeError::new_err("Invalid hex color"))?;
        Ok(PyColor {
            inner: RustColor::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0),
        })
    }

    /// Black color.
    #[staticmethod]
    fn black() -> Self {
        PyColor {
            inner: RustColor::black(),
        }
    }

    /// White color.
    #[staticmethod]
    fn white() -> Self {
        PyColor {
            inner: RustColor::white(),
        }
    }

    /// Red color.
    #[staticmethod]
    fn red() -> Self {
        PyColor {
            inner: RustColor::new(1.0, 0.0, 0.0),
        }
    }

    /// Green color.
    #[staticmethod]
    fn green() -> Self {
        PyColor {
            inner: RustColor::new(0.0, 1.0, 0.0),
        }
    }

    /// Blue color.
    #[staticmethod]
    fn blue() -> Self {
        PyColor {
            inner: RustColor::new(0.0, 0.0, 1.0),
        }
    }

    /// Get red component.
    #[getter]
    fn r(&self) -> f32 {
        self.inner.r
    }

    /// Get green component.
    #[getter]
    fn g(&self) -> f32 {
        self.inner.g
    }

    /// Get blue component.
    #[getter]
    fn b(&self) -> f32 {
        self.inner.b
    }

    fn __repr__(&self) -> String {
        format!("Color({}, {}, {})", self.inner.r, self.inner.g, self.inner.b)
    }
}

/// Blend modes for transparency effects.
///
/// Example:
///     >>> gs = ExtGState().blend_mode(BlendMode.MULTIPLY)
#[pyclass(name = "BlendMode")]
#[derive(Clone)]
pub struct PyBlendMode {
    inner: RustBlendMode,
}

#[pymethods]
impl PyBlendMode {
    /// Normal blend mode (default).
    #[staticmethod]
    #[allow(non_snake_case)]
    fn NORMAL() -> Self {
        PyBlendMode {
            inner: RustBlendMode::Normal,
        }
    }

    /// Multiply blend mode.
    #[staticmethod]
    #[allow(non_snake_case)]
    fn MULTIPLY() -> Self {
        PyBlendMode {
            inner: RustBlendMode::Multiply,
        }
    }

    /// Screen blend mode.
    #[staticmethod]
    #[allow(non_snake_case)]
    fn SCREEN() -> Self {
        PyBlendMode {
            inner: RustBlendMode::Screen,
        }
    }

    /// Overlay blend mode.
    #[staticmethod]
    #[allow(non_snake_case)]
    fn OVERLAY() -> Self {
        PyBlendMode {
            inner: RustBlendMode::Overlay,
        }
    }

    /// Darken blend mode.
    #[staticmethod]
    #[allow(non_snake_case)]
    fn DARKEN() -> Self {
        PyBlendMode {
            inner: RustBlendMode::Darken,
        }
    }

    /// Lighten blend mode.
    #[staticmethod]
    #[allow(non_snake_case)]
    fn LIGHTEN() -> Self {
        PyBlendMode {
            inner: RustBlendMode::Lighten,
        }
    }

    /// Color dodge blend mode.
    #[staticmethod]
    #[allow(non_snake_case)]
    fn COLOR_DODGE() -> Self {
        PyBlendMode {
            inner: RustBlendMode::ColorDodge,
        }
    }

    /// Color burn blend mode.
    #[staticmethod]
    #[allow(non_snake_case)]
    fn COLOR_BURN() -> Self {
        PyBlendMode {
            inner: RustBlendMode::ColorBurn,
        }
    }

    /// Hard light blend mode.
    #[staticmethod]
    #[allow(non_snake_case)]
    fn HARD_LIGHT() -> Self {
        PyBlendMode {
            inner: RustBlendMode::HardLight,
        }
    }

    /// Soft light blend mode.
    #[staticmethod]
    #[allow(non_snake_case)]
    fn SOFT_LIGHT() -> Self {
        PyBlendMode {
            inner: RustBlendMode::SoftLight,
        }
    }

    /// Difference blend mode.
    #[staticmethod]
    #[allow(non_snake_case)]
    fn DIFFERENCE() -> Self {
        PyBlendMode {
            inner: RustBlendMode::Difference,
        }
    }

    /// Exclusion blend mode.
    #[staticmethod]
    #[allow(non_snake_case)]
    fn EXCLUSION() -> Self {
        PyBlendMode {
            inner: RustBlendMode::Exclusion,
        }
    }

    fn __repr__(&self) -> String {
        format!("BlendMode.{}", self.inner.as_pdf_name())
    }
}

/// Extended Graphics State for transparency and blend effects.
///
/// Example:
///     >>> gs = ExtGState().alpha(0.5).blend_mode(BlendMode.MULTIPLY)
#[pyclass(name = "ExtGState")]
#[derive(Clone)]
pub struct PyExtGState {
    fill_alpha: Option<f32>,
    stroke_alpha: Option<f32>,
    blend_mode: Option<RustBlendMode>,
}

#[pymethods]
impl PyExtGState {
    /// Create a new ExtGState builder.
    #[new]
    fn new() -> Self {
        PyExtGState {
            fill_alpha: None,
            stroke_alpha: None,
            blend_mode: None,
        }
    }

    /// Set fill opacity (0.0 = transparent, 1.0 = opaque).
    fn fill_alpha(&self, alpha: f32) -> Self {
        PyExtGState {
            fill_alpha: Some(alpha.clamp(0.0, 1.0)),
            stroke_alpha: self.stroke_alpha,
            blend_mode: self.blend_mode,
        }
    }

    /// Set stroke opacity (0.0 = transparent, 1.0 = opaque).
    fn stroke_alpha(&self, alpha: f32) -> Self {
        PyExtGState {
            fill_alpha: self.fill_alpha,
            stroke_alpha: Some(alpha.clamp(0.0, 1.0)),
            blend_mode: self.blend_mode,
        }
    }

    /// Set both fill and stroke opacity.
    fn alpha(&self, alpha: f32) -> Self {
        let a = alpha.clamp(0.0, 1.0);
        PyExtGState {
            fill_alpha: Some(a),
            stroke_alpha: Some(a),
            blend_mode: self.blend_mode,
        }
    }

    /// Set blend mode.
    fn blend_mode(&self, mode: &PyBlendMode) -> Self {
        PyExtGState {
            fill_alpha: self.fill_alpha,
            stroke_alpha: self.stroke_alpha,
            blend_mode: Some(mode.inner),
        }
    }

    /// Create semi-transparent state (50% opacity).
    #[staticmethod]
    fn semi_transparent() -> Self {
        PyExtGState {
            fill_alpha: Some(0.5),
            stroke_alpha: Some(0.5),
            blend_mode: None,
        }
    }

    fn __repr__(&self) -> String {
        let mut parts = Vec::new();
        if let Some(a) = self.fill_alpha {
            parts.push(format!("fill_alpha={}", a));
        }
        if let Some(a) = self.stroke_alpha {
            parts.push(format!("stroke_alpha={}", a));
        }
        if let Some(ref m) = self.blend_mode {
            parts.push(format!("blend_mode={}", m.as_pdf_name()));
        }
        format!("ExtGState({})", parts.join(", "))
    }
}

/// Linear gradient builder.
///
/// Example:
///     >>> gradient = LinearGradient() \
///     ...     .start(0, 0).end(100, 100) \
///     ...     .add_stop(0.0, Color.red()) \
///     ...     .add_stop(1.0, Color.blue())
#[pyclass(name = "LinearGradient")]
#[derive(Clone)]
pub struct PyLinearGradient {
    start: (f32, f32),
    end: (f32, f32),
    stops: Vec<(f32, RustColor)>,
    extend_start: bool,
    extend_end: bool,
}

#[pymethods]
impl PyLinearGradient {
    /// Create a new linear gradient.
    #[new]
    fn new() -> Self {
        PyLinearGradient {
            start: (0.0, 0.0),
            end: (100.0, 0.0),
            stops: Vec::new(),
            extend_start: true,
            extend_end: true,
        }
    }

    /// Set start point.
    fn start(&self, x: f32, y: f32) -> Self {
        PyLinearGradient {
            start: (x, y),
            end: self.end,
            stops: self.stops.clone(),
            extend_start: self.extend_start,
            extend_end: self.extend_end,
        }
    }

    /// Set end point.
    fn end(&self, x: f32, y: f32) -> Self {
        PyLinearGradient {
            start: self.start,
            end: (x, y),
            stops: self.stops.clone(),
            extend_start: self.extend_start,
            extend_end: self.extend_end,
        }
    }

    /// Add a color stop.
    ///
    /// Args:
    ///     position (float): Position along gradient (0.0 to 1.0)
    ///     color (Color): Color at this position
    fn add_stop(&self, position: f32, color: &PyColor) -> Self {
        let mut stops = self.stops.clone();
        stops.push((position.clamp(0.0, 1.0), color.inner));
        PyLinearGradient {
            start: self.start,
            end: self.end,
            stops,
            extend_start: self.extend_start,
            extend_end: self.extend_end,
        }
    }

    /// Set whether to extend gradient beyond endpoints.
    fn extend(&self, extend: bool) -> Self {
        PyLinearGradient {
            start: self.start,
            end: self.end,
            stops: self.stops.clone(),
            extend_start: extend,
            extend_end: extend,
        }
    }

    /// Create a horizontal gradient.
    #[staticmethod]
    fn horizontal(width: f32, start_color: &PyColor, end_color: &PyColor) -> Self {
        PyLinearGradient {
            start: (0.0, 0.0),
            end: (width, 0.0),
            stops: vec![(0.0, start_color.inner), (1.0, end_color.inner)],
            extend_start: true,
            extend_end: true,
        }
    }

    /// Create a vertical gradient.
    #[staticmethod]
    fn vertical(height: f32, start_color: &PyColor, end_color: &PyColor) -> Self {
        PyLinearGradient {
            start: (0.0, 0.0),
            end: (0.0, height),
            stops: vec![(0.0, start_color.inner), (1.0, end_color.inner)],
            extend_start: true,
            extend_end: true,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "LinearGradient(({}, {}) -> ({}, {}), {} stops)",
            self.start.0,
            self.start.1,
            self.end.0,
            self.end.1,
            self.stops.len()
        )
    }
}

/// Radial gradient builder.
///
/// Example:
///     >>> gradient = RadialGradient.centered(50, 50, 50) \
///     ...     .add_stop(0.0, Color.white()) \
///     ...     .add_stop(1.0, Color.black())
#[pyclass(name = "RadialGradient")]
#[derive(Clone)]
pub struct PyRadialGradient {
    inner_center: (f32, f32),
    inner_radius: f32,
    outer_center: (f32, f32),
    outer_radius: f32,
    stops: Vec<(f32, RustColor)>,
}

#[pymethods]
impl PyRadialGradient {
    /// Create a new radial gradient.
    #[new]
    fn new() -> Self {
        PyRadialGradient {
            inner_center: (50.0, 50.0),
            inner_radius: 0.0,
            outer_center: (50.0, 50.0),
            outer_radius: 50.0,
            stops: Vec::new(),
        }
    }

    /// Create a centered radial gradient.
    #[staticmethod]
    fn centered(cx: f32, cy: f32, radius: f32) -> Self {
        PyRadialGradient {
            inner_center: (cx, cy),
            inner_radius: 0.0,
            outer_center: (cx, cy),
            outer_radius: radius,
            stops: Vec::new(),
        }
    }

    /// Set inner circle.
    fn inner_circle(&self, cx: f32, cy: f32, radius: f32) -> Self {
        PyRadialGradient {
            inner_center: (cx, cy),
            inner_radius: radius,
            outer_center: self.outer_center,
            outer_radius: self.outer_radius,
            stops: self.stops.clone(),
        }
    }

    /// Set outer circle.
    fn outer_circle(&self, cx: f32, cy: f32, radius: f32) -> Self {
        PyRadialGradient {
            inner_center: self.inner_center,
            inner_radius: self.inner_radius,
            outer_center: (cx, cy),
            outer_radius: radius,
            stops: self.stops.clone(),
        }
    }

    /// Add a color stop.
    fn add_stop(&self, position: f32, color: &PyColor) -> Self {
        let mut stops = self.stops.clone();
        stops.push((position.clamp(0.0, 1.0), color.inner));
        PyRadialGradient {
            inner_center: self.inner_center,
            inner_radius: self.inner_radius,
            outer_center: self.outer_center,
            outer_radius: self.outer_radius,
            stops,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "RadialGradient(center=({}, {}), radius={}, {} stops)",
            self.outer_center.0,
            self.outer_center.1,
            self.outer_radius,
            self.stops.len()
        )
    }
}

/// Line cap styles.
#[pyclass(name = "LineCap")]
#[derive(Clone)]
pub struct PyLineCap {
    #[allow(dead_code)]
    inner: RustLineCap,
}

#[pymethods]
impl PyLineCap {
    /// Butt cap (default).
    #[staticmethod]
    #[allow(non_snake_case)]
    fn BUTT() -> Self {
        PyLineCap {
            inner: RustLineCap::Butt,
        }
    }

    /// Round cap.
    #[staticmethod]
    #[allow(non_snake_case)]
    fn ROUND() -> Self {
        PyLineCap {
            inner: RustLineCap::Round,
        }
    }

    /// Square cap.
    #[staticmethod]
    #[allow(non_snake_case)]
    fn SQUARE() -> Self {
        PyLineCap {
            inner: RustLineCap::Square,
        }
    }
}

/// Line join styles.
#[pyclass(name = "LineJoin")]
#[derive(Clone)]
pub struct PyLineJoin {
    #[allow(dead_code)]
    inner: RustLineJoin,
}

#[pymethods]
impl PyLineJoin {
    /// Miter join (default).
    #[staticmethod]
    #[allow(non_snake_case)]
    fn MITER() -> Self {
        PyLineJoin {
            inner: RustLineJoin::Miter,
        }
    }

    /// Round join.
    #[staticmethod]
    #[allow(non_snake_case)]
    fn ROUND() -> Self {
        PyLineJoin {
            inner: RustLineJoin::Round,
        }
    }

    /// Bevel join.
    #[staticmethod]
    #[allow(non_snake_case)]
    fn BEVEL() -> Self {
        PyLineJoin {
            inner: RustLineJoin::Bevel,
        }
    }
}

/// Pattern presets for common fill patterns.
///
/// Example:
///     >>> content = PatternPresets.checkerboard(10, Color.white(), Color.black())
#[pyclass(name = "PatternPresets")]
pub struct PyPatternPresets;

#[pymethods]
impl PyPatternPresets {
    /// Create horizontal stripes pattern.
    #[staticmethod]
    fn horizontal_stripes(width: f32, height: f32, stripe_height: f32, color: &PyColor) -> Vec<u8> {
        RustPatternPresets::horizontal_stripes(width, height, stripe_height, color.inner)
    }

    /// Create vertical stripes pattern.
    #[staticmethod]
    fn vertical_stripes(width: f32, height: f32, stripe_width: f32, color: &PyColor) -> Vec<u8> {
        RustPatternPresets::vertical_stripes(width, height, stripe_width, color.inner)
    }

    /// Create checkerboard pattern.
    #[staticmethod]
    fn checkerboard(size: f32, color1: &PyColor, color2: &PyColor) -> Vec<u8> {
        RustPatternPresets::checkerboard(size, color1.inner, color2.inner)
    }

    /// Create dot pattern.
    #[staticmethod]
    fn dots(spacing: f32, radius: f32, color: &PyColor) -> Vec<u8> {
        RustPatternPresets::dots(spacing, radius, color.inner)
    }

    /// Create diagonal lines pattern.
    #[staticmethod]
    fn diagonal_lines(size: f32, line_width: f32, color: &PyColor) -> Vec<u8> {
        RustPatternPresets::diagonal_lines(size, line_width, color.inner)
    }

    /// Create crosshatch pattern.
    #[staticmethod]
    fn crosshatch(size: f32, line_width: f32, color: &PyColor) -> Vec<u8> {
        RustPatternPresets::crosshatch(size, line_width, color.inner)
    }
}

/// Python module for PDF library.
///
/// This is the internal module (pdf_oxide) that gets imported by the Python package.
#[pymodule]
fn pdf_oxide(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Document reading
    m.add_class::<PyPdfDocument>()?;

    // PDF creation
    m.add_class::<PyPdf>()?;

    // Advanced graphics
    m.add_class::<PyColor>()?;
    m.add_class::<PyBlendMode>()?;
    m.add_class::<PyExtGState>()?;
    m.add_class::<PyLinearGradient>()?;
    m.add_class::<PyRadialGradient>()?;
    m.add_class::<PyLineCap>()?;
    m.add_class::<PyLineJoin>()?;
    m.add_class::<PyPatternPresets>()?;

    m.add("VERSION", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
