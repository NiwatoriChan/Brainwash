//! Brainwash - Local-first personal knowledge base and graph.

mod state;
mod theme;
mod views;

use std::path::PathBuf;

use eframe::egui;
use egui::{CornerRadius, CursorIcon, Frame, Margin, RichText, Sense, Stroke};
use state::{AppState, SidebarSection, TopBarMode, ViewMode};
use theme::Tokens;

const KEY_THEME: &str = "theme_kind";
const KEY_VIEW_MODE: &str = "view_mode";
const KEY_SHOW_BACKLINKS: &str = "show_backlinks";
const KEY_SHOW_SIDEBAR: &str = "show_sidebar";
const KEY_TOP_MODE: &str = "top_mode";
const KEY_SIDEBAR_SECTION: &str = "sidebar_section";

struct BrainwashApp {
    state: AppState,
}

impl BrainwashApp {
    fn new(state: AppState) -> Self {
        Self { state }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // Ctrl+S: Save note
            if i.modifiers.command && i.key_pressed(egui::Key::S) {
                self.state.save_current();
            }
            // Ctrl+P / Ctrl+O: Command Palette
            if i.modifiers.command && (i.key_pressed(egui::Key::P) || i.key_pressed(egui::Key::O)) {
                self.state.show_command_palette = !self.state.show_command_palette;
            }
            // Ctrl+Q: Quick Capture / Keep view
            if i.modifiers.command && i.key_pressed(egui::Key::Q) {
                self.state.sidebar_section = SidebarSection::Quick;
                self.state.top_mode = TopBarMode::Notes;
                self.state.quick_composer_expanded = true;
            }
            // Ctrl+N: New Note
            if i.modifiers.command && i.key_pressed(egui::Key::N) {
                self.state.show_new_note_dialog = true;
            }
            // Ctrl+B or Ctrl+\: Toggle Sidebar
            if i.modifiers.command && (i.key_pressed(egui::Key::B) || i.key_pressed(egui::Key::Backslash)) {
                self.state.show_sidebar = !self.state.show_sidebar;
            }
            // Ctrl+1: Notes, Ctrl+2: Calendar, Ctrl+3: Graph, Ctrl+4: Canvas
            if i.modifiers.command && i.key_pressed(egui::Key::Num1) {
                self.state.top_mode = TopBarMode::Notes;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Num2) {
                self.state.top_mode = TopBarMode::Calendar;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Num3) {
                self.state.top_mode = TopBarMode::Graph;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Num4) {
                self.state.top_mode = TopBarMode::Canvas;
            }
            // Ctrl+G: Toggle Notes / Graph modes
            if i.modifiers.command && i.key_pressed(egui::Key::G) {
                self.state.top_mode = match self.state.top_mode {
                    TopBarMode::Notes => TopBarMode::Graph,
                    TopBarMode::Graph => TopBarMode::Notes,
                    TopBarMode::Calendar => TopBarMode::Notes,
                    TopBarMode::Canvas => TopBarMode::Notes,
                };
            }
            // Ctrl+,: Open Settings
            if i.modifiers.command && i.key_pressed(egui::Key::Comma) {
                self.state.show_settings_dialog = true;
            }
            // Ctrl+O: Open Vault Switcher
            if i.modifiers.command && i.key_pressed(egui::Key::O) {
                self.state.show_vault_dialog = true;
            }
            // Ctrl+E: Toggle View Mode (Preview / Edit)
            if i.modifiers.command && i.key_pressed(egui::Key::E) {
                self.state.view_mode = match self.state.view_mode {
                    ViewMode::PreviewOnly => ViewMode::EditOnly,
                    ViewMode::EditOnly => ViewMode::PreviewOnly,
                    ViewMode::Split => ViewMode::PreviewOnly,
                };
            }
            // Alt+Left / Alt+Right: Navigate history
            if i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft) {
                self.state.go_back();
            }
            if i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight) {
                self.state.go_forward();
            }
        });
    }

    /// Single application header: brand, mode switcher (Notes | Graph), search, and global actions.
    fn top_bar(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, t: &Tokens) {
        egui::Panel::top("top_bar")
            .exact_size(48.0)
            .frame(
                Frame::NONE
                    .fill(t.chrome)
                    .inner_margin(Margin::symmetric(14, 0)),
            )
            .show(ui, |ui| {
                theme::bottom_hairline(ui, ui.max_rect(), t);
                ui.horizontal_centered(|ui| {
                    // Sidebar toggle button
                    if theme::icon_sidebar_button(ui, t, self.state.show_sidebar)
                        .on_hover_text("Toggle Sidebar (Ctrl+B)")
                        .clicked()
                    {
                        self.state.show_sidebar = !self.state.show_sidebar;
                    }

                    ui.add_space(6.0);

                    // Brand mark
                    let (mark, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
                    ui.painter().rect_filled(mark, CornerRadius::same(5), t.accent);
                    ui.painter().circle_filled(mark.center(), 3.5, t.card);
                    ui.label(RichText::new("Brainwash").strong().size(15.0).color(t.text));

                    // Vault switcher dropdown
                    let vault_name = self.state.vault.root().file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Vault".to_string());
                    if theme::ghost_button(ui, t, RichText::new(format!("📁 {vault_name} ▾")).size(12.5).color(t.text_dim))
                        .on_hover_text("Open or switch vault (Ctrl+O)")
                        .clicked()
                    {
                        self.state.show_vault_dialog = true;
                    }

                    ui.add_space(10.0);

                    // Top Mode Icon Switcher: Notes (Pencil), Calendar, Graph, Canvas
                    if let Some(sel) = theme::top_mode_segmented(ui, t, self.state.top_mode) {
                        self.state.top_mode = sel;
                    }

                    ui.add_space(12.0);

                    // Search field-styled button that opens the command palette
                    let search_w = (ui.available_width() - 330.0).clamp(140.0, 360.0);
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(search_w, 28.0), Sense::click());
                    let hovered = resp.hovered();
                    ui.painter().rect(
                        rect,
                        CornerRadius::same(t.radius as u8),
                        if hovered { t.hover } else { t.field },
                        Stroke::new(1.0, t.field_border),
                        egui::StrokeKind::Inside,
                    );
                    theme::paint_search_icon(
                        ui.painter(),
                        rect.left_center() + egui::vec2(14.0, 0.0),
                        13.0,
                        t.text_faint,
                    );
                    ui.painter().text(
                        rect.left_center() + egui::vec2(28.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        "Search notes…",
                        egui::FontId::proportional(12.5),
                        t.text_faint,
                    );
                    ui.painter().text(
                        rect.right_center() - egui::vec2(10.0, 0.0),
                        egui::Align2::RIGHT_CENTER,
                        "Ctrl P",
                        egui::FontId::monospace(10.5),
                        t.text_faint,
                    );
                    if resp.on_hover_cursor(CursorIcon::Text).clicked() {
                        self.state.show_command_palette = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Connections panel toggle
                        let links_text = RichText::new("Links").color(if self.state.show_backlinks {
                            t.accent_text
                        } else {
                            t.text_dim
                        });
                        if theme::ghost_button(ui, t, links_text)
                            .on_hover_text("Toggle connections panel")
                            .clicked()
                        {
                            self.state.show_backlinks = !self.state.show_backlinks;
                        }

                        // Settings button
                        if theme::ghost_button(ui, t, RichText::new("Settings").color(t.text_dim))
                            .on_hover_text("Preferences & Settings (Ctrl+,)")
                            .clicked()
                        {
                            self.state.show_settings_dialog = true;
                        }

                        ui.add_space(4.0);

                        // Create menu
                        let new_btn = egui::Button::new(
                            RichText::new("+ New").strong().color(egui::Color32::WHITE),
                        )
                        .fill(t.accent)
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(t.radius_sm as u8));
                        egui::containers::menu::MenuButton::from_button(new_btn).ui(ui, |ui| {
                            ui.set_min_width(190.0);
                            if menu_item(ui, t, "Note", "Ctrl N") {
                                self.state.show_new_note_dialog = true;
                                ui.close();
                            }
                            if menu_item(ui, t, "Event", "") {
                                let target_date = self.state.active_note.as_ref()
                                    .and_then(|id| self.state.daily_config.date_of(id))
                                    .unwrap_or(self.state.selected_daily_date);
                                self.state.new_event_draft = mybrain_core::EventDraft::new(target_date);
                                self.state.show_new_event_dialog = true;
                                ui.close();
                            }
                            if menu_item(ui, t, "Visual Canvas", "") {
                                self.state.show_new_canvas_dialog = true;
                                ui.close();
                            }
                            if menu_item(ui, t, "Folder", "") {
                                self.state.show_new_folder_dialog = true;
                                ui.close();
                            }
                            ui.separator();
                            if menu_item(ui, t, "Quick notes", "Ctrl Q") {
                                self.state.sidebar_section = SidebarSection::Quick;
                                self.state.top_mode = TopBarMode::Notes;
                                self.state.quick_composer_expanded = true;
                                ui.close();
                            }
                            if menu_item(ui, t, "Today's daily note", "") {
                                self.state.open_daily_today();
                                ui.close();
                            }
                        });
                    });
                });
            });
    }

    /// Thin footer showing vault/note stats and transient status messages.
    fn status_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, t: &Tokens) {
        egui::Panel::bottom("status_bar")
            .exact_size(26.0)
            .frame(
                Frame::NONE
                    .fill(t.chrome)
                    .stroke(Stroke::new(1.0, t.separator))
                    .inner_margin(Margin::symmetric(14, 0)),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let small = |s: String, c| RichText::new(s).size(11.0).color(c);
                    ui.label(small(format!("{} notes", self.state.vault.len()), t.text_faint));

                    if self.state.top_mode == TopBarMode::Notes && self.state.active_note.is_some() {
                        let words = self.state.editor_buffer.split_whitespace().count();
                        let chars = self.state.editor_buffer.chars().count();
                        ui.label(small("·".into(), t.text_faint));
                        ui.label(small(format!("{words} words"), t.text_faint));
                        ui.label(small("·".into(), t.text_faint));
                        ui.label(small(format!("{chars} characters"), t.text_faint));
                    } else if self.state.top_mode == TopBarMode::Graph {
                        ui.label(small("·".into(), t.text_faint));
                        ui.label(small(format!("{} connections", self.state.graph.edges.len()), t.text_faint));
                    } else if self.state.top_mode == TopBarMode::Calendar {
                        let events_count = self.state.event_store.on(&self.state.vault, self.state.selected_daily_date).len();
                        ui.label(small("·".into(), t.text_faint));
                        ui.label(small(format!("Calendar — {events_count} event{} on {}", if events_count == 1 { "" } else { "s" }, self.state.selected_daily_date.format("%b %-d")), t.text_faint));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.state.active_note.is_some() && self.state.top_mode == TopBarMode::Notes {
                            if self.state.editor_dirty {
                                ui.label(small("Unsaved changes".into(), t.warning));
                            } else {
                                ui.label(small("Saved".into(), t.text_faint));
                            }
                        }

                        if let Some((_, at)) = &self.state.status {
                            let age = at.elapsed();
                            if age < state::STATUS_TTL {
                                let remaining = (state::STATUS_TTL - age).as_secs_f32();
                                let alpha = remaining.clamp(0.0, 1.0);
                                let msg = self.state.active_status().unwrap_or_default().to_owned();
                                ui.add_space(12.0);
                                ui.label(small(msg, t.accent_text.gamma_multiply(alpha)));
                                ctx.request_repaint_after(std::time::Duration::from_millis(50));
                            }
                        }
                    });
                });
            });
    }
}

