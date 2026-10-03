//! Real parent recording/commit paths with CPU-only graph completion fixtures.
//! No allocation backing, GPU submission, model arithmetic, or numerical claim.

use super::*;
use crate::tp_execution::{
    EngineeringTp2GraphCollectiveBindingV1 as Binding,
    EngineeringTp2GraphCompletionV1 as Completion, EngineeringTp2GraphPolicyEvidenceV1 as Evidence,
    EngineeringTp2GraphPolicyV1 as Policy, EngineeringTp2GraphRankV1 as GraphRank,
    EngineeringTp2PreparedGraphReceiptV1 as GraphReceipt, EngineeringTp2PreparedInputV1 as Input,
};
use std::panic::{AssertUnwindSafe, catch_unwind};

const PLAN: [u8; 32] = [0x51; 32];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fault {
    None,
    RegistrationError,
    RegistrationUnwind,
    ExecutionError,
    ExecutionUnwind,
    LastCompletion,
    LastBinding,
    StaleGeneration,
    WrongPolicy,
    InvalidChoice,
    InvalidAndCloseError,
}

struct Control {
    policy: Policy,
    fault: Fault,
    registrations: usize,
    inputs: Vec<Input>,
    closes: Vec<u32>,
}

struct GraphMock {
    metadata: MetadataOnly,
    control: Rc<RefCell<Control>>,
}

impl EngineeringTpRankTransportV1 for GraphMock {
    fn peer_group_rank(&self) -> Option<(u32, u32, u32)> {
        self.metadata.peer_group_rank()
    }
    fn supports_peer_dependency_collectives(&self) -> bool {
        true
    }
    fn supports_prepared_peer_graph(&self, policy: Policy) -> bool {
        self.control.borrow().policy == policy
    }
    fn allocate(&mut self, bytes: usize) -> TpResult<u64> {
        self.metadata.allocate(bytes)
    }
    fn allocate_peer_readable(&mut self, bytes: usize) -> TpResult<u64> {
        self.metadata.allocate_peer_readable(bytes)
    }
    fn register_prepared_peer_graph(
        &mut self,
        program: &Program,
        policy: Policy,
    ) -> TpResult<[u8; 32]> {
        let fault = {
            let mut control = self.control.borrow_mut();
            assert_eq!(policy, control.policy);
            control.registrations += 1;
            control.fault
        };
        // The existing fixture validates every actual recorded buffer extent.
        // Faults happen after sealing, when retrying would be ambiguous.
        let hash = self.metadata.register_prepared_peer(program)?;
        match fault {
            Fault::RegistrationError => Err("graph registration injected error".into()),
            Fault::RegistrationUnwind => panic!("graph registration injected unwind"),
            _ => Ok(hash),
        }
    }
    fn execute_prepared_peer_graph(&mut self, input: &Input) -> TpResult<GraphReceipt> {
        assert_eq!(self.metadata.rank, 0);
        let (policy, fault) = {
            let mut control = self.control.borrow_mut();
            control.inputs.push(input.clone());
            (control.policy, control.fault)
        };
        match fault {
            Fault::ExecutionError => return Err("graph execution injected error".into()),
            Fault::ExecutionUnwind => panic!("graph execution injected unwind"),
            _ => {}
        }
        let catalog = self.metadata.catalog.borrow();
        let program = catalog.registered.as_ref().unwrap();
        let mut receipt = completion_fixture(program, input, policy);
        match fault {
            Fault::LastCompletion | Fault::InvalidAndCloseError => {
                *receipt.completion.ranks[1]
                    .completion_values
                    .last_mut()
                    .unwrap() = 1;
            }
            Fault::LastBinding => receipt.completion.collectives[71].consumers[1] += 1,
            Fault::StaleGeneration => receipt.generation -= 1,
            Fault::WrongPolicy => receipt.policy = policy_evidence(other_policy(policy)),
            Fault::InvalidChoice => receipt.output_token = 151_936,
            _ => {}
        }
        Ok(receipt)
    }
    fn write(&mut self, id: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        self.metadata.write(id, offset, bytes)
    }
    fn read(&mut self, id: u64, offset: usize, bytes: &mut [u8]) -> TpResult<()> {
        self.metadata.read(id, offset, bytes)
    }
    fn submit(&mut self, dispatch: &EngineeringTpDispatchV1) -> TpResult<()> {
        self.metadata.submit(dispatch)
    }
    fn wait(&mut self) -> TpResult<()> {
        self.metadata.wait()
    }
    fn close(&mut self) -> TpResult<()> {
        let mut control = self.control.borrow_mut();
        control.closes.push(self.metadata.rank);
        if control.fault == Fault::InvalidAndCloseError {
            Err("graph close injected error".into())
        } else {
            Ok(())
        }
    }
}

