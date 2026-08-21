use crate::app::components::{card_container, section_header, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::debloat::executor::is_rule_in_preset;
use eframe::egui::{self, Button, Color32, Frame, Margin, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.horizontal(|ui| {
        section_header(
            ui,
            "Windows Debloater & Optimizer",
            "Eliminate preinstalled bloat, remove advertising tracking, and optimize background services safely",
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("↻ Rescan Targets").clicked() {
                state.refresh_debloat();
            }
        });
    });

    // 1. Preset Selector & Rule Statistics
    let safe_count = state.debloat_items.iter().filter(|i| i.rule.preset == "Safe").count();
    let balanced_count = state.debloat_items.iter().filter(|i| i.rule.preset == "Safe" || i.rule.preset == "Balanced").count();
    let aggressive_count = state.debloat_items.len();

    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Optimization Preset:").strong());

            // Safe Preset button
            let is_safe = state.debloat_preset == "Safe";
            let safe_btn = Button::new(RichText::new(format!("Safe ({} rules)", safe_count)).color(if is_safe { Color32::WHITE } else { colors.text_primary }))
                .fill(if is_safe { Color32::from_rgb(34, 197, 94) } else { colors.bg_card_hover })
                .rounding(Rounding::same(6.0));
            if ui.add(safe_btn).clicked() {
                state.debloat_preset = "Safe".to_string();
                state.debloat_show_confirm = false;
            }

            // Balanced Preset button
            let is_balanced = state.debloat_preset == "Balanced";
            let balanced_btn = Button::new(RichText::new(format!("Balanced ({} rules)", balanced_count)).color(if is_balanced { Color32::WHITE } else { colors.text_primary }))
                .fill(if is_balanced { Color32::from_rgb(234, 179, 8) } else { colors.bg_card_hover })
                .rounding(Rounding::same(6.0));
            if ui.add(balanced_btn).clicked() {
                state.debloat_preset = "Balanced".to_string();
                state.debloat_show_confirm = false;
            }

            // Aggressive Preset button
            let is_agg = state.debloat_preset == "Aggressive";
            let agg_btn = Button::new(RichText::new(format!("Aggressive ({} rules)", aggressive_count)).color(if is_agg { Color32::WHITE } else { colors.text_primary }))
                .fill(if is_agg { Color32::from_rgb(239, 68, 68) } else { colors.bg_card_hover })
                .rounding(Rounding::same(6.0));
            if ui.add(agg_btn).clicked() {
                state.debloat_preset = "Aggressive".to_string();
                state.debloat_show_confirm = false;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let apply_btn = Button::new(RichText::new(format!("Apply '{}' Preset", state.debloat_preset)).strong().color(Color32::WHITE))
                    .fill(match state.debloat_preset.as_str() {
                        "Balanced" => Color32::from_rgb(202, 138, 4),
                        "Aggressive" => Color32::from_rgb(220, 38, 38),
                        _ => colors.accent,
                    })
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(140.0, 30.0));

                if ui.add(apply_btn).clicked() {
                    if state.debloat_preset == "Safe" {
                        // Safe preset executes directly with snapshot
                        let results = crate::debloat::executor::apply_debloat_preset("Safe", false);
                        state.set_toast(&format!("Applied Safe preset ({} optimizations executed).", results.len()));
                        state.refresh_debloat();
                    } else {
                        // Open confirmation attention modal for Balanced or Aggressive
                        state.debloat_confirm_preset = state.debloat_preset.clone();
                        state.debloat_show_confirm = true;
                    }
                }

                let dry_btn = Button::new("Dry-Run Test")
                    .fill(colors.bg_card_hover)
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(95.0, 30.0));

                if ui.add(dry_btn).clicked() {
                    let results = crate::debloat::executor::apply_debloat_preset(&state.debloat_preset, true);
                    state.set_toast(&format!("Dry run: {} actions would be performed under '{}'.", results.len(), state.debloat_preset));
                }
            });
        });
    });

    ui.add_space(8.0);

    // 2. Preset Attention & Description Banner
    let (banner_bg, banner_border, banner_icon, banner_title, banner_desc) = match state.debloat_preset.as_str() {
        "Balanced" => (
            Color32::from_rgba_unmultiplied(234, 179, 8, 20),
            Color32::from_rgb(234, 179, 8),
            "⚡",
            "Balanced Preset Attention — Workstation & Productivity Profile",
            "Includes all Safe optimizations PLUS disables Windows 11 Widgets (recovers ~200 MB RAM), Copilot side panel, Microsoft Edge startup background prelaunch, and idle Xbox Live background services (for non-gamers).",
        ),
        "Aggressive" => (
            Color32::from_rgba_unmultiplied(239, 68, 68, 20),
            Color32::from_rgb(239, 68, 68),
            "⚠️",
            "Aggressive Preset Attention & Caution — Deep Debloat Profile",
            "Includes Safe + Balanced rules PLUS deeply uninstalls OEM factory promotional stubs (TikTok, Spotify, Disney, Prime Video), disables background location sensors, activity history tracking, and feedback survey prompts. A restore snapshot is always created automatically.",
        ),
        _ => (
            Color32::from_rgba_unmultiplied(34, 197, 94, 20),
            Color32::from_rgb(34, 197, 94),
            "🛡",
            "Safe Preset — Recommended For All Users (Zero Risk)",
            "Removes promotional UWP app stubs (Candy Crush, Feedback Hub, Solitaire, Tips), disables Bing web queries in Start Menu, and disables advertising tracking ID. 100% reversible with zero feature breakage.",
        ),
    };

    Frame::none()
        .fill(banner_bg)
        .stroke(Stroke::new(1.0_f32, banner_border))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(banner_icon).size(18.0).color(banner_border));
                ui.vertical(|ui| {
                    ui.label(RichText::new(banner_title).size(13.5).strong().color(banner_border));
                    ui.add_space(1.0);
                    ui.label(RichText::new(banner_desc).size(11.5).color(colors.text_secondary));
                });
            });
        });

    ui.add_space(8.0);

    // 3. Attention & Confirmation Box (when Balanced / Aggressive Apply is requested)
    if state.debloat_show_confirm {
        let confirm_preset = state.debloat_confirm_preset.clone();
        let applicable_count = state.debloat_items.iter().filter(|i| !i.is_applied && is_rule_in_preset(&i.rule.preset, &confirm_preset)).count();

        Frame::none()
            .fill(if confirm_preset == "Aggressive" { Color32::from_rgba_unmultiplied(239, 68, 68, 30) } else { Color32::from_rgba_unmultiplied(234, 179, 8, 30) })
            .stroke(Stroke::new(1.5_f32, if confirm_preset == "Aggressive" { Color32::from_rgb(239, 68, 68) } else { Color32::from_rgb(234, 179, 8) }))
            .rounding(Rounding::same(10.0))
            .inner_margin(Margin::same(14.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚠️ Confirmation Required:").size(15.0).strong().color(if confirm_preset == "Aggressive" { Color32::from_rgb(239, 68, 68) } else { Color32::from_rgb(234, 179, 8) }));
                        ui.label(RichText::new(format!("Apply '{}' Preset ({} pending optimizations)", confirm_preset, applicable_count)).size(14.0).strong());
                    });

                    ui.add_space(4.0);
                    ui.label(RichText::new(format!(
                        "You are about to apply the {} debloat preset. A safety configuration snapshot will be created automatically before any changes are applied.",
                        confirm_preset
                    )).size(12.0).color(colors.text_primary));

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let confirm_btn = Button::new(RichText::new("✓ Yes, Create Snapshot & Apply").strong().color(Color32::WHITE))
                            .fill(if confirm_preset == "Aggressive" { Color32::from_rgb(220, 38, 38) } else { Color32::from_rgb(202, 138, 4) })
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(200.0, 32.0));

                        if ui.add(confirm_btn).clicked() {
                            let results = crate::debloat::executor::apply_debloat_preset(&confirm_preset, false);
                            state.set_toast(&format!("Successfully applied {} preset ({} optimizations executed).", confirm_preset, results.len()));
                            state.debloat_show_confirm = false;
                            state.refresh_debloat();
                        }

                        ui.add_space(8.0);

                        let cancel_btn = Button::new(RichText::new("✕ Cancel / Stay on Safe").color(colors.text_muted))
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

    // 4. Filter & Search Controls
    ui.horizontal(|ui| {
        ui.label("Filter View:");
        for f in &["All", "Safe", "Balanced", "Aggressive"] {
            let is_sel = state.debloat_filter_preset == *f;
            let btn = Button::new(*f)
                .fill(if is_sel { colors.accent } else { colors.bg_card })
                .rounding(Rounding::same(5.0));
            if ui.add(btn).clicked() {
                state.debloat_filter_preset = f.to_string();
            }
        }

        ui.add_space(12.0);
        ui.label("Search:");
        ui.text_edit_singleline(&mut state.search_query);
    });

    ui.add_space(8.0);

    // 5. Scanned Debloat Item Cards
    let mut apply_item_id: Option<String> = None;
    let search = state.search_query.to_lowercase();
    let filter = state.debloat_filter_preset.clone();

    for item in &state.debloat_items {
        // Apply filter preset and search query
        if filter != "All" && item.rule.preset != filter {
            continue;
        }

        if !search.is_empty() && !item.rule.name.to_lowercase().contains(&search) && !item.rule.description.to_lowercase().contains(&search) {
            continue;
        }

        let (preset_rgb, preset_label) = match item.rule.preset.as_str() {
            "Balanced" => ((234, 179, 8), "Balanced"),
            "Aggressive" => ((239, 68, 68), "Aggressive"),
            _ => ((34, 197, 94), "Safe"),
        };

        card_container(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&item.rule.name).size(14.0).strong());
                        status_badge(ui, preset_label, preset_rgb);
                        status_badge(ui, &item.rule.category, (100, 116, 139));
                    });
                    ui.add_space(2.0);
                    ui.label(RichText::new(&item.rule.description).size(12.0).color(colors.text_secondary));
                    ui.add_space(2.0);
                    ui.label(RichText::new(format!("Benefit: {}", item.rule.estimated_benefit)).size(10.5).color(colors.text_muted));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if item.is_applied {
                        ui.label(RichText::new("✓ Optimized").color(colors.success).strong());
                    } else {
                        let btn = Button::new(RichText::new("Apply").color(Color32::WHITE))
                            .fill(colors.accent)
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(70.0, 28.0));
                        if ui.add(btn).clicked() {
                            apply_item_id = Some(item.rule.id.clone());
                        }
                    }
                });
            });
        });
        ui.add_space(4.0);
    }

    if let Some(id) = apply_item_id {
        if let Some(it) = state.debloat_items.iter().find(|i| i.rule.id == id) {
            let res = crate::debloat::executor::apply_debloat_rule(&it.rule, false);
            state.set_toast(&format!("Applied: {} ({} steps executed)", it.rule.name, res.len()));
            state.refresh_debloat();
        }
    }
}
