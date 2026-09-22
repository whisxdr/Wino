//! Tests for the v2.6 subsystems.
//!
//! Every test here covers a pure decision component: a classifier, a threshold,
//! a parser, or a serialization round-trip. That is deliberate — the decision
//! logic is where a wrong answer is dangerous, and it is the part that can be
//! verified without a live Windows subsystem. The behaviours that genuinely
//! need the OS (registry reads, DISM, ICMP, service control) are reached through
//! the same pure helpers at runtime and are not faked here.

use wino::apps::models::{
    classify_package_type, classify_uninstall_method, format_size, normalize_install_date,
    split_command_line, AppRecord, AppSort, AppSource, UninstallMethod, UpdateState, WingetStatus,
    WingetUpdate,
};
use wino::benchmark::models::{byte_delta_label, compare, BenchmarkSample, DeltaDirection};
use wino::core::cancel::CancelToken;
use wino::core::config::AppConfig;
use wino::core::proc::ToolOutput;
use wino::core::safety::{OperationDescriptor, RiskLevel, SafetyEngine};
use wino::core::system::SystemInfo;
use wino::health::center::{
    classify_disk_free, classify_service_state, overall_state, HealthCheck, HealthState,
};
use wino::health::integrity::{interpret_sfc_output, IntegrityOutcome};
use wino::health::update::UpdateHealth;
use wino::network::diagnostics::{
    is_link_local_v4, is_link_local_v6, strip_zone_index, AdapterDetail, DnsLookupResult,
    IpAddressInfo, NetworkReport, PingResult,
};
use wino::power::models::{scheme_guids, PowerPlanKind, PowerSetting};
use wino::profiles::models::{
    slugify, Profile, ProfileApplyReport, ProfileOrigin, ProfileStep, StepKind, StepOutcome,
};
use wino::recommendations::models::{
    sort_recommendations, Recommendation, RecommendationArea, RecommendationSeverity, ReviewTarget,
};
use wino::restore::snapshots::{
    build_snapshot, NetworkBackupEntry, PowerBackupEntry, RegistryBackupEntry, SnapshotCategory,
    TaskBackupEntry,
};
use wino::security::center::{
    classify_critical_services, classify_defender, classify_firewall, classify_secure_boot,
    classify_tpm, classify_uac, ProtectionItem, ProtectionState, SecurityReport,
};
use wino::startup::scanner::parse_startup_approved;
use wino::storage::models::{
    classify_path, is_large_file, is_old_file, StorageBucket, StorageEntry, StorageTree,
};
use wino::windows_features::models::{apply_curation, find_curated, FeatureState, WindowsFeature};

fn elevated_win11() -> SystemInfo {
    SystemInfo {
        os_name: "Windows 11 Pro".to_string(),
        display_version: "24H2".to_string(),
        build_number: "26100".to_string(),
        architecture: "x86_64".to_string(),
        processor_name: "Test Processor".to_string(),
        processor_count: 8,
        total_ram_bytes: 16 * 1024 * 1024 * 1024,
        is_windows_11: true,
        is_admin: true,
    }
}

// ==================== Safety Engine descriptors ====================

#[test]
fn test_safety_engine_validates_descriptors() {
    let descriptor = OperationDescriptor::simple(
        "test_rule",
        "Test operation",
        "Component",
        RiskLevel::Medium,
    )
    .with_admin(true)
    .with_windows(&["10", "11"])
    .with_reason("Because it changes a setting.")
    .with_states("off", "on");

    assert!(SafetyEngine::validate_descriptor(&descriptor, &elevated_win11()).is_allowed);
    assert_eq!(descriptor.current_state, "off");
    assert_eq!(descriptor.target_state, "on");

    let unelevated = SystemInfo {
        is_admin: false,
        ..elevated_win11()
    };
    assert!(!SafetyEngine::validate_descriptor(&descriptor, &unelevated).is_allowed);
}

#[test]
fn test_safety_filter_for_preset_separates_blocked_descriptors() {
    let descriptors = vec![
        OperationDescriptor::simple("a", "Safe op", "c", RiskLevel::Safe),
        OperationDescriptor::simple("b", "Low op", "c", RiskLevel::Low),
        OperationDescriptor::simple("c", "High op", "c", RiskLevel::High),
        OperationDescriptor::simple("d", "Critical op", "c", RiskLevel::Critical),
    ];

    let (allowed, blocked) = SafetyEngine::filter_for_preset(&descriptors, "Balanced");
    assert_eq!(allowed.len(), 2);
    assert_eq!(blocked.len(), 2);
    assert!(
        blocked.iter().any(|d| d.id == "d"),
        "Critical must always be blocked"
    );
}

#[test]
fn test_risk_level_hard_block_and_colors_are_consistent() {
    assert!(RiskLevel::Critical.is_hard_blocked());
    for level in [
        RiskLevel::Safe,
        RiskLevel::Low,
        RiskLevel::Medium,
        RiskLevel::High,
    ] {
        assert!(
            !level.is_hard_blocked(),
            "{:?} must not be hard-blocked",
            level
        );
    }
    // Every level paints a distinct colour, so the UI cannot confuse two risks.
    let colors: Vec<(u8, u8, u8)> = [
        RiskLevel::Safe,
        RiskLevel::Low,
        RiskLevel::Medium,
        RiskLevel::High,
        RiskLevel::Critical,
    ]
    .iter()
    .map(|l| l.color_rgb())
    .collect();
    let unique: std::collections::HashSet<_> = colors.iter().collect();
    assert_eq!(unique.len(), 5);
}

// ==================== Profiles ====================

#[test]
fn test_builtin_profiles_are_well_formed() {
    let profiles = wino::profiles::builtins::builtin_profiles();
    assert!(!profiles.is_empty(), "Built-in profiles must exist");

    for profile in &profiles {
        assert!(
            profile.validate().is_ok(),
            "Built-in '{}' must validate",
            profile.id
        );
        assert!(
            profile.is_builtin(),
            "Built-in '{}' must be marked built-in",
            profile.id
        );
        assert!(
            !profile.steps.is_empty(),
            "Built-in '{}' must have steps",
            profile.id
        );
        assert!(
            !profile.description.is_empty(),
            "Built-in '{}' must describe itself",
            profile.id
        );

        for step in &profile.steps {
            assert!(
                !step.target.is_empty(),
                "Step target in '{}' must not be empty",
                profile.id
            );
            assert!(
                !step.label.is_empty(),
                "Step label in '{}' must not be empty",
                profile.id
            );
        }
    }
}

#[test]
fn test_builtin_profiles_declare_no_critical_steps() {
    for profile in wino::profiles::builtins::builtin_profiles() {
        assert!(
            profile.blocked_steps().is_empty(),
            "Built-in '{}' must not declare a Critical step",
            profile.id
        );
    }
}

