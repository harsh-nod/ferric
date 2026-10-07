//! Explicit native688 decode policy; no packet-count-based mode inference.
use super::{
    ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19, ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21,
    EngineeringTpArgumentV1, EngineeringTpArtifactV1, EngineeringTpDispatchV1, HostTiming, Path,
    RuntimeOptions, Shape, TokenProgramBackend, TpResult, Worker, WorkerEntry,
};
use ferric_m1_engineering_execution_v1::tp_artifact::ENGINEERING_TP_SPLITK_DOWN_EXPORTS_R1 as ROOTS;
use ferric_m1_engineering_execution_v1::tp_execution::EngineeringTpBufferAccessV1 as Access;
use std::collections::BTreeSet;

pub(crate) const fn profile(enabled: bool, counters: bool) -> &'static str {
    match (enabled, counters) {
        (false, false) => "prefill32-native649-decode652-down-control-r1",
        (true, false) => "prefill32-native649-decode688-down-splitk8-r1",
        (false, true) => "prefill32-native649-decode652-down-control-counters-r1",
        (true, true) => "prefill32-native649-decode688-down-splitk8-counters-r1",
    }
}

fn view(
    argument: &EngineeringTpArgumentV1,
    elements: usize,
    element_bytes: u32,
    access: Access,
) -> TpResult<u64> {
    match argument {
        EngineeringTpArgumentV1::Buffer {
            id,
            offset: 0,
            elements: count,
            element_bytes: width,
            access: actual,
        } if *id != 0 && *count == elements && *width == element_bytes && *actual == access => {
            Ok(*id)
        }
        _ => Err("native688 requires exact short-view ABI".into()),
    }
}

pub(super) fn validate_route(commands: &[EngineeringTpDispatchV1]) -> TpResult<()> {
    if commands.len() != 688 {
        return Err("native688 requires its entire fixed graph".into());
    }
    let mut scratch = None;
    let mut weights = BTreeSet::new();
    let mut targets = BTreeSet::new();
    let mut activations = BTreeSet::new();
    let mut expected_indices = BTreeSet::new();
    for layer in 0..36 {
        let base = 1 + layer * 19;
        if commands[base + 7].kernel != ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19[0]
            || commands[base + 8].kernel != ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21[0]
        {
            return Err("native688 attention/copy layer ordering changed".into());
        }
        let partial = &commands[base + 16];
        let merge = &commands[base + 17];
        expected_indices.extend([base + 16, base + 17]);
        if partial.kernel != ROOTS[0]
            || merge.kernel != ROOTS[1]
            || partial.workgroup_size != 64
            || partial.grid_workgroups != 2048
            || merge.workgroup_size != 64
            || merge.grid_workgroups != 64
            || partial.arguments.len() != 8
            || merge.arguments.len() != 2
            || partial.arguments[3..] != [1, 4096, 12_288, 1, 2].map(EngineeringTpArgumentV1::U32)
        {
            return Err("native688 split-K pair order, role or geometry changed".into());
        }
        let input = view(&partial.arguments[0], 12_288, 2, Access::Read)?;
        let weight = view(&partial.arguments[1], 4096 * 12_288, 2, Access::Read)?;
        let temporary = view(&partial.arguments[2], 8 * 4096, 4, Access::Write)?;
        let source = view(&merge.arguments[0], 8 * 4096, 4, Access::Read)?;
        let target = view(&merge.arguments[1], 4096, 4, Access::Write)?;
        let ids = [input, weight, temporary, target];
        if source != temporary
            || scratch.is_some_and(|s| s != temporary)
            || !weights.insert(weight)
            || ids
                .iter()
                .enumerate()
                .any(|(i, id)| ids[i + 1..].contains(id))
        {
            return Err(
                "native688 activation, weight, merge or shared scratch identity changed".into(),
            );
        }
        scratch = Some(temporary);
        activations.insert(input);
        targets.insert(target);
    }
    if commands.iter().enumerate().any(|(index, command)| {
        ROOTS.contains(&command.kernel) != expected_indices.contains(&index)
    }) || scratch
        .is_none_or(|s| targets.contains(&s) || weights.contains(&s) || activations.contains(&s))
        || weights
            .iter()
            .any(|id| targets.contains(id) || activations.contains(id))
        || targets.iter().any(|id| activations.contains(id))
    {
        return Err("native688 extra split-K root or cross-layer alias".into());
    }
    Ok(())
}

impl Worker {
    pub(crate) fn configure_down_shape(&mut self, enabled: bool) -> TpResult<()> {
        if !self.buffers.is_empty()
            || !self.kernels.is_empty()
            || self.queue_packets != 0
            || self.queue_epoch != 0
            || self.pending.is_some()
            || self.failed
            || self.exited
        {
            return self.reject("native down requires its fresh worker entry");
        }
        let state = self
            .token_program
            .as_mut()
            .ok_or("native down backend unavailable")?;
        if state.backend != TokenProgramBackend::NativeWholeProgramSlots512V1
            || state.registered.is_some()
            || state.counter_snapshots != 0
            || state.down_selected.is_some()
            || state.completed_executions != 0
            || state.completed_prefills != 0
            || state.registrations != 0
            || state.releases != 0
        {
            return self.reject("native down selection cannot change after worker initialization");
        }
        state.down_selected = Some(enabled);
        state.prefill_enabled = true;
        state.prefill_rows = 32;
        state.decode_shape = if enabled {
            Shape::DecodeDown688
        } else {
            Shape::Decode
        };
        Ok(())
    }

    #[allow(dead_code, clippy::too_many_arguments)]
    pub(crate) fn spawn_native_down_program(
        executable: &Path,
        unique_id: u64,
        artifact: &EngineeringTpArtifactV1,
        options: RuntimeOptions,
        timing: HostTiming,
        rank: u32,
        enabled: bool,
        counters: bool,
    ) -> TpResult<Self> {
        Self::spawn_entry(
            executable,
            unique_id,
            artifact,
            options,
            timing,
            rank,
            WorkerEntry::NativeDownProgram { enabled, counters },
        )
    }
}
