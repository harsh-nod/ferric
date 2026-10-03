//! Immutable baseline TP2 recording and one post-completion host-state commit.
//! This interpreter removes IPC round trips; it does not imply GPU overlap.

use super::super::{
    EngineeringTp2CollectiveRequestV1, EngineeringTp2GraphGeometryV1 as Geometry,
    EngineeringTp2GraphInputV1 as GraphInput, EngineeringTp2GraphKernelProfileV1 as GraphKernels,
    EngineeringTp2GraphPolicyV1 as GraphPolicy, EngineeringTp2PreparedMetadataV1 as Metadata,
    EngineeringTp2PreparedProgramV1 as Program, EngineeringTp2PreparedStepV1 as Step,
    EngineeringTpArgumentV1::U32, EngineeringTpDispatchV1, EngineeringTpProjectionModeV3,
    ReductionWorkspace,
};
use super::{
    APPEND, ARGMAX, ATTENTION, EMBEDDING, EngineeringTpBatchExecutionV2,
    EngineeringTpPreparedBatchV1, EngineeringTpRankTransportV1, EngineeringTpReductionModeV3, GEMM,
    PARTIAL, Qwen3TensorKind, Qwen3TensorParallelCollectiveV1, ROPE, Rank, SWIGLU, Tensor,
    TpResult, dispatch, norm, rope_bytes,
};
use ferric_engine::tensor_parallel::Qwen3TensorParallelCollectiveStateV1;

pub(super) mod finite_composition;
pub(super) mod finite_packing;
pub(super) mod finite_source_recorder;

pub(super) struct State {
    program: Program,
    plan_sha256: [u8; 32],
    graph_policy: Option<GraphPolicy>,
    kernel_profile: GraphKernels,
    geometry: Geometry,
}

impl State {
    pub(super) fn reuses_completed_ring(&self) -> bool {
        self.graph_policy.is_some() && self.geometry == Geometry::Long2304
    }

    pub(super) fn max_packets(&self) -> u64 {
        self.kernel_profile.graph_packet_counts()[0]
    }

    pub(super) fn split_attention(&self) -> bool {
        self.kernel_profile.split_attention()
    }

    pub(super) fn kernel_counts(&self) -> [u32; 2] {
        self.kernel_profile.kernel_counts()
    }
}

fn rank_dispatch(
    program: &mut Program,
    index: usize,
    rank: u32,
) -> TpResult<&mut EngineeringTpDispatchV1> {
    match program.steps.get_mut(index) {
        Some(Step::Rank {
            rank: actual,
            dispatch,
        }) if *actual == rank => Ok(dispatch),
        _ => Err("wave-stack exact rank/site mismatch".into()),
    }
}

fn push_each(
    steps: &mut Vec<Step>,
    ranks: &[Rank; 2],
    make: impl Fn(&Rank) -> EngineeringTpDispatchV1,
) {
    for rank in ranks {
        steps.push(Step::Rank {
            rank: rank.geometry.rank,
            dispatch: make(rank),
        });
    }
}

fn advance(cursor: &mut Qwen3TensorParallelCollectiveStateV1) -> TpResult<()> {
    let key = cursor.expected();
    for rank in 0..2 {
        cursor
            .arrive(rank, key)
            .map_err(|error| format!("prepared arrival: {error:?}"))?;
    }
    cursor
        .advance()
        .map_err(|error| format!("prepared advance: {error:?}"))
}

