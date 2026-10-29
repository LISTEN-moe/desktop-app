//! Shared DSP and color math ported from v7's AudioVisualizer/useAccentColor.
//! No GPUI, device, or OS APIs belong here.
use rustfft::{Fft, FftDirection, algorithm::Radix4, num_complex::Complex};
use std::{
    f32::consts::TAU,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub const FFT_SIZE: usize = 1024;
pub const BIN_COUNT: usize = FFT_SIZE / 2;
pub const BAND_COUNT: usize = 64;
pub const PAUSE_FADE: Duration = Duration::from_millis(250);
pub const COLOR_FADE: Duration = Duration::from_millis(500);
pub type Bands = [f32; BAND_COUNT];
pub type Color = [u8; 3];

#[derive(Clone, Debug)]
pub struct SpectrumFrame {
    pub generation: u64,
    pub bands: Bands,
    pub captured_at: Instant,
}

/// Latest-only handoff between the analysis worker and renderer. The audio
/// callback never touches this lock; each holder only copies a small frame.
#[derive(Clone, Default)]
pub struct Spectrum(Arc<Mutex<Option<SpectrumFrame>>>);
impl Spectrum {
    pub fn publish(&self, frame: SpectrumFrame) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = Some(frame);
        }
    }
    pub fn latest(&self) -> Option<SpectrumFrame> {
        self.0.try_lock().ok().and_then(|slot| slot.clone())
    }
}

pub struct Analyzer {
    fft: Radix4<f32>,
    window: [f32; FFT_SIZE],
    input: [Complex<f32>; FFT_SIZE],
    scratch: Vec<Complex<f32>>,
    smoothed: [f32; BIN_COUNT],
}
impl Default for Analyzer {
    fn default() -> Self {
        // The visualizer always uses 1024 samples. A fixed radix-4 plan avoids
        // linking the arbitrary-size planner's unused SIMD/prime algorithms.
        let fft = Radix4::new(FFT_SIZE, FftDirection::Forward);
        let scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];
        Self {
            fft,
            scratch,
            window: std::array::from_fn(|i| {
                let phase = TAU * i as f32 / FFT_SIZE as f32;
                0.42 - 0.5 * phase.cos() + 0.08 * (2. * phase).cos()
            }),
            input: [Complex::default(); FFT_SIZE],
            smoothed: [0.; BIN_COUNT],
        }
    }
}
impl Analyzer {
    /// Web Audio Blackman window, 1/N FFT normalization, magnitude smoothing,
    /// then dB-to-byte conversion (-80..-30). Time-correct smoothing matches
    /// the site's 0.8 coefficient at 60 analysis updates per second.
    pub fn analyze(&mut self, samples: &[f32; FFT_SIZE], elapsed: Duration) -> Bands {
        for ((input, sample), window) in self.input.iter_mut().zip(samples).zip(self.window) {
            *input = Complex::new(
                if sample.is_finite() {
                    sample * window
                } else {
                    0.
                },
                0.,
            );
        }
        self.fft
            .process_with_scratch(&mut self.input, &mut self.scratch);
        let smoothing = 0.8_f32.powf(elapsed.as_secs_f32() * 60.);
        let mut bins = [0_u8; BIN_COUNT];
        for (i, value) in self.smoothed.iter_mut().enumerate() {
            let magnitude = self.input[i].norm() / FFT_SIZE as f32;
            *value = smoothing * *value + (1. - smoothing) * magnitude;
            let db = 20. * value.max(f32::MIN_POSITIVE).log10();
            bins[i] = ((db + 80.) / 50. * 255.).clamp(0., 255.).floor() as u8;
        }
        map_bands(&bins)
    }
}

pub fn map_bands(data: &[u8; BIN_COUNT]) -> Bands {
    let usable = (BIN_COUNT as f32 * 0.75).floor();
    std::array::from_fn(|i| {
        let start = 1 + (usable * (i as f32 / BAND_COUNT as f32).powf(1.5)).floor() as usize;
        let end = (1 + (usable * ((i + 1) as f32 / BAND_COUNT as f32).powf(1.5)).floor() as usize)
            .max(start + 1)
            .min(BIN_COUNT);
        data[start..end]
            .iter()
            .map(|value| f32::from(*value))
            .sum::<f32>()
            / (end - start) as f32
    })
}

#[derive(Clone, Copy, Debug)]
pub struct Bar {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
}

/// Interior bars are rounded quads. Only the few bars at the outer corners
/// need physical-pixel strips clipped against the rounded player's boundary.
/// GPUI content masks are rectangular, so relying on overflow clipping alone
/// would paint into the transparent corners.
pub fn bar_geometry(bands: &Bands, width: f32, height: f32, scale: f32) -> Vec<Bar> {
    let count = BAND_COUNT * 2;
    let step = width / count as f32;
    let bar_width = (step - 1.).max(0.);
    let outer_radius = 12_f32.min(width / 2.).min(height / 2.);
    let mut bars = Vec::with_capacity(count + 64);
    if bar_width == 0. {
        return bars;
    }
    for i in 0..count {
        let band = if i < BAND_COUNT {
            BAND_COUNT - 1 - i
        } else {
            i - BAND_COUNT
        };
        let h = ((bands[band] - 24.) / (256. - 24.)).clamp(0., 1.) * height * 0.7;
        if h <= 0. {
            continue;
        }
        let x = i as f32 * step;
        let top = height - h;
        let radius = 2_f32.min(bar_width / 2.).min(h / 2.);
        if x >= outer_radius && x + bar_width <= width - outer_radius {
            bars.push(Bar {
                x,
                y: top,
                width: bar_width,
                height: h,
                radius,
            });
            continue;
        }
        let bottom = |at: f32| {
            let distance = (outer_radius - at).max(at - (width - outer_radius)).max(0.);
            height - outer_radius
                + (outer_radius * outer_radius - distance * distance)
                    .max(0.)
                    .sqrt()
        };
        let cap = |at: f32| {
            let distance = ((x + radius) - at)
                .max(at - (x + bar_width - radius))
                .max(0.);
            top + radius - (radius * radius - distance * distance).max(0.).sqrt()
        };
        let pixel = 1. / scale.max(1.);
        let mut left = x;
        while left < x + bar_width {
            let right = (left + pixel).min(x + bar_width);
            let y = cap(left).max(cap(right));
            let bottom = bottom(left).min(bottom(right));
            if bottom > y {
                bars.push(Bar {
                    x: left,
                    y,
                    width: right - left,
                    height: bottom - y,
                    radius: 0.,
                });
            }
            left = right;
        }
    }
    bars
}

