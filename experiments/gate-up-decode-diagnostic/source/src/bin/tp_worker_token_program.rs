//! Fixed TP1/C1 client policy over fe2o3-owned wire types. No packet cache.

use super::{
    BTreeMap, BufferAccessV1, CommandV1, DISPATCH_TIMEOUT_MS, EngineeringTpArgumentV1,
    EngineeringTpArtifactV1, EngineeringTpDispatchV1, EngineeringTpRankTransportV1, HostTiming,
    LoadedKernel, OrderedBatchDispatchV1, Path, PendingRequest, ResponseV1, RuntimeOptions,
    TokenProgramBackend, TpResult, Worker, WorkerEntry, pack_dispatch_into, wire,
};
use ferric_m1_engineering_execution_v1::tp_artifact::{
    ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19, ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21,
};
use wire::{TokenProgramDefinitionV1, TokenProgramSlotV1 as Slot, TokenProgramUpdateV1 as Update};

const PACKETS: usize = 652;
const SLOTS: usize = 180;

#[path = "tp_worker_gate_up_program.rs"]
mod gate_up;

#[path = "tp_worker_decode_diagnostic.rs"]
mod decode_diagnostic;

pub(super) const fn prefill_width_profile(rows: u32, counters: bool) -> &'static str {
    match (rows, counters) {
        (16, false) => "prefill16-native613-slots512-worker-decode652-v1",
        (32, false) => "prefill32-native649-slots512-worker-decode652-v1",
        (16, true) => "prefill16-native613-slots512-worker-decode652-counters-v1",
        (32, true) => "prefill32-native649-slots512-worker-decode652-counters-v1",
        _ => "invalid-prefill-width",
    }
}

#[path = "tp_worker_prefill_program.rs"]
mod prefill;
#[path = "tp_worker_prefill32_program.rs"]
mod prefill32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Shape {
    Decode,
    DecodeGateUp724,
    Prefill16,
    Prefill32,
}

impl Shape {
    const fn packets(self) -> usize {
        match self {
            Self::Decode => PACKETS,
            Self::DecodeGateUp724 => 724,
            Self::Prefill16 => 613,
            Self::Prefill32 => 649,
        }
    }
    const fn slots(self) -> usize {
        match self {
            Self::Decode | Self::DecodeGateUp724 => SLOTS,
            Self::Prefill16 => 216,
            Self::Prefill32 => 396,
        }
    }
    const fn is_prefill(self) -> bool {
        matches!(self, Self::Prefill16 | Self::Prefill32)
    }
}

pub(super) struct State {
    registered: Option<Registered>,
    pub(super) backend: TokenProgramBackend,
    counter_snapshots: u8,
    completed_executions: u64,
    pub(super) prefill_enabled: bool,
    pub(super) prefill_rows: u32,
    completed_prefills: u64,
    registrations: u64,
    releases: u64,
    gate_up_selected: Option<bool>,
    decode_shape: Shape,
    diagnostic: Option<Box<decode_diagnostic::Recorder>>,
}

struct Registered {
    shape: Shape,
    program: u64,
    epoch: u64,
    immutable: (TokenProgramDefinitionV1, Vec<u8>),
}

struct Plan {
    definition: TokenProgramDefinitionV1,
    kernargs: Vec<u8>,
    updates: Vec<Update>,
}

impl Plan {
    fn immutable(&self) -> TpResult<(TokenProgramDefinitionV1, Vec<u8>)> {
        let mut definition = self.definition.clone();
        let mut bytes = self.kernargs.clone();
        let mut offsets = Vec::with_capacity(definition.dispatches.len());
        let mut total = 0usize;
        for command in &definition.dispatches {
            offsets.push(total);
            total = total
                .checked_add(command.payload_bytes as usize)
                .ok_or("token payload overflow")?;
        }
        if total != bytes.len() {
            return Err("token payload length drifted".into());
        }
        for slot in &definition.slots {
            match *slot {
                Slot::ScalarU32 {
                    dispatch, offset, ..
                } => {
                    let start = offsets[usize::from(dispatch)] + offset as usize;
                    bytes
                        .get_mut(start..start + 4)
                        .ok_or("token scalar extent")?
                        .fill(0);
                }
                Slot::Pointer {
                    dispatch, pointer, ..
                } => {
                    definition.dispatches[usize::from(dispatch)].pointers[usize::from(pointer)]
                        .buffer_offset = 0;
                }
            }
        }
        Ok((definition, bytes))
    }
}

