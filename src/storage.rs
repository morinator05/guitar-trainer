//! Settings and progress as JSON files in ~/.local/share/guitar-trainer/ (local only).

use serde::{Serialize, de::DeserializeOwned};
use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("guitar-trainer")
}

pub fn load<T: DeserializeOwned + Default>(name: &str) -> T {
    let path = data_dir().join(name);
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            eprintln!("could not parse {}: {e} - using defaults", path.display());
            // keep the broken file for inspection
            let _ = std::fs::rename(&path, path.with_extension("json.broken"));
            T::default()
        }),
        Err(_) => T::default(),
    }
}

pub fn save<T: Serialize>(name: &str, value: &T) {
    let dir = data_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("could not create {}: {e}", dir.display());
        return;
    }
    let path = dir.join(name);
    let tmp = path.with_extension("json.tmp");
    match serde_json::to_string_pretty(value) {
        Ok(s) => {
            if std::fs::write(&tmp, s).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
        }
        Err(e) => eprintln!("could not serialise {name}: {e}"),
    }
}
