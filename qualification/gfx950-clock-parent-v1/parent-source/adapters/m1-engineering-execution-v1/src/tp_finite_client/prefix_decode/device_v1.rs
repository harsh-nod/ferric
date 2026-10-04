//! Explicit raw completion ticks, joined only after the owned Four child closes.
use super::host_policy_v2::{pin_file, preflight};
use super::{Config, FilePin, Observation, Result, hash, require, wire};
use crate::finite_setup_wire_v1::Part;
use crate::prefix_decode_device_observation_v1 as data;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const REQUEST_SCHEMA: &str = "FerricFinitePrefixDecodeDeviceRequestV1";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub decode: Config,
}
impl Request {
    pub fn parse(raw: &[u8]) -> Result<Self> {
        require(
            !raw.is_empty() && raw.len() <= 65536,
            "device request byte bound",
        )?;
        let value: Self = serde_json::from_slice(raw).map_err(|error| error.to_string())?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        require(self.schema == REQUEST_SCHEMA, "device request schema")?;
        self.decode.validate()
    }
}

fn sidecar_path(config: &Config) -> Result<PathBuf> {
    let name = config
        .evidence_directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("device evidence UTF8 filename")?;
    Ok(config
        .evidence_directory
        .with_file_name(format!("{name}-device-v1.json")))
}

fn same_part(part: Part, pin: &FilePin) -> bool {
    u64::from(part.bytes) == pin.bytes && part.sha256 == pin.sha256
}

pub(super) fn require_closed(observation: &Observation) -> Result<()> {
    require(
        observation.child_exit_zero
            && observation.process_group_absent
            && observation.native_closed
            && observation.completed_forwards == 4,
        "device sidecar requires closed and reaped Four",
    )?;
    Ok(())
}

pub(super) fn validate(path: &Path, observation: &Observation) -> Result<FilePin> {
    require_closed(observation)?;
    let pin = pin_file(path, data::MAX_BYTES as u64)?;
    let raw = pin.read(data::MAX_BYTES as u64, true)?;
    let report = data::Report::decode(&raw).map_err(|error| error.to_string())?;
    validate_report(&report, observation, pin.bytes)?;
    pin.read(data::MAX_BYTES as u64, false)?;
    Ok(pin)
}

// Both wire versions must join their raw report to the actual retained frames.
pub(super) fn validate_report(
    report: &data::Report,
    observation: &Observation,
    sidecar_bytes: u64,
) -> Result<()> {
    require_closed(observation)?;
    report.validate().map_err(|error| error.to_string())?;
    let bootstrap = &report.bootstrap;
    let tail = bootstrap
        .begin
        .tail_image
        .ok_or("device diagnostic tail image absent")?;
    require(
        report.child_pid == observation.child_pid
            && report.worker_sha256 == observation.request.worker.sha256
            && report.profile_sha256 == observation.profile_sha256
            && report.transcript_sha256 == observation.transcript_sha256
            && bootstrap.registration == observation.registration_sha256
            && bootstrap.begin.source_program.sha256 == observation.source_program_sha256
            && bootstrap.begin.uploads.sha256 == observation.upload_manifest_sha256
            && bootstrap.scope.bundle_id == observation.request.expected_bundle_id
            && bootstrap.scope.model_id == observation.request.expected_model_id
            && bootstrap.scope.session == observation.request.session
            && bootstrap.device_ids == observation.request.device_ids
            && bootstrap.timeout_ms == observation.request.dispatch_timeout_ms
            && bootstrap.mode == observation.request.mode
            && same_part(bootstrap.prefix_image, &observation.request.prefix_image)
            && same_part(bootstrap.tiles_image, &observation.request.tiles_image)
            && same_part(
                bootstrap.begin.prefix_image,
                &observation.request.images.prefix,
            )
            && same_part(bootstrap.begin.mlp_image, &observation.request.images.mlp)
            && same_part(
                bootstrap.begin.residual_image,
                &observation.request.images.residual,
            )
            && same_part(tail, &observation.request.images.tail)
            && report.images.tail == tail.sha256
            && report.images.copy == bootstrap.begin.residual_image.sha256
            && serde_json::to_vec(bootstrap).map_err(|error| error.to_string())?
                == serde_json::to_vec(&observation.bootstrap).map_err(|error| error.to_string())?
            && observation.files.frames.len() == 4,
        "device sidecar actual worker/PID/profile/source/images differ",
    )?;
    for (position, (completed, frame)) in report
        .completions
        .iter()
        .zip(&observation.files.frames)
        .enumerate()
    {
        require(
            matches!(&frame.response.event, wire::Event::Completed(value) if value == completed)
                && same_part(completed.control, &frame.control)
                && same_part(completed.observation, &frame.observation),
            "device sidecar actual completion/control/payload differs",
        )?;
        // Decode actual retained bytes, not a caller-supplied timing projection.
        let control = wire::Control::decode(&frame.control.read(wire::CONTROL_BYTES as u64, true)?)
            .map_err(|error| error.to_string())?;
        report
            .validate_control(
                u32::try_from(position).map_err(|error| error.to_string())?,
                &control,
            )
            .map_err(|error| error.to_string())?;
        frame
            .observation
            .read(super::old::OBSERVATION_BYTES as u64, false)?;
    }
    require(
        observation
            .files
            .bytes_before_summary
            .checked_add(sidecar_bytes)
            .and_then(|bytes| bytes.checked_add(super::evidence::SUMMARY_LIMIT as u64))
            .is_some_and(|bytes| bytes <= 8 << 20),
        "device diagnostic aggregate8MiB bound",
    )?;
    Ok(())
}

