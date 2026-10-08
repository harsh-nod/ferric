//! Feature-only, host-only phase data authenticated to the unchanged two records.
use crate::finite_guarded_mlp_readiness_bank_scoped_census_tail_v4 as policy;
use crate::finite_guarded_mlp_readiness_currentness_durations_v1 as old;
use crate::finite_guarded_mlp_readiness_wire_v1 as ready;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io;

pub const SCHEMA: &str = "FerricReadiness40ForwardPhaseDurationsV1";
pub const PHASE_COUNT: usize = 9;
pub const PHASE_ORDER: [&str; PHASE_COUNT] = [
    "input",
    "metadata",
    "embedding",
    "bank",
    "layers",
    "tail",
    "frame",
    "fence",
    "commit",
];
pub const MAX_BYTES: usize = 32_768;
pub const STDERR_MAX_BYTES: usize = old::STDERR_MAX_BYTES;
const WHOLE_NS: u64 = 3_600_000_000_000;

fn require(ok: bool, why: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(why))
    }
}

fn add(a: u64, b: u64) -> io::Result<u64> {
    a.checked_add(b)
        .ok_or_else(|| io::Error::other("forward phase nanosecond overflow"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForwardRow {
    pub position: u32,
    pub phase_ns: [u64; PHASE_COUNT],
    pub forward_body_ns: u64,
}

impl ForwardRow {
    pub fn validate(&self) -> io::Result<()> {
        require(self.position < 40, "forward phase position bound")?;
        let sum = self
            .phase_ns
            .iter()
            .try_fold(0, |sum, value| add(sum, *value))?;
        require(
            sum == self.forward_body_ns && sum <= WHOLE_NS,
            "forward phase exact disjoint sum and duration bound",
        )
    }

    pub fn validate_against(&self, callback: &old::ForwardRow) -> io::Result<()> {
        self.validate()?;
        require(
            self.position == callback.position,
            "forward phase original callback position",
        )?;
        match (self.position < 2, callback.measured) {
            (true, None) => Ok(()),
            (false, Some(value)) => {
                value.validate()?;
                require(
                    value.bank_guarded_body_ns <= self.phase_ns[3]
                        && value.layers.elapsed_subtotal()? <= self.phase_ns[4]
                        && value.tail.elapsed_subtotal()? <= self.phase_ns[5],
                    "forward phase original callback containment",
                )
            }
            _ => Err(io::Error::other(
                "forward phase first-use unmeasured and warm callbacks required",
            )),
        }
    }
}

pub fn validate_rows(rows: &[ForwardRow], callbacks: &old::Record) -> io::Result<()> {
    require(
        rows.len() == 40 && callbacks.forwards.len() == 40,
        "forward phase forty original forwards",
    )?;
    let mut total = 0;
    for (position, (row, callback)) in rows.iter().zip(&callbacks.forwards).enumerate() {
        require(
            row.position as usize == position && callback.position as usize == position,
            "forward phase original forward order",
        )?;
        row.validate_against(callback)?;
        total = add(total, row.forward_body_ns)?;
    }
    require(total <= WHOLE_NS, "forward phase whole-case duration bound")
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub schema: String,
    pub instrumented: bool,
    pub policy_sha256: [u8; 32],
    pub currentness_record_sha256: [u8; 32],
    pub session: [u8; 32],
    pub worker_sha256: [u8; 32],
    pub transcript_sha256: [u8; 32],
    pub phase_order: [String; PHASE_COUNT],
    pub forwards: Vec<ForwardRow>,
    pub host_elapsed_nanoseconds: bool,
    pub disjoint_phases: bool,
    pub currentness_durations_nested: bool,
    pub gpu_timing: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub execution_authority: bool,
}

impl Record {
    pub fn new(
        policy: &policy::PolicyRecord,
        callbacks: &old::Record,
        forwards: Vec<ForwardRow>,
    ) -> io::Result<Self> {
        callbacks.validate(policy)?;
        validate_rows(&forwards, callbacks)?;
        Ok(Self {
            schema: SCHEMA.into(),
            instrumented: true,
            policy_sha256: Sha256::digest(policy.encode()?).into(),
            currentness_record_sha256: Sha256::digest(callbacks.encode()?).into(),
            session: policy.session,
            worker_sha256: policy.worker_sha256,
            transcript_sha256: policy.transcript_sha256,
            phase_order: std::array::from_fn(|index| PHASE_ORDER[index].into()),
            forwards,
            host_elapsed_nanoseconds: true,
            disjoint_phases: true,
            currentness_durations_nested: true,
            gpu_timing: false,
            numerical_acceptance: false,
            performance_claim: false,
            execution_authority: false,
        })
    }

    pub fn validate(
        &self,
        policy: &policy::PolicyRecord,
        callbacks: &old::Record,
    ) -> io::Result<()> {
        require(
            self == &Self::new(policy, callbacks, self.forwards.clone())?,
            "forward phase exact original policy, callback and Close identity",
        )
    }

    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut raw = serde_json::to_vec(self).map_err(io::Error::other)?;
        raw.push(b'\n');
        require(raw.len() <= MAX_BYTES, "forward phase record byte bound")?;
        Ok(raw)
    }
}

pub fn decode_stderr(
    raw: &[u8],
    bootstrap: &ready::Bootstrap,
    transcript: [u8; 32],
    worker: [u8; 32],
) -> io::Result<(policy::PolicyRecord, old::Record, Record)> {
    require(
        !raw.is_empty() && raw.len() <= STDERR_MAX_BYTES,
        "forward phase entire original stderr bound",
    )?;
    let split = raw
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == b'\n')
        .nth(1)
        .map(|(index, _)| index + 1)
        .ok_or_else(|| io::Error::other("forward phase two original record newlines"))?;
    let (original, third) = raw.split_at(split);
    let (policy, callbacks) = old::decode_stderr(original, bootstrap, transcript, worker)?;
    require(
        !third.is_empty()
            && third.len() <= MAX_BYTES
            && third.last() == Some(&b'\n')
            && !third[..third.len() - 1].contains(&b'\n'),
        "forward phase exactly one third record",
    )?;
    let value: Record = serde_json::from_slice(third).map_err(io::Error::other)?;
    value.validate(&policy, &callbacks)?;
    require(
        value.encode()? == third,
        "forward phase canonical third record",
    )?;
    Ok((policy, callbacks, value))
}

#[cfg(test)]
#[path = "finite_guarded_mlp_readiness_forward_durations_v1_tests.rs"]
pub(crate) mod tests;
