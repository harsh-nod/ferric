// Exact retained pre-candidate inert validator; test-only differential oracle.
fn legacy_validate_layout_module_v1<const N: usize>(
    bytes: &[u8],
    layout: &Layout<N>,
    module: &kir::Module,
    budget: &mut Budget<'_>,
) -> ResultV1<()> {
    let function = module
        .functions
        .get(layout.function as usize)
        .ok_or(E::Correspondence)?;
    let body = function.body.as_ref().ok_or(E::Correspondence)?;
    let kir_length = layout.kir.len();
    budget.charge_work(mul(kir_length, 16)?)?;
    let rows = add(
        add(layout.reason_count, layout.allocation_count)?,
        add(layout.memory_count, layout.sync_count)?,
    )?;
    budget.charge_work(join_work(function, rows)?)?;
    // Unknown complete-effect summaries may allocate a small effect vector.
    // Reserve actual wire-sized temporary space before any such query.
    let scratch = add(mul(kir_length, 16)?, 1024)?;
    budget.reserve_storage(scratch)?;
    let result = (|| {
        if body.parameters.len() != N || function.signature.parameters.len() != N {
            return Err(E::Correspondence);
        }
        for (index, block) in body.blocks.iter().enumerate() {
            if body.blocks[..index]
                .iter()
                .any(|prior| prior.id == block.id)
            {
                return Err(E::Correspondence);
            }
        }
        for root in &layout.roots {
            let index = root.parameter_index as usize;
            let expected_access = if root.role.is_read_only() {
                kir::AccessMode::ReadOnly
            } else {
                kir::AccessMode::ReadWrite
            };
            if root.role.profile() != layout.profile
                || body.parameters.get(index) != Some(&root.parameter)
                || !matches!(function.signature.parameters.get(index), Some(kir::Type::Pointer(pointer))
                    if pointer.address_space == kir::AddressSpace::Global
                        && *pointer.pointee == kir::Type::Scalar(root.role.scalar())
                        && pointer.access == expected_access)
                || root.allocation_origin
                    != root.role.logical_allocation_origin(root.source_argument)
            {
                return Err(E::Correspondence);
            }
        }
        validate_raw_binding(bytes, layout, module, function)?;
        let mut allocations = Reader::new(&bytes[layout.allocations.clone()]);
        let mut memory = Reader::new(&bytes[layout.memory.clone()]);
        let mut sync = Reader::new(&bytes[layout.sync.clone()]);
        let (mut allocations_seen, mut memory_seen, mut sync_seen) = (0usize, 0usize, 0usize);
        for block in &body.blocks {
            for (index, operation) in block.operations.iter().enumerate() {
                let location = Location::new(block.id, index);
                if !operation.has_complete_effect_summary() {
                    return Err(E::Correspondence);
                }
                if internal_allocation(operation)? {
                    if allocations_seen >= layout.allocation_count
                        || allocations.location()? != location
                    {
                        return Err(E::Correspondence);
                    }
                    allocations_seen += 1;
                } else if is_memory(operation) {
                    if memory_seen >= layout.memory_count {
                        return Err(E::Correspondence);
                    }
                    let row = read_memory(&mut memory)?;
                    if row.location != location
                        || row.access_ordinal != 0
                        || memory_shape(operation)?
                            != (
                                row.pointer,
                                row.access_tag,
                                row.memory_space_tag,
                                row.atomic_contract,
                            )
                    {
                        return Err(E::Correspondence);
                    }
                    let kir::Type::Pointer(pointer) = pointer_type(function, row.pointer)? else {
                        return Err(E::Correspondence);
                    };
                    if kir_space(pointer.address_space)? != row.memory_space_tag {
                        return Err(E::Correspondence);
                    }
                    memory_seen += 1;
                } else if is_sync(operation) {
                    if sync_seen >= layout.sync_count || read_sync(&mut sync)?.location != location
                    {
                        return Err(E::Correspondence);
                    }
                    sync_seen += 1;
                } else if !operation.memory_effects().is_empty() {
                    return Err(E::Correspondence);
                }
            }
        }
        if (allocations_seen, memory_seen, sync_seen)
            != (
                layout.allocation_count,
                layout.memory_count,
                layout.sync_count,
            )
        {
            return Err(E::Correspondence);
        }
        allocations.finish()?;
        memory.finish()?;
        sync.finish()?;
        let mut reasons = Reader::new(&bytes[layout.reasons.clone()]);
        for _ in 0..layout.reason_count {
            let record = read_reason_record(&mut reasons)?;
            if record.covered_operation != reason_location(record.reason) {
                return Err(E::Correspondence);
            }
            if let Some(location) = record.covered_operation {
                operation_at(module, layout.function as usize, location)?;
            }
        }
        reasons.finish()?;
        Ok(())
    })();
    budget.release_storage(scratch)?;
    result
}


