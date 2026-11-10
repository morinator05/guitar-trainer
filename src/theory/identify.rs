//! Chord identification: name any combination of notes.
//!
//! Every played pitch class is tried as a root against a dictionary of chord
//! types (the library catalog plus extra naming-only types). Exact matches may
//! leave out optional tones (usually the 5th). If nothing matches exactly, a
//! fallback names the notes as a basic chord plus tensions, e.g. "Cmaj7(#11)".

use super::chord::{CHORD_TYPES, ChordCategory, ChordType};
use super::interval::Interval;
use super::note::{Naming, Note, Pc};
use super::scale::{SCALE_TYPES, Scale};

/// Chord types used only for naming (not in the library or drills).
pub const IDENT_TYPES: &[ChordType] = &[
    t("6", "6", "major 6th", "1 3 5 6", "5"),
    t("m6", "m6", "minor 6th", "1 b3 5 6", "5"),
    t("69", "6/9", "6/9", "1 3 5 6 9", "5"),
    t("7sus4", "7sus4", "dominant 7th sus4", "1 4 5 b7", "5"),
    t("9sus4", "9sus4", "dominant 9th sus4", "1 4 5 b7 9", "5"),
    t("sus24", "sus2sus4", "sus2 + sus4", "1 2 4 5", "5"),
    t("maj9", "maj9", "major 9th", "1 3 5 7 9", "5"),
    t("m9", "m9", "minor 9th", "1 b3 5 b7 9", "5"),
    t("11", "11", "dominant 11th", "1 3 5 b7 9 11", "3 5 9"),
    t("m11", "m11", "minor 11th", "1 b3 5 b7 9 11", "5 9"),
    t("maj13", "maj13", "major 13th", "1 3 5 7 9 13", "5 9"),
    t("7b9", "7b9", "dominant 7th b9", "1 3 5 b7 b9", "5"),
    t("7b5", "7b5", "dominant 7th b5", "1 3 b5 b7", ""),
    t("7s5", "7#5", "augmented 7th", "1 3 #5 b7", ""),
    t("7s11", "7#11", "dominant 7th #11", "1 3 5 b7 #11", "5"),
    t("7b13", "7b13", "dominant 7th b13", "1 3 5 b7 b13", "5"),
    t("add11", "add11", "add 11", "1 3 5 11", "5"),
    t("madd11", "m(add11)", "minor add 11", "1 b3 5 11", "5"),
    t("maj7s5", "maj7#5", "major 7th #5", "1 3 #5 7", ""),
    t("maj7b5", "maj7b5", "major 7th b5", "1 3 b5 7", ""),
    t("mb6", "m(b6)", "minor add b6", "1 b3 5 b6", ""),
];

const fn t(id: &'static str, suffix: &'static str, name: &'static str, formula: &'static str, optional: &'static str) -> ChordType {
    ChordType {
        id,
        suffix,
        name,
        formula,
        optional,
        category: ChordCategory::Basics,
        info: "",
    }
}

/// Basic chords the fallback builds on.
const FALLBACK_BASES: &[&str] = &[
    "maj", "min", "dim", "aug", "sus2", "sus4", "5", "7", "maj7", "m7", "m7b5", "dim7", "mmaj7", "6", "m6",
];

fn all_types() -> impl Iterator<Item = &'static ChordType> {
    CHORD_TYPES.iter().chain(IDENT_TYPES.iter())
}

fn type_by_id(id: &str) -> Option<&'static ChordType> {
    all_types().find(|t| t.id == id)
}

#[derive(Clone)]
pub struct Candidate {
    pub root: Note,
    pub ty: &'static ChordType,
    /// Bass note if it is not the root (slash chord).
    pub bass: Option<Note>,
    /// Chord tones that are not played (optional ones only).
    pub missing: Vec<Interval>,
    /// Extra notes named as tensions (fallback only).
    pub tensions: Vec<Interval>,
    pub score: f32,
    /// Set when this chord type is in the chord library.
    pub catalog_id: Option<&'static str>,
}

impl Candidate {
    pub fn symbol(&self, naming: Naming) -> String {
        let mut s = format!("{}{}", self.root.name(naming), self.ty.suffix);
        if !self.tensions.is_empty() {
            let t: Vec<String> = self.tensions.iter().map(|i| i.formula_name()).collect();
            s.push_str(&format!("({})", t.join(",")));
        }
        if let Some(b) = self.bass {
            s.push_str(&format!("/{}", b.name(naming)));
        }
        s
    }

