//! Ordered AR4 shared-full observation, published only after Close/EOF/reap.
use super::projection_ordered::{OrderedDecodeConfig, OrderedDecodeObservation};
use super::{FilePin, HostDiagnostic, Observation, Result, hash, require, wire};
use crate::projection_residual_mlp_ordered_observation_v1 as data;
use serde::Serialize;
use std::path::{Path, PathBuf};

fn sidecar_path(config: &OrderedDecodeConfig, policy: data::Policy) -> Result<PathBuf> {
    let name = config
        .decode
        .evidence_directory
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("projection host evidence filename")?;
    let suffix = "projection-residual-mlp-ordered";
    let _ = policy;
    Ok(config
        .decode
        .evidence_directory
        .with_file_name(format!("{name}-{suffix}-observation.json")))
}
pub(super) fn validate(path: &Path, actual: &Observation, image: &FilePin) -> Result<FilePin> {
    validate_policy(path, actual, image, data::Policy::OrderedSharedFull)
}
fn validate_policy(
    path: &Path,
    actual: &Observation,
    image: &FilePin,
    policy: data::Policy,
) -> Result<FilePin> {
    require(
        actual.child_exit_zero
            && actual.process_group_absent
            && actual.native_closed
            && actual.completed_forwards == 4
            && actual.files.frames.len() == 4,
        "projection host report requires actual Close/EOF/reap",
    )?;
    let pin = super::host_observation::pin_file(path, data::MAX_BYTES as u64)?;
    let raw = pin.read(data::MAX_BYTES as u64, true)?;
    let report = data::SharedReport::decode(&raw)
        .map_err(|e| e.to_string())?
        .observation;
    validate_binding(&report, actual, image, policy)?;
    for (completed, frame) in report.completions.iter().zip(&actual.files.frames) {
        require(
            matches!(&frame.response.event, wire::Event::Completed(value) if value == completed),
            "projection host completion/control/payload differs",
        )?;
        frame.control.read(
            crate::finite_projection_residual_mlp_ordered_wire_v1::CONTROL_BYTES as u64,
            false,
        )?;
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
fn validate_binding(
    report: &data::Report,
    actual: &Observation,
    image: &FilePin,
    policy: data::Policy,
) -> Result<()> {
    report
        .validate_for_policy(policy)
        .map_err(|e| e.to_string())?;
    require(
        actual.request.mode == wire::InputMode::Autoregressive
            && report.child_pid == actual.child_pid
            && report.worker_sha256 == actual.request.worker.sha256
            && report.profile_sha256 == actual.profile_sha256
            && report.transcript_sha256 == actual.transcript_sha256
            && report.bootstrap
                == super::projection_ordered::bootstrap(actual.bootstrap.clone(), image)?
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
    pub observation: OrderedDecodeObservation,
    pub host_sidecar: FilePin,
    pub inclusive_nested_host_scopes: bool,
    pub gpu_time: bool,
    pub performance_claim: bool,
    pub numerical_acceptance: bool,
    pub production_authority: bool,
}
pub fn run(config: OrderedDecodeConfig, allow: bool) -> Result<Diagnostic> {
    run_policy(config, allow, data::Policy::OrderedSharedFull)
}
fn run_policy(
    config: OrderedDecodeConfig,
    allow: bool,
    policy: data::Policy,
) -> Result<Diagnostic> {
    require(allow, "projection host explicit engineering opt-in")?;
    config.validate()?;
    require(
        config.decode.mode == wire::InputMode::Autoregressive,
        "projection host AR4 only",
    )?;
    let path = sidecar_path(&config, policy)?;
    super::host_observation::preflight(&path)?;
    let parent_path = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let parent = super::host_observation::pin_file(&parent_path, 512 << 20)?;
    let request_sha = hash(&serde_json::to_vec(&config).map_err(|e| e.to_string())?);
    let diagnostic = HostDiagnostic::Ordered(path, config.projection_residual_image.clone());
    let (actual, sidecar, bootstrap) = super::run_selected(
        config.decode.clone(),
        true,
        Some(diagnostic),
        Some(&config.projection_residual_image),
    )?;
    parent.read(512 << 20, false)?;
    let host_sidecar = sidecar.ok_or("projection host sidecar absent")?;
    host_sidecar.read(data::MAX_BYTES as u64, false)?;
    let mut observation = OrderedDecodeObservation::closed(
        config,
        actual,
        super::projection_ordered::selected(&bootstrap.ok_or("ordered bootstrap absent")?),
    )?;
    observation.publish()?;
    let result = Diagnostic {
        schema: "FerricFiniteProjectionResidualMlpOrderedHostDiagnosticV1",
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