fn scalar(arguments: &[EngineeringTpArgumentV1], index: usize) -> TpResult<u32> {
    match arguments.get(index) {
        Some(EngineeringTpArgumentV1::U32(value)) => Ok(*value),
        _ => Err("token dynamic argument is not U32".into()),
    }
}

fn plan(
    dispatches: &[EngineeringTpDispatchV1],
    kernels: &BTreeMap<String, LoadedKernel>,
    buffers: &BTreeMap<u64, usize>,
) -> TpResult<Plan> {
    plan_for_shape(dispatches, kernels, buffers, Shape::Decode)
}

fn plan_for_shape(
    dispatches: &[EngineeringTpDispatchV1],
    kernels: &BTreeMap<String, LoadedKernel>,
    buffers: &BTreeMap<u64, usize>,
    shape: Shape,
) -> TpResult<Plan> {
    if shape.is_prefill() || dispatches.len() != shape.packets() {
        return Err("token program requires its explicitly selected full decode graph".into());
    }
    if shape == Shape::DecodeGateUp724 {
        gate_up::validate_route(dispatches)?;
    } else if dispatches.iter().any(|command|
        ferric_m1_engineering_execution_v1::tp_artifact::ENGINEERING_TP_SPLITK_GATE_UP_EXPORTS_R1.contains(&command.kernel)) {
        return Err("split-K roots are excluded from the control decode program".into());
    }
    let mut result = Plan {
        definition: TokenProgramDefinitionV1 {
            dispatches: Vec::with_capacity(shape.packets()),
            slots: Vec::with_capacity(SLOTS),
        },
        kernargs: Vec::new(),
        updates: Vec::with_capacity(SLOTS),
    };
    let mut copy_count = 0;
    let mut attention_count = 0;
    let mut position = None;
    let mut physical_slot = None;
    let mut context = None;
    for (index, dispatch) in dispatches.iter().enumerate() {
        let dispatch_index = u16::try_from(index).map_err(|_| "token dispatch index")?;
        let loaded = kernels
            .get(dispatch.kernel)
            .ok_or("unloaded token program kernel")?;
        let header = pack_dispatch_into(loaded, dispatch, buffers, &mut result.kernargs)?;
        let CommandV1::Dispatch {
            kernel,
            payload_bytes,
            workgroup,
            grid,
            pointers,
            ..
        } = header
        else {
            return Err("token dispatch packing changed".into());
        };
        let entry = OrderedBatchDispatchV1 {
            kernel,
            payload_bytes,
            workgroup,
            grid,
            pointers,
        };
        let copy = dispatch.kernel == ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19[0];
        let attention = dispatch.kernel == ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21[0];
        if copy {
            copy_count += 1;
            let pos = scalar(&dispatch.arguments, 4)?;
            let page = scalar(&dispatch.arguments, 5)?;
            let pages = scalar(&dispatch.arguments, 6)?;
            if dispatch.arguments.len() != 7
                || !(127..=255).contains(&pos)
                || !(1..=512).contains(&pages)
                || page >= pages
                || position.is_some_and(|previous| previous != pos)
            {
                return Err("token V19 position/page contract".into());
            }
            position = Some(pos);
            if physical_slot.is_some_and(|previous| previous != (page, pages)) {
                return Err("token V19 physical page must agree across every layer".into());
            }
            physical_slot = Some((page, pages));
            let offset = (u64::from(page) * 16 + u64::from(pos % 16)) * 1024 * 2;
            for pointer in [2usize, 3] {
                let fixup = entry
                    .pointers
                    .get(pointer)
                    .ok_or("token V19 pointer roster")?;
                if fixup.buffer_offset != offset
                    || fixup.extent_bytes != 2048
                    || fixup.access != BufferAccessV1::Write
                {
                    return Err("token V19 offset does not match its prepared physical slot".into());
                }
            }
        }
        if attention {
            attention_count += 1;
            let current = scalar(&dispatch.arguments, 11)?;
            if dispatch.arguments.len() != 12
                || !(128..=256).contains(&current)
                || scalar(&dispatch.arguments, 7)? != 1
                || scalar(&dispatch.arguments, 8)? != 1
                || context.is_some_and(|previous| previous != current)
            {
                return Err("token split8 requires consistent C1 context".into());
            }
            context = Some(current);
        }
        let mut fields = loaded.metadata.explicit_arguments().iter();
        let mut pointer_index = 0u16;
        for (argument, value) in dispatch.arguments.iter().enumerate() {
            let field = fields.next().ok_or("token explicit ABI roster")?;
            match value {
                EngineeringTpArgumentV1::Buffer { id, .. } => {
                    fields.next().ok_or("token slice length ABI")?;
                    if copy && matches!(argument, 2 | 3) {
                        let fixup = &entry.pointers[usize::from(pointer_index)];
                        let maximum_offset = (*buffers.get(id).ok_or("token buffer ownership")?
                            as u64)
                            .checked_sub(fixup.extent_bytes)
                            .ok_or("token pointer extent")?;
                        result.definition.slots.push(Slot::Pointer {
                            dispatch: dispatch_index,
                            pointer: pointer_index,
                            buffers: vec![*id],
                            maximum_offset,
                        });
                        result.updates.push(Update::Pointer {
                            buffer: *id,
                            offset: fixup.buffer_offset,
                        });
                    }
                    pointer_index += 1;
                }
                EngineeringTpArgumentV1::U32(value)
                    if (copy && matches!(argument, 4 | 5)) || (attention && argument == 11) =>
                {
                    let (minimum, maximum) = if attention {
                        (128, 256)
                    } else if argument == 4 {
                        (127, 255)
                    } else {
                        (0, scalar(&dispatch.arguments, 6)? - 1)
                    };
                    result.definition.slots.push(Slot::ScalarU32 {
                        dispatch: dispatch_index,
                        offset: u32::try_from(field.offset()).map_err(|_| "token scalar offset")?,
                        minimum,
                        maximum,
                    });
                    result.updates.push(Update::ScalarU32 { value: *value });
                }
                _ => {}
            }
        }
        result.definition.dispatches.push(entry);
    }
    if copy_count != 36
        || attention_count != 36
        || result.definition.slots.len() != SLOTS
        || position.and_then(|value| value.checked_add(1)) != context
    {
        return Err("token graph must contain all 36 V19/split8 layers at one position".into());
    }
    Ok(result)
}

