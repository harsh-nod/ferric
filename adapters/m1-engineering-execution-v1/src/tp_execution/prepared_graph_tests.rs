use super::*;
use crate::tp_execution::{
    EngineeringTp2CollectiveRequestV1 as Collective, EngineeringTp2PreparedMetadataV1 as Metadata,
    EngineeringTpArgumentV1 as Arg, EngineeringTpBufferAccessV1 as Access,
    EngineeringTpDispatchV1 as Dispatch,
};

type Receipt = EngineeringTp2PreparedGraphReceiptV1;
type Policy = EngineeringTp2GraphPolicyV1;
type Evidence = EngineeringTp2GraphPolicyEvidenceV1;

#[test]
fn split_receipts_bind_counts_ring_reuse_and_reject_all_old_profiles() {
    let selected = EngineeringTp2GraphKernelProfileV1::WaveStackNormSplitAttentionKvMlp;
    for geometry in EngineeringTp2GraphGeometryV1::ALL {
        for epoch in [0, 63, 164, 165, 166, 2047, 2302, 2303] {
            if epoch >= u64::from(geometry.context_tokens()) {
                continue;
            }
            let (program, short, receipt) = profile_fixture(epoch, true);
            let mut input = EngineeringTp2GraphInputV1::from(&short);
            input.geometry = geometry;
            input.page_table = (0..geometry.pages())
                .map(|page| {
                    if u64::from(page) <= epoch / 16 {
                        page
                    } else {
                        u32::MAX
                    }
                })
                .collect();
            receipt
                .validate_graph_input_for_profile(
                    &program,
                    &input,
                    Policy::QueuedBaseline,
                    selected,
                )
                .unwrap();
            for wrong in EngineeringTp2GraphKernelProfileV1::ALL
                .into_iter()
                .filter(|profile| *profile != selected)
            {
                assert!(
                    receipt
                        .validate_graph_input_for_profile(
                            &program,
                            &input,
                            Policy::QueuedBaseline,
                            wrong
                        )
                        .is_err()
                );
            }
            for rank in 0..2 {
                let mut bad = receipt.clone();
                *bad.completion.ranks[rank]
                    .completion_values
                    .last_mut()
                    .unwrap() = 1;
                assert!(
                    bad.validate_graph_input_for_profile(
                        &program,
                        &input,
                        Policy::QueuedBaseline,
                        selected
                    )
                    .is_err()
                );
                let mut bad = receipt.clone();
                bad.completion.ranks[rank].final_read -= 1;
                assert!(
                    bad.validate_graph_input_for_profile(
                        &program,
                        &input,
                        Policy::QueuedBaseline,
                        selected
                    )
                    .is_err()
                );
            }
        }
    }
}

#[test]
fn split_policy_evidence_uses_9543_plus_1592_scan_congruence_only() {
    let selected = EngineeringTp2GraphKernelProfileV1::WaveStackNormSplitAttentionKvMlp;
    let (program, short, mut receipt) = profile_fixture(0, true);
    let input = EngineeringTp2GraphInputV1::from(&short);
    for loops in [9543, 11_135, 12_727] {
        receipt.policy = Evidence::TransactionFences {
            full_boundaries: 2,
            operational_boundaries: 9,
            reset_only_rounds: 1,
            loop_checks: loops,
        };
        receipt
            .validate_graph_input_for_profile(&program, &input, Policy::TransactionFences, selected)
            .unwrap();
    }
    for loops in [9111, 10_631, 9542, 9544] {
        receipt.policy = Evidence::TransactionFences {
            full_boundaries: 2,
            operational_boundaries: 9,
            reset_only_rounds: 1,
            loop_checks: loops,
        };
        assert!(
            receipt
                .validate_graph_input_for_profile(
                    &program,
                    &input,
                    Policy::TransactionFences,
                    selected
                )
                .is_err()
        );
    }
}

