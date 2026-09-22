//! Storage Analyzer view.
//!
//! WHY the view is shaped this way:
//!
//! * Nothing here deletes anything. The analyzer measures, and every reclaim
//!   path stays in the Storage Cleaner with its own safety workflow, so the view
//!   carries no destructive control at all — not even a disabled one, which
//!   would suggest the capability exists here.
//! * A partial scan is presented as partial. The tree records whether the entry
//!   budget ran out, the user cancelled, or directories were skipped; each of
//!   those is stated on the card rather than leaving a truncated tree looking
//!   complete.
//! * Drill-down is honest about what it can do. [`StorageTree`] holds the
//!   children of the directory the scan measured, and the background scan always
//!   roots at the system drive, so opening a child directory cannot produce its
//!   entries from cached data. Selecting one records the intent and says plainly
//!   that the tree does not cover it yet, instead of rendering an empty list that
//!   would read as "this folder is empty".
//! * Thresholds come from the live config already held by [`AppState`] rather
//!   than a fresh `AppConfig::load()` per frame: same values, no TOML read in the
//!   render loop.

use crate::app::components::{
    action_card, bento_card_sized, card_container, end_two_columns, progress_track_row,
    start_two_columns, status_badge, view_header,
};
use crate::app::state::AppState;
use crate::app::theme::get_colors;
use crate::app::worker::ScanKind;
use crate::core::i18n::{tr, Lang};
use crate::storage::analyzer::{bucket_percentages, format_bytes, top_entries};
use crate::storage::models::{LargeFile, StorageTree};
use eframe::egui::{self, Button, Color32, RichText, Rounding, Stroke, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let colors = get_colors(ui.visuals().dark_mode);
    let lang = state.lang;
    let scanning = state.is_scanning(ScanKind::Storage);

    // 1. Header: scan when idle, cancel while a scan is in flight
    view_header(
        ui,
        tr(lang, "storage.title"),
        tr(lang, "storage.subtitle"),
        |ui| {
            if scanning {
                let cancel_btn = Button::new(format!("✕ {}", tr(lang, "storage.cancel_scan")))
                    .fill(colors.bg_card)
                    .stroke(Stroke::new(1.0_f32, colors.danger))
                    .rounding(Rounding::same(6.0));
                if ui.add(cancel_btn).clicked() {
                    state.storage_scan_cancel.cancel();
                }
            } else {
                let scan_btn = Button::new(format!("↻ {}", tr(lang, "storage.scan_drive")))
                    .fill(colors.bg_card)
                    .stroke(Stroke::new(1.0_f32, colors.border))
                    .rounding(Rounding::same(6.0));
                if ui.add(scan_btn).clicked() {
                    // A fresh scan clears the previous cancellation, otherwise
                    // the banner would claim this scan was cancelled too.
                    state.storage_scan_cancel.reset();
                    state.storage_current_path = None;
                    state.refresh_storage();
                }
            }
        },
    );

    // 9. A cancelled token is reported before anything else, so partial numbers
    //    below are never read as a finished scan.
    if state.storage_scan_cancel.is_cancelled() {
        ui.label(
            RichText::new(format!("⚠ {}", tr(lang, "storage.cancelled")))
                .size(11.5)
                .color(colors.warning),
        );
        ui.add_space(6.0);
    }

    if scanning {
        ui.label(
            RichText::new(tr(lang, "storage.scanning"))
                .size(11.5)
                .color(colors.secondary),
        );
        ui.add_space(6.0);
    }

    // 2. No scan yet: explain what the analyzer is and stop.
    if state.storage_tree.is_none() {
        card_container(ui, |ui| {
            ui.label(
                RichText::new(tr(lang, "storage.informational_note"))
                    .size(12.0)
                    .color(colors.text_muted),
            );
        });
        return;
    }

    let large_file_mb = state.config.storage.large_file_mb;
    let old_file_days = state.config.storage.old_file_days;

    // Collected during the render and applied after it: the cards hold an
    // immutable borrow of the tree, so state cannot be mutated inside them.
    let mut drill_request: Option<String> = None;
    let mut go_up = false;
    let mut rescan_requested = false;

    {
        let Some(tree) = state.storage_tree.as_ref() else {
            return;
        };

        // 3 + 4. Drive usage and bucket breakdown, side by side
        let col_gap = 12.0_f32;
        let (mut left_ui, mut right_ui, pos, w) = start_two_columns(ui, 0.50, col_gap);

        bento_card_sized(
            &mut left_ui,
            tr(lang, "storage.usage_overview"),
            "🗂",
            None,
            Some(215.0),
            |ui| usage_card(ui, tree, lang),
        );

        bento_card_sized(
            &mut right_ui,
            tr(lang, "storage.folder_breakdown"),
            "📁",
            None,
            Some(215.0),
            |ui| {
                for (bucket, bytes, pct) in bucket_percentages(tree) {
                    let rgb = bucket.color_rgb();
                    progress_track_row(
                        ui,
                        tr(lang, bucket.i18n_key()),
                        &format!("{}  ({:.0}%)", format_bytes(bytes), pct),
                        pct / 100.0,
                        Color32::from_rgb(rgb.0, rgb.1, rgb.2),
                    );
                }
            },
        );

        end_two_columns(ui, left_ui, right_ui, pos, w);

        ui.add_space(10.0);

        // 5. Directory drill-down over the scanned tree
        card_container(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        tr(lang, "storage.breadcrumb"),
                        tree.current_path
                    ))
                    .size(11.5)
                    .color(colors.text_muted),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if state.storage_current_path.is_some() {
                        let up_btn = Button::new(tr(lang, "storage.up_level"))
                            .fill(colors.bg_card_hover)
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(70.0, 24.0));
                        if ui.add(up_btn).clicked() {
                            go_up = true;
                        }
                    }
                });
            });

            drill_down_panel(
                ui,
                state.storage_current_path.as_deref(),
                tree,
                lang,
                &mut rescan_requested,
            );

            ui.add_space(8.0);

            for entry in top_entries(tree, 50) {
                let rgb = entry.bucket.color_rgb();
                let fraction = entry.fraction_of(tree.total_bytes);
                let entry_path = entry.path.clone();
                let is_dir = entry.is_dir;

                action_card(
                    ui,
                    90.0,
                    |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(&entry.name).size(13.0).strong());
                            status_badge(ui, tr(lang, entry.bucket.i18n_key()), rgb);
                            if entry.partial {
                                status_badge(ui, tr(lang, "common.unknown"), (148, 163, 184));
                            }
                        });
                        ui.add_space(2.0);
                        note(
                            ui,
                            &size_files_line(
                                entry.size_bytes,
                                entry.file_count,
                                tr(lang, "common.files"),
                            ),
                            colors.text_secondary,
                        );
                        ui.add_space(4.0);
                        progress_track_row(
                            ui,
                            "",
                            "",
                            fraction,
                            Color32::from_rgb(rgb.0, rgb.1, rgb.2),
                        );
                    },
                    |ui| {
                        // Only directories can be opened; files have nothing under them.
                        if is_dir {
                            let open_btn = Button::new(tr(lang, "storage.drill_down"))
                                .fill(colors.bg_card_hover)
                                .rounding(Rounding::same(6.0))
                                .min_size(Vec2::new(80.0, 26.0));
                            if ui.add(open_btn).clicked() {
                                drill_request = Some(entry_path);
                            }
                        }
                    },
                );
                ui.add_space(4.0);
            }
        });

        ui.add_space(10.0);

        // 6. Large files
        file_list_card(
            ui,
            tr(lang, "storage.large_files"),
            &format!(
                "{} {} {}",
                tr(lang, "storage.min_size"),
                large_file_mb,
                tr(lang, "common.mb")
            ),
            &tree.large_files,
            tr(lang, "storage.no_large_files"),
        );

        ui.add_space(10.0);

        // 7. Old files
        file_list_card(
            ui,
            tr(lang, "storage.old_files"),
            &format!("{} {}", tr(lang, "storage.min_age"), old_file_days),
            &tree.old_files,
            tr(lang, "storage.no_old_files"),
        );

        ui.add_space(10.0);

        // 8. Cache locations. Read-only: reclaiming one goes through the Cleaner.
        card_container(ui, |ui| {
            ui.label(
                RichText::new(tr(lang, "storage.cache_usage"))
                    .size(14.5)
                    .strong(),
            );
            ui.add_space(6.0);

            if tree.cache_locations.is_empty() {
                note(ui, tr(lang, "common.none"), colors.text_muted);
            }

            for cache in &tree.cache_locations {
                ui.label(RichText::new(&cache.label).size(12.5).strong());
                note(
                    ui,
                    &size_files_line(cache.size_bytes, cache.file_count, tr(lang, "common.files")),
                    colors.text_secondary,
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(&cache.path)
                            .size(10.5)
                            .color(colors.text_muted),
                    )
                    .truncate(),
                );
                ui.add_space(4.0);
            }
        });
    }

    if let Some(path) = drill_request {
        state.storage_current_path = Some(path);
    }
    if go_up {
        state.storage_current_path = None;
    }
    if rescan_requested {
        state.storage_current_path = None;
        state.storage_scan_cancel.reset();
        state.refresh_storage();
    }

    // 8b. Footnote: the analyzer is informational, the Cleaner is the reclaim path
    ui.add_space(10.0);
    card_container(ui, |ui| {
        ui.label(
            RichText::new(tr(lang, "storage.informational_note"))
                .size(11.0)
                .color(colors.text_muted),
        );
    });
}

