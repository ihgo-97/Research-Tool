//! Synthesises the placeholder sound effects in `assets/sfx/` (retro
//! handheld-style blips, thumps and whooshes). Original, generated from code.
//! Run: `cargo run --release --bin gen_sfx`.

use std::path::{Path, PathBuf};

const RATE: u32 = 22050;

#[derive(Clone, Copy)]
enum Wave {
    Square(f32),
    Saw,
    Sine,
    Noise,
}

/// One synthesised layer: frequency slides from `f0` to `f1` over `dur`
/// seconds with an attack/decay envelope, optional vibrato and low-pass.
#[derive(Clone, Copy)]
struct Voice {
    wave: Wave,
    f0: f32,
    f1: f32,
    dur: f32,
    start: f32,
    attack: f32,
    gain: f32,
    vibrato: (f32, f32),
    /// One-pole low-pass coefficient (1 = off).
    lowpass: f32,
}

impl Voice {
    fn new(wave: Wave, f0: f32, f1: f32, dur: f32) -> Self {
        Self {
            wave,
            f0,
            f1,
            dur,
            start: 0.0,
            attack: 0.005,
            gain: 0.5,
            vibrato: (0.0, 0.0),
            lowpass: 1.0,
        }
    }
    fn at(mut self, t: f32) -> Self {
        self.start = t;
        self
    }
    fn gain(mut self, g: f32) -> Self {
        self.gain = g;
        self
    }
    fn vib(mut self, rate: f32, depth: f32) -> Self {
        self.vibrato = (rate, depth);
        self
    }
    fn lp(mut self, k: f32) -> Self {
        self.lowpass = k;
        self
    }
    fn attack(mut self, a: f32) -> Self {
        self.attack = a;
        self
    }
}

struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn render(voices: &[Voice]) -> Vec<f32> {
    let total = voices.iter().map(|v| v.start + v.dur).fold(0.0, f32::max);
    let n = (total * RATE as f32) as usize + 1;
    let mut out = vec![0.0f32; n];
    let mut rng = Rng(0x1234_5678);
    for v in voices {
        let mut phase = 0.0f32;
        let mut lp = 0.0f32;
        let mut noise = 0.0f32;
        let s0 = (v.start * RATE as f32) as usize;
        let len = (v.dur * RATE as f32) as usize;
        for i in 0..len {
            let t = i as f32 / RATE as f32;
            let u = t / v.dur;
            // Exponential frequency slide.
            let mut f = v.f0 * (v.f1 / v.f0).powf(u);
            if v.vibrato.1 > 0.0 {
                f *= 1.0 + v.vibrato.1 * (t * v.vibrato.0 * std::f32::consts::TAU).sin();
            }
            let prev = phase;
            phase = (phase + f / RATE as f32) % 1.0;
            let x = match v.wave {
                Wave::Square(duty) => {
                    if phase < duty {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Wave::Saw => phase * 2.0 - 1.0,
                Wave::Sine => (phase * std::f32::consts::TAU).sin(),
                Wave::Noise => {
                    // New noise sample each cycle: pitch controls the "grain".
                    if phase < prev {
                        noise = rng.next();
                    }
                    noise
                }
            };
            lp += (x - lp) * v.lowpass;
            let env = if t < v.attack { t / v.attack } else { (1.0 - u).max(0.0).powf(1.6) };
            if let Some(o) = out.get_mut(s0 + i) {
                *o += lp * env * v.gain;
            }
        }
    }
    out
}

fn write_wav(path: &Path, samples: &[f32]) {
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(1e-6);
    let norm = 0.85 / peak;
    let data: Vec<u8> = samples
        .iter()
        .flat_map(|s| (((s * norm).clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())
        .collect();
    let mut wav = Vec::with_capacity(44 + data.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&RATE.to_le_bytes());
    wav.extend_from_slice(&(RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
    wav.extend_from_slice(&data);
    std::fs::create_dir_all(path.parent().unwrap()).expect("create sfx dir");
    std::fs::write(path, wav).expect("write wav");
    println!("wrote {} ({:.2}s)", path.display(), samples.len() as f32 / RATE as f32);
}

fn sounds() -> Vec<(&'static str, Vec<Voice>)> {
    use Wave::*;
    vec![
        // Battle events.
        ("hit", vec![Voice::new(Noise, 4000.0, 800.0, 0.12).gain(0.6), Voice::new(Square(0.5), 220.0, 90.0, 0.1).gain(0.4)]),
        (
            "hit_strong",
            vec![
                Voice::new(Noise, 5000.0, 400.0, 0.22).gain(0.7),
                Voice::new(Square(0.5), 300.0, 60.0, 0.2).gain(0.5),
                Voice::new(Square(0.25), 880.0, 1320.0, 0.08).gain(0.25),
            ],
        ),
        ("dodge", vec![Voice::new(Sine, 500.0, 1600.0, 0.16).gain(0.5), Voice::new(Noise, 8000.0, 3000.0, 0.12).gain(0.15).lp(0.3)]),
        ("dash", vec![Voice::new(Noise, 6000.0, 1500.0, 0.18).gain(0.5).lp(0.25).attack(0.03)]),
        (
            "ko",
            vec![
                Voice::new(Square(0.5), 660.0, 600.0, 0.12).gain(0.4),
                Voice::new(Square(0.5), 520.0, 480.0, 0.12).gain(0.4).at(0.12),
                Voice::new(Square(0.5), 390.0, 120.0, 0.35).gain(0.45).at(0.24),
            ],
        ),
        ("status", vec![Voice::new(Square(0.25), 300.0, 300.0, 0.3).vib(14.0, 0.15).gain(0.4)]),
        (
            "switch",
            vec![
                Voice::new(Square(0.5), 523.0, 523.0, 0.06).gain(0.35),
                Voice::new(Square(0.5), 659.0, 659.0, 0.06).gain(0.35).at(0.06),
                Voice::new(Square(0.5), 784.0, 784.0, 0.09).gain(0.35).at(0.12),
            ],
        ),
        (
            "send_out",
            vec![
                Voice::new(Noise, 3000.0, 6000.0, 0.08).gain(0.4),
                Voice::new(Square(0.25), 784.0, 784.0, 0.08).gain(0.3).at(0.08),
                Voice::new(Square(0.25), 1046.0, 1046.0, 0.16).gain(0.3).at(0.16),
            ],
        ),
        ("launch", vec![Voice::new(Square(0.5), 600.0, 900.0, 0.07).gain(0.35)]),
        // Per-type move launches.
        ("fire", vec![Voice::new(Noise, 1800.0, 600.0, 0.25).gain(0.6).lp(0.35), Voice::new(Saw, 160.0, 90.0, 0.2).gain(0.2)]),
        ("water", vec![Voice::new(Sine, 400.0, 1200.0, 0.12).vib(30.0, 0.08).gain(0.5), Voice::new(Noise, 7000.0, 7000.0, 0.1).gain(0.15).lp(0.4)]),
        ("grass", vec![Voice::new(Noise, 5000.0, 2500.0, 0.2).gain(0.45).lp(0.2).attack(0.04), Voice::new(Sine, 700.0, 500.0, 0.12).gain(0.2)]),
        ("electric", vec![Voice::new(Square(0.12), 1400.0, 900.0, 0.18).vib(60.0, 0.2).gain(0.35), Voice::new(Noise, 9000.0, 9000.0, 0.15).gain(0.2)]),
        ("rock", vec![Voice::new(Noise, 900.0, 200.0, 0.25).gain(0.6).lp(0.3), Voice::new(Square(0.5), 110.0, 55.0, 0.2).gain(0.4)]),
        ("wind", vec![Voice::new(Noise, 3000.0, 6000.0, 0.3).gain(0.5).lp(0.12).attack(0.08)]),
        ("normal", vec![Voice::new(Square(0.5), 700.0, 1000.0, 0.08).gain(0.35), Voice::new(Square(0.5), 1000.0, 1400.0, 0.06).gain(0.2).at(0.05)]),
    ]
}

fn assets_dir() -> PathBuf {
    let local = PathBuf::from("assets");
    if local.is_dir() {
        local
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
    }
}

fn main() {
    let dir = assets_dir().join("sfx");
    for (name, voices) in sounds() {
        write_wav(&dir.join(format!("{name}.wav")), &render(&voices));
    }
}
