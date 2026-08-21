pub mod cleaner;
pub mod dashboard;
pub mod debloat;
pub mod gaming;
pub mod health;
pub mod logs;
pub mod memory;
pub mod privacy;
pub mod processes;
pub mod restore;
pub mod services;
pub mod settings;
pub mod startup;

use crate::app::navigation::NavTab;
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Ui};

pub fn render_active_view(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    // Toast notification bar
    if let Some((msg, _)) = &state.toast_message {
        Frame::none()
            .fill(colors.accent)
            .rounding(Rounding::same(6.0))
            .inner_margin(Margin::symmetric(14.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("[*] {}", msg)).color(Color32::WHITE).strong());
                });
            });
        ui.add_space(8.0);
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            match state.current_tab {
                NavTab::Dashboard => dashboard::render(ui, state),
                NavTab::Memory => memory::render(ui, state),
                NavTab::Processes => processes::render(ui, state),
                NavTab::Debloat => debloat::render(ui, state),
                NavTab::Startup => startup::render(ui, state),
                NavTab::Services => services::render(ui, state),
                NavTab::Privacy => privacy::render(ui, state),
                NavTab::Cleaner => cleaner::render(ui, state),
                NavTab::Gaming => gaming::render(ui, state),
                NavTab::Health => health::render(ui, state),
                NavTab::Restore => restore::render(ui, state),
                NavTab::Logs => logs::render(ui, state),
                NavTab::Settings => settings::render(ui, state),
            }
        });
}
