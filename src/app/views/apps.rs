//! Application Manager view.
//!
//! WHY the view is shaped this way:
//!
//! * Uninstall routes through [`AppState::request_confirm`] rather than calling
//!   [`crate::apps::uninstall::uninstall_app`] directly. Removal is the one
//!   irreversible action in this view, so the shared dialog discloses what runs
//!   and that a snapshot is taken first.
//! * A package with no removal path renders the words "Uninstall unavailable"
//!   instead of a disabled button. A dead button implies the action exists and is
//!   merely blocked, which is a different claim from "this application exposes no
//!   uninstaller".
//! * Winget is presented as an optional provider. When it is absent the view says
//!   so and the rest of Wino keeps working, because the native scan never depends
//!   on it.
//! * Selection for batch updates is keyed by Winget package id, not by row index,
//!   so re-sorting or re-scanning the list cannot silently change what is ticked.

use crate::app::components::{action_card, card_container, search_bar, status_badge, view_header};
use crate::app::state::{AppState, PendingConfirm};
use crate::app::theme::get_colors;
use crate::apps::manager::{filter_apps, open_install_location, sort_apps};
use crate::apps::models::{normalize_install_date, AppSort, AppSource};
use crate::core::i18n::tr;
use eframe::egui::{Button, Color32, RichText, Rounding, Stroke, Ui, Vec2};