pub fn color_from_rgb(value: u32) -> Color {
    [(value >> 16) as u8, (value >> 8) as u8, value as u8]
}

/// Same 50x50 sampling, hue buckets, saturation weights and lightness filter
/// as useAccentColor.ts. Decode/resize once in the artwork worker, never paint.
pub fn artwork_color(bytes: &[u8]) -> Option<Color> {
    let image = image::load_from_memory(bytes).ok()?;
    Some(color_from_image(&image))
}

pub(crate) fn color_from_image(image: &image::DynamicImage) -> Color {
    let pixels = image
        .resize_exact(50, 50, image::imageops::FilterType::Triangle)
        .to_rgba8();
    dominant_color(pixels.as_raw())
}

pub fn dominant_color(rgba: &[u8]) -> Color {
    let mut buckets = [[0_f32; 4]; 12];
    for pixel in rgba.as_chunks::<4>().0.iter().step_by(4) {
        let [r, g, b] = [
            f32::from(pixel[0]),
            f32::from(pixel[1]),
            f32::from(pixel[2]),
        ];
        let [rn, gn, bn] = [r / 255., g / 255., b / 255.];
        let max = rn.max(gn).max(bn);
        let min = rn.min(gn).min(bn);
        let lightness = (max + min) / 2.;
        if !(0.15..=0.85).contains(&lightness) {
            continue;
        }
        let delta = max - min;
        if delta == 0. {
            continue;
        }
        let saturation = if lightness > 0.5 {
            delta / (2. - max - min)
        } else {
            delta / (max + min)
        };
        let hue = if max == rn {
            ((gn - bn) / delta).rem_euclid(6.)
        } else if max == gn {
            (bn - rn) / delta + 2.
        } else {
            (rn - gn) / delta + 4.
        } * 60.;
        let bucket = &mut buckets[(hue / 30.).floor() as usize % 12];
        bucket[0] += saturation;
        bucket[1] += r * saturation;
        bucket[2] += g * saturation;
        bucket[3] += b * saturation;
    }
    let best = buckets
        .iter()
        .reduce(|best, bucket| if bucket[0] > best[0] { bucket } else { best })
        .unwrap();
    if best[0] == 0. {
        return [128; 3];
    }
    [
        (best[1] / best[0]).round() as u8,
        (best[2] / best[0]).round() as u8,
        (best[3] / best[0]).round() as u8,
    ]
}

/// Frame-rate-independent equivalents of the web's RGB transition and 15-frame
/// pause fade at 60Hz. Interruption always starts from the current visible value.
pub struct Motion {
    bands: Bands,
    from: [f32; 3],
    target: Color,
    color_started: Duration,
    fade_started: Option<Duration>,
    visible: bool,
}
pub struct Presentation {
    pub bands: Bands,
    pub color: Color,
    pub alpha: f32,
    pub animating: bool,
}
impl Motion {
    pub fn new(color: Color) -> Self {
        Self {
            bands: [0.; BAND_COUNT],
            from: color.map(f32::from),
            target: color,
            color_started: Duration::ZERO,
            fade_started: None,
            visible: false,
        }
    }
    fn color_at(&self, now: Duration) -> [f32; 3] {
        let t = (now.saturating_sub(self.color_started).as_secs_f32() / COLOR_FADE.as_secs_f32())
            .min(1.);
        std::array::from_fn(|i| self.from[i] + (f32::from(self.target[i]) - self.from[i]) * t)
    }
    pub fn sample(
        &mut self,
        now: Duration,
        spectrum: Option<&Bands>,
        color: Color,
        playing: bool,
    ) -> Presentation {
        if color != self.target {
            self.from = self.color_at(now);
            self.target = color;
            self.color_started = now;
        }
        let alpha = if let Some(bands) = spectrum {
            self.bands = *bands;
            self.visible = true;
            self.fade_started = None;
            0.35
        } else if self.visible {
            let elapsed = now.saturating_sub(*self.fade_started.get_or_insert(now));
            if elapsed >= PAUSE_FADE {
                self.visible = false;
                self.fade_started = None;
                0.
            } else {
                0.35 * 0.85_f32.powf(elapsed.as_secs_f32() * 60.)
            }
        } else {
            0.
        };
        let color = self.color_at(now).map(|value| value.round() as u8);
        Presentation {
            bands: self.bands,
            color,
            alpha,
            animating: playing || self.visible,
        }
    }
}
