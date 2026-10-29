use super::{
    api::{Api, Login},
    artwork,
    audio::{self, AudioEvent, Playback},
    model::{Channel, GatewayMessage, Song, parse_gateway_message},
    preferences::{Preferences, PreferencesStore},
    visualizer::Spectrum,
};
use crate::platform::credentials;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::time::{Duration, Instant};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tokio_tungstenite::tungstenite::Message;

pub enum Command {
    Channel {
        channel: Channel,
        generation: u64,
    },
    Playback(Playback),
    Save(Preferences),
    Flush {
        preferences: Preferences,
        complete: tokio::sync::oneshot::Sender<()>,
    },
    Restore {
        attempt: u64,
    },
    Login {
        attempt: u64,
        username: String,
        password: String,
    },
    Mfa {
        attempt: u64,
        temporary_token: String,
        code: String,
    },
    Remember(String),
    Logout,
    Favorite {
        attempt: u64,
        song_id: u64,
        token: String,
        toggle: bool,
        current: bool,
    },
    Artwork {
        generation: u64,
        song_id: u64,
        url: String,
    },
}

pub enum Event {
    Track {
        generation: u64,
        song: Song,
    },
    Connection {
        generation: u64,
        connected: bool,
        message: Option<String>,
    },
    Audio(AudioEvent),
    Auth {
        attempt: u64,
        result: Result<Option<Login>, String>,
    },
    Favorite {
        attempt: u64,
        song_id: u64,
        result: Result<bool, String>,
    },
    Artwork {
        generation: u64,
        song_id: u64,
        bytes: Option<Vec<u8>>,
        accent: Option<[u8; 3]>,
    },
    Notice(String),
}

#[derive(Clone)]
pub struct Service {
    commands: mpsc::UnboundedSender<Command>,
    pub spectrum: Spectrum,
}

impl Service {
    pub fn start(
        preferences: Preferences,
        store: PreferencesStore,
    ) -> (Self, async_channel::Receiver<Event>) {
        let (commands, receiver) = mpsc::unbounded_channel();
        let (events, notifications) = async_channel::bounded(64);
        let spectrum = Spectrum::default();
        let audio_spectrum = spectrum.clone();
        std::thread::spawn(move || {
            match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
            {
                Ok(runtime) => {
                    runtime.block_on(run(receiver, events, preferences, store, audio_spectrum))
                }
                Err(error) => {
                    let _ = events.send_blocking(Event::Notice(format!(
                        "Cannot start radio services: {error}"
                    )));
                }
            }
        });
        (Self { commands, spectrum }, notifications)
    }

    #[cfg(feature = "ui-test")]
    pub fn offline() -> (
        Self,
        async_channel::Receiver<Event>,
        async_channel::Sender<Event>,
        mpsc::UnboundedReceiver<Command>,
    ) {
        let (commands, receiver) = mpsc::unbounded_channel();
        let (sender, events) = async_channel::bounded(16);
        (
            Self {
                commands,
                spectrum: Spectrum::default(),
            },
            events,
            sender,
            receiver,
        )
    }

    pub fn send(&self, command: Command) -> Result<()> {
        self.commands
            .send(command)
            .map_err(|_| anyhow::anyhow!("Radio service has stopped"))
    }
}

