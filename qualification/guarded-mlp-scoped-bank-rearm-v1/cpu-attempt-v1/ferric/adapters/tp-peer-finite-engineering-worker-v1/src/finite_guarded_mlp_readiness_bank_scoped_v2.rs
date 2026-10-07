//! Closed data identity for the explicit scoped-warm route, not execution authority.
use crate::finite_guarded_mlp_long_wire_v2 as long;
use crate::finite_guarded_mlp_readiness_wire_v1 as ready;
use serde::{Deserialize, Serialize};
use std::io;

pub const WORKER_FLAG: &str =
    "--engineering-native-guarded-mlp-readiness40-position5-bank-scoped-warm-v2";
pub const SCHEMA: &str = "FerricReadiness40Position5BankScopedWarmPolicyV2";
pub const EXECUTION_PROFILE: &str = "Readiness40Position5BankScopedWarmCurrentnessV2";
pub const MAX_BYTES: usize = 4096;

pub use crate::finite_guarded_mlp_readiness_scoped_v1::Counts as LayerCounts;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BankCounts {
    pub ordinary_initial_banks: u32,
    pub scoped_rearms: u32,
    pub scoped_rearms_by_forward: Vec<u32>,
    pub final_generations: [u64; 2],
    pub full_discoveries: u64,
    pub local_checkpoints: u64,
    pub before_calls: u64,
    pub after_calls: u64,
    pub generation_probes: u64,
}
impl BankCounts {
    pub fn validate_closed(&self) -> io::Result<()> {
        let calls = self
            .local_checkpoints
            .checked_mul(2)
            .and_then(|n| n.checked_add(4 * 38));
        let probes = self
            .local_checkpoints
            .checked_mul(2)
            .and_then(|n| n.checked_add(3 * 38));
        if self.ordinary_initial_banks != 2
            || self.scoped_rearms != 38
            || self.scoped_rearms_by_forward
                != (0..40)
                    .map(|p| if p < 2 { 0 } else { 1 })
                    .collect::<Vec<_>>()
            || self.final_generations != [20, 20]
            || self.full_discoveries != 2 * 38
            || self.local_checkpoints < 38
            || calls != Some(self.before_calls)
            || self.before_calls != self.after_calls
            || probes != Some(self.generation_probes)
        {
            return Err(io::Error::other(
                "bank scoped complete actual rearm/counter census",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Counts {
    pub layers: LayerCounts,
    pub banks: BankCounts,
}
impl Counts {
    pub fn validate_closed(&self) -> io::Result<()> {
        self.layers.validate_closed()?;
        self.banks.validate_closed()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyRecord {
    pub schema: String,
    pub execution_profile: String,
    pub session: [u8; 32],
    pub device_ids: [u64; 2],
    pub child_pid: u32,
    pub worker_sha256: [u8; 32],
    pub profile_sha256: [u8; 32],
    pub registration_sha256: [u8; 32],
    pub transcript_sha256: [u8; 32],
    pub completed_forwards: u32,
    pub generated_tokens: Vec<u32>,
    pub capture_positions: [u32; 4],
    pub counts: Counts,
    pub scoped_warm_currentness: bool,
    pub scoped_bank_rearm: bool,
    pub full_entry_exit_per_scoped_bank: bool,
    pub allocation_preflights_outside_windows: bool,
    pub full_entry_exit_per_scoped_layer: bool,
    pub participant_local_between_boundaries: bool,
    pub scope_includes_prefix_mlp_hidden: bool,
    pub temporal_equivalent_to_full: bool,
    pub default_group_policy_unchanged: bool,
    pub shared_full_currentness: bool,
    pub cache_kernel_admission: bool,
    pub operational_currentness: bool,
    pub host_observer: bool,
    pub paired_hidden_reads: bool,
    pub paired_terminal: bool,
    pub native_closed: bool,
    pub full_long_workload: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
}
impl PolicyRecord {
    pub fn new(
        b: &ready::Bootstrap,
        transcript: [u8; 32],
        worker: [u8; 32],
        counts: Counts,
    ) -> io::Result<Self> {
        b.validate(
            b.sequence.device_ids,
            b.sequence.timeout_ms,
            b.sequence.scope.child_identity,
        )?;
        counts.validate_closed()?;
        if b.sequence.profile != long::Profile::Readiness40Position5
            || transcript == [0; 32]
            || worker == [0; 32]
        {
            return Err(io::Error::other(
                "bank scoped warm requires closed Position5 identity",
            ));
        }
        Ok(Self {
            schema: SCHEMA.into(),
            execution_profile: EXECUTION_PROFILE.into(),
            session: b.sequence.scope.session,
            device_ids: b.sequence.device_ids,
            child_pid: b.sequence.scope.child_identity,
            worker_sha256: worker,
            profile_sha256: b.sequence.sha256()?,
            registration_sha256: b.sequence.registration,
            transcript_sha256: transcript,
            completed_forwards: 40,
            generated_tokens: Vec::new(),
            capture_positions: [0, 5, 16, 39],
            counts,
            scoped_warm_currentness: true,
            scoped_bank_rearm: true,
            full_entry_exit_per_scoped_bank: true,
            allocation_preflights_outside_windows: true,
            full_entry_exit_per_scoped_layer: true,
            participant_local_between_boundaries: true,
            scope_includes_prefix_mlp_hidden: true,
            temporal_equivalent_to_full: false,
            default_group_policy_unchanged: true,
            shared_full_currentness: false,
            cache_kernel_admission: false,
            operational_currentness: false,
            host_observer: false,
            paired_hidden_reads: false,
            paired_terminal: false,
            native_closed: true,
            full_long_workload: false,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        })
    }
    pub fn validate(
        &self,
        b: &ready::Bootstrap,
        transcript: [u8; 32],
        worker: [u8; 32],
    ) -> io::Result<()> {
        if self != &Self::new(b, transcript, worker, self.counts.clone())? {
            return Err(io::Error::other(
                "bank scoped warm exact policy and Close identity",
            ));
        }
        Ok(())
    }
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut raw = serde_json::to_vec(self).map_err(io::Error::other)?;
        raw.push(b'\n');
        if raw.len() > MAX_BYTES {
            return Err(io::Error::other("bank scoped warm record bound"));
        }
        Ok(raw)
    }
    pub fn write_to(&self, w: &mut (impl io::Write + ?Sized)) -> io::Result<()> {
        w.write_all(&self.encode()?)?;
        w.flush()
    }
    pub fn decode(
        raw: &[u8],
        b: &ready::Bootstrap,
        transcript: [u8; 32],
        worker: [u8; 32],
    ) -> io::Result<Self> {
        if raw.is_empty() || raw.len() > MAX_BYTES || raw.last() != Some(&b'\n') {
            return Err(io::Error::other(
                "bank scoped warm single bounded policy record",
            ));
        }
        let value: Self = serde_json::from_slice(raw).map_err(io::Error::other)?;
        value.validate(b, transcript, worker)?;
        if value.encode()? != raw {
            return Err(io::Error::other("bank scoped warm exact record framing"));
        }
        Ok(value)
    }
}

#[cfg(test)]
#[path = "finite_guarded_mlp_readiness_bank_scoped_v2_tests.rs"]
mod tests;