/// Drive usage body: measured totals, the used fraction, and every reason the
/// scan may be short of complete.
fn usage_card(ui: &mut Ui, tree: &StorageTree, lang: Lang) {
    let colors = get_colors(ui.visuals().dark_mode);

    ui.label(
        RichText::new(format!(
            "{} / {}",
            format_bytes(tree.used_bytes),
            format_bytes(tree.total_bytes)
        ))
        .size(16.0)
        .strong(),
    );
    ui.add_space(6.0);
    progress_track_row(
        ui,
        tr(lang, "dash.system_volume_usage"),
        &format_bytes(tree.free_bytes),
        tree.used_fraction(),
        colors.secondary,
    );

    ui.add_space(2.0);
    count_note(
        ui,
        tr(lang, "storage.files_scanned"),
        tree.files_scanned,
        colors.text_secondary,
    );
    count_note(
        ui,
        tr(lang, "storage.dirs_scanned"),
        tree.dirs_scanned,
        colors.text_secondary,
    );

    // A partial tree says why it is partial, so a truncated scan is never
    // presented as a complete one.
    if tree.is_partial() {
        ui.add_space(6.0);
        if tree.limit_reached {
            note(ui, tr(lang, "storage.limit_reached"), colors.warning);
        }
        if tree.cancelled {
            note(ui, tr(lang, "storage.cancelled"), colors.warning);
        }
        if tree.skipped_inaccessible > 0 {
            count_note(
                ui,
                tr(lang, "storage.skipped_inaccessible"),
                tree.skipped_inaccessible,
                colors.text_muted,
            );
        }
        if tree.skipped_reparse > 0 {
            count_note(
                ui,
                tr(lang, "storage.skipped_reparse"),
                tree.skipped_reparse,
                colors.text_muted,
            );
        }
    }
}

