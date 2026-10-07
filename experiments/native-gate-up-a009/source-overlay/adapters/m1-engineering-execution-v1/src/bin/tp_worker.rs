//! Owned-byte IPC to a separately opted-in, non-authoritative KFD process.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use engineering_wire::{
    self as wire, BufferAccessV1, CommandV1, KernelMetadataV1, OrderedBatchDispatchV1,
    PointerFixupV1, ResponseV1, SequenceDispatchV1,
};
use fe2o3_hsaco::{
    ArgumentAccess, ExplicitArgument, ExplicitValueKind, ExplicitValueType, InspectedKernel,
};
#[cfg(not(any(feature = "model-timestamps", feature = "c1-ordered64")))]
use fe2o3_kfd::engineering_wire;
#[cfg(any(feature = "model-timestamps", feature = "c1-ordered64"))]
use fe2o3_kfd_current_wire::engineering_wire;
use ferric_m1_engineering_execution_v1::host_timing::{HostTiming, IpcTiming};
use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpArtifactV1;
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTpArgumentV1, EngineeringTpBufferAccessV1, EngineeringTpDispatchV1,
    EngineeringTpRankTransportV1, TpResult,
};
use sha2::{Digest, Sha256};

const OP_TIMEOUT: Duration = Duration::from_mins(2);
const EXIT_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(feature = "model-timestamps")]
#[path = "tp_worker_model_timestamps.rs"]
mod model_timestamps;

#[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
#[path = "tp_worker_token_program.rs"]
mod token_program;

#[derive(Clone, Copy, Default)]
#[allow(clippy::struct_excessive_bools)] // Independent ablation flags, not lifecycle state.
pub struct RuntimeOptions {
    pub cache_admission: bool,
    pub operational: bool,
    pub sequences: bool,
    pub ordered_batches: bool,
    pub ordered64: bool,
    pub rollover: bool,
    pub shared_full_currentness: bool,
    pub profile: bool,
    pub ordered64_runtime_counters: bool,
    pub ordered64_packet_ticks: bool,
}

impl RuntimeOptions {
    fn validate_ordered64(self) -> TpResult<()> {
        if self.ordered64_packet_ticks
            && (!cfg!(all(feature = "model-timestamps", feature = "c1-ordered64"))
                || !self.ordered64
                || self.profile
                || self.ordered64_runtime_counters)
        {
            return Err(
                "ordered64 packet ticks require their separate unprofiled diagnostic".into(),
            );
        }
        if self.ordered64_runtime_counters && (!self.ordered64 || !self.profile) {
            return Err("ordered64 runtime counters require their explicit profiled mode".into());
        }
        if self.ordered64
            && (!cfg!(feature = "c1-ordered64")
                || (cfg!(feature = "model-timestamps") && !self.ordered64_packet_ticks)
                || !self.ordered_batches
                || self.sequences
                || self.shared_full_currentness
                || (self.profile && !self.ordered64_runtime_counters))
        {
            return Err("ordered64 requires its explicit build and dedicated ordered non-diagnostic runtime".into());
        }
        Ok(())
    }

    #[allow(dead_code)] // Only the dedicated ordered64 diagnostic can select counters.
    pub fn with_ordered64_runtime_counters(mut self) -> TpResult<Self> {
        self.validate_ordered64()?;
        if !self.ordered64
            || self.profile
            || self.ordered64_runtime_counters
            || self.ordered64_packet_ticks
        {
            return Err("runtime counters require fresh unprofiled ordered64 options".into());
        }
        self.profile = true;
        self.ordered64_runtime_counters = true;
        self.validate_ordered64()?;
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenProgramBackend {
    Ordered64GroupsV1,
    NativeWholeProgramV1,
    NativeWholeProgramSlots512V1,
}

impl TokenProgramBackend {
    pub const fn identity(self) -> &'static str {
        match self {
            Self::Ordered64GroupsV1 => "ordered64-groups-v1",
            Self::NativeWholeProgramV1 => "native-whole-program-v1",
            Self::NativeWholeProgramSlots512V1 => "native-whole-program-slots512-v1",
        }
    }

    pub const fn argument(self) -> &'static str {
        match self {
            Self::Ordered64GroupsV1 => "--diagnostic-token-program-v1",
            Self::NativeWholeProgramV1 => "--diagnostic-token-program-native-v1",
            Self::NativeWholeProgramSlots512V1 => "--diagnostic-token-program-native-slots512-v1",
        }
    }

    #[allow(dead_code)] // Legacy controllers share the worker but not token latency metadata.
    pub const fn profile(self) -> &'static str {
        match self {
            Self::Ordered64GroupsV1 => "prefill16-decode-fixed-token-program-v1",
            Self::NativeWholeProgramV1 => "prefill16-decode-native-whole-token-program-v1",
            Self::NativeWholeProgramSlots512V1 => "prefill-width-native-slots512-v1",
        }
    }

    pub const fn counter_profile(self) -> &'static str {
        match self {
            Self::Ordered64GroupsV1 => "prefill16-decode-fixed-token-program-counters-v1",
            Self::NativeWholeProgramV1 => "prefill16-decode-native-whole-token-program-counters-v1",
            Self::NativeWholeProgramSlots512V1 => "prefill-width-native-slots512-counters-v1",
        }
    }
}

#[derive(Clone, Copy)]
enum WorkerEntry {
    Ordinary,
    TokenProgram,
    NativeTokenProgram,
    TokenProgramCounters(TokenProgramBackend),
    NativePrefillProgram { counters: bool },
    NativePrefillWidthProgram { counters: bool, rows: u32 },
    NativeGateUpProgram { counters: bool, enabled: bool },
    ActivePoll10ms,
}

impl WorkerEntry {
    fn token_backend(self) -> Option<TokenProgramBackend> {
        match self {
            Self::TokenProgram => Some(TokenProgramBackend::Ordered64GroupsV1),
            Self::NativeTokenProgram | Self::NativePrefillProgram { .. } => {
                Some(TokenProgramBackend::NativeWholeProgramV1)
            }
            Self::TokenProgramCounters(backend) => Some(backend),
            Self::NativePrefillWidthProgram { .. } | Self::NativeGateUpProgram { .. } => {
                Some(TokenProgramBackend::NativeWholeProgramSlots512V1)
            }
            Self::Ordinary | Self::ActivePoll10ms => None,
        }
    }

    fn command(
        self,
        executable: &Path,
        unique_id: u64,
        options: RuntimeOptions,
        timing: &HostTiming,
        rank: u32,
    ) -> TpResult<Command> {
        options.validate_ordered64()?;
        if matches!(self, Self::ActivePoll10ms) && !timing.is_ordered64_diagnostic() {
            return Err("active polling requires the bounded ordered64 host recorder".into());
        }
        let token_counters = matches!(
            self,
            Self::TokenProgramCounters(_)
                | Self::NativePrefillProgram { counters: true }
                | Self::NativePrefillWidthProgram { counters: true, .. }
                | Self::NativeGateUpProgram { counters: true, .. }
        );
        if matches!(self, Self::NativePrefillWidthProgram { rows, .. } if !matches!(rows, 16 | 32))
        {
            return Err("native prefill width must be explicitly 16 or 32".into());
        }
        if self.token_backend().is_some()
            && (!cfg!(all(
                feature = "c1-token-program",
                not(feature = "model-timestamps")
            )) || !options.ordered64
                || options.profile != token_counters
                || options.ordered64_runtime_counters != token_counters
                || options.ordered64_packet_ticks
                || timing.is_enabled()
                || rank != 0)
        {
            return Err("token program requires its separate unprofiled TP1 process entry".into());
        }
        if matches!(self, Self::ActivePoll10ms)
            && (!cfg!(all(
                feature = "c1-ordered64",
                not(feature = "model-timestamps")
            )) || !options.ordered64
                || !options.profile
                || !options.ordered64_runtime_counters
                || options.ordered64_packet_ticks
                || rank != 0)
        {
            return Err("active polling requires the separate profiled ordered64 TP1 entry".into());
        }
        if options.shared_full_currentness {
            return Err("shared full currentness requires the explicit peer worker".into());
        }
        if options.sequences && options.ordered_batches {
            return Err("ordered batches and legacy dispatch sequences are distinct modes".into());
        }
        let mut command = Command::new(executable);
        command
            .arg("--device-unique-id")
            .arg(unique_id.to_string())
            .arg("--allow-unauthenticated-machine-code");
        match self {
            Self::Ordinary => {}
            Self::TokenProgram
            | Self::NativeTokenProgram
            | Self::TokenProgramCounters(_)
            | Self::NativePrefillProgram { .. }
            | Self::NativePrefillWidthProgram { .. }
            | Self::NativeGateUpProgram { .. } => {
                command.arg(self.token_backend().expect("token entry").argument());
            }
            Self::ActivePoll10ms => {
                command.arg("--diagnostic-active-poll-10ms");
            }
        }
        Ok(command)
    }
}

struct Outgoing {
    header: CommandV1,
    payload: Vec<u8>,
}

struct Incoming {
    header: ResponseV1,
    payload: Vec<u8>,
}

pub(super) struct LoadedKernel {
    pub(super) id: u64,
    pub(super) image: [u8; 32],
    pub(super) metadata: InspectedKernel,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum PendingRequest {
    Other,
    Dispatch,
    Sequence(usize),
    OrderedBatch(usize),
    #[cfg(feature = "c1-ordered64")]
    OrderedBatch64(usize),
    #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
    TokenRegister,
    #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
    TokenExecute {
        program: u64,
        epoch: u64,
        next: u64,
    },
    #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
    TokenRelease {
        program: u64,
        epoch: u64,
    },
}

struct WorkerTiming {
    timing: HostTiming,
    rank: u32,
    pending: Option<IpcTiming>,
}

pub struct Worker {
    child: Child,
    writer: Option<SyncSender<Outgoing>>,
    written: Receiver<TpResult<()>>,
    reader: Receiver<TpResult<Incoming>>,
    io_threads: Vec<JoinHandle<()>>,
    buffers: BTreeMap<u64, usize>,
    kernels: BTreeMap<String, LoadedKernel>,
    pending: Option<PendingRequest>,
    failed: bool,
    exited: bool,
    timeout: Duration,
    options: RuntimeOptions,
    queue_epoch: u64,
    queue_packets: u64,
    timing: Option<Box<WorkerTiming>>,
    diagnostic_identity: (u64, u32),
    diagnostic_snapshots: u32,
    last_diagnostic: Option<Box<wire::PerformanceCountersV1>>,
    #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
    token_program: Option<Box<token_program::State>>,
    #[cfg(feature = "model-timestamps")]
    model_timestamps: Option<Box<model_timestamps::State>>,
}

impl Worker {
    // Shared with the legacy token-at-a-time executable.
    #[allow(dead_code)]
    pub fn spawn(
        executable: &Path,
        unique_id: u64,
        artifact: &EngineeringTpArtifactV1,
    ) -> TpResult<Self> {
        Self::spawn_with_options(executable, unique_id, artifact, RuntimeOptions::default())
    }

    pub fn spawn_with_options(
        executable: &Path,
        unique_id: u64,
        artifact: &EngineeringTpArtifactV1,
        options: RuntimeOptions,
    ) -> TpResult<Self> {
        Self::spawn_with_timing(
            executable,
            unique_id,
            artifact,
            options,
            HostTiming::default(),
            0,
        )
    }

