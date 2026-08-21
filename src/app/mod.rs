pub mod components;
pub mod navigation;
pub mod state;
pub mod theme;
pub mod views;

use crate::app::navigation::NavTab;
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Sense, Stroke};

pub struct WinoApp {
    pub state: AppState,
}

impl WinoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure native Windows Segoe UI and Segoe UI Symbol fonts for crisp glyph rendering
        theme::configure_fonts(&cc.egui_ctx);

        Self {
            state: AppState::new(),
        }
    }
}

impl eframe::App for WinoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll background metrics periodically
        self.state.update_tick(ctx);

        // Apply theme styling
        theme::apply_theme(ctx, &self.state.config.general.theme);
        let colors = get_colors(ctx.style().visuals.dark_mode);

        // 1. Bottom Status & Live Event Bar (Footer across entire window)
        egui::TopBottomPanel::bottom("wino_bottom_bar")
            .exact_height(34.0)
            .frame(
                Frame::none()
                    .fill(colors.bg_panel)
                    .stroke(Stroke::new(1.0_f32, colors.border))
                    .inner_margin(Margin::symmetric(14.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left Status: Host info and summary metrics
                    let admin_tag = if self.state.sys_info.is_admin { "Admin" } else { "User" };
                    let win_tag = if self.state.sys_info.is_windows_11 { "Win 11" } else { "Win 10" };

                    ui.label(RichText::new("●").size(10.0).color(colors.success));
                    ui.label(
                        RichText::new(format!(
                            "{} ({}) | CPU: {:.1}% | RAM: {:.1}% | {} Procs",
                            win_tag,
                            admin_tag,
                            self.state.metrics.cpu_usage_pct,
                            self.state.metrics.ram_usage_pct,
                            self.state.metrics.process_count
                        ))
                        .size(11.0)
                        .color(colors.text_muted),
                    );

                    // Right Status: Little Live Event Badge / Ticker
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (event_text, event_time, is_recent) = if let Some(ev) = &self.state.last_event {
                            let recent = ev.created_at.elapsed().as_secs() < 8;
                            (
                                format!("[{}] {}", ev.category, ev.message),
                                ev.timestamp.clone(),
                                recent,
                            )
                        } else {
                            ("System Ready".to_string(), "".to_string(), false)
                        };

                        let pill_bg = if is_recent {
                            Color32::from_rgba_unmultiplied(colors.accent.r(), colors.accent.g(), colors.accent.b(), 35)
                        } else {
                            colors.bg_card
                        };

                        let pill_stroke = if is_recent {
                            Stroke::new(1.0_f32, colors.accent)
                        } else {
                            Stroke::new(1.0_f32, colors.border)
                        };

                        let event_pill = Frame::none()
                            .fill(pill_bg)
                            .stroke(pill_stroke)
                            .rounding(Rounding::same(12.0))
                            .inner_margin(Margin::symmetric(10.0, 3.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if is_recent {
                                        ui.label(RichText::new("⚡").size(11.0).color(colors.accent));
                                    } else {
                                        ui.label(RichText::new("📋").size(11.0).color(colors.text_muted));
                                    }
                                    if !event_time.is_empty() {
                                        ui.label(RichText::new(&event_time).size(10.0).color(colors.text_muted));
                                    }
                                    ui.label(
                                        RichText::new(if event_text.len() > 50 {
                                            format!("{}...", &event_text[..47])
                                        } else {
                                            event_text
                                        })
                                        .size(11.0)
                                        .color(if is_recent { colors.text_primary } else { colors.text_secondary }),
                                    );
                                });
                            });

                        let resp = ui.interact(event_pill.response.rect, ui.id().with("bottom_event_pill"), Sense::click());
                        if resp.clicked() {
                            self.state.current_tab = NavTab::Logs;
                        }
                        if resp.hovered() {
                            ui.ctx().output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        }
                    });
                });
            });

        // 2. Left Navigation Sidebar
        egui::SidePanel::left("wino_sidebar")
            .resizable(false)
            .exact_width(215.0)
            .frame(
                Frame::none()
                    .fill(colors.bg_panel)
                    .stroke(Stroke::new(1.0_f32, colors.border))
                    .inner_margin(Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                navigation::render_sidebar(ui, &mut self.state);
            });

        // 3. Main Central Panel View
        egui::CentralPanel::default().show(ctx, |ui| {
            views::render_active_view(ui, &mut self.state);
        });
    }
}
