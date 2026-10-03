//! Metadata-only integration fixture for the production prepared-program builder.
//! No weights, GPU execution, completed receipts, or numerical output are mocked.

#[path = "finite_composition.rs"]
mod finite_composition;
#[path = "prepared_context2304.rs"]
mod prepared_context2304;
#[path = "prepared_graph.rs"]
mod prepared_graph;
#[path = "prepared_split_attention.rs"]
mod prepared_split_attention;
#[path = "prepared_wave_stack.rs"]
mod prepared_wave_stack;

use super::*;
use crate::tp_execution::{
    EngineeringTp2PreparedProgramV1 as Program, EngineeringTp2PreparedStepV1 as Step,
};
use serde_json::{Value, json};
use std::io::Write;

const PROFILE: &str = "qwen3-8b-tp2-mfma-baseline-attention-bf16-row1-cap16-context64-pages4";

#[derive(Clone, Copy)]
struct Allocation {
    id: u64,
    rank: u32,
    bytes: usize,
    peer_readable: bool,
}

#[derive(Default)]
struct Catalog {
    allocations: Vec<Allocation>,
    registered: Option<Program>,
    allocation_fault: Option<(usize, bool)>,
    registration_fault: Option<bool>,
    deny_dependency: bool,
    deny_graph: bool,
}

struct MetadataOnly {
    rank: u32,
    catalog: Rc<RefCell<Catalog>>,
}

impl MetadataOnly {
    fn allocate_metadata(&mut self, bytes: usize, peer_readable: bool) -> TpResult<u64> {
        let mut catalog = self.catalog.borrow_mut();
        if bytes == 0 || catalog.registered.is_some() {
            return Err("fixture allocation after seal or empty allocation".into());
        }
        if let Some((index, unwind)) = catalog.allocation_fault
            && index == catalog.allocations.len()
        {
            assert!(!unwind, "selected allocation unwind");
            return Err("selected allocation failure".into());
        }
        let id = u64::try_from(catalog.allocations.len()).unwrap() + 1;
        catalog.allocations.push(Allocation {
            id,
            rank: self.rank,
            bytes,
            peer_readable,
        });
        Ok(id)
    }
}

impl EngineeringTpRankTransportV1 for MetadataOnly {
    fn peer_group_rank(&self) -> Option<(u32, u32, u32)> {
        // A logical fixture identity, never a native worker PID claim.
        Some((1, self.rank, 2))
    }
    fn supports_peer_dependency_collectives(&self) -> bool {
        !self.catalog.borrow().deny_dependency
    }
    fn supports_prepared_peer(&self) -> bool {
        true
    }
    fn supports_prepared_peer_graph(
        &self,
        _: crate::tp_execution::EngineeringTp2GraphPolicyV1,
    ) -> bool {
        true
    }
    fn supports_prepared_peer_graph_profile(
        &self,
        _: crate::tp_execution::EngineeringTp2GraphPolicyV1,
        _: crate::tp_execution::EngineeringTp2GraphKernelProfileV1,
    ) -> bool {
        true
    }
    fn register_prepared_peer_graph(
        &mut self,
        program: &Program,
        _: crate::tp_execution::EngineeringTp2GraphPolicyV1,
    ) -> TpResult<[u8; 32]> {
        self.register_prepared_peer(program)
    }
    fn supports_prepared_peer_graph_geometry(
        &self,
        _: crate::tp_execution::EngineeringTp2GraphPolicyV1,
        _: crate::tp_execution::EngineeringTp2GraphKernelProfileV1,
        _: crate::tp_execution::EngineeringTp2GraphGeometryV1,
    ) -> bool {
        !self.catalog.borrow().deny_graph
    }
    fn register_prepared_peer_graph_geometry(
        &mut self,
        program: &Program,
        _: crate::tp_execution::EngineeringTp2GraphPolicyV1,
        _: crate::tp_execution::EngineeringTp2GraphGeometryV1,
    ) -> TpResult<[u8; 32]> {
        self.register_prepared_peer(program)
    }
    fn allocate(&mut self, bytes: usize) -> TpResult<u64> {
        self.allocate_metadata(bytes, false)
    }
    fn allocate_peer_readable(&mut self, bytes: usize) -> TpResult<u64> {
        self.allocate_metadata(bytes, true)
    }
    fn register_prepared_peer(&mut self, program: &Program) -> TpResult<[u8; 32]> {
        let mut catalog = self.catalog.borrow_mut();
        if self.rank != 0 || catalog.registered.is_some() {
            return Err("fixture registration must be exactly once on rank zero".into());
        }
        validate_extents(program, &catalog.allocations);
        if let Some(unwind) = catalog.registration_fault {
            assert!(!unwind, "selected registration unwind");
            return Err("selected registration failure".into());
        }
        catalog.registered = Some(program.clone());
        // Registration identity only; no execution receipt is fabricated.
        Ok([0x51; 32])
    }
    fn write(&mut self, _: u64, _: usize, _: &[u8]) -> TpResult<()> {
        Err("metadata-only fixture forbids data writes".into())
    }
    fn read(&mut self, _: u64, _: usize, _: &mut [u8]) -> TpResult<()> {
        Err("metadata-only fixture forbids data reads".into())
    }
    fn submit(&mut self, _: &EngineeringTpDispatchV1) -> TpResult<()> {
        Err("metadata-only fixture forbids dispatch".into())
    }
    fn wait(&mut self) -> TpResult<()> {
        Err("metadata-only fixture forbids completion".into())
    }
    fn close(&mut self) -> TpResult<()> {
        Ok(())
    }
}

