//! Power Manager view.
//!
//! WHY the view is shaped this way:
//!
//! * Every plan switch and every setting write routes through
//!   [`AppState::request_confirm`]. Power changes are reversible and
//!   snapshotted, but they change machine behavior immediately, so they get the
//!   same disclosure step as any other system change.
//! * A setting the active scheme does not expose renders explanatory text with
//!   no control at all. A slider bound to a value the API refused to read would
//!   show a fabricated number and write it back on Apply.
//! * Creating Ultimate Performance and activating it are separate actions. The
//!   scheme does not exist on a stock install, so the button that creates it
//!   says so, and switching to it stays an explicit second step.

use crate::app::components::{card_container, status_badge, view_header};
use crate::app::state::{AppState, PendingConfirm};
use crate::app::theme::get_colors;
use crate::core::i18n::tr;
use crate::power::models::{setting_label_key, PowerPlanKind};
use eframe::egui::{Button, Color32, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    view_header(
        ui,
        tr(lang, "power.title"),
        tr(lang, "power.subtitle"),
        |ui| {
            let rescan_btn = Button::new(format!("↻ {}", tr(lang, "common.refresh")))
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(rescan_btn).clicked() {
                state.refresh_power();
            }
        },
    );

    if !state.sys_info.is_admin {
        card_container(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("⚠").size(12.0).color(colors.warning));
                ui.label(
                    RichText::new(tr(lang, "common.requires_admin"))
                        .size(11.5)
                        .color(colors.warning),
                );
            });
        });
        ui.add_space(10.0);
    }

    // Collected during the loops, applied after them.
    let mut stage_plan: Option<(String, String)> = None;
    let mut stage_setting: Option<(crate::power::models::PowerSetting, u32, u32)> = None;
    let mut create_ultimate = false;
    let mut edit_ac: Option<(String, u32)> = None;
    let mut edit_dc: Option<(String, u32)> = None;

    // ---- Active plan ----
    let active = state.power_plans.iter().find(|p| p.is_active).cloned();

    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🔌").size(15.0).color(colors.accent));
            ui.label(
                RichText::new(tr(lang, "power.active_plan"))
                    .size(15.0)
                    .strong(),
            );
        });
        ui.add_space(6.0);

        match &active {
            Some(plan) => {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&plan.name).size(16.0).strong());
                    status_badge(ui, tr(lang, plan.kind_i18n_key()), (60, 144, 255));
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new(&plan.guid)
                        .size(10.5)
                        .color(colors.text_muted),
                );
            }
            None => {
                ui.label(
                    RichText::new(tr(lang, "common.loading"))
                        .size(12.0)
                        .color(colors.text_muted),
                );
            }
        }
    });

    ui.add_space(10.0);

    // ---- Available plans ----
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("⚡").size(15.0).color(colors.secondary));
            ui.label(
                RichText::new(tr(lang, "power.available_plans"))
                    .size(15.0)
                    .strong(),
            );
        });
        ui.add_space(8.0);

        if state.power_plans.is_empty() {
            ui.label(
                RichText::new(tr(lang, "common.loading"))
                    .size(12.0)
                    .color(colors.text_muted),
            );
            return;
        }

        let mut has_ultimate = false;

        for plan in state
            .power_plans
            .iter()
            .filter(|p| p.is_offered() || p.kind == PowerPlanKind::Custom)
        {
            if plan.kind == PowerPlanKind::Ultimate {
                has_ultimate = true;
            }

            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(260.0, 24.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.label(RichText::new(&plan.name).size(12.5).strong());
                        ui.add_space(4.0);
                        status_badge(ui, tr(lang, plan.kind_i18n_key()), (100, 116, 139));
                    },
                );

                if plan.is_active {
                    status_badge(ui, tr(lang, "profiles.active_badge"), (78, 222, 163));
                } else {
                    let apply_btn = Button::new(tr(lang, "power.apply_plan"))
                        .fill(colors.bg_card_hover)
                        .stroke(Stroke::new(1.0_f32, colors.border))
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(110.0, 24.0));
                    if ui.add(apply_btn).clicked() {
                        stage_plan = Some((plan.guid.clone(), plan.name.clone()));
                    }
                }
            });
            ui.add_space(3.0);
        }

        // Ultimate Performance is absent on a stock install. Offering to create
        // it is honest only if the copy says that is what the button does.
        if !has_ultimate {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(tr(lang, "power.plan_ultimate_missing"))
                        .size(11.0)
                        .color(colors.text_muted),
                );
                let create_btn = Button::new(tr(lang, "power.plan_ultimate"))
                    .fill(colors.bg_card_hover)
                    .stroke(Stroke::new(1.0_f32, colors.border))
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(160.0, 24.0));
                if ui.add(create_btn).clicked() {
                    create_ultimate = true;
                }
            });
        }
    });

    ui.add_space(10.0);

    // ---- Advanced settings ----
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🛠").size(15.0).color(colors.tertiary));
            ui.label(
                RichText::new(tr(lang, "power.settings_title"))
                    .size(15.0)
                    .strong(),
            );
        });
        ui.add_space(2.0);
        ui.label(
            RichText::new(tr(lang, "power.settings_subtitle"))
                .size(11.5)
                .color(colors.text_muted),
        );
        ui.add_space(10.0);

        if state.power_settings.is_empty() {
            ui.label(
                RichText::new(tr(lang, "common.loading"))
                    .size(12.0)
                    .color(colors.text_muted),
            );
            return;
        }

        for setting in &state.power_settings {
            ui.horizontal(|ui| {
                // Label column
                ui.allocate_ui_with_layout(
                    Vec2::new(280.0, 26.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.label(
                            RichText::new(tr(lang, setting_label_key(&setting.id)))
                                .size(12.0)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(&setting.name)
                                .size(10.0)
                                .color(colors.text_muted),
                        );
                    },
                );

                if !setting.is_available() {
                    // The scheme does not expose this setting. Say so and render
                    // no control: a slider over an unreadable value would show
                    // and write back a fabricated number.
                    ui.label(
                        RichText::new(tr(lang, "power.setting_unavailable"))
                            .size(11.0)
                            .color(colors.text_muted),
                    );
                    return;
                }

                let ac_current = state
                    .power_edit_ac
                    .get(&setting.id)
                    .copied()
                    .or(setting.ac_value)
                    .unwrap_or(setting.min_value);
                let dc_current = state
                    .power_edit_dc
                    .get(&setting.id)
                    .copied()
                    .or(setting.dc_value)
                    .unwrap_or(setting.min_value);

                let mut ac_new = ac_current;
                let mut dc_new = dc_current;

                if setting.is_enumerated {
                    egui::ComboBox::from_id_salt(format!("ac_{}", setting.id))
                        .selected_text(setting.format_value(Some(ac_current)))
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for (idx, label) in setting.enum_labels.iter().enumerate() {
                                ui.selectable_value(&mut ac_new, idx as u32, label);
                            }
                        });
                    ui.add_space(4.0);
                    egui::ComboBox::from_id_salt(format!("dc_{}", setting.id))
                        .selected_text(setting.format_value(Some(dc_current)))
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for (idx, label) in setting.enum_labels.iter().enumerate() {
                                ui.selectable_value(&mut dc_new, idx as u32, label);
                            }
                        });
                } else {
                    ui.add(
                        egui::Slider::new(&mut ac_new, setting.min_value..=setting.max_value)
                            .suffix(&setting.unit)
                            .text(tr(lang, "power.ac_value")),
                    );
                    ui.add_space(4.0);
                    ui.add(
                        egui::Slider::new(&mut dc_new, setting.min_value..=setting.max_value)
                            .suffix(&setting.unit)
                            .text(tr(lang, "power.dc_value")),
                    );
                }

                if ac_new != ac_current {
                    edit_ac = Some((setting.id.clone(), setting.clamp(ac_new)));
                }
                if dc_new != dc_current {
                    edit_dc = Some((setting.id.clone(), setting.clamp(dc_new)));
                }

                let dirty = ac_new != ac_current || dc_new != dc_current;

                let apply_btn = Button::new(tr(lang, "common.apply"))
                    .fill(if dirty {
                        colors.accent
                    } else {
                        colors.bg_card_hover
                    })
                    .stroke(Stroke::new(1.0_f32, colors.border))
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(84.0, 24.0));
                if ui.add_enabled(dirty, apply_btn).clicked() {
                    stage_setting = Some((setting.clone(), ac_new, dc_new));
                }
            });

            ui.add_space(4.0);
            let (line_rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
            ui.painter().rect_filled(
                line_rect,
                0.0,
                Color32::from_rgba_unmultiplied(49, 53, 61, 100),
            );
            ui.add_space(4.0);
        }

        ui.add_space(4.0);
        ui.label(
            RichText::new(tr(lang, "power.reboot_note"))
                .size(11.0)
                .color(colors.text_muted),
        );
        ui.add_space(2.0);
        ui.label(
            RichText::new(tr(lang, "power.plan_ac_only"))
                .size(11.0)
                .color(colors.text_muted),
        );
    });

    // ---- Apply collected edits ----
    if let Some((id, value)) = edit_ac {
        state.power_edit_ac.insert(id, value);
    }
    if let Some((id, value)) = edit_dc {
        state.power_edit_dc.insert(id, value);
    }

    if let Some((plan_id, label)) = stage_plan {
        state.request_confirm(PendingConfirm::SetPowerPlan { plan_id, label });
    }

    if let Some((setting, ac, dc)) = stage_setting {
        state.request_confirm(PendingConfirm::SetPowerSetting {
            setting: Box::new(setting),
            ac,
            dc,
        });
    }

    if create_ultimate {
        // Creating the scheme does not activate it, so this is safe to run
        // directly; switching to it remains a separate confirmed step.
        match crate::power::manager::ensure_ultimate_performance() {
            Ok(msg) => {
                state.set_toast(&msg);
                state.refresh_power();
            }
            Err(e) => state.record_event("Power", &e, false),
        }
    }
}
