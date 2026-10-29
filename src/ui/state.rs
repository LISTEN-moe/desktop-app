use gpui_kit::*;
use listen_moe_desktop::{
    platform::{
        media::MediaSession,
        presence::{NowPlaying, Presence, notify_event},
        window as platform_window,
    },
    radio::{
        api::User,
        audio::{Playback, Status},
        model::{Channel, Song},
        preferences::{Preferences, PreferencesStore},
        service::{Command, Event, Service},
        visualizer::{Color, Spectrum, SpectrumFrame, color_from_rgb},
    },
};
use souvlaki::MediaControlEvent;
use std::sync::Arc;

pub struct Account {
    pub user: User,
    pub token: String,
}

pub struct PlayerState {
    pub preferences: Preferences,
    pub song: Option<Song>,
    pub artwork: Option<Arc<Image>>,
    pub visualizer_accent: Color,
    pub spectrum: Spectrum,
    pub playing: bool,
    pub audio_status: Status,
    pub connected: bool,
    pub favorite: bool,
    pub favorite_pending: bool,
    pub account: Option<Account>,
    pub temporary_token: Option<String>,
    pub auth_pending: bool,
    pub notice: Option<String>,
    pub preferences_window: Option<AnyWindowHandle>,
    pub widget_window: Option<AnyWindowHandle>,
    pub can_pin: bool,
    service: Service,
    gateway_generation: u64,
    audio_generation: u64,
    auth_attempt: u64,
    favorite_attempt: u64,
    media: Option<MediaSession>,
    presence: Presence,
    last_event: Option<String>,
    received_first_track: bool,
    _events: Task<()>,
    _media_events: Option<Task<()>>,
}

impl PlayerState {
    pub fn visualizer_frame(&self) -> Option<SpectrumFrame> {
        if !self.playing || !matches!(self.audio_status, Status::Playing) {
            return None;
        }
        self.spectrum.latest().filter(|frame| {
            frame.generation == self.audio_generation
                && frame.captured_at.elapsed() < std::time::Duration::from_millis(250)
        })
    }

    pub fn new(
        preferences: Preferences,
        store: PreferencesStore,
        notice: Option<String>,
        cx: &mut Context<Self>,
    ) -> Self {
        let (service, receiver) = Service::start(preferences.clone(), store);
        let mut state = Self::with_service(preferences, notice, service, receiver, cx);
        state.send(Command::Restore { attempt: 0 });
        state
    }

