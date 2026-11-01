use listen_moe_desktop::radio::visualizer::*;
use std::{f32::consts::TAU, time::Duration};

fn tone(bin: usize, amplitude: f32) -> [f32; FFT_SIZE] {
    std::array::from_fn(|i| (TAU * bin as f32 * i as f32 / FFT_SIZE as f32).sin() * amplitude)
}

#[test]
fn mapping_and_color_match_the_actual_v7_functions() {
    #[derive(serde::Deserialize)]
    struct Reference {
        bins: Vec<u8>,
        bands: Vec<f32>,
        pixels: Vec<u8>,
        color: String,
    }
    let reference: Reference =
        serde_json::from_str(include_str!("fixtures/visualizer-reference.json")).unwrap();
    let bins: [u8; BIN_COUNT] = reference.bins.try_into().unwrap();
    for (actual, expected) in map_bands(&bins).iter().zip(&reference.bands) {
        assert!((actual - expected).abs() < 0.001);
    }
    let [r, g, b] = dominant_color(&reference.pixels);
    assert_eq!(format!("#{r:02x}{g:02x}{b:02x}"), reference.color);
}

#[test]
fn fft_matches_an_independent_scalar_dft_and_web_audio_normalization() {
    let samples = tone(32, 0.05);
    let mut reference = [0; BIN_COUNT];
    for (k, bin) in reference.iter_mut().enumerate() {
        let (mut real, mut imaginary) = (0_f64, 0_f64);
        for (n, sample) in samples.iter().enumerate() {
            let angle = std::f64::consts::TAU * n as f64 / FFT_SIZE as f64;
            let window = 0.42 - 0.5 * angle.cos() + 0.08 * (2. * angle).cos();
            real += f64::from(*sample) * window * (angle * k as f64).cos();
            imaginary -= f64::from(*sample) * window * (angle * k as f64).sin();
        }
        let magnitude = real.hypot(imaginary) / FFT_SIZE as f64;
        let db = 20. * magnitude.max(f64::MIN_POSITIVE).log10();
        *bin = ((db + 80.) / 50. * 255.).clamp(0., 255.).floor() as u8;
    }
    let actual = Analyzer::default().analyze(&samples, Duration::from_secs(100));
    for (actual, expected) in actual.iter().zip(map_bands(&reference)) {
        assert!((actual - expected).abs() < 1., "{actual} vs {expected}");
    }
}

#[test]
fn silence_is_empty_and_frequency_moves_energy_from_center_to_edges() {
    let mut analyzer = Analyzer::default();
    assert_eq!(
        analyzer.analyze(&[0.; FFT_SIZE], Duration::from_secs(1)),
        [0.; BAND_COUNT]
    );
    let low = Analyzer::default().analyze(&tone(4, 0.2), Duration::from_secs(1));
    let high = Analyzer::default().analyze(&tone(240, 0.2), Duration::from_secs(1));
    let peak = |bands: Bands| {
        bands
            .into_iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
            .0
    };
    assert!(peak(low) < 8);
    assert!(peak(high) > 40);
    assert!(bar_geometry(&[0.; BAND_COUNT], 476., 72., 2.).is_empty());
}

#[test]
fn smoothing_retains_energy_and_decays_after_silence() {
    let mut analyzer = Analyzer::default();
    let interval = Duration::from_secs_f64(1. / 60.);
    let first = analyzer.analyze(&tone(12, 0.05), interval);
    let second = analyzer.analyze(&tone(12, 0.05), interval);
    assert!(second.iter().sum::<f32>() > first.iter().sum::<f32>());
    let tail = analyzer.analyze(&[0.; FFT_SIZE], interval);
    assert!(tail.iter().any(|value| *value > 24.));
    assert!(tail.iter().sum::<f32>() < second.iter().sum::<f32>());
    for _ in 0..120 {
        analyzer.analyze(&[0.; FFT_SIZE], interval);
    }
    assert_eq!(
        analyzer.analyze(&[0.; FFT_SIZE], interval),
        [0.; BAND_COUNT]
    );
}

#[test]
fn geometry_is_mirrored_and_stays_inside_rounded_player_at_all_sizes() {
    let bands = std::array::from_fn(|i| 255. - i as f32 * 2.);
    for width in [276., 476.] {
        for scale in [1., 1.5, 2.] {
            let bars = bar_geometry(&bands, width, 72., scale);
            for bar in &bars {
                assert!(bar.x >= 0. && bar.x + bar.width <= width + 0.001);
                assert!(bar.y >= 72. * 0.3 - 0.001 && bar.y + bar.height <= 72.001);
                let mirror_x = width - bar.x - bar.width - 1.;
                // Edge bars are split into pixel strips for corner clipping;
                // only compare complete bars at least one step inward.
                if bar.x >= 12. + width / 128. && bar.x + bar.width <= width - 12. - width / 128. {
                    let mirror = bars
                        .iter()
                        .find(|other| (other.x - mirror_x).abs() < 0.001)
                        .unwrap();
                    assert!((mirror.height - bar.height).abs() < 0.001);
                }
                if bar.x < 12. || bar.x + bar.width > width - 12. {
                    for x in [bar.x, bar.x + bar.width] {
                        let distance = (12. - x).max(x - (width - 12.)).max(0.);
                        let bottom = 60. + (144. - distance * distance).max(0.).sqrt();
                        assert!(bar.y + bar.height <= bottom + 0.001);
                    }
                }
            }
        }
    }
}

#[test]
fn color_extraction_decodes_cover_and_handles_neutral_or_missing_pixels() {
    assert!(artwork_color(include_bytes!("fixtures/visualizer-cover.jpg")).is_some());
    assert!(artwork_color(b"bad cover").is_none());
    assert_eq!(dominant_color(&[0, 0, 0, 255].repeat(100)), [128; 3]);
    assert_eq!(dominant_color(&[255, 255, 255, 255].repeat(100)), [128; 3]);
    assert_eq!(
        dominant_color(&[210, 50, 70, 255].repeat(100)),
        [210, 50, 70]
    );
}

#[test]
fn color_transition_and_pause_fade_are_continuous_and_stop_when_idle() {
    let mut motion = Motion::new([255, 0, 0]);
    assert!(
        !motion
            .sample(Duration::ZERO, None, [255, 0, 0], false)
            .animating
    );
    motion.sample(Duration::ZERO, Some(&[200.; BAND_COUNT]), [255, 0, 0], true);
    motion.sample(Duration::ZERO, Some(&[200.; BAND_COUNT]), [0, 0, 255], true);
    let midpoint = motion.sample(
        Duration::from_millis(250),
        Some(&[200.; BAND_COUNT]),
        [0, 0, 255],
        true,
    );
    assert_eq!(midpoint.color, [128, 0, 128]);
    let reversal = motion.sample(Duration::from_millis(250), None, [0, 255, 0], false);
    assert_eq!(reversal.color, midpoint.color);
    let fading = motion.sample(Duration::from_millis(350), None, [0, 255, 0], false);
    assert!(fading.alpha > 0. && fading.alpha < 0.35);
    let resumed = motion.sample(
        Duration::from_millis(350),
        Some(&[180.; BAND_COUNT]),
        [0, 255, 0],
        true,
    );
    assert_eq!(resumed.alpha, 0.35);
    motion.sample(Duration::from_millis(400), None, [0, 255, 0], false);
    let stopped = motion.sample(Duration::from_millis(650), None, [0, 255, 0], false);
    assert_eq!(stopped.alpha, 0.);
    assert!(!stopped.animating);
}