fn validate_extents(program: &Program, allocations: &[Allocation]) {
    let check = |rank, dispatch: &EngineeringTpDispatchV1| {
        for argument in &dispatch.arguments {
            if let EngineeringTpArgumentV1::Buffer {
                id,
                offset,
                elements,
                element_bytes,
                access,
            } = *argument
            {
                let allocation = allocations.iter().find(|item| item.id == id).unwrap();
                assert!(
                    allocation.rank == rank
                        || (allocation.peer_readable
                            && access == EngineeringTpBufferAccessV1::Read)
                );
                let extent = elements.checked_mul(element_bytes as usize).unwrap();
                assert!(offset.checked_add(extent).unwrap() <= allocation.bytes);
            }
        }
    };
    for step in &program.steps {
        match step {
            Step::Rank { rank, dispatch } => check(*rank, dispatch),
            Step::Collective(request) => {
                request.validate().unwrap();
                for rank in 0..2 {
                    check(u32::try_from(rank).unwrap(), &request.producers[rank]);
                    check(u32::try_from(rank).unwrap(), &request.consumers[rank]);
                }
            }
        }
    }
}

fn builder() -> EngineeringTpBatchExecutionV2<MetadataOnly> {
    builder_for_pool(&pool())
}

fn builder_for_pool(
    pool: &EngineeringTpPagedPoolV1,
) -> EngineeringTpBatchExecutionV2<MetadataOnly> {
    let model = target();
    let row_capacity = pool.row_capacity();
    let context = pool.limits().context_tokens();
    let physical_pages = pool.limits().physical_page_count();
    let table_stride = pool.limits().page_table_stride();
    assert_eq!(row_capacity, 16);
    let catalog = Rc::new(RefCell::new(Catalog::default()));
    let mut transports = (0..2)
        .map(|rank| MetadataOnly {
            rank,
            catalog: catalog.clone(),
        })
        .collect::<Vec<_>>();
    let plan = Qwen3TensorParallelPlanV1::new(model, 2).unwrap();
    let mut ranks = Vec::new();
    let mut positions = Vec::new();
    let mut page_tables = Vec::new();
    let mut transposes = Vec::new();
    let mut transpose_bytes = 0_u64;
    for (index, transport) in transports.iter_mut().enumerate() {
        let mut rank = allocate_rank_storage(
            transport,
            &plan,
            u32::try_from(index).unwrap(),
            physical_pages * 16,
            16,
        )
        .unwrap();
        let mut transposed = BTreeMap::new();
        for layer in &mut rank.layers {
            // Exact resident BF16 TP2 tensor extents. Only allocation metadata
            // is created; authenticated weight intake/transposition is separate.
            for (kind, elements, projection) in [
                (Qwen3TensorKind::InputLayerNorm, 4096, false),
                (Qwen3TensorKind::PostAttentionLayerNorm, 4096, false),
                (Qwen3TensorKind::QueryNorm, 128, false),
                (Qwen3TensorKind::KeyNorm, 128, false),
                (Qwen3TensorKind::QueryProjection, 2048 * 4096, true),
                (Qwen3TensorKind::KeyProjection, 512 * 4096, true),
                (Qwen3TensorKind::ValueProjection, 512 * 4096, true),
                (Qwen3TensorKind::OutputProjection, 4096 * 2048, true),
                (Qwen3TensorKind::GateProjection, 6144 * 4096, true),
                (Qwen3TensorKind::UpProjection, 6144 * 4096, true),
                (Qwen3TensorKind::DownProjection, 4096 * 6144, true),
            ] {
                let original = allocate_tensor(transport, elements, 2).unwrap();
                layer.weights.push((kind, original));
                if projection {
                    let tensor = allocate_tensor(transport, elements, 2).unwrap();
                    assert!(transposed.insert(original.id, tensor).is_none());
                    transpose_bytes += u64::try_from(elements * 2).unwrap();
                }
            }
        }
        if index == 0 {
            for (kind, elements) in [
                (Qwen3TensorKind::TokenEmbedding, 151_936 * 4096),
                (Qwen3TensorKind::FinalNorm, 4096),
                (Qwen3TensorKind::LanguageModelHead, 151_936 * 4096),
            ] {
                let original = allocate_tensor(transport, elements, 2).unwrap();
                rank.globals.push((kind, original));
                if kind == Qwen3TensorKind::LanguageModelHead {
                    let tensor = allocate_tensor(transport, elements, 2).unwrap();
                    assert!(transposed.insert(original.id, tensor).is_none());
                    transpose_bytes += u64::try_from(elements * 2).unwrap();
                }
            }
        }
        ranks.push(rank);
        transposes.push(transposed);
        positions.push(allocate_tensor(transport, row_capacity, 4).unwrap());
        page_tables.push(
            allocate_tensor(
                transport,
                row_capacity * usize::try_from(table_stride).unwrap(),
                4,
            )
            .unwrap(),
        );
    }
    assert_eq!(transpose_bytes, 15_136_194_560);
    let inner = EngineeringTpExecutionV1 {
        finite_model_binding: None,
        transports,
        ranks,
        plan,
        sequence: TensorParallelSequenceV1::new(context, model.vocabulary_size).unwrap(),
        collective: Qwen3TensorParallelCollectiveStateV1::new(&plan, 0, 0),
        capacity: context,
        row_capacity: 16,
        large_kv: false,
        draft_v10: false,
        hidden: vec![0; model.hidden_size as usize * row_capacity],
        reduction: super::super::super::ReductionWorkspace::default(),
        sequences: None,
        ordered_batches: None,
        full_forward_enabled: false,
        full_forward: None,
        peer_dependency_pending: None,
        timing: crate::host_timing::HostTiming::default(),
        closed: false,
    };
    let projection =
        super::super::super::projection::ProjectionPolicy::synthetic_mfma_ranks_for_recording(
            transposes,
            transpose_bytes,
        );
    let mut driver = EngineeringTpBatchExecutionV2 {
        inner,
        row_capacity,
        positions,
        page_tables,
        scope: pool.scope(),
        pool_identity: pool.identity(),
        context_tokens: context,
        physical_pages,
        table_stride,
        last_batch: 0,
        completed_batches: 0,
        poisoned: false,
        prune_output_head: false,
        projection,
        projection_configured: true,
        c1_wave_layers: false,
        wave_attention: false,
        numerical: None,
        head_profile_configured: false,
        fp32_logits: None,
        fp32_argmax_v11: None,
        admitted_argmax_v11: None,
        query_hoist_v14: None,
        admitted_query_hoist_v14: None,
        wave_rmsnorm_v15: None,
        admitted_wave_rmsnorm_v15: None,
        parallel_kv_v16: None,
        admitted_parallel_kv_v16: None,
        prepared_peer: None,
        split_attention_scratch: None,
    };
    driver
        .configure_reduction(EngineeringTpReductionModeV3::DevicePeerDependencyV1)
        .unwrap();
    driver
}