impl Worker {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub(crate) fn spawn_native_prefill_width_program(
        executable: &Path,
        unique_id: u64,
        artifact: &EngineeringTpArtifactV1,
        options: RuntimeOptions,
        timing: HostTiming,
        rank: u32,
        rows: u32,
        counters: bool,
    ) -> TpResult<Self> {
        Self::spawn_entry(
            executable,
            unique_id,
            artifact,
            options,
            timing,
            rank,
            WorkerEntry::NativePrefillWidthProgram { rows, counters },
        )
    }

    pub(super) fn registered_program_packets(&self) -> usize {
        self.token_program
            .as_ref()
            .and_then(|state| state.registered.as_ref())
            .map_or(0, |registered| registered.shape.packets())
    }

    pub(super) fn prefill_program_enabled(&self) -> bool {
        self.token_program.as_ref().is_some_and(|state| {
            state.prefill_enabled
                && matches!(
                    state.backend,
                    TokenProgramBackend::NativeWholeProgramV1
                        | TokenProgramBackend::NativeWholeProgramSlots512V1
                )
        })
    }

    pub(super) fn prefill32_program_enabled(&self) -> bool {
        self.prefill_program_enabled()
            && self.token_program.as_ref().is_some_and(|state| {
                state.backend == TokenProgramBackend::NativeWholeProgramSlots512V1
                    && state.prefill_rows == 32
            })
    }

