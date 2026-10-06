//! Explicit single-factor host policies; the Four wire and executor are unchanged.
use crate::finite_prefix_decode_wire_v1::{Bootstrap, Completion};
use crate::native_prefix_decode_cli_v1::{self as plain, NativeOptions};
use crate::native_prefix_decode_host_v1::{executable_sha, ns, preflight_path};
use crate::prefix_decode_host_observation_v2::{
    self as data, Interval, Policy, Rank, Report, Snapshot,
};
use fe2o3_kfd::{
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerHostObservationV1 as NativeSnapshot,
};
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct Options {
    pub(crate) native: NativeOptions,
    pub(crate) path: PathBuf,
    pub(crate) policy: Policy,
}
pub fn parse_args(args: &[OsString]) -> io::Result<Options> {
    if args.len() != 12
        || args[0] != "--engineering-native-prefix-decode-host-v2"
        || args[8] != "--host-sidecar"
        || args[10] != "--host-policy"
    {
        return Err(io::Error::other("exact host diagnostic arguments required"));
    }
    let mut plain_args = args[..8].to_vec();
    plain_args[0] = "--engineering-native-prefix-decode-v1".into();
    let native = plain::parse_args(&plain_args)?;
    let path = PathBuf::from(&args[9]);
    if !path.is_absolute() || path.file_name().is_none() || path.as_os_str().len() > 512 {
        return Err(io::Error::other("host sidecar absolute bounded path"));
    }
    let policy = Policy::parse(
        args[11]
            .to_str()
            .ok_or_else(|| io::Error::other("host policy UTF8"))?,
    )?;
    Ok(Options {
        native,
        path,
        policy,
    })
}
macro_rules! counters {
    ($c:expr) => {{
        let c = $c;
        [
            c.commands,
            c.command_ns,
            c.full_currentness_checks,
            c.full_currentness_ns,
            c.operational_currentness_checks,
            c.operational_currentness_ns,
            c.kernel_admissions,
            c.kernel_admission_ns,
            c.dispatches,
            c.dispatch_prepare_ns,
            c.dispatch_publish_ns,
            c.dispatch_wait_ns,
            c.completion_polls,
            c.reads,
            c.read_bytes,
            c.read_ns,
            c.writes,
            c.write_bytes,
            c.write_ns,
        ]
    }};
}
macro_rules! shared {
    ($c:expr) => {{
        let c = $c;
        [
            c.group_full_checks,
            c.group_full_ns,
            c.publication_full_checks,
            c.publication_full_ns,
        ]
    }};
}
fn snapshot(
    raw: &NativeSnapshot,
    phase: &str,
    devices: [u64; 2],
    policy: Policy,
) -> io::Result<Snapshot> {
    let ranks = raw
        .participants()
        .iter()
        .map(|r| Rank {
            rank: r.rank() as u32,
            unique_id: r.unique_id(),
            queue_epoch: r.queue_epoch(),
            cache_kernel_admission: r.cache_kernel_admission(),
            raw_timestamp_queue: r.raw_timestamp_queue(),
            counters: counters!(r.counters()),
        })
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| io::Error::other("host rank count"))?;
    let value = Snapshot {
        phase: phase.into(),
        group_incarnation: raw.group_incarnation(),
        shared_full_currentness: raw.shared_full_currentness(),
        ranks,
        shared: shared!(raw.shared_counters()),
    };
    value.validate(devices, policy)?;
    Ok(value)
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
        policy: Policy,
    ) -> io::Result<Self> {
        // Even A configures explicitly, before observer enable and all allocations.
        // The runtime owns admission-cache and full mutable-currentness semantics.
        let (cache, operational, shared) = policy.options();
        group
            .configure_performance_v2(cache, operational, shared)
            .map_err(io::Error::other)?;
        group
            .enable_host_observation_v1()
            .map_err(io::Error::other)?;
        let fresh = group.host_observation_v1().map_err(io::Error::other)?;
        let initial = snapshot(&fresh, data::PHASES[0], bootstrap.device_ids, policy)?;
        if initial.shared != [0; 4] || initial.ranks.iter().any(|r| r.counters != [0; 19]) {
            return Err(io::Error::other("host enable baseline not fresh"));
        }
        Ok(Self {
            previous: fresh,
            report: Report {
                schema: data::SCHEMA.into(),
                policy,
                bootstrap: bootstrap.clone(),
                worker_sha256: worker,
                child_pid: std::process::id(),
                profile_sha256: bootstrap.sha256()?,
                snapshots: vec![initial],
                intervals: Vec::with_capacity(6),
                forward_host_ns: [0; 4],
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
            .ok_or_else(|| io::Error::other("too many host snapshots"))?;
        let delta = raw
            .checked_delta(&self.previous)
            .map_err(io::Error::other)?;
        let current = snapshot(
            &raw,
            phase,
            self.report.bootstrap.device_ids,
            self.report.policy,
        )?;
        let ranks = delta
            .participants()
            .iter()
            .map(|p| counters!(p.counters()))
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| io::Error::other("host delta ranks"))?;
        self.report.intervals.push(Interval {
            host_elapsed_ns: delta.host_elapsed_ns(),
            ranks,
            shared: shared!(delta.shared_counters()),
        });
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
            .ok_or_else(|| io::Error::other("host setup snapshot missing"))?;
        if position >= 4 || self.report.completions.len() != position {
            return Err(io::Error::other("host forward snapshot order"));
        }
        self.report.forward_host_ns[position] = ns(elapsed)?;
        self.record(raw)
    }
    pub(crate) fn completed(&mut self, completion: &Completion) -> io::Result<()> {
        let position = self.report.completions.len();
        if position >= 4
            || self.report.snapshots.len() != position + 3
            || completion.position as usize != position
        {
            return Err(io::Error::other("host completion snapshot order"));
        }
        self.report.completions.push(completion.clone());
        Ok(())
    }
    pub(crate) fn closed(&mut self, elapsed: Duration) -> io::Result<()> {
        if self.report.snapshots.len() != 7 || self.report.completions.len() != 4 {
            return Err(io::Error::other("host Close before all snapshots"));
        }
        self.report.close_host_ns = ns(elapsed)?;
        self.report.native_closed = true;
        self.report.transcript_sha256 = self.report.completions[3].chain;
        Ok(())
    }
    pub(crate) fn finish(self, path: &Path) -> io::Result<()> {
        self.report.validate()?;
        if executable_sha()? != self.report.worker_sha256 {
            return Err(io::Error::other("host observer executable drift"));
        }
        let raw = serde_json::to_vec(&self.report).map_err(io::Error::other)?;
        if raw.len() > data::MAX_BYTES {
            return Err(io::Error::other("host sidecar bound"));
        }
        preflight_path(path)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        output.write_all(&raw)?;
        output.sync_all()?;
        Ok(())
    }
}

/// # Safety
/// Same trusted-parent Four premises, with a closed explicit host-policy opt-in.
#[allow(unsafe_code)]
pub unsafe fn run_native(
    options: Options,
    r: &mut impl Read,
    w: &mut impl Write,
) -> io::Result<()> {
    preflight_path(&options.path)?;
    let worker = executable_sha()?;
    unsafe {
        plain::run_policy_observed(options.native, r, w, options.path, worker, options.policy)
    }
}

#[cfg(test)]
#[path = "native_prefix_decode_host_v2_tests.rs"]
mod tests;
