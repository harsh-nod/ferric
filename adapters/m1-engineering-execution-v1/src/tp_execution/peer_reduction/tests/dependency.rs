//! Records the actual driver boundary; no GPU arithmetic or timing is simulated.

use super::*;
use crate::tp_execution::EngineeringTpReductionModeV3;
use crate::tp_execution::peer_dependency::{
    EngineeringTp2CollectiveReceiptV1, EngineeringTp2CollectiveRequestV1,
};

const MFMA: &str = "ferric_qwen3_tp_mfma_gemm_partial_f32_v3";

#[derive(Clone, Copy)]
pub(super) enum ReceiptFailure {
    Transport,
    WrongKey,
    IncompleteSignal,
    ProducerFrontier,
    ConsumerFrontier,
    WrongCounts,
    WrongEpoch,
    ZeroGeneration,
}

pub(super) fn record_transaction(
    transport: &mut Transport,
    request: &EngineeringTp2CollectiveRequestV1,
) -> TpResult<EngineeringTp2CollectiveReceiptV1> {
    assert!(transport.dependency_supported);
    assert_eq!(transport.rank, 0);
    assert!(transport.pending.is_none());
    request.validate()?;
    let mut memory = transport.memory.borrow_mut();
    memory.dependency_requests.push(request.clone());
    let generation = u64::try_from(memory.dependency_requests.len()).unwrap();
    let first = (generation - 1) * 3;
    let mut receipt = EngineeringTp2CollectiveReceiptV1 {
        key: request.key,
        generation,
        kernel_dispatches: [2, 2],
        barrier_packets: [1, 1],
        packet_counts: [3, 3],
        completion_values: [0; 6],
        queue_epochs: [0; 2],
        first_packet_ids: [first; 2],
        frontiers: [[first + 3; 2]; 2],
    };
    match transport.dependency_failure {
        None => {}
        Some(ReceiptFailure::Transport) => return Err("injected dependency failure".into()),
        Some(ReceiptFailure::WrongKey) => receipt.key.epoch += 1,
        Some(ReceiptFailure::IncompleteSignal) => receipt.completion_values[4] = 1,
        Some(ReceiptFailure::ProducerFrontier) => receipt.frontiers[0][1] -= 1,
        Some(ReceiptFailure::ConsumerFrontier) => receipt.frontiers[1][1] -= 1,
        Some(ReceiptFailure::WrongCounts) => receipt.kernel_dispatches[1] = 1,
        Some(ReceiptFailure::WrongEpoch) => receipt.queue_epochs[1] = 1,
        Some(ReceiptFailure::ZeroGeneration) => receipt.generation = 0,
    }
    Ok(receipt)
}

fn dependency_fixture() -> EngineeringTpExecutionV1<Transport> {
    let mut execution = fixture(2);
    for transport in &mut execution.transports {
        transport.dependency_supported = true;
    }
    execution
        .configure_reduction(EngineeringTpReductionModeV3::DevicePeerDependencyV1)
        .unwrap();
    initialize(&mut execution, 1);
    execution
}

fn producers(
    execution: &EngineeringTpExecutionV1<Transport>,
    operation: Qwen3TensorParallelCollectiveV1,
) -> [EngineeringTpDispatchV1; 2] {
    std::array::from_fn(|index| {
        let rank = &execution.ranks[index];
        let (input, k, tag) = match operation {
            Qwen3TensorParallelCollectiveV1::AttentionOutputSum => (rank.attention, 2048, 1),
            Qwen3TensorParallelCollectiveV1::FeedForwardDownSum => (rank.activation, 6144, 2),
        };
        // Exact metadata with synthetic weight IDs. These tests do not execute
        // tensor arithmetic or assert that this is an admitted device allocation.
        let weight = Tensor {
            id: 1_000_000 + index as u64,
            elements: 4096 * k,
            element_bytes: 2,
        };
        dispatch(
            MFMA,
            256,
            vec![
                input.read(),
                weight.read(),
                rank.partial.write(),
                EngineeringTpArgumentV1::U32(1),
                EngineeringTpArgumentV1::U32(4096),
                EngineeringTpArgumentV1::U32(u32::try_from(k).unwrap()),
                EngineeringTpArgumentV1::U32(2),
                EngineeringTpArgumentV1::U32(tag),
            ],
        )
    })
}

