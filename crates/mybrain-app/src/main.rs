//! Brainwash - Local-first personal knowledge base and graph.

pub mod plugins;
pub mod shell;
pub mod theme;

use std::path::PathBuf;
use eframe::egui;
use shell::AppShell;

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
    let shell = match AppShell::new(vault_dir.clone()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize vault at {}: {e}", vault_dir.display());
            std::process::exit(1);
        }
    };

    let viewport = egui::ViewportBuilder::default()
        .with_title("Brainwash")
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([760.0, 480.0]);

    #[cfg(target_os = "macos")]
    let viewport = viewport
        .with_fullsize_content_view(true)
        .with_titlebar_shown(false)
        .with_title_shown(false);

    #[cfg(not(target_os = "macos"))]
    let viewport = viewport.with_decorations(false);

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Brainwash",
        options,
        Box::new(|cc| {
            theme::install_fonts(&cc.egui_ctx);
            theme::apply_theme(&cc.egui_ctx, shell.theme_kind);
            cc.egui_ctx.set_zoom_factor(shell.settings.ui_scale.clamp(0.8, 1.5));
            Ok(Box::new(shell))
        }),
    )
}
