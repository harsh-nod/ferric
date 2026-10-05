//! Explicit host-timing opt-in; the Four wire and executor are shared unchanged.
use crate::finite_prefix_decode_wire_v1::{Bootstrap, Completion};
use crate::native_prefix_decode_cli_v1::{self as plain, NativeOptions};
use crate::prefix_decode_host_observation_v1::{self as data, Interval, Rank, Report, Snapshot};
use fe2o3_kfd::{
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerHostObservationV1 as NativeSnapshot,
};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct Options {
    pub(crate) native: NativeOptions,
    pub(crate) path: PathBuf,
}
pub fn parse_args(args: &[OsString]) -> io::Result<Options> {
    if args.len() != 10
        || args[0] != "--engineering-native-prefix-decode-host-v1"
        || args[8] != "--host-sidecar"
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
    Ok(Options { native, path })
}
pub(crate) fn preflight_path(path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("host sidecar parent"))?;
    if parent.canonicalize()? != parent || std::fs::symlink_metadata(path).is_ok() {
        return Err(io::Error::other(
            "host sidecar must be new under canonical parent",
        ));
    }
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        _ => Err(io::Error::other("host sidecar path unavailable")),
    }
}
pub(crate) fn executable_sha() -> io::Result<[u8; 32]> {
    let path = std::env::current_exe()?.canonicalize()?;
    let mut file = File::open(&path)?;
    let before = file.metadata()?;
    if !before.is_file() || before.len() == 0 || before.len() > 512 << 20 {
        return Err(io::Error::other("host diagnostic executable bound"));
    }
    let mut hash = Sha256::new();
    let mut block = [0; 65536];
    let mut total = 0_u64;
    loop {
        let n = file.read(&mut block)?;
        if n == 0 {
            break;
        }
        total = total
            .checked_add(n as u64)
            .ok_or_else(|| io::Error::other("executable extent"))?;
        if total > before.len() {
            return Err(io::Error::other("executable grew"));
        }
        hash.update(&block[..n]);
    }
    let after = file.metadata()?;
    let named = std::fs::symlink_metadata(path)?;
    let identity = |m: &std::fs::Metadata| {
        (
            m.dev(),
            m.ino(),
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    };
    if total != before.len()
        || !named.is_file()
        || identity(&before) != identity(&after)
        || identity(&after) != identity(&named)
    {
        return Err(io::Error::other("host diagnostic executable changed"));
    }
    Ok(hash.finalize().into())
}
pub(crate) fn ns(value: Duration) -> io::Result<u64> {
    u64::try_from(value.as_nanos()).map_err(io::Error::other)
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
pub(crate) fn snapshot(
    raw: &NativeSnapshot,
    phase: &str,
    devices: [u64; 2],
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
    value.validate(devices)?;
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
    ) -> io::Result<Self> {
        group
            .enable_host_observation_v1()
            .map_err(io::Error::other)?;
        let fresh = group.host_observation_v1().map_err(io::Error::other)?;
        let initial = snapshot(&fresh, data::PHASES[0], bootstrap.device_ids)?;
        if initial.shared != [0; 4] || initial.ranks.iter().any(|r| r.counters != [0; 19]) {
            return Err(io::Error::other("host enable baseline not fresh"));
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
        let current = snapshot(&raw, phase, self.report.bootstrap.device_ids)?;
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
/// Same trusted-parent Four premises; this only enables independent host timers.
#[allow(unsafe_code)]
pub unsafe fn run_native(
    options: Options,
    r: &mut impl Read,
    w: &mut impl Write,
) -> io::Result<()> {
    preflight_path(&options.path)?;
    let worker = executable_sha()?;
    unsafe { plain::run_observed(options.native, r, w, Some((options.path, worker))) }
}

#[cfg(test)]
#[path = "native_prefix_decode_host_v1_tests.rs"]
mod tests;
