// OBS WebSocket client integration

use obws::Client;
use tracing::{error, info, warn};

pub struct ObsClient {
    client: Option<Client>,
    config: crate::config::ObsConfig,
}

impl ObsClient {
    pub fn new(config: crate::config::ObsConfig) -> Self {
        Self {
            client: None,
            config,
        }
    }

    pub async fn connect_with_warnings<F>(&mut self, mut warn: F) -> Result<(), String>
    where
        F: FnMut(String),
    {
        if !self.config.enabled {
            let msg = "OBS WebSocket is disabled".to_string();
            warn(msg.clone());
            return Err(msg);
        }

        info!(
            "Connecting to OBS WebSocket at {}:{}",
            self.config.host, self.config.port
        );

        let password = if self.config.password.is_empty() {
            None
        } else {
            Some(self.config.password.as_str())
        };

        match Client::connect(&self.config.host, self.config.port, password).await {
            Ok(client) => {
                info!("Successfully connected to OBS WebSocket");
                self.client = Some(client);
                Ok(())
            }
            Err(e) => {
                let msg = format!("Failed to connect: {}", e);
                warn(msg.clone());
                error!("Failed to connect to OBS: {}", e);
                Err(msg)
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    pub async fn check_and_update_source(
        &self,
        url: &str,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        let client = self.client.as_ref().ok_or("Not connected to OBS")?;

        // Check if scene exists
        let scenes = client.scenes().list().await.map_err(|e| e.to_string())?;
        let scene_exists = scenes
            .scenes
            .iter()
            .any(|s| s.id.name == *self.config.scene_name);

        if !scene_exists {
            warn!("Scene '{}' does not exist", self.config.scene_name);
            return Ok(());
        }

        info!("Scene '{}' exists", self.config.scene_name);

        // Get scene items to check for source
        use obws::requests::scenes::SceneId;
        let items = client
            .scene_items()
            .list(SceneId::Name(&self.config.scene_name))
            .await
            .map_err(|e| e.to_string())?;

        let source_item = items
            .iter()
            .find(|item| item.source_name == self.config.source_name);

        if let Some(_item) = source_item {
            info!(
                "Source '{}' exists in scene, updating...",
                self.config.source_name
            );

            // Update source settings
            let mut settings = serde_json::Map::new();
            settings.insert(
                "url".to_string(),
                serde_json::Value::String(url.to_string()),
            );
            settings.insert("width".to_string(), serde_json::Value::Number(width.into()));
            settings.insert(
                "height".to_string(),
                serde_json::Value::Number(height.into()),
            );

            use obws::requests::inputs::{InputId, SetSettings};
            client
                .inputs()
                .set_settings(SetSettings {
                    input: InputId::Name(&self.config.source_name),
                    settings: &settings,
                    overlay: Some(true),
                })
                .await
                .map_err(|e| format!("Failed to update source: {}", e))?;

            info!("Successfully updated source settings");
            Ok(())
        } else {
            warn!(
                "Source '{}' does not exist in scene '{}'",
                self.config.source_name, self.config.scene_name
            );
            Ok(())
        }
    }

    pub async fn create_scene_and_source(
        &self,
        url: &str,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        let client = self.client.as_ref().ok_or("Not connected to OBS")?;

        let scene_name = &self.config.scene_name;
        let source_name = "Now Playing Widget";

        if scene_name == source_name {
            return Err("Scene name and source name must be different".to_string());
        }

        info!(
            "Creating scene '{}' and source '{}'",
            scene_name, source_name
        );

        // Check if scene exists, create if not
        let scenes = client.scenes().list().await.map_err(|e| e.to_string())?;
        let scene_exists = scenes.scenes.iter().any(|s| s.id.name == *scene_name);

        if !scene_exists {
            info!("Creating scene '{}'", scene_name);
            client
                .scenes()
                .create(scene_name)
                .await
                .map_err(|e| format!("Failed to create scene: {}", e))?;
        } else {
            info!("Scene '{}' already exists", scene_name);
        }

        // Check if source exists globally (not just in the scene)
        let all_inputs = client
            .inputs()
            .list(None)
            .await
            .map_err(|e| e.to_string())?;
        let global_source_exists = all_inputs.iter().any(|input| input.id.name == source_name);

        if global_source_exists {
            info!(
                "Source '{}' already exists globally, updating settings",
                source_name
            );
            let mut settings = serde_json::Map::new();
            settings.insert(
                "url".to_string(),
                serde_json::Value::String(url.to_string()),
            );
            settings.insert("width".to_string(), serde_json::Value::Number(width.into()));
            settings.insert(
                "height".to_string(),
                serde_json::Value::Number(height.into()),
            );

            use obws::requests::inputs::{InputId, SetSettings};
            client
                .inputs()
                .set_settings(SetSettings {
                    input: InputId::Name(source_name),
                    settings: &settings,
                    overlay: Some(true),
                })
                .await
                .map_err(|e| format!("Failed to update source: {}", e))?;

            // Ensure the source is in the target scene
            use obws::requests::scenes::SceneId;
            let items = client
                .scene_items()
                .list(SceneId::Name(scene_name))
                .await
                .map_err(|e| e.to_string())?;
            let in_scene = items.iter().any(|item| item.source_name == source_name);
            if !in_scene {
                warn!(
                    "Source '{}' exists globally but cannot be added to scene '{}' directly via obws API v0.14.0. Consider duplicating or recreating the source if needed.",
                    source_name, scene_name
                );
                // No direct API method to add an existing input/source to a scene in obws v0.14.0
            }
            return Ok(());
        }

        // Create browser source if it does not exist anywhere
        info!("Creating browser source '{}'", source_name);
        let mut settings = serde_json::Map::new();
        settings.insert(
            "url".to_string(),
            serde_json::Value::String(url.to_string()),
        );
        settings.insert("width".to_string(), serde_json::Value::Number(width.into()));
        settings.insert(
            "height".to_string(),
            serde_json::Value::Number(height.into()),
        );
        settings.insert("shutdown".to_string(), serde_json::Value::Bool(false));

        use obws::requests::inputs::Create;
        use obws::requests::scenes::SceneId as CreateSceneId;
        client
            .inputs()
            .create(Create {
                scene: CreateSceneId::Name(scene_name),
                input: source_name,
                kind: "browser_source",
                settings: Some(&settings),
                enabled: Some(true),
            })
            .await
            .map_err(|e| format!("Failed to create source: {}", e))?;

        info!("Successfully created scene and source");
        Ok(())
    }

    pub async fn disconnect(&mut self) {
        if let Some(client) = self.client.take() {
            info!("Disconnecting from OBS WebSocket");
            drop(client);
        }
    }
}

impl Drop for ObsClient {
    fn drop(&mut self) {
        // Client will disconnect automatically when dropped
    }
}
