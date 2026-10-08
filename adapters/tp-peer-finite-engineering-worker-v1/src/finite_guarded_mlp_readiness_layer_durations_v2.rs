//! Feature-only closed layer stages nested in the original forward phase intervals.
use crate::finite_guarded_mlp_readiness_bank_scoped_census_tail_v4 as policy;
use crate::finite_guarded_mlp_readiness_currentness_durations_v1 as old;
use crate::finite_guarded_mlp_readiness_forward_durations_v1 as forward;
use crate::finite_guarded_mlp_readiness_wire_v1 as ready;
use serde::{Deserialize, Serialize};
use std::io;

pub const SCHEMA: &str = "FerricReadiness40ForwardLayerDurationsV2";
pub const MAX_BYTES: usize = forward::MAX_BYTES;
pub const STDERR_MAX_BYTES: usize = forward::STDERR_MAX_BYTES;
pub const PHASE_COUNT: usize = forward::PHASE_COUNT;
pub const PHASE_ORDER: [&str; PHASE_COUNT] = forward::PHASE_ORDER;
pub const LAYER_STAGE_ORDER: [&str; 6] = [
    "enter_pre_census",
    "prefix",
    "mlp_retired_seal",
    "hidden_post_census",
    "full_exit",
    "commit_prepare",
];
pub const PAIRED_MLP_STAGE_ORDER: [&str; 7] = [
    "preflight",
    "consume",
    "reserve",
    "publish",
    "poll",
    "retire",
    "terminal",
];
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
        .ok_or_else(|| io::Error::other("layer stage nanosecond overflow"))
}
fn sum(values: &[u64]) -> io::Result<u64> {
    values.iter().try_fold(0, |total, value| add(total, *value))
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerMetrics {
    pub layers: u32,
    pub phase_ns: [u64; 6],
    pub layer_body_ns: u64,
    pub paired_mlp_phase_ns: [u64; 7],
    pub paired_mlp_body_ns: u64,
}
impl LayerMetrics {
    pub fn validate(&self) -> io::Result<()> {
        require((1..=36).contains(&self.layers), "layer stage count")?;
        require(
            sum(&self.phase_ns)? == self.layer_body_ns && self.layer_body_ns <= WHOLE_NS,
            "closed layer exact sum and bound",
        )?;
        require(
            sum(&self.paired_mlp_phase_ns)? == self.paired_mlp_body_ns
                && self.paired_mlp_body_ns <= self.phase_ns[2],
            "paired MLP exact sum and closed MLP containment",
        )
    }
    pub fn checked_add(self, value: Self) -> io::Result<Self> {
        if self.layers == 0 {
            require(self == Self::default(), "empty layer stage accumulator")?;
        } else {
            self.validate()?;
        }
        value.validate()?;
        let mut output = Self {
            layers: self
                .layers
                .checked_add(value.layers)
                .ok_or_else(|| io::Error::other("layer stage count overflow"))?,
            layer_body_ns: add(self.layer_body_ns, value.layer_body_ns)?,
            paired_mlp_body_ns: add(self.paired_mlp_body_ns, value.paired_mlp_body_ns)?,
            ..Self::default()
        };
        for (out, (a, b)) in output
            .phase_ns
            .iter_mut()
            .zip(self.phase_ns.into_iter().zip(value.phase_ns))
        {
            *out = add(a, b)?;
        }
        for (out, (a, b)) in output.paired_mlp_phase_ns.iter_mut().zip(
            self.paired_mlp_phase_ns
                .into_iter()
                .zip(value.paired_mlp_phase_ns),
        ) {
            *out = add(a, b)?;
        }
        output.validate()?;
        Ok(output)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForwardRow {
    pub position: u32,
    pub phase_ns: [u64; PHASE_COUNT],
    pub forward_body_ns: u64,
    pub layer_metrics: Option<LayerMetrics>,
}
impl ForwardRow {
    pub fn original(&self) -> forward::ForwardRow {
        forward::ForwardRow {
            position: self.position,
            phase_ns: self.phase_ns,
            forward_body_ns: self.forward_body_ns,
        }
    }
    pub fn validate(&self) -> io::Result<()> {
        self.original().validate()?;
        match (self.position < 2, self.layer_metrics) {
            (true, None) => Ok(()),
            (false, Some(metrics)) => {
                metrics.validate()?;
                require(
                    metrics.layers == 36 && metrics.layer_body_ns <= self.phase_ns[4],
                    "thirty-six closed layers inside worker layers phase",
                )
            }
            _ => Err(io::Error::other(
                "cold layers unmeasured and warm stages required",
            )),
        }
    }
    pub fn validate_against(&self, callback: &old::ForwardRow) -> io::Result<()> {
        self.validate()?;
        self.original().validate_against(callback)?;
        if let (Some(metrics), Some(measured)) = (self.layer_metrics, callback.measured) {
            require(
                measured.layers.elapsed_subtotal()? <= metrics.layer_body_ns,
                "layer callbacks nested in closed layer body",
            )?;
        }
        Ok(())
    }
}
pub fn validate_rows(rows: &[ForwardRow], callbacks: &old::Record) -> io::Result<()> {
    let originals: Vec<_> = rows.iter().map(ForwardRow::original).collect();
    forward::validate_rows(&originals, callbacks)?;
    for (row, callback) in rows.iter().zip(&callbacks.forwards) {
        row.validate_against(callback)?;
    }
    Ok(())
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
    pub layer_stage_order: [String; 6],
    pub paired_mlp_stage_order: [String; 7],
    pub forwards: Vec<ForwardRow>,
    pub host_elapsed_nanoseconds: bool,
    pub disjoint_phases: bool,
    pub currentness_durations_nested: bool,
    pub layer_durations_nested: bool,
    pub paired_mlp_durations_nested: bool,
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
        validate_rows(&forwards, callbacks)?;
        let original = forward::Record::new(
            policy,
            callbacks,
            forwards.iter().map(ForwardRow::original).collect(),
        )?;
        Ok(Self {
            schema: SCHEMA.into(),
            instrumented: original.instrumented,
            policy_sha256: original.policy_sha256,
            currentness_record_sha256: original.currentness_record_sha256,
            session: original.session,
            worker_sha256: original.worker_sha256,
            transcript_sha256: original.transcript_sha256,
            phase_order: original.phase_order,
            layer_stage_order: std::array::from_fn(|i| LAYER_STAGE_ORDER[i].into()),
            paired_mlp_stage_order: std::array::from_fn(|i| PAIRED_MLP_STAGE_ORDER[i].into()),
            forwards,
            host_elapsed_nanoseconds: true,
            disjoint_phases: true,
            currentness_durations_nested: true,
            layer_durations_nested: true,
            paired_mlp_durations_nested: true,
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
            "layer stage original identity, order and authority",
        )
    }
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut raw = serde_json::to_vec(self).map_err(io::Error::other)?;
        raw.push(b'\n');
        require(
            raw.len() <= MAX_BYTES,
            "layer stage third-record byte bound",
        )?;
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
        "layer stage whole stderr bound",
    )?;
    let split = raw
        .iter()
        .enumerate()
        .filter(|(_, b)| **b == b'\n')
        .nth(1)
        .map(|(i, _)| i + 1)
        .ok_or_else(|| io::Error::other("layer stage two original newlines"))?;
    let (prefix, third) = raw.split_at(split);
    let (policy, callbacks) = old::decode_stderr(prefix, bootstrap, transcript, worker)?;
    require(
        !third.is_empty()
            && third.len() <= MAX_BYTES
            && third.last() == Some(&b'\n')
            && !third[..third.len() - 1].contains(&b'\n'),
        "layer stage one third record",
    )?;
    let record: Record = serde_json::from_slice(third).map_err(io::Error::other)?;
    record.validate(&policy, &callbacks)?;
    require(
        record.encode()? == third,
        "layer stage canonical third record",
    )?;
    Ok((policy, callbacks, record))
}

#[cfg(test)]
#[path = "finite_guarded_mlp_readiness_layer_durations_v2_tests.rs"]
pub(crate) mod tests;
