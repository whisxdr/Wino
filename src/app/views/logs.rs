use crate::app::components::{card_container, section_header, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::core::logger::MemoryLogger;
use eframe::egui::{self, RichText, Ui};

pub fn render(ui: &mut Ui, _state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.horizontal(|ui| {
        section_header(ui, "Audit & Event Logs", "Complete chronological record of all system operations and scans");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Clear Logs").clicked() {
                if let Some(logger) = MemoryLogger::global() {
                    logger.clear();
                }
            }
        });
    });

    let entries = if let Some(logger) = MemoryLogger::global() {
        logger.get_entries()
    } else {
        Vec::new()
    };

    card_container(ui, |ui| {
        if entries.is_empty() {
            ui.label(RichText::new("No log events recorded yet.").color(colors.text_muted));
        }

        for entry in entries.iter().rev() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&entry.timestamp).size(10.5).color(colors.text_muted));

                let lvl_rgb = match entry.level.as_str() {
                    "ERROR" => (239, 68, 68),
                    "WARN" => (234, 179, 8),
                    _ => (34, 197, 94),
                };
                status_badge(ui, &entry.level, lvl_rgb);
                ui.label(RichText::new(format!("[{}]", entry.target)).size(11.0).color(colors.accent));
                ui.label(RichText::new(&entry.message).size(12.0));
            });
            ui.add_space(3.0);
        }
    });
}
