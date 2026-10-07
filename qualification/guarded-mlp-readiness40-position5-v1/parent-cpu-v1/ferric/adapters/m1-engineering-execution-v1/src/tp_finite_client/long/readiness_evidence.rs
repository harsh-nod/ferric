//! Compact canonical frame records and four selected captures, published after Close.
use super::{FilePin, Observation, Result, hash, long, require};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::{
    fs::{DirBuilder, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const STDERR_BYTES: u64 = 2 << 20;
const SUMMARY_BYTES: u64 = 128 << 10;
const RESERVE: u64 = 512 << 10;
const CAPTURE_BYTES: u64 = (crate::finite_guarded_mlp_decode_wire_v1::CONTROL_BYTES
    + crate::finite_forward_wire_v1::OBSERVATION_BYTES) as u64;
#[derive(Serialize)]
pub struct Capture {
    pub position: u32,
    pub file: FilePin,
}
#[derive(Serialize)]
pub struct Files {
    pub frames: FilePin,
    pub captures: Vec<Capture>,
    pub child_stderr: FilePin,
    pub rows: u32,
    pub bytes_before_summary: u64,
    pub summary_bytes: u64,
    pub total_bytes: u64,
    pub supervisor_metadata_allowance: u64,
}
struct Retained {
    path: PathBuf,
    file: File,
    digest: Sha256,
    bytes: u64,
    limit: u64,
}
impl Retained {
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
    fn append(&mut self, raw: &[u8]) -> Result<()> {
        let next = self
            .bytes
            .checked_add(raw.len() as u64)
            .ok_or("readiness retention overflow")?;
        require(next <= self.limit, "readiness retained file bound")?;
        self.file.write_all(raw).map_err(|e| e.to_string())?;
        self.digest.update(raw);
        self.bytes = next;
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
    require(
        pin.path.is_absolute() && pin.path.canonicalize().map_err(|e| e.to_string())? == pin.path,
        "readiness empty stderr canonical path",
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
            && pin.sha256 == hash(&[]),
        "readiness empty stderr changed",
    )
}
pub(super) struct Evidence {
    directory: PathBuf,
    frames: Retained,
    captures: Vec<Capture>,
    rows: u32,
    profile: long::Profile,
}
impl Evidence {
    pub(super) fn create(directory: &Path) -> Result<Self> {
        Self::create_for(directory, long::Profile::Readiness40)
    }
    pub(super) fn create_for(directory: &Path, profile: long::Profile) -> Result<Self> {
        super::observation_schema(profile)?;
        let parent = directory
            .parent()
            .ok_or("readiness evidence parent absent")?;
        require(
            directory.is_absolute()
                && parent.canonicalize().map_err(|e| e.to_string())? == parent
                && directory.file_name().is_some(),
            "readiness fresh canonical evidence parent",
        )?;
        DirBuilder::new()
            .mode(0o700)
            .create(directory)
            .map_err(|e| e.to_string())?;
        require(
            directory.canonicalize().map_err(|e| e.to_string())? == directory,
            "readiness evidence directory changed",
        )?;
        Ok(Self {
            directory: directory.to_owned(),
            frames: Retained::create(
                directory.join("frames.ndjson"),
                40 * (long::RECORD_BYTES as u64 + 1),
            )?,
            captures: Vec::with_capacity(4),
            rows: 0,
            profile,
        })
    }
    pub(super) fn append(
        &mut self,
        frame: &long::Frame,
        capture: Option<&long::Capture>,
    ) -> Result<()> {
        frame.validate(self.profile).map_err(|e| e.to_string())?;
        require(
            self.rows < 40
                && frame.completion.position == self.rows
                && frame.completion.captured == capture.is_some(),
            "readiness evidence order/capture extent",
        )?;
        let mut raw = serde_json::to_vec(frame).map_err(|e| e.to_string())?;
        require(
            !raw.is_empty() && raw.len() <= long::RECORD_BYTES,
            "readiness canonical frame bound",
        )?;
        raw.push(b'\n');
        if let Some(capture) = capture {
            let actual = long::Completion::from_observation(
                self.profile,
                &frame.request,
                &capture.control,
                &capture.observation,
                frame.completion.output_token,
            )
            .map_err(|e| e.to_string())?;
            let mut expected = frame.completion.clone();
            expected.chain = [0; 32];
            require(
                actual == expected,
                "readiness retained selected capture join",
            )?;
            let mut file = Retained::create(
                self.directory.join(format!("capture-{}.bin", self.rows)),
                CAPTURE_BYTES,
            )?;
            file.append(&capture.control.encode())?;
            file.append(&capture.observation)?;
            self.captures.push(Capture {
                position: self.rows,
                file: file.finish()?,
            });
        }
        self.frames.append(&raw)?;
        self.frames.file.sync_data().map_err(|e| e.to_string())?;
        self.rows += 1;
        Ok(())
    }
    pub(super) fn finish(self, stderr: &[u8]) -> Result<Files> {
        require(
            self.rows == 40
                && self
                    .captures
                    .iter()
                    .map(|c| c.position)
                    .eq(self.profile.capture_positions()),
            "readiness complete selected roster",
        )?;
        let frames = self.frames.finish()?;
        for capture in &self.captures {
            verify(&capture.file, CAPTURE_BYTES)?;
        }
        let mut file = Retained::create(self.directory.join("child-stderr.bin"), STDERR_BYTES)?;
        file.append(stderr)?;
        let child_stderr = file.finish()?;
        let n = frames.bytes
            + child_stderr.bytes
            + self.captures.iter().map(|c| c.file.bytes).sum::<u64>();
        require(
            n + SUMMARY_BYTES + RESERVE <= long::EVIDENCE_BYTES as u64,
            "readiness aggregate retained evidence bound",
        )?;
        Ok(Files {
            frames,
            captures: self.captures,
            child_stderr,
            rows: self.rows,
            bytes_before_summary: n,
            summary_bytes: 0,
            total_bytes: n,
            supervisor_metadata_allowance: RESERVE,
        })
    }
}
fn reconcile(value: &Observation) -> Result<()> {
    let raw = value
        .files
        .frames
        .read(40 * (long::RECORD_BYTES as u64 + 1), true)?;
    require(
        raw.last() == Some(&b'\n'),
        "readiness complete frame records",
    )?;
    let rows = raw[..raw.len() - 1]
        .split(|b| *b == b'\n')
        .collect::<Vec<_>>();
    require(rows.len() == 40, "readiness retained frame census")?;
    let mut transcript =
        long::Transcript::new(value.bootstrap.sequence.clone()).map_err(|e| e.to_string())?;
    let mut selected = value.files.captures.iter();
    for (position, row) in rows.into_iter().enumerate() {
        require(
            !row.is_empty() && row.len() <= long::RECORD_BYTES,
            "readiness retained record bound",
        )?;
        let frame: long::Frame = serde_json::from_slice(row).map_err(|e| e.to_string())?;
        require(
            frame.completion.position as usize == position,
            "readiness retained record order",
        )?;
        transcript
            .begin(&frame.request)
            .map_err(|e| e.to_string())?;
        if frame.completion.captured {
            let pin = selected.next().ok_or("readiness retained capture absent")?;
            require(
                pin.position as usize == position && pin.file.bytes == CAPTURE_BYTES,
                "readiness retained capture identity",
            )?;
            let body = pin.file.read(CAPTURE_BYTES, true)?;
            let n = crate::finite_guarded_mlp_decode_wire_v1::CONTROL_BYTES;
            let control = long::decode_control(
                &body[..n],
                value.bootstrap.sequence.profile,
                frame.completion.generation,
            )
            .map_err(|e| e.to_string())?;
            let actual = long::Completion::from_observation(
                value.bootstrap.sequence.profile,
                &frame.request,
                &control,
                &body[n..],
                frame.completion.output_token,
            )
            .map_err(|e| e.to_string())?;
            let mut expected = frame.completion.clone();
            expected.chain = [0; 32];
            require(
                actual == expected,
                "readiness retained original selected completion",
            )?;
        }
        transcript.advance(&frame).map_err(|e| e.to_string())?;
    }
    require(
        selected.next().is_none()
            && transcript.output_tokens().is_empty()
            && transcript.digest() == value.transcript_sha256,
        "readiness retained complete transcript",
    )?;
    transcript
        .close(&value.close.request, value.transcript_sha256)
        .map_err(|e| e.to_string())?;
    let n = value.files.frames.bytes
        + value.files.child_stderr.bytes
        + value
            .files
            .captures
            .iter()
            .map(|c| c.file.bytes)
            .sum::<u64>();
    require(
        n == value.files.bytes_before_summary
            && value.files.supervisor_metadata_allowance == RESERVE,
        "readiness retained aggregate accounting",
    )
}
pub(super) fn publish(value: &mut Observation) -> Result<()> {
    super::validate_summary(value)?;
    verify(&value.files.frames, 40 * (long::RECORD_BYTES as u64 + 1))?;
    verify(&value.files.child_stderr, STDERR_BYTES)?;
    for capture in &value.files.captures {
        verify(&capture.file, CAPTURE_BYTES)?;
    }
    reconcile(value)?;
    let mut raw = Vec::new();
    for _ in 0..8 {
        raw = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        raw.push(b'\n');
        require(
            raw.len() as u64 <= SUMMARY_BYTES,
            "readiness complete summary bound",
        )?;
        if value.files.summary_bytes == raw.len() as u64 {
            break;
        }
        value.files.summary_bytes = raw.len() as u64;
        value.files.total_bytes = value.files.bytes_before_summary + raw.len() as u64;
    }
    require(
        value.files.summary_bytes == raw.len() as u64
            && value.files.total_bytes + RESERVE <= long::EVIDENCE_BYTES as u64,
        "readiness final exact accounting",
    )?;
    let directory = &value.request.base.evidence_directory;
    let path = directory.join("complete.pending");
    let mut file = Retained::create(path.clone(), SUMMARY_BYTES)?;
    file.append(&raw)?;
    file.finish()?;
    std::fs::hard_link(&path, directory.join("complete.json")).map_err(|e| e.to_string())?;
    std::fs::remove_file(path).map_err(|e| e.to_string())?;
    File::open(directory)
        .and_then(|dir| dir.sync_all())
        .map_err(|e| e.to_string())
}
