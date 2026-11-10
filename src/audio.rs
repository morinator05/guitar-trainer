//! Offline sound: Karplus-Strong plucked strings + metronome click, mixed in the
//! audio thread. No samples, no network.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

enum Cmd {
    /// Pluck `midi` after `delay` seconds.
    Pluck { midi: i32, delay: f32, gain: f32 },
    /// Cancel all scheduled (not yet started) plucks and fade out ringing strings.
    Stop,
    /// Start (Some(bpm)) or stop (None) the metronome.
    Metronome { bpm: Option<f32>, beats_per_bar: u32 },
}

pub struct Audio {
    tx: Option<Sender<Cmd>>,
    volume: Arc<AtomicU32>,
    _stream: Option<cpal::Stream>,
    pub error: Option<String>,
    /// When the last sequence started (for UI highlighting).
    pub seq_start: Option<Instant>,
}

impl Audio {
    pub fn new(volume: f32) -> Audio {
        let vol = Arc::new(AtomicU32::new(volume.to_bits()));
        match start_stream(vol.clone()) {
            Ok((tx, stream)) => Audio {
                tx: Some(tx),
                volume: vol,
                _stream: Some(stream),
                error: None,
                seq_start: None,
            },
            Err(e) => Audio {
                tx: None,
                volume: vol,
                _stream: None,
                error: Some(e),
                seq_start: None,
            },
        }
    }

    pub fn available(&self) -> bool {
        self.tx.is_some()
    }

    pub fn set_volume(&self, v: f32) {
        self.volume.store(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    fn send(&self, c: Cmd) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(c);
        }
    }

    pub fn note(&mut self, midi: i32) {
        self.send(Cmd::Pluck { midi, delay: 0.0, gain: 1.0 });
    }

    /// Strum: low to high with a small offset between strings.
    pub fn strum(&mut self, midis: &[i32]) {
        self.stop();
        for (i, &m) in midis.iter().enumerate() {
            self.send(Cmd::Pluck {
                midi: m,
                delay: i as f32 * 0.035,
                gain: 0.8,
            });
        }
    }

    /// Play notes one after another at `bpm` (one note per beat * `per_beat`).
    pub fn sequence(&mut self, midis: &[i32], bpm: f32, per_beat: f32) {
        self.stop();
        let step = 60.0 / bpm / per_beat;
        for (i, &m) in midis.iter().enumerate() {
            self.send(Cmd::Pluck {
                midi: m,
                delay: i as f32 * step,
                gain: 1.0,
            });
        }
        self.seq_start = Some(Instant::now());
    }

    pub fn stop(&mut self) {
        self.send(Cmd::Stop);
        self.seq_start = None;
    }

    pub fn metronome(&self, bpm: Option<f32>, beats_per_bar: u32) {
        self.send(Cmd::Metronome { bpm, beats_per_bar });
    }
}

fn start_stream(volume: Arc<AtomicU32>) -> Result<(Sender<Cmd>, cpal::Stream), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("no audio output device found")?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let format = supported.sample_format();
    let config = supported.config();
    let (tx, rx) = channel();
    let stream = match format {
        cpal::SampleFormat::F32 => build::<f32>(&device, config, rx, volume),
        cpal::SampleFormat::I16 => build::<i16>(&device, config, rx, volume),
        cpal::SampleFormat::U16 => build::<u16>(&device, config, rx, volume),
        cpal::SampleFormat::I32 => build::<i32>(&device, config, rx, volume),
        f => return Err(format!("unsupported sample format {f:?}")),
    }?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((tx, stream))
}

fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    rx: Receiver<Cmd>,
    volume: Arc<AtomicU32>,
) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let mut synth = Synth::new(config.sample_rate as f32, rx);
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let vol = f32::from_bits(volume.load(Ordering::Relaxed));
                synth.poll();
                for frame in data.chunks_mut(channels) {
                    let s = (synth.next_sample() * vol).clamp(-1.0, 1.0);
                    for out in frame.iter_mut() {
                        *out = T::from_sample(s);
                    }
                }
            },
            |e| eprintln!("audio error: {e}"),
            None,
        )
        .map_err(|e| e.to_string())
}

struct Voice {
    buf: Vec<f32>,
    idx: usize,
    decay: f32,
    gain: f32,
    age: usize,
    release: bool,
}

