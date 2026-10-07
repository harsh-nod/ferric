//! Closed data identity for the explicit scoped-warm route, not execution authority.
use crate::finite_guarded_mlp_full2303_wire_v1 as full;
use crate::finite_guarded_mlp_long_wire_v2 as long;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io;

pub const WORKER_FLAG: &str = "--engineering-native-guarded-mlp-full2303-scoped-warm-v1";
pub const SCHEMA: &str = "FerricFull2303ScopedWarmPolicyV1";
pub const EXECUTION_PROFILE: &str = "Full2303ScopedWarmCurrentnessV1";
pub const MAX_BYTES: usize = 4096;
const SCOPED_LAYERS: u64 = 2301 * 36;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Counts {
    pub ordinary_layers: u32,
    pub scoped_layers: u32,
    // Only discoveries inside the scoped windows, not all route preflights.
    pub full_discoveries: u64,
    pub local_checkpoints: u64,
    pub before_calls: u64,
    pub after_calls: u64,
    pub generation_probes: u64,
}
impl Counts {
    pub fn validate_closed(&self) -> io::Result<()> {
        let lower = self.local_checkpoints.checked_add(4 * SCOPED_LAYERS);
        let upper = self
            .local_checkpoints
            .checked_mul(2)
            .and_then(|v| v.checked_add(4 * SCOPED_LAYERS));
        let probes = self
            .local_checkpoints
            .checked_mul(2)
            .and_then(|v| v.checked_add(3 * SCOPED_LAYERS));
        if self.ordinary_layers != 72
            || u64::from(self.scoped_layers) != SCOPED_LAYERS
            || self.full_discoveries != 2 * SCOPED_LAYERS
            || self.local_checkpoints < SCOPED_LAYERS
            || self.before_calls != self.after_calls
            || lower.is_none_or(|n| self.before_calls < n)
            || upper.is_none_or(|n| self.before_calls > n)
            || probes != Some(self.generation_probes)
        {
            return Err(io::Error::other(
                "scoped warm complete actual call/counter census",
            ));
        }
        Ok(())
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
    pub prompt_positions: u32,
    pub generated_token_count: u32,
    pub generated_tokens_sha256: [u8; 32],
    pub capture_positions: [u32; 4],
    pub first_scoped_position: u32,
    pub layers_per_forward: u32,
    pub counts: Counts,
    pub scoped_warm_currentness: bool,
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
        b: &full::Bootstrap,
        transcript: [u8; 32],
        generated_tokens: &[u32],
        worker: [u8; 32],
        counts: Counts,
    ) -> io::Result<Self> {
        b.validate(
            b.sequence.device_ids,
            b.sequence.timeout_ms,
            b.sequence.scope.child_identity,
        )?;
        counts.validate_closed()?;
        if b.sequence.profile != long::Profile::Full2303
            || transcript == [0; 32]
            || worker == [0; 32]
            || generated_tokens.len() != long::OUTPUT_TOKENS
            || generated_tokens.iter().any(|n| *n >= 151_936)
        {
            return Err(io::Error::other(
                "full2303 scoped warm requires closed own-output identity",
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
            completed_forwards: long::FORWARDS,
            prompt_positions: 2048,
            generated_token_count: long::OUTPUT_TOKENS as u32,
            generated_tokens_sha256: {
                let mut hash = Sha256::new();
                for token in generated_tokens {
                    hash.update(token.to_le_bytes());
                }
                hash.finalize().into()
            },
            capture_positions: [0, 2047, 2048, 2302],
            first_scoped_position: 2,
            layers_per_forward: 36,
            counts,
            scoped_warm_currentness: true,
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
            full_long_workload: true,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        })
    }
    pub fn validate(
        &self,
        b: &full::Bootstrap,
        transcript: [u8; 32],
        generated_tokens: &[u32],
        worker: [u8; 32],
    ) -> io::Result<()> {
        if self != &Self::new(b, transcript, generated_tokens, worker, self.counts.clone())? {
            return Err(io::Error::other(
                "scoped warm exact policy and Close identity",
            ));
        }
        Ok(())
    }
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut raw = serde_json::to_vec(self).map_err(io::Error::other)?;
        raw.push(b'\n');
        if raw.len() > MAX_BYTES {
            return Err(io::Error::other("scoped warm record bound"));
        }
        Ok(raw)
    }
    pub fn write_to(&self, w: &mut (impl io::Write + ?Sized)) -> io::Result<()> {
        w.write_all(&self.encode()?)?;
        w.flush()
    }
    pub fn decode(
        raw: &[u8],
        b: &full::Bootstrap,
        transcript: [u8; 32],
        generated_tokens: &[u32],
        worker: [u8; 32],
    ) -> io::Result<Self> {
        if raw.is_empty() || raw.len() > MAX_BYTES || raw.last() != Some(&b'\n') {
            return Err(io::Error::other("scoped warm single bounded policy record"));
        }
        let value: Self = serde_json::from_slice(raw).map_err(io::Error::other)?;
        value.validate(b, transcript, generated_tokens, worker)?;
        if value.encode()? != raw {
            return Err(io::Error::other("scoped warm exact record framing"));
        }
        Ok(value)
    }
}

#[cfg(test)]
#[path = "finite_guarded_mlp_full2303_scoped_v1_tests.rs"]
mod tests;
