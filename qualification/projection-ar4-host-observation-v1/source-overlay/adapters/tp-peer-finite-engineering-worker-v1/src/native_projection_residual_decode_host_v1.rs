//! Separate AR4 host observer; the authenticated projection wire is unchanged.
use crate::finite_projection_residual_decode_wire_v1::{
    Bootstrap, Completion, FrameBudget, InputMode,
};
use crate::native_prefix_decode_host_v1::{executable_sha, ns, preflight_path, snapshot};
use crate::native_projection_residual_decode_cli_v1 as plain;
use crate::projection_residual_decode_host_observation_v1::{
    self as data, Interval, Report, difference,
};
use fe2o3_kfd::{
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerHostObservationV1 as NativeSnapshot,
};
use std::{
    ffi::OsString,
    fs::OpenOptions,
    io::{self, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

pub struct Options {
    native: plain::NativeOptions,
    path: PathBuf,
}
pub fn parse_args(args: &[OsString]) -> io::Result<Options> {
    if args.len() != 10
        || args[0] != "--engineering-native-projection-residual-decode-host-v1"
        || args[8] != "--host-sidecar"
    {
        return Err(io::Error::other("exact projection host arguments required"));
    }
    let mut ordinary = args[..8].to_vec();
    ordinary[0] = "--engineering-native-projection-residual-decode-v1".into();
    let native = plain::parse_args(&ordinary)?;
    let path = PathBuf::from(&args[9]);
    if native.mode != InputMode::Autoregressive
        || !path.is_absolute()
        || path.file_name().is_none()
        || path.as_os_str().len() > 512
    {
        return Err(io::Error::other(
            "projection host requires AR4 and bounded absolute sidecar",
        ));
    }
    Ok(Options { native, path })
}

pub(crate) struct Recorder {
    previous: NativeSnapshot,
    report: Report,
}
impl Recorder {
    pub(crate) fn enable(
        group: &mut Group,
        bootstrap: &Bootstrap,
        worker: [u8; 32],
    ) -> io::Result<Self> {
        bootstrap.validate(
            bootstrap.decode.device_ids,
            bootstrap.decode.timeout_ms,
            std::process::id(),
            InputMode::Autoregressive,
        )?;
        group
            .enable_host_observation_v1()
            .map_err(io::Error::other)?;
        let fresh = group.host_observation_v1().map_err(io::Error::other)?;
        let initial = snapshot(&fresh, data::PHASES[0], bootstrap.decode.device_ids)?;
        if initial.shared != [0; 4] || initial.ranks.iter().any(|r| r.counters != [0; 19]) {
            return Err(io::Error::other("projection host baseline not fresh"));
        }
        Ok(Self {
            previous: fresh,
            report: Report {
                schema: data::SCHEMA.into(),
                bootstrap: bootstrap.clone(),
                worker_sha256: worker,
                child_pid: std::process::id(),
                profile_sha256: bootstrap.sha256()?,
                snapshots: vec![initial],
                intervals: Vec::with_capacity(6),
                forward_host_ns: [0; 4],
                serialization_host_ns: [0; 4],
                close_host_ns: 0,
                completions: Vec::with_capacity(4),
                transcript_sha256: [0; 32],
                native_closed: false,
                inclusive_nested_host_scopes: true,
                gpu_time: false,
                numerical_acceptance: false,
                performance_claim: false,
                production_authority: false,
            },
        })
    }
    pub(crate) fn record(&mut self, raw: NativeSnapshot) -> io::Result<()> {
        let phase = *data::PHASES
            .get(self.report.snapshots.len())
            .ok_or_else(|| io::Error::other("too many projection host snapshots"))?;
        let delta = raw
            .checked_delta(&self.previous)
            .map_err(io::Error::other)?;
        let current = snapshot(&raw, phase, self.report.bootstrap.decode.device_ids)?;
        let previous = self
            .report
            .snapshots
            .last()
            .ok_or_else(|| io::Error::other("host baseline absent"))?;
        let interval = Interval {
            host_elapsed_ns: delta.host_elapsed_ns(),
            ranks: [
                difference(&current.ranks[0].counters, &previous.ranks[0].counters)?,
                difference(&current.ranks[1].counters, &previous.ranks[1].counters)?,
            ],
            shared: difference(&current.shared, &previous.shared)?,
        };
        self.report.intervals.push(interval);
        self.report.snapshots.push(current);
        self.previous = raw;
        Ok(())
    }
    pub(crate) fn forward(&mut self, raw: NativeSnapshot, elapsed: Duration) -> io::Result<()> {
        let position = self
            .report
            .snapshots
            .len()
            .checked_sub(2)
            .ok_or_else(|| io::Error::other("projection host setup snapshot absent"))?;
        if position >= 4 || self.report.completions.len() != position {
            return Err(io::Error::other("projection host forward order"));
        }
        self.report.forward_host_ns[position] = ns(elapsed)?;
        self.record(raw)
    }
    pub(crate) fn completed(
        &mut self,
        completion: &Completion,
        elapsed: Duration,
    ) -> io::Result<()> {
        record_completion(&mut self.report, completion, ns(elapsed)?)
    }
    pub(crate) fn closed(&mut self, elapsed: Duration) -> io::Result<()> {
        record_close(&mut self.report, ns(elapsed)?)
    }
    pub(crate) fn finish(self, path: &Path) -> io::Result<()> {
        self.report.validate()?;
        if executable_sha()? != self.report.worker_sha256 {
            return Err(io::Error::other("projection host executable changed"));
        }
        let raw = serde_json::to_vec(&self.report).map_err(io::Error::other)?;
        if raw.len() > data::MAX_BYTES {
            return Err(io::Error::other("projection host report bound"));
        }
        preflight_path(path)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        output.write_all(&raw)?;
        output.sync_all()
    }
}
fn record_completion(report: &mut Report, completion: &Completion, elapsed: u64) -> io::Result<()> {
    let position = report.completions.len();
    if position >= 4
        || report.snapshots.len() != position + 3
        || completion.position as usize != position
        || report.native_closed
    {
        return Err(io::Error::other("projection host completion order"));
    }
    report.serialization_host_ns[position] = elapsed;
    report.completions.push(completion.clone());
    Ok(())
}
fn record_close(report: &mut Report, elapsed: u64) -> io::Result<()> {
    if report.snapshots.len() != 7 || report.completions.len() != 4 || report.native_closed {
        return Err(io::Error::other(
            "projection host Close before four completions",
        ));
    }
    report.close_host_ns = elapsed;
    report.native_closed = true;
    report.transcript_sha256 = report.completions[3].chain;
    Ok(())
}

/// # Safety
/// The plain projection route's owned-child and trusted-machine-code obligations
/// remain unchanged. These counters confer neither timing nor arithmetic authority.
#[allow(unsafe_code)]
pub unsafe fn run_native(
    options: Options,
    r: &mut impl Read,
    w: &mut impl Write,
) -> io::Result<()> {
    preflight_path(&options.path)?;
    let worker = executable_sha()?;
    let mut incoming = FrameBudget::new();
    let (b, prepared) = plain::prepare(&options.native, r, &mut incoming)?;
    let mut group =
        unsafe { Group::open_unchecked(&prepared.device_ids()) }.map_err(io::Error::other)?;
    let mut recorder = Recorder::enable(&mut group, &b, worker)?;
    let mut setup = prepared.into_processor(group).map_err(io::Error::other)?;
    setup.serve(r, w).map_err(io::Error::other)?;
    if setup.is_closed() {
        return Err(io::Error::other(
            "projection host setup closed before forwards",
        ));
    }
    let mut owner = unsafe { setup.into_decode() }.map_err(io::Error::other)?;
    recorder.record(owner.host_observation().map_err(io::Error::other)?)?;
    crate::native_prefix_decode_cli_v1::serve_projection_observed(
        owner,
        recorder,
        &options.path,
        r,
        w,
        &b,
        &mut incoming,
    )
}

#[cfg(test)]
#[path = "native_projection_residual_decode_host_v1_tests.rs"]
mod tests;
