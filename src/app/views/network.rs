use crate::app::components::{action_card, card_container, section_header, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::core::i18n::tr;
use crate::network::dns::{list_adapters, set_adapter_dns, ALL_PRESETS};
use crate::network::flush::flush_dns_cache;
use eframe::egui::{Button, Color32, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    section_header(ui, tr(lang, "net.title"), tr(lang, "net.subtitle"));

    // 1. Flush DNS Cache Card (100% Full Width)
    action_card(
        ui,
        170.0,
        |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🌐").size(15.0).color(colors.accent));
                ui.label(RichText::new("DNS Resolver Cache").size(14.5).strong());
            });
            ui.add_space(2.0);
            ui.label(RichText::new(tr(lang, "net.flush_hint")).size(12.0).color(colors.text_secondary));
        },
        |ui| {
            let flush_btn = Button::new(RichText::new(tr(lang, "net.flush_cache")).strong().color(Color32::WHITE))
                .fill(colors.accent)
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(160.0, 32.0));
            if ui.add(flush_btn).clicked() {
                match flush_dns_cache() {
                    Ok(msg) => state.set_toast(&msg),
                    Err(e) => state.record_event("Network", &e, false),
                }
            }
        },
    );

    ui.add_space(10.0);

    // 2. DNS Preset Selector Card (100% Full Width)
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("⚡").size(15.0).color(colors.secondary));
            ui.label(RichText::new(tr(lang, "net.preset_label")).size(14.5).strong());
        });
        ui.add_space(8.0);

        for preset in ALL_PRESETS {
            let is_selected = state.dns_preset_id == preset.id;
            let (fill, text_color, border_color) = if is_selected {
                (colors.accent, Color32::WHITE, colors.accent)
            } else {
                (colors.bg_card_hover, colors.text_primary, colors.border)
            };
            let label = if preset.id == "auto" {
                format!("{} (DHCP)", preset.name)
            } else {
                format!("{}  [{}]", preset.name, preset.name_server_value())
            };
            let btn = Button::new(RichText::new(label).color(text_color))
                .fill(fill)
                .stroke(Stroke::new(1.0_f32, border_color))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(ui.available_width(), 28.0));
            if ui.add(btn).clicked() {
                state.dns_preset_id = preset.id.to_string();
            }
            ui.add_space(3.0);
        }

        ui.add_space(6.0);
        ui.label(RichText::new(tr(lang, "net.auto_note")).size(11.0).color(colors.text_muted));

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let apply_btn = Button::new(RichText::new(tr(lang, "net.apply_all_adapters")).strong().color(Color32::WHITE))
                .fill(Color32::from_rgb(34, 197, 94))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(190.0, 32.0));
            if ui.add(apply_btn).clicked() {
                apply_selected_preset(state);
            }
            if !state.sys_info.is_admin {
                ui.label(RichText::new(format!("⚠ {}", tr(lang, "net.admin_note"))).size(11.0).color(colors.warning));
            }
        });
    });

    ui.add_space(10.0);

    // 3. Network Adapters List (100% Full Width)
    ui.label(RichText::new(tr(lang, "net.adapters")).size(14.5).strong());
    ui.add_space(4.0);

    let adapters = list_adapters();
    if adapters.is_empty() {
        card_container(ui, |ui| {
            ui.label(RichText::new(tr(lang, "tasks.none_found")).size(12.0).color(colors.text_muted));
        });
    }

    for adapter in &adapters {
        action_card(
            ui,
            90.0,
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&adapter.friendly_name).size(13.5).strong());
                    if !adapter.current_name_server.is_empty() {
                        status_badge(ui, "Static", (234, 179, 8));
                    } else {
                        status_badge(ui, "DHCP", (34, 197, 94));
                    }
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        tr(lang, "net.current_servers"),
                        if adapter.current_name_server.is_empty() { "Automatic" } else { &adapter.current_name_server }
                    ))
                    .size(11.0)
                    .color(colors.text_muted),
                );
            },
            |ui| {
                let name = adapter.friendly_name.clone();
                let btn = Button::new(tr(lang, "common.apply"))
                    .fill(colors.bg_card_hover)
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(80.0, 26.0));
                if ui.add(btn).clicked() {
                    if let Some(preset) = crate::network::dns::find_preset(&state.dns_preset_id) {
                        match set_adapter_dns(adapter, preset, false) {
                            Ok(()) => {
                                let _ = flush_dns_cache();
                                state.set_toast(&format!("DNS '{}' applied to {}.", preset.name, name));
                            }
                            Err(e) => state.record_event("Network", &format!("DNS update failed: {}", e), false),
                        }
                    }
                }
            },
        );
        ui.add_space(4.0);
    }
}

fn apply_selected_preset(state: &mut AppState) {
    let Some(preset) = crate::network::dns::find_preset(&state.dns_preset_id) else { return };
    let adapters = list_adapters();

    if adapters.is_empty() {
        state.record_event("Network", "No configurable network adapters found.", false);
        return;
    }

    let mut ok = 0usize;
    for adapter in &adapters {
        if set_adapter_dns(adapter, preset, false).is_ok() {
            ok += 1;
        }
    }

    let flushed = flush_dns_cache().is_ok();
    state.set_toast(&format!(
        "Preset '{}' applied to {}/{} adapters{}. Reconnect Wi-Fi/Ethernet once if changes are not instant.",
        preset.name,
        ok,
        adapters.len(),
        if flushed { " and DNS cache flushed" } else { "" }
    ));
}


