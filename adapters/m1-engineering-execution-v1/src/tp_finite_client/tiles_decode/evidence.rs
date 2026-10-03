//! Four bounded typed-state observations and actual requests; no numerical admission.

use super::{FilePin, Observation, Result, old, require, wire};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{DirBuilder, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

const OWN_LIMIT: u64 = 8 << 20;
pub(super) const SUMMARY_LIMIT: usize = 64 << 10;
const STDERR_LIMIT: u64 = 2 << 20;
const REQUEST_LIMIT: u64 = 16 << 10;

#[derive(Serialize)]
pub struct Frame {
    pub response: wire::Response,
    pub control: FilePin,
    pub observation: FilePin,
    pub request: FilePin,
}
#[derive(Serialize)]
pub struct Files {
    pub frames: Vec<Frame>,
    pub child_stderr: FilePin,
    pub bytes_before_summary: u64,
    pub summary_bytes: u64,
    /// Exact files in this private directory, not uninspected supervisor files.
    pub total_bytes: u64,
}

fn verify(pin: &FilePin, limit: u64) -> Result<()> {
    if pin.bytes != 0 {
        pin.read(limit, false)?;
        return Ok(());
    }
    require(
        pin.path.is_absolute() && pin.path.canonicalize().map_err(|e| e.to_string())? == pin.path,
        "tiles decode empty file canonical path",
    )?;
    let mut file = File::open(&pin.path).map_err(|e| e.to_string())?;
    let opened = file.metadata().map_err(|e| e.to_string())?;
    let named = std::fs::symlink_metadata(&pin.path).map_err(|e| e.to_string())?;
    let mut byte = [0];
    require(
        opened.is_file()
            && named.is_file()
            && opened.len() == 0
            && named.len() == 0
            && (opened.dev(), opened.ino()) == (named.dev(), named.ino())
            && file.read(&mut byte).map_err(|e| e.to_string())? == 0
            && pin.sha256 == super::hash(&[]),
        "tiles decode empty evidence changed",
    )
}

fn write_file(path: PathBuf, bytes: &[u8], limit: u64) -> Result<FilePin> {
    require(
        bytes.len() as u64 <= limit,
        "tiles decode evidence file bound",
    )?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    let pin = FilePin {
        path,
        bytes: bytes.len() as u64,
        sha256: Sha256::digest(bytes).into(),
    };
    verify(&pin, limit)?;
    Ok(pin)
}

pub(super) struct Evidence {
    directory: PathBuf,
    frames: Vec<Frame>,
    bytes: u64,
}
impl Evidence {
    pub(super) fn create(directory: &Path) -> Result<Self> {
        let parent = directory
            .parent()
            .ok_or("tiles decode evidence parent missing")?;
        require(
            directory.is_absolute()
                && directory.file_name().is_some()
                && parent.canonicalize().map_err(|e| e.to_string())? == parent,
            "tiles decode evidence requires canonical parent",
        )?;
        DirBuilder::new()
            .mode(0o700)
            .create(directory)
            .map_err(|e| e.to_string())?;
        require(
            directory.canonicalize().map_err(|e| e.to_string())? == directory,
            "tiles decode evidence changed",
        )?;
        Ok(Self {
            directory: directory.to_owned(),
            frames: Vec::with_capacity(4),
            bytes: 0,
        })
    }
    fn add(&mut self, name: &str, bytes: &[u8], limit: u64) -> Result<FilePin> {
        let total = self
            .bytes
            .checked_add(bytes.len() as u64)
            .ok_or("tiles decode evidence size overflow")?;
        require(
            total + SUMMARY_LIMIT as u64 <= OWN_LIMIT,
            "tiles decode aggregate retained output bound",
        )?;
        let pin = write_file(self.directory.join(name), bytes, limit)?;
        self.bytes = total;
        Ok(pin)
    }
    pub(super) fn append(
        &mut self,
        request: &wire::Request,
        response: &wire::Response,
        control: &wire::Control,
        payload: &[u8],
    ) -> Result<()> {
        let wire::Event::Completed(c) = &response.event else {
            return Err("tiles decode evidence expects Completed".into());
        };
        let position = self.frames.len();
        require(
            position < 4
                && c.position as usize == position
                && c.generation == position as u64 + 1
                && payload.len() == old::OBSERVATION_BYTES
                && request.id == response.id
                && request.id == c.generation
                && old::part(payload) == c.observation
                && old::part(&control.encode()) == c.control,
            "tiles decode evidence order/extent",
        )?;
        let control = self.add(
            &format!("control-{position}.bin"),
            &control.encode(),
            wire::CONTROL_BYTES as u64,
        )?;
        let observation = self.add(
            &format!("observation-{position}.bin"),
            payload,
            old::OBSERVATION_BYTES as u64,
        )?;
        let mut request_bytes = serde_json::to_vec(request).map_err(|e| e.to_string())?;
        request_bytes.push(b'\n');
        let request = self.add(
            &format!("request-{position}.json"),
            &request_bytes,
            REQUEST_LIMIT,
        )?;
        self.frames.push(Frame {
            response: response.clone(),
            control,
            observation,
            request,
        });
        Ok(())
    }
    pub(super) fn finish(mut self, stderr: &[u8]) -> Result<Files> {
        require(
            self.frames.len() == 4,
            "tiles decode incomplete retained stream",
        )?;
        let child_stderr = self.add("child-stderr.bin", stderr, STDERR_LIMIT)?;
        for frame in &self.frames {
            verify(&frame.control, wire::CONTROL_BYTES as u64)?;
            verify(&frame.observation, old::OBSERVATION_BYTES as u64)?;
            verify(&frame.request, REQUEST_LIMIT)?;
        }
        Ok(Files {
            frames: self.frames,
            child_stderr,
            bytes_before_summary: self.bytes,
            summary_bytes: 0,
            total_bytes: self.bytes,
        })
    }
}

pub(super) fn publish(observation: &mut Observation) -> Result<()> {
    let mut bytes = Vec::new();
    for _ in 0..8 {
        bytes = serde_json::to_vec(observation).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        require(
            bytes.len() <= SUMMARY_LIMIT,
            "tiles decode final summary bound",
        )?;
        if observation.files.summary_bytes == bytes.len() as u64 {
            break;
        }
        observation.files.summary_bytes = bytes.len() as u64;
        observation.files.total_bytes = observation.files.bytes_before_summary + bytes.len() as u64;
    }
    require(
        observation.files.summary_bytes == bytes.len() as u64
            && observation.files.total_bytes <= OWN_LIMIT,
        "tiles decode final accounting",
    )?;
    let directory = &observation.request.evidence_directory;
    let pending = directory.join("complete.pending");
    write_file(pending.clone(), &bytes, SUMMARY_LIMIT as u64)?;
    std::fs::hard_link(&pending, directory.join("complete.json")).map_err(|e| e.to_string())?;
    std::fs::remove_file(pending).map_err(|e| e.to_string())?;
    File::open(directory)
        .and_then(|dir| dir.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
                "finite-tiles-decode-evidence-{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn worst_typed_capture_requests_and_stderr_fit_existing_output_cap() {
        let total = 4 * (wire::CONTROL_BYTES + old::OBSERVATION_BYTES) as u64
            + 4 * REQUEST_LIMIT
            + STDERR_LIMIT
            + SUMMARY_LIMIT as u64;
        assert_eq!(total, 5_322_144);
        assert!(total < OWN_LIMIT);
        assert!(OWN_LIMIT + SUMMARY_LIMIT as u64 + (512 << 10) < 32 << 20);
    }
    #[test]
    fn incomplete_exclusive_evidence_does_not_publish_or_replace() {
        let temp = Temp::new();
        let path = temp.0.join("run");
        let mut evidence = Evidence::create(&path).unwrap();
        evidence.add("provisional.bin", &[1, 2], 2).unwrap();
        assert!(evidence.add("provisional.bin", &[3], 2).is_err());
        assert!(evidence.finish(&[]).is_err());
        assert_eq!(std::fs::read(path.join("provisional.bin")).unwrap(), [1, 2]);
        assert!(!path.join("complete.json").exists());
        assert!(Evidence::create(&path).is_err());
    }
    #[test]
    fn output_pins_rehash_empty_and_nonempty_files_and_refuse_links() {
        let temp = Temp::new();
        let empty = write_file(temp.0.join("empty"), &[], 0).unwrap();
        verify(&empty, 0).unwrap();
        let pin = write_file(temp.0.join("bytes"), &[1, 2, 3], 3).unwrap();
        assert!(write_file(temp.0.join("oversize"), &[1, 2, 3], 2).is_err());
        let link = temp.0.join("link");
        std::os::unix::fs::symlink(&pin.path, &link).unwrap();
        assert!(write_file(link, &[0], 1).is_err());
        std::fs::write(&pin.path, [3, 2, 1]).unwrap();
        assert!(verify(&pin, 3).is_err());
    }
}
