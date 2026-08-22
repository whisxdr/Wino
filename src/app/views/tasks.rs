use crate::app::components::{action_card, card_container, search_bar, status_badge, view_header};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::core::i18n::tr;
use crate::tasks::manager::set_task_enabled;
use eframe::egui::{Button, Color32, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    // 1. Contextual Header with Rescan Button
    view_header(
        ui,
        tr(lang, "tasks.title"),
        tr(lang, "tasks.subtitle"),
        |ui| {
            let rescan_btn = Button::new(format!("↻ {}", tr(lang, "tasks.rescan")))
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(rescan_btn).clicked() {
                state.refresh_tasks();
            }
        },
    );

    // 2. Search Bar at Top
    let count_str = format!("{} tasks", state.task_items.len());
    search_bar(ui, &mut state.search_query, "Filter scheduled tasks by name or path...", Some(&count_str));

    // 3. Hint banner
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("ℹ").size(15.0).color(colors.secondary));
            ui.vertical(|ui| {
                ui.label(RichText::new(tr(lang, "tasks.hint")).size(12.0).color(colors.text_secondary));
                ui.label(RichText::new(tr(lang, "tasks.snapshot_note")).size(11.0).color(colors.text_muted));
            });
        });
        ui.add_space(2.0);
        ui.label(RichText::new(tr(lang, "tasks.safe_note")).size(11.0).color(colors.text_muted));
    });

    ui.add_space(8.0);

    if state.task_items.is_empty() && !state.pending_scans.contains(&crate::app::worker::ScanKind::Tasks) {
        card_container(ui, |ui| {
            ui.label(RichText::new(tr(lang, "tasks.none_found")).size(12.5).color(colors.text_muted));
        });
        return;
    }

    let search = state.search_query.to_lowercase();
    let mut toggle_request: Option<(usize, bool)> = None;

    // 4. Scheduled Task Items (Action Cards)
    for (idx, item) in state.task_items.iter().enumerate() {
        if !search.is_empty()
            && !item.name.to_lowercase().contains(&search)
            && !item.task_path.to_lowercase().contains(&search)
        {
            continue;
        }

        action_card(
            ui,
            120.0,
            |ui| {
                ui.label(RichText::new(&item.name).size(13.5).strong());
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    status_badge(ui, &item.category_label, (100, 116, 139));
                    if !item.xml_readable {
                        status_badge(ui, tr(lang, "tasks.xml_locked"), (239, 68, 68));
                    }
                    if item.is_enabled {
                        status_badge(ui, tr(lang, "common.enabled"), (34, 197, 94));
                    } else {
                        status_badge(ui, tr(lang, "common.disabled"), (148, 163, 184));
                    }
                });
                ui.add_space(2.0);
                ui.label(RichText::new(&item.description).size(11.5).color(colors.text_secondary));
                ui.label(
                    RichText::new(&item.task_path)
                        .size(10.5)
                        .color(colors.text_muted),
                );
            },
            |ui| {
                if item.is_enabled {
                    let btn = Button::new(RichText::new(tr(lang, "tasks.disable_btn")).color(Color32::WHITE))
                        .fill(Color32::from_rgb(239, 68, 68))
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(110.0, 26.0));
                    if ui.add(btn).clicked() {
                        toggle_request = Some((idx, false));
                    }
                } else {
                    let btn = Button::new(RichText::new(tr(lang, "tasks.enable_btn")).color(Color32::WHITE))
                        .fill(colors.accent)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(110.0, 26.0));
                    if ui.add(btn).clicked() {
                        toggle_request = Some((idx, true));
                    }
                }
            },
        );
        ui.add_space(4.0);
    }

    if let Some((idx, enable)) = toggle_request {
        if let Some(item) = state.task_items.get(idx).cloned() {
            match set_task_enabled(&item.task_path, enable, false) {
                Ok(()) => {
                    state.set_toast(&format!(
                        "{} '{}'.",
                        if enable { "Enabled" } else { "Disabled" },
                        item.name
                    ));
                    state.refresh_tasks();
                }
                Err(e) => state.record_event("Scheduled Tasks", &e, false),
            }
        }
    }
}