    pub fn spawn_with_timing(
        executable: &Path,
        unique_id: u64,
        artifact: &EngineeringTpArtifactV1,
        options: RuntimeOptions,
        timing: HostTiming,
        rank: u32,
    ) -> TpResult<Self> {
        Self::spawn_mode(
            executable, unique_id, artifact, options, timing, rank, false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_mode(
        executable: &Path,
        unique_id: u64,
        artifact: &EngineeringTpArtifactV1,
        options: RuntimeOptions,
        timing: HostTiming,
        rank: u32,
        token_program: bool,
    ) -> TpResult<Self> {
        let entry = if token_program {
            WorkerEntry::TokenProgram
        } else {
            WorkerEntry::Ordinary
        };
        Self::spawn_entry(
            executable, unique_id, artifact, options, timing, rank, entry,
        )
    }

    #[allow(dead_code)] // Only the explicit profiled ordered64 diagnostic selects this entry.
    pub fn spawn_active_poll_with_timing(
        executable: &Path,
        unique_id: u64,
        artifact: &EngineeringTpArtifactV1,
        options: RuntimeOptions,
        timing: HostTiming,
        rank: u32,
    ) -> TpResult<Self> {
        Self::spawn_entry(
            executable,
            unique_id,
            artifact,
            options,
            timing,
            rank,
            WorkerEntry::ActivePoll10ms,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_entry(
        executable: &Path,
        unique_id: u64,
        artifact: &EngineeringTpArtifactV1,
        options: RuntimeOptions,
        timing: HostTiming,
        rank: u32,
        entry: WorkerEntry,
    ) -> TpResult<Self> {
        let mut command = entry.command(executable, unique_id, options, &timing, rank)?;
        let child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("spawn GPU worker: {error}"))?;
        let mut worker = Self::connect_with_timing(child, unique_id, OP_TIMEOUT, timing, rank)?;
        #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
        if let Some(backend) = entry.token_backend() {
            worker.verify_token_program_backend(backend)?;
            if matches!(
                entry,
                WorkerEntry::NativePrefillProgram { .. }
                    | WorkerEntry::NativePrefillWidthProgram { .. }
            ) {
                worker
                    .token_program
                    .as_mut()
                    .expect("verified native backend")
                    .prefill_enabled = true;
            }
            if let WorkerEntry::NativePrefillWidthProgram { rows, .. } = entry {
                worker
                    .token_program
                    .as_mut()
                    .expect("verified slots512 backend")
                    .prefill_rows = rows;
            }
            if let WorkerEntry::NativeGateUpProgram { enabled, .. } = entry {
                worker.configure_gate_up_shape(enabled)?;
            }
        }
        worker.configure_options(options)?;
        #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
        if matches!(
            entry,
            WorkerEntry::TokenProgramCounters(_)
                | WorkerEntry::NativePrefillProgram { counters: true }
                | WorkerEntry::NativePrefillWidthProgram { counters: true, .. }
                | WorkerEntry::NativeGateUpProgram { counters: true, .. }
        ) {
            worker.emit_token_program_counters()?;
        }
        worker.load_artifact(artifact)?;
        Ok(worker)
    }

    fn configure_options(&mut self, options: RuntimeOptions) -> TpResult<()> {
        if let Err(error) = options.validate_ordered64() {
            return self.reject(error);
        }
        if (self.options.ordered64 != options.ordered64
            || self.options.ordered64_packet_ticks != options.ordered64_packet_ticks
            || self.options.ordered64_runtime_counters != options.ordered64_runtime_counters)
            && (self.queue_packets != 0 || self.queue_epoch != 0 || self.pending.is_some())
        {
            return self.reject("ordered arena mode cannot change after queue use");
        }
        if options.sequences && options.ordered_batches {
            return self.reject("ordered batches and legacy dispatch sequences are incompatible");
        }
        self.options = options;
        if options.cache_admission || options.operational || options.profile {
            self.send(
                CommandV1::ConfigurePerformance {
                    cache_kernel_admission: options.cache_admission,
                    operational_currentness: options.operational,
                    profile: options.profile,
                },
                vec![],
            )?;
            if !matches!(self.receive()?.header, ResponseV1::PerformanceConfigured) {
                return self.reject("runtime performance configuration was not acknowledged");
            }
        }
        Ok(())
    }

    #[cfg(test)]
    fn connect(child: Child, unique_id: u64, timeout: Duration) -> TpResult<Self> {
        Self::connect_with_timing(child, unique_id, timeout, HostTiming::default(), 0)
    }

    fn connect_with_timing(
        mut child: Child,
        unique_id: u64,
        timeout: Duration,
        timing: HostTiming,
        timing_rank: u32,
    ) -> TpResult<Self> {
        let Some(mut input) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("worker stdin missing".into());
        };
        let Some(mut output) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("worker stdout missing".into());
        };
        let (writer, pending_frames) = mpsc::sync_channel::<Outgoing>(1);
        let (acknowledge, written) = mpsc::sync_channel(1);
        let (responses, reader) = mpsc::sync_channel(1);
        // Both pipe directions have independent bounded queues. The controller
        // never blocks in a pipe write when a child stops consuming input.
        let write_thread = thread::spawn(move || {
            while let Ok(frame) = pending_frames.recv() {
                let result = (|| {
                    wire::write_header_v1(&mut input, &frame.header)?;
                    input.write_all(&frame.payload)?;
                    input.flush()
                })()
                .map_err(|error: std::io::Error| format!("worker pipe write: {error}"));
                let failed = result.is_err();
                if acknowledge.send(result).is_err() || failed {
                    break;
                }
            }
        });
        let read_thread = thread::spawn(move || {
            loop {
                let result = read_response(&mut output);
                let failed = result.is_err();
                if responses.send(result).is_err() || failed {
                    break;
                }
            }
        });
        let mut worker = Self {
            child,
            writer: Some(writer),
            written,
            reader,
            io_threads: vec![write_thread, read_thread],
            buffers: BTreeMap::new(),
            kernels: BTreeMap::new(),
            pending: Some(PendingRequest::Other),
            failed: false,
            exited: false,
            timeout,
            options: RuntimeOptions::default(),
            queue_epoch: 0,
            queue_packets: 0,
            diagnostic_identity: (unique_id, timing_rank),
            diagnostic_snapshots: 0,
            last_diagnostic: None,
            #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
            token_program: None,
            #[cfg(feature = "model-timestamps")]
            model_timestamps: None,
            timing: timing.is_enabled().then(|| {
                Box::new(WorkerTiming {
                    timing,
                    rank: timing_rank,
                    pending: None,
                })
            }),
        };
        let ready = worker.receive()?;
        if !matches!(ready.header, ResponseV1::Ready { protocol, target, device_unique_id, authority }
            if protocol == wire::PROTOCOL_VERSION_V1 && target == wire::TARGET_V1
                && device_unique_id == unique_id && authority == "none")
        {
            return worker.reject("worker ready identity mismatch");
        }
        Ok(worker)
    }

    /// Adds a disjoint admitted image only before any buffer allocation or dispatch.
    #[allow(dead_code)] // The legacy single-sequence controller shares this module.
    pub fn load_additional_artifact(&mut self, artifact: &EngineeringTpArtifactV1) -> TpResult<()> {
        let names = artifact
            .inspection()
            .hsaco()
            .kernels()
            .iter()
            .map(InspectedKernel::name)
            .collect::<Vec<_>>();
        self.with_additional_image(&names, |worker| worker.load_artifact(artifact))
    }

    fn with_additional_image(
        &mut self,
        names: &[&str],
        load: impl FnOnce(&mut Self) -> TpResult<()>,
    ) -> TpResult<()> {
        if self.failed
            || self.exited
            || self.pending.is_some()
            || !self.buffers.is_empty()
            || self.queue_packets != 0
            || self.queue_epoch != 0
            || names.is_empty()
            || names.iter().any(|name| self.kernels.contains_key(*name))
            || names
                .iter()
                .enumerate()
                .any(|(index, name)| names[..index].contains(name))
        {
            return Err(
                "additional artifact requires a fresh worker and disjoint kernel symbols".into(),
            );
        }
        let result = load(self);
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn load_artifact(&mut self, artifact: &EngineeringTpArtifactV1) -> TpResult<()> {
        let hash: [u8; 32] = Sha256::digest(artifact.bytes()).into();
        for metadata in artifact.inspection().hsaco().kernels() {
            let length =
                u32::try_from(artifact.bytes().len()).map_err(|_| "HSACO length overflow")?;
            self.send(
                CommandV1::LoadKernel {
                    payload_bytes: length,
                    object_sha256: hash,
                    symbol: metadata.name().into(),
                },
                artifact.bytes().to_vec(),
            )?;
            let response = self.receive()?;
            let ResponseV1::LoadedKernel {
                kernel,
                metadata: actual,
            } = response.header
            else {
                return self.reject("worker did not load kernel");
            };
            if kernel == 0
                || self.kernels.values().any(|loaded| loaded.id == kernel)
                || !metadata_matches(metadata, &actual, hash)
            {
                return self.reject("worker kernel metadata or identity mismatch");
            }
            self.kernels.insert(
                metadata.name().into(),
                LoadedKernel {
                    id: kernel,
                    image: hash,
                    metadata: metadata.clone(),
                },
            );
        }
        Ok(())
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    fn reject<T>(&mut self, message: impl Into<String>) -> TpResult<T> {
        self.failed = true;
        #[cfg(feature = "model-timestamps")]
        self.poison_model_timestamps();
        if let Some(timing) = &mut self.timing {
            timing.pending.take();
        }
        let message = message.into();
        let cleanup = self.terminate();
        Err(match cleanup {
            Ok(()) => message,
            Err(error) => format!("{message}; {error}"),
        })
    }

    fn ordered_limit(&self) -> usize {
        #[cfg(feature = "c1-ordered64")]
        if self.options.ordered64 {
            return wire::MAX_ORDERED_BATCH64_DISPATCHES_V1;
        }
        #[cfg(not(feature = "c1-ordered64"))]
        let _ = self;
        wire::MAX_ORDERED_BATCH_DISPATCHES_V1
    }

    fn ordered_pending(&self, count: usize) -> PendingRequest {
        #[cfg(feature = "c1-ordered64")]
        if self.options.ordered64 {
            return PendingRequest::OrderedBatch64(count);
        }
        #[cfg(not(feature = "c1-ordered64"))]
        let _ = self;
        PendingRequest::OrderedBatch(count)
    }

    fn send(&mut self, header: CommandV1, payload: Vec<u8>) -> TpResult<()> {
        if self.failed || self.exited || self.pending.is_some() {
            return self.reject("worker is not ready for a request");
        }
        #[cfg(any(
            feature = "model-timestamps",
            all(feature = "c1-ordered64", not(feature = "c1-token-program"))
        ))]
        if matches!(
            header,
            CommandV1::RegisterTokenProgram { .. }
                | CommandV1::RegisterTokenProgramSlots512V1 { .. }
                | CommandV1::ExecuteTokenProgram { .. }
                | CommandV1::ExecuteTokenProgramSlots512V1 { .. }
                | CommandV1::ReleaseTokenProgram { .. }
        ) {
            return self.reject("token commands require the explicit unprofiled token mode");
        }
        #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
        if matches!(
            header,
            CommandV1::Dispatch { .. }
                | CommandV1::DispatchSequence { .. }
                | CommandV1::DispatchOrderedBatch { .. }
                | CommandV1::DispatchOrderedBatch64 { .. }
                | CommandV1::DispatchOrderedBatch64Profiled { .. }
        ) {
            // Ordinary bootstrap/fallback must release the template before publication.
            self.release_registered_token()?;
        }
        #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
        if let Err(error) = self.validate_token_request(&header) {
            return self.reject(error);
        }
        if self.options.ordered64 && matches!(header, CommandV1::DispatchOrderedBatch { .. }) {
            return self.reject("ordered64 cannot publish an ordered16 packet group");
        }
        #[cfg(feature = "c1-ordered64")]
        if matches!(header, CommandV1::DispatchOrderedBatch64 { .. })
            && (!self.options.ordered64 || self.options.validate_ordered64().is_err())
        {
            return self.reject("ordered64 command was not explicitly admitted");
        }
        if header.payload_bytes().map_err(|error| error.to_string())? != payload.len() {
            return self.reject("outgoing payload length mismatch");
        }
        let (command, dispatches) = match &header {
            #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
            CommandV1::RegisterTokenProgram { .. } => ("register_token_program", 0),
            #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
            CommandV1::RegisterTokenProgramSlots512V1 { .. } => {
                ("register_token_program_slots512_v1", 0)
            }
            #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
            CommandV1::ExecuteTokenProgram { .. } => (
                "execute_token_program",
                self.registered_program_packets() as u64,
            ),
            #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
            CommandV1::ExecuteTokenProgramSlots512V1 { .. } => (
                "execute_token_program_slots512_v1",
                self.registered_program_packets() as u64,
            ),
            #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
            CommandV1::ReleaseTokenProgram { .. } => ("release_token_program", 0),
            CommandV1::Dispatch { .. } => ("dispatch", 1),
            CommandV1::DispatchSequence { dispatches } => {
                ("dispatch_sequence", dispatches.len() as u64)
            }
            CommandV1::DispatchOrderedBatch { dispatches, .. } => {
                ("dispatch_ordered_batch", dispatches.len() as u64)
            }
            #[cfg(feature = "c1-ordered64")]
            CommandV1::DispatchOrderedBatch64 { dispatches, .. } => {
                ("dispatch_ordered_batch64", dispatches.len() as u64)
            }
            CommandV1::Allocate { .. } => ("allocate", 0),
            CommandV1::Write { .. } => ("write", 0),
            CommandV1::Read { .. } => ("read", 0),
            CommandV1::LoadKernel { .. } => ("load_kernel", 0),
            CommandV1::ConfigurePerformance { .. } => ("configure_performance", 0),
            CommandV1::RolloverQueue { .. } => ("rollover_queue", 0),
            CommandV1::Close => ("close", 0),
            _ => ("other", 0),
        };
        if let Some(timing) = &mut self.timing {
            timing.pending = Some(timing.timing.request(
                Some(timing.rank),
                command,
                payload.len(),
                dispatches,
            ));
        }
        self.pending = Some(match &header {
            #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
            CommandV1::RegisterTokenProgram { .. }
            | CommandV1::RegisterTokenProgramSlots512V1 { .. } => PendingRequest::TokenRegister,
            #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
            CommandV1::ExecuteTokenProgram {
                program,
                expected_epoch,
                ..
            }
            | CommandV1::ExecuteTokenProgramSlots512V1 {
                program,
                expected_epoch,
                ..
            } => PendingRequest::TokenExecute {
                program: *program,
                epoch: *expected_epoch,
                next: self.queue_packets + self.registered_program_packets() as u64,
            },
            #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
            CommandV1::ReleaseTokenProgram {
                program,
                expected_epoch,
            } => PendingRequest::TokenRelease {
                program: *program,
                epoch: *expected_epoch,
            },
            CommandV1::Dispatch { .. } => PendingRequest::Dispatch,
            CommandV1::DispatchSequence { dispatches } => {
                PendingRequest::Sequence(dispatches.len())
            }
            CommandV1::DispatchOrderedBatch { dispatches, .. } => {
                PendingRequest::OrderedBatch(dispatches.len())
            }
            #[cfg(feature = "c1-ordered64")]
            CommandV1::DispatchOrderedBatch64 { dispatches, .. } => {
                PendingRequest::OrderedBatch64(dispatches.len())
            }
            _ => PendingRequest::Other,
        });
        #[cfg(feature = "model-timestamps")]
        let header = match self.profile_model_header(header, &payload) {
            Ok(header) => header,
            Err(error) => return self.reject(error),
        };
        let Some(writer) = &self.writer else {
            return self.reject("worker writer closed");
        };
        if writer.try_send(Outgoing { header, payload }).is_err() {
            return self.reject("worker writer unavailable");
        }
        let written = self.written.recv_timeout(self.timeout);
        if let Some(ticket) = self
            .timing
            .as_mut()
            .and_then(|timing| timing.pending.as_mut())
        {
            ticket.sent(matches!(&written, Ok(Ok(()))));
        }
        match written {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => self.reject(error),
            Err(error) => self.reject(format!("worker write deadline/disconnect: {error}")),
        }
    }