fn record_collective(
    steps: &mut Vec<Step>,
    ranks: &mut [Rank; 2],
    scratch: &mut [Tensor; 2],
    cursor: &mut Qwen3TensorParallelCollectiveStateV1,
    producers: [EngineeringTpDispatchV1; 2],
) -> TpResult<()> {
    let partials = ranks.each_ref().map(|rank| Tensor {
        elements: 4096,
        ..rank.partial
    });
    let consumers = std::array::from_fn(|index| {
        let mut arguments = (0..8)
            .map(|peer| {
                partials
                    .get(peer)
                    .copied()
                    .unwrap_or(Tensor {
                        elements: 0,
                        ..partials[0]
                    })
                    .read()
            })
            .collect::<Vec<_>>();
        arguments.extend([
            Tensor {
                elements: 4096,
                ..ranks[index].hidden
            }
            .read(),
            Tensor {
                elements: 4096,
                ..scratch[index]
            }
            .write(),
            U32(1),
            U32(2),
        ]);
        dispatch(super::super::peer_dependency::CONSUMER, 64, arguments)
    });
    let request = EngineeringTp2CollectiveRequestV1 {
        key: cursor.expected(),
        rows: 1,
        producers,
        consumers,
    };
    request.validate()?;
    advance(cursor)?;
    steps.push(Step::Collective(request));
    for (rank, output) in ranks.iter_mut().zip(scratch) {
        std::mem::swap(&mut rank.hidden, output);
    }
    Ok(())
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchExecutionV2<R> {
    fn check_prepared_profile(
        &self,
        graph_policy: Option<GraphPolicy>,
        kernel_profile: GraphKernels,
        geometry: Geometry,
    ) -> TpResult<()> {
        self.inner.check_peer_dependency_profile()?;
        self.check_prepared_source_profile(graph_policy, kernel_profile, geometry)?;
        if self.inner.transports.iter().any(|rank| match graph_policy {
            None => !rank.supports_prepared_peer(),
            Some(policy) => {
                !rank.supports_prepared_peer_graph_geometry(policy, kernel_profile, geometry)
            }
        }) {
            return Err("prepared TP2 requires the exact baseline BF16 row1/cap16 profile".into());
        }
        Ok(())
    }

    fn check_prepared_source_profile(
        &self,
        graph_policy: Option<GraphPolicy>,
        kernel_profile: GraphKernels,
        geometry: Geometry,
    ) -> TpResult<()> {
        let model = self.inner.plan.model();
        if self.poisoned
            || graph_policy.is_none() && kernel_profile != GraphKernels::Baseline
            || graph_policy.is_none() && geometry != Geometry::Short64
            || self.inner.closed
            || self.reduction_mode() != EngineeringTpReductionModeV3::DevicePeerDependencyV1
            || !self.projection_configured
            || self.projection.mode != EngineeringTpProjectionModeV3::Mfma
            || self.row_capacity != 16
            || self.context_tokens != geometry.context_tokens()
            || self.physical_pages != geometry.pages()
            || self.table_stride != geometry.pages()
            || model.vocabulary_size != 151_936
            || model.hidden_size != 4096
            || model.layers != 36
            || model.intermediate_size != 12_288
            || self.prune_output_head
            || self.c1_wave_layers
            || self.wave_attention
            || self.numerical.is_some()
            || self.fp32_logits.is_some()
            || self.fp32_argmax_v11.is_some()
            || self.admitted_argmax_v11.is_some()
            || self.query_hoist_v14.is_some()
            || self.admitted_query_hoist_v14.is_some()
            || self.wave_rmsnorm_v15.is_some()
            || self.admitted_wave_rmsnorm_v15.is_some()
            || self.parallel_kv_v16.is_some()
            || self.admitted_parallel_kv_v16.is_some()
        {
            return Err("prepared TP2 requires the exact baseline BF16 row1/cap16 profile".into());
        }
        Ok(())
    }

    /// Records and registers one immutable baseline graph after resident setup.
    /// No kernel is submitted and no live decode counter is advanced here.
    /// # Errors
    /// Rejects incompatible profiles, nonfresh state or failed registration.
    pub fn configure_prepared_peer(&mut self) -> TpResult<()> {
        self.configure_prepared_peer_mode(None, GraphKernels::Baseline)
    }

    /// Registers a distinct jointly drained graph with an immutable native policy.
    /// No kernel is submitted or live decode state advanced during registration.
    /// # Errors
    /// Rejects unsupported transports, incompatible profiles or nonfresh state.
    pub fn configure_prepared_peer_graph(&mut self, policy: GraphPolicy) -> TpResult<()> {
        self.configure_prepared_peer_mode(Some(policy), GraphKernels::Baseline)
    }

    /// Registers the matched graph-only V22 scalar/Wave ablation after ordinary admission.
    /// All other shapes, kernels, precision and host commit boundaries remain unchanged.
    /// # Errors
    /// Rejects a foreign image, baseline selection, unsupported transport or nonfresh state.
    pub fn configure_prepared_peer_graph_with_argmax_v22(
        &mut self,
        policy: GraphPolicy,
        kernel_profile: GraphKernels,
        artifact: &crate::tp_artifact::EngineeringTpArtifactV1,
    ) -> TpResult<()> {
        if !kernel_profile.has_v22()
            || kernel_profile.has_v15()
            || !artifact.is_graph_bf16_argmax_v22()
        {
            return Err("graph V22 selection requires its exact ordinarily admitted image".into());
        }
        self.configure_prepared_peer_mode(Some(policy), kernel_profile)
    }

    /// Registers a matched wave-stack graph after ordinary admission of both sidecars.
    /// No additional allocation or layout conversion occurs during registration.
    /// # Errors
    /// Rejects another profile, foreign images, nonfresh state or failed registration.
    pub fn configure_prepared_peer_graph_with_wave_stack(
        &mut self,
        policy: GraphPolicy,
        kernel_profile: GraphKernels,
        argmax: &crate::tp_artifact::EngineeringTpArtifactV1,
        norm: &crate::tp_artifact::EngineeringTpArtifactV1,
    ) -> TpResult<()> {
        if !kernel_profile.has_v15()
            || kernel_profile.split_attention()
            || !argmax.is_graph_bf16_argmax_v22()
            || !norm.is_graph_wave_rmsnorm_v15()
        {
            return Err("wave-stack requires both exact ordinarily admitted sidecars".into());
        }
        self.configure_prepared_peer_mode(Some(policy), kernel_profile)
    }

    fn configure_prepared_peer_mode(
        &mut self,
        graph_policy: Option<GraphPolicy>,
        kernel_profile: GraphKernels,
    ) -> TpResult<()> {
        self.configure_prepared_peer_geometry_mode(graph_policy, kernel_profile, Geometry::Short64)
    }

    /// Registers the explicit graph geometry after exact matching sidecar admission.
    /// Existing public wrappers continue to select only the short geometry.
    /// # Errors
    /// Rejects mismatched geometry, sidecars, transports, or nonfresh state.
    pub fn configure_prepared_peer_graph_geometry(
        &mut self,
        policy: GraphPolicy,
        profile: GraphKernels,
        geometry: Geometry,
        argmax: Option<&crate::tp_artifact::EngineeringTpArtifactV1>,
        norm: Option<&crate::tp_artifact::EngineeringTpArtifactV1>,
    ) -> TpResult<()> {
        if profile.split_attention()
            || profile.has_v22() != argmax.is_some()
            || profile.has_v15() != norm.is_some()
            || argmax.is_some_and(|image| !image.is_graph_bf16_argmax_v22())
            || norm.is_some_and(|image| !image.is_graph_wave_rmsnorm_v15())
        {
            return Err("graph geometry/profile sidecar admission mismatch".into());
        }
        self.configure_prepared_peer_geometry_mode(Some(policy), profile, geometry)
    }

    /// Allocates private rank-local split scratch after ordinary sidecar admission.
    /// The old graph configuration entry points cannot select this profile.
    /// # Errors
    /// Rejects nonfresh state or foreign sidecars; mutation failure is terminal.
    pub fn configure_prepared_peer_graph_split_attention(
        &mut self,
        policy: GraphPolicy,
        geometry: Geometry,
        argmax: &crate::tp_artifact::EngineeringTpArtifactV1,
        norm: &crate::tp_artifact::EngineeringTpArtifactV1,
        split: &crate::tp_artifact::EngineeringTpArtifactV1,
    ) -> TpResult<()> {
        if !argmax.is_graph_bf16_argmax_v22()
            || !norm.is_graph_wave_rmsnorm_v15()
            || !split.is_graph_split_attention_v1()
        {
            return Err("split attention requires three exact ordinarily admitted sidecars".into());
        }
        self.configure_split_attention_state(policy, geometry)
    }

    fn configure_split_attention_state(
        &mut self,
        policy: GraphPolicy,
        geometry: Geometry,
    ) -> TpResult<()> {
        let profile = GraphKernels::WaveStackNormSplitAttentionKvMlp;
        self.check_prepared_profile(Some(policy), profile, geometry)?;
        if self.prepared_peer.is_some()
            || self.split_attention_scratch.is_some()
            || self.completed_batches != 0
            || self.last_batch != 0
            || self.inner.ranks.iter().any(|rank| rank.dispatches != 0)
        {
            return Err("split attention requires fresh resident state".into());
        }
        self.poisoned = true;
        let result = (|| {
            self.allocate_split_attention_scratch(geometry)?;
            let program = self.record_prepared_program_with_geometry(profile, geometry)?;
            let plan_sha256 = self.inner.transports[0]
                .register_prepared_peer_graph_geometry(&program, policy, geometry)?;
            self.prepared_peer = Some(State {
                program,
                plan_sha256,
                graph_policy: Some(policy),
                kernel_profile: profile,
                geometry,
            });
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.poisoned = false;
                Ok(())
            }
            Err(error) => Err(match self.inner.close() {
                Ok(()) => error,
                Err(close) => format!("{error}; split registration close: {close}"),
            }),
        }
    }

    fn allocate_split_attention_scratch(&mut self, geometry: Geometry) -> TpResult<()> {
        let elements = usize::try_from(geometry.attention_scratch_values())
            .map_err(|_| "split scratch extent overflow")?;
        let first = super::super::allocate_tensor(&mut self.inner.transports[0], elements, 4)?;
        let second = super::super::allocate_tensor(&mut self.inner.transports[1], elements, 4)?;
        self.split_attention_scratch = Some([first, second]);
        Ok(())
    }

    fn configure_prepared_peer_geometry_mode(
        &mut self,
        graph_policy: Option<GraphPolicy>,
        kernel_profile: GraphKernels,
        geometry: Geometry,
    ) -> TpResult<()> {
        self.check_prepared_profile(graph_policy, kernel_profile, geometry)?;
        if self.prepared_peer.is_some()
            || self.completed_batches != 0
            || self.last_batch != 0
            || self.inner.ranks.iter().any(|rank| rank.dispatches != 0)
        {
            return Err("prepared TP2 registration requires fresh resident state".into());
        }
        let program = self.record_prepared_program_with_geometry(kernel_profile, geometry)?;
        // Registration can seal a transport before it unwinds. Keep the model
        // unusable until the immutable graph state has been installed.
        if graph_policy.is_some() {
            self.poisoned = true;
        }
        let registration = match graph_policy {
            None => self.inner.transports[0].register_prepared_peer(&program),
            Some(policy) => self.inner.transports[0]
                .register_prepared_peer_graph_geometry(&program, policy, geometry),
        };
        match registration {
            Ok(plan_sha256) => {
                self.prepared_peer = Some(State {
                    program,
                    plan_sha256,
                    graph_policy,
                    kernel_profile,
                    geometry,
                });
                if graph_policy.is_some() {
                    self.poisoned = false;
                }
                Ok(())
            }
            Err(error) => {
                self.poisoned = true;
                Err(match self.inner.close() {
                    Ok(()) => error,
                    Err(close) => format!("{error}; prepared registration close: {close}"),
                })
            }
        }
    }

    fn record_prepared_program_with_geometry(
        &self,
        profile: GraphKernels,
        geometry: Geometry,
    ) -> TpResult<Program> {
        let mut program = self.record_prepared_program(geometry)?;
        if profile.wave_argmax() {
            if program.steps.len() != 1013 {
                return Err("graph argmax baseline cardinality drift".into());
            }
            let Some(Step::Rank { rank: 0, dispatch }) = program.steps.last_mut() else {
                return Err("graph argmax requires the baseline final rank-zero descriptor".into());
            };
            if dispatch.kernel != ARGMAX {
                return Err("graph argmax baseline root drift".into());
            }
            dispatch.kernel = crate::tp_artifact::ENGINEERING_TP_GRAPH_ARGMAX_ROOT_V22;
        }
        if profile.has_v15() {
            self.record_wave_stack(
                &mut program,
                if profile.split_attention() {
                    GraphKernels::WaveStackNormAttentionKvMlp
                } else {
                    profile
                },
            )?;
        }
        if profile.split_attention() {
            self.record_split_attention(&mut program, geometry)?;
        }
        Ok(program)
    }

    pub(super) fn record_split_attention(
        &self,
        program: &mut Program,
        geometry: Geometry,
    ) -> TpResult<()> {
        let scratch = self
            .split_attention_scratch
            .ok_or("split scratch not allocated")?;
        if program.steps.len() != 1013 {
            return Err("split transform baseline cardinality drift".into());
        }
        let roots = crate::tp_artifact::ENGINEERING_TP_GRAPH_SPLIT_ATTENTION_ROOTS_V1;
        let mut output = Vec::with_capacity(1085);
        let mut cursor = 0;
        for layer in 0..36 {
            let site = 2 + 28 * layer + 16;
            output.extend_from_slice(&program.steps[cursor..site]);
            let mut partials = Vec::with_capacity(2);
            let mut merges = Vec::with_capacity(2);
            for (index, rank) in self.inner.ranks.iter().enumerate() {
                let Some(Step::Rank {
                    rank: actual,
                    dispatch: old,
                }) = program.steps.get(site + index)
                else {
                    return Err("split transform attention site mismatch".into());
                };
                let expected = dispatch(
                    "ferric_qwen3_tp_wave_paged_gqa_bf16_v3",
                    16,
                    vec![
                        rank.q_rotated.read(),
                        rank.layers[layer].k_cache.read(),
                        rank.layers[layer].v_cache.read(),
                        self.positions[index].read(),
                        self.page_tables[index].read(),
                        rank.attention.write(),
                        U32(1),
                        U32(2),
                        U32(geometry.pages()),
                        U32(geometry.pages()),
                        U32(1),
                    ],
                );
                if *actual != rank.geometry.rank
                    || *old != expected
                    || scratch[index].element_bytes != 4
                    || u64::try_from(scratch[index].elements).ok()
                        != Some(geometry.attention_scratch_values())
                {
                    return Err("split transform exact attention ABI/role/shape drift".into());
                }
                let mut arguments = old.arguments.clone();
                arguments[5] = scratch[index].write();
                partials.push(Step::Rank {
                    rank: *actual,
                    dispatch: dispatch(roots[0], 16 * geometry.attention_splits(), arguments),
                });
                merges.push(Step::Rank {
                    rank: *actual,
                    dispatch: dispatch(
                        roots[1],
                        16,
                        vec![
                            scratch[index].read(),
                            self.positions[index].read(),
                            rank.attention.write(),
                            U32(1),
                            U32(2),
                            U32(geometry.pages()),
                            U32(1),
                        ],
                    ),
                });
            }
            output.extend(partials);
            output.extend(merges);
            cursor = site + 2;
        }
        output.extend_from_slice(&program.steps[cursor..]);
        if output.len() != GraphKernels::WaveStackNormSplitAttentionKvMlp.steps() {
            return Err("split transform output cardinality drift".into());
        }
        program.steps = output;
        Ok(())
    }

    fn record_wave_stack(&self, program: &mut Program, profile: GraphKernels) -> TpResult<()> {
        for layer in 0..36 {
            let first = 2 + 28 * layer;
            for rank in &self.inner.ranks {
                let index =
                    usize::try_from(rank.geometry.rank).map_err(|_| "wave-stack rank bound")?;
                if profile.wave_hidden_norm() {
                    for offset in [0, 19] {
                        rank_dispatch(program, first + offset + index, rank.geometry.rank)?
                            .kernel =
                            crate::tp_artifact::ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15[0];
                    }
                }
                if profile.wave_attention() {
                    // The ABI and context scalar offset stay unchanged; the worker patches both roots.
                    rank_dispatch(program, first + 16 + index, rank.geometry.rank)?.kernel =
                        "ferric_qwen3_tp_wave_paged_gqa_bf16_v3";
                }
                if profile.wave_kv() {
                    for (offset, kind, output) in [
                        (4, Qwen3TensorKind::KeyProjection, rank.k),
                        (6, Qwen3TensorKind::ValueProjection, rank.v),
                    ] {
                        let target =
                            rank_dispatch(program, first + offset + index, rank.geometry.rank)?;
                        *target = self.projection.rowone_graph_replacement(
                            index,
                            kind,
                            rank.normalized,
                            rank.layers[layer].weight(kind),
                            output,
                            target,
                        )?;
                    }
                }
                if profile.wave_mlp() {
                    // Gate/up remain separately rounded BF16 tensors before unchanged SwiGLU.
                    for (offset, kind, output) in [
                        (21, Qwen3TensorKind::GateProjection, rank.gate),
                        (23, Qwen3TensorKind::UpProjection, rank.up),
                    ] {
                        let target =
                            rank_dispatch(program, first + offset + index, rank.geometry.rank)?;
                        *target = self.projection.rowone_graph_replacement(
                            index,
                            kind,
                            rank.normalized,
                            rank.layers[layer].weight(kind),
                            output,
                            target,
                        )?;
                    }
                }
            }
            if profile.wave_mlp() {
                let Some(Step::Collective(down)) = program.steps.get_mut(first + 27) else {
                    return Err("wave-stack down collective site mismatch".into());
                };
                if down.key.layer != u32::try_from(layer).map_err(|_| "wave-stack layer bound")?
                    || down.key.operation != Qwen3TensorParallelCollectiveV1::FeedForwardDownSum
                {
                    return Err("wave-stack down collective identity mismatch".into());
                }
                // Wave partials stay FP32; rank-ordered residual consumers are untouched.
                for (index, rank) in self.inner.ranks.iter().enumerate() {
                    down.producers[index] = self.projection.rowone_graph_replacement(
                        index,
                        Qwen3TensorKind::DownProjection,
                        rank.activation,
                        rank.layers[layer].weight(Qwen3TensorKind::DownProjection),
                        rank.partial,
                        &down.producers[index],
                    )?;
                }
                down.validate()?;
            }
        }
        if profile.wave_hidden_norm() {
            rank_dispatch(program, 1010, 0)?.kernel =
                crate::tp_artifact::ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15[0];
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn configure_split_geometry_fixture(
        &mut self,
        policy: GraphPolicy,
        geometry: Geometry,
    ) -> TpResult<()> {
        self.configure_split_attention_state(policy, geometry)
    }

    #[cfg(test)]
    pub(super) fn record_split_geometry_fixture(
        &mut self,
        geometry: Geometry,
    ) -> TpResult<Program> {
        if self.split_attention_scratch.is_none() {
            self.allocate_split_attention_scratch(geometry)?;
        }
        self.record_prepared_program_with_geometry(
            GraphKernels::WaveStackNormSplitAttentionKvMlp,
            geometry,
        )
    }

    #[cfg(test)]
    pub(super) fn record_graph_kernel_profile_fixture(
        &self,
        profile: GraphKernels,
    ) -> TpResult<Program> {
        self.record_prepared_program_with_geometry(profile, Geometry::Short64)
    }

    #[cfg(test)]
    pub(super) fn record_graph_geometry_fixture(
        &self,
        profile: GraphKernels,
        geometry: Geometry,
    ) -> TpResult<Program> {
        self.record_prepared_program_with_geometry(profile, geometry)
    }

    fn record_prepared_program(&self, geometry: Geometry) -> TpResult<Program> {
        let ReductionWorkspace::DevicePeer(live_scratch, _) = &self.inner.reduction else {
            return Err("prepared peer scratch missing".into());
        };
        let mut ranks: [Rank; 2] = self
            .inner
            .ranks
            .clone()
            .try_into()
            .map_err(|_| "prepared rank cardinality")?;
        let mut scratch: [Tensor; 2] = live_scratch
            .clone()
            .try_into()
            .map_err(|_| "prepared scratch cardinality")?;
        let initial_hidden = ranks.each_ref().map(|rank| rank.hidden);
        let initial_scratch = scratch;
        let mut cursor = self.inner.collective;
        let initial = cursor.expected();
        if initial.epoch != 0
            || initial.layer != 0
            || initial.operation != Qwen3TensorParallelCollectiveV1::AttentionOutputSum
        {
            return Err("prepared recording requires initial collective cursor".into());
        }
        let mut steps = Vec::with_capacity(1013);
        let zero = &ranks[0];
        steps.push(Step::Rank {
            rank: 0,
            dispatch: dispatch(
                EMBEDDING,
                64,
                vec![
                    zero.token.read(),
                    zero.global(Qwen3TensorKind::TokenEmbedding).read(),
                    zero.hidden.write(),
                    U32(1),
                ],
            ),
        });
        steps.push(Step::Rank {
            rank: 1,
            dispatch: dispatch(
                "ferric_qwen3_tp_peer_copy_bf16_v4",
                64,
                vec![
                    Tensor {
                        elements: 4096,
                        ..zero.hidden
                    }
                    .read(),
                    Tensor {
                        elements: 4096,
                        ..ranks[1].hidden
                    }
                    .write(),
                    U32(1),
                ],
            ),
        });
        for layer in 0..36 {
            push_each(&mut steps, &ranks, |rank| {
                norm(
                    rank,
                    rank.hidden,
                    rank.layers[layer].weight(Qwen3TensorKind::InputLayerNorm),
                    rank.normalized,
                    1,
                    4096,
                )
            });
            for (kind, tag) in [
                (Qwen3TensorKind::QueryProjection, 1),
                (Qwen3TensorKind::KeyProjection, 2),
                (Qwen3TensorKind::ValueProjection, 3),
            ] {
                push_each(&mut steps, &ranks, |rank| {
                    let (output, n) = match tag {
                        1 => (rank.q, 2048),
                        2 => (rank.k, 512),
                        _ => (rank.v, 512),
                    };
                    self.projection.command(
                        rank.geometry.rank as usize,
                        GEMM,
                        rank.normalized,
                        rank.layers[layer].weight(kind),
                        output,
                        [1, n, 4096, 2, tag],
                    )
                });
            }
            push_each(&mut steps, &ranks, |rank| {
                norm(
                    rank,
                    rank.q,
                    rank.layers[layer].weight(Qwen3TensorKind::QueryNorm),
                    rank.q_normalized,
                    16,
                    128,
                )
            });
            push_each(&mut steps, &ranks, |rank| {
                norm(
                    rank,
                    rank.k,
                    rank.layers[layer].weight(Qwen3TensorKind::KeyNorm),
                    rank.k_normalized,
                    4,
                    128,
                )
            });
            push_each(&mut steps, &ranks, |rank| {
                dispatch(
                    ROPE,
                    1,
                    vec![
                        rank.q_normalized.read(),
                        rank.k_normalized.read(),
                        rank.cos.read(),
                        rank.sin.read(),
                        self.positions[rank.geometry.rank as usize].read(),
                        rank.q_rotated.write(),
                        rank.k_rotated.write(),
                        U32(1),
                        U32(2),
                    ],
                )
            });
            push_each(&mut steps, &ranks, |rank| {
                dispatch(
                    APPEND,
                    1,
                    vec![
                        rank.k_rotated.read(),
                        rank.v.read(),
                        self.positions[rank.geometry.rank as usize].read(),
                        self.page_tables[rank.geometry.rank as usize].read(),
                        rank.layers[layer].k_cache.write(),
                        rank.layers[layer].v_cache.write(),
                        U32(1),
                        U32(2),
                        U32(geometry.pages()),
                        U32(geometry.pages()),
                    ],
                )
            });
            push_each(&mut steps, &ranks, |rank| {
                dispatch(
                    ATTENTION,
                    16,
                    vec![
                        rank.q_rotated.read(),
                        rank.layers[layer].k_cache.read(),
                        rank.layers[layer].v_cache.read(),
                        self.positions[rank.geometry.rank as usize].read(),
                        self.page_tables[rank.geometry.rank as usize].read(),
                        rank.attention.write(),
                        U32(1),
                        U32(2),
                        U32(geometry.pages()),
                        U32(geometry.pages()),
                        U32(1),
                    ],
                )
            });
            let producers = ranks.each_ref().map(|rank| {
                self.projection.command(
                    rank.geometry.rank as usize,
                    PARTIAL,
                    rank.attention,
                    rank.layers[layer].weight(Qwen3TensorKind::OutputProjection),
                    rank.partial,
                    [1, 4096, 2048, 2, 1],
                )
            });
            record_collective(&mut steps, &mut ranks, &mut scratch, &mut cursor, producers)?;
            push_each(&mut steps, &ranks, |rank| {
                norm(
                    rank,
                    rank.hidden,
                    rank.layers[layer].weight(Qwen3TensorKind::PostAttentionLayerNorm),
                    rank.normalized,
                    1,
                    4096,
                )
            });
            for (kind, tag) in [
                (Qwen3TensorKind::GateProjection, 4),
                (Qwen3TensorKind::UpProjection, 5),
            ] {
                push_each(&mut steps, &ranks, |rank| {
                    self.projection.command(
                        rank.geometry.rank as usize,
                        GEMM,
                        rank.normalized,
                        rank.layers[layer].weight(kind),
                        if tag == 4 { rank.gate } else { rank.up },
                        [1, 6144, 4096, 2, tag],
                    )
                });
            }
            push_each(&mut steps, &ranks, |rank| {
                dispatch(
                    SWIGLU,
                    96,
                    vec![
                        rank.gate.read(),
                        rank.up.read(),
                        rank.activation.write(),
                        U32(1),
                        U32(2),
                    ],
                )
            });
            let producers = ranks.each_ref().map(|rank| {
                self.projection.command(
                    rank.geometry.rank as usize,
                    PARTIAL,
                    rank.activation,
                    rank.layers[layer].weight(Qwen3TensorKind::DownProjection),
                    rank.partial,
                    [1, 4096, 6144, 2, 2],
                )
            });
            record_collective(&mut steps, &mut ranks, &mut scratch, &mut cursor, producers)?;
        }
        let zero = &ranks[0];
        steps.push(Step::Rank {
            rank: 0,
            dispatch: norm(
                zero,
                zero.hidden,
                zero.global(Qwen3TensorKind::FinalNorm),
                zero.normalized,
                1,
                4096,
            ),
        });
        steps.push(Step::Rank {
            rank: 0,
            dispatch: self.projection.command(
                0,
                GEMM,
                zero.normalized,
                zero.global(Qwen3TensorKind::LanguageModelHead),
                zero.logits,
                [1, 151_936, 4096, 2, 6],
            ),
        });
        steps.push(Step::Rank {
            rank: 0,
            dispatch: dispatch(
                ARGMAX,
                1,
                vec![zero.logits.read(), zero.choice.write(), U32(1)],
            ),
        });
        if steps.len() != 1013
            || ranks.each_ref().map(|rank| rank.hidden) != initial_hidden
            || scratch != initial_scratch
            || cursor.expected().epoch != 1
        {
            return Err("prepared source recording count or ping-pong parity mismatch".into());
        }
        Ok(Program {
            group_id: initial.group_id,
            token_buffer: zero.token.id,
            result_buffer: zero.choice.id,
            metadata: std::array::from_fn(|rank| Metadata {
                positions: self.positions[rank].id,
                page_table: self.page_tables[rank].id,
                cos: ranks[rank].cos.id,
                sin: ranks[rank].sin.id,
            }),
            steps,
        })
    }

    pub(super) fn forward_prepared_peer(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        output_rows: &[usize],
    ) -> TpResult<Vec<u32>> {
        let state = self
            .prepared_peer
            .as_ref()
            .ok_or("prepared program missing")?;
        let graph_policy = state.graph_policy;
        let geometry = state.geometry;
        self.check_prepared_profile(graph_policy, state.kernel_profile, geometry)?;
        if batch.rows().len() != 1 || !matches!(output_rows, [] | [0]) {
            return Err("prepared TP2 expects exactly one active row".into());
        }
        let row = &batch.rows()[0];
        if u64::from(row.position()) != self.completed_batches
            || row.position() >= geometry.context_tokens()
            || row.token() >= 151_936
            || row.physical_pages().len()
                > usize::try_from(geometry.pages()).map_err(|_| "graph page bound")?
        {
            return Err("prepared TP2 position, vocabulary or page bound".into());
        }
        let state = self
            .prepared_peer
            .as_ref()
            .ok_or("prepared program missing")?;
        let mut page_table =
            vec![u32::MAX; usize::try_from(geometry.pages()).map_err(|_| "graph page bound")?];
        page_table[..row.physical_pages().len()].copy_from_slice(row.physical_pages());
        let (mut cos_sin, sin) = rope_bytes(row.position(), self.inner.plan.model().rope_theta);
        cos_sin.extend(sin);
        let generation = self
            .completed_batches
            .checked_add(1)
            .ok_or("prepared generation overflow")?;
        let input = GraphInput {
            geometry,
            plan_sha256: state.plan_sha256,
            generation,
            epoch: self.completed_batches,
            token: row.token(),
            position: row.position(),
            page_table,
            cos_sin,
        };
        let mut next_cursor = self.inner.collective;
        if next_cursor.expected().epoch != input.epoch
            || next_cursor.expected().layer != 0
            || next_cursor.expected().operation
                != Qwen3TensorParallelCollectiveV1::AttentionOutputSum
        {
            return Err("prepared live collective cursor drift".into());
        }
        for _ in 0..72 {
            advance(&mut next_cursor)?;
        }
        let next_counts = [
            self.inner.ranks[0]
                .dispatches
                .checked_add(u64::from(state.kernel_profile.kernel_counts()[0])),
            self.inner.ranks[1]
                .dispatches
                .checked_add(u64::from(state.kernel_profile.kernel_counts()[1])),
        ];
        let [Some(first), Some(second)] = next_counts else {
            return Err("prepared dispatch counter overflow".into());
        };
        // A graph transport unwind must not leave a reusable model cursor.
        // Only a completely validated and committed forward clears this flag.
        if graph_policy.is_some() {
            self.poisoned = true;
        }
        // Each path validates its own actual completion type before the common
        // all-or-terminal state commit. No intermediate graph drains are invented.
        let completion = match state.graph_policy {
            None => self.inner.transports[0]
                .execute_prepared_peer(&input.short_input()?)
                .and_then(|receipt| {
                    receipt.validate_for(&state.program, &input.short_input()?)?;
                    Ok(receipt.output_token)
                }),
            Some(policy) => self.inner.transports[0]
                .execute_prepared_peer_graph_geometry(&input)
                .and_then(|receipt| {
                    receipt.validate_graph_input_for_profile(
                        &state.program,
                        &input,
                        policy,
                        state.kernel_profile,
                    )?;
                    Ok(receipt.output_token)
                }),
        };
        let output_token = match completion {
            Ok(token) => token,
            Err(error) => {
                self.poisoned = true;
                return Err(match self.inner.close() {
                    Ok(()) => error,
                    Err(close) => format!("{error}; prepared execution close: {close}"),
                });
            }
        };
        self.inner.collective = next_cursor;
        self.inner.ranks[0].dispatches = first;
        self.inner.ranks[1].dispatches = second;
        self.inner.hidden = vec![0; 4096];
        if graph_policy.is_some() {
            self.poisoned = false;
        }
        Ok(if output_rows.is_empty() {
            Vec::new()
        } else {
            vec![output_token]
        })
    }
}
