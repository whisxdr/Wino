//! Profile Engine view.
//!
//! WHY the view is shaped this way:
//!
//! * Applying a profile routes through [`AppState::request_confirm`] instead of
//!   calling [`crate::profiles::manager::apply_profile`] directly. A profile is a
//!   batch of system changes, so it goes through the same disclosure step as a
//!   single change, and the preview is available before that step rather than
//!   after it.
//! * The preview renders the same [`crate::core::safety::OperationDescriptor`]s
//!   the Safety Engine validates, so what the user reads is what the engine will
//!   judge — not a separately maintained description that can drift.
//! * Built-in profiles cannot be deleted; the view says why and offers Duplicate
//!   as the path to customisation, rather than hiding the Delete button and
//!   leaving the user to guess.
//! * Steps the Safety Engine will hard-block are named on the card before Apply
//!   is pressed. Discovering a skipped step only from the result report would
//!   mean the user confirmed something different from what ran.
//! * Rename and import use separate buffers. Sharing one text field between two
//!   unrelated actions means typing a path silently rewrites the pending name.

use crate::app::components::{card_container, start_two_columns, status_badge, view_header};
use crate::app::state::{AppState, PendingConfirm};
use crate::app::theme::get_colors;
use crate::core::i18n::tr;
use crate::profiles::manager::{
    export_profile, import_profile, preview_profile, save_user_profile,
};
use crate::profiles::models::{slugify, Profile};
use eframe::egui::{Button, Color32, RichText, Rounding, Stroke, TextEdit, Ui, Vec2};

/// Actions the view collects while iterating, applied after the loop so the
/// immutable borrow of `state.profiles` has ended.
#[derive(Default)]
struct Deferred {
    select: Option<String>,
    stage_apply: Option<Box<Profile>>,
    duplicate: Option<String>,
    delete: Option<String>,
    start_rename: Option<(String, String)>,
    commit_rename: bool,
    cancel_rename: bool,
    reset: Option<String>,
    export: Option<String>,
    import: bool,
    toggle_preview: Option<String>,
}

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    view_header(
        ui,
        tr(lang, "profiles.title"),
        tr(lang, "profiles.subtitle"),
        |ui| {
            let rescan_btn = Button::new(format!("↻ {}", tr(lang, "common.refresh")))
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(rescan_btn).clicked() {
                state.refresh_profiles();
            }
        },
    );

    if state.profiles.is_empty() {
        card_container(ui, |ui| {
            ui.label(
                RichText::new(tr(lang, "common.loading"))
                    .size(12.0)
                    .color(colors.text_muted),
            );
        });
        return;
    }

    let mut deferred = Deferred::default();

    let (mut left_ui, mut right_ui, initial_pos, total_w) = start_two_columns(ui, 0.40, 12.0);

    // The selected profile is cloned up front so the list borrow below cannot
    // conflict with the mutable access the detail panel needs.
    let selected = state
        .profiles_selected
        .as_ref()
        .and_then(|id| state.profiles.iter().find(|p| &p.id == id))
        .cloned();

    // ---- Left: profile list ----
    for profile in &state.profiles {
        let is_selected = state.profiles_selected.as_deref() == Some(profile.id.as_str());
        let origin_key = if profile.is_builtin() {
            "profiles.builtin"
        } else {
            "profiles.custom"
        };
        let (risk_r, risk_g, risk_b) = profile.max_risk().color_rgb();

        let response = egui::Frame::none()
            .fill(if is_selected {
                colors.bg_card_hover
            } else {
                colors.bg_card
            })
            .stroke(Stroke::new(
                1.0_f32,
                if is_selected {
                    colors.accent
                } else {
                    colors.border
                },
            ))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::same(12.0))
            .show(&mut left_ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&profile.name).size(13.5).strong());
                    status_badge(ui, tr(lang, origin_key), (100, 116, 139));
                });
                ui.add_space(3.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} {}",
                            profile.steps.len(),
                            tr(lang, "profiles.steps_count")
                        ))
                        .size(11.0)
                        .color(colors.text_secondary),
                    );
                    status_badge(ui, profile.max_risk().as_str(), (risk_r, risk_g, risk_b));
                    if profile.is_default {
                        status_badge(ui, tr(lang, "profiles.default_badge"), (60, 144, 255));
                    }
                    if profile.last_applied.is_some() {
                        status_badge(ui, tr(lang, "profiles.active_badge"), (78, 222, 163));
                    }
                });
                ui.add_space(3.0);
                ui.label(
                    RichText::new(&profile.description)
                        .size(11.0)
                        .color(colors.text_muted),
                );
            })
            .response;

        if ui
            .interact(
                response.rect,
                ui.id().with(&profile.id),
                egui::Sense::click(),
            )
            .clicked()
        {
            deferred.select = Some(profile.id.clone());
        }

        left_ui.add_space(6.0);
    }

    // ---- Right: detail of the selected profile ----
    match &selected {
        None => {
            card_container(&mut right_ui, |ui| {
                ui.label(
                    RichText::new(tr(lang, "profiles.no_steps"))
                        .size(12.0)
                        .color(colors.text_muted),
                );
            });
        }
        Some(profile) => {
            render_detail(&mut right_ui, state, &mut deferred, profile, &colors);
        }
    }

    crate::app::components::end_two_columns(ui, left_ui, right_ui, initial_pos, total_w);

    apply_deferred(state, deferred, &colors);
}

