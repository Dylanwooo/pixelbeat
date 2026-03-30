use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

const NUM_BARS: usize = 32;
const SMOOTHING: f32 = 0.15; // Lower = more reactive/jumpy

/// Spectrum analyzer that generates frequency bar data
/// Currently uses simulated spectrum with beat-like patterns
/// Future: will tap into actual PCM audio data
pub struct SpectrumAnalyzer {
    bars: Vec<f32>,
    targets: Vec<f32>,
    tick: u64,
    beat_phase: f64,
    beat_energy: f32,
    _planner: FftPlanner<f32>,
}

impl SpectrumAnalyzer {
    pub fn new() -> Self {
        Self {
            bars: vec![0.0; NUM_BARS],
            targets: vec![0.0; NUM_BARS],
            tick: 0,
            beat_phase: 0.0,
            beat_energy: 0.0,
            _planner: FftPlanner::new(),
        }
    }

    /// Generate spectrum data.
    /// When playing: beat-synced animated bars with dramatic jumps
    /// When paused: bars decay to zero
    pub fn generate(&mut self, playing: bool) -> Vec<f32> {
        self.tick += 1;

        if playing {
            // Faster beat — ~160 BPM feel (energetic)
            self.beat_phase += 0.21;
            let beat_hit = (self.beat_phase * std::f64::consts::PI).sin().powi(4) as f32;

            // Beat energy: instant attack, fast decay for punchy feel
            if beat_hit > 0.4 {
                self.beat_energy = 1.0;
            } else {
                self.beat_energy *= 0.75; // Faster decay = more punchy beats
            }

            // Update targets every tick with much wider variance
            for i in 0..NUM_BARS {
                let freq_pos = i as f32 / NUM_BARS as f32;
                let bass_boost = (1.0 - freq_pos).powi(2) * 0.7;

                // Multiple fast-moving waves with large phase differences between bars
                let wave1 = Self::wave(self.tick, i, 0.31, 4.7, 43758.5453);
                let wave2 = Self::wave(self.tick, i, 0.47, 3.1, 28461.2731);
                let wave3 = Self::wave(self.tick, i, 0.13, 7.3, 17853.9142);
                let wave4 = Self::wave(self.tick, i, 0.59, 5.9, 63721.8314);

                // Heavily contrast-boosted variation: push toward extremes
                let raw = wave1 * 0.3 + wave2 * 0.25 + wave3 * 0.25 + wave4 * 0.2;
                // Apply contrast curve: push mid-values toward 0 or 1
                let contrasted = if raw > 0.5 {
                    0.5 + (raw - 0.5).powf(0.5) * 0.5 / 0.5_f32.powf(0.5)
                } else {
                    0.5 - (0.5 - raw).powf(0.5) * 0.5 / 0.5_f32.powf(0.5)
                };

                let beat_contribution = self.beat_energy * bass_boost * 0.8;

                // Random spikes: deterministic but spiky — some bars occasionally shoot to max
                let spike_hash = Self::hash_spike(self.tick, i);
                let spike = if spike_hash > 0.88 { 1.0_f32 } else { 0.0 };

                let target = (contrasted * 0.7 + beat_contribution + spike * 0.5).clamp(0.0, 1.0);
                // Inject zeros: some bars forced low for contrast between neighbors
                let kill_hash = Self::hash_spike(self.tick.wrapping_add(9999), i);
                self.targets[i] = if kill_hash < 0.25 {
                    target * 0.08 // near-zero bar
                } else {
                    target
                };
            }

            // Very fast interpolation — nearly instant reaction
            for i in 0..NUM_BARS {
                let target = self.targets[i];
                if target > self.bars[i] {
                    // Near-instant attack for dramatic jumps
                    self.bars[i] = self.bars[i] * 0.1 + target * 0.9;
                } else {
                    // Fast decay
                    self.bars[i] = self.bars[i] * SMOOTHING + target * (1.0 - SMOOTHING);
                }
            }
        } else {
            // Decay when paused — bars fall with gravity
            for bar in &mut self.bars {
                *bar *= 0.75;
                if *bar < 0.01 {
                    *bar = 0.0;
                }
            }
            self.beat_energy = 0.0;
        }

        self.bars.clone()
    }

    /// Smooth pseudo-random wave with large inter-bar phase jumps
    fn wave(tick: u64, idx: usize, freq: f64, phase_offset: f64, seed: f64) -> f32 {
        let t = tick as f64 * freq + idx as f64 * phase_offset;
        let x = (t.sin() * seed).fract().abs();
        // Square it for more extreme distribution (push toward 0 and 1)
        (x * x * 1.5).min(1.0) as f32
    }

    /// Deterministic pseudo-random hash for spike injection (0.0..1.0)
    fn hash_spike(tick: u64, idx: usize) -> f32 {
        let v = ((tick as f64 * 127.1 + idx as f64 * 311.7).sin() * 43758.5453)
            .fract()
            .abs();
        v as f32
    }

    /// Analyze real PCM data (for future use with audio tap)
    #[allow(dead_code)]
    pub fn analyze_pcm(&mut self, samples: &[f32]) -> Vec<f32> {
        let fft_size = samples.len().next_power_of_two();
        let fft = self._planner.plan_fft_forward(fft_size);

        let mut buffer: Vec<Complex<f32>> = samples.iter().map(|&s| Complex::new(s, 0.0)).collect();
        buffer.resize(fft_size, Complex::new(0.0, 0.0));

        // Apply Hanning window
        for (i, sample) in buffer.iter_mut().enumerate() {
            let window =
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / fft_size as f32).cos());
            sample.re *= window;
        }

        fft.process(&mut buffer);

        // Convert to magnitude and bin into bars
        let useful_bins = fft_size / 2;
        let bins_per_bar = useful_bins / NUM_BARS;
        let mut result = vec![0.0f32; NUM_BARS];

        for (i, bar) in result.iter_mut().enumerate() {
            let start = i * bins_per_bar;
            let end = start + bins_per_bar;
            let sum: f32 = buffer[start..end].iter().map(|c| c.norm()).sum();
            *bar = (sum / bins_per_bar as f32).log10().max(0.0) / 2.0;
        }

        // Smooth
        for i in 0..NUM_BARS {
            self.bars[i] = self.bars[i] * SMOOTHING + result[i] * (1.0 - SMOOTHING);
        }

        self.bars.clone()
    }
}
