// Проверочные треки с известным темпом, тональностью и строем (WAV 44,1 кГц, 16 бит, стерео) и expected.json.
// Сборка и запуск: cargo run --release --example make_tests -- <папка>
use std::f64::consts::PI;
use std::fs::File;
use std::io::{BufWriter, Write};

const SR: f64 = 44100.0;
const SECONDS: f64 = 45.0;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0) as f32
    }
}

struct Mix {
    l: Vec<f32>,
    r: Vec<f32>,
}

impl Mix {
    fn new() -> Self {
        let n = (SECONDS * SR) as usize;
        Mix { l: vec![0.0; n], r: vec![0.0; n] }
    }

    fn add(&mut self, at: f64, sig: &[f32], gain: f32, pan: f32) {
        let start = (at * SR).round() as usize;
        let (gl, gr) = (gain * (1.0 - pan.max(0.0)), gain * (1.0 + pan.min(0.0)));
        for (i, &v) in sig.iter().enumerate() {
            let k = start + i;
            if k >= self.l.len() {
                break;
            }
            self.l[k] += v * gl;
            self.r[k] += v * gr;
        }
    }
}

fn kick() -> Vec<f32> {
    let n = (0.35 * SR) as usize;
    let mut ph = 0.0f64;
    (0..n)
        .map(|i| {
            let t = i as f64 / SR;
            let f = 45.0 + 115.0 * (-t / 0.03).exp();
            ph += 2.0 * PI * f / SR;
            (ph.sin() * (-t / 0.16).exp()) as f32
        })
        .collect()
}

fn hat(rng: &mut Rng) -> Vec<f32> {
    let n = (0.06 * SR) as usize;
    let mut prev = 0f32;
    (0..n)
        .map(|i| {
            let x = rng.next();
            let y = x - prev;
            prev = x;
            y * (-(i as f64 / SR) / 0.015).exp() as f32
        })
        .collect()
}

fn snare(rng: &mut Rng) -> Vec<f32> {
    let n = (0.22 * SR) as usize;
    (0..n)
        .map(|i| {
            let t = i as f64 / SR;
            (0.6 * rng.next() as f64 * (-t / 0.06).exp() + 0.5 * (2.0 * PI * 190.0 * t).sin() * (-t / 0.05).exp()) as f32
        })
        .collect()
}

/// Нота из суммы гармоник 1/k^rolloff с линейной атакой и затуханием.
fn tone(freq: f64, dur: f64, harmonics: usize, rolloff: f64, attack: f64, release: f64, decay: f64) -> Vec<f32> {
    let n = (dur * SR) as usize;
    let parts: Vec<(f64, f64)> = (1..=harmonics)
        .filter(|&k| freq * (k as f64) < 9000.0)
        .map(|k| (freq * k as f64, 1.0 / (k as f64).powf(rolloff)))
        .collect();
    let norm: f64 = parts.iter().map(|p| p.1).sum();
    (0..n)
        .map(|i| {
            let t = i as f64 / SR;
            let mut env = (t / attack).min(1.0) * ((dur - t) / release).clamp(0.0, 1.0);
            if decay > 0.0 {
                env *= (-t / decay).exp();
            }
            let v: f64 = parts.iter().map(|(f, a)| a * (2.0 * PI * f * t).sin()).sum();
            (v / norm * env) as f32
        })
        .collect()
}

fn freq(midi: f64, ref_a: f64) -> f64 {
    ref_a * 2f64.powf((midi - 69.0) / 12.0)
}

#[derive(Clone, Copy)]
enum Drums {
    Four,
    HalfTime,
    Dnb,
    Dubstep,
    None,
}

#[derive(Clone, Copy)]
enum Bass {
    Offbeat8,
    Psy16,
    Whole,
}

struct Spec {
    file: &'static str,
    bpm: f64,
    tonic: u8,
    minor: bool,
    ref_a: f64,
    /// Аккорды по тактам: (корень MIDI, минор?)
    chords: &'static [(u8, bool)],
    drums: Drums,
    bass: Bass,
    stabs: bool,
    /// Басовая педаль: бас всё время на этой ноте (MIDI аккорда), а не на корне аккорда.
    pedal: Option<u8>,
}

