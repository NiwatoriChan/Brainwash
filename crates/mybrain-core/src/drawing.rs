//! Core domain models and logic for Noteful notebooks in Brainwash.
//!
//! Reverse-engineered to match Noteful iOS (NotesStore & NotesKit) feature-for-feature,
//! including multi-layer vector inking, study tape active-recall concealment,
//! auto-shape detection, and multi-template page backgrounds.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

/// Unique identifier for documents, pages, layers, and elements.
pub type ElementId = String;

/// Standard paper orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaperOrientation {
    Portrait,
    Landscape,
}

impl Default for PaperOrientation {
    fn default() -> Self {
        Self::Portrait
    }
}

/// Paper color presets matching Noteful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaperColor {
    White,
    Cream,
    Dark,
    Sepia,
}

impl Default for PaperColor {
    fn default() -> Self {
        Self::White
    }
}

/// Built-in template patterns matching Noteful's `templates.bundle`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackgroundTemplate {
    Blank,
    Ruled { spacing: f32, margin: bool },
    Grid { spacing: f32 },
    Dotted { spacing: f32 },
    Cornell { summary_height: f32, cue_width: f32 },
    TwoColumn { divider: f32 },
    ThreeColumn,
    MusicStaff { staves: usize },
    Storyboard { frames: usize },
}

impl Default for BackgroundTemplate {
    fn default() -> Self {
        Self::Ruled { spacing: 28.0, margin: true }
    }
}

/// Inking tool type for a vector stroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrokeToolKind {
    Pen,
    Highlighter,
}

impl Default for StrokeToolKind {
    fn default() -> Self {
        Self::Pen
    }
}

/// Pen tip dynamic style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PenType {
    Ballpoint,
    Fountain,
    Brush,
}

impl Default for PenType {
    fn default() -> Self {
        Self::Fountain
    }
}

/// An individual sampled point in a vector stroke.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StrokePoint {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub timestamp_ms: u64,
}

impl StrokePoint {
    pub fn new(x: f32, y: f32, pressure: f32, timestamp_ms: u64) -> Self {
        Self { x, y, pressure: pressure.clamp(0.05, 2.0), timestamp_ms }
    }
}

/// RGBA 8-bit color definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbaColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RgbaColor {
    pub const BLACK: Self = Self { r: 18, g: 18, b: 20, a: 255 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255, a: 255 };
    pub const BLUE: Self = Self { r: 37, g: 99, b: 235, a: 255 };
    pub const RED: Self = Self { r: 220, g: 38, b: 38, a: 255 };
    pub const GREEN: Self = Self { r: 22, g: 163, b: 74, a: 255 };
    pub const YELLOW_HIGHLIGHT: Self = Self { r: 250, g: 204, b: 21, a: 110 };
    pub const GREEN_HIGHLIGHT: Self = Self { r: 74, g: 222, b: 128, a: 110 };
    pub const BLUE_HIGHLIGHT: Self = Self { r: 96, g: 165, b: 250, a: 110 };
    pub const ORANGE_HIGHLIGHT: Self = Self { r: 251, g: 146, b: 60, a: 110 };
    pub const PURPLE_HIGHLIGHT: Self = Self { r: 192, g: 132, b: 252, a: 110 };
    pub const TAPE_YELLOW: Self = Self { r: 253, g: 224, b: 71, a: 235 };
    pub const TAPE_BLUE: Self = Self { r: 147, g: 197, b: 253, a: 235 };
    pub const TAPE_PINK: Self = Self { r: 244, g: 114, b: 182, a: 235 };
    pub const TAPE_GREEN: Self = Self { r: 134, g: 239, b: 172, a: 235 };

    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
    }
}

/// A smooth vector stroke.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorStroke {
    pub id: ElementId,
    pub tool: StrokeToolKind,
    pub pen_type: PenType,
    pub points: Vec<StrokePoint>,
    pub color: RgbaColor,
    pub base_width: f32,
    #[serde(default)]
    pub is_straight: bool,
}

