use super::{MUTED, PLAYER_SURFACE, icon_button, preferences, state::PlayerState};
use gpui_kit::base::{Transition, transition};
use std::time::Duration;

pub(crate) const VOLUME_FADE_DURATION: Duration = Duration::from_millis(140);
use super::visualizer::Visualizer;
use gpui_kit::component::{
    button::{ButtonCustomVariant, ButtonVariants},
    slider::{Slider, SliderEvent, SliderState, SliderValue},
    tooltip::Tooltip,
    *,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use listen_moe_desktop::platform::window as platform_window;
use listen_moe_desktop::radio::audio::Status;

gpui_kit::actions!(
    player,
    [
        TogglePlayback,
        OpenPreferences,
        CloseVolume,
        NextControl,
        PreviousControl,
        Quit
    ]
);

/// Keep the animated canvas and cached controls in sibling views: a spectrum
/// tick dirties this small container but leaves text, buttons and artwork cached.
pub struct Widget {
    model: Entity<PlayerState>,
    controls: Entity<PlayerControls>,
    visualizer: Entity<Visualizer>,
    _observer: Subscription,
}
impl Widget {
    fn new(model: Entity<PlayerState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let visualizer = cx.new(|cx| Visualizer::new(model.clone(), window, cx));
        let controls = cx.new(|cx| PlayerControls::new(model.clone(), window, cx));
        let observer = cx.observe(&model, |_, _, cx| cx.notify());
        Self {
            model,
            controls,
            visualizer,
            _observer: observer,
        }
    }
    #[cfg(feature = "ui-test")]
    #[allow(dead_code)] // Called by the external GPU integration test.
    pub fn visualizer_animation_pending(&self, cx: &App) -> bool {
        self.visualizer.read(cx).animation_pending()
    }
    #[cfg(feature = "ui-test")]
    #[allow(dead_code)]
    pub fn controls_render_count(&self, cx: &App) -> usize {
        self.controls.read(cx).render_count
    }
}
impl Render for Widget {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let height = self.model.read(cx).preferences.widget_height();
        div()
            .relative()
            .size_full()
            // Native drag hitboxes are not replayed with cached paint in GPUI.
            // Keep this on the outer view so visualizer frames retain dragging.
            .window_control_area(WindowControlArea::Drag)
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right(px(height + 12.))
                    .top(px((height - 72.) / 2.))
                    .h(px(72.))
                    .bg(rgb(PLAYER_SURFACE))
                    .rounded(px(12.))
                    .child(self.visualizer.clone()),
            )
            .child(
                self.controls
                    .clone()
                    .cached(StyleRefinement::default().size_full()),
            )
    }
}

struct PlayerControls {
    #[cfg(feature = "ui-test")]
    render_count: usize,
    model: Entity<PlayerState>,
    slider: Entity<SliderState>,
    volume_open: bool,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("space", TogglePlayback, Some("Player")),
        KeyBinding::new("tab", NextControl, Some("Player")),
        KeyBinding::new("shift-tab", PreviousControl, Some("Player")),
        KeyBinding::new("escape", CloseVolume, Some("Player")),
        KeyBinding::new("ctrl-,", OpenPreferences, Some("Player")),
        KeyBinding::new("cmd-,", OpenPreferences, Some("Player")),
        KeyBinding::new("ctrl-q", Quit, None),
        KeyBinding::new("cmd-q", Quit, None),
    ]);
}

