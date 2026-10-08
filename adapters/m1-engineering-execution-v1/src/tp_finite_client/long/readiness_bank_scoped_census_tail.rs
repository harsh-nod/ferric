//! Original policy-file admission for the distinct bank/layer/census/tail route.
use super::*;
#[cfg(any(not(feature = "engineering-currentness-duration-diagnostics"), test))]
use crate::finite_guarded_mlp_readiness_bank_scoped_census_tail_v4::MAX_BYTES;
use crate::finite_guarded_mlp_readiness_bank_scoped_census_tail_v4::PolicyRecord;

pub(super) const TIMED_SCHEMA: &str =
    "FerricReadiness40Position5BankScopedWarmCensusTailTimedObservationV4";

pub(super) fn admit_entry(schema: &str, opted_in: bool) -> Result<()> {
    require(
        schema == POSITION5_REQUEST_SCHEMA && opted_in,
        "bank scoped census tail requires explicit ordinary Position5 opt-in",
    )
}
fn read_policy(
    pin: &FilePin,
    b: &ready::Bootstrap,
    digest: [u8; 32],
    worker: [u8; 32],
) -> Result<PolicyRecord> {
    #[cfg(not(feature = "engineering-currentness-duration-diagnostics"))]
    {
        let raw = pin.read(MAX_BYTES as u64, true)?;
        PolicyRecord::decode(&raw, b, digest, worker).map_err(|e| e.to_string())
    }
    #[cfg(feature = "engineering-currentness-duration-diagnostics")]
    {
        use crate::finite_guarded_mlp_readiness_forward_durations_v1 as diagnostic;
        let raw = pin.read(diagnostic::STDERR_MAX_BYTES as u64, true)?;
        diagnostic::decode_stderr(&raw, b, digest, worker)
            .map(|(policy, _, _)| policy)
            .map_err(|e| e.to_string())
    }
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
        "bank scoped census tail requires healthy ordinary Close",
    )?;
    read_policy(
        &observation.files.child_stderr,
        &observation.bootstrap,
        observation.transcript_sha256,
        observation.request.base.worker.sha256,
    )
}

#[cfg(test)]
#[path = "readiness_bank_scoped_census_tail_tests.rs"]
mod tests;