impl VectorStroke {
    pub fn new(tool: StrokeToolKind, color: RgbaColor, base_width: f32) -> Self {
        Self {
            id: generate_id("stroke"),
            tool,
            pen_type: PenType::Fountain,
            points: Vec::new(),
            color,
            base_width,
            is_straight: false,
        }
    }

    pub fn bounding_box(&self) -> Option<[f32; 4]> {
        if self.points.is_empty() {
            return None;
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let pad = self.base_width * 1.5;

        for p in &self.points {
            min_x = min_x.min(p.x - pad);
            min_y = min_y.min(p.y - pad);
            max_x = max_x.max(p.x + pad);
            max_y = max_y.max(p.y + pad);
        }
        Some([min_x, min_y, max_x, max_y])
    }
}

/// Noteful Study Tape element for active recall and revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotefulTape {
    pub id: ElementId,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: RgbaColor,
    /// When revealed is false, the tape is opaque/concealed. When revealed is true, it is translucent/transparent.
    pub revealed: bool,
    #[serde(default)]
    pub label: Option<String>,
}

impl NotefulTape {
    pub fn new(x: f32, y: f32, width: f32, height: f32, color: RgbaColor) -> Self {
        Self {
            id: generate_id("tape"),
            x,
            y,
            width: width.max(16.0),
            height: height.max(12.0),
            color,
            revealed: false,
            label: None,
        }
    }

    pub fn toggle(&mut self) {
        self.revealed = !self.revealed;
    }

    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.width && py >= self.y && py <= self.y + self.height
    }

    pub fn bounds(&self) -> [f32; 4] {
        [self.x, self.y, self.x + self.width, self.y + self.height]
    }
}

/// Geometric shape kinds supported by Noteful auto-shape recognition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShapeKind {
    Line { from: [f32; 2], to: [f32; 2], arrow: bool },
    Rectangle { x: f32, y: f32, width: f32, height: f32, corner_radius: f32 },
    Ellipse { cx: f32, cy: f32, rx: f32, ry: f32 },
    Polygon { points: Vec<[f32; 2]> },
}

/// Vector shape element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotefulShape {
    pub id: ElementId,
    pub kind: ShapeKind,
    pub stroke_color: RgbaColor,
    pub fill_color: Option<RgbaColor>,
    pub stroke_width: f32,
}

impl NotefulShape {
    pub fn new(kind: ShapeKind, stroke_color: RgbaColor, stroke_width: f32) -> Self {
        Self {
            id: generate_id("shape"),
            kind,
            stroke_color,
            fill_color: None,
            stroke_width,
        }
    }

    pub fn bounds(&self) -> [f32; 4] {
        match &self.kind {
            ShapeKind::Line { from, to, .. } => [
                from[0].min(to[0]) - self.stroke_width,
                from[1].min(to[1]) - self.stroke_width,
                from[0].max(to[0]) + self.stroke_width,
                from[1].max(to[1]) + self.stroke_width,
            ],
            ShapeKind::Rectangle { x, y, width, height, .. } => {
                [*x, *y, *x + *width, *y + *height]
            }
            ShapeKind::Ellipse { cx, cy, rx, ry } => {
                [*cx - *rx, *cy - *ry, *cx + *rx, *cy + *ry]
            }
            ShapeKind::Polygon { points } => {
                if points.is_empty() {
                    return [0.0, 0.0, 0.0, 0.0];
                }
                let mut min_x = f32::INFINITY;
                let mut min_y = f32::INFINITY;
                let mut max_x = f32::NEG_INFINITY;
                let mut max_y = f32::NEG_INFINITY;
                for p in points {
                    min_x = min_x.min(p[0]);
                    min_y = min_y.min(p[1]);
                    max_x = max_x.max(p[0]);
                    max_y = max_y.max(p[1]);
                }
                [min_x, min_y, max_x, max_y]
            }
        }
    }
}

/// Freeform rich text box element placed on a page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotefulTextBox {
    pub id: ElementId,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub text: String,
    pub font_size: f32,
    pub text_color: RgbaColor,
    pub bg_color: Option<RgbaColor>,
    pub has_border: bool,
}