pub fn open(
    model: Entity<PlayerState>,
    cx: &mut App,
) -> anyhow::Result<(AnyWindowHandle, Entity<Widget>)> {
    let height = model.read(cx).preferences.widget_height();
    // Kit's standard Root adds a border and 20px shadow insets to client-decorated
    // Linux windows. This widget owns its entire surface instead. Its buttons
    // use GPUI tooltips and focus traversal, without Component's window overlays.
    let handle = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(560.), px(height)), cx)),
            titlebar: None,
            window_decorations: Some(WindowDecorations::Client),
            is_resizable: false,
            is_minimizable: false,
            app_owns_titlebar_drag: true,
            window_background: WindowBackgroundAppearance::Transparent,
            app_id: Some("moe.listen.desktop".into()),
            window_min_size: Some(size(px(360.), px(72.))),
            ..Default::default()
        },
        |window, cx| {
            platform_window::configure_frameless(window);
            window.set_window_title("LISTEN.moe");
            window.set_rem_size(cx.theme().font_size);
            cx.new(|cx| Widget::new(model, window, cx))
        },
    )?;
    Ok((handle.into(), handle.entity(cx)?))
}

impl PlayerControls {
    pub fn new(model: Entity<PlayerState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let volume = model.read(cx).preferences.volume;
        let slider = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .step(1.)
                .default_value(volume)
        });
        let observer = cx.observe_in(&model, window, |this, model, window, cx| {
            let value = model.read(cx).preferences.volume;
            if this.slider.read(cx).value() != SliderValue::Single(value) {
                this.slider
                    .update(cx, |slider, cx| slider.set_value(value, window, cx));
            }
            cx.notify();
        });
        let slider_events = cx.subscribe(&slider, |this, _, event: &SliderEvent, cx| {
            if let SliderEvent::Change(SliderValue::Single(value)) = event {
                this.model
                    .update(cx, |state, cx| state.set_volume(*value, cx));
            }
        });
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        model.update(cx, |state, cx| state.attach_window(window, cx));
        Self {
            #[cfg(feature = "ui-test")]
            render_count: 0,
            model,
            slider,
            volume_open: false,
            focus,
            _subscriptions: vec![observer, slider_events],
        }
    }

    fn settings(&mut self, cx: &mut Context<Self>) {
        self.volume_open = false;
        if let Err(error) = preferences::open(self.model.clone(), cx) {
            self.model.update(cx, |state, cx| {
                state.notice = Some(error.to_string());
                cx.notify();
            });
        }
        cx.notify();
    }
}

pub(crate) fn alternate_metadata(
    song: &listen_moe_desktop::radio::model::Song,
    prefer_romaji: bool,
) -> String {
    format!(
        "{}\n{}",
        song.title(!prefer_romaji),
        song.artist_names(!prefer_romaji)
    )
}