#[test]
fn graph_context2304_completion_keeps_absolute_frontiers_and_complete_drain_at_ring_wraps() {
    for epoch in [0, 15, 16, 63, 64, 171, 172, 173, 174, 2047, 2302, 2303] {
        let (program, short, receipt) = fixture(epoch);
        let mut input = EngineeringTp2GraphInputV1::from(&short);
        input.geometry = EngineeringTp2GraphGeometryV1::Long2304;
        input.page_table = (0..144)
            .map(|page| {
                if u64::from(page) <= epoch / 16 {
                    page
                } else {
                    u32::MAX
                }
            })
            .collect();
        receipt
            .validate_graph_input(&program, &input, Policy::QueuedBaseline)
            .unwrap();
        if epoch >= 64 {
            assert!(
                receipt
                    .validate_for(&program, &short, Policy::QueuedBaseline)
                    .is_err()
            );
        }
        for rank in 0..2 {
            let mut incomplete = receipt.clone();
            incomplete.completion.ranks[rank].final_read -= 1;
            assert!(
                incomplete
                    .validate_graph_input(&program, &input, Policy::QueuedBaseline)
                    .is_err()
            );
            let mut incomplete = receipt.clone();
            *incomplete.completion.ranks[rank]
                .completion_values
                .last_mut()
                .unwrap() = 1;
            assert!(
                incomplete
                    .validate_graph_input(&program, &input, Policy::QueuedBaseline)
                    .is_err()
            );
        }
        for length in [4, 143, 145] {
            let mut wrong = input.clone();
            wrong.page_table.resize(length, u32::MAX);
            assert!(
                receipt
                    .validate_graph_input(&program, &wrong, Policy::QueuedBaseline)
                    .is_err()
            );
        }
    }
    let (program, short, receipt) = fixture(2304);
    let mut input = EngineeringTp2GraphInputV1::from(&short);
    input.geometry = EngineeringTp2GraphGeometryV1::Long2304;
    input.page_table = (0..144).collect();
    assert!(
        receipt
            .validate_graph_input(&program, &input, Policy::QueuedBaseline)
            .is_err()
    );
    assert!(input.short_input().is_err());
}

fn slice(id: u64, elements: usize, element_bytes: u32, access: Access) -> Arg {
    Arg::Buffer {
        id,
        offset: 0,
        elements,
        element_bytes,
        access,
    }
}

fn collective(layer: u32, operation: Operation) -> Collective {
    let (k, tag) = if operation == Operation::AttentionOutputSum {
        (2048, 1)
    } else {
        (6144, 2)
    };
    Collective {
        key: Key {
            group_id: 7,
            model_role: Qwen3ModelRole::Target8B,
            epoch: 0,
            layer,
            operation,
        },
        rows: 1,
        producers: std::array::from_fn(|rank| Dispatch {
            kernel: "ferric_qwen3_tp_mfma_gemm_partial_f32_v3",
            grid_workgroups: 256,
            workgroup_size: 64,
            arguments: vec![
                slice(1 + rank as u64 * 3, 16 * k, 2, Access::Read),
                slice(2 + rank as u64 * 3, 4096 * k, 2, Access::Read),
                slice(3 + rank as u64 * 3, 16 * 4096, 4, Access::Write),
                Arg::U32(1),
                Arg::U32(4096),
                Arg::U32(k as u32),
                Arg::U32(2),
                Arg::U32(tag),
            ],
        }),
        consumers: std::array::from_fn(|rank| {
            let mut arguments = vec![
                slice(3, 4096, 4, Access::Read),
                slice(6, 4096, 4, Access::Read),
            ];
            arguments.extend([slice(3, 0, 4, Access::Read); 6]);
            arguments.extend([
                slice(7 + rank as u64 * 2, 4096, 2, Access::Read),
                slice(8 + rank as u64 * 2, 4096, 2, Access::Write),
                Arg::U32(1),
                Arg::U32(2),
            ]);
            Dispatch {
                kernel: "ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18",
                grid_workgroups: 64,
                workgroup_size: 64,
                arguments,
            }
        }),
    }
}

fn ordinary(rank: u32, kernel: &'static str) -> Step {
    Step::Rank {
        rank,
        dispatch: Dispatch {
            kernel,
            grid_workgroups: 1,
            workgroup_size: 64,
            arguments: Vec::new(),
        },
    }
}

