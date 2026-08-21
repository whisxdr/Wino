use crate::app::components::{card_container, metric_card, section_header, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, Button, Color32, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.horizontal(|ui| {
        section_header(ui, "System Dashboard", "Real-time Windows telemetry and health status");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            status_badge(
                ui,
                state.health_report.rating.as_str(),
                state.health_report.rating.color_rgb(),
            );
        });
    });

    ui.add_space(4.0);

    // Top Metric Cards Row with responsive spacing
    ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
    ui.horizontal_wrapped(|ui| {
        let cpu_pct = state.metrics.cpu_usage_pct;
        metric_card(
            ui,
            "CPU Usage",
            &format!("{:.1}%", cpu_pct),
            &format!("{} Processors", state.sys_info.processor_count),
            if cpu_pct > 80.0 { colors.danger } else { colors.accent },
        );

        let ram_used_gb = state.metrics.ram_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let ram_tot_gb = state.metrics.ram_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        metric_card(
            ui,
            "RAM Usage",
            &format!("{:.1}%", state.metrics.ram_usage_pct),
            &format!("{:.1} / {:.1} GB", ram_used_gb, ram_tot_gb),
            colors.accent,
        );

        metric_card(
            ui,
            "Memory Pressure",
            state.memory_details.pressure.as_str(),
            &format!("{:.1} GB Free", state.metrics.ram_available_bytes as f64 / (1024.0 * 1024.0 * 1024.0)),
            Color32::from_rgb(
                state.memory_details.pressure.color_rgb().0,
                state.memory_details.pressure.color_rgb().1,
                state.memory_details.pressure.color_rgb().2,
            ),
        );

        let disk_used_gb = (state.metrics.disk_total_bytes.saturating_sub(state.metrics.disk_free_bytes)) as f64 / (1024.0 * 1024.0 * 1024.0);
        let disk_tot_gb = state.metrics.disk_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        metric_card(
            ui,
            "Primary Drive (C:)",
            &format!("{:.1}%", state.metrics.disk_usage_pct),
            &format!("{:.1} / {:.1} GB", disk_used_gb, disk_tot_gb),
            if state.metrics.disk_usage_pct > 90.0 { colors.danger } else { colors.success },
        );

        let net_kb = (state.metrics.net_recv_bytes_per_sec + state.metrics.net_send_bytes_per_sec) / 1024;
        metric_card(
            ui,
            "Network",
            &format!("{} KB/s", net_kb),
            &format!("Down: {} KB/s", state.metrics.net_recv_bytes_per_sec / 1024),
            colors.accent,
        );
    });

    ui.add_space(14.0);

    // Health & Recommendations Card
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("System Health & Recommendations").size(14.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Review Details").clicked() {
                    state.current_tab = crate::app::navigation::NavTab::Health;
                }
            });
        });

        ui.add_space(6.0);

        if state.health_report.issues.is_empty() {
            ui.horizontal(|ui| {
                ui.label(RichText::new("✓").color(colors.success).strong());
                ui.label("No critical system changes required. Windows is operating stably.");
            });
        } else {
            for issue in &state.health_report.issues {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("!").color(colors.warning).strong());
                    ui.label(issue);
                });
            }
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Recommended Actions:").size(11.5).color(colors.text_muted));
                for r in &state.health_report.recommendations {
                    ui.label(format!("  • {}", r));
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let opt_btn = Button::new(RichText::new("Smart Optimize").strong().color(Color32::WHITE))
                    .fill(colors.accent)
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(130.0, 34.0));

                if ui.add(opt_btn).clicked() {
                    let rep = crate::memory::optimizer::optimize_memory(false);
                    state.set_toast(&rep.message);
                }
            });
        });
    });

    ui.add_space(14.0);

    // Host & Hardware Information
    card_container(ui, |ui| {
        ui.label(RichText::new("Host Information").size(14.0).strong());
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Operating System:").color(colors.text_muted));
            ui.label(format!("{} {} (Build {})", state.sys_info.os_name, state.sys_info.display_version, state.sys_info.build_number));
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("GPU Adapter:").color(colors.text_muted));
            ui.label(&state.metrics.gpu_name);
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Active Processes:").color(colors.text_muted));
            ui.label(format!("{} running", state.metrics.process_count));
        });
    });
}
