use crate::app::components::{bento_card, card_container, live_histogram, progress_track_row, section_header, stat_row, status_badge};
use crate::app::navigation::NavTab;
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, Button, Color32, Frame, Margin, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    // Contextual Header
    ui.horizontal(|ui| {
        section_header(
            ui,
            "System Overview",
            "Real-time telemetry, hardware metrics & automated health recommendations",
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("↻ Refresh Telemetry").clicked() {
                state.refresh_processes();
            }
        });
    });

    ui.add_space(6.0);

    // 1. Top Bento Grid (CPU Telemetry + Host Information)
    let available_w = ui.available_width();
    let col_gap = 12.0_f32;
    let (left_w, right_w) = if available_w > 720.0 {
        let left = (available_w - col_gap) * 0.64;
        let right = (available_w - col_gap) * 0.36;
        (left, right)
    } else {
        (available_w, available_w)
    };

    ui.horizontal(|ui| {
        // Bento 1: CPU Telemetry (Spans ~64% width)
        ui.allocate_ui_with_layout(Vec2::new(left_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            bento_card(ui, "CPU Telemetry", "⚡", Some("LIVE"), |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&state.sys_info.processor_name).size(11.5).color(colors.text_muted));
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new(format!("{} Cores / {} Threads", state.sys_info.processor_count, state.sys_info.processor_count * 2))
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
                            ui.label(RichText::new("Active Load").size(10.5).color(colors.secondary));
                        });
                    });
                });

                ui.add_space(8.0);
                // Live Glowing Histogram Chart
                live_histogram(ui, &state.cpu_history, 100.0, 110.0);
            });
        });

        if available_w > 720.0 {
            ui.add_space(col_gap);
        }

        // Bento 2: Host Information (Spans ~36% width)
        ui.allocate_ui_with_layout(Vec2::new(right_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            bento_card(ui, "Host Info", "🗖", None, |ui| {
                let os_display = format!("{} ({})", state.sys_info.os_name, state.sys_info.display_version);
                stat_row(ui, "OS BUILD", if os_display.len() > 24 { &os_display[..24] } else { &os_display }, false);
                stat_row(ui, "UPTIME", &state.sys_info.get_uptime_formatted(), false);
                
                let gpu_display = if state.metrics.gpu_name.is_empty() { "GPU Connected" } else { &state.metrics.gpu_name };
                stat_row(ui, "GPU TARGET", if gpu_display.len() > 22 { &gpu_display[..22] } else { gpu_display }, false);
                
                stat_row(ui, "ACTIVE PROCS", &state.metrics.process_count.to_string(), true);
                stat_row(ui, "ARCHITECTURE", &state.sys_info.architecture, false);
            });
        });
    });

    ui.add_space(10.0);

    // 2. Middle Row: RAM & Storage Quick Cards
    ui.horizontal(|ui| {
        let card_w = (ui.available_width() - col_gap) / 2.0;

        // RAM Quick Card
        ui.allocate_ui_with_layout(Vec2::new(card_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            bento_card(ui, "Memory Engine", "⚡", Some("RAM"), |ui| {
                let used_gb = state.metrics.ram_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let total_gb = state.metrics.ram_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let avail_gb = state.metrics.ram_available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{:.1} GB / {:.1} GB", used_gb, total_gb)).size(16.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{:.1}%", state.metrics.ram_usage_pct)).size(16.0).strong().color(colors.accent));
                    });
                });

                ui.add_space(6.0);
                progress_track_row(ui, "Physical Utilization", &format!("{:.1} GB Available", avail_gb), (state.metrics.ram_usage_pct / 100.0) as f32, colors.accent);

                ui.horizontal(|ui| {
                    let trim_btn = Button::new(RichText::new("⚡ Quick Memory Trim").strong().color(Color32::WHITE))
                        .fill(colors.accent)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(140.0, 26.0));
                    if ui.add(trim_btn).clicked() {
                        let res = crate::memory::optimizer::optimize_memory(false);
                        state.set_toast(&format!("Trimmed {} processes: {:.1} MB recovered.", res.processes_trimmed, res.freed_bytes as f64 / 1_048_576.0));
                    }
                    if ui.button("Inspect Engine →").clicked() {
                        state.current_tab = NavTab::Memory;
                    }
                });
            });
        });

        ui.add_space(col_gap);

        // Storage Quick Card
        ui.allocate_ui_with_layout(Vec2::new(card_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            bento_card(ui, "Storage Status (C:)", "🗑", Some("DRIVE"), |ui| {
                let free_gb = state.metrics.disk_free_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let total_gb = state.metrics.disk_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{:.1} GB Free", free_gb)).size(16.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{:.1}% Used", state.metrics.disk_usage_pct)).size(16.0).strong().color(colors.secondary));
                    });
                });

                ui.add_space(6.0);
                progress_track_row(ui, "System Volume Usage", &format!("{:.1} GB Total", total_gb), (state.metrics.disk_usage_pct / 100.0) as f32, colors.secondary);

                ui.horizontal(|ui| {
                    let clean_btn = Button::new(RichText::new("Scan Junk Files").color(colors.text_primary))
                        .fill(colors.bg_card_hover)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(130.0, 26.0));
                    if ui.add(clean_btn).clicked() {
                        state.refresh_cleaner();
                        state.current_tab = NavTab::Cleaner;
                    }
                });
            });
        });
    });

    ui.add_space(10.0);

    // 3. Bottom Bento Card: System Health & Actions
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("✚").size(16.0).color(colors.secondary).strong());
            ui.label(RichText::new("System Health & Actions").size(15.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if state.health_report.defender_active && state.health_report.firewall_active {
                    status_badge(ui, "0 Critical Alerts • Protected", (78, 222, 163));
                } else {
                    status_badge(ui, "1 Alert • Attention Required", (255, 185, 95));
                }
            });
        });

        ui.add_space(8.0);

        // Security Status Row
        Frame::none()
            .fill(if state.health_report.defender_active { Color32::from_rgba_unmultiplied(78, 222, 163, 15) } else { Color32::from_rgba_unmultiplied(239, 68, 68, 20) })
            .stroke(Stroke::new(1.0_f32, if state.health_report.defender_active { Color32::from_rgba_unmultiplied(78, 222, 163, 60) } else { Color32::from_rgb(239, 68, 68) }))
            .rounding(Rounding::same(6.0))
            .inner_margin(Margin::symmetric(12.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if state.health_report.defender_active {
                        ui.label(RichText::new("🛡").size(16.0).color(colors.secondary));
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Antivirus & Real-Time Protection Active").size(13.0).strong().color(colors.secondary));
                            ui.label(RichText::new("Windows Defender real-time monitoring and firewall filters are operating normally.").size(11.0).color(colors.text_secondary));
                        });
                    } else {
                        ui.label(RichText::new("⚠️").size(16.0).color(colors.danger));
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Antivirus Protection Disabled").size(13.0).strong().color(colors.danger));
                            ui.label(RichText::new("Real-time protection is offline. System may be vulnerable to unauthorized execution.").size(11.0).color(colors.text_secondary));
                        });
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Health Diagnostics →").clicked() {
                            state.current_tab = NavTab::Health;
                        }
                    });
                });
            });
    });
}
