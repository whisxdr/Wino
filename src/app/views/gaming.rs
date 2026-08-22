use crate::app::components::{action_card, card_container, section_header, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{Button, Color32, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let game_mode_on = crate::gaming::optimizer::is_windows_game_mode_enabled();

    section_header(ui, "Gaming Profile", "Optimize background latency and prioritize game thread execution");

    // 1. Windows Game Mode Optimization Card (100% Full Width)
    action_card(
        ui,
        180.0,
        |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Windows Game Mode").size(15.5).strong());
                if game_mode_on {
                    status_badge(ui, "Active", (34, 197, 94));
                } else {
                    status_badge(ui, "Inactive", (234, 179, 8));
                }
            });
            ui.add_space(3.0);
            ui.label(RichText::new("Directs GPU and CPU scheduling priority to fullscreen game processes and minimizes background interruptions.").size(12.0).color(colors.text_secondary));
        },
        |ui| {
            let btn = Button::new(RichText::new("⚡ Optimize for Gaming").strong().color(Color32::WHITE))
                .fill(colors.accent)
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(170.0, 34.0));

            if ui.add(btn).clicked() {
                let rep = crate::gaming::optimizer::enable_gaming_profile(false);
                state.set_toast(&rep.message);
            }
        },
    );

    ui.add_space(10.0);

    // 2. Zero-Placebo Commitment Card (100% Full Width)
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🛡").size(16.0).color(colors.secondary));
            ui.label(RichText::new("Zero-Placebo Commitment").size(14.5).strong());
        });
        ui.add_space(4.0);
        ui.label(RichText::new("Wino does NOT apply fake registry 'FPS tweaks', timer resolution hacks, or network placebo. Only documented Windows scheduling and memory optimizations are applied.").size(12.0).color(colors.text_muted));
    });
}

