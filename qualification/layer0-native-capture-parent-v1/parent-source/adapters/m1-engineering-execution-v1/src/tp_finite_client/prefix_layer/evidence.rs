//! Fixed one-layer qualification evidence, separate from the old 8 MiB modes.
use super::{Result, hash, require};
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
pub(super) const LIMIT: usize = 64 << 20;
const SUMMARY_RESERVE: usize = 65_536;
const CAPTURE_FILES: [&str; 11] = [
    "request.json",
    "candidate-registration.json",
    "candidate-program.json",
    "candidate-uploads.json",
    "candidate-bootstrap.json",
    "candidate-request-1.json",
    "candidate-response-1.json",
    "candidate-request-2.json",
    "candidate-response-2.json",
    "candidate-capture.bin",
    "candidate-stderr.bin",
];
#[derive(Clone, Debug, Serialize)]
pub struct File {
    pub name: String,
    pub bytes: usize,
    pub sha256: [u8; 32],
}
pub(super) struct Evidence {
    path: PathBuf,
    bytes: usize,
    files: Vec<File>,
}
impl Evidence {
    pub fn create(path: &Path) -> Result<Self> {
        let parent = path.parent().ok_or("layer evidence parent")?;
        require(
            parent.canonicalize().map_err(|e| e.to_string())? == parent,
            "canonical layer evidence parent",
        )?;
        fs::DirBuilder::new()
            .mode(0o700)
            .create(path)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            path: path.to_owned(),
            bytes: 0,
            files: Vec::new(),
        })
    }
    pub fn append(&mut self, name: &str, raw: &[u8], maximum: usize) -> Result<File> {
        require(
            (!raw.is_empty() || matches!(name, "baseline-stderr.bin" | "candidate-stderr.bin"))
                && raw.len() <= maximum
                && !name.contains('/')
                && !name.contains("..")
                && self.files.len() < 23
                && !self.files.iter().any(|f| f.name == name),
            "layer evidence file/entry bound",
        )?;
        let total = self
            .bytes
            .checked_add(raw.len())
            .ok_or("layer evidence sum overflow")?;
        require(
            total <= LIMIT - SUMMARY_RESERVE,
            "layer whole evidence bound",
        )?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(self.path.join(name))
            .map_err(|e| e.to_string())?;
        file.write_all(raw)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        let pin = File {
            name: name.to_owned(),
            bytes: raw.len(),
            sha256: hash(raw),
        };
        self.bytes = total;
        self.files.push(pin.clone());
        Ok(pin)
    }
    pub fn json(&mut self, name: &str, value: &impl Serialize, maximum: usize) -> Result<File> {
        self.append(
            name,
            &serde_json::to_vec(value).map_err(|e| e.to_string())?,
            maximum,
        )
    }
    pub fn files(&self) -> Vec<File> {
        self.files.clone()
    }
    pub fn finish(&mut self, value: &impl Serialize) -> Result<()> {
        require(self.files.len() == 22, "layer closed evidence roster")?;
        self.write_summary(value)
    }
    pub(super) fn finish_capture(&mut self, value: &impl Serialize) -> Result<()> {
        require(
            self.files.len() == CAPTURE_FILES.len()
                && self
                    .files
                    .iter()
                    .zip(CAPTURE_FILES)
                    .all(|(pin, name)| pin.name == name),
            "layer candidate-only evidence roster",
        )?;
        self.write_summary(value)
    }
    fn write_summary(&mut self, value: &impl Serialize) -> Result<()> {
        for pin in &self.files {
            let path = self.path.join(&pin.name);
            let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            require(
                metadata.is_file() && metadata.len() == pin.bytes as u64,
                "layer evidence replaced",
            )?;
            let mut raw = Vec::new();
            fs::File::open(&path)
                .map_err(|e| e.to_string())?
                .take(pin.bytes as u64 + 1)
                .read_to_end(&mut raw)
                .map_err(|e| e.to_string())?;
            require(
                raw.len() == pin.bytes && hash(&raw) == pin.sha256,
                "layer evidence changed",
            )?;
        }
        let raw = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        require(
            raw.len() <= SUMMARY_RESERVE
                && self
                    .bytes
                    .checked_add(raw.len())
                    .is_some_and(|v| v <= LIMIT),
            "layer final summary bound",
        )?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(self.path.join("summary.json"))
            .map_err(|e| e.to_string())?;
        file.write_all(&raw)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())
    }
}
#[cfg(test)]
#[path = "evidence_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "capture_evidence_tests.rs"]
mod capture_tests;