#[test]
fn test_builtin_profile_ids_match_declared_list() {
    let profiles = wino::profiles::builtins::builtin_profiles();
    let ids: Vec<&str> = profiles.iter().map(|p| p.id.as_str()).collect();
    for expected in wino::profiles::models::BUILTIN_IDS {
        assert!(
            ids.contains(expected),
            "Missing built-in profile '{}'",
            expected
        );
    }
}

#[test]
fn test_builtin_profiles_never_target_do_not_touch_services() {
    use wino::services::scanner::load_service_rules;

    let rules = load_service_rules();
    for profile in wino::profiles::builtins::builtin_profiles() {
        for step in profile.steps.iter().filter(|s| s.kind == StepKind::Service) {
            if let Some(rule) = rules
                .iter()
                .find(|r| r.service_name.eq_ignore_ascii_case(&step.target))
            {
                assert_ne!(
                    rule.classification, "Do not touch",
                    "Built-in '{}' targets protected service '{}'",
                    profile.id, step.target
                );
            }
        }
    }
}

#[test]
fn test_custom_profile_serializes_and_round_trips() {
    let profile = Profile::new(
        "my_profile",
        "My Profile",
        "Test description",
        ProfileOrigin::User,
    )
    .with_step(
        ProfileStep::new(
            StepKind::Power,
            "high_performance",
            "High performance",
            RiskLevel::Low,
        )
        .with_reason("Reduce latency"),
    )
    .with_step(ProfileStep::new(
        StepKind::Service,
        "DiagTrack",
        "Telemetry service",
        RiskLevel::Safe,
    ));

    let toml_text = toml::to_string_pretty(&profile).expect("serialize profile");
    let restored: Profile = toml::from_str(&toml_text).expect("deserialize profile");

    assert_eq!(restored.id, profile.id);
    assert_eq!(restored.steps.len(), 2);
    assert_eq!(restored.steps[0].kind, StepKind::Power);
    assert_eq!(restored.steps[0].reason, "Reduce latency");
    assert_eq!(restored.max_risk(), RiskLevel::Low);
    assert_eq!(restored.origin, ProfileOrigin::User);
}

#[test]
fn test_profile_validation_rejects_incomplete_definitions() {
    let mut profile = Profile::new("ok", "OK", "", ProfileOrigin::User);
    assert!(profile.validate().is_ok());

    profile.name = "  ".to_string();
    assert!(profile.validate().is_err(), "Empty name must be rejected");

    let no_id = Profile::new("", "Named", "", ProfileOrigin::User);
    assert!(no_id.validate().is_err(), "Empty id must be rejected");

    let no_target = Profile::new("ok", "OK", "", ProfileOrigin::User).with_step(ProfileStep::new(
        StepKind::Registry,
        "",
        "label",
        RiskLevel::Safe,
    ));
    assert!(
        no_target.validate().is_err(),
        "Empty step target must be rejected"
    );
}

#[test]
fn test_profile_slugify_produces_safe_ids() {
    assert_eq!(slugify("Gaming (custom)"), "gaming_custom");
    assert_eq!(slugify("  Low   RAM  "), "low_ram");
    assert_eq!(slugify("!!!"), "profile");
}

#[test]
fn test_profile_duplicate_becomes_user_profile() {
    let builtin = Profile::new("gaming", "Gaming", "desc", ProfileOrigin::BuiltIn).with_step(
        ProfileStep::new(StepKind::Power, "high_performance", "HP", RiskLevel::Low),
    );
    let copy = builtin.duplicate_as("gaming_custom", "Gaming (custom)");

    assert_eq!(copy.id, "gaming_custom");
    assert_eq!(copy.origin, ProfileOrigin::User);
    assert_eq!(copy.steps.len(), 1);
    assert!(builtin.is_builtin());
}

#[test]
fn test_apply_report_never_reports_partial_as_complete() {
    let complete = ProfileApplyReport {
        profile_id: "p".to_string(),
        profile_name: "P".to_string(),
        outcomes: vec![StepOutcome {
            label: "step".to_string(),
            kind: StepKind::Registry,
            success: true,
            skipped_reason: None,
        }],
        message: String::new(),
        snapshot_id: None,
    };
    assert!(!complete.is_partial());

    let failed = ProfileApplyReport {
        outcomes: vec![
            StepOutcome {
                label: "a".to_string(),
                kind: StepKind::Registry,
                success: true,
                skipped_reason: None,
            },
            StepOutcome {
                label: "b".to_string(),
                kind: StepKind::Service,
                success: false,
                skipped_reason: None,
            },
        ],
        ..complete.clone()
    };
    assert!(
        failed.is_partial(),
        "A failed step must mark the report partial"
    );
    assert_eq!(failed.failed_count(), 1);

    let blocked = ProfileApplyReport {
        outcomes: vec![StepOutcome {
            label: "c".to_string(),
            kind: StepKind::Registry,
            success: false,
            skipped_reason: Some("Blocked by the Safety Engine".to_string()),
        }],
        ..complete
    };
    assert!(
        blocked.is_partial(),
        "A safety-skipped step must mark the report partial"
    );
    assert_eq!(blocked.skipped_count(), 1);
}

// ==================== Applications ====================

#[test]
fn test_app_package_and_source_classification() {
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

    assert_eq!(AppSource::Win32.label(), "Win32");
    assert_eq!(AppSource::MicrosoftStore.label(), "Microsoft Store");
    assert_eq!(AppSource::AppX.label(), "AppX");
    assert_eq!(AppSource::Winget.label(), "Winget");
    assert_eq!(AppSource::ALL.len(), 4);
}

#[test]
fn test_app_uninstall_availability_and_precedence() {
    assert!(matches!(
        classify_uninstall_method("quiet.exe /q", "plain.exe", "{GUID}", ""),
        Some(UninstallMethod::RegistryQuietCommand { .. })
    ));
    assert!(matches!(
        classify_uninstall_method("", "plain.exe", "{GUID}", ""),
        Some(UninstallMethod::MsiProductCode { .. })
    ));
    assert!(matches!(
        classify_uninstall_method("", "plain.exe", "", ""),
        Some(UninstallMethod::RegistryCommand { .. })
    ));
    assert!(matches!(
        classify_uninstall_method("", "", "", "Pkg_1.0_x64__abc"),
        Some(UninstallMethod::AppxPackage { .. })
    ));

    // No removal path at all: the UI must show "Uninstall unavailable".
    assert_eq!(classify_uninstall_method("", "  ", "", ""), None);
}

