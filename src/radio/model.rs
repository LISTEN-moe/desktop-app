use std::time::Duration;

use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Jpop,
    Kpop,
}

impl Channel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Jpop => "JPOP",
            Self::Kpop => "KPOP",
        }
    }

    pub fn gateway_url(self) -> &'static str {
        match self {
            Self::Jpop => "wss://listen.moe/gateway_v2",
            Self::Kpop => "wss://listen.moe/kpop/gateway_v2",
        }
    }

    // v7 uses this MP3 endpoint when Opus is unavailable. It gives the native
    // decoder the same stream on every desktop without an external codec pack.
    pub fn stream_url(self) -> &'static str {
        match self {
            Self::Jpop => "https://listen.moe/fallback",
            Self::Kpop => "https://listen.moe/kpop/fallback",
        }
    }

    pub fn accent(self) -> u32 {
        match self {
            Self::Jpop => 0xff015b,
            Self::Kpop => 0x30a9ed,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: u64,
    #[serde(default)]
    pub name: Option<String>,
    pub name_romaji: Option<String>,
    #[serde(default)]
    pub characters: Vec<Character>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub name: Option<String>,
    pub name_romaji: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: u64,
    pub name: Option<String>,
    pub name_romaji: Option<String>,
    pub image: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Requester {
    pub username: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BroadcastEvent {
    pub name: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub id: u64,
    pub title: Option<String>,
    pub title_romaji: Option<String>,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub albums: Vec<Album>,
    #[serde(default)]
    pub duration: u64,
    pub start_time: Option<String>,
    pub requester: Option<Requester>,
    pub event: Option<BroadcastEvent>,
}

pub fn localized<'a>(
    native: Option<&'a str>,
    romaji: Option<&'a str>,
    prefer_romaji: bool,
) -> &'a str {
    let native = native.filter(|value| !value.trim().is_empty());
    let romaji = romaji.filter(|value| !value.trim().is_empty());
    if prefer_romaji {
        romaji.or(native)
    } else {
        native.or(romaji)
    }
    .unwrap_or("Unknown")
}

impl Song {
    pub fn title(&self, prefer_romaji: bool) -> &str {
        localized(
            self.title.as_deref(),
            self.title_romaji.as_deref(),
            prefer_romaji,
        )
    }

    pub fn artist_names(&self, prefer_romaji: bool) -> String {
        let names: Vec<_> = self
            .artists
            .iter()
            .map(|artist| {
                let name = localized(
                    artist.name.as_deref(),
                    artist.name_romaji.as_deref(),
                    prefer_romaji,
                );
                if artist.characters.is_empty() {
                    return name.to_owned();
                }
                let characters = artist
                    .characters
                    .iter()
                    .map(|character| {
                        localized(
                            character.name.as_deref(),
                            character.name_romaji.as_deref(),
                            prefer_romaji,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{characters} (CV: {name})")
            })
            .collect();
        if names.is_empty() {
            "Unknown artist".into()
        } else {
            names.join(", ")
        }
    }

    pub fn cover_url(&self) -> Option<String> {
        let name = self.albums.first()?.image.as_deref()?.trim();
        if name.is_empty() {
            return None;
        }
        let mut url = Url::parse("https://cdn.listen.moe/covers/").ok()?;
        // Treat the server's filename as one path segment, including Japanese,
        // spaces, #, and ?. Never concatenate unescaped data into a URL.
        url.path_segments_mut().ok()?.pop_if_empty().push(name);
        Some(url.into())
    }

    pub fn album_url(&self) -> Option<String> {
        self.albums
            .first()
            .map(|album| format!("https://listen.moe/albums/{}", album.id))
    }

    pub fn artist_url(&self) -> Option<String> {
        self.artists
            .first()
            .map(|artist| format!("https://listen.moe/artists/{}", artist.id))
    }

    pub fn context_label(&self) -> Option<String> {
        self.event
            .as_ref()
            .map(|event| event.name.clone())
            .or_else(|| {
                self.requester
                    .as_ref()
                    .map(|requester| format!("Requested by {}", requester.display_name))
            })
    }
}

#[derive(Deserialize)]
struct Envelope {
    op: u8,
    d: Option<GatewayData>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GatewayData {
    heartbeat: Option<u64>,
    song: Option<Song>,
    start_time: Option<String>,
    requester: Option<Requester>,
    event: Option<BroadcastEvent>,
}

#[derive(Debug)]
pub enum GatewayMessage {
    Hello(Duration),
    Track(Song),
    HeartbeatAck,
    Ignore,
}

pub fn parse_gateway_message(json: &str) -> serde_json::Result<GatewayMessage> {
    let envelope: Envelope = serde_json::from_str(json)?;
    let data = envelope.d.unwrap_or_default();
    Ok(match envelope.op {
        0 => GatewayMessage::Hello(Duration::from_millis(
            data.heartbeat.unwrap_or(45_000).clamp(1000, 120_000),
        )),
        1 => match data.song {
            Some(mut song) => {
                song.start_time = data.start_time.or(song.start_time);
                song.requester = data.requester.or(song.requester);
                song.event = data.event.or(song.event);
                GatewayMessage::Track(song)
            }
            None => GatewayMessage::Ignore,
        },
        10 => GatewayMessage::HeartbeatAck,
        _ => GatewayMessage::Ignore,
    })
}

pub fn slider_to_gain(volume: f32) -> f32 {
    (4_f32.powf(volume.clamp(0., 100.) / 100.) - 1.) / 3.
}
