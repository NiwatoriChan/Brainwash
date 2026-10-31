//! Persistent application settings and preferences for Brainwash.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub const DEFAULT_UI_SCALE: f32 = 1.0;
pub const DEFAULT_EDITOR_FONT_SIZE: f32 = 14.0;

/// User preferences and configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// Active theme key (Studio, StudioLight, Pro, SolarizedDark, AdwaitaDark).
    pub theme: String,
    /// Overall UI scaling factor (0.80 to 1.50).
    pub ui_scale: f32,
    /// Editor text font size (11.0 to 24.0).
    pub editor_font_size: f32,
    /// Preferred proportional font family name or "System".
    pub font_family: String,
    /// Default directory for daily notes.
    pub daily_folder: String,
    /// Default directory for new calendar events.
    pub events_folder: String,
    /// Whether to snap canvas nodes to grid points.
    pub canvas_snap_to_grid: bool,
    /// List of recently opened vault directory paths.
    pub recent_vaults: Vec<String>,
    /// Custom emoji icons mapped to relative folder paths in tree view.
    pub folder_emojis: BTreeMap<String, String>,
    /// Custom color mapped to relative folder paths in tree view.
    #[serde(default)]
    pub folder_colors: BTreeMap<String, String>,
    /// Customizable favorite color palette for notes, folders, and emojis.
    #[serde(default = "default_color_palette")]
    pub color_palette: Vec<ColorDef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorDef {
    pub name: String,
    pub label: String,
    pub hex: String,
}

pub fn default_color_palette() -> Vec<ColorDef> {
    vec![
        ColorDef { name: "default".into(), label: "Default".into(), hex: "#8c8c91".into() },
        ColorDef { name: "coral".into(), label: "Coral".into(), hex: "#eb5757".into() },
        ColorDef { name: "peach".into(), label: "Peach".into(), hex: "#f2994a".into() },
        ColorDef { name: "sand".into(), label: "Sand".into(), hex: "#f2c94c".into() },
        ColorDef { name: "mint".into(), label: "Mint".into(), hex: "#27ae60".into() },
        ColorDef { name: "teal".into(), label: "Teal".into(), hex: "#2d9cdb".into() },
        ColorDef { name: "sky".into(), label: "Sky Blue".into(), hex: "#56ccf2".into() },
        ColorDef { name: "dusk".into(), label: "Dusk".into(), hex: "#6f52d8".into() },
        ColorDef { name: "lavender".into(), label: "Lavender".into(), hex: "#9b51e0".into() },
        ColorDef { name: "pink".into(), label: "Blossom".into(), hex: "#eb5789".into() },
        ColorDef { name: "clay".into(), label: "Clay".into(), hex: "#b48264".into() },
    ]
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "Studio".into(),
            ui_scale: DEFAULT_UI_SCALE,
            editor_font_size: DEFAULT_EDITOR_FONT_SIZE,
            font_family: "System".into(),
            daily_folder: "Daily".into(),
            events_folder: "Events".into(),
            canvas_snap_to_grid: true,
            recent_vaults: Vec::new(),
            folder_emojis: BTreeMap::new(),
            folder_colors: BTreeMap::new(),
            color_palette: default_color_palette(),
        }
    }
}

impl AppSettings {
    pub fn effective_color_palette(&self) -> &[ColorDef] {
        if self.color_palette.is_empty() {
            static DEFAULT: std::sync::OnceLock<Vec<ColorDef>> = std::sync::OnceLock::new();
            DEFAULT.get_or_init(default_color_palette)
        } else {
            &self.color_palette
        }
    }
    /// Load settings from a JSON file, or returns default if file does not exist.
    pub fn load_from_file(path: &Path) -> Self {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(parsed) = serde_json::from_str(&content) {
                return parsed;
            }
        }
        Self::default()
    }

    /// Save settings atomically to a JSON file.
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| Error::InvalidEvent(format!("settings serialization error: {e}")))?;
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, json).map_err(|e| Error::io(&tmp, e))?;
        fs::rename(&tmp, path).map_err(|e| Error::io(path, e))?;
        Ok(())
    }

    /// Adds a vault path to recent vaults list, keeping unique entries and capping at 10.
    pub fn record_recent_vault(&mut self, vault_path: &str) {
        self.recent_vaults.retain(|p| p != vault_path);
        self.recent_vaults.insert(0, vault_path.to_owned());
        if self.recent_vaults.len() > 10 {
            self.recent_vaults.truncate(10);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");

        let mut settings = AppSettings::default();
        settings.theme = "Pro".into();
        settings.ui_scale = 1.15;
        settings.record_recent_vault("/home/user/vault1");
        settings.record_recent_vault("/home/user/vault2");
        settings.folder_emojis.insert("Projects".into(), "🚀".into());

        settings.save_to_file(&path).expect("save succeeds");
        let loaded = AppSettings::load_from_file(&path);

        assert_eq!(loaded.theme, "Pro");
        assert_eq!(loaded.ui_scale, 1.15);
        assert_eq!(loaded.recent_vaults, vec!["/home/user/vault2", "/home/user/vault1"]);
        assert_eq!(loaded.folder_emojis.get("Projects"), Some(&"🚀".to_string()));
    }
}