    pub fn with_service(
        preferences: Preferences,
        notice: Option<String>,
        service: Service,
        receiver: async_channel::Receiver<Event>,
        cx: &mut Context<Self>,
    ) -> Self {
        let events = cx.spawn(async move |state, cx| {
            while let Ok(event) = receiver.recv().await {
                if state
                    .update(cx, |state, cx| state.apply_event(event, cx))
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            visualizer_accent: color_from_rgb(preferences.channel.accent()),
            spectrum: service.spectrum.clone(),
            preferences,
            song: None,
            artwork: None,
            playing: false,
            audio_status: Status::Paused,
            connected: false,
            favorite: false,
            favorite_pending: false,
            account: None,
            temporary_token: None,
            auth_pending: false,
            notice,
            preferences_window: None,
            widget_window: None,
            can_pin: false,
            service,
            gateway_generation: 0,
            audio_generation: 0,
            auth_attempt: 0,
            favorite_attempt: 0,
            media: None,
            presence: Presence::default(),
            last_event: None,
            received_first_track: false,
            _events: events,
            _media_events: None,
        }
    }

    pub fn attach_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.widget_window = Some(window.window_handle());
        self.can_pin = platform_window::supports_always_on_top(window);
        if self.preferences.always_on_top
            && self.can_pin
            && let Err(error) = platform_window::always_on_top(window, true)
        {
            self.notice = Some(error.to_string());
        }
        if cx.is_test() {
            return;
        }
        let (events, receiver) = async_channel::bounded(16);
        match MediaSession::new(window, events) {
            Ok(media) => self.media = Some(media),
            Err(error) => self.notice = Some(format!("Media controls unavailable: {error}")),
        }
        self._media_events = Some(cx.spawn(async move |state, cx| {
            while let Ok(event) = receiver.recv().await {
                if state
                    .update(cx, |state, cx| match event {
                        MediaControlEvent::Play => state.set_playing(true, cx),
                        MediaControlEvent::Pause | MediaControlEvent::Stop => {
                            state.set_playing(false, cx)
                        }
                        MediaControlEvent::Toggle => state.set_playing(!state.playing, cx),
                        MediaControlEvent::SetVolume(volume) if volume.is_finite() => {
                            state.set_volume((volume * 100.) as f32, cx)
                        }
                        MediaControlEvent::Raise => {
                            if let Some(window) = state.widget_window {
                                let _ = window.update(cx, |_, window, _| window.activate_window());
                            }
                        }
                        MediaControlEvent::Quit => cx.quit(),
                        _ => {}
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
        self.update_integrations();
    }

    fn send(&mut self, command: Command) {
        if let Err(error) = self.service.send(command) {
            self.notice = Some(error.to_string());
        }
    }

    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        if self.playing == playing {
            return;
        }
        self.playing = playing;
        self.audio_generation += 1;
        self.audio_status = if playing {
            Status::Loading
        } else {
            Status::Paused
        };
        self.configure_audio();
        self.update_integrations();
        cx.notify();
    }

    fn configure_audio(&mut self) {
        self.send(Command::Playback(Playback {
            channel: self.preferences.channel,
            generation: self.audio_generation,
            playing: self.playing,
            volume: self.preferences.volume,
        }));
    }

    pub fn set_volume(&mut self, value: f32, cx: &mut Context<Self>) {
        if !value.is_finite() {
            return;
        }
        self.preferences.volume = value.clamp(0., 100.);
        self.save_preferences(cx);
    }

    pub fn set_channel(&mut self, channel: Channel, cx: &mut Context<Self>) {
        if self.preferences.channel == channel {
            return;
        }
        self.preferences.channel = channel;
        self.gateway_generation += 1;
        self.audio_generation += 1;
        self.favorite_attempt += 1;
        self.song = None;
        self.artwork = None;
        self.visualizer_accent = color_from_rgb(channel.accent());
        self.favorite = false;
        self.favorite_pending = false;
        self.connected = false;
        self.received_first_track = false;
        self.last_event = None;
        self.send(Command::Channel {
            channel,
            generation: self.gateway_generation,
        });
        super::apply_theme(channel, cx);
        self.save_preferences(cx);
    }

    pub fn save_preferences(&mut self, cx: &mut Context<Self>) {
        self.send(Command::Save(self.preferences.clone()));
        self.configure_audio();
        self.update_integrations();
        if let Some(handle) = self.widget_window {
            let height = self.preferences.widget_height();
            let pin = self.preferences.always_on_top;
            let can_pin = self.can_pin;
            let result = handle.update(cx, |_, window, _| {
                let width = window.viewport_size().width;
                window.resize(size(width, px(height)));
                if can_pin {
                    platform_window::always_on_top(window, pin)
                } else {
                    Ok(())
                }
            });
            if let Ok(Err(error)) = result {
                self.notice = Some(error.to_string());
            }
        }
        cx.notify();
    }

    pub fn flush_preferences(&self) -> tokio::sync::oneshot::Receiver<()> {
        let (complete, receiver) = tokio::sync::oneshot::channel();
        let _ = self.service.send(Command::Flush {
            preferences: self.preferences.clone(),
            complete,
        });
        receiver
    }

    pub fn login(&mut self, username: String, password: String, cx: &mut Context<Self>) {
        if self.auth_pending {
            return;
        }
        if username.trim().is_empty() || password.is_empty() {
            self.notice = Some("Enter your username and password".into());
            cx.notify();
            return;
        }
        self.auth_attempt += 1;
        self.auth_pending = true;
        self.notice = None;
        self.send(Command::Login {
            attempt: self.auth_attempt,
            username,
            password,
        });
        cx.notify();
    }

    pub fn verify_mfa(&mut self, code: String, cx: &mut Context<Self>) {
        if self.auth_pending {
            return;
        }
        let Some(temporary_token) = self.temporary_token.clone() else {
            return;
        };
        if code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
            self.notice = Some("Enter the six-digit authentication code".into());
            cx.notify();
            return;
        }
        self.auth_attempt += 1;
        self.auth_pending = true;
        self.notice = None;
        self.send(Command::Mfa {
            attempt: self.auth_attempt,
            temporary_token,
            code,
        });
        cx.notify();
    }

    pub fn logout(&mut self, cx: &mut Context<Self>) {
        self.auth_attempt += 1;
        self.favorite_attempt += 1;
        self.account = None;
        self.temporary_token = None;
        self.auth_pending = false;
        self.favorite = false;
        self.favorite_pending = false;
        self.notice = None;
        self.send(Command::Logout);
        cx.notify();
    }

    pub fn toggle_favorite(&mut self, cx: &mut Context<Self>) {
        if self.favorite_pending {
            return;
        }
        let (Some(account), Some(song)) = (&self.account, &self.song) else {
            return;
        };
        self.favorite_attempt += 1;
        let command = Command::Favorite {
            attempt: self.favorite_attempt,
            song_id: song.id,
            token: account.token.clone(),
            toggle: true,
            current: self.favorite,
        };
        self.favorite_pending = true;
        self.send(command);
        cx.notify();
    }

    fn check_favorite(&mut self) {
        self.favorite_attempt += 1;
        self.favorite = false;
        self.favorite_pending = false;
        if let (Some(account), Some(song)) = (&self.account, &self.song) {
            let command = Command::Favorite {
                attempt: self.favorite_attempt,
                song_id: song.id,
                token: account.token.clone(),
                toggle: false,
                current: false,
            };
            self.favorite_pending = true;
            self.send(command);
        }
    }

    fn apply_event(&mut self, event: Event, cx: &mut Context<Self>) {
        match event {
            Event::Track { generation, song } if generation == self.gateway_generation => {
                let changed = self
                    .song
                    .as_ref()
                    .is_none_or(|old| old.id != song.id || old.start_time != song.start_time);
                let event = song.event.as_ref().map(|event| event.name.clone());
                if event != self.last_event
                    && self.received_first_track
                    && self.preferences.event_notifications
                    && let Some(event) = &event
                {
                    notify_event(event.clone());
                }
                self.last_event = event;
                self.received_first_track = true;
                if changed {
                    self.artwork = None;
                    if song.cover_url().is_none() {
                        self.visualizer_accent = color_from_rgb(self.preferences.channel.accent());
                    }
                    if let Some(url) = song.cover_url() {
                        self.send(Command::Artwork {
                            generation,
                            song_id: song.id,
                            url,
                        });
                    }
                }
                self.song = Some(song);
                if changed {
                    self.check_favorite();
                }
                self.update_integrations();
            }
            Event::Artwork {
                generation,
                song_id,
                bytes,
                accent,
            } if generation == self.gateway_generation
                && self.song.as_ref().is_some_and(|song| song.id == song_id) =>
            {
                self.artwork = bytes.map(super::artwork_image);
                self.visualizer_accent =
                    accent.unwrap_or_else(|| color_from_rgb(self.preferences.channel.accent()));
            }
            Event::Connection {
                generation,
                connected,
                message,
            } if generation == self.gateway_generation => {
                self.connected = connected;
                if let Some(message) = message {
                    self.notice = Some(format!("Metadata: {message}. Retrying."));
                }
            }
            Event::Audio(event) if event.generation == self.audio_generation => {
                self.audio_status = event.status;
                if let Status::Error(error) = &self.audio_status {
                    self.notice = Some(error.clone());
                }
                self.update_integrations();
            }
            Event::Auth { attempt, result } if attempt == self.auth_attempt => {
                self.auth_pending = false;
                match result {
                    Ok(Some(login)) if login.mfa => {
                        self.temporary_token = login.token;
                        if self.temporary_token.is_none() {
                            self.notice = Some(
                                "The server did not provide a 2FA session. Try signing in again."
                                    .into(),
                            );
                        }
                    }
                    Ok(Some(login)) => {
                        if let (Some(user), Some(token)) = (login.user, login.token) {
                            self.send(Command::Remember(token.clone()));
                            self.account = Some(Account { user, token });
                            self.temporary_token = None;
                            self.notice = None;
                            self.check_favorite();
                        } else {
                            self.notice =
                                Some("The server returned an incomplete sign-in response".into());
                        }
                    }
                    Ok(None) => {}
                    Err(error) => self.notice = Some(error),
                }
            }
            Event::Favorite {
                attempt,
                song_id,
                result,
            } if attempt == self.favorite_attempt
                && self.song.as_ref().is_some_and(|song| song.id == song_id) =>
            {
                self.favorite_pending = false;
                match result {
                    Ok(favorite) => self.favorite = favorite,
                    Err(error) => self.notice = Some(error),
                }
            }
            Event::Notice(message) => self.notice = Some(message),
            _ => return, // Ignore completions from a replaced channel, track, or session.
        }
        cx.notify();
    }

    fn update_integrations(&mut self) {
        let audible = matches!(self.audio_status, Status::Playing) && self.playing;
        if let Some(media) = &mut self.media
            && let Err(error) = media.update(
                self.song.as_ref(),
                audible,
                self.preferences.prefer_romaji,
                self.preferences.volume,
            )
        {
            self.notice = Some(format!("Media controls: {error}"));
        }
        let presence = if self.preferences.discord_presence && audible {
            self.song.as_ref().map(|song| NowPlaying {
                title: song.title(self.preferences.prefer_romaji).into(),
                artists: song.artist_names(self.preferences.prefer_romaji),
                channel: self.preferences.channel.label(),
            })
        } else {
            None
        };
        self.presence.update(presence);
    }
}
