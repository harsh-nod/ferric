//! One-factor host-policy request, bound to actual worker snapshots after reap.
use super::{Config, FilePin, Observation, Result, hash, require, wire};
use crate::prefix_decode_host_observation_v2::{self as data, Policy};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const REQUEST_SCHEMA: &str = "FerricFinitePrefixDecodeHostPolicyRequestV2";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub policy: Policy,
    pub decode: Config,
}
impl Request {
    pub fn parse(raw: &[u8]) -> Result<Self> {
        require(
            !raw.is_empty() && raw.len() <= 65536,
            "host policy request byte bound",
        )?;
        let value: Self = serde_json::from_slice(raw).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        require(self.schema == REQUEST_SCHEMA, "host policy request schema")?;
        self.decode.validate()
    }
}

pub(super) fn preflight(path: &Path) -> Result<()> {
    let parent = path.parent().ok_or("host sidecar parent")?;
    require(
        path.is_absolute()
            && path.file_name().is_some()
            && path.as_os_str().len() <= 512
            && parent.canonicalize().map_err(|e| e.to_string())? == parent,
        "host sidecar absolute canonical parent",
    )?;
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err("host sidecar must be absent before child start".into()),
    }
}
fn sidecar_path(config: &Config) -> Result<PathBuf> {
    let name = config
        .evidence_directory
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("host evidence UTF8 filename")?;
    Ok(config
        .evidence_directory
        .with_file_name(format!("{name}-host-policy-v2.json")))
}
pub(super) fn pin_file(path: &Path, limit: u64) -> Result<FilePin> {
    require(
        path.is_absolute() && path.canonicalize().map_err(|e| e.to_string())? == path,
        "host retained file canonical path",
    )?;
    let meta = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    require(
        meta.is_file() && meta.len() > 0 && meta.len() <= limit,
        "host retained file extent",
    )?;
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut digest = Sha256::new();
    let mut bytes = 0_u64;
    let mut buf = [0; 65536];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        bytes = bytes
            .checked_add(n as u64)
            .ok_or("host retained file overflow")?;
        require(bytes <= limit, "host retained file grew")?;
        digest.update(&buf[..n]);
    }
    require(bytes == meta.len(), "host retained file extent changed")?;
    let pin = FilePin {
        path: path.to_owned(),
        bytes,
        sha256: digest.finalize().into(),
    };
    pin.read(limit, false)?;
    Ok(pin)
}
pub(super) fn validate(path: &Path, observation: &Observation, policy: Policy) -> Result<FilePin> {
    require(
        observation.child_exit_zero
            && observation.process_group_absent
            && observation.native_closed
            && observation.completed_forwards == 4,
        "host sidecar cannot precede closed/reaped Four",
    )?;
    let pin = pin_file(path, data::MAX_BYTES as u64)?;
    let raw = pin.read(data::MAX_BYTES as u64, true)?;
    let report = data::Report::decode(&raw).map_err(|e| e.to_string())?;
    require(
        report.policy == policy,
        "actual host policy differs from request",
    )?;
    require(
        report.child_pid == observation.child_pid
            && report.worker_sha256 == observation.request.worker.sha256
            && report.profile_sha256 == observation.profile_sha256
            && report.transcript_sha256 == observation.transcript_sha256
            && report.bootstrap.registration == observation.registration_sha256
            && report.bootstrap.begin.source_program.sha256 == observation.source_program_sha256
            && report.bootstrap.begin.uploads.sha256 == observation.upload_manifest_sha256
            && report.bootstrap.scope.bundle_id == observation.request.expected_bundle_id
            && report.bootstrap.scope.model_id == observation.request.expected_model_id
            && report.bootstrap.scope.session == observation.request.session
            && report.bootstrap.device_ids == observation.request.device_ids
            && report.bootstrap.timeout_ms == observation.request.dispatch_timeout_ms
            && report.bootstrap.mode == observation.request.mode
            && report.bootstrap.prefix_image.sha256 == observation.request.prefix_image.sha256
            && u64::from(report.bootstrap.prefix_image.bytes)
                == observation.request.prefix_image.bytes
            && report.bootstrap.tiles_image.sha256 == observation.request.tiles_image.sha256
            && u64::from(report.bootstrap.tiles_image.bytes)
                == observation.request.tiles_image.bytes
            && serde_json::to_vec(&report.bootstrap).map_err(|e| e.to_string())?
                == serde_json::to_vec(&observation.bootstrap).map_err(|e| e.to_string())?
            && observation.files.frames.len() == 4,
        "host sidecar actual worker/profile/source/images differs",
    )?;
    for (completed, frame) in report.completions.iter().zip(&observation.files.frames) {
        require(
            matches!(&frame.response.event, wire::Event::Completed(c) if c == completed),
            "host sidecar actual control/payload completion differs",
        )?;
        frame.control.read(wire::CONTROL_BYTES as u64, false)?;
        frame
            .observation
            .read(super::old::OBSERVATION_BYTES as u64, false)?;
    }
    require(
        observation
            .files
            .bytes_before_summary
            .checked_add(pin.bytes)
            .and_then(|n| n.checked_add(super::evidence::SUMMARY_LIMIT as u64))
            .is_some_and(|n| n <= 8 << 20),
        "host diagnostic aggregate8MiB bound",
    )?;
    Ok(pin)
}
#[derive(Serialize)]
pub struct Diagnostic {
    pub schema: &'static str,
    pub policy: Policy,
    pub parent_binary: FilePin,
    pub request_projection_sha256: [u8; 32],
    pub observation: Observation,
    pub host_sidecar: FilePin,
    pub inclusive_nested_host_scopes: bool,
    pub gpu_time: bool,
    pub performance_claim: bool,
    pub numerical_acceptance: bool,
    pub production_authority: bool,
}
pub fn run(request: Request, allow: bool) -> Result<Diagnostic> {
    require(allow, "host diagnostic explicit engineering opt-in")?;
    request.validate()?;
    let path = sidecar_path(&request.decode)?;
    preflight(&path)?;
    let parent_path = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let parent = pin_file(&parent_path, 512 << 20)?;
    let request_sha = hash(&serde_json::to_vec(&request).map_err(|e| e.to_string())?);
    let policy = request.policy;
    let (mut observation, host_sidecar) = super::run_with_diagnostic(
        request.decode,
        true,
        Some(super::HostDiagnostic::V2(path, policy)),
    )?;
    parent.read(512 << 20, false)?;
    let host_sidecar = host_sidecar.ok_or("host sidecar missing after closed child")?;
    host_sidecar.read(data::MAX_BYTES as u64, false)?;
    super::evidence::publish(&mut observation)?;
    let value = Diagnostic {
        schema: "FerricFinitePrefixDecodeHostDiagnosticV2",
        policy,
        parent_binary: parent,
        request_projection_sha256: request_sha,
        observation,
        host_sidecar,
        inclusive_nested_host_scopes: true,
        gpu_time: false,
        performance_claim: false,
        numerical_acceptance: false,
        production_authority: false,
    };
    require(
        serde_json::to_vec(&value).map_err(|e| e.to_string())?.len() < 65536,
        "host diagnostic unchanged stdout bound",
    )?;
    Ok(value)
}

#[cfg(test)]
#[path = "host_policy_v2_tests.rs"]
mod tests;
