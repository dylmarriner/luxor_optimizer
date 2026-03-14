use crate::models::PluginMetadata;
use crate::models::OptimizationRecommendation;
use anyhow::{Result, Context};
use std::fs;
use std::path::PathBuf;
use serde_json;

pub struct PluginEngine {
    plugin_dir: PathBuf,
}

impl PluginEngine {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .context("no config dir")?
            .join("luxor")
            .join("plugins");
        
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        Ok(Self { plugin_dir: config_dir })
    }

    pub fn list_plugins(&self) -> Result<Vec<PluginMetadata>> {
        let mut plugins = Vec::new();
        if !self.plugin_dir.exists() { return Ok(plugins); }

        for entry in fs::read_dir(&self.plugin_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("manifest.json");
                if manifest_path.exists() {
                    let content = fs::read_to_string(manifest_path)?;
                    let meta: PluginMetadata = serde_json::from_str(&content)?;
                    plugins.push(meta);
                }
            }
        }
        Ok(plugins)
    }

    pub fn load_optimizations(&self, plugin_id: &str) -> Result<Vec<OptimizationRecommendation>> {
        let plugin_path = self.plugin_dir.join(plugin_id).join("optimizations.json");
        if !plugin_path.exists() { return Ok(vec![]); }

        let content = fs::read_to_string(plugin_path)?;
        let recs: Vec<OptimizationRecommendation> = serde_json::from_str(&content)?;
        Ok(recs)
    }
}