fn indexed_probe_v1<const N: usize>(
    module: &kir::Module,
    bytes: &[u8],
    layout: &Layout<N>,
    work_limit: usize,
    storage_limit: usize,
) -> (ResultV1<()>, usize, usize) {
    let mut work = kir::CanonicalKernelIrWorkBudgetV1::new(work_limit);
    let mut budget = Budget::new(&mut work, storage_limit);
    budget.charge_work(11).unwrap();
    budget.reserve_storage(7).unwrap();
    let result = validate_layout_module(bytes, layout, module, &mut budget);
    assert_eq!(budget.storage(), 7);
    (result, budget.work(), budget.peak_storage())
}

fn indexed_differential_v1<const N: usize>(
    module: &kir::Module,
    bytes: &[u8],
    layout: &Layout<N>,
) -> bool {
    let original = module.clone();
    let original_bytes = bytes.to_vec();
    let mut work = kir::CanonicalKernelIrWorkBudgetV1::new(usize::MAX);
    let mut budget = Budget::new(&mut work, 128 * 1024 * 1024);
    budget.charge_work(11).unwrap();
    budget.reserve_storage(7).unwrap();
    let old = legacy_validate_layout_module_v1(bytes, layout, module, &mut budget);
    assert_eq!(budget.storage(), 7);
    let (new, _, _) = indexed_probe_v1(module, bytes, layout, usize::MAX, 128 * 1024 * 1024);
    assert_eq!(
        old.as_ref().err().map(|error| format!("{error:?}")),
        new.as_ref().err().map(|error| format!("{error:?}"))
    );
    assert_eq!(*module, original);
    assert_eq!(bytes, original_bytes);
    new.is_ok()
}

fn compare_index_lookups_v1(function: &kir::Function, ids: &[u32]) {
    let mut work = kir::CanonicalKernelIrWorkBudgetV1::new(32 * 1024 * 1024);
    let mut budget = Budget::new(&mut work, 32 * 1024 * 1024);
    budget.reserve_storage(7).unwrap();
    let index = InertJoinIndexV1::build(function, &mut budget).unwrap();
    for id in ids {
        let legacy = pointer_type(function, kir::ValueId(*id));
        let indexed = index.pointer_type(kir::ValueId(*id), &mut budget);
        assert_eq!(legacy.is_ok(), indexed.is_ok());
        if let (Ok(legacy), Ok(indexed)) = (legacy, indexed) {
            assert_eq!(legacy, indexed);
            assert!(std::ptr::eq(legacy, indexed));
        }
    }
    drop(index);
    release_to(&mut budget, 7).unwrap();
    assert_eq!(budget.storage(), 7);
}

#[test]
fn indexed_formal_join_preserves_six_root_census() {
    let (module, bytes, layout) = join_fixture();
    assert!(indexed_differential_v1(&module, &bytes, &layout));
}

#[test]
fn indexed_formal_join_preserves_fifteen_root_prefix_census() {
    let (module, bytes, layout) = prefix_tile_fixture_v6();
    assert!(indexed_differential_v1(&module, &bytes, &layout));
}

