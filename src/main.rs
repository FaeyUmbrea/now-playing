use now_playing::{MusicService, WebServer, Config, ObsClient, MusicServiceType, SpotifyMonitor};
use now_playing::rt::RUNTIME;
use eframe::egui;
use std::sync::Arc;
use parking_lot::RwLock as SyncRwLock;
use tokio::sync::RwLock as AsyncRwLock;
use now_playing::TemplateMode;
use rfd::FileDialog;

fn main() -> Result<(), eframe::Error> {
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 500.0])
            .with_title("Now Playing"),
        ..Default::default()
    };

    eframe::run_native(
        "Now Playing",
        options,
        Box::new(|_cc| Ok(Box::new(NowPlayingApp::new()))),
    )
}

struct NowPlayingApp {
    config: Config,
    music_service: Option<MusicService>,
    server: Option<Arc<SyncRwLock<WebServer>>>,
    obs_client: Option<Arc<AsyncRwLock<ObsClient>>>,
    status_message: String,
    obs_status: String,
    selected_tab: Tab,
    warnings: Vec<String>, // UI warnings

    // Add a field to store the OBS status receiver
    obs_status_rx: Option<std::sync::mpsc::Receiver<String>>,
}

#[derive(PartialEq)]
enum Tab {
    MusicService,
    Server,
    Obs,
    Widget,
}

impl NowPlayingApp {
    fn new() -> Self {
        let config = Config::load();
        Self {
            config,
            music_service: None,
            server: None,
            obs_client: None,
            status_message: "Not started".to_string(),
            obs_status: "Not connected".to_string(),
            selected_tab: Tab::MusicService,
            obs_status_rx: None,
            warnings: Vec::new(),
        }
    }

    fn start_services(&mut self) {
        // Create the appropriate music service
        let service = match self.config.service.service_type {
            #[cfg(target_os = "macos")]
            MusicServiceType::AppleMusic => MusicService::new_apple_music(),
            MusicServiceType::Cider => MusicService::new_cider(),
            MusicServiceType::Spotify => {
                if let Some(spotify_cfg) = &self.config.service.spotify {
                    MusicService::Spotify(SpotifyMonitor::new(&spotify_cfg.client_id, &spotify_cfg.client_secret, &spotify_cfg.redirect_uri))
                } else {
                    self.status_message = "Spotify not configured".to_string();
                    return;
                }
            }
        };

        // Start monitoring. Ensure AppleMusic (macOS), Cider, and Spotify monitoring are started.
        match &self.config.service.service_type {
            #[cfg(target_os = "macos")]
            MusicServiceType::AppleMusic => {
                // AppleMusic::start_monitoring ignores the cider host/port parameters, so pass None.
                service.start_monitoring(None, None);
            }
            MusicServiceType::Cider => {
                service.start_monitoring(
                    Some(self.config.service.cider.host.clone()),
                    Some(self.config.service.cider.port),
                );
            }
            MusicServiceType::Spotify => {
                // The generic MusicService::start_monitoring is a no-op for Spotify; the SpotifyMonitor
                // requires explicit credentials and a different start API, so call it directly.
                if let Some(s) = &self.config.service.spotify {
                    if let MusicService::Spotify(m) = &service {
                        m.start_monitoring(s.client_id.clone(), s.client_secret.clone(), s.redirect_uri.clone());
                    }
                }
            }
        }

        // Get the receiver for the web server
        let receiver = service.get_receiver();

        // Start web server
        match WebServer::new(
            self.config.server.host.clone(),
            self.config.server.port,
            receiver,
            self.config.widget.clone(),
        ) {
            Ok(server) => {
                let url = server.get_url();
                self.status_message = format!("Running on {}", url);
                self.server = Some(Arc::new(SyncRwLock::new(server)));
                self.music_service = Some(service);
            }
            Err(e) => {
                self.status_message = format!("Failed to start: {}", e);
            }
        }
    }