    pub(super) fn submit_fixed_prefill(
        &mut self,
        dispatches: &[EngineeringTpDispatchV1],
    ) -> TpResult<()> {
        if !self.prefill_program_enabled() || self.pending.is_some() || self.failed || self.exited {
            return self.reject("prefill program not explicitly ready");
        }
        let shape = if self.prefill32_program_enabled() {
            Shape::Prefill32
        } else {
            Shape::Prefill16
        };
        let prepared = if shape == Shape::Prefill32 {
            prefill32::plan_prefill32(dispatches, &self.kernels, &self.buffers)
        } else {
            prefill::plan_prefill(dispatches, &self.kernels, &self.buffers)
        }
        .and_then(|plan| {
            let immutable = plan.immutable()?;
            (if shape == Shape::Prefill32 {
                wire::validate_token_program_slots512_encoding_v1(&plan.definition, &plan.kernargs)
            } else {
                wire::validate_token_program_encoding_v1(&plan.definition, &plan.kernargs)
            })
            .map_err(|error| error.to_string())?;
            Ok((plan, immutable))
        });
        let (plan, immutable) = match prepared {
            Ok(value) => value,
            Err(error) => return self.reject(error),
        };
        self.submit_shape(plan, immutable, shape)
    }

    pub(super) fn token_program_backend(&self) -> Option<TokenProgramBackend> {
        self.token_program.as_ref().map(|state| state.backend)
    }

    pub(super) fn verify_token_program_backend(
        &mut self,
        expected: TokenProgramBackend,
    ) -> TpResult<()> {
        if self.token_program.is_some()
            || !self.buffers.is_empty()
            || !self.kernels.is_empty()
            || self.queue_packets != 0
            || self.queue_epoch != 0
        {
            return self.reject("token backend identity requires a fresh worker");
        }
        self.send(CommandV1::DescribeTokenProgramBackendV1 {}, vec![])?;
        if !matches!(self.receive()?.header,
            ResponseV1::TokenProgramBackendV1 { backend } if backend == expected.identity())
        {
            return self.reject("token backend identity mismatch; fallback is forbidden");
        }
        self.token_program = Some(Box::new(State {
            registered: None,
            backend: expected,
            counter_snapshots: 0,
            completed_executions: 0,
            prefill_enabled: false,
            prefill_rows: 16,
            completed_prefills: 0,
            registrations: 0,
            releases: 0,
            gate_up_selected: None,
            decode_shape: Shape::Decode,
            diagnostic: None,
        }));
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn spawn_token_program_with_timing(
        executable: &Path,
        unique_id: u64,
        artifact: &EngineeringTpArtifactV1,
        options: RuntimeOptions,
        timing: HostTiming,
        rank: u32,
    ) -> TpResult<Self> {
        Self::spawn_mode(executable, unique_id, artifact, options, timing, rank, true)
    }

    #[allow(dead_code)]
    pub(crate) fn spawn_native_token_program_with_timing(
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
            WorkerEntry::NativeTokenProgram,
        )
    }