#[test]
fn indexed_formal_join_handles_sparse_nonmonotonic_blocks_and_values() {
    let (mut module, bytes, layout) = join_fixture();
    let body = module.functions[0].body.as_mut().unwrap();
    for id in [u32::MAX, 1000, 17] {
        let mut block = kir::BasicBlock::new(kir::BlockId(id));
        block.parameters.push(kir::ValueDef::new(kir::ValueId(id), kir::Type::Scalar(kir::ScalarType::U32)));
        body.blocks.insert(0, block);
    }
    assert!(indexed_differential_v1(&module, &bytes, &layout));
    compare_index_lookups_v1(&module.functions[0], &[0, 9, 14, 17, 1000, u32::MAX, 888]);
}

#[test]
fn indexed_formal_join_preserves_parameter_first_definition() {
    let mut module = fixture_module();
    let body = module.functions[0].body.as_mut().unwrap();
    body.blocks[0].parameters.push(kir::ValueDef::new(kir::ValueId(0), kir::Type::F32));
    compare_index_lookups_v1(&module.functions[0], &[0, 1, 5, 14]);
}

#[test]
fn indexed_formal_join_preserves_block_parameter_first_definition() {
    let mut module = fixture_module();
    let mut first = kir::BasicBlock::new(kir::BlockId(999));
    first.parameters.push(kir::ValueDef::new(kir::ValueId(14), kir::Type::F32));
    module.functions[0].body.as_mut().unwrap().blocks.insert(0, first);
    compare_index_lookups_v1(&module.functions[0], &[14, 9, u32::MAX]);
}

#[test]
fn indexed_formal_join_preserves_operation_first_definition() {
    let mut module = fixture_module();
    let body = module.functions[0].body.as_mut().unwrap();
    body.blocks[0].operations.push(Operation::effect_free(
        kir::ValueDef::new(kir::ValueId(9), kir::Type::F32),
        OperationKind::Constant(kir::Constant::U32(0)),
    ));
    compare_index_lookups_v1(&module.functions[0], &[9, 11, 14]);
}

#[test]
fn indexed_formal_join_rejects_absent_pointer_definition() {
    let (mut module, bytes, layout) = join_fixture();
    module.functions[0].body.as_mut().unwrap().blocks[0].operations[1].results[0].id =
        kir::ValueId(u32::MAX);
    assert!(!indexed_differential_v1(&module, &bytes, &layout));
}

#[test]
fn indexed_formal_join_rejects_duplicate_block_ids() {
    let (mut module, bytes, layout) = join_fixture();
    module.functions[0].body.as_mut().unwrap().blocks.push(kir::BasicBlock::new(kir::BlockId(0)));
    assert!(!indexed_differential_v1(&module, &bytes, &layout));
}

#[test]
fn indexed_formal_join_matches_root_type_space_and_census_mutations() {
    for mutation in 0..10 {
        let (mut module, mut bytes, mut layout) = join_fixture();
        match mutation {
            0 => layout.roots[0].parameter = kir::ValueId(999),
            1 => layout.roots[0].allocation_origin ^= 1,
            2 => layout.allocation_count -= 1,
            3 => layout.memory_count -= 1,
            4 => layout.sync_count -= 1,
            5 => module.functions[0].body.as_mut().unwrap().parameters.pop().map(|_| ()).unwrap(),
            6 => module.functions[0].signature.parameters[0] = kir::Type::F32,
            7 => module.functions[0].body.as_mut().unwrap().blocks[0].operations[1].results[0].ty =
                kir::Type::F32,
            8 => bytes[layout.memory.start + 12] = 1,
            9 => module.kernels[0].workgroup_size = Some(kir::WorkgroupSize::new(32, 1, 1)),
            _ => unreachable!(),
        }
        assert!(!indexed_differential_v1(&module, &bytes, &layout), "mutation {mutation}");
    }
}

