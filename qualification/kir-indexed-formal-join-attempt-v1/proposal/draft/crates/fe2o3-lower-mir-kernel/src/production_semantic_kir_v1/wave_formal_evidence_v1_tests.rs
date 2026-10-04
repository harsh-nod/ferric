use super::*;

fn fixture_module() -> kir::Module {
    let pointer = |ty, space, access| kir::Type::pointer(ty, space, access);
    let parameters = Role::ALL
        .map(|role| {
            pointer(
                if role == Role::State {
                    kir::Type::Scalar(kir::ScalarType::U32)
                } else {
                    kir::Type::Scalar(kir::ScalarType::U16)
                },
                kir::AddressSpace::Global,
                if matches!(role, Role::State | Role::Normalized | Role::KeyOutput) {
                    kir::AccessMode::ReadWrite
                } else {
                    kir::AccessMode::ReadOnly
                },
            )
        })
        .to_vec();
    let state = kir::ValueId(Role::State.field_index());
    let output = kir::ValueId(Role::Normalized.field_index());
    let global = kir::MemoryAccess::new(kir::AddressSpace::Global, 2);
    let private = kir::MemoryAccess::new(kir::AddressSpace::Private, 2);
    let workgroup = kir::MemoryAccess::new(kir::AddressSpace::Workgroup, 2);
    let def = |id, ty, kind| Operation::effect_free(kir::ValueDef::new(kir::ValueId(id), ty), kind);
    let mut block = kir::BasicBlock::new(kir::BlockId(0));
    block.operations = vec![
        def(
            13,
            kir::Type::INDEX,
            OperationKind::Constant(kir::Constant::Index(2)),
        ),
        def(
            14,
            parameters[0].clone(),
            OperationKind::GetElementPointer {
                base: kir::ValueId(0),
                offset: kir::ValueId(13),
            },
        ),
        def(
            6,
            kir::Type::Scalar(kir::ScalarType::U32),
            OperationKind::Constant(kir::Constant::U32(1)),
        ),
        def(
            7,
            kir::Type::Scalar(kir::ScalarType::U16),
            OperationKind::Load {
                pointer: kir::ValueId(14),
                access: global,
            },
        ),
        def(
            8,
            kir::Type::Scalar(kir::ScalarType::U32),
            OperationKind::Atomic(kir::Atomic {
                kind: kir::AtomicKind::Add,
                pointer: state,
                value: Some(kir::ValueId(6)),
                compare: None,
                access: kir::MemoryAccess::new(kir::AddressSpace::Global, 4),
                scope: kir::SynchronizationScope::System,
                ordering: kir::MemoryOrdering::AcquireRelease,
                failure_ordering: None,
            }),
        ),
        Operation::new(
            vec![],
            OperationKind::Store {
                pointer: output,
                value: kir::ValueId(7),
                access: global,
            },
        ),
        def(
            9,
            pointer(
                kir::Type::Scalar(kir::ScalarType::U16),
                kir::AddressSpace::Private,
                kir::AccessMode::ReadWrite,
            ),
            OperationKind::Alloca {
                element: kir::Type::Scalar(kir::ScalarType::U16),
                count: None,
                address_space: kir::AddressSpace::Private,
                alignment: 2,
            },
        ),
        Operation::new(
            vec![],
            OperationKind::Store {
                pointer: kir::ValueId(9),
                value: kir::ValueId(7),
                access: private,
            },
        ),
        def(
            10,
            kir::Type::Scalar(kir::ScalarType::U16),
            OperationKind::Load {
                pointer: kir::ValueId(9),
                access: private,
            },
        ),
        def(
            11,
            pointer(
                kir::Type::Scalar(kir::ScalarType::U16),
                kir::AddressSpace::Workgroup,
                kir::AccessMode::ReadWrite,
            ),
            OperationKind::WorkgroupMemory(kir::WorkgroupMemory {
                element: kir::Type::Scalar(kir::ScalarType::U16),
                extent: kir::WorkgroupMemoryExtent::Static(128),
                alignment: 2,
            }),
        ),
        Operation::new(
            vec![],
            OperationKind::Store {
                pointer: kir::ValueId(11),
                value: kir::ValueId(10),
                access: workgroup,
            },
        ),
        def(
            12,
            kir::Type::Scalar(kir::ScalarType::U16),
            OperationKind::Load {
                pointer: kir::ValueId(11),
                access: workgroup,
            },
        ),
        Operation::new(
            vec![],
            OperationKind::WorkgroupBarrier(kir::WorkgroupBarrier {
                memory_scope: kir::SynchronizationScope::Workgroup,
                semantics: kir::BarrierSemantics::new(
                    kir::MemoryOrdering::AcquireRelease,
                    [kir::AddressSpace::Workgroup],
                ),
                convergence: kir::Convergence::uniform(kir::SynchronizationScope::Workgroup),
            }),
        ),
    ];
    block.terminator = Some(kir::Terminator::Return { values: vec![] });
    let mut module = kir::Module::new("inert_archive");
    module.functions.push(kir::Function::kernel_entry(
        "archive",
        kir::Signature::new(parameters, vec![]),
        (0..6).map(kir::ValueId).collect(),
        vec![block],
    ));
    module.kernels.push(kir::Kernel::new(
        "archive",
        "archive",
        kir::LaunchDomain::D1 {
            x: kir::LaunchExtent::Static(128),
        },
    ));
    module.kernels[0].workgroup_size = Some(kir::WorkgroupSize::new(64, 1, 1));
    module.functions[0].required_capabilities = module.functions[0].derived_capabilities();
    module.kernels[0].required_capabilities = module.functions[0].derived_capabilities();
    module.required_capabilities = module.derived_capabilities();
    module
}
fn raw_analysis(module: &kir::Module) -> kir::FormalMemoryObligationAnalysis {
    kir::derive_kernel_memory_obligations_for_launch(
        module,
        &module.kernels[0].id,
        kir::ExplicitLaunchExtent::Exact {
            rank: 1,
            extents: [128, 1, 1],
        },
        kir::FormalIndexWidth::Bits64,
    )
    .unwrap()
}
fn join_fixture() -> (kir::Module, Vec<u8>, Layout) {
    // These are untrusted inert mapping records, never live Rust/source seals.
    let module = fixture_module();
    let raw = raw_analysis(&module);
    let receipt = RawReceipt::from_obligations(raw.obligations()).unwrap();
    assert_eq!(
        raw_receipt_length(raw.obligations()).unwrap(),
        receipt.canonical_bytes().len()
    );
    let kir = kir::encode_module_v11(&module).unwrap();
    let mut w = Writer::owning(64 * 1024).unwrap();
    let start = w.len;
    w.bytes(&kir).unwrap();
    let kir_range = start..w.len;
    let start = w.len;
    w.bytes(receipt.canonical_bytes()).unwrap();
    let receipt_range = start..w.len;
    let allocations = [6, 9];
    let start = w.len;
    for index in allocations {
        w.location(Location::new(kir::BlockId(0), index)).unwrap();
    }
    let allocation_range = start..w.len;
    let memory_indices = [3, 4, 5, 7, 8, 10, 11];
    let start = w.len;
    for index in memory_indices {
        let operation = &module.functions[0].body.as_ref().unwrap().blocks[0].operations[index];
        let (pointer, access_tag, memory_space_tag, atomic_contract) =
            memory_shape(operation).unwrap();
        write_memory(
            &mut w,
            InertWaveTaskFormalMemoryBindingV1 {
                location: Location::new(kir::BlockId(0), index),
                access_ordinal: 0,
                pointer,
                access_tag,
                memory_space_tag,
                atomic_contract,
                source_site: (memory_space_tag != 2).then_some((0, Some(index as u32), 0)),
                ranked_site: (memory_space_tag != 2).then_some((0, index as u32)),
            },
        )
        .unwrap();
    }
    let memory_range = start..w.len;
    let start = w.len;
    w.location(Location::new(kir::BlockId(0), 12)).unwrap();
    w.u32(0).unwrap();
    w.u32(12).unwrap();
    let sync_range = start..w.len;
    let roots = Role::ALL.map(|role| InertWaveTaskFormalRootV1 {
        source_argument: 0,
        source_local: 0,
        component_type: 100 + role.field_index(),
        role,
        parameter_index: role.field_index(),
        parameter: kir::ValueId(role.field_index()),
        allocation_origin: role.logical_allocation_origin(0),
    });
    let empty = w.len..w.len;
    let layout = Layout {
        profile: Profile::KeyV1,
        identity: [0; 32],
        kir_identity: published_identity(&kir).unwrap(),
        function: 0,
        status_complete: raw.is_complete(),
        rank: 1,
        global: [128, 1, 1],
        workgroup: [64, 1, 1],
        roots,
        middle: empty.clone(),
        kir: kir_range,
        receipt: receipt_range,
        reasons: empty.clone(),
        reason_count: 0,
        allocations: allocation_range,
        allocation_count: 2,
        memory: memory_range,
        memory_count: 7,
        sync: sync_range,
        sync_count: 1,
    };
    (module, w.finish().unwrap(), layout)
}
fn validate_fixture(module: &kir::Module, bytes: &[u8], layout: &Layout) -> ResultV1<()> {
    let mut work = kir::CanonicalKernelIrWorkBudgetV1::new(32 * 1024 * 1024);
    let mut budget = Budget::new(&mut work, 32 * 1024 * 1024);
    budget.charge_work(11).unwrap();
    budget.reserve_storage(7).unwrap();
    let result = validate_layout_module(bytes, layout, module, &mut budget);
    assert_eq!(budget.storage(), 7);
    result
}