fn argument_json(argument: &EngineeringTpArgumentV1) -> Value {
    match *argument {
        EngineeringTpArgumentV1::Buffer {
            id,
            offset,
            elements,
            element_bytes,
            access,
        } => {
            let access = match access {
                EngineeringTpBufferAccessV1::Read => "read",
                EngineeringTpBufferAccessV1::Write => "write",
                EngineeringTpBufferAccessV1::ReadWrite => "read_write",
            };
            json!({"kind":"buffer", "id":id, "offset":offset, "elements":elements,
                "element_bytes":element_bytes, "access":access})
        }
        EngineeringTpArgumentV1::U32(value) => json!({"kind":"u32", "value":value}),
        EngineeringTpArgumentV1::F32(value) => json!({"kind":"f32_bits", "value":value.to_bits()}),
    }
}

fn dispatch_json(dispatch: &EngineeringTpDispatchV1) -> Value {
    json!({"kernel":dispatch.kernel, "grid_workgroups":dispatch.grid_workgroups,
        "workgroup_size":dispatch.workgroup_size,
        "arguments":dispatch.arguments.iter().map(argument_json).collect::<Vec<_>>()})
}

fn fixture_json(driver: &EngineeringTpBatchExecutionV2<MetadataOnly>) -> Value {
    let catalog = driver.inner.transports[0].catalog.borrow();
    let program = catalog.registered.as_ref().unwrap();
    let steps = program
        .steps
        .iter()
        .map(|step| match step {
            Step::Rank { rank, dispatch } => json!({"step":"rank", "rank":rank,
            "dispatch":dispatch_json(dispatch)}),
            Step::Collective(request) => {
                assert_eq!(request.key.model_role, Qwen3ModelRole::Target8B);
                let operation = match request.key.operation {
                    Qwen3TensorParallelCollectiveV1::AttentionOutputSum => "attention_output_sum",
                    Qwen3TensorParallelCollectiveV1::FeedForwardDownSum => "feed_forward_down_sum",
                };
                json!({"step":"collective", "key":{"group_id":request.key.group_id,
                "model_role":"target8b", "epoch":request.key.epoch, "layer":request.key.layer,
                "operation":operation}, "rows":request.rows,
                "producers":request.producers.iter().map(dispatch_json).collect::<Vec<_>>(),
                "consumers":request.consumers.iter().map(dispatch_json).collect::<Vec<_>>()})
            }
        })
        .collect::<Vec<_>>();
    let positions: &[u32] = if driver.context_tokens == 2304 {
        &[0, 1, 17, 63, 64, 173, 174, 2047, 2302, 2303]
    } else {
        &[0, 1, 17, 63]
    };
    let rope_fixtures = positions
        .iter()
        .copied()
        .map(|position| {
            let (mut bytes, sin) = super::super::super::rope_bytes(position, target().rope_theta);
            bytes.extend(sin);
            json!({"position":position, "bytes":bytes})
        })
        .collect::<Vec<_>>();
    let mut value = json!({"schema":"ferric-prepared-parent-builder-fixture-v1", "profile":PROFILE,
        "rope_fixtures":rope_fixtures,
        "program":{"group_id":program.group_id, "token_buffer":program.token_buffer,
            "result_buffer":program.result_buffer,
            "metadata":program.metadata.iter().map(|metadata| json!({
                "positions":metadata.positions, "page_table":metadata.page_table,
                "cos":metadata.cos, "sin":metadata.sin})).collect::<Vec<_>>(), "steps":steps},
        "buffers":catalog.allocations.iter().map(|allocation| json!({"id":allocation.id,
            "rank":allocation.rank, "bytes":allocation.bytes,
            "peer_readable":allocation.peer_readable})).collect::<Vec<_>>()});
    if driver.context_tokens == 2304 {
        value["schema"] = json!("ferric-prepared-parent-builder-context2304-fixture-v2");
        value["geometry"] = json!("long2304");
    }
    value
}

