// Inlining AtomicU32 can retain an unused UnsafeCell-to-Align4 sibling cast.
// This exception proves it dead; it never adds pointer or allocation authority.
fn indexed_atomic_dead_cast_v1(
    types: &[SemanticTypeDeclV1],
    census: &mut IndexedAtomicDeadCastCensusV1<'_>,
    inventory: &AuthenticatedAtomicAllocationsV1,
    site: ScalarAssignmentSiteV1,
    assignment: &fe2o3_mir_model::semantic_mir_v1::SemanticAssignmentV1,
) -> Result<bool, ProductionRankedProjectionErrorV1> {
    let SemanticRvalueKindV1::Cast {
        kind: SemanticCastKindV1::Pointer,
        operand,
    } = assignment.value().kind()
    else {
        return Ok(false);
    };
    let function = census.function;
    let destination = assignment.destination();
    let Some(declaration) = function.locals().get(destination.local().index() as usize) else {
        return Ok(false);
    };
    if !destination.projections().is_empty()
        || declaration.role() != SemanticLocalRoleV1::Temporary
        || declaration.ty() != destination.ty()
        || destination.ty() != assignment.value().result_type()
    {
        return Ok(false);
    }
    let Some(source_local) = simple_operand_local(operand) else {
        return Ok(false);
    };
    if source_local == destination.local() || !inventory.contains_local(source_local)? {
        return Ok(false);
    }
    let Some(source) = atomic_u32_pointer_v1(types, operand.ty()) else {
        return Ok(false);
    };
    let Some(target) = atomic_u32_pointer_v1(types, destination.ty()) else {
        return Ok(false);
    };
    if source.kind() != SemanticPointerKindV1::Raw || target.kind() != SemanticPointerKindV1::Raw {
        return Ok(false);
    }
    let Some(storage) = types.get(source.pointee().index() as usize) else {
        return Ok(false);
    };
    let Some(field) = types.get(target.pointee().index() as usize) else {
        return Ok(false);
    };
    let SemanticTypeShapeV1::Aggregate(fields) = storage.shape() else {
        return Ok(false);
    };
    let fe2o3_mir_model::semantic_mir_v1::SemanticFieldsShapeV1::Arbitrary {
        source_order_offsets_bytes,
        ..
    } = storage.layout().fields()
    else {
        return Ok(false);
    };
    if storage.layout().size_bytes() != Some(4)
        || storage.layout().alignment_bytes() != 4
        || field.layout().size_bytes() != Some(4)
        || field.layout().alignment_bytes() != 4
        || fields.fields() != [target.pointee()]
        || source_order_offsets_bytes.as_ref() != [0]
    {
        return Ok(false);
    }
    census.temporary_unused(inventory, site, destination.local())
}

#[derive(Clone, Copy, Debug)]
enum IndexedAtomicDeadCastMentionV1 {
    Never,
    Statement { block: usize, statement: usize },
    Other,
}

// Owned by one escape-validation invocation; the referenced function is immutable.
struct IndexedAtomicDeadCastCensusV1<'a> {
    function: &'a SemanticFunctionDeclV1,
    mentions: Option<Vec<IndexedAtomicDeadCastMentionV1>>,
}

impl<'a> IndexedAtomicDeadCastCensusV1<'a> {
    fn new(function: &'a SemanticFunctionDeclV1) -> Self {
        Self {
            function,
            mentions: None,
        }
    }

    fn temporary_unused(
        &mut self,
        inventory: &AuthenticatedAtomicAllocationsV1,
        definition: ScalarAssignmentSiteV1,
        local: SemanticLocalIdV1,
    ) -> Result<bool, ProductionRankedProjectionErrorV1> {
        // Return implicitly reads its return local, so only a temporary can qualify.
        if self
            .function
            .locals()
            .get(local.index() as usize)
            .is_none_or(|decl| decl.role() != SemanticLocalRoleV1::Temporary)
        {
            return Ok(false);
        }
        inventory.charge(1)?;
        if self.mentions.is_none() {
            // No cache is published if allocation or any traversal charge fails.
            let mentions = indexed_atomic_dead_cast_census_build_v1(self.function, inventory)?;
            self.mentions = Some(mentions);
        }
        let mentions = self.mentions.as_ref().expect("completed dead-cast census");
        Ok(match mentions[local.index() as usize] {
            IndexedAtomicDeadCastMentionV1::Never => true,
            IndexedAtomicDeadCastMentionV1::Statement { block, statement } => {
                block == definition.block && statement == definition.statement
            }
            IndexedAtomicDeadCastMentionV1::Other => false,
        })
    }
}

fn indexed_atomic_dead_cast_census_storage_v1(
    count: usize,
    inventory: &AuthenticatedAtomicAllocationsV1,
) -> Result<Vec<IndexedAtomicDeadCastMentionV1>, ProductionRankedProjectionErrorV1> {
    if count > MAX_PROJECTED_CAPABILITY_STATE_ENTRIES_V1 {
        return Err(ProductionRankedProjectionErrorV1::Unsupported(
            "indexed atomic dead-cast census exceeds its storage limit",
        ));
    }
    inventory.charge(count)?;
    let mut mentions = Vec::new();
    mentions.try_reserve_exact(count).map_err(|_| {
        ProductionRankedProjectionErrorV1::Unsupported(
            "indexed atomic dead-cast census allocation failed",
        )
    })?;
    mentions.resize(count, IndexedAtomicDeadCastMentionV1::Never);
    Ok(mentions)
}