    fn receive(&mut self) -> TpResult<Incoming> {
        let _timing = self
            .timing
            .as_ref()
            .map(|timing| timing.timing.span("ipc_response_wait", Some(timing.rank)));
        if self.pending.is_none() || self.failed || self.exited {
            return self.reject("worker has no admissible pending response");
        }
        match self.reader.recv_timeout(self.timeout) {
            Ok(Ok(Incoming {
                header: ResponseV1::Error { message, fatal },
                ..
            })) => self.reject(format!("GPU worker failed (fatal={fatal}): {message}")),
            Ok(Ok(response)) => {
                #[cfg(feature = "model-timestamps")]
                let response = match self.validate_model_response(response) {
                    Ok(response) => response,
                    Err(error) => return self.reject(error),
                };
                self.pending = None;
                if let Some(ticket) = self
                    .timing
                    .as_mut()
                    .and_then(|timing| timing.pending.take())
                {
                    ticket.finish(response.payload.len(), true);
                }
                Ok(response)
            }
            Ok(Err(error)) => self.reject(error),
            Err(error) => self.reject(format!("worker response deadline/disconnect: {error}")),
        }
    }

    fn extent(&self, buffer: u64, offset: usize, bytes: usize) -> TpResult<()> {
        let capacity = self.buffers.get(&buffer).ok_or("unknown owned buffer")?;
        let end = offset.checked_add(bytes).ok_or("buffer extent overflow")?;
        if offset > *capacity || end > *capacity {
            return Err("buffer extent outside allocation".into());
        }
        Ok(())
    }

    fn terminate(&mut self) -> TpResult<()> {
        self.writer.take();
        if self.exited {
            return Ok(());
        }
        if self
            .child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_none()
        {
            self.child
                .kill()
                .map_err(|error| format!("cannot kill worker {}: {error}", self.pid()))?;
        }
        self.await_exit(false)
    }

