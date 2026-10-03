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
        "prefix decode empty file canonical path",
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
        "prefix decode empty evidence changed",
    )
}

fn write_file(path: PathBuf, bytes: &[u8], limit: u64) -> Result<FilePin> {
    require(
        bytes.len() as u64 <= limit,
        "prefix decode evidence file bound",
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
            .ok_or("prefix decode evidence parent missing")?;
        require(
            directory.is_absolute()
                && directory.file_name().is_some()
                && parent.canonicalize().map_err(|e| e.to_string())? == parent,
            "prefix decode evidence requires canonical parent",
        )?;
        DirBuilder::new()
            .mode(0o700)
            .create(directory)
            .map_err(|e| e.to_string())?;
        require(
            directory.canonicalize().map_err(|e| e.to_string())? == directory,
            "prefix decode evidence changed",
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
            .ok_or("prefix decode evidence size overflow")?;
        require(
            total
                .checked_add(SUMMARY_LIMIT as u64)
                .is_some_and(|n| n <= OWN_LIMIT),
            "prefix decode aggregate retained output bound",
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
            return Err("prefix decode evidence expects Completed".into());
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
            "prefix decode evidence order/extent",
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
            "prefix decode incomplete retained stream",
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
    require(
        observation.completed_forwards == 4
            && observation.native_closed
            && observation.child_exit_zero
            && observation.process_group_absent
            && observation.files.frames.len() == 4
            && observation.input_tokens.len() == 4
            && observation.observed_output_tokens.len() == 4
            && !observation.numerical_acceptance
            && !observation.performance_claim
            && !observation.production_authority
            && !observation.full_long_workload,
        "prefix decode publication requires closed/reaped four-forward observation",
    )?;
    let close_request = wire::Request {
        protocol: wire::PROTOCOL,
        id: 5,
        device_ids: observation.bootstrap.device_ids,
        session: observation.bootstrap.scope.session,
        registration: observation.bootstrap.registration,
        profile_sha256: observation.profile_sha256,
        command: wire::Command::Close,
    };
    super::validate_close(
        &close_request,
        &observation.close,
        None,
        &[],
        observation.transcript_sha256,
    )?;
    for frame in &observation.files.frames {
        verify(&frame.control, wire::CONTROL_BYTES as u64)?;
        verify(&frame.observation, old::OBSERVATION_BYTES as u64)?;
        verify(&frame.request, REQUEST_LIMIT)?;
    }
    verify(&observation.files.child_stderr, STDERR_LIMIT)?;
    let mut expected = vec!["child-stderr.bin".to_owned()];
    for i in 0..4 {
        for name in [
            format!("control-{i}.bin"),
            format!("observation-{i}.bin"),
            format!("request-{i}.json"),
        ] {
            expected.push(name);
        }
    }
    expected.sort();
    let mut actual = std::fs::read_dir(&observation.request.evidence_directory)
        .map_err(|e| e.to_string())?
        .map(|entry| {
            entry
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>>>()?;
    actual.sort();
    require(
        actual == expected,
        "prefix decode exact pre-publication evidence census",
    )?;
    let mut bytes = Vec::new();
    for _ in 0..8 {
        bytes = serde_json::to_vec(observation).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        require(
            bytes.len() <= SUMMARY_LIMIT,
            "prefix decode final summary bound",
        )?;
        if observation.files.summary_bytes == bytes.len() as u64 {
            break;
        }
        observation.files.summary_bytes = bytes.len() as u64;
        observation.files.total_bytes = observation
            .files
            .bytes_before_summary
            .checked_add(bytes.len() as u64)
            .ok_or("prefix decode summary accounting overflow")?;
    }
    require(
        observation.files.summary_bytes == bytes.len() as u64
            && observation.files.total_bytes <= OWN_LIMIT,
        "prefix decode final accounting",
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
#[path = "evidence_tests.rs"]
mod tests;
