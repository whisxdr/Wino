use wino::cleaner::scanner::load_cleanup_rules;
use wino::core::safety::{RiskLevel, SafetyEngine};
use wino::core::system::SystemInfo;
use wino::debloat::rules::load_default_rules;
use wino::memory::pressure::{calculate_memory_pressure, MemoryPressure};
use wino::monitoring::ram::RamStats;
use wino::privacy::policies::load_privacy_rules;
use wino::restore::snapshots::Snapshot;
use wino::services::scanner::load_service_rules;

#[test]
fn test_memory_pressure_calculation() {
    // 1. Healthy / Low pressure test
    let healthy_stats = RamStats {
        total_bytes: 16 * 1024 * 1024 * 1024,
        used_bytes: 8 * 1024 * 1024 * 1024,
        available_bytes: 8 * 1024 * 1024 * 1024,
        cached_bytes: 2 * 1024 * 1024 * 1024,
        commit_used_bytes: 9 * 1024 * 1024 * 1024,
        commit_limit_bytes: 18 * 1024 * 1024 * 1024,
        usage_pct: 50.0,
        process_count: 80,
        handle_count: 20000,
        thread_count: 1000,
    };
    assert_eq!(calculate_memory_pressure(&healthy_stats), MemoryPressure::Low);

    // 2. High usage with plenty of available memory test
    let moderate_stats = RamStats {
        usage_pct: 75.0,
        available_bytes: 4 * 1024 * 1024 * 1024,
        commit_used_bytes: 10 * 1024 * 1024 * 1024,
        commit_limit_bytes: 18 * 1024 * 1024 * 1024,
        ..healthy_stats
    };
    assert_eq!(calculate_memory_pressure(&moderate_stats), MemoryPressure::Moderate);

    // 3. Critical memory pressure test
    let critical_stats = RamStats {
        usage_pct: 95.0,
        available_bytes: 500 * 1024 * 1024, // < 1 GB
        commit_used_bytes: 17 * 1024 * 1024 * 1024,
        commit_limit_bytes: 18 * 1024 * 1024 * 1024,
        ..healthy_stats
    };
    assert_eq!(calculate_memory_pressure(&critical_stats), MemoryPressure::Critical);
}

#[test]
fn test_debloat_rules_load_and_validate() {
    let rules = load_default_rules();
    assert!(!rules.is_empty(), "Debloat rules should not be empty");

    for r in &rules {
        assert!(!r.id.is_empty(), "Rule ID must not be empty");
        assert!(!r.name.is_empty(), "Rule Name must not be empty");
    }
}

#[test]
fn test_privacy_rules_load_and_validate() {
    let rules = load_privacy_rules();
    assert!(!rules.is_empty(), "Privacy rules should not be empty");

    for r in &rules {
        assert!(!r.id.is_empty(), "Privacy rule ID must not be empty");
        assert!(!r.name.is_empty(), "Privacy rule Name must not be empty");
    }
}

#[test]
fn test_service_rules_load_and_validate() {
    let rules = load_service_rules();
    assert!(!rules.is_empty(), "Service rules should not be empty");

    let rpc_rule = rules.iter().find(|s| s.service_name.eq_ignore_ascii_case("rpcss"));
    assert!(rpc_rule.is_some(), "RpcSs must be present in service rules");
    assert_eq!(rpc_rule.unwrap().classification, "Do not touch");
}

#[test]
fn test_cleanup_rules_load_and_validate() {
    let rules = load_cleanup_rules();
    assert!(!rules.is_empty(), "Cleanup rules should not be empty");
}

#[test]
fn test_risk_preset_gating() {
    assert!(RiskLevel::Safe.is_safe_for_preset("Safe"));
    assert!(!RiskLevel::Low.is_safe_for_preset("Safe"));

    assert!(RiskLevel::Safe.is_safe_for_preset("Balanced"));
    assert!(RiskLevel::Low.is_safe_for_preset("Balanced"));
    assert!(!RiskLevel::Medium.is_safe_for_preset("Balanced"));

    assert!(RiskLevel::Medium.is_safe_for_preset("Aggressive"));
    assert!(!RiskLevel::Critical.is_safe_for_preset("Aggressive"));
}

#[test]
fn test_safety_engine_blocks_critical() {
    let sys_info = SystemInfo {
        os_name: "Windows 11 Pro".to_string(),
        display_version: "23H2".to_string(),
        build_number: "22631".to_string(),
        architecture: "x86_64".to_string(),
        processor_count: 8,
        total_ram_bytes: 16 * 1024 * 1024 * 1024,
        is_windows_11: true,
        is_admin: true,
    };

    let result = SafetyEngine::validate_operation(
        "Critical Kernel Service",
        RiskLevel::Critical,
        true,
        &["11".to_string()],
        &sys_info,
    );

    assert!(!result.is_allowed, "Safety engine must block critical risk operations");
}

#[test]
fn test_snapshot_serialization() {
    let snap = Snapshot {
        id: "test1234".to_string(),
        timestamp: "2026-08-21 10:00:00".to_string(),
        description: "Test Snapshot".to_string(),
        registry_entries: Vec::new(),
        service_entries: Vec::new(),
    };

    let json = serde_json::to_string(&snap).expect("Failed to serialize snapshot");
    let deserialized: Snapshot = serde_json::from_str(&json).expect("Failed to deserialize snapshot");
    assert_eq!(snap.id, deserialized.id);
    assert_eq!(snap.description, deserialized.description);
}
