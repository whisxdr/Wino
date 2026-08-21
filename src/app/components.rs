use crate::app::theme::get_colors;
use eframe::egui::{self, Color32, Frame, Margin, Pos2, Rect, RichText, Rounding, Stroke, Ui, Vec2};

/// Contextual Section Header matching Wino Pro top title & live badge
pub fn section_header(ui: &mut Ui, title: &str, subtitle: &str) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).size(22.0).strong().color(colors.text_primary));
            ui.add_space(4.0);
            status_badge(ui, "LIVE", (78, 222, 163));
        });
        if !subtitle.is_empty() {
            ui.add_space(2.0);
            ui.label(RichText::new(subtitle).size(12.5).color(colors.text_secondary));
        }
        ui.add_space(8.0);
    });
}

/// Bento Grid Glass Card Container
pub fn bento_card<R>(
    ui: &mut Ui,
    title: &str,
    icon: &str,
    badge: Option<&str>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let colors = get_colors(ui.visuals().dark_mode);

    Frame::none()
        .fill(colors.bg_card)
        .stroke(Stroke::new(1.0_f32, colors.border))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::same(16.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if !icon.is_empty() {
                    ui.label(RichText::new(icon).size(16.0).color(colors.accent).strong());
                    ui.add_space(2.0);
                }
                ui.label(RichText::new(title).size(15.0).strong().color(colors.text_primary));

                if let Some(b) = badge {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_badge(ui, b, (60, 144, 255));
                    });
                }
            });
            ui.add_space(8.0);
            add_contents(ui)
        })
        .inner
}

/// Generic Card Container
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

/// Standardized Metric Card
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
            ui.set_min_size(Vec2::new(130.0, 75.0));
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).size(11.0).color(colors.text_secondary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 3.5, accent_color);
                    });
                });
                ui.add_space(2.0);
                ui.label(RichText::new(value).size(19.0).strong().color(colors.text_primary));
                ui.add_space(1.0);
                ui.label(RichText::new(subtext).size(10.5).color(colors.text_muted));
            });
        });
}

/// Monospace pill status badge
pub fn status_badge(ui: &mut Ui, text: &str, rgb: (u8, u8, u8)) {
    let color = Color32::from_rgb(rgb.0, rgb.1, rgb.2);
    let bg_color = Color32::from_rgba_unmultiplied(rgb.0, rgb.1, rgb.2, 25);

    Frame::none()
        .fill(bg_color)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(rgb.0, rgb.1, rgb.2, 80)))
        .rounding(Rounding::same(4.0))
        .inner_margin(Margin::symmetric(8.0, 2.5))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(10.5).color(color).strong());
        });
}

/// Key-Value Stat Row with subtle border divider
pub fn stat_row(ui: &mut Ui, label: &str, value: &str, is_highlight: bool) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.5).color(colors.text_muted));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if is_highlight {
                ui.label(RichText::new(value).size(12.5).strong().color(colors.accent));
            } else {
                ui.label(RichText::new(value).size(12.5).strong().color(colors.text_primary));
            }
        });
    });
    ui.add_space(2.0);
    let (line_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(line_rect, 0.0, Color32::from_rgba_unmultiplied(49, 53, 61, 100));
    ui.add_space(4.0);
}

/// Progress Track Row
pub fn progress_track_row(ui: &mut Ui, label: &str, value_str: &str, fraction: f32, color: Color32) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.0).color(colors.text_secondary));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value_str).size(11.0).color(colors.text_muted));
        });
    });
    ui.add_space(2.0);

    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(3.0), colors.bg_card_highest);

    let clamped = fraction.clamp(0.0, 1.0);
    let fill_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width() * clamped, rect.height()));
    painter.rect_filled(fill_rect, Rounding::same(3.0), color);
    ui.add_space(6.0);
}

/// Live Glowing Histogram Chart (Faithful to CPU chart in Wino Pro HTML)
pub fn live_histogram(ui: &mut Ui, samples: &[f32], max_val: f32, height: f32) {
    let colors = get_colors(ui.visuals().dark_mode);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), egui::Sense::hover());
    let painter = ui.painter();

    // Chart background container
    painter.rect_filled(rect, Rounding::same(6.0), Color32::from_rgb(11, 14, 21));
    painter.rect_stroke(rect, Rounding::same(6.0), Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(49, 53, 61, 80)));

    // Grid lines (horizontal)
    for i in 1..4 {
        let y = rect.min.y + (rect.height() / 4.0) * (i as f32);
        painter.line_segment(
            [Pos2::new(rect.min.x + 8.0, y), Pos2::new(rect.max.x - 8.0, y)],
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(49, 53, 61, 60)),
        );
    }

    if samples.is_empty() {
        return;
    }

    let count = samples.len();
    let gap = 3.0_f32;
    let available_w = rect.width() - 16.0;
    let bar_w = ((available_w - (gap * (count as f32 - 1.0))) / (count as f32)).max(2.0);

    for (i, &sample) in samples.iter().enumerate() {
        let norm = (sample / max_val).clamp(0.05, 1.0);
        let bar_h = (rect.height() - 20.0) * norm;
        let x = rect.min.x + 8.0 + (i as f32) * (bar_w + gap);
        let y = rect.max.y - 8.0 - bar_h;

        let bar_rect = Rect::from_min_max(Pos2::new(x, y), Pos2::new(x + bar_w, rect.max.y - 8.0));
        
        // Gradient color intensity based on recentness and value
        let alpha = ((i as f32 / count as f32) * 180.0 + 75.0) as u8;
        let bar_color = Color32::from_rgba_unmultiplied(colors.accent.r(), colors.accent.g(), colors.accent.b(), alpha);

        painter.rect_filled(bar_rect, Rounding { nw: 2.0, ne: 2.0, sw: 0.0, se: 0.0 }, bar_color);
    }
}
