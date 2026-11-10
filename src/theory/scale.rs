//! Scales: formulas, spelled notes, harmonisation.

use super::chord::{ChordType, triad_type};
use super::interval::Interval;
use super::note::{Note, Pc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScaleCategory {
    BluesRock,
    Metal,
}

impl ScaleCategory {
    pub const ALL: [ScaleCategory; 2] = [ScaleCategory::BluesRock, ScaleCategory::Metal];
    pub fn label(self) -> &'static str {
        match self {
            ScaleCategory::BluesRock => "Blues / Rock",
            ScaleCategory::Metal => "Metal",
        }
    }
}

pub struct ScaleType {
    pub id: &'static str,
    pub name: &'static str,
    pub formula: &'static str,
    pub category: ScaleCategory,
    /// Scale whose box shapes are used as a base (blues = minor pentatonic + b5).
    pub parent: Option<&'static str>,
    pub info: &'static str,
}

pub const SCALE_TYPES: &[ScaleType] = &[
    // ---------- Blues / Rock ----------
    ScaleType {
        id: "minor_pentatonic",
        name: "Minor pentatonic",
        formula: "1 b3 4 5 b7",
        category: ScaleCategory::BluesRock,
        parent: None,
        info: "The rock/blues workhorse. Five notes, no 'wrong' notes over a minor or dominant blues. \
               Learn box 1 first (it starts on the root on the lowest string), then connect it to box 2 and box 5.",
    },
    ScaleType {
        id: "blues",
        name: "Blues scale",
        formula: "1 b3 4 b5 5 b7",
        category: ScaleCategory::BluesRock,
        parent: Some("minor_pentatonic"),
        info: "Minor pentatonic + the b5 'blue note'. Use the b5 as a passing note: slide or bend through it \
               (4 → b5 → 5) instead of resting on it.",
    },
    ScaleType {
        id: "major_pentatonic",
        name: "Major pentatonic",
        formula: "1 2 3 5 6",
        category: ScaleCategory::BluesRock,
        parent: None,
        info: "Happy, country/Southern-rock sound. Same shapes as the minor pentatonic three frets lower \
               (A major pent = F# minor pent).",
    },
    ScaleType {
        id: "major_blues",
        name: "Major blues scale",
        formula: "1 2 b3 3 5 6",
        category: ScaleCategory::BluesRock,
        parent: Some("major_pentatonic"),
        info: "Major pentatonic + b3. Slide or hammer b3 → 3 for the classic B.B. King / Allman sound.",
    },
    ScaleType {
        id: "mixolydian",
        name: "Mixolydian",
        formula: "1 2 3 4 5 6 b7",
        category: ScaleCategory::BluesRock,
        parent: None,
        info: "Major scale with b7: fits dominant 7th chords. Mix it with the minor pentatonic over a blues.",
    },
    ScaleType {
        id: "dorian",
        name: "Dorian",
        formula: "1 2 b3 4 5 6 b7",
        category: ScaleCategory::BluesRock,
        parent: None,
        info: "Minor with a major 6th: less dark than natural minor (Santana, 'Oye Como Va').",
    },
    ScaleType {
        id: "major",
        name: "Major (Ionian)",
        formula: "1 2 3 4 5 6 7",
        category: ScaleCategory::BluesRock,
        parent: None,
        info: "The reference scale: every other scale formula is written relative to it.",
    },
    // ---------- Metal ----------
    ScaleType {
        id: "aeolian",
        name: "Natural minor (Aeolian)",
        formula: "1 2 b3 4 5 b6 b7",
        category: ScaleCategory::Metal,
        parent: None,
        info: "The default metal scale (Maiden, Metallica, melodic death metal). Chords: i bIII iv v bVI bVII.",
    },
    ScaleType {
        id: "harmonic_minor",
        name: "Harmonic minor",
        formula: "1 2 b3 4 5 b6 7",
        category: ScaleCategory::Metal,
        parent: None,
        info: "Natural minor with a raised 7th: the augmented 2nd between b6 and 7 sounds 'classical' and sinister. \
               Neoclassical shred (Malmsteen) and black metal (tremolo-picked melodies, e.g. Dissection, Emperor). \
               The V chord becomes major (E in A minor) - that's the dramatic resolution.",
    },
    ScaleType {
        id: "phrygian",
        name: "Phrygian",
        formula: "1 b2 b3 4 5 b6 b7",
        category: ScaleCategory::Metal,
        parent: None,
        info: "Minor with b2: the half-step above the root is the heaviest sound in metal \
               (chug the open string, then hit the b2). Death metal, thrash, black metal.",
    },
    ScaleType {
        id: "phrygian_dominant",
        name: "Phrygian dominant",
        formula: "1 b2 3 4 5 b6 b7",
        category: ScaleCategory::Metal,
        parent: None,
        info: "5th mode of harmonic minor (E phrygian dominant = A harmonic minor from E). \
               Middle-Eastern, 'evil' sound: Nile, Behemoth, Slayer riffs.",
    },
    ScaleType {
        id: "locrian",
        name: "Locrian",
        formula: "1 b2 b3 4 b5 b6 b7",
        category: ScaleCategory::Metal,
        parent: None,
        info: "b2 and b5: very unstable, dissonant. Used in djent/death metal riffs (Meshuggah, Slipknot).",
    },
    ScaleType {
        id: "hungarian_minor",
        name: "Hungarian minor (Gypsy)",
        formula: "1 2 b3 #4 5 b6 7",
        category: ScaleCategory::Metal,
        parent: None,
        info: "Harmonic minor with #4: two augmented 2nds. Exotic and dark, symphonic/black metal.",
    },
    ScaleType {
        id: "diminished_hw",
        name: "Diminished (half-whole)",
        formula: "1 b2 #2 3 #4 5 6 b7",
        category: ScaleCategory::Metal,
        parent: None,
        info: "Symmetrical 8-note scale (half step, whole step ...). Every shape repeats 3 frets higher. \
               Tech-death and shred runs, fits over dim7 and 7b9 chords.",
    },
];

