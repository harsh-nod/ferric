//! Streaming retained evidence, not a numerical acceptance report.

use super::{FilePin, Observation, Result, hex, require, wire};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{DirBuilder, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

pub(super) const INDEX_LIMIT: u64 = 1 << 20;
pub(super) const ROW_LIMIT: usize = 384;
pub(super) const SUMMARY_LIMIT: usize = 64 << 10;
const STDERR_LIMIT: u64 = 2 << 20;
const TOTAL_LIMIT: u64 = 32 << 20;
const SUPERVISOR_ALLOWANCE: u64 = 512 << 10;
const OWN_LIMIT: u64 = TOTAL_LIMIT - SUMMARY_LIMIT as u64 - SUPERVISOR_ALLOWANCE;

#[derive(Serialize)]
pub struct Capture {
    pub position: u32,
    pub file: FilePin,
}

#[derive(Serialize)]
pub struct Files {
    pub controls: FilePin,
    pub index: FilePin,
    pub captures: Vec<Capture>,
    pub child_stderr: FilePin,
    pub index_rows: u32,
    pub bytes_before_summary: u64,
    pub summary_bytes: u64,
    /// Includes the single complete.json, not external supervisor files.
    pub total_bytes: u64,
    /// Reserved but not measured here; the outer supervisor must bound its own files.
    pub supervisor_metadata_allowance: u64,
}

struct RetainedFile {
    path: PathBuf,
    file: File,
    digest: Sha256,
    bytes: u64,
    limit: u64,
}
impl RetainedFile {
    fn create(path: PathBuf, limit: u64) -> Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            path,
            file,
            digest: Sha256::new(),
            bytes: 0,
            limit,
        })
    }
    fn append(&mut self, bytes: &[u8]) -> Result<()> {
        let total = self
            .bytes
            .checked_add(bytes.len() as u64)
            .ok_or("long evidence size overflow")?;
        require(total <= self.limit, "long evidence file bound")?;
        self.file.write_all(bytes).map_err(|e| e.to_string())?;
        self.digest.update(bytes);
        self.bytes = total;
        Ok(())
    }
    fn finish(self) -> Result<FilePin> {
        self.file.sync_all().map_err(|e| e.to_string())?;
        drop(self.file);
        let pin = FilePin {
            path: self.path,
            bytes: self.bytes,
            sha256: self.digest.finalize().into(),
        };
        verify(&pin, self.limit)?;
        Ok(pin)
    }
}

fn verify(pin: &FilePin, limit: u64) -> Result<()> {
    if pin.bytes != 0 {
        pin.read(limit, false)?;
        return Ok(());
    }
    // Empty stderr is legitimate; the general nonempty-input FilePin refuses it.
    require(
        pin.path.is_absolute() && pin.path.canonicalize().map_err(|e| e.to_string())? == pin.path,
        "long empty evidence canonical path",
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
        "long empty evidence changed",
    )
}

#[derive(Serialize)]
struct Row {
    generation: u64,
    position: u32,
    input_token: u32,
    output_token: u32,
    control_offset: u64,
    control_sha256: String,
    observation_sha256: String,
    chain: String,
    captured: bool,
}
fn row_bytes(c: &wire::Completion, offset: u64) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(&Row {
        generation: c.generation,
        position: c.position,
        input_token: c.input_token,
        output_token: c.output_token,
        control_offset: offset,
        control_sha256: hex(&c.control.sha256),
        observation_sha256: hex(&c.observation.sha256),
        chain: hex(&c.chain),
        captured: c.capture.is_some(),
    })
    .map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    require(bytes.len() <= ROW_LIMIT, "long complete NDJSON row bound")?;
    Ok(bytes)
}

