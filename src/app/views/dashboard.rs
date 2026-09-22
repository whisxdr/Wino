use crate::app::components::{
    bento_card_sized, card_container, end_two_columns, live_histogram, progress_track_row,
    start_two_columns, stat_row, status_badge, view_header,
};
use crate::app::navigation::NavTab;
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::app::worker::Action;
use crate::core::i18n::tr;
use eframe::egui::{self, Button, Color32, Frame, Margin, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    // 1. Contextual Header with Refresh Button
    view_header(
        ui,
        tr(state.lang, "dash.title"),
        tr(state.lang, "dash.subtitle"),
        |ui| {
            let refresh_btn =
                Button::new(format!("↻ {}", tr(state.lang, "dash.refresh_telemetry")))
                    .fill(colors.bg_card)
                    .stroke(Stroke::new(1.0_f32, colors.border))
                    .rounding(Rounding::same(6.0));
            if ui.add(refresh_btn).clicked() {
                state.refresh_processes();
            }
        },
    );

    ui.add_space(4.0);

    // 2. Top Bento Grid (CPU Telemetry + Host Information)
    let col_gap = 12.0_f32;
    let (mut top_left_ui, mut top_right_ui, top_pos, top_w) = start_two_columns(ui, 0.58, col_gap);

    // Bento 1: CPU Telemetry
    bento_card_sized(
        &mut top_left_ui,
        tr(state.lang, "dash.cpu_telemetry"),
        "⚡",
        Some(tr(state.lang, "common.live_badge")),
        Some(210.0),
        |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&state.sys_info.processor_name)
                            .size(11.5)
                            .color(colors.text_muted),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(format!(
                            "{} Cores / {} Threads",
                            state.sys_info.processor_count,
                            state.sys_info.processor_count * 2
                        ))
                        .size(11.0)
                        .color(colors.text_secondary),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!("{:.1}%", state.metrics.cpu_usage_pct))
                                .size(26.0)
                                .strong()
                                .color(colors.accent),
                        );
                        ui.label(
                            RichText::new(tr(state.lang, "dash.active_load"))
                                .size(10.5)
                                .color(colors.secondary),
                        );
                    });
                });
            });

            ui.add_space(8.0);
            // Live Glowing Histogram Chart
            live_histogram(ui, state.cpu_history.make_contiguous(), 100.0, 105.0);
        },
    );

    // Bento 2: Host Information
    bento_card_sized(
        &mut top_right_ui,
        tr(state.lang, "dash.host_info"),
        "🗖",
        None,
        Some(210.0),
        |ui| {
            let os_display = if state.sys_info.display_version.is_empty() {
                state.sys_info.os_name.clone()
            } else {
                format!(
                    "{} ({})",
                    state.sys_info.os_name, state.sys_info.display_version
                )
            };
            stat_row(ui, "OS BUILD", &os_display, false);
            stat_row(ui, "UPTIME", &state.sys_info.get_uptime_formatted(), false);

            let gpu_display = if state.metrics.gpu_name.is_empty() {
                "GPU Connected"
            } else {
                &state.metrics.gpu_name
            };
            stat_row(ui, "GPU TARGET", gpu_display, false);

            stat_row(
                ui,
                "ACTIVE PROCS",
                &state.metrics.process_count.to_string(),
                true,
            );
            stat_row(ui, "ARCHITECTURE", &state.sys_info.architecture, false);
        },
    );

    end_two_columns(ui, top_left_ui, top_right_ui, top_pos, top_w);

    ui.add_space(10.0);

    // 3. Middle Row: RAM & Storage Quick Cards (50% / 50%)
    let (mut mid_left_ui, mut mid_right_ui, mid_pos, mid_w) = start_two_columns(ui, 0.50, col_gap);

    // RAM Quick Card
    bento_card_sized(
        &mut mid_left_ui,
        tr(state.lang, "dash.memory_engine"),
        "⚡",
        Some("RAM"),
        Some(170.0),
        |ui| {
            let used_gb = state.metrics.ram_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let total_gb = state.metrics.ram_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let avail_gb = state.metrics.ram_available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{:.1} GB / {:.1} GB", used_gb, total_gb))
                        .size(16.0)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{:.1}%", state.metrics.ram_usage_pct))
                            .size(16.0)
                            .strong()
                            .color(colors.accent),
                    );
                });
            });

            ui.add_space(6.0);
            progress_track_row(
                ui,
                tr(state.lang, "dash.physical_utilization"),
                &format!("{:.1} {}", avail_gb, tr(state.lang, "dash.gb_available")),
                state.metrics.ram_usage_pct / 100.0,
                colors.accent,
            );

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let busy = state.action_busy;
                let trim_label = if busy {
                    tr(state.lang, "common.loading")
                } else {
                    tr(state.lang, "dash.quick_trim")
                };
                let trim_btn =
                    Button::new(RichText::new(trim_label).strong().color(Color32::WHITE))
                        .fill(colors.accent)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(130.0, 26.0));
                if ui.add_enabled(!busy, trim_btn).clicked() {
                    state.request_action(Action::OptimizeMemory(false));
                }
                if ui.button(tr(state.lang, "dash.inspect_engine")).clicked() {
                    state.current_tab = NavTab::Memory;
                }
            });
        },
    );

    // Storage Quick Card
    bento_card_sized(
        &mut mid_right_ui,
        tr(state.lang, "dash.storage_status"),
        "🗑",
        Some("DRIVE"),
        Some(170.0),
        |ui| {
            let free_gb = state.metrics.disk_free_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let total_gb = state.metrics.disk_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{:.1} GB Free", free_gb))
                        .size(16.0)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{:.1}% Used", state.metrics.disk_usage_pct))
                            .size(16.0)
                            .strong()
                            .color(colors.secondary),
                    );
                });
            });

            ui.add_space(6.0);
            progress_track_row(
                ui,
                tr(state.lang, "dash.system_volume_usage"),
                &format!("{:.1} {}", total_gb, tr(state.lang, "dash.gb_total")),
                state.metrics.disk_usage_pct / 100.0,
                colors.secondary,
            );

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let clean_btn = Button::new(
                    RichText::new(tr(state.lang, "dash.scan_junk")).color(colors.text_primary),
                )
                .fill(colors.bg_card_hover)
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(130.0, 26.0));
                if ui.add(clean_btn).clicked() {
                    state.refresh_cleaner();
                    state.current_tab = NavTab::Cleaner;
                }
            });
        },
    );

    end_two_columns(ui, mid_left_ui, mid_right_ui, mid_pos, mid_w);

    ui.add_space(10.0);

    // 4. Bottom Bento Card: System Health & Actions
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("✚")
                    .size(16.0)
                    .color(colors.secondary)
                    .strong(),
            );
            ui.label(
                RichText::new(tr(state.lang, "dash.health_actions"))
                    .size(15.0)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if state.health_report.defender_active && state.health_report.firewall_active {
                    status_badge(ui, tr(state.lang, "dash.protected_ok"), (78, 222, 163));
                } else {
                    status_badge(ui, tr(state.lang, "dash.attention_needed"), (255, 185, 95));
                }
            });
        });

        ui.add_space(8.0);

        // Security Status Row
        Frame::none()
            .fill(if state.health_report.defender_active {
                Color32::from_rgba_unmultiplied(78, 222, 163, 15)
            } else {
                Color32::from_rgba_unmultiplied(239, 68, 68, 20)
            })
            .stroke(Stroke::new(
                1.0_f32,
                if state.health_report.defender_active {
                    Color32::from_rgba_unmultiplied(78, 222, 163, 60)
                } else {
                    Color32::from_rgb(239, 68, 68)
                },
            ))
            .rounding(Rounding::same(6.0))
            .inner_margin(Margin::symmetric(12.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if state.health_report.defender_active {
                        ui.label(RichText::new("🛡").size(16.0).color(colors.secondary));
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(tr(state.lang, "dash.av_active_title"))
                                    .size(13.0)
                                    .strong()
                                    .color(colors.secondary),
                            );
                            ui.label(
                                RichText::new(tr(state.lang, "dash.av_active_desc"))
                                    .size(11.0)
                                    .color(colors.text_secondary),
                            );
                        });
                    } else {
                        ui.label(RichText::new("⚠️").size(16.0).color(colors.danger));
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(tr(state.lang, "dash.av_off_title"))
                                    .size(13.0)
                                    .strong()
                                    .color(colors.danger),
                            );
                            ui.label(
                                RichText::new(tr(state.lang, "dash.av_off_desc"))
                                    .size(11.0)
                                    .color(colors.text_secondary),
                            );
                        });
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr(state.lang, "dash.open_health")).clicked() {
                            state.current_tab = NavTab::Health;
                        }
                    });
                });
            });
    });
}
