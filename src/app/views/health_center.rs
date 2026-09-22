//! Health Center view.
//!
//! WHY the view is shaped this way:
//!
//! * The overall verdict is rendered with its unknowns counted and named. A
//!   report that says "Healthy" while two checks could not be queried would be
//!   the exact failure this center was built to fix, so the unknown count is
//!   visible next to the rating rather than buried below it.
//! * SFC and DISM run through the worker system and their output is shown in the
//!   view and written to the audit log. These tools take minutes; running them
//!   inline would freeze the frame loop, which is what the previous version did
//!   with a bare `std::thread::spawn` that discarded its output.
//! * The integrity check reports "not checked" until SFC has actually run. Wino
//!   has no way to know file integrity without running the tool, so it does not
//!   guess.
//! * A stopped core service is the highest-signal problem on this page, so
//!   problems are listed in severity order above the full check list.

use crate::app::components::{action_card, card_container, status_badge, view_header};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::app::worker::Action;
use crate::core::i18n::tr;
use crate::health::center::HealthState;
use eframe::egui::{Button, Color32, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    view_header(
        ui,
        tr(lang, "healthcenter.title"),
        tr(lang, "healthcenter.subtitle"),
        |ui| {
            let rescan_btn = Button::new(format!("↻ {}", tr(lang, "healthcenter.rescan")))
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(rescan_btn).clicked() {
                state.refresh_health_center();
            }
        },
    );

    let report = &state.health_center;

    // ---- Overall ----
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tr(lang, "healthcenter.overall"))
                    .size(15.5)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                status_badge(
                    ui,
                    tr(lang, report.overall.i18n_key()),
                    report.overall.color_rgb(),
                );
            });
        });

        ui.add_space(6.0);

        let healthy = report.count(HealthState::Healthy);
        let unknown = report.count(HealthState::Unknown);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!(
                    "{} / {} {}",
                    healthy,
                    report.checks.len(),
                    tr(lang, "healthcenter.checks")
                ))
                .size(11.5)
                .color(colors.text_secondary),
            );
            if unknown > 0 {
                status_badge(ui, tr(lang, "healthcenter.state_unknown"), (148, 163, 184));
                ui.label(
                    RichText::new(format!("{} {}", unknown, tr(lang, "common.unknown")))
                        .size(11.0)
                        .color(colors.text_muted),
                );
            }
        });

        // Problems first: a stopped core service must not require scrolling.
        let problems = report.problems();
        if !problems.is_empty() {
            ui.add_space(8.0);
            for check in problems {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("•").size(12.0).color(Color32::from_rgb(
                        check.state.color_rgb().0,
                        check.state.color_rgb().1,
                        check.state.color_rgb().2,
                    )));
                    ui.label(RichText::new(tr(lang, &check.name_key)).size(12.0).strong());
                    status_badge(
                        ui,
                        tr(lang, check.state.i18n_key()),
                        check.state.color_rgb(),
                    );
                    ui.label(
                        RichText::new(&check.detail)
                            .size(11.5)
                            .color(colors.text_secondary),
                    );
                });
                if !check.recommendation.is_empty() {
                    ui.label(
                        RichText::new(format!("    {}", check.recommendation))
                            .size(11.0)
                            .color(colors.text_muted),
                    );
                }
                ui.add_space(3.0);
            }
        }

        ui.add_space(4.0);
        ui.label(
            RichText::new(tr(lang, "healthcenter.no_fabrication_note"))
                .size(10.5)
                .color(colors.text_muted),
        );
    });

    ui.add_space(10.0);

    // ---- Long-running tools ----
    card_container(ui, |ui| {
        ui.label(
            RichText::new(tr(lang, "healthcenter.file_integrity"))
                .size(15.0)
                .strong(),
        );
        ui.add_space(2.0);
        ui.label(
            RichText::new(tr(lang, "healthcenter.long_running_note"))
                .size(11.5)
                .color(colors.text_muted),
        );

        if !state.sys_info.is_admin {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("⚠").size(12.0).color(colors.warning));
                ui.label(
                    RichText::new(tr(lang, "healthcenter.admin_required"))
                        .size(11.0)
                        .color(colors.warning),
                );
            });
        }

        ui.add_space(10.0);

        let busy = state.action_busy;
        ui.horizontal_wrapped(|ui| {
            let sfc_btn = Button::new(
                RichText::new(tr(lang, "healthcenter.run_sfc")).color(colors.text_primary),
            )
            .fill(colors.bg_card_hover)
            .stroke(Stroke::new(1.0_f32, colors.border))
            .rounding(Rounding::same(6.0))
            .min_size(Vec2::new(150.0, 30.0));
            if ui.add_enabled(!busy, sfc_btn).clicked() {
                state.request_action(Action::RunSfc { dry_run: false });
            }

            ui.add_space(6.0);

            let dism_check_btn = Button::new(
                RichText::new(tr(lang, "healthcenter.run_dism_check")).color(colors.text_primary),
            )
            .fill(colors.bg_card_hover)
            .stroke(Stroke::new(1.0_f32, colors.border))
            .rounding(Rounding::same(6.0))
            .min_size(Vec2::new(190.0, 30.0));
            if ui.add_enabled(!busy, dism_check_btn).clicked() {
                state.request_action(Action::RunDism {
                    scan_health: false,
                    dry_run: false,
                });
            }

            ui.add_space(6.0);

            let dism_scan_btn = Button::new(
                RichText::new(tr(lang, "healthcenter.run_dism_scan")).color(colors.text_primary),
            )
            .fill(colors.bg_card_hover)
            .stroke(Stroke::new(1.0_f32, colors.border))
            .rounding(Rounding::same(6.0))
            .min_size(Vec2::new(190.0, 30.0));
            if ui.add_enabled(!busy, dism_scan_btn).clicked() {
                state.request_action(Action::RunDism {
                    scan_health: true,
                    dry_run: false,
                });
            }
        });

        if busy {
            ui.add_space(6.0);
            ui.label(
                RichText::new(tr(lang, "healthcenter.tool_running"))
                    .size(11.5)
                    .color(colors.secondary),
            );
        }

        // Tool output, when one has been run this session.
        if let Some((title, detail, success)) = &state.health_tool_output {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr(lang, "healthcenter.tool_output"))
                        .size(13.0)
                        .strong(),
                );
                status_badge(
                    ui,
                    title,
                    if *success {
                        (34, 197, 94)
                    } else {
                        (239, 68, 68)
                    },
                );
            });
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(detail)
                                .size(10.5)
                                .monospace()
                                .color(colors.text_secondary),
                        )
                        .wrap(),
                    );
                });
        }
    });

    ui.add_space(10.0);

    // ---- Full check list ----
    for check in &state.health_center.checks {
        action_card(
            ui,
            130.0,
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(tr(lang, &check.name_key)).size(13.0).strong());
                    status_badge(
                        ui,
                        tr(lang, check.state.i18n_key()),
                        check.state.color_rgb(),
                    );
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new(&check.detail)
                        .size(11.5)
                        .color(colors.text_secondary),
                );
                if !check.recommendation.is_empty() {
                    ui.add_space(1.0);
                    ui.label(
                        RichText::new(&check.recommendation)
                            .size(11.0)
                            .color(colors.text_muted),
                    );
                }
            },
            |ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(120.0, 26.0),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label(RichText::new(check.state.as_str()).size(10.5).color(
                            Color32::from_rgb(
                                check.state.color_rgb().0,
                                check.state.color_rgb().1,
                                check.state.color_rgb().2,
                            ),
                        ));
                    },
                );
            },
        );
        ui.add_space(4.0);
    }
}