    /// "A minor 7th, 7th in the bass (3rd inversion)" style description.
    pub fn description(&self, naming: Naming) -> String {
        let mut d = format!("{} {}", self.root.name(naming), self.ty.name);
        if !self.tensions.is_empty() {
            let t: Vec<String> = self.tensions.iter().map(|i| i.formula_name()).collect();
            d.push_str(&format!(" with added {}", t.join(", ")));
        }
        if !self.missing.is_empty() {
            let m: Vec<String> = self.missing.iter().map(|i| i.formula_name()).collect();
            d.push_str(&format!(" (no {})", m.join(", ")));
        }
        d
    }

    /// Which chord tone is in the bass.
    pub fn inversion(&self) -> String {
        let Some(b) = self.bass else {
            return "root position (root in the bass)".into();
        };
        match self.interval_of(b.pc()) {
            Some(i) => match (i.number - 1) % 7 + 1 {
                3 => "3rd in the bass (1st inversion)".into(),
                5 => "5th in the bass (2nd inversion)".into(),
                7 => "7th in the bass (3rd inversion)".into(),
                _ => format!("{} in the bass", i.formula_name()),
            },
            None => "other note in the bass".into(),
        }
    }

    /// Interval of a played pitch class relative to this chord's root.
    pub fn interval_of(&self, pc: Pc) -> Option<Interval> {
        let off = (pc as i32 - self.root.pc() as i32).rem_euclid(12) as Pc;
        self.ty
            .intervals()
            .into_iter()
            .chain(self.tensions.iter().copied())
            .find(|i| i.pc_offset() == off)
    }

    /// Spelled note for a pitch class in this chord.
    pub fn spell(&self, pc: Pc) -> Note {
        self.interval_of(pc)
            .map(|i| i.apply(self.root))
            .unwrap_or_else(|| Note::from_pc(pc))
    }
}

pub struct Analysis {
    /// Best first. Empty for a single note.
    pub candidates: Vec<Candidate>,
    /// Played pitch classes, bass first, no duplicates.
    pub pcs: Vec<Pc>,
    /// Interval name for two different notes ("perfect 5th").
    pub dyad: Option<String>,
}

fn interval_name(semis: i32) -> String {
    const N: [&str; 12] = [
        "unison/octave",
        "minor 2nd",
        "major 2nd",
        "minor 3rd",
        "major 3rd",
        "perfect 4th",
        "tritone",
        "perfect 5th",
        "minor 6th",
        "major 6th",
        "minor 7th",
        "major 7th",
    ];
    if semis > 0 && semis % 12 == 0 {
        return if semis == 12 { "octave".into() } else { "octaves".into() };
    }
    let base = N[semis.rem_euclid(12) as usize];
    if semis > 12 && semis % 12 != 0 { format!("{base} (+ octave)") } else { base.to_string() }
}

/// Interval from the bass to every other note: ("E", "major 3rd").
pub fn bass_intervals(midis: &[i32], spell: impl Fn(Pc) -> Note, naming: Naming) -> Vec<(String, String)> {
    let mut m: Vec<i32> = midis.to_vec();
    m.sort();
    let Some(&bass) = m.first() else { return Vec::new() };
    m.iter()
        .skip(1)
        .map(|&x| {
            let n = spell(x.rem_euclid(12) as Pc).name(naming);
            (n, interval_name(x - bass))
        })
        .collect()
}

fn tension_interval(off: Pc) -> Interval {
    let s = match off {
        1 => "b9",
        2 => "9",
        3 => "#9",
        4 => "3",
        5 => "11",
        6 => "#11",
        7 => "5",
        8 => "b13",
        9 => "13",
        10 => "b7",
        _ => "7",
    };
    Interval::parse(s).unwrap()
}

