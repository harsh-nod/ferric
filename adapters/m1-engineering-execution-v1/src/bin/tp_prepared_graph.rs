//! Bind the graph-specific wire to the registered program before host commit.

#[cfg(test)]
use super::super::Input;
use super::super::{GraphPolicy, Program, TpResult};
use super::{graph, protocol};
use ferric_engine::tensor_parallel::{
    Qwen3TensorParallelCollectiveKeyV1 as Key, Qwen3TensorParallelCollectiveV1 as Operation,
};
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTp2GraphCollectiveBindingV1 as Binding,
    EngineeringTp2GraphCompletionV1 as Completion, EngineeringTp2GraphInputV1 as GraphInput,
    EngineeringTp2GraphPolicyEvidenceV1 as Evidence, EngineeringTp2GraphRankV1 as Rank,
    EngineeringTp2PreparedGraphReceiptV1 as Receipt,
};
use ferric_spec::Qwen3ModelRole;

pub(super) fn valid_ready(
    response: &graph::Response,
    expected: graph::ExecutionMode,
    expected_kernels: graph::KernelProfile,
    expected_metadata: graph::MetadataUploadMode,
    ids: [u64; 2],
    pid: u32,
) -> bool {
    valid_ready_geometry(
        response,
        expected,
        expected_kernels,
        expected_metadata,
        graph::GraphGeometry::Short64,
        ids,
        pid,
    )
}

pub(super) fn valid_ready_geometry(
    response: &graph::Response,
    expected: graph::ExecutionMode,
    expected_kernels: graph::KernelProfile,
    expected_metadata: graph::MetadataUploadMode,
    geometry: graph::GraphGeometry,
    ids: [u64; 2],
    pid: u32,
) -> bool {
    if expected.decode_token() && expected_metadata != graph::MetadataUploadMode::Batched {
        return false;
    }
    if expected.finite_request()
        && expected_kernels != graph::KernelProfile::WaveStackNormAttentionKvMlp
    {
        return false;
    }
    let (version, mode) = match geometry {
        graph::GraphGeometry::Short64 => (graph::PROTOCOL, graph::MODE),
        graph::GraphGeometry::Long2304 => (graph::LONG_PROTOCOL, graph::LONG_MODE),
    };
    let expected_profile = geometry.program_profile(expected_kernels);
    matches!(response, graph::Response::Ready {
        protocol: actual_version, mode: actual_mode, execution_mode, kernel_profile, metadata_upload_mode, profile, unique_ids, process_id,
        authority, currentness, control_allocation_flags, native_source_sha256,
        native_program_deadline_ms, native_queued_deadline_ms,
    } if *actual_version == version && actual_mode == mode
        && *execution_mode == expected && *kernel_profile == expected_kernels
        && *metadata_upload_mode == expected_metadata
        && profile == &expected_profile
        && *unique_ids == ids && *process_id == pid && authority == "none"
        && currentness == expected.currentness()
        && *control_allocation_flags == super::dep::CONTROL_ALLOCATION_FLAGS
        && native_source_sha256 == graph::NATIVE_SOURCE
        && *native_program_deadline_ms == graph::NATIVE_PROGRAM_DEADLINE_MS
        && *native_queued_deadline_ms == graph::NATIVE_QUEUED_DEADLINE_MS)
}

// Setup shares the existing buffer/image protocol. Execution never fabricates
// serial collective receipts from graph schedule bindings.
pub(super) fn common_response(value: graph::Response) -> TpResult<protocol::Response> {
    Ok(match value {
        graph::Response::Setup { id, response } => protocol::Response::Setup { id, response },
        graph::Response::Registered {
            id,
            plan_sha256,
            catalog_sha256,
            steps,
            kernel_counts,
        } => protocol::Response::Registered {
            id,
            plan_sha256,
            catalog_sha256,
            steps,
            kernel_counts,
        },
        graph::Response::Closed { id } => protocol::Response::Closed { id },
        graph::Response::Fatal { id, message } => protocol::Response::Fatal { id, message },
        graph::Response::Ready { .. }
        | graph::Response::Executed { .. }
        | graph::Response::RegisteredFinite { .. } => {
            return Err("graph execution/ready cannot enter the serial response path".into());
        }
    })
}

#[cfg(test)]
pub(in super::super) fn validate_graph_receipt(
    actual: graph::GraphExecutionReceipt,
    program: &Program,
    input: &Input,
    ids: [u64; 2],
    request_id: u64,
    policy: GraphPolicy,
) -> TpResult<Receipt> {
    validate_geometry_graph_receipt(
        actual,
        program,
        &GraphInput::from(input),
        ids,
        request_id,
        policy,
        ferric_m1_engineering_execution_v1::tp_execution::EngineeringTp2GraphKernelProfileV1::Baseline,
    )
}

