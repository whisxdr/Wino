use crate::app::components::{action_card, search_bar, status_badge, view_header};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{Button, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    // 1. Contextual Header with Refresh Button
    view_header(
        ui,
        "Startup Applications",
        "Manage apps that start with Windows to improve boot time and reduce idle load",
        |ui| {
            let refresh_btn = Button::new("↻ Refresh List")
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(refresh_btn).clicked() {
                state.refresh_startup();
            }
        },
    );

    // 2. Search Bar
    let count_str = format!("{} apps", state.startup_items.len());
    search_bar(ui, &mut state.search_query, "Filter startup apps by name or command...", Some(&count_str));

    let mut toggle_target_idx: Option<usize> = None;
    let search = state.search_query.to_lowercase();

    for (idx, item) in state.startup_items.iter().enumerate() {
        if !search.is_empty() && !item.name.to_lowercase().contains(&search) && !item.command.to_lowercase().contains(&search) {
            continue;
        }

        let impact_rgb = match item.impact.as_str() {
            "High" => (239, 68, 68),
            "Medium" => (234, 179, 8),
            _ => (34, 197, 94),
        };

        action_card(
            ui,
            90.0,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&item.name).size(14.0).strong());
                    status_badge(ui, &format!("{} Impact", item.impact), impact_rgb);
                    status_badge(ui, &item.source, (100, 116, 139));
                });
                ui.add_space(2.0);
                ui.label(RichText::new(&item.command).size(11.0).color(colors.text_muted));
            },
            |ui| {
                let btn = Button::new(RichText::new("Disable").color(colors.danger))
                    .fill(colors.bg_card_hover)
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(80.0, 26.0));

                if ui.add(btn).clicked() {
                    toggle_target_idx = Some(idx);
                }
            },
        );
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