/// Name a set of notes. `midis` in any order; the lowest is the bass.
pub fn analyse(midis: &[i32]) -> Analysis {
    let mut sorted: Vec<i32> = midis.to_vec();
    sorted.sort();
    let mut pcs: Vec<Pc> = Vec::new();
    for m in &sorted {
        let pc = m.rem_euclid(12) as Pc;
        if !pcs.contains(&pc) {
            pcs.push(pc);
        }
    }
    let dyad = (pcs.len() == 2).then(|| interval_name((pcs[1] as i32 - pcs[0] as i32).rem_euclid(12)));
    if pcs.len() < 2 {
        return Analysis { candidates: Vec::new(), pcs, dyad: None };
    }
    let bass = pcs[0];
    let mut out: Vec<Candidate> = Vec::new();

    for &r in &pcs {
        let root = Note::from_pc(r);
        let played: Vec<Pc> = pcs.iter().map(|&p| (p as i32 - r as i32).rem_euclid(12) as Pc).collect();
        let slash = (r != bass).then_some(bass);

        // exact matches
        for ty in all_types() {
            let ints = ty.intervals();
            let offs: Vec<Pc> = ints.iter().map(|i| i.pc_offset()).collect();
            if !played.iter().all(|p| offs.contains(p)) {
                continue;
            }
            let optional = ty.optional_intervals();
            let missing: Vec<Interval> = ints.iter().copied().filter(|i| !played.contains(&i.pc_offset())).collect();
            if missing.iter().any(|m| !optional.contains(m)) {
                continue;
            }
            let mut c = Candidate {
                root,
                ty,
                bass: None,
                missing,
                tensions: Vec::new(),
                score: 0.0,
                catalog_id: CHORD_TYPES.iter().any(|c| std::ptr::eq(c, ty)).then_some(ty.id),
            };
            c.bass = slash.map(|b| c.spell(b));
            c.score = c.missing.len() as f32 * 1.5 + if slash.is_some() { 1.0 } else { 0.0 } + ints.len() as f32 * 0.1;
            out.push(c);
        }

        // fallback: basic chord + tensions
        for id in FALLBACK_BASES {
            let ty = type_by_id(id).unwrap();
            let ints = ty.intervals();
            let optional = ty.optional_intervals();
            let offs: Vec<Pc> = ints.iter().map(|i| i.pc_offset()).collect();
            if !ints.iter().all(|i| played.contains(&i.pc_offset()) || optional.contains(i)) {
                continue;
            }
            let extras: Vec<Pc> = played.iter().copied().filter(|p| !offs.contains(p)).collect();
            if extras.is_empty() {
                continue; // already an exact match
            }
            // only real tensions (9ths, 11ths, 13ths) may be added; an extra 3rd,
            // 5th or 7th means another base chord describes the notes better
            if extras.iter().any(|e| matches!(e, 4 | 7 | 10 | 11)) {
                continue;
            }
            let missing: Vec<Interval> = ints.iter().copied().filter(|i| !played.contains(&i.pc_offset())).collect();
            let mut tensions: Vec<Interval> = extras.into_iter().map(tension_interval).collect();
            tensions.sort_by_key(|i| i.semitones());
            let mut c = Candidate {
                root,
                ty,
                bass: None,
                missing,
                tensions,
                score: 0.0,
                catalog_id: None,
            };
            c.bass = slash.map(|b| c.spell(b));
            c.score = c.missing.len() as f32 * 1.5
                + c.tensions.len() as f32 * 1.2
                + if slash.is_some() { 1.0 } else { 0.0 }
                + (ints.len() + c.tensions.len()) as f32 * 0.1
                + 0.3;
            out.push(c);
        }
    }

    out.sort_by(|a, b| a.score.total_cmp(&b.score));
    // drop duplicates that read the same (e.g. identical spellings from two tables)
    let mut seen = std::collections::HashSet::new();
    out.retain(|c| seen.insert(c.symbol(Naming::English)));
    // keep only names that are nearly as plausible as the best one
    if let Some(best) = out.first().map(|c| c.score) {
        out.retain(|c| c.score <= best + 2.0);
    }
    out.truncate(8);
    Analysis { candidates: out, pcs, dyad }
}