    #[allow(dead_code)]
    pub(crate) fn spawn_token_program_counters_with_timing(
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
            WorkerEntry::TokenProgramCounters(TokenProgramBackend::Ordered64GroupsV1),
        )
    }

    #[allow(dead_code)]
    pub(crate) fn spawn_native_token_program_counters_with_timing(
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
            WorkerEntry::TokenProgramCounters(TokenProgramBackend::NativeWholeProgramV1),
        )
    }

    #[allow(dead_code)]
    pub(crate) fn spawn_native_prefill_program_with_timing(
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
            WorkerEntry::NativePrefillProgram { counters: false },
        )
    }

    #[allow(dead_code)]
    pub(crate) fn spawn_native_prefill_program_counters_with_timing(
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
            WorkerEntry::NativePrefillProgram { counters: true },
        )
    }

    fn token_program_counter_snapshot(&mut self) -> TpResult<serde_json::Value> {
        let state = self
            .token_program
            .as_ref()
            .ok_or("token counter backend missing")?;
        let ordinal = state.counter_snapshots;
        let backend = state.backend;
        let completed_executions = state.completed_executions;
        let prefills = state.completed_prefills;
        let prefill_enabled = state.prefill_enabled;
        let prefill_rows = state.prefill_rows;
        let decode_shape = state.decode_shape;
        let gate_up_selected = state.gate_up_selected;
        let prefill_shape = if prefill_rows == 32 {
            Shape::Prefill32
        } else {
            Shape::Prefill16
        };
        let registrations = state.registrations;
        let releases = state.releases;
        if ordinal >= 2 || !self.options.profile || !self.options.ordered64_runtime_counters {
            return self.reject("token counters require the explicit bounded diagnostic");
        }
        self.send(CommandV1::TokenProgramSnapshotV1 {}, vec![])?;
        let ResponseV1::TokenProgramSnapshotV1 {
            backend: actual,
            counters,
        } = self.receive()?.header
        else {
            return self.reject("token counter snapshot response mismatch");
        };
        let groups = match backend {
            TokenProgramBackend::Ordered64GroupsV1 => 11,
            TokenProgramBackend::NativeWholeProgramV1
            | TokenProgramBackend::NativeWholeProgramSlots512V1 => 1,
        };
        let executions = completed_executions
            .checked_add(prefills)
            .ok_or("execution counter overflow")?;
        let dispatches = completed_executions
            .checked_mul(decode_shape.packets() as u64)
            .and_then(|count| {
                prefills
                    .checked_mul(prefill_shape.packets() as u64)
                    .and_then(|prefill| count.checked_add(prefill))
            })
            .ok_or("program dispatch counter overflow")?;
        let publications = completed_executions
            .checked_mul(groups)
            .and_then(|count| count.checked_add(prefills))
            .ok_or("publication counter overflow")?;
        if actual != backend.identity()
            || (!prefill_enabled && prefills != 0)
            || (prefill_enabled
                && !matches!(
                    backend,
                    TokenProgramBackend::NativeWholeProgramV1
                        | TokenProgramBackend::NativeWholeProgramSlots512V1
                ))
            || counters.executions != executions
            || counters.dispatches != dispatches
            || counters.publications != publications
            || counters.final_waits != counters.publications
            || counters.retirement_signals != counters.dispatches
            || (ordinal == 0
                && (counters.executions != 0
                    || counters.staging_ns != 0
                    || counters.kernarg_initialized_bytes != 0))
        {
            return self.reject("token counter identity or complete-program mechanism mismatch");
        }
        self.token_program
            .as_mut()
            .expect("token counter backend")
            .counter_snapshots += 1;
        let mut snapshot = serde_json::json!({
            "schema":"FerricTokenProgramCountersV1", "authority":"none",
            "performance_qualified":false, "runtime_profiling":true,
            "latency_sample_admitted":false, "backend":backend.identity(),
            "worker_entry":backend.argument(), "live_profile":backend.counter_profile(),
            "process_id":self.pid(), "device_unique_id":self.diagnostic_identity.0,
            "ordinal":ordinal, "phase":if ordinal == 0 { "worker_start" } else { "before_close" },
            "scope":"successful token executions only; excludes registration, ordinary prefill and readback",
            "staging_scope":"host staging wall time; not GPU time",
            "kernarg_bytes_scope":"initialized stores; baseline includes slot clear and exact copy",
            "counters":counters,
        });
        if prefill_enabled {
            snapshot["schema"] = serde_json::json!("FerricPrefillProgramCountersV1");
            snapshot["live_profile"] =
                serde_json::json!("prefill16-native613-decode-native652-counters-v1");
            snapshot["scope"] = serde_json::json!(
                "successful prefill613 and decode652 programs; excludes head singletons, registration and readback"
            );
            snapshot["program_phases"] = serde_json::json!({
                "prefill16":{"executions":prefills,"dispatches_per_execution":613,"dynamic_slots":216},
                "decode_c1":{"executions":completed_executions,"dispatches_per_execution":652,"dynamic_slots":180},
                "registrations":registrations,"releases":releases,
                "transition_policy":"idle-explicit-shape-release-register-v1"
            });
            if backend == TokenProgramBackend::NativeWholeProgramSlots512V1 {
                snapshot["schema"] = serde_json::json!("FerricPrefillWidthProgramCountersV1");
                snapshot["live_profile"] =
                    serde_json::json!(prefill_width_profile(prefill_rows, true));
                snapshot["scope"] = serde_json::json!(
                    "successful selected-width prefill and decode652 programs; excludes head singletons, registration and readback"
                );
                snapshot["program_phases"] = serde_json::json!({
                    "prefill":{"rows":prefill_rows,"executions":prefills,"dispatches_per_execution":prefill_shape.packets(),"dynamic_slots":prefill_shape.slots(),"command_family":if prefill_rows == 32 { "slots512-v1" } else { "legacy256-v1" }},
                    "decode_c1":{"executions":completed_executions,"dispatches_per_execution":decode_shape.packets(),"dynamic_slots":decode_shape.slots(),"command_family":"legacy256-v1"},
                    "registrations":registrations,"releases":releases,
                    "transition_policy":"idle-explicit-shape-release-register-v1"
                });
                if let Some(enabled) = gate_up_selected {
                    snapshot["schema"] = serde_json::json!("FerricNativeGateUpProgramCountersR1");
                    snapshot["live_profile"] = serde_json::json!(gate_up::profile(enabled, true));
                    snapshot["gate_up_splitk4"] = serde_json::json!({
                        "enabled":enabled,"scratch_bytes":196_608,
                        "prefill_unchanged":true,"decode_dispatches":decode_shape.packets()
                    });
                    snapshot["scope"] = serde_json::json!(
                        "successful prefill649 and explicitly selected decode programs; excludes head singletons, registration and readback"
                    );
                }
            }
        }
        Ok(snapshot)
    }

    pub(super) fn emit_token_program_counters(&mut self) -> TpResult<()> {
        use std::io::Write;
        let snapshot = self.token_program_counter_snapshot()?;
        writeln!(std::io::stderr().lock(), "{snapshot}")
            .map_err(|error| format!("token counter output: {error}"))
    }

    pub(super) fn validate_token_request(&self, header: &CommandV1) -> TpResult<()> {
        let registered = self
            .token_program
            .as_ref()
            .and_then(|state| state.registered.as_ref());
        match header {
            CommandV1::RegisterTokenProgram { .. }
                if self.token_program.is_some() && registered.is_none() =>
            {
                Ok(())
            }
            CommandV1::RegisterTokenProgramSlots512V1 { .. }
                if self.prefill32_program_enabled() && registered.is_none() =>
            {
                Ok(())
            }
            CommandV1::ExecuteTokenProgram {
                program,
                expected_epoch,
                expected_completed_packets,
                timeout_ms,
                updates,
            }
            | CommandV1::ExecuteTokenProgramSlots512V1 {
                program,
                expected_epoch,
                expected_completed_packets,
                timeout_ms,
                updates,
            } if registered.is_some_and(|r| {
                (r.shape == Shape::Prefill32)
                    == matches!(header, CommandV1::ExecuteTokenProgramSlots512V1 { .. })
            }) && registered
                .is_some_and(|r| r.program == *program && r.epoch == *expected_epoch)
                && *expected_epoch == self.queue_epoch
                && *expected_completed_packets == self.queue_packets
                && *timeout_ms == DISPATCH_TIMEOUT_MS
                && registered.is_some_and(|r| updates.len() == r.shape.slots())
                && self
                    .queue_packets
                    .checked_add(self.registered_program_packets() as u64)
                    .is_some_and(|next| next <= wire::MAX_UNRETIRED_RING_PACKETS_V1) =>
            {
                Ok(())
            }
            CommandV1::ReleaseTokenProgram {
                program,
                expected_epoch,
            } if registered
                .is_some_and(|r| r.program == *program && r.epoch == *expected_epoch)
                && *expected_epoch == self.queue_epoch =>
            {
                Ok(())
            }
            CommandV1::RegisterTokenProgram { .. }
            | CommandV1::RegisterTokenProgramSlots512V1 { .. }
            | CommandV1::ExecuteTokenProgram { .. }
            | CommandV1::ExecuteTokenProgramSlots512V1 { .. }
            | CommandV1::ReleaseTokenProgram { .. } => {
                Err("unadmitted token program request".into())
            }
            CommandV1::Allocate { .. }
            | CommandV1::Free { .. }
            | CommandV1::LoadKernel { .. }
            | CommandV1::RolloverQueue { .. }
                if registered.is_some() =>
            {
                Err("release token program before resource or queue mutation".into())
            }
            _ => Ok(()),
        }
    }

    pub(super) fn submit_fixed_token(
        &mut self,
        dispatches: &[EngineeringTpDispatchV1],
    ) -> TpResult<()> {
        if !self.supports_token_program() || self.pending.is_some() || self.failed || self.exited {
            return self.reject("fixed token program not ready");
        }
        // The full graph, including its last binding, is checked before register or execute can send.
        let diagnostic_start = self.diagnostic_planner_start();
        let shape = self
            .token_program
            .as_ref()
            .ok_or("token mode unavailable")?
            .decode_shape;
        let prepared = if shape == Shape::Decode {
            plan(dispatches, &self.kernels, &self.buffers)
        } else {
            plan_for_shape(dispatches, &self.kernels, &self.buffers, shape)
        }
        .and_then(|plan| {
            let immutable = plan.immutable()?;
            wire::validate_token_program_encoding_v1(&plan.definition, &plan.kernargs)
                .map_err(|error| error.to_string())?;
            Ok((plan, immutable))
        });
        let (plan, immutable) = match prepared {
            Ok(value) => value,
            Err(error) => return self.reject(error),
        };
        self.diagnostic_planner_end(diagnostic_start)?;
        if shape == Shape::Decode {
            self.submit_plan(plan, immutable)
        } else {
            self.submit_shape(plan, immutable, shape)
        }
    }

    fn submit_plan(
        &mut self,
        plan: Plan,
        immutable: (TokenProgramDefinitionV1, Vec<u8>),
    ) -> TpResult<()> {
        self.submit_shape(plan, immutable, Shape::Decode)
    }

    fn submit_shape(
        &mut self,
        plan: Plan,
        immutable: (TokenProgramDefinitionV1, Vec<u8>),
        shape: Shape,
    ) -> TpResult<()> {
        if self.pending.is_some()
            || self.failed
            || self.exited
            || plan.definition.dispatches.len() != shape.packets()
            || plan.definition.slots.len() != shape.slots()
            || plan.updates.len() != shape.slots()
            || (!shape.is_prefill()
                && self
                    .token_program
                    .as_ref()
                    .is_none_or(|state| state.decode_shape != shape))
            || (shape == Shape::Prefill16 && !self.prefill_program_enabled())
            || (shape == Shape::Prefill16 && self.prefill32_program_enabled())
            || (shape == Shape::Prefill32 && !self.prefill32_program_enabled())
        {
            return self.reject("program shape or idle transition changed");
        }
        if self
            .token_program
            .as_ref()
            .and_then(|state| state.registered.as_ref())
            .is_some_and(|registered| registered.shape != shape)
        {
            if !self.prefill_program_enabled() {
                return self.reject("program phase transition was not explicitly selected");
            }
            self.release_registered_token()?;
        }
        if let Some(registered) = self
            .token_program
            .as_ref()
            .and_then(|state| state.registered.as_ref())
        {
            if registered.epoch != self.queue_epoch || registered.immutable != immutable {
                return self.reject("token immutable graph, ABI, bindings or epoch changed");
            }
        } else {
            let (command, payload) = match if shape == Shape::Prefill32 {
                wire::encode_token_program_slots512_v1(&plan.definition, &plan.kernargs)
            } else {
                wire::encode_token_program_v1(&plan.definition, &plan.kernargs)
            } {
                Ok(value) => value,
                Err(error) => return self.reject(error.to_string()),
            };
            self.send(command, payload)?;
            if self.pending != Some(PendingRequest::TokenRegister) {
                return self.reject("token registration pending kind");
            }
            let response = self.receive()?;
            let program = match response.header {
                ResponseV1::TokenProgramRegistered {
                    program,
                    device_unique_id,
                    queue_epoch,
                    dispatches,
                    slots,
                } if program != 0
                    && device_unique_id == self.diagnostic_identity.0
                    && queue_epoch == self.queue_epoch
                    && usize::try_from(dispatches) == Ok(shape.packets())
                    && usize::try_from(slots) == Ok(shape.slots())
                    && response.payload.is_empty() =>
                {
                    program
                }
                _ => return self.reject("token registration response mismatch"),
            };
            self.token_program
                .as_mut()
                .expect("selected token mode")
                .registered = Some(Registered {
                shape,
                program,
                epoch: self.queue_epoch,
                immutable,
            });
            let state = self.token_program.as_mut().expect("selected token mode");
            state.registrations = state
                .registrations
                .checked_add(1)
                .ok_or("registration count overflow")?;
        }
        let registered = self
            .token_program
            .as_ref()
            .and_then(|state| state.registered.as_ref())
            .expect("registered token");
        let command = if shape == Shape::Prefill32 {
            CommandV1::ExecuteTokenProgramSlots512V1 {
                program: registered.program,
                expected_epoch: registered.epoch,
                expected_completed_packets: self.queue_packets,
                timeout_ms: DISPATCH_TIMEOUT_MS,
                updates: plan.updates,
            }
        } else {
            CommandV1::ExecuteTokenProgram {
                program: registered.program,
                expected_epoch: registered.epoch,
                expected_completed_packets: self.queue_packets,
                timeout_ms: DISPATCH_TIMEOUT_MS,
                updates: plan.updates,
            }
        };
        self.diagnostic_execute_start()?;
        self.send(command, Vec::new())
    }

    pub(super) fn wait_fixed_token(&mut self, count: usize) -> TpResult<()> {
        let Some(PendingRequest::TokenExecute {
            program,
            epoch,
            next,
        }) = self.pending
        else {
            return self.reject("no pending token execution");
        };
        if count != self.registered_program_packets()
            || epoch != self.queue_epoch
            || self.queue_packets.checked_add(count as u64) != Some(next)
        {
            return self.reject("pending token count or frontier drifted");
        }
        let response = self.receive()?;
        if !matches!(response.header, ResponseV1::TokenProgramCompleted {
            program: actual, device_unique_id, queue_epoch, completed_dispatches, completed_packets, ..
        } if actual == program && device_unique_id == self.diagnostic_identity.0 && queue_epoch == epoch
            && usize::try_from(completed_dispatches) == Ok(count) && completed_packets == next)
            || !response.payload.is_empty()
        {
            return self.reject("token aggregate completion identity or frontier mismatch");
        }
        let state = self.token_program.as_mut().expect("selected token mode");
        let completed = if state
            .registered
            .as_ref()
            .is_some_and(|r| r.shape.is_prefill())
        {
            &mut state.completed_prefills
        } else {
            &mut state.completed_executions
        };
        let Some(next_completed) = completed.checked_add(1) else {
            return self.reject("token execution count overflow");
        };
        *completed = next_completed;
        self.queue_packets = next;
        self.diagnostic_execute_end(program, epoch, next)?;
        Ok(())
    }

    pub(super) fn release_registered_token(&mut self) -> TpResult<()> {
        let Some(registered) = self
            .token_program
            .as_ref()
            .and_then(|state| state.registered.as_ref())
        else {
            return Ok(());
        };
        let (program, epoch) = (registered.program, registered.epoch);
        self.send(
            CommandV1::ReleaseTokenProgram {
                program,
                expected_epoch: epoch,
            },
            Vec::new(),
        )?;
        if self.pending != Some(PendingRequest::TokenRelease { program, epoch }) {
            return self.reject("token release pending kind");
        }
        let response = self.receive()?;
        if !matches!(response.header, ResponseV1::TokenProgramReleased { program: actual, queue_epoch }
            if actual == program && queue_epoch == epoch)
            || !response.payload.is_empty()
        {
            return self.reject("token release response mismatch");
        }
        self.token_program
            .as_mut()
            .expect("selected token mode")
            .registered = None;
        let state = self.token_program.as_mut().expect("selected token mode");
        state.releases = state
            .releases
            .checked_add(1)
            .ok_or("release count overflow")?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "tp_worker_token_program_tests.rs"]
mod tests;
