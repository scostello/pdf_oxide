//! Page rendering module for converting PDF pages to images.
//!
//! This module provides functionality to render PDF pages to raster images
//! using the pure-Rust `tiny-skia` library.
//!
//! ## Features
//!
//! - Render pages to PNG/JPEG images
//! - Configurable DPI and image quality
//! - Support for text, paths, and images
//! - Transparency and blend modes
//!
//! ## Example
//!
//! ```ignore
//! use pdf_oxide::api::Pdf;
//! use pdf_oxide::rendering::{RenderOptions, ImageFormat};
//!
//! let mut pdf = Pdf::open("document.pdf")?;
//! let image = pdf.render_page(0, &RenderOptions::default())?;
//! image.save("page1.png")?;
//! ```
//!
//! ## Architecture
//!
//! The rendering pipeline:
//!
//! 1. Parse page content stream into operators
//! 2. Execute operators against graphics state machine
//! 3. Rasterize paths, text, and images to tiny-skia pixmap
//! 4. Convert to output format (PNG/JPEG)

mod page_renderer;
mod path_rasterizer;
mod text_rasterizer;

pub use page_renderer::{ImageFormat, PageRenderer, RenderOptions, RenderedImage};

use crate::error::Result;

/// Render a PDF page to an image.
///
/// This is a convenience function that creates a PageRenderer and renders
/// a single page.
///
/// # Arguments
///
/// * `doc` - The PDF document
/// * `page_num` - Zero-based page number
/// * `options` - Rendering options (DPI, format, etc.)
///
/// # Returns
///
/// The rendered image as bytes in the specified format.
pub fn render_page(
    doc: &mut crate::document::PdfDocument,
    page_num: usize,
    options: &RenderOptions,
) -> Result<RenderedImage> {
    let mut renderer = PageRenderer::new(options.clone());
    renderer.render_page(doc, page_num)
}
