use crate::app::components::{card_container, section_header, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{Button, RichText, Rounding, Ui};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    section_header(ui, "Settings & Preferences", "Configure application behavior, themes, and privileges");

    card_container(ui, |ui| {
        ui.label(RichText::new("Appearance & Theme").size(15.0).strong());
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Theme:");
            if ui.selectable_label(state.config.general.theme == "dark", "Dark").clicked() {
                state.config.general.theme = "dark".to_string();
                let _ = state.config.save();
            }
            if ui.selectable_label(state.config.general.theme == "light", "Light").clicked() {
                state.config.general.theme = "light".to_string();
                let _ = state.config.save();
            }
        });
    });

    ui.add_space(14.0);

    card_container(ui, |ui| {
        ui.label(RichText::new("Execution Privileges").size(15.0).strong());
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Current Status:");
            if state.sys_info.is_admin {
                status_badge(ui, "Administrator (Elevated)", (34, 197, 94));
            } else {
                status_badge(ui, "Standard User (Non-Elevated)", (234, 179, 8));
            }
        });

        if !state.sys_info.is_admin {
            ui.add_space(8.0);
            ui.label(RichText::new("Some deep registry modifications and system package removals require administrator privileges.").size(12.0).color(colors.text_muted));
            ui.add_space(6.0);
            let elevate_btn = Button::new("Restart as Administrator")
                .fill(colors.accent)
                .rounding(Rounding::same(6.0));

            if ui.add(elevate_btn).clicked() {
                let _ = crate::core::permissions::restart_elevated();
            }
        }
    });

    ui.add_space(14.0);

    card_container(ui, |ui| {
        ui.label(RichText::new("About Wino").size(15.0).strong());
        ui.add_space(4.0);
        ui.label("Wino — Rust-Native Windows Debloater, Optimizer & Memory Suite");
        ui.label(RichText::new("Version: 0.1.0 | Pure Rust Implementation | Safe, Reversible & Zero-Placebo").size(11.0).color(colors.text_muted));
        ui.add_space(4.0);
        ui.label(RichText::new("License: MIT / Apache-2.0").size(11.0).color(colors.text_muted));
    });
}
