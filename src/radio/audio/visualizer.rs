use super::AudioSource;
use crate::radio::visualizer::{Analyzer, FFT_SIZE, Spectrum, SpectrumFrame};
use rodio::Source;
use rtrb::{Consumer, Producer, RingBuffer};
use std::{
    thread,
    time::{Duration, Instant},
};

const CHUNK_SIZE: usize = 128;
type Chunk = [f32; CHUNK_SIZE];

struct Tap {
    source: AudioSource,
    producer: Producer<Chunk>,
    chunk: Chunk,
    filled: usize,
    channel: u16,
    channel_sum: f32,
}

/// Called by the output thread, not the device callback. The callback only
/// passes samples through, downmixes, and tries a preallocated lock-free push.
pub(super) fn tap(source: AudioSource, generation: u64, spectrum: Spectrum) -> AudioSource {
    let sample_rate = source.sample_rate();
    let (producer, consumer) = RingBuffer::new(64);
    if let Err(error) = thread::Builder::new()
        .name("radio-spectrum".into())
        .spawn(move || analyze(consumer, sample_rate, generation, spectrum))
    {
        eprintln!("Cannot start visualizer: {error}");
        return source;
    }
    Box::new(Tap {
        source,
        producer,
        chunk: [0.; CHUNK_SIZE],
        filled: 0,
        channel: 0,
        channel_sum: 0.,
    })
}

impl Iterator for Tap {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let sample = self.source.next()?;
        self.channel_sum += sample;
        self.channel += 1;
        if self.channel == self.source.channels() {
            self.chunk[self.filled] = self.channel_sum / f32::from(self.channel);
            self.filled += 1;
            self.channel_sum = 0.;
            self.channel = 0;
            if self.filled == CHUNK_SIZE {
                // Dropping analysis data under load must never stall sound.
                let _ = self.producer.push(self.chunk);
                self.filled = 0;
            }
        }
        Some(sample)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.source.size_hint()
    }
}
impl Source for Tap {
    fn current_span_len(&self) -> Option<usize> {
        self.source.current_span_len()
    }
    fn channels(&self) -> u16 {
        self.source.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
}

fn analyze(mut consumer: Consumer<Chunk>, sample_rate: u32, generation: u64, spectrum: Spectrum) {
    let mut analyzer = Analyzer::default();
    let mut history = [0.; FFT_SIZE];
    let mut position = 0;
    let mut received = 0;
    let mut pending = 0;
    loop {
        while let Ok(chunk) = consumer.pop() {
            for sample in chunk {
                history[position] = sample;
                position = (position + 1) % FFT_SIZE;
            }
            received += CHUNK_SIZE;
            pending += CHUNK_SIZE;
        }
        if received >= FFT_SIZE && pending > 0 {
            let samples = std::array::from_fn(|i| history[(position + i) % FFT_SIZE]);
            let elapsed = Duration::from_secs_f64(pending as f64 / f64::from(sample_rate));
            let bands = analyzer.analyze(&samples, elapsed);
            spectrum.publish(SpectrumFrame {
                generation,
                bands,
                captured_at: Instant::now(),
            });
            pending = 0;
        }
        if consumer.is_abandoned() {
            break;
        }
        thread::sleep(Duration::from_secs_f64(1. / 60.));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tap_preserves_stereo_samples_and_drops_analysis_without_backpressure() {
        let samples: Vec<f32> = (0..20000).map(|i| (i as f32 * 0.1).sin()).collect();
        let source = rodio::buffer::SamplesBuffer::new(2, 48000, samples.clone());
        let (producer, _consumer) = RingBuffer::new(1); // Deliberately never consumed.
        let mut tap = Tap {
            source: Box::new(source),
            producer,
            chunk: [0.; CHUNK_SIZE],
            filled: 0,
            channel: 0,
            channel_sum: 0.,
        };
        assert_eq!(tap.channels(), 2);
        assert_eq!(tap.sample_rate(), 48000);
        assert_eq!(tap.by_ref().collect::<Vec<_>>(), samples);
    }
    #[test]
    fn stereo_downmix_matches_web_audio_and_can_cancel_opposite_channels() {
        let source = rodio::buffer::SamplesBuffer::new(2, 48000, [1., -1.].repeat(CHUNK_SIZE));
        let (producer, mut consumer) = RingBuffer::new(1);
        let tap = Tap {
            source: Box::new(source),
            producer,
            chunk: [0.; CHUNK_SIZE],
            filled: 0,
            channel: 0,
            channel_sum: 0.,
        };
        assert_eq!(tap.count(), CHUNK_SIZE * 2);
        assert_eq!(consumer.pop().unwrap(), [0.; CHUNK_SIZE]);
        assert!(consumer.is_abandoned());
    }

    #[test]
    fn worker_analyzes_before_volume_and_tags_the_playback_generation() {
        let samples: Vec<f32> = (0..48000)
            .map(|i| (std::f32::consts::TAU * 440. * i as f32 / 48000.).sin() * 0.2)
            .collect();
        let source = rodio::buffer::SamplesBuffer::new(1, 48000, samples);
        let spectrum = Spectrum::default();
        let mut muted = tap(Box::new(source), 42, spectrum.clone()).amplify(0.);
        // Consume at approximately device cadence rather than filling the
        // bounded queue in a tight loop before the worker can wake.
        for _ in 0..10 {
            assert!(muted.by_ref().take(800).all(|sample| sample == 0.));
            thread::sleep(Duration::from_millis(17));
        }
        let frame = spectrum.latest().expect("worker publishes spectrum");
        assert_eq!(frame.generation, 42);
        assert!(frame.bands.iter().any(|value| *value > 24.));
        assert!(frame.captured_at.elapsed() < Duration::from_secs(1));
    }
}
