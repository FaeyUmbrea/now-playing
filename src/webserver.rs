use crate::config::WidgetConfig;
use crate::rt::RUNTIME;
use crate::template::TemplateEngine;
use crate::TrackInfo;
use parking_lot::RwLock;
use std::net::TcpListener;
use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::wrappers::TcpListenerStream;
use tracing::info;
use warp::Filter;

pub struct WebServer {
    pub host: String,
    pub port: u16,
    widget_config: Arc<RwLock<WidgetConfig>>, // Store config for template selection
}

impl WebServer {
    pub fn new(
        host: String,
        preferred_port: u16,
        receiver: watch::Receiver<TrackInfo>,
        widget_config: WidgetConfig,
    ) -> Result<Self, String> {
        let port_result = (0..100).find_map(|offset| {
            let p = preferred_port + offset;
            TcpListener::bind((host.as_str(), p)).ok().map(|l| (p, l))
        });

        let (chosen_port, listener) = port_result.ok_or_else(|| {
            format!(
                "Unable to bind to {}:{}-{}",
                host,
                preferred_port,
                preferred_port + 99
            )
        })?;

        listener.set_nonblocking(true).map_err(|e| e.to_string())?;

        let widget_config_arc = Arc::new(RwLock::new(widget_config));
        let widget_config_clone = widget_config_arc.clone();

        let host_for_log = host.clone();
        let _task = RUNTIME.spawn(async move {
            let incoming = match tokio::net::TcpListener::from_std(listener) {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!("Failed to convert listener: {}", e);
                    return;
                }
            };

            let stream = TcpListenerStream::new(incoming);

            let widget_config_for_root = widget_config_clone.clone();
            let root = warp::path::end().map(move || {
                let config = widget_config_for_root.read();
                let html = match TemplateEngine::render_live_template_with_config(&config) {
                    Ok(html) => html,
                    Err(e) => format!("<div style='color:red;background:#222;padding:1em;'>Warning: {}</div>", e),
                };
                warp::reply::with_header(html, "Content-Type", "text/html; charset=utf-8")
            });

            let widget_config_for_widget = widget_config_clone.clone();
            let widget_html = warp::path("widget.html").and(warp::get()).map(move || {
                let config = widget_config_for_widget.read();
                let html = match TemplateEngine::render_live_template_with_config(&config) {
                    Ok(html) => html,
                    Err(e) => format!("<div style='color:red;background:#222;padding:1em;'>Warning: {}</div>", e),
                };
                warp::reply::with_header(html, "Content-Type", "text/html; charset=utf-8")
            });

            let json_receiver = receiver.clone();
            let json_route = warp::path("now-playing").and(warp::get()).map(move || {
                // Clone the current TrackInfo into an owned value and serialize it.
                let value = json_receiver.borrow().clone();
                let json = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
                tracing::debug!("/now-playing reply: {}", json);
                warp::reply::with_header(json, "Content-Type", "application/json; charset=utf-8")
            });

            let sse_receiver = receiver.clone();
            let events = warp::path("events").and(warp::get()).map(move || {
                let mut rx = sse_receiver.clone();
                let stream = async_stream::stream! {
                    // Immediately send the current value so clients get an initial state
                    let init = rx.borrow().clone();
                    let init_json = serde_json::to_string(&init).unwrap_or("{}".into());
                    tracing::info!("SSE initial send: {}", init_json);
                    yield Ok::<_, std::convert::Infallible>(warp::sse::Event::default().data(init_json));

                    loop {
                        if rx.changed().await.is_err() { break; }
                        let data = rx.borrow().clone();
                        let json = serde_json::to_string(&data).unwrap_or("{}".into());
                        tracing::info!("SSE send: {}", json);
                        yield Ok::<_, std::convert::Infallible>(warp::sse::Event::default().data(json));
                    }
                };
                warp::sse::reply(warp::sse::keep_alive().stream(stream))
            });

            let health = warp::path("health").and(warp::get()).map(|| "ok");
            let routes = root.or(widget_html).or(json_route).or(events).or(health)
                .with(warp::cors().allow_any_origin());

            info!("Starting web server at {}:{}", host_for_log, chosen_port);
            warp::serve(routes).run_incoming(stream).await;
        });

        info!("Now Playing web server bound to {}:{}", host, chosen_port);
        Ok(Self {
            host: host.clone(),
            port: chosen_port,
            widget_config: widget_config_arc,
        })
    }

    pub fn update_template(&self, widget_config: WidgetConfig) {
        *self.widget_config.write() = widget_config;
    }

    pub fn get_url(&self) -> String {
        // Always use localhost for local display, even if bound to 0.0.0.0
        let display_host = if self.host == "0.0.0.0" || self.host == "::" {
            "127.0.0.1"
        } else {
            &self.host
        };
        format!("http://{}:{}/", display_host, self.port)
    }
}

impl Drop for WebServer {
    fn drop(&mut self) {}
}
