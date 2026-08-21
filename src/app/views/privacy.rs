use crate::app::components::{card_container, section_header, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, Button, Color32, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.horizontal(|ui| {
        section_header(ui, "Privacy Center", "Configure telemetry, diagnostic data collection, and advertising tracking");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Refresh Privacy Status").clicked() {
                state.refresh_privacy();
            }
        });
    });

    let mut toggle_rule_id: Option<(String, bool)> = None;

    for item in &state.privacy_items {
        card_container(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&item.rule.name).size(14.0).strong());
                        status_badge(ui, &item.rule.category, (100, 116, 139));
                        if item.is_applied {
                            status_badge(ui, "Protected", (34, 197, 94));
                        } else {
                            status_badge(ui, "Default / Tracking", (234, 179, 8));
                        }
                    });
                    ui.add_space(2.0);
                    ui.label(RichText::new(&item.rule.description).size(12.0).color(colors.text_secondary));
                    ui.add_space(2.0);
                    ui.label(RichText::new(format!("Impact: {}", item.rule.impact)).size(11.0).color(colors.text_muted));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if item.is_applied {
                        let revert_btn = Button::new(RichText::new("Revert").color(colors.text_muted))
                            .fill(colors.bg_card_hover)
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(75.0, 26.0));
                        if ui.add(revert_btn).clicked() {
                            toggle_rule_id = Some((item.rule.id.clone(), false));
                        }
                    } else {
                        let enable_btn = Button::new(RichText::new("Protect").color(Color32::WHITE))
                            .fill(colors.accent)
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(75.0, 26.0));
                        if ui.add(enable_btn).clicked() {
                            toggle_rule_id = Some((item.rule.id.clone(), true));
                        }
                    }
                });
            });
        });
        ui.add_space(4.0);
    }

    if let Some((id, enable)) = toggle_rule_id {
        if let Some(item) = state.privacy_items.iter().find(|i| i.rule.id == id) {
            let res = crate::privacy::scanner::apply_privacy_rule(&item.rule, enable, false);
            state.set_toast(&format!("Privacy policy updated: {} ({} steps)", item.rule.name, res.len()));
            state.refresh_privacy();
        }
    }
}