impl ScaleType {
    pub fn by_id(id: &str) -> Option<&'static ScaleType> {
        SCALE_TYPES.iter().find(|s| s.id == id)
    }
    pub fn intervals(&self) -> Vec<Interval> {
        Interval::parse_formula(self.formula)
    }
    pub fn len(&self) -> usize {
        self.intervals().len()
    }
}

#[derive(Clone, Copy)]
pub struct Scale {
    pub root: Note,
    pub ty: &'static ScaleType,
}

/// A chord built on a scale degree.
pub struct DegreeChord {
    pub roman: String,
    pub root: Note,
    pub ty: &'static ChordType,
}

impl Scale {
    pub fn new(root: Note, ty: &'static ScaleType) -> Scale {
        Scale { root, ty }
    }

    pub fn notes(&self) -> Vec<Note> {
        self.ty.intervals().iter().map(|i| i.apply(self.root)).collect()
    }

    pub fn pcs(&self) -> Vec<Pc> {
        self.notes().iter().map(|n| n.pc()).collect()
    }

    pub fn contains(&self, pc: Pc) -> bool {
        self.pcs().contains(&(pc % 12))
    }

    pub fn interval_of(&self, pc: Pc) -> Option<Interval> {
        let off = (pc as i32 - self.root.pc() as i32).rem_euclid(12) as Pc;
        self.ty.intervals().into_iter().find(|i| i.pc_offset() == off)
    }

    pub fn spell(&self, pc: Pc) -> Note {
        self.interval_of(pc)
            .map(|i| i.apply(self.root))
            .unwrap_or_else(|| Note::from_pc(pc))
    }

