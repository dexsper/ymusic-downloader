//! Settings screen: download quality, cover resolution, library organization, and download folder.

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
    ui.label(theme::heading("Настройки", 26.0));
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

            ui.label("Умная организация медиатеки:");
            if ui
                .checkbox(
                    &mut app.settings.smart_library_organization,
                    "Артист / Альбом (Год) / Диск N / Трек",
                )
                .changed()
            {
                changed = true;
            }
            ui.end_row();

            ui.label("Индексация треков:");
            if ui
                .checkbox(
                    &mut app.settings.track_indexing,
                    "Добавлять префиксы 01 -, 02 - к именам файлов",
                )
                .changed()
            {
                changed = true;
            }
            ui.end_row();

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
