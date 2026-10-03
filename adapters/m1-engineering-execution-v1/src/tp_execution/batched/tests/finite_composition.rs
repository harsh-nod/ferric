//! Metadata-only fixtures exercise the real recorder, not authentic weight
//! admission, native registration, GPU execution or a numerical model reference.
use super::*;
use crate::tp_execution::batched::{
    EngineeringTp2FiniteAuxiliaryKindV1 as Auxiliary,
    EngineeringTp2FiniteCompositionV1 as Contract,
    EngineeringTp2FinitePendingBufferKindV1 as Pending,
    EngineeringTp2FiniteScratchRoleV1 as Scratch, EngineeringTp2FiniteStateKindV1 as StateKind,
};
use std::collections::BTreeSet;

fn finite_fixture() -> EngineeringTpBatchExecutionV2<MetadataOnly> {
    let geometry = crate::tp_execution::EngineeringTp2GraphGeometryV1::Long2304;
    let long = EngineeringTpPagedPoolV1::new(
        pool().scope(),
        EngineeringTpPagedLimitsV1::new(geometry.context_tokens(), 32, geometry.pages(), 100)
            .unwrap(),
    )
    .unwrap();
    let mut driver = builder_for_pool(&long);
    // Only this private metadata fixture may synthesize this value. Production
    // sets it after equality with the authentic layout and verified uploads.
    driver.inner.finite_model_binding = Some(([0x42; 32], target()));
    driver
}

#[test]
fn finite_composition_records_all_layers_without_registration_or_execution() {
    let driver = finite_fixture();
    let allocations = driver.inner.transports[0]
        .catalog
        .borrow()
        .allocations
        .len();
    let contract = driver.prepare_finite_two_forward_composition_v1().unwrap();
    assert_eq!(contract.bundle_id(), &[0x42; 32]);
    assert_eq!(contract.model_id(), &[1; 32]);
    assert_eq!(contract.session(), &[2; 32]);
    assert_eq!(contract.pool_identity(), driver.pool_identity);
    assert_eq!(contract.child_identity(), 1);
    assert_eq!(
        contract.group_id(),
        driver.inner.collective.expected().group_id
    );
    assert_eq!(contract.layers().len(), 72);
    assert_eq!(contract.scratch().len(), 18);
    assert_eq!(contract.source_program().steps.len(), 1013);
    validate_extents(
        contract.source_program(),
        &driver.inner.transports[0].catalog.borrow().allocations,
    );
    assert_eq!(
        driver.inner.transports[0]
            .catalog
            .borrow()
            .allocations
            .len(),
        allocations
    );
    assert!(
        driver.inner.transports[0]
            .catalog
            .borrow()
            .registered
            .is_none()
    );
    assert!(driver.prepared_peer.is_none());
    assert_eq!(driver.dispatch_counts(), [0, 0]);
    assert_eq!(driver.completed_batches, 0);
    assert_eq!(driver.last_batch, 0);
    assert!(!driver.poisoned);
}

#[test]
fn finite_composition_retains_original_weights_and_full_distinct_caches() {
    let driver = finite_fixture();
    let contract = driver.prepare_finite_two_forward_composition_v1().unwrap();
    for layer in contract.layers() {
        let original = &driver.inner.ranks[layer.rank() as usize].layers[layer.layer() as usize];
        assert_eq!(layer.weights().len(), 11);
        for &(kind, binding) in layer.weights() {
            let tensor = original.weight(kind);
            assert_eq!(binding.id(), tensor.id);
            assert_eq!(binding.elements(), tensor.elements);
            assert_eq!(binding.element_bytes(), 2);
            assert_eq!(binding.rank(), layer.rank());
        }
        assert_eq!(layer.caches()[0].id(), original.k_cache.id);
        assert_eq!(layer.caches()[1].id(), original.v_cache.id);
        assert_ne!(layer.caches()[0].id(), layer.caches()[1].id());
        assert!(
            layer
                .caches()
                .iter()
                .all(|b| b.elements() == 2304 * 512 && b.element_bytes() == 2)
        );
    }
}

