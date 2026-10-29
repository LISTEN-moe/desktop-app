use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use listen_moe_desktop::radio::{
    audio,
    model::{Channel, GatewayMessage, parse_gateway_message},
};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

pub fn check_live() -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        for channel in [Channel::Jpop, Channel::Kpop] {
            let title = tokio::time::timeout(Duration::from_secs(20), async {
                let (mut socket, _) = tokio_tungstenite::connect_async(channel.gateway_url()).await?;
                socket.send(Message::Text(r#"{"op":0,"d":{"auth":""}}"#.into())).await?;
                while let Some(message) = socket.next().await {
                    if let Message::Text(text) = message? {
                        match parse_gateway_message(&text)? {
                            GatewayMessage::Hello(_) => socket.send(Message::Text(r#"{"op":9}"#.into())).await?,
                            GatewayMessage::Track(song) => return Ok::<_, anyhow::Error>(song.title(true).to_owned()),
                            _ => {}
                        }
                    }
                }
                anyhow::bail!("No song received")
            }).await.context("Metadata timed out")??;
            println!("{}: live metadata OK ({title})", channel.label());
            let (sample_rate, channels, samples) = tokio::time::timeout(Duration::from_secs(25), audio::probe(channel)).await.context("Audio decoding timed out")??;
            println!("{}: live metadata OK ({title}); MP3 decode OK ({sample_rate} Hz, {channels} channels, {samples} samples)", channel.label());
        }
        Ok(())
    })
}

pub fn check_audio_device() -> Result<()> {
    use audio::{AudioEvent, Playback, Status};
    use tokio::sync::watch;
    let seconds = std::env::args()
        .skip_while(|argument| argument != "--check-audio-device")
        .nth(1)
        .map(|value| value.parse::<u64>())
        .transpose()
        .context("Audio check duration must be seconds")?
        .unwrap_or(2);
    anyhow::ensure!(
        (1..=3600).contains(&seconds),
        "Audio check duration must be 1..3600 seconds"
    );
    tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build()?.block_on(async {
        let (configuration, receiver) = watch::channel(Playback { generation: 0, channel: Channel::Jpop, playing: true, volume: 0. });
        let (events, notifications) = async_channel::bounded::<AudioEvent>(8);
        let spectrum = listen_moe_desktop::radio::visualizer::Spectrum::default();
        let driver = tokio::spawn(audio::run(receiver, events, spectrum.clone()));
        let result = tokio::time::timeout(Duration::from_secs(25 + seconds), async {
            loop {
                match notifications.recv().await?.status {
                    Status::Playing => break,
                    Status::Error(error) => anyhow::bail!("{error}"),
                    _ => {}
                }
            }
            let started = tokio::time::Instant::now();
            while started.elapsed() < Duration::from_secs(seconds) {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(250)) => {},
                    event = notifications.recv() => {
                        if let Status::Error(error) = event?.status { anyhow::bail!("{error}"); }
                    }
                }
            }
            anyhow::ensure!(spectrum.latest().is_some_and(|frame| frame.captured_at.elapsed() < Duration::from_millis(500) && frame.bands.iter().any(|value| *value > 24.)), "No live spectrum reached the visualizer at zero output volume");
            Ok::<_, anyhow::Error>(())
        }).await.context("Audio output timed out")?;
        drop(configuration);
        tokio::time::timeout(Duration::from_secs(5), driver).await.context("Audio shutdown timed out")??;
        result?;
        println!("PASS: live JPOP audio and spectrum reached the default output device at zero volume for {seconds}s; shutdown completed");
        Ok(())
    })
}
