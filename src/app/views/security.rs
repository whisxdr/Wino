//! Security Center view.
//!
//! WHY the view is shaped this way:
//!
//! * Wino reports protection state and offers no way to turn a protection off.
//!   The footnote saying so is rendered unconditionally, because the absence of
//!   a toggle is only reassuring if it is stated rather than left to inference.
//! * A component that is `Off` or `Warning` is lifted out of the list into a
//!   banner at the top and tagged as disabled outside Wino. A disabled protection
//!   is the one thing in this view the user must not have to scroll to find.
//! * `Unknown` is a real result, not a failure: an unreadable key is listed in a
//!   subdued line so the user knows the check ran and could not conclude, instead
//!   of seeing a component silently missing or assumed healthy.

use crate::app::components::{action_card, card_container, status_badge, view_header};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::core::i18n::tr;
use eframe::egui::{self, Button, RichText, Rounding, Stroke, Ui};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    // 1. Contextual header with rescan
    view_header(
        ui,
        tr(lang, "security.title"),
        tr(lang, "security.subtitle"),
        |ui| {
            let rescan_btn = Button::new(format!("↻ {}", tr(lang, "security.rescan")))
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(rescan_btn).clicked() {
                state.refresh_security();
            }
        },
    );

    let total = state.security_report.items.len();
    let on_count = state.security_report.on_count();
    let fully_protected = state.security_report.is_fully_protected();

    // 2. Overall card: how many protections answered "on", plus the verdict badge
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(format!("{} / {}", on_count, total))
                        .size(24.0)
                        .strong()
                        .color(colors.accent),
                );
                // "4 / 8 items" — protections observed on, out of every component
                // that was queried.
                ui.label(
                    RichText::new(tr(lang, "common.items"))
                        .size(11.0)
                        .color(colors.text_muted),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if fully_protected {
                    status_badge(ui, tr(lang, "security.state_on"), (34, 197, 94));
                } else if state.security_report.warnings().is_empty() {
                    status_badge(ui, tr(lang, "security.state_unknown"), (148, 163, 184));
                } else {
                    status_badge(ui, tr(lang, "security.state_warning"), (234, 179, 8));
                }
            });
        });

        // 3. Disabled or degraded protections, stated in full and never collapsed
        for item in state.security_report.warnings() {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("⚠").size(14.0).color(colors.danger));
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(tr(lang, &item.name_key))
                                .size(13.0)
                                .strong()
                                .color(colors.danger),
                        );
                        status_badge(ui, tr(lang, item.state.i18n_key()), item.state.color_rgb());
                        status_badge(ui, tr(lang, "security.disabled_externally"), (239, 68, 68));
                    });
                    ui.label(
                        RichText::new(&item.detail)
                            .size(11.5)
                            .color(colors.text_secondary),
                    );
                });
            });
        }

        // 4. Components that could not be queried, reported as unknown rather
        //    than as protected.
        let unknowns = state.security_report.unknowns();
        if !unknowns.is_empty() {
            ui.add_space(6.0);
            let names: Vec<&str> = unknowns.iter().map(|i| tr(lang, &i.name_key)).collect();
            ui.label(
                RichText::new(format!(
                    "{}: {}",
                    tr(lang, "common.unknown"),
                    names.join(", ")
                ))
                .size(11.0)
                .color(colors.text_muted),
            );
        }
    });

    ui.add_space(10.0);

    // 5. One card per protection component, with the raw observation
    for item in &state.security_report.items {
        action_card(
            ui,
            130.0,
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(tr(lang, &item.name_key)).size(13.5).strong());
                    status_badge(ui, tr(lang, item.state.i18n_key()), item.state.color_rgb());
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new(&item.detail)
                        .size(11.5)
                        .color(colors.text_secondary),
                );

                if !item.extra.is_empty() {
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(format!(
                            "{}: {}",
                            tr(lang, "security.tpm_version"),
                            item.extra
                        ))
                        .size(10.5)
                        .color(colors.text_muted),
                    );
                }
            },
            |ui| {
                // Requirement 5: a component that is not fully on says so beside
                // its own row too, not only in the top banner.
                if item.state.needs_attention() {
                    ui.label(
                        RichText::new(tr(lang, "security.disabled_externally"))
                            .size(11.0)
                            .color(colors.warning),
                    );
                }
            },
        );
        ui.add_space(4.0);
    }

    // 6. Elevation note: a check that needs admin may simply not have answered.
    if !state.sys_info.is_admin {
        ui.label(
            RichText::new(format!("⚠ {}", tr(lang, "common.requires_admin")))
                .size(11.0)
                .color(colors.warning),
        );
        ui.add_space(6.0);
    }

    // 7. Footnote: this view reports, it never disables
    card_container(ui, |ui| {
        ui.label(
            RichText::new(tr(lang, "security.no_disable_note"))
                .size(11.0)
                .color(colors.text_muted),
        );
    });
}
