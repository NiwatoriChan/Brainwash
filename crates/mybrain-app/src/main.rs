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

struct MyBrainApp {
    state: AppState,
}

impl MyBrainApp {
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

impl eframe::App for MyBrainApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_keyboard_shortcuts(&ctx);

        // Top Navigation Menu Bar
        egui::Panel::top("top_bar")
            .frame(
                Frame::NONE
                    .fill(theme::BG_PANEL)
                    .stroke(Stroke::new(1.0, theme::BORDER_COLOR))
                    .inner_margin(Margin::symmetric(12, 8)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🧠 MyBrain").strong().size(15.0).color(theme::TEXT_PRIMARY));
                    ui.separator();

                    if ui.button("📄 Notes").clicked() {
                        self.state.main_tab = MainTab::Notes;
                    }
                    if ui.button("📅 Daily Notes").clicked() {
                        self.state.open_daily_today();
                    }
                    if ui.button("🗓 Calendar & Events").clicked() {
                        self.state.main_tab = MainTab::Events;
                    }

                    ui.separator();

                    if ui.button(RichText::new("+ Note (Ctrl+N)").color(theme::ACCENT_PURPLE_LIGHT)).clicked() {
                        self.state.show_new_note_dialog = true;
                    }
                    if ui.button(RichText::new("⚡ Quick Note (Ctrl+Q)").color(theme::ACCENT_AMBER)).clicked() {
                        self.state.show_quick_capture = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Toggle backlinks panel
                        let bl_label = if self.state.show_backlinks {
                            "Hide Connections ⮞"
                        } else {
                            "⮜ Connections"
                        };
                        if ui.selectable_label(self.state.show_backlinks, bl_label).clicked() {
                            self.state.show_backlinks = !self.state.show_backlinks;
                        }

                        ui.separator();

                        if ui.button(RichText::new("🔍 Search (Ctrl+P)").color(theme::TEXT_MUTED)).clicked() {
                            self.state.show_command_palette = true;
                        }

                        if self.state.editor_dirty {
                            if ui.button(RichText::new("💾 Save (Ctrl+S)").color(theme::ACCENT_PURPLE_LIGHT)).clicked() {
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
                        .fill(theme::BG_PANEL)
                        .stroke(Stroke::new(1.0, theme::BORDER_COLOR))
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
                    .fill(theme::BG_CANVAS)
                    .inner_margin(Margin::same(12)),
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
        home.push("MyBrainVault");
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
            .with_title("MyBrain")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "MyBrain",
        options,
        Box::new(|cc| {
            theme::apply_theme(&cc.egui_ctx);
            Ok(Box::new(MyBrainApp::new(state)))
        }),
    )
}