/// The right-hand panel for one profile: description, steps, actions, preview.
///
/// Takes `&mut AppState` because the rename and import fields are edited in
/// place here. The profile itself is passed as a clone so the borrow of the
/// list that produced it has already ended.
fn render_detail(
    ui: &mut Ui,
    state: &mut AppState,
    deferred: &mut Deferred,
    profile: &Profile,
    colors: &crate::app::theme::FluentColors,
) {
    let lang = state.lang;

    card_container(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(&profile.name).size(16.0).strong());
            status_badge(
                ui,
                tr(
                    lang,
                    if profile.is_builtin() {
                        "profiles.builtin"
                    } else {
                        "profiles.custom"
                    },
                ),
                (100, 116, 139),
            );
        });
        ui.add_space(2.0);
        ui.label(
            RichText::new(&profile.description)
                .size(11.5)
                .color(colors.text_secondary),
        );

        ui.add_space(8.0);

        // Blocked steps are disclosed before Apply is pressed.
        let blocked = profile.blocked_steps();
        if !blocked.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("⚠").size(12.0).color(colors.danger));
                ui.label(
                    RichText::new(format!(
                        "{} ({} step(s))",
                        tr(lang, "common.operation_blocked"),
                        blocked.len()
                    ))
                    .size(11.5)
                    .color(colors.danger),
                );
            });
            ui.add_space(4.0);
        }

        if profile.steps.is_empty() {
            ui.label(
                RichText::new(tr(lang, "profiles.no_steps"))
                    .size(12.0)
                    .color(colors.text_muted),
            );
        } else {
            for (kind, steps) in profile.steps_by_kind() {
                let (kr, kg, kb) = kind.color_rgb();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr(lang, kind.i18n_key()))
                            .size(12.0)
                            .strong()
                            .color(Color32::from_rgb(kr, kg, kb)),
                    );
                    ui.label(
                        RichText::new(format!("({})", steps.len()))
                            .size(10.5)
                            .color(colors.text_muted),
                    );
                });
                ui.add_space(2.0);
                for step in steps {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("•").size(11.0).color(colors.text_muted));
                        ui.label(
                            RichText::new(&step.label)
                                .size(11.5)
                                .color(colors.text_primary),
                        );
                        let (r, g, b) = step.risk.color_rgb();
                        status_badge(ui, step.risk.as_str(), (r, g, b));
                        if !step.enable {
                            status_badge(ui, tr(lang, "common.disabled"), (100, 116, 139));
                        }
                    });
                    ui.add_space(1.0);
                }
                ui.add_space(6.0);
            }
        }
    });

    ui.add_space(8.0);

    // ---- Actions ----
    card_container(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let apply_btn = Button::new(
                RichText::new(tr(lang, "profiles.apply"))
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(colors.accent)
            .rounding(Rounding::same(6.0))
            .min_size(Vec2::new(120.0, 30.0));
            if ui.add(apply_btn).clicked() {
                deferred.stage_apply = Some(Box::new(profile.clone()));
            }

            let preview_btn = Button::new(tr(lang, "profiles.preview"))
                .fill(colors.bg_card_hover)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(100.0, 30.0));
            if ui.add(preview_btn).clicked() {
                deferred.toggle_preview = Some(profile.id.clone());
            }

            let dup_btn = Button::new(tr(lang, "profiles.duplicate"))
                .fill(colors.bg_card_hover)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(100.0, 30.0));
            if ui.add(dup_btn).clicked() {
                deferred.duplicate = Some(profile.id.clone());
            }

            let rename_btn = Button::new(tr(lang, "profiles.rename"))
                .fill(colors.bg_card_hover)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(96.0, 30.0));
            if ui.add(rename_btn).clicked() {
                deferred.start_rename = Some((profile.id.clone(), profile.name.clone()));
            }

            let export_btn = Button::new(tr(lang, "profiles.export"))
                .fill(colors.bg_card_hover)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(88.0, 30.0));
            if ui.add(export_btn).clicked() {
                deferred.export = Some(profile.id.clone());
            }

            if profile.is_builtin() {
                ui.label(
                    RichText::new(tr(lang, "profiles.builtin_locked"))
                        .size(10.5)
                        .color(colors.text_muted),
                );
            } else {
                let delete_btn =
                    Button::new(RichText::new(tr(lang, "profiles.delete")).color(colors.danger))
                        .fill(colors.bg_card_hover)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(88.0, 30.0));
                if ui.add(delete_btn).clicked() {
                    deferred.delete = Some(profile.id.clone());
                }
            }

            let reset_btn = Button::new(tr(lang, "profiles.reset"))
                .fill(colors.bg_card_hover)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(130.0, 30.0));
            if ui.add(reset_btn).clicked() {
                deferred.reset = Some(profile.id.clone());
            }
        });
    });

    // ---- Rename editor, only while this profile is the one being renamed ----
    let renaming_this =
        state.profile_editor.as_ref().map(|e| e.id.as_str()) == Some(profile.id.as_str());
    if renaming_this {
        ui.add_space(8.0);
        card_container(ui, |ui| {
            ui.label(
                RichText::new(tr(lang, "profiles.rename_prompt"))
                    .size(12.0)
                    .strong(),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        TextEdit::singleline(&mut state.profile_rename_buffer).desired_width(220.0),
                    )
                    .changed()
                {
                    // The buffer is the source of truth while renaming; the
                    // editor copy is only used to remember which profile is
                    // being renamed.
                }
                if ui
                    .add(
                        Button::new(tr(lang, "common.save"))
                            .fill(colors.accent)
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(80.0, 26.0)),
                    )
                    .clicked()
                {
                    deferred.commit_rename = true;
                }
                if ui
                    .add(
                        Button::new(tr(lang, "common.cancel"))
                            .fill(colors.bg_card_hover)
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(80.0, 26.0)),
                    )
                    .clicked()
                {
                    deferred.cancel_rename = true;
                }
            });
        });
    }

    // ---- Preview ----
    if state.profile_preview_id.as_deref() == Some(profile.id.as_str()) {
        ui.add_space(8.0);
        let descriptors = preview_profile(profile);
        card_container(ui, |ui| {
            ui.label(
                RichText::new(tr(lang, "profiles.preview"))
                    .size(14.0)
                    .strong(),
            );
            ui.add_space(6.0);
            for desc in &descriptors {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&desc.name).size(11.5).strong());
                    let (r, g, b) = desc.risk.color_rgb();
                    status_badge(ui, desc.risk.as_str(), (r, g, b));
                    if desc.reversible {
                        status_badge(ui, tr(lang, "common.reversible"), (78, 222, 163));
                    }
                    if desc.requires_admin {
                        status_badge(ui, tr(lang, "common.requires_admin"), (234, 179, 8));
                    }
                    if desc.requires_reboot {
                        status_badge(ui, tr(lang, "common.requires_reboot"), (249, 115, 22));
                    }
                });
                if !desc.component.is_empty() {
                    ui.label(
                        RichText::new(format!(
                            "{} {}",
                            tr(lang, "common.component"),
                            desc.component
                        ))
                        .size(10.5)
                        .color(colors.text_muted),
                    );
                }
                if !desc.current_state.is_empty() || !desc.target_state.is_empty() {
                    ui.label(
                        RichText::new(format!(
                            "{} {}  |  {} {}",
                            tr(lang, "common.current_state"),
                            desc.current_state,
                            tr(lang, "common.target_state"),
                            desc.target_state
                        ))
                        .size(10.5)
                        .color(colors.text_muted),
                    );
                }
                ui.add_space(4.0);
            }
        });
    }

    // ---- Import ----
    ui.add_space(8.0);
    card_container(ui, |ui| {
        ui.label(
            RichText::new(tr(lang, "profiles.import"))
                .size(13.0)
                .strong(),
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .add(
                    TextEdit::singleline(&mut state.profile_import_buffer)
                        .hint_text(tr(lang, "common.import"))
                        .desired_width(300.0),
                )
                .changed()
            {
                // Edited in place; applied when the Import button is pressed.
            }
            if ui
                .add(
                    Button::new(tr(lang, "common.import"))
                        .fill(colors.bg_card_hover)
                        .stroke(Stroke::new(1.0_f32, colors.border))
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(90.0, 26.0)),
                )
                .clicked()
            {
                deferred.import = true;
            }
        });
    });
}