/// Badge colour per source, so a row's origin is readable at a glance.
fn source_rgb(source: AppSource) -> (u8, u8, u8) {
    match source {
        AppSource::Win32 => (60, 144, 255),
        AppSource::MicrosoftStore => (78, 222, 163),
        AppSource::AppX => (167, 139, 250),
        AppSource::Winget => (255, 185, 95),
    }
}

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    // 1. Contextual header with rescan and update check
    view_header(
        ui,
        tr(lang, "apps.title"),
        tr(lang, "apps.subtitle"),
        |ui| {
            let updates_btn = Button::new(format!("↻ {}", tr(lang, "apps.check_updates")))
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(updates_btn).clicked() {
                state.refresh_app_updates();
            }

            ui.add_space(6.0);

            let rescan_btn = Button::new(format!("↻ {}", tr(lang, "apps.rescan")))
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(rescan_btn).clicked() {
                state.refresh_apps();
            }
        },
    );

    // 2. Winget availability. Informational, never an error: Wino is fully
    //    functional without it and only update metadata needs it.
    if !state.winget_status.available {
        card_container(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("📦").size(15.0).color(colors.text_muted));
                ui.label(
                    RichText::new(tr(lang, "apps.winget_missing"))
                        .size(14.0)
                        .strong(),
                );
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new(tr(lang, "apps.installing_winget_note"))
                    .size(11.5)
                    .color(colors.text_secondary),
            );
            if !state.winget_status.detail.is_empty() {
                ui.add_space(2.0);
                ui.label(
                    RichText::new(&state.winget_status.detail)
                        .size(11.0)
                        .color(colors.text_muted),
                );
            }
        });
        ui.add_space(10.0);
    }

    // 3. Source filter chips
    card_container(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(tr(lang, "apps.source_filter"))
                    .size(12.5)
                    .strong(),
            );
            ui.add_space(4.0);

            let all_selected = state.apps_source_filter.is_none();
            let all_btn = Button::new(RichText::new(tr(lang, "apps.source_all")).color(
                if all_selected {
                    Color32::WHITE
                } else {
                    colors.text_secondary
                },
            ))
            .fill(if all_selected {
                colors.accent
            } else {
                colors.bg_card_hover
            })
            .stroke(Stroke::new(
                1.0_f32,
                if all_selected {
                    colors.accent
                } else {
                    colors.border
                },
            ))
            .rounding(Rounding::same(6.0))
            .min_size(Vec2::new(74.0, 24.0));
            if ui.add(all_btn).clicked() {
                state.apps_source_filter = None;
            }

            for source in AppSource::ALL {
                let is_selected = state.apps_source_filter == Some(source);
                let btn = Button::new(RichText::new(tr(lang, source.i18n_key())).color(
                    if is_selected {
                        Color32::WHITE
                    } else {
                        colors.text_secondary
                    },
                ))
                .fill(if is_selected {
                    colors.accent
                } else {
                    colors.bg_card_hover
                })
                .stroke(Stroke::new(
                    1.0_f32,
                    if is_selected {
                        colors.accent
                    } else {
                        colors.border
                    },
                ))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(96.0, 24.0));
                if ui.add(btn).clicked() {
                    state.apps_source_filter = Some(source);
                }
            }
        });

        ui.add_space(8.0);

        // 4. Sort selector
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr(lang, "apps.sort_by")).size(12.5).strong());
            ui.add_space(4.0);
            for sort in AppSort::ALL {
                let is_selected = state.apps_sort == sort;
                let btn = Button::new(RichText::new(tr(lang, sort.i18n_key())).color(
                    if is_selected {
                        Color32::WHITE
                    } else {
                        colors.text_secondary
                    },
                ))
                .fill(if is_selected {
                    colors.accent
                } else {
                    colors.bg_card_hover
                })
                .stroke(Stroke::new(
                    1.0_f32,
                    if is_selected {
                        colors.accent
                    } else {
                        colors.border
                    },
                ))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(88.0, 24.0));
                if ui.add(btn).clicked() {
                    state.apps_sort = sort;
                }
            }
        });
    });

    ui.add_space(10.0);

    // 5. Search
    let count_str = format!("{} {}", state.apps.len(), tr(lang, "apps.count_fmt"));
    search_bar(
        ui,
        &mut state.apps_search,
        tr(lang, "apps.search_placeholder"),
        Some(&count_str),
    );

    // 6. Application list. Actions are collected during the loop and applied
    //    after it, so `state` is never borrowed twice.
    let query_lower = state.apps_search.to_lowercase();
    let mut visible: Vec<_> = filter_apps(&state.apps, &query_lower, state.apps_source_filter)
        .into_iter()
        .cloned()
        .collect();
    sort_apps(&mut visible, state.apps_sort);

    let mut open_location: Option<String> = None;
    let mut stage_uninstall: Option<Box<crate::apps::models::AppRecord>> = None;

    if visible.is_empty() {
        card_container(ui, |ui| {
            ui.label(
                RichText::new(tr(lang, "apps.no_match"))
                    .size(12.0)
                    .color(colors.text_muted),
            );
        });
    }

    for record in &visible {
        let publisher = if record.publisher.trim().is_empty() {
            tr(lang, "apps.unknown_publisher").to_string()
        } else {
            record.publisher.clone()
        };
        let size_label = {
            let s = record.size_label();
            if s.is_empty() {
                tr(lang, "apps.unknown_size").to_string()
            } else {
                s
            }
        };
        let date_label = {
            let d = normalize_install_date(&record.install_date);
            if d.is_empty() {
                tr(lang, "apps.unknown_date").to_string()
            } else {
                d
            }
        };
        let location_label = if record.install_location.trim().is_empty() {
            tr(lang, "apps.no_location").to_string()
        } else {
            record.install_location.clone()
        };

        action_card(
            ui,
            190.0,
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&record.display_name).size(13.5).strong());
                    status_badge(ui, record.source.label(), source_rgb(record.source));
                    if !record.package_type.is_empty() {
                        status_badge(ui, &record.package_type, (100, 116, 139));
                    }
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new(publisher)
                        .size(11.5)
                        .color(colors.text_secondary),
                );
                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("{} {}", tr(lang, "common.version"), record.version))
                            .size(11.0)
                            .color(colors.text_muted),
                    );
                    ui.label(RichText::new("•").size(10.0).color(colors.text_muted));
                    ui.label(
                        RichText::new(format!("{} {}", tr(lang, "apps.install_size"), size_label))
                            .size(11.0)
                            .color(colors.text_muted),
                    );
                    ui.label(RichText::new("•").size(10.0).color(colors.text_muted));
                    ui.label(
                        RichText::new(format!("{} {}", tr(lang, "apps.install_date"), date_label))
                            .size(11.0)
                            .color(colors.text_muted),
                    );
                });
                ui.add_space(1.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(location_label)
                            .size(10.5)
                            .color(colors.text_muted),
                    )
                    .truncate(),
                );
            },
            |ui| {
                let can_open = !record.install_location.trim().is_empty();
                let open_btn = Button::new(tr(lang, "common.open_install_location"))
                    .fill(colors.bg_card_hover)
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(160.0, 26.0));
                if ui.add_enabled(can_open, open_btn).clicked() {
                    open_location = Some(record.name.clone());
                }

                ui.add_space(3.0);

                if record.is_uninstallable() {
                    let uninstall_btn =
                        Button::new(RichText::new(tr(lang, "apps.uninstall")).color(colors.danger))
                            .fill(colors.bg_card_hover)
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(160.0, 26.0));
                    if ui.add(uninstall_btn).clicked() {
                        stage_uninstall = Some(Box::new(record.clone()));
                    }
                } else {
                    // No removal path: say so plainly rather than showing a
                    // button that could never work.
                    ui.allocate_ui_with_layout(
                        Vec2::new(160.0, 26.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.label(
                                RichText::new(tr(lang, "apps.uninstall_unavailable"))
                                    .size(11.0)
                                    .color(colors.text_muted),
                            );
                        },
                    );
                }
            },
        );
        ui.add_space(4.0);
    }

    // Apply collected actions after the loop.
    if let Some(name) = open_location {
        if let Some(record) = state.apps.iter().find(|a| a.name == name).cloned() {
            match open_install_location(&record) {
                Ok(()) => {}
                Err(e) => state.record_event("Apps", &e, false),
            }
        }
    }
    if let Some(record) = stage_uninstall {
        state.request_confirm(PendingConfirm::UninstallApp(record));
    }

    ui.add_space(10.0);

    // 7. Updates section
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("⬆").size(15.0).color(colors.accent));
            ui.label(
                RichText::new(tr(lang, "apps.updates_title"))
                    .size(15.0)
                    .strong(),
            );
            if state.winget_status.available {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    status_badge(ui, tr(lang, "apps.winget_available"), (34, 197, 94));
                });
            }
        });
        ui.add_space(2.0);
        ui.label(
            RichText::new(tr(lang, "apps.updates_subtitle"))
                .size(11.5)
                .color(colors.text_muted),
        );
        ui.add_space(8.0);

        if state.app_updates.is_empty() {
            ui.label(
                RichText::new(tr(lang, "apps.update_none"))
                    .size(12.0)
                    .color(colors.text_muted),
            );
            return;
        }

        let mut toggle_selection: Option<(String, bool)> = None;

        for update in &state.app_updates {
            ui.horizontal(|ui| {
                if update.is_actionable() {
                    let mut ticked = state.apps_selected.contains(&update.package_id);
                    if ui.checkbox(&mut ticked, "").changed() {
                        toggle_selection = Some((update.package_id.clone(), ticked));
                    }
                } else {
                    ui.add_space(18.0);
                }

                ui.label(RichText::new(&update.name).size(12.5).strong());
                status_badge(
                    ui,
                    tr(lang, update.state.i18n_key()),
                    update.state.color_rgb(),
                );

                if !update.current_version.is_empty() || !update.available_version.is_empty() {
                    ui.label(
                        RichText::new(format!(
                            "{}  →  {}",
                            update.current_version, update.available_version
                        ))
                        .size(11.0)
                        .color(colors.text_secondary),
                    );
                }
                ui.label(
                    RichText::new(&update.package_id)
                        .size(10.5)
                        .color(colors.text_muted),
                );
            });
            ui.add_space(2.0);
        }

        if let Some((id, ticked)) = toggle_selection {
            if ticked {
                state.apps_selected.insert(id);
            } else {
                state.apps_selected.remove(&id);
            }
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let selected_ids: Vec<String> = state
                .app_updates
                .iter()
                .filter(|u| u.is_actionable() && state.apps_selected.contains(&u.package_id))
                .map(|u| u.package_id.clone())
                .collect();

            let selected_label =
                format!("{} {}", selected_ids.len(), tr(lang, "apps.selected_count"));

            let update_selected = Button::new(
                RichText::new(tr(lang, "apps.update_selected"))
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(colors.accent)
            .rounding(Rounding::same(6.0))
            .min_size(Vec2::new(150.0, 30.0));
            if ui
                .add_enabled(!selected_ids.is_empty(), update_selected)
                .clicked()
            {
                let label = selected_label.clone();
                state.request_confirm(PendingConfirm::UpdateWinget {
                    package_ids: selected_ids,
                    label,
                });
            }

            ui.add_space(6.0);

            let all_ids: Vec<String> = state
                .app_updates
                .iter()
                .filter(|u| u.is_actionable())
                .map(|u| u.package_id.clone())
                .collect();

            let update_all = Button::new(tr(lang, "apps.update_all"))
                .fill(colors.bg_card_hover)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(120.0, 30.0));
            if ui.add_enabled(!all_ids.is_empty(), update_all).clicked() {
                let label = format!("{} {}", all_ids.len(), tr(lang, "apps.count_fmt"));
                state.request_confirm(PendingConfirm::UpdateWinget {
                    package_ids: all_ids,
                    label,
                });
            }

            ui.add_space(6.0);
            ui.label(
                RichText::new(selected_label)
                    .size(11.0)
                    .color(colors.text_muted),
            );
        });
    });
}
