//! Page renderer - converts PDF pages to raster images.

use crate::content::{parse_content_stream, GraphicsState, GraphicsStateStack, Matrix, Operator};
use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::object::Object;

use tiny_skia::{Color, FillRule, PathBuilder, Pixmap, PixmapPaint, Transform};

use super::path_rasterizer::PathRasterizer;
use super::text_rasterizer::TextRasterizer;

/// Output image format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageFormat {
    /// PNG format (lossless, supports transparency)
    #[default]
    Png,
    /// JPEG format (lossy, smaller file size)
    Jpeg,
}

/// Options for page rendering.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Dots per inch (default: 150)
    pub dpi: u32,
    /// Output image format
    pub format: ImageFormat,
    /// Background color (None for transparent)
    pub background: Option<[f32; 4]>,
    /// Whether to render annotations
    pub render_annotations: bool,
    /// JPEG quality (1-100, only for JPEG format)
    pub jpeg_quality: u8,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            dpi: 150,
            format: ImageFormat::Png,
            background: Some([1.0, 1.0, 1.0, 1.0]), // White background
            render_annotations: true,
            jpeg_quality: 85,
        }
    }
}

impl RenderOptions {
    /// Create options with custom DPI.
    pub fn with_dpi(dpi: u32) -> Self {
        Self {
            dpi,
            ..Default::default()
        }
    }

    /// Set transparent background.
    pub fn with_transparent_background(mut self) -> Self {
        self.background = None;
        self
    }

    /// Set JPEG format with quality.
    pub fn as_jpeg(mut self, quality: u8) -> Self {
        self.format = ImageFormat::Jpeg;
        self.jpeg_quality = quality.clamp(1, 100);
        self
    }
}

/// Rendered image output.
#[derive(Debug, Clone)]
pub struct RenderedImage {
    /// Image data in the specified format
    pub data: Vec<u8>,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Output format
    pub format: ImageFormat,
}

impl RenderedImage {
    /// Save the image to a file.
    pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        std::fs::write(path.as_ref(), &self.data).map_err(|e| Error::Io(std::io::Error::other(e)))
    }

    /// Get the image data as bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

/// Page renderer that converts PDF pages to raster images.
pub struct PageRenderer {
    options: RenderOptions,
    path_rasterizer: PathRasterizer,
    text_rasterizer: TextRasterizer,
}

impl PageRenderer {
    /// Create a new page renderer with the given options.
    pub fn new(options: RenderOptions) -> Self {
        Self {
            options,
            path_rasterizer: PathRasterizer::new(),
            text_rasterizer: TextRasterizer::new(),
        }
    }

    /// Render a page to an image.
    pub fn render_page(&mut self, doc: &mut PdfDocument, page_num: usize) -> Result<RenderedImage> {
        // Get page dimensions
        let page_info = doc.get_page_info(page_num)?;
        let media_box = page_info.media_box;

        // Calculate pixel dimensions based on DPI
        let scale = self.options.dpi as f32 / 72.0; // PDF uses 72 points per inch
        let width = (media_box.width * scale).ceil() as u32;
        let height = (media_box.height * scale).ceil() as u32;

        // Create pixmap with background
        let mut pixmap = Pixmap::new(width, height).ok_or_else(|| {
            Error::InvalidPdf(format!("Failed to create pixmap {}x{}", width, height))
        })?;

        // Fill background
        if let Some([r, g, b, a]) = self.options.background {
            pixmap.fill(Color::from_rgba(r, g, b, a).unwrap_or(Color::WHITE));
        }

        // Create transform: PDF coordinates to pixel coordinates
        // PDF origin is bottom-left, we need to flip Y axis
        let transform = Transform::from_scale(scale, -scale)
            .post_translate(0.0, height as f32)
            .post_translate(-media_box.x * scale, media_box.y * scale);

        // Get page content stream
        let content_data = doc.get_page_content_data(page_num)?;

        // Parse content stream
        let operators = parse_content_stream(&content_data)?;

        // Get page resources for fonts and images
        let resources = doc.get_page_resources(page_num)?;

        // Execute operators and render
        self.execute_operators(&mut pixmap, transform, &operators, doc, page_num, &resources)?;

        // Encode to output format
        let data = match self.options.format {
            ImageFormat::Png => pixmap
                .encode_png()
                .map_err(|e| Error::InvalidPdf(format!("PNG encoding failed: {}", e)))?,
            ImageFormat::Jpeg => {
                // Convert RGBA to RGB for JPEG
                self.encode_jpeg(&pixmap)?
            },
        };

        Ok(RenderedImage {
            data,
            width,
            height,
            format: self.options.format,
        })
    }

