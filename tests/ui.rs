#[path = "../src/ui/mod.rs"]
#[allow(dead_code)]
mod ui;

fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    check::run()?;
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    println!("GPUI does not provide the offscreen GPU renderer on this platform");
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod check {
    use crate::ui::{state::PlayerState, widget};
    use gpui_kit::{test::TestWindowExt, *};
    use listen_moe_desktop::radio::{
        audio::Status,
        model::{GatewayMessage, parse_gateway_message},
        preferences::Preferences,
        service::Service,
        visualizer::{Analyzer, FFT_SIZE, SpectrumFrame, artwork_color},
    };
    use std::{sync::Arc, time::Duration};

    fn assert_control_consumes_press(window: &mut Window, id: &'static str, cx: &mut App) {
        let position = window.find(id).bounds().center();
        let result = window.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                button: MouseButton::Left,
                position,
                ..Default::default()
            }),
            cx,
        );
        assert!(
            !result.propagate,
            "{id} must consume the press before Windows starts a caption drag"
        );
        // Release outside the control to cancel the click without activating it.
        window.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                button: MouseButton::Left,
                position: point(px(-1.), px(-1.)),
                ..Default::default()
            }),
            cx,
        );
        window.render_frame(cx);
    }

    fn save_preferences_screenshot(
        context: &mut HeadlessAppContext,
        window: AnyWindowHandle,
        name: &str,
    ) -> anyhow::Result<()> {
        if let Some(directory) = std::env::var_os("LISTEN_MOE_SCREENSHOT_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory)?;
            context
                .capture_screenshot(window)?
                .save(directory.join(name))?;
        }
        Ok(())
    }

    fn assert_player_surfaces(window: &Window) {
        let bar = window.find("player-bar").bounds();
        let art = window.find("album-link").bounds();
        let scaled_bar = bar.scale(window.scale_factor());
        let quads = window.painted_quads();
        let surface = quads
            .iter()
            .find(|quad| quad.bounds == scaled_bar && !quad.background.is_transparent())
            .expect("opaque player surface");
        assert_eq!(surface.background.as_solid().unwrap().a, 1.);
        let gap = point(
            bar.right() + (art.left() - bar.right()) / 2.,
            bar.center().y,
        )
        .scale(window.scale_factor());
        assert!(
            quads.iter().all(|quad| {
                quad.background.is_transparent()
                    || !quad.bounds.contains(&gap)
                    || !quad.content_mask.bounds.contains(&gap)
            }),
            "the artwork gap stays unpainted with both artwork sizes"
        );
    }

    fn check_visualizer(context: &mut HeadlessAppContext) -> anyhow::Result<()> {
        let (service, events, sender, _commands) = Service::offline();
        let spectrum = service.spectrum.clone();
        let cover = include_bytes!("fixtures/visualizer-cover.jpg");
        let accent = artwork_color(cover).unwrap();
        let model = context.new(|cx| {
            let mut state =
                PlayerState::with_service(Preferences::default(), None, service, events, cx);
            let GatewayMessage::Track(song) =
                parse_gateway_message(include_str!("fixtures/track-update.json")).unwrap()
            else {
                panic!("track fixture");
            };
            state.song = Some(song);
            state.artwork = Some(Arc::new(Image::from_bytes(
                ImageFormat::Jpeg,
                cover.to_vec(),
            )));
            state.visualizer_accent = accent;
            state.playing = true;
            state.audio_status = Status::Playing;
            state
        });
        let (widget, view) = context.update(|cx| widget::open(model.clone(), cx))?;
        context.update_window(widget, |_, window, cx| window.render_frame(cx))?;
        // Deterministic multitone PCM exercises the real FFT and canvas, rather
        // than manually assigning decorative bar heights.
        let samples = std::array::from_fn(|i| {
            [3, 7, 13, 23, 37, 61, 97, 149, 223, 337]
                .iter()
                .map(|bin| {
                    (std::f32::consts::TAU * *bin as f32 * i as f32 / FFT_SIZE as f32).sin() * 0.04
                })
                .sum()
        });
        let bands = Analyzer::default().analyze(&samples, Duration::from_secs(1));
        spectrum.publish(SpectrumFrame {
            generation: 0,
            bands,
            captured_at: std::time::Instant::now(),
        });
        context.update(|cx| model.update(cx, |_, cx| cx.notify()));
        context.run_until_parked();
        let expected: Hsla =
            rgb((u32::from(accent[0]) << 16) | (u32::from(accent[1]) << 8) | u32::from(accent[2]))
                .into();
        let bars = |window: &Window| {
            window
                .painted_quads()
                .iter()
                .filter_map(|quad| {
                    let color = quad.background.as_solid()?;
                    // Identify the artwork tint, excluding translucent button
                    // fills and borders whose HSL saturation can be unstable
                    // when floating-point RGB values are nearly white.
                    (color.a > 0.
                        && color.a < 0.36
                        && (color.h - expected.h).abs() < 0.001
                        && (color.l - expected.l).abs() < 0.001)
                        .then_some((quad.bounds, color))
                })
                .collect::<Vec<_>>()
        };
        context.update_window(widget, |_, window, cx| {
            window.render_frame(cx);
            assert_player_surfaces(window);
            let bar = window
                .find("player-bar")
                .bounds()
                .scale(window.scale_factor());
            let quads = bars(window);
            assert!(
                quads.len() > 20,
                "real spectrum paints mirrored bars: {} quads",
                quads.len()
            );
            for (bounds, color) in quads {
                assert!(
                    bounds.top() >= bar.top() + bar.size.height * 0.3,
                    "spectrum {bounds:?}; player {bar:?}; color {color:?}"
                );
                assert!(bounds.bottom() <= bar.bottom());
                assert!((color.a - 0.35).abs() < 0.001);
                assert!((color.h - expected.h).abs() < 0.001);
            }
            assert!(
                view.read(cx).visualizer_animation_pending(cx),
                "playback schedules a capped animation tick"
            );
        })?;
        let control_renders = context.update(|cx| view.read(cx).controls_render_count(cx));
        context.advance_clock(Duration::from_millis(8));
        assert!(
            context.update(|cx| view.read(cx).visualizer_animation_pending(cx)),
            "the timer does not tick at a 120Hz monitor's next frame"
        );
        spectrum.publish(SpectrumFrame {
            generation: 0,
            bands: [0.; 64],
            captured_at: std::time::Instant::now(),
        });
        context.advance_clock(Duration::from_millis(10));
        context.update_window(widget, |_, window, cx| {
            // A normal draw, without render_frame's force-refresh, exercises
            // actual GPUI view caching and the timer's independent invalidation.
            window.draw(cx).clear(cx);
            assert!(
                bars(window).is_empty(),
                "the timer reads new spectrum without a model event"
            );
            assert_eq!(
                view.read(cx).controls_render_count(cx),
                control_renders,
                "audio frames reuse cached controls, text, and artwork"
            );
        })?;
        spectrum.publish(SpectrumFrame {
            generation: 0,
            bands,
            captured_at: std::time::Instant::now(),
        });
        context.advance_clock(Duration::from_millis(18));
        context.update_window(widget, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert!(
                !bars(window).is_empty(),
                "the next capped tick restores the spectrum"
            );
            assert_eq!(view.read(cx).controls_render_count(cx), control_renders);
        })?;
        if let Some(directory) = std::env::var_os("LISTEN_MOE_SCREENSHOT_DIR") {
            context
                .capture_screenshot(widget)?
                .save(std::path::PathBuf::from(directory).join("visualizer.png"))?;
        }
        context.update(|cx| {
            model.update(cx, |state, cx| {
                state.preferences.large_artwork = true;
                state.save_preferences(cx);
            });
        });
        context.run_until_parked();
        spectrum.publish(SpectrumFrame {
            generation: 0,
            bands,
            captured_at: std::time::Instant::now(),
        });
        context.update_window(widget, |_, window, cx| {
            window.bounds_changed(cx);
            window.render_frame(cx);
            assert_eq!(window.viewport_size().height, px(88.));
            assert_player_surfaces(window);
            let player = window
                .find("player-bar")
                .bounds()
                .scale(window.scale_factor());
            assert!(bars(window).iter().all(|(bounds, _)| {
                bounds.top() >= player.top() && bounds.bottom() <= player.bottom()
            }));
        })?;
        context.update_window(widget, |_, window, cx| {
            cx.set_reduce_motion(true);
            window.render_frame(cx);
            assert!(
                bars(window).is_empty(),
                "reduced motion suppresses the spectrum"
            );
            cx.set_reduce_motion(false);
            model.update(cx, |state, cx| {
                state.playing = false;
                cx.notify();
            });
            window.render_frame(cx);
        })?;
        context.advance_clock(Duration::from_millis(100));
        context.update_window(widget, |_, window, cx| {
            window.render_frame(cx);
            let quads = bars(window);
            assert!(!quads.is_empty(), "pause keeps bars during the fade");
            assert!(quads.iter().all(|(_, color)| color.a < 0.35));
        })?;
        context.advance_clock(Duration::from_millis(200));
        context.update_window(widget, |_, window, cx| {
            window.render_frame(cx);
            assert!(
                bars(window).is_empty(),
                "pause finishes without residual bars"
            );
            assert!(
                !view.read(cx).visualizer_animation_pending(cx),
                "idle stops scheduling frames"
            );
            model.update(cx, |state, cx| {
                state.playing = true;
                state.audio_status = Status::Playing;
                spectrum.publish(SpectrumFrame {
                    generation: 999,
                    bands,
                    captured_at: std::time::Instant::now(),
                });
                assert!(
                    state.visualizer_frame().is_none(),
                    "old playback is rejected"
                );
                cx.notify();
            });
        })?;
        sender.try_send(listen_moe_desktop::radio::service::Event::Artwork {
            generation: 999,
            song_id: 42,
            bytes: None,
            accent: None,
        })?;
        context.run_until_parked();
        assert_eq!(
            context.update(|cx| model.read(cx).visualizer_accent),
            accent
        );
        sender.try_send(listen_moe_desktop::radio::service::Event::Artwork {
            generation: 0,
            song_id: 42,
            bytes: None,
            accent: None,
        })?;
        context.run_until_parked();
        context.update(|cx| {
            let state = model.read(cx);
            assert!(
                state.artwork.is_none(),
                "failed cover clears previous artwork"
            );
            assert_eq!(
                state.visualizer_accent,
                listen_moe_desktop::radio::visualizer::color_from_rgb(
                    state.preferences.channel.accent()
                ),
                "failed cover restores the channel accent"
            );
        });
        println!(
            "PASS: native FFT canvas, artwork color and fallback, both artwork sizes and transparent gap, reduced motion, pause fade, stale playback rejection"
        );
        Ok(())
    }

    pub fn run() -> anyhow::Result<()> {
        let mut context = HeadlessAppContext::with_platform(
            gpui_kit::platform::current_platform(true).text_system(),
            Arc::new(assets::AllAssets),
            gpui_kit::platform::current_headless_renderer,
        );
        context.update(|cx| {
            gpui_kit::init(cx);
            widget::bind_keys(cx);
            crate::ui::apply_theme(Default::default(), cx);
        });
        let (service, events, event_sender, _commands) = Service::offline();
        let model = context
            .new(|cx| PlayerState::with_service(Preferences::default(), None, service, events, cx));
        context.update(|cx| {
            model.update(cx, |state, cx| {
                let GatewayMessage::Track(song) =
                    parse_gateway_message(include_str!("fixtures/track-update.json")).unwrap()
                else {
                    panic!("track fixture");
                };
                assert_eq!(
                    widget::alternate_metadata(&song, true),
                    "日本語の曲\n役 (CV: 歌手)"
                );
                assert_eq!(
                    widget::alternate_metadata(&song, false),
                    "Nihongo no Uta\nCharacter (CV: Singer)"
                );
                state.song = Some(song);
                cx.notify();
            })
        });
        let (widget, _) = context.update(|cx| widget::open(model.clone(), cx))?;
        context.update_window(widget, |_, window, cx| window.render_frame(cx))?;
        context.update_window(widget, |_, window, cx| {
            for id in [
                "play",
                "favorite",
                "volume",
                "preferences",
                "artist-link",
                "album-link",
            ] {
                assert_control_consumes_press(window, id, cx);
            }
        })?;
        let idle_icons = context.capture_screenshot(widget)?;
        let icon_bounds = context.update_window(widget, |_, window, cx| {
            window.hover("preferences", cx);
            window.find("preferences").bounds()
        })?;
        let hovered_icons = context.capture_screenshot(widget)?;
        let scale = idle_icons.width() as f32 / 560.;
        let mut brighter_pixels = 0;
        for y in (f32::from(icon_bounds.top()) * scale) as u32
            ..(f32::from(icon_bounds.bottom()) * scale) as u32
        {
            for x in (f32::from(icon_bounds.left()) * scale) as u32
                ..(f32::from(icon_bounds.right()) * scale) as u32
            {
                let before = idle_icons.get_pixel(x, y).0;
                let after = hovered_icons.get_pixel(x, y).0;
                if before != after {
                    assert!(
                        after[..3]
                            .iter()
                            .map(|value| u32::from(*value))
                            .sum::<u32>()
                            > before[..3]
                                .iter()
                                .map(|value| u32::from(*value))
                                .sum::<u32>()
                    );
                    brighter_pixels += 1;
                }
            }
        }
        assert!(
            brighter_pixels > 0,
            "hover brightens the actual icon pixels"
        );
        let background_pixel = (
            (f32::from(icon_bounds.left()) * scale) as u32 + 3,
            (f32::from(icon_bounds.top()) * scale) as u32 + 3,
        );
        assert_eq!(
            idle_icons.get_pixel(background_pixel.0, background_pixel.1),
            hovered_icons.get_pixel(background_pixel.0, background_pixel.1),
            "hover leaves the button background unchanged"
        );
        context.update_window(widget, |_, window, cx| {
            window.hover("track-title", cx);
            window.render_frame(cx);
            assert_eq!(window.viewport_size(), size(px(560.), px(72.)));
            assert_eq!(
                window.find("radio-widget").bounds(),
                Bounds::new(point(px(0.), px(0.)), window.viewport_size()),
                "the widget fills its window without a frame or shadow inset"
            );
            assert!(window.try_find("window-backdrop").is_none());
            let bar = window.find("player-bar").bounds();
            let art = window.find("album-link").bounds();
            assert_eq!(art.size, size(px(72.), px(72.)));
            assert_eq!(art.left() - bar.right(), px(12.));
            let track_width = window.find("track-area").bounds().size.width;
            for id in ["track-title", "artist-link"] {
                assert!(
                    window.find(id).bounds().size.width < track_width,
                    "{id} hover target follows its text instead of stretching across the row"
                );
            }
            window.press("tab", cx);
            let first_control = window.focused(cx).expect("first tab stop");
            window.press("tab", cx);
            assert_ne!(window.focused(cx), Some(first_control.clone()));
            window.press("shift-tab", cx);
            assert_eq!(window.focused(cx), Some(first_control));
            window.click("volume", cx);
            window.render_frame(cx);
            assert_eq!(window.viewport_size().height, px(72.));
        })?;
        context.advance_clock(Duration::from_millis(70));
        context.update_window(widget, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("volume-slider-control").is_some());
            assert!(
                window.try_find("track-title").is_some(),
                "metadata crossfades with volume"
            );
        })?;
        context.advance_clock(widget::VOLUME_FADE_DURATION);
        context.update_window(widget, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("track-title").is_none());
            assert_control_consumes_press(window, "volume-slider-control", cx);
            let bounds = window.find("volume-slider-control").bounds();
            window.click_at(
                "volume-slider-control",
                point(bounds.size.width * 0.8, bounds.size.height / 2.),
                cx,
            );
        })?;
        context.run_until_parked();
        assert!(
            context.update(|cx| model.read(cx).preferences.volume > 65.),
            "the faded-in slider remains usable"
        );
        context.update_window(widget, |_, window, cx| {
            window.press("escape", cx);
            assert!(
                window.try_find("volume-slider-control").is_some(),
                "exit keeps the slider mounted while fading"
            );
        })?;
        context.advance_clock(Duration::from_millis(70));
        context.update_window(widget, |_, window, cx| {
            window.render_frame(cx);
            window.click("volume", cx); // Reverse a partially completed exit.
        })?;
        context.advance_clock(widget::VOLUME_FADE_DURATION);
        context.update_window(widget, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("track-title").is_none());
            window.press("escape", cx);
        })?;
        context.advance_clock(widget::VOLUME_FADE_DURATION);
        context.update_window(widget, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("volume-slider-control").is_none());
            assert!(window.try_find("track-title").is_some());
            cx.set_reduce_motion(true);
            window.click("volume", cx);
            assert!(window.try_find("track-title").is_none());
            window.press("escape", cx);
            assert!(window.try_find("volume-slider-control").is_none());
            cx.set_reduce_motion(false);
            let position = window.find("track-title").bounds().center();
            let result = window.dispatch_event(
                PlatformInput::MouseDown(MouseDownEvent {
                    button: MouseButton::Right,
                    position,
                    ..Default::default()
                }),
                cx,
            );
            assert!(
                !result.propagate,
                "opening preferences consumes the Windows caption context-menu press"
            );
            assert!(model.read(cx).preferences_window.is_some());
            window.click("preferences", cx);
        })?;
        let original_song = context.update(|cx| model.read(cx).song.clone().unwrap());
        context.update(|cx| {
            model.update(cx, |state, cx| {
                let song = state.song.as_mut().unwrap();
                song.title_romaji = Some("A long song title ".repeat(40));
                song.artists[0].characters.clear();
                song.artists[0].name_romaji = Some("A long artist name ".repeat(40));
                cx.notify();
            })
        });
        context.update_window(widget, |_, window, cx| {
            window.render_frame(cx);
            let track = window.find("track-area").bounds();
            let controls = window.find("player-controls").bounds();
            for id in ["track-title", "artist-link"] {
                let text = window.find(id).bounds();
                assert!(text.size.width <= track.size.width);
                assert!(
                    text.right() < controls.left(),
                    "{id} cannot push or overlap controls"
                );
            }
        })?;
        context.update(|cx| {
            model.update(cx, |state, cx| {
                state.song = Some(original_song);
                cx.notify();
            })
        });
        let preferences = context.update(|cx| {
            model
                .read(cx)
                .preferences_window
                .expect("separate preferences window")
        });
        assert_ne!(widget.window_id(), preferences.window_id());
        context.update_window(preferences, |_, window, cx| window.render_frame(cx))?;
        save_preferences_screenshot(&mut context, preferences, "preferences-default.png")?;
        context.update_window(preferences, |_, window, cx| {
            window.render_frame(cx);
            assert_control_consumes_press(window, "close-preferences", cx);
            assert!(window.try_find("spacing").is_none());
            assert!(window.try_find("glass").is_none());
            assert_eq!(window.viewport_size(), size(px(440.), px(700.)));
            let content = window.find("preferences-content").bounds();
            let footer = window.find("preferences-footer").bounds();
            assert!(
                window.find("account-section").bounds().bottom() <= content.bottom(),
                "all default preferences fit without clipping the account row"
            );
            assert!(footer.top() >= content.bottom() + px(20.));
            assert!(footer.bottom() <= window.viewport_size().height - px(24.));
            assert!(window.try_find("username-field").is_none());
            window.click("show-sign-in", cx);
            window.render_frame(cx);
            assert!(window.try_find("username-field").is_some());
            assert!(window.try_find("password-field").is_some());
            assert_eq!(
                window.find("preferences-footer").bounds(),
                footer,
                "account form expansion keeps the footer fixed"
            );
            window.click("cancel-sign-in", cx);
            window.render_frame(cx);
            assert!(window.try_find("username-field").is_none());
            window.click("romaji-row", cx);
            window.render_frame(cx);
            assert!(
                !model.read(cx).preferences.prefer_romaji,
                "the full setting row is clickable"
            );
            window.click("romaji-row", cx);
            window.render_frame(cx);
            assert!(model.read(cx).preferences.prefer_romaji);
            window.click("KPOP", cx);
            window.render_frame(cx);
            assert_eq!(
                model.read(cx).preferences.channel,
                listen_moe_desktop::radio::model::Channel::Kpop
            );
            window.click("romaji", cx);
            window.render_frame(cx);
            assert!(!model.read(cx).preferences.prefer_romaji);
            window.click("artwork", cx);
            window.render_frame(cx);
        })?;
        context.update_window(widget, |_, window, cx| {
            window.bounds_changed(cx);
            window.render_frame(cx);
            assert_eq!(window.viewport_size().height, px(88.));
            assert_eq!(
                window.find("album-link").bounds().size,
                size(px(88.), px(88.))
            );
            assert_eq!(window.find("player-bar").bounds().size.height, px(72.));
            assert_player_surfaces(window);
        })?;
        context.update_window(preferences, |_, window, cx| {
            window.click("artwork", cx);
            window.render_frame(cx);
        })?;
        // A replaced channel's queued messages must never overwrite the new one.
        let GatewayMessage::Track(song) =
            parse_gateway_message(include_str!("fixtures/track-update.json")).unwrap()
        else {
            panic!("fixture");
        };
        event_sender.try_send(listen_moe_desktop::radio::service::Event::Track {
            generation: 0,
            song: song.clone(),
        })?;
        context.run_until_parked();
        assert!(context.update(|cx| model.read(cx).song.is_none()));
        event_sender.try_send(listen_moe_desktop::radio::service::Event::Track {
            generation: 1,
            song,
        })?;
        context.run_until_parked();
        assert_eq!(
            context.update(|cx| model.read(cx).song.as_ref().unwrap().id),
            42
        );
        context.update_window(widget, |_, window, cx| {
            window.blur(cx);
            window.bounds_changed(cx);
            window.render_frame(cx);
            assert_player_surfaces(window);
        })?;
        let screenshot = context.capture_screenshot(widget)?;
        context.advance_clock(Duration::from_millis(300)); // Settle switch springs for captures.
        // The headless WGPU renderer clears to opaque black, even for windows
        // requesting transparency. Check gap paint coverage above, not PNG alpha.
        let output = std::env::var_os("LISTEN_MOE_SCREENSHOT_DIR");
        if let Some(directory) = output {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory)?;
            screenshot.save(directory.join("widget.png"))?;
            hovered_icons.save(directory.join("icons-hover.png"))?;
            context
                .capture_screenshot(preferences)?
                .save(directory.join("preferences.png"))?;
        }
        context.update_window(preferences, |_, window, cx| {
            window.click("show-sign-in", cx);
            window.render_frame(cx);
            let content = window.find("preferences-content").bounds();
            let account = window.find("account-section").bounds();
            assert!(
                account.top() >= content.top() && account.bottom() <= content.bottom(),
                "opening sign in scrolls the full form and its actions into view"
            );
        })?;
        save_preferences_screenshot(&mut context, preferences, "preferences-sign-in.png")?;
        context.update(|cx| {
            model.update(cx, |state, cx| {
                state.temporary_token = Some("fixture-mfa".into());
                cx.notify();
            })
        });
        context.run_until_parked();
        context.update_window(preferences, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("code-field").is_some());
            assert!(window.try_find("password-field").is_none());
        })?;
        save_preferences_screenshot(&mut context, preferences, "preferences-mfa.png")?;
        context.update_window(preferences, |_, window, cx| {
            window.click("create-account", cx);
            window.render_frame(cx);
            assert!(model.read(cx).temporary_token.is_none());
            assert!(
                window.try_find("username-field").is_some(),
                "MFA back returns to sign in"
            );
            window.click("cancel-sign-in", cx);
            window.render_frame(cx);
        })?;
        context.update(|cx| {
            model.update(cx, |state, cx| {
                state.account = Some(crate::ui::state::Account {
                    user: listen_moe_desktop::radio::api::User {
                        username: "fixture".into(),
                        display_name: "Senchou".into(),
                    },
                    token: "fixture-only".into(),
                });
                cx.notify();
            })
        });
        context.run_until_parked();
        context.update_window(preferences, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("sign-out").is_some());
            assert!(window.try_find("username-field").is_none());
        })?;
        save_preferences_screenshot(&mut context, preferences, "preferences-signed-in.png")?;
        context.update_window(preferences, |_, window, cx| {
            window.click("sign-out", cx);
            window.render_frame(cx);
            assert!(model.read(cx).account.is_none());
            assert!(window.try_find("show-sign-in").is_some());
        })?;
        context.update_window(preferences, |_, window, cx| {
            window.click("close-preferences", cx);
        })?;
        assert!(context.update_window(preferences, |_, _, _| ()).is_err());
        context.update_window(widget, |_, window, cx| {
            window.bounds_changed(cx);
            window.render_frame(cx);
            assert_eq!(window.viewport_size().height, px(72.));
            assert_eq!(window.find("play").bounds().size, size(px(48.), px(48.)));
            for id in ["favorite", "volume", "preferences"] {
                assert_eq!(window.find(id).bounds().size, size(px(40.), px(40.)));
            }
        })?;
        println!(
            "PASS: native GPU rendering, detached artwork and unpainted gap, compact text hit areas and long-text truncation, alternate metadata, keyboard traversal, inline volume, grouped preferences, pinned footer, sign-in/MFA/sign-out, shared state, close lifecycle"
        );
        check_visualizer(&mut context)?;
        Ok(())
    }
}
