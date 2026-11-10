# Guitar Trainer

A small native desktop app (Rust + egui) for learning the guitar fretboard, chords and scales,
with a focus on blues and metal. Runs fully offline, uses about 40 MB of RAM for its own data.

## Features

**Fretboard**
- *Explore*: the whole neck, every copy of a note highlighted, octave shapes drawn, heatmap of weak spots.
- Drills: *name the note*, *find the note* (on one string / all copies in a zone),
  *staff → fret* (a written note, treble clef 8vb, → every place it can be played).
- Four levels, from "strings 6 + 5, natural notes" to "whole neck, all notes".

**Chords**
- *Library*: formula, spelled notes, notation, interval-coloured voicings
  (★ hand-made shapes from chords-db + generated voicings for any tuning), filters, strum/arpeggio playback.
- *Build it*: chord tones around a root on string 6/5/4/3, with "N strings up, M frets higher" directions.
- Drills: *sight-read* (symbol or notation → play it in a fret zone; click it or self-check on the guitar),
  *name this chord* (from a diagram or notation).

**Scales**
- Blues/rock (pentatonics, blues, mixolydian, dorian, major) and metal (natural/harmonic minor,
  phrygian, phrygian dominant, locrian, hungarian minor, diminished).
- Box / 3-notes-per-string / fret-window positions, tab + notation, playback with BPM and metronome,
  harmonised chords linking to the chord library.
- Drills: *fill the box*, *which degree?*, *play the position*.

**Progress**: Leitner-box spaced repetition per question, accuracy per area, weakest items,
and a 30-question "today's practice" session across your weakest drills.

**Settings**: E standard, D standard, Drop D, Drop C · English or German note names (H/B, Cis/Es) ·
labels as notes or intervals · left-handed mirror · sound/volume · UI scale.

Keyboard: `Space` reveal / next, `Enter` check, `1` `2` `3` self-rating.

## Build and run

```sh
cargo run --release        # run
./install.sh               # install to ~/.local/bin + menu entry
cargo test                 # theory engine tests
```

### Offline builds

All crate sources can be vendored so building never needs the internet:

```sh
cargo vendor --versioned-dirs vendor
# then put the printed [source...] lines into .cargo/config.toml
```

`vendor/` (≈470 MB) and `.cargo/config.toml` are kept out of git.

## Data

- Settings and progress: `~/.local/share/guitar-trainer/` (JSON).
- Chord shapes: [chords-db](https://github.com/tombatossals/chords-db) (MIT), music font: Bravura (SIL OFL).
  Both are embedded in the binary; see `data/LICENSES.md`.

## Developer notes

- `src/theory/` is pure logic (notes, intervals, tunings, chords, voicing generator, scales, positions),
  unit-tested with `cargo test`.
- `GT_START=<screen>` opens a screen directly, e.g. `GT_START=chords:read cargo run`
  (`fb`, `fb:name`, `fb:find`, `fb:staff`, `chords`, `chords:build`, `chords:read`, `chords:name`,
  `scales`, `scales:fill`, `scales:degree`, `scales:play`, `progress`, `settings`).
