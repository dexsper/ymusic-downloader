//! Settings screen: download quality, cover resolution, library organization, and download folder.

use egui_material_icons::icons::{ICON_EXPAND_LESS, ICON_EXPAND_MORE};

use crate::app::YmdApp;
use crate::app::theme;
use crate::config::CoverSize;
use crate::download::quality::Quality;

const COVER_SIZES: [CoverSize; 5] = [
    CoverSize::Px200,
    CoverSize::Px400,
    CoverSize::Px600,
    CoverSize::Px800,
    CoverSize::Px1000,
];

pub fn show(ui: &mut egui::Ui, app: &mut YmdApp) {
    let mut changed = false;

    ui.add_space(8.0);

    egui::Grid::new("settings_grid")
        .num_columns(2)
        .spacing([16.0, 10.0])
        .show(ui, |ui| {
            ui.label("Качество скачивания:");
            egui::ComboBox::from_id_salt("quality_combo")
                .selected_text(app.settings.quality.label())
                .show_ui(ui, |ui| {
                    for quality in Quality::all() {
                        if ui
                            .selectable_value(&mut app.settings.quality, quality, quality.label())
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Разрешение обложки:");
            egui::ComboBox::from_id_salt("cover_size_combo")
                .selected_text(format!(
                    "{}×{}",
                    app.settings.cover_size.pixels(),
                    app.settings.cover_size.pixels()
                ))
                .show_ui(ui, |ui| {
                    for size in COVER_SIZES {
                        if ui
                            .selectable_value(
                                &mut app.settings.cover_size,
                                size,
                                format!("{}×{}", size.pixels(), size.pixels()),
                            )
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Умная организация:");
            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut app.settings.smart_library_organization, "Включить")
                    .changed()
                {
                    changed = true;
                    if !app.settings.smart_library_organization {
                        app.smart_org_expanded = false;
                    }
                }

                if app.settings.smart_library_organization {
                    let icon = if app.smart_org_expanded {
                        ICON_EXPAND_LESS
                    } else {
                        ICON_EXPAND_MORE
                    };

                    let tooltip = if app.smart_org_expanded {
                        "Скрыть"
                    } else {
                        "Настройки…"
                    };
                    if egui_material_icons::icon_button(ui, icon)
                        .on_hover_text(tooltip)
                        .clicked()
                    {
                        app.smart_org_expanded = !app.smart_org_expanded;
                    }
                }
            });
            ui.end_row();

            if app.smart_org_expanded && app.settings.smart_library_organization {
                ui.label("");
                egui::Frame::new()
                    .fill(theme::SECONDARY_BG_HOVER)
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.add_space(2.0);
                            if ui
                                .checkbox(
                                    &mut app.settings.track_indexing,
                                    "Индексация треков (01 -, 02 - …)",
                                )
                                .changed()
                            {
                                changed = true;
                            }
                            ui.add_space(4.0);
                            if ui
                                .checkbox(
                                    &mut app.settings.album_year_in_folder,
                                    "Год в названии папки альбома",
                                )
                                .changed()
                            {
                                changed = true;
                            }
                            ui.add_space(2.0);
                        });
                    });
                ui.end_row();
            }

            ui.label("Параллельные загрузки:");
            let slider_changed = ui
                .scope(|ui| {
                    let visuals = &mut ui.style_mut().visuals;
                    visuals.widgets.inactive.bg_fill = theme::SECONDARY_BG_HOVER;
                    visuals.widgets.hovered.bg_fill = theme::OUTLINE;
                    visuals.widgets.active.bg_fill = theme::OUTLINE;
                    visuals.selection.bg_fill = theme::ACCENT;
                    // Track corner radius must be smaller than the thumb radius to prevent
                    // the filled progress segment from overflowing the thumb edge.
                    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);
                    ui.add(
                        egui::Slider::new(&mut app.settings.max_parallel_downloads, 1..=8)
                            .trailing_fill(true),
                    )
                    .changed()
                })
                .inner;
            if slider_changed {
                changed = true;
            }
            ui.end_row();

            ui.label("Папка загрузок:");
            ui.horizontal(|ui| {
                let path_text = app
                    .settings
                    .download_dir
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "(не выбрана)".to_owned());
                ui.label(path_text);
                if ui.button("Выбрать…").clicked()
                    && let Some(dir) = rfd::FileDialog::new().pick_folder()
                {
                    app.settings.download_dir = Some(dir);
                    changed = true;
                }
            });
            ui.end_row();
        });

    if changed && let Err(err) = app.settings.save() {
        tracing::warn!(%err, "failed to save settings");
    }
}
