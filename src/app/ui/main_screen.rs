//! Main screen: sidebar (logo + tabs + profile island) and content area.

use egui_material_icons::icons::{ICON_DOWNLOAD, ICON_FOLDER_OPEN};

use crate::app::theme;
use crate::app::ui::widgets::{self, SIDEBAR_W};
use crate::app::{SidebarTab, YmdApp};

/// Horizontal margin inside sidebar for tab buttons.
const TAB_MARGIN: f32 = 8.0;
/// Height of a single tab button.
const TAB_H: f32 = 38.0;
/// Gap between tab buttons.
const TAB_GAP: f32 = 4.0;

pub fn show(ui: &mut egui::Ui, app: &mut YmdApp) {
    egui::Panel::left("sidebar")
        .exact_size(SIDEBAR_W)
        .resizable(false)
        .show_separator_line(false)
        .frame(
            egui::Frame::new()
                .fill(theme::BG_CONTENT)
                .inner_margin(egui::Margin::ZERO),
        )
        .show(ui, |ui| {
            // Reserve space for the logo (drawn as layer overlay).
            // Use texture size if available, fallback to a sensible constant.
            // The sidebar panel starts just below the title bar, so panel-local y=0
            // corresponds to window y=TITLE_H. The logo center in panel coords is at
            // (30 + logo_h/2), so we skip to the logo bottom plus a small gap.
            let logo_space = if let Some(tex) = &app.logo_texture {
                let aspect = tex.size_vec2().y / tex.size_vec2().x;
                let logo_w = SIDEBAR_W - 40.0;
                let logo_h = logo_w * aspect;
                30.0 + logo_h + 24.0
            } else {
                120.0
            };
            ui.add_space(logo_space);

            show_tabs(ui, app);
        });

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(theme::BG_CONTENT)
                .inner_margin(egui::Margin::same(16)),
        )
        .show(ui, |ui| match app.active_tab {
            SidebarTab::Downloads => super::queue::show(ui, app),
            SidebarTab::Project => super::settings::show(ui, app),
        });

    // Logo drawn by the unified animation system (AuthCenter → Sidebar).
    let layer = egui::LayerId::new(egui::Order::Middle, egui::Id::new("logo"));
    widgets::draw_logo(ui.ctx(), app, layer);

    super::auth::show_island(ui.ctx(), app);
}

fn show_tabs(ui: &mut egui::Ui, app: &mut YmdApp) {
    let tab_w = SIDEBAR_W - TAB_MARGIN * 2.0;

    for (tab, icon, label) in [
        (SidebarTab::Downloads, ICON_DOWNLOAD, "Загрузки"),
        (SidebarTab::Project, ICON_FOLDER_OPEN, "Проект"),
    ] {
        let active = app.active_tab == tab;

        let (rect, resp) = ui.allocate_exact_size(
            egui::vec2(SIDEBAR_W, TAB_H + TAB_GAP),
            egui::Sense::click(),
        );
        // Inset the visual rect by the margin.
        let btn_rect = egui::Rect::from_min_size(
            egui::pos2(rect.min.x + TAB_MARGIN, rect.min.y),
            egui::vec2(tab_w, TAB_H),
        );

        let (bg, text_color, icon_color) = if active {
            (theme::SECONDARY_BG_HOVER, theme::TEXT_PRIMARY, theme::ACCENT)
        } else if resp.hovered() {
            (theme::SECONDARY_BG, theme::TEXT_PRIMARY, theme::TEXT_MUTED)
        } else {
            (egui::Color32::TRANSPARENT, theme::TEXT_MUTED, theme::TEXT_MUTED)
        };

        ui.painter().rect_filled(btn_rect, egui::CornerRadius::same(8), bg);

        // Active indicator: 3px accent bar on the left edge.
        if active {
            let bar = egui::Rect::from_min_size(
                btn_rect.min,
                egui::vec2(3.0, TAB_H),
            );
            ui.painter().rect_filled(bar, egui::CornerRadius::same(2), theme::ACCENT);
        }

        // Icon.
        let icon_x = btn_rect.min.x + 14.0;
        let icon_y = btn_rect.center().y;
        ui.painter().text(
            egui::pos2(icon_x, icon_y),
            egui::Align2::LEFT_CENTER,
            icon.codepoint,
            egui::FontId::new(18.0, icon.font_family()),
            icon_color,
        );

        // Label.
        ui.painter().text(
            egui::pos2(icon_x + 26.0, icon_y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(13.5),
            text_color,
        );

        if resp.clicked() {
            app.active_tab = tab;
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }
}
