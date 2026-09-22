//! Before / After Benchmark view.
//!
//! WHY the view is shaped this way:
//!
//! * The comparison renders only rows built from values that were actually
//!   captured. There is no percentage-improvement field anywhere, because any
//!   such number would be derived rather than measured, and a derived
//!   "optimization score" is exactly the kind of claim this release removes.
//! * A capture is read-only. The view can only ask for one; it cannot apply,
//!   tune, or change anything, so a "before" capture taken mid-experiment does
//!   not perturb the machine.
//! * Two captures are selected explicitly rather than comparing the newest two
//!   automatically, so a comparison is always between states the user chose.

use crate::app::components::{card_container, status_badge, view_header};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::app::worker::Action;
use crate::benchmark::models::{byte_delta_label, compare, DeltaDirection};
use crate::core::i18n::tr;
use eframe::egui::{Button, Color32, RichText, Rounding, Stroke, Ui, Vec2};

/// Colour for a delta, given which direction the user is hoping for.
///
/// A decrease in RAM, process count, startup count, or temp bytes is the
/// improvement; an increase is not. Textual rows get a neutral colour.
fn delta_color(direction: DeltaDirection, colors: &crate::app::theme::FluentColors) -> Color32 {
    match direction {
        DeltaDirection::Decreased => colors.success,
        DeltaDirection::Increased => colors.warning,
        DeltaDirection::Unchanged => colors.text_muted,
        DeltaDirection::Textual => colors.text_secondary,
    }
}

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    view_header(
        ui,
        tr(lang, "bench.title"),
        tr(lang, "bench.subtitle"),
        |ui| {
            let before_btn = Button::new(tr(lang, "bench.capture_before"))
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add_enabled(!state.action_busy, before_btn).clicked() {
                state.request_action(Action::CaptureBenchmark {
                    label: tr(lang, "bench.before").to_string(),
                });
            }

            ui.add_space(6.0);

            let after_btn = Button::new(tr(lang, "bench.capture_after"))
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add_enabled(!state.action_busy, after_btn).clicked() {
                state.request_action(Action::CaptureBenchmark {
                    label: tr(lang, "bench.after").to_string(),
                });
            }
        },
    );

    // Measured-only note is unconditional.
    card_container(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("📊").size(14.0).color(colors.accent));
            ui.label(
                RichText::new(tr(lang, "bench.measured_only_note"))
                    .size(11.5)
                    .color(colors.text_secondary),
            );
        });
    });

    ui.add_space(10.0);

    if state.benchmark_samples.is_empty() {
        card_container(ui, |ui| {
            ui.label(
                RichText::new(tr(lang, "bench.no_samples"))
                    .size(13.0)
                    .strong(),
            );
        });
        return;
    }

    let mut toggle: Option<String> = None;
    let mut clear = false;

    // ---- Captures ----
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr(lang, "bench.samples")).size(15.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let clear_btn =
                    Button::new(RichText::new(tr(lang, "bench.clear")).color(colors.danger))
                        .fill(colors.bg_card_hover)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(110.0, 26.0));
                if ui.add(clear_btn).clicked() {
                    clear = true;
                }
            });
        });
        ui.add_space(8.0);

        for sample in &state.benchmark_samples {
            let ticked = state.benchmark_selected.contains(&sample.id);

            ui.horizontal(|ui| {
                let mut checked = ticked;
                if ui.checkbox(&mut checked, "").changed() {
                    toggle = Some(sample.id.clone());
                }

                ui.allocate_ui_with_layout(
                    Vec2::new(150.0, 20.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.label(RichText::new(&sample.label).size(12.5).strong());
                    },
                );

                ui.label(
                    RichText::new(&sample.timestamp)
                        .size(11.0)
                        .color(colors.text_muted),
                );

                ui.add_space(8.0);

                ui.label(
                    RichText::new(format!(
                        "{}: {}",
                        tr(lang, "bench.idle_ram"),
                        crate::apps::models::format_size(sample.ram_used_bytes)
                    ))
                    .size(11.0)
                    .color(colors.text_secondary),
                );
                ui.label(
                    RichText::new(format!(
                        "{}: {}",
                        tr(lang, "bench.process_count"),
                        sample.process_count
                    ))
                    .size(11.0)
                    .color(colors.text_secondary),
                );
                ui.label(
                    RichText::new(format!(
                        "{}: {}",
                        tr(lang, "bench.startup_count"),
                        sample.startup_count
                    ))
                    .size(11.0)
                    .color(colors.text_secondary),
                );
                ui.label(
                    RichText::new(format!(
                        "{}: {}",
                        tr(lang, "bench.temp_usage"),
                        crate::apps::models::format_size(sample.temp_bytes)
                    ))
                    .size(11.0)
                    .color(colors.text_secondary),
                );
                if !sample.power_plan.is_empty() {
                    ui.label(
                        RichText::new(format!(
                            "{}: {}",
                            tr(lang, "bench.power_plan"),
                            sample.power_plan
                        ))
                        .size(11.0)
                        .color(colors.text_secondary),
                    );
                }
            });
            ui.add_space(4.0);
        }
    });

    // ---- Comparison of exactly two captures ----
    let selected: Vec<_> = state
        .benchmark_samples
        .iter()
        .filter(|s| state.benchmark_selected.contains(&s.id))
        .cloned()
        .collect();

    ui.add_space(10.0);

    card_container(ui, |ui| {
        ui.label(RichText::new(tr(lang, "bench.compare")).size(15.0).strong());
        ui.add_space(6.0);

        if selected.len() != 2 {
            ui.label(
                RichText::new(tr(lang, "bench.select_two"))
                    .size(11.5)
                    .color(colors.text_muted),
            );
            return;
        }

        // Order by capture time so "before" is genuinely the earlier sample.
        let (first, second) = if selected[0].timestamp <= selected[1].timestamp {
            (&selected[0], &selected[1])
        } else {
            (&selected[1], &selected[0])
        };

        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(240.0, 18.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(RichText::new("").size(11.0));
                },
            );
            ui.allocate_ui_with_layout(
                Vec2::new(170.0, 18.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(
                        RichText::new(tr(lang, "bench.before"))
                            .size(11.0)
                            .strong()
                            .color(colors.text_muted),
                    );
                },
            );
            ui.allocate_ui_with_layout(
                Vec2::new(170.0, 18.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(
                        RichText::new(tr(lang, "bench.after"))
                            .size(11.0)
                            .strong()
                            .color(colors.text_muted),
                    );
                },
            );
            ui.label(
                RichText::new(tr(lang, "bench.delta"))
                    .size(11.0)
                    .strong()
                    .color(colors.text_muted),
            );
        });

        ui.add_space(4.0);

        for row in compare(first, second) {
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(240.0, 18.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.label(
                            RichText::new(tr(lang, row.label_key))
                                .size(11.5)
                                .color(colors.text_secondary),
                        );
                    },
                );
                ui.allocate_ui_with_layout(
                    Vec2::new(170.0, 18.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.label(
                            RichText::new(&row.before)
                                .size(11.5)
                                .color(colors.text_primary),
                        );
                    },
                );
                ui.allocate_ui_with_layout(
                    Vec2::new(170.0, 18.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.label(
                            RichText::new(&row.after)
                                .size(11.5)
                                .color(colors.text_primary),
                        );
                    },
                );
                ui.label(
                    RichText::new(match row.direction {
                        DeltaDirection::Unchanged => tr(lang, "bench.no_change").to_string(),
                        _ => byte_delta_label(
                            parse_first_number(&row.before),
                            parse_first_number(&row.after),
                        ),
                    })
                    .size(11.0)
                    .strong()
                    .color(delta_color(row.direction, &colors)),
                );
            });
            ui.add_space(3.0);
        }

        // Service and privacy state changes, when both captures recorded them.
        let service_diffs = first.service_diffs(second);
        let privacy_diffs = first.privacy_diffs(second);

        if !service_diffs.is_empty() {
            ui.add_space(8.0);
            ui.label(
                RichText::new(tr(lang, "bench.service_states"))
                    .size(12.5)
                    .strong(),
            );
            ui.add_space(3.0);
            for (name, before, after) in &service_diffs {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(name).size(11.0).color(colors.text_secondary));
                    status_badge(ui, before, (100, 116, 139));
                    ui.label(RichText::new("→").size(11.0).color(colors.text_muted));
                    status_badge(
                        ui,
                        after,
                        if after == "Running" {
                            (34, 197, 94)
                        } else {
                            (100, 116, 139)
                        },
                    );
                });
                ui.add_space(2.0);
            }
        }

        if !privacy_diffs.is_empty() {
            ui.add_space(8.0);
            ui.label(
                RichText::new(tr(lang, "bench.privacy_states"))
                    .size(12.5)
                    .strong(),
            );
            ui.add_space(3.0);
            for (id, before, after) in &privacy_diffs {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(id).size(11.0).color(colors.text_secondary));
                    status_badge(
                        ui,
                        if *before {
                            tr(lang, "common.enabled")
                        } else {
                            tr(lang, "common.disabled")
                        },
                        if *before {
                            (34, 197, 94)
                        } else {
                            (100, 116, 139)
                        },
                    );
                    ui.label(RichText::new("→").size(11.0).color(colors.text_muted));
                    status_badge(
                        ui,
                        if *after {
                            tr(lang, "common.enabled")
                        } else {
                            tr(lang, "common.disabled")
                        },
                        if *after {
                            (34, 197, 94)
                        } else {
                            (100, 116, 139)
                        },
                    );
                });
                ui.add_space(2.0);
            }
        }
    });

    // ---- Apply collected actions ----
    if let Some(id) = toggle {
        if state.benchmark_selected.contains(&id) {
            state.benchmark_selected.retain(|s| s != &id);
        } else {
            state.benchmark_selected.push(id);
        }
    }

    if clear {
        state.benchmark_samples.clear();
        state.benchmark_selected.clear();
    }
}

/// Pull the leading number out of a formatted size or count string.
///
/// The comparison rows carry pre-formatted display strings, so the delta label
/// re-reads the number from them rather than keeping a parallel numeric copy
/// that could drift out of sync with what is displayed.
fn parse_first_number(text: &str) -> u64 {
    let number: String = text
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let value = number.parse::<f64>().unwrap_or(0.0);
    // Sizes are displayed in binary units; scale back to bytes so the delta
    // label renders in the same unit family.
    let lower = text.to_lowercase();
    if lower.contains("gb") {
        (value * 1024.0 * 1024.0 * 1024.0) as u64
    } else if lower.contains("mb") {
        (value * 1024.0 * 1024.0) as u64
    } else if lower.contains("kb") {
        (value * 1024.0) as u64
    } else {
        value as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_first_number_scales_binary_units() {
        assert_eq!(parse_first_number("4.00 GB"), 4 * 1024 * 1024 * 1024);
        assert_eq!(parse_first_number("120 MB"), 120 * 1024 * 1024);
        assert_eq!(parse_first_number("17"), 17);
        assert_eq!(parse_first_number("no change"), 0);
    }
}
