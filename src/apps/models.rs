//! Application Manager data model.
//!
//! One [`AppRecord`] per discovered application regardless of where it came
//! from. Sources are kept explicit rather than merged so the UI can filter and
//! so an uninstall path is only offered when a real one exists.

use serde::{Deserialize, Serialize};

/// Where an application record came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppSource {
    /// Classic installed program: `Uninstall` registry keys (HKLM/HKCU, both
    /// the 64-bit and WOW6432Node views).
    Win32,
    /// Installed per-user Microsoft Store package (AppX/MSIX).
    MicrosoftStore,
    /// Provisioned AppX package present in the system image but not installed
    /// for the current user.
    AppX,
    /// Known to Winget.
    Winget,
}

impl AppSource {
    pub fn label(&self) -> &'static str {
        match self {
            AppSource::Win32 => "Win32",
            AppSource::MicrosoftStore => "Microsoft Store",
            AppSource::AppX => "AppX",
            AppSource::Winget => "Winget",
        }
    }

    /// i18n key for the source filter chip.
    pub fn i18n_key(&self) -> &'static str {
        match self {
            AppSource::Win32 => "apps.source_win32",
            AppSource::MicrosoftStore => "apps.source_store",
            AppSource::AppX => "apps.source_appx",
            AppSource::Winget => "apps.source_winget",
        }
    }

    /// All sources in filter order.
    pub const ALL: [AppSource; 4] = [
        AppSource::Win32,
        AppSource::MicrosoftStore,
        AppSource::AppX,
        AppSource::Winget,
    ];
}

/// How an application can be removed, if at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UninstallMethod {
    /// `UninstallString` from the registry, run directly.
    RegistryCommand { command: String },
    /// `QuietUninstallString` from the registry (preferred when present).
    RegistryQuietCommand { command: String },
    /// MSI product code, removed through `msiexec.exe /x`.
    MsiProductCode { product_code: String },
    /// AppX/MSIX full package name, removed through `Remove-AppxPackage`
    /// semantics (implemented natively via the deployment API surface).
    AppxPackage { full_name: String },
    /// Winget package id.
    WingetPackage { package_id: String },
}

impl UninstallMethod {
    /// Short technical label shown in the confirm dialog.
    pub fn kind_label(&self) -> &'static str {
        match self {
            UninstallMethod::RegistryCommand { .. } => "UninstallString",
            UninstallMethod::RegistryQuietCommand { .. } => "QuietUninstallString",
            UninstallMethod::MsiProductCode { .. } => "MSI",
            UninstallMethod::AppxPackage { .. } => "AppX",
            UninstallMethod::WingetPackage { .. } => "Winget",
        }
    }
}

/// One installed application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRecord {
    /// Registry key name or package family name — unique per source.
    pub name: String,
    pub display_name: String,
    pub publisher: String,
    pub version: String,
    /// Empty when the source does not record a location.
    pub install_location: String,
    /// Bytes. `None` when the source does not report a size.
    pub install_size: Option<u64>,
    /// Raw `InstallDate` as reported (usually `YYYYMMDD`), empty when absent.
    pub install_date: String,
    /// Raw package type string from the source, e.g. "MSI", "MSIX", "EXE".
    pub package_type: String,
    pub source: AppSource,
    /// Authenticode result. `None` when no executable could be located.
    pub signed: Option<bool>,
    /// How to remove it. `None` means the UI must show "Uninstall unavailable".
    pub uninstall: Option<UninstallMethod>,
    /// True when the record came from a provisioned (not installed) package.
    pub provisioned: bool,
    /// Winget package id when known, used to correlate update metadata.
    pub winget_id: Option<String>,
}

impl AppRecord {
    /// Whether an uninstall action can be offered.
    pub fn is_uninstallable(&self) -> bool {
        self.uninstall.is_some()
    }

    /// Human size, or an empty string when the source reported no size.
    pub fn size_label(&self) -> String {
        match self.install_size {
            Some(bytes) => format_size(bytes),
            None => String::new(),
        }
    }

    /// Sort key: lowercase display name, falling back to the raw name.
    pub fn sort_key(&self) -> String {
        let d = self.display_name.trim();
        if d.is_empty() {
            self.name.to_lowercase()
        } else {
            d.to_lowercase()
        }
    }

    /// Case-insensitive match against name, display name, publisher, and path.
    pub fn matches(&self, query_lower: &str) -> bool {
        if query_lower.is_empty() {
            return true;
        }
        self.name.to_lowercase().contains(query_lower)
            || self.display_name.to_lowercase().contains(query_lower)
            || self.publisher.to_lowercase().contains(query_lower)
            || self.install_location.to_lowercase().contains(query_lower)
    }
}

/// How the application list is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppSort {
    Name,
    Publisher,
    Size,
    InstallDate,
    Version,
}