    /// Execute content stream operators and render to pixmap.
    fn execute_operators(
        &mut self,
        pixmap: &mut Pixmap,
        base_transform: Transform,
        operators: &[Operator],
        doc: &mut PdfDocument,
        page_num: usize,
        resources: &Object,
    ) -> Result<()> {
        let mut gs_stack = GraphicsStateStack::new();
        let mut current_path = PathBuilder::new();
        let mut in_text_object = false;

        for op in operators {
            match op {
                // Graphics state operators
                Operator::SaveState => {
                    gs_stack.save();
                },
                Operator::RestoreState => {
                    gs_stack.restore();
                },
                Operator::Cm { a, b, c, d, e, f } => {
                    let matrix = Matrix {
                        a: *a,
                        b: *b,
                        c: *c,
                        d: *d,
                        e: *e,
                        f: *f,
                    };
                    let current = gs_stack.current_mut();
                    current.ctm = current.ctm.multiply(&matrix);
                },

                // Color operators
                Operator::SetFillRgb { r, g, b } => {
                    gs_stack.current_mut().fill_color_rgb = (*r, *g, *b);
                    gs_stack.current_mut().fill_color_space = "DeviceRGB".to_string();
                },
                Operator::SetStrokeRgb { r, g, b } => {
                    gs_stack.current_mut().stroke_color_rgb = (*r, *g, *b);
                    gs_stack.current_mut().stroke_color_space = "DeviceRGB".to_string();
                },
                Operator::SetFillGray { gray } => {
                    let g = *gray;
                    gs_stack.current_mut().fill_color_rgb = (g, g, g);
                    gs_stack.current_mut().fill_color_space = "DeviceGray".to_string();
                },
                Operator::SetStrokeGray { gray } => {
                    let g = *gray;
                    gs_stack.current_mut().stroke_color_rgb = (g, g, g);
                    gs_stack.current_mut().stroke_color_space = "DeviceGray".to_string();
                },
                Operator::SetFillCmyk { c, m, y, k } => {
                    // Convert CMYK to RGB
                    let (r, g, b) = cmyk_to_rgb(*c, *m, *y, *k);
                    gs_stack.current_mut().fill_color_rgb = (r, g, b);
                    gs_stack.current_mut().fill_color_cmyk = Some((*c, *m, *y, *k));
                    gs_stack.current_mut().fill_color_space = "DeviceCMYK".to_string();
                },
                Operator::SetStrokeCmyk { c, m, y, k } => {
                    let (r, g, b) = cmyk_to_rgb(*c, *m, *y, *k);
                    gs_stack.current_mut().stroke_color_rgb = (r, g, b);
                    gs_stack.current_mut().stroke_color_cmyk = Some((*c, *m, *y, *k));
                    gs_stack.current_mut().stroke_color_space = "DeviceCMYK".to_string();
                },

                // Line style operators
                Operator::SetLineWidth { width } => {
                    gs_stack.current_mut().line_width = *width;
                },
                Operator::SetLineCap { cap_style } => {
                    gs_stack.current_mut().line_cap = *cap_style;
                },
                Operator::SetLineJoin { join_style } => {
                    gs_stack.current_mut().line_join = *join_style;
                },
                Operator::SetMiterLimit { limit } => {
                    gs_stack.current_mut().miter_limit = *limit;
                },
                Operator::SetDash { array, phase } => {
                    gs_stack.current_mut().dash_pattern = (array.clone(), *phase);
                },

                // Path construction
                Operator::MoveTo { x, y } => {
                    current_path.move_to(*x, *y);
                },
                Operator::LineTo { x, y } => {
                    current_path.line_to(*x, *y);
                },
                Operator::CurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x3,
                    y3,
                } => {
                    current_path.cubic_to(*x1, *y1, *x2, *y2, *x3, *y3);
                },
                Operator::CurveToV { x2, y2, x3, y3 } => {
                    // First control point is current point
                    if let Some(last) = current_path.last_point() {
                        current_path.cubic_to(last.x, last.y, *x2, *y2, *x3, *y3);
                    }
                },
                Operator::CurveToY { x1, y1, x3, y3 } => {
                    // Second control point is end point
                    current_path.cubic_to(*x1, *y1, *x3, *y3, *x3, *y3);
                },
                Operator::Rectangle {
                    x,
                    y,
                    width,
                    height,
                } => {
                    current_path.push_rect(
                        tiny_skia::Rect::from_xywh(*x, *y, *width, *height)
                            .unwrap_or(tiny_skia::Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap()),
                    );
                },
                Operator::ClosePath => {
                    current_path.close();
                },

                // Path painting
                Operator::Stroke => {
                    if let Some(path) = current_path.finish() {
                        let gs = gs_stack.current();
                        let transform = combine_transforms(base_transform, &gs.ctm);
                        self.path_rasterizer
                            .stroke_path(pixmap, &path, transform, gs);
                    }
                    current_path = PathBuilder::new();
                },
                Operator::Fill | Operator::CloseFillStroke => {
                    if let Some(path) = current_path.finish() {
                        let gs = gs_stack.current();
                        let transform = combine_transforms(base_transform, &gs.ctm);
                        self.path_rasterizer.fill_path(
                            pixmap,
                            &path,
                            transform,
                            gs,
                            FillRule::Winding,
                        );
                        if matches!(op, Operator::CloseFillStroke) {
                            self.path_rasterizer
                                .stroke_path(pixmap, &path, transform, gs);
                        }
                    }
                    current_path = PathBuilder::new();
                },
                Operator::FillEvenOdd => {
                    if let Some(path) = current_path.finish() {
                        let gs = gs_stack.current();
                        let transform = combine_transforms(base_transform, &gs.ctm);
                        self.path_rasterizer.fill_path(
                            pixmap,
                            &path,
                            transform,
                            gs,
                            FillRule::EvenOdd,
                        );
                    }
                    current_path = PathBuilder::new();
                },
                Operator::EndPath => {
                    current_path = PathBuilder::new();
                },
                Operator::ClipNonZero | Operator::ClipEvenOdd => {
                    // TODO: Implement clipping
                    current_path = PathBuilder::new();
                },

                // Text operators
                Operator::BeginText => {
                    in_text_object = true;
                    let gs = gs_stack.current_mut();
                    gs.text_matrix = Matrix::identity();
                    gs.text_line_matrix = Matrix::identity();
                },
                Operator::EndText => {
                    in_text_object = false;
                },
                Operator::Td { tx, ty } => {
                    if in_text_object {
                        let gs = gs_stack.current_mut();
                        let translation = Matrix::translation(*tx, *ty);
                        gs.text_line_matrix = gs.text_line_matrix.multiply(&translation);
                        gs.text_matrix = gs.text_line_matrix;
                    }
                },
                Operator::TD { tx, ty } => {
                    if in_text_object {
                        let gs = gs_stack.current_mut();
                        gs.leading = -(*ty);
                        let translation = Matrix::translation(*tx, *ty);
                        gs.text_line_matrix = gs.text_line_matrix.multiply(&translation);
                        gs.text_matrix = gs.text_line_matrix;
                    }
                },
                Operator::Tm { a, b, c, d, e, f } => {
                    if in_text_object {
                        let gs = gs_stack.current_mut();
                        gs.text_matrix = Matrix {
                            a: *a,
                            b: *b,
                            c: *c,
                            d: *d,
                            e: *e,
                            f: *f,
                        };
                        gs.text_line_matrix = gs.text_matrix;
                    }
                },
                Operator::TStar => {
                    if in_text_object {
                        let gs = gs_stack.current_mut();
                        let leading = gs.leading;
                        let translation = Matrix::translation(0.0, -leading);
                        gs.text_line_matrix = gs.text_line_matrix.multiply(&translation);
                        gs.text_matrix = gs.text_line_matrix;
                    }
                },
                Operator::Tf { font, size } => {
                    let gs = gs_stack.current_mut();
                    gs.font_name = Some(font.clone());
                    gs.font_size = *size;
                },
                Operator::Tc { char_space } => {
                    gs_stack.current_mut().char_space = *char_space;
                },
                Operator::Tw { word_space } => {
                    gs_stack.current_mut().word_space = *word_space;
                },
                Operator::Tz { scale } => {
                    gs_stack.current_mut().horizontal_scaling = *scale;
                },
                Operator::TL { leading } => {
                    gs_stack.current_mut().leading = *leading;
                },
                Operator::Ts { rise } => {
                    gs_stack.current_mut().text_rise = *rise;
                },
                Operator::Tr { render } => {
                    gs_stack.current_mut().render_mode = *render;
                },

                // Text showing
                Operator::Tj { text } | Operator::Quote { text } => {
                    if in_text_object {
                        let gs = gs_stack.current();
                        let transform = combine_transforms(base_transform, &gs.ctm);
                        self.text_rasterizer
                            .render_text(pixmap, text, transform, gs, resources, doc)?;
                        // Advance text position
                        // TODO: Calculate proper advance width based on font metrics
                    }
                },
                Operator::TJ { array } => {
                    if in_text_object {
                        let gs = gs_stack.current();
                        let transform = combine_transforms(base_transform, &gs.ctm);
                        self.text_rasterizer
                            .render_tj_array(pixmap, array, transform, gs, resources, doc)?;
                    }
                },
                Operator::DoubleQuote {
                    word_space,
                    char_space,
                    text,
                } => {
                    if in_text_object {
                        gs_stack.current_mut().word_space = *word_space;
                        gs_stack.current_mut().char_space = *char_space;
                        let gs = gs_stack.current();
                        let transform = combine_transforms(base_transform, &gs.ctm);
                        self.text_rasterizer
                            .render_text(pixmap, text, transform, gs, resources, doc)?;
                    }
                },

                // XObject (images)
                Operator::Do { name } => {
                    let gs = gs_stack.current();
                    let transform = combine_transforms(base_transform, &gs.ctm);
                    self.render_xobject(pixmap, name, transform, gs, resources, doc, page_num)?;
                },

                // Extended graphics state
                Operator::SetExtGState { dict_name } => {
                    self.apply_ext_g_state(gs_stack.current_mut(), dict_name, resources)?;
                },

                // Ignore other operators for now
                _ => {},
            }
        }

