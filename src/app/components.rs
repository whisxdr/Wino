use crate::app::theme::get_colors;
use eframe::egui::{self, Color32, Frame, Margin, Pos2, Rect, RichText, Rounding, Stroke, TextEdit, Ui, Vec2};

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

/// Unified View Header with Title, Live Badge, Subtitle on Left, and Right-Aligned Action/Status
pub fn view_header(
    ui: &mut Ui,
    title: &str,
    subtitle: &str,
    add_right_action: impl FnOnce(&mut Ui),
) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.horizontal(|ui| {
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
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            add_right_action(ui);
        });
    });

    ui.add_space(8.0);
}

/// Unified Modern Search Bar with icon, count badge, and clear button
pub fn search_bar(ui: &mut Ui, query: &mut String, placeholder: &str, count_label: Option<&str>) {
    let colors = get_colors(ui.visuals().dark_mode);

    Frame::none()
        .fill(colors.bg_card)
        .stroke(Stroke::new(1.0_f32, colors.border))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::symmetric(10.0, 6.0))
        .show(ui, |ui| {
            let total_w = ui.available_width();
            ui.set_min_width(total_w);
            ui.set_width(total_w);

            ui.horizontal(|ui| {
                ui.label(RichText::new("🔍").size(12.0).color(colors.text_muted));
                ui.add_space(2.0);

                let desired_w = if count_label.is_some() {
                    (total_w - 200.0).max(120.0)
                } else {
                    (total_w - 70.0).max(120.0)
                };

                let text_edit = TextEdit::singleline(query)
                    .hint_text(RichText::new(placeholder).color(colors.text_muted))
                    .frame(false)
                    .desired_width(desired_w);

                ui.add(text_edit);

                if !query.is_empty() && ui.small_button("✕").clicked() {
                    query.clear();
                }

                if let Some(count) = count_label {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(count).size(11.5).color(colors.text_muted));
                    });
                }
            });
        });

    ui.add_space(8.0);
}

/// Bento Grid Glass Card Container — always expands to the width of its parent allocation
pub fn bento_card<R>(
    ui: &mut Ui,
    title: &str,
    icon: &str,
    badge: Option<&str>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    bento_card_sized(ui, title, icon, badge, None, add_contents)
}

/// Start a rock-solid two column grid using child_ui.
/// Guaranteed 100% top alignment, zero diagonal stair-stepping or line-wrapping.
pub fn start_two_columns(
    ui: &mut Ui,
    left_ratio: f32,
    gap: f32,
) -> (Ui, Ui, egui::Pos2, f32) {
    let total_w = ui.available_width();
    let initial_pos = ui.cursor().min;
    let left_w = ((total_w - gap) * left_ratio).floor().max(80.0);
    let right_w = (total_w - gap - left_w).floor().max(80.0);

    let left_rect = egui::Rect::from_min_size(initial_pos, Vec2::new(left_w, ui.available_height()));
    let right_rect = egui::Rect::from_min_size(
        egui::pos2(initial_pos.x + left_w + gap, initial_pos.y),
        Vec2::new(right_w, ui.available_height()),
    );

    let left_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(left_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let right_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(right_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    (left_ui, right_ui, initial_pos, total_w)
}

/// Finalize a two column grid and advance the parent cursor past the tallest column.
pub fn end_two_columns(
    ui: &mut Ui,
    left_ui: Ui,
    right_ui: Ui,
    initial_pos: egui::Pos2,
    total_w: f32,
) {
    let max_height = left_ui.min_rect().height().max(right_ui.min_rect().height());
    let advance_rect = egui::Rect::from_min_size(initial_pos, Vec2::new(total_w, max_height));
    ui.advance_cursor_after_rect(advance_rect);
}

/// Standardized Bento Card with explicit height and full-width bounding
pub fn bento_card_sized<R>(
    ui: &mut Ui,
    title: &str,
    icon: &str,
    badge: Option<&str>,
    min_height: Option<f32>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let colors = get_colors(ui.visuals().dark_mode);

    Frame::none()
        .fill(colors.bg_card)
        .stroke(Stroke::new(1.0_f32, colors.border))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::same(14.0))
        .show(ui, |ui| {
            let inner_w = ui.available_width();
            ui.set_min_width(inner_w);
            ui.set_max_width(inner_w);
            ui.set_width(inner_w);
            if let Some(h) = min_height {
                ui.set_min_height((h - 28.0).max(0.0));
            }

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

/// Generic Card Container — always stretches to 100% available width inside ScrollArea
pub fn card_container<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let colors = get_colors(ui.visuals().dark_mode);
    let full_w = ui.available_width();

    Frame::none()
        .fill(colors.bg_card)
        .stroke(Stroke::new(1.0_f32, colors.border))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(14.0))
        .show(ui, |ui| {
            let inner_w = full_w - 28.0;
            ui.set_min_width(inner_w);
            ui.set_width(inner_w);
            ui.vertical(|ui| {
                ui.set_width(inner_w);
                add_contents(ui)
            })
            .inner
        })
        .inner
}

/// Standardized List Action Card — prevents text & button overlapping by allocating two explicit columns
pub fn action_card<L, R>(
    ui: &mut Ui,
    action_width: f32,
    render_left: impl FnOnce(&mut Ui) -> L,
    render_right: impl FnOnce(&mut Ui) -> R,
) {
    let colors = get_colors(ui.visuals().dark_mode);
    let full_w = ui.available_width();

    Frame::none()
        .fill(colors.bg_card)
        .stroke(Stroke::new(1.0_f32, colors.border))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(14.0))
        .show(ui, |ui| {
            let inner_w = full_w - 28.0;
            ui.set_min_width(inner_w);
            ui.set_width(inner_w);

            let gap = 12.0_f32;
            let left_w = (inner_w - action_width - gap).max(100.0);

            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(left_w, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        render_left(ui);
                    },
                );

                ui.add_space(gap);

                ui.allocate_ui_with_layout(
                    Vec2::new(action_width, 0.0),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        render_right(ui);
                    },
                );
            });
        });
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

