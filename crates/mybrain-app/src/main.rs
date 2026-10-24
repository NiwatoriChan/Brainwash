//! MyBrain - An Obsidian clone in Rust with egui.
//!
//! Featuring:
//! - [[Double bracket wiki-links]] with live auto-completion and resolution
//! - Daily notes with custom templates and quick date switching
//! - Quick notes capture with automatic daily note logging
//! - Calendar events stored in standard Markdown front-matter
//! - Live rendered Markdown preview and split-view editing
//! - Full backlinks, outgoing links, and unresolved links explorer
//! - Command palette / quick note switcher (Ctrl+P)

mod state;
mod theme;
mod views;

use std::path::PathBuf;

use eframe::egui;
use egui::{Frame, Margin, RichText, Stroke};
use state::{AppState, MainTab};

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
            // Ctrl+Q: Quick Capture
            if i.modifiers.command && i.key_pressed(egui::Key::Q) {
                self.state.show_quick_capture = !self.state.show_quick_capture;
            }
            // Ctrl+N: New Note
            if i.modifiers.command && i.key_pressed(egui::Key::N) {
                self.state.show_new_note_dialog = true;
            }
            // Ctrl+E: Toggle View Mode (Preview / Edit)
            if i.modifiers.command && i.key_pressed(egui::Key::E) {
                self.state.view_mode = match self.state.view_mode {
                    crate::state::ViewMode::PreviewOnly => crate::state::ViewMode::EditOnly,
                    crate::state::ViewMode::EditOnly => crate::state::ViewMode::PreviewOnly,
                    crate::state::ViewMode::Split => crate::state::ViewMode::PreviewOnly,
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
}

impl eframe::App for BrainwashApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_keyboard_shortcuts(&ctx);

        let tokens = theme::Tokens::get(&ctx);

        // Top Navigation Menu Bar
        egui::Panel::top("top_bar")
            .frame(
                Frame::NONE
                    .fill(tokens.chrome)
                    .stroke(Stroke::new(1.0, tokens.separator))
                    .inner_margin(Margin::symmetric(14, 8)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Brainwash")
                            .strong()
                            .size(16.0)
                            .color(tokens.text),
                    );
                    ui.label(
                        RichText::new("v0.1")
                            .size(10.0)
                            .color(tokens.text_faint),
                    );

                    ui.separator();

                    if ui.selectable_label(self.state.main_tab == MainTab::Notes, "Notes").clicked() {
                        self.state.main_tab = MainTab::Notes;
                    }
                    if ui.button("Daily Notes").clicked() {
                        self.state.open_daily_today();
                    }
                    if ui.selectable_label(self.state.main_tab == MainTab::Events, "Calendar").clicked() {
                        self.state.main_tab = MainTab::Events;
                    }

                    ui.separator();

                    if ui.button(RichText::new("+ Note (Ctrl+N)").color(tokens.accent_text)).clicked() {
                        self.state.show_new_note_dialog = true;
                    }
                    if ui.button(RichText::new("+ Quick Note (Ctrl+Q)").color(tokens.warning)).clicked() {
                        self.state.show_quick_capture = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Toggle backlinks panel
                        let bl_label = if self.state.show_backlinks {
                            "Hide Links →"
                        } else {
                            "← Links"
                        };
                        if ui.selectable_label(self.state.show_backlinks, bl_label).clicked() {
                            self.state.show_backlinks = !self.state.show_backlinks;
                        }

                        ui.separator();

                        // Theme switcher
                        let mut changed_theme = None;
                        ui.menu_button(format!("Theme: {}", self.state.theme_kind.label()), |ui| {
                            for kind in theme::ThemeKind::ALL {
                                if ui.selectable_label(self.state.theme_kind == kind, kind.label()).clicked() {
                                    changed_theme = Some(kind);
                                    ui.close();
                                }
                            }
                        });
                        if let Some(kind) = changed_theme {
                            self.state.theme_kind = kind;
                            theme::apply_theme(&ctx, kind);
                        }

                        ui.separator();

                        if ui.button(RichText::new("Search (Ctrl+P)").color(tokens.text_dim)).clicked() {
                            self.state.show_command_palette = true;
                        }

                        if self.state.editor_dirty {
                            if ui.button(RichText::new("Save (Ctrl+S)").color(tokens.accent_text).strong()).clicked() {
                                self.state.save_current();
                            }
                        }
                    });
                });
            });

        // Left Sidebar: Notes explorer & search
        egui::Panel::left("sidebar_panel")
            .default_size(240.0)
            .min_size(180.0)
            .resizable(true)
            .frame(
                Frame::NONE
                    .fill(theme::BG_PANEL)
                    .stroke(Stroke::new(1.0, theme::BORDER_COLOR))
                    .inner_margin(Margin::same(10)),
            )
            .show(ui, |ui| {
                views::sidebar::show(ui, &mut self.state);
            });

        // Right Sidebar: Connections (Backlinks & Outgoing links)
        if self.state.show_backlinks {
            egui::Panel::right("backlinks_panel")
                .default_size(250.0)
                .min_size(200.0)
                .resizable(true)
                .frame(
                    Frame::NONE
                        .fill(tokens.dock)
                        .stroke(Stroke::new(1.0, tokens.separator))
                        .inner_margin(Margin::same(10)),
                )
                .show(ui, |ui| {
                    views::backlinks::show(ui, &mut self.state);
                });
        }

        // Central Panel: Editor / Daily / Events
        egui::CentralPanel::default()
            .frame(
                Frame::NONE
                    .fill(tokens.canvas)
                    .inner_margin(Margin::ZERO),
            )
            .show(ui, |ui| {
                match self.state.main_tab {
                    MainTab::Notes => {
                        views::editor::show(ui, &mut self.state);
                    }
                    MainTab::Daily => {
                        views::daily_view::show(ui, &mut self.state);
                    }
                    MainTab::Events => {
                        views::events_view::show(ui, &mut self.state);
                    }
                }
            });

        // Modals & Floating Dialogs
        views::command_palette::show(&ctx, &mut self.state);
        views::quick_capture::show(&ctx, &mut self.state);
        views::dialogs::show_new_note(&ctx, &mut self.state);
        views::dialogs::show_new_event(&ctx, &mut self.state);
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
    let state = match AppState::new(vault_dir.clone()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize vault at {}: {e}", vault_dir.display());
            std::process::exit(1);
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Brainwash")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Brainwash",
        options,
        Box::new(|cc| {
            theme::install_fonts(&cc.egui_ctx);
            theme::apply_theme(&cc.egui_ctx, state.theme_kind);
            Ok(Box::new(BrainwashApp::new(state)))
        }),
    )
}