#[derive(Serialize)]
pub struct Diagnostic {
    pub schema: &'static str,
    pub parent_binary: FilePin,
    pub request_projection_sha256: [u8; 32],
    pub observation: Observation,
    pub device_sidecar: FilePin,
    pub raw_completion_ticks: bool,
    pub calibrated_nanoseconds: bool,
    pub cross_device_clock_alignment: bool,
    pub overlap_claim: bool,
    pub performance_claim: bool,
    pub numerical_acceptance: bool,
    pub full_model_acceptance: bool,
    pub production_authority: bool,
}
pub fn run(request: Request, allow: bool) -> Result<Diagnostic> {
    require(allow, "device diagnostic explicit engineering opt-in")?;
    request.validate()?;
    let path = sidecar_path(&request.decode)?;
    preflight(&path)?;
    let parent_path = std::env::current_exe()
        .map_err(|error| error.to_string())?
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let parent = pin_file(&parent_path, 512 << 20)?;
    let request_sha = hash(&serde_json::to_vec(&request).map_err(|error| error.to_string())?);
    let (mut observation, device_sidecar) = super::run_with_diagnostic(
        request.decode,
        true,
        Some(super::HostDiagnostic::Device(path)),
    )?;
    parent.read(512 << 20, false)?;
    let device_sidecar = device_sidecar.ok_or("device sidecar missing after owned Close")?;
    device_sidecar.read(data::MAX_BYTES as u64, false)?;
    super::evidence::publish(&mut observation)?;
    let value = Diagnostic {
        schema: "FerricFinitePrefixDecodeDeviceDiagnosticV1",
        parent_binary: parent,
        request_projection_sha256: request_sha,
        observation,
        device_sidecar,
        raw_completion_ticks: true,
        calibrated_nanoseconds: false,
        cross_device_clock_alignment: false,
        overlap_claim: false,
        performance_claim: false,
        numerical_acceptance: false,
        full_model_acceptance: false,
        production_authority: false,
    };
    require(
        serde_json::to_vec(&value)
            .map_err(|error| error.to_string())?
            .len()
            < 65536,
        "device diagnostic unchanged stdout bound",
    )?;
    Ok(value)
}

#[cfg(test)]
#[path = "device_v1_tests.rs"]
pub(super) mod tests;
