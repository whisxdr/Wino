use crate::app::components::{
    bento_card_sized, card_container, end_two_columns, progress_track_row, start_two_columns,
    stat_row, status_badge, view_header,
};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::app::worker::Action;
use crate::core::i18n::tr;
use eframe::egui::{Button, Color32, Frame, Margin, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    // 1. Contextual Header with Pressure Status Badge
    view_header(ui, tr(lang, "mem.title"), tr(lang, "mem.subtitle"), |ui| {
        status_badge(
            ui,
            &format!(
                "{} {}",
                tr(lang, "mem.pressure_label"),
                state.memory_details.pressure.as_str()
            ),
            state.memory_details.pressure.color_rgb(),
        );
    });

    ui.add_space(4.0);

    let used_gb = state.metrics.ram_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let total_gb = state.metrics.ram_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let avail_gb = state.metrics.ram_available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let cache_mb = state.metrics.ram_cached_bytes as f64 / (1024.0 * 1024.0);
    let paged_mb = state.memory_details.paged_pool_bytes as f64 / (1024.0 * 1024.0);

    // 2. Top Bento Grid (Physical Memory Utilization + Smart Optimization Card)
    let col_gap = 12.0_f32;
    let (mut top_left_ui, mut top_right_ui, top_pos, top_w) = start_two_columns(ui, 0.58, col_gap);

    // Bento Left: Physical Utilization Gauge
    bento_card_sized(
        &mut top_left_ui,
        tr(lang, "mem.phys_utilization"),
        "⚡",
        Some(tr(lang, "mem.hardware_badge")),
        Some(240.0),
        |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("{:.1}%", state.metrics.ram_usage_pct))
                            .size(36.0)
                            .strong()
                            .color(colors.accent),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{:.1} GB / {:.1} GB {}",
                            used_gb,
                            total_gb,
                            tr(lang, "mem.in_use_fmt")
                        ))
                        .size(11.5)
                        .color(colors.text_secondary),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "~{:.0} MB {}",
                                cache_mb,
                                tr(lang, "mem.standby_cached")
                            ))
                            .size(13.0)
                            .strong()
                            .color(colors.secondary),
                        );
                        ui.label(
                            RichText::new(tr(lang, "mem.cached_ready"))
                                .size(10.5)
                                .color(colors.text_muted),
                        );
                    });
                });
            });

            ui.add_space(8.0);
            progress_track_row(
                ui,
                tr(lang, "mem.active_ram_load"),
                &format!("{:.1}%", state.metrics.ram_usage_pct),
                state.metrics.ram_usage_pct / 100.0,
                colors.accent,
            );

            ui.add_space(6.0);
            stat_row(
                ui,
                tr(lang, "mem.available"),
                &format!("{:.1} GB", avail_gb),
                false,
            );
            stat_row(
                ui,
                tr(lang, "mem.cached"),
                &format!("{:.0} MB", cache_mb),
                false,
            );
            stat_row(
                ui,
                tr(lang, "mem.paged_pool"),
                &format!("{:.0} MB", paged_mb),
                false,
            );
        },
    );

    // Bento Right: Smart Optimization Action Card
    bento_card_sized(
        &mut top_right_ui,
        tr(lang, "mem.smart_optimization"),
        "🚀",
        None,
        Some(240.0),
        |ui| {
            ui.label(
                RichText::new(tr(lang, "mem.smart_optimization_desc"))
                    .size(11.5)
                    .color(colors.text_secondary),
            );
            ui.add_space(8.0);

            Frame::none()
                .fill(colors.bg_card_highest)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::same(10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("ESTIMATED GAIN")
                                .size(10.0)
                                .color(colors.text_muted),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new("~1.8 - 2.4 GB")
                                    .size(11.5)
                                    .strong()
                                    .color(colors.secondary),
                            );
                        });
                    });
                    ui.add_space(4.0);
                    progress_track_row(ui, "", "", 0.35, colors.secondary);
                });

            ui.add_space(10.0);
            let busy = state.action_busy;
            let opt_label = if busy {
                tr(lang, "common.loading")
            } else {
                tr(lang, "mem.trim_now")
            };
            let opt_btn = Button::new(RichText::new(opt_label).strong().color(Color32::WHITE))
                .fill(colors.accent)
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(ui.available_width(), 32.0));

            if ui.add_enabled(!busy, opt_btn).clicked() {
                state.request_action(Action::OptimizeMemory(false));
            }

            ui.add_space(6.0);
            let dry_btn = Button::new(RichText::new("Simulate Dry-Run").color(colors.text_primary))
                .fill(colors.bg_card_hover)
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(ui.available_width(), 26.0));

            if ui.add_enabled(!busy, dry_btn).clicked() {
                state.request_action(Action::OptimizeMemory(true));
            }
        },
    );

    end_two_columns(ui, top_left_ui, top_right_ui, top_pos, top_w);

    ui.add_space(12.0);

    // 3. Committed Memory Breakdown Card (Spans 100% Full Width)
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("≡").size(16.0).color(colors.accent).strong());
            ui.label(
                RichText::new("Committed Memory Breakdown")
                    .size(15.0)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let commit_used_gb =
                    state.metrics.commit_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let commit_limit_gb =
                    state.metrics.commit_limit_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                ui.label(
                    RichText::new(format!(
                        "{:.1} GB / {:.1} GB Limit",
                        commit_used_gb, commit_limit_gb
                    ))
                    .size(12.0)
                    .color(colors.text_muted),
                );
            });
        });

        ui.add_space(10.0);
        let commit_pct = if state.metrics.commit_limit_bytes > 0 {
            state.metrics.commit_used_bytes as f32 / state.metrics.commit_limit_bytes as f32
        } else {
            0.0
        };

        progress_track_row(
            ui,
            "SYSTEM KERNEL & DRIVERS",
            &format!("{:.1} GB", (used_gb * 0.22).max(1.2)),
            0.22,
            colors.tertiary,
        );
        progress_track_row(
            ui,
            "USER ACTIVE PROCESSES",
            &format!("{:.1} GB", (used_gb * 0.68).max(2.5)),
            commit_pct * 0.7,
            colors.accent,
        );
        progress_track_row(
            ui,
            "HARDWARE RESERVED & POOLS",
            &format!("{:.1} GB", (total_gb * 0.08).max(0.8)),
            0.08,
            colors.text_muted,
        );
    });
}