// Accounting-only ordinary placeholders are never passed to native admission.
// Receipt packet IDs use closed per-layer formulas, not the validator's walker.
fn fixture(epoch: u64) -> (Program, Input, Receipt) {
    profile_fixture(epoch, false)
}

fn profile_fixture(epoch: u64, split: bool) -> (Program, Input, Receipt) {
    let mut steps = vec![
        ordinary(0, "ferric_qwen3_tp_batch_embedding_bf16_v2"),
        ordinary(1, "ferric_qwen3_tp_peer_copy_bf16_v4"),
    ];
    for layer in 0..36 {
        for _ in 0..if split { 10 } else { 9 } {
            steps.extend([ordinary(0, "cpu-only"), ordinary(1, "cpu-only")]);
        }
        steps.push(Step::Collective(collective(
            layer,
            Operation::AttentionOutputSum,
        )));
        for _ in 0..4 {
            steps.extend([ordinary(0, "cpu-only"), ordinary(1, "cpu-only")]);
        }
        steps.push(Step::Collective(collective(
            layer,
            Operation::FeedForwardDownSum,
        )));
    }
    steps.extend([
        ordinary(0, "cpu-only"),
        ordinary(0, "cpu-only"),
        ordinary(0, "cpu-only"),
    ]);
    let program = Program {
        group_id: 7,
        token_buffer: 20,
        result_buffer: 21,
        metadata: [Metadata {
            positions: 22,
            page_table: 23,
            cos: 24,
            sin: 25,
        }; 2],
        steps,
    };
    let input = Input {
        plan_sha256: [9; 32],
        generation: epoch + 1,
        epoch,
        token: 42,
        position: epoch as u32,
        page_table: std::array::from_fn(|slot| {
            if slot <= epoch as usize / 16 {
                slot as u32
            } else {
                u32::MAX
            }
        }),
        cos_sin: vec![0; 512],
    };
    let packets = if split { [795_u64, 793] } else { [759, 757] };
    let first = packets.map(|count| epoch * count);
    let bindings = (0_u64..72)
        .map(|index| {
            let layer = index / 2;
            let offset = layer * if split { 22 } else { 21 }
                + if index.is_multiple_of(2) {
                    if split { 11 } else { 10 }
                } else if split {
                    19
                } else {
                    18
                };
            let producers = [first[0] + offset, first[1] + offset + 1];
            let generation = epoch * 72 + index + 1;
            EngineeringTp2GraphCollectiveBindingV1 {
                key: Key {
                    group_id: 7,
                    model_role: Qwen3ModelRole::Target8B,
                    epoch,
                    layer: layer as u32,
                    operation: if index.is_multiple_of(2) {
                        Operation::AttentionOutputSum
                    } else {
                        Operation::FeedForwardDownSum
                    },
                },
                request_id: generation,
                generation,
                before_producers: if index == 0 {
                    None
                } else {
                    Some(producers.map(|value| value - 1))
                },
                producers,
                after_producers: producers.map(|value| value + 1),
                consumers: producers.map(|value| value + 2),
            }
        })
        .collect();
    let receipt = Receipt {
        plan_sha256: input.plan_sha256,
        generation: epoch + 1,
        epoch,
        position: input.position,
        input_token: 42,
        output_token: 12095,
        policy: Evidence::QueuedBaseline,
        completion: EngineeringTp2GraphCompletionV1 {
            program_id: epoch + 1,
            group_id: 7,
            epoch,
            unique_ids: [11, 22],
            graph_api_calls: 1,
            logical_steps: if split { 1085 } else { 1013 },
            rank_dispatches: if split { 1013 } else { 941 },
            kernel_counts: if split { [652, 649] } else { [616, 613] },
            barrier_counts: [143, 144],
            packet_counts: packets.map(|count| u32::try_from(count).unwrap()),
            ranks: std::array::from_fn(|rank| {
                let count = packets[rank];
                let next = (epoch + 1) * count;
                EngineeringTp2GraphRankV1 {
                    unique_id: [11, 22][rank],
                    queue_epoch: 0,
                    first_packet: first[rank],
                    next_packet: next,
                    final_write: next,
                    final_read: next,
                    completion_values: vec![0; count as usize],
                }
            }),
            embedding_copy_packets: [first[0], first[1], first[1] + 1],
            collectives: bindings,
        },
    };
    (program, input, receipt)
}