fn indexed_atomic_dead_cast_record_v1(
    mentions: &mut [IndexedAtomicDeadCastMentionV1],
    site: Option<ScalarAssignmentSiteV1>,
    local: SemanticLocalIdV1,
) {
    // An unrelated invalid ID cannot equal any valid temporary queried by the old scan.
    let Some(mention) = mentions.get_mut(local.index() as usize) else {
        return;
    };
    *mention = match (*mention, site) {
        (_, None) | (IndexedAtomicDeadCastMentionV1::Other, _) => {
            IndexedAtomicDeadCastMentionV1::Other
        }
        (IndexedAtomicDeadCastMentionV1::Never, Some(site)) => {
            IndexedAtomicDeadCastMentionV1::Statement {
                block: site.block,
                statement: site.statement,
            }
        }
        (IndexedAtomicDeadCastMentionV1::Statement { block, statement }, Some(site))
            if block == site.block && statement == site.statement =>
        {
            IndexedAtomicDeadCastMentionV1::Statement { block, statement }
        }
        _ => IndexedAtomicDeadCastMentionV1::Other,
    };
}

fn indexed_atomic_dead_cast_place_v1(
    inventory: &AuthenticatedAtomicAllocationsV1,
    mentions: &mut [IndexedAtomicDeadCastMentionV1],
    site: Option<ScalarAssignmentSiteV1>,
    place: &SemanticPlaceV1,
) -> Result<(), ProductionRankedProjectionErrorV1> {
    inventory.charge(place.projections().len() + 1)?;
    indexed_atomic_dead_cast_record_v1(mentions, site, place.local());
    for projection in place.projections() {
        if let SemanticProjectionKindV1::Index(local) = projection.kind() {
            indexed_atomic_dead_cast_record_v1(mentions, site, local);
        }
    }
    Ok(())
}

fn indexed_atomic_dead_cast_operand_v1(
    inventory: &AuthenticatedAtomicAllocationsV1,
    mentions: &mut [IndexedAtomicDeadCastMentionV1],
    site: Option<ScalarAssignmentSiteV1>,
    operand: &SemanticOperandV1,
) -> Result<(), ProductionRankedProjectionErrorV1> {
    inventory.charge(1)?;
    if let Some(place) = raw_operand_place(operand) {
        indexed_atomic_dead_cast_place_v1(inventory, mentions, site, place)?;
    }
    Ok(())
}

