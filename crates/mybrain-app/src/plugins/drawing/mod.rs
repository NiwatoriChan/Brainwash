//! Noteful Notes & PDF Annotation plugin for Brainwash.
//!
//! A feature-parity recreation of Noteful iOS reverse-engineered using REA:
//! - Multi-layer vector inking (unlimited layers, opacity, locking, hide/show, merge down).
//! - Vector pen with Catmull-Rom curve smoothing and pressure-responsive width.
//! - Translucent highlighter rendering behind ink.
//! - Study Tape active-recall tool (tap to reveal/conceal, Cover All, Reveal All).
//! - Whole stroke and segment eraser with highlighter-only filtering.
//! - Auto-shape detection on draw-and-hold (lines, rectangles, arrows, circles).
//! - Lasso selection (move, scale, duplicate, recolor, move between layers).
//! - Rich paper templates (Ruled, Grid, Dotted, Cornell, Staff, Storyboard).
//! - Night Mode (intelligent color inversion) and Sepia paper tints.
//! - Multi-page navigation, page organizer, and native `.noteful` format support.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, FontId, Frame, Margin,
    PointerButton, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2,
};
use mybrain_core::events_bus::AppEvent;
use mybrain_core::drawing::{
    recognize_stroke_shape, BackgroundTemplate, NotefulDocument, NotefulShape, NotefulTape,
    NotefulTextBox, PaperColor, PaperOrientation, PdfBackground, PenType, RgbaColor, ShapeKind, StrokePoint,
    StrokeToolKind, VectorStroke,
};

use crate::plugins::{
    MyBrainPlugin, PluginCommand, PluginContext, PluginViewType, RibbonButton, RibbonSlot,
};
use crate::theme::{self, Tokens};

/// Currently selected tool in the Noteful palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTool {
    Pen,
    Highlighter,
    Eraser,
    Tape,
    Lasso,
    Shape,
    Text,
    Laser,
    Hand,
}

/// Eraser operating mode matching Noteful.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EraserMode {
    WholeStroke,
    Segment,
}

/// Action performed when the stylus barrel / secondary button is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StylusButtonAction {
    QuickEraser,
    QuickLasso,
}

/// Shape kind selected in the shape tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeToolKind {
    Line,
    Arrow,
    Rectangle,
    Ellipse,
}

/// A glowing point in the laser pointer presentation trail.
#[derive(Debug, Clone)]
struct LaserPoint {
    pos: Pos2,
    created: Instant,
}

pub struct DrawingPlugin {
    pub active_path: Option<String>,
    pub open_tabs: Vec<String>,
    pub doc: NotefulDocument,
    pub active_page_idx: usize,
    pub dirty: bool,

    // Canvas navigation
    pub pan: Vec2,
    pub zoom: f32,

    // Active tool state
    pub active_tool: ActiveTool,
    pub pen_color: RgbaColor,
    pub pen_width: f32,
    pub pen_type: PenType,
    pub pen_auto_shape: bool,

    pub highlighter_color: RgbaColor,
    pub highlighter_width: f32,
    pub highlighter_straight: bool,

    pub eraser_mode: EraserMode,
    pub eraser_size: f32,
    pub erase_highlight_only: bool,
    pub erase_tapes: bool,
    pub erase_shapes: bool,

    pub tape_color: RgbaColor,
    pub tape_height: f32,

    pub shape_kind: ShapeToolKind,
    pub shape_color: RgbaColor,
    pub shape_width: f32,

    pub text_font_size: f32,
    pub text_color: RgbaColor,

    // Wacom / Stylus hardware pressure and dynamics
    pub stylus_pressure: Option<f32>,
    pub stylus_barrel_action: StylusButtonAction,
    last_point_time: Option<Instant>,

    // In-progress stroke
    current_stroke_points: Vec<StrokePoint>,
    stroke_start_time: Option<Instant>,
    last_point_pos: Option<Pos2>,
    hold_start: Option<Instant>,

    // Laser trail
    laser_trail: Vec<LaserPoint>,

    // Lasso selection
    lasso_polygon: Vec<Pos2>,
    lasso_active: bool,
    selected_stroke_ids: Vec<String>,
    selected_tape_ids: Vec<String>,
    selected_shape_ids: Vec<String>,
    selected_text_ids: Vec<String>,
    selection_drag_start: Option<Pos2>,

    // Drawers & Modals
    pub night_mode: bool,
    pub show_layers_drawer: bool,
    pub show_pages_drawer: bool,
    pub show_new_notebook_dialog: bool,
    pub show_export_dialog: bool,
    pub export_include_bg: bool,
    pub export_include_annotations: bool,
    pub export_include_tapes: bool,

    // New notebook form
    pub new_title: String,
    pub new_template_idx: usize,
    pub new_color_idx: usize,
    pub new_orientation_idx: usize,

    // PDF Import & Texture Cache
    pub show_import_pdf_dialog: bool,
    pub import_pdf_path: String,
    pub import_pdf_append_to_current: bool,
    pub pdf_texture_cache: HashMap<(String, usize), egui::TextureHandle>,
}

pub type NotefulPlugin = DrawingPlugin;

impl Default for DrawingPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl DrawingPlugin {
    pub fn new() -> Self {
        Self {
            active_path: None,
            open_tabs: Vec::new(),
            doc: NotefulDocument::new("Drawing Document"),
            active_page_idx: 0,
            dirty: false,

            pan: Vec2::ZERO,
            zoom: 1.0,

            active_tool: ActiveTool::Pen,
            pen_color: RgbaColor::BLUE,
            pen_width: 2.5,
            pen_type: PenType::Fountain,
            pen_auto_shape: true,

            highlighter_color: RgbaColor::YELLOW_HIGHLIGHT,
            highlighter_width: 18.0,
            highlighter_straight: false,

            eraser_mode: EraserMode::WholeStroke,
            eraser_size: 24.0,
            erase_highlight_only: false,
            erase_tapes: true,
            erase_shapes: true,

            tape_color: RgbaColor::TAPE_YELLOW,
            tape_height: 24.0,

            shape_kind: ShapeToolKind::Rectangle,
            shape_color: RgbaColor::BLACK,
            shape_width: 2.0,

            text_font_size: 16.0,
            text_color: RgbaColor::BLACK,

            stylus_pressure: None,
            stylus_barrel_action: StylusButtonAction::QuickEraser,
            last_point_time: None,

            current_stroke_points: Vec::new(),
            stroke_start_time: None,
            last_point_pos: None,
            hold_start: None,

            laser_trail: Vec::new(),

            lasso_polygon: Vec::new(),
            lasso_active: false,
            selected_stroke_ids: Vec::new(),
            selected_tape_ids: Vec::new(),
            selected_shape_ids: Vec::new(),
            selected_text_ids: Vec::new(),
            selection_drag_start: None,

            night_mode: false,
            show_layers_drawer: false,
            show_pages_drawer: false,
            show_new_notebook_dialog: false,
            show_export_dialog: false,
            export_include_bg: true,
            export_include_annotations: true,
            export_include_tapes: true,

            new_title: "Untitled Notebook".to_string(),
            new_template_idx: 1, // Ruled by default
            new_color_idx: 0,    // White
            new_orientation_idx: 0, // Portrait

            show_import_pdf_dialog: false,
            import_pdf_path: String::new(),
            import_pdf_append_to_current: false,
            pdf_texture_cache: HashMap::new(),
        }
    }

    /// Calculate effective stroke pressure from hardware stylus force, velocity fallback, and pen type.
    pub fn compute_pressure(
        &self,
        hardware_force: Option<f32>,
        speed_px_per_ms: Option<f32>,
        tool: ActiveTool,
        pen_type: PenType,
    ) -> f32 {
        let normalized = match hardware_force {
            Some(f) => f.clamp(0.0, 1.0),
            None => {
                // Fallback to speed-based dynamic pressure for mouse/trackpad
                if let Some(speed) = speed_px_per_ms {
                    (1.15 - (speed / 2.5) * 0.55).clamp(0.25, 1.0)
                } else {
                    1.0
                }
            }
        };

        match tool {
            ActiveTool::Highlighter => {
                // Highlighters have mostly uniform flow with subtle touch response
                (0.92 + 0.16 * normalized).clamp(0.1, 2.0)
            }
            ActiveTool::Pen => match pen_type {
                PenType::Ballpoint => {
                    // Firm, consistent line with subtle pressure variation (0.80..=1.20)
                    (0.80 + 0.40 * normalized).clamp(0.1, 2.0)
                }
                PenType::Fountain => {
                    // Responsive calligraphy nib: delicate at light touch, rich at full press (0.35..=1.65)
                    (0.35 + 1.30 * normalized.powf(1.2)).clamp(0.1, 2.0)
                }
                PenType::Brush => {
                    // Broad dynamic range from fine hairline to broad stroke (0.15..=1.85)
                    (0.15 + 1.70 * normalized).clamp(0.05, 2.0)
                }
            },
            _ => 1.0,
        }
    }

    /// Erase vector elements at the specified page position using current eraser settings.
    pub fn erase_at_point(&mut self, page_idx: usize, erase_center: Pos2) {
        let page = match self.doc.pages.get_mut(page_idx) {
            Some(p) => p,
            None => return,
        };

        let erase_radius = self.eraser_size;
        let highlight_only = self.erase_highlight_only;

        if let Some(layer) = page.active_layer_mut() {
            if !layer.locked {
                // 1. Erase strokes
                match self.eraser_mode {
                    EraserMode::WholeStroke => {
                        let prev_len = layer.strokes.len();
                        layer.strokes.retain(|s| {
                            if highlight_only && s.tool != StrokeToolKind::Highlighter {
                                return true;
                            }
                            !s.points.iter().any(|pt| {
                                let dx = pt.x - erase_center.x;
                                let dy = pt.y - erase_center.y;
                                (dx * dx + dy * dy).sqrt() <= erase_radius
                            })
                        });
                        if layer.strokes.len() != prev_len {
                            self.dirty = true;
                        }
                    }
                    EraserMode::Segment => {
                        for s in &mut layer.strokes {
                            if highlight_only && s.tool != StrokeToolKind::Highlighter {
                                continue;
                            }
                            s.points.retain(|pt| {
                                let dx = pt.x - erase_center.x;
                                let dy = pt.y - erase_center.y;
                                (dx * dx + dy * dy).sqrt() > erase_radius
                            });
                        }
                        layer.strokes.retain(|s| s.points.len() >= 2);
                        self.dirty = true;
                    }
                }

                // 2. Erase tapes if enabled
                if self.erase_tapes {
                    let prev_tapes = layer.tapes.len();
                    layer.tapes.retain(|t| !t.contains_point(erase_center.x, erase_center.y));
                    if layer.tapes.len() != prev_tapes {
                        self.dirty = true;
                    }
                }

                // 3. Erase shapes if enabled
                if self.erase_shapes {
                    let prev_shapes = layer.shapes.len();
                    layer.shapes.retain(|sh| {
                        let b = sh.bounds();
                        !(erase_center.x >= b[0] && erase_center.x <= b[2] && erase_center.y >= b[1] && erase_center.y <= b[3])
                    });
                    if layer.shapes.len() != prev_shapes {
                        self.dirty = true;
                    }
                }
            }
        }
    }

    pub fn open_notebook(&mut self, rel_path: &str, ctx: &mut PluginContext) {
        match ctx.vault.load_drawing(rel_path) {
            Ok(doc) => {
                self.active_path = Some(rel_path.to_owned());
                if !self.open_tabs.contains(&rel_path.to_string()) {
                    self.open_tabs.push(rel_path.to_string());
                }
                self.doc = doc;
                self.active_page_idx = 0;
                self.dirty = false;
                self.clear_selection();
                ctx.activate_plugin("core.drawing");
                ctx.toast(format!("Opened drawing: {}", self.doc.title));
            }
            Err(e) => ctx.toast(format!("Failed to open drawing: {e}")),
        }
    }

    pub fn save_notebook(&mut self, ctx: &mut PluginContext) {
        if let Some(path) = &self.active_path {
            if let Err(e) = ctx.vault.save_drawing(path, &self.doc) {
                ctx.toast(format!("Failed to save drawing: {e}"));
            } else {
                self.dirty = false;
                ctx.toast(format!("Saved {}", self.doc.title));
            }
        }
    }

    pub fn create_notebook(&mut self, title: &str, template: BackgroundTemplate, color: PaperColor, orient: PaperOrientation, ctx: &mut PluginContext) -> Result<String, mybrain_core::Error> {
        let clean = title.trim();
        let safe_name = if clean.is_empty() { "Untitled" } else { clean };
        let rel_path = format!("Notebooks/{safe_name}.drawing");

        let mut doc = NotefulDocument::new(safe_name);
        if let Some(page) = doc.pages.first_mut() {
            page.template = template;
            page.paper_color = color;
            page.orientation = orient;
            if orient == PaperOrientation::Landscape {
                std::mem::swap(&mut page.width, &mut page.height);
            }
        }

        ctx.vault.save_drawing(&rel_path, &doc)?;
        self.open_notebook(&rel_path, ctx);
        Ok(rel_path)
    }

