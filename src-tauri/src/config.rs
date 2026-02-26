use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ApiConfig {
    #[serde(default)]
    pub genius_token: String,
    #[serde(default)]
    pub anthropic_key: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub api: ApiConfig,
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Config not found at {}: {e}", path.display());
                return Self::default();
            }
        };
        match toml::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Invalid config: {e}");
                Self::default()
            }
        }
    }

    pub fn has_keys(&self) -> bool {
        !self.api.genius_token.is_empty() && !self.api.anthropic_key.is_empty()
    }
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home)
        .join(".config")
        .join("enhanced-music")
        .join("config.toml")
}
