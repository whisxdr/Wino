use crate::app::components::{card_container, status_badge, view_header};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::app::worker::Action;
use crate::core::i18n::tr;
use eframe::egui::{Button, Color32, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    let total_bytes: u64 = state.cleaner_items.iter().map(|i| i.total_bytes).sum();
    let total_files: usize = state.cleaner_items.iter().map(|i| i.file_count).sum();

    // 1. Contextual Header with Scan Button
    view_header(
        ui,
        tr(lang, "clean.title"),
        tr(lang, "clean.subtitle"),
        |ui| {
            let scan_btn = Button::new(format!("↻ {}", tr(lang, "clean.scan_storage")))
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(scan_btn).clicked() {
                state.refresh_cleaner();
            }
        },
    );

    // 2. Summary Card & Action (100% Full Width)
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(tr(lang, "dash.recoverable_space")).size(13.0).color(colors.text_muted));
                ui.label(RichText::new(format!("{:.2} GB", total_bytes as f64 / (1024.0 * 1024.0 * 1024.0))).size(24.0).strong().color(colors.accent));
                ui.label(RichText::new(format!("{} {}", total_files, tr(lang, "clean.files_ready"))).size(11.0).color(colors.text_muted));
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if state.action_busy { tr(lang, "common.loading") } else { tr(lang, "clean.clean_now") };
                let clean_btn = Button::new(RichText::new(label).strong().color(Color32::WHITE))
                    .fill(colors.accent)
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(130.0, 34.0));

                if ui.add_enabled(!state.action_busy && !state.cleaner_items.is_empty(), clean_btn).clicked() {
                    let items = state.cleaner_items.clone();
                    state.request_action(Action::CleanupFiles(items));
                }
            });
        });
    });

    ui.add_space(10.0);

    // 3. Cleaner Category Cards (Each spanning 100% Full Width)
    for item in &state.cleaner_items {
        card_container(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&item.rule.name).size(14.0).strong());
                        status_badge(ui, &item.rule.category, (100, 116, 139));
                        status_badge(ui, &format!("{:.1} {}", item.total_bytes as f64 / (1024.0 * 1024.0), tr(lang, "clean.mb_size")), (34, 197, 94));
                    });
                    ui.add_space(2.0);
                    ui.label(RichText::new(&item.rule.description).size(12.0).color(colors.text_secondary));
                    ui.add_space(2.0);
                    ui.label(RichText::new(format!("{} {} ({} files)", tr(lang, "common.target"), item.resolved_path.display(), item.file_count)).size(10.5).color(colors.text_muted));
                });
            });
        });
        ui.add_space(4.0);
    }
}

