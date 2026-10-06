//! Three closed host-policy experiments. V1 remains a separate frozen schema.
use crate::finite_prefix_decode_wire_v1::{Bootstrap, Completion};
pub use crate::prefix_decode_host_observation_v1::{
    COUNTER_NAMES, Interval, MAX_BYTES, PHASES, Rank, SHARED_NAMES, difference,
};
use serde::{Deserialize, Serialize};
use std::io;

pub const SCHEMA: &str = "FerricPrefixDecodeHostObservationV2";
/// Exactly one optimization can be selected; combined policies need a new study.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Policy {
    Baseline,
    ImmutableAdmissionCache,
    SharedFullCurrentness,
}
impl Policy {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::ImmutableAdmissionCache => "immutable-admission-cache",
            Self::SharedFullCurrentness => "shared-full-currentness",
        }
    }
    pub fn parse(value: &str) -> io::Result<Self> {
        match value {
            "baseline" => Ok(Self::Baseline),
            "immutable-admission-cache" => Ok(Self::ImmutableAdmissionCache),
            "shared-full-currentness" => Ok(Self::SharedFullCurrentness),
            _ => Err(io::Error::other("closed host policy required")),
        }
    }
    /// Operational checks, legacy profiling and raw queue timestamps stay off.
    pub const fn options(self) -> (bool, bool, bool) {
        (
            matches!(self, Self::ImmutableAdmissionCache),
            false,
            matches!(self, Self::SharedFullCurrentness),
        )
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub phase: String,
    pub group_incarnation: u64,
    pub shared_full_currentness: bool,
    pub ranks: [Rank; 2],
    pub shared: [u64; 4],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema: String,
    pub policy: Policy,
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
impl Snapshot {
    pub fn validate(&self, devices: [u64; 2], policy: Policy) -> io::Result<()> {
        let (cache, _, shared) = policy.options();
        require(
            self.group_incarnation != 0 && self.shared_full_currentness == shared,
            "host observer group/shared policy",
        )?;
        for (rank, v) in self.ranks.iter().enumerate() {
            require(
                v.rank == rank as u32
                    && v.unique_id == devices[rank]
                    && v.unique_id != 0
                    && v.cache_kernel_admission == cache
                    && !v.raw_timestamp_queue
                    && v.counters[4] == 0
                    && v.counters[5] == 0,
                "host observer rank or optimized policy",
            )?;
        }
        require(
            devices[0] != devices[1],
            "host observer repeated participant",
        )
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
        require(
            self.schema == SCHEMA
                && self.child_pid != 0
                && self.worker_sha256 != [0; 32]
                && self.profile_sha256 == self.bootstrap.sha256()?
                && self.bootstrap.scope.child_identity == self.child_pid
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
            self.bootstrap.device_ids,
            self.bootstrap.timeout_ms,
            self.child_pid,
            self.bootstrap.mode,
        )?;
        let first = &self.snapshots[0];
        require(
            first.shared == [0; 4] && first.ranks.iter().all(|r| r.counters == [0; 19]),
            "host observer baseline is not fresh",
        )?;
        for (i, current) in self.snapshots.iter().enumerate() {
            current.validate(self.bootstrap.device_ids, self.policy)?;
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
            self.bootstrap.registration,
            self.profile_sha256,
        );
        let mut previous = None;
        for (i, c) in self.completions.iter().enumerate() {
            require(
                c.position == i as u32
                    && c.generation == i as u64 + 1
                    && c.input_token == self.bootstrap.input(i as u32, previous)?
                    && c.output_token < 151936
                    && c.control.bytes == 241960
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
#[path = "prefix_decode_host_observation_v2_tests.rs"]
pub(crate) mod tests;
