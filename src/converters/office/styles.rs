//! Office style to PDF style mapping.
//!
//! Maps formatting from Office documents (DOCX, XLSX, PPTX) to PDF equivalents.

#![allow(dead_code)]

use crate::writer::TextAlign;

/// Text formatting style from Office documents.
#[derive(Debug, Clone, Default)]
pub struct TextStyle {
    /// Font name
    pub font_name: Option<String>,
    /// Font size in points
    pub font_size: Option<f32>,
    /// Bold
    pub bold: bool,
    /// Italic
    pub italic: bool,
    /// Underline
    pub underline: bool,
    /// Strikethrough
    pub strikethrough: bool,
    /// Text color (RGB)
    pub color: Option<(f32, f32, f32)>,
    /// Background/highlight color (RGB)
    pub highlight: Option<(f32, f32, f32)>,
}

impl TextStyle {
    /// Get the PDF font name based on style.
    pub fn pdf_font_name(&self) -> &str {
        let base = self.font_name.as_deref().unwrap_or("Helvetica");

        // Map common Office fonts to PDF base fonts
        let mapped = match base.to_lowercase().as_str() {
            "times new roman" | "times" => "Times-Roman",
            "arial" | "helvetica" => "Helvetica",
            "courier new" | "courier" => "Courier",
            "symbol" => "Symbol",
            "zapfdingbats" | "wingdings" => "ZapfDingbats",
            _ => "Helvetica",
        };

        // Apply bold/italic variants
        match (self.bold, self.italic) {
            (true, true) => match mapped {
                "Helvetica" => "Helvetica-BoldOblique",
                "Times-Roman" => "Times-BoldItalic",
                "Courier" => "Courier-BoldOblique",
                _ => mapped,
            },
            (true, false) => match mapped {
                "Helvetica" => "Helvetica-Bold",
                "Times-Roman" => "Times-Bold",
                "Courier" => "Courier-Bold",
                _ => mapped,
            },
            (false, true) => match mapped {
                "Helvetica" => "Helvetica-Oblique",
                "Times-Roman" => "Times-Italic",
                "Courier" => "Courier-Oblique",
                _ => mapped,
            },
            (false, false) => mapped,
        }
    }
}

/// Paragraph formatting style from Office documents.
#[derive(Debug, Clone, Default)]
pub struct ParagraphStyle {
    /// Text alignment
    pub alignment: ParagraphAlignment,
    /// First line indent in points
    pub first_line_indent: f32,
    /// Left indent in points
    pub left_indent: f32,
    /// Right indent in points
    pub right_indent: f32,
    /// Space before paragraph in points
    pub space_before: f32,
    /// Space after paragraph in points
    pub space_after: f32,
    /// Line spacing multiplier
    pub line_spacing: f32,
    /// Whether this is a heading
    pub heading_level: Option<u8>,
    /// Whether this is a list item
    pub list_level: Option<u8>,
    /// List item bullet/number
    pub list_marker: Option<String>,
}

impl ParagraphStyle {
    /// Convert to PDF TextAlign.
    pub fn to_text_align(&self) -> TextAlign {
        match self.alignment {
            ParagraphAlignment::Left => TextAlign::Left,
            ParagraphAlignment::Center => TextAlign::Center,
            ParagraphAlignment::Right => TextAlign::Right,
            ParagraphAlignment::Justify => TextAlign::Left, // PDF doesn't have justify
        }
    }
}

/// Paragraph alignment options.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ParagraphAlignment {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

/// Table style from Office documents.
#[derive(Debug, Clone, Default)]
pub struct TableStyle {
    /// Border style
    pub borders: TableBorders,
    /// Cell padding in points
    pub cell_padding: f32,
    /// Header row background color
    pub header_background: Option<(f32, f32, f32)>,
    /// Alternating row colors
    pub alternating_rows: bool,
}

/// Table border configuration.
#[derive(Debug, Clone, Default)]
pub struct TableBorders {
    /// Show top border
    pub top: bool,
    /// Show bottom border
    pub bottom: bool,
    /// Show left border
    pub left: bool,
    /// Show right border
    pub right: bool,
    /// Show inner horizontal borders
    pub inner_horizontal: bool,
    /// Show inner vertical borders
    pub inner_vertical: bool,
    /// Border width in points
    pub width: f32,
    /// Border color (RGB)
    pub color: (f32, f32, f32),
}

