//! Notes, pitch classes, spelling and naming (English / German).

use serde::{Deserialize, Serialize};

/// Pitch class 0..12 (C = 0).
pub type Pc = u8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Letter {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

pub const LETTERS: [Letter; 7] = [
    Letter::C,
    Letter::D,
    Letter::E,
    Letter::F,
    Letter::G,
    Letter::A,
    Letter::B,
];

impl Letter {
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn from_index(i: usize) -> Letter {
        LETTERS[i % 7]
    }
    /// Pitch class of the natural note.
    pub fn natural_pc(self) -> i32 {
        [0, 2, 4, 5, 7, 9, 11][self.index()]
    }
    pub fn english(self) -> &'static str {
        ["C", "D", "E", "F", "G", "A", "B"][self.index()]
    }
}

/// A spelled note without octave, e.g. F#, Bb, E.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Note {
    pub letter: Letter,
    /// -2..=2 (bb, b, natural, #, x)
    pub acc: i8,
}

impl Note {
    pub const fn new(letter: Letter, acc: i8) -> Note {
        Note { letter, acc }
    }

    pub fn pc(self) -> Pc {
        (self.letter.natural_pc() + self.acc as i32).rem_euclid(12) as Pc
    }

    /// Default spelling of a pitch class when there is no key context.
    /// Uses the spellings guitarists see most often: C# Eb F# G# Bb.
    pub fn from_pc(pc: Pc) -> Note {
        use Letter::*;
        match pc % 12 {
            0 => Note::new(C, 0),
            1 => Note::new(C, 1),
            2 => Note::new(D, 0),
            3 => Note::new(E, -1),
            4 => Note::new(E, 0),
            5 => Note::new(F, 0),
            6 => Note::new(F, 1),
            7 => Note::new(G, 0),
            8 => Note::new(G, 1),
            9 => Note::new(A, 0),
            10 => Note::new(B, -1),
            _ => Note::new(B, 0),
        }
    }

    /// Spell `pc` using the given letter (accidental computed).
    pub fn with_letter(letter: Letter, pc: Pc) -> Note {
        let mut diff = (pc as i32 - letter.natural_pc()).rem_euclid(12);
        if diff > 6 {
            diff -= 12;
        }
        Note::new(letter, diff as i8)
    }

    /// Parse "C", "F#", "Bb", "Ebb", "Cx" (English names).
    pub fn parse(s: &str) -> Option<Note> {
        let mut chars = s.trim().chars();
        let letter = match chars.next()?.to_ascii_uppercase() {
            'C' => Letter::C,
            'D' => Letter::D,
            'E' => Letter::E,
            'F' => Letter::F,
            'G' => Letter::G,
            'A' => Letter::A,
            'B' => Letter::B,
            _ => return None,
        };
        let mut acc = 0i8;
        for c in chars {
            match c {
                '#' | '♯' => acc += 1,
                'b' | '♭' => acc -= 1,
                'x' => acc += 2,
                _ => return None,
            }
        }
        Some(Note::new(letter, acc))
    }

    pub fn name(self, style: Naming) -> String {
        style.note_name(self)
    }
}

/// A spelled pitch with octave (scientific pitch notation, C4 = middle C = MIDI 60).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pitch {
    pub note: Note,
    pub octave: i8,
}

impl Pitch {
    pub const fn new(letter: Letter, acc: i8, octave: i8) -> Pitch {
        Pitch {
            note: Note::new(letter, acc),
            octave,
        }
    }

    pub fn midi(self) -> i32 {
        (self.octave as i32 + 1) * 12 + self.note.letter.natural_pc() + self.note.acc as i32
    }

    /// Spell a MIDI number with a given note spelling (the octave follows the letter,
    /// so B#3 and C4 are both MIDI 60).
    pub fn from_midi_spelled(midi: i32, note: Note) -> Pitch {
        let base = note.letter.natural_pc() + note.acc as i32;
        let octave = (midi - base).div_euclid(12) - 1;
        Pitch { note, octave: octave as i8 }
    }

    pub fn from_midi(midi: i32) -> Pitch {
        Pitch::from_midi_spelled(midi, Note::from_pc(midi.rem_euclid(12) as Pc))
    }

    /// Diatonic step number (letter index + 7 * octave). Used to place notes on a staff.
    pub fn diatonic_step(self) -> i32 {
        self.octave as i32 * 7 + self.note.letter.index() as i32
    }

    pub fn name(self, style: Naming) -> String {
        format!("{}{}", style.note_name(self.note), self.octave)
    }
}

/// How note names are displayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Naming {
    /// C C# D Eb ... Bb B
    #[default]
    English,
    /// Like English, but B → H and Bb → B (German convention).
    GermanHB,
    /// Full German names: Cis, Es, Fis, As, B, H ...
    GermanFull,
}

impl Naming {
    pub const ALL: [Naming; 3] = [Naming::English, Naming::GermanHB, Naming::GermanFull];

