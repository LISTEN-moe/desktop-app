//! Reproducible native profile: idle, audio (no animation), or visual.
//! Uses fixture metadata, muted live audio, and no credentials/preferences.
#[path = "../src/ui/mod.rs"]
#[allow(dead_code)]
mod ui;

use gpui_kit::*;
use listen_moe_desktop::radio::{
    artwork,
    audio::{self, Playback, Status},
    model::{Channel, GatewayMessage, parse_gateway_message},
    preferences::Preferences,
    service::Service,
    visualizer::artwork_color,
};
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mode = std::env::args().nth(1).unwrap_or_else(|| "visual".into());
    anyhow::ensure!(
        ["idle", "audio", "visual"].contains(&mode.as_str()),
        "Use idle, audio, or visual"
    );
    let seconds = std::env::args()
        .nth(2)
        .map(|value| value.parse::<u64>())
        .transpose()?
        .unwrap_or(30);
    let cover = std::env::args()
        .nth(3)
        .map(std::fs::read)
        .transpose()?
        .unwrap_or_else(|| include_bytes!("../tests/fixtures/visualizer-cover.jpg").to_vec());
    let original = std::env::args()
        .nth(4)
        .is_some_and(|value| value == "original");
    gpui_kit::application()
        .with_assets(assets::AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            ui::apply_theme(Channel::Jpop, cx);
            cx.set_reduce_motion(mode != "visual");
            ui::widget::bind_keys(cx);
            let (service, events, _sender, _commands) = Service::offline();
            let spectrum = service.spectrum.clone();
            let model = cx.new(|cx| {
                let mut state = ui::state::PlayerState::with_service(
                    Preferences::default(),
                    None,
                    service,
                    events,
                    cx,
                );
                let GatewayMessage::Track(song) =
                    parse_gateway_message(include_str!("../tests/fixtures/track-update.json"))
                        .unwrap()
                else {
                    unreachable!()
                };
                state.song = Some(song);
                let (cover, accent) = if original {
                    let accent = artwork_color(&cover).unwrap();
                    (cover, accent)
                } else {
                    let prepared = artwork::prepare(cover).unwrap();
                    (prepared.bytes, prepared.accent)
                };
                state.artwork = Some(ui::artwork_image(cover));
                state.visualizer_accent = accent;
                state.playing = mode != "idle";
                state.audio_status = if state.playing {
                    Status::Playing
                } else {
                    Status::Paused
                };
                state
            });
            ui::widget::open(model, cx).unwrap();
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();
            let (configuration, receiver) = tokio::sync::watch::channel(Playback {
                generation: 0,
                channel: Channel::Jpop,
                playing: mode != "idle",
                volume: 0.,
            });
            let (notifications, _events) = async_channel::bounded(32);
            runtime.spawn(audio::run(receiver, notifications, spectrum));
            let timeout = cx.background_executor().timer(Duration::from_secs(seconds));
            cx.spawn(async move |cx| {
                timeout.await;
                drop(configuration);
                drop(_events);
                drop(runtime);
                cx.update(|cx| cx.quit());
            })
            .detach();
            cx.activate(true);
        });
    Ok(())
}
