use crate::app::components::{card_container, section_header, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, Button, Color32, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    let total_bytes: u64 = state.cleaner_items.iter().map(|i| i.total_bytes).sum();
    let total_files: usize = state.cleaner_items.iter().map(|i| i.file_count).sum();

    ui.horizontal(|ui| {
        section_header(ui, "Storage Cleaner", "Safe removal of obsolete temporary files, caches, and crash logs");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Scan Storage").clicked() {
                state.refresh_cleaner();
            }
        });
    });

    // Summary Card & Action
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Recoverable Disk Space").size(13.0).color(colors.text_muted));
                ui.label(RichText::new(format!("{:.2} GB", total_bytes as f64 / (1024.0 * 1024.0 * 1024.0))).size(22.0).strong().color(colors.accent));
                ui.label(RichText::new(format!("{} total files ready for safe cleanup", total_files)).size(11.0).color(colors.text_muted));
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let clean_btn = Button::new(RichText::new("Clean Now").strong().color(Color32::WHITE))
                    .fill(colors.accent)
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(120.0, 32.0));

                if ui.add(clean_btn).clicked() {
                    let rep = crate::cleaner::cleaner::execute_cleanup(&state.cleaner_items, false);
                    state.set_toast(&rep.message);
                    state.refresh_cleaner();
                }
            });
        });
    });

    ui.add_space(10.0);

    for item in &state.cleaner_items {
        card_container(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&item.rule.name).size(14.0).strong());
                        status_badge(ui, &item.rule.category, (100, 116, 139));
                        status_badge(ui, &format!("{:.1} MB", item.total_bytes as f64 / (1024.0 * 1024.0)), (34, 197, 94));
                    });
                    ui.add_space(2.0);
                    ui.label(RichText::new(&item.rule.description).size(12.0).color(colors.text_secondary));
                    ui.add_space(2.0);
                    ui.label(RichText::new(format!("Target: {} ({} files)", item.resolved_path.display(), item.file_count)).size(10.5).color(colors.text_muted));
                });
            });
        });
        ui.add_space(4.0);
    }
}
