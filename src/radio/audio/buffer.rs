//! Decoding and stream reads stay on a producer thread. The device-facing
//! source only copies queued PCM and emits silence if the producer is late.
use super::AudioSource;
use anyhow::{Context, Result};
use rodio::Source;
use rtrb::{Consumer, RingBuffer};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

const CHUNK_SAMPLES: usize = 2048;
#[derive(Default)]
struct Shared {
    stopped: AtomicBool,
    underflows: AtomicUsize,
}
#[derive(Clone)]
pub(super) struct Stats(Arc<Shared>);
impl Stats {
    pub fn take_underflows(&self) -> usize {
        self.0.underflows.swap(0, Ordering::Relaxed)
    }
}
struct Pcm {
    consumer: Consumer<f32>,
    samples: [f32; CHUNK_SAMPLES],
    index: usize,
    len: usize,
    chunk_len: usize,
    channels: u16,
    sample_rate: u32,
    duration: Option<Duration>,
    shared: Arc<Shared>,
}

/// Prebuffer half a second, with a one-second hard capacity (384KB at
/// 48kHz stereo). This function may wait and must run on a blocking worker.
pub(super) fn prepare(source: AudioSource) -> Result<(AudioSource, Stats)> {
    let channels = source.channels();
    let sample_rate = source.sample_rate();
    anyhow::ensure!(
        channels > 0 && usize::from(channels) <= CHUNK_SAMPLES && sample_rate > 0,
        "Invalid decoded audio format"
    );
    let capacity = usize::try_from(sample_rate)?
        .checked_mul(usize::from(channels))
        .context("Decoded audio buffer size overflow")?;
    let duration = source.total_duration();
    let chunk_len = CHUNK_SAMPLES - CHUNK_SAMPLES % usize::from(channels);
    let target = (capacity / 2).max(1);
    let (mut producer, consumer) = RingBuffer::new(capacity);
    let shared = Arc::new(Shared::default());
    let worker = shared.clone();
    let (ready, buffered) = mpsc::sync_channel(1);
    thread::Builder::new()
        .name("radio-decode".into())
        .spawn(move || {
            // Keep the queue's format stable even if an MP3 stream changes format.
            let mut source =
                rodio::source::UniformSourceIterator::new(source, channels, sample_rate);
            let mut samples = [0.; CHUNK_SAMPLES];
            let mut ready = Some(ready);
            loop {
                if worker.stopped.load(Ordering::Relaxed) || producer.is_abandoned() {
                    break;
                }
                let available = producer.slots();
                if available == 0 {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }
                let count = available.min(chunk_len);
                let mut len = 0;
                for sample in samples.iter_mut().take(count) {
                    let Some(value) = source.next() else {
                        break;
                    };
                    *sample = value;
                    len += 1;
                }
                if len > 0 {
                    // Only this producer writes; the consumer can only free slots.
                    if producer.push_entire_slice(&samples[..len]).is_err() {
                        break;
                    }
                }
                if len < count {
                    // Close before signalling readiness for a short finite input.
                    drop(producer);
                    if let Some(ready) = ready.take() {
                        let _ = ready.send(());
                    }
                    break;
                }
                if capacity - producer.slots() >= target
                    && let Some(ready) = ready.take()
                {
                    let _ = ready.send(());
                }
            }
        })
        .context("Cannot start audio decoder worker")?;
    let pcm = Pcm {
        consumer,
        samples: [0.; CHUNK_SAMPLES],
        index: 0,
        len: 0,
        chunk_len,
        channels,
        sample_rate,
        duration,
        shared: shared.clone(),
    };
    // Drop Pcm on timeout/disconnection to cancel an unconsumed producer.
    buffered
        .recv_timeout(Duration::from_secs(10))
        .context("Audio prebuffer timed out")?;
    Ok((Box::new(pcm), Stats(shared)))
}
impl Iterator for Pcm {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.index == self.len {
            self.len = self
                .consumer
                .pop_partial_slice(&mut self.samples[..self.chunk_len])
                .0
                .len();
            self.index = 0;
            if self.len == 0 {
                if self.consumer.is_abandoned() {
                    // The producer may have published its final samples after
                    // the first empty read but before closing the queue.
                    self.len = self
                        .consumer
                        .pop_partial_slice(&mut self.samples[..self.chunk_len])
                        .0
                        .len();
                    if self.len == 0 {
                        return None;
                    }
                } else {
                    self.shared.underflows.fetch_add(1, Ordering::Relaxed);
                    self.samples.fill(0.);
                    self.len = self.chunk_len;
                }
            }
        }
        let sample = self.samples[self.index];
        self.index += 1;
        Some(sample)
    }
}
impl Source for Pcm {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        self.channels
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        self.duration
    }
}
impl Drop for Pcm {
    fn drop(&mut self) {
        self.shared.stopped.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::Source;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };

    struct StallingSource {
        stall: Arc<AtomicBool>,
    }
    impl Iterator for StallingSource {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            if self.stall.swap(false, Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
            }
            Some(0.25)
        }
    }
    impl Source for StallingSource {
        fn current_span_len(&self) -> Option<usize> {
            None
        }
        fn channels(&self) -> u16 {
            2
        }
        fn sample_rate(&self) -> u32 {
            48000
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    #[test]
    fn decoder_stall_does_not_block_audio_callback() {
        let stall = Arc::new(AtomicBool::new(false));
        let (mut source, _stats) = prepare(Box::new(StallingSource {
            stall: stall.clone(),
        }))
        .unwrap();
        stall.store(true, Ordering::Relaxed);
        let started = Instant::now();
        let output: Vec<_> = source.by_ref().take(1024).collect();
        assert_eq!(output.len(), 1024);
        assert!(
            started.elapsed() < Duration::from_millis(20),
            "a decoder stall blocked a device-sized callback for {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn finite_stereo_input_preserves_samples_and_end_of_stream() {
        let samples: Vec<f32> = (0..8192).map(|i| (i as f32 * 0.1).sin()).collect();
        let source = rodio::buffer::SamplesBuffer::new(2, 48000, samples.clone());
        let (source, stats) = prepare(Box::new(source)).unwrap();
        assert_eq!(source.channels(), 2);
        assert_eq!(source.sample_rate(), 48000);
        assert_eq!(source.collect::<Vec<_>>(), samples);
        assert_eq!(stats.take_underflows(), 0);
    }

    #[test]
    fn empty_queue_returns_silence_instead_of_waiting() {
        let (_producer, consumer) = RingBuffer::new(4096);
        let shared = Arc::new(Shared::default());
        let mut source = Pcm {
            consumer,
            samples: [0.; CHUNK_SAMPLES],
            index: 0,
            len: 0,
            chunk_len: CHUNK_SAMPLES,
            channels: 2,
            sample_rate: 48000,
            duration: None,
            shared: shared.clone(),
        };
        let started = Instant::now();
        assert!(source.by_ref().take(2048).all(|sample| sample == 0.));
        assert!(started.elapsed() < Duration::from_millis(20));
        assert_eq!(shared.underflows.load(Ordering::Relaxed), 1);
        // Refilling recovers at a whole-channel boundary without losing data.
        let mut producer = _producer;
        producer.push_entire_slice(&[0.25; 2048]).unwrap();
        assert!(source.by_ref().take(2048).all(|sample| sample == 0.25));
        drop(source);
        assert!(shared.stopped.load(Ordering::Relaxed));
    }

    #[test]
    fn decoder_backpressure_bounds_memory_and_dropping_playback_stops_worker() {
        struct CountingSource {
            decoded: Arc<AtomicUsize>,
            dropped: Arc<AtomicBool>,
        }
        impl Iterator for CountingSource {
            type Item = f32;
            fn next(&mut self) -> Option<f32> {
                self.decoded.fetch_add(1, Ordering::Relaxed);
                Some(0.25)
            }
        }
        impl Source for CountingSource {
            fn current_span_len(&self) -> Option<usize> {
                None
            }
            fn channels(&self) -> u16 {
                2
            }
            fn sample_rate(&self) -> u32 {
                48000
            }
            fn total_duration(&self) -> Option<Duration> {
                None
            }
        }
        impl Drop for CountingSource {
            fn drop(&mut self) {
                self.dropped.store(true, Ordering::Relaxed);
            }
        }
        let decoded = Arc::new(AtomicUsize::new(0));
        let dropped = Arc::new(AtomicBool::new(false));
        let (source, _) = prepare(Box::new(CountingSource {
            decoded: decoded.clone(),
            dropped: dropped.clone(),
        }))
        .unwrap();
        std::thread::sleep(Duration::from_millis(30));
        assert!(
            decoded.load(Ordering::Relaxed) <= 48000 * 2,
            "an unconsumed stream cannot decode more than the one-second queue"
        );
        drop(source);
        let started = Instant::now();
        while !dropped.load(Ordering::Relaxed) && started.elapsed() < Duration::from_secs(1) {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            dropped.load(Ordering::Relaxed),
            "a full queue does not leak its decoder worker"
        );
    }
}