pub(in super::super) fn validate_geometry_graph_receipt(
    actual: graph::GraphExecutionReceipt,
    program: &Program,
    input: &GraphInput,
    ids: [u64; 2],
    request_id: u64,
    policy: GraphPolicy,
    profile: ferric_m1_engineering_execution_v1::tp_execution::EngineeringTp2GraphKernelProfileV1,
) -> TpResult<Receipt> {
    if actual.id != request_id
        || actual.graph.unique_ids != ids
        || actual.graph.ranks.each_ref().map(|rank| rank.unique_id) != ids
    {
        return Err("graph transport request or ordered physical device mismatch".into());
    }
    if policy.finite_request() {
        let geometry = match input.geometry {
            ferric_m1_engineering_execution_v1::tp_execution::EngineeringTp2GraphGeometryV1::Short64 => graph::GraphGeometry::Short64,
            ferric_m1_engineering_execution_v1::tp_execution::EngineeringTp2GraphGeometryV1::Long2304 => graph::GraphGeometry::Long2304,
        };
        graph::validate_finite_completion(
            &actual.policy,
            &actual.graph,
            graph::FiniteBudget::for_geometry(geometry),
        )
        .map_err(|e| e.to_string())?;
    }
    let graph = actual.graph;
    let mut collectives = Vec::with_capacity(graph.collectives.len());
    for binding in graph.collectives {
        let identity = binding.identity;
        if identity.model_role != super::dep::ModelRole::Target8b {
            return Err("graph collective model role mismatch".into());
        }
        collectives.push(Binding {
            key: Key {
                group_id: identity.group_id,
                model_role: Qwen3ModelRole::Target8B,
                epoch: identity.epoch,
                layer: identity.layer,
                operation: match identity.operation {
                    super::dep::Operation::AttentionOutputSum => Operation::AttentionOutputSum,
                    super::dep::Operation::FeedForwardDownSum => Operation::FeedForwardDownSum,
                },
            },
            request_id: identity.request_id,
            generation: identity.generation,
            before_producers: binding.before_producers,
            producers: binding.producers,
            after_producers: binding.after_producers,
            consumers: binding.consumers,
        });
    }
    let receipt = Receipt {
        plan_sha256: actual.plan_sha256,
        generation: actual.generation,
        epoch: actual.epoch,
        position: actual.position,
        input_token: actual.input_token,
        output_token: actual.output_token,
        completion: Completion {
            program_id: graph.program_id,
            group_id: graph.group_id,
            epoch: graph.epoch,
            unique_ids: graph.unique_ids,
            graph_api_calls: graph.graph_api_calls,
            logical_steps: graph.logical_steps,
            rank_dispatches: graph.rank_dispatches,
            kernel_counts: graph.kernel_counts,
            barrier_counts: graph.barrier_counts,
            packet_counts: graph.packet_counts,
            ranks: graph.ranks.map(|rank| Rank {
                unique_id: rank.unique_id,
                queue_epoch: rank.queue_epoch,
                first_packet: rank.first_packet,
                next_packet: rank.next_packet,
                final_write: rank.final_write,
                final_read: rank.final_read,
                completion_values: rank.completion_values,
            }),
            embedding_copy_packets: graph.embedding_copy_packets,
            collectives,
        },
        policy: match actual.policy {
            graph::PolicyEvidence::FiniteRequestAdmissionCache {
                full_boundaries,
                graph_operational_boundaries,
                token_boundaries,
                reset_only_rounds,
                loop_checks,
                token_ordinal,
                provisional,
                ..
            } => Evidence::FiniteRequestAdmissionCache {
                full_boundaries,
                graph_operational_boundaries,
                token_boundaries,
                reset_only_rounds,
                loop_checks,
                token_ordinal,
                provisional,
            },
            graph::PolicyEvidence::ClosedTokenAdmissionCache {
                full_boundaries,
                graph_operational_boundaries,
                token_boundaries,
                reset_only_rounds,
                loop_checks,
            } => Evidence::ClosedTokenAdmissionCache {
                full_boundaries,
                graph_operational_boundaries,
                token_boundaries,
                reset_only_rounds,
                loop_checks,
            },
            graph::PolicyEvidence::QueuedBaseline {} => Evidence::QueuedBaseline,
            graph::PolicyEvidence::TransactionFences {
                full_boundaries,
                operational_boundaries,
                reset_only_rounds,
                loop_checks,
            } => Evidence::TransactionFences {
                full_boundaries,
                operational_boundaries,
                reset_only_rounds,
                loop_checks,
            },
            graph::PolicyEvidence::TransactionFencesAdmissionCache {
                full_boundaries,
                operational_boundaries,
                reset_only_rounds,
                loop_checks,
            } => Evidence::TransactionFencesAdmissionCache {
                full_boundaries,
                operational_boundaries,
                reset_only_rounds,
                loop_checks,
            },
            graph::PolicyEvidence::TransactionFencesAdmissionCacheScopedObservations {
                full_boundaries,
                operational_boundaries,
                reset_only_rounds,
                loop_checks,
            } => Evidence::TransactionFencesAdmissionCacheScopedObservations {
                full_boundaries,
                operational_boundaries,
                reset_only_rounds,
                loop_checks,
            },
        },
    };
    receipt.validate_graph_input_for_profile(program, input, policy, profile)?;
    Ok(receipt)
}
