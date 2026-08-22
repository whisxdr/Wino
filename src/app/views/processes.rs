use crate::app::components::{card_container, search_bar, status_badge, view_header};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, Button, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    // 1. Header with Refresh Button
    view_header(
        ui,
        "Process Manager",
        "Inspect active processes, memory working sets, CPU usage, and signature trust",
        |ui| {
            let refresh_btn = Button::new("↻ Refresh Processes")
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(refresh_btn).clicked() {
                state.refresh_processes();
            }
        },
    );

    // 2. Modern Unified Search Bar
    let count_str = format!("{} active tasks", state.processes.len());
    search_bar(ui, &mut state.search_query, "Filter processes by name or PID...", Some(&count_str));

    // 3. Process Table Container (Spans 100% Full Width)
    card_container(ui, |ui| {
        let total_w = ui.available_width();
        let name_col_w = (total_w - 460.0).max(180.0);

        egui::Grid::new("process_grid")
            .striped(true)
            .spacing([12.0, 8.0])
            .min_col_width(60.0)
            .show(ui, |ui| {
                // Table Header Row
                ui.allocate_ui_with_layout(Vec2::new(name_col_w, 20.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Process Name").strong().color(colors.text_primary));
                });
                ui.allocate_ui_with_layout(Vec2::new(70.0, 20.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("PID").strong().color(colors.text_primary));
                });
                ui.allocate_ui_with_layout(Vec2::new(110.0, 20.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Working Set").strong().color(colors.text_primary));
                });
                ui.allocate_ui_with_layout(Vec2::new(130.0, 20.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Security Status").strong().color(colors.text_primary));
                });
                ui.allocate_ui_with_layout(Vec2::new(90.0, 20.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Action").strong().color(colors.text_primary));
                });
                ui.end_row();

                let query = state.search_query.to_lowercase();
                let mut kill_target_pid: Option<u32> = None;

                for p in &state.processes {
                    if !query.is_empty() && !p.name.to_lowercase().contains(&query) && !p.pid.to_string().contains(&query) {
                        continue;
                    }

                    // Process Name (Truncated cleanly if super long)
                    ui.allocate_ui_with_layout(Vec2::new(name_col_w, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.label(RichText::new(&p.name).strong());
                    });

                    // PID
                    ui.allocate_ui_with_layout(Vec2::new(70.0, 22.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{}", p.pid)).color(colors.text_muted));
                    });

                    // Memory Working Set
                    ui.allocate_ui_with_layout(Vec2::new(110.0, 22.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{:.1} MB", p.memory_working_set_bytes as f64 / (1024.0 * 1024.0))).strong().color(colors.accent));
                    });

                    // Security Badge
                    ui.allocate_ui_with_layout(Vec2::new(130.0, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        if p.is_system_critical {
                            status_badge(ui, "Critical System", (239, 68, 68));
                        } else if p.is_signed {
                            status_badge(ui, "Signed Trust", (78, 222, 163));
                        } else {
                            status_badge(ui, "User App", (138, 145, 160));
                        }
                    });

                    // Action Button
                    ui.allocate_ui_with_layout(Vec2::new(90.0, 22.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if p.is_system_critical {
                            ui.label(RichText::new("Protected").color(colors.text_muted));
                        } else {
                            let end_btn = Button::new(RichText::new("End Task").color(colors.danger))
                                .fill(colors.bg_card_hover)
                                .rounding(Rounding::same(5.0))
                                .min_size(Vec2::new(75.0, 22.0));
                            if ui.add(end_btn).clicked() {
                                kill_target_pid = Some(p.pid);
                            }
                        }
                    });

                    ui.end_row();
                }

                if let Some(pid) = kill_target_pid {
                    let _ = crate::monitoring::process::kill_process(pid);
                    state.set_toast(&format!("Terminated process PID {}", pid));
                    state.refresh_processes();
                }
            });
    });
}

