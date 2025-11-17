// Now Playing - Music monitoring and web interface

pub mod rt;
pub mod music_service;

#[cfg(target_os = "macos")]
mod apple_music;
mod cider_client;
mod spotify;

mod webserver;
mod config;
mod template;
mod obs_client;

#[cfg(target_os = "macos")]
pub use apple_music::AppleMusicMonitor;
pub use cider_client::CiderMonitor;
pub use music_service::{MusicService, TrackInfo};
pub use webserver::*;
pub use config::*;
pub use obs_client::*;
pub use spotify::SpotifyMonitor;