#[test]
fn graph_v22_actual_recording_changes_only_the_last_root_and_preserves_state() {
    use crate::tp_execution::EngineeringTp2GraphKernelProfileV1 as Profile;
    let driver = builder();
    let before = driver.inner.collective.expected();
    let baseline = driver
        .record_graph_kernel_profile_fixture(Profile::Baseline)
        .unwrap();
    let scalar = driver
        .record_graph_kernel_profile_fixture(Profile::V22Scalar)
        .unwrap();
    let mut wave = driver
        .record_graph_kernel_profile_fixture(Profile::V22Wave)
        .unwrap();
    assert_eq!(baseline, scalar);
    assert_eq!(wave.steps.len(), 1013);
    assert_eq!(&wave.steps[..1012], &scalar.steps[..1012]);
    let Step::Rank { rank, dispatch } = &mut wave.steps[1012] else {
        panic!("rank-zero argmax")
    };
    assert_eq!(*rank, 0);
    assert_eq!(
        dispatch.kernel,
        crate::tp_artifact::ENGINEERING_TP_GRAPH_ARGMAX_ROOT_V22
    );
    dispatch.kernel = "ferric_qwen3_tp_batch_argmax_bf16_v2";
    assert_eq!(wave, scalar);
    assert_eq!(driver.inner.collective.expected(), before);
    assert_eq!(driver.dispatch_counts(), [0, 0]);
    assert!(driver.prepared_peer.is_none());
    assert!(
        driver.inner.transports[0]
            .catalog
            .borrow()
            .registered
            .is_none()
    );
}