#[test]
fn finite_composition_residual_and_partial_bindings_preserve_row_one_dataflow() {
    let driver = finite_fixture();
    let contract = driver.prepare_finite_two_forward_composition_v1().unwrap();
    let crate::tp_execution::ReductionWorkspace::DevicePeer(residuals, _) = &driver.inner.reduction
    else {
        panic!()
    };
    for rank in 0..2 {
        let get = |role| {
            contract
                .scratch()
                .iter()
                .find(|(r, b)| *r == role && b.rank() == rank)
                .unwrap()
                .1
        };
        let original = &driver.inner.ranks[rank as usize];
        assert_eq!(get(Scratch::Hidden).id(), original.hidden.id);
        assert_eq!(
            get(Scratch::PostAttentionResidual).id(),
            residuals[rank as usize].id
        );
        assert_ne!(
            get(Scratch::PostAttentionResidual).id(),
            get(Scratch::Normalized).id()
        );
        assert_ne!(
            get(Scratch::PostAttentionResidual).id(),
            get(Scratch::Hidden).id()
        );
        assert_eq!(get(Scratch::Partial).id(), original.partial.id);
        assert_eq!(get(Scratch::Partial).element_bytes(), 4);
        assert_eq!(get(Scratch::Partial).elements(), 4096);
        assert_eq!(get(Scratch::Query).elements(), 2048);
        assert_eq!(get(Scratch::Gate).elements(), 6144);
    }
}

#[test]
fn finite_composition_pending_packing_is_not_an_existing_allocation() {
    let driver = finite_fixture();
    let contract = driver.prepare_finite_two_forward_composition_v1().unwrap();
    assert_eq!(contract.pending_buffers().len(), 150);
    for rank in 0..2 {
        let buffers: Vec<_> = contract
            .pending_buffers()
            .iter()
            .filter(|b| b.rank() == rank)
            .collect();
        assert_eq!(buffers.len(), 75);
        for layer in 0..36 {
            let packed: Vec<_> = buffers
                .iter()
                .filter(|b| b.layer() == Some(layer))
                .collect();
            assert_eq!(packed.len(), 2);
            assert_eq!(packed[0].kind(), Pending::PackedQkvWeight);
            assert_eq!(
                (packed[0].elements(), packed[0].element_bytes()),
                (3072 * 4096, 2)
            );
            assert_eq!(packed[1].kind(), Pending::PackedHeadNormWeight);
            assert_eq!((packed[1].elements(), packed[1].element_bytes()), (256, 2));
        }
        for (kind, elements, bytes) in [
            (Pending::QkvOutput, 3072, 2),
            (Pending::Rotary, 128, 4),
            (Pending::CacheMetadata, 145, 4),
        ] {
            let value = buffers.iter().find(|b| b.kind() == kind).unwrap();
            assert_eq!(value.layer(), None);
            assert_eq!((value.elements(), value.element_bytes()), (elements, bytes));
        }
    }
}

#[test]
fn finite_composition_has_288_unique_two_forward_typed_use_sites() {
    let driver = finite_fixture();
    let contract = driver.prepare_finite_two_forward_composition_v1().unwrap();
    assert_eq!(Contract::FORWARDS, 2);
    assert_eq!(contract.state_slots().len(), Contract::STATE_SLOTS);
    let unique: BTreeSet<_> = contract
        .state_slots()
        .iter()
        .map(|s| (s.forward(), s.layer(), s.rank(), s.kind()))
        .collect();
    assert_eq!(unique.len(), 288);
    for forward in 0..2 {
        for layer in 0..36 {
            for rank in 0..2 {
                for kind in [StateKind::PrefixV5, StateKind::MlpV1] {
                    assert!(unique.contains(&(forward, layer, rank, kind)));
                }
            }
        }
    }
    for rank in 0..2 {
        assert_eq!(
            contract
                .state_slots()
                .iter()
                .filter(|s| s.rank() == rank)
                .count(),
            144
        );
    }
    for slot in contract.state_slots() {
        assert_eq!(
            slot.atomic_words(),
            if slot.kind() == StateKind::PrefixV5 {
                22
            } else {
                11
            }
        );
    }
    assert_eq!(contract.state_slots()[0].kind(), StateKind::PrefixV5);
    assert_eq!(contract.state_slots()[2].kind(), StateKind::MlpV1);
    assert_eq!(contract.state_slots()[144].forward(), 1);
}

