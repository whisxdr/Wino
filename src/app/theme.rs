use eframe::egui::{self, Color32, Margin, Rounding, Stroke, Vec2, Visuals};

pub struct FluentColors {
    pub bg_panel: Color32,
    pub bg_card: Color32,
    pub bg_card_hover: Color32,
    pub border: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
}

pub fn get_colors(is_dark: bool) -> FluentColors {
    if is_dark {
        FluentColors {
            bg_panel: Color32::from_rgb(20, 22, 27),
            bg_card: Color32::from_rgb(30, 33, 40),
            bg_card_hover: Color32::from_rgb(38, 42, 51),
            border: Color32::from_rgb(45, 50, 60),
            accent: Color32::from_rgb(0, 120, 212),
            accent_hover: Color32::from_rgb(20, 140, 235),
            text_primary: Color32::from_rgb(240, 243, 246),
            text_secondary: Color32::from_rgb(180, 188, 200),
            text_muted: Color32::from_rgb(120, 130, 145),
            success: Color32::from_rgb(34, 197, 94),
            warning: Color32::from_rgb(234, 179, 8),
            danger: Color32::from_rgb(239, 68, 68),
        }
    } else {
        FluentColors {
            bg_panel: Color32::from_rgb(243, 244, 246),
            bg_card: Color32::from_rgb(255, 255, 255),
            bg_card_hover: Color32::from_rgb(248, 250, 252),
            border: Color32::from_rgb(226, 232, 240),
            accent: Color32::from_rgb(0, 120, 212),
            accent_hover: Color32::from_rgb(20, 140, 235),
            text_primary: Color32::from_rgb(15, 23, 42),
            text_secondary: Color32::from_rgb(71, 85, 105),
            text_muted: Color32::from_rgb(148, 163, 184),
            success: Color32::from_rgb(22, 163, 74),
            warning: Color32::from_rgb(202, 138, 4),
            danger: Color32::from_rgb(220, 38, 38),
        }
    }
}

pub fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 1. Windows Native Segoe UI Font
    if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\segoeui.ttf") {
        fonts.font_data.insert(
            "segoe_ui".to_owned(),
            egui::FontData::from_owned(font_data),
        );
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            family.insert(0, "segoe_ui".to_owned());
        }
    }

    // 2. Windows Segoe UI Symbol Font (Rich Fluent Glyphs & UI Symbols)
    if let Ok(sym_data) = std::fs::read("C:\\Windows\\Fonts\\seguisym.ttf") {
        fonts.font_data.insert(
            "segoe_sym".to_owned(),
            egui::FontData::from_owned(sym_data),
        );
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            family.push("segoe_sym".to_owned());
        }
    }

    // 3. Windows Segoe MDL2 Assets Font
    if let Ok(mdl_data) = std::fs::read("C:\\Windows\\Fonts\\segmdl2.ttf") {
        fonts.font_data.insert(
            "segoe_mdl2".to_owned(),
            egui::FontData::from_owned(mdl_data),
        );
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            family.push("segoe_mdl2".to_owned());
        }
    }

    ctx.set_fonts(fonts);
}

pub fn apply_theme(ctx: &egui::Context, theme_name: &str) {
    let is_dark = theme_name != "light";
    let colors = get_colors(is_dark);

    let mut visuals = if is_dark { Visuals::dark() } else { Visuals::light() };

    visuals.override_text_color = Some(colors.text_primary);
    visuals.panel_fill = colors.bg_panel;
    visuals.window_fill = colors.bg_panel;
    visuals.window_stroke = Stroke::new(1.0_f32, colors.border);
    visuals.window_rounding = Rounding::same(10.0);

    visuals.widgets.noninteractive.bg_fill = colors.bg_card;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, colors.border);
    visuals.widgets.noninteractive.rounding = Rounding::same(8.0);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, colors.text_primary);

    visuals.widgets.inactive.bg_fill = colors.bg_card;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, colors.border);
    visuals.widgets.inactive.rounding = Rounding::same(8.0);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, colors.text_primary);

    visuals.widgets.hovered.bg_fill = colors.bg_card_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, colors.accent);
    visuals.widgets.hovered.rounding = Rounding::same(8.0);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, colors.accent_hover);

    visuals.widgets.active.bg_fill = colors.accent;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, colors.accent);
    visuals.widgets.active.rounding = Rounding::same(8.0);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.button_padding = Vec2::new(14.0, 8.0);
    style.spacing.window_margin = Margin::same(16.0);
    ctx.set_style(style);
}