#[test]
fn prepared_actual_builder_registers_once_without_execution_or_state_commit() {
    let mut driver = builder();
    let initial = driver.inner.collective.expected();
    let hidden = driver
        .inner
        .ranks
        .iter()
        .map(|rank| rank.hidden)
        .collect::<Vec<_>>();
    driver.configure_prepared_peer().unwrap();
    let fixture = fixture_json(&driver);
    let steps = fixture["program"]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 1013);
    assert_eq!(
        steps
            .iter()
            .filter(|step| step["step"] == "collective")
            .count(),
        72
    );
    assert_eq!(driver.inner.collective.expected(), initial);
    assert_eq!(
        driver
            .inner
            .ranks
            .iter()
            .map(|rank| rank.hidden)
            .collect::<Vec<_>>(),
        hidden
    );
    assert!(driver.inner.ranks.iter().all(|rank| rank.dispatches == 0));
    assert_eq!(driver.last_batch, 0);
    assert_eq!(driver.completed_batches, 0);
    assert!(driver.configure_prepared_peer().is_err());
    assert_eq!(fixture_json(&driver), fixture);
}

#[test]
fn prepared_actual_builder_rejects_incompatible_profiles_before_registration() {
    for mutation in 0..4 {
        let mut driver = builder();
        match mutation {
            0 => driver.context_tokens = 65,
            1 => driver.prune_output_head = true,
            2 => driver.wave_attention = true,
            3 => driver.projection_configured = false,
            _ => unreachable!(),
        }
        assert!(driver.configure_prepared_peer().is_err());
        assert!(
            driver.inner.transports[0]
                .catalog
                .borrow()
                .registered
                .is_none()
        );
    }
}

#[test]
fn prepared_actual_driver_execution_failure_has_no_commit_and_cannot_resume() {
    let mut pool = pool();
    let mut driver = builder_for_pool(&pool);
    driver.configure_prepared_peer().unwrap();
    let initial_cursor = driver.inner.collective.expected();
    let initial_hidden = driver.inner.hidden.clone();
    let initial_rank_hidden = driver
        .inner
        .ranks
        .iter()
        .map(|rank| rank.hidden)
        .collect::<Vec<_>>();
    let registered = fixture_json(&driver);
    let batch = prepare(&mut pool, 1);
    pool.begin_submission(&batch).unwrap();
    let error = driver
        .execute_selected(&batch, &[0])
        .err()
        .expect("execution must fail");
    assert_eq!(error, "transport does not support prepared TP2 execution");
    assert!(driver.poisoned && driver.inner.closed);
    assert_eq!(driver.inner.collective.expected(), initial_cursor);
    assert_eq!(driver.dispatch_counts(), [0, 0]);
    assert_eq!(driver.last_batch, 0);
    assert_eq!(driver.completed_batches, 0);
    assert_eq!(driver.inner.hidden, initial_hidden);
    assert_eq!(
        driver
            .inner
            .ranks
            .iter()
            .map(|rank| rank.hidden)
            .collect::<Vec<_>>(),
        initial_rank_hidden
    );
    assert_eq!(fixture_json(&driver), registered);
    assert!(driver.execute_selected(&batch, &[0]).is_err());
    assert!(driver.configure_prepared_peer().is_err());
    assert_eq!(driver.inner.collective.expected(), initial_cursor);
    assert_eq!(driver.dispatch_counts(), [0, 0]);
    assert_eq!(driver.last_batch, 0);
    assert_eq!(driver.completed_batches, 0);
}

