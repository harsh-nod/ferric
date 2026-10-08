//! Opt-in, authority-free capture of the admitted input to pre-ranked lowering.

use fe2o3_mir_model::semantic_mir_v1::AdmittedInertSemanticMirV1;
use std::fs::OpenOptions;
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

const CAPTURE_PATH_ENV_V1: &str = "FE2O3_DIAGNOSTIC_SEMANTIC_MIR_PATH_V1";
const MAX_CAPTURE_BYTES_V1: usize = 16 * 1024 * 1024;
const SOURCE_MAP_PATH_ENV_V1: &str = "FE2O3_DIAGNOSTIC_SEMANTIC_SOURCE_MAP_PATH_V1";
const MAX_SOURCE_MAP_BYTES_V1: usize = 1024 * 1024;
const MAX_SOURCE_MAP_FILES_V1: usize = 4096;

/// Captures existing canonical bytes without changing lowering or its result.
/// Neither snapshot is consumed by production admission or grants authority.
pub(super) fn capture_before_llvm_v1(neutral: &[u8], target: &[u8]) {
    use sha2::{Digest, Sha256};
    for (variable, stage, bytes) in [
        ("FE2O3_DIAGNOSTIC_NEUTRAL_KIR_PATH_V1", "neutral", neutral),
        (
            "FE2O3_DIAGNOSTIC_TARGET_KIR_PATH_V1",
            "target-before-llvm",
            target,
        ),
    ] {
        let Some(path) = std::env::var_os(variable) else {
            continue;
        };
        let result = write_new_capture_bytes_v1(Path::new(&path), bytes, MAX_CAPTURE_BYTES_V1)
            .map_err(|error| error.kind());
        let digest: [u8; 32] = Sha256::digest(bytes).into();
        let status = if result.is_ok() { "complete" } else { "failed" };
        let _ = writeln!(
            io::stderr().lock(),
            "fe2o3 diagnostic KIR: stage={stage} status={status} bytes={} sha256={} error_kind={:?}",
            bytes.len(),
            crate::encode_hex(&digest),
            result.err()
        );
    }
}

struct SemanticCaptureOutcomeV1 {
    bytes: usize,
    semantic_sha256: [u8; 32],
    result: Result<(), io::ErrorKind>,
}

/// The destination must be selected outside any temporary build directory by
/// the caller. This diagnostic is never consumed as compiler admission evidence.
pub(super) fn capture_before_pre_ranked_v1(
    semantic: &AdmittedInertSemanticMirV1,
    files: &[fe2o3_kernel_ir::DebugSourceMapFileV1],
) {
    let Some(destination) = std::env::var_os(CAPTURE_PATH_ENV_V1) else {
        return;
    };
    if let Some(outcome) = capture_requested_semantic_v1(Some(Path::new(&destination)), semantic) {
        // A failed snapshot or stderr write must not replace the compiler result.
        let _ = write_capture_receipt_v1(&mut io::stderr().lock(), &outcome);
        if let Some(map) = capture_requested_source_map_v1(
            std::env::var_os(SOURCE_MAP_PATH_ENV_V1)
                .as_deref()
                .map(Path::new),
            semantic,
            files,
            &outcome,
        ) {
            let _ = write_source_map_receipt_v1(&mut io::stderr().lock(), &map);
        }
    }
}

#[derive(serde::Serialize)]
struct SemanticSourceMapCaptureV1<'a> {
    schema: &'static str,
    stage: &'static str,
    semantic_bytes: usize,
    semantic_sha256: String,
    semantic_file_sha256: String,
    files: &'a [fe2o3_kernel_ir::DebugSourceMapFileV1],
    execution_authority: bool,
}

struct BoundedSourceMapBytesV1 {
    bytes: Vec<u8>,
    maximum: usize,
}