    fn await_exit(&mut self, successful: bool) -> TpResult<()> {
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().map_err(|error| error.to_string())? {
                self.exited = true;
                self.writer.take();
                while self.io_threads.iter().any(|thread| !thread.is_finished()) {
                    if start.elapsed() >= EXIT_TIMEOUT {
                        return Err(format!(
                            "worker {} exited but IPC threads did not stop",
                            self.pid()
                        ));
                    }
                    thread::sleep(Duration::from_millis(1));
                }
                for handle in self.io_threads.drain(..) {
                    if handle.is_finished() {
                        let _ = handle.join();
                    }
                }
                return if successful && !status.success() {
                    Err(format!("worker exited with {status}"))
                } else {
                    Ok(())
                };
            }
            if start.elapsed() >= EXIT_TIMEOUT {
                return Err(format!(
                    "worker {} did not exit; resources are not confirmed released",
                    self.pid()
                ));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl EngineeringTpRankTransportV1 for Worker {
    #[cfg(feature = "model-timestamps")]
    fn model_timestamps_enabled(&self) -> bool {
        self.model_timestamps.is_some()
    }

    #[cfg(feature = "model-timestamps")]
    fn model_timestamp_ordered64_enabled(&self) -> bool {
        self.options.ordered64_packet_ticks && self.model_timestamps.is_some()
    }

    #[cfg(feature = "model-timestamps")]
    fn model_timestamp_batch(
        &mut self,
        batch: ferric_m1_engineering_execution_v1::model_timestamps::BatchIdentity,
    ) -> TpResult<()> {
        self.bind_model_timestamp_batch(batch)
    }

    #[cfg(feature = "model-timestamps")]
    fn model_timestamp_tag(
        &mut self,
        tag: ferric_m1_engineering_execution_v1::model_timestamps::PacketTag,
    ) -> TpResult<()> {
        self.enqueue_model_timestamp_tag(tag)
    }
    fn require_loaded_image(&mut self, image: [u8; 32], kernels: &[&str]) -> TpResult<()> {
        if self.failed
            || self.exited
            || self.pending.is_some()
            || !self.buffers.is_empty()
            || self.queue_packets != 0
            || self.queue_epoch != 0
            || image == [0; 32]
            || kernels.is_empty()
            || kernels.len() > 32
            || kernels.iter().enumerate().any(|(index, name)| {
                kernels[..index].contains(name)
                    || self
                        .kernels
                        .get(*name)
                        .is_none_or(|loaded| loaded.image != image)
            })
        {
            return self.reject("additional image binding is not fresh or complete");
        }
        Ok(())
    }
    fn runtime_diagnostic_snapshot(&mut self) -> TpResult<serde_json::Value> {
        if !self.options.profile
            || self.failed
            || self.exited
            || self.pending.is_some()
            || self.diagnostic_snapshots >= 2
        {
            return self.reject("runtime diagnostic snapshot is disabled or unavailable");
        }
        self.send(CommandV1::PerformanceSnapshot, vec![])?;
        let incoming = self.receive()?;
        let ResponseV1::PerformanceSnapshot { counters } = incoming.header else {
            return self.reject("runtime diagnostic snapshot response mismatch");
        };
        if !incoming.payload.is_empty() {
            return self.reject("runtime diagnostic snapshot has an unexpected payload");
        }
        let values = serde_json::to_value(&counters).map_err(|error| error.to_string())?;
        if let Some(previous) = &self.last_diagnostic {
            let before = serde_json::to_value(previous).map_err(|error| error.to_string())?;
            if before.as_object().is_none_or(|object| {
                object.iter().any(|(key, value)| {
                    values.get(key).and_then(serde_json::Value::as_u64) < value.as_u64()
                })
            }) {
                return self.reject("runtime diagnostic counters regressed");
            }
        }
        let ordinal = self.diagnostic_snapshots;
        self.diagnostic_snapshots += 1;
        self.last_diagnostic = Some(Box::new(counters));
        Ok(serde_json::json!({
            "schema": "FerricRuntimeDiagnosticSnapshotV1",
            "authority": "none",
            "performance_qualified": false,
            "scope": "cumulative overlapping worker host-wall counters, not GPU timestamps",
            "process_id": self.child.id(),
            "device_unique_id": self.diagnostic_identity.0,
            "rank": self.diagnostic_identity.1,
            "ordinal": ordinal,
            "counters": values,
        }))
    }
    fn allocate(&mut self, byte_len: usize) -> TpResult<u64> {
        if byte_len == 0 {
            return self.reject("zero allocation");
        }
        self.send(
            CommandV1::Allocate {
                bytes: byte_len as u64,
            },
            vec![],
        )?;
        let response = self.receive()?;
        match response.header {
            ResponseV1::Allocated { buffer, bytes }
                if buffer != 0
                    && bytes == byte_len as u64
                    && !self.buffers.contains_key(&buffer) =>
            {
                self.buffers.insert(buffer, byte_len);
                Ok(buffer)
            }
            _ => self.reject("allocation response mismatch"),
        }
    }

    fn write(&mut self, buffer: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        self.extent(buffer, offset, bytes.len())?;
        for (index, chunk) in bytes
            .chunks(wire::MAX_TRANSFER_BYTES_V1 as usize)
            .enumerate()
        {
            let position = offset + index * wire::MAX_TRANSFER_BYTES_V1 as usize;
            self.send(
                CommandV1::Write {
                    buffer,
                    offset: position as u64,
                    payload_bytes: u32::try_from(chunk.len()).map_err(|_| "write chunk length")?,
                },
                chunk.to_vec(),
            )?;
            if !matches!(self.receive()?.header, ResponseV1::Written) {
                return self.reject("write response mismatch");
            }
        }
        Ok(())
    }

    fn read(&mut self, buffer: u64, offset: usize, bytes: &mut [u8]) -> TpResult<()> {
        self.extent(buffer, offset, bytes.len())?;
        for (index, chunk) in bytes
            .chunks_mut(wire::MAX_TRANSFER_BYTES_V1 as usize)
            .enumerate()
        {
            let position = offset + index * wire::MAX_TRANSFER_BYTES_V1 as usize;
            self.send(
                CommandV1::Read {
                    buffer,
                    offset: position as u64,
                    bytes: u32::try_from(chunk.len()).map_err(|_| "read chunk length")?,
                },
                vec![],
            )?;
            let response = self.receive()?;
            if !matches!(response.header, ResponseV1::Read { payload_bytes } if payload_bytes as usize == chunk.len())
                || response.payload.len() != chunk.len()
            {
                return self.reject("read response mismatch");
            }
            chunk.copy_from_slice(&response.payload);
        }
        Ok(())
    }

    fn submit(&mut self, dispatch: &EngineeringTpDispatchV1) -> TpResult<()> {
        let loaded = self.kernels.get(dispatch.kernel).ok_or("unloaded kernel")?;
        let (header, bytes) = pack_dispatch(loaded, dispatch, &self.buffers)?;
        self.send(header, bytes)
    }

    fn wait(&mut self) -> TpResult<()> {
        if self.pending != Some(PendingRequest::Dispatch) {
            return self.reject("no pending GPU dispatch");
        }
        if !matches!(self.receive()?.header, ResponseV1::Dispatched { .. }) {
            return self.reject("dispatch completion response mismatch");
        }
        self.queue_packets = self
            .queue_packets
            .checked_add(1)
            .ok_or("queue packet overflow")?;
        Ok(())
    }

    fn supports_sequences(&self) -> bool {
        self.options.sequences
    }

    fn submit_sequence(&mut self, dispatches: &[EngineeringTpDispatchV1]) -> TpResult<()> {
        if !self.options.sequences || !(1..=16).contains(&dispatches.len()) {
            return self.reject("dispatch sequence policy or bound mismatch");
        }
        let mut entries = Vec::with_capacity(dispatches.len());
        let mut payload = Vec::new();
        for dispatch in dispatches {
            let loaded = self
                .kernels
                .get(dispatch.kernel)
                .ok_or("unloaded sequence kernel")?;
            let header = pack_dispatch_into(loaded, dispatch, &self.buffers, &mut payload)?;
            let CommandV1::Dispatch {
                kernel,
                payload_bytes,
                workgroup,
                grid,
                pointers,
                timeout_ms,
            } = header
            else {
                return self.reject("sequence dispatch packing drifted");
            };
            let timeout_ms = timeout_ms.min(
                600_000
                    / u32::try_from(dispatches.len()).map_err(|_| "sequence length overflow")?,
            );
            entries.push(SequenceDispatchV1 {
                kernel,
                payload_bytes,
                workgroup,
                grid,
                pointers,
                timeout_ms,
            });
        }
        self.send(
            CommandV1::DispatchSequence {
                dispatches: entries,
            },
            payload,
        )
    }

    fn wait_sequence(&mut self, count: usize) -> TpResult<()> {
        if self.pending != Some(PendingRequest::Sequence(count)) {
            return self.reject("pending sequence count mismatch");
        }
        match self.receive()?.header {
            ResponseV1::DispatchSequenceCompleted { elapsed_ns } if elapsed_ns.len() == count => {
                self.queue_packets = self
                    .queue_packets
                    .checked_add(count as u64)
                    .ok_or("queue packet overflow")?;
                Ok(())
            }
            response => self.reject(format!(
                "dispatch sequence failed or completion drifted: {response:?}"
            )),
        }
    }

    fn supports_ordered_batches(&self) -> bool {
        self.options.ordered_batches
    }

    fn supports_ordered_batches64(&self) -> bool {
        self.options.ordered64 && self.options.validate_ordered64().is_ok()
    }

    #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
    fn supports_token_program(&self) -> bool {
        self.token_program_backend().is_some() && self.supports_ordered_batches64()
    }

    #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
    fn submit_token_program(&mut self, dispatches: &[EngineeringTpDispatchV1]) -> TpResult<()> {
        self.submit_fixed_token(dispatches)
    }

    #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
    fn supports_prefill_program(&self) -> bool {
        self.prefill_program_enabled() && self.supports_token_program()
    }

    #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
    fn supports_prefill32_program(&self) -> bool {
        self.prefill32_program_enabled() && self.supports_token_program()
    }

    #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
    fn submit_prefill_program(&mut self, dispatches: &[EngineeringTpDispatchV1]) -> TpResult<()> {
        self.submit_fixed_prefill(dispatches)
    }

    #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
    fn wait_prefill_program(&mut self, count: usize) -> TpResult<()> {
        if count
            != if self.prefill32_program_enabled() {
                649
            } else {
                613
            }
        {
            return self.reject("prefill completion shape changed");
        }
        self.wait_fixed_token(count)
    }

    #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
    fn wait_token_program(&mut self, count: usize) -> TpResult<()> {
        self.wait_fixed_token(count)
    }

    fn submit_ordered_batch(&mut self, dispatches: &[EngineeringTpDispatchV1]) -> TpResult<()> {
        if !self.options.ordered_batches
            || self.options.sequences
            || self.options.validate_ordered64().is_err()
            || !(1..=self.ordered_limit()).contains(&dispatches.len())
            || self
                .queue_packets
                .checked_add(dispatches.len() as u64)
                .is_none_or(|end| end > wire::MAX_UNRETIRED_RING_PACKETS_V1)
        {
            return self.reject("ordered batch policy, count or packet budget mismatch");
        }
        // Every existing metadata, ownership and scalar pack check precedes publication.
        let prepared = (|| -> TpResult<_> {
            let _packing = self
                .timing
                .as_ref()
                .and_then(|timing| timing.timing.ordered_packing_span(timing.rank));
            let mut entries = Vec::with_capacity(dispatches.len());
            let mut payload = Vec::new();
            for dispatch in dispatches {
                let loaded = self
                    .kernels
                    .get(dispatch.kernel)
                    .ok_or("unloaded ordered batch kernel")?;
                let header = pack_dispatch_into(loaded, dispatch, &self.buffers, &mut payload)?;
                let CommandV1::Dispatch {
                    kernel,
                    payload_bytes,
                    workgroup,
                    grid,
                    pointers,
                    ..
                } = header
                else {
                    return Err("ordered batch dispatch packing drifted".into());
                };
                entries.push(OrderedBatchDispatchV1 {
                    kernel,
                    payload_bytes,
                    workgroup,
                    grid,
                    pointers,
                });
            }
            let header = CommandV1::DispatchOrderedBatch {
                dispatches: entries,
                timeout_ms: DISPATCH_TIMEOUT_MS,
            };
            #[cfg(feature = "c1-ordered64")]
            let header = if self.options.ordered64 {
                let CommandV1::DispatchOrderedBatch {
                    dispatches,
                    timeout_ms,
                } = header
                else {
                    return Err("ordered dispatch construction drifted".into());
                };
                CommandV1::DispatchOrderedBatch64 {
                    dispatches,
                    timeout_ms,
                }
            } else {
                header
            };
            if header.payload_bytes().map_err(|error| error.to_string())? != payload.len() {
                return Err("ordered batch cumulative payload mismatch".into());
            }
            Ok((header, payload))
        })();
        let (header, payload) = match prepared {
            Ok(value) => value,
            Err(error) => return self.reject(error),
        };
        self.send(header, payload)
    }

    fn wait_ordered_batch(&mut self, count: usize) -> TpResult<()> {
        if !(1..=self.ordered_limit()).contains(&count)
            || self.pending != Some(self.ordered_pending(count))
        {
            return self.reject("pending ordered batch count mismatch");
        }
        let next = match self.queue_packets.checked_add(count as u64) {
            Some(value) if value <= wire::MAX_UNRETIRED_RING_PACKETS_V1 => value,
            _ => return self.reject("ordered batch completed packet count overflow"),
        };
        let response = self.receive()?;
        let elapsed_ns = match response.header {
            ResponseV1::DispatchOrderedBatchCompleted {
                completed_dispatches,
                elapsed_ns,
            } if !self.options.ordered64 && completed_dispatches as usize == count => {
                Some(elapsed_ns)
            }
            #[cfg(feature = "c1-ordered64")]
            ResponseV1::DispatchOrderedBatch64Completed {
                completed_dispatches,
                elapsed_ns,
            } if self.options.ordered64 && completed_dispatches as usize == count => {
                Some(elapsed_ns)
            }
            _ => None,
        };
        let Some(elapsed_ns) = elapsed_ns.filter(|_| response.payload.is_empty()) else {
            return self.reject("ordered batch failed or aggregate completion drifted");
        };
        if self.options.ordered64
            && let Some(timing) = &self.timing
        {
            timing
                .timing
                .ordered_worker_elapsed(timing.rank, count as u64, elapsed_ns);
        }
        self.queue_packets = next;
        Ok(())
    }

    fn supports_queue_rollover(&self) -> bool {
        self.options.rollover
    }

    fn prepare_packets(&mut self, count: u64) -> TpResult<()> {
        let limit = wire::MAX_UNRETIRED_RING_PACKETS_V1;
        if count == 0 || count > limit {
            return self.reject("invalid next packet reservation");
        }
        if self
            .queue_packets
            .checked_add(count)
            .is_some_and(|end| end <= limit)
        {
            return Ok(());
        }
        if !self.options.rollover {
            return self.reject("queue requires explicitly enabled rollover");
        }
        #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
        self.release_registered_token()?;
        let next = self
            .queue_epoch
            .checked_add(1)
            .ok_or("queue epoch overflow")?;
        self.send(
            CommandV1::RolloverQueue {
                expected_epoch: self.queue_epoch,
                expected_completed_packets: self.queue_packets,
            },
            vec![],
        )?;
        if !matches!(self.receive()?.header, ResponseV1::QueueRolledOver { retired_packets, queue_epoch }
            if retired_packets == self.queue_packets && queue_epoch == next)
        {
            return self.reject("queue rollover receipt mismatch");
        }
        self.queue_epoch = next;
        self.queue_packets = 0;
        Ok(())
    }

    fn close(&mut self) -> TpResult<()> {
        if self.exited {
            return Ok(());
        }
        if self.failed {
            return self.terminate();
        }
        #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
        if matches!(self.pending, Some(PendingRequest::TokenExecute { .. })) {
            self.wait_fixed_token(self.registered_program_packets())?;
        }
        if self.pending == Some(PendingRequest::Dispatch) {
            self.wait()?;
        } else if let Some(PendingRequest::Sequence(count)) = self.pending {
            self.wait_sequence(count)?;
        } else if let Some(PendingRequest::OrderedBatch(count)) = self.pending {
            self.wait_ordered_batch(count)?;
        }
        #[cfg(feature = "c1-ordered64")]
        if let Some(PendingRequest::OrderedBatch64(count)) = self.pending {
            self.wait_ordered_batch(count)?;
        }
        #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
        self.release_registered_token()?;
        #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
        if self.token_program_backend().is_some() && self.options.profile {
            self.emit_token_program_counters()?;
        }
        self.send(CommandV1::Close, vec![])?;
        if !matches!(self.receive()?.header, ResponseV1::Closed) {
            return self.reject("close response mismatch");
        }
        let result = self.await_exit(true);
        if result.is_err() {
            let _ = self.terminate();
        }
        result
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if !self.exited
            && let Err(error) = self.terminate()
        {
            eprintln!("GPU worker cleanup: {error}");
        }
    }
}

#[cfg(test)]
#[path = "tp_worker_diagnostics_tests.rs"]
mod diagnostic_tests;

fn read_response(input: &mut impl Read) -> TpResult<Incoming> {
    let header = wire::read_header_v1(input)
        .map_err(|error| format!("worker pipe header: {error}"))?
        .ok_or("worker closed its response pipe")?;
    let length = match &header {
        ResponseV1::Read { payload_bytes } if *payload_bytes <= wire::MAX_TRANSFER_BYTES_V1 => {
            *payload_bytes as usize
        }
        ResponseV1::Read { .. } => return Err("oversized worker payload".into()),
        _ => 0,
    };
    let mut payload = vec![0; length];
    input
        .read_exact(&mut payload)
        .map_err(|error| format!("worker pipe payload: {error}"))?;
    Ok(Incoming { header, payload })
}

include!("tp_worker_dispatch.rs");

#[cfg(test)]
mod tests {
    use super::*;

    fn active_poll_options() -> RuntimeOptions {
        RuntimeOptions {
            ordered_batches: true,
            ordered64: true,
            profile: true,
            ordered64_runtime_counters: true,
            ..RuntimeOptions::default()
        }
    }

    #[test]
    fn active_poll_worker_argv_is_exact_and_default_entry_is_unchanged() {
        let executable = Path::new("worker-not-executed");
        let ordinary = WorkerEntry::Ordinary
            .command(
                executable,
                71,
                RuntimeOptions::default(),
                &HostTiming::default(),
                0,
            )
            .unwrap();
        let expected = [
            "--device-unique-id",
            "71",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(ordinary.get_program(), executable.as_os_str());
        assert_eq!(ordinary.get_args().collect::<Vec<_>>(), expected);
        let timing = HostTiming::ordered64_diagnostic();
        let active =
            WorkerEntry::ActivePoll10ms.command(executable, 71, active_poll_options(), &timing, 0);
        if !cfg!(all(
            feature = "c1-ordered64",
            not(feature = "model-timestamps")
        )) {
            assert!(active.is_err());
            return;
        }
        let active = active.unwrap();
        assert_eq!(active.get_program(), ordinary.get_program());
        assert_eq!(
            active.get_args().collect::<Vec<_>>(),
            [
                "--device-unique-id",
                "71",
                "--allow-unauthenticated-machine-code",
                "--diagnostic-active-poll-10ms",
            ]
        );
        let control = WorkerEntry::Ordinary
            .command(executable, 71, active_poll_options(), &timing, 0)
            .unwrap();
        assert_eq!(control.get_args().collect::<Vec<_>>(), expected);
    }

    #[test]
    fn active_poll_rejects_every_incompatible_mode_before_process_creation() {
        let executable = Path::new("worker-not-executed");
        let timing = HostTiming::ordered64_diagnostic();
        for mutation in 0..8 {
            let mut options = active_poll_options();
            let mut rank = 0;
            match mutation {
                0 => options.ordered64 = false,
                1 => options.profile = false,
                2 => options.ordered64_runtime_counters = false,
                3 => options.ordered_batches = false,
                4 => options.sequences = true,
                5 => options.shared_full_currentness = true,
                6 => options.ordered64_packet_ticks = true,
                _ => rank = 1,
            }
            assert!(
                WorkerEntry::ActivePoll10ms
                    .command(executable, 71, options, &timing, rank)
                    .is_err()
            );
        }
    }

    #[test]
    fn active_poll_requires_the_dedicated_timing_recorder_before_process_creation() {
        let executable = Path::new("worker-not-executed");
        let available = cfg!(all(
            feature = "c1-ordered64",
            not(feature = "model-timestamps")
        ));
        for timing in [HostTiming::default(), HostTiming::enabled()] {
            let command = WorkerEntry::ActivePoll10ms.command(
                executable,
                71,
                active_poll_options(),
                &timing,
                0,
            );
            assert!(command.is_err());
            if available {
                assert_eq!(
                    command.unwrap_err(),
                    "active polling requires the bounded ordered64 host recorder"
                );
                assert!(
                    WorkerEntry::Ordinary
                        .command(executable, 71, active_poll_options(), &timing, 0)
                        .is_ok()
                );
            }
        }
        let timing = HostTiming::ordered64_diagnostic();
        let command =
            WorkerEntry::ActivePoll10ms.command(executable, 71, active_poll_options(), &timing, 0);
        assert_eq!(command.is_ok(), available);
    }

    #[test]
    fn active_poll_and_token_entries_cannot_share_their_profile_contracts() {
        let executable = Path::new("worker-not-executed");
        let timing = HostTiming::ordered64_diagnostic();
        assert!(
            WorkerEntry::TokenProgram
                .command(executable, 71, active_poll_options(), &timing, 0)
                .is_err()
        );
        let token = RuntimeOptions {
            ordered_batches: true,
            ordered64: true,
            ..RuntimeOptions::default()
        };
        assert!(
            WorkerEntry::ActivePoll10ms
                .command(executable, 71, token, &timing, 0)
                .is_err()
        );
        let command =
            WorkerEntry::TokenProgram.command(executable, 71, token, &HostTiming::default(), 0);
        if cfg!(all(
            feature = "c1-token-program",
            not(feature = "model-timestamps")
        )) {
            assert_eq!(
                command.unwrap().get_args().collect::<Vec<_>>(),
                [
                    "--device-unique-id",
                    "71",
                    "--allow-unauthenticated-machine-code",
                    "--diagnostic-token-program-v1",
                ]
            );
        } else {
            assert!(command.is_err());
        }
    }

    #[test]
    fn native_token_entry_is_explicit_and_preserves_the_unprofiled_token_contract() {
        let executable = Path::new("worker-not-executed");
        let options = RuntimeOptions {
            ordered_batches: true,
            ordered64: true,
            ..RuntimeOptions::default()
        };
        let available = cfg!(all(
            feature = "c1-token-program",
            not(feature = "model-timestamps")
        ));
        let command = WorkerEntry::NativeTokenProgram.command(
            executable,
            71,
            options,
            &HostTiming::default(),
            0,
        );
        assert_eq!(command.is_ok(), available);
        if let Ok(command) = command {
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                [
                    "--device-unique-id",
                    "71",
                    "--allow-unauthenticated-machine-code",
                    "--diagnostic-token-program-native-v1",
                ]
            );
        }
        for (options, rank) in [
            (options, 1),
            (
                RuntimeOptions {
                    ordered64: false,
                    ..options
                },
                0,
            ),
            (
                RuntimeOptions {
                    profile: true,
                    ..options
                },
                0,
            ),
            (active_poll_options(), 0),
        ] {
            assert!(
                WorkerEntry::NativeTokenProgram
                    .command(executable, 71, options, &HostTiming::default(), rank,)
                    .is_err()
            );
        }
        let baseline = TokenProgramBackend::Ordered64GroupsV1;
        let candidate = TokenProgramBackend::NativeWholeProgramV1;
        assert_ne!(baseline.identity(), candidate.identity());
        assert_ne!(baseline.argument(), candidate.argument());
        assert_ne!(baseline.profile(), candidate.profile());
    }

    #[test]
    fn prefill_entry_keeps_native_backend_and_refuses_rank_profile_or_feature_drift() {
        for counters in [false, true] {
            let options = RuntimeOptions {
                ordered_batches: true,
                ordered64: true,
                profile: counters,
                ordered64_runtime_counters: counters,
                ..RuntimeOptions::default()
            };
            let entry = WorkerEntry::NativePrefillProgram { counters };
            assert_eq!(
                entry.token_backend(),
                Some(TokenProgramBackend::NativeWholeProgramV1)
            );
            let command = entry.command(
                Path::new("not-executed"),
                71,
                options,
                &HostTiming::default(),
                0,
            );
            assert_eq!(
                command.is_ok(),
                cfg!(all(
                    feature = "c1-token-program",
                    not(feature = "model-timestamps")
                ))
            );
            if let Ok(command) = command {
                assert_eq!(
                    command.get_args().collect::<Vec<_>>(),
                    [
                        "--device-unique-id",
                        "71",
                        "--allow-unauthenticated-machine-code",
                        "--diagnostic-token-program-native-v1"
                    ]
                );
            }
            for mutation in 0..4 {
                let mut invalid = options;
                let rank = if mutation == 0 { 1 } else { 0 };
                match mutation {
                    1 => invalid.ordered64 = false,
                    2 => invalid.profile = !counters,
                    3 => invalid.ordered64_packet_ticks = true,
                    _ => {}
                }
                assert!(
                    entry
                        .command(
                            Path::new("not-executed"),
                            71,
                            invalid,
                            &HostTiming::default(),
                            rank
                        )
                        .is_err()
                );
            }
        }
    }

    #[test]
    fn width_entries_share_only_the_explicit_slots512_worker() {
        for rows in [16, 32] {
            for counters in [false, true] {
                let entry = WorkerEntry::NativePrefillWidthProgram { rows, counters };
                let options = RuntimeOptions {
                    ordered_batches: true,
                    ordered64: true,
                    profile: counters,
                    ordered64_runtime_counters: counters,
                    ..RuntimeOptions::default()
                };
                assert_eq!(
                    entry.token_backend(),
                    Some(TokenProgramBackend::NativeWholeProgramSlots512V1)
                );
                let command = entry.command(
                    Path::new("not-executed"),
                    71,
                    options,
                    &HostTiming::default(),
                    0,
                );
                assert_eq!(
                    command.is_ok(),
                    cfg!(all(
                        feature = "c1-token-program",
                        not(feature = "model-timestamps")
                    ))
                );
                if let Ok(command) = command {
                    assert_eq!(
                        command.get_args().collect::<Vec<_>>(),
                        [
                            "--device-unique-id",
                            "71",
                            "--allow-unauthenticated-machine-code",
                            "--diagnostic-token-program-native-slots512-v1"
                        ]
                    );
                }
                assert!(
                    entry
                        .command(
                            Path::new("not-executed"),
                            71,
                            options,
                            &HostTiming::default(),
                            1
                        )
                        .is_err()
                );
                assert!(
                    WorkerEntry::NativePrefillWidthProgram { rows: 64, counters }
                        .command(
                            Path::new("not-executed"),
                            71,
                            options,
                            &HostTiming::default(),
                            0
                        )
                        .is_err()
                );
            }
        }
    }

    #[test]
    fn token_counter_entry_requires_explicit_profile_and_keeps_backend_flags_distinct() {
        let executable = Path::new("worker-not-executed");
        let available = cfg!(all(
            feature = "c1-token-program",
            not(feature = "model-timestamps")
        ));
        for backend in [
            TokenProgramBackend::Ordered64GroupsV1,
            TokenProgramBackend::NativeWholeProgramV1,
        ] {
            let command = WorkerEntry::TokenProgramCounters(backend).command(
                executable,
                71,
                active_poll_options(),
                &HostTiming::default(),
                0,
            );
            assert_eq!(command.is_ok(), available);
            if let Ok(command) = command {
                assert_eq!(
                    command.get_args().last(),
                    Some(std::ffi::OsStr::new(backend.argument()))
                );
            }
            assert!(
                WorkerEntry::TokenProgramCounters(backend)
                    .command(
                        executable,
                        71,
                        RuntimeOptions {
                            profile: false,
                            ..active_poll_options()
                        },
                        &HostTiming::default(),
                        0,
                    )
                    .is_err()
            );
            assert!(
                WorkerEntry::TokenProgramCounters(backend)
                    .command(
                        executable,
                        71,
                        active_poll_options(),
                        &HostTiming::ordered64_diagnostic(),
                        0,
                    )
                    .is_err()
            );
        }
    }

    #[test]
    fn kernarg_append_is_relative_zero_filled_and_preserves_prior_dispatches() {
        let mut payload = vec![0xA5; 19];
        for length in [0, 4, 17, wire::MAX_KERNARG_BYTES_V1] {
            let before = payload.clone();
            let written = append_kernarg_payload(&mut payload, length, |bytes| {
                assert_eq!(bytes.len(), length as usize);
                assert!(bytes.iter().all(|&byte| byte == 0));
                if let Some(last) = bytes.last_mut() {
                    *last = 0x3C;
                }
                Ok(bytes.len())
            })
            .unwrap();
            assert_eq!(written, length as usize);
            assert_eq!(&payload[..before.len()], before);
            assert_eq!(payload.len(), before.len() + length as usize);
            if length != 0 {
                assert_eq!(payload.last(), Some(&0x3C));
            }
        }
    }

    #[test]
    fn kernarg_append_rolls_back_errors_and_zeroes_reused_suffix() {
        let mut payload = vec![0xA5; 19];
        let before = payload.clone();
        let failed: TpResult<()> = append_kernarg_payload(&mut payload, 64, |bytes| {
            bytes.fill(0xFF);
            Err("injected late packing error".into())
        });
        assert_eq!(failed.unwrap_err(), "injected late packing error");
        assert_eq!(payload, before);
        append_kernarg_payload(&mut payload, 64, |bytes| {
            assert_eq!(bytes, &[0; 64]);
            Ok(())
        })
        .unwrap();
        assert_eq!(&payload[..before.len()], before);
    }

    #[test]
    fn kernarg_append_rejects_oversized_extent_before_mutation_or_callback() {
        let mut payload = vec![0xA5; 19];
        let before = payload.clone();
        let result: TpResult<()> =
            append_kernarg_payload(&mut payload, wire::MAX_KERNARG_BYTES_V1 + 1, |_| {
                panic!("oversized kernarg must not reach its packer")
            });
        assert_eq!(result.unwrap_err(), "kernarg bound exceeded");
        assert_eq!(payload, before);
    }

    #[test]
    fn kernarg_append_reuses_existing_aggregate_capacity() {
        let mut payload = Vec::with_capacity(16 * 64);
        let base = payload.as_ptr();
        for value in 0..16_u8 {
            append_kernarg_payload(&mut payload, 64, |bytes| {
                bytes.fill(value);
                Ok(())
            })
            .unwrap();
            assert_eq!(payload.as_ptr(), base);
        }
        for (index, bytes) in payload.chunks_exact(64).enumerate() {
            assert!(bytes.iter().all(|&byte| usize::from(byte) == index));
        }
    }

    const FAKE_WORKER: &str = r"
import json, struct, sys, time
mode = sys.argv[1]
def send(value, payload=b''):
    header = json.dumps(value).encode()
    sys.stdout.buffer.write(struct.pack('<I', len(header)) + header + payload)
    sys.stdout.buffer.flush()
send({'op':'ready','protocol':1,'target':'gfx950:xnack-',
      'device_unique_id':2 if mode == 'wrong' else 1,'authority':'none'})
if mode in ('wrong', 'stall'):
    time.sleep(60)
buffers = {}
while True:
    prefix = sys.stdin.buffer.read(4)
    if len(prefix) != 4: sys.exit(3)
    command = json.loads(sys.stdin.buffer.read(struct.unpack('<I',prefix)[0]))
    op = command['op']
    if op == 'allocate':
        identity = len(buffers) + 1
        buffers[identity] = bytearray(command['bytes'])
        send({'op':'allocated','buffer':identity,'bytes':command['bytes']})
    elif op == 'write':
        size = command['payload_bytes']
        value = sys.stdin.buffer.read(size)
        if len(value) != size: sys.exit(4)
        start = command['offset']
        buffers[command['buffer']][start:start+size] = value
        send({'op':'written'})
    elif op == 'read':
        start, size = command['offset'], command['bytes']
        send({'op':'read','payload_bytes':size}, buffers[command['buffer']][start:start+size])
    elif op == 'close':
        send({'op':'closed'})
        sys.exit(0)
    elif op == 'rollover_queue':
        send({'op':'queue_rolled_over', 'retired_packets':command['expected_completed_packets'],
              'queue_epoch':command['expected_epoch'] + (2 if mode == 'badrollover' else 1)})
    elif op == 'dispatch_sequence':
        entries = command['dispatches']
        sys.stdin.buffer.read(sum(entry['payload_bytes'] for entry in entries))
        send({'op':'dispatch_sequence_completed', 'elapsed_ns':[1] * (len(entries) - (1 if mode == 'shortsequence' else 0))})
    elif op in ('dispatch_ordered_batch', 'dispatch_ordered_batch64'):
        entries = command['dispatches']
        limit = 64 if op == 'dispatch_ordered_batch64' else 16
        if command['timeout_ms'] != 60000 or not 1 <= len(entries) <= limit: sys.exit(6)
        if any('timeout_ms' in entry for entry in entries): sys.exit(7)
        size = sum(entry['payload_bytes'] for entry in entries)
        if len(sys.stdin.buffer.read(size)) != size: sys.exit(4)
        if mode == 'orderederror': send({'op':'error','message':'injected','fatal':True})
        elif mode == 'orderedsequence': send({'op':'dispatch_sequence_completed','elapsed_ns':[1]*len(entries)})
        elif mode == 'orderedpayload': send({'op':'read','payload_bytes':1}, b'x')
        elif mode == 'orderedstall': time.sleep(60)
        else:
            reply = op + '_completed'
            if mode == 'orderedwrongkind':
                reply = 'dispatch_ordered_batch_completed' if limit == 64 else 'dispatch_ordered_batch64_completed'
            send({'op':reply,
                  'completed_dispatches':len(entries) - (1 if mode == 'orderedshort' else 0),
                  'elapsed_ns':7})
    else:
        sys.exit(5)
";

    fn fake(mode: &str) -> Child {
        Command::new("python3")
            .arg("-u")
            .arg("-c")
            .arg(FAKE_WORKER)
            .arg(mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap()
    }

    #[test]
    #[ignore = "requires FERRIC_V7_TEST_ARTIFACT pointing to a separately emitted closed image; no GPU"]
    #[cfg(feature = "tp-batch-engineering")]
    fn actual_v7_image_fake_worker_loads_rejects_duplicates_and_reaps_partial_failures() {
        let path = std::env::var_os("FERRIC_V7_TEST_ARTIFACT").expect("explicit v7 artifact path");
        let artifact = EngineeringTpArtifactV1::open_fp32_head(
            Path::new(&path),
            &ferric_qwen3_tp_fp32_head_kernels_device_v7::compiler_expectation_roster_v7(),
        )
        .unwrap();
        actual_image_fake_worker(&artifact, false);
    }

    #[test]
    #[ignore = "requires FERRIC_V9_TEST_ARTIFACT pointing to the emitted v9 image; no GPU"]
    #[cfg(feature = "tp-batch-engineering")]
    fn actual_v9_image_binding_is_fresh_exact_and_precedes_allocation() {
        use ferric_m1_engineering_execution_v1::tp_paged::{
            EngineeringTpPagedLimitsV1, EngineeringTpPagedPoolV1, EngineeringTpPoolScopeV1,
        };
        let path = std::env::var_os("FERRIC_V9_TEST_ARTIFACT").expect("explicit v9 image");
        let expected = ferric_qwen3_tp_large_kv_kernels_device_v9::compiler_expectation_roster_v9();
        let artifact =
            EngineeringTpArtifactV1::open_large_kv32(Path::new(&path), &expected).unwrap();
        assert!(EngineeringTpArtifactV1::open_fp32_head32(Path::new(&path), &expected).is_err());
        let scope = EngineeringTpPoolScopeV1 {
            model: [1; 32],
            session: [2; 32],
        };
        for pages in [1, 512, 513, 16384] {
            let limits = EngineeringTpPagedLimitsV1::new_large_kv32(8192, 32, pages, 100).unwrap();
            assert!(EngineeringTpPagedPoolV1::new_wide32(scope, limits).is_err());
            let pool = EngineeringTpPagedPoolV1::new_large_kv32(scope, limits, &artifact).unwrap();
            assert_eq!(pool.limits(), limits);
        }
        actual_image_fake_worker(&artifact, true);
    }

    #[cfg(feature = "tp-batch-engineering")]
    fn actual_image_fake_worker(artifact: &EngineeringTpArtifactV1, large: bool) {
        let hash: [u8; 32] = Sha256::digest(artifact.bytes()).into();
        let metadata = artifact.inspection().hsaco().kernels().iter().map(|kernel| {
            let arguments = kernel.explicit_arguments().iter().map(|arg| serde_json::json!({
                "offset":arg.offset(), "bytes":arg.size(),
                "global_buffer":arg.value_kind() == ExplicitValueKind::GlobalBuffer,
                "pointee_alignment":arg.pointee_alignment(), "access":arg.access().map(wire_access)
            })).collect::<Vec<_>>();
            (kernel.name().to_owned(), serde_json::json!({
                "symbol":kernel.name(), "object_sha256":hash,
                "kernarg_bytes":kernel.kernarg_segment_size(), "kernarg_alignment":kernel.kernarg_segment_alignment(),
                "group_segment_bytes":kernel.group_segment_fixed_size(), "private_segment_bytes":kernel.private_segment_fixed_size(),
                "wavefront_size":kernel.wavefront_size(), "implicit_argument_offset":kernel.implicit_argument_offset(),
                "implicit_argument_bytes":kernel.implicit_argument_size(), "explicit_arguments":arguments
            }))
        }).collect::<serde_json::Map<_, _>>();
        let script = r"
import copy, json, struct, sys
metadata, mode = json.loads(sys.argv[1]), sys.argv[2]
def send(value):
    data = json.dumps(value, separators=(',', ':')).encode()
    sys.stdout.buffer.write(struct.pack('<I', len(data)) + data)
    sys.stdout.buffer.flush()
send({'op':'ready','protocol':1,'target':'gfx950:xnack-','device_unique_id':1,'authority':'none'})
count = 0
while True:
    prefix = sys.stdin.buffer.read(4)
    if len(prefix) != 4: sys.exit(3)
    command = json.loads(sys.stdin.buffer.read(struct.unpack('<I', prefix)[0]))
    payload = sys.stdin.buffer.read(command.get('payload_bytes', 0))
    if len(payload) != command.get('payload_bytes', 0): sys.exit(4)
    if command['op'] == 'load_kernel':
        count += 1
        item = copy.deepcopy(metadata[command['symbol']])
        if mode == 'metadata' and count == 2: item['wavefront_size'] = 32
        send({'op':'loaded_kernel','kernel':1 if mode == 'duplicate_id' and count == 2 else count,'metadata':item})
    elif command['op'] == 'close':
        send({'op':'closed'})
        sys.exit(0)
    else: sys.exit(5)
";
        let modes: &[&str] = if large {
            &[
                "normal",
                "duplicate_id",
                "metadata",
                "wrong_image",
                "missing_root",
                "duplicate_root",
                "allocated",
                "pending",
                "used_queue",
                "old_epoch",
                "failed",
            ]
        } else {
            &["normal", "duplicate_id", "metadata"]
        };
        for &mode in modes {
            let child = Command::new("python3")
                .args([
                    "-u",
                    "-c",
                    script,
                    &serde_json::to_string(&metadata).unwrap(),
                    mode,
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap();
            let mut worker = Worker::connect(child, 1, Duration::from_secs(5)).unwrap();
            let result = worker.load_additional_artifact(artifact);
            if matches!(mode, "duplicate_id" | "metadata") {
                assert!(result.is_err() && worker.failed);
                assert_eq!(worker.kernels.len(), 1);
                assert!(worker.load_additional_artifact(artifact).is_err());
            } else {
                result.unwrap();
                assert_eq!(worker.kernels.len(), if large { 2 } else { 3 });
                assert!(!worker.failed);
                if large {
                    use ferric_m1_engineering_execution_v1::tp_artifact::ENGINEERING_TP_LARGE_KV_EXPORTS_V9;
                    let mut names = ENGINEERING_TP_LARGE_KV_EXPORTS_V9;
                    let mut expected = hash;
                    match mode {
                        "wrong_image" => expected[0] ^= 1,
                        "missing_root" => names[0] = "missing",
                        "duplicate_root" => names[1] = names[0],
                        "allocated" => {
                            worker.buffers.insert(999, 4);
                        }
                        "pending" => worker.pending = Some(PendingRequest::Other),
                        "used_queue" => worker.queue_packets = 1,
                        "old_epoch" => worker.queue_epoch = 1,
                        "failed" => worker.failed = true,
                        _ => {}
                    }
                    assert_eq!(
                        worker.require_loaded_image(expected, &names).is_ok(),
                        mode == "normal"
                    );
                    if mode != "normal" {
                        assert!(worker.failed);
                        worker.pending = None;
                        worker.buffers.clear();
                        worker.close().unwrap();
                        assert!(worker.exited && worker.child.try_wait().unwrap().is_some());
                        continue;
                    }
                }
                assert!(worker.load_additional_artifact(artifact).is_err());
                assert!(!worker.failed);
            }
            worker.close().unwrap();
            assert!(worker.exited && worker.io_threads.is_empty());
            assert!(worker.child.try_wait().unwrap().is_some());
        }
    }

    #[test]
    fn additional_image_is_fresh_only_and_partial_failure_poisoned_and_reaped() {
        let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
        assert!(
            worker
                .with_additional_image(&[], |_| panic!("empty roster loaded"))
                .is_err()
        );
        assert!(
            worker
                .with_additional_image(&["new", "new"], |_| panic!("duplicate loaded"))
                .is_err()
        );
        worker.queue_packets = 1;
        assert!(
            worker
                .with_additional_image(&["new"], |_| panic!("post-dispatch load"))
                .is_err()
        );
        worker.queue_packets = 0;
        worker.queue_epoch = 1;
        assert!(
            worker
                .with_additional_image(&["new"], |_| panic!("post-rollover load"))
                .is_err()
        );
        worker.queue_epoch = 0;
        worker.pending = Some(PendingRequest::Other);
        assert!(
            worker
                .with_additional_image(&["new"], |_| panic!("pending load"))
                .is_err()
        );
        worker.pending = None;
        worker.with_additional_image(&["new"], |_| Ok(())).unwrap();
        let result = worker.with_additional_image(&["another"], |worker| {
            // An acknowledged IPC operation precedes the injected later load failure.
            worker.allocate(4)?;
            Err("injected second-symbol descriptor failure".into())
        });
        assert!(result.is_err() && worker.failed);
        assert!(
            worker
                .with_additional_image(&["later"], |_| panic!("poisoned load"))
                .is_err()
        );
        worker.close().unwrap();
        assert!(worker.exited && worker.io_threads.is_empty());
        assert!(worker.child.try_wait().unwrap().is_some());
        let mut allocated = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
        allocated.allocate(1).unwrap();
        assert!(
            allocated
                .with_additional_image(&["new"], |_| panic!("allocated load"))
                .is_err()
        );
        allocated.close().unwrap();
        assert!(
            allocated
                .with_additional_image(&["new"], |_| panic!("closed load"))
                .is_err()
        );
    }

    #[test]
    fn owned_child_chunked_roundtrip_closes_threads_and_reaps_process() {
        let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
        let size = wire::MAX_TRANSFER_BYTES_V1 as usize + 17;
        let buffer = worker.allocate(size + 1).unwrap();
        let source = vec![0xa5; size];
        worker.write(buffer, 1, &source).unwrap();
        let mut actual = vec![0; size];
        worker.read(buffer, 1, &mut actual).unwrap();
        assert_eq!(actual, source);
        let mut first = [1];
        worker.read(buffer, 0, &mut first).unwrap();
        assert_eq!(first, [0]);
        worker.close().unwrap();
        assert!(worker.exited);
        assert!(worker.writer.is_none());
        assert!(worker.io_threads.is_empty());
        assert!(worker.child.try_wait().unwrap().unwrap().success());
    }

    #[test]
    fn host_tickets_preserve_roundtrip_payloads_and_explicit_failure() {
        let timing = HostTiming::enabled();
        let mut worker = Worker::connect_with_timing(
            fake("normal"),
            1,
            Duration::from_secs(5),
            timing.clone(),
            1,
        )
        .unwrap();
        let buffer = worker.allocate(4).unwrap();
        worker.write(buffer, 0, &[1, 2, 3, 4]).unwrap();
        let mut bytes = [0; 4];
        worker.read(buffer, 0, &mut bytes).unwrap();
        assert_eq!(bytes, [1, 2, 3, 4]);
        worker.close().unwrap();
        let snapshot = timing.snapshot();
        assert_eq!(snapshot["incomplete"], false);
        let rows = snapshot["records"].as_array().unwrap();
        let requests = rows
            .iter()
            .filter(|row| row["category"] == "ipc_roundtrip")
            .collect::<Vec<_>>();
        assert_eq!(requests.len(), 4);
        assert!(
            requests
                .iter()
                .all(|row| row["rank"] == 1 && row["count"] == 1 && row["failed"] == 0)
        );
        assert_eq!(
            requests.iter().find(|row| row["label"] == "write").unwrap()["request_payload_bytes"],
            4
        );
        assert_eq!(
            requests.iter().find(|row| row["label"] == "read").unwrap()["response_payload_bytes"],
            4
        );

        let timing = HostTiming::enabled();
        let mut worker = Worker::connect_with_timing(
            fake("stall"),
            1,
            Duration::from_millis(100),
            timing.clone(),
            0,
        )
        .unwrap();
        assert!(worker.allocate(4).is_err());
        assert!(worker.exited);
        let snapshot = timing.snapshot();
        assert!(
            snapshot["records"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["label"] == "allocate" && row["failed"] == 1)
        );
    }

    #[test]
    fn wrong_ready_identity_is_rejected_and_child_is_reaped() {
        let child = fake("wrong");
        let pid = child.id();
        assert!(Worker::connect(child, 1, Duration::from_secs(5)).is_err());
        assert!(!Path::new(&format!("/proc/{pid}")).exists());
    }

    #[test]
    fn missing_response_has_a_deadline_and_kills_owned_child() {
        let mut worker = Worker::connect(fake("stall"), 1, Duration::from_secs(5)).unwrap();
        worker.timeout = Duration::from_millis(100);
        let start = Instant::now();
        assert!(worker.allocate(8).is_err());
        assert!(start.elapsed() < Duration::from_secs(10));
        assert!(worker.failed && worker.exited);
        assert!(worker.io_threads.is_empty());
    }

    #[test]
    fn blocked_pipe_write_has_a_deadline_and_kills_owned_child() {
        let mut worker = Worker::connect(fake("stall"), 1, Duration::from_secs(5)).unwrap();
        worker.timeout = Duration::from_millis(100);
        worker
            .buffers
            .insert(1, wire::MAX_TRANSFER_BYTES_V1 as usize);
        let start = Instant::now();
        assert!(
            worker
                .write(1, 0, &vec![7; wire::MAX_TRANSFER_BYTES_V1 as usize])
                .is_err()
        );
        assert!(start.elapsed() < Duration::from_secs(10));
        assert!(worker.failed && worker.exited);
        assert!(worker.io_threads.is_empty());
    }

    #[test]
    fn bounded_reader_requires_exact_payload_and_rejects_oversized_frames() {
        let mut bytes = Vec::new();
        wire::write_header_v1(&mut bytes, &ResponseV1::Read { payload_bytes: 3 }).unwrap();
        bytes.extend_from_slice(&[1, 2, 3]);
        let response = read_response(&mut bytes.as_slice()).unwrap();
        assert_eq!(response.payload, [1, 2, 3]);
        bytes.pop();
        assert!(read_response(&mut bytes.as_slice()).is_err());
        bytes.clear();
        wire::write_header_v1(
            &mut bytes,
            &ResponseV1::Read {
                payload_bytes: wire::MAX_TRANSFER_BYTES_V1 + 1,
            },
        )
        .unwrap();
        assert!(read_response(&mut bytes.as_slice()).is_err());
        assert!(read_response(&mut [].as_slice()).is_err());
    }

    #[cfg(any(
        feature = "model-timestamps",
        all(feature = "c1-ordered64", not(feature = "c1-token-program"))
    ))]
    #[test]
    fn token_commands_outside_token_mode_are_rejected_before_writer_traffic() {
        for header in [
            CommandV1::RegisterTokenProgram {
                definition_bytes: 0,
                kernarg_bytes: 0,
            },
            CommandV1::ExecuteTokenProgram {
                program: 1,
                expected_epoch: 0,
                expected_completed_packets: 0,
                timeout_ms: DISPATCH_TIMEOUT_MS,
                updates: vec![],
            },
            CommandV1::ReleaseTokenProgram {
                program: 1,
                expected_epoch: 0,
            },
        ] {
            let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
            let (writer, queued) = mpsc::sync_channel(1);
            worker.writer = Some(writer);
            assert_eq!(
                worker.send(header, vec![]).unwrap_err(),
                "token commands require the explicit unprofiled token mode"
            );
            assert!(matches!(
                queued.try_recv(),
                Err(mpsc::TryRecvError::Disconnected)
            ));
            assert!(worker.pending.is_none());
            assert_eq!((worker.queue_epoch, worker.queue_packets), (0, 0));
            assert!(worker.failed && worker.exited && worker.io_threads.is_empty());
        }
    }

    #[test]
    fn compiler_pointee_width_cannot_be_substituted_by_caller() {
        assert!(pointee_size_matches(Some(ExplicitValueType::U16), 2));
        assert!(!pointee_size_matches(Some(ExplicitValueType::U16), 4));
        assert!(pointee_size_matches(Some(ExplicitValueType::F32), 4));
        assert!(!pointee_size_matches(Some(ExplicitValueType::F32), 2));
        assert!(!pointee_size_matches(Some(ExplicitValueType::Struct), 4));
        assert!(!pointee_size_matches(None, 0));
    }

    #[test]
    fn rollover_requires_exact_receipt_and_retains_owned_buffers() {
        for mode in ["normal", "badrollover"] {
            let mut worker = Worker::connect(fake(mode), 1, Duration::from_secs(5)).unwrap();
            worker.options.rollover = true;
            let buffer = worker.allocate(16).unwrap();
            worker.write(buffer, 0, &[7; 16]).unwrap();
            worker.queue_packets = wire::MAX_UNRETIRED_RING_PACKETS_V1 - 10;
            worker.prepare_packets(10).unwrap();
            assert_eq!(worker.queue_epoch, 0);
            let result = worker.prepare_packets(11);
            if mode == "badrollover" {
                assert!(result.is_err());
                assert!(worker.failed && worker.exited);
            } else {
                result.unwrap();
                assert_eq!((worker.queue_epoch, worker.queue_packets), (1, 0));
                let mut bytes = [0; 16];
                worker.read(buffer, 0, &mut bytes).unwrap();
                assert_eq!(bytes, [7; 16]);
                worker.close().unwrap();
            }
        }
    }

    #[test]
    fn sequence_completion_is_counted_only_when_exact() {
        for mode in ["normal", "shortsequence"] {
            let mut worker = Worker::connect(fake(mode), 1, Duration::from_secs(5)).unwrap();
            let dispatches = (0..2)
                .map(|_| SequenceDispatchV1 {
                    kernel: 1,
                    payload_bytes: 0,
                    workgroup: [64, 1, 1],
                    grid: [64, 1, 1],
                    pointers: vec![],
                    timeout_ms: 1000,
                })
                .collect();
            worker
                .send(CommandV1::DispatchSequence { dispatches }, vec![])
                .unwrap();
            let result = worker.wait_sequence(2);
            if mode == "shortsequence" {
                assert!(result.is_err());
                assert_eq!(worker.queue_packets, 0);
                assert!(worker.failed && worker.exited);
            } else {
                result.unwrap();
                assert_eq!(worker.queue_packets, 2);
                worker.close().unwrap();
            }
        }
    }

    fn ordered_header(count: usize) -> CommandV1 {
        CommandV1::DispatchOrderedBatch {
            dispatches: vec![
                OrderedBatchDispatchV1 {
                    kernel: 1,
                    payload_bytes: 0,
                    workgroup: [64, 1, 1],
                    grid: [64, 1, 1],
                    pointers: vec![],
                };
                count
            ],
            timeout_ms: DISPATCH_TIMEOUT_MS,
        }
    }

    #[cfg(feature = "c1-ordered64")]
    include!("tp_worker_ordered64_tests.rs");

    #[test]
    fn ordered64_runtime_options_reject_incompatible_modes_before_spawn() {
        let options = RuntimeOptions {
            ordered_batches: true,
            ordered64: true,
            ..RuntimeOptions::default()
        };
        assert_eq!(
            options.validate_ordered64().is_ok(),
            cfg!(all(
                feature = "c1-ordered64",
                not(feature = "model-timestamps")
            ))
        );
        for case in 0..4 {
            let mut invalid = options;
            match case {
                0 => invalid.ordered_batches = false,
                1 => invalid.sequences = true,
                2 => invalid.shared_full_currentness = true,
                _ => invalid.profile = true,
            }
            assert!(invalid.validate_ordered64().is_err());
        }
        assert!(RuntimeOptions::default().validate_ordered64().is_ok());
    }

    #[test]
    fn packet_tick_runtime_marker_requires_exact_features_and_excludes_counters() {
        let options = RuntimeOptions {
            ordered_batches: true,
            ordered64: true,
            ordered64_packet_ticks: true,
            ..RuntimeOptions::default()
        };
        assert_eq!(
            options.validate_ordered64().is_ok(),
            cfg!(all(feature = "c1-ordered64", feature = "model-timestamps"))
        );
        for mutation in 0..6 {
            let mut invalid = options;
            match mutation {
                0 => invalid.ordered64 = false,
                1 => invalid.ordered_batches = false,
                2 => invalid.profile = true,
                3 => invalid.ordered64_runtime_counters = true,
                4 => invalid.sequences = true,
                _ => invalid.shared_full_currentness = true,
            }
            assert!(invalid.validate_ordered64().is_err());
        }
        assert!(options.with_ordered64_runtime_counters().is_err());
    }

    #[test]
    fn ordered64_counter_opt_in_preserves_flags_and_rejects_legacy_profile() {
        let original = RuntimeOptions {
            cache_admission: true,
            operational: true,
            ordered_batches: true,
            ordered64: true,
            rollover: true,
            ..RuntimeOptions::default()
        };
        let result = original.with_ordered64_runtime_counters();
        if !cfg!(feature = "c1-ordered64") || cfg!(feature = "model-timestamps") {
            assert!(result.is_err());
            return;
        }
        let counters = result.unwrap();
        assert!(counters.profile && counters.ordered64_runtime_counters);
        assert_eq!(counters.cache_admission, original.cache_admission);
        assert_eq!(counters.operational, original.operational);
        assert_eq!(counters.sequences, original.sequences);
        assert_eq!(counters.ordered_batches, original.ordered_batches);
        assert_eq!(counters.ordered64, original.ordered64);
        assert_eq!(counters.rollover, original.rollover);
        assert_eq!(
            counters.shared_full_currentness,
            original.shared_full_currentness
        );
        assert!(!original.profile && !original.ordered64_runtime_counters);
        assert!(counters.with_ordered64_runtime_counters().is_err());
        let mut legacy = original;
        legacy.profile = true;
        assert!(legacy.validate_ordered64().is_err());
        assert!(legacy.with_ordered64_runtime_counters().is_err());
    }

    #[test]
    fn ordered64_counter_marker_never_grants_incompatible_modes() {
        let options = RuntimeOptions {
            ordered_batches: true,
            ordered64: true,
            profile: true,
            ordered64_runtime_counters: true,
            ..RuntimeOptions::default()
        };
        for case in 0..6 {
            let mut invalid = options;
            match case {
                0 => invalid.ordered64 = false,
                1 => invalid.profile = false,
                2 => invalid.ordered_batches = false,
                3 => invalid.sequences = true,
                4 => invalid.shared_full_currentness = true,
                _ => invalid.ordered64_runtime_counters = false,
            }
            assert!(invalid.validate_ordered64().is_err());
        }
        assert!(
            RuntimeOptions::default()
                .with_ordered64_runtime_counters()
                .is_err()
        );
    }

    #[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
    #[test]
    fn ordered64_counter_mode_cannot_change_after_queue_use() {
        let original = RuntimeOptions {
            ordered_batches: true,
            ordered64: true,
            ..RuntimeOptions::default()
        };
        let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
        worker.options = original;
        worker.queue_packets = 1;
        assert!(
            worker
                .configure_options(original.with_ordered64_runtime_counters().unwrap())
                .is_err()
        );
        assert!(worker.failed && worker.exited);
    }

    #[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
    // Synthetic counters deliberately exercise the real snapshot boundary skew.
    const ORDERED64_COUNTER_WORKER: &str = r"
import json, struct, sys
counters = json.loads(sys.argv[1])
def send(value):
    data = json.dumps(value).encode()
    sys.stdout.buffer.write(struct.pack('<I', len(data)) + data)
    sys.stdout.buffer.flush()
def receive():
    prefix = sys.stdin.buffer.read(4)
    assert len(prefix) == 4
    return json.loads(sys.stdin.buffer.read(struct.unpack('<I', prefix)[0]))
send({'op':'ready','protocol':1,'target':'gfx950:xnack-','device_unique_id':1,'authority':'none'})
assert receive() == {'op':'configure_performance','cache_kernel_admission':True,
                     'operational_currentness':True,'profile':True}
send({'op':'performance_configured'})
assert receive() == {'op':'performance_snapshot'}
counters['operational_currentness_checks'] += 1
counters['operational_currentness_ns'] += 5
snapshot = dict(counters)
counters['commands'] += 1
counters['command_ns'] += 5
send({'op':'performance_snapshot','counters':snapshot})
dispatch = receive()
assert dispatch['op'] == 'dispatch_ordered_batch64'
assert len(dispatch['dispatches']) == 17
assert all(item['payload_bytes'] == 0 for item in dispatch['dispatches'])
counters['commands'] += 1
counters['command_ns'] += 20
counters['operational_currentness_checks'] += 1
counters['operational_currentness_ns'] += 11
counters['dispatches'] += 17
counters['dispatch_prepare_ns'] += 3
counters['dispatch_publish_ns'] += 5
counters['dispatch_wait_ns'] += 7
counters['completion_polls'] += 2
send({'op':'dispatch_ordered_batch64_completed','completed_dispatches':17,'elapsed_ns':12})
assert receive() == {'op':'performance_snapshot'}
counters['operational_currentness_checks'] += 1
counters['operational_currentness_ns'] += 50
snapshot = dict(counters)
counters['commands'] += 1
counters['command_ns'] += 50
send({'op':'performance_snapshot','counters':snapshot})
assert receive() == {'op':'close'}
send({'op':'closed'})
";

    #[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
    #[test]
    fn ordered64_counter_transport_profiles_dispatches_snapshots_and_closes() {
        let child = Command::new("python3")
            .args([
                "-u",
                "-c",
                ORDERED64_COUNTER_WORKER,
                &serde_json::to_string(&wire::PerformanceCountersV1::default()).unwrap(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut worker = Worker::connect(child, 1, Duration::from_secs(5)).unwrap();
        worker
            .configure_options(
                RuntimeOptions {
                    cache_admission: true,
                    operational: true,
                    ordered_batches: true,
                    ordered64: true,
                    rollover: true,
                    ..RuntimeOptions::default()
                }
                .with_ordered64_runtime_counters()
                .unwrap(),
            )
            .unwrap();
        let before = worker.runtime_diagnostic_snapshot().unwrap();
        let CommandV1::DispatchOrderedBatch {
            dispatches,
            timeout_ms,
        } = ordered_header(17)
        else {
            unreachable!()
        };
        worker
            .send(
                CommandV1::DispatchOrderedBatch64 {
                    dispatches,
                    timeout_ms,
                },
                vec![],
            )
            .unwrap();
        worker.wait_ordered_batch(17).unwrap();
        let after = worker.runtime_diagnostic_snapshot().unwrap();
        for (ordinal, snapshot) in [(0, &before), (1, &after)] {
            assert_eq!(snapshot["schema"], "FerricRuntimeDiagnosticSnapshotV1");
            assert_eq!(snapshot["process_id"], worker.pid());
            assert_eq!(snapshot["device_unique_id"], 1);
            assert_eq!(snapshot["rank"], 0);
            assert_eq!(snapshot["ordinal"], ordinal);
            assert_eq!(snapshot["performance_qualified"], false);
        }
        assert_eq!(before["counters"]["commands"], 0);
        assert_eq!(before["counters"]["dispatches"], 0);
        assert_eq!(before["counters"]["operational_currentness_ns"], 5);
        assert_eq!(after["counters"]["commands"], 2);
        assert_eq!(after["counters"]["command_ns"], 25);
        assert_eq!(after["counters"]["dispatches"], 17);
        assert_eq!(after["counters"]["operational_currentness_checks"], 3);
        assert_eq!(after["counters"]["operational_currentness_ns"], 66);
        assert_eq!(after["counters"]["dispatch_prepare_ns"], 3);
        assert_eq!(after["counters"]["dispatch_publish_ns"], 5);
        assert_eq!(after["counters"]["dispatch_wait_ns"], 7);
        assert_eq!(after["counters"]["completion_polls"], 2);
        assert_eq!(worker.queue_packets, 17);
        worker.close().unwrap();
        assert!(worker.exited && !worker.failed);
    }

    #[test]
    fn ordered_completion_is_distinct_exact_and_terminal_on_failure() {
        for mode in [
            "normal",
            "orderedshort",
            "orderedsequence",
            "orderedpayload",
            "orderederror",
            "orderedstall",
        ] {
            let mut worker = Worker::connect(fake(mode), 1, Duration::from_secs(5)).unwrap();
            if mode == "orderedstall" {
                worker.timeout = Duration::from_millis(100);
            }
            worker.send(ordered_header(10), vec![]).unwrap();
            let result = worker.wait_ordered_batch(10);
            if mode == "normal" {
                result.unwrap();
                assert_eq!(worker.queue_packets, 10);
                worker.close().unwrap();
            } else {
                assert!(result.is_err());
                assert_eq!(worker.queue_packets, 0);
                assert!(worker.failed && worker.exited);
                assert!(worker.send(ordered_header(1), vec![]).is_err());
            }
        }
    }

    #[test]
    fn ordered_pending_rejects_mismatched_wait_and_host_commands() {
        for operation in 0..5 {
            let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
            let buffer = worker.allocate(4).unwrap();
            worker.send(ordered_header(5), vec![]).unwrap();
            let result = match operation {
                0 => worker.wait_ordered_batch(4),
                1 => worker.wait_sequence(5),
                2 => worker.write(buffer, 0, &[0; 4]),
                3 => worker.read(buffer, 0, &mut [0; 4]),
                _ => worker.runtime_diagnostic_snapshot().map(|_| ()),
            };
            assert!(result.is_err());
            assert!(worker.failed && worker.exited);
            assert_eq!(worker.queue_packets, 0);
        }
    }

    #[test]
    fn ordered_packets_not_groups_are_retired_and_pending_close_is_drained() {
        let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
        worker.options.rollover = true;
        worker.queue_packets = wire::MAX_UNRETIRED_RING_PACKETS_V1 - 15;
        for count in [10, 5] {
            worker.send(ordered_header(count), vec![]).unwrap();
            worker.wait_ordered_batch(count).unwrap();
        }
        assert_eq!(worker.queue_packets, wire::MAX_UNRETIRED_RING_PACKETS_V1);
        worker.prepare_packets(616).unwrap();
        assert_eq!((worker.queue_epoch, worker.queue_packets), (1, 0));
        worker.send(ordered_header(10), vec![]).unwrap();
        worker.close().unwrap();
        assert_eq!(worker.queue_packets, 10);
        assert!(worker.exited);
    }

    #[test]
    fn ordered_pack_rejects_disabled_empty_oversized_and_unloaded_before_publication() {
        for (enabled, count) in [(false, 1), (true, 0), (true, 17), (true, 1)] {
            let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
            worker.options.ordered_batches = enabled;
            let commands = vec![
                EngineeringTpDispatchV1 {
                    kernel: "not_loaded",
                    grid_workgroups: 1,
                    workgroup_size: 64,
                    arguments: vec![],
                };
                count
            ];
            assert!(worker.submit_ordered_batch(&commands).is_err());
            assert_eq!(worker.queue_packets, 0);
            assert!(worker.failed && worker.exited);
        }
    }

    #[test]
    #[ignore = "requires FERRIC_V8_TEST_ARTIFACT; real metadata packing with a CPU fake worker, no GPU"]
    #[cfg(feature = "tp-batch-engineering")]
    fn actual_v8_metadata_direct_pack_matches_standalone_and_rolls_back_late_errors() {
        let path = std::env::var_os("FERRIC_V8_TEST_ARTIFACT").expect("explicit v8 artifact path");
        let artifact = EngineeringTpArtifactV1::open_fp32_head32(
            Path::new(&path),
            &ferric_qwen3_tp_fp32_head32_kernels_device_v8::compiler_expectation_roster_v8(),
        )
        .unwrap();
        let symbol = "ferric_qwen3_tp_batch32_argmax_f32_v8";
        let metadata = artifact
            .inspection()
            .hsaco()
            .kernels()
            .iter()
            .find(|kernel| kernel.name() == symbol)
            .unwrap();
        let loaded = LoadedKernel {
            id: 7,
            image: *artifact.hsaco_id().as_bytes(),
            metadata: metadata.clone(),
        };
        let buffers = BTreeMap::from([(11, 32 * 151_936 * 4), (12, 32 * 4)]);
        let command = EngineeringTpDispatchV1 {
            kernel: symbol,
            grid_workgroups: 1,
            workgroup_size: 64,
            arguments: vec![
                EngineeringTpArgumentV1::Buffer {
                    id: 11,
                    offset: 0,
                    elements: 32 * 151_936,
                    element_bytes: 4,
                    access: EngineeringTpBufferAccessV1::Read,
                },
                EngineeringTpArgumentV1::Buffer {
                    id: 12,
                    offset: 0,
                    elements: 32,
                    element_bytes: 4,
                    access: EngineeringTpBufferAccessV1::Write,
                },
                EngineeringTpArgumentV1::U32(1),
            ],
        };
        let mut aggregate = vec![0xA5; 19];
        for rows in 1..=16 {
            let mut next = command.clone();
            next.grid_workgroups = rows;
            next.arguments[2] = EngineeringTpArgumentV1::U32(rows);
            let (standalone, bytes) = pack_dispatch(&loaded, &next, &buffers).unwrap();
            let before = aggregate.clone();
            let direct = pack_dispatch_into(&loaded, &next, &buffers, &mut aggregate).unwrap();
            assert_eq!(direct, standalone);
            assert_eq!(&aggregate[..before.len()], before);
            assert_eq!(&aggregate[before.len()..], bytes);
            let mut expected = vec![0; usize::try_from(metadata.kernarg_segment_size()).unwrap()];
            let fields = metadata.explicit_arguments();
            for (index, value) in [
                (1, (32 * 151_936_u64).to_le_bytes()),
                (3, 32_u64.to_le_bytes()),
            ] {
                let offset = usize::try_from(fields[index].offset()).unwrap();
                expected[offset..offset + 8].copy_from_slice(&value);
            }
            let offset = usize::try_from(fields[4].offset()).unwrap();
            expected[offset..offset + 4].copy_from_slice(&rows.to_le_bytes());
            assert_eq!(bytes, expected);
        }
        for mutation in 0..9 {
            let mut bad = command.clone();
            match mutation {
                0 => bad.grid_workgroups = 0,
                1 => bad.arguments.push(EngineeringTpArgumentV1::U32(1)),
                2 => {
                    bad.arguments.pop();
                }
                3 => bad.arguments[2] = EngineeringTpArgumentV1::F32(f32::NAN),
                change => {
                    let EngineeringTpArgumentV1::Buffer {
                        id,
                        offset,
                        elements,
                        element_bytes,
                        access: _,
                    } = &mut bad.arguments[1]
                    else {
                        unreachable!()
                    };
                    match change {
                        4 => *id = 99,
                        5 => *offset = 1,
                        6 => *elements = 33,
                        7 => *element_bytes = 0,
                        8 => *elements = usize::MAX,
                        _ => unreachable!(),
                    }
                }
            }
            let before = aggregate.clone();
            let error = pack_dispatch(&loaded, &bad, &buffers).unwrap_err();
            assert_eq!(
                pack_dispatch_into(&loaded, &bad, &buffers, &mut aggregate).unwrap_err(),
                error
            );
            assert_eq!(aggregate, before);
        }
    }

    #[test]
    #[ignore = "requires FERRIC_V8_TEST_ARTIFACT; real metadata packing with a CPU fake worker, no GPU"]
    #[cfg(feature = "tp-batch-engineering")]
    fn actual_v8_metadata_ordered_pack_preserves_bounds_and_aggregate_timeout() {
        let path = std::env::var_os("FERRIC_V8_TEST_ARTIFACT").expect("explicit v8 artifact path");
        let artifact = EngineeringTpArtifactV1::open_fp32_head32(
            Path::new(&path),
            &ferric_qwen3_tp_fp32_head32_kernels_device_v8::compiler_expectation_roster_v8(),
        )
        .unwrap();
        let symbol = "ferric_qwen3_tp_batch32_argmax_f32_v8";
        let metadata = artifact
            .inspection()
            .hsaco()
            .kernels()
            .iter()
            .find(|kernel| kernel.name() == symbol)
            .unwrap();
        let profiles = if cfg!(all(
            feature = "c1-ordered64",
            not(feature = "model-timestamps")
        )) {
            vec![(false, 16), (true, 16), (true, 17), (true, 64), (true, 65)]
        } else {
            vec![(false, 16)]
        };
        for (wide, count) in profiles {
            for malformed in [false, true] {
                let mut worker =
                    Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
                worker.options.ordered_batches = true;
                worker.options.ordered64 = wide;
                worker.kernels.insert(
                    symbol.into(),
                    LoadedKernel {
                        id: 1,
                        image: *artifact.hsaco_id().as_bytes(),
                        metadata: metadata.clone(),
                    },
                );
                let logits = worker.allocate(32 * 151_936 * 4).unwrap();
                let choice = worker.allocate(32 * 4).unwrap();
                let command = EngineeringTpDispatchV1 {
                    kernel: symbol,
                    grid_workgroups: 1,
                    workgroup_size: 64,
                    arguments: vec![
                        EngineeringTpArgumentV1::Buffer {
                            id: logits,
                            offset: 0,
                            elements: 32 * 151_936,
                            element_bytes: 4,
                            access: EngineeringTpBufferAccessV1::Read,
                        },
                        EngineeringTpArgumentV1::Buffer {
                            id: choice,
                            offset: 0,
                            elements: if malformed { 33 } else { 32 },
                            element_bytes: 4,
                            access: EngineeringTpBufferAccessV1::Write,
                        },
                        EngineeringTpArgumentV1::U32(1),
                    ],
                };
                let result = worker.submit_ordered_batch(&vec![command; count]);
                if malformed || count == 65 {
                    assert!(result.is_err() && worker.failed && worker.exited);
                    assert_eq!(worker.queue_packets, 0);
                } else {
                    result.unwrap();
                    worker.wait_ordered_batch(count).unwrap();
                    assert_eq!(worker.queue_packets, count as u64);
                    worker.close().unwrap();
                }
            }
        }
    }
}
