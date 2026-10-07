//! Explicit native724 decode policy; no packet-count-based mode inference.
use super::{
    ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19, ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21,
    EngineeringTpArgumentV1, EngineeringTpArtifactV1, EngineeringTpDispatchV1, HostTiming, Path,
    RuntimeOptions, Shape, TokenProgramBackend, TpResult, Worker, WorkerEntry,
};
use ferric_m1_engineering_execution_v1::tp_artifact::ENGINEERING_TP_SPLITK_GATE_UP_EXPORTS_R1 as ROOTS;
use ferric_m1_engineering_execution_v1::tp_execution::EngineeringTpBufferAccessV1 as Access;
use std::collections::BTreeSet;

pub(crate) const fn profile(enabled: bool, counters: bool) -> &'static str {
    match (enabled, counters) {
        (false, false) => "prefill32-native649-decode652-gate-up-control-r1",
        (true, false) => "prefill32-native649-decode724-gate-up-splitk4-r1",
        (false, true) => "prefill32-native649-decode652-gate-up-control-counters-r1",
        (true, true) => "prefill32-native649-decode724-gate-up-splitk4-counters-r1",
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
        _ => Err("native724 requires exact short-view ABI".into()),
    }
}

pub(super) fn validate_route(commands: &[EngineeringTpDispatchV1]) -> TpResult<()> {
    if commands.len() != 724 {
        return Err("native724 requires its entire fixed graph".into());
    }
    let mut scratch = None;
    let mut weights = BTreeSet::new();
    let mut targets = BTreeSet::new();
    let mut expected_indices = BTreeSet::new();
    for layer in 0..36 {
        let base = 1 + layer * 20;
        if commands[base + 7].kernel != ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19[0]
            || commands[base + 8].kernel != ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21[0]
        {
            return Err("native724 attention/copy layer ordering changed".into());
        }
        let mut activation = None;
        let mut layer_targets = BTreeSet::new();
        for (tag, offset) in [(4, 13), (5, 15)] {
            let partial = &commands[base + offset];
            let merge = &commands[base + offset + 1];
            expected_indices.extend([base + offset, base + offset + 1]);
            if partial.kernel != ROOTS[0]
                || merge.kernel != ROOTS[1]
                || partial.workgroup_size != 64
                || partial.grid_workgroups != 3072
                || merge.workgroup_size != 64
                || merge.grid_workgroups != 192
                || partial.arguments.len() != 8
                || merge.arguments.len() != 2
                || partial.arguments[3..]
                    != [1, 12_288, 4096, 1, tag].map(EngineeringTpArgumentV1::U32)
            {
                return Err("native724 split-K pair order, role or geometry changed".into());
            }
            let input = view(&partial.arguments[0], 4096, 2, Access::Read)?;
            let weight = view(&partial.arguments[1], 4096 * 12_288, 2, Access::Read)?;
            let temporary = view(&partial.arguments[2], 4 * 12_288, 4, Access::Write)?;
            let source = view(&merge.arguments[0], 4 * 12_288, 4, Access::Read)?;
            let target = view(&merge.arguments[1], 12_288, 2, Access::Write)?;
            let ids = [input, weight, temporary, target];
            if source != temporary
                || scratch.is_some_and(|s| s != temporary)
                || activation.is_some_and(|a| a != input)
                || !weights.insert(weight)
                || !layer_targets.insert(target)
                || ids
                    .iter()
                    .enumerate()
                    .any(|(i, id)| ids[i + 1..].contains(id))
            {
                return Err(
                    "native724 activation, weight, merge or shared scratch identity changed".into(),
                );
            }
            scratch = Some(temporary);
            activation = Some(input);
            targets.insert(target);
        }
    }
    if commands.iter().enumerate().any(|(index, command)| {
        ROOTS.contains(&command.kernel) != expected_indices.contains(&index)
    }) || scratch.is_none_or(|s| targets.contains(&s) || weights.contains(&s))
        || weights.iter().any(|id| targets.contains(id))
    {
        return Err("native724 extra split-K root or cross-layer alias".into());
    }
    Ok(())
}

impl Worker {
    pub(crate) fn configure_gate_up_shape(&mut self, enabled: bool) -> TpResult<()> {
        if !self.buffers.is_empty()
            || !self.kernels.is_empty()
            || self.queue_packets != 0
            || self.queue_epoch != 0
            || self.pending.is_some()
            || self.failed
            || self.exited
        {
            return self.reject("native gate/up requires its fresh worker entry");
        }
        let state = self
            .token_program
            .as_mut()
            .ok_or("native gate/up backend unavailable")?;
        if state.backend != TokenProgramBackend::NativeWholeProgramSlots512V1
            || state.registered.is_some()
            || state.counter_snapshots != 0
            || state.gate_up_selected.is_some()
            || state.completed_executions != 0
            || state.completed_prefills != 0
            || state.registrations != 0
            || state.releases != 0
        {
            return self
                .reject("native gate/up selection cannot change after worker initialization");
        }
        state.gate_up_selected = Some(enabled);
        state.prefill_enabled = true;
        state.prefill_rows = 32;
        state.decode_shape = if enabled {
            Shape::DecodeGateUp724
        } else {
            Shape::Decode
        };
        Ok(())
    }

    #[allow(dead_code, clippy::too_many_arguments)]
    pub(crate) fn spawn_native_gate_up_program(
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
            WorkerEntry::NativeGateUpProgram { enabled, counters },
        )
    }
}
