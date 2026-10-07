//! Decode-only split-K4 over the resident authenticated KN weights.

use super::{
    AuthenticatedModelWeightLayout, EngineeringTpArgumentV1, EngineeringTpBatchExecutionV2,
    EngineeringTpDispatchV1, EngineeringTpPagedPoolV1, EngineeringTpPreparedBatchV1,
    EngineeringTpRankTransportV1, EngineeringTpWaveTargetArtifactsV17, ModelConfig, Qwen3ModelRole,
    Qwen3TensorKind, Rank, Tensor, TpResult, allocate_tensor, dispatch,
};
use crate::tp_artifact::{
    ENGINEERING_TP_SPLITK_GATE_UP_EXPORTS_R1 as ROOTS, EngineeringTpArtifactV1,
    EngineeringTpSplitKGateUpArtifactR1, SplitKGateUpBindingR1,
};
use crate::tp_scheduler::{TpBatchRowKindV1, TpBatchRowV1};

const K_U32: u32 = 4096;
const N_U32: u32 = 12_288;
const K: usize = K_U32 as usize;
const N: usize = N_U32 as usize;
const SCRATCH: usize = 4 * N;

pub(super) fn admit_transports<R: EngineeringTpRankTransportV1>(
    transports: &mut [R],
    binding: SplitKGateUpBindingR1,
) -> TpResult<()> {
    let admitted = (|| {
        if !cfg!(all(
            feature = "c1-token-program",
            not(feature = "model-timestamps")
        )) || transports.len() != 1
            || transports[0].peer_group_rank().is_some()
            || !transports[0].supports_prefill32_program()
            || !transports[0].supports_token_program()
        {
            return Err("split-K gate/up requires the explicit native32 transport".into());
        }
        transports[0].require_loaded_image(binding.hsaco(), &ROOTS)
    })();
    admitted.map_err(|error| {
        let failures = transports
            .iter_mut()
            .enumerate()
            .filter_map(|(rank, transport)| {
                transport
                    .close()
                    .err()
                    .map(|close| format!("rank {rank}: {close}"))
            })
            .collect::<Vec<_>>();
        if failures.is_empty() {
            error
        } else {
            format!(
                "{error}; split-K gate/up admission close {}",
                failures.join("; ")
            )
        }
    })
}

pub(super) struct Workspace {
    pub(super) binding: SplitKGateUpBindingR1,
    pub(super) scratch: Tensor,
    pub(super) selected: Option<bool>,
    batch: Option<BatchBinding>,
}

struct BatchBinding {
    pool: u64,
    batch: u64,
    rows: Vec<(u32, u32, TpBatchRowKindV1)>,
}

impl BatchBinding {
    fn active(&self, batch: &EngineeringTpPreparedBatchV1, published: usize) -> TpResult<bool> {
        if self.pool != batch.pool_identity()
            || self.batch != batch.id()
            || self.rows.len() != batch.rows().len()
            || self
                .rows
                .iter()
                .zip(batch.rows())
                .any(|(&(token, position, _), row)| {
                    token != row.token() || position != row.position()
                })
            || published
                != self
                    .rows
                    .iter()
                    .filter(|row| row.2 != TpBatchRowKindV1::PrefillIntermediate)
                    .count()
        {
            return Err("split-K gate/up scheduler binding changed".into());
        }
        Ok(self.rows.len() == 1 && self.rows[0].2 == TpBatchRowKindV1::Decode)
    }
}

impl Workspace {
    #[cfg(test)]
    pub(super) fn recording(scratch: Tensor) -> Self {
        Self {
            binding: SplitKGateUpBindingR1::recording(),
            scratch,
            selected: None,
            batch: None,
        }
    }

