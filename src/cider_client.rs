// Cider music player client using Socket.IO
use crate::music_service::TrackInfo;
use crate::rt::RUNTIME;
use parking_lot::RwLock;
use rust_socketio::{ClientBuilder, Payload, RawClient};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{error, info};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CiderPlaybackData {
    name: Option<String>,
    artist_name: Option<String>,
    album_name: Option<String>,
    artwork: Option<CiderArtwork>,
    duration_in_millis: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct CiderArtwork {
    url: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CiderPlaybackTime {
    current_playback_time: Option<f64>,
    current_playback_duration: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct CiderPlaybackState {
    state: Option<String>,
    attributes: Option<CiderPlaybackData>,
}

#[derive(Clone)]
pub struct CiderMonitor {
    sender: watch::Sender<TrackInfo>,
    pub receiver: watch::Receiver<TrackInfo>,
    handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    current_track: Arc<RwLock<TrackInfo>>,
}

impl Default for CiderMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl CiderMonitor {
    pub fn new() -> Self {
        let (sender, receiver) = watch::channel(TrackInfo::default());
        Self {
            sender,
            receiver,
            handle: Arc::new(RwLock::new(None)),
            current_track: Arc::new(RwLock::new(TrackInfo::default())),
        }
    }

    pub fn start_monitoring(&self, host: String, port: u16) {
        let sender = self.sender.clone();
        let current_track = self.current_track.clone();

        let handle = RUNTIME.spawn(async move {
            let url = format!("http://{}:{}", host, port);
            info!("Connecting to Cider at {}", url);

            loop {
                match Self::connect_to_cider(&url, sender.clone(), current_track.clone()).await {
                    Ok(_) => {
                        info!("Cider connection ended normally");
                    }
                    Err(e) => {
                        error!("Cider connection error: {}", e);
                    }
                }

                // Wait before reconnecting
                tokio::time::sleep(Duration::from_secs(5)).await;
                info!("Attempting to reconnect to Cider...");
            }
        });

        *self.handle.write() = Some(handle);
    }

    async fn connect_to_cider(
        url: &str,
        sender: watch::Sender<TrackInfo>,
        current_track: Arc<RwLock<TrackInfo>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let current_track_clone = current_track.clone();
        let sender_clone = sender.clone();

        let on_playback = move |payload: Payload, _socket: RawClient| {
            if let Payload::Text(json_values) = payload {
                if let Some(json_str) = json_values.first() {
                    Self::handle_playback_event(
                        json_str.as_str().unwrap_or(""),
                        &sender_clone,
                        &current_track_clone,
                    );
                }
            }
        };

        let _socket = ClientBuilder::new(url)
            .namespace("/")
            .on("API:Playback", on_playback)
            .on("connect", |_payload, _socket| {
                info!("Connected to Cider!");
            })
            .on("disconnect", |_payload, _socket| {
                info!("Disconnected from Cider");
            })
            .on("error", |payload, _socket| {
                error!("Socket error: {:?}", payload);
            })
            .connect()
            .map_err(|e| format!("Failed to connect: {}", e))?;

        // Keep the connection alive - the socket will disconnect on its own
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }

    fn handle_playback_event(
        json_str: &str,
        sender: &watch::Sender<TrackInfo>,
        current_track: &Arc<RwLock<TrackInfo>>,
    ) {
        // Parse the event which has format: [{"data": {...}, "type": "..."}]
        #[derive(Deserialize)]
        struct EventWrapper {
            data: serde_json::Value,
            #[serde(rename = "type")]
            event_type: String,
        }

        let events: Result<Vec<EventWrapper>, _> = serde_json::from_str(json_str);

        if let Ok(events) = events {
            for event in events {
                match event.event_type.as_str() {
                    "playbackStatus.nowPlayingItemDidChange" => {
                        if let Ok(data) = serde_json::from_value::<CiderPlaybackData>(event.data) {
                            let mut track = current_track.write();
                            track.title = data.name.unwrap_or_default();
                            track.artist = data.artist_name.unwrap_or_default();
                            track.album = data.album_name.unwrap_or_default();
                            track.duration_seconds =
                                (data.duration_in_millis.unwrap_or(0.0) / 1000.0) as u32;

                            // Handle artwork
                            if let Some(artwork) = data.artwork {
                                if let Some(url) = artwork.url {
                                    let width = artwork.width.unwrap_or(512);
                                    let height = artwork.height.unwrap_or(512);
                                    let artwork_url = url
                                        .replace("{w}", &width.to_string())
                                        .replace("{h}", &height.to_string());

                                    // For now, just store the URL as a placeholder
                                    // In a full implementation, you'd fetch and convert to base64
                                    track.artwork_base64 = Some(artwork_url);
                                }
                            }

                            drop(track);
                            let sent = sender.send(current_track.read().clone()).is_ok();
                            tracing::info!("Cider nowPlayingItemDidChange send: {}", sent);
                        }
                    }
                    "playbackStatus.playbackTimeDidChange" => {
                        if let Ok(data) = serde_json::from_value::<CiderPlaybackTime>(event.data) {
                            let mut track = current_track.write();
                            track.position_seconds =
                                data.current_playback_time.unwrap_or(0.0) as u32;
                            if let Some(duration) = data.current_playback_duration {
                                track.duration_seconds = duration as u32;
                            }
                            drop(track);
                            let sent = sender.send(current_track.read().clone()).is_ok();
                            tracing::info!("Cider playbackTimeDidChange send: {}", sent);
                        }
                    }
                    "playbackStatus.playbackStateDidChange" => {
                        if let Ok(data) = serde_json::from_value::<CiderPlaybackState>(event.data) {
                            let mut track = current_track.write();

                            if let Some(state) = data.state {
                                track.is_playing = state == "playing";
                            }

                            // Update full track info if provided
                            if let Some(attributes) = data.attributes {
                                track.title = attributes.name.unwrap_or_default();
                                track.artist = attributes.artist_name.unwrap_or_default();
                                track.album = attributes.album_name.unwrap_or_default();
                                track.duration_seconds =
                                    (attributes.duration_in_millis.unwrap_or(0.0) / 1000.0) as u32;

                                if let Some(artwork) = attributes.artwork {
                                    if let Some(url) = artwork.url {
                                        let width = artwork.width.unwrap_or(512);
                                        let height = artwork.height.unwrap_or(512);
                                        let artwork_url = url
                                            .replace("{w}", &width.to_string())
                                            .replace("{h}", &height.to_string());
                                        track.artwork_base64 = Some(artwork_url);
                                    }
                                }
                            }

                            drop(track);
                            let sent = sender.send(current_track.read().clone()).is_ok();
                            tracing::info!("Cider playbackStateDidChange send: {}", sent);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn stop_monitoring(&self) {
        if let Some(handle) = self.handle.write().take() {
            handle.abort();
        }
    }
}
