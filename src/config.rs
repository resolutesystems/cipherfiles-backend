use serde::Deserialize;
use tokio::fs;

use crate::errors::AppResult;

pub async fn load_config(path: &str) -> AppResult<Config> {
    let contents = fs::read_to_string(path).await?;
    let parsed = toml::from_str(&contents)?;
    Ok(parsed)
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeneralConfig {
    pub bind_address: String,
    pub cors_origin: String,
    pub storage_dir: String,
    pub temp_dir: String,
    pub max_preview_bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstrumentationConfig {
    pub directives: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub blacklist: Vec<String>,
    pub database: DatabaseConfig,
    pub general: GeneralConfig,
    pub instrumentation: InstrumentationConfig,
}