fn capture(
    execution: &mut EngineeringTpExecutionV1<Transport>,
    operation: Qwen3TensorParallelCollectiveV1,
) {
    let commands = producers(execution, operation);
    execution
        .dispatch_collective_producers(0, operation, |rank| {
            commands[rank.geometry.rank as usize].clone()
        })
        .unwrap();
}

#[test]
fn both_collectives_defer_exact_producers_until_one_complete_transaction() {
    let mut execution = dependency_fixture();
    for (ordinal, operation) in [
        Qwen3TensorParallelCollectiveV1::AttentionOutputSum,
        Qwen3TensorParallelCollectiveV1::FeedForwardDownSum,
    ]
    .into_iter()
    .enumerate()
    {
        let previous = execution.collective;
        let hidden = execution
            .ranks
            .iter()
            .map(|rank| rank.hidden)
            .collect::<Vec<_>>();
        let expected = producers(&execution, operation);
        capture(&mut execution, operation);
        assert_eq!(execution.collective, previous);
        assert!(execution.transports[0].memory.borrow().commands.is_empty());
        assert_eq!(execution.dispatch_counts(), vec![ordinal as u64 * 2; 2]);
        let retained = execution.peer_dependency_pending.as_ref().unwrap();
        assert_eq!(retained.producers, expected);
        for consumer in &retained.consumers {
            assert_eq!(
                consumer.kernel,
                crate::tp_execution::peer_dependency::CONSUMER
            );
            assert_ne!(consumer.kernel, REDUCE);
            assert_eq!(tensor(consumer, 0).0, execution.ranks[0].partial.id);
            assert_eq!(tensor(consumer, 1).0, execution.ranks[1].partial.id);
            for index in 2..8 {
                assert_eq!(tensor(consumer, index).1, 0);
            }
        }
        execution.reduce(0, operation).unwrap();
        assert!(execution.peer_dependency_pending.is_none());
        assert_eq!(
            execution.dispatch_counts(),
            vec![(ordinal as u64 + 1) * 2; 2]
        );
        assert_eq!(
            execution.transports[0]
                .memory
                .borrow()
                .dependency_requests
                .len(),
            ordinal + 1
        );
        let ReductionWorkspace::DevicePeer(scratch, _) = &execution.reduction else {
            panic!("peer workspace lost");
        };
        assert_eq!(scratch, &hidden);
        for (rank, old) in execution.ranks.iter().zip(hidden) {
            assert_ne!(rank.hidden.id, old.id);
        }
    }
    assert_eq!(execution.collective.expected().layer, 1);
}

#[test]
fn malformed_or_failed_receipt_never_commits_any_model_state() {
    for failure in [
        ReceiptFailure::Transport,
        ReceiptFailure::WrongKey,
        ReceiptFailure::IncompleteSignal,
        ReceiptFailure::ProducerFrontier,
        ReceiptFailure::ConsumerFrontier,
        ReceiptFailure::WrongCounts,
        ReceiptFailure::WrongEpoch,
        ReceiptFailure::ZeroGeneration,
    ] {
        let mut execution = dependency_fixture();
        let operation = Qwen3TensorParallelCollectiveV1::AttentionOutputSum;
        capture(&mut execution, operation);
        let before = execution.collective;
        let hidden = execution
            .ranks
            .iter()
            .map(|rank| rank.hidden)
            .collect::<Vec<_>>();
        execution.transports[0].dependency_failure = Some(failure);
        assert!(execution.reduce(0, operation).is_err());
        assert_eq!(execution.collective, before);
        assert_eq!(execution.dispatch_counts(), [0, 0]);
        assert_eq!(
            execution
                .ranks
                .iter()
                .map(|rank| rank.hidden)
                .collect::<Vec<_>>(),
            hidden
        );
        assert!(execution.closed);
        assert_eq!(execution.transports[0].memory.borrow().closed, [0, 1]);
    }
}