/// The drill-down status panel.
///
/// The background scan always roots at the system drive and [`StorageTree`]
/// only holds the children of the directory it measured, so a selected child
/// directory cannot have its own entries rendered from cached data. This panel
/// therefore reports what the tree does know about the selection — its measured
/// size when it is a top-level child — and, when it knows nothing, says so
/// plainly. Rendering an empty list instead would read as "this folder is
/// empty", which would be a fabricated result.
fn drill_down_panel(
    ui: &mut Ui,
    selected: Option<&str>,
    tree: &StorageTree,
    lang: Lang,
    rescan_requested: &mut bool,
) {
    let colors = get_colors(ui.visuals().dark_mode);

    let Some(selected) = selected else { return };
    if selected == tree.current_path {
        return;
    }

    ui.add_space(6.0);
    ui.label(
        RichText::new(format!("{} {}", tr(lang, "storage.breadcrumb"), selected))
            .size(11.5)
            .color(colors.text_secondary),
    );

    match tree.entries.iter().find(|e| e.path == selected) {
        Some(entry) => {
            let rgb = entry.bucket.color_rgb();
            ui.horizontal_wrapped(|ui| {
                note(
                    ui,
                    &size_files_line(entry.size_bytes, entry.file_count, tr(lang, "common.files")),
                    colors.text_secondary,
                );
                status_badge(ui, tr(lang, entry.bucket.i18n_key()), rgb);
            });
        }
        None => note(ui, tr(lang, "storage.limit_reached"), colors.warning),
    }

    ui.add_space(4.0);
    let rescan_btn = Button::new(format!("↻ {}", tr(lang, "common.rescan")))
        .fill(colors.bg_card_hover)
        .stroke(Stroke::new(1.0_f32, colors.border))
        .rounding(Rounding::same(6.0))
        .min_size(Vec2::new(110.0, 26.0));
    if ui.add(rescan_btn).clicked() {
        *rescan_requested = true;
    }
}

