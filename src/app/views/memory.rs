use crate::app::components::{card_container, metric_card, section_header, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, Button, Color32, ProgressBar, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.horizontal(|ui| {
        section_header(ui, "Memory Engine", "Advanced multi-metric pressure analysis & safe working set management");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            status_badge(
                ui,
                &format!("Pressure: {}", state.memory_details.pressure.as_str()),
                state.memory_details.pressure.color_rgb(),
            );
        });
    });

    // Memory Usage Bar Card
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Physical Memory Utilization").size(15.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!("{:.1}% Used", state.metrics.ram_usage_pct)).strong());
            });
        });

        ui.add_space(8.0);
        let progress = state.metrics.ram_usage_pct / 100.0;
        ui.add(ProgressBar::new(progress).show_percentage());

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let used_gb = state.metrics.ram_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let total_gb = state.metrics.ram_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let avail_gb = state.metrics.ram_available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let cache_mb = state.metrics.ram_cached_bytes as f64 / (1024.0 * 1024.0);

            ui.label(format!("Used: {:.2} GB", used_gb));
            ui.label("|");
            ui.label(format!("Available: {:.2} GB", avail_gb));
            ui.label("|");
            ui.label(format!("Total: {:.2} GB", total_gb));
            ui.label("|");
            ui.label(format!("Standby Cache: {:.0} MB", cache_mb));
        });
    });

    ui.add_space(16.0);

    // Commit & Compression Stats
    ui.horizontal_wrapped(|ui| {
        let commit_used_gb = state.metrics.commit_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let commit_limit_gb = state.metrics.commit_limit_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        metric_card(
            ui,
            "Committed Memory",
            &format!("{:.1} GB", commit_used_gb),
            &format!("Limit: {:.1} GB", commit_limit_gb),
            colors.accent,
        );

        let comp_mb = state.memory_details.compressed_bytes as f64 / (1024.0 * 1024.0);
        metric_card(
            ui,
            "Memory Compression",
            if state.memory_details.compression_enabled { "Enabled" } else { "Disabled" },
            &format!("~{:.0} MB Compressed", comp_mb),
            colors.success,
        );

        metric_card(
            ui,
            "Active Handles",
            &format!("{}", state.memory_details.stats.handle_count),
            &format!("{} Threads", state.memory_details.stats.thread_count),
            colors.text_muted,
        );
    });

    ui.add_space(16.0);

    // Smart Optimization Action Card
    card_container(ui, |ui| {
        ui.label(RichText::new("Smart Memory Optimization").size(15.0).strong());
        ui.add_space(4.0);
        ui.label(RichText::new("Safely trims working sets of idle background processes. Wino never continuously flushes RAM (zero placebo).").size(12.0).color(colors.text_muted));

        ui.add_space(14.0);
        ui.horizontal(|ui| {
            let opt_btn = Button::new(RichText::new("Optimize Memory Now").strong().color(Color32::WHITE))
                .fill(colors.accent)
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(160.0, 36.0));

            if ui.add(opt_btn).clicked() {
                let rep = crate::memory::optimizer::optimize_memory(false);
                state.set_toast(&rep.message);
            }

            let dry_btn = Button::new(RichText::new("Simulate (Dry-Run)").color(colors.text_primary))
                .fill(colors.bg_card_hover)
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(140.0, 36.0));

            if ui.add(dry_btn).clicked() {
                let rep = crate::memory::optimizer::optimize_memory(true);
                state.set_toast(&rep.message);
            }
        });
    });
}