fn apply_deferred(
    state: &mut AppState,
    deferred: Deferred,
    colors: &crate::app::theme::FluentColors,
) {
    let lang = state.lang;

    if let Some(id) = deferred.select {
        state.profiles_selected = Some(id);
    }

    if let Some(profile) = deferred.stage_apply {
        state.request_confirm(PendingConfirm::ApplyProfile(profile));
    }

    if let Some(id) = deferred.toggle_preview {
        state.profile_preview_id = if state.profile_preview_id.as_deref() == Some(id.as_str()) {
            None
        } else {
            Some(id)
        };
    }

    if let Some(id) = deferred.duplicate {
        if let Some(source) = state.profiles.iter().find(|p| p.id == id).cloned() {
            let new_name = format!("{} (custom)", source.name);
            let new_id = unique_slug(&state.profiles, &slugify(&new_name));
            let copy = source.duplicate_as(&new_id, &new_name);
            match save_user_profile(&copy) {
                Ok(()) => {
                    state.set_toast(tr(lang, "profiles.exported"));
                    state.refresh_profiles();
                }
                Err(e) => state.record_event("Profiles", &e, false),
            }
        }
    }

    if let Some(id) = deferred.delete {
        match crate::profiles::manager::delete_user_profile(&id) {
            Ok(()) => {
                state.set_toast(tr(lang, "profiles.delete_confirm"));
                state.profiles_selected = None;
                state.refresh_profiles();
            }
            Err(e) => state.record_event("Profiles", &e, false),
        }
    }

    if let Some((id, name)) = deferred.start_rename {
        if let Some(profile) = state.profiles.iter().find(|p| p.id == id).cloned() {
            let mut editor = profile;
            editor.name = name.clone();
            state.profile_editor = Some(editor);
            state.profile_rename_buffer = name;
        }
    }

    if deferred.commit_rename {
        let new_name = state.profile_rename_buffer.trim().to_string();
        if let Some(editor) = state.profile_editor.clone() {
            if new_name.is_empty() {
                state.record_event("Profiles", "Profile name must not be empty.", false);
            } else {
                let mut updated = editor;
                updated.name = new_name;
                match save_user_profile(&updated) {
                    Ok(()) => {
                        state.set_toast(tr(lang, "common.save"));
                        state.profile_editor = None;
                        state.refresh_profiles();
                    }
                    Err(e) => state.record_event("Profiles", &e, false),
                }
            }
        }
    }

    if deferred.cancel_rename {
        state.profile_editor = None;
        state.profile_rename_buffer.clear();
    }

    if let Some(id) = deferred.reset {
        if let Some(builtin) = crate::profiles::builtins::builtin_profiles()
            .into_iter()
            .find(|p| p.id == id)
        {
            match save_user_profile(&builtin) {
                Ok(()) => {
                    state.set_toast(tr(lang, "profiles.reset_done"));
                    state.refresh_profiles();
                }
                Err(e) => state.record_event("Profiles", &e, false),
            }
        } else {
            state.record_event(
                "Profiles",
                "No built-in definition exists for this profile.",
                false,
            );
        }
    }

    if let Some(id) = deferred.export {
        if let Some(profile) = state.profiles.iter().find(|p| p.id == id) {
            let destination =
                crate::profiles::manager::profiles_dir().join(format!("{}.toml", profile.id));
            match export_profile(profile, &destination.to_string_lossy()) {
                Ok(msg) => state.set_toast(&msg),
                Err(e) => state.record_event("Profiles", &e, false),
            }
        }
    }

    if deferred.import {
        let path = state.profile_import_buffer.trim().to_string();
        if path.is_empty() {
            state.record_event("Profiles", "Enter a path to a profile TOML file.", false);
        } else {
            match import_profile(&path) {
                Ok(profile) => match save_user_profile(&profile) {
                    Ok(()) => {
                        state.set_toast(tr(lang, "profiles.imported"));
                        state.profile_import_buffer.clear();
                        state.refresh_profiles();
                    }
                    Err(e) => state.record_event("Profiles", &e, false),
                },
                Err(e) => state.record_event(
                    "Profiles",
                    &format!("{} {}", tr(lang, "profiles.import_failed"), e),
                    false,
                ),
            }
        }
    }

    let _ = colors;
}

/// Append a numeric suffix until the slug is free, so duplicating twice does
/// not overwrite the first copy.
fn unique_slug(existing: &[Profile], base: &str) -> String {
    if !existing.iter().any(|p| p.id == base) {
        return base.to_string();
    }
    for n in 2..1000 {
        let candidate = format!("{}_{}", base, n);
        if !existing.iter().any(|p| p.id == candidate) {
            return candidate;
        }
    }
    format!("{}_{}", base, chrono::Local::now().timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::safety::RiskLevel;
    use crate::profiles::models::{ProfileOrigin, ProfileStep, StepKind};

    fn profile(id: &str) -> Profile {
        Profile::new(id, id, "", ProfileOrigin::User).with_step(ProfileStep::new(
            StepKind::Power,
            "balanced",
            "Balanced",
            RiskLevel::Low,
        ))
    }

    #[test]
    fn unique_slug_avoids_collisions() {
        let existing = vec![profile("gaming_custom"), profile("gaming_custom_2")];
        assert_eq!(unique_slug(&existing, "gaming_custom"), "gaming_custom_3");
        assert_eq!(unique_slug(&existing, "fresh"), "fresh");
    }
}