async fn run(
    mut commands: mpsc::UnboundedReceiver<Command>,
    events: async_channel::Sender<Event>,
    preferences: Preferences,
    store: PreferencesStore,
    spectrum: Spectrum,
) {
    let api = match Api::new() {
        Ok(api) => api,
        Err(error) => {
            let _ = events
                .send(Event::Notice(format!("Cannot initialize API: {error}")))
                .await;
            return;
        }
    };
    let mut gateway = tokio::spawn(gateway(preferences.channel, 0, events.clone()));
    let (playback, config) = watch::channel(Playback {
        channel: preferences.channel,
        generation: 0,
        playing: false,
        volume: preferences.volume,
    });
    let (audio_events, receiver) = async_channel::bounded(16);
    let audio_task = tokio::spawn(audio::run(config, audio_events, spectrum));
    let forward_events = events.clone();
    let audio_forward = tokio::spawn(async move {
        while let Ok(event) = receiver.recv().await {
            if forward_events.send(Event::Audio(event)).await.is_err() {
                break;
            }
        }
    });
    let mut pending_preferences = None;
    let mut save_interval = tokio::time::interval(Duration::from_millis(400));
    let mut artwork_task: Option<JoinHandle<()>> = None;
    loop {
        let command = tokio::select! {
            command = commands.recv() => match command { Some(command) => command, None => break },
            _ = save_interval.tick() => {
                if let Some(preferences) = pending_preferences.take() {
                    save_preferences(store.clone(), preferences, &events).await;
                }
                continue;
            }
        };
        let api = api.clone();
        let events = events.clone();
        match command {
            Command::Channel {
                channel,
                generation,
            } => {
                gateway.abort();
                if let Some(task) = artwork_task.take() {
                    task.abort();
                }
                gateway = tokio::spawn(self::gateway(channel, generation, events));
            }
            Command::Playback(configuration) => {
                let _ = playback.send(configuration);
            }
            Command::Save(preferences) => pending_preferences = Some(preferences),
            Command::Flush {
                preferences,
                complete,
            } => {
                pending_preferences = None;
                save_preferences(store.clone(), preferences, &events).await;
                let _ = complete.send(());
            }
            Command::Remember(token) => {
                if let Err(error) =
                    tokio::task::spawn_blocking(move || credentials::save_token(&token))
                        .await
                        .unwrap_or_else(|error| Err(error.into()))
                {
                    let _ = events
                        .send(Event::Notice(format!(
                            "Signed in for this session. Credential storage: {error}"
                        )))
                        .await;
                }
            }
            Command::Logout => {
                if let Err(error) = tokio::task::spawn_blocking(credentials::delete_token)
                    .await
                    .unwrap_or_else(|error| Err(error.into()))
                {
                    let _ = events
                        .send(Event::Notice(format!(
                            "Cannot remove saved session: {error}"
                        )))
                        .await;
                }
            }
            Command::Restore { attempt } => {
                tokio::spawn(async move {
                    let result = async {
                        let token = tokio::task::spawn_blocking(credentials::load_token).await??;
                        let Some(token) = token else {
                            return Ok(None);
                        };
                        let user = api.user(&token).await?;
                        Ok(Some(Login {
                            token: Some(token),
                            user: Some(user),
                            mfa: false,
                        }))
                    }
                    .await
                    .map_err(|error: anyhow::Error| error.to_string());
                    let _ = events.send(Event::Auth { attempt, result }).await;
                });
            }
            Command::Login {
                attempt,
                username,
                password,
            } => {
                tokio::spawn(async move {
                    let result = api
                        .login(&username, &password)
                        .await
                        .map(Some)
                        .map_err(|error| error.to_string());
                    let _ = events.send(Event::Auth { attempt, result }).await;
                });
            }
            Command::Mfa {
                attempt,
                temporary_token,
                code,
            } => {
                tokio::spawn(async move {
                    let result = api
                        .verify_mfa(&temporary_token, &code)
                        .await
                        .map(Some)
                        .map_err(|error| error.to_string());
                    let _ = events.send(Event::Auth { attempt, result }).await;
                });
            }
            Command::Favorite {
                attempt,
                song_id,
                token,
                toggle,
                current,
            } => {
                tokio::spawn(async move {
                    let result = async {
                        if toggle {
                            api.toggle_favorite(&token, song_id).await?;
                            Ok(!current)
                        } else {
                            api.is_favorite(&token, song_id).await
                        }
                    }
                    .await
                    .map_err(|error: anyhow::Error| error.to_string());
                    let _ = events
                        .send(Event::Favorite {
                            attempt,
                            song_id,
                            result,
                        })
                        .await;
                });
            }
            Command::Artwork {
                generation,
                song_id,
                url,
            } => {
                if let Some(task) = artwork_task.take() {
                    task.abort();
                }
                artwork_task = Some(tokio::spawn(async move {
                    let (bytes, accent) = match api.cover(&url).await {
                        Ok(bytes) => tokio::task::spawn_blocking(move || {
                            artwork::prepare(bytes).map_or((None, None), |artwork| {
                                (Some(artwork.bytes), Some(artwork.accent))
                            })
                        })
                        .await
                        .unwrap_or((None, None)),
                        Err(_) => (None, None),
                    };
                    let _ = events
                        .send(Event::Artwork {
                            generation,
                            song_id,
                            bytes,
                            accent,
                        })
                        .await;
                }));
            }
        }
    }
    if let Some(preferences) = pending_preferences {
        save_preferences(store, preferences, &events).await;
    }
    gateway.abort();
    if let Some(task) = artwork_task {
        task.abort();
    }
    drop(playback);
    let _ = audio_task.await;
    audio_forward.abort();
}

async fn save_preferences(
    store: PreferencesStore,
    preferences: Preferences,
    events: &async_channel::Sender<Event>,
) {
    if let Err(error) = tokio::task::spawn_blocking(move || store.save(&preferences))
        .await
        .unwrap_or_else(|error| Err(error.into()))
    {
        let _ = events.send(Event::Notice(error.to_string())).await;
    }
}

pub async fn gateway(channel: Channel, generation: u64, events: async_channel::Sender<Event>) {
    loop {
        let result = gateway_connection(channel, generation, &events).await;
        if events.is_closed() {
            break;
        }
        let _ = events
            .send(Event::Connection {
                generation,
                connected: false,
                message: result.err().map(|error| error.to_string()),
            })
            .await;
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn gateway_connection(
    channel: Channel,
    generation: u64,
    events: &async_channel::Sender<Event>,
) -> Result<()> {
    let (mut socket, _) = tokio::time::timeout(
        Duration::from_secs(15),
        tokio_tungstenite::connect_async(channel.gateway_url()),
    )
    .await??;
    socket
        .send(Message::Text(r#"{"op":0,"d":{"auth":""}}"#.into()))
        .await?;
    events
        .send(Event::Connection {
            generation,
            connected: true,
            message: None,
        })
        .await?;
    let mut heartbeat = tokio::time::interval(Duration::from_secs(45));
    heartbeat.tick().await;
    let mut last_message = Instant::now();
    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                anyhow::ensure!(last_message.elapsed() < Duration::from_secs(120), "Metadata heartbeat timed out");
                socket.send(Message::Text(r#"{"op":9}"#.into())).await?;
            }
            message = socket.next() => {
                let Some(message) = message else { anyhow::bail!("Metadata connection closed"); };
                last_message = Instant::now();
                match message? {
                    Message::Text(text) => match parse_gateway_message(&text)? {
                        GatewayMessage::Hello(interval) => {
                            heartbeat = tokio::time::interval(interval);
                            heartbeat.tick().await;
                            socket.send(Message::Text(r#"{"op":9}"#.into())).await?;
                        }
                        GatewayMessage::Track(song) => { events.send(Event::Track { generation, song }).await?; }
                        _ => {}
                    }
                    Message::Ping(payload) => socket.send(Message::Pong(payload)).await?,
                    Message::Close(_) => anyhow::bail!("Metadata connection closed"),
                    _ => {}
                }
            }
        }
    }
}
