//! Explicit ordered AR4 shared-full report; legacy report decoders remain closed.
use crate::finite_projection_residual_mlp_ordered_wire_v1::{Bootstrap, Completion, InputMode};
pub use crate::prefix_decode_host_observation_v1::{
    COUNTER_NAMES, Interval, MAX_BYTES, PHASES, Rank, SHARED_NAMES, Snapshot, difference,
};
use serde::{Deserialize, Serialize};
use std::io;

pub const SCHEMA: &str = "FerricProjectionResidualMlpOrderedHostObservationV1";
pub const SHARED_SCHEMA: &str = "FerricProjectionResidualMlpOrderedHostObservationV1";
pub const SHARED_ENVELOPE: &str = "FerricProjectionResidualMlpOrderedHostEnvelopeV1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Policy {
    OrderedSharedFull,
}
impl Policy {
    pub const fn shared(self) -> bool {
        true
    }
    pub const fn schema(self) -> &'static str {
        SCHEMA
    }
    pub const fn worker_flag(self) -> &'static str {
        "--engineering-native-projection-residual-mlp-ordered-v1"
    }
}

/// Shared policy configuration runs before observer enable. Its duration is
/// inclusive host wall time outside all zero-based snapshots, never GPU time.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedReport {
    pub schema: String,
    pub policy: Policy,
    pub configuration_host_ns: u64,
    pub observation: Report,
}
impl SharedReport {
    pub fn decode(raw: &[u8]) -> io::Result<Self> {
        require(
            !raw.is_empty() && raw.len() <= MAX_BYTES,
            "shared host byte bound",
        )?;
        let value: Self = serde_json::from_slice(raw).map_err(io::Error::other)?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> io::Result<()> {
        require(
            self.schema == SHARED_ENVELOPE && self.policy == Policy::OrderedSharedFull,
            "shared host schema/policy",
        )?;
        self.observation
            .validate_for_policy(Policy::OrderedSharedFull)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema: String,
    pub bootstrap: Bootstrap,
    pub worker_sha256: [u8; 32],
    pub child_pid: u32,
    pub profile_sha256: [u8; 32],
    pub snapshots: Vec<Snapshot>,
    pub intervals: Vec<Interval>,
    /// Each interval brackets Owner.run only; it still includes host and GPU waits.
    pub forward_host_ns: [u64; 4],
    /// Host wall time for consuming Close, not a device teardown measurement.
    pub close_host_ns: u64,
    /// Encoding, hashing and response pipe writes after Owner.run; not GPU time.
    pub serialization_host_ns: [u64; 4],
    pub completions: Vec<Completion>,
    pub transcript_sha256: [u8; 32],
    pub native_closed: bool,
    pub inclusive_nested_host_scopes: bool,
    pub gpu_time: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
}
fn require(ok: bool, why: &'static str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(why))
    }
}
impl Report {
    pub fn decode(raw: &[u8]) -> io::Result<Self> {
        require(
            !raw.is_empty() && raw.len() <= MAX_BYTES,
            "host sidecar byte bound",
        )?;
        let value: Self = serde_json::from_slice(raw).map_err(io::Error::other)?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> io::Result<()> {
        self.validate_for_policy(Policy::OrderedSharedFull)
    }
    pub(crate) fn validate_for_policy(&self, policy: Policy) -> io::Result<()> {
        require(
            self.schema == policy.schema()
                && self.child_pid != 0
                && self.worker_sha256 != [0; 32]
                && self.profile_sha256 == self.bootstrap.sha256()?
                && self.bootstrap.decode.scope.child_identity == self.child_pid
                && self.snapshots.len() == 7
                && self.intervals.len() == 6
                && self.completions.len() == 4
                && self.native_closed
                && self.inclusive_nested_host_scopes
                && !self.gpu_time
                && !self.numerical_acceptance
                && !self.performance_claim
                && !self.production_authority,
            "host sidecar identity/completion/claims",
        )?;
        self.bootstrap.validate(
            self.bootstrap.decode.device_ids,
            self.bootstrap.decode.timeout_ms,
            self.child_pid,
            InputMode::Autoregressive,
        )?;
        let first = &self.snapshots[0];
        require(
            first.shared == [0; 4] && first.ranks.iter().all(|r| r.counters == [0; 19]),
            "host observer baseline is not fresh",
        )?;
        for (i, current) in self.snapshots.iter().enumerate() {
            current.validate_shared_policy(self.bootstrap.decode.device_ids, policy.shared())?;
            require(
                current.phase == PHASES[i] && current.group_incarnation == first.group_incarnation,
                "host snapshot order/group",
            )?;
            for rank in 0..2 {
                require(
                    current.ranks[rank].queue_epoch == first.ranks[rank].queue_epoch,
                    "host queue epoch changed",
                )?;
            }
            if i > 0 {
                let old = &self.snapshots[i - 1];
                let delta = &self.intervals[i - 1];
                require(
                    delta.shared == difference(&current.shared, &old.shared)?,
                    "host shared delta differs",
                )?;
                for rank in 0..2 {
                    require(
                        delta.ranks[rank]
                            == difference(
                                &current.ranks[rank].counters,
                                &old.ranks[rank].counters,
                            )?,
                        "host rank delta differs",
                    )?;
                }
                if (2..=5).contains(&i) {
                    require(
                        self.forward_host_ns[i - 2] <= delta.host_elapsed_ns,
                        "host forward interval exceeds snapshot interval",
                    )?;
                }
            }
        }
        let mut chain = crate::finite_prefix_decode_wire_v1::Chain::new(
            self.bootstrap.decode.registration,
            self.profile_sha256,
        );
        let mut previous = None;
        for (i, c) in self.completions.iter().enumerate() {
            require(
                self.serialization_host_ns[i] <= self.intervals[i + 2].host_elapsed_ns,
                "host serialization exceeds following snapshot interval",
            )?;
            require(
                c.position == i as u32
                    && c.generation == i as u64 + 1
                    && c.input_token == self.bootstrap.input(i as u32, previous)?
                    && c.output_token < 151936
                    && c.control.bytes as usize
                        == crate::finite_projection_residual_mlp_ordered_wire_v1::CONTROL_BYTES
                    && c.observation.bytes == 606976
                    && c.control.sha256 != [0; 32]
                    && c.observation.sha256 != [0; 32]
                    && c.capture.total == c.observation
                    && chain.advance(c) == c.chain,
                "host completion trajectory/bindings",
            )?;
            previous = Some(c.output_token);
        }
        require(
            chain.digest() == self.transcript_sha256,
            "host final transcript",
        )
    }
}

#[cfg(test)]
#[path = "projection_residual_mlp_ordered_observation_v1_tests.rs"]
pub(crate) mod tests;