        Ok(())
    }

    /// Render an XObject (image or form).
    fn render_xobject(
        &mut self,
        pixmap: &mut Pixmap,
        name: &str,
        transform: Transform,
        _gs: &GraphicsState,
        resources: &Object,
        doc: &mut PdfDocument,
        _page_num: usize,
    ) -> Result<()> {
        // Get XObject from resources
        if let Object::Dictionary(res_dict) = resources {
            if let Some(Object::Dictionary(xobjects)) = res_dict.get("XObjects") {
                if let Some(xobj_ref) = xobjects.get(name) {
                    // Resolve reference if needed
                    let xobj = doc.resolve_object(xobj_ref)?;

                    if let Object::Stream { dict, data } = xobj {
                        // Check subtype
                        if let Some(Object::Name(subtype)) = dict.get("Subtype") {
                            match subtype.as_str() {
                                "Image" => {
                                    self.render_image(pixmap, &dict, &data, transform)?;
                                },
                                "Form" => {
                                    // TODO: Render form XObject (recursive content stream)
                                },
                                _ => {},
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Render an image XObject.
    fn render_image(
        &mut self,
        pixmap: &mut Pixmap,
        dict: &std::collections::HashMap<String, Object>,
        data: &[u8],
        transform: Transform,
    ) -> Result<()> {
        // Get image dimensions
        let width = dict
            .get("Width")
            .and_then(|o| match o {
                Object::Integer(i) => Some(*i as u32),
                _ => None,
            })
            .unwrap_or(1);
        let height = dict
            .get("Height")
            .and_then(|o| match o {
                Object::Integer(i) => Some(*i as u32),
                _ => None,
            })
            .unwrap_or(1);

        // Decode image data to RGBA
        // This is a simplified implementation - real PDF images need proper
        // color space handling, filters, etc.
        let rgba_data = self.decode_image_data(dict, data, width, height)?;

        // Create tiny-skia pixmap from RGBA data
        if let Some(img_pixmap) =
            Pixmap::from_vec(rgba_data, tiny_skia::IntSize::from_wh(width, height).unwrap())
        {
            // Draw image with transform
            let paint = PixmapPaint::default();
            pixmap.draw_pixmap(0, 0, img_pixmap.as_ref(), &paint, transform, None);
        }

        Ok(())
    }

    /// Decode image data to RGBA.
    fn decode_image_data(
        &self,
        dict: &std::collections::HashMap<String, Object>,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>> {
        let _bits_per_component = dict
            .get("BitsPerComponent")
            .and_then(|o| match o {
                Object::Integer(i) => Some(*i as u8),
                _ => None,
            })
            .unwrap_or(8);

        let color_space = dict
            .get("ColorSpace")
            .and_then(|o| match o {
                Object::Name(n) => Some(n.as_str()),
                _ => None,
            })
            .unwrap_or("DeviceRGB");

        let components = match color_space {
            "DeviceGray" => 1,
            "DeviceRGB" => 3,
            "DeviceCMYK" => 4,
            _ => 3, // Default to RGB
        };

        let _expected_size = (width * height * components as u32) as usize;

        // Convert to RGBA based on color space
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);

        match color_space {
            "DeviceGray" => {
                for i in 0..(width * height) as usize {
                    let g = if i < data.len() { data[i] } else { 0 };
                    rgba.extend_from_slice(&[g, g, g, 255]);
                }
            },
            "DeviceRGB" => {
                for i in 0..(width * height) as usize {
                    let base = i * 3;
                    let r = data.get(base).copied().unwrap_or(0);
                    let g = data.get(base + 1).copied().unwrap_or(0);
                    let b = data.get(base + 2).copied().unwrap_or(0);
                    rgba.extend_from_slice(&[r, g, b, 255]);
                }
            },
            "DeviceCMYK" => {
                for i in 0..(width * height) as usize {
                    let base = i * 4;
                    let c = data.get(base).copied().unwrap_or(0) as f32 / 255.0;
                    let m = data.get(base + 1).copied().unwrap_or(0) as f32 / 255.0;
                    let y = data.get(base + 2).copied().unwrap_or(0) as f32 / 255.0;
                    let k = data.get(base + 3).copied().unwrap_or(0) as f32 / 255.0;
                    let (r, g, b) = cmyk_to_rgb(c, m, y, k);
                    rgba.extend_from_slice(&[
                        (r * 255.0) as u8,
                        (g * 255.0) as u8,
                        (b * 255.0) as u8,
                        255,
                    ]);
                }
            },
            _ => {
                // Unknown color space - fill with gray
                for _ in 0..(width * height) {
                    rgba.extend_from_slice(&[128, 128, 128, 255]);
                }
            },
        }

        Ok(rgba)
    }

    /// Apply extended graphics state parameters.
    fn apply_ext_g_state(
        &self,
        gs: &mut GraphicsState,
        dict_name: &str,
        resources: &Object,
    ) -> Result<()> {
        if let Object::Dictionary(res_dict) = resources {
            if let Some(Object::Dictionary(ext_gstates)) = res_dict.get("ExtGState") {
                if let Some(Object::Dictionary(state_dict)) = ext_gstates.get(dict_name) {
                    // Apply transparency
                    if let Some(Object::Real(ca)) = state_dict.get("ca") {
                        gs.fill_alpha = *ca as f32;
                    }
                    if let Some(Object::Real(ca)) = state_dict.get("CA") {
                        gs.stroke_alpha = *ca as f32;
                    }
                    // Apply blend mode
                    if let Some(Object::Name(bm)) = state_dict.get("BM") {
                        gs.blend_mode = bm.clone();
                    }
                    // Apply line width
                    if let Some(Object::Real(lw)) = state_dict.get("LW") {
                        gs.line_width = *lw as f32;
                    }
                }
            }
        }
        Ok(())
    }

    /// Encode pixmap to JPEG format.
    fn encode_jpeg(&self, pixmap: &Pixmap) -> Result<Vec<u8>> {
        use image::ImageBuffer;

        // Convert to image crate format
        let width = pixmap.width();
        let height = pixmap.height();
        let data = pixmap.data();

        // Create RGB image (JPEG doesn't support alpha)
        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
        for chunk in data.chunks(4) {
            rgb_data.push(chunk[0]); // R
            rgb_data.push(chunk[1]); // G
            rgb_data.push(chunk[2]); // B
        }

        let img: ImageBuffer<image::Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_raw(width, height, rgb_data)
                .ok_or_else(|| Error::InvalidPdf("Failed to create image buffer".to_string()))?;

        // Encode to JPEG
        let mut output = std::io::Cursor::new(Vec::new());
        img.write_to(&mut output, image::ImageFormat::Jpeg)
            .map_err(|e| Error::InvalidPdf(format!("JPEG encoding failed: {}", e)))?;

        Ok(output.into_inner())
    }
}

/// Convert CMYK to RGB.
fn cmyk_to_rgb(c: f32, m: f32, y: f32, k: f32) -> (f32, f32, f32) {
    let r = (1.0 - c) * (1.0 - k);
    let g = (1.0 - m) * (1.0 - k);
    let b = (1.0 - y) * (1.0 - k);
    (r, g, b)
}

/// Combine base transform with PDF matrix.
fn combine_transforms(base: Transform, matrix: &Matrix) -> Transform {
    let pdf_transform =
        Transform::from_row(matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f);
    base.pre_concat(pdf_transform)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_options_default() {
        let opts = RenderOptions::default();
        assert_eq!(opts.dpi, 150);
        assert_eq!(opts.format, ImageFormat::Png);
        assert!(opts.background.is_some());
    }

    #[test]
    fn test_render_options_with_dpi() {
        let opts = RenderOptions::with_dpi(300);
        assert_eq!(opts.dpi, 300);
    }

    #[test]
    fn test_cmyk_to_rgb() {
        let (r, g, b) = cmyk_to_rgb(0.0, 0.0, 0.0, 0.0);
        assert!((r - 1.0).abs() < 0.01);
        assert!((g - 1.0).abs() < 0.01);
        assert!((b - 1.0).abs() < 0.01);

        let (r, g, b) = cmyk_to_rgb(0.0, 0.0, 0.0, 1.0);
        assert!((r - 0.0).abs() < 0.01);
        assert!((g - 0.0).abs() < 0.01);
        assert!((b - 0.0).abs() < 0.01);
    }
}
