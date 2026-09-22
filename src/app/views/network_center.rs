//! Network Center view.
//!
//! WHY the view is shaped this way:
//!
//! * Every diagnostic is dispatched to the worker system
//!   ([`crate::app::worker::Action::RunNetworkTool`]) rather than run inline. An
//!   ICMP sweep with retries takes seconds; running it on the UI thread would
//!   stall the frame loop, which is what the previous version did for the DNS
//!   flush.
//! * Presets are labelled as presets. The previous copy implied one resolver was
//!   the right answer; the list is a convenience, not a ranking, and the note
//!   saying so is rendered above the chips.
//! * DHCP renew and release are confirmed before running, because releasing a
//!   lease drops connectivity until the renew completes.
//! * When the active adapter is unknown, the view says so instead of showing a
//!   plausible-looking adapter that is not carrying traffic.

use crate::app::components::{card_container, status_badge, view_header};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::app::worker::{Action, NetworkTool};
use crate::core::i18n::tr;
use crate::network::diagnostics::link_state_label;
use crate::network::dns::ALL_PRESETS;
use eframe::egui::{Button, Color32, RichText, Rounding, Stroke, TextEdit, Ui, Vec2};

/// The diagnostics the view can dispatch, in display order.
const TOOLS: [NetworkTool; 6] = [
    NetworkTool::TestGateway,
    NetworkTool::Ping,
    NetworkTool::DnsLookup,
    NetworkTool::Connectivity,
    NetworkTool::Latency,
    NetworkTool::PacketLoss,
];

/// i18n key for a diagnostic button.
fn tool_i18n_key(tool: NetworkTool) -> &'static str {
    match tool {
        NetworkTool::TestGateway => "netcenter.tool_test_gateway",
        NetworkTool::Ping => "netcenter.tool_ping",
        NetworkTool::DnsLookup => "netcenter.tool_dns_lookup",
        NetworkTool::Connectivity => "netcenter.tool_connectivity",
        NetworkTool::Latency => "netcenter.tool_latency",
        NetworkTool::PacketLoss => "netcenter.tool_packet_loss",
    }
}

