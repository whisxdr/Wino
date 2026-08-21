use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Sense, Ui, Vec2};
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
            NavTab::Processes => "Processes",
            NavTab::Debloat => "Debloat",
            NavTab::Startup => "Startup Apps",
            NavTab::Services => "Services",
            NavTab::Privacy => "Privacy Center",
            NavTab::Cleaner => "Storage Cleaner",
            NavTab::Gaming => "Gaming Profile",
            NavTab::Health => "Windows Health",
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

    ui.add_space(6.0);

    // Sidebar Header / Brand
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(RichText::new("WINO").size(20.0).strong().color(colors.accent));
        ui.label(RichText::new("PRO").size(10.0).color(colors.text_muted));
    });
    ui.add_space(8.0);

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

    // Scrollable navigation list that adjusts gracefully when windowed
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.vertical(|ui| {
                for tab in tabs {
                    let is_selected = state.current_tab == tab;
                    let bg_color = if is_selected {
                        colors.accent
                    } else {
                        Color32::TRANSPARENT
                    };
                    let text_color = if is_selected {
                        Color32::WHITE
                    } else {
                        colors.text_primary
                    };
                    let icon_color = if is_selected {
                        Color32::WHITE
                    } else {
                        colors.accent
                    };

                    let button_rect = Frame::none()
                        .fill(bg_color)
                        .rounding(Rounding::same(6.0))
                        .inner_margin(Margin::symmetric(10.0, 7.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                // Dedicated fixed-width icon slot so it never overlaps or clips
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
                                        .size(13.0)
                                        .color(text_color)
                                        .strong(),
                                );
                            });
                        });

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

                    ui.add_space(2.0);
                }
            });
        });
}