#[test]
fn graph_receipts_match_closed_form_schedule_at_page_and_context_boundaries() {
    for epoch in [0, 1, 15, 16, 17, 35, 63] {
        let (program, input, receipt) = fixture(epoch);
        receipt
            .validate_for(&program, &input, Policy::QueuedBaseline)
            .unwrap();
        assert_eq!(
            receipt.completion.collectives[0].producers,
            [epoch * 759 + 10, epoch * 757 + 11]
        );
        assert_eq!(
            receipt.completion.collectives[71].consumers,
            [epoch * 759 + 755, epoch * 757 + 756]
        );
    }
}

#[test]
fn graph_receipts_reject_each_missing_or_nonzero_completion_slot() {
    let (program, input, baseline) = fixture(1);
    for rank in 0..2 {
        for slot in 0..baseline.completion.ranks[rank].completion_values.len() {
            let mut receipt = baseline.clone();
            receipt.completion.ranks[rank].completion_values[slot] = 1;
            assert!(
                receipt
                    .validate_for(&program, &input, Policy::QueuedBaseline)
                    .is_err()
            );
        }
        let mut receipt = baseline.clone();
        receipt.completion.ranks[rank].completion_values.pop();
        assert!(
            receipt
                .validate_for(&program, &input, Policy::QueuedBaseline)
                .is_err()
        );
    }
}

#[test]
fn graph_receipts_bind_all_collective_identities_and_packet_positions() {
    let (program, input, baseline) = fixture(16);
    for index in 0..72 {
        let mutations: [fn(&mut EngineeringTp2GraphCollectiveBindingV1); 8] = [
            |binding| binding.request_id += 1,
            |binding| binding.generation += 1,
            |binding| binding.key.epoch += 1,
            |binding| binding.key.layer ^= 1,
            |binding| binding.before_producers = Some([0, 0]),
            |binding| binding.producers[0] += 1,
            |binding| binding.after_producers[1] += 1,
            |binding| binding.consumers[0] += 1,
        ];
        for mutate in mutations {
            let mut receipt = baseline.clone();
            mutate(&mut receipt.completion.collectives[index]);
            assert!(
                receipt
                    .validate_for(&program, &input, Policy::QueuedBaseline)
                    .is_err()
            );
        }
    }
}

#[test]
fn graph_receipts_reject_wrong_transport_independent_metadata_and_serial_counts() {
    let (program, input, baseline) = fixture(0);
    let mutations: [fn(&mut Receipt); 21] = [
        |r| r.plan_sha256[0] ^= 1,
        |r| r.generation += 1,
        |r| r.epoch += 1,
        |r| r.position += 1,
        |r| r.input_token += 1,
        |r| r.output_token = 151936,
        |r| r.completion.program_id += 1,
        |r| r.completion.group_id += 1,
        |r| r.completion.epoch += 1,
        |r| r.completion.unique_ids = [11, 11],
        |r| r.completion.graph_api_calls = 0,
        |r| r.completion.logical_steps -= 1,
        |r| r.completion.rank_dispatches -= 1,
        |r| r.completion.kernel_counts[0] -= 1,
        |r| r.completion.barrier_counts = [72, 72],
        |r| r.completion.packet_counts = [688, 685],
        |r| r.completion.ranks[0].queue_epoch = 1,
        |r| r.completion.ranks[1].final_read -= 1,
        |r| r.completion.ranks[1].unique_id = 11,
        |r| r.completion.embedding_copy_packets[1] += 1,
        |r| {
            r.completion.collectives.pop();
        },
    ];
    for mutate in mutations {
        let mut receipt = baseline.clone();
        mutate(&mut receipt);
        assert!(
            receipt
                .validate_for(&program, &input, Policy::QueuedBaseline)
                .is_err()
        );
    }
}

