use crate::theory::note::Naming;
use crate::theory::tuning::{Tuning, TuningId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LabelMode {
    #[default]
    Notes,
    Intervals,
    None,
}

impl LabelMode {
    pub const ALL: [LabelMode; 3] = [LabelMode::Notes, LabelMode::Intervals, LabelMode::None];
    pub fn label(self) -> &'static str {
        match self {
            LabelMode::Notes => "Note names",
            LabelMode::Intervals => "Intervals",
            LabelMode::None => "No labels",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub tuning: TuningId,
    pub naming: Naming,
    pub labels: LabelMode,
    pub frets: u8,
    pub left_handed: bool,
    pub sound: bool,
    pub volume: f32,
    pub ui_scale: f32,
    pub dark: bool,
    pub bpm: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            tuning: TuningId::EStandard,
            naming: Naming::English,
            labels: LabelMode::Notes,
            frets: 22,
            left_handed: false,
            sound: true,
            volume: 0.7,
            ui_scale: 1.2,
            dark: true,
            bpm: 80.0,
        }
    }
}

impl Settings {
    pub fn tuning(&self) -> &'static Tuning {
        self.tuning.tuning()
    }
}
