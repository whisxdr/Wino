use crate::app::components::{card_container, section_header, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, Button, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.horizontal(|ui| {
        section_header(ui, "Windows Services", "Review service configurations and safe recommendations");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Refresh Services").clicked() {
                state.refresh_services();
            }
        });
    });

    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.text_edit_singleline(&mut state.search_query);
        ui.label(format!("({} total)", state.services.len()));
    });

    ui.add_space(8.0);

    let mut set_manual_svc: Option<String> = None;
    let query = state.search_query.to_lowercase();

    for svc in &state.services {
        if !query.is_empty() && !svc.service_name.to_lowercase().contains(&query) && !svc.display_name.to_lowercase().contains(&query) {
            continue;
        }

        card_container(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&svc.display_name).size(13.5).strong());
                        ui.label(RichText::new(format!("({})", svc.service_name)).size(10.5).color(colors.text_muted));

                        let class_rgb = match svc.classification.as_str() {
                            "Safe to change" => (34, 197, 94),
                            "Usually safe" => (16, 185, 129),
                            "Optional" => (234, 179, 8),
                            _ => (239, 68, 68),
                        };
                        status_badge(ui, &svc.classification, class_rgb);
                        status_badge(ui, &svc.status, if svc.status == "Running" { (34, 197, 94) } else { (100, 116, 139) });
                    });
                    ui.add_space(2.0);
                    ui.label(RichText::new(&svc.reason).size(11.5).color(colors.text_secondary));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if svc.classification == "Safe to change" || svc.classification == "Optional" {
                        let btn = Button::new(RichText::new("Set Manual").color(colors.accent))
                            .fill(colors.bg_card_hover)
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(90.0, 26.0));

                        if ui.add(btn).clicked() {
                            set_manual_svc = Some(svc.service_name.clone());
                        }
                    } else {
                        ui.label(RichText::new("Protected").color(colors.text_muted));
                    }
                });
            });
        });
        ui.add_space(4.0);
    }

    if let Some(svc_name) = set_manual_svc {
        let res = crate::services::manager::set_service_startup(&svc_name, crate::services::manager::StartupType::Manual);
        match res {
            Ok(_) => {
                state.set_toast(&format!("Set service '{}' to Manual startup.", svc_name));
                state.refresh_services();
            }
            Err(e) => state.set_toast(&format!("Error: {}", e)),
        }
    }
}