#[test]
fn graph_receipts_bind_observation_policy_and_exact_checkpoint_congruence() {
    let (program, input, mut receipt) = fixture(63);
    assert!(
        receipt
            .validate_for(&program, &input, Policy::TransactionFences)
            .is_err()
    );
    for loops in [9111, 10631, 12151] {
        receipt.policy = Evidence::TransactionFences {
            full_boundaries: 2,
            operational_boundaries: 9,
            reset_only_rounds: 1,
            loop_checks: loops,
        };
        receipt
            .validate_for(&program, &input, Policy::TransactionFences)
            .unwrap();
        assert!(
            receipt
                .validate_for(&program, &input, Policy::QueuedBaseline)
                .is_err()
        );
    }
    for (full, operational, reset, loops) in [
        (1, 9, 0, 9111),
        (2, 8, 0, 9111),
        (2, 9, 0, 9110),
        (2, 9, 0, 9112),
        (2, 9, 9112, 9111),
    ] {
        receipt.policy = Evidence::TransactionFences {
            full_boundaries: full,
            operational_boundaries: operational,
            reset_only_rounds: reset,
            loop_checks: loops,
        };
        assert!(
            receipt
                .validate_for(&program, &input, Policy::TransactionFences)
                .is_err()
        );
    }
}

#[test]
fn cached_graph_receipts_require_matching_typed_policy_and_transaction_counters() {
    let (program, input, mut receipt) = fixture(0);
    receipt.policy = Evidence::TransactionFencesAdmissionCache {
        full_boundaries: 2,
        operational_boundaries: 9,
        reset_only_rounds: 1,
        loop_checks: 9_111,
    };
    for policy in Policy::PRE_FINITE {
        assert_eq!(
            receipt.validate_for(&program, &input, policy).is_ok(),
            policy == Policy::TransactionFencesAdmissionCache
        );
    }
    for loops in [0, 9_110, 9_112, 10_630] {
        let mut invalid = receipt.clone();
        invalid.policy = Evidence::TransactionFencesAdmissionCache {
            full_boundaries: 2,
            operational_boundaries: 9,
            reset_only_rounds: 1,
            loop_checks: loops,
        };
        assert!(
            invalid
                .validate_for(&program, &input, Policy::TransactionFencesAdmissionCache)
                .is_err()
        );
    }
    receipt.policy = Evidence::TransactionFences {
        full_boundaries: 2,
        operational_boundaries: 9,
        reset_only_rounds: 1,
        loop_checks: 9_111,
    };
    assert!(
        receipt
            .validate_for(&program, &input, Policy::TransactionFencesAdmissionCache)
            .is_err()
    );
}

#[test]
fn scoped_graph_receipts_require_the_distinct_policy_not_just_admission_cache() {
    let (program, input, mut receipt) = fixture(0);
    receipt.policy = Evidence::TransactionFencesAdmissionCacheScopedObservations {
        full_boundaries: 2,
        operational_boundaries: 9,
        reset_only_rounds: 1,
        loop_checks: 9_111,
    };
    for policy in Policy::PRE_FINITE {
        assert_eq!(
            receipt.validate_for(&program, &input, policy).is_ok(),
            policy == Policy::TransactionFencesAdmissionCacheScopedObservations
        );
        assert_eq!(
            policy.scoped_operation_observations(),
            policy == Policy::TransactionFencesAdmissionCacheScopedObservations
        );
    }
    for loops in [0, 9_110, 9_112, 10_630] {
        let mut invalid = receipt.clone();
        invalid.policy = Evidence::TransactionFencesAdmissionCacheScopedObservations {
            full_boundaries: 2,
            operational_boundaries: 9,
            reset_only_rounds: 1,
            loop_checks: loops,
        };
        assert!(
            invalid
                .validate_for(
                    &program,
                    &input,
                    Policy::TransactionFencesAdmissionCacheScopedObservations
                )
                .is_err()
        );
    }
}

