//! Parent wall spans only; the ordinary request, worker and wire are unchanged.
use super::{FilePin, Observation, ReadinessConfig, Result, hash, long, require};
use serde::{Deserialize, Serialize};
use std::{fs::OpenOptions, io::Write, os::unix::fs::OpenOptionsExt, time::Instant};

const SCHEMA: &str = "FerricReadiness40Position5ParentHostTimingV1";
const WRAPPER_SCHEMA: &str = "FerricReadiness40Position5TimedObservationV1";
const FILE_BYTES: u64 = 64 << 10;
const SUMMARY_BYTES: u64 = 128 << 10;
const EVENTS: usize = 124;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Event {
    Spawn,
    Sealed,
    Flushed(u32),
    Read(u32),
    Committed(u32),
    Retired,
    Finished,
}
fn expected_event(index: usize) -> Option<Event> {
    match index {
        0 => Some(Event::Spawn),
        1 => Some(Event::Sealed),
        2..=121 => {
            let position = ((index - 2) / 3) as u32;
            Some(match (index - 2) % 3 {
                0 => Event::Flushed(position),
                1 => Event::Read(position),
                _ => Event::Committed(position),
            })
        }
        122 => Some(Event::Retired),
        123 => Some(Event::Finished),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Span {
    start_ns: u64,
    end_ns: u64,
    elapsed_ns: u64,
}
impl Span {
    fn new(start_ns: u64, end_ns: u64) -> Result<Self> {
        Ok(Self {
            start_ns,
            end_ns,
            elapsed_ns: end_ns
                .checked_sub(start_ns)
                .ok_or("host timing clock regression")?,
        })
    }
    fn advance(&self, cursor: &mut u64, sum: &mut u64) -> Result<()> {
        require(
            self.start_ns == *cursor
                && self.end_ns.checked_sub(self.start_ns) == Some(self.elapsed_ns),
            "host timing gap, overlap or duration mismatch",
        )?;
        *sum = sum
            .checked_add(self.elapsed_ns)
            .ok_or("host timing sum overflow")?;
        *cursor = self.end_ns;
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Forward {
    position: u32,
    generation: u64,
    prepare_write: Span,
    flush_to_frame_read: Span,
    validate_retain_commit: Span,
    elapsed_ns: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Timeline {
    source_preparation: Span,
    spawn_to_setup_seal: Span,
    forwards: Vec<Forward>,
    close_and_retirement: Span,
    postcheck_and_ordinary_publication: Span,
    total_ns: u64,
}
impl Timeline {
    fn validate(&self) -> Result<()> {
        require(self.forwards.len() == 40, "host timing exact forty rows")?;
        let (mut cursor, mut sum) = (0, 0);
        self.source_preparation.advance(&mut cursor, &mut sum)?;
        self.spawn_to_setup_seal.advance(&mut cursor, &mut sum)?;
        for (position, row) in self.forwards.iter().enumerate() {
            require(
                row.position as usize == position && row.generation == position as u64 + 1,
                "host timing ordered position/generation",
            )?;
            let before = cursor;
            row.prepare_write.advance(&mut cursor, &mut sum)?;
            row.flush_to_frame_read.advance(&mut cursor, &mut sum)?;
            row.validate_retain_commit.advance(&mut cursor, &mut sum)?;
            require(
                cursor.checked_sub(before) == Some(row.elapsed_ns),
                "host timing forward extent",
            )?;
        }
        self.close_and_retirement.advance(&mut cursor, &mut sum)?;
        self.postcheck_and_ordinary_publication
            .advance(&mut cursor, &mut sum)?;
        require(
            cursor == self.total_ns && sum == self.total_ns,
            "host timing exact disjoint reconciliation",
        )
    }
}
struct Trace {
    times: Vec<u64>,
    poisoned: bool,
}
impl Trace {
    fn new() -> Self {
        Self {
            times: Vec::with_capacity(EVENTS),
            poisoned: false,
        }
    }
    fn at(&mut self, event: Event, ns: u64) -> Result<()> {
        if self.poisoned
            || expected_event(self.times.len()) != Some(event)
            || self.times.last().is_some_and(|prior| ns < *prior)
        {
            self.poisoned = true;
            return Err("host timing event/order/clock boundary".into());
        }
        self.times.push(ns);
        Ok(())
    }
    fn finish(self) -> Result<Timeline> {
        require(
            !self.poisoned && self.times.len() == EVENTS,
            "host timing incomplete or failed trace",
        )?;
        let t = self.times;
        let mut cursor = t[1];
        let mut forwards = Vec::with_capacity(40);
        for position in 0..40 {
            let i = 2 + position * 3;
            forwards.push(Forward {
                position: position as u32,
                generation: position as u64 + 1,
                prepare_write: Span::new(cursor, t[i])?,
                flush_to_frame_read: Span::new(t[i], t[i + 1])?,
                validate_retain_commit: Span::new(t[i + 1], t[i + 2])?,
                elapsed_ns: t[i + 2]
                    .checked_sub(cursor)
                    .ok_or("host timing forward clock regression")?,
            });
            cursor = t[i + 2];
        }
        let value = Timeline {
            source_preparation: Span::new(0, t[0])?,
            spawn_to_setup_seal: Span::new(t[0], t[1])?,
            forwards,
            close_and_retirement: Span::new(cursor, t[122])?,
            postcheck_and_ordinary_publication: Span::new(t[122], t[123])?,
            total_ns: t[123],
        };
        value.validate()?;
        Ok(value)
    }
}
pub(super) struct Recorder {
    started: Instant,
    trace: Trace,
}
impl Recorder {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            trace: Trace::new(),
        }
    }
    fn mark(&mut self, event: Event) -> Result<()> {
        let ns = u64::try_from(self.started.elapsed().as_nanos())
            .map_err(|_| "host timing clock extent")?;
        self.trace.at(event, ns)
    }
    fn finish(mut self) -> Result<Timeline> {
        self.mark(Event::Finished)?;
        self.trace.finish()
    }
}
pub(super) fn mark(recorder: &mut Option<Recorder>, event: Event) -> Result<()> {
    match recorder {
        Some(value) => value.mark(event),
        None => Ok(()),
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Report {
    schema: String,
    ordinary_complete: FilePin,
    profile_sha256: [u8; 32],
    transcript_sha256: [u8; 32],
    completed_forwards: u32,
    generated_tokens: u32,
    capture_positions: [u32; 4],
    timeline: Timeline,
    native_closed: bool,
    child_exit_zero: bool,
    process_group_absent: bool,
    parent_host_measurement: bool,
    gpu_timing: bool,
    nested_control_timers_included: bool,
    sidecar_publication_timed: bool,
    full_long_workload: bool,
    numerical_acceptance: bool,
    performance_claim: bool,
    production_authority: bool,
}
impl Report {
    fn validate(&self) -> Result<()> {
        self.timeline.validate()?;
        require(
            self.schema == SCHEMA
                && self.ordinary_complete.bytes > 0
                && self.ordinary_complete.bytes <= SUMMARY_BYTES
                && self.ordinary_complete.path.is_absolute()
                && self.ordinary_complete.sha256 != [0; 32]
                && self.profile_sha256 != [0; 32]
                && self.transcript_sha256 != [0; 32]
                && self.completed_forwards == 40
                && self.generated_tokens == 0
                && self.capture_positions == [0, 5, 16, 39]
                && self.native_closed
                && self.child_exit_zero
                && self.process_group_absent
                && self.parent_host_measurement
                && !self.gpu_timing
                && !self.nested_control_timers_included
                && !self.sidecar_publication_timed
                && !self.full_long_workload
                && !self.numerical_acceptance
                && !self.performance_claim
                && !self.production_authority,
            "host timing complete scope and false authority",
        )
    }
}
#[derive(Serialize)]
pub struct TimingStatus {
    pub complete: bool,
    pub file: FilePin,
    pub ordinary_retained_bytes: u64,
    pub retained_bytes_with_timing: u64,
    pub supervisor_metadata_allowance: u64,
    pub parent_host_measurement: bool,
    pub gpu_timing: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
}
#[derive(Serialize)]
pub struct TimedObservation {
    pub schema: &'static str,
    pub observation: Observation,
    pub host_timing: TimingStatus,
}
fn retained_total(ordinary: u64, sidecar: u64, reserve: u64) -> Result<u64> {
    let total = ordinary
        .checked_add(sidecar)
        .ok_or("host timing retained extent overflow")?;
    require(
        sidecar > 0
            && sidecar <= FILE_BYTES
            && total
                .checked_add(reserve)
                .is_some_and(|n| n <= long::EVIDENCE_BYTES as u64),
        "host timing unchanged aggregate evidence bound",
    )?;
    Ok(total)
}
fn publish(report: &Report, ordinary: u64, reserve: u64) -> Result<TimingStatus> {
    report.validate()?;
    let original = report.ordinary_complete.read(SUMMARY_BYTES, true)?;
    let directory = report
        .ordinary_complete
        .path
        .parent()
        .ok_or("host timing evidence parent absent")?;
    require(
        report
            .ordinary_complete
            .path
            .file_name()
            .is_some_and(|name| name == "complete.json")
            && directory.canonicalize().map_err(|e| e.to_string())? == directory,
        "host timing original complete parent",
    )?;
    let mut raw = serde_json::to_vec(report).map_err(|e| e.to_string())?;
    raw.push(b'\n');
    let total = retained_total(ordinary, raw.len() as u64, reserve)?;
    let pending = directory.join("host-timing.pending");
    let path = directory.join("host-timing.json");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&pending)
        .map_err(|e| e.to_string())?;
    file.write_all(&raw)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    let pending_pin = FilePin {
        path: pending.clone(),
        bytes: raw.len() as u64,
        sha256: hash(&raw),
    };
    require(
        pending_pin.read(FILE_BYTES, true)? == raw,
        "host timing pending body changed",
    )?;
    require(
        report.ordinary_complete.read(SUMMARY_BYTES, true)? == original,
        "host timing original completion changed",
    )?;
    std::fs::hard_link(&pending, &path).map_err(|e| e.to_string())?;
    std::fs::remove_file(pending).map_err(|e| e.to_string())?;
    std::fs::File::open(directory)
        .and_then(|dir| dir.sync_all())
        .map_err(|e| e.to_string())?;
    let pin = FilePin {
        path,
        bytes: raw.len() as u64,
        sha256: hash(&raw),
    };
    require(
        pin.read(FILE_BYTES, true)? == raw,
        "host timing published body changed",
    )?;
    Ok(TimingStatus {
        complete: true,
        file: pin,
        ordinary_retained_bytes: ordinary,
        retained_bytes_with_timing: total,
        supervisor_metadata_allowance: reserve,
        parent_host_measurement: true,
        gpu_timing: false,
        numerical_acceptance: false,
        performance_claim: false,
    })
}

/// Measures the existing Position5 route; no new worker, request or wire mode.
fn admit_entry(schema: &str, opted_in: bool) -> Result<()> {
    require(
        opted_in && schema == super::POSITION5_REQUEST_SCHEMA,
        "host timing explicit Position5 entry and machine-code opt-in",
    )
}
/// Returns a separate timing wrapper without changing the ordinary observation.
pub fn run_position5_host_timing(
    config: ReadinessConfig,
    allow_unauthenticated_machine_code: bool,
) -> Result<TimedObservation> {
    admit_entry(&config.schema, allow_unauthenticated_machine_code)?;
    let mut recorder = Some(Recorder::new());
    let observation = super::run_inner(
        config,
        allow_unauthenticated_machine_code,
        long::Profile::Readiness40Position5,
        false,
        &mut recorder,
    )?;
    let timeline = recorder
        .take()
        .ok_or("host timing recorder absent")?
        .finish()?;
    // run_inner returns only after actual Close, child retirement and ordinary publication.
    super::validate_summary(&observation)?;
    require(
        observation.request.schema == super::POSITION5_REQUEST_SCHEMA
            && observation.causal_layer_zero.is_none(),
        "host timing ordinary Position5 observation",
    )?;
    let mut raw = serde_json::to_vec(&observation).map_err(|e| e.to_string())?;
    raw.push(b'\n');
    let original = FilePin {
        path: observation
            .request
            .base
            .evidence_directory
            .join("complete.json"),
        bytes: raw.len() as u64,
        sha256: hash(&raw),
    };
    require(
        original.read(SUMMARY_BYTES, true)? == raw,
        "host timing actual ordinary completion",
    )?;
    let report = Report {
        schema: SCHEMA.into(),
        ordinary_complete: original,
        profile_sha256: observation.profile_sha256,
        transcript_sha256: observation.transcript_sha256,
        completed_forwards: observation.completed_forwards,
        generated_tokens: u32::try_from(observation.generated_tokens.len())
            .map_err(|_| "host timing generated extent")?,
        capture_positions: [0, 5, 16, 39],
        timeline,
        native_closed: observation.native_closed,
        child_exit_zero: observation.child_exit_zero,
        process_group_absent: observation.process_group_absent,
        parent_host_measurement: true,
        gpu_timing: false,
        nested_control_timers_included: false,
        sidecar_publication_timed: false,
        full_long_workload: false,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    };
    let host_timing = publish(
        &report,
        observation.files.total_bytes,
        observation.files.supervisor_metadata_allowance,
    )?;
    Ok(TimedObservation {
        schema: WRAPPER_SCHEMA,
        observation,
        host_timing,
    })
}

#[cfg(test)]
#[path = "readiness_host_timing_tests.rs"]
mod tests;
