//! Progress tracking with Leitner-box spaced repetition.
//!
//! Every drill question has a key (e.g. "fb:name:s0f5"). Correct answers move the
//! item up a box (seen less often), wrong answers send it back to box 0.

use crate::util::{Rng, today, unix_now};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const BOXES: u8 = 5;
/// Days until an item in box n is due again.
const INTERVAL_DAYS: [u64; BOXES as usize] = [0, 1, 3, 7, 14];

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ItemStats {
    pub level: u8,
    pub attempts: u32,
    pub correct: u32,
    /// Exponential moving average of answer time in milliseconds.
    pub avg_ms: f32,
    pub last: u64,
    pub due: u64,
}

impl ItemStats {
    pub fn accuracy(&self) -> f32 {
        if self.attempts == 0 {
            0.0
        } else {
            self.correct as f32 / self.attempts as f32
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DayLog {
    pub day: u32,
    pub attempts: u32,
    pub correct: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    pub items: HashMap<String, ItemStats>,
    pub days: Vec<DayLog>,
    #[serde(skip)]
    pub dirty: bool,
}

impl Progress {
    pub fn record(&mut self, key: &str, correct: bool, ms: Option<f32>) {
        let now = unix_now();
        let it = self.items.entry(key.to_string()).or_default();
        it.level = it.level.min(BOXES - 1); // guard against corrupt files
        let was_due = it.due <= now;
        it.attempts += 1;
        if correct {
            it.correct += 1;
            // Only a correct answer at (or after) the due date moves the item up a box;
            // answering it again and again in one sitting does not.
            if was_due {
                it.level = (it.level + 1).min(BOXES - 1);
            }
        } else {
            it.level = 0;
        }
        if let Some(ms) = ms {
            let ms = ms.min(30_000.0);
            it.avg_ms = if it.attempts == 1 || it.avg_ms == 0.0 {
                ms
            } else {
                it.avg_ms * 0.7 + ms * 0.3
            };
        }
        it.last = now;
        it.due = now + INTERVAL_DAYS[it.level as usize] * 86_400;

        let d = today();
        if self.days.last().map(|l| l.day) != Some(d) {
            self.days.push(DayLog { day: d, attempts: 0, correct: 0 });
        }
        let l = self.days.last_mut().unwrap();
        l.attempts += 1;
        if correct {
            l.correct += 1;
        }
        self.dirty = true;
    }

    pub fn get(&self, key: &str) -> Option<&ItemStats> {
        self.items.get(key)
    }

    /// How urgently an item should be practised (higher = sooner).
    pub fn weight(&self, key: &str) -> f32 {
        match self.items.get(key) {
            None => 1.5,
            Some(it) => {
                let lvl = (BOXES - it.level.min(BOXES - 1)) as f32; // 1..5
                let mut w = lvl * lvl / 5.0;
                if it.due <= unix_now() {
                    w *= 2.0;
                }
                // slow answers count as weak too
                if it.avg_ms > 4000.0 {
                    w *= 1.5;
                }
                w.max(0.05)
            }
        }
    }

    /// Pick one of the candidate keys, weak/due/new items more likely.
    pub fn pick(&self, rng: &mut Rng, keys: &[String]) -> usize {
        let w: Vec<f32> = keys.iter().map(|k| self.weight(k)).collect();
        rng.weighted(&w)
    }

    pub fn today(&self) -> DayLog {
        let d = today();
        self.days
            .iter()
            .find(|l| l.day == d)
            .cloned()
            .unwrap_or(DayLog { day: d, attempts: 0, correct: 0 })
    }

    /// (attempts, accuracy) for all items whose key starts with `prefix`.
    pub fn summary(&self, prefix: &str) -> (u32, f32, usize) {
        let mut a = 0;
        let mut c = 0;
        let mut n = 0;
        for (k, it) in &self.items {
            if k.starts_with(prefix) {
                a += it.attempts;
                c += it.correct;
                n += 1;
            }
        }
        (a, if a == 0 { 0.0 } else { c as f32 / a as f32 }, n)
    }

    /// Consecutive days with practice, ending today or yesterday.
    pub fn streak(&self) -> u32 {
        let mut d = today();
        let days: std::collections::HashSet<u32> =
            self.days.iter().filter(|l| l.attempts > 0).map(|l| l.day).collect();
        if !days.contains(&d) {
            d = d.saturating_sub(1);
        }
        let mut n = 0;
        while days.contains(&d) {
            n += 1;
            d = d.saturating_sub(1);
        }
        n
    }

    /// The weakest practised items (lowest box, then accuracy).
    pub fn weakest(&self, n: usize) -> Vec<(&String, &ItemStats)> {
        let mut v: Vec<_> = self.items.iter().filter(|(_, it)| it.attempts > 0).collect();
        v.sort_by(|a, b| {
            (a.1.level, a.1.accuracy())
                .partial_cmp(&(b.1.level, b.1.accuracy()))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        v.truncate(n);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leitner() {
        let mut p = Progress::default();
        p.record("a", true, Some(1000.0));
        assert_eq!(p.get("a").unwrap().level, 1);
        // not due again until tomorrow: a second correct answer does not promote
        p.record("a", true, None);
        assert_eq!(p.get("a").unwrap().level, 1);
        p.items.get_mut("a").unwrap().due = 0; // pretend it is due
        p.record("a", true, None);
        assert_eq!(p.get("a").unwrap().level, 2);
        p.record("a", false, None);
        assert_eq!(p.get("a").unwrap().level, 0);
        assert_eq!(p.get("a").unwrap().attempts, 4);
        assert!(p.weight("a") > p.weight("missing") * 0.5);
        assert_eq!(p.today().attempts, 4);
        assert_eq!(p.streak(), 1);
    }

    #[test]
    fn corrupt_level_is_clamped() {
        let mut p = Progress::default();
        p.items.insert("x".into(), ItemStats { level: 200, ..Default::default() });
        let _ = p.weight("x");
        p.record("x", true, None);
        assert!(p.get("x").unwrap().level < BOXES);
    }
}
