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
        paged_pool_bytes: 512 * 1024 * 1024,
        nonpaged_pool_bytes: 256 * 1024 * 1024,
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
        processor_name: "AMD Ryzen 9".to_string(),
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
        string_entries: Vec::new(),
        task_entries: Vec::new(),
    };

    let json = serde_json::to_string(&snap).expect("Failed to serialize snapshot");
    let deserialized: Snapshot = serde_json::from_str(&json).expect("Failed to deserialize snapshot");
    assert_eq!(snap.id, deserialized.id);
    assert_eq!(snap.description, deserialized.description);
}

#[test]
fn test_snapshot_backward_compat_old_json_without_new_fields() {
    // Old snapshots persisted before v2.5 lack string_entries / task_entries.
    let old_json = r#"{"id":"oldid","timestamp":"2026-01-01 00:00:00","description":"Old snapshot","registry_entries":[],"service_entries":[]}"#;
    let snap: Snapshot = serde_json::from_str(old_json).expect("Old snapshot JSON must deserialize with defaults");
    assert!(snap.string_entries.is_empty());
    assert!(snap.task_entries.is_empty());
}

// ── Ring buffer VecDeque history ─────────────────────────────────────────

#[test]
fn test_push_history_sample_caps_at_history_capacity() {
    use std::collections::VecDeque;
    use wino::app::state::{push_history_sample, HISTORY_CAPACITY};

    let mut buf: VecDeque<f32> = VecDeque::new();
    for i in 0..(HISTORY_CAPACITY + 10) {
        push_history_sample(&mut buf, i as f32);
    }
    assert_eq!(buf.len(), HISTORY_CAPACITY);
    // Oldest samples were evicted — first retained value should be 10.
    assert_eq!(buf.front().copied(), Some(10.0));
    assert_eq!(buf.back().copied(), Some((HISTORY_CAPACITY + 9) as f32));
}

// ── i18n dictionary completeness ─────────────────────────────────────────

#[test]
fn test_i18n_keys_have_translations_in_both_languages() {
    use wino::core::i18n::{tr, Lang, KEYS};
    assert!(!KEYS.is_empty(), "KEYS registry must not be empty");

    for key in KEYS {
        let en = tr(Lang::En, key);
        assert!(!en.is_empty(), "EN translation missing for key '{}'", key);
        let id = tr(Lang::Id, key);
        assert!(!id.is_empty(), "ID translation missing for key '{}'", key);
    }
}

#[test]
fn test_i18n_unknown_key_returns_empty_and_lang_code_roundtrips() {
    use wino::core::i18n::{tr, Lang};
    assert_eq!(tr(Lang::En, "nonexistent.key.xyz"), "");
    assert_eq!(Lang::from_code("id"), Lang::Id);
    assert_eq!(Lang::from_code("en"), Lang::En);
    assert_eq!(Lang::from_code("xx"), Lang::En);
    assert_eq!(Lang::Id.code(), "id");
    assert_eq!(Lang::En.code(), "en");
}

// ── DNS presets ──────────────────────────────────────────────────────────

#[test]
fn test_dns_presets_lookup_and_value_joining() {
    use wino::network::dns::{find_preset, ALL_PRESETS};

    assert_eq!(ALL_PRESETS.len(), 4);
    let cf = find_preset("cloudflare").expect("cloudflare preset must exist");
    assert_eq!(cf.primary, Some("1.1.1.1"));
    assert_eq!(cf.secondary, Some("1.0.0.1"));
    assert_eq!(cf.name_server_value(), "1.1.1.1,1.0.0.1");

    let google = find_preset("google").unwrap();
    assert_eq!(google.name_server_value(), "8.8.8.8,8.8.4.4");

    let quad9 = find_preset("quad9").unwrap();
    assert_eq!(quad9.name_server_value(), "9.9.9.9,149.112.112.112");

    let auto = find_preset("auto").unwrap();
    assert_eq!(auto.primary, None);
    assert_eq!(auto.name_server_value(), "");
    assert!(find_preset("unknown_xyz").is_none());
}

// ── Scheduled tasks pure helpers ─────────────────────────────────────────

#[test]
fn test_tasks_heuristics_and_rule_lookup() {
    use wino::tasks::rules::{find_rule, matches_third_party_heuristic};

    assert!(matches_third_party_heuristic("googleupdatetaskmachinecore"));
    assert!(matches_third_party_heuristic("adobe arm updater"));
    assert!(!matches_third_party_heuristic(r"\microsoft\windows\defender\scan"));

    let rule = find_rule(r"\microsoft\windows\application experience\microsoft compatibility appraiser")
        .expect("Compatibility Appraiser rule must be found (case-insensitive)");
    assert_eq!(rule.name, "Microsoft Compatibility Appraiser");

    assert!(find_rule(r"\nonexistent\task\path").is_none());
}

#[test]
fn test_tasks_enabled_flag_parsing_from_xml() {
    use wino::tasks::scanner::enabled_flag_from_xml_for_test;

    let xml_true = r#"<Task><Settings><Enabled>true</Enabled></Settings></Task>"#;
    assert_eq!(enabled_flag_from_xml_for_test(xml_true), Some(true));

    let xml_false = r#"<Enabled>false</Enabled>"#;
    assert_eq!(enabled_flag_from_xml_for_test(xml_false), Some(false));

    let xml_missing = r#"<Task><Triggers/></Task>"#;
    assert_eq!(enabled_flag_from_xml_for_test(xml_missing), None);
}

// ── Auto memory trimmer decision ─────────────────────────────────────────

#[test]
fn test_should_trim_respects_threshold_and_cooldown() {
    use std::time::Duration;
    use wino::memory::auto_trim::should_trim;

    // Below threshold — must not trim regardless of cooldown.
    assert!(!should_trim(70.0, 85.0, Duration::from_secs(999), Duration::from_secs(10)));
    // Above threshold but cooldown not yet elapsed.
    assert!(!should_trim(90.0, 85.0, Duration::from_secs(5), Duration::from_secs(300)));
    // Above threshold and cooldown elapsed — should trim.
    assert!(should_trim(90.0, 85.0, Duration::from_secs(400), Duration::from_secs(300)));
    // Exactly at threshold counts as triggered.
    assert!(should_trim(85.0, 85.0, Duration::from_secs(300), Duration::from_secs(300)));
}
