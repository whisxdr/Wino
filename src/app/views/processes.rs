use crate::app::components::{card_container, search_bar, status_badge, view_header};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::core::i18n::tr;
use eframe::egui::{self, Button, Color32, Frame, Margin, RichText, Rounding, Stroke, Ui, Vec2};

/// Exact-width clipped table cell to guarantee 0% column overlap across all screen resolutions
fn table_cell<R>(
    ui: &mut Ui,
    width: f32,
    height: f32,
    layout: egui::Layout,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    let mut cell_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout));
    cell_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
    add_contents(&mut cell_ui)
}

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    // 1. Header with Refresh Button
    view_header(
        ui,
        tr(lang, "proc.title"),
        tr(lang, "proc.subtitle"),
        |ui| {
            let refresh_btn = Button::new(tr(lang, "proc.refresh"))
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(refresh_btn).clicked() {
                state.refresh_processes();
            }
        },
    );

    // 2. Modern Unified Search Bar
    let count_str = format!(
        "{} {}",
        state.processes.len(),
        tr(lang, "proc.active_tasks")
    );
    search_bar(
        ui,
        &mut state.search_query,
        tr(lang, "proc.filter_placeholder"),
        Some(&count_str),
    );

    // 3. Process Table Container (Spans 100% Full Width)
    card_container(ui, |ui| {
        let total_w = ui.available_width();
        let gap = 12.0_f32;
        let pid_col_w = 75.0_f32;
        let ws_col_w = 115.0_f32;
        let sec_col_w = 135.0_f32;
        let act_col_w = 90.0_f32;
        let fixed_sum = pid_col_w + ws_col_w + sec_col_w + act_col_w + (4.0 * gap);
        let name_col_w = (total_w - fixed_sum).max(180.0);

        // Header Row
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;

            table_cell(
                ui,
                name_col_w,
                24.0,
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(
                        RichText::new(tr(lang, "proc.col_name"))
                            .strong()
                            .color(colors.text_primary),
                    );
                },
            );
            table_cell(
                ui,
                pid_col_w,
                24.0,
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    ui.label(
                        RichText::new(tr(lang, "proc.col_pid"))
                            .strong()
                            .color(colors.text_primary),
                    );
                },
            );
            table_cell(
                ui,
                ws_col_w,
                24.0,
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    ui.label(
                        RichText::new(tr(lang, "proc.col_working_set"))
                            .strong()
                            .color(colors.text_primary),
                    );
                },
            );
            table_cell(
                ui,
                sec_col_w,
                24.0,
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(
                        RichText::new(tr(lang, "proc.col_security"))
                            .strong()
                            .color(colors.text_primary),
                    );
                },
            );
            table_cell(
                ui,
                act_col_w,
                24.0,
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    ui.label(
                        RichText::new(tr(lang, "proc.col_action"))
                            .strong()
                            .color(colors.text_primary),
                    );
                },
            );
        });

        ui.add_space(4.0);
        let (sep_rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().rect_filled(sep_rect, 0.0, colors.border);
        ui.add_space(4.0);

        let query = state.search_query.to_lowercase();
        let mut kill_target_pid: Option<u32> = None;
        let mut rendered_count = 0;

        for (idx, p) in state.processes.iter().enumerate() {
            if !query.is_empty()
                && !p.name.to_lowercase().contains(&query)
                && !p.pid.to_string().contains(&query)
            {
                continue;
            }
            rendered_count += 1;

            let row_bg = if idx % 2 == 1 {
                colors.bg_card_highest
            } else {
                Color32::TRANSPARENT
            };

            Frame::none()
                .fill(row_bg)
                .rounding(Rounding::same(4.0))
                .inner_margin(Margin::symmetric(4.0, 3.0))
                .show(ui, |ui| {
                    let row_h = 24.0;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;

                        // Process Name (Truncated cleanly if super long)
                        table_cell(
                            ui,
                            name_col_w - 8.0,
                            row_h,
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&p.name).strong().color(colors.text_primary),
                                    )
                                    .truncate(),
                                );
                            },
                        );

                        // PID
                        table_cell(
                            ui,
                            pid_col_w,
                            row_h,
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    RichText::new(format!("{}", p.pid)).color(colors.text_muted),
                                );
                            },
                        );

                        // Memory Working Set
                        table_cell(
                            ui,
                            ws_col_w,
                            row_h,
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{:.1} MB",
                                        p.memory_working_set_bytes as f64 / (1024.0 * 1024.0)
                                    ))
                                    .strong()
                                    .color(colors.accent),
                                );
                            },
                        );

                        // Security Badge
                        table_cell(
                            ui,
                            sec_col_w,
                            row_h,
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                if p.is_system_critical {
                                    status_badge(
                                        ui,
                                        tr(lang, "proc.badge_critical"),
                                        (239, 68, 68),
                                    );
                                } else if p.is_signed {
                                    status_badge(ui, tr(lang, "proc.badge_signed"), (78, 222, 163));
                                } else {
                                    status_badge(ui, tr(lang, "proc.badge_user"), (138, 145, 160));
                                }
                            },
                        );

                        // Action Button
                        table_cell(
                            ui,
                            act_col_w,
                            row_h,
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if p.is_system_critical {
                                    ui.label(
                                        RichText::new(tr(lang, "proc.protected"))
                                            .color(colors.text_muted),
                                    );
                                } else {
                                    let end_btn = Button::new(
                                        RichText::new(tr(lang, "proc.end_task"))
                                            .color(colors.danger),
                                    )
                                    .fill(colors.bg_card_hover)
                                    .rounding(Rounding::same(5.0))
                                    .min_size(Vec2::new(75.0, 22.0));
                                    if ui.add(end_btn).clicked() {
                                        kill_target_pid = Some(p.pid);
                                    }
                                }
                            },
                        );
                    });
                });

            ui.add_space(2.0);
        }

        if rendered_count == 0 {
            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(tr(lang, "proc.no_match")).color(colors.text_muted));
            });
            ui.add_space(16.0);
        }

        if let Some(pid) = kill_target_pid {
            let _ = crate::monitoring::process::kill_process(pid);
            state.set_toast(&format!("{} {}", tr(lang, "proc.terminated_toast"), pid));
            state.refresh_processes();
        }
    });
}