fn indexed_atomic_dead_cast_census_build_v1(
    function: &SemanticFunctionDeclV1,
    inventory: &AuthenticatedAtomicAllocationsV1,
) -> Result<Vec<IndexedAtomicDeadCastMentionV1>, ProductionRankedProjectionErrorV1> {
    let mut mentions =
        indexed_atomic_dead_cast_census_storage_v1(function.locals().len(), inventory)?;
    for (block, body) in function.blocks().iter().enumerate() {
        inventory.charge(1)?;
        for (statement, record) in body.statements().iter().enumerate() {
            inventory.charge(1)?;
            let site = Some(ScalarAssignmentSiteV1 { block, statement });
            match record.kind() {
                SemanticStatementKindV1::Assign(assignment) => {
                    assignment.value().kind().try_visit_operands(|operand| {
                        indexed_atomic_dead_cast_operand_v1(
                            inventory, &mut mentions, site, operand,
                        )
                    })?;
                    indexed_atomic_dead_cast_place_v1(
                        inventory, &mut mentions, site, assignment.destination(),
                    )?;
                    match assignment.value().kind() {
                        SemanticRvalueKindV1::Load(load) => {
                            indexed_atomic_dead_cast_place_v1(
                                inventory, &mut mentions, site, load.source(),
                            )?;
                        }
                        SemanticRvalueKindV1::Borrow { place, .. }
                        | SemanticRvalueKindV1::AddressOf { place, .. }
                        | SemanticRvalueKindV1::Length(place)
                        | SemanticRvalueKindV1::Discriminant(place) => {
                            indexed_atomic_dead_cast_place_v1(
                                inventory, &mut mentions, site, place,
                            )?;
                        }
                        SemanticRvalueKindV1::Use(_)
                        | SemanticRvalueKindV1::Unary { .. }
                        | SemanticRvalueKindV1::Binary { .. }
                        | SemanticRvalueKindV1::CheckedBinary { .. }
                        | SemanticRvalueKindV1::UncheckedBinary { .. }
                        | SemanticRvalueKindV1::Cast { .. }
                        | SemanticRvalueKindV1::Aggregate { .. } => {}
                    }
                }
                SemanticStatementKindV1::Store(store) => {
                    indexed_atomic_dead_cast_place_v1(
                        inventory, &mut mentions, site, store.destination(),
                    )?;
                    indexed_atomic_dead_cast_operand_v1(
                        inventory, &mut mentions, site, store.value(),
                    )?;
                }
                SemanticStatementKindV1::AtomicRmw(atomic) => {
                    indexed_atomic_dead_cast_place_v1(
                        inventory, &mut mentions, site, atomic.destination(),
                    )?;
                    indexed_atomic_dead_cast_place_v1(
                        inventory, &mut mentions, site, atomic.address(),
                    )?;
                    indexed_atomic_dead_cast_operand_v1(
                        inventory, &mut mentions, site, atomic.value(),
                    )?;
                }
                SemanticStatementKindV1::AtomicCompareExchange(atomic) => {
                    indexed_atomic_dead_cast_place_v1(
                        inventory, &mut mentions, site, atomic.destination(),
                    )?;
                    indexed_atomic_dead_cast_place_v1(
                        inventory, &mut mentions, site, atomic.address(),
                    )?;
                    indexed_atomic_dead_cast_operand_v1(
                        inventory, &mut mentions, site, atomic.expected(),
                    )?;
                    indexed_atomic_dead_cast_operand_v1(
                        inventory, &mut mentions, site, atomic.replacement(),
                    )?;
                }
                SemanticStatementKindV1::SetDiscriminant { place, .. }
                | SemanticStatementKindV1::Deinitialize(place) => {
                    indexed_atomic_dead_cast_place_v1(
                        inventory, &mut mentions, site, place,
                    )?;
                }
                SemanticStatementKindV1::Assume(operand) => {
                    indexed_atomic_dead_cast_operand_v1(
                        inventory, &mut mentions, site, operand,
                    )?;
                }
                SemanticStatementKindV1::StorageLive(_)
                | SemanticStatementKindV1::StorageDead(_)
                | SemanticStatementKindV1::Nop => {}
            }
        }
        inventory.charge(1)?;
        match body.terminator().kind() {
            SemanticTerminatorKindV1::Call(call) => {
                for argument in call.arguments() {
                    indexed_atomic_dead_cast_operand_v1(
                        inventory, &mut mentions, None, argument,
                    )?;
                }
                if let Some(destination) = call.destination() {
                    indexed_atomic_dead_cast_place_v1(
                        inventory, &mut mentions, None, destination.place(),
                    )?;
                }
            }
            SemanticTerminatorKindV1::TailCall(call) => {
                for argument in call.arguments() {
                    indexed_atomic_dead_cast_operand_v1(
                        inventory, &mut mentions, None, argument,
                    )?;
                }
            }
            SemanticTerminatorKindV1::SwitchInt { discriminant, .. } => {
                indexed_atomic_dead_cast_operand_v1(
                    inventory, &mut mentions, None, discriminant,
                )?;
            }
            SemanticTerminatorKindV1::Drop { place, .. } => {
                indexed_atomic_dead_cast_place_v1(
                    inventory, &mut mentions, None, place,
                )?;
            }
            SemanticTerminatorKindV1::Assert { condition, message, .. } => {
                indexed_atomic_dead_cast_operand_v1(
                    inventory, &mut mentions, None, condition,
                )?;
                match message {
                    SemanticAssertMessageV1::BoundsCheck { length, index } => {
                        indexed_atomic_dead_cast_operand_v1(
                            inventory, &mut mentions, None, length,
                        )?;
                        indexed_atomic_dead_cast_operand_v1(
                            inventory, &mut mentions, None, index,
                        )?;
                    }
                    SemanticAssertMessageV1::Overflow { left, right, .. } => {
                        indexed_atomic_dead_cast_operand_v1(
                            inventory, &mut mentions, None, left,
                        )?;
                        indexed_atomic_dead_cast_operand_v1(
                            inventory, &mut mentions, None, right,
                        )?;
                    }
                    SemanticAssertMessageV1::DivisionByZero(operand)
                    | SemanticAssertMessageV1::RemainderByZero(operand) => {
                        indexed_atomic_dead_cast_operand_v1(
                            inventory, &mut mentions, None, operand,
                        )?;
                    }
                    SemanticAssertMessageV1::MisalignedPointerDereference {
                        required_alignment, found_alignment,
                    } => {
                        indexed_atomic_dead_cast_operand_v1(
                            inventory, &mut mentions, None, required_alignment,
                        )?;
                        indexed_atomic_dead_cast_operand_v1(
                            inventory, &mut mentions, None, found_alignment,
                        )?;
                    }
                    SemanticAssertMessageV1::NullPointerDereference
                    | SemanticAssertMessageV1::ResumedAfterReturn
                    | SemanticAssertMessageV1::ResumedAfterPanic => {}
                }
            }
            SemanticTerminatorKindV1::Goto(_)
            | SemanticTerminatorKindV1::FalseEdge { .. }
            | SemanticTerminatorKindV1::Return
            | SemanticTerminatorKindV1::UnwindResume
            | SemanticTerminatorKindV1::UnwindTerminate
            | SemanticTerminatorKindV1::Abort
            | SemanticTerminatorKindV1::Unreachable => {}
        }
    }
    Ok(mentions)
}
