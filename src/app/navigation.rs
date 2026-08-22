use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::core::i18n::{tr, Lang};
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
    ContextMenu,
    Network,
    Tasks,
    Gaming,
    Health,
    Restore,
    Logs,
    Settings,
}

impl NavTab {
    /// i18n dictionary key for this tab's display title.
    pub fn i18n_key(&self) -> &'static str {
        match self {
            NavTab::Dashboard => "nav.dashboard",
            NavTab::Memory => "nav.memory",
            NavTab::Processes => "nav.processes",
            NavTab::Debloat => "nav.debloat",
            NavTab::Startup => "nav.startup",
            NavTab::Services => "nav.services",
            NavTab::Privacy => "nav.privacy",
            NavTab::Cleaner => "nav.cleaner",
            NavTab::ContextMenu => "nav.contextmenu",
            NavTab::Network => "nav.network",
            NavTab::Tasks => "nav.tasks",
            NavTab::Gaming => "nav.gaming",
            NavTab::Health => "nav.health",
            NavTab::Restore => "nav.restore",
            NavTab::Logs => "nav.logs",
            NavTab::Settings => "nav.settings",
        }
    }

    pub fn title(&self) -> &'static str {
        tr(Lang::En, self.i18n_key())
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
            NavTab::ContextMenu => "🗂",
            NavTab::Network => "🌐",
            NavTab::Tasks => "⏱",
            NavTab::Gaming => "🎮",
            NavTab::Health => "✚",
            NavTab::Restore => "↺",
            NavTab::Logs => "📋",
            NavTab::Settings => "🔧",
        }
    }

    /// Which background scan should kick off when this tab opens.
    pub fn on_open_scan(&self) -> Option<crate::app::worker::ScanKind> {
        match self {
            NavTab::Processes => Some(crate::app::worker::ScanKind::Processes),
            NavTab::Debloat => Some(crate::app::worker::ScanKind::Debloat),
            NavTab::Startup => Some(crate::app::worker::ScanKind::Startup),
            NavTab::Services => Some(crate::app::worker::ScanKind::Services),
            NavTab::Privacy => Some(crate::app::worker::ScanKind::Privacy),
            NavTab::Cleaner => Some(crate::app::worker::ScanKind::Cleaner),
            NavTab::ContextMenu => Some(crate::app::worker::ScanKind::ContextMenu),
            NavTab::Tasks => Some(crate::app::worker::ScanKind::Tasks),
            NavTab::Restore => Some(crate::app::worker::ScanKind::Snapshots),
            _ => None,
        }
    }
}

pub fn render_sidebar(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.add_space(2.0);

    // 1. Sidebar Brand Header
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Wino").size(20.0).strong().color(colors.accent));
                ui.label(RichText::new("Pro").size(20.0).strong().color(colors.text_primary));
            });
            ui.add_space(1.0);
            ui.label(RichText::new("v2.5.0 • Rust-Native").size(10.5).color(colors.text_muted));
        });
    });

    ui.add_space(10.0);

    let tabs = [
        NavTab::Dashboard,
        NavTab::Memory,
        NavTab::Processes,
        NavTab::Debloat,
        NavTab::Startup,
        NavTab::Services,
        NavTab::Privacy,
        NavTab::Cleaner,
        NavTab::ContextMenu,
        NavTab::Network,
        NavTab::Tasks,
        NavTab::Gaming,
        NavTab::Health,
        NavTab::Restore,
        NavTab::Logs,
        NavTab::Settings,
    ];

    // 2. Navigation Items in dedicated vertical scroll area (leaving 48px for bottom card)
    let max_scroll_h = (ui.available_height() - 48.0).max(100.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(max_scroll_h)
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
                        .inner_margin(Margin::symmetric(10.0, 6.0))
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
                                    RichText::new(tr(state.lang, tab.i18n_key()))
                                        .size(12.5)
                                        .color(text_color)
                                        .strong(),
                                );
                            });
                        });

                    // Modern Windows 11 Fluent: Left vertical accent indicator bar for active item
                    if is_selected {
                        let r = button_rect.response.rect;
                        let bar_rect = egui::Rect::from_min_max(
                            Pos2::new(r.min.x + 2.0, r.min.y + 6.0),
                            Pos2::new(r.min.x + 5.5, r.max.y - 6.0),
                        );
                        ui.painter().rect_filled(bar_rect, Rounding::same(2.0), colors.accent);
                    }

                    let response = ui.interact(button_rect.response.rect, ui.id().with(tab.title()), Sense::click());
                    if response.clicked() {
                        state.current_tab = tab;
                        if let Some(kind) = tab.on_open_scan() {
                            state.request_scan(kind);
                        }
                    }

                    ui.add_space(1.5);
                }
            });
        });

    // 3. Sidebar Footer User / Admin Profile Card (Permanently docked, never cut off)
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        Frame::none()
            .fill(colors.bg_card_highest)
            .stroke(Stroke::new(1.0_f32, colors.border))
            .rounding(Rounding::same(6.0))
            .inner_margin(Margin::symmetric(10.0, 8.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    if state.sys_info.is_admin {
                        ui.label(RichText::new("●").size(10.0).color(colors.success));
                        ui.label(RichText::new(tr(state.lang, "common.admin_elevated")).size(11.0).strong().color(colors.text_primary));
                    } else {
                        ui.label(RichText::new("●").size(10.0).color(colors.warning));
                        ui.label(RichText::new(tr(state.lang, "common.standard_user")).size(11.0).color(colors.text_secondary));
                    }
                });
            });
    });
}

