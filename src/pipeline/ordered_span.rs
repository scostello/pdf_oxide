//! Ordered text spans for output conversion.
//!
//! This module provides the OrderedTextSpan type which wraps TextSpan
//! with reading order information.

use crate::layout::TextSpan;

/// Source of reading order assignment.
///
/// Tracks which strategy/method determined the reading order for a span.
/// This follows the SpaceSource pattern for consistency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadingOrderSource {
    /// Order from PDF structure tree (Tagged PDF).
    ///
    /// Confidence: 1.0 (explicit PDF semantic markup per ISO 32000-1:2008 Section 14.7).
    StructureTree,
    /// Order from XY-Cut recursive partitioning.
    ///
    /// Confidence: 0.90 (robust for multi-column layouts).
    XYCut,
    /// Order from geometric column analysis.
    ///
    /// Confidence: 0.85 (good for standard column layouts).
    Geometric,
    /// Order from simple top-to-bottom, left-to-right.
    ///
    /// Confidence: 0.75 (basic, works for single-column).
    #[default]
    Simple,
    /// Order explicitly set by user/API.
    ///
    /// Confidence: 1.0 (explicit assignment).
    UserAssigned,
    /// Fallback order (e.g., untagged spans in mixed document).
    ///
    /// Confidence: 0.65 (best-effort).
    Fallback,
}

impl ReadingOrderSource {
    /// Get the default confidence for this source type.
    pub fn default_confidence(&self) -> f32 {
        match self {
            ReadingOrderSource::StructureTree => 1.0,
            ReadingOrderSource::XYCut => 0.90,
            ReadingOrderSource::Geometric => 0.85,
            ReadingOrderSource::Simple => 0.75,
            ReadingOrderSource::UserAssigned => 1.0,
            ReadingOrderSource::Fallback => 0.65,
        }
    }

    /// Get strategy name for debugging.
    pub fn name(&self) -> &'static str {
        match self {
            ReadingOrderSource::StructureTree => "StructureTree",
            ReadingOrderSource::XYCut => "XYCut",
            ReadingOrderSource::Geometric => "Geometric",
            ReadingOrderSource::Simple => "Simple",
            ReadingOrderSource::UserAssigned => "UserAssigned",
            ReadingOrderSource::Fallback => "Fallback",
        }
    }
}

/// Reading order metadata for a span.
///
/// Contains the source and confidence of the reading order assignment,
/// following the SpaceDecision pattern.
#[derive(Debug, Clone, Default)]
pub struct ReadingOrderInfo {
    /// Which strategy assigned this reading order.
    pub source: ReadingOrderSource,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
}

impl ReadingOrderInfo {
    /// Create with source and default confidence.
    pub fn from_source(source: ReadingOrderSource) -> Self {
        Self {
            confidence: source.default_confidence(),
            source,
        }
    }

