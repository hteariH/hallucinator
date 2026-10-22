use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub file: &'static str,
    pub url: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

pub const CATALOG: [ModelSpec; 2] = [
    ModelSpec {
        id: "gemma-3-4b",
        name: "Gemma 3 4B",
        file: "gemma-3-4b-it-Q4_K_M.gguf",
        url: "https://huggingface.co/ggml-org/gemma-3-4b-it-GGUF/resolve/main/gemma-3-4b-it-Q4_K_M.gguf",
        size: 2_489_757_856,
        sha256: "882e8d2db44dc554fb0ea5077cb7e4bc49e7342a1f0da57901c0802ea21a0863",
    },
    ModelSpec {
        id: "gemma-3-1b",
        name: "Gemma 3 1B",
        file: "gemma-3-1b-it-Q4_K_M.gguf",
        url: "https://huggingface.co/ggml-org/gemma-3-1b-it-GGUF/resolve/main/gemma-3-1b-it-Q4_K_M.gguf",
        size: 806_058_240,
        sha256: "8ccc5cd1f1b3602548715ae25a66ed73fd5dc68a210412eea643eb20eb75a135",
    },
];

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub downloaded: bool,
}

pub fn find(id: &str) -> Option<&'static ModelSpec> {
    CATALOG.iter().find(|m| m.id == id)
}

pub fn model_path(dir: &Path, spec: &ModelSpec) -> PathBuf {
    dir.join(spec.file)
}

pub fn list(dir: &Path) -> Vec<ModelInfo> {
    CATALOG
        .iter()
        .map(|m| ModelInfo {
            id: m.id.into(),
            name: m.name.into(),
            size: m.size,
            downloaded: model_path(dir, m).is_file(),
        })
        .collect()
}

pub fn delete(dir: &Path, spec: &ModelSpec) -> std::io::Result<()> {
    let path = model_path(dir, spec);
    for p in [path.clone(), crate::download::part_path(&path)] {
        match std::fs::remove_file(&p) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
    }
    Ok(())
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_known_and_unknown() {
        assert_eq!(find("gemma-3-1b").unwrap().name, "Gemma 3 1B");
        assert!(find("gpt-5").is_none());
    }

    #[test]
    fn list_reports_downloaded_files_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(model_path(dir.path(), &CATALOG[1]), b"x").unwrap();
        std::fs::write(dir.path().join("gemma-3-4b-it-Q4_K_M.gguf.part"), b"x").unwrap();
        let l = list(dir.path());
        assert_eq!(l.iter().map(|m| (m.id.as_str(), m.downloaded)).collect::<Vec<_>>(),
                   vec![("gemma-3-4b", false), ("gemma-3-1b", true)]);
    }

    #[test]
    fn list_of_missing_dir_is_all_not_downloaded() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list(&dir.path().join("none")).iter().all(|m| !m.downloaded));
    }

    #[test]
    fn delete_removes_file_and_part_and_tolerates_missing() {
        let dir = tempfile::tempdir().unwrap();
        let p = model_path(dir.path(), &CATALOG[0]);
        std::fs::write(&p, b"x").unwrap();
        std::fs::write(dir.path().join("gemma-3-4b-it-Q4_K_M.gguf.part"), b"x").unwrap();
        delete(dir.path(), &CATALOG[0]).unwrap();
        assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
        delete(dir.path(), &CATALOG[0]).unwrap();
    }

    #[test]
    fn sha256_of_known_content() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f");
        std::fs::write(&p, b"abc").unwrap();
        assert_eq!(sha256_file(&p).unwrap(),
                   "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}
