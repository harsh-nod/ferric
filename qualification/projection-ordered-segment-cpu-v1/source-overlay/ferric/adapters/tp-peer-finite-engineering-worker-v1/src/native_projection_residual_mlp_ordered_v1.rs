//! Explicit ordered AR4 shared-full route with distinct bootstrap and Control.
use crate::finite_projection_residual_mlp_ordered_wire_v1::{
    Bootstrap, Completion, FrameBudget, InputMode,
};
use crate::native_prefix_decode_host_v1::{
    executable_sha, ns, preflight_path, snapshot_shared_policy,
};
use crate::native_projection_residual_decode_cli_v1 as plain;
use crate::projection_residual_mlp_ordered_observation_v1::{
    self as data, Interval, Policy, Report, SharedReport, difference,
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
    time::{Duration, Instant},
};

pub struct Options {
    native: plain::NativeOptions,
    path: PathBuf,
    policy: Policy,
}
pub fn parse_args(args: &[OsString]) -> io::Result<Options> {
    parse_policy_args(args, Policy::OrderedSharedFull)
}
fn parse_policy_args(args: &[OsString], policy: Policy) -> io::Result<Options> {
    if args.len() != 10 || args[0] != policy.worker_flag() || args[8] != "--host-sidecar" {
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
    Ok(Options {
        native,
        path,
        policy,
    })
}

trait ObservationSetup {
    fn configure(&mut self, options: (bool, bool, bool)) -> io::Result<()>;
    fn enable(&mut self) -> io::Result<()>;
}
impl ObservationSetup for Group {
    fn configure(&mut self, (cache, operational, shared): (bool, bool, bool)) -> io::Result<()> {
        self.configure_performance_v2(cache, operational, shared)
            .map_err(io::Error::other)
    }
    fn enable(&mut self) -> io::Result<()> {
        self.enable_host_observation_v1().map_err(io::Error::other)
    }
}
fn configure_observer(setup: &mut impl ObservationSetup, policy: Policy) -> io::Result<u64> {
    let configuration_host_ns = if policy.shared() {
        let started = Instant::now();
        setup.configure((false, false, true))?;
        ns(started.elapsed())?
    } else {
        0
    };
    setup.enable()?;
    Ok(configuration_host_ns)
}

pub(crate) struct Recorder {
    previous: NativeSnapshot,
    report: Report,
    policy: Policy,
    configuration_host_ns: u64,
}
impl Recorder {
    pub(crate) fn enable(
        group: &mut Group,
        bootstrap: &Bootstrap,
        worker: [u8; 32],
    ) -> io::Result<Self> {
        Self::enable_policy(group, bootstrap, worker, Policy::OrderedSharedFull)
    }
    fn enable_policy(
        group: &mut Group,
        bootstrap: &Bootstrap,
        worker: [u8; 32],
        policy: Policy,
    ) -> io::Result<Self> {
        bootstrap.validate(
            bootstrap.decode.device_ids,
            bootstrap.decode.timeout_ms,
            std::process::id(),
            InputMode::Autoregressive,
        )?;
        // Configure before observer enable; setup and all forwards use one fixed policy.
        let configuration_host_ns = configure_observer(group, policy)?;
        let fresh = group.host_observation_v1().map_err(io::Error::other)?;
        let initial = snapshot_shared_policy(
            &fresh,
            data::PHASES[0],
            bootstrap.decode.device_ids,
            policy.shared(),
        )?;
        if initial.shared != [0; 4] || initial.ranks.iter().any(|r| r.counters != [0; 19]) {
            return Err(io::Error::other("projection host baseline not fresh"));
        }
        Ok(Self {
            previous: fresh,
            policy,
            configuration_host_ns,
            report: Report {
                schema: policy.schema().into(),
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
        let current = snapshot_shared_policy(
            &raw,
            phase,
            self.report.bootstrap.decode.device_ids,
            self.policy.shared(),
        )?;
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
        self.report.validate_for_policy(self.policy)?;
        if executable_sha()? != self.report.worker_sha256 {
            return Err(io::Error::other("projection host executable changed"));
        }
        let value = SharedReport {
            schema: data::SHARED_ENVELOPE.into(),
            policy: self.policy,
            configuration_host_ns: self.configuration_host_ns,
            observation: self.report,
        };
        value.validate()?;
        let raw = serde_json::to_vec(&value).map_err(io::Error::other)?;
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
    let (b, prepared) = prepare(&options.native, r, &mut incoming)?;
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
    serve_observed(owner, recorder, &options.path, r, w, &b, &mut incoming)
}

fn prepare(
    options: &plain::NativeOptions,
    r: &mut impl Read,
    incoming: &mut FrameBudget,
) -> io::Result<(
    Bootstrap,
    crate::native_setup::prefix_decode_v6::PreparedPrefixDecodeSetup,
)> {
    use crate::finite_projection_residual_mlp_ordered_wire_v1 as wire;
    use crate::native_setup::{PreparedSetup, prefix_decode_v6::PreparedPrefixDecodeSetup};
    use crate::resident_layer::{
        mlp_tiles_v2::Image as MlpImage,
        prefix_tiles_v6::{artifacts::Image as PrefixImage, projection_residual::Image},
    };
    let (b, mlp, prefix, projection) = wire::read_bootstrap(r, incoming)?
        .ok_or_else(|| io::Error::other("ordered bootstrap absent"))?;
    b.validate(
        options.devices,
        options.timeout_ms,
        std::process::id(),
        options.mode,
    )?;
    let (request, payload) = wire::read_begin(r, incoming, &b)?;
    let setup = PreparedSetup::prepare(request, payload, options.devices, &b.decode.scope)
        .map_err(io::Error::other)?;
    let prepared = PreparedPrefixDecodeSetup::new_projection(
        setup,
        PrefixImage::new(prefix, &b.decode.prefix_image).map_err(io::Error::other)?,
        MlpImage::new(mlp, &b.decode.tiles_image).map_err(io::Error::other)?,
        Image::new(projection, &b.projection_residual_image).map_err(io::Error::other)?,
        crate::native_catalog::forward::prefix_tiles_decode_v6::Mode::Autoregressive {
            first: b.decode.input_tokens[0],
        },
        options.timeout_ms,
    )
    .and_then(PreparedPrefixDecodeSetup::ordered)
    .map_err(io::Error::other)?;
    if prepared.profile_sha256() != b.sha256()? {
        return Err(io::Error::other("ordered profile mismatch"));
    }
    Ok((b, prepared))
}

use crate::forward_sequence::ForwardInput;
use crate::native_catalog::forward::prefix_tiles_decode_v6::{Owner, Run};
struct OrderedNative {
    owner: Option<Owner>,
    profile: [u8; 32],
    observer: Option<Recorder>,
    serialization_started: Option<std::time::Instant>,
}
impl crate::native_prefix_decode_cli_v1::Backend for OrderedNative {
    fn ordered(&self) -> bool {
        true
    }
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run> {
        let started = self.observer.as_ref().map(|_| std::time::Instant::now());
        let owner = self
            .owner
            .as_mut()
            .ok_or_else(|| io::Error::other("projection owner consumed"))?;
        let run = owner.run(self.profile, input).map_err(io::Error::other)?;
        if let Some(observer) = self.observer.as_mut() {
            let elapsed = started
                .ok_or_else(|| io::Error::other("projection host timer absent"))?
                .elapsed();
            observer.forward(owner.host_observation().map_err(io::Error::other)?, elapsed)?;
            self.serialization_started = Some(std::time::Instant::now());
        }
        Ok(run)
    }
    fn completed(&mut self, completion: &Completion) -> io::Result<()> {
        if let Some(observer) = self.observer.as_mut() {
            let elapsed = self
                .serialization_started
                .take()
                .ok_or_else(|| io::Error::other("projection serialization timer absent"))?
                .elapsed();
            observer.completed(completion, elapsed)?;
        }
        Ok(())
    }
    fn close(&mut self) -> io::Result<()> {
        let mut owner = self
            .owner
            .take()
            .ok_or_else(|| io::Error::other("projection owner consumed"))?;
        if let Some(observer) = self.observer.as_mut() {
            observer.record(owner.host_observation().map_err(io::Error::other)?)?;
        }
        let started = self.observer.as_ref().map(|_| std::time::Instant::now());
        owner.close().map_err(io::Error::other)?;
        if let Some(observer) = self.observer.as_mut() {
            observer.closed(
                started
                    .ok_or_else(|| io::Error::other("projection Close timer absent"))?
                    .elapsed(),
            )?;
        }
        Ok(())
    }
    fn failed(&mut self) {
        self.observer = None;
        self.serialization_started = None;
        if let Some(owner) = self.owner.as_mut() {
            owner.poison_device_recording();
        }
    }
}

fn serve_observed(
    owner: Owner,
    observer: Recorder,
    path: &Path,
    r: &mut impl Read,
    w: &mut impl Write,
    b: &Bootstrap,
    incoming: &mut FrameBudget,
) -> io::Result<()> {
    use crate::native_prefix_decode_cli_v1::Backend;
    let mut backend = OrderedNative {
        owner: Some(owner),
        profile: b.sha256()?,
        observer: Some(observer),
        serialization_started: None,
    };
    let result = crate::native_prefix_decode_cli_v1::serve_profile(
        &mut backend,
        r,
        w,
        &b.decode,
        incoming,
        b.sha256()?,
    );
    if let Err(error) = result {
        backend.failed();
        return Err(error);
    }
    backend
        .observer
        .take()
        .ok_or_else(|| io::Error::other("ordered observer missing"))?
        .finish(path)
}
#[cfg(test)]
#[path = "native_projection_residual_mlp_ordered_v1_tests.rs"]
mod tests;
