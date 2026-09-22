pub mod components;
pub mod navigation;
pub mod state;
pub mod theme;
pub mod views;
pub mod worker;

use crate::app::navigation::NavTab;
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::core::i18n::tr;
use eframe::egui::{self, Button, Color32, Frame, Margin, RichText, Rounding, Sense, Stroke, Vec2};

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

        // 1. Bottom Status & Live Event Bar (Footer across entire window matching Wino Pro)
        egui::TopBottomPanel::bottom("wino_bottom_bar")
            .exact_height(32.0)
            .frame(
                Frame::none()
                    .fill(colors.bg_footer)
                    .stroke(Stroke::new(1.0_f32, colors.border))
                    .inner_margin(Margin::symmetric(16.0, 5.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left Status: Host info and summary metrics
                    let admin_tag = if self.state.sys_info.is_admin {
                        "Admin"
                    } else {
                        "User"
                    };
                    let win_tag = if self.state.sys_info.is_windows_11 {
                        "Win 11"
                    } else {
                        "Win 10"
                    };

                    ui.label(RichText::new("●").size(10.0).color(colors.success));
                    ui.label(
                        RichText::new(format!(
                            "Wino Pro Engine • {} ({}) | CPU: {:.1}% | RAM: {:.1}% | {} Procs",
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
                        let (event_text, event_time, is_recent) =
                            if let Some(ev) = &self.state.last_event {
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
                            colors.accent_glow
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
                            .inner_margin(Margin::symmetric(10.0, 2.5))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if is_recent {
                                        ui.label(
                                            RichText::new("⚡").size(11.0).color(colors.accent),
                                        );
                                    } else {
                                        ui.label(
                                            RichText::new("📋").size(11.0).color(colors.text_muted),
                                        );
                                    }
                                    if !event_time.is_empty() {
                                        ui.label(
                                            RichText::new(&event_time)
                                                .size(10.0)
                                                .color(colors.text_muted),
                                        );
                                    }
                                    ui.label(
                                        RichText::new(if event_text.len() > 46 {
                                            format!("{}...", &event_text[..43])
                                        } else {
                                            event_text
                                        })
                                        .size(11.0)
                                        .color(
                                            if is_recent {
                                                colors.text_primary
                                            } else {
                                                colors.text_secondary
                                            },
                                        ),
                                    );
                                });
                            });

                        let resp = ui.interact(
                            event_pill.response.rect,
                            ui.id().with("bottom_event_pill"),
                            Sense::click(),
                        );
                        if resp.clicked() {
                            self.state.current_tab = NavTab::Logs;
                        }
                        if resp.hovered() {
                            ui.ctx()
                                .output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        }
                    });
                });
            });

        // 2. Left Navigation Sidebar (240px matching Wino Pro)
        egui::SidePanel::left("wino_sidebar")
            .resizable(false)
            .exact_width(240.0)
            .frame(
                Frame::none()
                    .fill(colors.bg_panel)
                    .stroke(Stroke::new(1.0_f32, colors.border))
                    .inner_margin(Margin::same(10.0)),
            )
            .show(ctx, |ui| {
                navigation::render_sidebar(ui, &mut self.state);
            });

        // 3. Main Central Panel View
        egui::CentralPanel::default()
            .frame(
                Frame::none()
                    .fill(colors.bg_canvas)
                    .inner_margin(Margin::same(16.0)),
            )
            .show(ctx, |ui| {
                views::render_active_view(ui, &mut self.state);
            });

        // 4. Shared confirmation dialog for every mutating action. Rendering it
        //    once here is what guarantees no view can apply a change without
        //    disclosing its risk, reversibility, and privilege requirement.
        self.render_confirmation(ctx, &colors);
    }
}

impl WinoApp {
    /// Modal confirmation for the action currently staged in
    /// [`AppState::pending_confirm`].
    ///
    /// The dialog is the single choke point for destructive operations, so the
    /// fields it shows (risk, reversibility, admin requirement) are read from
    /// the staged action itself rather than supplied by the calling view.
    fn render_confirmation(
        &mut self,
        ctx: &egui::Context,
        colors: &crate::app::theme::FluentColors,
    ) {
        let Some(pending) = self.state.pending_confirm.clone() else {
            return;
        };

        let lang = self.state.lang;
        let mut decision: Option<bool> = None;

        egui::Window::new(pending.title(lang))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::new(0.0, 0.0))
            .fixed_size(Vec2::new(520.0, 0.0))
            .frame(
                Frame::none()
                    .fill(colors.bg_card)
                    .stroke(Stroke::new(1.0_f32, colors.border))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::same(18.0)),
            )
            .show(ctx, |ui| {
                ui.set_max_width(484.0);

                ui.label(
                    RichText::new(pending.body(lang))
                        .size(12.5)
                        .color(colors.text_secondary),
                );
                ui.add_space(10.0);

                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(tr(lang, "common.reversible"))
                            .size(11.0)
                            .color(colors.text_muted),
                    );
                    if pending.reversible() {
                        ui.label(
                            RichText::new(tr(lang, "common.yes"))
                                .size(11.5)
                                .strong()
                                .color(colors.success),
                        );
                    } else {
                        ui.label(
                            RichText::new(tr(lang, "common.no"))
                                .size(11.5)
                                .strong()
                                .color(colors.danger),
                        );
                    }

                    ui.add_space(12.0);

                    ui.label(
                        RichText::new(tr(lang, "common.requires_admin"))
                            .size(11.0)
                            .color(colors.text_muted),
                    );
                    if pending.requires_admin() {
                        if self.state.sys_info.is_admin {
                            ui.label(
                                RichText::new(tr(lang, "common.yes"))
                                    .size(11.5)
                                    .strong()
                                    .color(colors.success),
                            );
                        } else {
                            ui.label(
                                RichText::new(tr(lang, "common.no"))
                                    .size(11.5)
                                    .strong()
                                    .color(colors.warning),
                            );
                        }
                    } else {
                        ui.label(
                            RichText::new(tr(lang, "common.no"))
                                .size(11.5)
                                .color(colors.text_secondary),
                        );
                    }
                });

                // Elevation is a hard requirement for some operations; say so
                // before the user confirms rather than failing afterwards.
                if pending.requires_admin() && !self.state.sys_info.is_admin {
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("⚠").size(12.0).color(colors.warning));
                        ui.label(
                            RichText::new(tr(lang, "healthcenter.admin_required"))
                                .size(11.5)
                                .color(colors.warning),
                        );
                    });
                }

                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    let confirm_btn = Button::new(
                        RichText::new(tr(lang, "common.confirm"))
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(if pending.reversible() {
                        colors.accent
                    } else {
                        colors.danger
                    })
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(140.0, 32.0));
                    if ui.add(confirm_btn).clicked() {
                        decision = Some(true);
                    }

                    ui.add_space(8.0);

                    let cancel_btn = Button::new(tr(lang, "common.cancel"))
                        .fill(colors.bg_card_hover)
                        .stroke(Stroke::new(1.0_f32, colors.border))
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(120.0, 32.0));
                    if ui.add(cancel_btn).clicked() {
                        decision = Some(false);
                    }
                });
            });

        match decision {
            Some(true) => self.state.confirm_pending(),
            Some(false) => self.state.cancel_pending(),
            None => {}
        }
    }
}
