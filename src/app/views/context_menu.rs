use crate::app::components::{action_card, card_container, search_bar, status_badge, view_header};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::context_menu::manager::toggle_handler;
use crate::core::i18n::tr;
use eframe::egui::{Button, Color32, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    // 1. Contextual Header with Rescan Button
    view_header(
        ui,
        tr(lang, "ctx.title"),
        tr(lang, "ctx.subtitle"),
        |ui| {
            let rescan_btn = Button::new(format!("↻ {}", tr(lang, "ctx.rescan_handlers")))
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(rescan_btn).clicked() {
                state.refresh_context_menu();
            }
        },
    );

    // 2. Search Bar at Top
    let count_str = format!("{} handlers", state.context_menu_items.len());
    search_bar(ui, &mut state.search_query, "Filter context handlers by name, DLL, or CLSID...", Some(&count_str));

    // 3. Hint banner
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("ℹ").size(15.0).color(colors.secondary));
            ui.vertical(|ui| {
                ui.label(RichText::new(tr(lang, "ctx.scan_hint")).size(12.0).color(colors.text_secondary));
                ui.label(RichText::new(tr(lang, "ctx.backup_note")).size(11.0).color(colors.text_muted));
            });
        });
    });

    ui.add_space(8.0);

    if state.context_menu_items.is_empty() && !state.pending_scans.contains(&crate::app::worker::ScanKind::ContextMenu) {
        card_container(ui, |ui| {
            ui.label(RichText::new(tr(lang, "ctx.none_found")).size(12.5).color(colors.text_muted));
        });
        return;
    }

    let search = state.search_query.to_lowercase();
    let mut toggle_request: Option<(usize, bool)> = None;

    // 4. Handler Items (Non-overlapping Action Cards)
    for (idx, entry) in state.context_menu_items.iter().enumerate() {
        if !search.is_empty()
            && !entry.friendly_name.to_lowercase().contains(&search)
            && !entry.handler_name.to_lowercase().contains(&search)
            && !entry.clsid.to_lowercase().contains(&search)
        {
            continue;
        }

        let store_label = match entry.store {
            crate::context_menu::scanner::HandlerStore::Machine => tr(lang, "ctx.machine_store"),
            crate::context_menu::scanner::HandlerStore::User => tr(lang, "ctx.user_store"),
        };

        action_card(
            ui,
            100.0,
            |ui| {
                ui.label(RichText::new(&entry.friendly_name).size(13.5).strong());
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    status_badge(ui, &format!("{} {}", tr(lang, "ctx.root_badge"), entry.shell_root), (100, 116, 139));
                    status_badge(ui, store_label, (60, 144, 255));
                    if entry.is_enabled {
                        status_badge(ui, tr(lang, "ctx.active_badge"), (34, 197, 94));
                    } else {
                        status_badge(ui, tr(lang, "ctx.disabled_badge"), (148, 163, 184));
                    }
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new(if entry.dll_path.is_empty() {
                        format!("CLSID {}", entry.clsid)
                    } else {
                        format!("{} — CLSID {}", entry.dll_path, entry.clsid)
                    })
                    .size(10.5)
                    .color(colors.text_muted),
                );
            },
            |ui| {
                if entry.is_enabled {
                    let btn = Button::new(RichText::new(tr(lang, "ctx.disable_btn")).color(Color32::WHITE))
                        .fill(Color32::from_rgb(239, 68, 68))
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(90.0, 26.0));
                    if ui.add(btn).clicked() {
                        toggle_request = Some((idx, false));
                    }
                } else {
                    let btn = Button::new(RichText::new(tr(lang, "ctx.enable_btn")).color(Color32::WHITE))
                        .fill(colors.accent)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(90.0, 26.0));
                    if ui.add(btn).clicked() {
                        toggle_request = Some((idx, true));
                    }
                }
            },
        );
        ui.add_space(4.0);
    }

    if let Some((idx, enable)) = toggle_request {
        if let Some(entry) = state.context_menu_items.get(idx).cloned() {
            match toggle_handler(&entry, enable, false) {
                Ok(()) => {
                    state.set_toast(&format!(
                        "{} '{}' {}.",
                        if enable { "Enabled" } else { "Disabled" },
                        entry.friendly_name,
                        if enable { "(restored)" } else { "(snapshot saved)" }
                    ));
                    state.refresh_context_menu();
                }
                Err(e) => state.record_event("Context Menu", &e, false),
            }
        }
    }
}

