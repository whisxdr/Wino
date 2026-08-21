use crate::app::components::{card_container, section_header, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, Button, RichText, Rounding, Ui};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    section_header(ui, "Windows Health Diagnostics", "Verify system file integrity, malware protection, and Windows Update health");

    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Overall System Health Rating").size(15.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                status_badge(
                    ui,
                    state.health_report.rating.as_str(),
                    state.health_report.rating.color_rgb(),
                );
            });
        });

        ui.add_space(8.0);
        for rec in &state.health_report.recommendations {
            ui.label(format!("  - {}", rec));
        }
    });

    ui.add_space(16.0);

    // Diagnostics Runners Card
    card_container(ui, |ui| {
        ui.label(RichText::new("System File Integrity & Component Repair").size(15.0).strong());
        ui.add_space(4.0);
        ui.label(RichText::new("Execute official Windows System File Checker (SFC) and Deployment Image Servicing and Management (DISM) utilities.").size(12.0).color(colors.text_muted));

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let sfc_btn = Button::new("Run SFC Scan (/scannow)")
                .fill(colors.bg_card_hover)
                .rounding(Rounding::same(6.0));

            if ui.add(sfc_btn).clicked() {
                state.set_toast("Running SFC scan in background...");
                std::thread::spawn(|| {
                    let _ = crate::health::integrity::run_sfc_scan();
                });
            }

            let dism_btn = Button::new("Run DISM Health Check")
                .fill(colors.bg_card_hover)
                .rounding(Rounding::same(6.0));

            if ui.add(dism_btn).clicked() {
                state.set_toast("Running DISM check in background...");
                std::thread::spawn(|| {
                    let _ = crate::health::integrity::run_dism_check();
                });
            }
        });
    });
}