/// A menu row with a label and a right-aligned shortcut hint.
fn menu_item(ui: &mut egui::Ui, t: &Tokens, label: &str, shortcut: &str) -> bool {
    let resp = ui.add(
        egui::Button::new(RichText::new(label).color(t.text))
            .shortcut_text(RichText::new(shortcut).color(t.text_faint))
            .fill(egui::Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .min_size(egui::vec2(ui.available_width(), 24.0)),
    );
    resp.clicked()
}

impl eframe::App for BrainwashApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_keyboard_shortcuts(&ctx);

        let tokens = Tokens::get(&ctx);

        self.top_bar(ui, &ctx, &tokens);
        self.status_bar(ui, &ctx, &tokens);

        // Left Sidebar: collapsible note explorer & sections
        if self.state.show_sidebar {
            egui::Panel::left("sidebar_panel")
                .default_size(260.0)
                .min_size(200.0)
                .resizable(true)
                .frame(
                    Frame::NONE
                        .fill(tokens.dock)
                        .stroke(Stroke::new(1.0, tokens.separator))
                        .inner_margin(Margin::symmetric(10, 12)),
                )
                .show(ui, |ui| {
                    views::sidebar::show(ui, &mut self.state);
                });
        }

        // Right Sidebar: Connections (Backlinks & Outgoing links) - hidden by default
        if self.state.show_backlinks {
            egui::Panel::right("backlinks_panel")
                .default_size(260.0)
                .min_size(200.0)
                .resizable(true)
                .frame(
                    Frame::NONE
                        .fill(tokens.dock)
                        .stroke(Stroke::new(1.0, tokens.separator))
                        .inner_margin(Margin::symmetric(12, 12)),
                )
                .show(ui, |ui| {
                    views::backlinks::show(ui, &mut self.state);
                });
        }

        // Central Panel: Notes Editor vs Knowledge Graph
        egui::CentralPanel::default()
            .frame(
                Frame::NONE
                    .fill(tokens.canvas)
                    .inner_margin(Margin::ZERO),
            )
            .show(ui, |ui| {
                match self.state.top_mode {
                    TopBarMode::Calendar => {
                        views::daily_view::show(ui, &mut self.state);
                    }
                    TopBarMode::Graph => {
                        views::graph_view::show(ui, &mut self.state);
                    }
                    TopBarMode::Canvas => {
                        views::canvas_view::show(ui, &mut self.state);
                    }
                    TopBarMode::Notes => {
                        match self.state.sidebar_section {
                            SidebarSection::Notes => {
                                views::editor::show(ui, &mut self.state);
                            }
                            SidebarSection::Daily => {
                                views::daily_view::show(ui, &mut self.state);
                            }
                            SidebarSection::Events => {
                                views::events_view::show(ui, &mut self.state);
                            }
                            SidebarSection::Quick => {
                                views::quick_capture::show_keep_view(ui, &mut self.state);
                            }
                        }
                    }
                }
            });

        // Modals & Floating Dialogs
        views::command_palette::show(&ctx, &mut self.state);
        views::quick_capture::show(&ctx, &mut self.state);
        views::quick_capture::show_keep_edit_dialog(&ctx, &mut self.state);
        views::dialogs::show_new_note(&ctx, &mut self.state);
        views::dialogs::show_new_event(&ctx, &mut self.state);
        views::dialogs::show_new_folder(&ctx, &mut self.state);
        views::dialogs::show_new_canvas(&ctx, &mut self.state);
        views::dialogs::show_insert_table(&ctx, &mut self.state);
        views::dialogs::show_insert_image(&ctx, &mut self.state);
        views::settings_dialog::show(&ctx, &mut self.state);
        views::vault_switcher::show(&ctx, &mut self.state);

        // Drag-and-drop preview tooltip & unconsumed cleanup
        if let Some(dragged) = egui::DragAndDrop::payload::<crate::state::DraggedNote>(&ctx) {
            if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                egui::Area::new(egui::Id::unique("drag_preview_area"))
                    .fixed_pos(pos + egui::vec2(16.0, 16.0))
                    .order(egui::Order::Tooltip)
                    .interactable(false)
                    .show(&ctx, |ui| {
                        Frame::NONE
                            .fill(tokens.card)
                            .stroke(Stroke::new(1.0, tokens.accent))
                            .corner_radius(CornerRadius::same(tokens.radius_sm as u8))
                            .inner_margin(Margin::symmetric(10, 6))
                            .shadow(egui::Shadow {
                                offset: [0, 4],
                                blur: 12,
                                spread: 0,
                                color: tokens.shadow,
                            })
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("🔗").size(12.0));
                                    ui.label(
                                        RichText::new(format!("[[{}]]", dragged.name))
                                            .strong()
                                            .size(12.0)
                                            .color(tokens.text),
                                    );
                                });
                            });
                    });
            }
        }
        if ctx.input(|i| i.pointer.any_released()) {
            let _ = egui::DragAndDrop::take_payload::<crate::state::DraggedNote>(&ctx);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, KEY_THEME, &self.state.theme_kind);
        eframe::set_value(storage, KEY_VIEW_MODE, &self.state.view_mode);
        eframe::set_value(storage, KEY_SHOW_BACKLINKS, &self.state.show_backlinks);
        eframe::set_value(storage, KEY_SHOW_SIDEBAR, &self.state.show_sidebar);
        eframe::set_value(storage, KEY_TOP_MODE, &self.state.top_mode);
        eframe::set_value(storage, KEY_SIDEBAR_SECTION, &self.state.sidebar_section);
    }
}

