//! Profile the production FFT without a device, network, or renderer.
use listen_moe_desktop::radio::visualizer::{Analyzer, FFT_SIZE};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

fn main() {
    let samples = std::array::from_fn(|i| {
        (std::f32::consts::TAU * 17. * i as f32 / FFT_SIZE as f32).sin() * 0.1
    });
    let interval = Duration::from_secs_f64(1. / 60.);
    let mut analyzer = Analyzer::default();
    for _ in 0..100 {
        black_box(analyzer.analyze(black_box(&samples), interval));
    }
    let started = Instant::now();
    for _ in 0..10000 {
        black_box(analyzer.analyze(black_box(&samples), interval));
    }
    println!(
        "10000 production FFT frames: {:?} ({:.2} µs/frame)",
        started.elapsed(),
        started.elapsed().as_secs_f64() * 100.
    );
}
