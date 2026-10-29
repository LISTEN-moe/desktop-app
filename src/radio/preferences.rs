use super::model::Channel;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Preferences {
    pub channel: Channel,
    pub volume: f32,
    pub prefer_romaji: bool,
    pub large_artwork: bool,
    pub always_on_top: bool,
    pub event_notifications: bool,
    pub discord_presence: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            channel: Channel::Jpop,
            volume: 50.,
            prefer_romaji: true,
            large_artwork: false,
            always_on_top: false,
            event_notifications: true,
            discord_presence: false,
        }
    }
}

impl Preferences {
    pub fn widget_height(&self) -> f32 {
        if self.large_artwork { 88. } else { 72. }
    }
}

#[derive(Clone)]
pub struct PreferencesStore {
    path: PathBuf,
}

impl PreferencesStore {
    pub fn system() -> Result<Self> {
        let dirs = directories::ProjectDirs::from("moe", "LISTEN", "LISTEN.moe")
            .context("Cannot find the application configuration directory")?;
        Ok(Self::at(dirs.config_dir().join("preferences.json")))
    }

    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<Preferences> {
        match fs::read(&self.path) {
            Ok(bytes) => {
                let mut preferences: Preferences =
                    serde_json::from_slice(&bytes).context("Cannot read saved preferences")?;
                preferences.volume = preferences.volume.clamp(0., 100.);
                Ok(preferences)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(Preferences::default())
            }
            Err(error) => Err(error).context("Cannot open saved preferences"),
        }
    }

    pub fn save(&self, preferences: &Preferences) -> Result<()> {
        let directory = self.path.parent().context("Invalid configuration path")?;
        fs::create_dir_all(directory)?;
        let mut file = tempfile::NamedTempFile::new_in(directory)?;
        serde_json::to_writer_pretty(&mut file, preferences)?;
        file.write_all(b"\n")?;
        file.as_file().sync_all()?;
        file.persist(&self.path)
            .context("Cannot save preferences")?;
        Ok(())
    }
}