fn other_policy(policy: Policy) -> Policy {
    match policy {
        Policy::QueuedBaseline => Policy::TransactionFences,
        Policy::TransactionFences => Policy::QueuedBaseline,
        Policy::TransactionFencesAdmissionCache => Policy::TransactionFences,
        Policy::TransactionFencesAdmissionCacheScopedObservations => {
            Policy::TransactionFencesAdmissionCache
        }
        Policy::ClosedTokenAdmissionCache | Policy::FiniteRequestAdmissionCache => {
            Policy::TransactionFencesAdmissionCache
        }
    }
}

fn policy_evidence(policy: Policy) -> Evidence {
    match policy {
        Policy::QueuedBaseline => Evidence::QueuedBaseline,
        Policy::TransactionFences => Evidence::TransactionFences {
            full_boundaries: 2,
            operational_boundaries: 9,
            reset_only_rounds: 7,
            loop_checks: 10_631,
        },
        Policy::TransactionFencesAdmissionCache => Evidence::TransactionFencesAdmissionCache {
            full_boundaries: 2,
            operational_boundaries: 9,
            reset_only_rounds: 7,
            loop_checks: 10_631,
        },
        Policy::TransactionFencesAdmissionCacheScopedObservations => {
            Evidence::TransactionFencesAdmissionCacheScopedObservations {
                full_boundaries: 2,
                operational_boundaries: 9,
                reset_only_rounds: 7,
                loop_checks: 10_631,
            }
        }
        Policy::FiniteRequestAdmissionCache => panic!("finite fixture is separate"),
        Policy::ClosedTokenAdmissionCache => Evidence::ClosedTokenAdmissionCache {
            full_boundaries: 2,
            graph_operational_boundaries: 9,
            token_boundaries: 4,
            reset_only_rounds: 7,
            loop_checks: 10_631,
        },
    }
}

// Closed packet formulas deliberately differ from the production receipt walker.
// These are fabricated CPU protocol inputs, never native execution evidence.
fn completion_fixture(program: &Program, input: &Input, policy: Policy) -> GraphReceipt {
    let first = [input.epoch * 759, input.epoch * 757];
    let collectives = program
        .steps
        .iter()
        .filter_map(|step| match step {
            Step::Collective(value) => Some(value),
            Step::Rank { .. } => None,
        })
        .enumerate()
        .map(|(index, template)| {
            let index = u64::try_from(index).unwrap();
            let offset = 10 + 21 * (index / 2) + 8 * (index % 2);
            let producers = [first[0] + offset, first[1] + offset + 1];
            let generation = input.epoch * 72 + index + 1;
            let mut key = template.key;
            key.epoch = input.epoch;
            Binding {
                key,
                request_id: generation,
                generation,
                before_producers: (index != 0).then(|| producers.map(|packet| packet - 1)),
                producers,
                after_producers: producers.map(|packet| packet + 1),
                consumers: producers.map(|packet| packet + 2),
            }
        })
        .collect();
    GraphReceipt {
        plan_sha256: input.plan_sha256,
        generation: input.generation,
        epoch: input.epoch,
        position: input.position,
        input_token: input.token,
        output_token: input.token + 1,
        completion: Completion {
            program_id: input.generation,
            group_id: program.group_id,
            epoch: input.epoch,
            unique_ids: [11, 22],
            graph_api_calls: 1,
            logical_steps: 1013,
            rank_dispatches: 941,
            kernel_counts: [616, 613],
            barrier_counts: [143, 144],
            packet_counts: [759, 757],
            ranks: std::array::from_fn(|rank| {
                let count = [759, 757][rank];
                let next = input.generation * count;
                GraphRank {
                    unique_id: [11, 22][rank],
                    queue_epoch: 0,
                    first_packet: first[rank],
                    next_packet: next,
                    final_write: next,
                    final_read: next,
                    completion_values: vec![0; usize::try_from(count).unwrap()],
                }
            }),
            embedding_copy_packets: [first[0], first[1], first[1] + 1],
            collectives,
        },
        policy: policy_evidence(policy),
    }
}

