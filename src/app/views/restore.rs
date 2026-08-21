use crate::app::components::{card_container, section_header, status_badge};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{self, Button, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.horizontal(|ui| {
        section_header(ui, "Restore & Safety Snapshots", "Manage local configuration snapshots and rollback system changes");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Refresh Snapshots").clicked() {
                state.refresh_snapshots();
            }
        });
    });

    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Safety Snapshots").size(14.0).strong());
                ui.label(RichText::new("Wino automatically saves configuration snapshots before applying modifications.").size(12.0).color(colors.text_muted));
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let snap_btn = Button::new("Create Manual Snapshot")
                    .fill(colors.accent)
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(160.0, 30.0));

                if ui.add(snap_btn).clicked() {
                    let snap = crate::restore::snapshots::create_snapshot("Manual User Snapshot");
                    state.set_toast(&format!("Created snapshot: {}", snap.id));
                    state.refresh_snapshots();
                }
            });
        });
    });

    ui.add_space(10.0);

    let mut rollback_id: Option<String> = None;

    if state.snapshots.is_empty() {
        card_container(ui, |ui| {
            ui.label(RichText::new("No configuration snapshots found. Snapshots are created automatically before changes.").color(colors.text_muted));
        });
    }

    for snap in &state.snapshots {
        card_container(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&snap.description).size(14.0).strong());
                        status_badge(ui, &snap.id, (100, 116, 139));
                    });
                    ui.add_space(2.0);
                    ui.label(RichText::new(format!("Timestamp: {}", snap.timestamp)).size(11.0).color(colors.text_muted));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let btn = Button::new(RichText::new("Restore Snapshot").color(colors.accent))
                        .fill(colors.bg_card_hover)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(120.0, 26.0));

                    if ui.add(btn).clicked() {
                        rollback_id = Some(snap.id.clone());
                    }
                });
            });
        });
        ui.add_space(4.0);
    }

    if let Some(id) = rollback_id {
        match crate::restore::rollback::rollback_snapshot(&id) {
            Ok(msg) => {
                state.set_toast(&msg);
                state.refresh_snapshots();
            }
            Err(err) => state.set_toast(&format!("Error: {}", err)),
        }
    }
}