#[test]
fn wave_formal_archive_census_keeps_six_roots_private_lds_atomic_and_sync() {
    let (module, bytes, layout) = join_fixture();
    validate_fixture(&module, &bytes, &layout).unwrap();
    assert_eq!(layout.roots.len(), 6);
    assert_eq!(layout.allocation_count, 2);
    assert_eq!(layout.memory_count, 7);
    assert_eq!(layout.sync_count, 1);
    let raw = raw_analysis(&module);
    assert!(!raw.obligations().inter_invocation_conflicts().is_empty());
    let receipt = RawReceipt::from_obligations(raw.obligations()).unwrap();
    assert_eq!(&bytes[layout.receipt], receipt.canonical_bytes());
}

#[test]
fn wave_formal_archive_census_rejects_each_omitted_class_and_changed_identity() {
    for mutation in 0..8 {
        let (module, mut bytes, mut layout) = join_fixture();
        match mutation {
            0 => layout.allocation_count = 1,
            1 => layout.allocation_count = 0,
            2 => layout.memory_count = 6,
            3 => layout.memory_count = 0,
            4 => layout.sync_count = 0,
            5 => layout.roots[0].parameter = layout.roots[1].parameter,
            6 => layout.roots[0].allocation_origin = layout.roots[1].allocation_origin,
            7 => bytes[layout.memory.start] = 99,
            _ => unreachable!(),
        }
        assert!(
            validate_fixture(&module, &bytes, &layout).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn wave_formal_archive_reason_codec_retains_all_seventeen_variants() {
    let module = fixture_module();
    let raw = raw_analysis(&module);
    let allocation = raw.obligations().allocations()[0].identity();
    let location = Location::new(kir::BlockId(9), 23);
    let reasons = vec![
        Reason::UnsupportedIndexWidth {
            width: kir::FormalIndexWidth::Bits32,
        },
        Reason::LaunchExtentUnknown,
        Reason::LaunchExtentZero,
        Reason::LaunchRankUnsupported { rank: 7 },
        Reason::LaunchRankMismatch {
            domain_rank: 2,
            extent_rank: 3,
        },
        Reason::LaunchExtentShapeMismatch {
            rank: 2,
            extents: [5, 7, 11],
        },
        Reason::LaunchExtentOverflow {
            rank: 3,
            extents: [u64::MAX, 2, 3],
        },
        Reason::StaticLaunchExtentMismatch {
            expected: 19,
            actual: 29,
        },
        Reason::StaticLaunchAxisExtentMismatch {
            axis: kir::Axis::Z,
            expected: 31,
            actual: 37,
        },
        Reason::CallEffectsUnavailable {
            location,
            callee: kir::FunctionId::new("callee"),
        },
        Reason::UnsupportedMemoryEffect { location },
        Reason::GuardedAccessRequiresRankedProof { location },
        Reason::UnsupportedEntryBlockParameters {
            block: kir::BlockId(41),
        },
        Reason::UnsupportedPointerDerivation {
            location,
            pointer: kir::ValueId(43),
        },
        Reason::UnsupportedIndexExpression {
            location,
            index: kir::ValueId(47),
            allocation,
        },
        Reason::ElementWidthUnavailable {
            location,
            pointer: kir::ValueId(53),
        },
        Reason::AddressArithmeticOverflow { location },
    ];
    for (tag, reason) in reasons.iter().enumerate() {
        let mut w = Writer::owning(256).unwrap();
        write_reason(&mut w, reason).unwrap();
        w.u8(0).unwrap();
        let bytes = w.finish().unwrap();
        assert_eq!(bytes[0], tag as u8);
        let mut r = Reader::new(&bytes);
        let decoded = read_reason_record(&mut r).unwrap();
        assert_eq!(decoded.covered_operation, None);
        r.finish().unwrap();
        match (tag, decoded.reason) {
            (
                0,
                InertWaveTaskFormalReasonV1::UnsupportedIndexWidth(kir::FormalIndexWidth::Bits32),
            ) => {}
            (1, InertWaveTaskFormalReasonV1::LaunchExtentUnknown) => {}
            (2, InertWaveTaskFormalReasonV1::LaunchExtentZero) => {}
            (3, InertWaveTaskFormalReasonV1::LaunchRankUnsupported(7)) => {}
            (
                4,
                InertWaveTaskFormalReasonV1::LaunchRankMismatch {
                    domain_rank: 2,
                    extent_rank: 3,
                },
            ) => {}
            (
                5,
                InertWaveTaskFormalReasonV1::LaunchExtentShapeMismatch {
                    rank: 2,
                    extents: [5, 7, 11],
                },
            ) => {}
            (
                6,
                InertWaveTaskFormalReasonV1::LaunchExtentOverflow {
                    rank: 3,
                    extents: [u64::MAX, 2, 3],
                },
            ) => {}
            (
                7,
                InertWaveTaskFormalReasonV1::StaticLaunchExtentMismatch {
                    expected: 19,
                    actual: 29,
                },
            ) => {}
            (
                8,
                InertWaveTaskFormalReasonV1::StaticLaunchAxisExtentMismatch {
                    axis: kir::Axis::Z,
                    expected: 31,
                    actual: 37,
                },
            ) => {}
            (
                9,
                InertWaveTaskFormalReasonV1::CallEffectsUnavailable {
                    location: actual,
                    callee: "callee",
                },
            ) => assert_eq!(actual, location),
            (10, InertWaveTaskFormalReasonV1::UnsupportedMemoryEffect(actual)) => {
                assert_eq!(actual, location)
            }
            (11, InertWaveTaskFormalReasonV1::GuardedAccessRequiresRankedProof(actual)) => {
                assert_eq!(actual, location)
            }
            (
                12,
                InertWaveTaskFormalReasonV1::UnsupportedEntryBlockParameters(kir::BlockId(41)),
            ) => {}
            (
                13,
                InertWaveTaskFormalReasonV1::UnsupportedPointerDerivation {
                    location: actual,
                    pointer: kir::ValueId(43),
                },
            ) => assert_eq!(actual, location),
            (
                14,
                InertWaveTaskFormalReasonV1::UnsupportedIndexExpression {
                    location: actual,
                    index: kir::ValueId(47),
                    allocation: ordinal,
                },
            ) => {
                assert_eq!(actual, location);
                assert_eq!(ordinal, allocation.parameter_index());
            }
            (
                15,
                InertWaveTaskFormalReasonV1::ElementWidthUnavailable {
                    location: actual,
                    pointer: kir::ValueId(53),
                },
            ) => assert_eq!(actual, location),
            (16, InertWaveTaskFormalReasonV1::AddressArithmeticOverflow(actual)) => {
                assert_eq!(actual, location)
            }
            _ => panic!("reason payload changed"),
        }
        for end in 0..bytes.len() {
            assert!(read_reason_record(&mut Reader::new(&bytes[..end])).is_err());
        }
    }
    assert!(read_reason_record(&mut Reader::new(&[17, 0])).is_err());
    assert!(read_reason_record(&mut Reader::new(&[0, 3, 0])).is_err());
}

#[test]
fn wave_formal_archive_keeps_full_atomic_payload_and_complex_decoder_budget() {
    let original = fixture_module();
    let mut previous = None;
    for mutation in 0..6 {
        let mut module = original.clone();
        let operation = &mut module.functions[0].body.as_mut().unwrap().blocks[0].operations[4];
        let OperationKind::Atomic(atomic) = &mut operation.kind else {
            unreachable!()
        };
        match mutation {
            0 => {}
            1 => atomic.kind = kir::AtomicKind::Exchange,
            2 => atomic.value = Some(kir::ValueId(8)),
            3 => {
                atomic.kind = kir::AtomicKind::CompareExchange;
                atomic.compare = Some(kir::ValueId(6));
                atomic.failure_ordering = Some(kir::MemoryOrdering::Acquire);
            }
            4 => atomic.access.volatile = true,
            5 => atomic.access.alignment = 8,
            _ => unreachable!(),
        }
        // Wire-only mutants are not asserted to be semantically verified.
        let bytes = kir::encode_module_v11(&module).unwrap();
        let mut work = kir::CanonicalKernelIrWorkBudgetV1::new(32 * 1024 * 1024);
        work.charge_work(11).unwrap();
        let mut budget = Budget::new(&mut work, 32 * 1024 * 1024);
        budget.reserve_storage(7).unwrap();
        let (decoded, _) =
            kir::decode_inert_published_kernel_ir_with_budget_v1(&bytes, &mut budget).unwrap();
        assert_eq!(decoded, module);
        assert_eq!(budget.storage(), 7);
        let exact_work = budget.work();
        let exact_storage = budget.peak_storage();
        let identity = published_identity(&bytes).unwrap();
        if let Some(prior) = previous {
            assert_ne!(identity, prior);
        }
        previous = Some(identity);
        for (work_limit, storage_limit) in [
            (exact_work - 1, exact_storage),
            (exact_work, exact_storage - 1),
        ] {
            let mut work = kir::CanonicalKernelIrWorkBudgetV1::new(work_limit);
            work.charge_work(11).unwrap();
            let mut budget = Budget::new(&mut work, storage_limit);
            budget.reserve_storage(7).unwrap();
            assert!(
                kir::decode_inert_published_kernel_ir_with_budget_v1(&bytes, &mut budget).is_err()
            );
            assert_eq!(budget.storage(), 7);
            assert!(budget.work() >= 11);
        }
    }
}
#[test]
fn wave_formal_v1_wire_tags_cannot_erase_qkv_profiles() {
    for (index, role) in Role::ALL.into_iter().enumerate() {
        assert_eq!(wave_v1_role_tag(role).unwrap(), index as u8);
    }
    for role in kir::SemanticWaveTaskProfileV2::QkvV2.roles() {
        assert!(matches!(wave_v1_role_tag(role), Err(E::Encoding)));
    }
}

#[test]
fn wave_formal_v1_decoder_keeps_exact_legacy_role_tags() {
    let legacy = [
        Role::Input,
        Role::NormWeight,
        Role::KeyWeight,
        Role::Normalized,
        Role::KeyOutput,
        Role::State,
    ];
    for tag in 0..=u8::MAX {
        // Three u32s, role byte, two u32s, allocation-origin u64.
        let mut bytes = [0_u8; 29];
        bytes[12] = tag;
        let mut reader = Reader::new(&bytes);
        match legacy.get(usize::from(tag)) {
            Some(&role) => {
                assert_eq!(read_root(&mut reader, Profile::KeyV1).unwrap().role, role);
                reader.finish().unwrap();
            }
            None => assert!(matches!(
                read_root(&mut reader, Profile::KeyV1),
                Err(E::Encoding)
            )),
        }
    }
}

#[test]
fn wave_formal_v1_rejects_qkv_origins_under_legacy_tags() {
    for (index, role) in kir::SemanticWaveTaskProfileV2::QkvV2
        .roles()
        .into_iter()
        .enumerate()
    {
        let (module, bytes, mut layout) = join_fixture();
        let root = &mut layout.roots[index];
        root.allocation_origin = role.logical_allocation_origin(root.source_argument);
        assert!(matches!(
            validate_fixture(&module, &bytes, &layout),
            Err(E::Correspondence)
        ));
    }
}
#[test]
fn nominal_archive_wrappers_have_exact_core_storage() {
    assert_eq!(
        size_of::<InertCanonicalWaveTaskFormalMemoryEvidenceV1>(),
        size_of::<WaveArchiveCoreV2>()
    );
    assert_eq!(
        size_of::<InertCanonicalWaveQkvTaskFormalMemoryEvidenceV2>(),
        size_of::<WaveArchiveCoreV2>()
    );
}

#[test]
fn wave_qkv_codec_accepts_only_its_six_explicit_roles() {
    let roles = Profile::QkvV2.roles();
    for tag in 0..=u8::MAX {
        let mut bytes = [0_u8; 29];
        bytes[12] = tag;
        let mut reader = Reader::new(&bytes);
        match roles.get(usize::from(tag)) {
            Some(&role) => {
                assert_eq!(read_root(&mut reader, Profile::QkvV2).unwrap().role, role);
                assert_eq!(role_tag(role, Profile::QkvV2).unwrap(), tag);
                assert!(role_tag(role, Profile::KeyV1).is_err());
                reader.finish().unwrap();
            }
            None => assert!(read_root(&mut reader, Profile::QkvV2).is_err()),
        }
    }
    for &role in Profile::KeyV1.roles() {
        assert!(role_tag(role, Profile::QkvV2).is_err());
    }
}

include!("wave_formal_evidence_post_v3_tests.rs");
include!("wave_formal_evidence_post_join_v3_tests.rs");
include!("wave_formal_evidence_attention_v4_tests.rs");
include!("wave_formal_evidence_attention_output_v5_tests.rs");
include!("wave_formal_evidence_mlp_v1_tests.rs");
include!("wave_formal_evidence_mlp_tiles_v2_tests.rs");
include!("multiwave_join_formal_evidence_v1_tests.rs");
include!("prefix_tile_formal_evidence_v6_tests.rs");
include!("wave_formal_evidence_index_v1_tests.rs");
