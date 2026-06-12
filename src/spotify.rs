// filepath: src/spotify.rs

use crate::music_service::TrackInfo;
use crate::rt::RUNTIME;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use reqwest::Client as HttpClient;
use serde::Deserialize;
use tiny_http::{Response, Server};
use tokio::sync::watch;
use url::form_urlencoded;

#[derive(Clone)]
pub struct SpotifyMonitor {
    pub receiver: watch::Receiver<TrackInfo>,
    tx: watch::Sender<TrackInfo>,
}

impl SpotifyMonitor {
    pub fn new(_client_id: &str, _client_secret: &str, _redirect_uri: &str) -> Self {
        let (tx, rx) = watch::channel(TrackInfo::default());
        Self { receiver: rx, tx }
    }

    /// Start monitoring. Opens a browser for user authorization and polls Spotify.
    pub fn start_monitoring(&self, client_id: String, client_secret: String, redirect_uri: String) {
        let tx = self.tx.clone();
        RUNTIME.spawn(async move {
            // Build OAuth2 client
            let client = BasicClient::new(
                ClientId::new(client_id.clone()),
                Some(ClientSecret::new(client_secret.clone())),
                AuthUrl::new("https://accounts.spotify.com/authorize".to_string()).unwrap(),
                Some(TokenUrl::new("https://accounts.spotify.com/api/token".to_string()).unwrap()),
            )
            .set_redirect_uri(RedirectUrl::new(redirect_uri.clone()).unwrap());

            // Build the authorization URL
            let (auth_url, _csrf) = client
                .authorize_url(CsrfToken::new_random)
                .add_scope(Scope::new("user-read-currently-playing".to_string()))
                .add_scope(Scope::new("user-read-playback-state".to_string()))
                .url();

            // Try opening browser; otherwise print the URL
            let _ = open::that(auth_url.to_string());

            // Start tiny_http server to receive the redirect
            let server = match Server::http("0.0.0.0:8888") {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to start local redirect server: {}", e);
                    return;
                }
            };

            // Wait for one request
            let req = match server.recv() {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Failed to receive OAuth redirect request: {}", e);
                    return;
                }
            };

            // Parse the query string
            let query = req.url().split('?').nth(1).unwrap_or("");
            let params: Vec<(String, String)> = form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();
            let mut code_opt: Option<String> = None;
            for (k, v) in params {
                if k == "code" {
                    code_opt = Some(v);
                    break;
                }
            }

            if code_opt.is_none() {
                let response = Response::from_string("Missing code in callback");
                let _ = req.respond(response);
                tracing::error!("OAuth redirect did not contain a code");
                return;
            }

            let code = code_opt.unwrap();

            // Respond to the browser so the user sees a confirmation page
            let response =
                Response::from_string("Authorization complete - you may close this window.");
            let _ = req.respond(response);

            // Exchange code for a token
            let token_res = client
                .exchange_code(AuthorizationCode::new(code))
                .request_async(async_http_client)
                .await;
            let token = match token_res {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("Failed to exchange token: {}", e);
                    return;
                }
            };

            let access_token = token.access_token().secret().clone();
            let http = HttpClient::new();

            // Poll loop: fetch currently-playing and forward via watch channel
            loop {
                let resp = http
                    .get("https://api.spotify.com/v1/me/player/currently-playing")
                    .bearer_auth(&access_token)
                    .send()
                    .await;

                if let Ok(r) = resp {
                    if r.status().is_success() {
                        if let Ok(body) = r.text().await {
                            if let Ok(sp) = serde_json::from_str::<SpotifyCurrentlyPlaying>(&body) {
                                let track = TrackInfo {
                                    title: sp.item.name.unwrap_or_default(),
                                    artist: sp
                                        .item
                                        .artists
                                        .iter()
                                        .map(|a| a.name.clone())
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                    album: sp.item.album.name.unwrap_or_default(),
                                    duration_seconds: (sp.item.duration_ms.unwrap_or(0) / 1000)
                                        as u32,
                                    position_seconds: (sp.progress_ms.unwrap_or(0) / 1000) as u32,
                                    is_playing: sp.is_playing.unwrap_or(false),
                                    artwork_base64: None,
                                };
                                let _ = tx.send(track);
                            }
                        }
                    } else if r.status().as_u16() == 204 {
                        // No content: nothing playing
                    } else {
                        tracing::warn!("Spotify API returned {}", r.status());
                    }
                } else {
                    tracing::warn!("Failed to contact Spotify API");
                }

                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        });
    }

    pub fn stop_monitoring(&self) {
        // No-op: background task will exit with runtime
    }
}

#[derive(Deserialize)]
struct SpotifyCurrentlyPlaying {
    pub is_playing: Option<bool>,
    pub progress_ms: Option<u64>,
    pub item: SpotifyItem,
}

#[derive(Deserialize)]
struct SpotifyItem {
    pub name: Option<String>,
    pub duration_ms: Option<u64>,
    pub album: SpotifyAlbum,
    pub artists: Vec<SpotifyArtist>,
}

#[derive(Deserialize)]
struct SpotifyAlbum {
    pub name: Option<String>,
}

#[derive(Deserialize)]
struct SpotifyArtist {
    pub name: String,
}
