//! Chord voicings: generated from the tuning (works for any tuning, incl. drop tunings)
//! or loaded from the embedded chord database.

use super::chord::Chord;
use super::fretboard::{Pos, midi_at, pc_at};
use super::note::Pc;
use super::tuning::Tuning;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Generated,
    /// Hand-made shape from the chord database (E standard / transposed).
    Database,
}

#[derive(Clone, Debug)]
pub struct Voicing {
    /// Fret per string (index 0 = lowest string), None = muted.
    pub frets: [Option<u8>; 6],
    /// Finger per string: 0 = open or muted, 1 = index ... 4 = pinky.
    pub fingers: [u8; 6],
    /// Main barre: (fret, lowest string, highest string).
    pub barre: Option<(u8, u8, u8)>,
    pub source: Source,
    pub score: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Open,
    Barre,
    Power,
    Small,
    Movable,
}

impl Family {
    pub fn label(self) -> &'static str {
        match self {
            Family::Open => "open",
            Family::Barre => "barre",
            Family::Power => "power",
            Family::Small => "3-string",
            Family::Movable => "movable",
        }
    }
}

impl Voicing {
    pub fn played(&self) -> impl Iterator<Item = Pos> + '_ {
        self.frets
            .iter()
            .enumerate()
            .filter_map(|(s, f)| f.map(|f| Pos::new(s as u8, f)))
    }

    pub fn played_count(&self) -> usize {
        self.frets.iter().filter(|f| f.is_some()).count()
    }

    pub fn fretted(&self) -> impl Iterator<Item = u8> + '_ {
        self.frets.iter().filter_map(|f| *f).filter(|&f| f > 0)
    }

    pub fn min_fret(&self) -> u8 {
        self.fretted().min().unwrap_or(0)
    }

    pub fn max_fret(&self) -> u8 {
        self.fretted().max().unwrap_or(0)
    }

    pub fn has_open(&self) -> bool {
        self.frets.contains(&Some(0))
    }

    /// Lowest played string.
    pub fn bass_string(&self) -> Option<u8> {
        self.frets.iter().position(|f| f.is_some()).map(|s| s as u8)
    }

    pub fn bass_pos(&self) -> Option<Pos> {
        self.played().next()
    }

    pub fn midi_notes(&self, t: &Tuning) -> Vec<i32> {
        self.played().map(|p| midi_at(t, p)).collect()
    }

    /// "x32010" style; frets ≥ 10 are written with dashes: "x-10-12-12-12-x"
    pub fn tab_string(&self) -> String {
        let big = self.frets.iter().any(|f| matches!(f, Some(f) if *f >= 10));
        let parts: Vec<String> = self
            .frets
            .iter()
            .map(|f| match f {
                None => "x".to_string(),
                Some(f) => f.to_string(),
            })
            .collect();
        if big { parts.join("-") } else { parts.concat() }
    }

    /// Shape family; `power` = the chord is a power chord.
    pub fn family(&self, power: bool) -> Family {
        let n = self.played_count();
        if power {
            return Family::Power;
        }
        if self.has_open() && self.max_fret() <= 4 {
            return Family::Open;
        }
        if let Some((_, lo, hi)) = self.barre
            && hi - lo >= 3
        {
            return Family::Barre;
        }
        if n <= 3 {
            return Family::Small;
        }
        Family::Movable
    }

    /// Fret range used for a chord diagram: (first fret shown, number of frets).
    pub fn diagram_window(&self) -> (u8, u8) {
        let lo = self.min_fret();
        let hi = self.max_fret();
        if hi <= 4 {
            (1, 4.max(hi))
        } else {
            (lo, 4.max(hi - lo + 1))
        }
    }
}

/// Options for the generator.
#[derive(Clone, Copy, Debug)]
pub struct GenOptions {
    /// Highest fret any note may use.
    pub max_fret: u8,
    /// Max distance between lowest and highest fretted note.
    pub max_span: u8,
    /// Allow a bass note other than the root.
    pub inversions: bool,
}