#[test]
#[ignore = "CPU-only fixture export requires FERRIC_PREPARED_PROGRAM_FIXTURE create-new path"]
fn export_prepared_actual_builder_fixture() {
    export_builder_fixture(None);
}

#[test]
#[ignore = "CPU-only actual V22 admission and profile builder export require pinned paths"]
fn export_prepared_actual_v22_builder_fixture() {
    use crate::tp_execution::EngineeringTp2GraphKernelProfileV1 as Profile;
    let profile = match std::env::var("FERRIC_TP2_GRAPH_KERNEL_PROFILE")
        .unwrap()
        .as_str()
    {
        "bf16-argmax-v22-scalar" => Profile::V22Scalar,
        "bf16-argmax-v22-wave" => Profile::V22Wave,
        _ => panic!("exact V22 profile required"),
    };
    export_builder_fixture(Some(profile));
}

#[test]
#[ignore = "CPU-only wave-stack builder export requires pinned V22 and V15 image paths"]
fn export_prepared_actual_wave_stack_builder_fixture() {
    use crate::tp_execution::EngineeringTp2GraphKernelProfileV1 as Profile;
    let name = std::env::var("FERRIC_TP2_GRAPH_KERNEL_PROFILE").unwrap();
    let profile = Profile::ALL
        .into_iter()
        .find(|profile| profile.has_v15() && profile.argument() == name)
        .expect("exact wave-stack profile required");
    export_builder_fixture(Some(profile));
}

#[test]
#[ignore = "CPU-only context2304 builder export requires pinned profile image paths"]
fn export_prepared_actual_context2304_builder_fixture() {
    use crate::tp_execution::{
        EngineeringTp2GraphGeometryV1 as Geometry, EngineeringTp2GraphKernelProfileV1 as Profile,
        EngineeringTp2GraphPolicyV1 as Policy,
    };
    assert_eq!(
        std::env::var("FERRIC_TP2_GRAPH_GEOMETRY").unwrap(),
        "long2304"
    );
    let name = std::env::var("FERRIC_TP2_GRAPH_KERNEL_PROFILE").unwrap();
    let profile = Profile::ALL
        .into_iter()
        .find(|profile| profile.argument() == name)
        .unwrap();
    let pool = EngineeringTpPagedPoolV1::new(
        pool().scope(),
        EngineeringTpPagedLimitsV1::new(2304, 32, 144, 100).unwrap(),
    )
    .unwrap();
    let mut driver = builder_for_pool(&pool);
    let argmax = profile.has_v22().then(|| {
        crate::tp_artifact::EngineeringTpArtifactV1::open_graph_bf16_argmax_v22(
            std::path::Path::new(&std::env::var_os("FERRIC_PREPARED_V22_ARTIFACT").unwrap()),
        )
        .unwrap()
    });
    let norm = profile.has_v15().then(|| {
        crate::tp_artifact::EngineeringTpArtifactV1::open_graph_wave_rmsnorm_v15(
            std::path::Path::new(&std::env::var_os("FERRIC_PREPARED_V15_ARTIFACT").unwrap()),
        )
        .unwrap()
    });
    if profile.split_attention() {
        let split = crate::tp_artifact::EngineeringTpArtifactV1::open_graph_split_attention_v1(
            std::path::Path::new(
                &std::env::var_os("FERRIC_PREPARED_SPLIT_ATTENTION_ARTIFACT").unwrap(),
            ),
        )
        .unwrap();
        driver
            .configure_prepared_peer_graph_split_attention(
                Policy::TransactionFences,
                Geometry::Long2304,
                argmax.as_ref().unwrap(),
                norm.as_ref().unwrap(),
                &split,
            )
            .unwrap();
    } else {
        driver
            .configure_prepared_peer_graph_geometry(
                Policy::TransactionFences,
                profile,
                Geometry::Long2304,
                argmax.as_ref(),
                norm.as_ref(),
            )
            .unwrap();
    }
    let mut fixture = fixture_json(&driver);
    fixture["profile"] = json!(Geometry::Long2304.program_profile(profile));
    write_builder_fixture(&fixture);
}

