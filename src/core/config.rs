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
    /// v2.6 subsystems. Every field is `#[serde(default)]` so a `wino.toml`
    /// written by v2.5 keeps loading unchanged.
    #[serde(default)]
    pub profiles: ProfilesConfig,
    #[serde(default)]
    pub power: PowerConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub apps: AppsConfig,
    #[serde(default)]
    pub health: HealthConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub theme: String, // "system", "dark", "light"
    pub dry_run: bool,
    pub minimize_to_tray: bool,
    pub auto_create_restore_point: bool,
    #[serde(default = "default_language")]
    pub language: String, // "en", "id"
}

fn default_language() -> String {
    "en".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub mode: String, // "smart", "gaming", "low_ram", "manual"
    pub refresh_interval_ms: u64,
    pub auto_trim_threshold_pct: f32,
    pub smart_optimize_enabled: bool,
    /// Background auto-trim toggle (tray-less power saver mode).
    #[serde(default)]
    pub auto_trim_enabled: bool,
    /// How often the background monitor samples RAM usage.
    #[serde(default = "default_trim_interval")]
    pub auto_trim_interval_secs: u64,
    /// Minimum spacing between two automatic trims (anti-thrash cooldown).
    #[serde(default = "default_trim_cooldown")]
    pub auto_trim_cooldown_secs: u64,
}

fn default_trim_interval() -> u64 {
    30
}

fn default_trim_cooldown() -> u64 {
    300
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

/// Profile engine preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesConfig {
    /// Profile applied by `wino profile apply` with no explicit name.
    pub default_profile: String,
    /// Capture a before/after benchmark around profile application.
    pub benchmark_on_apply: bool,
}

impl Default for ProfilesConfig {
    fn default() -> Self {
        Self {
            default_profile: "Balanced".to_string(),
            benchmark_on_apply: true,
        }
    }
}

/// Power manager defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerConfig {
    /// Power plan Wino switches to when applying a profile ("", "balanced",
    /// "high_performance", "ultimate", or a GUID).
    pub default_plan: String,
    /// Refresh the active-plan display every N seconds.
    #[serde(default = "default_power_poll")]
    pub poll_secs: u64,
}

fn default_power_poll() -> u64 {
    10
}

impl Default for PowerConfig {
    fn default() -> Self {
        Self {
            default_plan: String::new(),
            poll_secs: default_power_poll(),
        }
    }
}

/// Network center preferences. DNS presets stay opt-in; nothing is applied
/// automatically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Preset id selected in the UI ("auto", "cloudflare", "google", "quad9",
    /// "adguard").
    pub preferred_preset: String,
    /// Host used by the ping / latency / packet-loss tools.
    #[serde(default = "default_ping_host")]
    pub ping_host: String,
    /// Echo requests per latency sample.
    #[serde(default = "default_ping_count")]
    pub ping_count: u32,
}

fn default_ping_host() -> String {
    "1.1.1.1".to_string()
}

fn default_ping_count() -> u32 {
    4
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            preferred_preset: "cloudflare".to_string(),
            ping_host: default_ping_host(),
            ping_count: default_ping_count(),
        }
    }
}

/// Storage analyzer limits. The analyzer is informational and never deletes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Files at least this large are reported as "large files".
    #[serde(default = "default_large_file_mb")]
    pub large_file_mb: u64,
    /// Files older than this are reported as "old files".
    #[serde(default = "default_old_file_days")]
    pub old_file_days: u64,
    /// Hard ceiling on directory entries visited in one scan.
    #[serde(default = "default_scan_budget")]
    pub max_entries: usize,
    /// Directory recursion ceiling.
    #[serde(default = "default_scan_depth")]
    pub max_depth: u8,
}

fn default_large_file_mb() -> u64 {
    100
}

fn default_old_file_days() -> u64 {
    365
}

fn default_scan_budget() -> usize {
    400_000
}

fn default_scan_depth() -> u8 {
    12
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            large_file_mb: default_large_file_mb(),
            old_file_days: default_old_file_days(),
            max_entries: default_scan_budget(),
            max_depth: default_scan_depth(),
        }
    }
}

/// Application manager preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppsConfig {
    /// Show only the requested source when the UI opens. "All" by default.
    pub default_source_filter: String,
    /// Query `winget` for update metadata during a full scan. Off keeps the
    /// scan fast and fully native; the updates view turns it on on demand.
    #[serde(default)]
    pub scan_winget_metadata: bool,
}

impl Default for AppsConfig {
    fn default() -> Self {
        Self {
            default_source_filter: "All".to_string(),
            scan_winget_metadata: false,
        }
    }
}

/// Health center preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Re-run the health scan automatically every N seconds in the UI.
    #[serde(default = "default_health_interval")]
    pub scan_interval_secs: u64,
    /// Include the slow DISM component-store check in a normal health scan.
    #[serde(default)]
    pub include_dism: bool,
}

fn default_health_interval() -> u64 {
    30
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            scan_interval_secs: default_health_interval(),
            include_dism: false,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                theme: "dark".to_string(),
                dry_run: false,
                minimize_to_tray: false,
                auto_create_restore_point: true,
                language: default_language(),
            },
            memory: MemoryConfig {
                mode: "smart".to_string(),
                refresh_interval_ms: 1000,
                auto_trim_threshold_pct: 85.0,
                smart_optimize_enabled: true,
                auto_trim_enabled: false,
                auto_trim_interval_secs: default_trim_interval(),
                auto_trim_cooldown_secs: default_trim_cooldown(),
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
            profiles: ProfilesConfig::default(),
            power: PowerConfig::default(),
            network: NetworkConfig::default(),
            storage: StorageConfig::default(),
            apps: AppsConfig::default(),
            health: HealthConfig::default(),
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
        let content = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        fs::write(path, content)
    }
}
