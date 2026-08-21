use crate::app::theme::get_colors;
use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Stroke, Ui, Vec2};

pub fn section_header(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.vertical(|ui| {
        ui.label(RichText::new(title).size(20.0).strong());
        if !subtitle.is_empty() {
            ui.add_space(2.0);
            let colors = get_colors(ui.visuals().dark_mode);
            ui.label(RichText::new(subtitle).size(12.0).color(colors.text_muted));
        }
        ui.add_space(8.0);
    });
}

pub fn metric_card(
    ui: &mut Ui,
    title: &str,
    value: &str,
    subtext: &str,
    accent_color: Color32,
) {
    let colors = get_colors(ui.visuals().dark_mode);

    Frame::none()
        .fill(colors.bg_card)
        .stroke(Stroke::new(1.0_f32, colors.border))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(130.0, 80.0));
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).size(11.0).color(colors.text_secondary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 3.5, accent_color);
                    });
                });
                ui.add_space(2.0);
                ui.label(RichText::new(value).size(18.0).strong());
                ui.add_space(1.0);
                ui.label(RichText::new(subtext).size(10.5).color(colors.text_muted));
            });
        });
}

pub fn status_badge(ui: &mut Ui, text: &str, rgb: (u8, u8, u8)) {
    let color = Color32::from_rgb(rgb.0, rgb.1, rgb.2);
    let bg_color = Color32::from_rgba_unmultiplied(rgb.0, rgb.1, rgb.2, 30);

    Frame::none()
        .fill(bg_color)
        .stroke(Stroke::new(1.0_f32, color))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::symmetric(8.0, 3.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(color).strong());
        });
}

pub fn card_container<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let colors = get_colors(ui.visuals().dark_mode);
    Frame::none()
        .fill(colors.bg_card)
        .stroke(Stroke::new(1.0_f32, colors.border))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(14.0))
        .show(ui, add_contents)
        .inner
}