    fn connect_obs(&mut self) {
        let config = self.config.obs.clone();
        self.obs_status = "Connecting...".to_string();
        let obs_client = Arc::new(AsyncRwLock::new(ObsClient::new(config)));
        self.obs_client = Some(obs_client.clone());
        let (tx, rx) = std::sync::mpsc::channel();
        self.obs_status_rx = Some(rx);
        RUNTIME.spawn(async move {
            let mut client = obs_client.write().await;
            let result = client.connect_with_warnings(|_| {
                // You can push to warnings here if you want UI feedback
            }).await;
            match result {
                Ok(_) => {
                    tracing::info!("Connected to OBS successfully");
                    let _ = tx.send("Connected to OBS".to_string());
                },
                Err(e) => {
                    tracing::error!("Failed to connect to OBS: {}", e);
                    let _ = tx.send(format!("Failed: {}", e));
                }
            }
        });
    }

    fn create_obs_scene(&mut self) {
        if let (Some(server), Some(obs_client)) = (&self.server, &self.obs_client) {
            let url = server.read().get_url();
            let width = self.config.widget.width;
            let height = self.config.widget.height;
            let obs_client = obs_client.clone();
            RUNTIME.spawn(async move {
                let client = obs_client.write().await;
                if client.is_connected() {
                    // Always check, then create if missing
                    let check_result = client.check_and_update_source(&url, width, height).await;
                    match check_result {
                        Ok(_) => {
                            tracing::info!("Source checked/updated successfully");
                        }
                        Err(_e) => {
                            tracing::warn!("Source or scene missing, attempting to create...");
                        }
                    }
                    // Always attempt to create (idempotent if already exists)
                    match client.create_scene_and_source(&url, width, height).await {
                        Ok(_) => {
                            tracing::info!("Scene and source created successfully");
                        }
                        Err(e) => {
                            tracing::error!("Failed to create scene/source: {}", e);
                        }
                    }
                }
            });
        }
    }

    fn save_config(&mut self) {
        if let Err(e) = self.config.save() {
            self.status_message = format!("Failed to save config: {}", e);
        } else {
            self.status_message = "Configuration saved".to_string();

            // Update template if server is running
            if let Some(server) = &self.server {
                server.read().update_template(self.config.widget.clone());
            }
        }
    }
}

impl eframe::App for NowPlayingApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Improved dark theme with red accents
        let mut style = (*ctx.style()).clone();
        style.visuals.dark_mode = true;
        ctx.set_style(style);

        egui::CentralPanel::default().show(ctx, |ui| {
            // Show warnings at the top
            let mut to_remove = Vec::new();
            if !self.warnings.is_empty() {
                ui.vertical(|ui| {
                    for (i, warning) in self.warnings.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::RED, format!("Warning: {}", warning));
                            if ui.button("Dismiss").clicked() {
                                to_remove.push(i);
                            }
                        });
                    }
                });
                ui.add_space(8.0);
            }
            // Remove warnings after rendering
            for i in to_remove.into_iter().rev() {
                self.warnings.remove(i);
            }

            // Header
            ui.horizontal(|ui| {
                ui.heading("Now Playing");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Save Config").clicked() {
                        self.save_config();
                    }
                });
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Status bar
            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.label(&self.status_message);
            });

            ui.add_space(8.0);

            // Tab selector
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.selected_tab, Tab::MusicService, "Music Service");
                ui.selectable_value(&mut self.selected_tab, Tab::Server, "Server");
                ui.selectable_value(&mut self.selected_tab, Tab::Obs, "OBS");
                ui.selectable_value(&mut self.selected_tab, Tab::Widget, "Widget");
            });

            ui.separator();
            ui.add_space(8.0);

            // Tab content
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.selected_tab {
                    Tab::MusicService => self.render_music_service_tab(ui),
                    Tab::Server => self.render_server_tab(ui),
                    Tab::Obs => self.render_obs_tab(ui),
                    Tab::Widget => self.render_widget_tab(ui),
                }
            });
        });

        // Poll OBS connection status from channel
        if let Some(rx) = &self.obs_status_rx {
            if let Ok(msg) = rx.try_recv() {
                self.obs_status = msg;
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }
}