impl NotefulTextBox {
    pub fn new(x: f32, y: f32, text: impl Into<String>) -> Self {
        Self {
            id: generate_id("text"),
            x,
            y,
            width: 220.0,
            height: 60.0,
            text: text.into(),
            font_size: 16.0,
            text_color: RgbaColor::BLACK,
            bg_color: None,
            has_border: false,
        }
    }

    pub fn bounds(&self) -> [f32; 4] {
        [self.x, self.y, self.x + self.width, self.y + self.height]
    }
}

/// Multi-layer canvas layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotefulLayer {
    pub id: ElementId,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
    pub strokes: Vec<VectorStroke>,
    pub tapes: Vec<NotefulTape>,
    pub shapes: Vec<NotefulShape>,
    pub text_boxes: Vec<NotefulTextBox>,
}

impl NotefulLayer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: generate_id("layer"),
            name: name.into(),
            visible: true,
            locked: false,
            opacity: 1.0,
            strokes: Vec::new(),
            tapes: Vec::new(),
            shapes: Vec::new(),
            text_boxes: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.strokes.is_empty() && self.tapes.is_empty() && self.shapes.is_empty() && self.text_boxes.is_empty()
    }

    pub fn clear(&mut self) {
        self.strokes.clear();
        self.tapes.clear();
        self.shapes.clear();
        self.text_boxes.clear();
    }
}

/// Background PDF page reference for annotating slides, documents, or textbooks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdfBackground {
    /// File path to the source PDF (relative to vault or absolute)
    pub pdf_path: String,
    /// 1-based page number in the source PDF
    pub page_number: usize,
    /// Total pages in the source PDF
    pub total_pages: usize,
    /// Display title or file name
    pub title: String,
    /// Native PDF point dimensions [width, height] (e.g. [595.0, 842.0] for A4)
    #[serde(default)]
    pub original_size: [f32; 2],
}

impl PdfBackground {
    pub fn new(pdf_path: impl Into<String>, page_number: usize, total_pages: usize, width: f32, height: f32) -> Self {
        let path_str = pdf_path.into();
        let name = path_str
            .split('/')
            .last()
            .unwrap_or(&path_str)
            .trim_end_matches(".pdf")
            .to_string();
        Self {
            pdf_path: path_str,
            page_number,
            total_pages,
            title: name,
            original_size: [width, height],
        }
    }
}

/// An individual page within a Noteful notebook.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotefulPage {
    pub id: ElementId,
    pub width: f32,
    pub height: f32,
    pub orientation: PaperOrientation,
    pub paper_color: PaperColor,
    pub template: BackgroundTemplate,
    pub layers: Vec<NotefulLayer>,
    pub active_layer_id: ElementId,
    #[serde(default)]
    pub bookmarked: bool,
    #[serde(default)]
    pub pdf_background: Option<PdfBackground>,
}

impl Default for NotefulPage {
    fn default() -> Self {
        Self::new(595.0, 842.0) // A4 standard point size
    }
}

impl NotefulPage {
    pub fn new(width: f32, height: f32) -> Self {
        let base_layer = NotefulLayer::new("Layer 1");
        let active_id = base_layer.id.clone();
        Self {
            id: generate_id("page"),
            width,
            height,
            orientation: PaperOrientation::Portrait,
            paper_color: PaperColor::White,
            template: BackgroundTemplate::default(),
            layers: vec![base_layer],
            active_layer_id: active_id,
            bookmarked: false,
            pdf_background: None,
        }
    }

    pub fn new_pdf_page(pdf_path: &str, page_number: usize, total_pages: usize, width: f32, height: f32) -> Self {
        let mut page = Self::new(width, height);
        page.orientation = if width > height {
            PaperOrientation::Landscape
        } else {
            PaperOrientation::Portrait
        };
        page.template = BackgroundTemplate::Blank;
        page.pdf_background = Some(PdfBackground::new(pdf_path, page_number, total_pages, width, height));
        page
    }

