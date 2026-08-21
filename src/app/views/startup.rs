use crate::app::components::{card_container, section_header, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, Button, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.horizontal(|ui| {
        section_header(ui, "Startup Applications", "Manage apps that start with Windows to improve boot time and reduce idle load");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Refresh List").clicked() {
                state.refresh_startup();
            }
        });
    });

    let mut toggle_target_idx: Option<usize> = None;

    for (idx, item) in state.startup_items.iter().enumerate() {
        card_container(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&item.name).size(14.0).strong());
                        let impact_rgb = match item.impact.as_str() {
                            "High" => (239, 68, 68),
                            "Medium" => (234, 179, 8),
                            _ => (34, 197, 94),
                        };
                        status_badge(ui, &format!("{} Impact", item.impact), impact_rgb);
                        status_badge(ui, &item.source, (100, 116, 139));
                    });
                    ui.add_space(2.0);
                    ui.label(RichText::new(&item.command).size(11.0).color(colors.text_muted));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let btn = Button::new(RichText::new("Disable").color(colors.danger))
                        .fill(colors.bg_card_hover)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(75.0, 26.0));

                    if ui.add(btn).clicked() {
                        toggle_target_idx = Some(idx);
                    }
                });
            });
        });
        ui.add_space(4.0);
    }

    if let Some(idx) = toggle_target_idx {
        if let Some(item) = state.startup_items.get(idx) {
            let res = crate::startup::manager::toggle_startup_item(item, false, false);
            match res {
                Ok(_) => {
                    state.set_toast(&format!("Disabled startup item: {}", item.name));
                    state.refresh_startup();
                }
                Err(e) => state.set_toast(&format!("Error: {}", e)),
            }
        }
    }
}
