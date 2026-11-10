//! Offline sound: Karplus-Strong plucked strings + metronome click, mixed in the
//! audio thread. No samples, no network.
//!
//! The audio callback never allocates: voices come from a preallocated pool and
//! scheduled notes live in a fixed-capacity queue.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

const MAX_VOICES: usize = 24;
const MAX_PENDING: usize = 256;
/// Lowest note we may need to synthesise (C1 ≈ 32.7 Hz) – sizes the voice buffers.
const LOWEST_FREQ: f32 = 32.0;
const NO_SEQ: u64 = u64::MAX;

enum Cmd {
    /// Pluck `midi` after `delay` seconds.
    Pluck { midi: i32, delay: f32, gain: f32 },
    /// Cancel scheduled notes and fade out ringing strings.
    Stop,
    /// Marks the time origin of a note sequence (for the UI highlight).
    SeqStart,
    /// Start (Some(bpm)) or stop (None) the metronome.
    Metronome { bpm: Option<f32>, beats_per_bar: u32 },
}

/// State shared between the UI and the audio thread.
struct Shared {
    volume: AtomicU32,
    /// samples rendered so far
    clock: AtomicU64,
    /// sample time at which the current sequence started (NO_SEQ = none)
    seq_start: AtomicU64,
    sample_rate: AtomicU32,
}

pub struct Audio {
    tx: Option<Sender<Cmd>>,
    shared: Arc<Shared>,
    _stream: Option<cpal::Stream>,
    pub error: Option<String>,
    /// Id of the most recent sequence; screens compare it with their own.
    seq_id: u64,
}

impl Audio {
    pub fn new(volume: f32) -> Audio {
        let shared = Arc::new(Shared {
            volume: AtomicU32::new(volume.to_bits()),
            clock: AtomicU64::new(0),
            seq_start: AtomicU64::new(NO_SEQ),
            sample_rate: AtomicU32::new(48_000),
        });
        let (tx, stream, error) = match start_stream(shared.clone()) {
            Ok((tx, s)) => (Some(tx), Some(s), None),
            Err(e) => (None, None, Some(e)),
        };
        Audio {
            tx,
            shared,
            _stream: stream,
            error,
            seq_id: 0,
        }
    }

    pub fn available(&self) -> bool {
        self.tx.is_some()
    }

    pub fn set_volume(&self, v: f32) {
        self.shared.volume.store(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
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

    /// Play notes one after another at `bpm` (`per_beat` notes per beat).
    /// Returns the sequence id (see [`Audio::seq_elapsed`]).
    pub fn sequence(&mut self, midis: &[i32], bpm: f32, per_beat: f32) -> u64 {
        self.stop();
        self.seq_id += 1;
        self.send(Cmd::SeqStart);
        let step = 60.0 / bpm / per_beat;
        for (i, &m) in midis.iter().enumerate() {
            self.send(Cmd::Pluck {
                midi: m,
                delay: i as f32 * step,
                gain: 1.0,
            });
        }
        self.seq_id
    }

    /// Seconds since sequence `id` actually started sounding (None if another
    /// sound replaced it or it has not started yet).
    pub fn seq_elapsed(&self, id: u64) -> Option<f32> {
        if id != self.seq_id || self.tx.is_none() {
            return None;
        }
        let start = self.shared.seq_start.load(Ordering::Relaxed);
        if start == NO_SEQ {
            return None;
        }
        let now = self.shared.clock.load(Ordering::Relaxed);
        let sr = self.shared.sample_rate.load(Ordering::Relaxed).max(1);
        Some(now.saturating_sub(start) as f32 / sr as f32)
    }

    /// Is `id` still the latest sequence (not replaced by another sound or stop)?
    pub fn is_current(&self, id: u64) -> bool {
        id == self.seq_id
    }

    pub fn stop(&mut self) {
        self.seq_id += 1;
        self.send(Cmd::Stop);
    }

    pub fn metronome(&self, bpm: Option<f32>, beats_per_bar: u32) {
        self.send(Cmd::Metronome { bpm, beats_per_bar });
    }
}

fn start_stream(shared: Arc<Shared>) -> Result<(Sender<Cmd>, cpal::Stream), String> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("no audio output device found")?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let format = supported.sample_format();
    let config = supported.config();
    shared.sample_rate.store(config.sample_rate, Ordering::Relaxed);
    let (tx, rx) = channel();
    let stream = match format {
        cpal::SampleFormat::F32 => build::<f32>(&device, config, rx, shared),
        cpal::SampleFormat::I16 => build::<i16>(&device, config, rx, shared),
        cpal::SampleFormat::U16 => build::<u16>(&device, config, rx, shared),
        cpal::SampleFormat::I32 => build::<i32>(&device, config, rx, shared),
        f => return Err(format!("unsupported sample format {f:?}")),
    }?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((tx, stream))
}

fn build<T>(device: &cpal::Device, config: cpal::StreamConfig, rx: Receiver<Cmd>, shared: Arc<Shared>) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let mut synth = Synth::new(config.sample_rate as f32, rx);
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let vol = f32::from_bits(shared.volume.load(Ordering::Relaxed));
                synth.poll(&shared);
                for frame in data.chunks_mut(channels) {
                    let s = (synth.next_sample() * vol).clamp(-1.0, 1.0);
                    for out in frame.iter_mut() {
                        *out = T::from_sample(s);
                    }
                }
                shared.clock.store(synth.t, Ordering::Relaxed);
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
    active: bool,
}

