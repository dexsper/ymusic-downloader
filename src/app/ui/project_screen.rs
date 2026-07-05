//! Project picker screen: shown after successful authentication.
//!
//! The logo is drawn by the unified [`crate::app::ui::widgets::draw_logo`] system — it
//! seamlessly continues whatever animation was started by the previous screen (splash or auth).

use crate::app::ui::widgets;
use crate::app::{LogoTarget, YmdApp, theme};
use crate::project;

/// Approximate logo height at AuthCenter size (240px wide), used to place content below it.
const LOGO_H_APPROX: f32 = 240.0 * 0.35;
/// Distance from title-bar to logo centre on this screen.
const LOGO_CY_FROM_TOP: f32 = crate::app::ui::widgets::TITLE_H + 72.0;
/// Top of the content block below the logo.
const CONTENT_TOP: f32 = LOGO_CY_FROM_TOP + LOGO_H_APPROX * 0.5 + 28.0;

pub fn show(ui: &mut egui::Ui, app: &mut YmdApp) {
    // Draw logo via unified system — animation continues from wherever it currently is.
    let layer = egui::LayerId::new(egui::Order::Middle, egui::Id::new("logo"));
    widgets::draw_logo(ui.ctx(), app, layer);

    let rect = ui.max_rect();
    let cx = rect.center().x;

    // Content block — only show once the logo is close to its final position.
    let logo_progress = if app.logo_anim.to == LogoTarget::AuthCenter {
        app.logo_anim.ease()
    } else {
        1.0
    };
    if logo_progress < 0.3 {
        return;
    }

    let available_w = (rect.width() - 80.0).min(480.0);
    let block_x = cx - available_w * 0.5;
    let block_top = CONTENT_TOP;

    let heading_font =
        egui::FontId::new(24.0, egui::FontFamily::Name(theme::HEADING_FAMILY.into()));

    ui.painter().text(
        egui::pos2(cx, block_top),
        egui::Align2::CENTER_TOP,
        "Выберите проект",
        heading_font,
        theme::TEXT_PRIMARY,
    );

    let list_top = block_top + 40.0;
    let item_h = 52.0;
    let item_gap = 8.0;

    // Only show entries that still have a project manifest on disk.
    let recent: Vec<_> = app
        .settings
        .recent_projects
        .iter()
        .filter(|p| crate::project::Project::exists_at(p))
        .cloned()
        .collect();

    // Two-line block heights:  title ~17 px + gap 3 px + path ~13 px = 33 px
    // Vertical padding inside item_h = 52 px: (52 - 33) / 2 ≈ 9.5 px
    const TITLE_FONT: f32 = 15.0;
    const PATH_FONT: f32 = 11.5;
    const LINE_GAP: f32 = 3.0;
    const TITLE_H: f32 = 17.0; // approximate rendered height
    const BLOCK_H: f32 = TITLE_H + LINE_GAP + PATH_FONT; // ~31.5
    let top_pad = (item_h - BLOCK_H) * 0.5;

    for (i, path) in recent.iter().enumerate() {
        let item_y = list_top + (item_h + item_gap) * i as f32;
        let item_rect = egui::Rect::from_min_size(
            egui::pos2(block_x, item_y),
            egui::vec2(available_w, item_h),
        );

        let response = ui.interact(item_rect, egui::Id::new(("proj_item", i)), egui::Sense::click());

        let bg = if response.hovered() {
            theme::SECONDARY_BG_HOVER
        } else {
            theme::SECONDARY_BG
        };

        ui.painter()
            .rect_filled(item_rect, egui::CornerRadius::same(10), bg);

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let text_x = block_x + 16.0;

        // Project name — vertically centred in the item.
        let name = project::path_display_name(path);
        ui.painter().text(
            egui::pos2(text_x, item_y + top_pad),
            egui::Align2::LEFT_TOP,
            name,
            egui::FontId::proportional(TITLE_FONT),
            theme::TEXT_PRIMARY,
        );

        // Full path (muted, small).
        let path_str = path.display().to_string();
        ui.painter().text(
            egui::pos2(text_x, item_y + top_pad + TITLE_H + LINE_GAP),
            egui::Align2::LEFT_TOP,
            &path_str,
            egui::FontId::proportional(PATH_FONT),
            theme::TEXT_MUTED,
        );

        if response.clicked() {
            app.open_project(path.clone());
            return;
        }
    }

    // "Create new project" button — placed below recent list (or at list_top if empty).
    let btn_top = list_top + (item_h + item_gap) * recent.len() as f32 + if recent.is_empty() { 0.0 } else { 8.0 };
    let btn_rect = egui::Rect::from_min_size(
        egui::pos2(cx - 140.0, btn_top),
        egui::vec2(280.0, 44.0),
    );
    let btn = egui::Button::new(
        egui::RichText::new("Создать новый проект")
            .color(theme::ON_ACCENT)
            .strong()
            .size(14.0),
    )
    .fill(theme::ACCENT)
    .corner_radius(egui::CornerRadius::same(10));

    if ui.put(btn_rect, btn).clicked()
        && let Some(dir) = rfd::FileDialog::new().set_title("Выберите папку проекта").pick_folder()
    {
        app.open_project(dir);
    }
}
