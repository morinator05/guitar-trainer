//! Chord types (formulas) and spelled chords.

use super::interval::Interval;
use super::note::{Naming, Note, Pc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChordCategory {
    Basics,
    Blues,
    Metal,
}

impl ChordCategory {
    pub const ALL: [ChordCategory; 3] =
        [ChordCategory::Basics, ChordCategory::Blues, ChordCategory::Metal];
    pub fn label(self) -> &'static str {
        match self {
            ChordCategory::Basics => "Basics",
            ChordCategory::Blues => "Blues / Rock",
            ChordCategory::Metal => "Metal",
        }
    }
}

pub struct ChordType {
    /// Stable id used for storage and the chord-db mapping.
    pub id: &'static str,
    /// Symbol suffix after the root: "", "m", "7", "maj7" ...
    pub suffix: &'static str,
    pub name: &'static str,
    pub formula: &'static str,
    /// Intervals that may be left out of a voicing.
    pub optional: &'static str,
    pub category: ChordCategory,
    pub info: &'static str,
}

pub const CHORD_TYPES: &[ChordType] = &[
    // ---------- Basics ----------
    ChordType {
        id: "maj",
        suffix: "",
        name: "major",
        formula: "1 3 5",
        optional: "",
        category: ChordCategory::Basics,
        info: "Root + major 3rd + perfect 5th. Bright, stable.",
    },
    ChordType {
        id: "min",
        suffix: "m",
        name: "minor",
        formula: "1 b3 5",
        optional: "",
        category: ChordCategory::Basics,
        info: "Root + minor 3rd + perfect 5th. Dark, sad.",
    },
    ChordType {
        id: "sus2",
        suffix: "sus2",
        name: "suspended 2nd",
        formula: "1 2 5",
        optional: "",
        category: ChordCategory::Basics,
        info: "The 3rd is replaced by the 2nd: open, neither major nor minor.",
    },
    ChordType {
        id: "sus4",
        suffix: "sus4",
        name: "suspended 4th",
        formula: "1 4 5",
        optional: "",
        category: ChordCategory::Basics,
        info: "The 3rd is replaced by the 4th: tense, wants to resolve to major.",
    },
    ChordType {
        id: "dim",
        suffix: "dim",
        name: "diminished",
        formula: "1 b3 b5",
        optional: "",
        category: ChordCategory::Basics,
        info: "Two stacked minor 3rds. Tense; vii° chord of major / ii° of minor.",
    },
    ChordType {
        id: "aug",
        suffix: "aug",
        name: "augmented",
        formula: "1 3 #5",
        optional: "",
        category: ChordCategory::Basics,
        info: "Two stacked major 3rds. III+ chord of harmonic minor.",
    },
    // ---------- Blues / Rock ----------
    ChordType {
        id: "7",
        suffix: "7",
        name: "dominant 7th",
        formula: "1 3 5 b7",
        optional: "5",
        category: ChordCategory::Blues,
        info: "THE blues chord. In a 12-bar blues all three chords (I, IV, V) are 7th chords.",
    },
    ChordType {
        id: "9",
        suffix: "9",
        name: "dominant 9th",
        formula: "1 3 5 b7 9",
        optional: "5",
        category: ChordCategory::Blues,
        info: "Dominant 7th plus the 9th. Funky, used in blues and soul.",
    },
    ChordType {
        id: "13",
        suffix: "13",
        name: "dominant 13th",
        formula: "1 3 5 b7 9 13",
        optional: "5 9",
        category: ChordCategory::Blues,
        info: "Jazzy blues sound; on guitar the 5th and 9th are usually left out.",
    },
    ChordType {
        id: "m7",
        suffix: "m7",
        name: "minor 7th",
        formula: "1 b3 5 b7",
        optional: "5",
        category: ChordCategory::Blues,
        info: "Minor blues, ii chord in major keys.",
    },
    ChordType {
        id: "maj7",
        suffix: "maj7",
        name: "major 7th",
        formula: "1 3 5 7",
        optional: "5",
        category: ChordCategory::Blues,
        info: "Soft, dreamy. Not bluesy but common in ballads.",
    },
    ChordType {
        id: "7#9",
        suffix: "7#9",
        name: "dominant 7th #9 (Hendrix chord)",
        formula: "1 3 5 b7 #9",
        optional: "5",
        category: ChordCategory::Blues,
        info: "Major 3rd and minor 3rd (#9) at the same time: the 'Purple Haze' chord.",
    },
    // ---------- Metal ----------
    ChordType {
        id: "5",
        suffix: "5",
        name: "power chord",
        formula: "1 5",
        optional: "",
        category: ChordCategory::Metal,
        info: "Root + 5th (often + octave). No 3rd, so it sounds good with heavy distortion.",
    },
    ChordType {
        id: "b5",
        suffix: "(b5)",
        name: "tritone power chord",
        formula: "1 b5",
        optional: "",
        category: ChordCategory::Metal,
        info: "Root + diminished 5th ('diabolus in musica'). Evil, used in thrash and doom.",
    },
    ChordType {
        id: "madd9",
        suffix: "m(add9)",
        name: "minor add 9",
        formula: "1 b3 5 9",
        optional: "5",
        category: ChordCategory::Metal,
        info: "Minor chord with the 9th: melancholic, typical in black metal and post-metal.",
    },
    ChordType {
        id: "add9",
        suffix: "add9",
        name: "add 9",
        formula: "1 3 5 9",
        optional: "5",
        category: ChordCategory::Metal,
        info: "Major chord with the 9th: open, shimmering.",
    },
    ChordType {
        id: "m7b5",
        suffix: "m7b5",
        name: "half-diminished",
        formula: "1 b3 b5 b7",
        optional: "",
        category: ChordCategory::Metal,
        info: "ii chord of minor keys. Dark and unresolved.",
    },
    ChordType {
        id: "dim7",
        suffix: "dim7",
        name: "diminished 7th",
        formula: "1 b3 b5 bb7",
        optional: "",
        category: ChordCategory::Metal,
        info: "Stacked minor 3rds, symmetrical: the shape repeats every 3 frets. vii° of harmonic minor.",
    },
    ChordType {
        id: "mmaj7",
        suffix: "m(maj7)",
        name: "minor major 7th",
        formula: "1 b3 5 7",
        optional: "5",
        category: ChordCategory::Metal,
        info: "The i chord of harmonic minor with its leading tone: eerie, horror-movie sound.",
    },
];

