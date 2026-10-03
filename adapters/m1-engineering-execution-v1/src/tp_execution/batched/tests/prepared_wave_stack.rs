//! CPU-only checks of actual recording and authenticated tensor-layout remapping.
use super::*;
use crate::tp_execution::EngineeringTp2GraphKernelProfileV1 as Kernels;

const WAVE: &str = "ferric_qwen3_tp_wave_gemv_bf16_v3";
const PARTIAL_WAVE: &str = "ferric_qwen3_tp_wave_gemv_partial_f32_v3";

fn ordinary(program: &mut Program, index: usize) -> &mut EngineeringTpDispatchV1 {
    let Step::Rank { dispatch, .. } = &mut program.steps[index] else {
        panic!("ordinary dispatch required")
    };
    dispatch
}

fn restore_projection(
    candidate: &mut EngineeringTpDispatchV1,
    control: &EngineeringTpDispatchV1,
    original_weight: Tensor,
    output: Tensor,
    partial: bool,
    grid: u32,
) {
    assert_eq!(candidate.kernel, if partial { PARTIAL_WAVE } else { WAVE });
    assert_eq!(candidate.grid_workgroups, grid);
    assert_eq!(candidate.arguments[1], original_weight.read());
    assert_ne!(candidate.arguments[1], control.arguments[1]);
    assert_eq!(candidate.arguments[2], output.write());
    assert_eq!(output.element_bytes, if partial { 4 } else { 2 });
    candidate.kernel = control.kernel;
    candidate.grid_workgroups = control.grid_workgroups;
    candidate.arguments[1] = control.arguments[1].clone();
    assert_eq!(*candidate, *control);
}

#[test]
fn wave_stack_profiles_rewrite_only_their_closed_sites_and_preserve_residency() {
    let execution = builder();
    let control = execution
        .record_graph_kernel_profile_fixture(Kernels::V22Wave)
        .unwrap();
    let saved = control.clone();
    let catalog = execution.inner.transports[0].catalog.borrow();
    let allocation_count = catalog.allocations.len();
    let allocation_bytes = catalog
        .allocations
        .iter()
        .map(|item| item.bytes)
        .sum::<usize>();
    drop(catalog);
    let transpose_bytes = execution.projection.bytes;
    for profile in Kernels::ALL
        .into_iter()
        .filter(|profile| profile.has_v15() && !profile.split_attention())
    {
        let mut candidate = execution
            .record_graph_kernel_profile_fixture(profile)
            .unwrap();
        validate_extents(
            &candidate,
            &execution.inner.transports[0].catalog.borrow().allocations,
        );
        let mut counts = [0; 5];
        for layer in 0..36 {
            let first = 2 + 28 * layer;
            for rank in 0..2 {
                let state = &execution.inner.ranks[rank];
                if profile.wave_hidden_norm() {
                    for offset in [0, 19] {
                        let index = first + offset + rank;
                        let Step::Rank {
                            dispatch: prior, ..
                        } = &control.steps[index]
                        else {
                            panic!()
                        };
                        let current = ordinary(&mut candidate, index);
                        assert_eq!(
                            current.kernel,
                            crate::tp_artifact::ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15[0]
                        );
                        current.kernel = prior.kernel;
                        assert_eq!(*current, *prior);
                        counts[0] += 1;
                    }
                }
                if profile.wave_attention() {
                    let index = first + 16 + rank;
                    let Step::Rank {
                        dispatch: prior, ..
                    } = &control.steps[index]
                    else {
                        panic!()
                    };
                    let current = ordinary(&mut candidate, index);
                    assert_eq!(current.kernel, "ferric_qwen3_tp_wave_paged_gqa_bf16_v3");
                    current.kernel = prior.kernel;
                    assert_eq!(*current, *prior);
                    counts[1] += 1;
                }
                for (enabled, offset, kind, output, grid, counter) in [
                    (
                        profile.wave_kv(),
                        4,
                        Qwen3TensorKind::KeyProjection,
                        state.k,
                        512,
                        2,
                    ),
                    (
                        profile.wave_kv(),
                        6,
                        Qwen3TensorKind::ValueProjection,
                        state.v,
                        512,
                        2,
                    ),
                    (
                        profile.wave_mlp(),
                        21,
                        Qwen3TensorKind::GateProjection,
                        state.gate,
                        6144,
                        3,
                    ),
                    (
                        profile.wave_mlp(),
                        23,
                        Qwen3TensorKind::UpProjection,
                        state.up,
                        6144,
                        3,
                    ),
                ] {
                    if !enabled {
                        continue;
                    }
                    let index = first + offset + rank;
                    let Step::Rank {
                        dispatch: prior, ..
                    } = &control.steps[index]
                    else {
                        panic!()
                    };
                    restore_projection(
                        ordinary(&mut candidate, index),
                        prior,
                        state.layers[layer].weight(kind),
                        output,
                        false,
                        grid,
                    );
                    counts[counter] += 1;
                }
            }
            if profile.wave_mlp() {
                let Step::Collective(prior) = &control.steps[first + 27] else {
                    panic!()
                };
                let Step::Collective(current) = &mut candidate.steps[first + 27] else {
                    panic!()
                };
                assert_eq!(current.consumers, prior.consumers);
                assert_eq!(current.key, prior.key);
                for (rank, producer) in current.producers.iter_mut().enumerate() {
                    let state = &execution.inner.ranks[rank];
                    restore_projection(
                        producer,
                        &prior.producers[rank],
                        state.layers[layer].weight(Qwen3TensorKind::DownProjection),
                        state.partial,
                        true,
                        4096,
                    );
                    counts[4] += 1;
                }
            }
        }
        if profile.wave_hidden_norm() {
            let Step::Rank {
                dispatch: prior, ..
            } = &control.steps[1010]
            else {
                panic!()
            };
            let current = ordinary(&mut candidate, 1010);
            assert_eq!(
                current.kernel,
                crate::tp_artifact::ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15[0]
            );
            current.kernel = prior.kernel;
            counts[0] += 1;
        }
        assert_eq!(
            counts,
            [
                if profile.wave_hidden_norm() { 145 } else { 0 },
                if profile.wave_attention() { 72 } else { 0 },
                if profile.wave_kv() { 144 } else { 0 },
                if profile.wave_mlp() { 144 } else { 0 },
                if profile.wave_mlp() { 72 } else { 0 },
            ]
        );
        assert_eq!(
            candidate, control,
            "unaccounted dispatch mutation: {profile:?}"
        );
    }
    assert_eq!(control, saved);
    assert_eq!(execution.projection.bytes, transpose_bytes);
    let catalog = execution.inner.transports[0].catalog.borrow();
    assert_eq!(catalog.allocations.len(), allocation_count);
    assert_eq!(
        catalog
            .allocations
            .iter()
            .map(|item| item.bytes)
            .sum::<usize>(),
        allocation_bytes
    );
    assert!(catalog.registered.is_none() && execution.prepared_peer.is_none());
    assert_eq!(execution.completed_batches, 0);
    assert_eq!(execution.dispatch_counts(), [0, 0]);
}