impl TableBorders {
    /// Create borders with all sides enabled.
    pub fn all(width: f32) -> Self {
        Self {
            top: true,
            bottom: true,
            left: true,
            right: true,
            inner_horizontal: true,
            inner_vertical: true,
            width,
            color: (0.0, 0.0, 0.0),
        }
    }

    /// Create borders with no borders.
    pub fn none() -> Self {
        Self::default()
    }
}

/// Convert Office color values to RGB.
///
/// Office uses various color formats (theme colors, RGB, etc.).
/// This function normalizes them to RGB (0.0-1.0 range).
pub fn parse_color(color_str: &str) -> Option<(f32, f32, f32)> {
    // Handle hex colors (RRGGBB format)
    if color_str.len() == 6 {
        let r = u8::from_str_radix(&color_str[0..2], 16).ok()?;
        let g = u8::from_str_radix(&color_str[2..4], 16).ok()?;
        let b = u8::from_str_radix(&color_str[4..6], 16).ok()?;
        return Some((r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0));
    }

    // Handle named colors
    match color_str.to_lowercase().as_str() {
        "black" => Some((0.0, 0.0, 0.0)),
        "white" => Some((1.0, 1.0, 1.0)),
        "red" => Some((1.0, 0.0, 0.0)),
        "green" => Some((0.0, 1.0, 0.0)),
        "blue" => Some((0.0, 0.0, 1.0)),
        "yellow" => Some((1.0, 1.0, 0.0)),
        "cyan" => Some((0.0, 1.0, 1.0)),
        "magenta" => Some((1.0, 0.0, 1.0)),
        "gray" | "grey" => Some((0.5, 0.5, 0.5)),
        _ => None,
    }
}

/// Convert Office EMU (English Metric Units) to points.
///
/// 1 inch = 914400 EMU
/// 1 point = 914400/72 = 12700 EMU
pub fn emu_to_points(emu: i64) -> f32 {
    emu as f32 / 12700.0
}

/// Convert Office twips to points.
///
/// 1 point = 20 twips
pub fn twips_to_points(twips: i32) -> f32 {
    twips as f32 / 20.0
}

/// Convert Office half-points to points.
///
/// Font sizes in DOCX are often in half-points.
pub fn half_points_to_points(half_points: i32) -> f32 {
    half_points as f32 / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_style_font_mapping() {
        let mut style = TextStyle::default();
        assert_eq!(style.pdf_font_name(), "Helvetica");

        style.bold = true;
        assert_eq!(style.pdf_font_name(), "Helvetica-Bold");

        style.italic = true;
        assert_eq!(style.pdf_font_name(), "Helvetica-BoldOblique");

        style.bold = false;
        assert_eq!(style.pdf_font_name(), "Helvetica-Oblique");
    }

    #[test]
    fn test_text_style_times_font() {
        let mut style = TextStyle {
            font_name: Some("Times New Roman".to_string()),
            ..Default::default()
        };
        assert_eq!(style.pdf_font_name(), "Times-Roman");

        style.bold = true;
        assert_eq!(style.pdf_font_name(), "Times-Bold");

        style.italic = true;
        assert_eq!(style.pdf_font_name(), "Times-BoldItalic");
    }

    #[test]
    fn test_parse_color_hex() {
        let color = parse_color("FF0000").unwrap();
        assert!((color.0 - 1.0).abs() < 0.01);
        assert!(color.1 < 0.01);
        assert!(color.2 < 0.01);
    }

    #[test]
    fn test_parse_color_named() {
        assert_eq!(parse_color("black"), Some((0.0, 0.0, 0.0)));
        assert_eq!(parse_color("white"), Some((1.0, 1.0, 1.0)));
        assert_eq!(parse_color("red"), Some((1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_emu_to_points() {
        // 1 inch = 72 points = 914400 EMU
        assert!((emu_to_points(914400) - 72.0).abs() < 0.1);
    }

    #[test]
    fn test_twips_to_points() {
        assert_eq!(twips_to_points(20), 1.0);
        assert_eq!(twips_to_points(240), 12.0);
    }

    #[test]
    fn test_half_points_to_points() {
        assert_eq!(half_points_to_points(24), 12.0);
        assert_eq!(half_points_to_points(22), 11.0);
    }
}