fn export_builder_fixture(
    profile: Option<crate::tp_execution::EngineeringTp2GraphKernelProfileV1>,
) {
    let mut driver = builder();
    if let Some(profile) = profile {
        let artifact = crate::tp_artifact::EngineeringTpArtifactV1::open_graph_bf16_argmax_v22(
            std::path::Path::new(&std::env::var_os("FERRIC_PREPARED_V22_ARTIFACT").unwrap()),
        )
        .unwrap();
        let policy = crate::tp_execution::EngineeringTp2GraphPolicyV1::TransactionFences;
        if profile.has_v15() {
            let norm = crate::tp_artifact::EngineeringTpArtifactV1::open_graph_wave_rmsnorm_v15(
                std::path::Path::new(&std::env::var_os("FERRIC_PREPARED_V15_ARTIFACT").unwrap()),
            )
            .unwrap();
            assert!(
                driver
                    .configure_prepared_peer_graph_with_argmax_v22(policy, profile, &artifact)
                    .is_err()
            );
            assert!(
                driver
                    .configure_prepared_peer_graph_with_wave_stack(
                        policy, profile, &artifact, &artifact
                    )
                    .is_err()
            );
            assert!(
                driver
                    .configure_prepared_peer_graph_with_wave_stack(policy, profile, &norm, &norm)
                    .is_err()
            );
            assert!(
                driver
                    .configure_prepared_peer_graph_with_wave_stack(
                        policy,
                        crate::tp_execution::EngineeringTp2GraphKernelProfileV1::V22Wave,
                        &artifact,
                        &norm
                    )
                    .is_err()
            );
            assert!(driver.prepared_peer.is_none());
            assert!(
                driver.inner.transports[0]
                    .catalog
                    .borrow()
                    .registered
                    .is_none()
            );
            if profile.split_attention() {
                let split =
                    crate::tp_artifact::EngineeringTpArtifactV1::open_graph_split_attention_v1(
                        std::path::Path::new(
                            &std::env::var_os("FERRIC_PREPARED_SPLIT_ATTENTION_ARTIFACT").unwrap(),
                        ),
                    )
                    .unwrap();
                driver
                    .configure_prepared_peer_graph_split_attention(
                        policy,
                        crate::tp_execution::EngineeringTp2GraphGeometryV1::Short64,
                        &artifact,
                        &norm,
                        &split,
                    )
                    .unwrap();
            } else {
                driver
                    .configure_prepared_peer_graph_with_wave_stack(
                        policy, profile, &artifact, &norm,
                    )
                    .unwrap();
            }
        } else {
            driver
                .configure_prepared_peer_graph_with_argmax_v22(policy, profile, &artifact)
                .unwrap();
        }
    } else {
        driver.configure_prepared_peer().unwrap();
    }
    let mut fixture = fixture_json(&driver);
    if let Some(profile) = profile {
        fixture["profile"] = json!(format!("{PROFILE}+{}", profile.argument()));
    }
    write_builder_fixture(&fixture);
}

fn write_builder_fixture(fixture: &Value) {
    let path = std::env::var_os("FERRIC_PREPARED_PROGRAM_FIXTURE").expect("fixture output path");
    let bytes = serde_json::to_vec(fixture).unwrap();
    assert!(bytes.len() <= 4 * 1024 * 1024);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    file.write_all(&bytes).unwrap();
    file.sync_all().unwrap();
}