    pub fn label(self) -> &'static str {
        match self {
            Naming::English => "English (B, Bb)",
            Naming::GermanHB => "German (H, B)",
            Naming::GermanFull => "German full (H, B, Cis, Es)",
        }
    }

    pub fn note_name(self, n: Note) -> String {
        match self {
            Naming::English => format!("{}{}", n.letter.english(), acc_symbols(n.acc)),
            Naming::GermanHB => {
                if n.letter == Letter::B {
                    match n.acc {
                        -1 => "B".to_string(),
                        0 => "H".to_string(),
                        a => format!("H{}", acc_symbols(a)),
                    }
                } else {
                    format!("{}{}", n.letter.english(), acc_symbols(n.acc))
                }
            }
            Naming::GermanFull => german_full(n),
        }
    }
}

fn acc_symbols(acc: i8) -> &'static str {
    match acc {
        -2 => "bb",
        -1 => "b",
        0 => "",
        1 => "#",
        2 => "x",
        _ => "?",
    }
}

fn german_full(n: Note) -> String {
    use Letter::*;
    let base = match n.letter {
        B => "H",
        l => l.english(),
    };
    match (n.letter, n.acc) {
        (_, 0) => base.to_string(),
        (B, -1) => "B".into(),
        (B, -2) => "Heses".into(),
        (E, -1) => "Es".into(),
        (E, -2) => "Eses".into(),
        (A, -1) => "As".into(),
        (A, -2) => "Ases".into(),
        (_, a) if a > 0 => format!("{}{}", base, "is".repeat(a as usize)),
        (_, a) => format!("{}{}", base, "es".repeat((-a) as usize)),
    }
}

/// All 12 pitch classes with a "C#/Db" style label for answer buttons.
pub fn pc_button_label(pc: Pc, style: Naming) -> String {
    let sharp = Note::with_letter(Letter::from_index(sharp_letter(pc)), pc);
    if sharp.acc == 0 {
        return style.note_name(sharp);
    }
    let flat = Note::with_letter(Letter::from_index(sharp_letter(pc) + 1), pc);
    format!("{}/{}", style.note_name(sharp), style.note_name(flat))
}

fn sharp_letter(pc: Pc) -> usize {
    // letter index whose natural or sharp gives pc
    [0, 0, 1, 1, 2, 3, 3, 4, 4, 5, 5, 6][pc as usize % 12]
}

/// Is the pitch class a natural note (white key)?
pub fn is_natural(pc: Pc) -> bool {
    matches!(pc % 12, 0 | 2 | 4 | 5 | 7 | 9 | 11)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_numbers() {
        assert_eq!(Pitch::new(Letter::C, 0, 4).midi(), 60);
        assert_eq!(Pitch::new(Letter::E, 0, 2).midi(), 40);
        assert_eq!(Pitch::new(Letter::B, 1, 3).midi(), 60);
        assert_eq!(Pitch::new(Letter::C, -1, 4).midi(), 59);
    }

    #[test]
    fn from_midi_roundtrip() {
        for m in 20..100 {
            assert_eq!(Pitch::from_midi(m).midi(), m);
        }
        let p = Pitch::from_midi_spelled(60, Note::new(Letter::B, 1));
        assert_eq!(p.octave, 3);
    }

    #[test]
    fn spelling_with_letter() {
        assert_eq!(Note::with_letter(Letter::G, 8), Note::new(Letter::G, 1));
        assert_eq!(Note::with_letter(Letter::A, 8), Note::new(Letter::A, -1));
        assert_eq!(Note::with_letter(Letter::B, 0), Note::new(Letter::B, 1));
        assert_eq!(Note::with_letter(Letter::F, 4), Note::new(Letter::F, -1));
    }

    #[test]
    fn german_names() {
        let bb = Note::new(Letter::B, -1);
        let b = Note::new(Letter::B, 0);
        assert_eq!(Naming::GermanHB.note_name(bb), "B");
        assert_eq!(Naming::GermanHB.note_name(b), "H");
        assert_eq!(Naming::English.note_name(bb), "Bb");
        assert_eq!(Naming::GermanFull.note_name(Note::new(Letter::C, 1)), "Cis");
        assert_eq!(Naming::GermanFull.note_name(Note::new(Letter::E, -1)), "Es");
        assert_eq!(Naming::GermanFull.note_name(Note::new(Letter::A, -1)), "As");
        assert_eq!(Naming::GermanFull.note_name(Note::new(Letter::D, -1)), "Des");
        assert_eq!(Naming::GermanFull.note_name(Note::new(Letter::F, 1)), "Fis");
    }

    #[test]
    fn parse_notes() {
        assert_eq!(Note::parse("F#"), Some(Note::new(Letter::F, 1)));
        assert_eq!(Note::parse("Bb"), Some(Note::new(Letter::B, -1)));
        assert_eq!(Note::parse("Ebb").unwrap().pc(), 2);
        assert_eq!(Note::parse("Q"), None);
    }

    #[test]
    fn button_labels() {
        assert_eq!(pc_button_label(1, Naming::English), "C#/Db");
        assert_eq!(pc_button_label(10, Naming::GermanHB), "A#/B");
        assert_eq!(pc_button_label(11, Naming::GermanHB), "H");
    }
}
