//! Closed, inclusive host-counter observations. Never GPU timing or authority.
use crate::finite_guarded_mlp_decode_wire_v1::{Bootstrap, Chain, Completion};
use crate::prefix_decode_host_observation_v1::difference;
pub use crate::prefix_decode_host_observation_v1::{
    COUNTER_NAMES, Interval, SHARED_NAMES, Snapshot,
};
use serde::{Deserialize, Serialize};
use std::io;

pub const SCHEMA: &str = "FerricGuardedMlpHostObservationV1";
pub const SHARED_SCHEMA: &str = "FerricGuardedMlpSharedFullHostObservationV1";
pub const PAIRED_READ_SCHEMA: &str = "FerricGuardedMlpSharedFullPairedReadHostObservationV1";
pub const MAX_BYTES: usize = 2 << 20;
pub const FORWARD_POINTS: usize = 146;
pub const SNAPSHOTS: usize = 2 + 4 * FORWARD_POINTS + 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Policy {
    DefaultFull,
    SharedFull,
    SharedFullPairedRead,
}

impl Policy {
    pub(crate) const fn shared(self) -> bool {
        matches!(self, Self::SharedFull | Self::SharedFullPairedRead)
    }

    pub(crate) const fn paired_read(self) -> bool {
        matches!(self, Self::SharedFullPairedRead)
    }

    pub(crate) const fn schema(self) -> &'static str {
        match self {
            Self::DefaultFull => SCHEMA,
            Self::SharedFull => SHARED_SCHEMA,
            Self::SharedFullPairedRead => PAIRED_READ_SCHEMA,
        }
    }

    pub(crate) const fn worker_flag(self) -> &'static str {
        match self {
            Self::DefaultFull => "--engineering-native-guarded-mlp-host-observation-v1",
            Self::SharedFull => "--engineering-native-guarded-mlp-host-shared-currentness-v1",
            Self::SharedFullPairedRead => "--engineering-native-guarded-mlp-host-paired-read-v1",
        }
    }
}