impl Default for GenOptions {
    fn default() -> Self {
        GenOptions {
            max_fret: 17,
            max_span: 3,
            inversions: false,
        }
    }
}

/// Number of fingers needed and the main barre, or None if unplayable.
fn finger_plan(frets: &[Option<u8>; 6]) -> Option<([u8; 6], Option<(u8, u8, u8)>)> {
    let fretted: Vec<(usize, u8)> = frets
        .iter()
        .enumerate()
        .filter_map(|(s, f)| f.filter(|&f| f > 0).map(|f| (s, f)))
        .collect();
    let mut fingers = [0u8; 6];
    if fretted.is_empty() {
        return Some((fingers, None));
    }
    let base = fretted.iter().map(|x| x.1).min().unwrap();

    // Main barre with the index finger on the lowest fret, if that fret is used on
    // several strings and every string between them is fretted at >= base.
    let at_base: Vec<usize> = fretted.iter().filter(|x| x.1 == base).map(|x| x.0).collect();
    let mut barre = None;
    if at_base.len() >= 2 {
        let lo = *at_base.first().unwrap();
        let hi = *at_base.last().unwrap();
        let ok = (lo..=hi).all(|s| matches!(frets[s], Some(f) if f >= base));
        if ok {
            barre = Some((base, lo as u8, hi as u8));
        }
    }

    let mut rest: Vec<(usize, u8)> = match barre {
        Some((_, lo, hi)) => fretted
            .iter()
            .copied()
            .filter(|&(s, f)| !(f == base && (lo as usize..=hi as usize).contains(&s)))
            .collect(),
        None => fretted.clone(),
    };
    if barre.is_some() {
        for &(s, f) in &fretted {
            if f == base {
                fingers[s] = 1;
            }
        }
    }
    rest.sort_by_key(|&(s, f)| (f, s));

    // Group notes on the same fret on adjacent strings (partial barre, e.g. A-shape
    // ring-finger barre) when that is needed to stay within four fingers.
    let mut groups: Vec<Vec<(usize, u8)>> = Vec::new();
    for &(s, f) in &rest {
        groups.push(vec![(s, f)]);
    }
    let available = if barre.is_some() { 3 } else { 4 };
    if groups.len() > available {
        let mut merged: Vec<Vec<(usize, u8)>> = Vec::new();
        for g in groups {
            if let Some(last) = merged.last_mut() {
                let (ls, lf) = *last.last().unwrap();
                let (s, f) = g[0];
                // same fret, adjacent string, and no lower-fret note on strings between
                if f == lf && s == ls + 1 {
                    last.push((s, f));
                    continue;
                }
            }
            merged.push(g);
        }
        groups = merged;
    }
    if groups.len() > available {
        return None;
    }
    let mut next = if barre.is_some() { 2 } else { 1 };
    for g in &groups {
        let f = g[0].1;
        let want = (1 + f - base).max(next).min(4);
        for &(s, _) in g {
            fingers[s] = want;
        }
        next = (want + 1).min(4);
    }
    Some((fingers, barre))
}

/// Check a voicing against a chord. Returns a list of problems (empty = fine).
pub fn check(t: &Tuning, chord: &Chord, frets: &[Option<u8>; 6], inversions: bool) -> Vec<String> {
    let mut problems = Vec::new();
    let pcs = chord.pcs();
    let played: Vec<Pos> = frets
        .iter()
        .enumerate()
        .filter_map(|(s, f)| f.map(|f| Pos::new(s as u8, f)))
        .collect();
    if played.is_empty() {
        problems.push("no strings played".into());
        return problems;
    }
    for p in &played {
        let pc = pc_at(t, *p);
        if !pcs.contains(&pc) {
            problems.push(format!("string {} fret {} is not a chord tone", 6 - p.string, p.fret));
        }
    }
    let present: Vec<Pc> = played.iter().map(|p| pc_at(t, *p)).collect();
    for r in chord.required_pcs() {
        if !present.contains(&r) {
            problems.push(format!("missing chord tone {}", chord.spell(r).name(Default::default())));
        }
    }
    if !inversions && pc_at(t, played[0]) != chord.root_pc() {
        problems.push("bass note is not the root".into());
    }
    problems
}

