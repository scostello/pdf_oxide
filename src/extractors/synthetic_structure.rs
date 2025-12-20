//! Synthetic structure generation for untagged PDFs.
//!
//! This module creates hierarchical document structures for PDFs that lack
//! explicit structure trees (untagged PDFs) using geometric analysis and clustering.
//!
//! The synthetic structure uses:
//! - **Document** as root
//! - **Sections** detected via heading sizes
//! - **Paragraphs** grouped by vertical proximity
//! - **Individual content** (text, images) as leaf elements

use crate::elements::{ContentElement, StructureElement};
use crate::error::Result;
use crate::geometry::Rect;

/// Configuration for synthetic structure generation.
#[derive(Debug, Clone)]
pub struct SyntheticStructureConfig {
    /// Vertical gap threshold for grouping into paragraphs (in points)
    pub paragraph_gap_threshold: f32,

    /// Font size threshold multiplier for heading detection
    /// (heading if size > avg_size * multiplier)
    pub heading_size_multiplier: f32,

    /// Minimum vertical distance to start a new section
    pub section_break_threshold: f32,
}

impl Default for SyntheticStructureConfig {
    fn default() -> Self {
        Self {
            paragraph_gap_threshold: 4.0,  // ~4 points
            heading_size_multiplier: 1.3,  // 30% larger than average
            section_break_threshold: 50.0, // ~50 points
        }
    }
}

/// Generates synthetic hierarchical structure for untagged PDFs.
pub struct SyntheticStructureGenerator {
    config: SyntheticStructureConfig,
}

