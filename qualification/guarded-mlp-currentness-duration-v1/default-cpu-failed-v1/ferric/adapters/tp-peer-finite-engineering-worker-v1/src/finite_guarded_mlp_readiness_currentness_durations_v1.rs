//! Feature-only diagnostic data. It never changes or replaces the V4 policy.
use crate::finite_guarded_mlp_readiness_bank_scoped_census_tail_v4 as policy;
use crate::finite_guarded_mlp_readiness_wire_v1 as ready;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io;

pub const SCHEMA: &str = "FerricReadiness40TailCurrentnessDurationsV1";
pub const MAX_BYTES: usize = 65_536;
pub const STDERR_MAX_BYTES: usize = policy::MAX_BYTES + MAX_BYTES;
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
        .ok_or_else(|| io::Error::other("diagnostic counter or nanosecond overflow"))
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallDuration {
    pub calls: u64,
    pub elapsed_ns: u64,
}
impl CallDuration {
    fn add(self, other: Self) -> io::Result<Self> {
        Ok(Self {
            calls: add(self.calls, other.calls)?,
            elapsed_ns: add(self.elapsed_ns, other.elapsed_ns)?,
        })
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Durations {
    pub before: CallDuration,
    pub discover: CallDuration,
    pub after: CallDuration,
    pub root_generation: CallDuration,
}
impl Durations {
    pub fn elapsed_subtotal(&self) -> io::Result<u64> {
        add(
            add(self.before.elapsed_ns, self.discover.elapsed_ns)?,
            add(self.after.elapsed_ns, self.root_generation.elapsed_ns)?,
        )
    }
    pub fn checked_add(self, other: Self) -> io::Result<Self> {
        Ok(Self {
            before: self.before.add(other.before)?,
            discover: self.discover.add(other.discover)?,
            after: self.after.add(other.after)?,
            root_generation: self.root_generation.add(other.root_generation)?,
        })
    }
    pub fn validate_calls(
        &self,
        full: u64,
        before: u64,
        after: u64,
        probes: u64,
    ) -> io::Result<()> {
        require(
            self.discover.calls == full
                && self.before.calls == before
                && self.after.calls == after
                && self.root_generation.calls == probes,
            "diagnostic calls must join original currentness counts",
        )?;
        require(
            [self.before, self.discover, self.after, self.root_generation]
                .iter()
                .all(|p| p.calls > 0 && p.elapsed_ns <= WHOLE_NS),
            "diagnostic callback duration bound",
        )
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeasuredForward {
    pub bank: Durations,
    pub layers: Durations,
    pub tail: Durations,
    pub bank_guarded_body_ns: u64,
}
impl MeasuredForward {
    pub fn validate(&self) -> io::Result<()> {
        self.bank.validate_calls(2, 726, 726, 725)?;
        for (value, discoveries) in [(self.bank, 2), (self.layers, 72), (self.tail, 2)] {
            value.validate_calls(
                discoveries,
                value.before.calls,
                value.after.calls,
                value.root_generation.calls,
            )?;
            require(
                value.before.calls == value.after.calls,
                "diagnostic balanced participant calls",
            )?;
        }
        require(
            self.bank_guarded_body_ns <= WHOLE_NS
                && self.bank.elapsed_subtotal()? <= self.bank_guarded_body_ns,
            "diagnostic guarded bank duration bound and callback containment",
        )
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForwardRow {
    pub position: u32,
    pub measured: Option<MeasuredForward>,
}
pub fn validate_rows(rows: &[ForwardRow], counts: &policy::Counts) -> io::Result<()> {
    counts.validate_closed()?;
    require(rows.len() == 40, "diagnostic forty original forwards")?;
    let mut banks = Durations::default();
    let mut layers = Durations::default();
    let mut tails = Durations::default();
    for (position, row) in rows.iter().enumerate() {
        require(
            row.position as usize == position,
            "diagnostic original forward order",
        )?;
        match (position < 2, row.measured) {
            (true, None) => (),
            (false, Some(value)) => {
                value.validate()?;
                banks = banks.checked_add(value.bank)?;
                layers = layers.checked_add(value.layers)?;
                tails = tails.checked_add(value.tail)?;
            }
            _ => {
                return Err(io::Error::other(
                    "diagnostic first-use unmeasured and warm-only census",
                ));
            }
        }
    }
    banks.validate_calls(
        counts.banks.full_discoveries,
        counts.banks.before_calls,
        counts.banks.after_calls,
        counts.banks.generation_probes,
    )?;
    layers.validate_calls(
        counts.layers.full_discoveries,
        counts.layers.before_calls,
        counts.layers.after_calls,
        counts.layers.generation_probes,
    )?;
    tails.validate_calls(
        counts.tails.full_discoveries,
        counts.tails.before_calls,
        counts.tails.after_calls,
        counts.tails.generation_probes,
    )
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub schema: String,
    pub instrumented: bool,
    pub policy_sha256: [u8; 32],
    pub session: [u8; 32],
    pub worker_sha256: [u8; 32],
    pub transcript_sha256: [u8; 32],
    pub forwards: Vec<ForwardRow>,
    pub host_elapsed_nanoseconds: bool,
    pub bank_guarded_body_includes_callbacks: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub execution_authority: bool,
}
impl Record {
    pub fn new(policy: &policy::PolicyRecord, forwards: Vec<ForwardRow>) -> io::Result<Self> {
        validate_rows(&forwards, &policy.counts)?;
        Ok(Self {
            schema: SCHEMA.into(),
            instrumented: true,
            policy_sha256: Sha256::digest(policy.encode()?).into(),
            session: policy.session,
            worker_sha256: policy.worker_sha256,
            transcript_sha256: policy.transcript_sha256,
            forwards,
            host_elapsed_nanoseconds: true,
            bank_guarded_body_includes_callbacks: true,
            numerical_acceptance: false,
            performance_claim: false,
            execution_authority: false,
        })
    }
    pub fn validate(&self, policy: &policy::PolicyRecord) -> io::Result<()> {
        require(
            self == &Self::new(policy, self.forwards.clone())?,
            "diagnostic exact policy and Close identity",
        )
    }
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut raw = serde_json::to_vec(self).map_err(io::Error::other)?;
        raw.push(b'\n');
        require(raw.len() <= MAX_BYTES, "diagnostic record byte bound")?;
        Ok(raw)
    }
}
pub fn decode_stderr(
    raw: &[u8],
    bootstrap: &ready::Bootstrap,
    transcript: [u8; 32],
    worker: [u8; 32],
) -> io::Result<(policy::PolicyRecord, Record)> {
    require(
        !raw.is_empty() && raw.len() <= STDERR_MAX_BYTES,
        "diagnostic entire original stderr bound",
    )?;
    let split = raw
        .iter()
        .position(|b| *b == b'\n')
        .ok_or_else(|| io::Error::other("diagnostic first canonical policy newline"))?
        + 1;
    let (first, second) = raw.split_at(split);
    let policy = policy::PolicyRecord::decode(first, bootstrap, transcript, worker)?;
    require(
        !second.is_empty()
            && second.len() <= MAX_BYTES
            && second.last() == Some(&b'\n')
            && !second[..second.len() - 1].contains(&b'\n'),
        "diagnostic exactly one second record",
    )?;
    let value: Record = serde_json::from_slice(second).map_err(io::Error::other)?;
    value.validate(&policy)?;
    require(
        value.encode()? == second,
        "diagnostic canonical second record",
    )?;
    Ok((policy, value))
}

#[cfg(test)]
#[path = "finite_guarded_mlp_readiness_currentness_durations_v1_tests.rs"]
pub(crate) mod tests;