impl NowPlayingApp {
    fn render_music_service_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Music Service Configuration");
        ui.add_space(8.0);

        ui.label("Select your music player:");
        ui.add_space(4.0);

        #[cfg(target_os = "macos")]
        ui.radio_value(&mut self.config.service.service_type, MusicServiceType::AppleMusic, "Apple Music (macOS)");

        ui.radio_value(&mut self.config.service.service_type, MusicServiceType::Cider, "Cider");
        ui.radio_value(&mut self.config.service.service_type, MusicServiceType::Spotify, "Spotify");

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        match self.config.service.service_type {
            #[cfg(target_os = "macos")]
            MusicServiceType::AppleMusic => {
                ui.heading("Apple Music");
                ui.add_space(4.0);
                ui.label("Apple Music integration uses AppleScript to monitor your music playback.");
                ui.label("No additional configuration required.");
            }
            MusicServiceType::Cider => {
                ui.heading("Cider Configuration");
                ui.add_space(4.0);
                ui.label("Cider is a cross-platform Apple Music client.");
                ui.add_space(8.0);

                egui::Grid::new("cider_grid")
                    .num_columns(2)
                    .spacing([40.0, 8.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Host:");
                        ui.text_edit_singleline(&mut self.config.service.cider.host);
                        ui.end_row();

                        ui.label("Port:");
                        ui.add(egui::DragValue::new(&mut self.config.service.cider.port).speed(1).range(1024..=65535));
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.label("Default Cider WebSocket server runs on localhost:10767");
            }
            MusicServiceType::Spotify => {
                ui.heading("Spotify Configuration");
                ui.add_space(4.0);
                ui.label("Spotify requires you to provide a Client ID and Client Secret.");
                ui.label("Register an app at https://developer.spotify.com/dashboard/");
                ui.add_space(8.0);

                if let Some(spotify_cfg) = &mut self.config.service.spotify {
                    egui::Grid::new("spotify_grid")
                        .num_columns(2)
                        .spacing([40.0, 8.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Client ID:");
                            ui.text_edit_singleline(&mut spotify_cfg.client_id);
                            ui.end_row();

                            ui.label("Client Secret:");
                            ui.add(egui::TextEdit::singleline(&mut spotify_cfg.client_secret).password(true));
                            ui.end_row();

                            ui.label("Redirect URI:");
                            ui.text_edit_singleline(&mut spotify_cfg.redirect_uri);
                            ui.end_row();
                        });

                    ui.add_space(8.0);
                    ui.label("After entering credentials, start the server and the app will prompt you to authorize Spotify when starting the service.");
                } else {
                    ui.label("No Spotify configuration present");
                }
            }
        }
    }

    fn render_server_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Web Server Configuration");
        ui.add_space(8.0);

        egui::Grid::new("server_grid")
            .num_columns(2)
            .spacing([40.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("Host:");
                ui.text_edit_singleline(&mut self.config.server.host);
                ui.end_row();

                ui.label("Port:");
                ui.add(egui::DragValue::new(&mut self.config.server.port).speed(1).range(1024..=65535));
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.label("Tip: Use '0.0.0.0' to listen on all IPv4 addresses, or '::' for all IPv6 addresses.");
        ui.label("   Default '127.0.0.1' only accepts connections from localhost.");

        ui.add_space(16.0);

        ui.horizontal(|ui| {
            if ui.button(if self.server.is_some() { "Restart Server" } else { "Start Server" }).clicked() {
                self.start_services();
            }

            if let Some(server) = &self.server {
                if ui.button("Open in Browser").clicked() {
                    let url = server.read().get_url();
                    if let Err(e) = open::that(&url) {
                        self.status_message = format!("Failed to open browser: {}", e);
                    }
                }
            }
        });

        ui.add_space(16.0);

        if self.server.is_some() {
            ui.separator();
            ui.add_space(8.0);
            ui.heading("Endpoints");
            ui.add_space(4.0);

            if let Some(server) = &self.server {
                let port = server.read().port;
                ui.label(format!("Widget:      http://127.0.0.1:{}/", port));
                ui.label(format!("JSON API:    http://127.0.0.1:{}/now-playing", port));
                ui.label(format!("SSE Stream:  http://127.0.0.1:{}/events", port));
                ui.label(format!("Health:      http://127.0.0.1:{}/health", port));
            }
        }
    }

    fn render_obs_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("OBS WebSocket Configuration");
        ui.add_space(8.0);

        ui.checkbox(&mut self.config.obs.enabled, "Enable OBS Integration");
        ui.add_space(8.0);

        egui::Grid::new("obs_grid")
            .num_columns(2)
            .spacing([40.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("Host:");
                ui.text_edit_singleline(&mut self.config.obs.host);
                ui.end_row();

                ui.label("Port:");
                ui.add(egui::DragValue::new(&mut self.config.obs.port).speed(1).range(1024..=65535));
                ui.end_row();

                ui.label("Password:");
                ui.add(egui::TextEdit::singleline(&mut self.config.obs.password).password(true));
                ui.end_row();

                ui.label("Scene Name:");
                ui.text_edit_singleline(&mut self.config.obs.scene_name);
                ui.end_row();

                ui.label("Source Name:");
                ui.text_edit_singleline(&mut self.config.obs.source_name);
                ui.end_row();
            });

        ui.add_space(16.0);

        let obs_connected = self.obs_status == "Connected to OBS";

        ui.horizontal(|ui| {
            if ui.button("Connect to OBS").clicked() && self.config.obs.enabled {
                self.connect_obs();
            }

            let create_btn = ui.add_enabled(obs_connected, egui::Button::new("Create Scene & Source"));
            if create_btn.clicked() {
                self.create_obs_scene();
            }

            ui.label(&self.obs_status);
        });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        ui.label("Tip: Enable OBS WebSocket server in OBS settings");
    }

    fn render_widget_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Widget Appearance");
        ui.add_space(8.0);

        egui::Grid::new("widget_grid")
            .num_columns(2)
            .spacing([40.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("Width:");
                ui.add(egui::DragValue::new(&mut self.config.widget.width).speed(1).range(100..=3840));
                ui.end_row();

                ui.label("Height:");
                ui.add(egui::DragValue::new(&mut self.config.widget.height).speed(1).range(100..=2160));
                ui.end_row();
            });

        ui.add_space(16.0);

        ui.label("Template Mode:");
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.config.widget.template_mode, TemplateMode::Default, "Default");
            ui.radio_value(&mut self.config.widget.template_mode, TemplateMode::Custom, "Custom");
        });
        ui.add_space(8.0);

        if self.config.widget.template_mode == TemplateMode::Custom {
            ui.label("Custom Template Path:");
            let custom_path = self.config.widget.custom_template_path.clone().unwrap_or_default();
            ui.horizontal(|ui| {
                ui.label(&custom_path);
                if ui.button("Select File...").clicked() {
                    if let Some(path) = FileDialog::new().add_filter("HTML", &["html"]).pick_file() {
                        self.config.widget.custom_template_path = Some(path.display().to_string());
                    }
                }
            });
            ui.add_space(4.0);
        }

        ui.label("Available placeholders: {title}, {artist}, {album}, {state}, {progress}, {current_time}, {duration_time}, {artwork_base64}");
        ui.add_space(4.0);
        // Template editing removed from UI
    }

}