pub(super) struct Evidence {
    directory: PathBuf,
    controls: RetainedFile,
    index: RetainedFile,
    captures: Vec<Capture>,
    rows: u32,
}
impl Evidence {
    pub(super) fn create(directory: &Path) -> Result<Self> {
        let parent = directory.parent().ok_or("long evidence parent missing")?;
        require(
            directory.is_absolute()
                && parent.canonicalize().map_err(|e| e.to_string())? == parent
                && directory.file_name().is_some(),
            "long evidence directory must have canonical parent",
        )?;
        DirBuilder::new()
            .mode(0o700)
            .create(directory)
            .map_err(|e| e.to_string())?;
        require(
            directory.canonicalize().map_err(|e| e.to_string())? == directory,
            "long evidence directory changed",
        )?;
        Ok(Self {
            directory: directory.to_owned(),
            controls: RetainedFile::create(
                directory.join("controls.bin"),
                u64::from(wire::FORWARDS) * wire::CONTROL_BYTES as u64,
            )?,
            index: RetainedFile::create(directory.join("frames.ndjson"), INDEX_LIMIT)?,
            captures: Vec::with_capacity(4),
            rows: 0,
        })
    }
    pub(super) fn append(
        &mut self,
        completion: &wire::Completion,
        control: &wire::Control,
        payload: &[u8],
    ) -> Result<()> {
        require(
            self.rows < wire::FORWARDS
                && completion.position == self.rows
                && completion.generation == u64::from(self.rows) + 1,
            "long evidence append order",
        )?;
        let selected = wire::capture_position(self.rows);
        require(
            selected == completion.capture.is_some()
                && payload.len()
                    == if selected {
                        crate::finite_forward_wire_v1::OBSERVATION_BYTES
                    } else {
                        0
                    },
            "long evidence capture extent",
        )?;
        let row = row_bytes(completion, self.controls.bytes)?;
        self.controls.append(&control.encode())?;
        if selected {
            let mut file = RetainedFile::create(
                self.directory.join(format!("capture-{}.bin", self.rows)),
                crate::finite_forward_wire_v1::OBSERVATION_BYTES as u64,
            )?;
            file.append(payload)?;
            self.captures.push(Capture {
                position: self.rows,
                file: file.finish()?,
            });
        }
        self.index.append(&row)?;
        self.rows += 1;
        Ok(())
    }
    pub(super) fn finish(self, stderr: &[u8]) -> Result<Files> {
        require(
            self.rows == wire::FORWARDS
                && self
                    .captures
                    .iter()
                    .map(|c| c.position)
                    .eq(wire::CAPTURE_POSITIONS),
            "long incomplete retained stream",
        )?;
        let controls = self.controls.finish()?;
        let index = self.index.finish()?;
        for capture in &self.captures {
            verify(
                &capture.file,
                crate::finite_forward_wire_v1::OBSERVATION_BYTES as u64,
            )?;
        }
        let mut diagnostic =
            RetainedFile::create(self.directory.join("child-stderr.bin"), STDERR_LIMIT)?;
        diagnostic.append(stderr)?;
        let child_stderr = diagnostic.finish()?;
        let bytes_before_summary = controls.bytes
            + index.bytes
            + child_stderr.bytes
            + self.captures.iter().map(|c| c.file.bytes).sum::<u64>();
        require(
            bytes_before_summary + SUMMARY_LIMIT as u64 <= OWN_LIMIT,
            "long aggregate evidence bound",
        )?;
        Ok(Files {
            controls,
            index,
            captures: self.captures,
            child_stderr,
            index_rows: self.rows,
            bytes_before_summary,
            summary_bytes: 0,
            total_bytes: bytes_before_summary,
            supervisor_metadata_allowance: SUPERVISOR_ALLOWANCE,
        })
    }
}