/// Stat Row with Bounded Width to Prevent Text Overlap
pub fn stat_row(ui: &mut Ui, label: &str, value: &str, is_highlight: bool) {
    let colors = get_colors(ui.visuals().dark_mode);
    let total_w = ui.available_width();
    let label_w = 88.0_f32;
    let val_w = (total_w - label_w - 6.0).max(40.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.allocate_ui_with_layout(Vec2::new(label_w, 18.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.label(RichText::new(label).size(10.5).color(colors.text_muted));
        });
        ui.allocate_ui_with_layout(Vec2::new(val_w, 18.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let val_color = if is_highlight { colors.accent } else { colors.text_primary };
            ui.add(egui::Label::new(RichText::new(value).size(11.5).strong().color(val_color)).truncate());
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

    if !label.is_empty() || !value_str.is_empty() {
        ui.horizontal(|ui| {
            if !label.is_empty() {
                ui.label(RichText::new(label).size(11.0).color(colors.text_secondary));
            }
            if !value_str.is_empty() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(value_str).size(11.0).color(colors.text_muted));
                });
            }
        });
        ui.add_space(2.0);
    }

    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(3.0), colors.bg_card_highest);

    let clamped = fraction.clamp(0.0, 1.0);
    let fill_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width() * clamped, rect.height()));
    painter.rect_filled(fill_rect, Rounding::same(3.0), color);
    ui.add_space(6.0);
}

/// Dry-run preview modal helpers
pub struct PreviewGroup {
    pub title: String,
    pub items: Vec<String>,
    pub color: (u8, u8, u8),
}

/// Render a boxed group inside the dry-run preview modal.
pub fn preview_group(ui: &mut Ui, group: &PreviewGroup) {
    let colors = get_colors(ui.visuals().dark_mode);
    if group.items.is_empty() {
        return;
    }
    ui.label(
        RichText::new(format!("{}  ({} {})", group.title, group.items.len(), if group.items.len() == 1 { "item" } else { "items" }))
            .size(12.5)
            .strong()
            .color(Color32::from_rgb(group.color.0, group.color.1, group.color.2)),
    );
    ui.add_space(4.0);
    egui::ScrollArea::vertical()
        .max_height(110.0)
        .show(ui, |ui| {
            for item in &group.items {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("•").size(11.0).color(colors.text_muted));
                    ui.label(RichText::new(item).size(11.0).color(colors.text_secondary));
                });
            }
        });
    ui.add_space(8.0);
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

