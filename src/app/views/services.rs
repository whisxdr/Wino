use crate::app::components::{action_card, search_bar, status_badge, view_header};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{Button, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    // 1. Contextual Header with Refresh Button
    view_header(
        ui,
        "Windows Services",
        "Review service configurations and safe recommendations",
        |ui| {
            let refresh_btn = Button::new("↻ Refresh Services")
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(refresh_btn).clicked() {
                state.refresh_services();
            }
        },
    );

    // 2. Search Bar
    let count_str = format!("{} services", state.services.len());
    search_bar(ui, &mut state.search_query, "Filter services by name or description...", Some(&count_str));

    let mut set_manual_svc: Option<String> = None;
    let query = state.search_query.to_lowercase();

    for svc in &state.services {
        if !query.is_empty() && !svc.service_name.to_lowercase().contains(&query) && !svc.display_name.to_lowercase().contains(&query) {
            continue;
        }

        let class_rgb = match svc.classification.as_str() {
            "Safe to change" => (34, 197, 94),
            "Usually safe" => (16, 185, 129),
            "Optional" => (234, 179, 8),
            _ => (239, 68, 68),
        };

        action_card(
            ui,
            110.0,
            |ui| {
                ui.label(RichText::new(&svc.display_name).size(13.5).strong());
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("({})", svc.service_name)).size(10.5).color(colors.text_muted));
                    status_badge(ui, &svc.classification, class_rgb);
                    status_badge(ui, &svc.status, if svc.status == "Running" { (34, 197, 94) } else { (100, 116, 139) });
                });
                ui.add_space(2.0);
                ui.label(RichText::new(&svc.reason).size(11.5).color(colors.text_secondary));
            },
            |ui| {
                if svc.classification == "Safe to change" || svc.classification == "Optional" {
                    let btn = Button::new(RichText::new("Set Manual").color(colors.accent))
                        .fill(colors.bg_card_hover)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(95.0, 26.0));

                    if ui.add(btn).clicked() {
                        set_manual_svc = Some(svc.service_name.clone());
                    }
                } else {
                    ui.label(RichText::new("Protected").color(colors.text_muted));
                }
            },
        );
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