    /// Create with explicit confidence.
    pub fn with_confidence(source: ReadingOrderSource, confidence: f32) -> Self {
        Self {
            source,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Create for structure tree source.
    pub fn structure_tree() -> Self {
        Self::from_source(ReadingOrderSource::StructureTree)
    }

    /// Create for XY-Cut source.
    pub fn xycut() -> Self {
        Self::from_source(ReadingOrderSource::XYCut)
    }

    /// Create for geometric source.
    pub fn geometric() -> Self {
        Self::from_source(ReadingOrderSource::Geometric)
    }

    /// Create for simple source.
    pub fn simple() -> Self {
        Self::from_source(ReadingOrderSource::Simple)
    }

    /// Create for fallback (untagged in mixed doc).
    pub fn fallback() -> Self {
        Self::from_source(ReadingOrderSource::Fallback)
    }
}

/// A text span with an assigned reading order index.
///
/// This wrapper adds ordering information to TextSpan without modifying
/// the original span data. The reading_order field represents the position
/// in the final document output (0 = first to be read).
#[derive(Debug, Clone)]
pub struct OrderedTextSpan {
    /// The underlying text span.
    pub span: TextSpan,

    /// Index in reading order (0 = first to be read).
    pub reading_order: usize,

    /// Group ID for paragraph/section grouping (optional).
    pub group_id: Option<usize>,

    /// Reading order source and confidence information.
    pub order_info: ReadingOrderInfo,
}

impl OrderedTextSpan {
    /// Create a new ordered span with the given reading order.
    /// Uses Simple source as default for backward compatibility.
    pub fn new(span: TextSpan, reading_order: usize) -> Self {
        Self {
            span,
            reading_order,
            group_id: None,
            order_info: ReadingOrderInfo::default(),
        }
    }

    /// Create with explicit source info.
    pub fn with_info(span: TextSpan, reading_order: usize, order_info: ReadingOrderInfo) -> Self {
        Self {
            span,
            reading_order,
            group_id: None,
            order_info,
        }
    }

    /// Set the group ID for paragraph grouping.
    pub fn with_group(mut self, group_id: usize) -> Self {
        self.group_id = Some(group_id);
        self
    }

    /// Set the reading order info.
    pub fn with_order_info(mut self, order_info: ReadingOrderInfo) -> Self {
        self.order_info = order_info;
        self
    }

    /// Get the reading order source.
    pub fn source(&self) -> ReadingOrderSource {
        self.order_info.source
    }

    /// Get the reading order confidence.
    pub fn confidence(&self) -> f32 {
        self.order_info.confidence
    }
}

/// A collection of ordered spans with helper methods.
pub struct OrderedSpans {
    spans: Vec<OrderedTextSpan>,
}

impl OrderedSpans {
    /// Create a new collection from a vector of ordered spans.
    pub fn new(spans: Vec<OrderedTextSpan>) -> Self {
        Self { spans }
    }

    /// Get the number of spans.
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// Get spans sorted by reading order.
    pub fn in_reading_order(&self) -> Vec<&OrderedTextSpan> {
        let mut sorted: Vec<_> = self.spans.iter().collect();
        sorted.sort_by_key(|s| s.reading_order);
        sorted
    }

    /// Get the underlying spans.
    pub fn spans(&self) -> &[OrderedTextSpan] {
        &self.spans
    }

    /// Convert to a vector of ordered spans.
    pub fn into_vec(self) -> Vec<OrderedTextSpan> {
        self.spans
    }

    /// Group spans into lines based on Y-coordinate proximity.
    ///
    /// Returns groups of spans that appear on the same line.
    pub fn group_into_lines(&self, tolerance: f32) -> Vec<Vec<&OrderedTextSpan>> {
        if self.spans.is_empty() {
            return Vec::new();
        }

        let mut sorted: Vec<_> = self.spans.iter().collect();
        sorted.sort_by(|a, b| {
            b.span
                .bbox
                .y
                .partial_cmp(&a.span.bbox.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut lines: Vec<Vec<&OrderedTextSpan>> = Vec::new();
        let mut current_line: Vec<&OrderedTextSpan> = vec![sorted[0]];
        let mut current_y = sorted[0].span.bbox.y;

        for span in sorted.into_iter().skip(1) {
            if (current_y - span.span.bbox.y).abs() <= tolerance {
                current_line.push(span);
            } else {
                lines.push(std::mem::take(&mut current_line));
                current_line = vec![span];
                current_y = span.span.bbox.y;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }
}

impl From<Vec<OrderedTextSpan>> for OrderedSpans {
    fn from(spans: Vec<OrderedTextSpan>) -> Self {
        Self::new(spans)
    }
}

impl IntoIterator for OrderedSpans {
    type Item = OrderedTextSpan;
    type IntoIter = std::vec::IntoIter<OrderedTextSpan>;

    fn into_iter(self) -> Self::IntoIter {
        self.spans.into_iter()
    }
}