    /// Triads (and 7th chords if `sevenths`) on each degree, for 7-note scales.
    pub fn harmonise(&self, sevenths: bool) -> Vec<Option<DegreeChord>> {
        let ints = self.ty.intervals();
        if ints.len() != 7 {
            return Vec::new();
        }
        let notes = self.notes();
        (0..7)
            .map(|i| {
                let r = notes[i];
                let off = |k: usize| -> Pc {
                    (notes[(i + k) % 7].pc() as i32 - r.pc() as i32).rem_euclid(12) as Pc
                };
                let (third, fifth, seventh) = (off(2), off(4), off(6));
                let triad = triad_type(third, fifth)?;
                let ty = if sevenths {
                    let id = match (triad.id, seventh) {
                        ("maj", 11) => "maj7",
                        ("maj", 10) => "7",
                        ("min", 10) => "m7",
                        ("min", 11) => "mmaj7",
                        ("dim", 10) => "m7b5",
                        ("dim", 9) => "dim7",
                        _ => return Some(DegreeChord { roman: roman(ints[i], triad.id), root: r, ty: triad }),
                    };
                    ChordType::by_id(id)?
                } else {
                    triad
                };
                Some(DegreeChord {
                    roman: roman(ints[i], triad.id),
                    root: r,
                    ty,
                })
            })
            .collect()
    }
}

fn roman(i: Interval, triad: &str) -> String {
    const R: [&str; 7] = ["I", "II", "III", "IV", "V", "VI", "VII"];
    let base = R[(i.number as usize - 1) % 7];
    let acc = match i.alter {
        -1 => "b",
        1 => "#",
        _ => "",
    };
    match triad {
        "min" => format!("{acc}{}", base.to_lowercase()),
        "dim" => format!("{acc}{}°", base.to_lowercase()),
        "aug" => format!("{acc}{base}+"),
        _ => format!("{acc}{base}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::note::Naming::English;

    fn names(s: &Scale) -> Vec<String> {
        s.notes().iter().map(|n| n.name(English)).collect()
    }

    #[test]
    fn spelled_scales() {
        let a_hm = Scale::new(Note::parse("A").unwrap(), ScaleType::by_id("harmonic_minor").unwrap());
        assert_eq!(names(&a_hm), ["A", "B", "C", "D", "E", "F", "G#"]);
        let e_pd = Scale::new(Note::parse("E").unwrap(), ScaleType::by_id("phrygian_dominant").unwrap());
        assert_eq!(names(&e_pd), ["E", "F", "G#", "A", "B", "C", "D"]);
        let a_bl = Scale::new(Note::parse("A").unwrap(), ScaleType::by_id("blues").unwrap());
        assert_eq!(names(&a_bl), ["A", "C", "D", "Eb", "E", "G"]);
        let d_phr = Scale::new(Note::parse("D").unwrap(), ScaleType::by_id("phrygian").unwrap());
        assert_eq!(names(&d_phr), ["D", "Eb", "F", "G", "A", "Bb", "C"]);
    }

    #[test]
    fn harmonic_minor_chords() {
        let a_hm = Scale::new(Note::parse("A").unwrap(), ScaleType::by_id("harmonic_minor").unwrap());
        let chords: Vec<String> = a_hm
            .harmonise(false)
            .into_iter()
            .map(|c| {
                let c = c.unwrap();
                format!("{} {}{}", c.roman, c.root.name(English), c.ty.suffix)
            })
            .collect();
        assert_eq!(
            chords,
            ["i Am", "ii° Bdim", "bIII+ Caug", "iv Dm", "V E", "bVI F", "vii° G#dim"]
        );
        let sev: Vec<&str> = a_hm.harmonise(true).into_iter().map(|c| c.unwrap().ty.id).collect();
        assert_eq!(sev[0], "mmaj7");
        assert_eq!(sev[4], "7");
        assert_eq!(sev[6], "dim7");
    }

    #[test]
    fn all_scales_parse() {
        for s in SCALE_TYPES {
            assert!(s.len() >= 5);
            if let Some(p) = s.parent {
                assert!(ScaleType::by_id(p).is_some());
            }
        }
    }
}
