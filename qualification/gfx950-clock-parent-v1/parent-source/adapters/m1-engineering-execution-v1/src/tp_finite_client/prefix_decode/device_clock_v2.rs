//! Explicit raw clock samples plus the unchanged raw completion-tick report.
use super::host_policy_v2::{pin_file, preflight};
use super::{Config, FilePin, Observation, Result, device_v1, hash, require};
use crate::prefix_decode_device_clock_observation_v2 as data;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const REQUEST_SCHEMA: &str = "FerricFinitePrefixDecodeDeviceClockRequestV2";
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
            "clock request byte bound",
        )?;
        let value: Self = serde_json::from_slice(raw).map_err(|error| error.to_string())?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        require(self.schema == REQUEST_SCHEMA, "clock request schema")?;
        self.decode.validate()
    }
}

fn sidecar_path(config: &Config) -> Result<PathBuf> {
    let name = config
        .evidence_directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("clock evidence UTF8 filename")?;
    Ok(config
        .evidence_directory
        .with_file_name(format!("{name}-device-clock-v2.json")))
}

pub(super) fn validate(path: &Path, observation: &Observation) -> Result<FilePin> {
    device_v1::require_closed(observation)?;
    let pin = pin_file(path, data::MAX_BYTES as u64)?;
    let raw = pin.read(data::MAX_BYTES as u64, true)?;
    // The V2 decoder joins all 16 samples to this raw report's rank identities.
    let report = data::Report::decode(&raw).map_err(|error| error.to_string())?;
    // Retain every V1 Control/payload/source join and charge the entire V2 file.
    device_v1::validate_report(&report.raw, observation, pin.bytes)?;
    pin.read(data::MAX_BYTES as u64, false)?;
    Ok(pin)
}

#[derive(Serialize)]
pub struct Diagnostic {
    pub schema: &'static str,
    pub parent_binary: FilePin,
    pub request_projection_sha256: [u8; 32],
    pub observation: Observation,
    pub device_sidecar: FilePin,
    pub raw_completion_ticks: bool,
    pub raw_clock_counters: bool,
    pub clock_domain_validated: bool,
    pub calibrated_nanoseconds: bool,
    pub cross_device_clock_alignment: bool,
    pub overlap_claim: bool,
    pub performance_claim: bool,
    pub numerical_acceptance: bool,
    pub full_model_acceptance: bool,
    pub production_authority: bool,
}
pub fn run(request: Request, allow: bool) -> Result<Diagnostic> {
    require(allow, "clock diagnostic explicit engineering opt-in")?;
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
        Some(super::HostDiagnostic::DeviceClock(path)),
    )?;
    parent.read(512 << 20, false)?;
    let device_sidecar = device_sidecar.ok_or("clock sidecar missing after owned Close")?;
    device_sidecar.read(data::MAX_BYTES as u64, false)?;
    super::evidence::publish(&mut observation)?;
    let value = Diagnostic {
        schema: "FerricFinitePrefixDecodeDeviceClockDiagnosticV2",
        parent_binary: parent,
        request_projection_sha256: request_sha,
        observation,
        device_sidecar,
        raw_completion_ticks: true,
        raw_clock_counters: true,
        clock_domain_validated: false,
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
        "clock diagnostic unchanged stdout bound",
    )?;
    Ok(value)
}

#[cfg(test)]
#[path = "device_clock_v2_tests.rs"]
mod tests;