fn render(s: &Spec, out: &str) {
    let mut mix = Mix::new();
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let beat = 60.0 / s.bpm;
    let bar = 4.0 * beat;
    let k = kick();
    let bars = (SECONDS / bar).ceil() as usize;
    for b in 0..bars {
        let t0 = 0.5 + b as f64 * bar;
        let (root, minor) = s.chords[b % s.chords.len()];
        let third = if minor { 3.0 } else { 4.0 };
        let notes = [root as f64, root as f64 + third, root as f64 + 7.0];
        // аккорды
        if s.stabs {
            for q in 0..4 {
                for (i, &m) in notes.iter().enumerate() {
                    let sig = tone(freq(m, s.ref_a), beat * 0.9, 8, 1.2, 0.004, 0.05, 0.35);
                    mix.add(t0 + q as f64 * beat, &sig, 0.28, (i as f32 - 1.0) * 0.3);
                }
            }
        } else {
            for (i, &m) in notes.iter().enumerate() {
                let sig = tone(freq(m, s.ref_a), bar * 0.98, 6, 1.5, 0.05, 0.2, 0.0);
                mix.add(t0, &sig, 0.16, (i as f32 - 1.0) * 0.4);
            }
        }
        // бас
        let bf = freq(s.pedal.unwrap_or(root) as f64 - 24.0, s.ref_a);
        match s.bass {
            Bass::Offbeat8 => {
                for q in 0..4 {
                    let sig = tone(bf, beat * 0.4, 8, 1.3, 0.005, 0.03, 0.0);
                    mix.add(t0 + (q as f64 + 0.5) * beat, &sig, 0.35, 0.0);
                }
            }
            Bass::Psy16 => {
                for q in 0..4 {
                    for j in 1..4 {
                        let sig = tone(bf, beat * 0.2, 8, 1.3, 0.003, 0.02, 0.0);
                        mix.add(t0 + (q as f64 + j as f64 * 0.25) * beat, &sig, 0.32, 0.0);
                    }
                }
            }
            Bass::Whole => {
                let sig = tone(bf, bar * 0.95, 8, 1.3, 0.01, 0.1, 0.0);
                mix.add(t0, &sig, 0.3, 0.0);
            }
        }
        // ударные: позиции в долях такта
        let (kicks, snares, hats): (Vec<f64>, Vec<f64>, Vec<f64>) = match s.drums {
            Drums::Four => (vec![0.0, 1.0, 2.0, 3.0], vec![], vec![0.5, 1.5, 2.5, 3.5]),
            Drums::HalfTime => (vec![0.0, 2.0], vec![1.0, 3.0], (0..8).map(|i| i as f64 * 0.5).collect()),
            Drums::Dnb => (vec![0.0, 2.5], vec![1.0, 3.0], (0..8).map(|i| i as f64 * 0.5).collect()),
            Drums::Dubstep => (vec![0.0], vec![2.0], vec![0.0, 1.0, 2.0, 3.0]),
            Drums::None => (vec![], vec![], vec![]),
        };
        for p in kicks {
            mix.add(t0 + p * beat, &k, 0.9, 0.0);
        }
        for p in snares {
            let sig = snare(&mut rng);
            mix.add(t0 + p * beat, &sig, 0.5, 0.0);
        }
        for p in hats {
            let sig = hat(&mut rng);
            mix.add(t0 + p * beat, &sig, 0.18, 0.2);
        }
    }
    write_wav(&format!("{out}/{}", s.file), &mix);
}

