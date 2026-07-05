//! Project settings tab: quality, organisation flags, and project actions.

use egui_material_icons::icons::{ICON_EXPAND_LESS, ICON_EXPAND_MORE};

use crate::app::YmdApp;
use crate::app::theme;
use crate::config::CoverSize;
use crate::download::quality::Quality;
use crate::project::ProjectSettings;

const COVER_SIZES: [CoverSize; 5] = [
    CoverSize::Px200,
    CoverSize::Px400,
    CoverSize::Px600,
    CoverSize::Px800,
    CoverSize::Px1000,
];

pub fn show(ui: &mut egui::Ui, app: &mut YmdApp) {
    egui::Panel::bottom("project_actions")
        .frame(
            egui::Frame::new()
                .fill(theme::BG_CONTENT)
                .inner_margin(egui::Margin {
                    left: 0,
                    right: 0,
                    top: 12,
                    bottom: 0,
                }),
        )
        .show_separator_line(false)
        .show(ui, |ui| {
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let switch_btn = egui::Button::new(
                    egui::RichText::new("Сменить проект").color(theme::TEXT_PRIMARY),
                )
                .fill(theme::SECONDARY_BG)
                .corner_radius(8.0);
                if ui
                    .add_sized([ui.available_width() * 0.5 - 4.0, 32.0], switch_btn)
                    .clicked()
                {
                    app.switch_to_project_picker();
                }

                ui.add_space(8.0);

                let open_btn = egui::Button::new(
                    egui::RichText::new("Открыть в проводнике").color(theme::TEXT_PRIMARY),
                )
                .fill(theme::SECONDARY_BG)
                .corner_radius(8.0);
                if ui
                    .add_sized([ui.available_width(), 32.0], open_btn)
                    .clicked()
                {
                    open_project_root(app);
                }
            });
            ui.add_space(4.0);
        });

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(4.0);
        if let Some(name) = project_display_name(app) {
            ui.label(theme::heading(&name, 17.0));
            ui.add_space(8.0);
        }
        show_settings_grid(ui, app);
    });
}

fn open_project_root(app: &YmdApp) {
    let Some(proj) = &app.current_project else {
        return;
    };
    let Ok(guard) = proj.lock() else { return };
    let _ = open::that(&guard.root);
}

fn project_display_name(app: &YmdApp) -> Option<String> {
    app.current_project
        .as_ref()
        .and_then(|p| p.lock().ok().map(|g| g.display_name().to_owned()))
}

fn show_settings_grid(ui: &mut egui::Ui, app: &mut YmdApp) {
    let mut global_changed = false;
    let mut proj_changed = false;
    let mut proj_snap: Option<ProjectSettings> = app
        .current_project
        .as_ref()
        .and_then(|p| p.lock().ok().map(|g| g.settings.clone()));

    egui::Grid::new("settings_grid")
        .num_columns(2)
        .spacing([16.0, 10.0])
        .show(ui, |ui| {
            ui.label("Параллельные загрузки:");
            let slider_changed = ui
                .scope(|ui| {
                    let visuals = &mut ui.style_mut().visuals;
                    visuals.widgets.inactive.bg_fill = theme::SECONDARY_BG_HOVER;
                    visuals.widgets.hovered.bg_fill = theme::OUTLINE;
                    visuals.widgets.active.bg_fill = theme::OUTLINE;
                    visuals.selection.bg_fill = theme::ACCENT;
                    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);
                    ui.add(
                        egui::Slider::new(&mut app.settings.max_parallel_downloads, 1..=8)
                            .trailing_fill(true),
                    )
                    .changed()
                })
                .inner;
            if slider_changed {
                global_changed = true;
            }
            ui.end_row();

            if let Some(ref mut proj) = proj_snap {
                ui.separator();
                ui.separator();
                ui.end_row();

                ui.label("Качество скачивания:");
                egui::ComboBox::from_id_salt("quality_combo")
                    .selected_text(proj.quality.label())
                    .show_ui(ui, |ui| {
                        for quality in Quality::all() {
                            if ui
                                .selectable_value(&mut proj.quality, quality, quality.label())
                                .changed()
                            {
                                proj_changed = true;
                            }
                        }
                    });
                ui.end_row();

                ui.label("Разрешение обложки:");
                egui::ComboBox::from_id_salt("cover_size_combo")
                    .selected_text(format!(
                        "{}×{}",
                        proj.cover_size.pixels(),
                        proj.cover_size.pixels()
                    ))
                    .show_ui(ui, |ui| {
                        for size in COVER_SIZES {
                            if ui
                                .selectable_value(
                                    &mut proj.cover_size,
                                    size,
                                    format!("{}×{}", size.pixels(), size.pixels()),
                                )
                                .changed()
                            {
                                proj_changed = true;
                            }
                        }
                    });
                ui.end_row();

                ui.label("Умная организация:");
                ui.horizontal(|ui| {
                    if ui
                        .checkbox(&mut proj.smart_library_organization, "Включить")
                        .changed()
                    {
                        proj_changed = true;
                        if !proj.smart_library_organization {
                            app.smart_org_expanded = false;
                        }
                    }

                    if proj.smart_library_organization {
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

                if app.smart_org_expanded && proj.smart_library_organization {
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
                                        &mut proj.track_indexing,
                                        "Индексация треков (01 -, 02 - …)",
                                    )
                                    .changed()
                                {
                                    proj_changed = true;
                                }
                                ui.add_space(4.0);
                                if ui
                                    .checkbox(
                                        &mut proj.album_year_in_folder,
                                        "Год в названии папки альбома",
                                    )
                                    .changed()
                                {
                                    proj_changed = true;
                                }
                                ui.add_space(4.0);
                                if ui
                                    .checkbox(
                                        &mut proj.download_album_cover,
                                        "Обложка альбома (cover.jpg)",
                                    )
                                    .on_hover_text("Сохраняет cover.jpg в папку альбома.")
                                    .changed()
                                {
                                    proj_changed = true;
                                }
                                ui.add_space(4.0);
                                if ui
                                    .checkbox(
                                        &mut proj.download_artist_image,
                                        "Фото исполнителя (artist.jpg)",
                                    )
                                    .on_hover_text("Сохраняет artist.jpg в папку исполнителя.")
                                    .changed()
                                {
                                    proj_changed = true;
                                }
                                ui.add_space(2.0);
                            });
                        });
                    ui.end_row();
                }
            }
        });

    if global_changed {
        persist_global(app);
    }
    if proj_changed {
        persist_project(proj_snap, app);
    }
}

fn persist_global(app: &YmdApp) {
    if let Err(err) = app.settings.save() {
        tracing::warn!(%err, "failed to save settings");
    }
}

fn persist_project(settings: Option<ProjectSettings>, app: &YmdApp) {
    let Some(new_settings) = settings else { return };
    let Some(proj_arc) = &app.current_project else {
        return;
    };
    let Ok(mut guard) = proj_arc.lock() else {
        return;
    };
    guard.settings = new_settings;
    if let Err(err) = guard.save() {
        tracing::warn!(%err, "failed to save project settings");
    }
}