// Reuse the existing production-sized metadata fixture without copying its
// allocation/weight/transposition recipe or allocating tensor backing memory.
fn graph_builder(
    pool: &EngineeringTpPagedPoolV1,
    policy: Policy,
) -> EngineeringTpBatchExecutionV2<GraphMock> {
    let old = builder_for_pool(pool);
    let inner = old.inner;
    let control = Rc::new(RefCell::new(Control {
        policy,
        fault: Fault::None,
        registrations: 0,
        inputs: Vec::new(),
        closes: Vec::new(),
    }));
    EngineeringTpBatchExecutionV2 {
        inner: EngineeringTpExecutionV1 {
            finite_model_binding: None,
            transports: inner
                .transports
                .into_iter()
                .map(|metadata| GraphMock {
                    metadata,
                    control: control.clone(),
                })
                .collect(),
            ranks: inner.ranks,
            plan: inner.plan,
            sequence: inner.sequence,
            collective: inner.collective,
            capacity: inner.capacity,
            row_capacity: inner.row_capacity,
            large_kv: inner.large_kv,
            draft_v10: inner.draft_v10,
            hidden: inner.hidden,
            reduction: inner.reduction,
            sequences: inner.sequences,
            ordered_batches: inner.ordered_batches,
            full_forward_enabled: inner.full_forward_enabled,
            full_forward: inner.full_forward,
            peer_dependency_pending: inner.peer_dependency_pending,
            timing: inner.timing,
            closed: inner.closed,
        },
        row_capacity: old.row_capacity,
        positions: old.positions,
        page_tables: old.page_tables,
        scope: old.scope,
        pool_identity: old.pool_identity,
        context_tokens: old.context_tokens,
        physical_pages: old.physical_pages,
        table_stride: old.table_stride,
        last_batch: old.last_batch,
        completed_batches: old.completed_batches,
        poisoned: old.poisoned,
        prune_output_head: old.prune_output_head,
        projection: old.projection,
        projection_configured: old.projection_configured,
        c1_wave_layers: old.c1_wave_layers,
        wave_attention: old.wave_attention,
        numerical: old.numerical,
        head_profile_configured: old.head_profile_configured,
        fp32_logits: old.fp32_logits,
        fp32_argmax_v11: old.fp32_argmax_v11,
        admitted_argmax_v11: old.admitted_argmax_v11,
        query_hoist_v14: old.query_hoist_v14,
        admitted_query_hoist_v14: old.admitted_query_hoist_v14,
        wave_rmsnorm_v15: old.wave_rmsnorm_v15,
        admitted_wave_rmsnorm_v15: old.admitted_wave_rmsnorm_v15,
        parallel_kv_v16: old.parallel_kv_v16,
        admitted_parallel_kv_v16: old.admitted_parallel_kv_v16,
        prepared_peer: old.prepared_peer,
        split_attention_scratch: old.split_attention_scratch,
    }
}

#[derive(Debug, PartialEq)]
struct HostSnapshot {
    cursor: ferric_engine::tensor_parallel::Qwen3TensorParallelCollectiveKeyV1,
    dispatches: Vec<u64>,
    completed_batches: u64,
    last_batch: u64,
    hidden: Vec<u16>,
    rank_hidden: Vec<Tensor>,
    registered: bool,
}

fn snapshot(driver: &EngineeringTpBatchExecutionV2<GraphMock>) -> HostSnapshot {
    HostSnapshot {
        cursor: driver.inner.collective.expected(),
        dispatches: driver.dispatch_counts(),
        completed_batches: driver.completed_batches,
        last_batch: driver.last_batch,
        hidden: driver.inner.hidden.clone(),
        rank_hidden: driver.inner.ranks.iter().map(|rank| rank.hidden).collect(),
        registered: driver.prepared_peer.is_some(),
    }
}

fn registered_program(driver: &EngineeringTpBatchExecutionV2<GraphMock>) -> Program {
    driver.inner.transports[0]
        .metadata
        .catalog
        .borrow()
        .registered
        .clone()
        .unwrap()
}