pub fn phase(index: usize) -> io::Result<String> {
    match index {
        0 => return Ok("fresh_enabled".into()),
        1 => return Ok("setup_sealed".into()),
        n if n == SNAPSHOTS - 1 => return Ok("before_close".into()),
        n if n >= SNAPSHOTS => return Err(io::Error::other("guarded host phase bound")),
        _ => {}
    }
    let forward = (index - 2) / FORWARD_POINTS;
    let at = (index - 2) % FORWARD_POINTS;
    Ok(match at {
        0 => format!("forward_{forward}/begin"),
        145 => format!("forward_{forward}/done"),
        _ => {
            let layer = (at - 1) / 4;
            let step = ["begin", "prefix", "paired", "hidden"][(at - 1) % 4];
            format!("forward_{forward}/layer_{layer:02}/{step}")
        }
    })
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
    pub forward_host_ns: [u64; 4],
    pub close_host_ns: u64,
    pub completions: Vec<Completion>,
    pub native_closed: bool,
    pub inclusive_nested_host_scopes: bool,
    pub paired_generic_dispatch_timers_complete: bool,
    pub tensor_stage_capture: bool,
    pub gpu_time: bool,
    pub gpu_overlap: bool,
    pub numerical_acceptance: bool,
    pub full_model_acceptance: bool,
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
    /// Data equality with the actual owned child's existing wire observations.
    pub fn validate_expected(
        &self,
        bootstrap: &Bootstrap,
        worker: [u8; 32],
        child: u32,
        completions: &[Completion],
    ) -> io::Result<()> {
        self.validate_expected_policy(bootstrap, worker, child, completions, Policy::DefaultFull)
    }

    pub fn validate_shared_expected(
        &self,
        bootstrap: &Bootstrap,
        worker: [u8; 32],
        child: u32,
        completions: &[Completion],
    ) -> io::Result<()> {
        self.validate_expected_policy(bootstrap, worker, child, completions, Policy::SharedFull)
    }

    pub fn validate_paired_read_expected(
        &self,
        bootstrap: &Bootstrap,
        worker: [u8; 32],
        child: u32,
        completions: &[Completion],
    ) -> io::Result<()> {
        self.validate_expected_policy(bootstrap, worker, child, completions, Policy::SharedFullPairedRead)
    }

    fn validate_expected_policy(
        &self,
        bootstrap: &Bootstrap,
        worker: [u8; 32],
        child: u32,
        completions: &[Completion],
        policy: Policy,
    ) -> io::Result<()> {
        self.validate_policy(policy)?;
        require(
            self.bootstrap == *bootstrap
                && self.worker_sha256 == worker
                && self.child_pid == child
                && self.completions == completions,
            "guarded host actual parent/worker/completion join",
        )
    }
    pub fn decode(raw: &[u8]) -> io::Result<Self> {
        Self::decode_policy(raw, Policy::DefaultFull)
    }

    pub fn decode_shared(raw: &[u8]) -> io::Result<Self> {
        Self::decode_policy(raw, Policy::SharedFull)
    }

    pub fn decode_paired_read(raw: &[u8]) -> io::Result<Self> {
        Self::decode_policy(raw, Policy::SharedFullPairedRead)
    }

    fn decode_policy(raw: &[u8], policy: Policy) -> io::Result<Self> {
        require(
            !raw.is_empty() && raw.len() <= MAX_BYTES,
            "guarded host report bound",
        )?;
        let value: Self = serde_json::from_slice(raw).map_err(io::Error::other)?;
        value.validate_policy(policy)?;
        Ok(value)
    }
    pub fn validate(&self) -> io::Result<()> {
        self.validate_policy(Policy::DefaultFull)
    }

    pub(crate) fn validate_policy(&self, policy: Policy) -> io::Result<()> {
        require(
            !policy.paired_read()
                || (self.bootstrap.schema == crate::finite_guarded_mlp_decode_wire_v1::SCHEMA
                    && self.bootstrap.decode.mode == crate::finite_guarded_mlp_decode_wire_v1::InputMode::Autoregressive),
            "paired hidden reads require the separate fresh AR4 host route",
        )?;
        require(
            self.schema == policy.schema()
                && self.worker_sha256 != [0; 32]
                && self.child_pid != 0
                && self.bootstrap.decode.scope.child_identity == self.child_pid
                && self.profile_sha256 == self.bootstrap.sha256()?
                && self.snapshots.len() == SNAPSHOTS
                && self.intervals.len() + 1 == SNAPSHOTS
                && self.completions.len() == 4
                && self.native_closed
                && self.inclusive_nested_host_scopes
                && !self.paired_generic_dispatch_timers_complete
                && !self.tensor_stage_capture
                && !self.gpu_time
                && !self.gpu_overlap
                && !self.numerical_acceptance
                && !self.full_model_acceptance
                && !self.performance_claim
                && !self.production_authority,
            "guarded host identity, coverage, Close or claims",
        )?;
        self.bootstrap.validate(
            self.bootstrap.decode.device_ids,
            self.bootstrap.decode.timeout_ms,
            self.child_pid,
            self.bootstrap.decode.mode,
        )?;
        let first = &self.snapshots[0];
        require(
            first.shared == [0; 4] && first.ranks.iter().all(|r| r.counters == [0; 19]),
            "guarded host nonfresh baseline",
        )?;
        for (index, current) in self.snapshots.iter().enumerate() {
            current.validate_shared_policy(self.bootstrap.decode.device_ids, policy.shared())?;
            require(
                current.phase == phase(index)?
                    && current.group_incarnation == first.group_incarnation,
                "guarded host phase or group drift",
            )?;
            for rank in 0..2 {
                require(
                    current.ranks[rank].queue_epoch == first.ranks[rank].queue_epoch,
                    "guarded host queue epoch drift",
                )?;
            }
            if index != 0 {
                let previous = &self.snapshots[index - 1];
                let delta = &self.intervals[index - 1];
                require(
                    delta.shared == difference(&current.shared, &previous.shared)?,
                    "guarded host shared delta",
                )?;
                for rank in 0..2 {
                    require(
                        delta.ranks[rank]
                            == difference(
                                &current.ranks[rank].counters,
                                &previous.ranks[rank].counters,
                            )?,
                        "guarded host rank delta",
                    )?;
                }
            }
        }
        let mut chain = Chain::new(self.bootstrap.decode.registration, self.profile_sha256);
        let mut previous = None;
        for (position, done) in self.completions.iter().enumerate() {
            require(
                done.generation == position as u64 + 1
                    && done.position == position as u32
                    && done.input_token == self.bootstrap.input(position as u32, previous)?
                    && done.output_token < 151936
                    && done.control.bytes as usize
                        == crate::finite_guarded_mlp_decode_wire_v1::CONTROL_BYTES
                    && done.observation.bytes as usize
                        == crate::finite_forward_wire_v1::OBSERVATION_BYTES
                    && done.capture.total == done.observation
                    && done.capture.layer_hidden.len() == 36
                    && done.capture.layer_hidden.iter().all(|p| p.bytes == 8192)
                    && done.capture.final_normalized.bytes == 8192
                    && done.capture.logits.bytes == 303872
                    && done.chain == chain.advance(done),
                "guarded host genuine completion chain",
            )?;
            previous = Some(done.output_token);
            let start = 2 + position * FORWARD_POINTS;
            let elapsed = self.intervals[start..start + FORWARD_POINTS - 1]
                .iter()
                .try_fold(0u64, |sum, v| {
                    sum.checked_add(v.host_elapsed_ns)
                        .ok_or_else(|| io::Error::other("guarded host duration overflow"))
                })?;
            require(
                self.forward_host_ns[position] <= elapsed,
                "guarded host forward duration bound",
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "guarded_mlp_host_observation_v1_tests.rs"]
pub(crate) mod tests;