/// Catalog scales that contain every played note; scales on `prefer_root` first.
pub fn fitting_scales(pcs: &[Pc], prefer_root: Option<Pc>, limit: usize) -> Vec<Scale> {
    let mut v: Vec<(u32, Scale)> = Vec::new();
    for (ti, ty) in SCALE_TYPES.iter().enumerate() {
        for r in 0..12u8 {
            let s = Scale::new(Note::from_pc(r), ty);
            if pcs.iter().all(|p| s.contains(*p)) {
                let rank = if Some(r) == prefer_root { 0 } else { 1000 } + ti as u32 * 12 + r as u32;
                v.push((rank, s));
            }
        }
    }
    v.sort_by_key(|x| x.0);
    v.into_iter().take(limit).map(|x| x.1).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::fretboard::{Pos, midi_at};
    use crate::theory::tuning::E_STANDARD;
    use crate::theory::voicing::parse_frets;

    fn midis(tab: &str) -> Vec<i32> {
        parse_frets(tab)
            .unwrap()
            .iter()
            .enumerate()
            .filter_map(|(s, f)| f.map(|f| midi_at(&E_STANDARD, Pos::new(s as u8, f))))
            .collect()
    }

    fn best(tab: &str) -> String {
        analyse(&midis(tab)).candidates[0].symbol(Naming::English)
    }

    fn names(tab: &str) -> Vec<String> {
        analyse(&midis(tab)).candidates.iter().map(|c| c.symbol(Naming::English)).collect()
    }

    #[test]
    fn common_chords() {
        assert_eq!(best("x32010"), "C");
        assert_eq!(best("x02210"), "Am");
        assert_eq!(best("022000"), "Em");
        assert_eq!(best("020100"), "E7");
        assert_eq!(best("x32000"), "Cmaj7");
        assert_eq!(best("x02010"), "Am7");
        assert_eq!(best("07678x"), "E7#9");
        assert_eq!(best("3x0003"), "G");
        assert_eq!(best("xx0232"), "D");
        assert_eq!(best("133211"), "F");
    }

    #[test]
    fn no_silly_alternatives() {
        let n = names("x32010");
        assert!(!n.iter().any(|s| s.starts_with("C5(") || s.starts_with("Em(b6)")), "{n:?}");
        // Cmaj7 (x32017): far-fetched tension names are filtered out
        let n = names("x32017");
        assert_eq!(n[0], "Cmaj7");
        assert!(!n.iter().any(|s| s.starts_with("E5(") || s.starts_with("G6(")), "{n:?}");
    }

    #[test]
    fn missing_fifth() {
        let a = analyse(&midis("x3231x"));
        assert_eq!(a.candidates[0].symbol(Naming::English), "C7");
        assert_eq!(a.candidates[0].missing.len(), 1);
        assert!(a.candidates[0].description(Naming::English).contains("no 5"));
    }

    #[test]
    fn slash_chords() {
        let n = names("x32210");
        assert_eq!(n[0], "Am/C");
        assert!(n.contains(&"C6".to_string()), "{n:?}");
        let a = analyse(&midis("x32210"));
        assert!(a.candidates[0].inversion().contains("1st inversion"));
    }

    #[test]
    fn power_chords_and_dyads() {
        let a = analyse(&midis("x02xxx"));
        assert_eq!(a.dyad.as_deref(), Some("perfect 5th"));
        assert_eq!(a.candidates[0].symbol(Naming::English), "A5");
        let a = analyse(&midis("x02xxx")[..1]);
        assert!(a.candidates.is_empty());
    }

    #[test]
    fn diminished_seventh_is_symmetric() {
        // x2323x is B F A D: a half-diminished chord
        assert_eq!(best("x2323x"), "Bm7b5");
        // B D F Ab (B2 D3 F3 Ab3)
        let n: Vec<String> = analyse(&[47, 50, 53, 56]).candidates.iter().map(|c| c.symbol(Naming::English)).collect();
        let dims: Vec<&String> = n.iter().filter(|s| s.contains("dim7")).collect();
        assert_eq!(dims.len(), 4, "{n:?}");
        assert_eq!(n[0], "Bdim7");
    }

    #[test]
    fn fallback_names_tensions() {
        // C E G B F# → Cmaj7(#11)
        let a = analyse(&[48, 52, 55, 59, 66]);
        assert_eq!(a.candidates[0].symbol(Naming::English), "Cmaj7(#11)");
        // A C E Bb → Am(b9) style name exists
        let n: Vec<String> = analyse(&[45, 48, 52, 58]).candidates.iter().map(|c| c.symbol(Naming::English)).collect();
        assert!(n.iter().any(|s| s == "Am(b9)"), "{n:?}");
    }

    #[test]
    fn scales_that_fit() {
        let s = fitting_scales(&[0, 4, 7], Some(0), 50);
        assert!(s.iter().any(|x| x.ty.id == "major" && x.root.pc() == 0));
        // A minor pentatonic (A C D E G) contains C E G; E minor pentatonic has no C
        assert!(s.iter().any(|x| x.ty.id == "minor_pentatonic" && x.root.pc() == 9));
        assert!(!s.iter().any(|x| x.ty.id == "minor_pentatonic" && x.root.pc() == 4));
        assert_eq!(s[0].root.pc(), 0);
    }
}
