use crate::app::components::{card_container, section_header, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::core::i18n::{tr, Lang};
use eframe::egui::{Button, Color32, RichText, Rounding, Slider, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    section_header(ui, tr(lang, "set.title"), tr(lang, "set.subtitle"));

    // 1. Appearance & Localization Card (100% Full Width)
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🎨").size(15.0).color(colors.accent));
            ui.label(RichText::new(tr(lang, "set.appearance")).size(15.0).strong());
        });
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new(tr(lang, "set.theme_label")).strong());
            ui.add_space(8.0);

            let is_dark = state.config.general.theme == "dark";
            let dark_btn = Button::new(RichText::new("🌙 Dark").color(if is_dark { Color32::WHITE } else { colors.text_secondary }))
                .fill(if is_dark { colors.accent } else { colors.bg_card_hover })
                .stroke(Stroke::new(1.0_f32, if is_dark { colors.accent } else { colors.border }))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(80.0, 26.0));
            if ui.add(dark_btn).clicked() {
                state.config.general.theme = "dark".to_string();
                let _ = state.config.save();
            }

            let is_light = state.config.general.theme == "light";
            let light_btn = Button::new(RichText::new("☀️ Light").color(if is_light { Color32::WHITE } else { colors.text_secondary }))
                .fill(if is_light { colors.accent } else { colors.bg_card_hover })
                .stroke(Stroke::new(1.0_f32, if is_light { colors.accent } else { colors.border }))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(80.0, 26.0));
            if ui.add(light_btn).clicked() {
                state.config.general.theme = "light".to_string();
                let _ = state.config.save();
            }
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new(tr(lang, "set.language_label")).strong());
            ui.add_space(8.0);

            let is_en = state.lang == Lang::En;
            let en_btn = Button::new(RichText::new("English 🇺🇸").color(if is_en { Color32::WHITE } else { colors.text_secondary }))
                .fill(if is_en { colors.accent } else { colors.bg_card_hover })
                .stroke(Stroke::new(1.0_f32, if is_en { colors.accent } else { colors.border }))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(110.0, 26.0));
            if ui.add(en_btn).clicked() {
                state.set_language(Lang::En);
            }

            let is_id = state.lang == Lang::Id;
            let id_btn = Button::new(RichText::new("Bahasa Indonesia 🇮🇩").color(if is_id { Color32::WHITE } else { colors.text_secondary }))
                .fill(if is_id { colors.accent } else { colors.bg_card_hover })
                .stroke(Stroke::new(1.0_f32, if is_id { colors.accent } else { colors.border }))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(150.0, 26.0));
            if ui.add(id_btn).clicked() {
                state.set_language(Lang::Id);
            }
        });

        ui.add_space(4.0);
        ui.label(RichText::new(tr(lang, "set.language_hint")).size(11.0).color(colors.text_muted));
    });

    ui.add_space(10.0);

    // 2. Memory Auto-Trim Configuration Card (100% Full Width)
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("⚡").size(15.0).color(colors.secondary));
            ui.label(RichText::new(tr(lang, "set.auto_trim_section")).size(15.0).strong());
        });
        ui.add_space(6.0);

        let mut enabled = state.config.memory.auto_trim_enabled;
        if ui.checkbox(&mut enabled, tr(lang, "set.auto_trim_enable")).changed() {
            state.config.memory.auto_trim_enabled = enabled;
            state.apply_auto_trim_settings();
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr(lang, "set.auto_trim_threshold")).size(12.0).color(colors.text_secondary));
            let mut pct = state.config.memory.auto_trim_threshold_pct;
            let slider = Slider::new(&mut pct, 60.0..=95.0).suffix("%").clamping(egui::SliderClamping::Always);
            if ui.add(slider).changed() {
                state.config.memory.auto_trim_threshold_pct = pct;
                state.apply_auto_trim_settings();
            }
        });

        ui.add_space(2.0);
        ui.label(RichText::new(tr(lang, "set.auto_trim_cooldown")).size(11.0).color(colors.text_muted));
    });

    ui.add_space(10.0);

    // 3. User & Administrator Privileges Card (100% Full Width)
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🛡").size(15.0).color(colors.tertiary));
            ui.label(RichText::new(tr(lang, "set.privileges")).size(15.0).strong());
        });
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.label(tr(lang, "set.current_status"));
            if state.sys_info.is_admin {
                status_badge(ui, tr(lang, "common.admin_elevated"), (34, 197, 94));
            } else {
                status_badge(ui, tr(lang, "common.standard_user"), (234, 179, 8));
            }
        });

        if !state.sys_info.is_admin {
            ui.add_space(8.0);
            ui.label(RichText::new(tr(lang, "set.elevate_note")).size(12.0).color(colors.text_muted));
            ui.add_space(6.0);
            let elevate_btn = Button::new(RichText::new(format!("⚡ {}", tr(lang, "set.restart_admin"))).strong().color(Color32::WHITE))
                .fill(colors.accent)
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(180.0, 30.0));

            if ui.add(elevate_btn).clicked() {
                let _ = crate::core::permissions::restart_elevated();
            }
        }
    });

    ui.add_space(10.0);

    // 4. About & Version Info Card (100% Full Width)
    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("ℹ").size(15.0).color(colors.accent));
            ui.label(RichText::new(tr(lang, "set.about")).size(15.0).strong());
        });
        ui.add_space(4.0);
        ui.label(RichText::new("Wino — Rust-Native Windows Debloater, Optimizer & Memory Suite").strong());
        ui.label(RichText::new(tr(lang, "set.about_version")).size(11.0).color(colors.text_muted));
        ui.add_space(2.0);
        ui.label(RichText::new(tr(lang, "set.license_line")).size(11.0).color(colors.text_muted));
    });
}

