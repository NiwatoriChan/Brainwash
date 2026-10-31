//! Obsidian-style interactive 2D knowledge graph view.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use egui::{
    pos2, vec2, Align2, CornerRadius, CursorIcon, FontId, Frame, Margin, Pos2, Rect, RichText,
    Sense, Stroke, Ui, UiBuilder, Vec2,
};
use mybrain_core::NoteId;

use crate::state::{AppState, SidebarSection, TopBarMode};
use crate::theme::Tokens;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GraphNode {
    pub id: NoteId,
    pub title: String,
    pub pos: Pos2,
    pub vel: Vec2,
    pub radius: f32,
    pub degree: usize,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: NoteId,
    pub to: NoteId,
}

#[derive(Debug, Clone)]
pub struct GraphState {
    pub nodes: HashMap<NoteId, GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub pan: Vec2,
    pub zoom: f32,
    pub dragged_node: Option<NoteId>,
    pub hovered_node: Option<NoteId>,
    pub initialized: bool,
    pub last_signature: u64,
    pub local_graph_only: bool,
}

impl Default for GraphState {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            pan: Vec2::ZERO,
            zoom: 1.0,
            dragged_node: None,
            hovered_node: None,
            initialized: false,
            last_signature: 0,
            local_graph_only: false,
        }
    }
}

impl GraphState {
    /// Synchronize graph nodes and edges with the current vault and link index.
    pub fn sync(
        &mut self,
        vault: &mybrain_core::vault::Vault,
        link_index: &mybrain_core::LinkIndex,
        active_note: Option<&NoteId>,
    ) {
        let mut hasher = DefaultHasher::new();
        self.local_graph_only.hash(&mut hasher);
        if self.local_graph_only {
            active_note.hash(&mut hasher);
        }
        for note in vault.notes() {
            note.id.hash(&mut hasher);
            for out in link_index.outgoing(&note.id) {
                out.resolved.hash(&mut hasher);
            }
        }
        let signature = hasher.finish();
        if self.initialized && signature == self.last_signature {
            return;
        }

        self.last_signature = signature;
        let mut new_nodes = HashMap::new();
        let mut new_edges = Vec::new();
        let all_note_ids: Vec<NoteId> = vault.notes().map(|n| n.id.clone()).collect();

        // Filter to 1-hop neighborhood if local_graph_only is enabled and active_note exists
        let allowed_ids: Option<HashSet<NoteId>> = if self.local_graph_only {
            active_note.map(|center_id| {
                let mut set = HashSet::new();
                set.insert(center_id.clone());
                for out in link_index.outgoing(center_id) {
                    if let Some(target) = &out.resolved {
                        set.insert(target.clone());
                    }
                }
                for bl in link_index.backlinks(center_id) {
                    set.insert(bl.source.clone());
                }
                set
            })
        } else {
            None
        };

        let note_ids: Vec<NoteId> = all_note_ids
            .into_iter()
            .filter(|id| allowed_ids.as_ref().map_or(true, |s| s.contains(id)))
            .collect();
        let n = note_ids.len();

        // Calculate degrees
        let mut degrees: HashMap<NoteId, usize> = HashMap::new();
        for id in &note_ids {
            for out in link_index.outgoing(id) {
                if let Some(target) = &out.resolved {
                    if allowed_ids.as_ref().map_or(true, |s| s.contains(target)) {
                        *degrees.entry(id.clone()).or_insert(0) += 1;
                        *degrees.entry(target.clone()).or_insert(0) += 1;
                        new_edges.push(GraphEdge {
                            from: id.clone(),
                            to: target.clone(),
                        });
                    }
                }
            }
        }

        let radius_step = 2.0 * std::f32::consts::PI / (n.max(1) as f32);
        let spread = 120.0 + (n as f32) * 18.0;

        for (i, id) in note_ids.into_iter().enumerate() {
            let degree = degrees.get(&id).copied().unwrap_or(0);
            let radius = (6.0 + (degree as f32) * 2.2).clamp(6.0, 22.0);

            let initial_pos = if let Some(existing) = self.nodes.get(&id) {
                existing.pos
            } else {
                let angle = (i as f32) * radius_step;
                pos2(angle.cos() * spread, angle.sin() * spread)
            };

            let title = id.name().to_string();
            new_nodes.insert(
                id.clone(),
                GraphNode {
                    id,
                    title,
                    pos: initial_pos,
                    vel: Vec2::ZERO,
                    radius,
                    degree,
                },
            );
        }

        self.nodes = new_nodes;
        self.edges = new_edges;
        self.initialized = true;
    }

