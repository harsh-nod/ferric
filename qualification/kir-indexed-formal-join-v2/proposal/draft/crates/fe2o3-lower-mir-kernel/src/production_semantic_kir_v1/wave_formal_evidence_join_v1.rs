// Live joins authenticate indices before encoding. Inert joins check internal
// consistency only; neither decoding nor a public content digest issues proof.

fn access_tag(access: dialect_kernel::AccessKindAttr) -> ResultV1<u8> {
    use dialect_kernel::AccessKindAttr::*;
    match access {
        Read => Ok(0),
        Write => Ok(1),
        AtomicRead => Ok(2),
        AtomicWrite => Ok(3),
        AtomicReadModifyWrite => Ok(4),
    }
}
fn space_tag(space: dialect_kernel::MemorySpaceAttr) -> ResultV1<u8> {
    use dialect_kernel::MemorySpaceAttr::*;
    match space {
        Global => Ok(0),
        Workgroup => Ok(1),
        Private => Ok(2),
        _ => Err(E::Correspondence),
    }
}
fn kir_space(space: kir::AddressSpace) -> ResultV1<u8> {
    match space {
        kir::AddressSpace::Global => Ok(0),
        kir::AddressSpace::Workgroup => Ok(1),
        kir::AddressSpace::Private => Ok(2),
        _ => Err(E::Correspondence),
    }
}
fn ordering(ordering: kir::MemoryOrdering) -> u8 {
    use kir::MemoryOrdering::*;
    match ordering {
        Relaxed => 0,
        Acquire => 1,
        Release => 2,
        AcquireRelease => 3,
        SequentiallyConsistent => 4,
    }
}
fn atomic_scope(scope: kir::SynchronizationScope) -> ResultV1<u8> {
    use kir::SynchronizationScope::*;
    match scope {
        Invocation => Ok(0),
        Workgroup => Ok(1),
        Device => Ok(2),
        System => Ok(4),
        Subgroup => Err(E::Correspondence),
    }
}
fn memory_shape(
    operation: &Operation,
) -> ResultV1<(kir::ValueId, u8, u8, Option<(u8, u8, Option<u8>)>)> {
    match &operation.kind {
        OperationKind::Load { pointer, access } => {
            Ok((*pointer, 0, kir_space(access.address_space)?, None))
        }
        OperationKind::Store {
            pointer, access, ..
        } => Ok((*pointer, 1, kir_space(access.address_space)?, None)),
        OperationKind::Atomic(atomic) => Ok((
            atomic.pointer,
            match atomic.kind {
                kir::AtomicKind::Load => 2,
                kir::AtomicKind::Store => 3,
                _ => 4,
            },
            kir_space(atomic.access.address_space)?,
            Some((
                ordering(atomic.ordering),
                atomic_scope(atomic.scope)?,
                atomic.failure_ordering.map(ordering),
            )),
        )),
        _ => Err(E::Correspondence),
    }
}
fn operation_at(module: &kir::Module, function: usize, location: Location) -> ResultV1<&Operation> {
    module
        .functions
        .get(function)
        .and_then(|function| function.body.as_ref())
        .and_then(|body| body.blocks.iter().find(|block| block.id == location.block))
        .and_then(|block| block.operations.get(location.operation_index))
        .ok_or(E::Correspondence)
}
fn pointer_type(function: &kir::Function, pointer: kir::ValueId) -> ResultV1<&kir::Type> {
    let body = function.body.as_ref().ok_or(E::Correspondence)?;
    for (id, ty) in body.parameters.iter().zip(&function.signature.parameters) {
        if *id == pointer {
            return Ok(ty);
        }
    }
    for block in &body.blocks {
        for value in &block.parameters {
            if value.id == pointer {
                return Ok(&value.ty);
            }
        }
        for operation in &block.operations {
            for value in &operation.results {
                if value.id == pointer {
                    return Ok(&value.ty);
                }
            }
        }
    }
    Err(E::Correspondence)
}
fn validate_geometry(rank: u8, global: [u64; 3], workgroup: [u64; 3]) -> ResultV1<()> {
    if !(1..=3).contains(&rank)
        || global.contains(&0)
        || workgroup.contains(&0)
        || global
            .iter()
            .zip(workgroup)
            .any(|(global, local)| global % local != 0)
        || (rank as usize..3).any(|axis| global[axis] != 1 || workgroup[axis] != 1)
    {
        return Err(E::Correspondence);
    }
    global
        .iter()
        .try_fold(1u64, |product, extent| product.checked_mul(*extent))
        .ok_or(E::Size)?;
    workgroup
        .iter()
        .try_fold(1u64, |product, extent| product.checked_mul(*extent))
        .ok_or(E::Size)?;
    Ok(())
}
fn internal_allocation(operation: &Operation) -> ResultV1<bool> {
    match operation.kind {
        OperationKind::Alloca {
            address_space: kir::AddressSpace::Private | kir::AddressSpace::Workgroup,
            ..
        }
        | OperationKind::WorkgroupMemory(_) => Ok(true),
        OperationKind::Alloca { .. } => Err(E::Correspondence),
        _ => Ok(false),
    }
}
fn is_sync(operation: &Operation) -> bool {
    matches!(
        operation.kind,
        OperationKind::Barrier(_) | OperationKind::Fence(_) | OperationKind::WorkgroupBarrier(_)
    )
}
fn is_memory(operation: &Operation) -> bool {
    matches!(
        operation.kind,
        OperationKind::Load { .. } | OperationKind::Store { .. } | OperationKind::Atomic(_)
    )
}
fn reason_location(reason: InertWaveTaskFormalReasonV1<'_>) -> Option<Location> {
    use InertWaveTaskFormalReasonV1::*;
    match reason {
        CallEffectsUnavailable { location, .. }
        | UnsupportedMemoryEffect(location)
        | GuardedAccessRequiresRankedProof(location)
        | UnsupportedPointerDerivation { location, .. }
        | UnsupportedIndexExpression { location, .. }
        | ElementWidthUnavailable { location, .. }
        | AddressArithmeticOverflow(location) => Some(location),
        _ => None,
    }
}
fn join_work(function: &kir::Function, rows: usize) -> ResultV1<usize> {
    let body = function.body.as_ref().ok_or(E::Correspondence)?;
    let mut nodes = add(body.parameters.len(), add(body.blocks.len(), 1)?)?;
    for block in &body.blocks {
        nodes = add(nodes, add(block.parameters.len(), block.operations.len())?)?;
        for operation in &block.operations {
            nodes = add(nodes, operation.results.len())?;
        }
    }
    mul(mul(add(rows, add(body.blocks.len(), 1)?)?, nodes)?, 128)
}
fn validate_live_indices(
    formal: &Formal,
    ranked: &Ranked,
    budget: &mut Budget<'_>,
) -> ResultV1<()> {
    let module = formal.semantic_kir().module();
    let function = module
        .functions
        .get(ranked.function_ordinal())
        .ok_or(E::Correspondence)?;
    budget.charge_work(formal.semantic_kir().canonical_kernel_ir_bytes().len())?;
    let rows = add(
        add(
            formal.covered_reasons().len(),
            formal.internal_allocations().len(),
        )?,
        add(
            formal.memory_census().len(),
            ranked.synchronization_bindings().len(),
        )?,
    )?;
    budget.charge_work(join_work(function, rows)?)?;
    for coverage in formal.covered_reasons() {
        if operation_at(module, ranked.function_ordinal(), coverage.location())?
            != coverage.operation()
        {
            return Err(E::Correspondence);
        }
    }
    for allocation in formal.internal_allocations() {
        let operation = operation_at(module, ranked.function_ordinal(), allocation.location())?;
        if operation != allocation.operation() || !internal_allocation(operation)? {
            return Err(E::Correspondence);
        }
    }
    for row in formal.memory_census() {
        let operation = operation_at(module, ranked.function_ordinal(), row.kernel_ir_location())?;
        if ranked.operation(row.kernel_ir_location()) != Some(operation)
            || pointer_type(function, row.pointer())? != row.pointer_type()
            || memory_shape(operation)?
                != (
                    row.pointer(),
                    access_tag(row.access())?,
                    space_tag(row.memory_space())?,
                    row.atomic_contract(),
                )
        {
            return Err(E::Correspondence);
        }
    }
    for row in ranked.synchronization_bindings() {
        if operation_at(module, ranked.function_ordinal(), row.kernel_ir_location())?
            != row.operation()
        {
            return Err(E::Correspondence);
        }
    }
    Ok(())
}

