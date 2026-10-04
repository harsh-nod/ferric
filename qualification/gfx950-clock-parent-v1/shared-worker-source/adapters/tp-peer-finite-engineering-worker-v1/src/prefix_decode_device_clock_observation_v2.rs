//! Raw KFD samples around the V1 dispatch recorder, not calibrated GPU time.
use crate::prefix_decode_device_observation_v1::{self as raw, require};
use serde::{Deserialize, Serialize};
use std::io;

pub const SCHEMA: &str = "FerricPrefixDecodeDeviceClockObservationV2";
pub const MAX_BYTES: usize = raw::MAX_BYTES;
pub const SAMPLE_COUNT: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Endpoint {
    Pre,
    Post,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    pub generation: u64,
    pub position: u32,
    pub endpoint: Endpoint,
    pub rank: u32,
    /// Number of V1 packet rows already recorded when this sample was taken.
    pub row_boundary: u32,
    pub group_incarnation: u64,
    pub unique_id: u64,
    pub queue_epoch: u64,
    pub gpu_id: u32,
    pub gpu_clock_counter: u64,
    pub cpu_clock_counter: u64,
    pub system_clock_counter: u64,
    pub system_clock_frequency_hz: u64,
    /// Checked offsets from one process-local Instant origin, not wall time.
    pub host_started_ns: u64,
    pub host_finished_ns: u64,
}

pub(crate) fn validate_sample(
    sample: &Sample,
    index: usize,
    prior: &[Sample],
    group: u64,
    ranks: &[raw::Rank; 2],
) -> io::Result<()> {
    require(
        index < SAMPLE_COUNT && prior.len() == index,
        "clock sample bound/order",
    )?;
    let position = index / 4;
    let rank = index % 2;
    let post = index % 4 >= 2;
    require(
        sample.generation == position as u64 + 1
            && sample.position == position as u32
            && sample.endpoint == (if post { Endpoint::Post } else { Endpoint::Pre })
            && sample.rank == rank as u32
            && sample.row_boundary == ((position + usize::from(post)) * raw::PER_FORWARD) as u32
            && sample.group_incarnation == group
            && sample.unique_id == ranks[rank].unique_id
            && sample.queue_epoch == ranks[rank].queue_epoch
            && sample.system_clock_frequency_hz != 0
            && sample.host_finished_ns >= sample.host_started_ns
            && prior
                .last()
                .is_none_or(|p| sample.host_started_ns >= p.host_finished_ns),
        "clock sample identity/endpoint/host bracket",
    )?;
    if index >= 2 {
        require(
            sample.gpu_id == prior[rank].gpu_id
                && sample.system_clock_frequency_hz == prior[rank].system_clock_frequency_hz,
            "clock sample KFD identity/frequency changed",
        )?;
    } else if rank == 1 {
        require(
            sample.gpu_id != prior[0].gpu_id,
            "clock sample duplicate KFD identity",
        )?;
    }
    // Counter order and dispatch clock-domain equivalence are deliberately not
    // inferred here. A future same-device conversion must qualify both.
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema: String,
    pub raw: raw::Report,
    pub samples: Vec<Sample>,
    pub raw_clock_counters: bool,
    pub clock_domain_validated: bool,
    pub calibrated_nanoseconds: bool,
    pub cross_device_clock_alignment: bool,
    pub overlap_claim: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
    pub full_model_acceptance: bool,
}
impl Report {
    pub fn validate(&self) -> io::Result<()> {
        self.raw.validate()?;
        require(
            self.schema == SCHEMA
                && self.samples.len() == SAMPLE_COUNT
                && self.raw_clock_counters
                && !self.clock_domain_validated
                && !self.calibrated_nanoseconds
                && !self.cross_device_clock_alignment
                && !self.overlap_claim
                && !self.numerical_acceptance
                && !self.performance_claim
                && !self.production_authority
                && !self.full_model_acceptance,
            "clock report census/non-authority",
        )?;
        for (index, sample) in self.samples.iter().enumerate() {
            validate_sample(
                sample,
                index,
                &self.samples[..index],
                self.raw.group_incarnation,
                &self.raw.ranks,
            )?;
        }
        Ok(())
    }
    pub fn validate_control(
        &self,
        position: u32,
        control: &crate::finite_prefix_decode_wire_v1::Control,
    ) -> io::Result<()> {
        self.validate()?;
        self.raw.validate_control(position, control)
    }
    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        require(
            !bytes.is_empty() && bytes.len() <= MAX_BYTES,
            "clock sidecar bound",
        )?;
        let value: Self = serde_json::from_slice(bytes).map_err(io::Error::other)?;
        value.validate()?;
        Ok(value)
    }
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        self.validate()?;
        let mut writer = Bounded(Vec::new());
        serde_json::to_writer(&mut writer, self).map_err(io::Error::other)?;
        Ok(writer.0)
    }
}

struct Bounded(Vec<u8>);
impl io::Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        require(
            bytes.len() <= MAX_BYTES.saturating_sub(self.0.len()),
            "clock sidecar bound",
        )?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "prefix_decode_device_clock_observation_v2_tests.rs"]
pub(crate) mod tests;