    pub fn clear_selection(&mut self) {
        self.lasso_polygon.clear();
        self.lasso_active = false;
        self.selected_stroke_ids.clear();
        self.selected_tape_ids.clear();
        self.selected_shape_ids.clear();
        self.selected_text_ids.clear();
        self.selection_drag_start = None;
    }

    pub fn has_selection(&self) -> bool {
        !self.selected_stroke_ids.is_empty()
            || !self.selected_tape_ids.is_empty()
            || !self.selected_shape_ids.is_empty()
            || !self.selected_text_ids.is_empty()
    }

    pub fn tab_label(&self) -> String {
        if let Some(ref path) = self.active_path {
            let name = path.rsplit('/').next().unwrap_or(path);
            let name = name.trim_end_matches(".drawing").trim_end_matches(".noteful").trim_end_matches(".json");
            format!("🎨 {}", name)
        } else {
            "🎨 Drawing Document".to_string()
        }
    }

    pub fn toggle_study_tapes(&mut self, reveal: bool) {
        if reveal {
            self.doc.reveal_all_tapes();
        } else {
            self.doc.cover_all_tapes();
        }
        self.dirty = true;
    }
}

impl MyBrainPlugin for DrawingPlugin {
    fn id(&self) -> &'static str {
        "core.drawing"
    }

    fn name(&self) -> &'static str {
        "Drawing Documents"
    }

    fn description(&self) -> &'static str {
        "Vector drawing documents with smooth inking, study tape active-recall, shapes, and layers."
    }

    fn view_type(&self) -> PluginViewType {
        PluginViewType::MainWorkspace
    }

    fn ribbon_button(&self) -> Option<RibbonButton> {
        Some(RibbonButton {
            icon: "🎨",
            tooltip: "Drawing Documents (Handwriting, Shapes, Layers & Study Tape)",
            slot: RibbonSlot::Top,
            view_type: PluginViewType::MainWorkspace,
            priority: 25,
        })
    }

    fn on_disable(&mut self, ctx: &mut PluginContext) {
        if self.dirty {
            self.save_notebook(ctx);
        }
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![
            PluginCommand {
                id: "drawing.open".into(),
                title: "Open Drawing Document View".into(),
                shortcut: Some("Ctrl+4".into()),
                plugin_id: "core.drawing",
            },
            PluginCommand {
                id: "drawing.new".into(),
                title: "New Drawing Document".into(),
                shortcut: Some("Ctrl+Shift+D".into()),
                plugin_id: "core.drawing",
            },
            PluginCommand {
                id: "drawing.cover_tapes".into(),
                title: "Drawing: Cover All Study Tapes".into(),
                shortcut: None,
                plugin_id: "core.drawing",
            },
            PluginCommand {
                id: "drawing.reveal_tapes".into(),
                title: "Drawing: Reveal All Study Tapes".into(),
                shortcut: None,
                plugin_id: "core.drawing",
            },
            PluginCommand {
                id: "drawing.toggle_layers".into(),
                title: "Drawing: Toggle Layers Drawer".into(),
                shortcut: Some("Ctrl+L".into()),
                plugin_id: "core.drawing",
            },
            PluginCommand {
                id: "drawing.toggle_night_mode".into(),
                title: "Drawing: Toggle Night Mode".into(),
                shortcut: None,
                plugin_id: "core.drawing",
            },
            PluginCommand {
                id: "drawing.import_pdf".into(),
                title: "Drawing: Import PDF Document".into(),
                shortcut: Some("Ctrl+Shift+P".into()),
                plugin_id: "core.drawing",
            },
        ]
    }

    fn execute_command(&mut self, command_id: &str, ctx: &mut PluginContext) {
        match command_id {
            "drawing.open" | "noteful.open" => ctx.activate_plugin("core.drawing"),
            "drawing.new" | "noteful.new" => {
                self.show_new_notebook_dialog = true;
                ctx.activate_plugin("core.drawing");
            }
            "drawing.import_pdf" | "noteful.import_pdf" => {
                self.show_import_pdf_dialog = true;
                ctx.activate_plugin("core.drawing");
            }
            "drawing.cover_tapes" | "noteful.cover_tapes" => {
                self.doc.cover_all_tapes();
                self.dirty = true;
                ctx.toast("Concealed all study tapes");
            }
            "drawing.reveal_tapes" | "noteful.reveal_tapes" => {
                self.doc.reveal_all_tapes();
                self.dirty = true;
                ctx.toast("Revealed all study tapes");
            }
            "drawing.toggle_layers" | "noteful.toggle_layers" => {
                self.show_layers_drawer = !self.show_layers_drawer;
            }
            "drawing.toggle_night_mode" | "noteful.toggle_night_mode" => {
                self.night_mode = !self.night_mode;
                ctx.toast(if self.night_mode { "Night Mode enabled" } else { "Day Mode enabled" });
            }
            _ => {}
        }
    }

    fn on_event(&mut self, event: &AppEvent, ctx: &mut PluginContext) {
        match event {
            AppEvent::RequestOpenDrawing(path) | AppEvent::RequestOpenNoteful(path) => {
                self.open_notebook(path, ctx);
            }
            _ => {}
        }
    }

    fn render_main_view(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        let t = Tokens::get(ui.ctx());

        let panel_max_rect = ui.max_rect();
        ui.set_max_width(panel_max_rect.width());
        ui.set_clip_rect(panel_max_rect.intersect(ui.clip_rect()));

        // Check if there is an active document
        if self.active_path.is_none() {
            if let Some(first) = ctx.vault.list_drawing_files().first().cloned() {
                self.open_notebook(&first, ctx);
            } else {
                // Top Navigation & Tabs Bar
                self.render_tab_bar(ui, &t, ctx);
                theme::bottom_hairline(ui, ui.max_rect(), &t);

                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(80.0);
                        ui.label(RichText::new("🎨").size(48.0));
                        ui.add_space(12.0);
                        ui.heading(RichText::new("Drawing Documents").size(22.0).color(t.text));
                        ui.add_space(8.0);
                        ui.label(RichText::new("Vector inking, study tape active-recall, auto-shapes, and multi-layer handwriting.").color(t.text_dim).size(13.5));
                        ui.add_space(20.0);
                        if ui.add(
                            egui::Button::new(RichText::new("＋ Create New Drawing Document").strong().color(Color32::WHITE))
                                .fill(t.accent)
                                .stroke(Stroke::NONE)
                                .corner_radius(CornerRadius::same(t.radius_sm as u8))
                        ).clicked() {
                            self.show_new_notebook_dialog = true;
                        }

                        ui.add_space(8.0);
                        if ui.add(
                            egui::Button::new(RichText::new("📄 Import PDF Document").size(13.0).color(t.text))
                                .corner_radius(CornerRadius::same(t.radius_sm as u8))
                        ).clicked() {
                            self.show_import_pdf_dialog = true;
                        }
                    });
                });

                if self.show_new_notebook_dialog {
                    self.render_new_notebook_modal(ui.ctx(), &t, ctx);
                }
                if self.show_import_pdf_dialog {
                    self.render_import_pdf_modal(ui.ctx(), &t, ctx);
                }
                return;
            }
        }

        // Top Navigation & Tabs Bar
        self.render_tab_bar(ui, &t, ctx);
        theme::bottom_hairline(ui, ui.max_rect(), &t);

        // Main Tool Palette (Floating / Sticky toolbar)
        self.render_tool_palette(ui, &t, ctx);
        theme::bottom_hairline(ui, ui.max_rect(), &t);

        // Central Canvas & Layers Area with exact space partitioning
        let total_w = ui.available_width();
        let total_h = ui.available_height();

        let drawer_w: f32 = if self.show_layers_drawer { 230.0 } else { 0.0 };
        let sep_w: f32 = if self.show_layers_drawer { 1.0 } else { 0.0 };
        let canvas_w = (total_w - drawer_w - sep_w).max(100.0);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            // Interactive drawing canvas with exact constrained size
            ui.allocate_ui_with_layout(
                vec2(canvas_w, total_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_max_size(vec2(canvas_w, total_h));
                    ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
                    self.render_canvas_area(ui, &t, ctx);
                },
            );

            // Optional Layers drawer on right side
            if self.show_layers_drawer {
                let (sep_rect, _) = ui.allocate_exact_size(vec2(sep_w, total_h), Sense::hover());
                ui.painter().vline(sep_rect.center().x, sep_rect.y_range(), Stroke::new(1.0, t.separator));

                ui.allocate_ui_with_layout(
                    vec2(drawer_w, total_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_max_size(vec2(drawer_w, total_h));
                        ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
                        self.render_layers_drawer(ui, &t, ctx);
                    },
                );
            }
        });

        // Modals
        if self.show_new_notebook_dialog {
            self.render_new_notebook_modal(ui.ctx(), &t, ctx);
        }
        if self.show_export_dialog {
            self.render_export_modal(ui.ctx(), &t, ctx);
        }
        if self.show_import_pdf_dialog {
            self.render_import_pdf_modal(ui.ctx(), &t, ctx);
        }
    }
}

