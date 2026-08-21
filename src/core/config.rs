use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub memory: MemoryConfig,
    pub monitoring: MonitoringConfig,
    pub privacy: PrivacyConfig,
    pub debloat: DebloatConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub theme: String, // "system", "dark", "light"
    pub dry_run: bool,
    pub minimize_to_tray: bool,
    pub auto_create_restore_point: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub mode: String, // "smart", "gaming", "low_ram", "manual"
    pub refresh_interval_ms: u64,
    pub auto_trim_threshold_pct: f32,
    pub smart_optimize_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub active_poll_ms: u64,
    pub background_poll_ms: u64,
    pub adaptive_polling: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub disable_telemetry: bool,
    pub disable_advertising_id: bool,
    pub disable_activity_feed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebloatConfig {
    pub default_preset: String, // "Safe", "Balanced", "Aggressive", "Custom"
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                theme: "dark".to_string(),
                dry_run: false,
                minimize_to_tray: false,
                auto_create_restore_point: true,
            },
            memory: MemoryConfig {
                mode: "smart".to_string(),
                refresh_interval_ms: 1000,
                auto_trim_threshold_pct: 85.0,
                smart_optimize_enabled: true,
            },
            monitoring: MonitoringConfig {
                active_poll_ms: 500,
                background_poll_ms: 3000,
                adaptive_polling: true,
            },
            privacy: PrivacyConfig {
                disable_telemetry: true,
                disable_advertising_id: true,
                disable_activity_feed: true,
            },
            debloat: DebloatConfig {
                default_preset: "Safe".to_string(),
            },
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        if Path::new("wino.toml").exists() {
            PathBuf::from("wino.toml")
        } else if let Some(appdata) = std::env::var_os("APPDATA") {
            let p = PathBuf::from(appdata).join("Wino");
            let _ = fs::create_dir_all(&p);
            p.join("config.toml")
        } else {
            PathBuf::from("wino.toml")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        let default_config = Self::default();
        let _ = default_config.save();
        default_config
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let content = toml::to_string_pretty(self).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, content)
    }
}
