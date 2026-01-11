//! PDF/A compliance validation and creation module.
//!
//! This module provides functionality for validating PDF documents against
//! PDF/A standards and converting documents to be PDF/A compliant.
//!
//! ## PDF/A Conformance Levels
//!
//! - **PDF/A-1b**: Basic conformance, visual appearance preservation
//! - **PDF/A-1a**: Full conformance, includes logical structure (Tagged PDF)
//! - **PDF/A-2b**: Based on PDF 1.7, allows JPEG2000, transparency
//! - **PDF/A-2a**: PDF/A-2b plus logical structure
//! - **PDF/A-2u**: PDF/A-2b plus Unicode mapping
//! - **PDF/A-3b**: PDF/A-2b plus embedded files of any type
//! - **PDF/A-3a**: PDF/A-3b plus logical structure
//! - **PDF/A-3u**: PDF/A-3b plus Unicode mapping
//!
//! ## Key Requirements
//!
//! - All fonts must be embedded
//! - No encryption allowed
//! - XMP metadata required with PDF/A identification
//! - Device-independent colors (ICC profiles or sRGB)
//! - No JavaScript, audio, or video
//! - No transparency (PDF/A-1 only)
//!
//! ## Example
//!
//! ```ignore
//! use pdf_oxide::api::Pdf;
//! use pdf_oxide::compliance::{PdfAValidator, PdfALevel};
//!
//! let pdf = Pdf::open("document.pdf")?;
//!
//! // Validate against PDF/A-2b
//! let validator = PdfAValidator::new();
//! let result = validator.validate(&pdf, PdfALevel::A2b)?;
//!
//! if result.is_compliant {
//!     println!("Document is PDF/A-2b compliant");
//! } else {
//!     for error in &result.errors {
//!         println!("Violation: {}", error);
//!     }
//! }
//! ```
//!
//! ## Standards Reference
//!
//! - ISO 19005-1:2005 (PDF/A-1)
//! - ISO 19005-2:2011 (PDF/A-2)
//! - ISO 19005-3:2012 (PDF/A-3)

mod pdf_a;
mod types;
mod validators;

pub use pdf_a::{validate_pdf_a, PdfAValidator};
pub use types::{
    ComplianceError, ComplianceWarning, ErrorCode, PdfALevel, PdfAPart, ValidationResult,
    ValidationStats, WarningCode,
};