#[test]
fn test_app_record_uninstallable_flag_and_matching() {
    let mut record = AppRecord {
        name: "Contoso".to_string(),
        display_name: "Contoso Editor".to_string(),
        publisher: "Contoso Ltd".to_string(),
        version: "1.0".to_string(),
        install_location: r"C:\Program Files\Contoso".to_string(),
        install_size: Some(5 * 1024 * 1024),
        install_date: "20240115".to_string(),
        package_type: "EXE".to_string(),
        source: AppSource::Win32,
        signed: Some(true),
        uninstall: None,
        provisioned: false,
        winget_id: None,
    };
    assert!(
        !record.is_uninstallable(),
        "A record with no method is not uninstallable"
    );
    assert!(record.matches("contoso"));
    assert!(record.matches("editor"));
    assert!(record.matches(""));
    assert!(!record.matches("fabrikam"));
    assert_eq!(record.size_label(), "5.0 MB");

    record.uninstall = Some(UninstallMethod::RegistryCommand {
        command: "unins.exe".to_string(),
    });
    assert!(record.is_uninstallable());
}

#[test]
fn test_app_helpers_format_and_parse() {
    assert_eq!(format_size(512), "512 B");
    assert_eq!(format_size(2048), "2 KB");
    assert_eq!(format_size(5 * 1024 * 1024), "5.0 MB");
    assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3.00 GB");

    assert_eq!(normalize_install_date("20240115"), "2024-01-15");
    assert_eq!(normalize_install_date("01/15/2024"), "01/15/2024");
    assert_eq!(normalize_install_date(""), "");

    let quoted = split_command_line(r#""C:\Program Files\App\unins.exe" /S /Q"#).unwrap();
    assert_eq!(quoted.0, r"C:\Program Files\App\unins.exe");
    assert_eq!(quoted.1, "/S /Q");
    let bare = split_command_line(r"C:\App\unins.exe").unwrap();
    assert_eq!(bare.0, r"C:\App\unins.exe");
    assert!(split_command_line("   ").is_none());
}

#[test]
fn test_app_sort_options_are_all_labelled() {
    assert_eq!(AppSort::ALL.len(), 5);
    for sort in AppSort::ALL {
        assert!(!sort.label().is_empty());
    }
}

#[test]
fn test_winget_update_state_gating() {
    let update = WingetUpdate {
        package_id: "Contoso.App".to_string(),
        name: "Contoso App".to_string(),
        current_version: "1.0".to_string(),
        available_version: "2.0".to_string(),
        app_name: None,
        state: UpdateState::UpdateAvailable,
    };
    assert!(update.is_actionable());
    assert!(!WingetUpdate {
        state: UpdateState::Latest,
        ..update.clone()
    }
    .is_actionable());
    assert!(!WingetUpdate {
        state: UpdateState::Unknown,
        ..update.clone()
    }
    .is_actionable());
    assert!(!WingetUpdate {
        package_id: String::new(),
        ..update
    }
    .is_actionable());
}

#[test]
fn test_winget_missing_status_is_not_an_error_state() {
    let status = WingetStatus::missing("winget not found on PATH");
    assert!(!status.available);
    assert!(status.version.is_empty());
    assert!(
        !status.detail.is_empty(),
        "A missing provider must explain itself"
    );
}

#[test]
fn test_winget_upgrade_table_parsing() {
    // The real shape: banner lines, then a fixed table, then a trailing note.
    let output = "\
Failed in attempting to update the source: winget
Name                          Id                      Version   Available  Source
--------------------------------------------------------------------------------
Mozilla Firefox               Mozilla.Firefox         126.0     127.0      winget
7-Zip 23.01 (x64)             7zip.7zip               23.01     24.05      winget
Contoso Editor                Contoso.Editor          1.0.0     1.2.0      winget
2 upgrades available.
";
    let updates = wino::apps::winget::parse_upgrade_table(output);
    assert_eq!(
        updates.len(),
        3,
        "Header, separator, banner, and trailer must be skipped"
    );

    assert_eq!(updates[0].name, "Mozilla Firefox");
    assert_eq!(updates[0].package_id, "Mozilla.Firefox");
    assert_eq!(updates[0].current_version, "126.0");
    assert_eq!(updates[0].available_version, "127.0");
    assert_eq!(updates[0].state, UpdateState::UpdateAvailable);

    // A package whose name starts with a digit must not be dropped as a count line.
    assert_eq!(updates[1].name, "7-Zip 23.01 (x64)");
    assert_eq!(updates[1].package_id, "7zip.7zip");

    assert!(updates.iter().all(|u| u.is_actionable()));
}

#[test]
fn test_winget_upgrade_table_parsing_handles_empty_and_garbage() {
    assert!(wino::apps::winget::parse_upgrade_table("").is_empty());
    assert!(wino::apps::winget::parse_upgrade_table(
        "No installed package found matching input criteria."
    )
    .is_empty());
}

// ==================== Network diagnostics ====================

#[test]
fn test_network_ip_helpers() {
    assert!(is_link_local_v4("169.254.10.1"));
    assert!(!is_link_local_v4("192.168.1.1"));
    assert!(is_link_local_v6("fe80::1"));
    assert!(!is_link_local_v6("2001:db8::1"));
    assert_eq!(strip_zone_index("fe80::1234%12"), "fe80::1234");
    assert_eq!(strip_zone_index("2001:db8::1"), "2001:db8::1");
}

fn adapter(name: &str, up: bool, ipv4: &[&str], gateways: &[&str]) -> AdapterDetail {
    AdapterDetail {
        friendly_name: name.to_string(),
        description: String::new(),
        guid: format!("{{{}}}", name),
        index: 1,
        ipv4: ipv4
            .iter()
            .map(|a| IpAddressInfo {
                address: a.to_string(),
                prefix_length: 24,
            })
            .collect(),
        ipv6: Vec::new(),
        gateways: gateways.iter().map(|g| g.to_string()).collect(),
        dns_servers: Vec::new(),
        is_up: up,
        dhcp_enabled: Some(true),
        mac_address: String::new(),
        transmit_link_speed: 0,
    }
}

#[test]
fn test_network_adapter_active_selection() {
    let report = NetworkReport {
        adapters: vec![
            adapter("Ethernet", true, &["169.254.10.5"], &[]),
            adapter("Wi-Fi", true, &["192.168.1.20"], &["192.168.1.1"]),
            adapter("Down", false, &["10.0.0.5"], &["10.0.0.1"]),
        ],
        active_adapter: String::new(),
        connected: true,
    };
    assert_eq!(report.identify_active().as_deref(), Some("Wi-Fi"));

    let no_active = NetworkReport {
        adapters: vec![adapter("Down", false, &["10.0.0.5"], &["10.0.0.1"])],
        active_adapter: String::new(),
        connected: false,
    };
    assert_eq!(no_active.identify_active(), None);
    assert_eq!(no_active.active_gateway(), None);
}

#[test]
fn test_network_effective_dns_falls_back_across_adapters() {
    let mut wifi = adapter("Wi-Fi", true, &["192.168.1.20"], &["192.168.1.1"]);
    wifi.dns_servers = vec!["9.9.9.9".to_string()];

    let report = NetworkReport {
        adapters: vec![wifi.clone()],
        active_adapter: "Wi-Fi".to_string(),
        connected: true,
    };
    assert_eq!(report.effective_dns(), vec!["9.9.9.9".to_string()]);

    // Active adapter has no DNS servers of its own: fall back to any that do.
    wifi.dns_servers.clear();
    let mut other = wifi.clone();
    other.friendly_name = "Ethernet".to_string();
    other.dns_servers = vec!["1.1.1.1".to_string()];
    let report = NetworkReport {
        adapters: vec![wifi, other],
        active_adapter: "Wi-Fi".to_string(),
        connected: true,
    };
    assert_eq!(report.effective_dns(), vec!["1.1.1.1".to_string()]);
}

#[test]
fn test_network_ping_result_classification() {
    let no_reply = PingResult {
        host: "example.invalid".to_string(),
        resolved_address: "0.0.0.0".to_string(),
        sent: 4,
        received: 0,
        rtts_ms: Vec::new(),
        last_status: 11010,
    };
    assert!(!no_reply.is_reachable());
    assert_eq!(no_reply.packet_loss_pct(), 100.0);
    assert_eq!(no_reply.avg_rtt_ms(), None);
    assert!(no_reply.summary().contains("No reply"));

    let partial = PingResult {
        host: "1.1.1.1".to_string(),
        resolved_address: "1.1.1.1".to_string(),
        sent: 4,
        received: 3,
        rtts_ms: vec![10, 20, 30],
        last_status: 0,
    };
    assert!(partial.is_reachable());
    assert_eq!(partial.packet_loss_pct(), 25.0);
    assert_eq!(partial.avg_rtt_ms(), Some(20.0));
    assert_eq!(partial.min_rtt_ms(), Some(10));
    assert_eq!(partial.max_rtt_ms(), Some(30));

    // A zero-sample result must not divide by zero.
    let empty = PingResult {
        sent: 0,
        received: 0,
        ..partial
    };
    assert_eq!(empty.packet_loss_pct(), 0.0);
    assert!(empty.summary().contains("No echo requests"));
}

#[test]
fn test_network_dns_lookup_result_classification() {
    let ok = DnsLookupResult {
        host: "example.com".to_string(),
        addresses: vec!["93.184.216.34".to_string()],
        status: 0,
        elapsed_ms: 12,
    };
    assert!(ok.succeeded());
    assert!(ok.summary().contains("1 address"));

    let failed = DnsLookupResult {
        host: "nope.invalid".to_string(),
        addresses: Vec::new(),
        status: 9003,
        elapsed_ms: 5,
    };
    assert!(!failed.succeeded());
    assert!(failed.summary().contains("failed"));
}

#[test]
fn test_network_adapter_summary_prefers_ipv4() {
    let v4 = adapter("A", true, &["192.168.0.5"], &[]);
    assert_eq!(v4.summary(), "192.168.0.5");

    let mut v6 = adapter("B", true, &[], &[]);
    v6.ipv6 = vec![IpAddressInfo {
        address: "2001:db8::5".to_string(),
        prefix_length: 64,
    }];
    assert_eq!(v6.summary(), "2001:db8::5");

    let none = adapter("C", true, &[], &[]);
    assert_eq!(none.summary(), "No address");
}

// ==================== Storage analyzer ====================

#[test]
fn test_storage_path_classification_is_component_anchored() {
    assert_eq!(
        classify_path(r"C:\Windows\System32"),
        StorageBucket::Windows
    );
    // "WindowsApps" is not "Windows": matching must be on whole components.
    assert_eq!(classify_path(r"C:\WindowsApps\Thing"), StorageBucket::Other);
    assert_eq!(
        classify_path(r"C:\Program Files\App"),
        StorageBucket::Applications
    );
    assert_eq!(
        classify_path(r"C:\Program Files (x86)\App"),
        StorageBucket::Applications
    );
    assert_eq!(classify_path(r"C:\Users\Ada"), StorageBucket::Users);
    assert_eq!(
        classify_path(r"C:\ProgramData\Vendor"),
        StorageBucket::ProgramData
    );
    assert_eq!(classify_path(r"C:\"), StorageBucket::Other);
}

#[test]
fn test_storage_temp_bucket_wins_over_parent_and_separators_normalize() {
    assert_eq!(
        classify_path(r"C:\Users\Ada\AppData\Local\Temp"),
        StorageBucket::Temp
    );
    assert_eq!(classify_path(r"C:\Windows\Temp"), StorageBucket::Temp);
    assert_eq!(classify_path("c:/windows/system32"), StorageBucket::Windows);
    assert_eq!(classify_path(r"C:\WINDOWS\"), StorageBucket::Windows);
}

#[test]
fn test_storage_age_and_size_filters() {
    assert!(is_old_file(Some(365), 365));
    assert!(!is_old_file(Some(364), 365));
    assert!(
        !is_old_file(None, 365),
        "An unreadable timestamp is never old"
    );

    assert!(is_large_file(100 * 1024 * 1024, 100));
    assert!(!is_large_file(99 * 1024 * 1024, 100));
}

#[test]
fn test_storage_aggregation_and_partial_detection() {
    let mut tree = StorageTree {
        total_bytes: 1000,
        used_bytes: 250,
        ..Default::default()
    };
    assert!((tree.used_fraction() - 0.25).abs() < 0.001);
    assert!(!tree.is_partial());

    tree.skipped_inaccessible = 1;
    assert!(
        tree.is_partial(),
        "Inaccessible paths must mark the scan partial"
    );

    tree.skipped_inaccessible = 0;
    tree.limit_reached = true;
    assert!(tree.is_partial(), "A limit stop must mark the scan partial");

    tree.limit_reached = false;
    tree.cancelled = true;
    assert!(
        tree.is_partial(),
        "A cancelled scan must mark itself partial"
    );

    assert_eq!(StorageTree::default().used_fraction(), 0.0);
    assert_eq!(
        StorageTree::default().bucket_total(StorageBucket::Windows),
        0
    );
}

#[test]
fn test_storage_entry_fraction_handles_zero_parent() {
    let entry = StorageEntry {
        name: "x".to_string(),
        path: r"C:\x".to_string(),
        is_dir: true,
        size_bytes: 50,
        file_count: 1,
        partial: false,
        bucket: StorageBucket::Other,
    };
    assert_eq!(entry.fraction_of(0), 0.0);
    assert!((entry.fraction_of(100) - 0.5).abs() < 0.001);
}

// ==================== Windows features ====================

#[test]
fn test_feature_state_parsing_never_guesses() {
    assert_eq!(FeatureState::parse("Enabled"), FeatureState::Enabled);
    assert_eq!(FeatureState::parse(" disabled "), FeatureState::Disabled);
    assert_eq!(
        FeatureState::parse("EnablePending"),
        FeatureState::RequiresReboot
    );
    assert_eq!(
        FeatureState::parse("RemovePending"),
        FeatureState::RequiresReboot
    );
    assert_eq!(FeatureState::parse("SomethingElse"), FeatureState::Unknown);
    assert_eq!(FeatureState::parse(""), FeatureState::Unknown);
}

#[test]
fn test_feature_unknown_state_is_not_toggleable() {
    assert!(FeatureState::Enabled.is_toggleable());
    assert!(FeatureState::Disabled.is_toggleable());
    assert!(!FeatureState::Unknown.is_toggleable());
    assert!(!FeatureState::RequiresReboot.is_toggleable());

    let unknown = WindowsFeature {
        name: "F".to_string(),
        display_name: "F".to_string(),
        state: FeatureState::Unknown,
        dependencies: Vec::new(),
        risk: RiskLevel::Low,
        impact: String::new(),
        curated: false,
    };
    assert_eq!(unknown.target_state(), None);
}

#[test]
fn test_feature_dependency_reporting() {
    let make = |name: &str, state: FeatureState| WindowsFeature {
        name: name.to_string(),
        display_name: name.to_string(),
        state,
        dependencies: Vec::new(),
        risk: RiskLevel::Medium,
        impact: String::new(),
        curated: true,
    };

    let all = vec![
        make("VirtualMachinePlatform", FeatureState::Disabled),
        make("Hyper-V", FeatureState::Enabled),
    ];
    let mut target = make("WindowsSandbox", FeatureState::Disabled);
    target.dependencies = vec!["VirtualMachinePlatform".to_string(), "Hyper-V".to_string()];

    let unmet = target.unmet_dependencies(&all);
    assert_eq!(unmet.len(), 1, "Only the disabled dependency is unmet");
    assert_eq!(unmet[0].name, "VirtualMachinePlatform");
}

#[test]
fn test_feature_curation_covers_listeners_and_falls_back() {
    let (name, risk, impact, curated) = apply_curation("OpenSSH.Server", "OpenSSH Server");
    assert_eq!(name, "OpenSSH Server");
    assert_eq!(
        risk,
        RiskLevel::High,
        "An SSH server opens a listener and must be High risk"
    );
    assert!(impact.contains("listener"));
    assert!(curated);

    let (_, risk, impact, curated) = apply_curation("Some-Unknown-Feature", "Friendly");
    assert_eq!(risk, RiskLevel::Low);
    assert!(!curated);
    assert!(
        !impact.is_empty(),
        "Even uncurated features get an explanation"
    );

    assert!(
        find_curated("openssh.server").is_some(),
        "Lookup is case-insensitive"
    );
}

#[test]
fn test_dism_feature_table_parsing() {
    // The real `dism /online /get-features /format:table` shape: a banner, then
    // a pipe-separated table with border lines, then the completion trailer.
    let output = "\
Deployment Image Servicing and Management tool
Version: 10.0.26100.1150

Image Version: 10.0.26100.1

Features listing for package : Microsoft-Windows-Foundation-Package~31bf3856ad364e35~amd64~~10.0.26100.1

------------------------------------------------------------------------------------------ | --------------------------------------------
Feature Name                                                                               | State
------------------------------------------------------------------------------------------ | --------------------------------------------
Hyper-V                                                                                    | Disabled
Microsoft-Hyper-V-All                                                                      | Disabled
OpenSSH.Client                                                                             | Enabled
Something-New                                                                              | SomeUnexpectedState
The operation completed successfully.
";
    let features = wino::windows_features::scanner::parse_dism_feature_table(output);
    assert_eq!(
        features.len(),
        4,
        "Banner, header, borders, and trailer must be skipped"
    );

    let hyperv = features
        .iter()
        .find(|f| f.name == "Hyper-V")
        .expect("Hyper-V present");
    assert_eq!(hyperv.state, FeatureState::Disabled);

    let ssh = features
        .iter()
        .find(|f| f.name == "OpenSSH.Client")
        .expect("OpenSSH present");
    assert_eq!(ssh.state, FeatureState::Enabled);

    // An unrecognized state string is Unknown, never folded into enabled or disabled.
    let unknown = features
        .iter()
        .find(|f| f.name == "Something-New")
        .expect("unknown present");
    assert_eq!(unknown.state, FeatureState::Unknown);
}

#[test]
fn test_dism_feature_table_parsing_handles_empty() {
    assert!(wino::windows_features::scanner::parse_dism_feature_table("").is_empty());
    assert!(wino::windows_features::scanner::parse_dism_feature_table(
        "The operation completed successfully."
    )
    .is_empty());
    // A non-elevated DISM prints an error and no table.
    assert!(wino::windows_features::scanner::parse_dism_feature_table(
        "Error: 740\n\nElevated permissions are required to run DISM."
    )
    .is_empty());
}

// ==================== Health center ====================

#[test]
fn test_health_unknown_status_handling() {
    let checks = vec![
        HealthCheck::new("a", HealthState::Healthy, ""),
        HealthCheck::new("b", HealthState::Unknown, ""),
    ];
    assert_eq!(
        overall_state(&checks),
        HealthState::Unknown,
        "An unqueryable check must not produce a healthy overall verdict"
    );

    assert_eq!(
        overall_state(&[]),
        HealthState::Unknown,
        "An empty report is unknown, not healthy"
    );
}

#[test]
fn test_health_state_classification_is_worst_wins() {
    let checks = vec![
        HealthCheck::new("a", HealthState::Healthy, ""),
        HealthCheck::new("b", HealthState::Attention, ""),
        HealthCheck::new("c", HealthState::Warning, ""),
    ];
    assert_eq!(overall_state(&checks), HealthState::Warning);

    let critical = vec![
        HealthCheck::new("a", HealthState::Warning, ""),
        HealthCheck::new("b", HealthState::Critical, ""),
    ];
    assert_eq!(overall_state(&critical), HealthState::Critical);

    let all_healthy = vec![
        HealthCheck::new("a", HealthState::Healthy, ""),
        HealthCheck::new("b", HealthState::Healthy, ""),
    ];
    assert_eq!(overall_state(&all_healthy), HealthState::Healthy);
}

#[test]
fn test_health_service_and_disk_classification() {
    assert_eq!(
        classify_service_state(Some("Running")),
        HealthState::Healthy
    );
    assert_eq!(
        classify_service_state(Some("Stopped")),
        HealthState::Critical
    );
    assert_eq!(classify_service_state(Some("Paused")), HealthState::Warning);
    assert_eq!(
        classify_service_state(Some("Starting")),
        HealthState::Attention
    );
    assert_eq!(classify_service_state(Some("Weird")), HealthState::Unknown);
    assert_eq!(classify_service_state(None), HealthState::Unknown);

    assert_eq!(classify_disk_free(500, 1000), HealthState::Healthy);
    assert_eq!(classify_disk_free(120, 1000), HealthState::Attention);
    assert_eq!(classify_disk_free(80, 1000), HealthState::Warning);
    assert_eq!(classify_disk_free(20, 1000), HealthState::Critical);
    assert_eq!(classify_disk_free(0, 0), HealthState::Unknown);
}

/// The v2.5 bug this release fixes: the Windows Update health check reported a
/// hard-coded `true` instead of querying the service.
#[test]
fn test_health_windows_update_service_state_is_observed_not_assumed() {
    let unknown = UpdateHealth {
        service_state: None,
        pending_reboot: false,
    };
    assert!(
        !unknown.service_running(),
        "An unqueried service must not read as running"
    );
    assert!(!unknown.service_stopped());

    let stopped = UpdateHealth {
        service_state: Some("Stopped".to_string()),
        pending_reboot: false,
    };
    assert!(stopped.service_stopped());
    assert!(!stopped.service_running());

    let running = UpdateHealth {
        service_state: Some("Running".to_string()),
        pending_reboot: false,
    };
    assert!(running.service_running());
}

#[test]
fn test_health_sfc_interpretation_never_guesses_clean() {
    assert_eq!(
        interpret_sfc_output("Windows Resource Protection did not find any integrity violations."),
        IntegrityOutcome::Clean
    );
    assert_eq!(
        interpret_sfc_output("found corrupt files and successfully repaired them"),
        IntegrityOutcome::Repaired
    );
    assert_eq!(interpret_sfc_output(""), IntegrityOutcome::Unknown);
    assert_eq!(
        interpret_sfc_output("Beginning verification phase of system scan."),
        IntegrityOutcome::Unknown
    );
}

#[test]
fn test_health_report_problems_are_severity_ordered() {
    let report = wino::health::center::HealthReport {
        overall: HealthState::Critical,
        checks: vec![
            HealthCheck::new("ok", HealthState::Healthy, ""),
            HealthCheck::new("warn", HealthState::Warning, ""),
            HealthCheck::new("crit", HealthState::Critical, ""),
            HealthCheck::new("unk", HealthState::Unknown, ""),
        ],
        sfc_outcome: None,
        pending_reboot: false,
    };

    let problems = report.problems();
    assert_eq!(problems.len(), 3);
    assert_eq!(problems[0].name_key, "crit");
    assert_eq!(problems[1].name_key, "warn");
    assert_eq!(problems[2].name_key, "unk");
    assert!(report.has_unknowns());
}

// ==================== Security center ====================

#[test]
fn test_security_classification_never_assumes_protected() {
    assert_eq!(classify_defender(None, None), ProtectionState::On);
    assert_eq!(classify_defender(Some(1), Some(0)), ProtectionState::Off);
    assert_eq!(
        classify_defender(Some(0), Some(1)),
        ProtectionState::Warning
    );

    assert_eq!(classify_firewall(None), ProtectionState::Unknown);
    assert_eq!(classify_firewall(Some(0)), ProtectionState::Off);
    assert_eq!(classify_firewall(Some(1)), ProtectionState::On);

    assert_eq!(classify_secure_boot(None), ProtectionState::Unknown);
    assert_eq!(classify_secure_boot(Some(1)), ProtectionState::On);
    assert_eq!(classify_secure_boot(Some(0)), ProtectionState::Off);

    assert_eq!(classify_uac(Some(0)), ProtectionState::Off);
    assert_eq!(classify_uac(Some(1)), ProtectionState::On);
    assert_eq!(classify_uac(None), ProtectionState::Unknown);

    assert_eq!(classify_tpm(Some("2.0")), ProtectionState::On);
    assert_eq!(classify_tpm(None), ProtectionState::Off);
    assert_eq!(classify_tpm(Some("  ")), ProtectionState::Unknown);
}

#[test]
fn test_security_critical_services_verdict() {
    let stopped = vec![
        ("WinDefend".to_string(), Some("Running".to_string())),
        ("MpsSvc".to_string(), Some("Stopped".to_string())),
        ("Ghost".to_string(), None),
    ];
    let (state, detail) = classify_critical_services(&stopped);
    assert_eq!(state, ProtectionState::Off);
    assert!(detail.contains("MpsSvc"));
    assert!(
        !detail.contains("Ghost"),
        "A stopped service outranks a missing one"
    );

    let all_running = vec![("WinDefend".to_string(), Some("Running".to_string()))];
    assert_eq!(
        classify_critical_services(&all_running).0,
        ProtectionState::On
    );

    let missing_only = vec![("Ghost".to_string(), None)];
    assert_eq!(
        classify_critical_services(&missing_only).0,
        ProtectionState::Warning
    );

    assert_eq!(classify_critical_services(&[]).0, ProtectionState::Unknown);
}

#[test]
fn test_security_report_never_claims_protection_with_unknowns() {
    let report = SecurityReport {
        items: vec![
            ProtectionItem::new("a", ProtectionState::On, ""),
            ProtectionItem::new("b", ProtectionState::Unknown, ""),
        ],
    };
    assert!(
        !report.is_fully_protected(),
        "An unknown component is not full protection"
    );
    assert_eq!(report.unknowns().len(), 1);
    assert_eq!(report.on_count(), 1);
    assert!(report.warnings().is_empty());

    let warning = SecurityReport {
        items: vec![ProtectionItem::new("a", ProtectionState::Off, "")],
    };
    assert_eq!(warning.warnings().len(), 1);
    assert!(warning.items[0].state.needs_attention());
}

// ==================== Snapshots ====================

#[test]
fn test_snapshot_rollback_metadata_reports_categories_and_counts() {
    let mut snap = build_snapshot("Mixed snapshot");
    assert!(!snap.is_restorable(), "An empty snapshot is not restorable");
    assert!(snap.categories().is_empty());

    snap.registry_entries.push(RegistryBackupEntry {
        hive: "HKCU".to_string(),
        path: "Software\\Test".to_string(),
        value_name: "Flag".to_string(),
        previous_value: Some(1),
    });
    snap.power_entries.push(PowerBackupEntry {
        scheme_guid: "AAAA".to_string(),
        subgroup_guid: "BBBB".to_string(),
        setting_guid: "CCCC".to_string(),
        setting_name: "Processor max".to_string(),
        previous_ac_value: Some(100),
        previous_dc_value: Some(50),
    });
    snap.network_entries.push(NetworkBackupEntry {
        interface_guid: "{GUID}".to_string(),
        value_name: "NameServer".to_string(),
        previous_value: Some("1.1.1.1".to_string()),
        adapter_label: "Wi-Fi".to_string(),
    });

    assert_eq!(snap.operation_count(), 3);
    assert!(snap.is_restorable());
    let categories = snap.categories();
    assert!(categories.contains(&SnapshotCategory::Registry));
    assert!(categories.contains(&SnapshotCategory::Power));
    assert!(categories.contains(&SnapshotCategory::Network));
}

#[test]
fn test_snapshot_task_entry_preserves_xml_for_faithful_restore() {
    let entry = TaskBackupEntry {
        task_path: "\\Microsoft\\Windows\\Test".to_string(),
        previous_enabled: true,
        xml_definition: Some(
            "<Task><Settings><Enabled>true</Enabled></Settings></Task>".to_string(),
        ),
        xml_readable: true,
    };
    let json = serde_json::to_string(&entry).expect("serialize");
    let back: TaskBackupEntry = serde_json::from_str(&json).expect("deserialize");
    assert!(back.xml_readable);
    assert!(back
        .xml_definition
        .as_deref()
        .unwrap_or("")
        .contains("<Task>"));

    // A pre-v2.6 entry without the XML fields must still load, and must report
    // that the XML was not captured rather than pretending it has it.
    let legacy = r#"{"task_path":"\\Test","previous_enabled":true}"#;
    let back: TaskBackupEntry = serde_json::from_str(legacy).expect("legacy entry");
    assert!(!back.xml_readable);
    assert!(back.xml_definition.is_none());
}

// ==================== Benchmark ====================

fn sample(ram: u64, procs: usize, startup: usize, temp: u64) -> BenchmarkSample {
    BenchmarkSample {
        id: "1".to_string(),
        label: "test".to_string(),
        timestamp: "2026-01-01 00:00:00".to_string(),
        ram_used_bytes: ram,
        ram_total_bytes: 16 * 1024 * 1024 * 1024,
        process_count: procs,
        startup_count: startup,
        temp_bytes: temp,
        power_plan: "Balanced".to_string(),
        service_states: Vec::new(),
        privacy_states: Vec::new(),
    }
}

#[test]
fn test_benchmark_compare_reports_measured_deltas() {
    let before = sample(5 * 1024 * 1024 * 1024, 120, 17, 8 * 1024 * 1024 * 1024);
    let after = sample(4 * 1024 * 1024 * 1024, 100, 11, 2 * 1024 * 1024 * 1024);
    let rows = compare(&before, &after);

    assert_eq!(rows.len(), 5);
    for row in rows.iter().take(4) {
        assert_eq!(
            row.direction,
            DeltaDirection::Decreased,
            "Row '{}' should be a decrease",
            row.label_key
        );
    }
    assert_eq!(
        rows[4].direction,
        DeltaDirection::Unchanged,
        "An unchanged power plan is reported as unchanged"
    );
}

#[test]
fn test_benchmark_byte_delta_label() {
    let gb = 1024u64 * 1024 * 1024;
    assert_eq!(byte_delta_label(5 * gb, 4 * gb), "-1.00 GB");
    assert_eq!(byte_delta_label(4 * gb, 5 * gb), "+1.00 GB");
    assert_eq!(byte_delta_label(1, 1), "no change");
    assert_eq!(byte_delta_label(0, 120 * 1024 * 1024), "+120 MB");
}

#[test]
fn test_benchmark_service_and_privacy_diffs() {
    let mut before = sample(1, 1, 1, 1);
    before.service_states = vec![("DiagTrack".to_string(), "Running".to_string())];
    before.privacy_states = vec![("privacy_advertising_id".to_string(), false)];

    let after = BenchmarkSample {
        service_states: vec![("DiagTrack".to_string(), "Stopped".to_string())],
        privacy_states: vec![("privacy_advertising_id".to_string(), true)],
        ..before.clone()
    };

    assert_eq!(
        before.service_diffs(&after),
        vec![(
            "DiagTrack".to_string(),
            "Running".to_string(),
            "Stopped".to_string()
        )]
    );
    assert_eq!(
        before.privacy_diffs(&after),
        vec![("privacy_advertising_id".to_string(), false, true)]
    );

    // No change: no diffs reported.
    let unchanged = BenchmarkSample { ..before.clone() };
    assert!(before.service_diffs(&unchanged).is_empty());
    assert!(before.privacy_diffs(&unchanged).is_empty());
}

#[test]
fn test_benchmark_ram_percentage_handles_unknown_total() {
    let mut s = sample(8 * 1024 * 1024 * 1024, 100, 10, 0);
    assert!((s.ram_pct() - 50.0).abs() < 0.01);
    s.ram_total_bytes = 0;
    assert_eq!(s.ram_pct(), 0.0);
}

// ==================== Recommendations ====================

#[test]
fn test_recommendations_sort_by_severity() {
    let mut items = vec![
        Recommendation::new(
            "b",
            RecommendationArea::Memory,
            RecommendationSeverity::Low,
            "low",
            "",
            ReviewTarget::Memory,
        ),
        Recommendation::new(
            "a",
            RecommendationArea::Startup,
            RecommendationSeverity::High,
            "high",
            "",
            ReviewTarget::Startup,
        ),
        Recommendation::new(
            "c",
            RecommendationArea::Storage,
            RecommendationSeverity::Medium,
            "med",
            "",
            ReviewTarget::Storage,
        ),
    ];
    sort_recommendations(&mut items);
    assert_eq!(items[0].severity, RecommendationSeverity::High);
    assert_eq!(items[1].severity, RecommendationSeverity::Medium);
    assert_eq!(items[2].severity, RecommendationSeverity::Low);
}

#[test]
fn test_recommendation_rules_are_observation_only() {
    use wino::recommendations::rules;

    // A healthy machine produces no recommendation for that area.
    assert!(rules::ram_usage(40.0, 6 * 1024 * 1024 * 1024, 16 * 1024 * 1024 * 1024).is_none());
    assert!(rules::disk_free_space(500, 1000).is_none());
    assert!(rules::windows_update_service(Some("Running")).is_none());
    assert!(rules::pending_reboot(false).is_none());
    assert!(rules::commit_charge(1, 100).is_none());

    // An unqueryable Windows Update state is not evidence of a problem.
    assert!(
        rules::windows_update_service(None).is_none(),
        "An unqueryable service state must produce no recommendation"
    );

    // A real problem produces one, carrying the measured value through.
    let ram = rules::ram_usage(92.0, 15 * 1024 * 1024 * 1024, 16 * 1024 * 1024 * 1024)
        .expect("High RAM usage must produce a recommendation");
    assert_eq!(ram.severity, RecommendationSeverity::High);
    assert!(
        !ram.measured.is_empty(),
        "A recommendation must carry its measured value"
    );

    let stopped = rules::windows_update_service(Some("Stopped"))
        .expect("A stopped Windows Update service must produce a recommendation");
    assert!(!stopped.measured.is_empty());

    let defender = rules::defender_real_time_protection(false)
        .expect("Disabled real-time protection must produce a recommendation");
    assert_eq!(defender.severity, RecommendationSeverity::High);
}

// ==================== Startup manager ====================

#[test]
fn test_startup_approved_byte_patterns() {
    // 0x02 -> enabled, 0x03 -> disabled, anything else unrecognized.
    assert_eq!(
        parse_startup_approved(&[0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        Some(true)
    );
    assert_eq!(
        parse_startup_approved(&[0x03, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        Some(false)
    );
    assert_eq!(
        parse_startup_approved(&[0x06, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        Some(true)
    );
    assert_eq!(parse_startup_approved(&[]), None);
}

// ==================== Power ====================

#[test]
fn test_power_plan_kind_classification() {
    assert_eq!(
        PowerPlanKind::from_guid(scheme_guids::BALANCED),
        PowerPlanKind::Balanced
    );
    assert_eq!(
        PowerPlanKind::from_guid("8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"),
        PowerPlanKind::HighPerformance
    );
    assert_eq!(
        PowerPlanKind::from_guid("e9a42b02-d5df-448d-aa00-03f14749eb61"),
        PowerPlanKind::Ultimate
    );
    assert_eq!(
        PowerPlanKind::from_guid("a1841308-3541-4fab-bc81-f71556f20b4a"),
        PowerPlanKind::PowerSaver
    );
    assert_eq!(
        PowerPlanKind::from_guid("11111111-2222-3333-4444-555555555555"),
        PowerPlanKind::Custom
    );
    // Braces and case are normalized before comparison.
    assert_eq!(
        PowerPlanKind::from_guid("{381B4222-F694-41F0-9685-FF5BB260DF2E}"),
        PowerPlanKind::Balanced
    );
}

#[test]
fn test_power_setting_formatting_and_clamping() {
    let setting = PowerSetting {
        id: "processor_boost".to_string(),
        name: "Processor boost".to_string(),
        subgroup_guid: "SUB".to_string(),
        setting_guid: "SET".to_string(),
        ac_value: Some(2),
        dc_value: Some(0),
        min_value: 0,
        max_value: 4,
        unit: String::new(),
        is_enumerated: true,
        enum_labels: vec![
            "Disabled".to_string(),
            "Enabled".to_string(),
            "Aggressive".to_string(),
        ],
        note: String::new(),
    };
    assert_eq!(setting.format_value(Some(2)), "Aggressive");
    assert_eq!(setting.format_value(None), "—");
    assert!(setting.is_available());
    assert_eq!(setting.clamp(99), 4);

    let ranged = PowerSetting {
        is_enumerated: false,
        unit: "%".to_string(),
        min_value: 5,
        max_value: 100,
        ..setting
    };
    assert_eq!(ranged.format_value(Some(100)), "100 %");
    assert_eq!(ranged.clamp(0), 5);
    assert_eq!(ranged.clamp(50), 50);

    let unavailable = PowerSetting {
        ac_value: None,
        dc_value: None,
        ..ranged
    };
    assert!(
        !unavailable.is_available(),
        "A setting the scheme does not expose must report as unavailable"
    );
}

#[test]
fn test_power_setting_labels_exist_for_every_exposed_id() {
    for id in wino::power::models::EXPOSED_SETTING_IDS {
        let key = wino::power::models::setting_label_key(id);
        assert_ne!(
            key, "power.setting_unavailable",
            "Setting '{}' must have its own label key",
            id
        );
    }
}

// ==================== Cancellation ====================

#[test]
fn test_cancel_token_shared_state() {
    let token = CancelToken::new();
    let observer = token.clone();
    assert!(!token.is_cancelled());

    token.cancel();
    assert!(observer.is_cancelled(), "Clones must observe the same flag");

    observer.reset();
    assert!(!token.is_cancelled());
}

// ==================== Configuration ====================

#[test]
fn test_config_defaults_are_sane() {
    let config = AppConfig::default();
    assert_eq!(config.general.language, "en");
    assert_eq!(config.profiles.default_profile, "Balanced");
    assert_eq!(config.network.ping_count, 4);
    assert!(config.storage.max_entries > 0, "A scan budget must exist");
    assert!(config.storage.max_depth > 0, "A depth ceiling must exist");
    assert!(!config.health.include_dism, "The slow DISM check is opt-in");
    assert!(
        !config.apps.scan_winget_metadata,
        "Winget metadata is opt-in"
    );
}

/// A `wino.toml` written by v2.5 has none of the v2.6 sections. Every new field
/// is `#[serde(default)]`, so it must load rather than reset the user's config.
#[test]
fn test_config_v25_toml_still_loads() {
    let v25 = r#"
[general]
theme = "dark"
dry_run = false
minimize_to_tray = false
auto_create_restore_point = true
language = "id"

[memory]
mode = "smart"
refresh_interval_ms = 1000
auto_trim_threshold_pct = 85.0
smart_optimize_enabled = true

[monitoring]
active_poll_ms = 500
background_poll_ms = 3000
adaptive_polling = true

[privacy]
disable_telemetry = true
disable_advertising_id = true
disable_activity_feed = true

[debloat]
default_preset = "Safe"
"#;
    let config: AppConfig = toml::from_str(v25).expect("A v2.5 config must still load");
    assert_eq!(
        config.general.language, "id",
        "Existing values must survive"
    );
    assert_eq!(config.debloat.default_preset, "Safe");
    assert_eq!(
        config.profiles.default_profile, "Balanced",
        "New sections fall back to defaults"
    );
    assert_eq!(config.storage.large_file_mb, 100);
    assert_eq!(config.health.scan_interval_secs, 30);
}

// ==================== Subprocess helper ====================

#[test]
fn test_tool_output_stream_selection() {
    let stdout_only = ToolOutput {
        success: true,
        exit_code: Some(0),
        stdout: "progress\n".to_string(),
        stderr: String::new(),
    };
    assert_eq!(stdout_only.combined(), "progress");
    assert_eq!(stdout_only.last_line(), "progress");

    let both = ToolOutput {
        success: false,
        exit_code: Some(1),
        stdout: "out".to_string(),
        stderr: "err".to_string(),
    };
    assert_eq!(both.combined(), "out\nerr");
    assert_eq!(
        both.last_line(),
        "err",
        "The decisive line for a failure is usually on stderr"
    );

    let empty = ToolOutput {
        success: true,
        exit_code: Some(0),
        stdout: "  \n".to_string(),
        stderr: String::new(),
    };
    assert_eq!(empty.last_line(), "");
}
