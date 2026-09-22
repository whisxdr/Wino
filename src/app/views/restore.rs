//! Restore & Safety Snapshots view.
//!
//! WHY the view is shaped this way:
//!
//! * Rollback and delete both route through [`AppState::request_confirm`].
//!   Rollback overwrites live system state and delete destroys the only record
//!   of a change, so neither fires on a single click.
//! * Each snapshot card shows its operation count and the categories it
//!   actually recorded. A snapshot that captured nothing renders as "nothing to
//!   restore" rather than presenting an empty rollback as a safety net.
//! * The Windows System Restore Point action is separate from Wino's own
//!   snapshots. They are different mechanisms with different scope, and
//!   conflating them in one button would misrepresent what gets restored.

use crate::app::components::{action_card, card_container, status_badge, view_header};
use crate::app::state::{AppState, PendingConfirm};
use crate::app::theme::get_colors;
use crate::app::worker::Action;
use crate::core::i18n::{tr, Lang};
use crate::restore::snapshots::Snapshot;
use eframe::egui::{Button, Color32, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;

    view_header(
        ui,
        tr(lang, "restore.title"),
        tr(lang, "restore.subtitle"),
        |ui| {
            let refresh_btn = Button::new(format!("↻ {}", tr(lang, "common.refresh")))
                .fill(colors.bg_card)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0));
            if ui.add(refresh_btn).clicked() {
                state.refresh_snapshots();
            }
        },
    );

    let mut create_snapshot = false;
    let mut create_vss = false;
    let mut stage_rollback: Option<(String, String)> = None;
    let mut stage_delete: Option<(String, String)> = None;
    let mut export_id: Option<String> = None;
    let mut inspect_id: Option<String> = None;

    // ---- Actions ----
    action_card(
        ui,
        200.0,
        |ui| {
            ui.label(
                RichText::new(tr(lang, "restore.create_manual"))
                    .size(15.0)
                    .strong(),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new(tr(lang, "restore.restore_confirm_body"))
                    .size(11.5)
                    .color(colors.text_muted),
            );
        },
        |ui| {
            let snap_btn = Button::new(
                RichText::new(tr(lang, "restore.create_manual"))
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(colors.accent)
            .rounding(Rounding::same(6.0))
            .min_size(Vec2::new(190.0, 32.0));
            if ui.add(snap_btn).clicked() {
                create_snapshot = true;
            }

            ui.add_space(4.0);

            // Separate mechanism, separate button: this creates a Windows
            // System Restore Point through VSS, not a Wino snapshot.
            let vss_btn = Button::new(tr(lang, "restore.vss_create"))
                .fill(colors.bg_card_hover)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(190.0, 28.0));
            if ui.add_enabled(!state.action_busy, vss_btn).clicked() {
                create_vss = true;
            }
        },
    );

    ui.add_space(10.0);

    if state.snapshots.is_empty() {
        card_container(ui, |ui| {
            ui.label(
                RichText::new(tr(lang, "restore.no_snapshots"))
                    .size(12.0)
                    .color(colors.text_muted),
            );
        });
        return;
    }

    // ---- Snapshot list ----
    for snap in &state.snapshots {
        let restorable = snap.is_restorable();
        let categories = snap.categories();

        action_card(
            ui,
            260.0,
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&snap.description).size(14.0).strong());
                    status_badge(ui, &snap.id, (100, 116, 139));
                    if restorable {
                        status_badge(ui, tr(lang, "restore.restore_available"), (34, 197, 94));
                    } else {
                        status_badge(ui, tr(lang, "restore.restore_unavailable"), (148, 163, 184));
                    }
                });

                ui.add_space(2.0);
                ui.label(
                    RichText::new(format!(
                        "{}  •  {} {}",
                        snap.timestamp,
                        snap.operation_count(),
                        tr(lang, "restore.op_count")
                    ))
                    .size(11.0)
                    .color(colors.text_muted),
                );

                if !categories.is_empty() {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(tr(lang, "restore.categories"))
                                .size(10.5)
                                .color(colors.text_muted),
                        );
                        for category in categories {
                            status_badge(ui, tr(lang, category.i18n_key()), category.color_rgb());
                        }
                    });
                }
            },
            |ui| {
                let rollback_btn = Button::new(
                    RichText::new(tr(lang, "restore.restore_snapshot")).color(colors.accent),
                )
                .fill(colors.bg_card_hover)
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(200.0, 26.0));
                if ui.add_enabled(restorable, rollback_btn).clicked() {
                    stage_rollback = Some((snap.id.clone(), snap.description.clone()));
                }

                ui.add_space(3.0);

                ui.horizontal(|ui| {
                    let inspect_btn = Button::new(tr(lang, "restore.inspect_snapshot"))
                        .fill(colors.bg_card_hover)
                        .stroke(Stroke::new(1.0_f32, colors.border))
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(96.0, 24.0));
                    if ui.add(inspect_btn).clicked() {
                        inspect_id = Some(snap.id.clone());
                    }

                    let export_btn = Button::new(tr(lang, "restore.export_snapshot"))
                        .fill(colors.bg_card_hover)
                        .stroke(Stroke::new(1.0_f32, colors.border))
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(96.0, 24.0));
                    if ui.add(export_btn).clicked() {
                        export_id = Some(snap.id.clone());
                    }
                });

                ui.add_space(3.0);

                let delete_btn = Button::new(
                    RichText::new(tr(lang, "restore.delete_snapshot")).color(colors.danger),
                )
                .fill(colors.bg_card_hover)
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(200.0, 24.0));
                if ui.add(delete_btn).clicked() {
                    stage_delete = Some((snap.id.clone(), snap.description.clone()));
                }
            },
        );
        ui.add_space(4.0);
    }

    // ---- Inspect panel ----
    if let Some(id) = &inspect_id {
        if let Some(snap) = state.snapshots.iter().find(|s| &s.id == id) {
            ui.add_space(6.0);
            render_inspector(ui, snap, &colors);
        }
    }

    // ---- Apply collected actions ----
    if create_snapshot {
        let snap = crate::restore::snapshots::create_snapshot("Manual User Snapshot");
        state.set_toast(&format!("Created snapshot: {}", snap.id));
        state.refresh_snapshots();
    }

    if create_vss {
        match crate::restore::vss_point::create_windows_restore_point("Manual user request") {
            Ok(msg) => state.set_toast(&msg),
            Err(e) => state.record_event(
                "Restore",
                &format!("{} {}", tr(lang, "restore.vss_failed"), e),
                false,
            ),
        }
    }

    if let Some((id, label)) = stage_rollback {
        state.request_confirm(PendingConfirm::RollbackSnapshot { id, label });
    }

    if let Some((id, label)) = stage_delete {
        state.request_confirm(PendingConfirm::DeleteSnapshot { id, label });
    }

    if let Some(id) = export_id {
        let destination = crate::restore::snapshots::snapshots_dir()
            .to_string_lossy()
            .to_string();
        state.request_action(Action::ExportSnapshot {
            snapshot_id: id,
            destination,
        });
    }
}

