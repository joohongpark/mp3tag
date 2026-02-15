use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 앱 전체 설정.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub spotify: SpotifyConfig,
}

/// Spotify API 자격증명 설정.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpotifyConfig {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

impl SpotifyConfig {
    /// client_id와 client_secret이 모두 설정되어 있는지 확인한다.
    pub fn is_configured(&self) -> bool {
        self.client_id.as_ref().is_some_and(|s| !s.is_empty())
            && self.client_secret.as_ref().is_some_and(|s| !s.is_empty())
    }
}

/// 설정 파일 경로를 반환한다. 현재 디렉토리의 config.toml.
fn config_path() -> PathBuf {
    PathBuf::from("config.toml")
}

/// 설정 파일을 읽어 Config를 반환한다. 파일이 없으면 기본값.
pub fn load_config() -> Config {
    let path = config_path();
    if !path.exists() {
        return Config::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

/// Config를 설정 파일에 저장한다.
pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path();
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}