#[test]
fn graph_actual_builder_success_commits_both_policies_through_page_boundary() {
    for policy in Policy::PRE_FINITE {
        let mut pool = pool();
        let mut driver = graph_builder(&pool, policy);
        let before = snapshot(&driver);
        driver.configure_prepared_peer_graph(policy).unwrap();
        let mut after_registration = snapshot(&driver);
        assert!(after_registration.registered);
        after_registration.registered = false;
        assert_eq!(after_registration, before);
        let program = registered_program(&driver);
        assert_eq!(program.steps.len(), 1013);
        let control = driver.inner.transports[0].control.clone();
        assert!(control.borrow().inputs.is_empty());
        assert_eq!(control.borrow().registrations, 1);
        assert!(!driver.poisoned);
        let sequence = pool
            .open_sequence(pool.scope(), &[42], 0)
            .unwrap()
            .sequence();
        for position in 0..18 {
            let token = 42 + position;
            let batch = pool
                .reserve_batch(&[EngineeringTpPageRowV1 {
                    sequence,
                    token,
                    position,
                }])
                .unwrap();
            let pages = batch.rows()[0].physical_pages().to_vec();
            pool.begin_submission(&batch).unwrap();
            let selected: &[usize] = if position == 0 { &[] } else { &[0] };
            let result = driver.execute_selected(&batch, selected).unwrap();
            assert_eq!(
                result.choices,
                if selected.is_empty() {
                    vec![]
                } else {
                    vec![token + 1]
                }
            );
            assert_eq!(pool.committed_position(sequence).unwrap(), position);
            assert_eq!(driver.completed_batches, u64::from(position) + 1);
            assert_eq!(driver.last_batch, batch.id());
            assert_eq!(
                driver.dispatch_counts(),
                [616, 613].map(|count| count * (u64::from(position) + 1))
            );
            assert_eq!(
                driver.inner.collective.expected().epoch,
                u64::from(position) + 1
            );
            assert!(!driver.poisoned && !driver.inner.closed);
            let input = control.borrow().inputs.last().unwrap().clone();
            assert_eq!(input.plan_sha256, PLAN);
            assert_eq!(input.generation, u64::from(position) + 1);
            assert_eq!(input.epoch, u64::from(position));
            assert_eq!(input.position, position);
            assert_eq!(input.token, token);
            assert_eq!(&input.page_table[..pages.len()], pages);
            assert!(
                input.page_table[pages.len()..]
                    .iter()
                    .all(|page| *page == u32::MAX)
            );
            let (mut rope, sin) = crate::tp_execution::rope_bytes(position, target().rope_theta);
            rope.extend(sin);
            assert_eq!(input.cos_sin, rope);
            assert_eq!(registered_program(&driver), program);
            pool.commit_batch(&batch, result.completion).unwrap();
            assert_eq!(pool.committed_position(sequence).unwrap(), position + 1);
        }
        assert_eq!(control.borrow().inputs.len(), 18);
        assert_eq!(control.borrow().registrations, 1);
        assert!(control.borrow().closes.is_empty());
    }
}

#[test]
fn graph_actual_builder_registration_error_or_unwind_is_terminal_after_seal() {
    for policy in Policy::PRE_FINITE {
        for fault in [Fault::RegistrationError, Fault::RegistrationUnwind] {
            let mut driver = graph_builder(&pool(), policy);
            let control = driver.inner.transports[0].control.clone();
            control.borrow_mut().fault = fault;
            let before = snapshot(&driver);
            let result = catch_unwind(AssertUnwindSafe(|| {
                driver.configure_prepared_peer_graph(policy)
            }));
            match fault {
                Fault::RegistrationUnwind => assert!(result.is_err()),
                _ => assert!(result.unwrap().is_err()),
            }
            assert!(driver.poisoned);
            assert_eq!(snapshot(&driver), before);
            assert_eq!(control.borrow().registrations, 1);
            assert!(
                driver.inner.transports[0]
                    .metadata
                    .catalog
                    .borrow()
                    .registered
                    .is_some()
            );
            control.borrow_mut().fault = Fault::None;
            assert!(driver.configure_prepared_peer_graph(policy).is_err());
            assert!(
                driver
                    .configure_prepared_peer_graph(other_policy(policy))
                    .is_err()
            );
            assert!(driver.configure_prepared_peer().is_err());
            assert_eq!(control.borrow().registrations, 1);
            assert!(control.borrow().inputs.is_empty());
        }
    }
}

