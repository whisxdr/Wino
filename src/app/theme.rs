use eframe::egui::{self, Color32, Margin, Rounding, Stroke, Vec2, Visuals};

pub struct FluentColors {
    pub bg_canvas: Color32,
    pub bg_panel: Color32,
    pub bg_footer: Color32,
    pub bg_card: Color32,
    pub bg_card_hover: Color32,
    pub bg_card_highest: Color32,
    pub border: Color32,
    pub border_subtle: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_glow: Color32,
    pub secondary: Color32,
    pub secondary_container: Color32,
    pub tertiary: Color32,
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
            bg_canvas: Color32::from_rgb(11, 16, 21), // #0B1015 (Deep glass canvas)
            bg_panel: Color32::from_rgb(11, 14, 21),  // #0B0E15 (Sidebar)
            bg_footer: Color32::from_rgb(24, 28, 35), // #181C23 (Bottom status bar)
            bg_card: Color32::from_rgb(28, 32, 39),   // #1C2027 (Bento glass cards)
            bg_card_hover: Color32::from_rgb(38, 42, 50), // #262A32 (Hover/High)
            bg_card_highest: Color32::from_rgb(49, 53, 61), // #31353D (Highest container)
            border: Color32::from_rgb(49, 53, 61),    // #31353D (1px crisp border)
            border_subtle: Color32::from_rgba_unmultiplied(255, 255, 255, 12),
            accent: Color32::from_rgb(60, 144, 255), // #3C90FF (Vibrant primary blue)
            accent_hover: Color32::from_rgb(100, 170, 255), // #64AAFF
            accent_glow: Color32::from_rgba_unmultiplied(60, 144, 255, 45),
            secondary: Color32::from_rgb(78, 222, 163), // #4EDEA3 (Emerald green)
            secondary_container: Color32::from_rgba_unmultiplied(78, 222, 163, 30),
            tertiary: Color32::from_rgb(255, 185, 95), // #FFB95F (Amber gold)
            text_primary: Color32::from_rgb(224, 226, 237), // #E0E2ED (Primary on-surface)
            text_secondary: Color32::from_rgb(192, 198, 214), // #C0C6D6
            text_muted: Color32::from_rgb(138, 145, 160), // #8A91A0 (Monospace/labels)
            success: Color32::from_rgb(78, 222, 163),  // #4EDEA3
            warning: Color32::from_rgb(255, 185, 95),  // #FFB95F
            danger: Color32::from_rgb(239, 68, 68),    // #EF4444
        }
    } else {
        FluentColors {
            bg_canvas: Color32::from_rgb(243, 244, 246),
            bg_panel: Color32::from_rgb(235, 238, 242),
            bg_footer: Color32::from_rgb(240, 242, 245),
            bg_card: Color32::from_rgb(255, 255, 255),
            bg_card_hover: Color32::from_rgb(248, 250, 252),
            bg_card_highest: Color32::from_rgb(226, 232, 240),
            border: Color32::from_rgb(226, 232, 240),
            border_subtle: Color32::from_rgba_unmultiplied(0, 0, 0, 15),
            accent: Color32::from_rgb(0, 120, 212),
            accent_hover: Color32::from_rgb(20, 140, 235),
            accent_glow: Color32::from_rgba_unmultiplied(0, 120, 212, 35),
            secondary: Color32::from_rgb(22, 163, 74),
            secondary_container: Color32::from_rgba_unmultiplied(22, 163, 74, 30),
            tertiary: Color32::from_rgb(202, 138, 4),
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
        fonts
            .font_data
            .insert("segoe_ui".to_owned(), egui::FontData::from_owned(font_data));
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            family.insert(0, "segoe_ui".to_owned());
        }
    }

    // 2. Windows Segoe UI Symbol Font (Rich Fluent Glyphs & UI Symbols)
    if let Ok(sym_data) = std::fs::read("C:\\Windows\\Fonts\\seguisym.ttf") {
        fonts
            .font_data
            .insert("segoe_sym".to_owned(), egui::FontData::from_owned(sym_data));
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

    let mut visuals = if is_dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    visuals.override_text_color = Some(colors.text_primary);
    visuals.panel_fill = colors.bg_canvas;
    visuals.window_fill = colors.bg_canvas;
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