    pub fn has_pdf(&self) -> bool {
        self.pdf_background.is_some()
    }

    /// Returns the active layer mutably, or the first unlocked/visible layer if found.
    pub fn active_layer_mut(&mut self) -> Option<&mut NotefulLayer> {
        let target_id = self.active_layer_id.clone();
        if let Some(pos) = self.layers.iter().position(|l| l.id == target_id) {
            return Some(&mut self.layers[pos]);
        }
        self.layers.first_mut()
    }

    pub fn add_layer(&mut self, name: impl Into<String>) -> ElementId {
        let layer = NotefulLayer::new(name);
        let id = layer.id.clone();
        self.layers.push(layer);
        self.active_layer_id = id.clone();
        id
    }

    pub fn merge_down(&mut self, layer_idx: usize) -> bool {
        if layer_idx == 0 || layer_idx >= self.layers.len() {
            return false;
        }
        let src = self.layers.remove(layer_idx);
        let dst = &mut self.layers[layer_idx - 1];
        dst.strokes.extend(src.strokes);
        dst.tapes.extend(src.tapes);
        dst.shapes.extend(src.shapes);
        dst.text_boxes.extend(src.text_boxes);
        self.active_layer_id = dst.id.clone();
        true
    }

    pub fn cover_all_tapes(&mut self) {
        for l in &mut self.layers {
            for t in &mut l.tapes {
                t.revealed = false;
            }
        }
    }

    pub fn reveal_all_tapes(&mut self) {
        for l in &mut self.layers {
            for t in &mut l.tapes {
                t.revealed = true;
            }
        }
    }
}

/// Outline / Table of contents entry for quick document navigation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotefulOutlineItem {
    pub id: ElementId,
    pub title: String,
    pub page_index: usize,
    pub level: usize,
}

/// Complete Noteful Notebook document model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotefulDocument {
    pub id: ElementId,
    pub title: String,
    pub pages: Vec<NotefulPage>,
    pub outlines: Vec<NotefulOutlineItem>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: i64,
    pub modified_at: i64,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl Default for NotefulDocument {
    fn default() -> Self {
        Self::new("Untitled Notebook")
    }
}

impl NotefulDocument {
    pub fn new(title: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: generate_uuid_hex(),
            title: title.into(),
            pages: vec![NotefulPage::default()],
            outlines: Vec::new(),
            tags: Vec::new(),
            created_at: now,
            modified_at: now,
            metadata: BTreeMap::new(),
        }
    }

    /// Create a new drawing notebook by importing an entire PDF document.
    /// Creates one drawing page for each page in the PDF.
    pub fn create_from_pdf(title: &str, pdf_path: &str, page_count: usize, page_size: [f32; 2]) -> Self {
        let mut doc = Self::new(title);
        doc.pages.clear();
        let total = page_count.max(1);
        for page_num in 1..=total {
            doc.pages.push(NotefulPage::new_pdf_page(
                pdf_path,
                page_num,
                total,
                page_size[0],
                page_size[1],
            ));
        }
        doc
    }

    /// Import pages from a PDF document into the existing notebook.
    pub fn append_pdf_pages(&mut self, pdf_path: &str, page_count: usize, page_size: [f32; 2]) -> usize {
        let start_idx = self.pages.len();
        let total = page_count.max(1);
        for page_num in 1..=total {
            self.pages.push(NotefulPage::new_pdf_page(
                pdf_path,
                page_num,
                total,
                page_size[0],
                page_size[1],
            ));
        }
        self.touch();
        start_idx
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn add_page(&mut self) -> usize {
        let mut new_page = NotefulPage::default();
        if let Some(last) = self.pages.last() {
            new_page.width = last.width;
            new_page.height = last.height;
            new_page.orientation = last.orientation;
            new_page.paper_color = last.paper_color;
            new_page.template = last.template.clone();
        }
        self.pages.push(new_page);
        self.touch();
        self.pages.len() - 1
    }

    pub fn duplicate_page(&mut self, page_idx: usize) -> Option<usize> {
        if page_idx >= self.pages.len() {
            return None;
        }
        let mut clone = self.pages[page_idx].clone();
        clone.id = generate_id("page");
        for l in &mut clone.layers {
            l.id = generate_id("layer");
        }
        let insert_idx = page_idx + 1;
        self.pages.insert(insert_idx, clone);
        self.touch();
        Some(insert_idx)
    }

    pub fn delete_page(&mut self, page_idx: usize) -> bool {
        if self.pages.len() <= 1 || page_idx >= self.pages.len() {
            return false;
        }
        self.pages.remove(page_idx);
        self.touch();
        true
    }

    pub fn touch(&mut self) {
        self.modified_at = chrono::Utc::now().timestamp();
    }

    pub fn cover_all_tapes(&mut self) {
        for p in &mut self.pages {
            p.cover_all_tapes();
        }
    }

    pub fn reveal_all_tapes(&mut self) {
        for p in &mut self.pages {
            p.reveal_all_tapes();
        }
    }
}

