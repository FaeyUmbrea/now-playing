// Unified music service abstraction
use crate::rt::RUNTIME;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

#[cfg(target_os = "macos")]
use crate::apple_music::AppleMusicMonitor;

use crate::cider_client::CiderMonitor;
use crate::spotify::SpotifyMonitor;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_seconds: u32,
    pub position_seconds: u32,
    pub is_playing: bool,
    pub artwork_base64: Option<String>,
}

#[derive(Clone)]
pub enum MusicService {
    #[cfg(target_os = "macos")]
    AppleMusic(AppleMusicMonitor),
    Cider(CiderMonitor),
    Spotify(SpotifyMonitor),
}

impl MusicService {
    #[cfg(target_os = "macos")]
    pub fn new_apple_music() -> Self {
        MusicService::AppleMusic(AppleMusicMonitor::new())
    }

    pub fn new_cider() -> Self {
        MusicService::Cider(CiderMonitor::new())
    }

    // Note: construct a Spotify variant directly where needed; no thin wrapper here.

    pub fn start_monitoring(&self, cider_host: Option<String>, cider_port: Option<u16>) {
        match self {
            #[cfg(target_os = "macos")]
            MusicService::AppleMusic(monitor) => {
                monitor.start_monitoring();
            }
            MusicService::Cider(monitor) => {
                let host = cider_host.unwrap_or_else(|| "localhost".to_string());
                let port = cider_port.unwrap_or(10767);
                monitor.start_monitoring(host, port);
            }
            MusicService::Spotify(_monitor) => {
                // No-op here; Spotify monitoring is started with explicit credentials where created.
            }
        }
    }

    pub fn stop_monitoring(&self) {
        match self {
            #[cfg(target_os = "macos")]
            MusicService::AppleMusic(monitor) => {
                monitor.stop_monitoring();
            }
            MusicService::Cider(monitor) => {
                monitor.stop_monitoring();
            }
            MusicService::Spotify(monitor) => {
                monitor.stop_monitoring();
            }
        }
    }

    pub fn get_receiver(&self) -> watch::Receiver<TrackInfo> {
        match self {
            #[cfg(target_os = "macos")]
            MusicService::AppleMusic(monitor) => {
                // Convert apple_music::TrackInfo to music_service::TrackInfo
                let rx = monitor.receiver.clone();
                let (tx, new_rx) = watch::channel(TrackInfo::default());

                RUNTIME.spawn(async move {
                    tracing::info!("AppleMusic forwarder started");
                    let mut rx = rx;
                    loop {
                        if rx.changed().await.is_err() {
                            tracing::info!("AppleMusic forwarder receiver closed");
                            break;
                        }
                        let apple_track = rx.borrow().clone();
                        let track = TrackInfo {
                            title: apple_track.title.clone(),
                            artist: apple_track.artist.clone(),
                            album: apple_track.album.clone(),
                            duration_seconds: apple_track.duration_seconds,
                            position_seconds: apple_track.position_seconds,
                            is_playing: apple_track.is_playing,
                            artwork_base64: apple_track.artwork_base64.clone(),
                        };
                        let sent = tx.send(track.clone()).is_ok();
                        tracing::info!(
                            "AppleMusic forwarder forwarded: sent={}, track={:?}",
                            sent,
                            track
                        );
                    }
                });

                new_rx
            }
            MusicService::Cider(monitor) => monitor.receiver.clone(),
            MusicService::Spotify(monitor) => monitor.receiver.clone(),
        }
    }
}