    pub(super) fn commands(
        &self,
        rank: &Rank,
        projection: &super::super::projection::ProjectionPolicy,
        layer: u32,
        tag: u32,
    ) -> TpResult<[EngineeringTpDispatchV1; 2]> {
        if self.selected != Some(true)
            || layer >= 36
            || !matches!(tag, 4 | 5)
            || rank.geometry.rank != 0
            || rank.geometry.intermediate.count != N_U32
            || rank.layers.len() != 36
            || rank.normalized.elements != 32 * K
            || rank.normalized.element_bytes != 2
            || self.scratch.elements != SCRATCH
            || self.scratch.element_bytes != 4
        {
            return Err("split-K gate/up command profile changed".into());
        }
        let kind = if tag == 4 {
            Qwen3TensorKind::GateProjection
        } else {
            Qwen3TensorKind::UpProjection
        };
        let original = rank.layers[layer as usize].weight(kind);
        let weight = projection.splitk_gate_up_weight(0, original)?;
        let output = if tag == 4 { rank.gate } else { rank.up };
        if output.elements != 32 * N || output.element_bytes != 2 {
            return Err("split-K gate/up output capacity changed".into());
        }
        let ids = [
            rank.normalized.id,
            original.id,
            weight.id,
            self.scratch.id,
            output.id,
        ];
        if ids
            .iter()
            .enumerate()
            .any(|(i, id)| *id == 0 || ids[i + 1..].contains(id))
        {
            return Err("split-K gate/up allocation alias".into());
        }
        let mut args = vec![
            Tensor {
                elements: K,
                ..rank.normalized
            }
            .read(),
            weight.read(),
            self.scratch.write(),
        ];
        args.extend([1, N_U32, K_U32, 1, tag].map(EngineeringTpArgumentV1::U32));
        let partial = dispatch(ROOTS[0], 3072, args);
        let merge = dispatch(
            ROOTS[1],
            192,
            vec![
                self.scratch.read(),
                Tensor {
                    elements: N,
                    ..output
                }
                .write(),
            ],
        );
        let bind = |command| super::super::row_profile::bind_mode(false, 32, false, command);
        // Admit both views before either packet can enter the native program.
        Ok([bind(partial)?, bind(merge)?])
    }
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchExecutionV2<R> {
    /// Reports the explicitly configured arm and its fixed resident scratch bytes.
    #[must_use]
    pub fn native_splitk_gate_up_state_r1(&self) -> Option<(bool, u64)> {
        let workspace = self.splitk_gate_up_r1.as_ref()?;
        Some((
            workspace.selected?,
            (workspace.scratch.elements * workspace.scratch.element_bytes as usize) as u64,
        ))
    }

    /// Retains the same tenth image and scratch in both experimental arms.
    /// # Errors
    /// Rejects nonnative transports or image drift and closes failed setup.
    #[allow(clippy::too_many_arguments)]
    pub fn new_wide32_with_native_splitk_gate_up_r1(
        mut transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        pool: &EngineeringTpPagedPoolV1,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        prefill: &EngineeringTpArtifactV1,
        split: &EngineeringTpArtifactV1,
        gemv: &EngineeringTpArtifactV1,
        copy: &EngineeringTpArtifactV1,
        candidate: &EngineeringTpSplitKGateUpArtifactR1,
    ) -> TpResult<Self> {
        let binding = candidate.binding();
        admit_transports(&mut transports, binding)?;
        let mut driver = Self::new_wide32_with_ordered64_kv_copy_v1(
            transports, model, weights, layout, pool, artifacts, prefill, split, gemv, copy,
        )?;
        let scratch = allocate_tensor(&mut driver.inner.transports[0], SCRATCH, 4);
        match scratch {
            Ok(scratch) => {
                driver.splitk_gate_up_r1 = Some(Workspace {
                    binding,
                    scratch,
                    selected: None,
                    batch: None,
                });
                if let Err(error) = driver.validate_splitk_scratch() {
                    driver.poisoned = true;
                    return Err(match driver.close() {
                        Ok(()) => error,
                        Err(close) => format!("{error}; close: {close}"),
                    });
                }
            }
            Err(error) => {
                driver.poisoned = true;
                return Err(match driver.close() {
                    Ok(()) => error,
                    Err(close) => format!("{error}; close: {close}"),
                });
            }
        }
        Ok(driver)
    }

    fn validate_splitk_scratch(&self) -> TpResult<()> {
        let scratch = self
            .splitk_gate_up_r1
            .as_ref()
            .ok_or("split-K scratch absent")?
            .scratch;
        if scratch.id == 0
            || scratch.elements != SCRATCH
            || scratch.element_bytes != 4
            || self.projection.owns_transposed_id(scratch.id)
            || self
                .positions
                .iter()
                .chain(&self.page_tables)
                .any(|t| t.id == scratch.id)
            || self.fp32_logits.is_some_and(|t| t.id == scratch.id)
            || self
                .split_attention_workspace_v25
                .is_some_and(|s| s.stats.id == scratch.id || s.numerators.id == scratch.id)
            || matches!(self.inner.reduction, super::super::reduction::ReductionWorkspace::DeviceTp1(t) if t.id == scratch.id)
            || self.inner.ranks.iter().any(|r| {
                [
                    r.hidden,
                    r.normalized,
                    r.q,
                    r.k,
                    r.v,
                    r.q_normalized,
                    r.k_normalized,
                    r.q_rotated,
                    r.k_rotated,
                    r.attention,
                    r.gate,
                    r.up,
                    r.activation,
                    r.partial,
                    r.cos,
                    r.sin,
                    r.empty,
                    r.token,
                    r.logits,
                    r.choice,
                ]
                .iter()
                .any(|t| t.id == scratch.id)
                    || r.globals.iter().any(|(_, t)| t.id == scratch.id)
                    || r.layers.iter().any(|l| {
                        l.k_cache.id == scratch.id
                            || l.v_cache.id == scratch.id
                            || l.weights.iter().any(|(_, t)| t.id == scratch.id)
                    })
            })
        {
            return Err("split-K gate/up scratch is not a distinct fixed allocation".into());
        }
        Ok(())
    }

    /// Selects the decode arm after configuring unchanged native32 prefill.
    /// # Errors
    /// Rejects late/repeated selection, other profiles, or missing authenticated weights.
    pub fn configure_native_splitk_gate_up_r1(
        &mut self,
        artifact: &EngineeringTpSplitKGateUpArtifactR1,
        enabled: bool,
    ) -> TpResult<()> {
        self.configure_splitk_gate_up_binding(artifact.binding(), enabled)
    }

    pub(super) fn configure_splitk_gate_up_binding(
        &mut self,
        binding: SplitKGateUpBindingR1,
        enabled: bool,
    ) -> TpResult<()> {
        self.validate_prefill_profile_v1(true, true)?;
        if !cfg!(all(
            feature = "c1-token-program",
            not(feature = "model-timestamps")
        )) || self.last_batch != 0
            || self.completed_batches != 0
            || self.poisoned
            || self.inner.closed
            || self.inner.plan.model().role != Qwen3ModelRole::Target8B
            || self.inner.plan.model().layers != 36
            || self.inner.plan.model().hidden_size != K_U32
            || self.row_capacity != 32
            || self.inner.plan.world_size() != 1
            || self.inner.ranks.len() != 1
            || self.inner.transports.len() != 1
            || self.inner.ranks[0].dispatches != 0
            || self.inner.timing.is_enabled()
            || self.numerical.is_some()
            || self.inner.residual_arithmetic.is_some()
            || self.prefill32_pages_v1 != Some(true)
            || self.c1_kv_copy_v19.is_none()
            || self.c1_split_attention_v25 != Some(true)
            || self.c1_packet_packing_v22 != Some(true)
            || self.packed_gate_up_r2.is_some()
            || self.packed_down_r1.is_some()
            || self
                .splitk_gate_up_r1
                .as_ref()
                .is_none_or(|w| w.binding != binding || w.selected.is_some() || w.batch.is_some())
        {
            return Err("split-K gate/up requires its fresh native32 composition".into());
        }
        self.validate_splitk_scratch()?;
        for layer in &self.inner.ranks[0].layers {
            for kind in [
                Qwen3TensorKind::GateProjection,
                Qwen3TensorKind::UpProjection,
            ] {
                self.projection
                    .splitk_gate_up_weight(0, layer.weight(kind))?;
            }
        }
        self.splitk_gate_up_r1
            .as_mut()
            .ok_or("split-K workspace absent")?
            .selected = Some(enabled);
        Ok(())
    }

    pub(super) fn bind_splitk_rows(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        rows: &[TpBatchRowV1],
    ) -> TpResult<()> {
        if self.splitk_gate_up_r1.is_none() {
            return Ok(());
        }
        if self
            .splitk_gate_up_r1
            .as_mut()
            .and_then(|w| w.batch.take())
            .is_some()
        {
            return Err("split-K gate/up duplicate scheduler binding".into());
        }
        self.validate(batch)?;
        self.validate_prefill_profile_v1(true, true)?;
        if !rows.is_empty() && rows[0].kind != TpBatchRowKindV1::Decode {
            let outputs = rows
                .iter()
                .enumerate()
                .filter_map(|(index, row)| {
                    (row.kind != TpBatchRowKindV1::PrefillIntermediate).then_some(index)
                })
                .collect::<Vec<_>>();
            if self.prefill32_plan_v1(batch, &outputs)?.is_none() {
                return Err(
                    "split-K gate/up prefill requires an actual native page-pair plan".into(),
                );
            }
        }
        let w = self
            .splitk_gate_up_r1
            .as_mut()
            .ok_or("split-K workspace absent")?;
        if w.selected.is_none()
            || rows.is_empty()
            || rows.len() != batch.rows().len()
            || rows.iter().any(|r| r.request != rows[0].request)
            || batch
                .rows()
                .iter()
                .any(|r| r.sequence() != batch.rows()[0].sequence())
            || rows
                .iter()
                .zip(batch.rows())
                .any(|(s, p)| s.token_id != p.token() || s.absolute_position != p.position())
            || (rows.iter().any(|r| r.kind == TpBatchRowKindV1::Decode)
                && (rows.len() != 1 || !(128..=255).contains(&rows[0].absolute_position)))
            || (rows[0].kind != TpBatchRowKindV1::Decode && rows.len() != 32)
        {
            return Err("split-K gate/up requires exact native32 prefill or C1 decode rows".into());
        }
        w.batch = Some(BatchBinding {
            pool: batch.pool_identity(),
            batch: batch.id(),
            rows: rows
                .iter()
                .map(|r| (r.token_id, r.absolute_position, r.kind))
                .collect(),
        });
        Ok(())
    }

    pub(super) fn abandon_splitk_rows(&mut self) {
        if let Some(w) = &mut self.splitk_gate_up_r1 {
            w.batch = None;
        }
    }

    pub(super) fn splitk_gate_up_active_for_batch(
        &self,
        batch: &EngineeringTpPreparedBatchV1,
        published: usize,
    ) -> TpResult<bool> {
        let Some(w) = &self.splitk_gate_up_r1 else {
            return Ok(false);
        };
        let active = w
            .batch
            .as_ref()
            .ok_or("split-K gate/up requires scheduler binding")?
            .active(batch, published)?;
        Ok(w.selected == Some(true) && active)
    }

    pub(super) fn take_splitk_gate_up_selection(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        outputs: &[usize],
    ) -> TpResult<bool> {
        let Some(w) = &mut self.splitk_gate_up_r1 else {
            return Ok(false);
        };
        let binding = w
            .batch
            .take()
            .ok_or("split-K gate/up requires fresh scheduler binding")?;
        let active = binding.active(batch, outputs.len())?;
        let expected = binding
            .rows
            .iter()
            .enumerate()
            .filter_map(|(i, r)| (r.2 != TpBatchRowKindV1::PrefillIntermediate).then_some(i))
            .collect::<Vec<_>>();
        if outputs != expected {
            return Err("split-K gate/up output selection differs from scheduler".into());
        }
        Ok(w.selected == Some(true) && active)
    }
}
