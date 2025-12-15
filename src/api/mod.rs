//! High-level PDF API for simple document creation and manipulation.
//!
//! This module provides a unified, easy-to-use API for common PDF operations:
//! - Creating PDFs from Markdown, HTML, or plain text
//! - Opening and editing existing PDFs
//! - Converting between formats
//!
//! ## Quick Start
//!
//! ```ignore
//! use pdf_oxide::api::Pdf;
//!
//! // Create from Markdown
//! let pdf = Pdf::from_markdown("# Hello World\n\nThis is a PDF.")?;
//! pdf.save("output.pdf")?;
//!
//! // Create from HTML
//! let pdf = Pdf::from_html("<h1>Hello</h1><p>World</p>")?;
//! pdf.save("output.pdf")?;
//!
//! // Create from plain text
//! let pdf = Pdf::from_text("Plain text content")?;
//! pdf.save("output.pdf")?;
//! ```
//!
//! ## Builder Pattern
//!
//! For more control, use the `PdfBuilder`:
//!
//! ```ignore
//! use pdf_oxide::api::PdfBuilder;
//!
//! let pdf = PdfBuilder::new()
//!     .title("My Document")
//!     .author("John Doe")
//!     .page_size(PageSize::A4)
//!     .margins(72.0, 72.0, 72.0, 72.0)
//!     .from_markdown("# Content")?;
//! ```
//!
//! ## Editing Existing PDFs
//!
//! ```ignore
//! use pdf_oxide::api::Pdf;
//!
//! let mut pdf = Pdf::open("existing.pdf")?;
//! pdf.set_title("New Title");
//! pdf.append_markdown("# New Section")?;
//! pdf.save("modified.pdf")?;
//! ```

mod pdf_builder;

pub use pdf_builder::{Pdf, PdfBuilder, PdfConfig};
