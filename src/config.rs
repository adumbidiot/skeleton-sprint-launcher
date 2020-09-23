use serde::Deserialize;
use std::path::{
    Path,
    PathBuf,
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub workshop_sync_path: PathBuf,

    #[serde(alias = "Levelbuilder")]
    pub levelbuilder: LaunchConfig,

    #[serde(alias = "Game")]
    pub game: LaunchConfig,
}

impl Config {
    pub fn get_workshop_sync_path(&self) -> &PathBuf {
        &self.workshop_sync_path
    }

    pub fn get_game_path(&self) -> &PathBuf {
        &self.game.path
    }

    pub fn get_levelbuilder_path(&self) -> &PathBuf {
        &self.levelbuilder.path
    }
}

#[derive(Debug, Deserialize)]
pub struct LaunchConfig {
    pub path: PathBuf,
}

pub fn load_from_file<T: AsRef<Path>>(path: T) -> Result<Config, std::io::Error> {
    let path = path.as_ref();
    let data = std::fs::read_to_string(path)?;
    let config = toml::from_str(&data)?;

    Ok(config)
}