fn write_wav(path: &str, mix: &Mix) {
    let peak = mix.l.iter().chain(&mix.r).fold(1e-9f32, |m, v| m.max(v.abs()));
    let g = 0.9 / peak;
    let n = mix.l.len();
    let mut w = BufWriter::new(File::create(path).expect("файл WAV"));
    let data_len = (n * 4) as u32;
    w.write_all(b"RIFF").unwrap();
    w.write_all(&(36 + data_len).to_le_bytes()).unwrap();
    w.write_all(b"WAVEfmt ").unwrap();
    w.write_all(&16u32.to_le_bytes()).unwrap();
    w.write_all(&1u16.to_le_bytes()).unwrap();
    w.write_all(&2u16.to_le_bytes()).unwrap();
    w.write_all(&(SR as u32).to_le_bytes()).unwrap();
    w.write_all(&(SR as u32 * 4).to_le_bytes()).unwrap();
    w.write_all(&4u16.to_le_bytes()).unwrap();
    w.write_all(&16u16.to_le_bytes()).unwrap();
    w.write_all(b"data").unwrap();
    w.write_all(&data_len.to_le_bytes()).unwrap();
    for i in 0..n {
        for v in [mix.l[i], mix.r[i]] {
            let s = (v * g * 32767.0).round().clamp(-32768.0, 32767.0) as i16;
            w.write_all(&s.to_le_bytes()).unwrap();
        }
    }
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    std::fs::create_dir_all(&out).unwrap();
    let a = |cents: f64| 440.0 * 2f64.powf(cents / 1200.0);
    // Минорные треки — с выделенной тоникой (i–i–VI–VII, i–VI–VII–i, басовая педаль у пситранса):
    // схема i–VI–III–VII («Am F C G») на слух двусмысленна с параллельным мажором и в проверку не годится.
    let specs = [
        Spec { file: "t1_128_Am.wav", bpm: 128.0, tonic: 9, minor: true, ref_a: 440.0,
               chords: &[(57, true), (57, true), (53, false), (55, false)], drums: Drums::Four, bass: Bass::Offbeat8, stabs: false, pedal: None },
        Spec { file: "t2_145_F#m_+20c.wav", bpm: 145.0, tonic: 6, minor: true, ref_a: a(20.0),
               chords: &[(54, true), (50, false), (52, false), (54, true)], drums: Drums::Four, bass: Bass::Psy16, stabs: false, pedal: Some(54) },
        Spec { file: "t3_87.5_D.wav", bpm: 87.5, tonic: 2, minor: false, ref_a: 440.0,
               chords: &[(50, false), (55, false), (57, false), (50, false)], drums: Drums::HalfTime, bass: Bass::Whole, stabs: false, pedal: None },
        Spec { file: "t4_174_Em.wav", bpm: 174.0, tonic: 4, minor: true, ref_a: 440.0,
               chords: &[(52, true), (48, false), (50, false), (52, true)], drums: Drums::Dnb, bass: Bass::Whole, stabs: false, pedal: None },
        Spec { file: "t5_100_C_nodrums.wav", bpm: 100.0, tonic: 0, minor: false, ref_a: 440.0,
               chords: &[(60, false), (53, false), (55, false), (60, false)], drums: Drums::None, bass: Bass::Whole, stabs: true, pedal: None },
        Spec { file: "t6_124.37_Bbm_432.wav", bpm: 124.37, tonic: 10, minor: true, ref_a: 432.0,
               chords: &[(58, true), (58, true), (54, false), (56, false)], drums: Drums::Four, bass: Bass::Offbeat8, stabs: false, pedal: None },
        Spec { file: "t7_140_Gm_halftime.wav", bpm: 140.0, tonic: 7, minor: true, ref_a: 440.0,
               chords: &[(55, true), (51, false), (53, false), (55, true)], drums: Drums::Dubstep, bass: Bass::Whole, stabs: false, pedal: None },
    ];
    let mut json = String::from("[\n");
    for (i, s) in specs.iter().enumerate() {
        render(s, &out);
        let cents = 1200.0 * (s.ref_a / 440.0).log2();
        json += &format!(
            "  {{\"file\": \"{}\", \"bpm\": {}, \"tonic\": {}, \"minor\": {}, \"cents\": {:.2}}}{}\n",
            s.file, s.bpm, s.tonic, s.minor, cents, if i + 1 < specs.len() { "," } else { "" }
        );
        println!("{}", s.file);
    }
    json += "]\n";
    std::fs::write(format!("{out}/expected.json"), json).unwrap();
}
