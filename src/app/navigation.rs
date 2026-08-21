use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, Color32, Frame, Margin, Pos2, RichText, Rounding, Sense, Stroke, Ui, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavTab {
    Dashboard,
    Memory,
    Processes,
    Debloat,
    Startup,
    Services,
    Privacy,
    Cleaner,
    Gaming,
    Health,
    Restore,
    Logs,
    Settings,
}

impl NavTab {
    pub fn title(&self) -> &'static str {
        match self {
            NavTab::Dashboard => "Dashboard",
            NavTab::Memory => "Memory Engine",
            NavTab::Processes => "Process Manager",
            NavTab::Debloat => "Debloater",
            NavTab::Startup => "Startup Apps",
            NavTab::Services => "Service Manager",
            NavTab::Privacy => "Privacy Center",
            NavTab::Cleaner => "Storage Cleaner",
            NavTab::Gaming => "Gaming Profile",
            NavTab::Health => "Health Diagnostics",
            NavTab::Restore => "Restore Points",
            NavTab::Logs => "Audit Logs",
            NavTab::Settings => "Settings",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            NavTab::Dashboard => "⊞",
            NavTab::Memory => "⚡",
            NavTab::Processes => "≡",
            NavTab::Debloat => "🧹",
            NavTab::Startup => "🚀",
            NavTab::Services => "⚙",
            NavTab::Privacy => "🛡",
            NavTab::Cleaner => "🗑",
            NavTab::Gaming => "🎮",
            NavTab::Health => "✚",
            NavTab::Restore => "↺",
            NavTab::Logs => "📋",
            NavTab::Settings => "🔧",
        }
    }
}

pub fn render_sidebar(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.add_space(4.0);

    // 1. Sidebar Brand Header (Faithful to Wino Pro v2.4.0)
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Wino").size(20.0).strong().color(colors.accent));
                ui.label(RichText::new("Pro").size(20.0).strong().color(colors.text_primary));
            });
            ui.add_space(1.0);
            ui.label(RichText::new("v2.4.0 • Rust-Native").size(10.5).color(colors.text_muted));
        });
    });

    ui.add_space(12.0);

    let tabs = [
        NavTab::Dashboard,
        NavTab::Memory,
        NavTab::Processes,
        NavTab::Debloat,
        NavTab::Startup,
        NavTab::Services,
        NavTab::Privacy,
        NavTab::Cleaner,
        NavTab::Gaming,
        NavTab::Health,
        NavTab::Restore,
        NavTab::Logs,
        NavTab::Settings,
    ];

    // 2. Navigation Items with active border highlight
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.vertical(|ui| {
                for tab in tabs {
                    let is_selected = state.current_tab == tab;
                    let bg_color = if is_selected {
                        colors.bg_card_hover
                    } else {
                        Color32::TRANSPARENT
                    };
                    let text_color = if is_selected {
                        colors.accent
                    } else {
                        colors.text_secondary
                    };
                    let icon_color = if is_selected {
                        colors.accent
                    } else {
                        colors.text_muted
                    };

                    let button_rect = Frame::none()
                        .fill(bg_color)
                        .rounding(Rounding::same(6.0))
                        .inner_margin(Margin::symmetric(10.0, 6.5))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                // Dedicated fixed-width icon container
                                ui.allocate_ui_with_layout(
                                    Vec2::new(22.0, 18.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.label(RichText::new(tab.icon()).size(14.0).color(icon_color).strong());
                                    },
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(tab.title())
                                        .size(12.5)
                                        .color(text_color)
                                        .strong(),
                                );
                            });
                        });

                    // Draw right border accent line for active item
                    if is_selected {
                        let r = button_rect.response.rect;
                        let line_stroke = Stroke::new(2.5_f32, colors.accent);
                        ui.painter().line_segment(
                            [Pos2::new(r.max.x - 1.0, r.min.y + 4.0), Pos2::new(r.max.x - 1.0, r.max.y - 4.0)],
                            line_stroke,
                        );
                    }

                    let response = ui.interact(button_rect.response.rect, ui.id().with(tab.title()), Sense::click());
                    if response.clicked() {
                        state.current_tab = tab;
                        match tab {
                            NavTab::Processes => state.refresh_processes(),
                            NavTab::Debloat => state.refresh_debloat(),
                            NavTab::Startup => state.refresh_startup(),
                            NavTab::Services => state.refresh_services(),
                            NavTab::Privacy => state.refresh_privacy(),
                            NavTab::Cleaner => state.refresh_cleaner(),
                            NavTab::Restore => state.refresh_snapshots(),
                            _ => {}
                        }
                    }

                    ui.add_space(1.5);
                }
            });

            // 3. Sidebar Footer User / Admin Profile Card
            ui.add_space(12.0);
            Frame::none()
                .fill(colors.bg_card_highest)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(10.0, 8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if state.sys_info.is_admin {
                            ui.label(RichText::new("●").size(10.0).color(colors.success));
                            ui.label(RichText::new("Admin • Elevated").size(11.0).strong().color(colors.text_primary));
                        } else {
                            ui.label(RichText::new("●").size(10.0).color(colors.warning));
                            ui.label(RichText::new("Standard User").size(11.0).color(colors.text_secondary));
                        }
                    });
                });
        });
}
