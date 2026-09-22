//! Recommendations view.
//!
//! WHY the view is shaped this way:
//!
//! * Every card leads to a Review action that navigates to the view where the
//!   change would actually be made. There is no apply button here at all: a
//!   recommendation is an observation, and the note saying so is rendered above
//!   the list so the absence of an apply button is a stated design rather than
//!   something the user has to infer.
//! * The measured value is rendered on every card. A recommendation without its
//!   number is indistinguishable from a guess, which is the failure mode this
//!   engine exists to avoid.
//! * An empty list renders an explicit "nothing needs attention" state. That is
//!   a real result and should not look like a failed scan.

use crate::app::components::{card_container, status_badge, view_header};
use crate::app::navigation::NavTab;
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::core::i18n::tr;
use crate::recommendations::models::ReviewTarget;
use eframe::egui::{Button, Color32, RichText, Rounding, Stroke, Ui, Vec2};

/// Map a review target to the tab that owns the change.
fn target_tab(target: ReviewTarget) -> NavTab {
    match target {
        ReviewTarget::Memory => NavTab::Memory,
        ReviewTarget::Processes => NavTab::Processes,
        ReviewTarget::Cleaner => NavTab::Cleaner,
        ReviewTarget::Storage => NavTab::Storage,
        ReviewTarget::Startup => NavTab::Startup,
        ReviewTarget::Services => NavTab::Services,
        ReviewTarget::Privacy => NavTab::Privacy,
        ReviewTarget::Power => NavTab::Power,
        ReviewTarget::Security => NavTab::Security,
        ReviewTarget::Network => NavTab::Network,
        ReviewTarget::Apps => NavTab::Apps,
        ReviewTarget::Health => NavTab::Health,
        ReviewTarget::Recommendations => NavTab::Recommendations,
    }
}

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    view_header(ui, tr(lang, "rec.title"), tr(lang, "rec.subtitle"), |ui| {
        let rescan_btn = Button::new(format!("↻ {}", tr(lang, "rec.rescan")))
            .fill(colors.bg_card)
            .stroke(Stroke::new(1.0_f32, colors.border))
            .rounding(Rounding::same(6.0));
        if ui.add(rescan_btn).clicked() {
            state.refresh_recommendations();
        }
    });

    // The no-auto-apply note is unconditional: the absence of an apply button is
    // only reassuring when it is stated.
    card_container(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("💡").size(14.0).color(colors.accent));
            ui.label(
                RichText::new(tr(lang, "rec.no_auto_apply_note"))
                    .size(11.5)
                    .color(colors.text_secondary),
            );
        });
    });

    ui.add_space(10.0);

    if state.recommendations.is_empty() {
        card_container(ui, |ui| {
            ui.label(RichText::new(tr(lang, "rec.none")).size(13.0).strong());
            ui.add_space(2.0);
            ui.label(
                RichText::new(tr(lang, "rec.none_note"))
                    .size(11.5)
                    .color(colors.text_muted),
            );
        });
        return;
    }

    let mut navigate: Option<NavTab> = None;

    for rec in &state.recommendations {
        let (ar, ag, ab) = rec.area.color_rgb();
        let (sr, sg, sb) = rec.severity.color_rgb();

        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width() - 120.0, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    card_container(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(&rec.title).size(13.0).strong());
                            status_badge(ui, tr(lang, rec.severity.i18n_key()), (sr, sg, sb));
                            status_badge(ui, tr(lang, rec.area.i18n_key()), (ar, ag, ab));
                        });

                        if !rec.measured.is_empty() {
                            ui.add_space(3.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(tr(lang, "rec.measured_value"))
                                        .size(10.5)
                                        .color(colors.text_muted),
                                );
                                ui.label(
                                    RichText::new(&rec.measured)
                                        .size(11.5)
                                        .strong()
                                        .color(colors.text_primary),
                                );
                            });
                        }
                    });
                },
            );

            ui.add_space(8.0);

            ui.allocate_ui_with_layout(
                Vec2::new(112.0, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.add_space(10.0);
                    let review_btn = Button::new(
                        RichText::new(tr(lang, "rec.review_action")).color(colors.accent),
                    )
                    .fill(colors.bg_card_hover)
                    .stroke(Stroke::new(1.0_f32, colors.border))
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(104.0, 28.0));
                    if ui.add(review_btn).clicked() {
                        navigate = Some(target_tab(rec.target));
                    }
                },
            );
        });

        ui.add_space(6.0);
    }

    // Navigate after the loop so the borrow of `state.recommendations` has ended.
    if let Some(tab) = navigate {
        state.current_tab = tab;
        if let Some(kind) = tab.on_open_scan() {
            state.request_scan(kind);
        }
    }

    ui.add_space(4.0);
    ui.label(
        RichText::new(format!(
            "{} {}",
            state.recommendations.len(),
            tr(lang, "rec.count_fmt")
        ))
        .size(10.5)
        .color(Color32::from_rgb(
            colors.text_muted.r(),
            colors.text_muted.g(),
            colors.text_muted.b(),
        )),
    );
}