/// A card listing one capped set of files (large or old). Both sections render
/// identically apart from their heading, threshold, and empty message, so they
/// share this rather than drifting apart as two copies.
fn file_list_card(
    ui: &mut Ui,
    heading: &str,
    threshold: &str,
    files: &[LargeFile],
    empty_msg: &str,
) {
    let colors = get_colors(ui.visuals().dark_mode);

    card_container(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(heading).size(14.5).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(threshold).size(11.0).color(colors.text_muted));
            });
        });
        ui.add_space(6.0);

        if files.is_empty() {
            note(ui, empty_msg, colors.text_muted);
        }

        // Capped so a machine with thousands of candidates cannot stall the frame.
        for file in files.iter().take(50) {
            file_row(
                ui,
                &file.path,
                &format_bytes(file.size_bytes),
                file.age_days,
                colors.text_muted,
                colors.accent,
            );
        }
    });
}

/// "12.4 GB  •  381 files" — the measured-size line shared by the entry,
/// drill-down, and cache rows so they never drift apart. `files_label` is the
/// caller's localized word for "files", not a literal, so this stays translated.
fn size_files_line(bytes: u64, file_count: usize, files_label: &str) -> String {
    format!("{}  •  {} {}", format_bytes(bytes), file_count, files_label)
}

/// A one-line explanatory label. Every "why the numbers look like this" string
/// in this view goes through here so they stay visually identical.
fn note(ui: &mut Ui, text: &str, color: Color32) {
    ui.label(RichText::new(text).size(11.0).color(color));
}

/// A "Label: 12" line, for the scan counters that need their number beside a
/// translated label without hard-coding the separator into a format string.
fn count_note(ui: &mut Ui, label: &str, count: usize, color: Color32) {
    note(ui, &format!("{}: {}", label, count), color);
}

/// One large/old file row: path truncated on the left of a fixed column, size
/// and age right-aligned in bounded columns so a long path can never push them
/// off the card. Same column discipline as `components::stat_row`.
fn file_row(
    ui: &mut Ui,
    path: &str,
    size: &str,
    age_days: Option<u64>,
    muted: Color32,
    accent: Color32,
) {
    let total_w = ui.available_width();
    let value_w = 130.0_f32;
    let path_w = (total_w - value_w - 6.0).max(80.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.allocate_ui_with_layout(
            Vec2::new(path_w, 18.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add(egui::Label::new(RichText::new(path).size(11.0).color(muted)).truncate());
            },
        );
        ui.allocate_ui_with_layout(
            Vec2::new(value_w, 18.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                // A bare day count: the card header carries the threshold this
                // number is meant to be compared against.
                if let Some(age) = age_days {
                    ui.label(RichText::new(age.to_string()).size(10.5).color(muted));
                }
                ui.label(RichText::new(size).size(11.0).strong().color(accent));
            },
        );
    });
    ui.add_space(2.0);
}
