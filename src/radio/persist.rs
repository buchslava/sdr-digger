use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const CASES_SUBDIR: &str = "cases";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub last_case_slug: Option<String>,
    #[serde(default)]
    pub last_ppm: f64,
    #[serde(default = "default_y_min")]
    pub y_min: f64,
    #[serde(default = "default_y_max")]
    pub y_max: f64,
    #[serde(default = "default_true")]
    pub show_overlay: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_y_min() -> f64 {
    -10.0
}
fn default_y_max() -> f64 {
    100.0
}
fn default_true() -> bool {
    true
}
fn default_theme() -> String {
    "digger".into()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            last_case_slug: Some("fm-broadcast-r1".into()),
            last_ppm: 0.0,
            y_min: -10.0,
            y_max: 100.0,
            show_overlay: true,
            theme: "digger".into(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".sdr-digger")
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.toml")
}

pub fn load_settings() -> AppSettings {
    let path = settings_path();
    match fs::read_to_string(&path) {
        Ok(raw) => toml::from_str(&raw).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let raw = toml::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(settings_path(), raw).map_err(|e| e.to_string())
}

/// Optional ALSA device override for cpal on Linux.
pub fn configure_linux_audio_env() {
    #[cfg(target_os = "linux")]
    {
        if let Ok(device) = std::env::var("SDR_DIGGER_ALSA_DEVICE") {
            unsafe {
                std::env::set_var("ALSA_PCM_DEVICE", device);
            }
        }
    }
}
