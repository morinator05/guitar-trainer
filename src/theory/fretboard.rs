//! Positions on the fretboard.

use super::note::Pc;
use super::tuning::Tuning;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Pos {
    /// 0 = lowest string
    pub string: u8,
    pub fret: u8,
}

impl Pos {
    pub const fn new(string: u8, fret: u8) -> Pos {
        Pos { string, fret }
    }
}

pub fn midi_at(t: &Tuning, p: Pos) -> i32 {
    t.open_midi(p.string as usize) + p.fret as i32
}

pub fn pc_at(t: &Tuning, p: Pos) -> Pc {
    midi_at(t, p).rem_euclid(12) as Pc
}

/// Every position of a pitch class within frets `lo..=hi`.
pub fn positions_of_pc(t: &Tuning, pc: Pc, lo: u8, hi: u8) -> Vec<Pos> {
    let mut out = Vec::new();
    for s in 0..Tuning::STRING_COUNT as u8 {
        for f in lo..=hi {
            let p = Pos::new(s, f);
            if pc_at(t, p) == pc % 12 {
                out.push(p);
            }
        }
    }
    out
}

/// Every position that sounds exactly this MIDI pitch within frets `0..=max_fret`.
pub fn positions_of_midi(t: &Tuning, midi: i32, max_fret: u8) -> Vec<Pos> {
    (0..Tuning::STRING_COUNT as u8)
        .filter_map(|s| {
            let f = midi - t.open_midi(s as usize);
            (0..=max_fret as i32)
                .contains(&f)
                .then_some(Pos::new(s, f as u8))
        })
        .collect()
}

/// Pairs of positions that are exactly one octave apart (for drawing octave shapes).
/// Only pairs that are "close" on the neck (string distance 1..=3) are returned.
pub fn octave_pairs(t: &Tuning, positions: &[Pos]) -> Vec<(Pos, Pos)> {
    let mut out = Vec::new();
    for &a in positions {
        for &b in positions {
            let ds = b.string as i32 - a.string as i32;
            if (1..=3).contains(&ds) && midi_at(t, b) - midi_at(t, a) == 12 {
                out.push((a, b));
            }
        }
    }
    // For each lower note keep the pair closest along the neck.
    out.sort_by_key(|(a, b)| (*a, (b.fret as i32 - a.fret as i32).abs(), b.string));
    out.dedup_by(|x, y| x.0 == y.0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::tuning::*;

    #[test]
    fn notes_at_positions() {
        assert_eq!(midi_at(&E_STANDARD, Pos::new(0, 5)), 45); // A2
        assert_eq!(pc_at(&E_STANDARD, Pos::new(0, 5)), 9);
        assert_eq!(midi_at(&DROP_D, Pos::new(0, 0)), 38); // D2
        assert_eq!(midi_at(&DROP_C, Pos::new(0, 0)), 36); // C2
        assert_eq!(pc_at(&E_STANDARD, Pos::new(4, 1)), 0); // C on B string
    }

    #[test]
    fn all_cs_up_to_12() {
        let cs = positions_of_pc(&E_STANDARD, 0, 0, 12);
        let expected = [
            Pos::new(0, 8),
            Pos::new(1, 3),
            Pos::new(2, 10),
            Pos::new(3, 5),
            Pos::new(4, 1),
            Pos::new(5, 8),
        ];
        assert_eq!(cs, expected);
    }

    #[test]
    fn exact_pitch() {
        // E4 (high open E) exists on all six strings within 24 frets.
        let p = positions_of_midi(&E_STANDARD, 64, 24);
        assert_eq!(p.len(), 6);
        // C4 within 12 frets: A-string? no (A2+12=A3). D string 10, G string 5, B string 1.
        let c4 = positions_of_midi(&E_STANDARD, 60, 12);
        assert_eq!(c4, [Pos::new(2, 10), Pos::new(3, 5), Pos::new(4, 1)]);
    }

    #[test]
    fn octave_shapes() {
        let cs = positions_of_pc(&E_STANDARD, 0, 0, 12);
        let pairs = octave_pairs(&E_STANDARD, &cs);
        // A string fret 3 -> G string fret 5 (two strings, two frets)
        assert!(pairs.contains(&(Pos::new(1, 3), Pos::new(3, 5))));
        // G string fret 5 -> high e fret 8 (crosses the B string: three frets)
        assert!(pairs.contains(&(Pos::new(3, 5), Pos::new(5, 8))));
    }
}
