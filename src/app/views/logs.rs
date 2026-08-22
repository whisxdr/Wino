use crate::app::components::{card_container, status_badge, view_header};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::core::logger::MemoryLogger;
use eframe::egui::{Button, RichText, Rounding, Stroke, Ui};

pub fn render(ui: &mut Ui, _state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    // 1. Contextual Header with Clear Logs Button
    view_header(
        ui,
        "Audit Logs",
        "Complete chronological record of all system operations, optimizations, and security scans",
        |ui| {
            let clear_btn = Button::new("🗑 Clear Logs")
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(clear_btn).clicked() {
                if let Some(logger) = MemoryLogger::global() {
                    logger.clear();
                }
            }
        },
    );

    let entries = if let Some(logger) = MemoryLogger::global() {
        logger.get_entries()
    } else {
        Vec::new()
    };

    // 2. Log Container (100% Full Width)
    card_container(ui, |ui| {
        if entries.is_empty() {
            ui.label(RichText::new("No log events recorded yet.").color(colors.text_muted));
        } else {
            for entry in entries.iter().rev() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&entry.timestamp).size(10.5).color(colors.text_muted));

                    let lvl_rgb = match entry.level.as_str() {
                        "ERROR" => (239, 68, 68),
                        "WARN" => (255, 185, 95),
                        _ => (78, 222, 163),
                    };
                    status_badge(ui, &entry.level, lvl_rgb);
                    ui.label(RichText::new(format!("[{}]", entry.target)).size(11.0).color(colors.accent));
                    ui.label(RichText::new(&entry.message).size(12.0).color(colors.text_primary));
                });
                ui.add_space(4.0);
            }
        }
    });
}

