// Now Playing - Music monitoring and web interface

pub mod music_service;
pub mod rt;

#[cfg(target_os = "macos")]
mod apple_music;
mod cider_client;
mod spotify;

mod config;
mod obs_client;
mod template;
mod webserver;

#[cfg(target_os = "macos")]
pub use apple_music::AppleMusicMonitor;
pub use cider_client::CiderMonitor;
pub use config::*;
pub use music_service::{MusicService, TrackInfo};
pub use obs_client::*;
pub use spotify::SpotifyMonitor;
pub use webserver::*;
