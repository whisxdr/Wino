use crate::app::components::{
    action_card, card_container, preview_group, status_badge, view_header, PreviewGroup,
};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::app::worker::Action;
use crate::core::i18n::tr;
use crate::debloat::executor::is_rule_in_preset;
use eframe::egui::{
    Button, Color32, Frame, Margin, RichText, Rounding, Stroke, TextEdit, Ui, Vec2,
};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    // 1. Contextual Header with Rescan Button
    view_header(ui, tr(lang, "deb.title"), tr(lang, "deb.subtitle"), |ui| {
        let rescan_btn = Button::new(format!("↻ {}", tr(lang, "common.rescan")))
            .fill(colors.bg_card)
            .stroke(Stroke::new(1.0_f32, colors.border))
            .rounding(Rounding::same(6.0));
        if ui.add(rescan_btn).clicked() {
            state.refresh_debloat();
        }
    });

    // 2. Preset Selector & Rule Statistics
    let safe_count = state
        .debloat_items
        .iter()
        .filter(|i| i.rule.preset == "Safe")
        .count();
    let balanced_count = state
        .debloat_items
        .iter()
        .filter(|i| i.rule.preset == "Safe" || i.rule.preset == "Balanced")
        .count();
    let aggressive_count = state.debloat_items.len();

    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr(lang, "deb.preset_label")).strong());

            let is_safe = state.debloat_preset == "Safe";
            let safe_btn = Button::new(
                RichText::new(format!(
                    "{} ({} {})",
                    tr(lang, "deb.safe_preset"),
                    safe_count,
                    tr(lang, "deb.rules_suffix")
                ))
                .color(if is_safe {
                    Color32::WHITE
                } else {
                    colors.text_primary
                }),
            )
            .fill(if is_safe {
                Color32::from_rgb(34, 197, 94)
            } else {
                colors.bg_card_hover
            })
            .rounding(Rounding::same(6.0));
            if ui.add(safe_btn).clicked() {
                state.debloat_preset = "Safe".to_string();
                state.debloat_show_confirm = false;
            }

            let is_balanced = state.debloat_preset == "Balanced";
            let balanced_btn = Button::new(
                RichText::new(format!(
                    "{} ({} {})",
                    tr(lang, "deb.balanced_preset"),
                    balanced_count,
                    tr(lang, "deb.rules_suffix")
                ))
                .color(if is_balanced {
                    Color32::WHITE
                } else {
                    colors.text_primary
                }),
            )
            .fill(if is_balanced {
                Color32::from_rgb(234, 179, 8)
            } else {
                colors.bg_card_hover
            })
            .rounding(Rounding::same(6.0));
            if ui.add(balanced_btn).clicked() {
                state.debloat_preset = "Balanced".to_string();
                state.debloat_show_confirm = false;
            }

            let is_agg = state.debloat_preset == "Aggressive";
            let agg_btn = Button::new(
                RichText::new(format!(
                    "{} ({} {})",
                    tr(lang, "deb.aggressive_preset"),
                    aggressive_count,
                    tr(lang, "deb.rules_suffix")
                ))
                .color(if is_agg {
                    Color32::WHITE
                } else {
                    colors.text_primary
                }),
            )
            .fill(if is_agg {
                Color32::from_rgb(239, 68, 68)
            } else {
                colors.bg_card_hover
            })
            .rounding(Rounding::same(6.0));
            if ui.add(agg_btn).clicked() {
                state.debloat_preset = "Aggressive".to_string();
                state.debloat_show_confirm = false;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let apply_btn = Button::new(
                    RichText::new(format!(
                        "{} '{}'",
                        tr(lang, "deb.apply_preset_btn"),
                        state.debloat_preset
                    ))
                    .strong()
                    .color(Color32::WHITE),
                )
                .fill(match state.debloat_preset.as_str() {
                    "Balanced" => Color32::from_rgb(202, 138, 4),
                    "Aggressive" => Color32::from_rgb(220, 38, 38),
                    _ => colors.accent,
                })
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(150.0, 30.0));

                if ui.add(apply_btn).clicked() {
                    state.debloat_confirm_preset = state.debloat_preset.clone();
                    state.debloat_show_confirm = true;
                }

                let dry_btn = Button::new(tr(lang, "common.dry_run_test"))
                    .fill(colors.bg_card_hover)
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(105.0, 30.0));

                if ui.add(dry_btn).clicked() {
                    let results =
                        crate::debloat::executor::apply_debloat_preset(&state.debloat_preset, true);
                    state.set_toast(&format!(
                        "Dry run: {} actions would be performed under '{}'.",
                        results.len(),
                        state.debloat_preset
                    ));
                }
            });
        });
    });

    ui.add_space(8.0);

    // 3. Preset Attention Banner (i18n)
    let (banner_bg, banner_border, banner_icon, banner_title, banner_desc) =
        match state.debloat_preset.as_str() {
            "Balanced" => (
                Color32::from_rgba_unmultiplied(234, 179, 8, 20),
                Color32::from_rgb(234, 179, 8),
                "⚡",
                tr(lang, "deb.balanced_banner_title"),
                tr(lang, "deb.balanced_banner_desc"),
            ),
            "Aggressive" => (
                Color32::from_rgba_unmultiplied(239, 68, 68, 20),
                Color32::from_rgb(239, 68, 68),
                "⚠️",
                tr(lang, "deb.aggressive_banner_title"),
                tr(lang, "deb.aggressive_banner_desc"),
            ),
            _ => (
                Color32::from_rgba_unmultiplied(34, 197, 94, 20),
                Color32::from_rgb(34, 197, 94),
                "🛡",
                tr(lang, "deb.safe_banner_title"),
                tr(lang, "deb.safe_banner_desc"),
            ),
        };

    Frame::none()
        .fill(banner_bg)
        .stroke(Stroke::new(1.0_f32, banner_border))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            let full_w = ui.available_width();
            ui.set_min_width(full_w);
            ui.horizontal(|ui| {
                ui.label(RichText::new(banner_icon).size(18.0).color(banner_border));
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(banner_title)
                            .size(13.5)
                            .strong()
                            .color(banner_border),
                    );
                    ui.add_space(1.0);
                    ui.label(
                        RichText::new(banner_desc)
                            .size(11.5)
                            .color(colors.text_secondary),
                    );
                });
            });
        });

    ui.add_space(8.0);

    // 4. Unified dry-run preview + confirmation modal (all presets)
    if state.debloat_show_confirm {
        let confirm_preset = state.debloat_confirm_preset.clone();
        let pending: Vec<_> = state
            .debloat_items
            .iter()
            .filter(|i| !i.is_applied && is_rule_in_preset(&i.rule.preset, &confirm_preset))
            .collect();
        let pending_count = pending.len();

        // Build preview groups
        let mut reg_items = Vec::new();
        let mut pkg_items = Vec::new();
        let mut svc_items = Vec::new();
        for item in &pending {
            for reg in &item.rule.registry_keys {
                reg_items.push(format!(
                    "[{}\\{}] {} = {}",
                    reg.hive, reg.path, reg.value_name, reg.value_data
                ));
            }
            pkg_items.extend(item.rule.package_names.iter().cloned());
            svc_items.extend(item.rule.services.iter().cloned());
        }

        let is_agg = confirm_preset == "Aggressive";
        let modal_fill = if is_agg {
            Color32::from_rgba_unmultiplied(239, 68, 68, 28)
        } else if confirm_preset == "Balanced" {
            Color32::from_rgba_unmultiplied(234, 179, 8, 28)
        } else {
            Color32::from_rgba_unmultiplied(34, 197, 94, 20)
        };
        let modal_border = if is_agg {
            Color32::from_rgb(239, 68, 68)
        } else if confirm_preset == "Balanced" {
            Color32::from_rgb(234, 179, 8)
        } else {
            Color32::from_rgb(34, 197, 94)
        };

        Frame::none()
            .fill(modal_fill)
            .stroke(Stroke::new(1.5_f32, modal_border))
            .rounding(Rounding::same(10.0))
            .inner_margin(Margin::same(14.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚠️").size(15.0).color(modal_border));
                        if pending_count == 0 {
                            ui.label(
                                RichText::new(tr(lang, "deb.no_pending"))
                                    .size(14.0)
                                    .strong(),
                            );
                        } else {
                            ui.label(
                                RichText::new(format!(
                                    "{}: '{}' ({} {})",
                                    tr(lang, "deb.confirm_required"),
                                    confirm_preset,
                                    pending_count,
                                    tr(lang, "deb.pending_changes")
                                ))
                                .size(14.0)
                                .strong(),
                            );
                        }
                    });

                    if pending_count > 0 {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(tr(lang, "deb.preview_desc"))
                                .size(11.5)
                                .color(colors.text_secondary),
                        );
                        ui.add_space(6.0);
                        // Preview groups inside scrollable container
                        egui::ScrollArea::vertical()
                            .max_height(220.0)
                            .show(ui, |ui| {
                                preview_group(
                                    ui,
                                    &PreviewGroup {
                                        title: tr(lang, "deb.registry_keys").to_string(),
                                        items: reg_items,
                                        color: (100, 116, 139),
                                    },
                                );
                                preview_group(
                                    ui,
                                    &PreviewGroup {
                                        title: tr(lang, "deb.packages").to_string(),
                                        items: pkg_items,
                                        color: (168, 85, 247),
                                    },
                                );
                                preview_group(
                                    ui,
                                    &PreviewGroup {
                                        title: tr(lang, "deb.services_label").to_string(),
                                        items: svc_items,
                                        color: (34, 197, 94),
                                    },
                                );
                            });

                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(tr(lang, "deb.vss_note"))
                                .size(11.0)
                                .color(colors.secondary),
                        );
                        if pending_count > 0 {
                            ui.label(
                                RichText::new(if is_agg || confirm_preset == "Balanced" {
                                    "Wino creates a VSS restore point before it changes anything."
                                } else {
                                    "Wino saves a local config snapshot before it changes anything."
                                })
                                .size(11.0)
                                .color(colors.text_muted),
                            );
                        }
                    }

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if pending_count > 0 {
                            let confirm_btn = Button::new(
                                RichText::new(tr(lang, "deb.confirm_apply_btn"))
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(modal_border)
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(210.0, 32.0));

                            if ui.add(confirm_btn).clicked() {
                                if state.action_busy {
                                    state.set_toast(
                                        "Another operation runs. Wait for it to finish.",
                                    );
                                } else {
                                    let preset = confirm_preset.clone();
                                    state.request_action(Action::ApplyDebloatPreset(preset));
                                    state.debloat_show_confirm = false;
                                }
                            }
                            ui.add_space(8.0);
                        }

                        let cancel_label = if pending_count == 0 {
                            tr(lang, "common.close")
                        } else {
                            tr(lang, "deb.cancel_safe_btn")
                        };
                        let cancel_btn =
                            Button::new(RichText::new(cancel_label).color(colors.text_muted))
                                .fill(colors.bg_card_hover)
                                .rounding(Rounding::same(6.0))
                                .min_size(Vec2::new(150.0, 32.0));

                        if ui.add(cancel_btn).clicked() {
                            state.debloat_show_confirm = false;
                        }
                    });
                });
            });

        ui.add_space(10.0);
    }

    // 5. Filter & Search Controls
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr(lang, "common.filter_view")).color(colors.text_secondary));
        for f in &["All", "Safe", "Balanced", "Aggressive"] {
            let is_sel = state.debloat_filter_preset == *f;
            let btn = Button::new(*f)
                .fill(if is_sel {
                    colors.accent
                } else {
                    colors.bg_card
                })
                .stroke(Stroke::new(
                    1.0_f32,
                    if is_sel { colors.accent } else { colors.border },
                ))
                .rounding(Rounding::same(5.0));
            if ui.add(btn).clicked() {
                state.debloat_filter_preset = f.to_string();
            }
        }

        ui.add_space(12.0);
        ui.label(RichText::new("🔍").size(11.0).color(colors.text_muted));
        let text_edit = TextEdit::singleline(&mut state.search_query)
            .hint_text(RichText::new(tr(lang, "common.search")).color(colors.text_muted))
            .desired_width(180.0);
        ui.add(text_edit);
        if !state.search_query.is_empty() && ui.small_button("✕").clicked() {
            state.search_query.clear();
        }
    });

    ui.add_space(8.0);

    // 6. Scanned Debloat Item Cards (Non-overlapping 2-column layout)
    let mut apply_item_rule: Option<crate::debloat::rules::DebloatRule> = None;
    let search = state.search_query.to_lowercase();
    let filter = state.debloat_filter_preset.clone();

    for item in &state.debloat_items {
        if filter != "All" && item.rule.preset != filter {
            continue;
        }
        if !search.is_empty()
            && !item.rule.name.to_lowercase().contains(&search)
            && !item.rule.description.to_lowercase().contains(&search)
        {
            continue;
        }

        let (preset_rgb, preset_label) = match item.rule.preset.as_str() {
            "Balanced" => ((234, 179, 8), tr(lang, "deb.balanced_preset")),
            "Aggressive" => ((239, 68, 68), tr(lang, "deb.aggressive_preset")),
            _ => ((34, 197, 94), tr(lang, "deb.safe_preset")),
        };

        action_card(
            ui,
            110.0,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&item.rule.name).size(14.0).strong());
                    status_badge(ui, preset_label, preset_rgb);
                    status_badge(ui, &item.rule.category, (100, 116, 139));
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new(&item.rule.description)
                        .size(12.0)
                        .color(colors.text_secondary),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        tr(lang, "common.benefit"),
                        item.rule.estimated_benefit
                    ))
                    .size(10.5)
                    .color(colors.text_muted),
                );
            },
            |ui| {
                if item.is_applied {
                    ui.label(
                        RichText::new(tr(lang, "common.optimized"))
                            .color(colors.success)
                            .strong(),
                    );
                } else {
                    let btn =
                        Button::new(RichText::new(tr(lang, "common.apply")).color(Color32::WHITE))
                            .fill(colors.accent)
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(75.0, 28.0));
                    if ui.add(btn).clicked() {
                        apply_item_rule = Some(item.rule.clone());
                    }
                }
            },
        );
        ui.add_space(4.0);
    }

    if let Some(rule) = apply_item_rule {
        if state.action_busy {
            state.set_toast("Another operation runs. Wait for it to finish.");
        } else {
            state.request_action(Action::ApplyDebloatRule(Box::new(rule)));
        }
    }
}