struct Pending {
    at: u64,
    midi: i32,
    gain: f32,
}

struct Synth {
    sr: f32,
    rx: Receiver<Cmd>,
    t: u64,
    voices: Vec<Voice>,
    pending: Vec<Pending>,
    rng: u32,
    metro: Option<(f32, u32)>,
    metro_next: u64,
    metro_beat: u32,
    click: Option<(usize, f32)>,
    lp: f32,
}

impl Synth {
    fn new(sr: f32, rx: Receiver<Cmd>) -> Synth {
        Synth {
            sr,
            rx,
            t: 0,
            voices: Vec::new(),
            pending: Vec::new(),
            rng: 0x1234_5678,
            metro: None,
            metro_next: 0,
            metro_beat: 0,
            click: None,
            lp: 0.0,
        }
    }

    fn rand(&mut self) -> f32 {
        // xorshift32
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn poll(&mut self) {
        while let Ok(c) = self.rx.try_recv() {
            match c {
                Cmd::Pluck { midi, delay, gain } => {
                    let at = self.t + (delay * self.sr) as u64;
                    self.pending.push(Pending { at, midi, gain });
                }
                Cmd::Stop => {
                    self.pending.clear();
                    for v in &mut self.voices {
                        v.release = true;
                    }
                }
                Cmd::Metronome { bpm, beats_per_bar } => {
                    self.metro = bpm.map(|b| (b, beats_per_bar.max(1)));
                    self.metro_next = self.t;
                    self.metro_beat = 0;
                }
            }
        }
    }

    fn pluck(&mut self, midi: i32, gain: f32) {
        let freq = 440.0 * 2f32.powf((midi as f32 - 69.0) / 12.0);
        let n = (self.sr / freq).round().max(2.0) as usize;
        let mut buf = Vec::with_capacity(n);
        // filtered noise burst = softer, more guitar-like attack
        let mut prev = 0.0;
        for _ in 0..n {
            let r = self.rand();
            prev = 0.5 * prev + 0.5 * r;
            buf.push(prev);
        }
        // low notes ring longer
        let decay = (0.9965 + (60.0 - midi as f32).max(0.0) * 0.00004).min(0.9992);
        if self.voices.len() >= 24 {
            self.voices.remove(0);
        }
        self.voices.push(Voice {
            buf,
            idx: 0,
            decay,
            gain,
            age: 0,
            release: false,
        });
    }

    fn next_sample(&mut self) -> f32 {
        // scheduled plucks
        if !self.pending.is_empty() {
            let t = self.t;
            let mut due = Vec::new();
            self.pending.retain(|p| {
                if p.at <= t {
                    due.push((p.midi, p.gain));
                    false
                } else {
                    true
                }
            });
            for (m, g) in due {
                self.pluck(m, g);
            }
        }

        let mut out = 0.0;
        let max_age = (self.sr * 5.0) as usize;
        let rel_len = self.sr * 0.08;
        for v in &mut self.voices {
            let n = v.buf.len();
            let i = v.idx;
            let j = (i + 1) % n;
            let s = v.buf[i];
            v.buf[i] = v.decay * 0.5 * (v.buf[i] + v.buf[j]);
            v.idx = j;
            v.age += 1;
            if v.release {
                v.gain *= (-1.0 / rel_len).exp();
            }
            out += s * v.gain * 0.35;
        }
        self.voices
            .retain(|v| v.age < max_age && v.gain > 0.001 && !(v.release && v.gain < 0.01));

        // metronome
        if let Some((bpm, bpb)) = self.metro
            && self.t >= self.metro_next
        {
            let accent = self.metro_beat % bpb == 0;
            self.click = Some((0, if accent { 1800.0 } else { 1100.0 }));
            self.metro_beat += 1;
            self.metro_next = self.t + (60.0 / bpm * self.sr) as u64;
        }
        if let Some((n, f)) = self.click {
            let len = (self.sr * 0.03) as usize;
            if n < len {
                let env = 1.0 - n as f32 / len as f32;
                out += (n as f32 / self.sr * f * std::f32::consts::TAU).sin() * env * env * 0.4;
                self.click = Some((n + 1, f));
            } else {
                self.click = None;
            }
        }

        // gentle low-pass + soft clip
        self.lp += 0.6 * (out - self.lp);
        self.t += 1;
        (self.lp * 1.2).tanh()
    }
}