impl DrawingPlugin {
    /// Render notebook tabs bar at top of workspace
    fn render_tab_bar(&mut self, ui: &mut Ui, t: &Tokens, ctx: &mut PluginContext) {
        Frame::NONE
            .fill(t.dock)
            .stroke(Stroke::new(1.0, t.separator))
            .inner_margin(Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;

                    // New Notebook Quick Button
                    let new_btn = ui.add(
                        egui::Button::new(RichText::new("＋ New Drawing").size(12.0).color(t.accent_text))
                            .fill(t.accent)
                            .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                    );
                    if new_btn.on_hover_text("Create a new Drawing Document").clicked() {
                        self.show_new_notebook_dialog = true;
                    }

                    // Import PDF Quick Button
                    let import_pdf_btn = ui.add(
                        egui::Button::new(RichText::new("📄 Import PDF").size(12.0).color(t.text))
                            .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                    );
                    if import_pdf_btn.on_hover_text("Import a PDF as a new notebook or append to current document").clicked() {
                        self.show_import_pdf_dialog = true;
                    }

                    ui.separator();

                    // Open Notebook Tabs
                    let mut tab_to_close: Option<String> = None;
                    let mut tab_to_select: Option<String> = None;

                    for tab_path in &self.open_tabs {
                        let is_active = self.active_path.as_deref() == Some(tab_path.as_str());
                        let name = tab_path.rsplit('/').next().unwrap_or(tab_path)
                            .trim_end_matches(".drawing")
                            .trim_end_matches(".noteful")
                            .trim_end_matches(".json");

                        let tab_frame = Frame::NONE
                            .fill(if is_active { t.card } else { t.dock })
                            .stroke(if is_active { Stroke::new(1.0, t.accent_border) } else { Stroke::new(1.0, t.separator) })
                            .corner_radius(CornerRadius::same(t.radius_sm as u8))
                            .inner_margin(Margin::symmetric(8, 4));

                        tab_frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let label = ui.add(
                                    egui::Label::new(RichText::new(format!("🎨 {name}")).size(12.0).color(if is_active { t.text } else { t.text_dim }))
                                        .sense(Sense::click()),
                                );
                                if label.clicked() {
                                    tab_to_select = Some(tab_path.clone());
                                }

                                if self.open_tabs.len() > 1 && theme::icon_close_button(ui, t).clicked() {
                                    tab_to_close = Some(tab_path.clone());
                                }
                            });
                        });
                    }

                    if let Some(sel) = tab_to_select {
                        self.open_notebook(&sel, ctx);
                    }
                    if let Some(cls) = tab_to_close {
                        self.open_tabs.retain(|p| p != &cls);
                        if self.active_path.as_deref() == Some(&cls) {
                            if let Some(next) = self.open_tabs.first().cloned() {
                                self.open_notebook(&next, ctx);
                            } else {
                                self.active_path = None;
                            }
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Export button
                        if ui.add(egui::Button::new(RichText::new("📤 Export").size(12.0))).clicked() {
                            self.show_export_dialog = true;
                        }

                        // Night Mode toggle
                        let night_label = if self.night_mode { "🌙 Night" } else { "☀️ Day" };
                        if ui.add(egui::Button::new(RichText::new(night_label).size(12.0))).clicked() {
                            self.night_mode = !self.night_mode;
                        }

                        // Save status
                        if self.dirty {
                            let save_btn = ui.add(
                                egui::Button::new(RichText::new("💾 Save*").size(12.0).color(t.warning))
                                    .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                            );
                            if save_btn.clicked() {
                                self.save_notebook(ctx);
                            }
                        }
                    });
                });
            });
    }

    /// Render Noteful toolbar (Pen, Highlighter, Eraser, Tape, Lasso, Shapes, Text, Laser, Hand, Presets)
    fn render_tool_palette(&mut self, ui: &mut Ui, t: &Tokens, ctx: &mut PluginContext) {
        Frame::NONE
            .fill(t.card)
            .stroke(Stroke::new(1.0, t.card_border))
            .inner_margin(Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;

                    let tools = [
                        (ActiveTool::Pen, "✍ Pen", "Vector Pen with smooth Catmull-Rom inking"),
                        (ActiveTool::Highlighter, "🖍 Highl.", "Translucent highlighter behind ink"),
                        (ActiveTool::Eraser, "Eraser", "Stroke / Segment eraser with filters"),
                        (ActiveTool::Tape, "🩹 Tape", "Active Recall Study Tape (tap to reveal)"),
                        (ActiveTool::Lasso, "✂ Lasso", "Select, move, duplicate, and recolor elements"),
                        (ActiveTool::Shape, "🔷 Shape", "Lines, rectangles, and ellipses with snapping"),
                        (ActiveTool::Text, "🔤 Text", "Place customizable text boxes on canvas"),
                        (ActiveTool::Laser, "🔴 Laser", "Presentation laser pointer with fading trail"),
                        (ActiveTool::Hand, "✋ Pan", "Pan and zoom around canvas"),
                    ];

                    for (tool, label, tip) in tools {
                        let is_active = self.active_tool == tool;
                        let btn = ui.add(
                            egui::Button::new(RichText::new(label).size(12.0).color(if is_active { t.accent_text } else { t.text }))
                                .fill(if is_active { t.accent } else { Color32::TRANSPARENT })
                                .stroke(if is_active { Stroke::NONE } else { Stroke::new(1.0, t.separator) })
                                .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                        );
                        if btn.on_hover_text(tip).clicked() {
                            self.active_tool = tool;
                            if tool != ActiveTool::Lasso {
                                self.clear_selection();
                            }
                        }
                    }

                    ui.separator();

                    // Tool-specific parameter controls
                    match self.active_tool {
                        ActiveTool::Pen => {
                            // Pen Color presets
                            let colors = [
                                (RgbaColor::BLACK, "#121214", "Black"),
                                (RgbaColor::BLUE, "#2563eb", "Blue"),
                                (RgbaColor::RED, "#dc2626", "Red"),
                                (RgbaColor::GREEN, "#16a34a", "Green"),
                                (RgbaColor::new(147, 51, 234, 255), "#9333ea", "Purple"),
                            ];
                            for (c, _, name) in colors {
                                let c32 = Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a);
                                let is_sel = self.pen_color == c;
                                let (rect, resp) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::click());
                                ui.painter().circle_filled(rect.center(), 7.0, c32);
                                if is_sel {
                                    ui.painter().circle_stroke(rect.center(), 8.5, Stroke::new(1.5, t.text));
                                }
                                if resp.on_hover_text(name).clicked() {
                                    self.pen_color = c;
                                }
                            }

                            ui.separator();
                            ui.label(RichText::new("Size:").size(11.0).color(t.text_dim));
                            ui.add(egui::Slider::new(&mut self.pen_width, 1.0..=12.0).text("pt").show_value(false));

                            ui.separator();
                            let pen_types = [
                                (PenType::Fountain, "✒ Fountain", "Expressive calligraphy nib with dynamic pressure"),
                                (PenType::Ballpoint, "🖊 Ballpoint", "Uniform, firm ink line with subtle pressure"),
                                (PenType::Brush, "🖌 Brush", "Broad dynamic range from fine hair to rich stroke"),
                            ];
                            for (pt, name, desc) in pen_types {
                                let is_active = self.pen_type == pt;
                                let btn = ui.add(
                                    egui::Button::new(RichText::new(name).size(11.0).color(if is_active { t.accent_text } else { t.text }))
                                        .fill(if is_active { t.accent } else { Color32::TRANSPARENT })
                                        .stroke(if is_active { Stroke::NONE } else { Stroke::new(1.0, t.separator) })
                                        .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                                );
                                if btn.on_hover_text(desc).clicked() {
                                    self.pen_type = pt;
                                }
                            }

                            ui.separator();
                            let auto_label = if self.pen_auto_shape { "⚡ Auto-Shape: On" } else { "Auto-Shape: Off" };
                            if ui.add(egui::Button::new(RichText::new(auto_label).size(11.0))).clicked() {
                                self.pen_auto_shape = !self.pen_auto_shape;
                            }

                            if self.stylus_pressure.is_some() {
                                ui.label(RichText::new("✍ Stylus Active").size(11.0).color(t.accent).strong());
                            }
                        }
                        ActiveTool::Highlighter => {
                            let colors = [
                                (RgbaColor::YELLOW_HIGHLIGHT, "Yellow"),
                                (RgbaColor::GREEN_HIGHLIGHT, "Green"),
                                (RgbaColor::BLUE_HIGHLIGHT, "Blue"),
                                (RgbaColor::ORANGE_HIGHLIGHT, "Orange"),
                                (RgbaColor::PURPLE_HIGHLIGHT, "Purple"),
                            ];
                            for (c, name) in colors {
                                let c32 = Color32::from_rgb(c.r, c.g, c.b);
                                let is_sel = self.highlighter_color == c;
                                let (rect, resp) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::click());
                                ui.painter().circle_filled(rect.center(), 7.0, c32);
                                if is_sel {
                                    ui.painter().circle_stroke(rect.center(), 8.5, Stroke::new(1.5, t.text));
                                }
                                if resp.on_hover_text(name).clicked() {
                                    self.highlighter_color = c;
                                }
                            }
                            ui.separator();
                            ui.label(RichText::new("Width:").size(11.0).color(t.text_dim));
                            ui.add(egui::Slider::new(&mut self.highlighter_width, 8.0..=36.0).show_value(false));

                            let straight_txt = if self.highlighter_straight { "Straight Line: ON" } else { "Straight Line: OFF" };
                            if ui.add(egui::Button::new(RichText::new(straight_txt).size(11.0))).clicked() {
                                self.highlighter_straight = !self.highlighter_straight;
                            }
                        }
                        ActiveTool::Eraser => {
                            let mode_txt = match self.eraser_mode {
                                EraserMode::WholeStroke => "Whole Stroke",
                                EraserMode::Segment => "Segment",
                            };
                            if ui.add(egui::Button::new(RichText::new(mode_txt).size(11.0))).on_hover_text("Switch between Whole Stroke and Segment eraser").clicked() {
                                self.eraser_mode = match self.eraser_mode {
                                    EraserMode::WholeStroke => EraserMode::Segment,
                                    EraserMode::Segment => EraserMode::WholeStroke,
                                };
                            }

                            let hl_txt = if self.erase_highlight_only { "Highlight Only: ON" } else { "Highlight Only: OFF" };
                            if ui.add(egui::Button::new(RichText::new(hl_txt).size(11.0))).clicked() {
                                self.erase_highlight_only = !self.erase_highlight_only;
                            }
                        }
                        ActiveTool::Tape => {
                            let tape_colors = [
                                (RgbaColor::TAPE_YELLOW, "Yellow Tape"),
                                (RgbaColor::TAPE_BLUE, "Blue Tape"),
                                (RgbaColor::TAPE_PINK, "Pink Tape"),
                                (RgbaColor::TAPE_GREEN, "Green Tape"),
                            ];
                            for (c, name) in tape_colors {
                                let c32 = Color32::from_rgb(c.r, c.g, c.b);
                                let is_sel = self.tape_color == c;
                                let (rect, resp) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::click());
                                ui.painter().rect_filled(rect, CornerRadius::same(2), c32);
                                if is_sel {
                                    ui.painter().rect_stroke(rect, CornerRadius::same(2), Stroke::new(1.5, t.text), egui::StrokeKind::Outside);
                                }
                                if resp.on_hover_text(name).clicked() {
                                    self.tape_color = c;
                                }
                            }

                            ui.separator();
                            // Quick Study Mode Batch Buttons
                            if ui.add(egui::Button::new(RichText::new("🙈 Cover All").size(11.0))).on_hover_text("Conceal all study tapes for quiz testing").clicked() {
                                self.doc.cover_all_tapes();
                                self.dirty = true;
                                ctx.toast("Concealed all study tapes");
                            }
                            if ui.add(egui::Button::new(RichText::new("👀 Reveal All").size(11.0))).on_hover_text("Reveal all study tapes").clicked() {
                                self.doc.reveal_all_tapes();
                                self.dirty = true;
                                ctx.toast("Revealed all study tapes");
                            }
                        }
                        ActiveTool::Shape => {
                            let shape_options = [
                                (ShapeToolKind::Rectangle, "▭ Box"),
                                (ShapeToolKind::Ellipse, "◯ Oval"),
                                (ShapeToolKind::Line, "╱ Line"),
                                (ShapeToolKind::Arrow, "➔ Arrow"),
                            ];
                            for (k, lbl) in shape_options {
                                let sel = self.shape_kind == k;
                                if ui.add(egui::Button::new(RichText::new(lbl).size(11.0)).fill(if sel { t.accent_soft } else { Color32::TRANSPARENT })).clicked() {
                                    self.shape_kind = k;
                                }
                            }
                        }
                        _ => {}
                    }

                    // Right End: Layers Drawer Toggle, Page Navigation & Zoom Reset
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Layers Drawer Toggle
                        let layer_count = self.doc.pages.get(self.active_page_idx).map(|p| p.layers.len()).unwrap_or(1);
                        let layer_btn = ui.add(
                            egui::Button::new(RichText::new(format!("Layers ({layer_count})")).size(12.0).color(if self.show_layers_drawer { t.accent_text } else { t.text }))
                                .fill(if self.show_layers_drawer { t.accent } else { t.field })
                                .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                        );
                        if layer_btn.clicked() {
                            self.show_layers_drawer = !self.show_layers_drawer;
                        }

                        // Zoom reset button
                        ui.separator();
                        let zoom_pct = (self.zoom * 100.0).round() as i32;
                        if ui.add(egui::Button::new(RichText::new(format!("{zoom_pct}%")).size(11.0))).on_hover_text("Reset Zoom & Pan to 100%").clicked() {
                            self.zoom = 1.0;
                            self.pan = Vec2::ZERO;
                        }

                        // Page Jumper Controls
                        ui.separator();
                        let total_pages = self.doc.page_count();
                        let cur_page = self.active_page_idx + 1;

                        if ui.add_enabled(self.active_page_idx + 1 < total_pages, egui::Button::new("▶")).clicked() {
                            self.active_page_idx += 1;
                        }
                        ui.label(RichText::new(format!("{cur_page} / {total_pages}")).size(12.0));
                        if ui.add_enabled(self.active_page_idx > 0, egui::Button::new("◀")).clicked() {
                            self.active_page_idx = self.active_page_idx.saturating_sub(1);
                        }

                        if ui.add(egui::Button::new(RichText::new("＋ Page").size(12.0))).on_hover_text("Insert new page with matching template").clicked() {
                            self.active_page_idx = self.doc.add_page();
                            self.dirty = true;
                        }

                        let add_pdf_btn = ui.add(egui::Button::new(RichText::new("📄+").size(12.0)));
                        if add_pdf_btn.on_hover_text("Import & append PDF pages to this document").clicked() {
                            self.import_pdf_append_to_current = true;
                            self.show_import_pdf_dialog = true;
                        }

                        if let Some(pdf_bg) = self.doc.pages.get(self.active_page_idx).and_then(|p| p.pdf_background.as_ref()) {
                            ui.label(
                                RichText::new(format!("📄 {} (P. {}/{})", pdf_bg.title, pdf_bg.page_number, pdf_bg.total_pages))
                                    .size(11.0)
                                    .color(t.accent),
                            );
                        }

                        if total_pages > 1
                            && ui.add(egui::Button::new(RichText::new("🗑").size(11.0).color(t.danger))).on_hover_text("Delete current page").clicked()
                        {
                            self.doc.pages.remove(self.active_page_idx);
                            self.active_page_idx = self.active_page_idx.min(self.doc.pages.len().saturating_sub(1));
                            self.dirty = true;
                        }
                    });
                });
            });
    }

    /// Render the interactive vector inking and study tape canvas area
    fn render_canvas_area(&mut self, ui: &mut Ui, t: &Tokens, ctx: &mut PluginContext) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        // Capture Wacom tablet stylus pressure and touch contacts from raw events
        let raw_events = ui.input(|i| i.raw.events.clone());
        for event in &raw_events {
            if let egui::Event::Touch { force, phase, .. } = event {
                match phase {
                    egui::TouchPhase::Start | egui::TouchPhase::Move => {
                        if let Some(f) = force {
                            self.stylus_pressure = Some(*f);
                        }
                    }
                    egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                        self.stylus_pressure = None;
                    }
                }
            }
        }

        // Fill background behind paper
        painter.rect_filled(rect, CornerRadius::ZERO, if self.night_mode { Color32::from_rgb(14, 14, 16) } else { t.canvas });

        // User Pointer Input Handling & Smooth Navigation
        let pointer = ui.input(|i| i.pointer.clone());
        let space_down = ui.input(|i| i.key_down(egui::Key::Space));
        let is_panning_shortcut = pointer.button_down(PointerButton::Middle)
            || (pointer.button_down(PointerButton::Primary) && space_down);

        if is_panning_shortcut {
            self.pan += ui.input(|i| i.pointer.delta());
        }

        // Mouse wheel scroll & zoom: Ctrl+Scroll zooms, normal scroll pans
        if response.hovered() {
            let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
            let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
            let scroll_x = ui.input(|i| i.smooth_scroll_delta.x);

            if ctrl {
                if scroll_y.abs() > 0.0 {
                    let zoom_delta = (scroll_y * 0.002).exp();
                    let old_zoom = self.zoom;
                    self.zoom = (self.zoom * zoom_delta).clamp(0.25, 4.0);
                    if let Some(mouse) = response.hover_pos() {
                        let offset = mouse - (rect.center() + self.pan);
                        self.pan -= offset * (self.zoom / old_zoom - 1.0);
                    }
                }
            } else {
                self.pan.y += scroll_y;
                self.pan.x += scroll_x;
            }
        }

        // Drag-and-drop file import (PDF or drawing notebook)
        let dropped_files = ui.input(|i| i.raw.dropped_files.clone());
        for dropped in dropped_files {
            let path = dropped.path();
            let path_str = path.to_string_lossy().to_string();
            if path_str.to_lowercase().ends_with(".pdf") {
                let _ = self.import_pdf(&path_str, false, ctx);
                break;
            } else if path_str.to_lowercase().ends_with(".drawing") || path_str.to_lowercase().ends_with(".noteful") {
                self.open_notebook(&path_str, ctx);
                break;
            }
        }

        // Pre-load / cache PDF texture for active page if needed
        if let Some(pdf_bg) = self.doc.pages.get(self.active_page_idx).and_then(|p| p.pdf_background.as_ref()) {
            let key = (pdf_bg.pdf_path.clone(), pdf_bg.page_number);
            if let std::collections::hash_map::Entry::Vacant(entry) = self.pdf_texture_cache.entry(key) {
                let abs_pdf_path = if Path::new(&pdf_bg.pdf_path).is_absolute() {
                    PathBuf::from(&pdf_bg.pdf_path)
                } else {
                    ctx.vault.root().join(&pdf_bg.pdf_path)
                };
                if abs_pdf_path.exists() {
                    if let Some(tex) = Self::rasterize_pdf_page(ui.ctx(), &abs_pdf_path, pdf_bg.page_number) {
                        entry.insert(tex);
                    }
                }
            }
        }

        let page_count = self.doc.pages.len();
        if self.active_page_idx >= page_count {
            self.active_page_idx = page_count.saturating_sub(1);
        }
        let page = &mut self.doc.pages[self.active_page_idx];

        // Coordinate transforms
        let current_zoom = self.zoom;
        let page_w = page.width * current_zoom;
        let page_h = page.height * current_zoom;
        let center = rect.center() + self.pan;
        let page_rect = Rect::from_center_size(center, vec2(page_w, page_h));

        let to_page = move |pos: Pos2| -> Pos2 {
            pos2(
                (pos.x - page_rect.min.x) / current_zoom,
                (pos.y - page_rect.min.y) / current_zoom,
            )
        };
        let to_screen = move |pos: Pos2| -> Pos2 {
            pos2(
                page_rect.min.x + pos.x * current_zoom,
                page_rect.min.y + pos.y * current_zoom,
            )
        };

        // 1. Draw Paper Shadow & Sheet
        let paper_bg = if self.night_mode {
            Color32::from_rgb(24, 24, 28)
        } else {
            match page.paper_color {
                PaperColor::White => Color32::WHITE,
                PaperColor::Cream => Color32::from_rgb(254, 252, 242),
                PaperColor::Dark => Color32::from_rgb(32, 33, 36),
                PaperColor::Sepia => Color32::from_rgb(245, 238, 220),
            }
        };

        painter.rect_filled(
            page_rect.expand(2.0),
            CornerRadius::same(6),
            Color32::from_black_alpha(if self.night_mode { 80 } else { 30 }),
        );
        painter.rect_filled(page_rect, CornerRadius::same(4), paper_bg);
        painter.rect_stroke(page_rect, CornerRadius::same(4), Stroke::new(1.0, t.separator), egui::StrokeKind::Outside);

        // 2. Render Page Background (PDF Background page or Template grid/lines)
        if let Some(pdf_bg) = &page.pdf_background {
            let key = (pdf_bg.pdf_path.clone(), pdf_bg.page_number);
            if let Some(texture) = self.pdf_texture_cache.get(&key) {
                let tint = if self.night_mode {
                    Color32::from_rgb(215, 215, 220)
                } else {
                    Color32::WHITE
                };
                painter.image(
                    texture.id(),
                    page_rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    tint,
                );
            } else {
                Self::draw_pdf_fallback_background(&painter, page_rect, pdf_bg, self.zoom, self.night_mode);
            }
        } else {
            Self::draw_template_background(&painter, page_rect, &page.template, self.zoom, self.night_mode);
        }

        // 3. Render Layers & Elements
        // Pass 1: Highlighters (behind pen strokes)
        for layer in &page.layers {
            if !layer.visible {
                continue;
            }
            let alpha_scale = layer.opacity.clamp(0.0, 1.0);
            for stroke in &layer.strokes {
                if stroke.tool == StrokeToolKind::Highlighter {
                    Self::draw_smooth_stroke(&painter, stroke, to_screen, self.zoom, alpha_scale, self.night_mode);
                }
            }
        }

        // Pass 2: Shapes & Pen Strokes
        for layer in &page.layers {
            if !layer.visible {
                continue;
            }
            let alpha_scale = layer.opacity.clamp(0.0, 1.0);
            // Shapes
            for shape in &layer.shapes {
                Self::draw_shape(&painter, shape, to_screen, self.zoom, alpha_scale, self.night_mode);
            }
            // Pen Strokes
            for stroke in &layer.strokes {
                if stroke.tool == StrokeToolKind::Pen {
                    Self::draw_smooth_stroke(&painter, stroke, to_screen, self.zoom, alpha_scale, self.night_mode);
                }
            }
        }

        // Pass 3: Text Boxes
        for layer in &page.layers {
            if !layer.visible {
                continue;
            }
            let alpha_scale = layer.opacity.clamp(0.0, 1.0);
            for tb in &layer.text_boxes {
                Self::draw_text_box(&painter, tb, to_screen, self.zoom, alpha_scale, t, self.night_mode);
            }
        }

        // Pass 4: Noteful Study Tapes (with interactive click-to-reveal)
        let mut tape_toggled = None;
        for (l_idx, layer) in page.layers.iter().enumerate() {
            if !layer.visible {
                continue;
            }
            for (t_idx, tape) in layer.tapes.iter().enumerate() {
                let tape_screen_min = to_screen(pos2(tape.x, tape.y));
                let tape_screen_max = to_screen(pos2(tape.x + tape.width, tape.y + tape.height));
                let tape_screen_rect = Rect::from_min_max(tape_screen_min, tape_screen_max);

                Self::draw_study_tape(&painter, tape, tape_screen_rect, self.night_mode);

                // Detect click to toggle reveal
                if response.clicked() {
                    if let Some(mouse_pos) = response.interact_pointer_pos() {
                        if tape_screen_rect.contains(mouse_pos) {
                            tape_toggled = Some((l_idx, t_idx));
                        }
                    }
                }
            }
        }

        if let Some((l_idx, t_idx)) = tape_toggled {
            page.layers[l_idx].tapes[t_idx].toggle();
            let is_rev = page.layers[l_idx].tapes[t_idx].revealed;
            self.dirty = true;
            ctx.toast(if is_rev { "Study Tape Revealed!" } else { "Study Tape Concealed" });
        }

        // 4. In-progress Stroke & Auto-Shape Feedback
        if !self.current_stroke_points.is_empty() {
            let active_tool = self.active_tool;
            let temp_stroke = VectorStroke {
                id: "in_progress".into(),
                tool: match active_tool {
                    ActiveTool::Highlighter => StrokeToolKind::Highlighter,
                    _ => StrokeToolKind::Pen,
                },
                pen_type: self.pen_type,
                points: self.current_stroke_points.clone(),
                color: match active_tool {
                    ActiveTool::Highlighter => self.highlighter_color,
                    _ => self.pen_color,
                },
                base_width: match active_tool {
                    ActiveTool::Highlighter => self.highlighter_width,
                    _ => self.pen_width,
                },
                is_straight: false,
            };
            Self::draw_smooth_stroke(&painter, &temp_stroke, to_screen, self.zoom, 1.0, self.night_mode);

            // If auto-shape is active, show hint outline if recognized
            if self.pen_auto_shape && active_tool == ActiveTool::Pen {
                if let Some(recognized) = recognize_stroke_shape(&self.current_stroke_points, self.pen_color, self.pen_width) {
                    Self::draw_shape(&painter, &recognized, to_screen, self.zoom, 0.5, self.night_mode);
                }
            }
        }

        // 5. Laser Pointer Glow & Trail
        let now = Instant::now();
        self.laser_trail.retain(|pt| now.duration_since(pt.created).as_millis() < 800);
        for i in 1..self.laser_trail.len() {
            let p1 = self.laser_trail[i - 1].pos;
            let p2 = self.laser_trail[i].pos;
            let age_ms = now.duration_since(self.laser_trail[i].created).as_millis() as f32;
            let alpha = ((1.0 - (age_ms / 800.0)) * 255.0).clamp(0.0, 255.0) as u8;
            painter.line_segment([p1, p2], Stroke::new(4.0, Color32::from_rgba_unmultiplied(239, 68, 68, alpha)));
        }

        // 6. User Pointer Input Handling & Active Tool Interaction
        if !is_panning_shortcut {
            if let Some(ptr_pos) = response.interact_pointer_pos() {
                let page_pos = to_page(ptr_pos);
                let inside_page = page_rect.contains(ptr_pos);

                let secondary_held = pointer.button_down(PointerButton::Secondary);
                let secondary_action = response.dragged_by(PointerButton::Secondary)
                    || response.clicked_by(PointerButton::Secondary)
                    || (response.dragged() && secondary_held);

                // Quick Stylus Barrel Button Action (e.g., Quick Eraser while holding button)
                if secondary_action && inside_page {
                    match self.stylus_barrel_action {
                        StylusButtonAction::QuickEraser => {
                            self.current_stroke_points.clear();
                            self.stroke_start_time = None;
                            self.hold_start = None;
                            self.last_point_pos = None;
                            self.last_point_time = None;
                            self.erase_at_point(self.active_page_idx, page_pos);
                        }
                        StylusButtonAction::QuickLasso => {}
                    }
                } else {
                    // Handle Active Tool
                    match self.active_tool {
                        ActiveTool::Hand => {
                            if response.dragged_by(PointerButton::Primary) || response.dragged_by(PointerButton::Middle) {
                                self.pan += response.drag_delta();
                            }
                        }
                        ActiveTool::Pen | ActiveTool::Highlighter => {
                            if response.drag_started() && inside_page {
                                self.current_stroke_points.clear();
                                self.stroke_start_time = Some(Instant::now());
                                self.hold_start = Some(Instant::now());
                                self.last_point_pos = Some(ptr_pos);
                                self.last_point_time = Some(Instant::now());

                                let initial_pressure = self.compute_pressure(
                                    self.stylus_pressure,
                                    None,
                                    self.active_tool,
                                    self.pen_type,
                                );
                                self.current_stroke_points.push(StrokePoint::new(page_pos.x, page_pos.y, initial_pressure, 0));
                            } else if response.dragged() && self.stroke_start_time.is_some() {
                                let now = Instant::now();
                                let ms = self.stroke_start_time.unwrap().elapsed().as_millis() as u64;

                                let dt_ms = self.last_point_time
                                    .map(|t| (now - t).as_millis() as f32)
                                    .unwrap_or(16.0)
                                    .max(1.0);

                                let mut dist = 0.0;
                                if let Some(last_pos) = self.last_point_pos {
                                    dist = (ptr_pos - last_pos).length();
                                    if dist < 4.0 {
                                        // Stationary hold detected
                                    } else {
                                        self.hold_start = Some(now);
                                    }
                                }

                                if dist >= 1.5 || self.current_stroke_points.is_empty() {
                                    self.last_point_pos = Some(ptr_pos);
                                    self.last_point_time = Some(now);

                                    let speed = dist / dt_ms;
                                    let pressure = self.compute_pressure(
                                        self.stylus_pressure,
                                        Some(speed),
                                        self.active_tool,
                                        self.pen_type,
                                    );

                                    // Straight line highlighter constraint
                                    let point_to_add = if self.active_tool == ActiveTool::Highlighter && self.highlighter_straight {
                                        if let Some(first) = self.current_stroke_points.first() {
                                            StrokePoint::new(page_pos.x, first.y, pressure, ms)
                                        } else {
                                            StrokePoint::new(page_pos.x, page_pos.y, pressure, ms)
                                        }
                                    } else {
                                        StrokePoint::new(page_pos.x, page_pos.y, pressure, ms)
                                    };

                                    self.current_stroke_points.push(point_to_add);
                                }
                            } else if response.drag_stopped() && !self.current_stroke_points.is_empty() {
                                // Commit stroke or auto-shape
                                let hold_ms = self.hold_start.map(|h| h.elapsed().as_millis()).unwrap_or(0);
                                let auto_shape_triggered = self.pen_auto_shape
                                    && self.active_tool == ActiveTool::Pen
                                    && hold_ms > 350;

                                let mut shape_added = false;
                                if auto_shape_triggered {
                                    if let Some(shape) = recognize_stroke_shape(&self.current_stroke_points, self.pen_color, self.pen_width) {
                                        if let Some(layer) = page.active_layer_mut() {
                                            if !layer.locked {
                                                layer.shapes.push(shape);
                                                self.dirty = true;
                                                shape_added = true;
                                            }
                                        }
                                    }
                                }

                                if !shape_added {
                                    let is_hl = self.active_tool == ActiveTool::Highlighter;
                                    let stroke = VectorStroke {
                                        id: format!("stroke_{:x}", Instant::now().elapsed().as_nanos()),
                                        tool: if is_hl { StrokeToolKind::Highlighter } else { StrokeToolKind::Pen },
                                        pen_type: self.pen_type,
                                        points: std::mem::take(&mut self.current_stroke_points),
                                        color: if is_hl { self.highlighter_color } else { self.pen_color },
                                        base_width: if is_hl { self.highlighter_width } else { self.pen_width },
                                        is_straight: is_hl && self.highlighter_straight,
                                    };
                                    if let Some(layer) = page.active_layer_mut() {
                                        if !layer.locked {
                                            layer.strokes.push(stroke);
                                            self.dirty = true;
                                        }
                                    }
                                }

                                self.current_stroke_points.clear();
                                self.stroke_start_time = None;
                                self.hold_start = None;
                                self.last_point_pos = None;
                                self.last_point_time = None;
                            }
                        }
                        ActiveTool::Eraser => {
                            if (response.dragged() || response.clicked()) && inside_page {
                                self.erase_at_point(self.active_page_idx, page_pos);
                            }
                        }
                ActiveTool::Tape => {
                    if response.drag_started() && inside_page {
                        self.selection_drag_start = Some(page_pos);
                    } else if response.drag_stopped() {
                        if let Some(start) = self.selection_drag_start.take() {
                            let min_x = start.x.min(page_pos.x);
                            let min_y = start.y.min(page_pos.y);
                            let w = (start.x - page_pos.x).abs().max(40.0);
                            let h = (start.y - page_pos.y).abs().max(self.tape_height);

                            let tape = NotefulTape::new(min_x, min_y, w, h, self.tape_color);
                            if let Some(layer) = page.active_layer_mut() {
                                if !layer.locked {
                                    layer.tapes.push(tape);
                                    self.dirty = true;
                                    ctx.toast("Study Tape created! Tap anytime to reveal/conceal.");
                                }
                            }
                        }
                    }
                }
                ActiveTool::Shape => {
                    if response.drag_started() && inside_page {
                        self.selection_drag_start = Some(page_pos);
                    } else if response.drag_stopped() {
                        if let Some(start) = self.selection_drag_start.take() {
                            let shape_kind = match self.shape_kind {
                                ShapeToolKind::Line => ShapeKind::Line {
                                    from: [start.x, start.y],
                                    to: [page_pos.x, page_pos.y],
                                    arrow: false,
                                },
                                ShapeToolKind::Arrow => ShapeKind::Line {
                                    from: [start.x, start.y],
                                    to: [page_pos.x, page_pos.y],
                                    arrow: true,
                                },
                                ShapeToolKind::Rectangle => ShapeKind::Rectangle {
                                    x: start.x.min(page_pos.x),
                                    y: start.y.min(page_pos.y),
                                    width: (start.x - page_pos.x).abs().max(20.0),
                                    height: (start.y - page_pos.y).abs().max(20.0),
                                    corner_radius: 4.0,
                                },
                                ShapeToolKind::Ellipse => {
                                    let cx = (start.x + page_pos.x) * 0.5;
                                    let cy = (start.y + page_pos.y) * 0.5;
                                    let rx = (start.x - page_pos.x).abs() * 0.5;
                                    let ry = (start.y - page_pos.y).abs() * 0.5;
                                    ShapeKind::Ellipse { cx, cy, rx, ry }
                                }
                            };

                            let shape = NotefulShape::new(shape_kind, self.shape_color, self.shape_width);
                            if let Some(layer) = page.active_layer_mut() {
                                if !layer.locked {
                                    layer.shapes.push(shape);
                                    self.dirty = true;
                                }
                            }
                        }
                    }
                }
                ActiveTool::Text => {
                    if response.clicked() && inside_page {
                        let text_box = NotefulTextBox::new(page_pos.x, page_pos.y, "Double click to edit note…");
                        if let Some(layer) = page.active_layer_mut() {
                            if !layer.locked {
                                layer.text_boxes.push(text_box);
                                self.dirty = true;
                            }
                        }
                    }
                }
                ActiveTool::Laser => {
                    self.laser_trail.push(LaserPoint {
                        pos: ptr_pos,
                        created: Instant::now(),
                    });
                }
                _ => {}
            }
        }
    }
}

        if response.drag_stopped() {
            self.stylus_pressure = None;
            self.last_point_time = None;
        }

        // 7. Stylus / Pointer Hover Feedback (Eraser outline & Pen tip preview)
        if response.hovered() && !is_panning_shortcut {
            if let Some(hover_pos) = response.hover_pos() {
                let is_quick_erasing = pointer.button_down(PointerButton::Secondary)
                    && self.stylus_barrel_action == StylusButtonAction::QuickEraser;

                if self.active_tool == ActiveTool::Eraser || is_quick_erasing {
                    let radius = self.eraser_size * self.zoom;
                    painter.circle_stroke(
                        hover_pos,
                        radius,
                        Stroke::new(1.5, Color32::from_rgba_unmultiplied(239, 68, 68, 180)),
                    );
                } else if self.active_tool == ActiveTool::Pen || self.active_tool == ActiveTool::Highlighter {
                    let base_w = if self.active_tool == ActiveTool::Highlighter {
                        self.highlighter_width
                    } else {
                        self.pen_width
                    };
                    let cur_p = self.stylus_pressure.unwrap_or(1.0);
                    let tip_radius = (base_w * 0.5 * cur_p * self.zoom).max(1.5);
                    let tip_color = if self.active_tool == ActiveTool::Highlighter {
                        Color32::from_rgba_unmultiplied(250, 204, 21, 140)
                    } else {
                        let c = self.pen_color;
                        Color32::from_rgba_unmultiplied(c.r, c.g, c.b, 160)
                    };
                    painter.circle_stroke(hover_pos, tip_radius, Stroke::new(1.0, tip_color));
                }
            }
        }
    }

    /// Render Paper Template background vector grid/lines
    fn draw_template_background(painter: &egui::Painter, rect: Rect, template: &BackgroundTemplate, zoom: f32, night_mode: bool) {
        let line_color = if night_mode {
            Color32::from_rgb(45, 47, 52)
        } else {
            Color32::from_rgb(222, 226, 230)
        };
        let margin_color = if night_mode {
            Color32::from_rgb(120, 50, 50)
        } else {
            Color32::from_rgb(248, 113, 113)
        };

        match template {
            BackgroundTemplate::Blank => {}
            BackgroundTemplate::Ruled { spacing, margin } => {
                let scaled_spacing = *spacing * zoom;
                let mut y = rect.min.y + 40.0 * zoom;
                while y < rect.max.y - 20.0 * zoom {
                    painter.line_segment([pos2(rect.min.x + 20.0 * zoom, y), pos2(rect.max.x - 20.0 * zoom, y)], Stroke::new(1.0, line_color));
                    y += scaled_spacing;
                }
                if *margin {
                    let margin_x = rect.min.x + 60.0 * zoom;
                    painter.line_segment([pos2(margin_x, rect.min.y + 20.0 * zoom), pos2(margin_x, rect.max.y - 20.0 * zoom)], Stroke::new(1.0, margin_color));
                }
            }
            BackgroundTemplate::Grid { spacing } => {
                let s = *spacing * zoom;
                let mut x = rect.min.x + s;
                while x < rect.max.x {
                    painter.line_segment([pos2(x, rect.min.y), pos2(x, rect.max.y)], Stroke::new(0.5, line_color));
                    x += s;
                }
                let mut y = rect.min.y + s;
                while y < rect.max.y {
                    painter.line_segment([pos2(rect.min.x, y), pos2(rect.max.x, y)], Stroke::new(0.5, line_color));
                    y += s;
                }
            }
            BackgroundTemplate::Dotted { spacing } => {
                let s = *spacing * zoom;
                let mut y = rect.min.y + s;
                while y < rect.max.y {
                    let mut x = rect.min.x + s;
                    while x < rect.max.x {
                        painter.circle_filled(pos2(x, y), 1.0 * zoom, line_color);
                        x += s;
                    }
                    y += s;
                }
            }
            BackgroundTemplate::Cornell { summary_height, cue_width } => {
                let cue_x = rect.min.x + *cue_width * zoom;
                let sum_y = rect.max.y - *summary_height * zoom;
                // Header line
                painter.line_segment([pos2(rect.min.x + 10.0, rect.min.y + 45.0 * zoom), pos2(rect.max.x - 10.0, rect.min.y + 45.0 * zoom)], Stroke::new(1.5, line_color));
                // Cue column divider
                painter.line_segment([pos2(cue_x, rect.min.y + 45.0 * zoom), pos2(cue_x, sum_y)], Stroke::new(1.5, margin_color));
                // Summary divider
                painter.line_segment([pos2(rect.min.x + 10.0, sum_y), pos2(rect.max.x - 10.0, sum_y)], Stroke::new(1.5, line_color));
            }
            _ => {
                // Default lined fallback
                let mut y = rect.min.y + 35.0 * zoom;
                while y < rect.max.y {
                    painter.line_segment([pos2(rect.min.x + 20.0, y), pos2(rect.max.x - 20.0, y)], Stroke::new(1.0, line_color));
                    y += 28.0 * zoom;
                }
            }
        }
    }

    /// Render a smooth vector stroke with Catmull-Rom spline interpolation
    fn draw_smooth_stroke(
        painter: &egui::Painter,
        stroke: &VectorStroke,
        to_screen: impl Fn(Pos2) -> Pos2,
        zoom: f32,
        layer_opacity: f32,
        night_mode: bool,
    ) {
        if stroke.points.len() < 2 {
            if let Some(p) = stroke.points.first() {
                let sc = to_screen(pos2(p.x, p.y));
                let mut c = stroke.color;
                if night_mode && c == RgbaColor::BLACK {
                    c = RgbaColor::WHITE;
                }
                let color = Color32::from_rgba_unmultiplied(c.r, c.g, c.b, ((c.a as f32) * layer_opacity) as u8);
                let p_factor = p.pressure;
                painter.circle_filled(sc, (stroke.base_width * 0.5 * p_factor * zoom).max(1.0), color);
            }
            return;
        }

        let mut c = stroke.color;
        if night_mode && c == RgbaColor::BLACK {
            c = RgbaColor::WHITE;
        }
        let color = Color32::from_rgba_unmultiplied(c.r, c.g, c.b, ((c.a as f32) * layer_opacity) as u8);

        // Catmull-Rom spline interpolation for smooth inking with continuous variable pressure
        let pts: Vec<Pos2> = stroke.points.iter().map(|p| to_screen(pos2(p.x, p.y))).collect();
        for i in 0..pts.len() - 1 {
            let p0 = if i > 0 { pts[i - 1] } else { pts[i] };
            let p1 = pts[i];
            let p2 = pts[i + 1];
            let p3 = if i + 2 < pts.len() { pts[i + 2] } else { p2 };

            let pr1 = stroke.points[i].pressure;
            let pr2 = stroke.points[i + 1].pressure;

            // Subdivide into 4 segments per point pair with smoothly interpolated pressure
            let mut prev = p1;
            for step in 1..=4 {
                let t = step as f32 / 4.0;
                let t2 = t * t;
                let t3 = t2 * t;

                let x = 0.5 * ((2.0 * p1.x) + (-p0.x + p2.x) * t + (2.0 * p0.x - 5.0 * p1.x + 4.0 * p2.x - p3.x) * t2 + (-p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x) * t3);
                let y = 0.5 * ((2.0 * p1.y) + (-p0.y + p2.y) * t + (2.0 * p0.y - 5.0 * p1.y + 4.0 * p2.y - p3.y) * t2 + (-p0.y + 3.0 * p1.y - 3.0 * p2.y + p3.y) * t3);
                let curr = pos2(x, y);

                let seg_pressure = pr1 + (pr2 - pr1) * t;
                let width = (stroke.base_width * seg_pressure * zoom).max(1.0);

                painter.line_segment([prev, curr], Stroke::new(width, color));
                prev = curr;
            }
        }
    }

    /// Render a geometric shape
    fn draw_shape(
        painter: &egui::Painter,
        shape: &NotefulShape,
        to_screen: impl Fn(Pos2) -> Pos2,
        zoom: f32,
        layer_opacity: f32,
        night_mode: bool,
    ) {
        let mut c = shape.stroke_color;
        if night_mode && c == RgbaColor::BLACK {
            c = RgbaColor::WHITE;
        }
        let stroke_color = Color32::from_rgba_unmultiplied(c.r, c.g, c.b, ((c.a as f32) * layer_opacity) as u8);
        let stroke = Stroke::new((shape.stroke_width * zoom).max(1.0), stroke_color);

        match &shape.kind {
            ShapeKind::Line { from, to, arrow } => {
                let p1 = to_screen(pos2(from[0], from[1]));
                let p2 = to_screen(pos2(to[0], to[1]));
                painter.line_segment([p1, p2], stroke);
                if *arrow {
                    let dir = (p2 - p1).normalized();
                    let normal = vec2(-dir.y, dir.x);
                    let arrow_len = 12.0 * zoom;
                    let a1 = p2 - dir * arrow_len + normal * (arrow_len * 0.5);
                    let a2 = p2 - dir * arrow_len - normal * (arrow_len * 0.5);
                    painter.line_segment([p2, a1], stroke);
                    painter.line_segment([p2, a2], stroke);
                }
            }
            ShapeKind::Rectangle { x, y, width, height, corner_radius } => {
                let p1 = to_screen(pos2(*x, *y));
                let p2 = to_screen(pos2(*x + *width, *y + *height));
                let r = Rect::from_min_max(p1, p2);
                painter.rect_stroke(r, CornerRadius::same((*corner_radius * zoom) as u8), stroke, egui::StrokeKind::Outside);
            }
            ShapeKind::Ellipse { cx, cy, rx, ry } => {
                let center = to_screen(pos2(*cx, *cy));
                let r_avg = (*rx + *ry) * 0.5 * zoom;
                painter.circle_stroke(center, r_avg, stroke);
            }
            ShapeKind::Polygon { points } => {
                if points.len() >= 2 {
                    let pts: Vec<Pos2> = points.iter().map(|p| to_screen(pos2(p[0], p[1]))).collect();
                    for i in 0..pts.len() {
                        let next = (i + 1) % pts.len();
                        painter.line_segment([pts[i], pts[next]], stroke);
                    }
                }
            }
        }
    }

    /// Render a rich text annotation box
    fn draw_text_box(
        painter: &egui::Painter,
        tb: &NotefulTextBox,
        to_screen: impl Fn(Pos2) -> Pos2,
        zoom: f32,
        layer_opacity: f32,
        t: &Tokens,
        night_mode: bool,
    ) {
        let p_min = to_screen(pos2(tb.x, tb.y));
        let p_max = to_screen(pos2(tb.x + tb.width, tb.y + tb.height));
        let rect = Rect::from_min_max(p_min, p_max);

        if tb.has_border {
            painter.rect_filled(rect, CornerRadius::same(4), Color32::from_white_alpha(30));
            painter.rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, t.separator), egui::StrokeKind::Outside);
        }

        let font_size = (tb.font_size * zoom).clamp(8.0, 48.0);
        let mut tc = tb.text_color;
        if night_mode && tc == RgbaColor::BLACK {
            tc = RgbaColor::WHITE;
        }
        let color = Color32::from_rgba_unmultiplied(tc.r, tc.g, tc.b, ((tc.a as f32) * layer_opacity) as u8);

        painter.text(
            p_min + vec2(4.0 * zoom, 4.0 * zoom),
            Align2::LEFT_TOP,
            &tb.text,
            FontId::proportional(font_size),
            color,
        );
    }

    /// Render Noteful signature Study Tape
    fn draw_study_tape(
        painter: &egui::Painter,
        tape: &NotefulTape,
        rect: Rect,
        _night_mode: bool,
    ) {
        let base_c = tape.color;
        if tape.revealed {
            // Revealed Mode: semi-transparent with dashed border and hatched indicator
            let fill_c = Color32::from_rgba_unmultiplied(base_c.r, base_c.g, base_c.b, 40);
            painter.rect_filled(rect, CornerRadius::same(3), fill_c);
            painter.rect_stroke(rect, CornerRadius::same(3), Stroke::new(1.5, Color32::from_rgba_unmultiplied(base_c.r, base_c.g, base_c.b, 160)), egui::StrokeKind::Outside);

            // Draw diagonal study hatch marks
            let mut x = rect.min.x + 8.0;
            while x < rect.max.x {
                painter.line_segment(
                    [pos2(x, rect.min.y + 2.0), pos2(x + 10.0, rect.max.y - 2.0)],
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(base_c.r, base_c.g, base_c.b, 60)),
                );
                x += 16.0;
            }

            // Small eye icon hint
            painter.text(rect.center(), Align2::CENTER_CENTER, "👀 [Tap to Conceal]", FontId::proportional(10.0), Color32::from_rgb(120, 120, 130));
        } else {
            // Concealed Mode: Opaque revision tape with slight textured bevel
            let fill_c = Color32::from_rgb(base_c.r, base_c.g, base_c.b);
            painter.rect_filled(rect, CornerRadius::same(3), fill_c);
            // 3D tape border highlights
            painter.line_segment([rect.left_top(), rect.right_top()], Stroke::new(1.5, Color32::from_white_alpha(120)));
            painter.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.5, Color32::from_black_alpha(50)));

            painter.text(rect.center(), Align2::CENTER_CENTER, "Tape (Tap to Reveal)", FontId::proportional(11.0), Color32::from_black_alpha(170));
        }
    }

    /// Render Layers Drawer panel on right side of workspace
    fn render_layers_drawer(&mut self, ui: &mut Ui, t: &Tokens, ctx: &mut PluginContext) {
        ui.vertical(|ui| {
            ui.set_width(220.0);
            Frame::NONE
                .fill(t.card)
                .inner_margin(Margin::same(8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Layers").strong().size(13.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new(RichText::new("＋ Add Layer").size(11.0))).clicked() {
                                if let Some(page) = self.doc.pages.get_mut(self.active_page_idx) {
                                    let idx = page.layers.len() + 1;
                                    page.add_layer(format!("Layer {idx}"));
                                    self.dirty = true;
                                    ctx.toast(format!("Added Layer {idx}"));
                                }
                            }
                        });
                    });

                    ui.separator();

                    if let Some(page) = self.doc.pages.get_mut(self.active_page_idx) {
                        let mut delete_layer_idx = None;
                        let mut merge_layer_idx = None;
                        let mut new_active_layer_id = None;
                        let total_layers = page.layers.len();
                        let current_active_id = page.active_layer_id.clone();

                        egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                            for (idx, layer) in page.layers.iter_mut().enumerate().rev() {
                                let is_active = layer.id == current_active_id;

                                Frame::NONE
                                    .fill(if is_active { t.field } else { Color32::TRANSPARENT })
                                    .stroke(if is_active { Stroke::new(1.0, t.accent_border) } else { Stroke::NONE })
                                    .corner_radius(CornerRadius::same(t.radius_sm as u8))
                                    .inner_margin(Margin::symmetric(6, 4))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            // Radio / Selection check
                                            if ui.selectable_label(is_active, &layer.name).clicked() {
                                                new_active_layer_id = Some(layer.id.clone());
                                            }

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                // Lock toggle
                                                let (lock_rect, lock_resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                                                if ui.is_rect_visible(lock_rect) {
                                                    let col = if lock_resp.hovered() { t.text } else { t.text_dim };
                                                    theme::paint_lock_icon(ui.painter(), lock_rect, col, layer.locked);
                                                }
                                                if lock_resp.on_hover_text(if layer.locked { "Unlock layer" } else { "Lock layer" }).clicked() {
                                                    layer.locked = !layer.locked;
                                                }

                                                // Visibility eye toggle
                                                let (eye_rect, eye_resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                                                if ui.is_rect_visible(eye_rect) {
                                                    let col = if eye_resp.hovered() { t.text } else { t.text_dim };
                                                    theme::paint_eye_icon(ui.painter(), eye_rect, col, layer.visible);
                                                }
                                                if eye_resp.on_hover_text(if layer.visible { "Hide layer" } else { "Show layer" }).clicked() {
                                                    layer.visible = !layer.visible;
                                                }
                                            });
                                        });

                                        // Opacity slider
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("Opacity:").size(10.0).color(t.text_dim));
                                            ui.add(egui::Slider::new(&mut layer.opacity, 0.1..=1.0).show_value(false));
                                        });

                                        // Layer actions context
                                        if is_active && total_layers > 1 {
                                            ui.horizontal(|ui| {
                                                if idx > 0 && ui.add(egui::Button::new(RichText::new("Merge Down").size(10.0))).clicked() {
                                                    merge_layer_idx = Some(idx);
                                                }
                                                if ui.add(egui::Button::new(RichText::new("Delete").size(10.0).color(t.danger))).clicked() {
                                                    delete_layer_idx = Some(idx);
                                                }
                                            });
                                        }
                                    });
                                ui.add_space(3.0);
                            }
                        });

                        if let Some(act) = new_active_layer_id {
                            page.active_layer_id = act;
                        }

                        if let Some(m_idx) = merge_layer_idx {
                            page.merge_down(m_idx);
                            self.dirty = true;
                            ctx.toast("Merged layer down");
                        }
                        if let Some(d_idx) = delete_layer_idx {
                            if page.layers.len() > 1 {
                                page.layers.remove(d_idx);
                                if let Some(last) = page.layers.last() {
                                    page.active_layer_id = last.id.clone();
                                }
                                self.dirty = true;
                                ctx.toast("Deleted layer");
                            }
                        }
                    }
                });
        });
    }

    /// Modal dialog to create a new Drawing Document
    fn render_new_notebook_modal(&mut self, ctx_egui: &egui::Context, t: &Tokens, ctx: &mut PluginContext) {
        egui::Window::new("🎨 Create Drawing Document")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx_egui, |ui| {
                ui.set_width(320.0);
                ui.spacing_mut().item_spacing.y = 8.0;

                ui.label(RichText::new("Document Title:").size(12.0));
                ui.text_edit_singleline(&mut self.new_title);

                ui.label(RichText::new("Background Template:").size(12.0));
                let templates = ["Blank Paper", "Ruled / Lined Paper", "Grid / Graph Paper", "Dotted Journal", "Cornell Note Style", "Music Staff Sheet"];
                egui::ComboBox::from_id_salt("drawing_tmpl_combo")
                    .selected_text(templates[self.new_template_idx])
                    .show_ui(ui, |ui| {
                        for (i, &name) in templates.iter().enumerate() {
                            ui.selectable_value(&mut self.new_template_idx, i, name);
                        }
                    });

                ui.label(RichText::new("Paper Color:").size(12.0));
                let colors = ["White", "Cream", "Dark", "Sepia"];
                egui::ComboBox::from_id_salt("drawing_color_combo")
                    .selected_text(colors[self.new_color_idx])
                    .show_ui(ui, |ui| {
                        for (i, &name) in colors.iter().enumerate() {
                            ui.selectable_value(&mut self.new_color_idx, i, name);
                        }
                    });

                ui.label(RichText::new("Orientation:").size(12.0));
                let orientations = ["Portrait", "Landscape"];
                egui::ComboBox::from_id_salt("drawing_orient_combo")
                    .selected_text(orientations[self.new_orientation_idx])
                    .show_ui(ui, |ui| {
                        for (i, &name) in orientations.iter().enumerate() {
                            ui.selectable_value(&mut self.new_orientation_idx, i, name);
                        }
                    });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("Cancel").size(12.0))).clicked() {
                        self.show_new_notebook_dialog = false;
                    }

                    if ui.add(egui::Button::new(RichText::new("Create Document").size(12.0).color(t.accent_text)).fill(t.accent)).clicked() {
                        let tmpl = match self.new_template_idx {
                            0 => BackgroundTemplate::Blank,
                            1 => BackgroundTemplate::Ruled { spacing: 28.0, margin: true },
                            2 => BackgroundTemplate::Grid { spacing: 20.0 },
                            3 => BackgroundTemplate::Dotted { spacing: 20.0 },
                            4 => BackgroundTemplate::Cornell { summary_height: 120.0, cue_width: 140.0 },
                            5 => BackgroundTemplate::MusicStaff { staves: 10 },
                            _ => BackgroundTemplate::default(),
                        };
                        let col = match self.new_color_idx {
                            0 => PaperColor::White,
                            1 => PaperColor::Cream,
                            2 => PaperColor::Dark,
                            3 => PaperColor::Sepia,
                            _ => PaperColor::White,
                        };
                        let ori = match self.new_orientation_idx {
                            1 => PaperOrientation::Landscape,
                            _ => PaperOrientation::Portrait,
                        };

                        let title = self.new_title.clone();
                        let _ = self.create_notebook(&title, tmpl, col, ori, ctx);
                        self.show_new_notebook_dialog = false;
                    }
                });
            });
    }

    /// Modal dialog to export drawing document
    fn render_export_modal(&mut self, ctx_egui: &egui::Context, t: &Tokens, ctx: &mut PluginContext) {
        egui::Window::new("📤 Export Drawing Document")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx_egui, |ui| {
                ui.set_width(300.0);
                ui.spacing_mut().item_spacing.y = 8.0;

                ui.label(RichText::new(format!("Exporting: {}", self.doc.title)).strong());
                ui.checkbox(&mut self.export_include_bg, "Include Paper Background");
                ui.checkbox(&mut self.export_include_annotations, "Include Inking & Annotations");
                ui.checkbox(&mut self.export_include_tapes, "Include Study Tapes");

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new("Cancel")).clicked() {
                        self.show_export_dialog = false;
                    }
                    if ui.add(egui::Button::new(RichText::new("Export to Vault").color(t.accent_text)).fill(t.accent)).clicked() {
                        let title = &self.doc.title;
                        let out_path = format!("Exports/{title}.drawing");
                        if let Err(e) = ctx.vault.save_drawing(&out_path, &self.doc) {
                            ctx.toast(format!("Export failed: {e}"));
                        } else {
                            ctx.toast(format!("Exported to {out_path}"));
                        }
                        self.show_export_dialog = false;
                    }
                });
            });
    }

    /// Inspect a PDF file to extract total page count and dimensions (width, height in points)
    pub fn inspect_pdf_file(path: &Path) -> (usize, [f32; 2]) {
        // 1. Try running pdfinfo
        if let Ok(output) = Command::new("pdfinfo").arg(path).output() {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                let mut page_count: usize = 1;
                let mut dimensions: [f32; 2] = [595.0, 842.0];

                for line in text.lines() {
                    let trimmed = line.trim();
                    if let Some(rest) = trimmed.strip_prefix("Pages:") {
                        if let Ok(cnt) = rest.trim().parse::<usize>() {
                            page_count = cnt.max(1);
                        }
                    } else if let Some(rest) = trimmed.strip_prefix("Page size:") {
                        let parts: Vec<&str> = rest.split_whitespace().collect();
                        if parts.len() >= 3 && parts[1] == "x" {
                            if let (Ok(w), Ok(h)) = (parts[0].parse::<f32>(), parts[2].parse::<f32>()) {
                                dimensions = [w.max(100.0), h.max(100.0)];
                            }
                        }
                    }
                }
                return (page_count, dimensions);
            }
        }

        // 2. Pure-Rust fallback: parse PDF markers
        if let Ok(bytes) = std::fs::read(path) {
            let content = String::from_utf8_lossy(&bytes);
            let mut page_count = 1;
            let mut dimensions = [595.0, 842.0];

            if let Some(idx) = content.find("/Count ") {
                let after = &content[idx + 7..];
                let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(cnt) = num_str.parse::<usize>() {
                    if cnt > 0 {
                        page_count = cnt;
                    }
                }
            } else {
                let page_markers = content.match_indices("/Type /Page").filter(|(i, _)| {
                    !content[*i..].starts_with("/Type /Pages")
                }).count();
                if page_markers > 0 {
                    page_count = page_markers;
                }
            }

            if let Some(idx) = content.find("/MediaBox") {
                if let Some(start_bracket) = content[idx..].find('[') {
                    if let Some(end_bracket) = content[idx + start_bracket..].find(']') {
                        let box_str = &content[idx + start_bracket + 1..idx + start_bracket + end_bracket];
                        let nums: Vec<f32> = box_str
                            .split_whitespace()
                            .filter_map(|s| s.parse::<f32>().ok())
                            .collect();
                        if nums.len() == 4 {
                            let w = (nums[2] - nums[0]).abs();
                            let h = (nums[3] - nums[1]).abs();
                            if w > 50.0 && h > 50.0 {
                                dimensions = [w, h];
                            }
                        }
                    }
                }
            }

            return (page_count, dimensions);
        }

        (1, [595.0, 842.0])
    }

    /// Rasterize a specific PDF page to an egui texture using pdftoppm
    pub fn rasterize_pdf_page(ctx: &egui::Context, abs_pdf_path: &Path, page_number: usize) -> Option<egui::TextureHandle> {
        use std::time::SystemTime;

        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_stem = format!("brainwash_pdf_{}_{}_{}", std::process::id(), page_number, nanos);
        let temp_dir = std::env::temp_dir();
        let temp_prefix = temp_dir.join(&temp_stem);
        let expected_png = temp_dir.join(format!("{}.png", temp_stem));

        let output = Command::new("pdftoppm")
            .arg("-png")
            .arg("-singlefile")
            .arg("-r")
            .arg("150")
            .arg("-f")
            .arg(page_number.to_string())
            .arg("-l")
            .arg(page_number.to_string())
            .arg(abs_pdf_path)
            .arg(&temp_prefix)
            .output();

        if let Ok(out) = output {
            if out.status.success() && expected_png.exists() {
                if let Ok(bytes) = std::fs::read(&expected_png) {
                    let _ = std::fs::remove_file(&expected_png);
                    if let Ok(dynamic_img) = image::load_from_memory(&bytes) {
                        let rgba = dynamic_img.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        let color_img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba.into_raw());
                        return Some(ctx.load_texture(
                            format!("pdf_tex_{}_{}", temp_stem, page_number),
                            color_img,
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                }
            }
        }

        if expected_png.exists() {
            let _ = std::fs::remove_file(&expected_png);
        }
        None
    }

    /// Render elegant vector placeholder for PDF page when pdftoppm is unavailable
    fn draw_pdf_fallback_background(
        painter: &egui::Painter,
        rect: Rect,
        pdf_bg: &PdfBackground,
        zoom: f32,
        night_mode: bool,
    ) {
        let text_color = if night_mode {
            Color32::from_rgb(200, 205, 215)
        } else {
            Color32::from_rgb(50, 55, 65)
        };
        let line_color = if night_mode {
            Color32::from_rgb(45, 50, 60)
        } else {
            Color32::from_rgb(225, 230, 238)
        };
        let header_bg = if night_mode {
            Color32::from_rgb(30, 34, 42)
        } else {
            Color32::from_rgb(240, 243, 248)
        };

        // Header band
        let header_h = 36.0 * zoom;
        let header_rect = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + header_h));
        painter.rect_filled(header_rect, CornerRadius::same(4), header_bg);
        painter.line_segment([header_rect.left_bottom(), header_rect.right_bottom()], Stroke::new(1.0, line_color));

        // Document title & page counter
        painter.text(
            pos2(rect.min.x + 16.0 * zoom, rect.min.y + header_h * 0.5),
            Align2::LEFT_CENTER,
            format!("📄 {} — Page {} of {}", pdf_bg.title, pdf_bg.page_number, pdf_bg.total_pages),
            FontId::proportional(12.0 * zoom),
            text_color,
        );

        // Document margins & simulated text baseline guides
        let margin_x = 40.0 * zoom;
        let mut y = rect.min.y + header_h + 30.0 * zoom;
        while y < rect.max.y - 40.0 * zoom {
            painter.line_segment(
                [pos2(rect.min.x + margin_x, y), pos2(rect.max.x - margin_x, y)],
                Stroke::new(0.5, line_color),
            );
            y += 24.0 * zoom;
        }

        // Center watermark
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            format!("PDF Document: {}\n(Vector annotation canvas)", pdf_bg.title),
            FontId::proportional(14.0 * zoom),
            if night_mode { Color32::from_rgb(70, 75, 85) } else { Color32::from_rgb(180, 185, 195) },
        );
    }

    /// Import a PDF file into Drawing Documents:
    /// Either creates a new drawing notebook (1 drawing page per PDF page)
    /// or appends the PDF pages to the currently active drawing notebook.
    pub fn import_pdf(
        &mut self,
        pdf_path_or_rel: &str,
        as_new_doc: bool,
        ctx: &mut PluginContext,
    ) -> Result<String, String> {
        let clean_path = pdf_path_or_rel.trim();
        if clean_path.is_empty() {
            return Err("Please specify a PDF file path".to_string());
        }

        let abs_path = if Path::new(clean_path).is_absolute() {
            PathBuf::from(clean_path)
        } else {
            ctx.vault.root().join(clean_path)
        };

        if !abs_path.exists() {
            return Err(format!("PDF file not found: {}", abs_path.display()));
        }

        // Determine vault-relative path and title
        let (vault_rel_path, title) = if let Ok(rel) = abs_path.strip_prefix(ctx.vault.root()) {
            let rel_str = rel.to_string_lossy().to_string();
            let title = abs_path.file_stem().and_then(|s| s.to_str()).unwrap_or("PDF Document").to_string();
            (rel_str, title)
        } else {
            let file_name = abs_path.file_name().and_then(|s| s.to_str()).unwrap_or("document.pdf");
            let target_dir = ctx.vault.root().join("PDFs");
            let _ = std::fs::create_dir_all(&target_dir);
            let target_path = target_dir.join(file_name);
            if let Err(e) = std::fs::copy(&abs_path, &target_path) {
                return Err(format!("Failed to copy PDF into vault: {e}"));
            }
            let rel_str = format!("PDFs/{}", file_name);
            let title = abs_path.file_stem().and_then(|s| s.to_str()).unwrap_or("PDF Document").to_string();
            (rel_str, title)
        };

        // Inspect PDF for page count & dimensions
        let (page_count, size) = Self::inspect_pdf_file(&abs_path);

        if as_new_doc || self.active_path.is_none() {
            let doc = NotefulDocument::create_from_pdf(&title, &vault_rel_path, page_count, size);
            let safe_title = title.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
            let drawing_rel_path = unique_drawing_path(ctx.vault.root(), &format!("Notebooks/{safe_title}.drawing"));
            ctx.vault.save_drawing(&drawing_rel_path, &doc).map_err(|e| e.to_string())?;
            self.open_notebook(&drawing_rel_path, ctx);
            ctx.toast(format!("Imported PDF '{}' ({} pages)", title, page_count));
            Ok(drawing_rel_path)
        } else {
            let added = self.doc.append_pdf_pages(&vault_rel_path, page_count, size);
            self.dirty = true;
            self.save_notebook(ctx);
            ctx.toast(format!("Appended {} PDF pages to '{}'", added, self.doc.title));
            Ok(self.active_path.clone().unwrap_or_default())
        }
    }

    /// Modal dialog to import a PDF file into Drawing Documents
    fn render_import_pdf_modal(&mut self, ctx_egui: &egui::Context, t: &Tokens, ctx: &mut PluginContext) {
        let mut close_dialog = false;
        let mut perform_import = false;

        egui::Window::new("📄 Import PDF Document")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx_egui, |ui| {
                ui.set_width(380.0);
                ui.spacing_mut().item_spacing.y = 10.0;

                ui.label(
                    RichText::new("Import a PDF file to annotate with vector inking, highlighters, study tapes, and shapes.")
                        .size(12.0)
                        .color(t.text_dim),
                );

                // Quick pick from vault PDFs if available
                let vault_pdfs = ctx.vault.list_pdf_files();
                if !vault_pdfs.is_empty() {
                    ui.label(RichText::new("Choose PDF from Vault:").size(12.0).strong());
                    egui::ComboBox::from_id_salt("vault_pdf_picker")
                        .width(360.0)
                        .selected_text(
                            if self.import_pdf_path.is_empty() {
                                "Select a PDF from vault...".to_string()
                            } else {
                                self.import_pdf_path.clone()
                            },
                        )
                        .show_ui(ui, |ui| {
                            for pdf in &vault_pdfs {
                                if ui.selectable_label(self.import_pdf_path == *pdf, pdf).clicked() {
                                    self.import_pdf_path = pdf.clone();
                                }
                            }
                        });
                }

                ui.label(RichText::new("Or Enter PDF File Path:").size(12.0).strong());
                ui.text_edit_singleline(&mut self.import_pdf_path);
                ui.label(
                    RichText::new("Tip: Vault-relative (e.g. PDFs/lecture.pdf) or absolute system path. You can also drag & drop .pdf files onto the canvas.")
                        .size(11.0)
                        .color(t.text_faint),
                );

                // Inspection Preview
                let clean_path = self.import_pdf_path.trim();
                let abs_path = if !clean_path.is_empty() {
                    if Path::new(clean_path).is_absolute() {
                        PathBuf::from(clean_path)
                    } else {
                        ctx.vault.root().join(clean_path)
                    }
                } else {
                    PathBuf::new()
                };

                let file_exists = abs_path.is_file();
                if file_exists {
                    let (pages, dims) = Self::inspect_pdf_file(&abs_path);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("✓ Valid PDF: {pages} pages ({:.0} × {:.0} pt)", dims[0], dims[1]))
                                .size(11.5)
                                .color(t.accent),
                        );
                    });
                } else if !clean_path.is_empty() {
                    ui.label(RichText::new("⚠ PDF file not found at specified path").size(11.5).color(t.danger));
                }

                ui.separator();

                // Import mode options
                ui.label(RichText::new("Import Destination:").size(12.0).strong());
                ui.radio_value(&mut self.import_pdf_append_to_current, false, "Create New Drawing Notebook (1 page per PDF page)");
                let append_response = ui.add_enabled(
                    self.active_path.is_some(),
                    egui::RadioButton::new(self.import_pdf_append_to_current, "Append Pages to Current Open Document"),
                );
                if append_response.clicked() {
                    self.import_pdf_append_to_current = true;
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("Cancel").size(12.0))).clicked() {
                        close_dialog = true;
                    }

                    let btn_label = if self.import_pdf_append_to_current {
                        "Append PDF Pages"
                    } else {
                        "Import as New Notebook"
                    };

                    let import_btn = ui.add_enabled(
                        file_exists,
                        egui::Button::new(RichText::new(btn_label).size(12.0).color(t.accent_text))
                            .fill(t.accent)
                            .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                    );

                    if import_btn.clicked() {
                        perform_import = true;
                    }
                });
            });

        if close_dialog {
            self.show_import_pdf_dialog = false;
        }

        if perform_import {
            let path_to_import = self.import_pdf_path.trim().to_string();
            let as_new = !self.import_pdf_append_to_current;
            match self.import_pdf(&path_to_import, as_new, ctx) {
                Ok(_) => {
                    self.show_import_pdf_dialog = false;
                    self.import_pdf_path.clear();
                }
                Err(e) => {
                    ctx.toast(format!("Failed to import PDF: {e}"));
                }
            }
        }
    }
}

