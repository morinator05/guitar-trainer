//! Scale shapes on the neck, generated for any tuning.

use super::fretboard::{Pos, midi_at, pc_at};
use super::scale::{Scale, ScaleType};
use super::tuning::Tuning;

#[derive(Clone, Debug)]
pub struct ScalePosition {
    pub name: String,
    /// Notes ordered by pitch (ascending), low string first for equal pitch.
    pub notes: Vec<Pos>,
}

impl ScalePosition {
    pub fn fret_range(&self) -> (u8, u8) {
        let lo = self.notes.iter().map(|p| p.fret).min().unwrap_or(0);
        let hi = self.notes.iter().map(|p| p.fret).max().unwrap_or(0);
        (lo, hi)
    }
}

fn sort_by_pitch(t: &Tuning, v: &mut Vec<Pos>) {
    v.sort_by_key(|p| (midi_at(t, *p), p.string));
    v.dedup();
}

/// Every scale note on the neck within frets 0..=max_fret.
pub fn full_neck(t: &Tuning, scale: &Scale, max_fret: u8) -> Vec<Pos> {
    let mut v = Vec::new();
    for s in 0..6u8 {
        for f in 0..=max_fret {
            let p = Pos::new(s, f);
            if scale.contains(pc_at(t, p)) {
                v.push(p);
            }
        }
    }
    v
}

/// "N notes per string" pattern starting on scale degree `start_degree` on the lowest string.
pub fn nps_pattern(t: &Tuning, scale: &Scale, start_degree: usize, nps: usize, max_fret: u8) -> Vec<Pos> {
    let offs: Vec<i32> = scale.ty.intervals().iter().map(|i| i.pc_offset() as i32).collect();
    let n = offs.len();
    let root = scale.root.pc() as i32;
    let open0 = t.open_midi(0);
    let f0 = (root + offs[start_degree % n] - open0).rem_euclid(12);
    let start_midi = open0 + f0;

    // ascending scale pitches from start
    let mut pitches = vec![start_midi];
    let mut k = start_degree % n;
    while pitches.len() < 6 * nps {
        let next = (k + 1) % n;
        let step = (offs[next] - offs[k]).rem_euclid(12);
        pitches.push(pitches.last().unwrap() + step);
        k = next;
    }
    let frets: Vec<(u8, i32)> = pitches
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let s = (i / nps) as u8;
            (s, m - t.open_midi(s as usize))
        })
        .collect();
    let min = frets.iter().map(|x| x.1).min().unwrap();
    let max = frets.iter().map(|x| x.1).max().unwrap();
    let shift = if min < 0 {
        12
    } else if max > max_fret as i32 && min >= 12 {
        -12
    } else {
        0
    };
    let mut v: Vec<Pos> = frets
        .into_iter()
        .map(|(s, f)| Pos::new(s, (f + shift).clamp(0, 30) as u8))
        .collect();
    sort_by_pitch(t, &mut v);
    v
}

/// Standard positions for a scale:
/// pentatonic → 5 boxes (2 notes per string), 6-note scales with a parent → parent boxes
/// plus the extra notes, 7+ notes → 3-notes-per-string patterns.
pub fn positions(t: &Tuning, scale: &Scale, max_fret: u8) -> Vec<ScalePosition> {
    let n = scale.ty.len();
    if let Some(parent_id) = scale.ty.parent {
        let parent = Scale::new(scale.root, ScaleType::by_id(parent_id).unwrap());
        return positions(t, &parent, max_fret)
            .into_iter()
            .map(|mut p| {
                let (lo, hi) = p.fret_range();
                for s in 0..6u8 {
                    for f in lo..=hi {
                        let pos = Pos::new(s, f);
                        let pc = pc_at(t, pos);
                        if scale.contains(pc) && !parent.contains(pc) {
                            p.notes.push(pos);
                        }
                    }
                }
                sort_by_pitch(t, &mut p.notes);
                p
            })
            .collect();
    }
    let (nps, label) = if n <= 6 { (2, "Box") } else { (3, "3NPS") };
    (0..n)
        .map(|d| ScalePosition {
            name: format!("{label} {}", d + 1),
            notes: nps_pattern(t, scale, d, nps, max_fret),
        })
        .collect()
}

/// All scale notes inside the fret window `start..start+width`.
pub fn window(t: &Tuning, scale: &Scale, start: u8, width: u8) -> ScalePosition {
    let mut v = Vec::new();
    for s in 0..6u8 {
        for f in start..start + width {
            let p = Pos::new(s, f);
            if scale.contains(pc_at(t, p)) {
                v.push(p);
            }
        }
    }
    sort_by_pitch(t, &mut v);
    ScalePosition {
        name: format!("Frets {}–{}", start, start + width - 1),
        notes: v,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::note::Note;
    use crate::theory::scale::SCALE_TYPES;
    use crate::theory::tuning::*;

    fn per_string(p: &ScalePosition) -> Vec<Vec<u8>> {
        let mut v = vec![Vec::new(); 6];
        for n in &p.notes {
            v[n.string as usize].push(n.fret);
        }
        for s in &mut v {
            s.sort();
        }
        v
    }

    #[test]
    fn a_minor_pentatonic_box1() {
        let s = Scale::new(Note::parse("A").unwrap(), ScaleType::by_id("minor_pentatonic").unwrap());
        let pos = positions(&E_STANDARD, &s, 24);
        assert_eq!(pos.len(), 5);
        assert_eq!(
            per_string(&pos[0]),
            [vec![5, 8], vec![5, 7], vec![5, 7], vec![5, 7], vec![5, 8], vec![5, 8]]
        );
        assert_eq!(
            per_string(&pos[1]),
            [vec![8, 10], vec![7, 10], vec![7, 10], vec![7, 9], vec![8, 10], vec![8, 10]]
        );
    }

    #[test]
    fn a_blues_box1_has_blue_notes() {
        let s = Scale::new(Note::parse("A").unwrap(), ScaleType::by_id("blues").unwrap());
        let pos = positions(&E_STANDARD, &s, 24);
        assert_eq!(
            per_string(&pos[0]),
            [vec![5, 8], vec![5, 6, 7], vec![5, 7], vec![5, 7, 8], vec![5, 8], vec![5, 8]]
        );
    }

    #[test]
    fn three_nps() {
        let s = Scale::new(Note::parse("E").unwrap(), ScaleType::by_id("aeolian").unwrap());
        let pos = positions(&E_STANDARD, &s, 24);
        assert_eq!(pos.len(), 7);
        // E natural minor, pattern 1 from open E: 0 2 3 | 0 2 3 | 0 2 4 | 0 2 4 | 1 3 5 | 2 3 5
        assert_eq!(
            per_string(&pos[0]),
            [vec![0, 2, 3], vec![0, 2, 3], vec![0, 2, 4], vec![0, 2, 4], vec![1, 3, 5], vec![2, 3, 5]]
        );
    }

    #[test]
    fn every_scale_every_tuning() {
        for t in ALL_TUNINGS {
            for ty in SCALE_TYPES {
                for r in ["C", "E", "F#", "Bb"] {
                    let s = Scale::new(Note::parse(r).unwrap(), ty);
                    let pos = positions(t, &s, 24);
                    assert!(!pos.is_empty());
                    for p in &pos {
                        assert!(p.notes.len() >= 10, "{} {} {}", ty.name, r, p.name);
                        for n in &p.notes {
                            assert!(s.contains(pc_at(t, *n)));
                            assert!(n.fret <= 24);
                        }
                    }
                }
            }
        }
    }
}