fn failed_forward_has_no_commit(policy: Policy, fault: Fault, successful_prefix: u32) {
    let mut pool = pool();
    let mut driver = graph_builder(&pool, policy);
    let control = driver.inner.transports[0].control.clone();
    driver.configure_prepared_peer_graph(policy).unwrap();
    let program = registered_program(&driver);
    let sequence = pool
        .open_sequence(pool.scope(), &[42], 0)
        .unwrap()
        .sequence();
    for position in 0..successful_prefix {
        let batch = pool
            .reserve_batch(&[EngineeringTpPageRowV1 {
                sequence,
                token: 42 + position,
                position,
            }])
            .unwrap();
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[0]).unwrap();
        pool.commit_batch(&batch, output.completion).unwrap();
    }
    let before = snapshot(&driver);
    let batch = pool
        .reserve_batch(&[EngineeringTpPageRowV1 {
            sequence,
            token: 42 + successful_prefix,
            position: successful_prefix,
        }])
        .unwrap();
    let pages = batch.rows()[0].physical_pages().to_vec();
    pool.begin_submission(&batch).unwrap();
    control.borrow_mut().fault = fault;
    let result = catch_unwind(AssertUnwindSafe(|| driver.execute_selected(&batch, &[0])));
    match fault {
        Fault::ExecutionUnwind => assert!(result.is_err()),
        _ => {
            let error = result.unwrap().err().expect("injected graph failure");
            if fault == Fault::InvalidAndCloseError {
                assert!(error.contains("graph close injected error"));
            }
            assert!(driver.inner.closed);
            assert_eq!(control.borrow().closes, [0, 1]);
        }
    }
    assert!(driver.poisoned);
    assert_eq!(snapshot(&driver), before);
    assert_eq!(registered_program(&driver), program);
    assert_eq!(
        pool.committed_position(sequence).unwrap(),
        successful_prefix
    );
    assert_eq!(batch.rows()[0].physical_pages(), pages);
    let submitted = control.borrow().inputs.len();
    assert_eq!(submitted, usize::try_from(successful_prefix).unwrap() + 1);
    control.borrow_mut().fault = Fault::None;
    assert!(driver.execute_selected(&batch, &[0]).is_err());
    assert!(driver.configure_prepared_peer_graph(policy).is_err());
    assert!(driver.configure_prepared_peer().is_err());
    assert_eq!(control.borrow().inputs.len(), submitted);
    assert_eq!(snapshot(&driver), before);
    pool.quarantine_batch(&batch).unwrap();
}

#[test]
fn graph_actual_builder_transport_error_or_unwind_preserves_initial_and_late_state() {
    for policy in Policy::PRE_FINITE {
        for fault in [Fault::ExecutionError, Fault::ExecutionUnwind] {
            for successful_prefix in [0, 2] {
                failed_forward_has_no_commit(policy, fault, successful_prefix);
            }
        }
    }
}

#[test]
fn graph_actual_builder_late_invalid_receipt_cannot_commit_or_resume() {
    for policy in Policy::PRE_FINITE {
        for fault in [
            Fault::LastCompletion,
            Fault::LastBinding,
            Fault::StaleGeneration,
            Fault::WrongPolicy,
            Fault::InvalidChoice,
            Fault::InvalidAndCloseError,
        ] {
            failed_forward_has_no_commit(policy, fault, 2);
        }
    }
}

#[test]
fn graph_actual_builder_rejects_policy_changes_and_reregistration_without_calls() {
    for policy in Policy::PRE_FINITE {
        let mut driver = graph_builder(&pool(), policy);
        let control = driver.inner.transports[0].control.clone();
        assert!(
            driver
                .configure_prepared_peer_graph(other_policy(policy))
                .is_err()
        );
        assert!(driver.configure_prepared_peer().is_err());
        assert_eq!(control.borrow().registrations, 0);
        driver.configure_prepared_peer_graph(policy).unwrap();
        let before = snapshot(&driver);
        assert!(driver.configure_prepared_peer_graph(policy).is_err());
        assert!(
            driver
                .configure_prepared_peer_graph(other_policy(policy))
                .is_err()
        );
        assert!(driver.configure_prepared_peer().is_err());
        assert_eq!(snapshot(&driver), before);
        assert!(!driver.poisoned);
        assert_eq!(control.borrow().registrations, 1);
        assert!(control.borrow().inputs.is_empty());
    }
}
