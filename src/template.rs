// filepath: src/template.rs
// HTML Template engine with live data replacement

use crate::config::{load_default_template, TemplateMode, WidgetConfig};
use std::fs;

pub struct TemplateEngine {}

impl TemplateEngine {
    pub fn asset_path(filename: &str) -> String {
        let base = env!("CARGO_MANIFEST_DIR");
        format!("{}/assets/{}", base, filename)
    }

    // Embed the template wrapper at compile time as a fallback so the binary still works
    // even if files aren't present in the bundle's Resources directory.
    const COMPILED_TEMPLATE_WRAPPER: &'static str = include_str!("../assets/template_wrapper.html");

    pub fn load_template_wrapper() -> Result<String, String> {
        // 1) Try the development/project asset path
        let path = Self::asset_path("template_wrapper.html");
        if let Ok(contents) = fs::read_to_string(&path) {
            return Ok(contents);
        }

        // 2) Try app bundle Resources relative to the running executable
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                if let Some(contents_dir) = exe_dir.parent() {
                    let candidate = contents_dir.join("Resources").join("template_wrapper.html");
                    if let Ok(contents) = fs::read_to_string(&candidate) {
                        return Ok(contents);
                    }
                }

                let alt = exe_dir.join("Resources").join("template_wrapper.html");
                if let Ok(contents) = fs::read_to_string(&alt) {
                    return Ok(contents);
                }
            }
        }

        // 3) Fallback to the compile-time embedded wrapper
        Ok(Self::COMPILED_TEMPLATE_WRAPPER.to_string())
    }

    pub fn load_template(config: &WidgetConfig) -> Result<String, String> {
        match config.template_mode {
            TemplateMode::Default => Ok(load_default_template()),
            TemplateMode::Custom => {
                if let Some(ref path) = config.custom_template_path {
                    match std::fs::read_to_string(path) {
                        Ok(contents) => Ok(contents),
                        Err(e) => {
                            let _fallback = load_default_template();
                            Err(format!(
                                "Failed to load custom template '{}': {}. Falling back to default.",
                                path, e
                            ))
                        }
                    }
                } else {
                    let _fallback = load_default_template();
                    Err("Custom template path not set. Falling back to default.".to_string())
                }
            }
        }
    }

    pub fn render_live_template_with_config(config: &WidgetConfig) -> Result<String, String> {
        let widget_template = Self::load_template(config)?;
        let wrapper = Self::load_template_wrapper()?;
        Ok(wrapper.replace("{{TEMPLATE}}", &widget_template))
    }
}