include!("wave_formal_evidence_index_v1.rs");

fn validate_layout_module<const N: usize>(
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
    budget.charge_work(inert_join_linear_work_v1(kir_length, rows, N, module.kernels.len())?)?;
    // Unknown complete-effect summaries may allocate a small effect vector.
    // Reserve actual wire-sized temporary space before any such query.
    let scratch = add(mul(kir_length, 16)?, 1024)?;
    let floor = budget.storage();
    budget.reserve_storage(scratch)?;
    let result = (|| {
        if body.parameters.len() != N || function.signature.parameters.len() != N {
            return Err(E::Correspondence);
        }
        let indices = InertJoinIndexV1::build(function, budget)?;
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
                    let kir::Type::Pointer(pointer) = indices.pointer_type(row.pointer, budget)? else {
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
                indices.operation(location, budget)?;
            }
        }
        reasons.finish()?;
        Ok(())
    })();
    release_to(budget, floor)?;
    result
}

fn validate_raw_binding<const N: usize>(
    bytes: &[u8],
    layout: &Layout<N>,
    module: &kir::Module,
    function: &kir::Function,
) -> ResultV1<()> {
    // The entire receipt has already passed its original lossless decoder.
    // Borrow only its stable header/allocation fields for the exact KIR join.
    let raw = &bytes[layout.receipt.clone()];
    let mut r = Reader::new(raw);
    r.take(20)?;
    let kernel_name = r.blob()?;
    let entry_name = r.blob()?;
    if &raw[entry_name] != function.id.as_str().as_bytes() {
        return Err(E::Correspondence);
    }
    let mut kernels = module
        .kernels
        .iter()
        .filter(|kernel| kernel.id.as_str().as_bytes() == &raw[kernel_name.clone()]);
    let kernel = kernels.next().ok_or(E::Correspondence)?;
    if kernels.next().is_some()
        || kernel.entry != function.id
        || kernel.domain.rank() != layout.rank
    {
        return Err(E::Correspondence);
    }
    let required = kernel.workgroup_size.ok_or(E::Correspondence)?;
    if layout.workgroup
        != [
            u64::from(required.x),
            u64::from(required.y),
            u64::from(required.z),
        ]
    {
        return Err(E::Correspondence);
    }
    for (axis, extent) in kernel.domain.extents().enumerate() {
        if let kir::LaunchExtent::Static(value) = extent {
            if layout.global[axis] != u64::from(value) {
                return Err(E::Correspondence);
            }
        }
    }
    r.take(4)?; // exact index-width/basis/reserved fields were validated above
    if r.boolean()? {
        r.take(16)?;
    }
    if r.count(12)? != N {
        return Err(E::Correspondence);
    }
    for _ in 0..N {
        let index = r.u32()?;
        let value = kir::ValueId(r.u32()?);
        let kind = r.u8()?;
        let space = r.u8()?;
        let access = r.u8()?;
        let reserved = r.u8()?;
        let root = layout
            .roots
            .iter()
            .find(|root| root.parameter_index == index)
            .ok_or(E::Correspondence)?;
        let Some(kir::Type::Pointer(pointer)) = function.signature.parameters.get(index as usize)
        else {
            return Err(E::Correspondence);
        };
        let expected_access = match pointer.access {
            kir::AccessMode::ReadOnly => 1,
            kir::AccessMode::ReadWrite => 2,
            kir::AccessMode::WriteOnly => 3,
        };
        if root.parameter != value
            || kind != 1
            || space != 3
            || access != expected_access
            || reserved != 0
        {
            return Err(E::Correspondence);
        }
    }
    Ok(())
}