#[test]
fn capture_rejects_overwrite_intervening_dispatch_reset_and_wrong_collective() {
    let mut execution = dependency_fixture();
    let operation = Qwen3TensorParallelCollectiveV1::AttentionOutputSum;
    let commands = producers(&execution, operation);
    capture(&mut execution, operation);
    assert!(
        execution
            .dispatch_collective_producers(0, operation, |_| commands[0].clone())
            .is_err()
    );
    assert!(execution.dispatch_each(|_| commands[0].clone()).is_err());
    assert!(execution.dispatch_zero(&commands[0]).is_err());
    assert!(execution.reset_sequence().is_err());
    assert!(
        execution
            .reduce(0, Qwen3TensorParallelCollectiveV1::FeedForwardDownSum)
            .is_err()
    );
    assert!(
        execution.transports[0]
            .memory
            .borrow()
            .dependency_requests
            .is_empty()
    );
    assert_eq!(execution.dispatch_counts(), [0, 0]);
}

#[test]
fn overflow_or_live_binding_drift_rejects_before_publication() {
    for counter_overflow in [false, true] {
        let mut execution = dependency_fixture();
        let operation = Qwen3TensorParallelCollectiveV1::AttentionOutputSum;
        capture(&mut execution, operation);
        if counter_overflow {
            execution.ranks[1].dispatches = u64::MAX - 1;
        } else {
            execution.ranks[1].hidden.id += 100_000;
        }
        let before = execution.collective;
        assert!(execution.reduce(0, operation).is_err());
        assert_eq!(execution.collective, before);
        assert!(
            execution.transports[0]
                .memory
                .borrow()
                .dependency_requests
                .is_empty()
        );
    }
}

#[test]
fn profile_is_explicit_tp2_capacity16_and_exactly_one_active_row() {
    for (world, capacity) in [(1, 16), (2, 32), (8, 16)] {
        let mut execution = fixture_with_capacity(world, capacity);
        for transport in &mut execution.transports {
            transport.dependency_supported = true;
        }
        assert!(
            execution
                .configure_reduction(EngineeringTpReductionModeV3::DevicePeerDependencyV1)
                .is_err()
        );
    }
    let mut unsupported = fixture(2);
    assert!(
        unsupported
            .configure_reduction(EngineeringTpReductionModeV3::DevicePeerDependencyV1)
            .is_err()
    );
    let mut execution = dependency_fixture();
    execution.hidden.resize(8192, 0);
    let commands = producers(
        &execution,
        Qwen3TensorParallelCollectiveV1::AttentionOutputSum,
    );
    assert!(
        execution
            .dispatch_collective_producers(
                0,
                Qwen3TensorParallelCollectiveV1::AttentionOutputSum,
                |rank| commands[rank.geometry.rank as usize].clone()
            )
            .is_err()
    );
    assert!(
        execution.transports[0]
            .memory
            .borrow()
            .dependency_requests
            .is_empty()
    );
}

#[test]
fn dependency_barriers_are_capacity_only_not_kernel_dispatches() {
    let mode = EngineeringTpReductionModeV3::DevicePeerDependencyV1;
    assert_eq!(mode.extra_dispatches_per_layer(), 2);
    assert_eq!(mode.barrier_packets_per_layer(), 2);
    assert_eq!(mode.extra_dispatches_per_forward(0), 0);
    assert_eq!(mode.extra_dispatches_per_forward(1), 1);
    assert_eq!(
        36 * (15 + mode.extra_dispatches_per_layer() + mode.barrier_packets_per_layer()) + 4,
        688
    );
    assert_eq!(
        EngineeringTpReductionModeV3::DevicePeerV4.barrier_packets_per_layer(),
        0
    );
}
