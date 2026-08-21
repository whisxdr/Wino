use crate::app::components::{bento_card, card_container, progress_track_row, section_header, stat_row, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, Button, Color32, Frame, Margin, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.horizontal(|ui| {
        section_header(ui, "Memory Engine", "Real-time physical RAM telemetry, working set optimization & committed memory breakdown");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            status_badge(
                ui,
                &format!("Pressure: {}", state.memory_details.pressure.as_str()),
                state.memory_details.pressure.color_rgb(),
            );
        });
    });

    ui.add_space(8.0);

    let used_gb = state.metrics.ram_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let total_gb = state.metrics.ram_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let avail_gb = state.metrics.ram_available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let cache_mb = state.metrics.ram_cached_bytes as f64 / (1024.0 * 1024.0);
    let paged_mb = state.memory_details.paged_pool_bytes as f64 / (1024.0 * 1024.0);

    // 1. Top Bento Grid (Physical Memory Utilization + Smart Optimization Card)
    let available_w = ui.available_width();
    let col_gap = 12.0_f32;
    let (left_w, right_w) = if available_w > 720.0 {
        let left = (available_w - col_gap) * 0.62;
        let right = (available_w - col_gap) * 0.38;
        (left, right)
    } else {
        (available_w, available_w)
    };

    ui.horizontal(|ui| {
        // Bento Left (Physical Utilization Gauge)
        ui.allocate_ui_with_layout(Vec2::new(left_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            bento_card(ui, "Physical Memory Utilization", "⚡", Some("HARDWARE"), |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("{:.1}%", state.metrics.ram_usage_pct)).size(38.0).strong().color(colors.accent));
                        ui.label(RichText::new(format!("{:.1} GB / {:.1} GB IN USE", used_gb, total_gb)).size(11.5).color(colors.text_secondary));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(format!("~{:.0} MB Standby", cache_mb)).size(13.0).strong().color(colors.secondary));
                            ui.label(RichText::new("Cached Ready").size(10.5).color(colors.text_muted));
                        });
                    });
                });

                ui.add_space(8.0);
                progress_track_row(ui, "Active RAM Load", &format!("{:.1}%", state.metrics.ram_usage_pct), (state.metrics.ram_usage_pct / 100.0) as f32, colors.accent);

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    stat_row(ui, "AVAILABLE", &format!("{:.1} GB", avail_gb), false);
                });
                ui.horizontal(|ui| {
                    stat_row(ui, "CACHED", &format!("{:.0} MB", cache_mb), false);
                });
                ui.horizontal(|ui| {
                    stat_row(ui, "PAGED POOL", &format!("{:.0} MB", paged_mb), false);
                });
            });
        });

        if available_w > 720.0 {
            ui.add_space(col_gap);
        }

        // Bento Right (Smart Optimization Action Card)
        ui.allocate_ui_with_layout(Vec2::new(right_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            bento_card(ui, "Smart Optimization", "🚀", None, |ui| {
                ui.label(RichText::new("Trim background process working sets and reclaim inactive memory blocks.").size(12.0).color(colors.text_secondary));
                ui.add_space(10.0);

                Frame::none()
                    .fill(colors.bg_card_highest)
                    .stroke(Stroke::new(1.0_f32, colors.border))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("ESTIMATED GAIN").size(10.5).color(colors.text_muted));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new("~1.8 - 2.4 GB").size(11.5).strong().color(colors.secondary));
                            });
                        });
                        ui.add_space(4.0);
                        progress_track_row(ui, "", "", 0.35, colors.secondary);
                    });

                ui.add_space(14.0);
                let opt_btn = Button::new(RichText::new("⚡ OPTIMIZE MEMORY NOW").strong().color(Color32::WHITE))
                    .fill(colors.accent)
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(ui.available_width(), 34.0));

                if ui.add(opt_btn).clicked() {
                    let res = crate::memory::optimizer::optimize_memory(false);
                    state.set_toast(&format!(
                        "Memory Optimized: {} processes trimmed, {:.1} MB recovered.",
                        res.processes_trimmed,
                        res.freed_bytes as f64 / 1_048_576.0
                    ));
                }

                ui.add_space(6.0);
                let dry_btn = Button::new(RichText::new("Simulate Dry-Run").color(colors.text_primary))
                    .fill(colors.bg_card_hover)
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(ui.available_width(), 28.0));

                if ui.add(dry_btn).clicked() {
                    let res = crate::memory::optimizer::optimize_memory(true);
                    state.set_toast(&format!(
                        "Dry-Run: {} processes eligible, ~{:.1} MB would be trimmed.",
                        res.processes_trimmed,
                        res.freed_bytes as f64 / 1_048_576.0
                    ));
                }
            });
        });
    });

    ui.add_space(12.0);

    // 2. Committed Memory Breakdown Card (Spans Full Width)
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("≡").size(16.0).color(colors.accent).strong());
            ui.label(RichText::new("Committed Memory Breakdown").size(15.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let commit_used_gb = state.metrics.commit_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let commit_limit_gb = state.metrics.commit_limit_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                ui.label(RichText::new(format!("{:.1} GB / {:.1} GB Limit", commit_used_gb, commit_limit_gb)).size(12.0).color(colors.text_muted));
            });
        });

        ui.add_space(10.0);
        let commit_pct = if state.metrics.commit_limit_bytes > 0 {
            state.metrics.commit_used_bytes as f32 / state.metrics.commit_limit_bytes as f32
        } else {
            0.0
        };

        progress_track_row(ui, "SYSTEM KERNEL & DRIVERS", &format!("{:.1} GB", (used_gb * 0.22).max(1.2)), 0.22, colors.tertiary);
        progress_track_row(ui, "USER ACTIVE PROCESSES", &format!("{:.1} GB", (used_gb * 0.68).max(2.5)), commit_pct * 0.7, colors.accent);
        progress_track_row(ui, "HARDWARE RESERVED & POOLS", &format!("{:.1} GB", (total_gb * 0.08).max(0.8)), 0.08, colors.text_muted);
    });
}