impl Render for PlayerControls {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(feature = "ui-test")]
        {
            self.render_count += 1;
        }
        let volume_opacity = transition(
            "volume-opacity",
            if self.volume_open { 1.0_f32 } else { 0.0_f32 },
            Transition::new(VOLUME_FADE_DURATION),
            window,
            cx,
        );
        let state = self.model.read(cx);
        let prefer_romaji = state.preferences.prefer_romaji;
        let title = state
            .song
            .as_ref()
            .map(|song| song.title(prefer_romaji).to_owned())
            .unwrap_or_else(|| {
                if state.connected {
                    "Waiting for song data…"
                } else {
                    "Connecting…"
                }
                .into()
            });
        let artist = state
            .song
            .as_ref()
            .map(|song| song.artist_names(prefer_romaji))
            .unwrap_or_else(|| state.preferences.channel.label().into());
        let tooltip: SharedString = state
            .song
            .as_ref()
            .map(|song| {
                let mut label = alternate_metadata(song, prefer_romaji);
                if let Some(context) = song.context_label() {
                    label.push('\n');
                    label.push_str(&context);
                }
                label
            })
            .unwrap_or_else(|| title.clone())
            .into();
        let album_url = state.song.as_ref().and_then(|song| song.album_url());
        let artist_url = state.song.as_ref().and_then(|song| song.artist_url());
        let artwork = state.artwork.clone();
        let art_size = state.preferences.widget_height();
        let playing = state.playing;
        let favorite = state.favorite;
        let favorite_pending = state.favorite_pending;
        let no_song = state.song.is_none();
        let signed_in = state.account.is_some();
        let accent = state.preferences.channel.accent();
        let play_tooltip = match &state.audio_status {
            Status::Loading => "Connecting audio; click to cancel".to_owned(),
            Status::Error(error) => format!("{error}\nClick to stop retrying"),
            _ => if playing { "Pause" } else { "Play" }.into(),
        };
        let volume_label = format!("Volume {}%", state.preferences.volume.round());
        let play_icon = svg()
            .data(if playing {
                include_bytes!("../../assets/icons/pause.svg").as_slice()
            } else {
                include_bytes!("../../assets/icons/play.svg").as_slice()
            })
            .when(!playing, |icon| icon.ml(px(2.)));
        let metadata = div()
            .id("track-area")
            .test_support()
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .when(volume_opacity < 1., |element| {
                element.child(
                    div()
                        .id("track-metadata")
                        .test_support()
                        .absolute()
                        .inset_0()
                        .flex()
                        .flex_col()
                        .justify_center()
                        .opacity(1. - volume_opacity)
                        .child(
                            div()
                                .id("track-title")
                                .test_support()
                                .self_start()
                                .max_w_full()
                                .min_w_0()
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .truncate()
                                .when(!self.volume_open, |element| {
                                    element.tooltip(move |window, cx| {
                                        Tooltip::new(tooltip.clone()).build(window, cx)
                                    })
                                })
                                .child(title),
                        )
                        .child(
                            div()
                                .id("artist-link")
                                .test_support()
                                .self_start()
                                .max_w_full()
                                .min_w_0()
                                .text_size(px(13.))
                                .line_height(px(18.))
                                .mt(px(1.))
                                .text_color(rgb(MUTED))
                                .truncate()
                                .when_some(
                                    artist_url.filter(|_| !self.volume_open),
                                    |element, url| {
                                        element
                                            .role(Role::Link)
                                            .cursor_pointer()
                                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                                cx.stop_propagation()
                                            })
                                            .hover(|style| style.text_color(rgb(0xf0f0f2)))
                                            .on_click(move |_, _, cx| cx.open_url(&url))
                                    },
                                )
                                .child(artist),
                        ),
                )
            })
            .when(self.volume_open || volume_opacity > 0., |element| {
                element.child(
                    div()
                        .id("volume-slider-control")
                        .test_support()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .opacity(volume_opacity)
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(
                            Slider::new(&self.slider)
                                .accessibility_label("Volume")
                                .disabled(!self.volume_open)
                                .cursor_pointer()
                                .w_full(),
                        ),
                )
            });
        let controls = div()
            .id("player-controls")
            .test_support()
            .flex()
            .items_center()
            .flex_shrink_0()
            .child(
                icon_button(
                    cx,
                    "favorite",
                    svg().path(assets::IconName::Heart.path()),
                    if signed_in {
                        if favorite {
                            "Unfavorite song"
                        } else {
                            "Favorite song"
                        }
                    } else {
                        "Sign in to favorite"
                    },
                    if favorite { accent } else { MUTED },
                )
                .disabled(no_song || favorite_pending)
                .accessibility_label(if favorite {
                    "Unfavorite song"
                } else {
                    "Favorite song"
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    if this.model.read(cx).account.is_some() {
                        this.model.update(cx, |state, cx| state.toggle_favorite(cx));
                    } else {
                        this.settings(cx);
                    }
                })),
            )
            .child(
                div()
                    .id("volume-wheel")
                    .on_scroll(cx.listener(|this, event: &ScrollEvent, window, cx| {
                        let delta = event.delta.pixel_delta(px(16.)).y;
                        let current = this.model.read(cx).preferences.volume;
                        let next = current + if delta > px(0.) { 5. } else { -5. };
                        this.model
                            .update(cx, |state, cx| state.set_volume(next, cx));
                        this.slider.update(cx, |state, cx| {
                            state.set_value(next.clamp(0., 100.), window, cx)
                        });
                        cx.stop_propagation();
                    }))
                    .child(
                        icon_button(
                            cx,
                            "volume",
                            svg().path(assets::IconName::Volume2.path()),
                            volume_label,
                            MUTED,
                        )
                        .accessibility_label("Volume")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.volume_open = !this.volume_open;
                            cx.notify();
                        })),
                    ),
            )
            .child(
                icon_button(
                    cx,
                    "preferences",
                    svg().path(assets::IconName::Settings.path()),
                    "Preferences",
                    MUTED,
                )
                .on_click(cx.listener(|this, _, _, cx| this.settings(cx))),
            );
        let artwork = div()
            .id("album-link")
            .test_support()
            .relative()
            .flex_shrink_0()
            .size(px(art_size))
            .rounded(px(12.))
            .overflow_hidden()
            .when_some(album_url, |element, url| {
                element
                    .role(Role::Link)
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(move |_, _, cx| cx.open_url(&url))
            })
            .child(match artwork {
                Some(image) => img(image)
                    .size_full()
                    .rounded(px(12.))
                    .object_fit(ObjectFit::Cover)
                    .into_any_element(),
                None => div()
                    .size_full()
                    .bg(rgb(0x292f3d))
                    .rounded(px(12.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        Icon::new(assets::IconName::Music)
                            .with_size(px(28.))
                            .text_color(rgb(MUTED)),
                    )
                    .into_any_element(),
            })
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .rounded(px(12.))
                    .border_1()
                    .border_color(rgba(0xffffff1a)),
            );
        div()
            .id("radio-widget")
            .test_support()
            .key_context("Player")
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &TogglePlayback, _, cx| {
                this.model
                    .update(cx, |state, cx| state.set_playing(!state.playing, cx));
            }))
            .on_action(cx.listener(|this, _: &OpenPreferences, _, cx| this.settings(cx)))
            .on_action(cx.listener(|this, _: &CloseVolume, window, cx| {
                this.volume_open = false;
                window.focus(&this.focus, cx);
                cx.notify();
            }))
            .on_action(|_: &Quit, _, cx| cx.quit())
            .on_action(|_: &NextControl, window, cx| window.focus_next(cx))
            .on_action(|_: &PreviousControl, window, cx| window.focus_prev(cx))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _, _, cx| {
                    this.settings(cx);
                    cx.stop_propagation();
                }),
            )
            // Windows uses the native caption hit-test; start_window_move handles
            // Linux/macOS. Interactive children consume the press before Windows
            // can start moving, including buttons, links, and the volume slider.
            .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
            .cursor_move()
            .flex()
            .items_center()
            .gap(px(12.))
            .size_full()
            .font_family(cx.theme().font_family.clone())
            .text_size(px(14.))
            .text_color(rgb(0xf0f0f2))
            .child(
                div()
                    .id("player-bar")
                    .test_support()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .p(px(12.))
                    .flex_1()
                    .min_w_0()
                    .h(px(72.))
                    .rounded(px(12.))
                    .child(
                        icon_button(cx, "play", play_icon, play_tooltip, 0xf0f0f2)
                            .size(px(48.))
                            .flex_shrink_0()
                            .rounded(px(24.))
                            .custom(
                                ButtonCustomVariant::new(cx)
                                    .color(rgba(0xffffff14).into())
                                    .hover(rgba(0xffffff14).into())
                                    .active(rgba(0xffffff30).into()),
                            )
                            .border_1()
                            .border_color(rgba(0xffffff26))
                            .accessibility_label(if playing { "Pause" } else { "Play" })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.model
                                    .update(cx, |state, cx| state.set_playing(!state.playing, cx));
                            })),
                    )
                    .child(metadata)
                    .child(controls),
            )
            .child(artwork)
    }
}