impl Write for BoundedSourceMapBytesV1 {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let end = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .filter(|end| *end <= self.maximum)
            .ok_or(io::ErrorKind::InvalidInput)?;
        self.bytes
            .try_reserve(end - self.bytes.len())
            .map_err(|_| io::ErrorKind::OutOfMemory)?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn encode_source_map_v1(
    semantic: &AdmittedInertSemanticMirV1,
    files: &[fe2o3_kernel_ir::DebugSourceMapFileV1],
    maximum: usize,
) -> io::Result<Vec<u8>> {
    use sha2::{Digest, Sha256};
    if files.len() > MAX_SOURCE_MAP_FILES_V1
        || files
            .windows(2)
            .any(|pair| pair[0].identity() >= pair[1].identity())
    {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let record = SemanticSourceMapCaptureV1 {
        schema: "fe2o3-diagnostic-semantic-source-map-v1",
        stage: "pre-ranked",
        semantic_bytes: semantic.canonical_encoding().len(),
        semantic_sha256: crate::encode_hex(semantic.semantic_sha256().as_bytes()),
        semantic_file_sha256: crate::encode_hex(&Sha256::digest(semantic.canonical_encoding())),
        files,
        execution_authority: false,
    };
    let mut output = BoundedSourceMapBytesV1 {
        bytes: Vec::new(),
        maximum,
    };
    serde_json::to_writer(&mut output, &record)
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
    output.write_all(b"\n")?;
    Ok(output.bytes)
}

fn capture_requested_source_map_v1(
    destination: Option<&Path>,
    semantic: &AdmittedInertSemanticMirV1,
    files: &[fe2o3_kernel_ir::DebugSourceMapFileV1],
    semantic_outcome: &SemanticCaptureOutcomeV1,
) -> Option<SemanticCaptureOutcomeV1> {
    let destination = destination?;
    // A sidecar never upgrades an absent or partial semantic snapshot.
    if semantic_outcome.result.is_err() {
        return None;
    }
    let mut outcome = SemanticCaptureOutcomeV1 {
        bytes: 0,
        semantic_sha256: *semantic.semantic_sha256().as_bytes(),
        result: Ok(()),
    };
    if semantic_outcome.bytes != semantic.canonical_encoding().len()
        || semantic_outcome.semantic_sha256 != *semantic.semantic_sha256().as_bytes()
    {
        outcome.result = Err(io::ErrorKind::InvalidData);
        return Some(outcome);
    }
    outcome.result = encode_source_map_v1(semantic, files, MAX_SOURCE_MAP_BYTES_V1)
        .and_then(|bytes| {
            outcome.bytes = bytes.len();
            write_new_capture_bytes_v1(destination, &bytes, MAX_SOURCE_MAP_BYTES_V1)
        })
        .map_err(|error| error.kind());
    Some(outcome)
}

fn write_source_map_receipt_v1(
    output: &mut impl Write,
    outcome: &SemanticCaptureOutcomeV1,
) -> io::Result<()> {
    let status = if outcome.result.is_ok() {
        "complete"
    } else {
        "failed"
    };
    writeln!(
        output,
        "fe2o3 diagnostic semantic source map: stage=pre-ranked status={status} bytes={} semantic_sha256={} error_kind={:?}",
        outcome.bytes,
        crate::encode_hex(&outcome.semantic_sha256),
        outcome.result.as_ref().err(),
    )
}

fn capture_requested_semantic_v1(
    destination: Option<&Path>,
    semantic: &AdmittedInertSemanticMirV1,
) -> Option<SemanticCaptureOutcomeV1> {
    let destination = destination?;
    let bytes = semantic.canonical_encoding();
    Some(SemanticCaptureOutcomeV1 {
        bytes: bytes.len(),
        semantic_sha256: *semantic.semantic_sha256().as_bytes(),
        result: write_new_capture_bytes_v1(destination, bytes, MAX_CAPTURE_BYTES_V1)
            .map_err(|error| error.kind()),
    })
}

fn write_new_capture_bytes_v1(output: &Path, bytes: &[u8], maximum: usize) -> io::Result<()> {
    if bytes.is_empty() || bytes.len() > maximum || !output.is_absolute() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(output)?;
    // Retain a partial create-new file on failure; never report it as complete.
    file.write_all(bytes)?;
    file.sync_all()?;
    #[cfg(unix)]
    {
        let descriptor = file.metadata()?;
        let path = std::fs::symlink_metadata(output)?;
        if !descriptor.is_file()
            || descriptor.len() != bytes.len() as u64
            || descriptor.dev() != path.dev()
            || descriptor.ino() != path.ino()
            || path.file_type().is_symlink()
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
    }
    Ok(())
}

fn write_capture_receipt_v1(
    output: &mut impl Write,
    outcome: &SemanticCaptureOutcomeV1,
) -> io::Result<()> {
    let (status, error_kind) = match outcome.result {
        Ok(()) => ("complete", None),
        Err(kind) => ("failed", Some(kind)),
    };
    writeln!(
        output,
        "fe2o3 diagnostic semantic MIR: stage=pre-ranked status={status} bytes={} semantic_sha256={} error_kind={error_kind:?}",
        outcome.bytes,
        crate::encode_hex(&outcome.semantic_sha256),
    )
}

#[cfg(test)]
#[path = "diagnostic_semantic_capture_v1_tests.rs"]
mod tests;
