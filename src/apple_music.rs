// Apple Music integration (macOS only)

use crate::rt::RUNTIME;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::error;

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
pub struct AppleMusicMonitor {
    sender: watch::Sender<TrackInfo>,
    pub receiver: watch::Receiver<TrackInfo>,
    handle: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl Default for AppleMusicMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl AppleMusicMonitor {
    pub fn new() -> Self {
        let (sender, receiver) = watch::channel(TrackInfo::default());
        Self { sender, receiver, handle: Arc::new(RwLock::new(None)) }
    }


    pub fn start_monitoring(&self) {
        let sender = self.sender.clone();
        let handle = RUNTIME.spawn(async move {
            let mut last = TrackInfo::default();
            loop {
                match query_apple_music() {
                    Ok(info) => {
                        if info != last {
                            if sender.send(info.clone()).is_ok() {
                                tracing::info!("AppleMusic send: {:?}", info);
                                last = info;
                            } else {
                                tracing::warn!("AppleMusic failed to send TrackInfo");
                            }
                        }
                    }
                    Err(e) => {
                        error!("Apple Music query failed: {}", e);
                    }
                }
                sleep(Duration::from_millis(750)).await;
            }
        });
        *self.handle.write() = Some(handle);
    }

    pub fn stop_monitoring(&self) {
        if let Some(handle) = self.handle.write().take() {
            handle.abort();
        }
    }
}

fn query_apple_music() -> Result<TrackInfo, String> {
    let script = r#"
tell application "Music"
    if it is running is false then
        return ""
    end if
    set pstate to player state as string
    set isPlaying to (pstate is equal to "playing")
    if current track exists then
        set t to current track
        set t_name to name of t
        set t_artist to artist of t
        set t_album to album of t
        set t_duration to duration of t
        set t_pos to player position
    else
        set t_name to ""
        set t_artist to ""
        set t_album to ""
        set t_duration to 0
        set t_pos to 0
    end if
    return t_name & tab & t_artist & tab & t_album & tab & (t_duration as integer) & tab & (t_pos as integer) & tab & (isPlaying as boolean as string)
end tell
    "#;

    let output = Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(format!(
            "osascript failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        tracing::debug!("Music app not running or no output");
        return Ok(TrackInfo::default());
    }
    tracing::debug!("Raw AppleScript output: {:?}", stdout);

    let mut track_info = parse_tab_output(&stdout)?;

    // Try to get artwork separately using a different approach
    if !track_info.title.is_empty() {
        track_info.artwork_base64 = get_artwork_base64();
    }

    Ok(track_info)
}

fn get_artwork_base64() -> Option<String> {
    use std::fs;

    // Create a temporary file path
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("now_playing_artwork.jpg");
    let temp_path_str = temp_path.to_string_lossy();

    let script = format!(r#"
tell application "Music"
    if it is running and current track exists then
        try
            set t to current track
            if exists (first artwork of t) then
                set artData to data of first artwork of t
                set outFile to open for access POSIX file "{}" with write permission
                set eof of outFile to 0
                write artData to outFile
                close access outFile
                return "success"
            end if
        on error errMsg
            try
                close access POSIX file "{}"
            end try
            return "error: " & errMsg
        end try
    end if
    return "no_artwork"
end tell
    "#, temp_path_str, temp_path_str);

    let output = Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .ok()?;

    let result = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if result == "success" {
        // Read the file and base64 encode it
        if let Ok(artwork_data) = fs::read(&temp_path) {
            // Clean up temp file
            let _ = fs::remove_file(&temp_path);

            // Encode to base64
            let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &artwork_data);
            return Some(format!("data:image/jpeg;base64,{}", encoded));
        }
    }

    // Clean up temp file if it exists
    let _ = fs::remove_file(&temp_path);
    None
}

fn parse_tab_output(s: &str) -> Result<TrackInfo, String> {
    let parts: Vec<&str> = s.split('\t').collect();
    if parts.len() < 6 {
        return Err(format!("Unexpected field count {} in '{}'", parts.len(), s));
    }
    let title = parts[0].to_string();
    let artist = parts[1].to_string();
    let album = parts[2].to_string();
    let duration_seconds = parts[3].parse::<u32>().unwrap_or(0);
    let position_seconds = parts[4].parse::<u32>().unwrap_or(0);
    let is_playing = match parts[5] { "true" => true, "false" => false, other => { other == "playing" } };

    Ok(TrackInfo {
        title,
        artist,
        album,
        duration_seconds,
        position_seconds,
        is_playing,
        artwork_base64: None  // Will be set by get_artwork_base64()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_ok() {
        let input = "Song\tArtist\tAlbum\t123\t5\ttrue\t";
        let ti = parse_tab_output(input).unwrap();
        assert_eq!(ti.title, "Song");
        assert!(ti.is_playing);
        assert_eq!(ti.duration_seconds, 123);
        assert_eq!(ti.position_seconds, 5);
    }
    #[test]
    fn parse_bad() {
        let bad = "only\tthree\tfields";
        assert!(parse_tab_output(bad).is_err());
    }
}