/// Generate playable voicings, best first.
pub fn generate(t: &Tuning, chord: &Chord, opt: GenOptions) -> Vec<Voicing> {
    let pcs = chord.pcs();
    let required = chord.required_pcs();
    let root = chord.root_pc();
    let power = chord.ty.is_power_chord();
    let min_strings = if power { 2 } else { 3 };
    let max_strings = if power { 3 } else { 6 };

    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();

    for w in 0..=opt.max_fret.saturating_sub(opt.max_span) {
        // candidate frets per string
        let mut options: Vec<Vec<Option<u8>>> = Vec::with_capacity(6);
        for s in 0..6u8 {
            let mut v = vec![None];
            if pcs.contains(&pc_at(t, Pos::new(s, 0))) {
                v.push(Some(0));
            }
            let lo = w.max(1);
            for f in lo..=(w + opt.max_span).min(opt.max_fret) {
                if pcs.contains(&pc_at(t, Pos::new(s, f))) {
                    v.push(Some(f));
                }
            }
            options.push(v);
        }
        let mut cur = [None; 6];
        enumerate(&options, 0, &mut cur, &mut |frets| {
            if !seen.insert(*frets) {
                return;
            }
            if let Some(v) = evaluate(t, frets, &pcs, &required, root, power, opt, min_strings, max_strings) {
                out.push(v);
            }
        });
    }

    prune_subsets(&mut out);
    out.sort_by(|a, b| a.score.total_cmp(&b.score));
    out
}

fn enumerate(
    options: &[Vec<Option<u8>>],
    s: usize,
    cur: &mut [Option<u8>; 6],
    f: &mut dyn FnMut(&[Option<u8>; 6]),
) {
    if s == 6 {
        f(cur);
        return;
    }
    for &o in &options[s] {
        cur[s] = o;
        enumerate(options, s + 1, cur, f);
    }
    cur[s] = None;
}

#[allow(clippy::too_many_arguments)]
fn evaluate(
    t: &Tuning,
    frets: &[Option<u8>; 6],
    pcs: &[Pc],
    required: &[Pc],
    root: Pc,
    power: bool,
    opt: GenOptions,
    min_strings: usize,
    max_strings: usize,
) -> Option<Voicing> {
    let played: Vec<usize> = (0..6).filter(|&s| frets[s].is_some()).collect();
    let n = played.len();
    if n < min_strings || n > max_strings {
        return None;
    }
    let lo_s = played[0];
    let hi_s = *played.last().unwrap();
    let interior_mutes = (lo_s..=hi_s).filter(|&s| frets[s].is_none()).count();
    if interior_mutes > 1 || (power && interior_mutes > 0) {
        return None;
    }
    let bass_pc = pc_at(t, Pos::new(lo_s as u8, frets[lo_s].unwrap()));
    if !opt.inversions && bass_pc != root {
        return None;
    }
    let present: Vec<Pc> = played
        .iter()
        .map(|&s| pc_at(t, Pos::new(s as u8, frets[s].unwrap())))
        .collect();
    if !required.iter().all(|r| present.contains(r)) {
        return None;
    }
    debug_assert!(present.iter().all(|p| pcs.contains(p)));

    let fretted: Vec<u8> = frets.iter().filter_map(|f| *f).filter(|&f| f > 0).collect();
    let (min_f, max_f) = (
        fretted.iter().copied().min().unwrap_or(0),
        fretted.iter().copied().max().unwrap_or(0),
    );
    if max_f - min_f > opt.max_span {
        return None;
    }
    let has_open = frets.contains(&Some(0));
    // Open strings: fine in open position (all fretted notes <= fret 4).
    // Higher up the neck only an open BASS string is allowed (e.g. 076780, metal chugs).
    if has_open && max_f > 4 {
        let other_open = played.iter().skip(1).any(|&s| frets[s] == Some(0));
        if other_open {
            return None;
        }
    }
    if power {
        // power chord: root, 5th (and octave) on adjacent strings, no open string
        // mixed with fretted notes except an open bass.
        let bass_midi = midi_at(t, Pos::new(lo_s as u8, frets[lo_s].unwrap()));
        let ok = played.iter().all(|&s| {
            let m = midi_at(t, Pos::new(s as u8, frets[s].unwrap()));
            m >= bass_midi && m - bass_midi <= 12
        });
        if !ok {
            return None;
        }
    }

    let (fingers, barre) = finger_plan(frets)?;
    let finger_count = {
        let mut f: Vec<u8> = fingers.iter().copied().filter(|&x| x > 0).collect();
        f.sort_unstable();
        f.dedup();
        f.len()
    };

    let mut score = 0.0f32;
    score += finger_count as f32 * 0.8;
    score += (max_f.saturating_sub(min_f)) as f32 * 0.6;
    score += interior_mutes as f32 * 2.0;
    score += (6 - n) as f32 * 0.35;
    score += min_f as f32 * 0.04;
    if has_open && min_f > 4 {
        score += 1.5;
    }
    if barre.is_some() {
        score += 0.4;
    }

    Some(Voicing {
        frets: *frets,
        fingers,
        barre,
        source: Source::Generated,
        score,
    })
}