#[test]
fn wave_stack_projection_rejects_descriptor_or_tensor_identity_substitution() {
    let execution = builder();
    let baseline = execution
        .record_graph_kernel_profile_fixture(Kernels::V22Wave)
        .unwrap();
    let state = &execution.inner.ranks[0];
    for (kind, index, output) in [
        (Qwen3TensorKind::KeyProjection, 6, state.k),
        (Qwen3TensorKind::ValueProjection, 8, state.v),
        (Qwen3TensorKind::GateProjection, 23, state.gate),
        (Qwen3TensorKind::UpProjection, 25, state.up),
    ] {
        let Step::Rank {
            dispatch: prior, ..
        } = &baseline.steps[index]
        else {
            panic!()
        };
        let weight = state.layers[0].weight(kind);
        for mutation in 0..11 {
            let mut changed = prior.clone();
            match mutation {
                0 => changed.arguments[1] = state.layers[1].weight(kind).read(),
                1 => changed.arguments[1] = execution.inner.ranks[1].layers[0].weight(kind).read(),
                2 => changed.arguments[1] = weight.read(),
                3 => changed.arguments[2] = state.partial.write(),
                4 => changed.arguments[3] = EngineeringTpArgumentV1::U32(2),
                5 => changed.arguments[4] = EngineeringTpArgumentV1::U32(1),
                6 => changed.arguments[5] = EngineeringTpArgumentV1::U32(1),
                7 => changed.arguments[6] = EngineeringTpArgumentV1::U32(1),
                8 => changed.arguments[7] = EngineeringTpArgumentV1::U32(0),
                9 => changed.grid_workgroups += 1,
                _ => changed.workgroup_size = 32,
            }
            assert!(
                execution
                    .projection
                    .rowone_graph_replacement(0, kind, state.normalized, weight, output, &changed)
                    .is_err(),
                "{kind:?}, mutation {mutation}"
            );
        }
        assert!(
            execution
                .projection
                .rowone_graph_replacement(1, kind, state.normalized, weight, output, prior)
                .is_err()
        );
        assert!(
            execution
                .projection
                .rowone_graph_replacement(
                    0,
                    kind,
                    state.normalized,
                    weight,
                    Tensor {
                        element_bytes: 4,
                        ..output
                    },
                    prior
                )
                .is_err()
        );
        let missing =
            crate::tp_execution::projection::ProjectionPolicy::synthetic_mfma_ranks_for_recording(
                vec![Default::default(), Default::default()],
                0,
            );
        assert!(
            missing
                .rowone_graph_replacement(0, kind, state.normalized, weight, output, prior)
                .is_err()
        );
    }
}

#[test]
fn wave_stack_down_partial_rejects_mfma_layout_and_precision_drift() {
    let execution = builder();
    let baseline = execution
        .record_graph_kernel_profile_fixture(Kernels::V22Wave)
        .unwrap();
    let Step::Collective(down) = &baseline.steps[29] else {
        panic!()
    };
    for (rank, prior) in down.producers.iter().enumerate() {
        let state = &execution.inner.ranks[rank];
        let weight = state.layers[0].weight(Qwen3TensorKind::DownProjection);
        let replace = |output: Tensor, prior: &EngineeringTpDispatchV1| {
            execution.projection.rowone_graph_replacement(
                rank,
                Qwen3TensorKind::DownProjection,
                state.activation,
                weight,
                output,
                prior,
            )
        };
        assert!(
            replace(
                Tensor {
                    element_bytes: 2,
                    ..state.partial
                },
                prior
            )
            .is_err()
        );
        for mutation in 0..4 {
            let mut changed = prior.clone();
            match mutation {
                0 => changed.arguments[1] = weight.read(),
                1 => changed.arguments[5] = EngineeringTpArgumentV1::U32(2048),
                2 => changed.arguments[7] = EngineeringTpArgumentV1::U32(1),
                _ => changed.kernel = PARTIAL_WAVE,
            }
            assert!(replace(state.partial, &changed).is_err());
        }
    }
}