impl AppSort {
    pub fn label(&self) -> &'static str {
        match self {
            AppSort::Name => "Name",
            AppSort::Publisher => "Publisher",
            AppSort::Size => "Size",
            AppSort::InstallDate => "Install Date",
            AppSort::Version => "Version",
        }
    }

    pub fn i18n_key(&self) -> &'static str {
        match self {
            AppSort::Name => "apps.sort_name",
            AppSort::Publisher => "apps.sort_publisher",
            AppSort::Size => "apps.sort_size",
            AppSort::InstallDate => "apps.sort_date",
            AppSort::Version => "apps.sort_version",
        }
    }

    pub const ALL: [AppSort; 5] = [
        AppSort::Name,
        AppSort::Publisher,
        AppSort::Size,
        AppSort::InstallDate,
        AppSort::Version,
    ];
}

/// Update state reported by Winget for one package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateState {
    /// Installed and no newer version is available.
    Latest,
    /// A newer version is available.
    UpdateAvailable,
    /// Winget could not determine the state.
    Unknown,
}

impl UpdateState {
    pub fn i18n_key(&self) -> &'static str {
        match self {
            UpdateState::Latest => "apps.up_to_date",
            UpdateState::UpdateAvailable => "apps.update_available",
            UpdateState::Unknown => "apps.update_unknown",
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            UpdateState::Latest => (34, 197, 94),
            UpdateState::UpdateAvailable => (234, 179, 8),
            UpdateState::Unknown => (148, 163, 184),
        }
    }
}

/// One row of `winget upgrade` output, correlated back to an [`AppRecord`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WingetUpdate {
    pub package_id: String,
    pub name: String,
    pub current_version: String,
    pub available_version: String,
    /// Package id of the record this update belongs to, when correlated.
    pub app_name: Option<String>,
    pub state: UpdateState,
}

impl WingetUpdate {
    /// Whether this update should be offered for installation.
    pub fn is_actionable(&self) -> bool {
        self.state == UpdateState::UpdateAvailable && !self.package_id.is_empty()
    }
}

/// Whether the `winget` CLI is usable on this machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WingetStatus {
    pub available: bool,
    /// Version string reported by `winget --version`, empty when unavailable.
    pub version: String,
    /// Why it is unavailable, for the UI to explain rather than just grey out.
    pub detail: String,
}

impl WingetStatus {
    pub fn missing(detail: &str) -> Self {
        Self {
            available: false,
            version: String::new(),
            detail: detail.to_string(),
        }
    }
}

/// Format a byte count the way the rest of Wino does (binary units, 1 decimal
/// below 10 units and none above).
pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Parse a registry `InstallDate` value (`YYYYMMDD`) into `YYYY-MM-DD`.
///
/// Returns the input unchanged when it is not the expected shape, so an
/// unexpected vendor format is displayed verbatim instead of being lost.
pub fn normalize_install_date(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() != 8 || !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return trimmed.to_string();
    }
    format!("{}-{}-{}", &trimmed[0..4], &trimmed[4..6], &trimmed[6..8])
}

/// Classify a registry `DisplayName`/key into a package type label.
///
/// Pure so the classification rules are testable without a registry.
pub fn classify_package_type(
    key_name: &str,
    uninstall_string: &str,
    has_msi_product_code: bool,
    is_appx: bool,
) -> String {
    if is_appx {
        return "AppX".to_string();
    }
    if has_msi_product_code {
        return "MSI".to_string();
    }
    let lower = uninstall_string.to_lowercase();
    if lower.contains("msiexec") {
        return "MSI".to_string();
    }
    if lower.is_empty() {
        return "Unknown".to_string();
    }
    let _ = key_name;
    "EXE".to_string()
}

/// Decide the uninstall method from the raw registry fields.
///
/// Precedence: quiet command, MSI product code, plain command. Returns `None`
/// when the application genuinely exposes no removal path — the UI then shows
/// "Uninstall unavailable" instead of a button that cannot work.
pub fn classify_uninstall_method(
    quiet_uninstall_string: &str,
    uninstall_string: &str,
    msi_product_code: &str,
    appx_full_name: &str,
) -> Option<UninstallMethod> {
    let quiet = quiet_uninstall_string.trim();
    if !quiet.is_empty() {
        return Some(UninstallMethod::RegistryQuietCommand {
            command: quiet.to_string(),
        });
    }

    let code = msi_product_code.trim();
    if !code.is_empty() {
        return Some(UninstallMethod::MsiProductCode {
            product_code: code.to_string(),
        });
    }

    let plain = uninstall_string.trim();
    if !plain.is_empty() {
        return Some(UninstallMethod::RegistryCommand {
            command: plain.to_string(),
        });
    }

    let appx = appx_full_name.trim();
    if !appx.is_empty() {
        return Some(UninstallMethod::AppxPackage {
            full_name: appx.to_string(),
        });
    }

    None
}

