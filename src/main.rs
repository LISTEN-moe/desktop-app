#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod diagnostics;
mod ui;

use gpui_kit::*;
use listen_moe_desktop::platform::application;
use listen_moe_desktop::radio::preferences::{Preferences, PreferencesStore};
use ui::{preferences::ClosePreferences, state::PlayerState};

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let _ = rustls::crypto::ring::default_provider().install_default();
    if std::env::args().any(|argument| argument == "--check-live") {
        return diagnostics::check_live();
    }
    if std::env::args().any(|argument| argument == "--check-audio-device") {
        return diagnostics::check_audio_device();
    }
    let store = PreferencesStore::system()?;
    let (preferences, notice) = match store.load() {
        Ok(preferences) => (preferences, None),
        Err(error) => (Preferences::default(), Some(error.to_string())),
    };
    gpui_kit::application()
        .with_assets(assets::AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            ui::apply_theme(preferences.channel, cx);
            cx.set_app_identity(application::APP_ID, application::APP_NAME);
            if let Err(error) = application::initialize() {
                eprintln!("Cannot configure the application icon: {error:#}");
            }
            ui::widget::bind_keys(cx);
            cx.bind_keys([KeyBinding::new(
                "escape",
                ClosePreferences,
                Some("Preferences"),
            )]);
            let model = cx.new(|cx| PlayerState::new(preferences, store, notice, cx));
            match ui::widget::open(model.clone(), cx) {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("Cannot open LISTEN.moe: {error:#}");
                    cx.quit();
                    return;
                }
            }
            let shutdown_model = model.clone();
            cx.on_app_quit(move |cx| {
                let complete = shutdown_model.read(cx).flush_preferences();
                async move {
                    let _ = complete.await;
                }
            })
            .detach();
            cx.on_window_closed(move |cx, closed| {
                let widget_closed = model
                    .read(cx)
                    .widget_window
                    .is_some_and(|window| window.window_id() == closed);
                if widget_closed || cx.windows().is_empty() {
                    cx.quit();
                    return;
                }
                let preferences_closed = model
                    .read(cx)
                    .preferences_window
                    .is_some_and(|window| window.window_id() == closed);
                if preferences_closed {
                    model.update(cx, |state, cx| {
                        state.preferences_window = None;
                        cx.notify();
                    });
                    if let Some(window) = model.read(cx).widget_window {
                        let _ = window.update(cx, |_, window, _| window.activate_window());
                    }
                }
            })
            .detach();
            cx.activate(true);
        });
    Ok(())
}
