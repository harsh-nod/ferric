//! Private descendant of the V1 recorder. Production samples come from Owner.
use super::{Recorder as RawRecorder, require};
use crate::finite_prefix_decode_wire_v1::{Bootstrap, Completion, Control};
use crate::forward_sequence::ForwardInput;
use crate::native_catalog::forward::prefix_tiles_decode_v6::{Owner, Run};
use crate::prefix_decode_device_clock_observation_v2::{self as data, Endpoint, Report, Sample};
use crate::prefix_decode_device_observation_v1 as raw_data;
use fe2o3_kfd::{
    Gfx950EngineeringPeerClockObservationV1 as NativeSample, Gfx950EngineeringPeerGroupV1 as Group,
};
use std::io;
use std::time::{Duration, Instant};

pub(crate) struct Recorder {
    raw: RawRecorder,
    origin: Instant,
    samples: Vec<Sample>,
    failed: bool,
}
/// Constructed only after consuming the V1 ClosedReport, never from a bool.
pub(crate) struct ClosedReport(Report);
impl ClosedReport {
    pub(crate) fn encode(&self) -> io::Result<Vec<u8>> {
        self.0.encode()
    }
    pub(crate) fn worker_sha256(&self) -> [u8; 32] {
        self.0.raw.worker_sha256
    }
}
fn duration_ns(elapsed: Duration) -> io::Result<u64> {
    u64::try_from(elapsed.as_nanos()).map_err(io::Error::other)
}
fn offset(origin: Instant, value: Instant) -> io::Result<u64> {
    let elapsed = value
        .checked_duration_since(origin)
        .ok_or_else(|| io::Error::other("clock host bracket precedes origin"))?;
    duration_ns(elapsed)
}
impl Recorder {
    pub(crate) fn enable(group: &mut Group, b: &Bootstrap, worker: [u8; 32]) -> io::Result<Self> {
        let origin = Instant::now();
        Ok(Self {
            raw: RawRecorder::enable(group, b, worker)?,
            origin,
            samples: Vec::with_capacity(data::SAMPLE_COUNT),
            failed: false,
        })
    }
    fn active(&self) -> io::Result<()> {
        require(!self.failed, "clock recorder terminal")?;
        self.raw.active()
    }
    pub(crate) fn poison(&mut self) {
        self.failed = true;
        self.raw.failed = true;
    }
    // The production callback always poisons the real Owner, including errors
    // discovered after the underlying forward has committed its state bank.
    fn finish<T>(&mut self, result: io::Result<T>, poison: impl FnOnce()) -> io::Result<T> {
        if result.is_err() {
            self.poison();
            poison();
        }
        result
    }
    pub(crate) fn bind_images(&mut self, owner: &mut Owner) -> io::Result<()> {
        let result = (|| {
            self.active()?;
            require(self.samples.is_empty(), "clock binding precedes samples")?;
            owner
                .bind_device_images(&mut self.raw)
                .map_err(io::Error::other)
        })();
        self.finish(result, || owner.poison_device_recording())
    }
    fn before_forward(&self, profile: [u8; 32], input: &ForwardInput) -> io::Result<usize> {
        self.active()?;
        let n = self.raw.report.completions.len();
        require(
            n < 4
                && self.raw.bound
                && self.raw.pending_control.is_none()
                && self.samples.len() == n * 4
                && self.raw.report.rows.len() == n * raw_data::PER_FORWARD
                && profile == self.raw.report.profile_sha256
                && input.generation == n as u64 + 1
                && input.cache_metadata[0] == n as u32,
            "clock forward identity/row/sample order",
        )?;
        Ok(n)
    }
    fn sample(&mut self, owner: &mut Owner, completed: usize) -> io::Result<()> {
        let observations = owner
            .sample_device_clocks(completed as u64)
            .map_err(io::Error::other)?;
        require(observations.len() == 2, "clock sample TP2 census")?;
        let index = self.samples.len();
        let mut values = Vec::with_capacity(2);
        for observation in observations {
            values.push(self.project(observation, index + values.len())?);
        }
        self.append_pair(values)
    }
    fn project(&self, observation: NativeSample, index: usize) -> io::Result<Sample> {
        require(index < data::SAMPLE_COUNT, "clock sample projection bound")?;
        let counters = observation.counters();
        Ok(Sample {
            generation: (index / 4) as u64 + 1,
            position: (index / 4) as u32,
            endpoint: if index % 4 < 2 {
                Endpoint::Pre
            } else {
                Endpoint::Post
            },
            rank: u32::try_from(observation.rank()).map_err(io::Error::other)?,
            row_boundary: u32::try_from(self.raw.report.rows.len()).map_err(io::Error::other)?,
            group_incarnation: observation.group_incarnation(),
            unique_id: observation.unique_id(),
            queue_epoch: observation.queue_epoch(),
            gpu_id: counters.gpu_id(),
            gpu_clock_counter: counters.gpu_clock_counter(),
            cpu_clock_counter: counters.cpu_clock_counter(),
            system_clock_counter: counters.system_clock_counter(),
            system_clock_frequency_hz: counters.system_clock_frequency_hz(),
            host_started_ns: offset(self.origin, observation.sample_started())?,
            host_finished_ns: offset(self.origin, observation.sample_finished())?,
        })
    }
    // Private test seam. The production caller above uses only native getters.
    fn append_pair(&mut self, values: Vec<Sample>) -> io::Result<()> {
        self.active()?;
        let index = self.samples.len();
        require(
            index % 2 == 0 && values.len() == 2 && index + 2 <= data::SAMPLE_COUNT,
            "clock sample pair census",
        )?;
        let n = self.raw.report.completions.len();
        require(
            index == n * 4 || index == n * 4 + 2,
            "clock pair completion order",
        )?;
        let boundary = (n + usize::from(index % 4 == 2)) * raw_data::PER_FORWARD;
        require(
            self.raw.report.rows.len() == boundary,
            "clock pair actual packet boundary",
        )?;
        let mut joined = self.samples.clone();
        for value in values {
            data::validate_sample(
                &value,
                joined.len(),
                &joined,
                self.raw.report.group_incarnation,
                &self.raw.report.ranks,
            )?;
            joined.push(value);
        }
        self.samples = joined;
        Ok(())
    }
    pub(crate) fn run(
        &mut self,
        owner: &mut Owner,
        profile: [u8; 32],
        input: &ForwardInput,
    ) -> io::Result<Run> {
        let result = (|| {
            let n = self.before_forward(profile, input)?;
            self.sample(owner, n)?;
            let run = owner
                .run_recorded(profile, input, &mut self.raw)
                .map_err(io::Error::other)?;
            require(
                self.raw.report.rows.len() == (n + 1) * raw_data::PER_FORWARD,
                "clock forward missing raw rows",
            )?;
            self.sample(owner, n + 1)?;
            Ok(run)
        })();
        self.finish(result, || owner.poison_device_recording())
    }
    fn ready_control(&self) -> io::Result<()> {
        self.active()?;
        let n = self.raw.report.completions.len();
        require(
            n < 4 && self.samples.len() == (n + 1) * 4,
            "clock control requires pre/post samples",
        )
    }
    pub(crate) fn control(&mut self, owner: &mut Owner, control: &Control) -> io::Result<()> {
        let result = self
            .ready_control()
            .and_then(|()| self.raw.control(control));
        self.finish(result, || owner.poison_device_recording())
    }
    pub(crate) fn completed(
        &mut self,
        owner: &mut Owner,
        completion: &Completion,
    ) -> io::Result<()> {
        let result = self
            .ready_control()
            .and_then(|()| self.raw.completed(completion));
        self.finish(result, || owner.poison_device_recording())
    }
    fn ready_close(&self) -> io::Result<()> {
        self.active()?;
        require(
            self.samples.len() == data::SAMPLE_COUNT
                && self.raw.report.rows.len() == raw_data::MAX_ROWS
                && self.raw.report.completions.len() == 4
                && self.raw.pending_control.is_none(),
            "clock recorder incomplete Close",
        )?;
        for (index, sample) in self.samples.iter().enumerate() {
            data::validate_sample(
                sample,
                index,
                &self.samples[..index],
                self.raw.report.group_incarnation,
                &self.raw.report.ranks,
            )?;
        }
        Ok(())
    }
    pub(crate) fn close(mut self, mut owner: Owner) -> io::Result<ClosedReport> {
        let ready = self.ready_close();
        self.finish(ready, || owner.poison_device_recording())?;
        self.close_with(|raw| raw.close(owner))
    }
    fn close_with(
        self,
        close: impl FnOnce(RawRecorder) -> io::Result<super::ClosedReport>,
    ) -> io::Result<ClosedReport> {
        self.ready_close()?;
        let closed = close(self.raw)?;
        let report = Report {
            schema: data::SCHEMA.into(),
            raw: closed.0,
            samples: self.samples,
            raw_clock_counters: true,
            clock_domain_validated: false,
            calibrated_nanoseconds: false,
            cross_device_clock_alignment: false,
            overlap_claim: false,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
            full_model_acceptance: false,
        };
        report.validate()?;
        Ok(ClosedReport(report))
    }
}

#[cfg(test)]
#[path = "native_prefix_device_clock_recorder_v2_tests.rs"]
mod tests;