/// Remove voicings that are the same shape as another voicing with fewer strings
/// when the larger one costs no extra fingers (e.g. drop x3201x because x32010 exists).
fn prune_subsets(v: &mut Vec<Voicing>) {
    let keep: Vec<bool> = (0..v.len())
        .map(|i| {
            let a = &v[i];
            let a_fingers = a.fingers.iter().filter(|&&f| f > 0).count();
            !v.iter().enumerate().any(|(j, b)| {
                if i == j || b.played_count() <= a.played_count() {
                    return false;
                }
                let same = (0..6).all(|s| a.frets[s].is_none() || a.frets[s] == b.frets[s]);
                let b_fingers = b.fingers.iter().filter(|&&f| f > 0).count();
                // keep movable shapes even if an open-string version exists
                let keeps_movable = !a.has_open() && b.has_open();
                same && b_fingers <= a_fingers && !keeps_movable
            })
        })
        .collect();
    let mut it = keep.iter();
    v.retain(|_| *it.next().unwrap());
}

/// Recompute fingers/barre for externally supplied frets (e.g. transposed DB shapes).
pub fn from_frets(frets: [Option<u8>; 6], source: Source) -> Option<Voicing> {
    let (fingers, barre) = finger_plan(&frets)?;
    Some(Voicing {
        frets,
        fingers,
        barre,
        source,
        score: 0.0,
    })
}