#[derive(Clone, Copy)]
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
    next_due: u64,
    rng: u32,
    metro: Option<(f32, u32)>,
    metro_next: u64,
    metro_beat: u32,
    click: Option<(usize, f32)>,
    lp: f32,
}

impl Synth {
    fn new(sr: f32, rx: Receiver<Cmd>) -> Synth {
        let cap = (sr / LOWEST_FREQ).ceil() as usize + 2;
        let voices = (0..MAX_VOICES)
            .map(|_| Voice {
                buf: Vec::with_capacity(cap),
                idx: 0,
                decay: 0.0,
                gain: 0.0,
                age: 0,
                release: false,
                active: false,
            })
            .collect();
        Synth {
            sr,
            rx,
            t: 0,
            voices,
            pending: Vec::with_capacity(MAX_PENDING),
            next_due: u64::MAX,
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

    fn poll(&mut self, shared: &Shared) {
        while let Ok(c) = self.rx.try_recv() {
            match c {
                Cmd::Pluck { midi, delay, gain } => {
                    if self.pending.len() < MAX_PENDING {
                        let at = self.t + (delay * self.sr) as u64;
                        self.pending.push(Pending { at, midi, gain });
                        self.next_due = self.next_due.min(at);
                    }
                }
                Cmd::Stop => {
                    self.pending.clear();
                    self.next_due = u64::MAX;
                    shared.seq_start.store(NO_SEQ, Ordering::Relaxed);
                    for v in &mut self.voices {
                        v.release = true;
                    }
                }
                Cmd::SeqStart => shared.seq_start.store(self.t, Ordering::Relaxed),
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
        let slot = match self.voices.iter().position(|v| !v.active) {
            Some(i) => i,
            // steal the oldest voice
            None => (0..self.voices.len()).max_by_key(|&i| self.voices[i].age).unwrap_or(0),
        };
        let cap = self.voices[slot].buf.capacity();
        let n = ((self.sr / freq).round().max(2.0) as usize).min(cap);
        // filtered noise burst = softer, more guitar-like attack (no allocation:
        // the buffer only changes length within its capacity)
        let mut prev = 0.0;
        self.voices[slot].buf.clear();
        for _ in 0..n {
            let r = self.rand();
            prev = 0.5 * prev + 0.5 * r;
            self.voices[slot].buf.push(prev);
        }
        let v = &mut self.voices[slot];
        v.idx = 0;
        v.decay = (0.9965 + (60.0 - midi as f32).max(0.0) * 0.00004).min(0.9992);
        v.gain = gain;
        v.age = 0;
        v.release = false;
        v.active = true;
    }

    fn next_sample(&mut self) -> f32 {
        // scheduled plucks (in place, no allocation)
        if self.t >= self.next_due {
            let mut i = 0;
            self.next_due = u64::MAX;
            while i < self.pending.len() {
                let p = self.pending[i];
                if p.at <= self.t {
                    self.pending.swap_remove(i);
                    self.pluck(p.midi, p.gain);
                } else {
                    self.next_due = self.next_due.min(p.at);
                    i += 1;
                }
            }
        }

        let mut out = 0.0;
        let max_age = (self.sr * 5.0) as usize;
        let rel = (-1.0 / (self.sr * 0.08)).exp();
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let n = v.buf.len();
            let i = v.idx;
            let j = if i + 1 == n { 0 } else { i + 1 };
            let s = v.buf[i];
            v.buf[i] = v.decay * 0.5 * (v.buf[i] + v.buf[j]);
            v.idx = j;
            v.age += 1;
            if v.release {
                v.gain *= rel;
            }
            out += s * v.gain * 0.35;
            if v.age >= max_age || v.gain < 0.001 || (v.release && v.gain < 0.01) {
                v.active = false;
            }
        }

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