#[test]
fn closed_token_typed_receipts_bind_all_profiles_geometries_and_distinct_policy() {
    assert!(Policy::ClosedTokenAdmissionCache.admission_cache());
    assert!(Policy::ClosedTokenAdmissionCache.closed_token());
    assert!(!Policy::ClosedTokenAdmissionCache.scoped_operation_observations());
    for profile in EngineeringTp2GraphKernelProfileV1::ALL {
        for geometry in EngineeringTp2GraphGeometryV1::ALL {
            for epoch in [0, u64::from(geometry.context_tokens() - 1)] {
                let (program, short, mut receipt) =
                    profile_fixture(epoch, profile.split_attention());
                let mut input = EngineeringTp2GraphInputV1::from(&short);
                input.geometry = geometry;
                input.page_table = (0..geometry.pages())
                    .map(|page| {
                        if u64::from(page) <= epoch / 16 {
                            page
                        } else {
                            u32::MAX
                        }
                    })
                    .collect();
                receipt.policy = Evidence::ClosedTokenAdmissionCache {
                    full_boundaries: 2,
                    graph_operational_boundaries: 9,
                    token_boundaries: 4,
                    reset_only_rounds: 17,
                    loop_checks: if profile.split_attention() {
                        9_543
                    } else {
                        9_111
                    },
                };
                for policy in Policy::PRE_FINITE {
                    assert_eq!(
                        receipt
                            .validate_graph_input_for_profile(&program, &input, policy, profile)
                            .is_ok(),
                        policy == Policy::ClosedTokenAdmissionCache,
                    );
                }
            }
        }
    }
}

#[test]
fn closed_token_typed_counters_reject_wrong_full_graph_token_reset_and_scan_counts() {
    for profile in EngineeringTp2GraphKernelProfileV1::ALL {
        let (minimum, stride) = if profile.split_attention() {
            (9_543, 1_592)
        } else {
            (9_111, 1_520)
        };
        for (full, graph, token, reset, loops) in [
            (0, 9, 4, 0, minimum),
            (1, 9, 4, 0, minimum),
            (3, 9, 4, 0, minimum),
            (2, 8, 4, 0, minimum),
            (2, 9, 3, 0, minimum),
            (2, 9, 5, 0, minimum),
            (2, 9, 4, u32::try_from(minimum + 1).unwrap(), minimum),
            (2, 9, 4, 0, minimum - 1),
            (2, 9, 4, 0, minimum + stride - 1),
        ] {
            let evidence = Evidence::ClosedTokenAdmissionCache {
                full_boundaries: full,
                graph_operational_boundaries: graph,
                token_boundaries: token,
                reset_only_rounds: reset,
                loop_checks: loops,
            };
            assert!(
                evidence
                    .validate_for_profile(Policy::ClosedTokenAdmissionCache, profile)
                    .is_err()
            );
        }
        for loops in [minimum, minimum + stride, minimum + 2 * stride] {
            let evidence = Evidence::ClosedTokenAdmissionCache {
                full_boundaries: 2,
                graph_operational_boundaries: 9,
                token_boundaries: 4,
                reset_only_rounds: 0,
                loop_checks: loops,
            };
            evidence
                .validate_for_profile(Policy::ClosedTokenAdmissionCache, profile)
                .unwrap();
        }
    }
}

#[test]
fn graph_receipts_reject_mutated_source_order_and_out_of_window_inputs() {
    let (program, input, receipt) = fixture(0);
    for index in [0, 1, 20, 1012] {
        let mut changed = program.clone();
        changed.steps[index] = ordinary(1, "changed-cpu-only");
        assert!(
            receipt
                .validate_for(&changed, &input, Policy::QueuedBaseline)
                .is_err()
        );
    }
    let mutations: [fn(&mut Input); 5] = [
        |i| i.epoch = u64::MAX,
        |i| i.position = 64,
        |i| i.generation = 0,
        |i| i.token = 151936,
        |i| {
            i.cos_sin.pop();
        },
    ];
    for mutate in mutations {
        let mut changed = input.clone();
        mutate(&mut changed);
        assert!(
            receipt
                .validate_for(&program, &changed, Policy::QueuedBaseline)
                .is_err()
        );
    }
}
