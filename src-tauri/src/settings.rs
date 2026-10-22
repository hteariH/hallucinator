use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Sampling {
    pub temperature: f32,
    pub top_k: u32,
    pub top_p: f32,
}

impl Default for Sampling {
    fn default() -> Self {
        Self { temperature: 0.7, top_k: 40, top_p: 0.95 }
    }
}

impl Sampling {
    pub fn clamped(self) -> Self {
        Self {
            temperature: self.temperature.clamp(0.0, 2.0),
            top_k: self.top_k.clamp(1, 100),
            top_p: self.top_p.clamp(0.0, 1.0),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub active_model: Option<String>,
    pub sampling: Sampling,
}

pub fn load(path: &Path) -> Settings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, s: &Settings) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Write-then-rename so a crash never leaves half a file.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(s)?)?;
    std::fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(&dir.path().join("nope.json")), Settings::default());
    }

    #[test]
    fn corrupt_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        std::fs::write(&p, "{not json").unwrap();
        assert_eq!(load(&p), Settings::default());
    }

    #[test]
    fn roundtrip_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a").join("settings.json");
        let s = Settings {
            active_model: Some("gemma-3-1b".into()),
            sampling: Sampling { temperature: 1.2, top_k: 10, top_p: 0.5 },
        };
        save(&p, &s).unwrap();
        assert_eq!(load(&p), s);
    }

    #[test]
    fn partial_json_fills_defaults_and_uses_camel_case() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        std::fs::write(&p, r#"{"sampling":{"topK":5}}"#).unwrap();
        let s = load(&p);
        assert_eq!(s.sampling, Sampling { top_k: 5, ..Sampling::default() });
        assert_eq!(s.active_model, None);
    }

    #[test]
    fn clamped_limits_ranges() {
        let s = Sampling { temperature: 9.0, top_k: 0, top_p: -1.0 }.clamped();
        assert_eq!(s, Sampling { temperature: 2.0, top_k: 1, top_p: 0.0 });
        let s = Sampling { temperature: -1.0, top_k: 500, top_p: 3.0 }.clamped();
        assert_eq!(s, Sampling { temperature: 0.0, top_k: 100, top_p: 1.0 });
    }
}