impl SyntheticStructureGenerator {
    /// Create a new synthetic structure generator with default configuration.
    pub fn new() -> Self {
        Self {
            config: SyntheticStructureConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: SyntheticStructureConfig) -> Self {
        Self { config }
    }

    /// Generate synthetic document structure.
    ///
    /// # Arguments
    ///
    /// * `content_elements` - Extracted page content in reading order
    /// * `page_bbox` - Bounding box of the page
    ///
    /// # Returns
    ///
    /// A StructureElement with synthetic Document hierarchy
    ///
    /// # Algorithm
    ///
    /// 1. Analyze content positioning and styling
    /// 2. Detect section breaks based on large gaps
    /// 3. Detect headings based on font size
    /// 4. Group paragraphs based on proximity
    /// 5. Build hierarchical structure: Document → Sections → Paragraphs → Content
    pub fn generate(
        &self,
        content_elements: &[ContentElement],
        page_bbox: Rect,
    ) -> Result<StructureElement> {
        if content_elements.is_empty() {
            return Ok(StructureElement {
                structure_type: "Document".to_string(),
                bbox: page_bbox,
                children: Vec::new(),
                reading_order: Some(0),
                alt_text: None,
                language: None,
            });
        }

        // Step 1: Group content into paragraphs based on proximity
        let paragraphs = self.group_into_paragraphs(content_elements);

        // Step 2: Detect headings and section breaks
        let sections = self.group_into_sections(&paragraphs);

        // Step 3: Build hierarchical structure
        let children = sections
            .into_iter()
            .map(ContentElement::Structure)
            .collect();

        Ok(StructureElement {
            structure_type: "Document".to_string(),
            bbox: page_bbox,
            children,
            reading_order: Some(0),
            alt_text: None,
            language: None,
        })
    }

    /// Group content elements into paragraphs based on vertical proximity.
    fn group_into_paragraphs(&self, elements: &[ContentElement]) -> Vec<StructureElement> {
        let mut paragraphs = Vec::new();
        let mut current_paragraph: Vec<ContentElement> = Vec::new();
        let mut last_y_end = f32::MAX;

        for element in elements {
            let bbox = element.bbox();

            // Calculate gap from last element
            let gap = if last_y_end != f32::MAX {
                (last_y_end - bbox.y).abs()
            } else {
                0.0
            };

            // If gap is too large, start a new paragraph
            if gap > self.config.paragraph_gap_threshold && !current_paragraph.is_empty() {
                paragraphs.push(self.create_paragraph(std::mem::take(&mut current_paragraph)));
            }

            current_paragraph.push(element.clone());
            last_y_end = bbox.y;
        }

        // Add final paragraph
        if !current_paragraph.is_empty() {
            paragraphs.push(self.create_paragraph(current_paragraph));
        }

        paragraphs
    }

    /// Group paragraphs into sections based on heading detection.
    fn group_into_sections(&self, paragraphs: &[StructureElement]) -> Vec<StructureElement> {
        let mut sections = Vec::new();
        let mut current_section: Vec<StructureElement> = Vec::new();

        for paragraph in paragraphs {
            // Check if this paragraph represents a heading
            if self.is_heading_paragraph(paragraph) {
                // Start new section
                if !current_section.is_empty() {
                    sections.push(self.create_section(std::mem::take(&mut current_section)));
                }
                // Add heading as first element of new section
                current_section.push(paragraph.clone());
            } else {
                current_section.push(paragraph.clone());
            }
        }

        // Add final section
        if !current_section.is_empty() {
            sections.push(self.create_section(current_section));
        }

        sections
    }

    /// Create a paragraph structure element.
    fn create_paragraph(&self, children: Vec<ContentElement>) -> StructureElement {
        let bbox = Self::calculate_bbox(&children);

        StructureElement {
            structure_type: "P".to_string(),
            bbox,
            children,
            reading_order: None,
            alt_text: None,
            language: None,
        }
    }

    /// Create a section structure element.
    fn create_section(&self, children: Vec<StructureElement>) -> StructureElement {
        let bbox =
            Self::calculate_struct_bbox(&children.iter().map(|s| s.bbox).collect::<Vec<_>>());

        let children_as_content: Vec<ContentElement> = children
            .into_iter()
            .map(ContentElement::Structure)
            .collect();

        StructureElement {
            structure_type: "Sect".to_string(),
            bbox,
            children: children_as_content,
            reading_order: None,
            alt_text: None,
            language: None,
        }
    }

    /// Check if a paragraph represents a heading.
    ///
    /// A simple heuristic: if the paragraph contains only one text element
    /// with significantly larger font size than others, it's a heading.
    fn is_heading_paragraph(&self, _paragraph: &StructureElement) -> bool {
        // This is a placeholder - full implementation would analyze text styling
        // For now, return false (all paragraphs are treated as body text)
        false
    }

    /// Calculate bounding box from content elements.
    fn calculate_bbox(elements: &[ContentElement]) -> Rect {
        if elements.is_empty() {
            return Rect::new(0.0, 0.0, 0.0, 0.0);
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for element in elements {
            let bbox = element.bbox();
            min_x = min_x.min(bbox.x);
            min_y = min_y.min(bbox.y);
            max_x = max_x.max(bbox.x + bbox.width);
            max_y = max_y.max(bbox.y + bbox.height);
        }

        if min_x == f32::MAX {
            Rect::new(0.0, 0.0, 0.0, 0.0)
        } else {
            Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
        }
    }

    /// Calculate bounding box from structure element rects.
    fn calculate_struct_bbox(rects: &[Rect]) -> Rect {
        if rects.is_empty() {
            return Rect::new(0.0, 0.0, 0.0, 0.0);
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for bbox in rects {
            min_x = min_x.min(bbox.x);
            min_y = min_y.min(bbox.y);
            max_x = max_x.max(bbox.x + bbox.width);
            max_y = max_y.max(bbox.y + bbox.height);
        }

        if min_x == f32::MAX {
            Rect::new(0.0, 0.0, 0.0, 0.0)
        } else {
            Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
        }
    }
}

impl Default for SyntheticStructureGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_creation() {
        let _generator = SyntheticStructureGenerator::new();
    }

    #[test]
    fn test_empty_content() {
        let generator = SyntheticStructureGenerator::new();
        let page_bbox = Rect::new(0.0, 0.0, 595.0, 842.0);
        let result = generator.generate(&[], page_bbox).unwrap();

        assert_eq!(result.structure_type, "Document");
        assert!(result.children.is_empty());
    }

    #[test]
    fn test_config_defaults() {
        let config = SyntheticStructureConfig::default();
        assert_eq!(config.paragraph_gap_threshold, 4.0);
        assert_eq!(config.heading_size_multiplier, 1.3);
        assert_eq!(config.section_break_threshold, 50.0);
    }

    #[test]
    fn test_calculate_bbox_empty() {
        let bbox = SyntheticStructureGenerator::calculate_bbox(&[]);
        assert_eq!(bbox.x, 0.0);
        assert_eq!(bbox.y, 0.0);
        assert_eq!(bbox.width, 0.0);
        assert_eq!(bbox.height, 0.0);
    }
}
