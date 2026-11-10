//! Hand-made chord shapes from tombatossals/chords-db (MIT, see data/LICENSES.md),
//! embedded in the binary. Shapes are for E standard; they are reused for
//! D standard (same fingering, everything a whole step lower) and, if the low
//! string is not used, for Drop D / Drop C.

use super::chord::Chord;
use super::note::{Note, Pc};
use super::tuning::{E_STANDARD, Tuning};
use super::voicing::{Source, Voicing, check};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

static RAW: &str = include_str!("../../data/chords-db-guitar.json");

#[derive(Deserialize)]
struct Db {
    chords: HashMap<String, Vec<Entry>>,
}

#[derive(Deserialize)]
struct Entry {
    key: String,
    suffix: String,
    positions: Vec<Position>,
}

#[derive(Deserialize)]
struct Position {
    frets: Vec<i32>,
    fingers: Vec<i32>,
    #[serde(rename = "baseFret")]
    base_fret: i32,
    #[serde(default)]
    barres: Vec<i32>,
}

/// (root pitch class, our chord type id) → E-standard voicings
type Table = HashMap<(Pc, &'static str), Vec<Voicing>>;

fn our_id(db_suffix: &str) -> Option<&'static str> {
    Some(match db_suffix {
        "major" => "maj",
        "minor" => "min",
        "dim" => "dim",
        "dim7" => "dim7",
        "sus2" => "sus2",
        "sus4" => "sus4",
        "aug" => "aug",
        "5" => "5",
        "7" => "7",
        "9" => "9",
        "13" => "13",
        "m7" => "m7",
        "maj7" => "maj7",
        "7#9" => "7#9",
        "m7b5" => "m7b5",
        "add9" => "add9",
        "madd9" => "madd9",
        "mmaj7" => "mmaj7",
        _ => return None,
    })
}

fn convert(p: &Position) -> Option<Voicing> {
    if p.frets.len() != 6 || p.fingers.len() != 6 {
        return None;
    }
    let abs = |f: i32| -> Option<u8> {
        match f {
            f if f < 0 => None,
            0 => Some(0),
            f => Some((p.base_fret + f - 1) as u8),
        }
    };
    let mut frets = [None; 6];
    let mut fingers = [0u8; 6];
    for s in 0..6 {
        frets[s] = abs(p.frets[s]);
        fingers[s] = p.fingers[s].clamp(0, 4) as u8;
    }
    let barre = p.barres.first().and_then(|&b| {
        let strings: Vec<usize> = (0..6).filter(|&s| p.frets[s] == b).collect();
        let (lo, hi) = (*strings.first()?, *strings.last()?);
        (hi > lo).then_some(((p.base_fret + b - 1) as u8, lo as u8, hi as u8))
    });
    Some(Voicing {
        frets,
        fingers,
        barre,
        source: Source::Database,
        score: 0.0,
    })
}

fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| {
        let db: Db = serde_json::from_str(RAW).expect("embedded chord db is valid JSON");
        let mut t: Table = HashMap::new();
        for entries in db.chords.values() {
            for e in entries {
                let (Some(id), Some(root)) = (our_id(&e.suffix), Note::parse(&e.key)) else {
                    continue;
                };
                let list = t.entry((root.pc(), id)).or_default();
                for p in &e.positions {
                    if let Some(v) = convert(p)
                        && !list.iter().any(|x: &Voicing| x.frets == v.frets)
                    {
                        list.push(v);
                    }
                }
            }
        }
        t
    })
}

/// How the tuning relates to E standard: (semitones lower, low string usable).
fn relation(t: &Tuning) -> Option<(i32, bool)> {
    if let Some(d) = t.transposition_from_e_standard() {
        return Some((d, true));
    }
    let d = E_STANDARD.open_midi(1) - t.open_midi(1);
    (1..6)
        .all(|s| E_STANDARD.open_midi(s) - t.open_midi(s) == d)
        .then_some((d, false))
}

/// Database voicings for this chord in this tuning. Every shape is verified
/// against the chord (wrong entries are dropped). Inversions are included.
pub fn voicings(t: &Tuning, chord: &Chord) -> Vec<Voicing> {
    let Some((d, low_ok)) = relation(t) else {
        return Vec::new();
    };
    let db_pc = ((chord.root_pc() as i32 + d).rem_euclid(12)) as Pc;
    let Some(list) = table().get(&(db_pc, chord.ty.id)) else {
        return Vec::new();
    };
    let mut out: Vec<Voicing> = list
        .iter()
        .filter(|v| low_ok || v.frets[0].is_none())
        .filter(|v| check(t, chord, &v.frets, true).is_empty())
        .cloned()
        .collect();
    out.sort_by_key(|v| (v.min_fret(), v.bass_string()));
    out
}

/// Number of DB shapes for our chord types and how many fail verification
/// (diagnostics for tests / About screen).
pub fn stats() -> (usize, usize) {
    let mut total = 0;
    let mut bad = 0;
    for ((pc, id), list) in table() {
        let ty = super::chord::ChordType::by_id(id).unwrap();
        let chord = Chord::new(Note::from_pc(*pc), ty);
        for v in list {
            total += 1;
            if !check(&E_STANDARD, &chord, &v.frets, true).is_empty() {
                bad += 1;
            }
        }
    }
    (total, bad)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::chord::ChordType;
    use crate::theory::tuning::*;

    fn chord(root: &str, id: &str) -> Chord {
        Chord::new(Note::parse(root).unwrap(), ChordType::by_id(id).unwrap())
    }

    #[test]
    fn loads_and_mostly_valid() {
        let (total, bad) = stats();
        assert!(total > 500, "only {total} shapes");
        assert!(bad * 20 < total, "{bad} of {total} DB shapes fail verification");
    }

    #[test]
    fn c_major_open() {
        let v = voicings(&E_STANDARD, &chord("C", "maj"));
        let tabs: Vec<String> = v.iter().map(|v| v.tab_string()).collect();
        assert!(tabs.contains(&"x32010".to_string()), "{tabs:?}");
        assert!(tabs.contains(&"x35553".to_string()), "{tabs:?}");
    }

    #[test]
    fn d_standard_reuses_e_shapes() {
        // In D standard, the open "E major" grip 022100 sounds D major.
        let v = voicings(&D_STANDARD, &chord("D", "maj"));
        assert!(v.iter().any(|v| v.tab_string() == "022100"));
    }

    #[test]
    fn drop_d_only_upper_strings() {
        let v = voicings(&DROP_D, &chord("C", "maj"));
        assert!(!v.is_empty());
        assert!(v.iter().all(|v| v.frets[0].is_none()));
    }
}
