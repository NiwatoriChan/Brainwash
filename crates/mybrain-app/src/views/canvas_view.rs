//! Obsidian-compatible 2D Visual Canvas editor and viewer.
//!
//! Features:
//! - Infinite 2D space with smooth pan and zoom.
//! - Obsidian JSON Canvas standard (.canvas).
//! - Text cards with Markdown and clickable `[[WikiLinks]]`.
//! - File cards embedding existing vault notes with direct "Open ↗" buttons.
//! - Directional edges and arrows connecting cards.
//! - Drag-and-drop card positioning and snap-to-grid.

use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Id, Margin,
    PointerButton, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2,
};
use mybrain_core::canvas::CanvasNode;
use mybrain_core::note::NoteId;

use crate::state::AppState;
use crate::theme::{self, Tokens};

/// UI state for the interactive 2D canvas view.
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasState {
    pub pan: Vec2,
    pub zoom: f32,
    pub selected_node: Option<String>,
    pub dragging_node: Option<String>,
    pub connecting_from: Option<String>,
    pub snap_to_grid: bool,
    pub editing_text_id: Option<String>,
    pub note_picker_open: bool,
    pub note_picker_query: String,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
            selected_node: None,
            dragging_node: None,
            connecting_from: None,
            snap_to_grid: true,
            editing_text_id: None,
            note_picker_open: false,
            note_picker_query: String::new(),
        }
    }
}

