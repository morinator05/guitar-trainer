//! Intervals written the way musicians write chord/scale formulas: 1 b3 5 b7 #9 ...

use super::note::{Letter, Note, Pc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Interval {
    /// Scale-degree number: 1..=13
    pub number: u8,
    /// Alteration relative to the major/perfect interval (b = -1, # = +1).
    pub alter: i8,
}

const MAJOR_SEMIS: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];

impl Interval {
    /// Parse "1", "b3", "#4", "bb7", "#9", "13".
    pub fn parse(s: &str) -> Option<Interval> {
        let s = s.trim();
        let mut alter = 0i8;
        let mut rest = s;
        while let Some(c) = rest.chars().next() {
            match c {
                'b' => alter -= 1,
                '#' => alter += 1,
                _ => break,
            }
            rest = &rest[1..];
        }
        let number: u8 = rest.parse().ok()?;
        if number == 0 || number > 15 {
            return None;
        }
        Some(Interval { number, alter })
    }

    /// Parse a whitespace separated formula.
    pub fn parse_formula(s: &str) -> Vec<Interval> {
        s.split_whitespace()
            .map(|t| Interval::parse(t).unwrap_or_else(|| panic!("bad interval {t:?}")))
            .collect()
    }

    pub fn semitones(self) -> i32 {
        let idx = (self.number as usize - 1) % 7;
        let oct = (self.number as i32 - 1) / 7;
        MAJOR_SEMIS[idx] + 12 * oct + self.alter as i32
    }

    /// Semitones reduced to one octave.
    pub fn pc_offset(self) -> Pc {
        self.semitones().rem_euclid(12) as Pc
    }

    /// Spell the note this interval reaches from `root`.
    pub fn apply(self, root: Note) -> Note {
        let letter = Letter::from_index(root.letter.index() + (self.number as usize - 1));
        let pc = (root.pc() as i32 + self.semitones()).rem_euclid(12) as Pc;
        Note::with_letter(letter, pc)
    }

    /// Label shown to the user (dots, buttons, formulas): 1, ♭3, 3, 5, ♭7 ...
    /// Real ♭/♯ signs, so "b3" can't be confused with the German note B.
    pub fn label(self) -> String {
        self.formula_name().replace('b', "♭").replace('#', "♯")
    }

    /// Label in formula style: 1, b3, 5 ...
    pub fn formula_name(self) -> String {
        let acc = match self.alter {
            -2 => "bb",
            -1 => "b",
            0 => "",
            1 => "#",
            2 => "##",
            _ => "?",
        };
        format!("{acc}{}", self.number)
    }

    /// Simple-interval name in German/English school terms (for explanations).
    pub fn long_name(self) -> &'static str {
        match (((self.number - 1) % 7) + 1, self.alter) {
            (1, 0) => "root",
            (2, -1) => "minor 2nd",
            (2, 0) => "major 2nd",
            (2, 1) => "augmented 2nd",
            (3, -1) => "minor 3rd",
            (3, 0) => "major 3rd",
            (4, 0) => "perfect 4th",
            (4, 1) => "augmented 4th (tritone)",
            (5, -1) => "diminished 5th (tritone)",
            (5, 0) => "perfect 5th",
            (5, 1) => "augmented 5th",
            (6, -1) => "minor 6th",
            (6, 0) => "major 6th",
            (7, -2) => "diminished 7th",
            (7, -1) => "minor 7th",
            (7, 0) => "major 7th",
            _ => "",
        }
    }
}

/// Fixed colour (sRGB) per semitone distance from the root, so that e.g. every
/// minor third looks the same everywhere in the app.
pub fn interval_color(semis_from_root: Pc) -> [u8; 3] {
    match semis_from_root % 12 {
        // Pairs that look alike with red-green colour blindness (b2/b3, 3/b3,
        // 4/b7) differ in lightness too. The root is also drawn as a square.
        0 => [224, 69, 63],    // root: signal red (never used as UI accent)
        1 => [92, 84, 196],    // b2/b9: indigo (darker than b3)
        2 => [42, 161, 152],   // 2/9: teal
        3 => [45, 116, 196],   // b3: blue (darker than 3)
        4 => [79, 216, 232],   // 3: light cyan
        5 => [104, 120, 20],   // 4/11: dark olive
        6 => [211, 54, 130],   // b5/#4: magenta (blue note)
        7 => [214, 120, 20],   // 5: deep orange (darker than b7)
        8 => [210, 168, 110],  // b6/#5: sand (visible on rosewood)
        9 => [100, 180, 100],  // 6/13: green
        10 => [245, 222, 120], // b7: light yellow
        _ => [236, 236, 150],  // 7: pale yellow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::note::Letter::*;

    #[test]
    fn semitones() {
        assert_eq!(Interval::parse("b3").unwrap().semitones(), 3);
        assert_eq!(Interval::parse("#9").unwrap().semitones(), 15);
        assert_eq!(Interval::parse("13").unwrap().semitones(), 21);
        assert_eq!(Interval::parse("bb7").unwrap().semitones(), 9);
        assert_eq!(Interval::parse("b5").unwrap().semitones(), 6);
    }

    #[test]
    fn spelling() {
        let a = Note::new(A, 0);
        let notes: Vec<Note> = Interval::parse_formula("1 2 b3 4 5 b6 7")
            .into_iter()
            .map(|i| i.apply(a))
            .collect();
        let names: Vec<String> = notes
            .iter()
            .map(|n| n.name(crate::theory::note::Naming::English))
            .collect();
        assert_eq!(names, ["A", "B", "C", "D", "E", "F", "G#"]);

        let e = Note::new(E, 0);
        assert_eq!(Interval::parse("#9").unwrap().apply(e), Note::new(F, 2));
        let fs = Note::new(F, 1);
        assert_eq!(Interval::parse("b3").unwrap().apply(fs), Note::new(A, 0));
    }

    #[test]
    fn labels() {
        assert_eq!(Interval::parse("1").unwrap().label(), "1");
        assert_eq!(Interval::parse("b7").unwrap().label(), "♭7");
        assert_eq!(Interval::parse("#9").unwrap().formula_name(), "#9");
    }
}
