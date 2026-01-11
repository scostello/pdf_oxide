//! Path rasterizer - renders PDF paths using tiny-skia.

use crate::content::GraphicsState;
use tiny_skia::{Color, FillRule, LineCap, LineJoin, Paint, Path, Pixmap, Stroke, Transform};

/// Rasterizer for PDF path operations.
pub struct PathRasterizer {
    // Could hold caches, state, etc.
}

impl PathRasterizer {
    /// Create a new path rasterizer.
    pub fn new() -> Self {
        Self {}
    }

    /// Fill a path with the current fill color.
    pub fn fill_path(
        &self,
        pixmap: &mut Pixmap,
        path: &Path,
        transform: Transform,
        gs: &GraphicsState,
        fill_rule: FillRule,
    ) {
        let (r, g, b) = gs.fill_color_rgb;
        let alpha = gs.fill_alpha;

        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba(r, g, b, alpha).unwrap_or(Color::BLACK));
        paint.anti_alias = true;

        // Apply blend mode if not normal
        if gs.blend_mode != "Normal" {
            paint.blend_mode = self.pdf_blend_mode_to_skia(&gs.blend_mode);
        }

        pixmap.fill_path(path, &paint, fill_rule, transform, None);
    }

    /// Stroke a path with the current stroke color and line style.
    pub fn stroke_path(
        &self,
        pixmap: &mut Pixmap,
        path: &Path,
        transform: Transform,
        gs: &GraphicsState,
    ) {
        let (r, g, b) = gs.stroke_color_rgb;
        let alpha = gs.stroke_alpha;

        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba(r, g, b, alpha).unwrap_or(Color::BLACK));
        paint.anti_alias = true;

        // Apply blend mode if not normal
        if gs.blend_mode != "Normal" {
            paint.blend_mode = self.pdf_blend_mode_to_skia(&gs.blend_mode);
        }

        let dash = if !gs.dash_pattern.0.is_empty() {
            tiny_skia::StrokeDash::new(gs.dash_pattern.0.clone(), gs.dash_pattern.1)
        } else {
            None
        };

        let stroke = Stroke {
            width: gs.line_width,
            line_cap: self.pdf_line_cap_to_skia(gs.line_cap),
            line_join: self.pdf_line_join_to_skia(gs.line_join),
            miter_limit: gs.miter_limit,
            dash,
        };

        pixmap.stroke_path(path, &paint, &stroke, transform, None);
    }

    /// Convert PDF line cap style to tiny-skia.
    fn pdf_line_cap_to_skia(&self, cap: u8) -> LineCap {
        match cap {
            0 => LineCap::Butt,
            1 => LineCap::Round,
            2 => LineCap::Square,
            _ => LineCap::Butt,
        }
    }

    /// Convert PDF line join style to tiny-skia.
    fn pdf_line_join_to_skia(&self, join: u8) -> LineJoin {
        match join {
            0 => LineJoin::Miter,
            1 => LineJoin::Round,
            2 => LineJoin::Bevel,
            _ => LineJoin::Miter,
        }
    }

    /// Convert PDF blend mode to tiny-skia.
    fn pdf_blend_mode_to_skia(&self, mode: &str) -> tiny_skia::BlendMode {
        match mode {
            "Normal" => tiny_skia::BlendMode::SourceOver,
            "Multiply" => tiny_skia::BlendMode::Multiply,
            "Screen" => tiny_skia::BlendMode::Screen,
            "Overlay" => tiny_skia::BlendMode::Overlay,
            "Darken" => tiny_skia::BlendMode::Darken,
            "Lighten" => tiny_skia::BlendMode::Lighten,
            "ColorDodge" => tiny_skia::BlendMode::ColorDodge,
            "ColorBurn" => tiny_skia::BlendMode::ColorBurn,
            "HardLight" => tiny_skia::BlendMode::HardLight,
            "SoftLight" => tiny_skia::BlendMode::SoftLight,
            "Difference" => tiny_skia::BlendMode::Difference,
            "Exclusion" => tiny_skia::BlendMode::Exclusion,
            _ => tiny_skia::BlendMode::SourceOver,
        }
    }
}

impl Default for PathRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_rasterizer_new() {
        let rasterizer = PathRasterizer::new();
        // Just verify it can be created
        assert_eq!(rasterizer.pdf_line_cap_to_skia(0), LineCap::Butt);
    }

    #[test]
    fn test_line_cap_conversion() {
        let rasterizer = PathRasterizer::new();
        assert_eq!(rasterizer.pdf_line_cap_to_skia(0), LineCap::Butt);
        assert_eq!(rasterizer.pdf_line_cap_to_skia(1), LineCap::Round);
        assert_eq!(rasterizer.pdf_line_cap_to_skia(2), LineCap::Square);
        assert_eq!(rasterizer.pdf_line_cap_to_skia(99), LineCap::Butt); // Unknown defaults to Butt
    }

    #[test]
    fn test_line_join_conversion() {
        let rasterizer = PathRasterizer::new();
        assert_eq!(rasterizer.pdf_line_join_to_skia(0), LineJoin::Miter);
        assert_eq!(rasterizer.pdf_line_join_to_skia(1), LineJoin::Round);
        assert_eq!(rasterizer.pdf_line_join_to_skia(2), LineJoin::Bevel);
    }

    #[test]
    fn test_blend_mode_conversion() {
        let rasterizer = PathRasterizer::new();
        assert_eq!(rasterizer.pdf_blend_mode_to_skia("Normal"), tiny_skia::BlendMode::SourceOver);
        assert_eq!(rasterizer.pdf_blend_mode_to_skia("Multiply"), tiny_skia::BlendMode::Multiply);
        assert_eq!(rasterizer.pdf_blend_mode_to_skia("Unknown"), tiny_skia::BlendMode::SourceOver);
    }
}
