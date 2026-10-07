//! Explicit scoped-warm identity around the unchanged full own-output observation.
use super::*;
use crate::finite_guarded_mlp_full2303_scoped_v1::{MAX_BYTES, PolicyRecord};

pub const SCHEMA: &str = "FerricFull2303ScopedWarmObservationV1";
#[derive(Serialize)]
pub struct ScopedWarmObservation {
    pub schema: &'static str,
    pub observation: Observation,
    pub currentness_policy: PolicyRecord,
}
pub(super) fn admit_entry(schema: &str, opted_in: bool) -> Result<()> {
    require(
        schema == REQUEST_SCHEMA && opted_in,
        "full2303 scoped warm requires explicit Full2303 opt-in",
    )
}
pub(super) fn validate_original(
    raw: &[u8],
    bootstrap: &full::Bootstrap,
    digest: [u8; 32],
    generated_tokens: &[u32],
    worker: [u8; 32],
) -> Result<PolicyRecord> {
    PolicyRecord::decode(raw, bootstrap, digest, generated_tokens, worker)
        .map_err(|e| e.to_string())
}
pub(super) fn read_policy(
    pin: &FilePin,
    bootstrap: &full::Bootstrap,
    digest: [u8; 32],
    generated_tokens: &[u32],
    worker: [u8; 32],
) -> Result<PolicyRecord> {
    let raw = pin.read(MAX_BYTES as u64, true)?;
    validate_original(&raw, bootstrap, digest, generated_tokens, worker)
}
pub(super) fn validate_file(observation: &Observation) -> Result<PolicyRecord> {
    super::validate_summary(observation)?;
    admit_entry(&observation.request.schema, true)?;
    read_policy(
        &observation.files.child_stderr,
        &observation.bootstrap,
        observation.transcript_sha256,
        &observation.generated_tokens,
        observation.request.base.worker.sha256,
    )
}

pub(super) fn check_stdout_bytes(bytes: usize) -> Result<()> {
    require(
        bytes != 0 && bytes <= 128 << 10,
        "full2303 scoped summary stdout bound",
    )
}
pub(super) fn validate_stdout_bound(observation: &Observation) -> Result<usize> {
    #[derive(Serialize)]
    struct Borrowed<'a> {
        schema: &'static str,
        observation: &'a Observation,
        currentness_policy: PolicyRecord,
    }
    let raw = serde_json::to_vec(&Borrowed {
        schema: SCHEMA,
        observation,
        currentness_policy: validate_file(observation)?,
    })
    .map_err(|e| e.to_string())?;
    let bytes = raw
        .len()
        .checked_add(1)
        .ok_or("full2303 scoped stdout overflow")?;
    check_stdout_bytes(bytes)?;
    Ok(bytes)
}
/// Changed currentness cadence, not temporal equivalence or launch admission.
/// Original worker policy bytes are admitted before successful publication.
pub fn run_scoped_warm(
    config: Full2303Config,
    allow_unauthenticated_machine_code: bool,
) -> Result<ScopedWarmObservation> {
    admit_entry(&config.schema, allow_unauthenticated_machine_code)?;
    let observation = super::run_inner(config, NativePolicy::ScopedWarm)?;
    let currentness_policy = validate_file(&observation)?;
    Ok(ScopedWarmObservation {
        schema: SCHEMA,
        observation,
        currentness_policy,
    })
}
