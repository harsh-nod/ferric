//! Explicit policy wrapper around the unchanged ordinary Position5 observation.
use super::*;
use crate::finite_guarded_mlp_readiness_shared_v1::MAX_BYTES;
use crate::finite_guarded_mlp_readiness_shared_v1::PolicyRecord;

pub const SCHEMA: &str = "FerricReadiness40Position5SharedFullObservationV1";
#[derive(Serialize)]
pub struct SharedFullObservation {
    pub schema: &'static str,
    pub observation: Observation,
    pub currentness_policy: PolicyRecord,
}
pub(super) fn admit_entry(schema: &str, opted_in: bool) -> Result<()> {
    require(
        schema == POSITION5_REQUEST_SCHEMA && opted_in,
        "shared full requires explicit ordinary Position5 opt-in",
    )
}
fn read_policy(
    pin: &FilePin,
    b: &ready::Bootstrap,
    digest: [u8; 32],
    worker: [u8; 32],
) -> Result<PolicyRecord> {
    let raw = pin.read(MAX_BYTES as u64, true)?;
    PolicyRecord::decode(&raw, b, digest, worker).map_err(|e| e.to_string())
}
pub(super) fn validate_file(observation: &Observation) -> Result<PolicyRecord> {
    admit_entry(&observation.request.schema, true)?;
    require(
        observation.native_closed
            && observation.child_exit_zero
            && observation.process_group_absent
            && observation.causal_layer_zero.is_none()
            && observation.completed_forwards == 40
            && observation.generated_tokens.is_empty(),
        "shared full requires healthy ordinary Close",
    )?;
    read_policy(
        &observation.files.child_stderr,
        &observation.bootstrap,
        observation.transcript_sha256,
        observation.request.base.worker.sha256,
    )
}
/// Separate explicit entry. Ordinary wire, captures, deadlines and publication
/// remain unchanged; the worker proves its selected policy only after Close.
pub fn run_position5_shared_full(
    config: ReadinessConfig,
    allow_unauthenticated_machine_code: bool,
) -> Result<SharedFullObservation> {
    admit_entry(&config.schema, allow_unauthenticated_machine_code)?;
    let observation = super::run_inner_policy(
        config,
        allow_unauthenticated_machine_code,
        long::Profile::Readiness40Position5,
        false,
        &mut None,
        true,
    )?;
    super::validate_summary(&observation)?;
    let currentness_policy = validate_file(&observation)?;
    Ok(SharedFullObservation {
        schema: SCHEMA,
        observation,
        currentness_policy,
    })
}

#[cfg(test)]
#[path = "readiness_shared_full_tests.rs"]
mod tests;
