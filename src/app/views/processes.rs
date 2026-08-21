use crate::app::components::{card_container, section_header, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, RichText, Ui};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.horizontal(|ui| {
        section_header(ui, "Process Manager", "Inspect active processes, memory working sets, CPU usage, and signature trust");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("↻ Refresh Processes").clicked() {
                state.refresh_processes();
            }
        });
    });

    ui.horizontal(|ui| {
        ui.label(RichText::new("Search:").color(colors.text_muted));
        ui.text_edit_singleline(&mut state.search_query);
        ui.label(RichText::new(format!("({} active tasks)", state.processes.len())).size(11.5).color(colors.text_muted));
    });

    ui.add_space(8.0);

    card_container(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height() - 20.0)
            .show(ui, |ui| {
                egui::Grid::new("process_grid")
                    .striped(true)
                    .spacing([14.0, 8.0])
                    .min_col_width(70.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Process Name").strong().color(colors.text_primary));
                        ui.label(RichText::new("PID").strong().color(colors.text_primary));
                        ui.label(RichText::new("Working Set (RAM)").strong().color(colors.text_primary));
                        ui.label(RichText::new("Security Status").strong().color(colors.text_primary));
                        ui.label(RichText::new("Action").strong().color(colors.text_primary));
                        ui.end_row();

                        let query = state.search_query.to_lowercase();
                        let mut kill_target_pid: Option<u32> = None;

                        for p in &state.processes {
                            if !query.is_empty() && !p.name.to_lowercase().contains(&query) && !p.pid.to_string().contains(&query) {
                                continue;
                            }

                            ui.label(RichText::new(&p.name).strong());
                            ui.label(RichText::new(format!("{}", p.pid)).color(colors.text_muted));
                            ui.label(RichText::new(format!("{:.1} MB", p.memory_working_set_bytes as f64 / (1024.0 * 1024.0))).color(colors.accent));

                            if p.is_system_critical {
                                status_badge(ui, "Critical System", (239, 68, 68));
                            } else if p.is_signed {
                                status_badge(ui, "Signed Trust", (78, 222, 163));
                            } else {
                                status_badge(ui, "User App", (138, 145, 160));
                            }

                            if p.is_system_critical {
                                ui.label(RichText::new("Protected").color(colors.text_muted));
                            } else {
                                if ui.button(RichText::new("End Task").color(colors.danger)).clicked() {
                                    kill_target_pid = Some(p.pid);
                                }
                            }

                            ui.end_row();
                        }

                        if let Some(pid) = kill_target_pid {
                            let _ = crate::monitoring::process::kill_process(pid);
                            state.set_toast(&format!("Terminated process PID {}", pid));
                            state.refresh_processes();
                        }
                    });
            });
    });
}