/// Detail panel listing exactly what one snapshot recorded.
///
/// Built from the snapshot's own entries rather than a summary field, so what
/// the user reads is the actual restore set.
fn render_inspector(ui: &mut Ui, snap: &Snapshot, colors: &crate::app::theme::FluentColors) {
    card_container(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(tr(Lang::En, "restore.inspect_snapshot"))
                    .size(14.0)
                    .strong(),
            );
            ui.label(
                RichText::new(&snap.description)
                    .size(11.5)
                    .color(colors.text_secondary),
            );
            status_badge(ui, &snap.id, (100, 116, 139));
        });
        ui.add_space(6.0);

        let mut rows: Vec<(String, String)> = Vec::new();

        for entry in &snap.registry_entries {
            let value = match entry.previous_value {
                Some(v) => format!("{} = {} (DWORD)", entry.value_name, v),
                None => format!("{} (value was absent)", entry.value_name),
            };
            rows.push((format!("{}\\{}", entry.hive, entry.path), value));
        }
        for entry in &snap.string_entries {
            let value = match &entry.previous_value {
                Some(v) => format!("{} = {}", entry.value_name, v),
                None => format!("{} (value was absent)", entry.value_name),
            };
            rows.push((format!("{}\\{}", entry.hive, entry.path), value));
        }
        for entry in &snap.service_entries {
            rows.push((
                format!("Service: {}", entry.service_name),
                format!("startup was {}", entry.previous_startup),
            ));
        }
        for entry in &snap.task_entries {
            let xml_note = if entry.xml_readable {
                "XML captured"
            } else {
                "XML not readable"
            };
            rows.push((
                format!("Task: {}", entry.task_path),
                format!(
                    "was {} ({})",
                    if entry.previous_enabled {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    xml_note
                ),
            ));
        }
        for entry in &snap.power_entries {
            let ac = entry
                .previous_ac_value
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string());
            let dc = entry
                .previous_dc_value
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string());
            rows.push((
                format!("Power: {}", entry.setting_name),
                format!("AC {} / DC {}", ac, dc),
            ));
        }
        for entry in &snap.network_entries {
            let label = if entry.adapter_label.is_empty() {
                entry.interface_guid.clone()
            } else {
                entry.adapter_label.clone()
            };
            let value = entry
                .previous_value
                .clone()
                .unwrap_or_else(|| "(absent)".to_string());
            rows.push((
                format!("Network: {}", label),
                format!("{} was {}", entry.value_name, value),
            ));
        }
        for entry in &snap.profile_entries {
            rows.push((
                format!("Profile: {}", entry.profile_name),
                format!("{} step(s) applied", entry.applied_steps.len()),
            ));
        }

        if rows.is_empty() {
            ui.label(
                RichText::new(tr(Lang::En, "restore.restore_unavailable"))
                    .size(11.5)
                    .color(colors.text_muted),
            );
            return;
        }

        egui::ScrollArea::vertical()
            .max_height(240.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for (target, detail) in &rows {
                    ui.horizontal_wrapped(|ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(target)
                                    .size(10.5)
                                    .monospace()
                                    .color(colors.text_secondary),
                            )
                            .truncate(),
                        );
                        ui.label(RichText::new(detail).size(10.5).color(colors.text_muted));
                    });
                }
            });
    });
}