#[test]
fn indexed_formal_join_preserves_stream_order_and_trailing_rejection() {
    for mutation in 0..4 {
        let (module, mut bytes, mut layout) = join_fixture();
        match mutation {
            0 => bytes[layout.allocations.start + 4..layout.allocations.start + 12]
                .copy_from_slice(&9u64.to_le_bytes()),
            1 => bytes[layout.memory.start + 4..layout.memory.start + 12]
                .copy_from_slice(&4u64.to_le_bytes()),
            2 => layout.sync.end -= 1,
            3 => layout.allocations.end += 1,
            _ => unreachable!(),
        }
        assert!(!indexed_differential_v1(&module, &bytes, &layout));
    }
}

#[test]
fn indexed_formal_join_rejects_unknown_call_effects() {
    let (mut module, bytes, layout) = join_fixture();
    module.functions[0].body.as_mut().unwrap().blocks[0].operations.push(Operation::new(
        vec![],
        OperationKind::Call { callee: kir::FunctionId::new("unregistered"), arguments: vec![] },
    ));
    assert!(!indexed_differential_v1(&module, &bytes, &layout));
}

#[test]
fn indexed_formal_join_preserves_reason_locations_and_missing_refusal() {
    for mutation in 0..4 {
        let (module, mut bytes, mut layout) = join_fixture();
        let reason = Location::new(kir::BlockId(0), 3);
        let covered = match mutation {
            0 => reason,
            1 => Location::new(kir::BlockId(777), 3),
            2 => Location::new(kir::BlockId(0), usize::MAX),
            3 => Location::new(kir::BlockId(0), 4),
            _ => unreachable!(),
        };
        let encoded_reason = if mutation == 3 { reason } else { covered };
        let mut writer = Writer::owning(256).unwrap();
        write_reason(&mut writer, &Reason::GuardedAccessRequiresRankedProof {
            location: encoded_reason,
        }).unwrap();
        writer.u8(1).unwrap();
        writer.location(covered).unwrap();
        let start = bytes.len();
        bytes.extend_from_slice(&writer.finish().unwrap());
        layout.reasons = start..bytes.len();
        layout.reason_count = 1;
        assert_eq!(indexed_differential_v1(&module, &bytes, &layout), mutation == 0);
    }
}

#[test]
fn indexed_formal_join_exact_and_one_under_resources_restore_floor() {
    let (module, bytes, layout) = join_fixture();
    let (result, work, peak) = indexed_probe_v1(&module, &bytes, &layout, usize::MAX, usize::MAX);
    result.unwrap();
    let (exact, exact_work, exact_peak) = indexed_probe_v1(&module, &bytes, &layout, work, peak);
    exact.unwrap();
    assert_eq!((exact_work, exact_peak), (work, peak));
    for (work_limit, storage_limit) in [(work - 1, peak), (work, peak - 1)] {
        let (result, _, _) = indexed_probe_v1(&module, &bytes, &layout, work_limit, storage_limit);
        assert!(matches!(result, Err(E::Resource(_))));
    }
}

#[test]
fn indexed_formal_join_work_refusal_prefixes_restore_floor() {
    let (module, bytes, layout) = join_fixture();
    let (result, work, peak) = indexed_probe_v1(&module, &bytes, &layout, usize::MAX, usize::MAX);
    result.unwrap();
    for limit in [11, 12, 100, work / 2, work - 1] {
        let (result, accepted, _) = indexed_probe_v1(&module, &bytes, &layout, limit, peak);
        assert!(matches!(result, Err(E::Resource(_))));
        assert!(accepted <= limit);
    }
}