/// Split a Windows command line into (executable, arguments).
///
/// Handles the two shapes the `UninstallString` value actually uses:
/// `"C:\path with spaces\app.exe" /flag` and `C:\path\app.exe /flag`.
/// Returns `None` when no executable token can be found.
pub fn split_command_line(command: &str) -> Option<(String, String)> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix('"') {
        let end = rest.find('"')?;
        let exe = rest[..end].to_string();
        let args = rest[end + 1..].trim().to_string();
        if exe.is_empty() {
            return None;
        }
        return Some((exe, args));
    }

    // Unquoted: the executable ends at the first space *after* a path
    // separator-free token, or at the first `.exe` boundary.
    let lower = trimmed.to_lowercase();
    if let Some(idx) = lower.find(".exe") {
        let split_at = idx + 4;
        let exe = trimmed[..split_at].to_string();
        let args = trimmed[split_at..].trim().to_string();
        return Some((exe, args));
    }

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let exe = parts.next()?.to_string();
    let args = parts.next().unwrap_or("").trim().to_string();
    Some((exe, args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_size_uses_binary_units() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(2048), "2 KB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3.00 GB");
    }

    #[test]
    fn install_date_normalizes_only_yyyymmdd() {
        assert_eq!(normalize_install_date("20240115"), "2024-01-15");
        assert_eq!(normalize_install_date(" 20240115 "), "2024-01-15");
        assert_eq!(normalize_install_date("01/15/2024"), "01/15/2024");
        assert_eq!(normalize_install_date(""), "");
    }

    #[test]
    fn uninstall_precedence_prefers_quiet_then_msi_then_plain() {
        let quiet = classify_uninstall_method("quiet.exe /q", "plain.exe", "{GUID}", "");
        assert_eq!(
            quiet,
            Some(UninstallMethod::RegistryQuietCommand {
                command: "quiet.exe /q".to_string()
            })
        );

        let msi = classify_uninstall_method("", "plain.exe", "{GUID}", "");
        assert_eq!(
            msi,
            Some(UninstallMethod::MsiProductCode {
                product_code: "{GUID}".to_string()
            })
        );

        let plain = classify_uninstall_method("", "plain.exe", "", "");
        assert_eq!(
            plain,
            Some(UninstallMethod::RegistryCommand {
                command: "plain.exe".to_string()
            })
        );

        let appx = classify_uninstall_method("", "", "", "Pkg_1.0_x64__abc");
        assert_eq!(
            appx,
            Some(UninstallMethod::AppxPackage {
                full_name: "Pkg_1.0_x64__abc".to_string()
            })
        );
    }

    #[test]
    fn missing_uninstall_fields_yield_no_method() {
        assert_eq!(classify_uninstall_method("", "   ", "", ""), None);
        assert_eq!(classify_uninstall_method("", "", "", "  "), None);
    }

    #[test]
    fn package_type_classification_is_stable() {
        assert_eq!(classify_package_type("K", "", false, true), "AppX");
        assert_eq!(classify_package_type("K", "", true, false), "MSI");
        assert_eq!(
            classify_package_type("K", "msiexec.exe /x {G}", false, false),
            "MSI"
        );
        assert_eq!(
            classify_package_type("K", "unins000.exe", false, false),
            "EXE"
        );
        assert_eq!(classify_package_type("K", "", false, false), "Unknown");
    }

    #[test]
    fn split_command_line_handles_quoted_and_bare_paths() {
        let quoted = split_command_line(r#""C:\Program Files\App\unins.exe" /S /Q"#).unwrap();
        assert_eq!(quoted.0, r"C:\Program Files\App\unins.exe");
        assert_eq!(quoted.1, "/S /Q");

        let bare = split_command_line(r"C:\App\unins.exe /S").unwrap();
        assert_eq!(bare.0, r"C:\App\unins.exe");
        assert_eq!(bare.1, "/S");

        let no_args = split_command_line(r"C:\App\unins.exe").unwrap();
        assert_eq!(no_args.0, r"C:\App\unins.exe");
        assert_eq!(no_args.1, "");

        assert!(split_command_line("   ").is_none());
    }

    #[test]
    fn record_matching_is_case_insensitive_across_fields() {
        let rec = AppRecord {
            name: "Contoso".to_string(),
            display_name: "Contoso Editor".to_string(),
            publisher: "Contoso Ltd".to_string(),
            version: "1.0".to_string(),
            install_location: r"C:\Program Files\Contoso".to_string(),
            install_size: None,
            install_date: String::new(),
            package_type: "EXE".to_string(),
            source: AppSource::Win32,
            signed: None,
            uninstall: None,
            provisioned: false,
            winget_id: None,
        };

        assert!(rec.matches("contoso"));
        assert!(rec.matches("editor"));
        assert!(rec.matches("ltd"));
        assert!(rec.matches("program files"));
        assert!(rec.matches(""));
        assert!(!rec.matches("fabrikam"));
        assert!(!rec.is_uninstallable());
    }

    #[test]
    fn winget_update_actionable_requires_available_state_and_id() {
        let actionable = WingetUpdate {
            package_id: "Contoso.App".to_string(),
            name: "Contoso App".to_string(),
            current_version: "1.0".to_string(),
            available_version: "2.0".to_string(),
            app_name: None,
            state: UpdateState::UpdateAvailable,
        };
        assert!(actionable.is_actionable());

        let latest = WingetUpdate {
            state: UpdateState::Latest,
            ..actionable.clone()
        };
        assert!(!latest.is_actionable());

        let no_id = WingetUpdate {
            package_id: String::new(),
            ..actionable.clone()
        };
        assert!(!no_id.is_actionable());
    }
}
