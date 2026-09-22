//! Windows Features view.
//!
//! WHY the view is shaped this way:
//!
//! * Every toggle routes through [`AppState::request_confirm`] instead of
//!   [`crate::windows_features::manager::set_feature_enabled`]. Optional
//!   components add services, open listeners, and change CPU virtualization, so
//!   the view only stages the change; the shared confirmation dialog owns the
//!   decision and discloses the same fields as every other mutating action.
//! * The impact text and the dependency list are rendered inline rather than
//!   hidden behind a tooltip. The point of the view is that the user reads what a
//!   component does *before* pressing Enable, so nothing that explains the change
//!   may be one hover away.
//! * A state Wino cannot toggle (unknown, or a pending reboot) renders as text,
//!   never as a disabled button: a dead button implies an action exists and is
//!   merely unavailable, which is not what "unknown" means.
//! * DISM resolves missing dependencies implicitly when a feature is enabled, so
//!   the view names them up front instead of letting Windows add them silently.

use crate::app::components::{action_card, card_container, search_bar, status_badge, view_header};
use crate::app::state::{AppState, PendingConfirm};
use crate::app::theme::get_colors;
use crate::core::i18n::tr;
use crate::windows_features::models::FeatureState;
use eframe::egui::{Button, Color32, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    // 1. Contextual header with rescan
    view_header(
        ui,
        tr(lang, "features.title"),
        tr(lang, "features.subtitle"),
        |ui| {
            let rescan_btn = Button::new(format!("↻ {}", tr(lang, "features.rescan")))
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(rescan_btn).clicked() {
                state.refresh_features();
            }
        },
    );

    // 2. Search bar over the visible subset.
    let query = state.features_search.to_lowercase();
    let visible = state.features.iter().filter(|f| f.matches(&query)).count();
    let count_str = format!("{} {}", visible, tr(lang, "common.items"));

    search_bar(
        ui,
        &mut state.features_search,
        tr(lang, "features.search_placeholder"),
        Some(&count_str),
    );

    // Restart note sits directly above the action cards, where the buttons are.
    ui.label(
        RichText::new(tr(lang, "features.restart_note"))
            .size(11.0)
            .color(colors.text_muted),
    );
    ui.add_space(8.0);

    // Collected during the loop and applied after it: the loop holds an immutable
    // borrow of `state.features`, so the confirmation cannot be staged inside.
    let mut toggle_request: Option<(String, bool, String)> = None;

    if visible == 0 {
        card_container(ui, |ui| {
            ui.label(
                RichText::new(tr(lang, "features.none_found"))
                    .size(12.5)
                    .color(colors.text_muted),
            );
        });
        ui.add_space(8.0);
    }

    // 3. One action card per matching feature
    for feature in &state.features {
        if !feature.matches(&query) {
            continue;
        }

        action_card(
            ui,
            130.0,
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&feature.display_name).size(13.5).strong());
                    status_badge(
                        ui,
                        tr(lang, feature.state.i18n_key()),
                        feature.state.color_rgb(),
                    );
                    status_badge(ui, feature.risk.as_str(), feature.risk.color_rgb());
                    if feature.curated {
                        status_badge(ui, tr(lang, "profiles.builtin"), (60, 144, 255));
                    }
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new(&feature.name)
                        .size(10.5)
                        .color(colors.text_muted),
                );
                ui.add_space(2.0);

                // Always visible: this is the disclosure that explains the change.
                ui.label(
                    RichText::new(&feature.impact)
                        .size(11.5)
                        .color(colors.text_secondary),
                );

                if !feature.dependencies.is_empty() {
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(format!(
                            "{}: {}",
                            tr(lang, "features.dependencies"),
                            feature.dependencies.join(", ")
                        ))
                        .size(10.5)
                        .color(colors.text_muted),
                    );
                }

                // DISM pulls disabled dependencies in on its own; say which ones.
                let unmet = feature.unmet_dependencies(&state.features);
                if !unmet.is_empty() {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("⚠ {}", tr(lang, "features.warning_body")))
                            .size(11.0)
                            .color(colors.warning),
                    );
                    let names: Vec<&str> = unmet.iter().map(|f| f.name.as_str()).collect();
                    ui.label(
                        RichText::new(format!(
                            "{}: {}",
                            tr(lang, "features.dependencies"),
                            names.join(", ")
                        ))
                        .size(11.0)
                        .color(colors.warning),
                    );
                }
            },
            |ui| match feature.target_state() {
                Some(target) if feature.can_toggle() => {
                    let enable = target == FeatureState::Enabled;
                    let text = if enable {
                        tr(lang, "features.enable")
                    } else {
                        tr(lang, "features.disable")
                    };
                    let btn = Button::new(RichText::new(text).strong().color(if enable {
                        Color32::WHITE
                    } else {
                        colors.danger
                    }))
                    .fill(if enable {
                        colors.accent
                    } else {
                        colors.bg_card_hover
                    })
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(110.0, 28.0));

                    if ui.add(btn).clicked() {
                        toggle_request =
                            Some((feature.name.clone(), enable, feature.display_name.clone()));
                    }
                }
                // No toggle is offered: state the reason as text, never a dead button.
                _ => {
                    ui.label(
                        RichText::new(tr(lang, feature.state.i18n_key()))
                            .size(11.0)
                            .color(colors.text_muted),
                    );
                }
            },
        );
        ui.add_space(4.0);
    }

    if let Some((name, enable, label)) = toggle_request {
        state.request_confirm(PendingConfirm::ToggleFeature {
            name,
            enable,
            label,
        });
    }

    // 4. Footnote: where the state actually comes from
    ui.add_space(8.0);
    card_container(ui, |ui| {
        ui.label(
            RichText::new(tr(lang, "features.dism_note"))
                .size(11.0)
                .color(colors.text_muted),
        );
    });
}