#[test]
fn indexed_formal_join_sort_matches_tuple_order_with_duplicate_ids() {
    for count in [0usize, 1, 2, 3, 16, 31, 64] {
        let mut rows: Vec<_> = (0..count).map(|i| (((count - i) % 7) as u32, i)).collect();
        let mut expected = rows.clone();
        expected.sort_unstable();
        let mut work = kir::CanonicalKernelIrWorkBudgetV1::new(usize::MAX);
        let mut budget = Budget::new(&mut work, 1024);
        inert_join_sort_v1(&mut rows, &mut budget, |row| *row).unwrap();
        assert_eq!(rows, expected);
        assert_eq!(budget.storage(), 0);
    }
}

#[test]
fn indexed_formal_join_sort_one_under_refuses_before_mutation() {
    let original = vec![(9u32, 0usize), (1, 1), (9, 2), (0, 3), (5, 4)];
    let mut rows = original.clone();
    let mut work = kir::CanonicalKernelIrWorkBudgetV1::new(usize::MAX);
    let mut budget = Budget::new(&mut work, 1024);
    inert_join_sort_v1(&mut rows, &mut budget, |row| *row).unwrap();
    let exact = budget.work();
    let mut rows = original.clone();
    let mut short = kir::CanonicalKernelIrWorkBudgetV1::new(exact - 1);
    let mut budget = Budget::new(&mut short, 1024);
    assert!(matches!(
        inert_join_sort_v1(&mut rows, &mut budget, |row| *row),
        Err(E::Resource(_))
    ));
    assert_eq!(rows, original);
    assert_eq!(budget.work(), 0);
}

#[test]
fn indexed_formal_join_checked_work_formulas_refuse_overflow() {
    for (bytes, rows, roots, kernels) in [
        (usize::MAX, 0, 0, 0), (0, usize::MAX, 0, 0),
        (0, 0, usize::MAX, 0), (0, 0, 0, usize::MAX),
    ] {
        assert!(matches!(
            inert_join_linear_work_v1(bytes, rows, roots, kernels),
            Err(E::Size)
        ));
    }
}

#[test]
fn indexed_formal_join_keeps_inert_identity_and_no_admission_result() {
    let (module, bytes, layout) = prefix_tile_fixture_v6();
    let identity = published_identity(&bytes[layout.kir.clone()]).unwrap();
    let canonical = kir::encode_module_v11(&module).unwrap();
    let (result, _, _) = indexed_probe_v1(&module, &bytes, &layout, usize::MAX, usize::MAX);
    let _: () = result.unwrap();
    assert_eq!(published_identity(&bytes[layout.kir.clone()]).unwrap(), identity);
    assert_eq!(kir::encode_module_v11(&module).unwrap(), canonical);
    // Layout checks do not issue a live/source/launch token or turn framing
    // fixtures with missing nested evidence into an admitted archive.
    assert!(InertCanonicalWaveQkvAttentionOutputTileFormalMemoryEvidenceV6::decode(
        &prefix_tile_layout_fixture_v6()
    ).is_err());
}

#[test]
fn indexed_formal_join_repeated_validation_has_identical_resources() {
    let (module, bytes, layout) = join_fixture();
    let (first, first_work, first_peak) =
        indexed_probe_v1(&module, &bytes, &layout, usize::MAX, usize::MAX);
    let (second, second_work, second_peak) =
        indexed_probe_v1(&module, &bytes, &layout, usize::MAX, usize::MAX);
    first.unwrap();
    second.unwrap();
    assert_eq!((first_work, first_peak), (second_work, second_peak));
}

#[test]
fn indexed_formal_join_duplicate_definition_type_refusals_match_legacy() {
    for valid in [false, true] {
        let (mut module, bytes, layout) = join_fixture();
        let ty = if valid {
            pointer_type(&module.functions[0], kir::ValueId(9)).unwrap().clone()
        } else {
            kir::Type::F32
        };
        let mut first = kir::BasicBlock::new(kir::BlockId(555));
        first.parameters.push(kir::ValueDef::new(kir::ValueId(9), ty));
        module.functions[0].body.as_mut().unwrap().blocks.insert(0, first);
        assert_eq!(indexed_differential_v1(&module, &bytes, &layout), valid);
    }
}