#[test]
fn finite_composition_rejects_unbound_model_foreign_scope_and_stale_owner() {
    for mutation in 0..12 {
        let mut driver = finite_fixture();
        match mutation {
            0 => driver.inner.finite_model_binding = None,
            1 => driver.inner.finite_model_binding = Some(([0; 32], target())),
            2 => {
                let mut config = target();
                config.config_id = ferric_spec::Identity::new([9; 32]);
                driver.inner.finite_model_binding = Some(([0x42; 32], config));
            }
            3 => driver.scope.model = [9; 32],
            4 => driver.scope.session = [0; 32],
            5 => driver.pool_identity = 0,
            6 => driver.completed_batches = 1,
            7 => driver.last_batch = 1,
            8 => driver.inner.ranks[0].dispatches = 1,
            9 => driver.inner.closed = true,
            10 => driver.poisoned = true,
            11 => driver.inner.transports[1].rank = 0,
            _ => unreachable!(),
        }
        assert!(
            driver.prepare_finite_two_forward_composition_v1().is_err(),
            "mutation {mutation}"
        );
        assert!(driver.prepared_peer.is_none());
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
fn finite_composition_rejects_incomplete_mixed_or_aliased_source_bindings() {
    for mutation in 0..12 {
        let mut driver = finite_fixture();
        match mutation {
            0 => {
                driver.inner.ranks[1].layers.pop();
            }
            1 => {
                driver.inner.ranks[0].layers[35].weights.pop();
            }
            2 => {
                let copy = driver.inner.ranks[0].layers[0].weights[0];
                driver.inner.ranks[0].layers[0].weights[1] = copy;
            }
            3 => driver.inner.ranks[1].layers[35].weights[0].1.element_bytes = 4,
            4 => driver.inner.ranks[1].layers[35].weights[10].1.elements -= 1,
            5 => {
                let cache = driver.inner.ranks[1].layers[35].k_cache;
                driver.inner.ranks[1].layers[35].v_cache = cache;
            }
            6 => driver.inner.ranks[0].layers[0].k_cache.elements = 64 * 512,
            7 => driver.inner.ranks[0].partial.element_bytes = 2,
            8 => driver.inner.ranks[0].gate = driver.inner.ranks[0].up,
            9 => driver.inner.ranks[0].hidden.id = driver.inner.ranks[0].normalized.id,
            10 => {
                driver.inner.ranks[0].globals.pop();
            }
            11 => driver.page_tables[1].elements = 64,
            _ => unreachable!(),
        }
        assert!(
            driver.prepare_finite_two_forward_composition_v1().is_err(),
            "mutation {mutation}"
        );
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
fn finite_composition_refuses_short_or_previously_registered_queued_profiles() {
    let mut short = builder();
    short.inner.finite_model_binding = Some(([0x42; 32], target()));
    assert!(short.prepare_finite_two_forward_composition_v1().is_err());
    let mut driver = finite_fixture();
    driver
        .configure_prepared_peer_graph_geometry(
            crate::tp_execution::EngineeringTp2GraphPolicyV1::TransactionFences,
            crate::tp_execution::EngineeringTp2GraphKernelProfileV1::Baseline,
            crate::tp_execution::EngineeringTp2GraphGeometryV1::Long2304,
            None,
            None,
        )
        .unwrap();
    assert!(driver.prepare_finite_two_forward_composition_v1().is_err());
    assert!(driver.prepared_peer.is_some());
    assert!(
        driver.inner.transports[0]
            .catalog
            .borrow()
            .registered
            .is_some()
    );
}

#[test]
fn finite_composition_does_not_change_queued_program_or_profile_roster() {
    use crate::tp_execution::{
        EngineeringTp2GraphGeometryV1 as Geometry, EngineeringTp2GraphKernelProfileV1 as Kernels,
    };
    let driver = finite_fixture();
    let before = driver
        .record_graph_geometry_fixture(Kernels::Baseline, Geometry::Long2304)
        .unwrap();
    let contract = driver.prepare_finite_two_forward_composition_v1().unwrap();
    assert_eq!(contract.source_program(), &before);
    assert_eq!(
        driver
            .record_graph_geometry_fixture(Kernels::Baseline, Geometry::Long2304)
            .unwrap(),
        before
    );
}

#[test]
fn finite_globals_auxiliary_and_wire_are_complete_source_owned_records() {
    use crate::finite_composition_wire as wire;
    use sha2::{Digest, Sha256};
    let driver = finite_fixture();
    let contract = driver.prepare_finite_two_forward_composition_v1().unwrap();
    assert_eq!(contract.globals().len(), 3);
    for (kind, value) in contract.globals() {
        let actual = driver.inner.ranks[0].global(*kind);
        assert_eq!(
            (
                value.id(),
                value.elements(),
                value.element_bytes(),
                value.rank()
            ),
            (actual.id, actual.elements, 2, 0)
        );
    }
    assert_eq!(contract.auxiliary().len(), 28);
    for rank in 0..2 {
        let get = |kind| {
            contract
                .auxiliary()
                .iter()
                .find(|(k, b)| *k == kind && b.rank() == rank)
                .unwrap()
                .1
        };
        let original = &driver.inner.ranks[rank as usize];
        assert_eq!(get(Auxiliary::Token).id(), original.token.id);
        assert_eq!(get(Auxiliary::Choice).id(), original.choice.id);
        assert_eq!(get(Auxiliary::Cosine).id(), original.cos.id);
        assert_eq!(get(Auxiliary::Sine).id(), original.sin.id);
        assert_eq!(
            get(Auxiliary::Positions).id(),
            driver.positions[rank as usize].id
        );
        assert_eq!(
            get(Auxiliary::PageTable).id(),
            driver.page_tables[rank as usize].id
        );
        assert_eq!(get(Auxiliary::Empty).elements(), 0);
        assert_ne!(get(Auxiliary::Empty).id(), 0);
        assert_eq!(
            get(Auxiliary::Logits).elements(),
            if rank == 0 { 2_430_976 } else { 1 }
        );
    }
    let projected = contract.wire_registration_v1().unwrap();
    projected.validate().unwrap();
    let source = contract.source_program_snapshot_v1().unwrap();
    assert_eq!(projected.source_program_bytes as usize, source.len());
    assert_eq!(
        projected.source_program_sha256,
        <[u8; 32]>::from(Sha256::digest(&source))
    );
    assert_eq!(source, contract.source_program_snapshot_v1().unwrap());
    let snapshot: serde_json::Value = serde_json::from_slice(&source).unwrap();
    assert_eq!(snapshot["steps"].as_array().unwrap().len(), 1013);
    assert_eq!(snapshot["token_buffer"], driver.inner.ranks[0].token.id);
    assert_eq!(snapshot["result_buffer"], driver.inner.ranks[0].choice.id);
    let encoded = serde_json::to_vec(&projected).unwrap();
    assert_eq!(
        serde_json::from_slice::<wire::Registration>(&encoded).unwrap(),
        projected
    );
    let request = wire::Request {
        protocol: wire::PROTOCOL,
        profile: wire::PROFILE.into(),
        device_ids: [11, 22],
        request: wire::Operation::Register {
            id: 1,
            bytes: encoded.len() as u32,
            sha256: Sha256::digest(&encoded).into(),
        },
    };
    let result = wire::refuse_unbound_request(&request, &encoded).unwrap();
    assert!(!result.native_opened && !result.gpu_execution && !result.production_authority);
    let original = projected
        .layers
        .iter()
        .flat_map(|l| l.weights.iter().map(|w| w.buffer).chain(l.caches))
        .chain(projected.scratch.iter().map(|s| s.buffer))
        .chain(projected.globals.iter().map(|g| g.buffer))
        .chain(projected.auxiliary.iter().map(|a| a.buffer))
        .collect::<Vec<_>>();
    assert_eq!(original.len(), 985);
    assert_eq!(
        original
            .iter()
            .map(|b| (b.rank, b.id))
            .collect::<BTreeSet<_>>()
            .len(),
        985
    );
    assert_eq!(original.iter().filter(|b| b.rank == 0).count() + 75, 569);
    assert_eq!(original.iter().filter(|b| b.rank == 1).count() + 75, 566);
}

#[test]
fn finite_source_boundary_join_rejects_metadata_globals_and_head_layout_substitution() {
    let driver = finite_fixture();
    let contract = driver.prepare_finite_two_forward_composition_v1().unwrap();
    driver
        .validate_finite_source_boundaries(contract.source_program())
        .unwrap();
    for mutation in 0..11 {
        let mut program = contract.source_program().clone();
        match mutation {
            0 => program.token_buffer = program.result_buffer,
            1 => program.result_buffer = program.token_buffer,
            2 => program.metadata[0].cos = program.metadata[0].sin,
            3 => program.metadata[1].positions = program.metadata[0].positions,
            4 => program.metadata[1].page_table = program.metadata[1].positions,
            5..=9 => {
                let index = [0, 1, 1010, 1011, 1012][mutation - 5];
                let Step::Rank { dispatch, .. } = &mut program.steps[index] else {
                    panic!()
                };
                dispatch.grid_workgroups += 1;
            }
            10 => {
                let Step::Rank { dispatch, .. } = &mut program.steps[1011] else {
                    panic!()
                };
                // Same-sized original NxK is not the retained queued MFMA transpose.
                dispatch.arguments[1] = driver.inner.ranks[0]
                    .global(Qwen3TensorKind::LanguageModelHead)
                    .read();
            }
            _ => unreachable!(),
        }
        assert!(
            driver.validate_finite_source_boundaries(&program).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn finite_prepare_does_not_require_or_grant_legacy_graph_capability() {
    for dependency in [false, true] {
        let mut driver = finite_fixture();
        {
            let mut catalog = driver.inner.transports[0].catalog.borrow_mut();
            catalog.deny_dependency = dependency;
            catalog.deny_graph = !dependency;
        }
        driver.prepare_finite_two_forward_composition_v1().unwrap();
        assert!(
            driver
                .configure_prepared_peer_graph_geometry(
                    crate::tp_execution::EngineeringTp2GraphPolicyV1::TransactionFences,
                    crate::tp_execution::EngineeringTp2GraphKernelProfileV1::Baseline,
                    crate::tp_execution::EngineeringTp2GraphGeometryV1::Long2304,
                    None,
                    None,
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
        assert_eq!(driver.dispatch_counts(), [0, 0]);
    }
}
