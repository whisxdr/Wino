use crate::app::components::{card_container, section_header};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, RichText, Ui};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.horizontal(|ui| {
        section_header(ui, "Process Manager", "Inspect active processes, memory working sets, and signature trust");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Refresh Processes").clicked() {
                state.refresh_processes();
            }
        });
    });

    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.text_edit_singleline(&mut state.search_query);
        ui.label(format!("({} total)", state.processes.len()));
    });

    ui.add_space(8.0);

    card_container(ui, |ui| {
        egui::Grid::new("process_grid")
            .striped(true)
            .spacing([12.0, 6.0])
            .min_col_width(70.0)
            .show(ui, |ui| {
                ui.label(RichText::new("Process Name").strong());
                ui.label(RichText::new("PID").strong());
                ui.label(RichText::new("Working Set (RAM)").strong());
                ui.label(RichText::new("Publisher / Trust").strong());
                ui.label(RichText::new("Actions").strong());
                ui.end_row();

                let query = state.search_query.to_lowercase();
                let mut kill_target_pid: Option<u32> = None;

                for p in &state.processes {
                    if !query.is_empty() && !p.name.to_lowercase().contains(&query) && !p.pid.to_string().contains(&query) {
                        continue;
                    }

                    ui.label(&p.name);
                    ui.label(format!("{}", p.pid));
                    ui.label(format!("{:.1} MB", p.memory_working_set_bytes as f64 / (1024.0 * 1024.0)));

                    if p.is_system_critical {
                        ui.label(RichText::new("System Critical").color(colors.warning).strong());
                    } else if p.is_signed {
                        ui.label(RichText::new("Verified Signed").color(colors.success));
                    } else {
                        ui.label(RichText::new("Unverified").color(colors.text_muted));
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
}
