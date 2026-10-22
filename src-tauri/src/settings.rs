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

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Personality {
    #[default]
    Plain,
    Overconfident,
    Anxious,
    Gaslighting,
}

// An unknown value (a newer or hand-edited file) must not reset every other setting.
fn lenient_personality<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Personality, D::Error> {
    Ok(serde_json::from_value(serde_json::Value::deserialize(d)?).unwrap_or_default())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub active_model: Option<String>,
    pub sampling: Sampling,
    #[serde(deserialize_with = "lenient_personality")]
    pub personality: Personality,
}

pub fn load(path: &Path) -> Settings {
    let mut s: Settings = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    s.sampling = s.sampling.clamped();
    s
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
            personality: Personality::Plain,
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
    fn load_clamps_out_of_range_sampling() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        std::fs::write(&p, r#"{"sampling":{"temperature":9,"topK":0,"topP":-2}}"#).unwrap();
        assert_eq!(load(&p).sampling, Sampling { temperature: 2.0, top_k: 1, top_p: 0.0 });
    }

    #[test]
    fn clamped_limits_ranges() {
        let s = Sampling { temperature: 9.0, top_k: 0, top_p: -1.0 }.clamped();
        assert_eq!(s, Sampling { temperature: 2.0, top_k: 1, top_p: 0.0 });
        let s = Sampling { temperature: -1.0, top_k: 500, top_p: 3.0 }.clamped();
        assert_eq!(s, Sampling { temperature: 0.0, top_k: 100, top_p: 1.0 });
    }

    #[test]
    fn personality_roundtrips_and_defaults_to_plain() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let s = Settings { personality: Personality::Gaslighting, ..Settings::default() };
        save(&p, &s).unwrap();
        assert_eq!(load(&p).personality, Personality::Gaslighting);
        assert_eq!(Settings::default().personality, Personality::Plain);
    }

    #[test]
    fn v01_file_without_personality_loads_plain() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        std::fs::write(&p, r#"{"activeModel":"gemma-3-1b","sampling":{"topK":5}}"#).unwrap();
        let s = load(&p);
        assert_eq!(s.personality, Personality::Plain);
        assert_eq!(s.active_model.as_deref(), Some("gemma-3-1b"));
    }

    #[test]
    fn unknown_personality_loads_plain_and_keeps_other_fields() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        std::fs::write(&p, r#"{"activeModel":"gemma-3-1b","personality":"sarcastic"}"#).unwrap();
        let s = load(&p);
        assert_eq!(s.personality, Personality::Plain);
        assert_eq!(s.active_model.as_deref(), Some("gemma-3-1b"));
    }

    #[test]
    fn personality_uses_camel_case_names() {
        assert_eq!(serde_json::to_string(&Personality::Overconfident).unwrap(), r#""overconfident""#);
    }
}