impl ChordType {
    pub fn by_id(id: &str) -> Option<&'static ChordType> {
        CHORD_TYPES.iter().find(|c| c.id == id)
    }

    pub fn intervals(&self) -> Vec<Interval> {
        Interval::parse_formula(self.formula)
    }

    pub fn optional_intervals(&self) -> Vec<Interval> {
        Interval::parse_formula(self.optional)
    }

    pub fn is_power_chord(&self) -> bool {
        self.intervals().len() == 2
    }
}

#[derive(Clone, Copy)]
pub struct Chord {
    pub root: Note,
    pub ty: &'static ChordType,
}

impl Chord {
    pub fn new(root: Note, ty: &'static ChordType) -> Chord {
        Chord { root, ty }
    }

    pub fn symbol(&self, naming: Naming) -> String {
        format!("{}{}", self.root.name(naming), self.ty.suffix)
    }

    pub fn notes(&self) -> Vec<Note> {
        self.ty.intervals().iter().map(|i| i.apply(self.root)).collect()
    }

    pub fn pcs(&self) -> Vec<Pc> {
        self.notes().iter().map(|n| n.pc()).collect()
    }

    pub fn root_pc(&self) -> Pc {
        self.root.pc()
    }

    /// Interval of a pitch class within this chord, if it is a chord tone.
    pub fn interval_of(&self, pc: Pc) -> Option<Interval> {
        let off = (pc as i32 - self.root.pc() as i32).rem_euclid(12) as Pc;
        self.ty.intervals().into_iter().find(|i| i.pc_offset() == off)
    }

    /// Pitch classes that every voicing must contain.
    pub fn required_pcs(&self) -> Vec<Pc> {
        let opt = self.ty.optional_intervals();
        self.ty
            .intervals()
            .into_iter()
            .filter(|i| !opt.contains(i))
            .map(|i| i.apply(self.root).pc())
            .collect()
    }

    /// Spelled note for a pitch class in this chord (falls back to default spelling).
    pub fn spell(&self, pc: Pc) -> Note {
        self.interval_of(pc)
            .map(|i| i.apply(self.root))
            .unwrap_or_else(|| Note::from_pc(pc))
    }
}

/// Roots offered in pickers, spelled the common way.
pub fn common_roots() -> Vec<Note> {
    // both spellings of every black key: G#m and Abm are different keys to a reader
    ["C", "C#", "Db", "D", "D#", "Eb", "E", "F", "F#", "Gb", "G", "G#", "Ab", "A", "A#", "Bb", "B"]
        .iter()
        .map(|s| Note::parse(s).unwrap())
        .collect()
}

/// Identify triad quality from root-relative semitone offsets of 3rd and 5th.
pub fn triad_type(third: Pc, fifth: Pc) -> Option<&'static ChordType> {
    let id = match (third, fifth) {
        (4, 7) => "maj",
        (3, 7) => "min",
        (3, 6) => "dim",
        (4, 8) => "aug",
        (2, 7) => "sus2",
        (5, 7) => "sus4",
        _ => return None,
    };
    ChordType::by_id(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::note::Naming::English;

    fn names(c: &Chord) -> Vec<String> {
        c.notes().iter().map(|n| n.name(English)).collect()
    }

    #[test]
    fn spelled_chords() {
        let fs_m7 = Chord::new(Note::parse("F#").unwrap(), ChordType::by_id("m7").unwrap());
        assert_eq!(names(&fs_m7), ["F#", "A", "C#", "E"]);
        let bb7 = Chord::new(Note::parse("Bb").unwrap(), ChordType::by_id("7").unwrap());
        assert_eq!(names(&bb7), ["Bb", "D", "F", "Ab"]);
        let e7s9 = Chord::new(Note::parse("E").unwrap(), ChordType::by_id("7#9").unwrap());
        assert_eq!(names(&e7s9), ["E", "G#", "B", "D", "Fx"]);
        let bdim7 = Chord::new(Note::parse("B").unwrap(), ChordType::by_id("dim7").unwrap());
        assert_eq!(names(&bdim7), ["B", "D", "F", "Ab"]);
        assert_eq!(bb7.symbol(English), "Bb7");
    }

    #[test]
    fn all_formulas_parse_and_ids_unique() {
        let mut ids = std::collections::HashSet::new();
        for t in CHORD_TYPES {
            assert!(!t.intervals().is_empty());
            let _ = t.optional_intervals();
            assert!(ids.insert(t.id), "duplicate id {}", t.id);
        }
    }

    #[test]
    fn required_tones() {
        let c7 = Chord::new(Note::parse("C").unwrap(), ChordType::by_id("7").unwrap());
        let mut req = c7.required_pcs();
        req.sort();
        assert_eq!(req, [0, 4, 10]); // 5th optional
    }
}
