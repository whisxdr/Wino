//! Startup Applications view.
//!
//! WHY the view is shaped this way:
//!
//! * Every discovered entry is listed with its real `enabled` state, read from
//!   the `StartupApproved` records rather than assumed. The previous version
//!   reported every entry as enabled, so a startup app the user had already
//!   turned off still appeared active.
//! * Toggling routes through [`AppState::request_confirm`]. Enabling or
//!   disabling a startup entry changes what runs at every sign-in, and a
//!   snapshot of the original value is recorded first.
//! * Sources Wino cannot toggle (Winlogon, scheduled tasks) render as text with
//!   an explanation. They are reported because the user should know they exist,
//!   but Wino does not pretend to offer control it does not have.
//! * The publisher column shows the actual signer name when it could be read,
//!   and "Unverified" only when verification genuinely failed.

use crate::app::components::{action_card, card_container, search_bar, status_badge, view_header};
use crate::app::state::{AppState, PendingConfirm};
use crate::app::theme::get_colors;
use crate::core::i18n::tr;
use crate::startup::scanner::{StartupItem, SOURCE_SCHEDULED_TASK, SOURCE_WINLOGON};
use eframe::egui::{Button, RichText, Rounding, Stroke, Ui, Vec2};

/// Impact colour, matching the process and service views.
fn impact_rgb(impact: &str) -> (u8, u8, u8) {
    match impact {
        "High" => (239, 68, 68),
        "Medium" => (234, 179, 8),
        _ => (34, 197, 94),
    }
}

/// Whether Wino can toggle this entry's source.
fn is_toggleable(item: &StartupItem) -> bool {
    !item.source.contains(SOURCE_WINLOGON) && !item.source.contains(SOURCE_SCHEDULED_TASK)
}

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    view_header(ui, tr(lang, "su.title"), tr(lang, "su.subtitle"), |ui| {
        let refresh_btn = Button::new(format!("↻ {}", tr(lang, "common.rescan")))
            .fill(colors.bg_card)
            .stroke(Stroke::new(1.0_f32, colors.border))
            .rounding(Rounding::same(6.0));
        if ui.add(refresh_btn).clicked() {
            state.refresh_startup();
        }
    });

    // The approval-record note explains where the enabled state comes from.
    card_container(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("ℹ").size(13.0).color(colors.accent));
            ui.label(
                RichText::new(tr(lang, "su.approved_note"))
                    .size(11.0)
                    .color(colors.text_muted),
            );
        });
    });

    ui.add_space(8.0);

    let count_str = format!("{} {}", state.startup_items.len(), tr(lang, "common.items"));
    search_bar(
        ui,
        &mut state.search_query,
        tr(lang, "su.col_name"),
        Some(&count_str),
    );

    let search = state.search_query.to_lowercase();
    let mut stage_toggle: Option<(Box<StartupItem>, bool)> = None;
    let mut inspect: Option<StartupItem> = None;

    if state.startup_items.is_empty() {
        card_container(ui, |ui| {
            ui.label(
                RichText::new(tr(lang, "su.none_found"))
                    .size(12.0)
                    .color(colors.text_muted),
            );
        });
        return;
    }

    for item in &state.startup_items {
        if !search.is_empty()
            && !item.name.to_lowercase().contains(&search)
            && !item.command.to_lowercase().contains(&search)
            && !item.publisher.to_lowercase().contains(&search)
        {
            continue;
        }

        action_card(
            ui,
            150.0,
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&item.name).size(14.0).strong());

                    // Real enabled state, from the approval record.
                    if item.enabled {
                        status_badge(ui, tr(lang, "common.enabled"), (34, 197, 94));
                    } else {
                        status_badge(ui, tr(lang, "su.disabled_badge"), (100, 116, 139));
                    }

                    status_badge(ui, &item.impact, impact_rgb(&item.impact));
                    status_badge(ui, &item.source, (100, 116, 139));
                });

                ui.add_space(2.0);
                ui.label(
                    RichText::new(&item.publisher)
                        .size(11.5)
                        .color(colors.text_secondary),
                );
                ui.add_space(1.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(&item.command)
                            .size(10.5)
                            .color(colors.text_muted),
                    )
                    .truncate(),
                );
            },
            |ui| {
                if is_toggleable(item) {
                    let label = if item.enabled {
                        tr(lang, "common.disable")
                    } else {
                        tr(lang, "common.enable")
                    };
                    let color = if item.enabled {
                        colors.danger
                    } else {
                        colors.success
                    };
                    let btn = Button::new(RichText::new(label).color(color))
                        .fill(colors.bg_card_hover)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(96.0, 26.0));
                    if ui.add(btn).clicked() {
                        stage_toggle = Some((Box::new(item.clone()), !item.enabled));
                    }
                } else {
                    // Not toggleable by design: Winlogon and scheduled tasks are
                    // managed by their own subsystems.
                    ui.label(
                        RichText::new(tr(lang, "common.protected"))
                            .size(10.5)
                            .color(colors.text_muted),
                    );
                }

                ui.add_space(3.0);

                let inspect_btn = Button::new(tr(lang, "common.inspect"))
                    .fill(colors.bg_card_hover)
                    .stroke(Stroke::new(1.0_f32, colors.border))
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(96.0, 24.0));
                if ui.add(inspect_btn).clicked() {
                    inspect = Some(item.clone());
                }
            },
        );
        ui.add_space(4.0);
    }

    // ---- Inspect panel ----
    if let Some(item) = &inspect {
        ui.add_space(6.0);
        card_container(ui, |ui| {
            ui.label(
                RichText::new(tr(lang, "su.inspect_title"))
                    .size(14.0)
                    .strong(),
            );
            ui.add_space(6.0);

            let row = |ui: &mut Ui, label: &str, value: String| {
                if value.is_empty() {
                    return;
                }
                ui.horizontal_wrapped(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(120.0, 16.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.label(RichText::new(label).size(10.5).color(colors.text_muted));
                        },
                    );
                    ui.label(RichText::new(value).size(11.0).color(colors.text_primary));
                });
            };

            row(ui, tr(lang, "su.col_name"), item.name.clone());
            row(ui, tr(lang, "su.col_publisher"), item.publisher.clone());
            row(ui, tr(lang, "su.col_source"), item.source.clone());
            row(
                ui,
                tr(lang, "su.col_signed"),
                if item.is_signed {
                    tr(lang, "common.yes").to_string()
                } else {
                    tr(lang, "common.no").to_string()
                },
            );
            row(ui, tr(lang, "su.col_impact"), item.impact.clone());
            row(ui, tr(lang, "su.col_path"), item.exe_path.clone());

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(tr(lang, "su.col_arguments"))
                        .size(10.5)
                        .color(colors.text_muted),
                );
                let args = item
                    .command
                    .strip_prefix(&item.exe_path)
                    .unwrap_or("")
                    .trim();
                let args = if args.is_empty() {
                    tr(lang, "su.no_arguments")
                } else {
                    args
                };
                ui.add(egui::Label::new(RichText::new(args).size(10.5).monospace()).truncate());
            });
        });
    }

    // ---- Apply collected actions ----
    if let Some((item, enable)) = stage_toggle {
        state.request_confirm(PendingConfirm::ToggleStartup { item, enable });
    }
}
