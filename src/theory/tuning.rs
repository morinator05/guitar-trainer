//! Guitar tunings. String index 0 is the LOWEST string (guitarists call it "string 6").

use super::note::{Letter::*, Pitch};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TuningId {
    #[default]
    EStandard,
    DStandard,
    DropD,
    DropC,
}

pub struct Tuning {
    pub id: TuningId,
    pub name: &'static str,
    /// Open-string pitches, low to high.
    pub strings: [Pitch; 6],
}

pub const E_STANDARD: Tuning = Tuning {
    id: TuningId::EStandard,
    name: "E standard (E A D G B E)",
    strings: [
        Pitch::new(E, 0, 2),
        Pitch::new(A, 0, 2),
        Pitch::new(D, 0, 3),
        Pitch::new(G, 0, 3),
        Pitch::new(B, 0, 3),
        Pitch::new(E, 0, 4),
    ],
};

pub const D_STANDARD: Tuning = Tuning {
    id: TuningId::DStandard,
    name: "D standard (D G C F A D)",
    strings: [
        Pitch::new(D, 0, 2),
        Pitch::new(G, 0, 2),
        Pitch::new(C, 0, 3),
        Pitch::new(F, 0, 3),
        Pitch::new(A, 0, 3),
        Pitch::new(D, 0, 4),
    ],
};

pub const DROP_D: Tuning = Tuning {
    id: TuningId::DropD,
    name: "Drop D (D A D G B E)",
    strings: [
        Pitch::new(D, 0, 2),
        Pitch::new(A, 0, 2),
        Pitch::new(D, 0, 3),
        Pitch::new(G, 0, 3),
        Pitch::new(B, 0, 3),
        Pitch::new(E, 0, 4),
    ],
};

pub const DROP_C: Tuning = Tuning {
    id: TuningId::DropC,
    name: "Drop C (C G C F A D)",
    strings: [
        Pitch::new(C, 0, 2),
        Pitch::new(G, 0, 2),
        Pitch::new(C, 0, 3),
        Pitch::new(F, 0, 3),
        Pitch::new(A, 0, 3),
        Pitch::new(D, 0, 4),
    ],
};

pub const ALL_TUNINGS: [&Tuning; 4] = [&E_STANDARD, &D_STANDARD, &DROP_D, &DROP_C];

impl TuningId {
    pub fn tuning(self) -> &'static Tuning {
        match self {
            TuningId::EStandard => &E_STANDARD,
            TuningId::DStandard => &D_STANDARD,
            TuningId::DropD => &DROP_D,
            TuningId::DropC => &DROP_C,
        }
    }
}

impl Tuning {
    pub const STRING_COUNT: usize = 6;

    pub fn open_midi(&self, string: usize) -> i32 {
        self.strings[string].midi()
    }

    /// Semitones this tuning sits below E standard on every string, if it is a pure
    /// transposition of E standard (D standard → 2). Drop tunings return None.
    pub fn transposition_from_e_standard(&self) -> Option<i32> {
        let d = E_STANDARD.open_midi(0) - self.open_midi(0);
        (0..6)
            .all(|s| E_STANDARD.open_midi(s) - self.open_midi(s) == d)
            .then_some(d)
    }

    /// Guitarist's string number: 6 = lowest.
    pub fn string_number(string: usize) -> usize {
        6 - string
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_strings() {
        assert_eq!(E_STANDARD.open_midi(0), 40);
        assert_eq!(E_STANDARD.open_midi(5), 64);
        assert_eq!(DROP_D.open_midi(0), 38);
        assert_eq!(DROP_C.open_midi(0), 36);
        assert_eq!(D_STANDARD.open_midi(5), 62);
    }

    #[test]
    fn transposition() {
        assert_eq!(E_STANDARD.transposition_from_e_standard(), Some(0));
        assert_eq!(D_STANDARD.transposition_from_e_standard(), Some(2));
        assert_eq!(DROP_D.transposition_from_e_standard(), None);
        assert_eq!(DROP_C.transposition_from_e_standard(), None);
    }
}
