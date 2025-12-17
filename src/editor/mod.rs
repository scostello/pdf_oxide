//! PDF editing module for modifying existing PDF documents.
#![allow(dead_code, unused_variables, unused_mut, clippy::write_with_newline)]
//!
//! This module provides a high-level API for editing PDF documents:
//! - Metadata editing (title, author, subject, keywords)
//! - Page operations (add, remove, reorder, extract)
//! - Content manipulation
//! - PDF merging
//!
//! ## Architecture
//!
//! ```text
//! PdfDocument (read-only source)
//!     ↓
//! [DocumentEditor] (tracks modifications)
//!     ↓
//! Save Options:
//!   - Incremental update (append to original)
//!   - Full rewrite (new PDF structure)
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use pdf_oxide::editor::DocumentEditor;
//!
//! // Open an existing PDF for editing
//! let mut editor = DocumentEditor::open("input.pdf")?;
//!
//! // Edit metadata
//! editor.set_title("Updated Title");
//! editor.set_author("John Doe");
//!
//! // Modify pages
//! editor.remove_page(5)?;
//! editor.move_page(2, 0)?;  // Move page 2 to front
//!
//! // Save changes
//! editor.save("output.pdf")?;  // Full rewrite
//! // or
//! editor.save_incremental("output.pdf")?;  // Append changes
//! ```

mod document_editor;
pub mod dom;
pub mod resource_manager;

pub use document_editor::{DocumentEditor, DocumentInfo, EditableDocument, PageInfo, SaveOptions};
pub use dom::{
    ElementId, ImageElementCollectionEditor, PageEditor, PdfElement, PdfImage, PdfPage, PdfPath,
    PdfStructure, PdfTable, PdfText, TextElementCollectionEditor,
};
pub use resource_manager::ResourceManager;