fn unique_drawing_path(vault_root: &Path, initial_rel: &str) -> String {
    let p = vault_root.join(initial_rel);
    if !p.exists() {
        return initial_rel.to_string();
    }
    let parent = Path::new(initial_rel).parent().unwrap_or_else(|| Path::new(""));
    let stem = Path::new(initial_rel)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Notebook");
    let ext = Path::new(initial_rel)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("drawing");

    let mut counter = 2;
    loop {
        let candidate = if parent.as_os_str().is_empty() {
            format!("{stem} {counter}.{ext}")
        } else {
            format!("{}/{stem} {counter}.{ext}", parent.display())
        };
        if !vault_root.join(&candidate).exists() {
            return candidate;
        }
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drawing_plugin_metadata_and_commands() {
        let plugin = DrawingPlugin::new();
        assert_eq!(plugin.id(), "core.drawing");
        assert_eq!(plugin.name(), "Drawing Documents");
        assert!(plugin.tab_label().contains("Drawing"));

        let cmds = plugin.commands();
        assert!(cmds.iter().any(|c| c.id == "drawing.new"));
        assert!(cmds.iter().any(|c| c.id == "drawing.import_pdf"));
        assert!(cmds.iter().any(|c| c.id == "drawing.cover_tapes"));
        assert!(cmds.iter().any(|c| c.id == "drawing.reveal_tapes"));
        assert!(cmds.iter().any(|c| c.id == "drawing.toggle_night_mode"));
        assert!(cmds.iter().any(|c| c.id == "drawing.toggle_layers"));
    }

    #[test]
    fn test_drawing_plugin_tool_state_and_study_tapes() {
        let mut plugin = DrawingPlugin::new();
        plugin.active_tool = ActiveTool::Tape;
        assert_eq!(plugin.active_tool, ActiveTool::Tape);

        // Add a page and a study tape
        assert_eq!(plugin.doc.pages.len(), 1);
        let tape = NotefulTape::new(100.0, 150.0, 80.0, 24.0, RgbaColor::new(245, 158, 11, 255));
        plugin.doc.pages[0].layers[0].tapes.push(tape);
        assert_eq!(plugin.doc.pages[0].layers[0].tapes.len(), 1);
        assert!(!plugin.doc.pages[0].layers[0].tapes[0].revealed);

        // Reveal all
        plugin.toggle_study_tapes(true);
        assert!(plugin.doc.pages[0].layers[0].tapes[0].revealed);

        // Conceal all
        plugin.toggle_study_tapes(false);
        assert!(!plugin.doc.pages[0].layers[0].tapes[0].revealed);

        // Add a layer
        plugin.doc.pages[0].add_layer("Annotations".to_string());
        assert_eq!(plugin.doc.pages[0].layers.len(), 2);
    }

    #[test]
    fn test_drawing_plugin_pdf_commands_and_state() {
        let plugin = DrawingPlugin::new();
        assert!(!plugin.show_import_pdf_dialog);
        assert!(plugin.import_pdf_path.is_empty());
        assert!(!plugin.import_pdf_append_to_current);

        let cmds = plugin.commands();
        assert!(cmds.iter().any(|c| c.id == "drawing.import_pdf"));
    }

    #[test]
    fn test_unique_drawing_path_resolution() {
        let temp_dir = std::env::temp_dir().join(format!("bw_test_unique_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let path1 = unique_drawing_path(&temp_dir, "Notebooks/Notes.drawing");
        assert_eq!(path1, "Notebooks/Notes.drawing");

        // Simulate file exists
        let _ = std::fs::create_dir_all(temp_dir.join("Notebooks"));
        let _ = std::fs::write(temp_dir.join(&path1), b"test");

        let path2 = unique_drawing_path(&temp_dir, "Notebooks/Notes.drawing");
        assert_eq!(path2, "Notebooks/Notes 2.drawing");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_inspect_pdf_fallback() {
        let temp_pdf = std::env::temp_dir().join(format!("test_sample_{}.pdf", std::process::id()));
        let sample = b"%PDF-1.4\n/Count 5\n/MediaBox [0 0 612 792]\n%%EOF";
        let _ = std::fs::write(&temp_pdf, sample);

        let (pages, size) = DrawingPlugin::inspect_pdf_file(&temp_pdf);
        assert_eq!(pages, 5);
        assert_eq!(size, [612.0, 792.0]);

        let _ = std::fs::remove_file(&temp_pdf);
    }

    #[test]
    fn test_wacom_stylus_pressure_curves() {
        let plugin = DrawingPlugin::new();

        // 1. Brush Pen: High dynamic range (hairline to bold)
        let brush_light = plugin.compute_pressure(Some(0.0), None, ActiveTool::Pen, PenType::Brush);
        let brush_med = plugin.compute_pressure(Some(0.5), None, ActiveTool::Pen, PenType::Brush);
        let brush_heavy = plugin.compute_pressure(Some(1.0), None, ActiveTool::Pen, PenType::Brush);
        assert!((brush_light - 0.15).abs() < 0.01);
        assert!((brush_med - 1.0).abs() < 0.01);
        assert!((brush_heavy - 1.85).abs() < 0.01);
        assert!(brush_heavy > brush_light * 10.0);

        // 2. Ballpoint: Firm and consistent
        let ball_light = plugin.compute_pressure(Some(0.0), None, ActiveTool::Pen, PenType::Ballpoint);
        let ball_heavy = plugin.compute_pressure(Some(1.0), None, ActiveTool::Pen, PenType::Ballpoint);
        assert!((ball_light - 0.80).abs() < 0.01);
        assert!((ball_heavy - 1.20).abs() < 0.01);

        // 3. Fountain Pen: Expressive non-linear curve
        let fountain_light = plugin.compute_pressure(Some(0.1), None, ActiveTool::Pen, PenType::Fountain);
        let fountain_heavy = plugin.compute_pressure(Some(1.0), None, ActiveTool::Pen, PenType::Fountain);
        assert!(fountain_light < 0.5);
        assert!(fountain_heavy > 1.6);

        // 4. Highlighter: Consistent width with subtle variation
        let hl_light = plugin.compute_pressure(Some(0.0), None, ActiveTool::Highlighter, PenType::Fountain);
        let hl_heavy = plugin.compute_pressure(Some(1.0), None, ActiveTool::Highlighter, PenType::Fountain);
        assert!((hl_light - 0.92).abs() < 0.01);
        assert!((hl_heavy - 1.08).abs() < 0.01);

        // 5. Velocity Fallback (when hardware force is None):
        // Slow speed (0.1 px/ms) -> full body
        let slow_p = plugin.compute_pressure(None, Some(0.1), ActiveTool::Pen, PenType::Brush);
        // Fast whip (2.5 px/ms) -> natural taper
        let fast_p = plugin.compute_pressure(None, Some(2.5), ActiveTool::Pen, PenType::Brush);
        assert!(slow_p > fast_p);
    }

    #[test]
    fn test_wacom_stylus_barrel_button_quick_erase() {
        let mut plugin = DrawingPlugin::new();
        plugin.eraser_size = 20.0;
        plugin.eraser_mode = EraserMode::WholeStroke;

        // Add two strokes: one at (50, 50), another at (200, 200)
        let stroke1 = VectorStroke {
            id: "s1".into(),
            tool: StrokeToolKind::Pen,
            pen_type: PenType::Fountain,
            points: vec![
                StrokePoint::new(50.0, 50.0, 1.0, 0),
                StrokePoint::new(55.0, 55.0, 1.0, 10),
            ],
            color: RgbaColor::BLACK,
            base_width: 2.0,
            is_straight: false,
        };
        let stroke2 = VectorStroke {
            id: "s2".into(),
            tool: StrokeToolKind::Pen,
            pen_type: PenType::Fountain,
            points: vec![
                StrokePoint::new(200.0, 200.0, 1.0, 0),
                StrokePoint::new(210.0, 210.0, 1.0, 10),
            ],
            color: RgbaColor::BLACK,
            base_width: 2.0,
            is_straight: false,
        };

        plugin.doc.pages[0].layers[0].strokes.push(stroke1);
        plugin.doc.pages[0].layers[0].strokes.push(stroke2);
        assert_eq!(plugin.doc.pages[0].layers[0].strokes.len(), 2);

        // Quick erase near s1 (52, 52)
        plugin.erase_at_point(0, pos2(52.0, 52.0));
        assert_eq!(plugin.doc.pages[0].layers[0].strokes.len(), 1);
        assert_eq!(plugin.doc.pages[0].layers[0].strokes[0].id, "s2");
        assert!(plugin.dirty);
    }

    #[test]
    fn test_smooth_stroke_interpolation_with_pressure() {
        let stroke = VectorStroke {
            id: "test_stroke".into(),
            tool: StrokeToolKind::Pen,
            pen_type: PenType::Brush,
            points: vec![
                StrokePoint::new(10.0, 10.0, 0.2, 0),
                StrokePoint::new(30.0, 40.0, 0.8, 10),
                StrokePoint::new(60.0, 70.0, 1.5, 20),
                StrokePoint::new(100.0, 120.0, 0.4, 30),
            ],
            color: RgbaColor::BLUE,
            base_width: 3.0,
            is_straight: false,
        };

        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(200.0, 200.0), Sense::hover());
                let painter = ui.painter_at(rect);
                // Verify Catmull-Rom stroke with variable pressure draws without issue
                DrawingPlugin::draw_smooth_stroke(&painter, &stroke, |p| p, 1.0, 1.0, false);
            });
        });
        out.textures_delta.clear();
    }
}

