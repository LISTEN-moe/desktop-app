use crate::radio::model::{Song, localized};
use anyhow::{Result, anyhow};
use gpui_kit::Window;
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};

pub struct MediaSession {
    controls: MediaControls,
}

impl MediaSession {
    pub fn new(window: &Window, events: async_channel::Sender<MediaControlEvent>) -> Result<Self> {
        let mut controls = MediaControls::new(PlatformConfig {
            dbus_name: "listen_moe",
            display_name: "LISTEN.moe",
            hwnd: super::window::media_window_handle(window),
        })
        .map_err(|error| anyhow!("{error:?}"))?;
        controls
            .attach(move |event| {
                let _ = events.try_send(event);
            })
            .map_err(|error| anyhow!("{error:?}"))?;
        Ok(Self { controls })
    }

    pub fn update(
        &mut self,
        song: Option<&Song>,
        playing: bool,
        prefer_romaji: bool,
        volume: f32,
    ) -> Result<()> {
        self.controls
            .set_playback(if playing {
                MediaPlayback::Playing { progress: None }
            } else {
                MediaPlayback::Paused { progress: None }
            })
            .map_err(|error| anyhow!("{error:?}"))?;
        if let Some(song) = song {
            let artists = song.artist_names(prefer_romaji);
            let cover = song.cover_url();
            let album = song.albums.first().map(|album| {
                localized(
                    album.name.as_deref(),
                    album.name_romaji.as_deref(),
                    prefer_romaji,
                )
            });
            self.controls
                .set_metadata(MediaMetadata {
                    title: Some(song.title(prefer_romaji)),
                    artist: Some(&artists),
                    album,
                    cover_url: cover.as_deref(),
                    duration: Some(std::time::Duration::from_secs(song.duration)),
                })
                .map_err(|error| anyhow!("{error:?}"))?;
        }
        #[cfg(target_os = "linux")]
        self.controls
            .set_volume(f64::from(volume / 100.))
            .map_err(|error| anyhow!("{error:?}"))?;
        #[cfg(not(target_os = "linux"))]
        let _ = volume;
        Ok(())
    }
}