use std::sync::atomic::{AtomicU64, Ordering};
static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a unique 32-character uppercase hex UUID matching Noteful format.
pub fn generate_uuid_hex() -> String {
    let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let count = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:016X}{:016X}", now as u64, count)
}

fn generate_id(prefix: &str) -> String {
    let count = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let now = chrono::Utc::now().timestamp_micros();
    format!("{prefix}_{now:x}_{count:x}")
}

// ---------------------------------------------------------------------------
// Shape Fitting / Auto-Shape Detection
// ---------------------------------------------------------------------------

/// Recognize whether a raw vector stroke forms a recognized shape (Line, Box, Circle, Arrow)
/// when drawn and held stationary at the finish.
pub fn recognize_stroke_shape(points: &[StrokePoint], stroke_color: RgbaColor, stroke_width: f32) -> Option<NotefulShape> {
    if points.len() < 5 {
        return None;
    }
    let p_start = points[0];
    let p_end = points[points.len() - 1];

    let dx = p_end.x - p_start.x;
    let dy = p_end.y - p_start.y;
    let direct_dist = (dx * dx + dy * dy).sqrt();

    // Compute total path length
    let mut path_len = 0.0;
    for i in 1..points.len() {
        let seg_dx = points[i].x - points[i - 1].x;
        let seg_dy = points[i].y - points[i - 1].y;
        path_len += (seg_dx * seg_dx + seg_dy * seg_dy).sqrt();
    }

    if path_len < 12.0 {
        return None;
    }

    // 1. Straight Line Check: direct distance is very close to path length
    if direct_dist / path_len > 0.88 {
        return Some(NotefulShape::new(
            ShapeKind::Line {
                from: [p_start.x, p_start.y],
                to: [p_end.x, p_end.y],
                arrow: false,
            },
            stroke_color,
            stroke_width,
        ));
    }

    // 2. Closed Loop Check (p_start close to p_end)
    let closure_dist = direct_dist;
    let is_closed = closure_dist < path_len * 0.25 || closure_dist < 40.0;

    if is_closed {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for p in points {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }

        let w = max_x - min_x;
        let h = max_y - min_y;

        if w > 15.0 && h > 15.0 {
            let cx = (min_x + max_x) * 0.5;
            let cy = (min_y + max_y) * 0.5;
            let rx = w * 0.5;
            let ry = h * 0.5;

            // Check if aspect ratio is close to circle or ellipse
            let aspect = rx / ry;
            if (0.75..=1.33).contains(&aspect) {
                let r = (rx + ry) * 0.5;
                return Some(NotefulShape::new(
                    ShapeKind::Ellipse { cx, cy, rx: r, ry: r },
                    stroke_color,
                    stroke_width,
                ));
            } else {
                return Some(NotefulShape::new(
                    ShapeKind::Rectangle { x: min_x, y: min_y, width: w, height: h, corner_radius: 4.0 },
                    stroke_color,
                    stroke_width,
                ));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noteful_document_creation_and_layers() {
        let mut doc = NotefulDocument::new("Biology 101");
        assert_eq!(doc.page_count(), 1);
        let page = &mut doc.pages[0];
        assert_eq!(page.layers.len(), 1);

        let layer2_id = page.add_layer("Layer 2 - Annotations");
        assert_eq!(page.layers.len(), 2);
        assert_eq!(page.active_layer_id, layer2_id);

        let stroke = VectorStroke::new(StrokeToolKind::Pen, RgbaColor::BLUE, 2.5);
        page.active_layer_mut().unwrap().strokes.push(stroke);
        assert_eq!(page.layers[1].strokes.len(), 1);
        assert_eq!(page.layers[0].strokes.len(), 0);

        // Test merge down
        assert!(page.merge_down(1));
        assert_eq!(page.layers.len(), 1);
        assert_eq!(page.layers[0].strokes.len(), 1);
    }

    #[test]
    fn test_study_tape_active_recall_toggles() {
        let mut doc = NotefulDocument::new("Medical Terms");
        let page = &mut doc.pages[0];
        let layer = page.active_layer_mut().unwrap();

        layer.tapes.push(NotefulTape::new(50.0, 100.0, 120.0, 24.0, RgbaColor::TAPE_YELLOW));
        layer.tapes.push(NotefulTape::new(50.0, 140.0, 120.0, 24.0, RgbaColor::TAPE_BLUE));

        assert!(!layer.tapes[0].revealed);
        assert!(!layer.tapes[1].revealed);

        // Tap toggle
        layer.tapes[0].toggle();
        assert!(layer.tapes[0].revealed);

        // Reveal all
        doc.reveal_all_tapes();
        assert!(doc.pages[0].layers[0].tapes[0].revealed);
        assert!(doc.pages[0].layers[0].tapes[1].revealed);

        // Cover all
        doc.cover_all_tapes();
        assert!(!doc.pages[0].layers[0].tapes[0].revealed);
        assert!(!doc.pages[0].layers[0].tapes[1].revealed);
    }

    #[test]
    fn test_auto_shape_straight_line() {
        let mut pts = Vec::new();
        for i in 0..15 {
            let x = 10.0 + (i as f32) * 10.0;
            let y = 20.0 + (i as f32) * 5.0;
            pts.push(StrokePoint::new(x, y, 1.0, i * 10));
        }

        let shape = recognize_stroke_shape(&pts, RgbaColor::BLACK, 2.0);
        assert!(shape.is_some());
        if let Some(s) = shape {
            match s.kind {
                ShapeKind::Line { from, to, arrow } => {
                    assert_eq!(from, [10.0, 20.0]);
                    assert_eq!(to, [150.0, 90.0]);
                    assert!(!arrow);
                }
                _ => panic!("Expected line shape"),
            }
        }
    }

    #[test]
    fn test_pdf_import_and_page_background() {
        let mut doc = NotefulDocument::create_from_pdf("Lecture Slides", "slides.pdf", 3, [960.0, 540.0]);
        assert_eq!(doc.pages.len(), 3);
        assert_eq!(doc.pages[0].width, 960.0);
        assert_eq!(doc.pages[0].height, 540.0);
        assert_eq!(doc.pages[0].orientation, PaperOrientation::Landscape);
        assert!(doc.pages[0].has_pdf());

        let bg = doc.pages[0].pdf_background.as_ref().unwrap();
        assert_eq!(bg.page_number, 1);
        assert_eq!(bg.total_pages, 3);
        assert_eq!(bg.pdf_path, "slides.pdf");

        // Append more pages
        doc.append_pdf_pages("appendix.pdf", 2, [595.0, 842.0]);
        assert_eq!(doc.pages.len(), 5);
        assert_eq!(doc.pages[3].pdf_background.as_ref().unwrap().pdf_path, "appendix.pdf");
        assert_eq!(doc.pages[3].pdf_background.as_ref().unwrap().page_number, 1);
        assert_eq!(doc.pages[4].pdf_background.as_ref().unwrap().page_number, 2);
    }
}