/// Parse "x32010" or "x-10-12-12-12-x".
#[cfg(test)]
pub fn parse_frets(s: &str) -> Option<[Option<u8>; 6]> {
    let parts: Vec<&str> = if s.contains('-') {
        s.split('-').collect()
    } else {
        s.split("").filter(|p| !p.is_empty()).collect()
    };
    if parts.len() != 6 {
        return None;
    }
    let mut out = [None; 6];
    for (i, p) in parts.iter().enumerate() {
        out[i] = match *p {
            "x" | "X" => None,
            n => Some(n.parse().ok()?),
        };
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::chord::{CHORD_TYPES, ChordType};
    use crate::theory::note::Note;
    use crate::theory::tuning::*;

    fn chord(root: &str, id: &str) -> Chord {
        Chord::new(Note::parse(root).unwrap(), ChordType::by_id(id).unwrap())
    }

    fn tabs(t: &Tuning, c: &Chord) -> Vec<String> {
        generate(t, c, GenOptions::default())
            .iter()
            .map(|v| v.tab_string())
            .collect()
    }

    #[test]
    fn c_major_contains_classics() {
        let t = tabs(&E_STANDARD, &chord("C", "maj"));
        assert!(t.contains(&"x32010".to_string()), "{t:?}");
        assert!(t.contains(&"x35553".to_string()), "{t:?}");
        assert!(t.contains(&"8-10-10-9-8-8".to_string()), "{t:?}");
    }

    #[test]
    fn open_chords() {
        assert!(tabs(&E_STANDARD, &chord("G", "maj")).contains(&"320003".to_string()));
        assert!(tabs(&E_STANDARD, &chord("E", "min")).contains(&"022000".to_string()));
        assert!(tabs(&E_STANDARD, &chord("A", "min")).contains(&"x02210".to_string()));
        assert!(tabs(&E_STANDARD, &chord("F", "maj")).contains(&"133211".to_string()));
        assert!(tabs(&E_STANDARD, &chord("E", "7")).contains(&"020100".to_string()));
    }

    #[test]
    fn hendrix_chord() {
        let t = tabs(&E_STANDARD, &chord("E", "7#9"));
        assert!(t.contains(&"07678x".to_string()), "{t:?}");
        assert!(t.contains(&"x7678x".to_string()), "{t:?}");
    }

    #[test]
    fn power_chords_drop_d() {
        let t = tabs(&DROP_D, &chord("D", "5"));
        assert!(t.contains(&"000xxx".to_string()), "{t:?}");
        let f = tabs(&DROP_D, &chord("F", "5"));
        assert!(f.contains(&"333xxx".to_string()), "{f:?}");
        let e = tabs(&E_STANDARD, &chord("A", "5"));
        assert!(e.contains(&"577xxx".to_string()), "{e:?}");
        assert!(e.contains(&"x02xxx".to_string()), "{e:?}");
    }

    #[test]
    fn generated_voicings_pass_check() {
        for t in ALL_TUNINGS {
            for ty in CHORD_TYPES {
                for root in ["C", "E", "F#", "Bb"] {
                    let c = Chord::new(Note::parse(root).unwrap(), ty);
                    let v = generate(t, &c, GenOptions::default());
                    assert!(!v.is_empty(), "no voicing for {root}{} in {}", ty.suffix, t.name);
                    for x in &v {
                        let p = check(t, &c, &x.frets, false);
                        assert!(p.is_empty(), "{} {:?}", x.tab_string(), p);
                    }
                }
            }
        }
    }

    #[test]
    fn fingers() {
        let v = from_frets(parse_frets("133211").unwrap(), Source::Generated).unwrap();
        assert_eq!(v.barre, Some((1, 0, 5)));
        assert_eq!(v.fingers, [1, 3, 4, 2, 1, 1]);
        let c = from_frets(parse_frets("x32010").unwrap(), Source::Generated).unwrap();
        assert_eq!(c.fingers, [0, 3, 2, 0, 1, 0]);
        assert!(from_frets(parse_frets("1x3x5x").unwrap(), Source::Generated).is_some());
        // five separate fingers: impossible
        assert!(from_frets(parse_frets("x12345").unwrap(), Source::Generated).is_none());
    }

    #[test]
    fn barre_finger_count_scores() {
        // F barre (133211) uses 4 fingers; its score must not count the barre finger 3 times
        let f = generate(&E_STANDARD, &chord("F", "maj"), GenOptions::default());
        let v = f.iter().find(|v| v.tab_string() == "133211").unwrap();
        // score = fingers*0.8 + span*0.6 + barre 0.4 + min_fret*0.04 → < 5 with 4 fingers
        assert!(v.score < 5.0, "score {}", v.score);
    }

    #[test]
    fn parse_tab() {
        assert_eq!(parse_frets("x32010").unwrap()[1], Some(3));
        assert_eq!(parse_frets("x-10-12-12-12-x").unwrap()[2], Some(12));
    }
}
