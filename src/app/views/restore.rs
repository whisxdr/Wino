use crate::app::components::{action_card, card_container, status_badge, view_header};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use eframe::egui::{Button, Color32, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);

    // 1. Contextual Header with Refresh Button
    view_header(
        ui,
        "Restore & Safety Snapshots",
        "Manage local configuration snapshots and rollback system changes",
        |ui| {
            let refresh_btn = Button::new("↻ Refresh Snapshots")
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(refresh_btn).clicked() {
                state.refresh_snapshots();
            }
        },
    );

    // 2. Action Header Card (100% Full Width)
    action_card(
        ui,
        180.0,
        |ui| {
            ui.label(RichText::new("Safety Snapshots").size(15.0).strong());
            ui.add_space(2.0);
            ui.label(RichText::new("Wino saves a config snapshot before it applies changes.").size(12.0).color(colors.text_muted));
        },
        |ui| {
            let snap_btn = Button::new(RichText::new("➕ Create Manual Snapshot").strong().color(Color32::WHITE))
                .fill(colors.accent)
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(170.0, 32.0));

            if ui.add(snap_btn).clicked() {
                let snap = crate::restore::snapshots::create_snapshot("Manual User Snapshot");
                state.set_toast(&format!("Created snapshot: {}", snap.id));
                state.refresh_snapshots();
            }
        },
    );

    ui.add_space(10.0);

    let mut rollback_id: Option<String> = None;

    if state.snapshots.is_empty() {
        card_container(ui, |ui| {
            ui.label(RichText::new("No snapshots yet. Wino saves one before it applies the next change.").color(colors.text_muted));
        });
    }

    // 3. Snapshot List Cards (100% Full Width)
    for snap in &state.snapshots {
        action_card(
            ui,
            140.0,
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&snap.description).size(14.0).strong());
                    status_badge(ui, &snap.id, (100, 116, 139));
                });
                ui.add_space(2.0);
                ui.label(RichText::new(format!("Timestamp: {}", snap.timestamp)).size(11.0).color(colors.text_muted));
            },
            |ui| {
                let btn = Button::new(RichText::new("↺ Restore Snapshot").color(colors.accent))
                    .fill(colors.bg_card_hover)
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(130.0, 26.0));

                if ui.add(btn).clicked() {
                    rollback_id = Some(snap.id.clone());
                }
            },
        );
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