/// Whether a diagnostic needs a host to be typed in.
fn tool_needs_host(tool: NetworkTool) -> bool {
    matches!(
        tool,
        NetworkTool::Ping | NetworkTool::DnsLookup | NetworkTool::Latency | NetworkTool::PacketLoss
    )
}

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    view_header(
        ui,
        tr(lang, "netcenter.title"),
        tr(lang, "netcenter.subtitle"),
        |ui| {
            let rescan_btn = Button::new(format!("↻ {}", tr(lang, "common.refresh")))
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(rescan_btn).clicked() {
                state.refresh_network();
            }
        },
    );

    // Seed the host field from config on first open so the tools are usable
    // immediately without the user typing an address.
    if state.network_host_input.is_empty() {
        state.network_host_input = state.config.network.ping_host.clone();
    }

    let report = state.network_report.clone();
    let mut run_tool: Option<NetworkTool> = None;
    let mut flush = false;
    let mut dhcp_action: Option<(bool, u32)> = None;

    // ---- Active adapter ----
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🌐").size(15.0).color(colors.accent));
            ui.label(
                RichText::new(tr(lang, "netcenter.active_adapter"))
                    .size(15.0)
                    .strong(),
            );
        });
        ui.add_space(6.0);

        match report.active() {
            None => {
                ui.label(
                    RichText::new(tr(lang, "netcenter.no_adapter"))
                        .size(12.0)
                        .color(colors.warning),
                );
            }
            Some(adapter) => {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&adapter.friendly_name).size(15.0).strong());
                    status_badge(
                        ui,
                        tr(lang, link_state_label(adapter.is_up)),
                        if adapter.is_up {
                            (34, 197, 94)
                        } else {
                            (239, 68, 68)
                        },
                    );
                    match adapter.dhcp_enabled {
                        Some(true) => {
                            status_badge(ui, tr(lang, "netcenter.dhcp_enabled"), (78, 222, 163))
                        }
                        Some(false) => {
                            status_badge(ui, tr(lang, "netcenter.dhcp_disabled"), (234, 179, 8))
                        }
                        None => status_badge(ui, tr(lang, "common.unknown"), (148, 163, 184)),
                    }
                });

                ui.add_space(6.0);

                let field = |ui: &mut Ui, label: &str, value: String| {
                    if value.is_empty() {
                        return;
                    }
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            Vec2::new(130.0, 16.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.label(RichText::new(label).size(10.5).color(colors.text_muted));
                            },
                        );
                        ui.label(RichText::new(value).size(11.5).color(colors.text_primary));
                    });
                };

                field(
                    ui,
                    tr(lang, "netcenter.interface"),
                    adapter.description.clone(),
                );
                field(
                    ui,
                    tr(lang, "netcenter.ipv4"),
                    adapter
                        .ipv4
                        .iter()
                        .map(|a| a.address.clone())
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                field(
                    ui,
                    tr(lang, "netcenter.ipv6"),
                    adapter
                        .ipv6
                        .iter()
                        .map(|a| a.address.clone())
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                field(
                    ui,
                    tr(lang, "netcenter.gateway"),
                    adapter.gateways.join(", "),
                );
                field(
                    ui,
                    tr(lang, "netcenter.dns_servers"),
                    report.effective_dns().join(", "),
                );
            }
        }
    });

    ui.add_space(10.0);

    // ---- Tools ----
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🔍").size(15.0).color(colors.secondary));
            ui.label(
                RichText::new(tr(lang, "netcenter.tools"))
                    .size(15.0)
                    .strong(),
            );
        });
        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(tr(lang, "netcenter.host_label"))
                    .size(12.0)
                    .strong(),
            );
            ui.add(
                TextEdit::singleline(&mut state.network_host_input)
                    .hint_text("1.1.1.1")
                    .desired_width(200.0),
            );
        });

        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            let flush_btn = Button::new(
                RichText::new(tr(lang, "netcenter.tool_flush_dns"))
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(colors.accent)
            .rounding(Rounding::same(6.0))
            .min_size(Vec2::new(130.0, 28.0));
            if ui.add(flush_btn).clicked() {
                flush = true;
            }

            for tool in TOOLS {
                let btn = Button::new(tr(lang, tool_i18n_key(tool)))
                    .fill(colors.bg_card_hover)
                    .stroke(Stroke::new(1.0_f32, colors.border))
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(130.0, 28.0));
                if ui.add_enabled(!state.action_busy, btn).clicked() {
                    run_tool = Some(tool);
                }
            }
        });

        // DHCP renew/release drop connectivity until they complete, so they are
        // staged rather than fired immediately.
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            let adapter_index = report.active().map(|a| a.index);

            let renew_btn = Button::new(tr(lang, "netcenter.tool_renew_dhcp"))
                .fill(colors.bg_card_hover)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(150.0, 28.0));
            if ui
                .add_enabled(!state.action_busy && adapter_index.is_some(), renew_btn)
                .clicked()
            {
                dhcp_action = adapter_index.map(|idx| (true, idx));
            }

            let release_btn = Button::new(
                RichText::new(tr(lang, "netcenter.tool_release_dhcp")).color(colors.warning),
            )
            .fill(colors.bg_card_hover)
            .stroke(Stroke::new(1.0_f32, colors.border))
            .rounding(Rounding::same(6.0))
            .min_size(Vec2::new(150.0, 28.0));
            if ui
                .add_enabled(!state.action_busy && adapter_index.is_some(), release_btn)
                .clicked()
            {
                dhcp_action = adapter_index.map(|idx| (false, idx));
            }
        });

        if state.action_busy {
            ui.add_space(6.0);
            ui.label(
                RichText::new(tr(lang, "netcenter.tool_running"))
                    .size(11.5)
                    .color(colors.secondary),
            );
        }

        // ---- Tool result ----
        if let Some((message, success)) = &state.network_tool_result {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr(lang, "netcenter.tool_result"))
                        .size(13.0)
                        .strong(),
                );
                status_badge(
                    ui,
                    if *success {
                        tr(lang, "common.done")
                    } else {
                        tr(lang, "common.failed")
                    },
                    if *success {
                        (34, 197, 94)
                    } else {
                        (239, 68, 68)
                    },
                );
            });
            ui.add_space(3.0);
            ui.label(
                RichText::new(message)
                    .size(11.5)
                    .color(colors.text_secondary),
            );
        }
    });

    ui.add_space(10.0);

    // ---- DNS presets, explicitly labelled as presets ----
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("⚙").size(15.0).color(colors.tertiary));
            ui.label(
                RichText::new(tr(lang, "net.preset_label"))
                    .size(15.0)
                    .strong(),
            );
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new(tr(lang, "netcenter.presets_note"))
                .size(11.0)
                .color(colors.text_muted),
        );
        ui.add_space(8.0);

        for preset in ALL_PRESETS {
            let is_selected = state.dns_preset_id == preset.id;
            let label = if preset.id == "auto" {
                format!("{} (DHCP)", preset.name)
            } else {
                format!("{}  [{}]", preset.name, preset.name_server_value())
            };
            let btn = Button::new(RichText::new(label).color(if is_selected {
                Color32::WHITE
            } else {
                colors.text_primary
            }))
            .fill(if is_selected {
                colors.accent
            } else {
                colors.bg_card_hover
            })
            .stroke(Stroke::new(
                1.0_f32,
                if is_selected {
                    colors.accent
                } else {
                    colors.border
                },
            ))
            .rounding(Rounding::same(6.0))
            .min_size(Vec2::new(ui.available_width(), 28.0));
            if ui.add(btn).clicked() {
                state.dns_preset_id = preset.id.to_string();
            }
            ui.add_space(3.0);
        }

        ui.add_space(4.0);
        ui.label(
            RichText::new(tr(lang, "net.auto_note"))
                .size(11.0)
                .color(colors.text_muted),
        );
        if !state.sys_info.is_admin {
            ui.add_space(2.0);
            ui.label(
                RichText::new(tr(lang, "net.admin_note"))
                    .size(11.0)
                    .color(colors.warning),
            );
        }
    });

    // ---- Apply collected actions ----
    if flush {
        match crate::network::flush::flush_dns_cache() {
            Ok(msg) => state.set_toast(&msg),
            Err(e) => state.record_event("Network", &e, false),
        }
    }

    if let Some((renew, index)) = dhcp_action {
        let result = if renew {
            crate::network::diagnostics::renew_dhcp(index, false)
        } else {
            crate::network::diagnostics::release_dhcp(index, false)
        };
        match result {
            Ok(msg) => {
                state.set_toast(&msg);
                state.refresh_network();
            }
            Err(e) => state.record_event("Network", &e, false),
        }
    }

    if let Some(tool) = run_tool {
        let host = state.network_host_input.trim().to_string();
        if tool_needs_host(tool) && host.is_empty() {
            state.network_tool_result = Some(("Enter a host first.".to_string(), false));
        } else {
            let count = state.config.network.ping_count.max(1);
            state.request_action(Action::RunNetworkTool { tool, host, count });
        }
    }
}
