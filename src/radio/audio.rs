use super::model::{Channel, slider_to_gain};
use super::visualizer::Spectrum;
use anyhow::{Context, Result};
use rodio::{Decoder, OutputStreamBuilder, Sink, Source};
use std::{num::NonZeroUsize, sync::mpsc, time::Duration};
use stream_download::{
    Settings, StreamDownload,
    http::HttpStream,
    source::DecodeError,
    storage::{bounded::BoundedStorageProvider, memory::MemoryStorageProvider},
};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
mod buffer;
mod visualizer;

#[derive(Clone, Copy)]
pub struct Playback {
    pub generation: u64,
    pub channel: Channel,
    pub playing: bool,
    pub volume: f32,
}

#[derive(Clone, Debug)]
pub enum Status {
    Loading,
    Playing,
    Paused,
    Error(String),
}

pub struct AudioEvent {
    pub generation: u64,
    pub status: Status,
}

type AudioSource = Box<dyn Source<Item = f32> + Send>;
struct Prepared {
    source: Option<AudioSource>,
    cancel: CancellationToken,
    stats: Option<buffer::Stats>,
}
impl Drop for Prepared {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

enum OutputCommand {
    Replace(Prepared, Playback),
    Volume(f32),
    Stop,
    Shutdown,
}

async fn prepare(channel: Channel) -> Result<Prepared> {
    let client = stream_http_client::Client::builder()
        .user_agent("LISTEN.moe Desktop/0.1")
        .http1_only()
        .connect_timeout(Duration::from_secs(10))
        .build()?;
    let stream = HttpStream::new(client, channel.stream_url().parse()?)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let reader = StreamDownload::from_stream(
        stream,
        BoundedStorageProvider::new(
            MemoryStorageProvider,
            NonZeroUsize::new(512 * 1024).expect("nonzero buffer"),
        ),
        Settings::default()
            .prefetch_bytes(16 * 1024)
            .retry_timeout(Duration::from_secs(10)),
    )
    .await;
    let reader = match reader {
        Ok(reader) => reader,
        Err(error) => anyhow::bail!("{}", error.decode_error().await),
    };
    // This guard also cancels an in-progress decoder if its async owner is dropped.
    let mut prepared = Prepared {
        source: None,
        cancel: reader.cancellation_token(),
        stats: None,
    };
    let (decoder, stats) = tokio::task::spawn_blocking(move || {
        Decoder::builder()
            .with_data(reader)
            .with_hint("mp3")
            .with_seekable(false)
            .build()
            .map_err(anyhow::Error::from)
            .and_then(|decoder| buffer::prepare(Box::new(decoder)))
    })
    .await??;
    prepared.source = Some(decoder);
    prepared.stats = Some(stats);
    Ok(prepared)
}

// CPAL's device stream stays on this thread, including on platforms where it
// cannot move between threads. No network read or device call runs on the UI.
fn output_thread(
    commands: mpsc::Receiver<OutputCommand>,
    events: async_channel::Sender<AudioEvent>,
    spectrum: Spectrum,
) {
    let mut output = None;
    let mut sink: Option<Sink> = None;
    let mut current: Option<(Prepared, u64)> = None;
    let mut last_buffer_report = std::time::Instant::now();
    loop {
        match commands.recv_timeout(Duration::from_millis(200)) {
            Ok(OutputCommand::Replace(mut prepared, playback)) => {
                current.take(); // cancel download before stopping a blocked reader
                if let Some(sink) = sink.take() {
                    sink.stop();
                }
                if output.is_none() {
                    match OutputStreamBuilder::open_default_stream() {
                        Ok(mut stream) => {
                            stream.log_on_drop(false);
                            output = Some(stream);
                        }
                        Err(error) => {
                            let _ = events.send_blocking(AudioEvent {
                                generation: playback.generation,
                                status: Status::Error(format!("Audio device: {error}")),
                            });
                            continue;
                        }
                    }
                }
                let player =
                    Sink::connect_new(output.as_ref().expect("initialized output").mixer());
                player.set_volume(slider_to_gain(playback.volume));
                if let Some(source) = prepared.source.take() {
                    player.append(visualizer::tap(
                        source,
                        playback.generation,
                        spectrum.clone(),
                    ));
                }
                current = Some((prepared, playback.generation));
                sink = Some(player);
                let _ = events.send_blocking(AudioEvent {
                    generation: playback.generation,
                    status: Status::Playing,
                });
            }
            Ok(OutputCommand::Volume(volume)) => {
                if let Some(sink) = &sink {
                    sink.set_volume(slider_to_gain(volume));
                }
            }
            Ok(OutputCommand::Stop) => {
                current.take();
                if let Some(sink) = sink.take() {
                    sink.stop();
                }
                // Paused radio has no source to service; stop device callbacks too.
                output = None;
            }
            Ok(OutputCommand::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if last_buffer_report.elapsed() >= Duration::from_secs(1) {
                    if let Some((prepared, _)) = &current {
                        let count = prepared
                            .stats
                            .as_ref()
                            .map_or(0, buffer::Stats::take_underflows);
                        if count > 0 {
                            eprintln!(
                                "Audio PCM buffer ran empty {count} times; decoder or network could not keep up"
                            );
                        }
                    }
                    last_buffer_report = std::time::Instant::now();
                }
                if sink.as_ref().is_some_and(Sink::empty) {
                    if let Some((_, generation)) = current.take() {
                        let _ = events.send_blocking(AudioEvent {
                            generation,
                            status: Status::Error("Audio stream ended; reconnecting".into()),
                        });
                    }
                    sink = None;
                }
            }
        }
    }
    current.take();
    if let Some(sink) = sink {
        sink.stop();
    }
}

pub async fn run(
    configuration: watch::Receiver<Playback>,
    events: async_channel::Sender<AudioEvent>,
    spectrum: Spectrum,
) {
    let (output, commands) = mpsc::channel();
    let (output_events, notifications) = async_channel::bounded(8);
    let worker = std::thread::spawn(move || output_thread(commands, output_events, spectrum));
    control(configuration, events, output, notifications).await;
    // The controller drops the last sender on every exit path. Await device
    // teardown outside the async executor rather than leaving an output thread.
    let _ = tokio::task::spawn_blocking(move || worker.join()).await;
}

async fn control(
    mut configuration: watch::Receiver<Playback>,
    events: async_channel::Sender<AudioEvent>,
    output: mpsc::Sender<OutputCommand>,
    notifications: async_channel::Receiver<AudioEvent>,
) {
    loop {
        let playback = *configuration.borrow_and_update();
        let _ = output.send(OutputCommand::Stop);
        if !playback.playing {
            let _ = events
                .send(AudioEvent {
                    generation: playback.generation,
                    status: Status::Paused,
                })
                .await;
            if configuration.changed().await.is_err() {
                break;
            }
            continue;
        }
        let _ = events
            .send(AudioEvent {
                generation: playback.generation,
                status: Status::Loading,
            })
            .await;
        let prepared = tokio::select! {
            result = tokio::time::timeout(Duration::from_secs(20), prepare(playback.channel)) => result.context("Audio connection timed out").and_then(|result| result),
            changed = configuration.changed() => { if changed.is_err() { break; } continue; }
        };
        match prepared {
            Ok(source) => {
                if output
                    .send(OutputCommand::Replace(source, playback))
                    .is_err()
                {
                    break;
                }
            }
            Err(error) => {
                let _ = events
                    .send(AudioEvent {
                        generation: playback.generation,
                        status: Status::Error(format!("{error:#}")),
                    })
                    .await;
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {},
                    changed = configuration.changed() => { if changed.is_err() { break; } }
                }
                continue;
            }
        }
        loop {
            tokio::select! {
                changed = configuration.changed() => {
                    if changed.is_err() { let _ = output.send(OutputCommand::Shutdown); return; }
                    let latest = *configuration.borrow_and_update();
                    if latest.generation != playback.generation { break; }
                    let _ = output.send(OutputCommand::Volume(latest.volume));
                }
                result = notifications.recv() => {
                    let Ok(event) = result else { return; };
                    if event.generation != playback.generation { continue; }
                    let failed = matches!(event.status, Status::Error(_));
                    if events.send(event).await.is_err() { let _ = output.send(OutputCommand::Shutdown); return; }
                    if failed {
                        tokio::select! {
                            _ = tokio::time::sleep(Duration::from_secs(5)) => {},
                            _ = configuration.changed() => {}
                        }
                        break;
                    }
                }
            }
        }
    }
    let _ = output.send(OutputCommand::Shutdown);
}

/// Decode a short live sample through the production pipeline without making sound.
pub async fn probe(channel: Channel) -> Result<(u32, u16, usize)> {
    let prepared = prepare(channel).await?;
    tokio::task::spawn_blocking(move || {
        let mut prepared = prepared;
        let source = prepared.source.as_mut().context("No decoder")?;
        let sample_rate = source.sample_rate();
        let channels = source.channels();
        let count = source.take(4096).count();
        anyhow::ensure!(
            count == 4096,
            "The decoder did not produce a complete sample"
        );
        Ok((sample_rate, channels, count))
    })
    .await?
}
