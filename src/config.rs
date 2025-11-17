// Configuration management for the application

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MusicServiceType {
    #[cfg(target_os = "macos")]
    AppleMusic,
    Cider,
    Spotify,
}

impl Default for MusicServiceType {
    fn default() -> Self {
        #[cfg(target_os = "macos")]
        return MusicServiceType::AppleMusic;

        #[cfg(not(target_os = "macos"))]
        return MusicServiceType::Cider;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub service: MusicServiceConfig,
    pub server: ServerConfig,
    pub obs: ObsConfig,
    pub widget: WidgetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicServiceConfig {
    pub service_type: MusicServiceType,
    pub cider: CiderConfig,
    pub spotify: Option<SpotifyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiderConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub password: String,
    pub scene_name: String,
    pub source_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum TemplateMode {
    #[default]
    Default,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub width: u32,
    pub height: u32,
    pub template: String,
    pub template_mode: TemplateMode,
    pub custom_template_path: Option<String>,
}

const DEFAULT_TEMPLATE_PATH: &str = "assets/default_template.html";

// Embed the default template at compile time as a reliable fallback. This guarantees
// the binary always has a usable template even if files are missing from the bundle.
const COMPILED_DEFAULT_TEMPLATE: &str = include_str!("../assets/default_template.html");

pub fn load_default_template() -> String {
    // 1) Try the straightforward project-relative path (development / run from source)
    if let Ok(contents) = fs::read_to_string(DEFAULT_TEMPLATE_PATH) {
        return contents;
    }

    // 2) If running from an app bundle (macOS) or other packaged layout, try to locate
    // the Resources directory relative to the running executable. Typical app bundle
    // layout: MyApp.app/Contents/MacOS/<exe>  -> Resources in ../Resources
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // Common macOS bundle: Contents/MacOS -> parent is Contents
            if let Some(contents_dir) = exe_dir.parent() {
                let candidate = contents_dir.join("Resources").join("default_template.html");
                if let Ok(contents) = fs::read_to_string(&candidate) {
                    return contents;
                }
            }

            // Also try a Resources sibling of the executable dir (defensive)
            let alt = exe_dir.join("Resources").join("default_template.html");
            if let Ok(contents) = fs::read_to_string(&alt) {
                return contents;
            }
        }
    }

    // 3) As a last resort, return the compile-time embedded template so the app still works.
    COMPILED_DEFAULT_TEMPLATE.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyConfig {
    /// Spotify OAuth2 Client ID (user-provided)
    pub client_id: String,
    /// Spotify OAuth2 Client Secret (user-provided)
    pub client_secret: String,
    /// Redirect URI registered with Spotify (e.g. http://localhost:8888/callback)
    pub redirect_uri: String,
    /// Optional cached access token file path (app may store tokens here)
    pub token_cache_path: Option<String>,
}

impl Default for SpotifyConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: "http://localhost:8888/callback".to_string(),
            token_cache_path: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            service: MusicServiceConfig {
                service_type: MusicServiceType::default(),
                cider: CiderConfig {
                    host: "localhost".to_string(),
                    port: 10767,
                },
                spotify: Some(SpotifyConfig::default()),
            },
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8765,
            },
            obs: ObsConfig {
                enabled: false,
                host: "localhost".to_string(),
                port: 4455,
                password: String::new(),
                scene_name: "Now Playing".to_string(),
                source_name: "Now Playing".to_string(),
            },
            widget: WidgetConfig {
                width: 360,
                height: 160,
                template: String::new(), // always loaded from file
                template_mode: TemplateMode::Default,
                custom_template_path: None,
            },
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        if let Some(config_dir) = directories::ProjectDirs::from("monster", "void", "NowPlaying") {
            let dir = config_dir.config_dir();
            fs::create_dir_all(dir).ok();
            dir.join("config.toml")
        } else {
            PathBuf::from("config.toml")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(contents) = fs::read_to_string(&path) {
            toml::from_str(&contents).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        let contents = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, contents).map_err(|e| e.to_string())?;
        Ok(())
    }
}