fn determine_vault_dir() -> PathBuf {
    if let Some(mut home) = dirs::home_dir() {
        home.push("BrainwashVault");
        home
    } else {
        PathBuf::from("./vault")
    }
}

fn main() -> eframe::Result {
    env_logger::init();

    let vault_dir = determine_vault_dir();
    let mut state = match AppState::new(vault_dir.clone()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize vault at {}: {e}", vault_dir.display());
            std::process::exit(1);
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Brainwash")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([760.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Brainwash",
        options,
        Box::new(|cc| {
            // Restore persisted UI preferences.
            if let Some(storage) = cc.storage {
                if let Some(kind) = eframe::get_value(storage, KEY_THEME) {
                    state.theme_kind = kind;
                }
                if let Some(mode) = eframe::get_value(storage, KEY_VIEW_MODE) {
                    state.view_mode = mode;
                }
                if let Some(show) = eframe::get_value(storage, KEY_SHOW_BACKLINKS) {
                    state.show_backlinks = show;
                }
                if let Some(show) = eframe::get_value(storage, KEY_SHOW_SIDEBAR) {
                    state.show_sidebar = show;
                }
                if let Some(top) = eframe::get_value(storage, KEY_TOP_MODE) {
                    state.top_mode = top;
                }
                if let Some(sec) = eframe::get_value(storage, KEY_SIDEBAR_SECTION) {
                    state.sidebar_section = sec;
                }
            }
            theme::install_fonts(&cc.egui_ctx);
            theme::apply_theme(&cc.egui_ctx, state.theme_kind);
            Ok(Box::new(BrainwashApp::new(state)))
        }),
    )
}