pub(super) fn publish(observation: &mut Observation) -> Result<()> {
    // Find the small self-length fixed point, including every JSON field and newline.
    let mut bytes = Vec::new();
    for _ in 0..8 {
        bytes = serde_json::to_vec(observation).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        require(bytes.len() <= SUMMARY_LIMIT, "long final summary bound")?;
        if observation.files.summary_bytes == bytes.len() as u64 {
            break;
        }
        observation.files.summary_bytes = bytes.len() as u64;
        observation.files.total_bytes = observation.files.bytes_before_summary + bytes.len() as u64;
    }
    require(
        observation.files.summary_bytes == bytes.len() as u64
            && observation.files.total_bytes <= OWN_LIMIT,
        "long final summary accounting",
    )?;
    let directory = &observation.request.evidence_directory;
    let pending = directory.join("complete.pending");
    let mut file = RetainedFile::create(pending.clone(), SUMMARY_LIMIT as u64)?;
    file.append(&bytes)?;
    file.finish()?;
    // Same-filesystem hard-link publication refuses an existing final path.
    // Interrupted or failed writes leave only a non-complete provisional file.
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
                "finite-long-evidence-{}-{}",
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
    fn retained_fixed_binary_and_json_caps_fit_without_output_inflation() {
        let worst = u64::from(wire::FORWARDS) * wire::CONTROL_BYTES as u64
            + 4 * crate::finite_forward_wire_v1::OBSERVATION_BYTES as u64
            + INDEX_LIMIT
            + STDERR_LIMIT
            + SUMMARY_LIMIT as u64;
        assert_eq!(worst, 32_925_112);
        assert!(worst < OWN_LIMIT);
        assert!(worst + SUMMARY_LIMIT as u64 + SUPERVISOR_ALLOWANCE < TOTAL_LIMIT);
    }
    #[test]
    fn index_counts_all_fields_and_trailing_newline_without_hash_arrays() {
        let c = wire::Completion {
            generation: 2303,
            position: 2302,
            input_token: 151_935,
            output_token: 151_935,
            control: crate::finite_setup_wire_v1::Part {
                bytes: wire::CONTROL_BYTES as u32,
                sha256: [255; 32],
            },
            observation: crate::finite_setup_wire_v1::Part {
                bytes: 606_976,
                sha256: [255; 32],
            },
            capture: None,
            chain: [255; 32],
        };
        let bytes = row_bytes(&c, 27_274_096).unwrap();
        assert_eq!(bytes.last(), Some(&b'\n'));
        assert!(bytes.len() <= ROW_LIMIT);
        assert!(bytes.len() as u64 * u64::from(wire::FORWARDS) <= INDEX_LIMIT);
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["chain"].as_str().unwrap().len(), 64);
        assert_eq!(value.as_object().unwrap().len(), 9);
    }
    #[test]
    fn file_limit_is_charged_before_write_and_exact_bytes_are_rehashed() {
        let temp = Temp::new();
        let path = temp.0.join("bounded.bin");
        let mut file = RetainedFile::create(path.clone(), 3).unwrap();
        file.append(&[1, 2, 3]).unwrap();
        assert!(file.append(&[4]).is_err());
        assert_eq!(file.bytes, 3);
        let pin = file.finish().unwrap();
        assert_eq!(pin.bytes, 3);
        assert_eq!(pin.sha256, super::super::hash(&[1, 2, 3]));
        assert_eq!(std::fs::read(path).unwrap(), [1, 2, 3]);
    }
    #[test]
    fn incomplete_stream_preserves_provisional_files_and_never_publishes_complete() {
        let temp = Temp::new();
        let path = temp.0.join("run");
        let evidence = Evidence::create(&path).unwrap();
        assert!(Evidence::create(&path).is_err());
        assert!(evidence.finish(&[]).is_err());
        assert!(path.join("controls.bin").is_file());
        assert!(path.join("frames.ndjson").is_file());
        assert!(!path.join("complete.json").exists());
    }
    #[test]
    fn exclusive_files_and_canonical_parents_reject_links_and_overwrite() {
        let temp = Temp::new();
        let path = temp.0.join("existing");
        let file = RetainedFile::create(path.clone(), 3).unwrap();
        assert!(RetainedFile::create(path.clone(), 3).is_err());
        file.finish().unwrap();
        let link = temp.0.join("link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(RetainedFile::create(link, 3).is_err());
        let directory_link = temp.0.join("directory-link");
        std::os::unix::fs::symlink(&temp.0, &directory_link).unwrap();
        assert!(Evidence::create(&directory_link.join("run")).is_err());
    }
}
