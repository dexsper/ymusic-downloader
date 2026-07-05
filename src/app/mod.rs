//! Root application type for eframe and screen routing.

pub mod theme;
mod ui;

use std::sync::{Arc, Mutex};

use crate::api::client::ApiClient;
use crate::config::Settings;
use crate::project::Project;

/// Active sidebar tab on the main screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Downloads,
    Project,
}

/// Active application screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Splash,
    /// Shown when the user is not authenticated.
    Auth,
    /// Project picker: select an existing project or create a new one.
    ProjectPicker,
    Main,
}

/// Named logo positions used by [`LogoAnim`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoTarget {
    /// Center of screen, 280 px wide — splash resting position.
    Splash,
    /// Top-center, 240 px wide — auth screen and project picker.
    AuthCenter,
    /// Top of the left sidebar — main screen.
    Sidebar,
}

/// Smooth logo animation between two named positions.
///
/// Every screen draws the logo by querying this struct; transitions simply update `from`/`to`
/// and reset `started`.
pub struct LogoAnim {
    pub from: LogoTarget,
    pub to: LogoTarget,
    pub started: std::time::Instant,
    pub duration_secs: f32,
}

impl LogoAnim {
    /// Logo snaps immediately to `target` (no animation).
    pub fn snap(target: LogoTarget) -> Self {
        Self {
            from: target,
            to: target,
            started: std::time::Instant::now(),
            duration_secs: 0.001,
        }
    }

    /// Logo animates from `from` to `to` over `duration_secs` seconds.
    pub fn animate(from: LogoTarget, to: LogoTarget, duration_secs: f32) -> Self {
        Self { from, to, started: std::time::Instant::now(), duration_secs }
    }

    /// Linear progress `[0.0, 1.0]`.
    #[must_use]
    pub fn t(&self) -> f32 {
        (self.started.elapsed().as_secs_f32() / self.duration_secs).min(1.0)
    }

    /// Eased progress (ease-out-quart).
    #[must_use]
    pub fn ease(&self) -> f32 {
        let t = self.t();
        1.0 - (1.0 - t).powi(4)
    }

    /// Whether the animation has fully completed.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.t() >= 1.0
    }
}

impl Default for LogoAnim {
    fn default() -> Self {
        Self::snap(LogoTarget::Splash)
    }
}

/// Root application state, holding settings, the API client, and UI state.
pub struct YmdApp {
    pub settings: Settings,
    pub screen: Screen,
    pub api_client: Arc<ApiClient>,
    pub runtime: tokio::runtime::Runtime,
    pub egui_ctx: egui::Context,
    pub auth_ui: ui::auth::AuthUiState,
    pub queue: crate::download::queue::DownloadQueue,
    pub link_input: String,
    pub show_account_popup: bool,
    pub smart_org_expanded: bool,
    pub active_tab: SidebarTab,
    pub splash_start: std::time::Instant,
    pub logo_texture: Option<egui::TextureHandle>,
    /// Drives the fade-in of auth screen content (text + button) independently of the logo.
    pub auth_started: std::time::Instant,
    /// Unified logo animation state — used by every screen.
    pub logo_anim: LogoAnim,
    /// Currently open project (set after the user picks or creates one on the project screen).
    pub current_project: Option<Arc<Mutex<Project>>>,
}

impl YmdApp {
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        egui_material_icons::initialize(&cc.egui_ctx);

        let mut settings = Settings::load().unwrap_or_else(|err| {
            tracing::warn!(%err, "failed to load settings, using defaults");
            Settings::default()
        });

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to create Tokio runtime");

        let api_client = ApiClient::new(&settings.auth).expect("failed to create HTTP client");
        if settings.auth.device_uuid.is_none() || settings.auth.device_id.is_none() {
            settings.auth.device_uuid = Some(api_client.device_uuid().to_owned());
            settings.auth.device_id = Some(api_client.device_id().to_owned());
            if let Err(err) = settings.save() {
                tracing::warn!(%err, "failed to save device_id/uuid");
            }
        }

        let api_client = Arc::new(api_client);
        let auth_ui = ui::auth::AuthUiState::default();
        if settings.auth.token.is_some() {
            ui::auth::spawn_account_check(
                &runtime,
                api_client.clone(),
                auth_ui.status.clone(),
                auth_ui.avatar_bytes.clone(),
                cc.egui_ctx.clone(),
            );
        }

        Self {
            settings,
            screen: Screen::Splash,
            api_client,
            runtime,
            egui_ctx: cc.egui_ctx.clone(),
            auth_ui,
            queue: crate::download::queue::DownloadQueue::default(),
            link_input: String::new(),
            show_account_popup: false,
            smart_org_expanded: false,
            active_tab: SidebarTab::Downloads,
            splash_start: std::time::Instant::now(),
            logo_texture: None,
            auth_started: std::time::Instant::now(),
            logo_anim: LogoAnim::default(),
            current_project: None,
        }
    }

    /// Opens a project, records it in recent list, sets the logo flying to the sidebar,
    /// and navigates to the main screen.
    pub fn open_project(&mut self, path: std::path::PathBuf) {
        match Project::open(path.clone()) {
            Ok(project) => {
                self.current_project = Some(Arc::new(Mutex::new(project)));
                self.settings.push_recent_project(path);
                if let Err(err) = self.settings.save() {
                    tracing::warn!(%err, "failed to save recent projects");
                }
                self.logo_anim = LogoAnim::animate(LogoTarget::AuthCenter, LogoTarget::Sidebar, 0.9);
                self.screen = Screen::Main;
            }
            Err(err) => {
                tracing::warn!(%err, "failed to open project");
            }
        }
    }

    /// Switches to the project picker, animating the logo back to the auth-center position.
    pub fn switch_to_project_picker(&mut self) {
        self.logo_anim = LogoAnim::animate(LogoTarget::Sidebar, LogoTarget::AuthCenter, 0.55);
        self.active_tab = SidebarTab::Downloads;
        self.screen = Screen::ProjectPicker;
    }
}

impl eframe::App for YmdApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, theme::BG_CONTENT);

        egui::Panel::top("title_bar")
            .exact_size(ui::widgets::TITLE_H)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_CONTENT)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let drag_rect = egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2(rect.max.x - ui::widgets::BTN_W * 3.0, rect.max.y),
                );
                let drag = ui.interact(
                    drag_rect,
                    egui::Id::new("win_drag"),
                    egui::Sense::click_and_drag(),
                );
                if drag.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
            });

        ui::widgets::show_win_controls(ui.ctx());
        match self.screen {
            Screen::Splash => ui::splash::show(ui, self),
            Screen::Auth => ui::auth_screen::show(ui, self),
            Screen::ProjectPicker => ui::project_screen::show(ui, self),
            Screen::Main => ui::main_screen::show(ui, self),
        }
    }
}