    /// Run one step of the spring-electrical force-directed simulation.
    /// Returns `true` if any node is still moving above the sleep threshold.
    pub fn step_physics(&mut self, is_dragging: bool) -> bool {
        let node_ids: Vec<NoteId> = self.nodes.keys().cloned().collect();
        let n = node_ids.len();
        if n == 0 {
            return false;
        }

        let k_repulsion = 7500.0;
        let k_spring = 0.045;
        let spring_len = 110.0;
        let k_center = 0.008;
        let damping = 0.82;
        let max_speed = 18.0;

        let mut forces: HashMap<NoteId, Vec2> = HashMap::with_capacity(n);
        for id in &node_ids {
            forces.insert(id.clone(), Vec2::ZERO);
        }

        // 1. Repulsion between all pairs
        for i in 0..n {
            let id1 = &node_ids[i];
            let pos1 = self.nodes[id1].pos;
            for j in (i + 1)..n {
                let id2 = &node_ids[j];
                let pos2 = self.nodes[id2].pos;

                let delta = pos1 - pos2;
                let dist = delta.length().max(18.0);
                let dir = delta / dist;
                let force = dir * (k_repulsion / (dist * dist));

                *forces.get_mut(id1).unwrap() += force;
                *forces.get_mut(id2).unwrap() -= force;
            }
        }

        // 2. Spring attraction along edges
        for edge in &self.edges {
            if let (Some(n1), Some(n2)) = (self.nodes.get(&edge.from), self.nodes.get(&edge.to)) {
                let delta = n2.pos - n1.pos;
                let dist = delta.length().max(1.0);
                let dir = delta / dist;
                let force = dir * ((dist - spring_len) * k_spring);

                if let Some(f) = forces.get_mut(&edge.from) {
                    *f += force;
                }
                if let Some(f) = forces.get_mut(&edge.to) {
                    *f -= force;
                }
            }
        }

        // 3. Central gravitational pull & integrate
        let mut any_moving = is_dragging;
        for (id, node) in self.nodes.iter_mut() {
            if Some(id) == self.dragged_node.as_ref() && is_dragging {
                node.vel = Vec2::ZERO;
                continue;
            }

            let mut f = forces.get(id).copied().unwrap_or(Vec2::ZERO);
            f -= node.pos.to_vec2() * k_center;

            node.vel = (node.vel + f) * damping;
            if node.vel.length() > max_speed {
                node.vel = node.vel.normalized() * max_speed;
            }
            if node.vel.length_sq() > 0.0016 {
                node.pos += node.vel;
                any_moving = true;
            } else {
                node.vel = Vec2::ZERO;
            }
        }
        any_moving
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let t = Tokens::get(ui.ctx());

    state
        .graph
        .sync(&state.vault, &state.link_index, state.active_note.as_ref());

    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = response.rect;
    let center = rect.center();

    // Pan & Zoom gestures
    let mut pan = state.graph.pan;
    let mut zoom = state.graph.zoom;

    // Zoom via mouse scroll wheel
    let scroll_delta: f32 = ui.input(|i| i.smooth_scroll_delta.y);
    if response.hovered() && scroll_delta.abs() > 0.0 {
        let factor = (1.0_f32 + scroll_delta * 0.0018_f32).clamp(0.85_f32, 1.15_f32);
        zoom = (zoom * factor).clamp(0.25, 3.5);
    }

    let mouse_pos = ui.input(|i| i.pointer.hover_pos());
    let is_dragging = response.dragged();

    // Pan when dragging empty canvas
    if is_dragging && state.graph.dragged_node.is_none() {
        pan += response.drag_delta();
    }

    state.graph.pan = pan;
    state.graph.zoom = zoom;

    // World to screen transform helpers
    let to_screen = |pos: Pos2| -> Pos2 { center + pan + pos.to_vec2() * zoom };
    let to_world = |screen_pos: Pos2| -> Pos2 {
        let offset = screen_pos - center - pan;
        pos2(offset.x / zoom, offset.y / zoom)
    };

    // Node dragging logic
    if is_dragging {
        if let Some(dragged_id) = &state.graph.dragged_node {
            if let Some(mouse) = mouse_pos {
                if let Some(node) = state.graph.nodes.get_mut(dragged_id) {
                    node.pos = to_world(mouse);
                    node.vel = Vec2::ZERO;
                }
            }
        }
    } else {
        state.graph.dragged_node = None;
    }

    // Physics step (only request repaint while nodes are still settling or being dragged)
    if state.graph.step_physics(is_dragging) {
        ui.ctx().request_repaint();
    }

    // Hover detection
    let mut hovered_node = None;
    if let Some(mouse) = mouse_pos {
        for (id, node) in &state.graph.nodes {
            let screen_node_pos = to_screen(node.pos);
            let hit_radius = (node.radius * zoom).max(12.0);
            if (mouse - screen_node_pos).length() <= hit_radius {
                hovered_node = Some(id.clone());
                break;
            }
        }
    }

    // If mouse was pressed on a node, set as dragged
    if response.drag_started() {
        if let Some(h) = &hovered_node {
            state.graph.dragged_node = Some(h.clone());
        }
    }

    state.graph.hovered_node = hovered_node.clone();

    // Highlighted neighbors for the hovered node
    let mut highlighted_nodes = HashSet::new();
    let mut highlighted_edges = HashSet::new();
    if let Some(h) = &hovered_node {
        highlighted_nodes.insert(h.clone());
        for (idx, edge) in state.graph.edges.iter().enumerate() {
            if &edge.from == h {
                highlighted_nodes.insert(edge.to.clone());
                highlighted_edges.insert(idx);
            } else if &edge.to == h {
                highlighted_nodes.insert(edge.from.clone());
                highlighted_edges.insert(idx);
            }
        }
    }

    // Draw grid background dots for spatial feel
    let grid_size = 40.0 * zoom;
    if grid_size > 14.0 {
        let x_start = (center.x + pan.x) % grid_size;
        let y_start = (center.y + pan.y) % grid_size;
        let dot_color = t.separator.gamma_multiply(0.4);
        let mut x = x_start;
        while x < rect.right() {
            let mut y = y_start;
            while y < rect.bottom() {
                if rect.contains(pos2(x, y)) {
                    painter.circle_filled(pos2(x, y), 1.0, dot_color);
                }
                y += grid_size;
            }
            x += grid_size;
        }
    }

    // 1. Draw Edges
    for (idx, edge) in state.graph.edges.iter().enumerate() {
        if let (Some(n1), Some(n2)) = (state.graph.nodes.get(&edge.from), state.graph.nodes.get(&edge.to)) {
            let p1 = to_screen(n1.pos);
            let p2 = to_screen(n2.pos);

            let is_edge_highlighted = highlighted_edges.contains(&idx);
            let stroke = if is_edge_highlighted {
                Stroke::new(1.8 * zoom.clamp(0.8, 1.5), t.accent)
            } else if hovered_node.is_some() {
                Stroke::new(0.8, t.card_border.gamma_multiply(0.2))
            } else {
                Stroke::new(1.0, t.separator.gamma_multiply(0.7))
            };

            painter.line_segment([p1, p2], stroke);
        }
    }

    // 2. Draw Nodes
    let mut clicked_node = None;
    let active_id = state.active_note.clone();

    for (id, node) in &state.graph.nodes {
        let screen_pos = to_screen(node.pos);
        let r = (node.radius * zoom).max(4.0);

        let is_active = active_id.as_ref() == Some(id);
        let is_hovered = hovered_node.as_ref() == Some(id);
        let is_connected = highlighted_nodes.contains(id);

        let base_color = if id.as_str().starts_with("Daily/") {
            t.accent
        } else if id.as_str().starts_with("Events/") {
            t.warning
        } else if id.as_str().starts_with("Quick Notes/") {
            t.success
        } else {
            t.accent_text
        };

        let node_color = if is_hovered || is_active {
            base_color
        } else if hovered_node.is_some() && !is_connected {
            base_color.gamma_multiply(0.25)
        } else {
            base_color.gamma_multiply(0.85)
        };

        if is_hovered || is_active {
            painter.circle_filled(screen_pos, r + 4.0 * zoom.clamp(0.8, 2.0), t.accent_soft);
        }

        painter.circle_filled(screen_pos, r, node_color);
        painter.circle_stroke(screen_pos, r, Stroke::new(1.0, t.card));

        let show_label = zoom > 0.75 || is_hovered || is_active || is_connected;
        if show_label {
            let label_color = if is_hovered || is_active {
                t.text
            } else if hovered_node.is_some() && !is_connected {
                t.text_faint.gamma_multiply(0.3)
            } else {
                t.text_dim
            };

            let font_size = (11.0 * zoom).clamp(9.0, 14.0);
            painter.text(
                screen_pos + vec2(0.0, r + 3.0),
                Align2::CENTER_TOP,
                &node.title,
                FontId::proportional(font_size),
                label_color,
            );
        }
    }

    // Node click handling: click to open note & jump to Note View
    if response.clicked() {
        if let Some(h) = hovered_node {
            clicked_node = Some(h);
        }
    }

    if let Some(id) = clicked_node {
        state.open_note(id);
        state.top_mode = TopBarMode::Notes;
        state.sidebar_section = SidebarSection::Notes;
    }

    // 3. Floating HUD Overlay
    let hud_rect = Rect::from_min_size(
        pos2(rect.left() + 16.0, rect.top() + 16.0),
        vec2(280.0, 48.0),
    );

    ui.scope_builder(UiBuilder::new().max_rect(hud_rect), |ui| {
        Frame::NONE
            .fill(t.card.gamma_multiply(0.92))
            .stroke(Stroke::new(1.0, t.card_border))
            .corner_radius(CornerRadius::same(t.radius as u8))
            .inner_margin(Margin::symmetric(14, 8))
            .shadow(egui::Shadow {
                offset: [0, 4],
                blur: 16,
                spread: 0,
                color: t.shadow,
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Knowledge Graph").strong().size(13.5).color(t.text));
                    ui.add_space(4.0);
                    let node_count = state.graph.nodes.len();
                    let edge_count = state.graph.edges.len();
                    crate::theme::pill(ui, &format!("{node_count} notes"), t.accent_text, t.accent_soft);
                    crate::theme::pill(ui, &format!("{edge_count} links"), t.text_dim, t.field);
                });
            });
    });

    // Top-right controls overlay: Local Graph toggle, Zoom & Reset
    let controls_rect = Rect::from_min_size(
        pos2(rect.right() - 290.0, rect.top() + 16.0),
        vec2(274.0, 42.0),
    );

    ui.scope_builder(UiBuilder::new().max_rect(controls_rect), |ui| {
        Frame::NONE
            .fill(t.card.gamma_multiply(0.92))
            .stroke(Stroke::new(1.0, t.card_border))
            .corner_radius(CornerRadius::same(t.radius as u8))
            .inner_margin(Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let zoom_pct = (state.graph.zoom * 100.0).round() as i32;
                    ui.label(RichText::new(format!("{zoom_pct}%")).color(t.text_dim).size(11.5));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if crate::theme::ghost_button(ui, &t, RichText::new("Reset View").color(t.accent_text).size(11.5)).clicked() {
                            state.graph.pan = Vec2::ZERO;
                            state.graph.zoom = 1.0;
                        }
                        let local_label = if state.graph.local_graph_only {
                            "Local (1-hop) ●"
                        } else {
                            "Local (1-hop)"
                        };
                        let local_col = if state.graph.local_graph_only {
                            t.accent_text
                        } else {
                            t.text_dim
                        };
                        if crate::theme::ghost_button(ui, &t, RichText::new(local_label).color(local_col).size(11.5))
                            .on_hover_text("Toggle between full vault graph and active note's 1-hop neighborhood")
                            .clicked()
                        {
                            state.graph.local_graph_only = !state.graph.local_graph_only;
                        }
                    });
                });
            });
    });

    // Hover tooltip with connection counts
    if let Some(h) = &state.graph.hovered_node {
        if let Some(node) = state.graph.nodes.get(h) {
            let screen_pos = to_screen(node.pos);
            let in_count = state.link_index.backlinks(h).len();
            let out_count = state.link_index.outgoing(h).len();

            let tooltip_rect = Rect::from_min_size(
                pos2(screen_pos.x + 12.0, screen_pos.y - 28.0),
                vec2(180.0, 48.0),
            );

            ui.scope_builder(UiBuilder::new().max_rect(tooltip_rect), |ui| {
                Frame::NONE
                    .fill(t.card)
                    .stroke(Stroke::new(1.0, t.accent_border))
                    .corner_radius(CornerRadius::same(t.radius_sm as u8))
                    .inner_margin(Margin::symmetric(10, 6))
                    .shadow(egui::Shadow {
                        offset: [0, 4],
                        blur: 12,
                        spread: 0,
                        color: t.shadow,
                    })
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&node.title).strong().size(12.5).color(t.text));
                            ui.label(
                                RichText::new(format!("{in_count} incoming · {out_count} outgoing"))
                                    .size(10.5)
                                    .color(t.text_faint),
                            );
                        });
                    });
            });
        }
    }

    if response.hovered() {
        ui.output_mut(|o| o.cursor_icon = CursorIcon::Grab);
    }
    if is_dragging {
        ui.output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);
    }
}