/// Render the canvas view.
pub fn show(ui: &mut Ui, state: &mut AppState) {
    let t = Tokens::get(ui.ctx());

    // Ensure a canvas is loaded; if none is active, find or create one
    if state.canvas_doc.is_none() {
        if let Some(first) = state.vault.list_canvas_files().first().cloned() {
            state.open_canvas(&first);
        } else {
            // Render an empty canvas placeholder with a "Create First Canvas" button
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.heading(RichText::new("Visual 2D Canvas").size(22.0).color(t.text));
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Create interactive visual diagrams, mind maps, and note connections.")
                        .color(t.text_dim)
                        .size(13.5),
                );
                ui.add_space(20.0);
                if ui
                    .add(
                        egui::Button::new(RichText::new("+ Create New Canvas").strong().color(Color32::WHITE))
                            .fill(t.accent)
                            .stroke(Stroke::NONE)
                            .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                    )
                    .clicked()
                {
                    let _ = state.create_canvas(None, "Diagram");
                }
            });
            return;
        }
    }

    let Some(doc) = state.canvas_doc.clone() else {
        return;
    };

    let (response_rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    let painter = ui.painter_at(response_rect);
    let center = response_rect.center();

    let zoom = state.canvas_state.zoom;
    let pan = state.canvas_state.pan;

    // Coordinate conversion closures
    let to_screen = |wx: f32, wy: f32| -> Pos2 {
        pos2(center.x + (wx + pan.x) * zoom, center.y + (wy + pan.y) * zoom)
    };
    let to_world = |sp: Pos2| -> (f32, f32) {
        (
            (sp.x - center.x) / zoom - pan.x,
            (sp.y - center.y) / zoom - pan.y,
        )
    };

    // Pan & Zoom input processing
    if response_rect.contains(ui.input(|i| i.pointer.hover_pos().unwrap_or(Pos2::ZERO))) {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.1 {
            let old_zoom = state.canvas_state.zoom;
            let factor = if scroll > 0.0 { 1.1 } else { 0.9 };
            let new_zoom = (old_zoom * factor).clamp(0.25, 2.5);
            state.canvas_state.zoom = new_zoom;
        }

        let is_panning = ui.input(|i| {
            i.pointer.button_down(PointerButton::Middle)
                || i.pointer.button_down(PointerButton::Secondary)
                || (i.key_down(egui::Key::Space) && i.pointer.primary_down())
        });

        if is_panning {
            let delta = ui.input(|i| i.pointer.delta());
            state.canvas_state.pan += delta / zoom;
        }
    }

    // Draw subtle grid dots
    let grid_step = 32.0 * zoom;
    if grid_step > 8.0 {
        let x_offset = (center.x + pan.x * zoom).rem_euclid(grid_step);
        let y_offset = (center.y + pan.y * zoom).rem_euclid(grid_step);

        let mut x = response_rect.left() + x_offset;
        while x <= response_rect.right() {
            let mut y = response_rect.top() + y_offset;
            while y <= response_rect.bottom() {
                painter.circle_filled(pos2(x, y), 1.0, t.separator);
                y += grid_step;
            }
            x += grid_step;
        }
    }

    let mut note_to_open: Option<NoteId> = None;
    let mut delete_node_id: Option<String> = None;
    let mut node_moved: Option<(String, f32, f32)> = None;
    let mut new_edge: Option<(String, String)> = None;
    let mut add_text_at: Option<(f32, f32)> = None;
    let mut add_file_at: Option<(String, f32, f32)> = None;
    let mut clicked_bg = false;

    // Background click clears selection
    let bg_resp = ui.interact(response_rect, Id::NULL.with("canvas_bg_interact"), Sense::click());
    if bg_resp.clicked() {
        clicked_bg = true;
    }

    // Render Edges
    for edge in &doc.edges {
        let from_node = doc.find_node(&edge.from_node);
        let to_node = doc.find_node(&edge.to_node);

        if let (Some(from), Some(to)) = (from_node, to_node) {
            let (fx, fy) = from.pos();
            let (fw, fh) = from.size();
            let (tx, ty) = to.pos();
            let (th, _) = (to.size().1, to.size().0);

            let start = to_screen(fx + fw, fy + fh * 0.5);
            let end = to_screen(tx, ty + th * 0.5);

            let edge_color = t.accent_border;
            painter.line_segment([start, end], Stroke::new(1.8 * zoom, edge_color));

            // Arrow head at target
            let dir = (end - start).normalized();
            let normal = vec2(-dir.y, dir.x);
            let arrow_len = 10.0 * zoom;
            let arrow_w = 5.0 * zoom;
            let tip = end;
            let p1 = tip - dir * arrow_len + normal * arrow_w;
            let p2 = tip - dir * arrow_len - normal * arrow_w;
            painter.add(egui::epaint::Shape::convex_polygon(vec![tip, p1, p2], edge_color, Stroke::NONE));

            // Optional edge label
            if let Some(lbl) = &edge.label {
                let mid = start + (end - start) * 0.5;
                let font = FontId::proportional(11.0 * zoom);
                painter.text(mid, Align2::CENTER_CENTER, lbl, font, t.text_dim);
            }
        }
    }

    // Render Nodes
    for node in &doc.nodes {
        let (nx, ny) = node.pos();
        let (nw, nh) = node.size();
        let screen_min = to_screen(nx, ny);
        let screen_size = vec2(nw * zoom, nh * zoom);
        let screen_rect = Rect::from_min_size(screen_min, screen_size);

        if !response_rect.intersects(screen_rect) {
            continue;
        }

        let is_selected = state.canvas_state.selected_node.as_deref() == Some(node.id());

        // Card base
        let card_color = t.card;
        let border_stroke = if is_selected {
            Stroke::new(2.0, t.accent)
        } else {
            Stroke::new(1.0, t.card_border)
        };
        painter.rect_filled(screen_rect, CornerRadius::same(6), card_color);
        painter.rect_stroke(screen_rect, CornerRadius::same(6), border_stroke, egui::StrokeKind::Inside);

        // Header bar / Drag handle
        let header_rect = Rect::from_min_size(screen_rect.min, vec2(screen_rect.width(), 24.0 * zoom));
        let header_resp = ui.interact(
            header_rect,
            Id::NULL.with(("canvas_node_header", node.id())),
            Sense::click_and_drag(),
        );

        if header_resp.clicked() {
            state.canvas_state.selected_node = Some(node.id().to_owned());
        }

        if header_resp.dragged() {
            let delta = ui.input(|i| i.pointer.delta()) / zoom;
            let (mut new_x, mut new_y) = (nx + delta.x, ny + delta.y);
            if state.canvas_state.snap_to_grid {
                new_x = (new_x / 16.0).round() * 16.0;
                new_y = (new_y / 16.0).round() * 16.0;
            }
            node_moved = Some((node.id().to_owned(), new_x, new_y));
        }

        // Header title & icon
        match node {
            CanvasNode::Text { .. } => {
                painter.text(
                    header_rect.left_center() + vec2(8.0 * zoom, 0.0),
                    Align2::LEFT_CENTER,
                    "Note Card",
                    FontId::proportional(11.0 * zoom),
                    t.text_dim,
                );
            }
            CanvasNode::File { file, .. } => {
                let note_name = file.rsplit('/').next().unwrap_or(file);
                painter.text(
                    header_rect.left_center() + vec2(8.0 * zoom, 0.0),
                    Align2::LEFT_CENTER,
                    format!("📄 {note_name}"),
                    FontId::proportional(12.0 * zoom),
                    t.accent_text,
                );

                // Quick open button on file card
                let open_btn_rect = Rect::from_min_size(
                    pos2(header_rect.right() - 28.0 * zoom, header_rect.top() + 2.0 * zoom),
                    vec2(24.0 * zoom, 20.0 * zoom),
                );
                let open_resp = ui.interact(
                    open_btn_rect,
                    Id::NULL.with(("open_canvas_note", node.id())),
                    Sense::click(),
                );
                if open_resp.hovered() {
                    painter.rect_filled(open_btn_rect, CornerRadius::same(3), t.hover);
                }
                painter.text(
                    open_btn_rect.center(),
                    Align2::CENTER_CENTER,
                    "↗",
                    FontId::proportional(13.0 * zoom),
                    t.text_dim,
                );
                if open_resp.clicked() {
                    if let Ok(id) = NoteId::new(file) {
                        note_to_open = Some(id);
                    }
                }
            }
            CanvasNode::Link { url, .. } => {
                painter.text(
                    header_rect.left_center() + vec2(8.0 * zoom, 0.0),
                    Align2::LEFT_CENTER,
                    format!("🔗 {url}"),
                    FontId::proportional(11.0 * zoom),
                    t.accent_text,
                );
            }
            CanvasNode::Group { label, .. } => {
                let lbl = label.as_deref().unwrap_or("Group");
                painter.text(
                    header_rect.left_center() + vec2(8.0 * zoom, 0.0),
                    Align2::LEFT_CENTER,
                    lbl,
                    FontId::proportional(12.0 * zoom),
                    t.text,
                );
            }
        }

        // Delete button on selected node
        if is_selected {
            let del_rect = Rect::from_min_size(
                pos2(header_rect.right() - 20.0 * zoom, header_rect.top() + 2.0 * zoom),
                vec2(16.0 * zoom, 16.0 * zoom),
            );
            let del_resp = ui.interact(
                del_rect,
                Id::NULL.with(("canvas_node_del", node.id())),
                Sense::click(),
            );
            if del_resp.hovered() {
                painter.rect_filled(del_rect, CornerRadius::same(3), t.danger);
            }
            painter.text(
                del_rect.center(),
                Align2::CENTER_CENTER,
                "×",
                FontId::proportional(12.0 * zoom),
                if del_resp.hovered() { Color32::WHITE } else { t.text_faint },
            );
            if del_resp.clicked() {
                delete_node_id = Some(node.id().to_owned());
            }

            // Connection handle on right edge
            let handle_center = pos2(screen_rect.right(), screen_rect.center().y);
            let handle_rect = Rect::from_center_size(handle_center, vec2(12.0 * zoom, 12.0 * zoom));
            let handle_resp = ui.interact(
                handle_rect,
                Id::NULL.with(("canvas_connect_handle", node.id())),
                Sense::click(),
            );
            painter.circle_filled(handle_center, 4.0 * zoom, t.accent);

            if handle_resp.clicked() {
                if let Some(from_id) = &state.canvas_state.connecting_from {
                    if from_id != node.id() {
                        new_edge = Some((from_id.clone(), node.id().to_owned()));
                    }
                    state.canvas_state.connecting_from = None;
                } else {
                    state.canvas_state.connecting_from = Some(node.id().to_owned());
                }
            }
        }

        // Card Body Content
        let body_rect = Rect::from_min_max(
            pos2(screen_rect.left() + 10.0 * zoom, header_rect.bottom() + 4.0 * zoom),
            pos2(screen_rect.right() - 10.0 * zoom, screen_rect.bottom() - 6.0 * zoom),
        );

        match node {
            CanvasNode::Text { text, .. } => {
                let text_font = FontId::proportional(12.0 * zoom);
                let lines: Vec<&str> = text.lines().take(6).collect();
                let mut line_y = body_rect.top();
                for l in lines {
                    if line_y + 14.0 * zoom > body_rect.bottom() {
                        break;
                    }
                    // Wikilink detection inside text cards
                    if l.contains("[[") && l.contains("]]") {
                        let link_target = l.split("[[").nth(1).and_then(|s| s.split("]]").next());
                        if let Some(target) = link_target {
                            let text_resp = ui.interact(
                                Rect::from_min_size(pos2(body_rect.left(), line_y), vec2(body_rect.width(), 16.0 * zoom)),
                                Id::NULL.with(("canvas_wikilink", node.id(), l)),
                                Sense::click(),
                            );
                            painter.text(
                                pos2(body_rect.left(), line_y),
                                Align2::LEFT_TOP,
                                format!("🔗 [[{target}]]"),
                                text_font.clone(),
                                t.accent_text,
                            );
                            if text_resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                                note_to_open = state.vault.resolve_link(target).cloned();
                            }
                        }
                    } else {
                        painter.text(
                            pos2(body_rect.left(), line_y),
                            Align2::LEFT_TOP,
                            l,
                            text_font.clone(),
                            t.text,
                        );
                    }
                    line_y += 16.0 * zoom;
                }
            }
            CanvasNode::File { file, .. } => {
                if let Ok(id) = NoteId::new(file) {
                    if let Some(note) = state.vault.get(&id) {
                        let snippet = note.body().lines().take(4).collect::<Vec<_>>().join("\n");
                        let text_font = FontId::proportional(11.0 * zoom);
                        painter.text(
                            pos2(body_rect.left(), body_rect.top()),
                            Align2::LEFT_TOP,
                            snippet,
                            text_font,
                            t.text_dim,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    if clicked_bg {
        state.canvas_state.selected_node = None;
        state.canvas_state.connecting_from = None;
    }

    // Floating Canvas Bottom Control Toolbar
    let toolbar_w = 340.0;
    let toolbar_rect = Rect::from_center_size(
        pos2(response_rect.center().x, response_rect.bottom() - 32.0),
        vec2(toolbar_w, 40.0),
    );

    Frame::NONE
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.card_border))
        .corner_radius(CornerRadius::same(t.radius as u8))
        .shadow(egui::Shadow {
            offset: [0, 6],
            blur: 16,
            spread: 0,
            color: t.shadow,
        })
        .inner_margin(Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.scope_builder(egui::UiBuilder::new().max_rect(toolbar_rect), |ui| {
                ui.horizontal_centered(|ui| {
                    if theme::ghost_button(ui, &t, RichText::new("+ Text Card").color(t.text))
                        .on_hover_text("Add text card at canvas center")
                        .clicked()
                    {
                        let (wx, wy) = to_world(response_rect.center());
                        add_text_at = Some((wx, wy));
                    }

                    ui.separator();

                    if theme::ghost_button(ui, &t, RichText::new("+ Note Card").color(t.accent_text))
                        .on_hover_text("Embed an existing note on canvas")
                        .clicked()
                    {
                        state.canvas_state.note_picker_open = true;
                    }

                    ui.separator();

                    // Snap to grid toggle
                    let snap_label = if state.canvas_state.snap_to_grid { "Snap: ON" } else { "Snap: OFF" };
                    if theme::ghost_button(ui, &t, RichText::new(snap_label).color(t.text_dim)).clicked() {
                        state.canvas_state.snap_to_grid = !state.canvas_state.snap_to_grid;
                    }

                    ui.separator();

                    // Zoom reset
                    let zoom_pct = format!("{:.0}%", state.canvas_state.zoom * 100.0);
                    if theme::ghost_button(ui, &t, RichText::new(zoom_pct).color(t.text_dim))
                        .on_hover_text("Reset Zoom to 100%")
                        .clicked()
                    {
                        state.canvas_state.zoom = 1.0;
                        state.canvas_state.pan = Vec2::ZERO;
                    }
                });
            });
        });

    // Quick Note Picker Modal for adding note cards to canvas
    if state.canvas_state.note_picker_open {
        let modal = egui::Modal::new(Id::unique("canvas_note_picker_modal"))
            .backdrop_color(Color32::from_black_alpha(150))
            .frame(theme::modal_frame(&t));

        let mut note_selected = None;
        let mut close_modal = false;

        modal.show(ui.ctx(), |ui| {
            ui.set_width(420.0);
            ui.horizontal(|ui| {
                ui.heading(RichText::new("Add Note to Canvas").color(t.text).strong().size(15.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::icon_close_button(ui, &t).clicked() {
                        close_modal = true;
                    }
                });
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                theme::icon_search(ui, 13.0, t.text_dim);
                ui.add(
                    egui::TextEdit::singleline(&mut state.canvas_state.note_picker_query)
                        .hint_text("Search note to embed…")
                        .desired_width(ui.available_width()),
                );
            });

            ui.add_space(8.0);
            egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                let query = state.canvas_state.note_picker_query.to_lowercase();
                for note in state.vault.notes() {
                    if query.is_empty() || note.id.as_str().to_lowercase().contains(&query) {
                        let dot = theme::note_kind_color(note.id.as_str(), &t);
                        if theme::list_row(ui, &t, false, Some(dot), note.id.name(), note.id.folder()).clicked() {
                            note_selected = Some(note.id.as_str().to_owned());
                            close_modal = true;
                        }
                    }
                }
            });
        });

        if let Some(file_path) = note_selected {
            let (wx, wy) = to_world(response_rect.center());
            add_file_at = Some((file_path, wx, wy));
        }

        if close_modal {
            state.canvas_state.note_picker_open = false;
            state.canvas_state.note_picker_query.clear();
        }
    }

    // Drag-and-drop linking of notes onto canvas
    if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
        if response_rect.contains(hover_pos) {
            if let Some(dragged) = egui::DragAndDrop::payload::<crate::state::DraggedNote>(ui.ctx()) {
                let ghost_rect = Rect::from_center_size(hover_pos, vec2(240.0 * zoom, 140.0 * zoom));
                painter.rect(
                    ghost_rect,
                    CornerRadius::same(t.radius as u8),
                    t.card,
                    Stroke::new(2.0, t.accent),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    ghost_rect.center(),
                    Align2::CENTER_CENTER,
                    format!("Drop [[{}]]", dragged.name),
                    FontId::proportional((13.0 * zoom).clamp(10.0, 18.0)),
                    t.accent_text,
                );
            }

            if ui.input(|i| i.pointer.any_released()) {
                if let Some(dragged) = egui::DragAndDrop::take_payload::<crate::state::DraggedNote>(ui.ctx()) {
                    let (wx, wy) = to_world(hover_pos);
                    let file_path = if dragged.is_canvas {
                        dragged.id.clone()
                    } else if dragged.id.ends_with(".md") {
                        dragged.id.clone()
                    } else {
                        format!("{}.md", dragged.id)
                    };
                    add_file_at = Some((file_path, wx, wy));
                    state.set_status(format!("Added [[{}]] to canvas", dragged.name));
                }
            }
        }
    }

    // Apply recorded mutations to state.canvas_doc
    if let Some((from_id, to_id)) = new_edge {
        if let Some(ref mut d) = state.canvas_doc {
            d.add_edge(from_id, to_id, None);
            state.canvas_dirty = true;
        }
    }

    if let Some((id, nx, ny)) = node_moved {
        if let Some(ref mut d) = state.canvas_doc {
            if let Some(node) = d.find_node_mut(&id) {
                node.set_pos(nx, ny);
                state.canvas_dirty = true;
            }
        }
    }

    if let Some(id) = delete_node_id {
        if let Some(ref mut d) = state.canvas_doc {
            d.remove_node(&id);
            state.canvas_state.selected_node = None;
            state.canvas_dirty = true;
        }
    }

    if let Some((wx, wy)) = add_text_at {
        if let Some(ref mut d) = state.canvas_doc {
            let id = d.add_text_node("New Idea\nType Markdown or [[Links]] here…", wx - 100.0, wy - 60.0, 220.0, 140.0);
            state.canvas_state.selected_node = Some(id);
            state.canvas_dirty = true;
        }
    }

    if let Some((file_path, wx, wy)) = add_file_at {
        if let Some(ref mut d) = state.canvas_doc {
            let id = d.add_file_node(file_path, wx - 120.0, wy - 80.0, 260.0, 160.0);
            state.canvas_state.selected_node = Some(id);
            state.canvas_dirty = true;
        }
    }

    // Navigate to note if requested from canvas
    if let Some(id) = note_to_open {
        state.top_mode = crate::state::TopBarMode::Notes;
        state.open_note(id);
    }

    // Auto-save canvas if modified
    if state.canvas_dirty {
        state.save_canvas();
    }
}
