use serde::Deserialize;
use std::path::{
    Path,
    PathBuf,
};

#[derive(Debug, Deserialize)]
pub struct Config {
    workshop_sync_path: PathBuf,

    #[serde(alias = "Levelbuilder")]
    levelbuilder: LaunchConfig,

    #[serde(alias = "Game")]
    game: LaunchConfig,
}

impl Config {
    pub fn get_workshop_sync_path(&self) -> &PathBuf {
        &self.workshop_sync_path
    }

    pub fn get_game_config(&self) -> &LaunchConfig {
        &self.game
    }

    pub fn get_levelbuilder_config(&self) -> &LaunchConfig {
        &self.levelbuilder
    }
}

#[derive(Debug, Deserialize)]
pub struct LaunchConfig {
    path: PathBuf,
}

impl LaunchConfig {
    pub fn get_path(&self) -> &PathBuf {
        &self.path
    }
}

pub fn load_from_file<T: AsRef<Path>>(p: T) -> Option<Config> {
    let data = std::fs::read_to_string(p.as_ref()).ok()?;
    let config = toml::from_str(&data).unwrap();
    Some(config)
}
