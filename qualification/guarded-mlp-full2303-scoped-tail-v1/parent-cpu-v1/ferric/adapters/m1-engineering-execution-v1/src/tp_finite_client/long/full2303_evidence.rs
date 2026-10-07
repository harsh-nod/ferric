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
const DECODED_BYTES: u64 = 128 << 10;
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
    pub decoded_output: FilePin,
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
            .ok_or("full2303 retention overflow")?;
        require(next <= self.limit, "full2303 retained file bound")?;
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
        "full2303 empty retained file canonical path",
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
        "full2303 empty retained file changed",
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
        let profile = long::Profile::Full2303;
        let parent = directory
            .parent()
            .ok_or("full2303 evidence parent absent")?;
        require(
            directory.is_absolute()
                && parent.canonicalize().map_err(|e| e.to_string())? == parent
                && directory.file_name().is_some(),
            "full2303 fresh canonical evidence parent",
        )?;
        DirBuilder::new()
            .mode(0o700)
            .create(directory)
            .map_err(|e| e.to_string())?;
        require(
            directory.canonicalize().map_err(|e| e.to_string())? == directory,
            "full2303 evidence directory changed",
        )?;
        Ok(Self {
            directory: directory.to_owned(),
            frames: Retained::create(
                directory.join("frames.ndjson"),
                u64::from(long::FORWARDS) * (long::RECORD_BYTES as u64 + 1),
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
            self.rows < long::FORWARDS
                && frame.completion.position == self.rows
                && frame.completion.captured == capture.is_some(),
            "full2303 evidence order/capture extent",
        )?;
        let mut raw = serde_json::to_vec(frame).map_err(|e| e.to_string())?;
        require(
            !raw.is_empty() && raw.len() <= long::RECORD_BYTES,
            "full2303 canonical frame bound",
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
                "full2303 retained selected capture join",
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
    pub(super) fn finish(self, stderr: &[u8], decoded: &[u8]) -> Result<Files> {
        require(
            self.rows == long::FORWARDS
                && self
                    .captures
                    .iter()
                    .map(|c| c.position)
                    .eq(self.profile.capture_positions()),
            "full2303 complete selected roster",
        )?;
        let frames = self.frames.finish()?;
        for capture in &self.captures {
            verify(&capture.file, CAPTURE_BYTES)?;
        }
        let mut file = Retained::create(self.directory.join("child-stderr.bin"), STDERR_BYTES)?;
        file.append(stderr)?;
        let child_stderr = file.finish()?;
        let mut file = Retained::create(self.directory.join("generated-text.bin"), DECODED_BYTES)?;
        file.append(decoded)?;
        let decoded_output = file.finish()?;
        let n = frames.bytes
            + child_stderr.bytes
            + decoded_output.bytes
            + self.captures.iter().map(|c| c.file.bytes).sum::<u64>();
        require(
            n + SUMMARY_BYTES + RESERVE <= long::EVIDENCE_BYTES as u64
                && n + SUMMARY_BYTES + RESERVE <= long::MAX_RETAINED_BYTES as u64,
            "full2303 aggregate retained evidence bound",
        )?;
        Ok(Files {
            frames,
            captures: self.captures,
            child_stderr,
            decoded_output,
            rows: self.rows,
            bytes_before_summary: n,
            summary_bytes: 0,
            total_bytes: n,
            supervisor_metadata_allowance: RESERVE,
        })
    }
}
fn reconcile(value: &Observation) -> Result<()> {
    let raw = value.files.frames.read(
        u64::from(long::FORWARDS) * (long::RECORD_BYTES as u64 + 1),
        true,
    )?;
    require(
        raw.last() == Some(&b'\n'),
        "full2303 complete frame records",
    )?;
    let rows = raw[..raw.len() - 1]
        .split(|b| *b == b'\n')
        .collect::<Vec<_>>();
    require(
        rows.len() == long::FORWARDS as usize,
        "full2303 retained frame census",
    )?;
    let mut transcript =
        long::Transcript::new(value.bootstrap.sequence.clone()).map_err(|e| e.to_string())?;
    let mut selected = value.files.captures.iter();
    for (position, row) in rows.into_iter().enumerate() {
        require(
            !row.is_empty() && row.len() <= long::RECORD_BYTES,
            "full2303 retained record bound",
        )?;
        let frame: long::Frame = serde_json::from_slice(row).map_err(|e| e.to_string())?;
        require(
            frame.completion.position as usize == position,
            "full2303 retained record order",
        )?;
        transcript
            .begin(&frame.request)
            .map_err(|e| e.to_string())?;
        if frame.completion.captured {
            let pin = selected.next().ok_or("full2303 retained capture absent")?;
            require(
                pin.position as usize == position && pin.file.bytes == CAPTURE_BYTES,
                "full2303 retained capture identity",
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
                "full2303 retained original selected completion",
            )?;
        }
        transcript.advance(&frame).map_err(|e| e.to_string())?;
    }
    require(
        selected.next().is_none()
            && transcript.output_tokens() == value.generated_tokens
            && transcript.output_tokens().len() == long::OUTPUT_TOKENS
            && transcript.digest() == value.transcript_sha256,
        "full2303 retained complete transcript",
    )?;
    transcript
        .close(&value.close.request, value.transcript_sha256)
        .map_err(|e| e.to_string())?;
    value
        .close
        .validate(
            &value.close.request,
            value.transcript_sha256,
            transcript.output_tokens(),
        )
        .map_err(|e| e.to_string())?;
    let n = value.files.frames.bytes
        + value.files.child_stderr.bytes
        + value.files.decoded_output.bytes
        + value
            .files
            .captures
            .iter()
            .map(|c| c.file.bytes)
            .sum::<u64>();
    require(
        n == value.files.bytes_before_summary
            && value.files.supervisor_metadata_allowance == RESERVE,
        "full2303 retained aggregate accounting",
    )
}
pub(super) fn publish(value: &mut Observation) -> Result<()> {
    publish_inner(value, None, super::NativePolicy::Full)
}
pub(super) fn publish_scoped(value: &mut Observation, deadline: std::time::Instant) -> Result<()> {
    check_deadline(Some(deadline))?;
    super::scoped::validate_file(value)?;
    publish_inner(value, Some(deadline), super::NativePolicy::ScopedWarm)
}
pub(super) fn publish_bank_scoped_census(
    value: &mut Observation,
    deadline: std::time::Instant,
) -> Result<()> {
    check_deadline(Some(deadline))?;
    super::bank_scoped_census::validate_file(value)?;
    publish_inner(value, Some(deadline), super::NativePolicy::BankScopedCensus)
}

pub(super) fn publish_bank_scoped_census_tail(
    value: &mut Observation,
    deadline: std::time::Instant,
) -> Result<()> {
    check_deadline(Some(deadline))?;
    super::bank_scoped_census_tail::validate_file(value)?;
    publish_inner(
        value,
        Some(deadline),
        super::NativePolicy::BankScopedCensusTail,
    )
}
pub(super) fn check_deadline(deadline: Option<std::time::Instant>) -> Result<()> {
    require(
        deadline.is_none_or(|end| std::time::Instant::now() < end),
        "full2303 scoped publication deadline",
    )
}
fn publish_pending(
    path: &Path,
    directory: &Path,
    deadline: Option<std::time::Instant>,
) -> Result<()> {
    check_deadline(deadline)?;
    let complete = directory.join("complete.json");
    std::fs::hard_link(path, &complete).map_err(|e| e.to_string())?;
    let result = (|| {
        check_deadline(deadline)?;
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
        File::open(directory)
            .and_then(|dir| dir.sync_all())
            .map_err(|e| e.to_string())?;
        check_deadline(deadline)
    })();
    if result.is_err() && deadline.is_some() {
        // Remove only the link successfully created above, never an existing publication.
        std::fs::remove_file(&complete).map_err(|e| e.to_string())?;
        File::open(directory)
            .and_then(|dir| dir.sync_all())
            .map_err(|e| e.to_string())?;
    }
    result
}
fn publish_inner(
    value: &mut Observation,
    deadline: Option<std::time::Instant>,
    policy: super::NativePolicy,
) -> Result<()> {
    check_deadline(deadline)?;
    super::validate_summary(value)?;
    verify(
        &value.files.frames,
        u64::from(long::FORWARDS) * (long::RECORD_BYTES as u64 + 1),
    )?;
    verify(&value.files.child_stderr, STDERR_BYTES)?;
    verify(&value.files.decoded_output, DECODED_BYTES)?;
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
            "full2303 complete summary bound",
        )?;
        if value.files.summary_bytes == raw.len() as u64 {
            break;
        }
        value.files.summary_bytes = raw.len() as u64;
        value.files.total_bytes = value.files.bytes_before_summary + raw.len() as u64;
    }
    require(
        value.files.summary_bytes == raw.len() as u64
            && value.files.total_bytes + RESERVE <= long::EVIDENCE_BYTES as u64
            && value.files.total_bytes + RESERVE <= long::MAX_RETAINED_BYTES as u64,
        "full2303 final exact accounting",
    )?;
    match policy {
        super::NativePolicy::Full => {}
        super::NativePolicy::ScopedWarm => {
            super::scoped::validate_stdout_bound(value)?;
        }
        super::NativePolicy::BankScopedCensus => {
            super::bank_scoped_census::validate_stdout_bound(value)?;
        }
        super::NativePolicy::BankScopedCensusTail => {
            super::bank_scoped_census_tail::validate_stdout_bound(value)?;
        }
    }
    let directory = &value.request.base.evidence_directory;
    let path = directory.join("complete.pending");
    let mut file = Retained::create(path.clone(), SUMMARY_BYTES)?;
    file.append(&raw)?;
    file.finish()?;
    publish_pending(&path, directory, deadline)
}

#[cfg(test)]
mod scoped_publication_tests {
    use super::*;
    #[test]
    fn full2303_scoped_publication_refuses_expired_post_write_deadline() {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let directory = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "ferric-full2303-scoped-publication-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("complete.pending");
        let mut file = Retained::create(path.clone(), SUMMARY_BYTES).unwrap();
        file.append(b"synthetic, not a native observation\n")
            .unwrap();
        file.finish().unwrap();
        let expired = std::time::Instant::now();
        assert!(check_deadline(Some(expired)).is_err());
        assert!(publish_pending(&path, &directory, Some(expired)).is_err());
        assert!(path.is_file());
        assert!(!directory.join("complete.json").exists());
        std::fs::write(directory.join("complete.json"), b"existing publication").unwrap();
        let future = std::time::Instant::now() + std::time::Duration::from_secs(60);
        assert!(publish_pending(&path, &directory, Some(future)).is_err());
        assert_eq!(
            std::fs::read(directory.join("complete.json")).unwrap(),
            b"existing publication"
        );
        assert!(path.is_file());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
