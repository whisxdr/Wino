use crate::app::components::{action_card, search_bar, status_badge, view_header};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{Button, Color32, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    // 1. Contextual Header with Refresh Button
    view_header(
        ui,
        "Privacy Center",
        "Configure telemetry, diagnostic data collection, and advertising tracking",
        |ui| {
            let refresh_btn = Button::new("↻ Refresh Privacy Status")
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(refresh_btn).clicked() {
                state.refresh_privacy();
            }
        },
    );

    // 2. Search Bar
    let count_str = format!("{} privacy rules", state.privacy_items.len());
    search_bar(
        ui,
        &mut state.search_query,
        "Filter privacy settings...",
        Some(&count_str),
    );

    let mut toggle_rule_id: Option<(String, bool)> = None;
    let search = state.search_query.to_lowercase();

    for item in &state.privacy_items {
        if !search.is_empty()
            && !item.rule.name.to_lowercase().contains(&search)
            && !item.rule.description.to_lowercase().contains(&search)
        {
            continue;
        }

        action_card(
            ui,
            95.0,
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&item.rule.name).size(14.0).strong());
                    status_badge(ui, &item.rule.category, (100, 116, 139));
                    if item.is_applied {
                        status_badge(ui, "Protected", (34, 197, 94));
                    } else {
                        status_badge(ui, "Default / Tracking", (234, 179, 8));
                    }
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new(&item.rule.description)
                        .size(12.0)
                        .color(colors.text_secondary),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new(format!("Impact: {}", item.rule.impact))
                        .size(11.0)
                        .color(colors.text_muted),
                );
            },
            |ui| {
                if item.is_applied {
                    let revert_btn = Button::new(RichText::new("Revert").color(colors.text_muted))
                        .fill(colors.bg_card_hover)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(80.0, 26.0));
                    if ui.add(revert_btn).clicked() {
                        toggle_rule_id = Some((item.rule.id.clone(), false));
                    }
                } else {
                    let enable_btn = Button::new(RichText::new("Protect").color(Color32::WHITE))
                        .fill(colors.accent)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(80.0, 26.0));
                    if ui.add(enable_btn).clicked() {
                        toggle_rule_id = Some((item.rule.id.clone(), true));
                    }
                }
            },
        );
        ui.add_space(4.0);
    }

    if let Some((id, enable)) = toggle_rule_id {
        if let Some(item) = state.privacy_items.iter().find(|i| i.rule.id == id) {
            let res = crate::privacy::scanner::apply_privacy_rule(&item.rule, enable, false);
            state.set_toast(&format!(
                "Privacy policy updated: {} ({} steps)",
                item.rule.name,
                res.len()
            ));
            state.refresh_privacy();
        }
    }
}
