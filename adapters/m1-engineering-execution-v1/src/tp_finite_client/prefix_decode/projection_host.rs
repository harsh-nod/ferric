//! Separately named projection AR4 observations, published only after EOF/reap.
use super::projection::{ProjectionDecodeConfig, ProjectionDecodeObservation};
use super::{FilePin, HostDiagnostic, Observation, Result, hash, require, wire};
use crate::projection_residual_decode_host_observation_v1 as data;
use serde::Serialize;
use std::path::{Path, PathBuf};

fn sidecar_path(config: &ProjectionDecodeConfig) -> Result<PathBuf> {
    let name = config
        .decode
        .evidence_directory
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("projection host evidence filename")?;
    Ok(config
        .decode
        .evidence_directory
        .with_file_name(format!("{name}-projection-host-observation.json")))
}
pub(super) fn validate(path: &Path, actual: &Observation, image: &FilePin) -> Result<FilePin> {
    require(
        actual.child_exit_zero
            && actual.process_group_absent
            && actual.native_closed
            && actual.completed_forwards == 4
            && actual.files.frames.len() == 4,
        "projection host report requires actual Close/EOF/reap",
    )?;
    let pin = super::host_observation::pin_file(path, data::MAX_BYTES as u64)?;
    let report = data::Report::decode(&pin.read(data::MAX_BYTES as u64, true)?)
        .map_err(|e| e.to_string())?;
    validate_binding(&report, actual, image)?;
    for (completed, frame) in report.completions.iter().zip(&actual.files.frames) {
        require(
            matches!(&frame.response.event, wire::Event::Completed(value) if value == completed),
            "projection host completion/control/payload differs",
        )?;
        frame.control.read(wire::CONTROL_BYTES as u64, false)?;
        frame
            .observation
            .read(super::old::OBSERVATION_BYTES as u64, false)?;
    }
    require(
        actual
            .files
            .bytes_before_summary
            .checked_add(pin.bytes)
            .and_then(|n| n.checked_add(super::evidence::SUMMARY_LIMIT as u64))
            .is_some_and(|n| n <= 8 << 20),
        "projection host aggregate8MiB bound",
    )?;
    Ok(pin)
}
fn validate_binding(report: &data::Report, actual: &Observation, image: &FilePin) -> Result<()> {
    report.validate().map_err(|e| e.to_string())?;
    require(
        actual.request.mode == wire::InputMode::Autoregressive
            && report.child_pid == actual.child_pid
            && report.worker_sha256 == actual.request.worker.sha256
            && report.profile_sha256 == actual.profile_sha256
            && report.transcript_sha256 == actual.transcript_sha256
            && report.bootstrap == super::projection::bootstrap(actual.bootstrap.clone(), image)?
            && report.bootstrap.decode.registration == actual.registration_sha256
            && report.bootstrap.decode.begin.source_program.sha256 == actual.source_program_sha256
            && report.bootstrap.decode.begin.uploads.sha256 == actual.upload_manifest_sha256,
        "projection host actual worker/bootstrap/profile/source mismatch",
    )
}
#[derive(Serialize)]
pub struct Diagnostic {
    pub schema: &'static str,
    pub parent_binary: FilePin,
    pub request_projection_sha256: [u8; 32],
    pub observation: ProjectionDecodeObservation,
    pub host_sidecar: FilePin,
    pub inclusive_nested_host_scopes: bool,
    pub gpu_time: bool,
    pub performance_claim: bool,
    pub numerical_acceptance: bool,
    pub production_authority: bool,
}
pub fn run(config: ProjectionDecodeConfig, allow: bool) -> Result<Diagnostic> {
    require(allow, "projection host explicit engineering opt-in")?;
    config.validate()?;
    require(
        config.decode.mode == wire::InputMode::Autoregressive,
        "projection host AR4 only",
    )?;
    let path = sidecar_path(&config)?;
    super::host_observation::preflight(&path)?;
    let parent_path = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let parent = super::host_observation::pin_file(&parent_path, 512 << 20)?;
    let request_sha = hash(&serde_json::to_vec(&config).map_err(|e| e.to_string())?);
    let (actual, sidecar, bootstrap) = super::run_selected(
        config.decode.clone(),
        true,
        Some(HostDiagnostic::Projection(
            path,
            config.projection_residual_image.clone(),
        )),
        Some(&config.projection_residual_image),
    )?;
    parent.read(512 << 20, false)?;
    let host_sidecar = sidecar.ok_or("projection host sidecar absent")?;
    host_sidecar.read(data::MAX_BYTES as u64, false)?;
    let mut observation = ProjectionDecodeObservation::closed(
        config,
        actual,
        bootstrap.ok_or("projection host bootstrap absent")?,
    )?;
    observation.publish()?;
    let result = Diagnostic {
        schema: "FerricFiniteProjectionResidualDecodeHostDiagnosticV1",
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
        serde_json::to_vec(&result)
            .map_err(|e| e.to_string())?
            .len()
            < 65536,
        "projection host stdout bound",
    )?;
    Ok(result)
}
