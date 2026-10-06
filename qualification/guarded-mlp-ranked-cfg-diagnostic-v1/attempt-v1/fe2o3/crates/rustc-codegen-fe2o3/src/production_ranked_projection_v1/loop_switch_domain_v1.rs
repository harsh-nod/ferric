fn switch_targets_exhaust_inhabited_variants_v1(
    variants: &[fe2o3_mir_model::semantic_mir_v1::SemanticEnumVariantV1],
    targets: &SemanticSwitchTargetsV1,
) -> bool {
    // Admission guarantees unique logical variant discriminants and sorted unique targets.
    let mut inhabited = variants.iter().filter(|variant| !variant.is_uninhabited());
    let count = inhabited.clone().count();
    count != 0
        && count == targets.values().len()
        && inhabited.all(|variant| {
            targets
                .values()
                .iter()
                .any(|target| target.value() == variant.discriminant())
        })
}

fn charge_loop_switch_inventory_v1(
    function: &SemanticFunctionDeclV1,
    work: &mut usize,
) -> Result<(), ProductionRankedProjectionErrorV1> {
    if function.blocks().is_empty() || function.blocks().len() > MAX_RANKED_BOUNDS_BLOCKS {
        return Err(ProductionRankedProjectionErrorV1::cfg_block_limit_v1(
            function,
            CfgBlockLimitSiteV1::LoopSwitchInventory,
        ));
    }
    project_loop_graph_charge_v1(work, function.locals().len().saturating_mul(3))?;
    project_loop_graph_charge_v1(work, function.blocks().len())?;
    for block in function.blocks() {
        let definitions = block.statements().len().saturating_mul(2).saturating_add(1);
        let sort_levels = (usize::BITS - definitions.leading_zeros()) as usize;
        // The reused inventory sorts each block's at-most-two-definitions-per-statement list.
        project_loop_graph_charge_v1(
            work,
            definitions.saturating_mul(sort_levels.saturating_add(1)),
        )?;
    }
    Ok(())
}

fn authenticated_loop_switch_fallback_v1(
    types: &[SemanticTypeDeclV1],
    function: &SemanticFunctionDeclV1,
    block_index: usize,
    inventory: &AssertionDefinitionInventoryV1,
    work: &mut usize,
) -> Result<bool, ProductionRankedProjectionErrorV1> {
    project_loop_graph_charge_v1(work, 1)?;
    let Some(block) = function.blocks().get(block_index) else {
        return Ok(false);
    };
    let SemanticTerminatorKindV1::SwitchInt {
        discriminant,
        targets,
    } = block.terminator().kind()
    else {
        return Ok(false);
    };
    let fallback = targets.otherwise().target().index() as usize;
    let Some(fallback_block) = function.blocks().get(fallback) else {
        return Ok(false);
    };
    project_loop_graph_charge_v1(work, fallback_block.statements().len())?;
    if !switch_fallback_is_empty_unreachable_v1(function, fallback) {
        return Ok(false);
    }
    project_loop_graph_charge_v1(work, targets.values().len())?;
    let Some(discriminant_type) = types.get(discriminant.ty().index() as usize) else {
        return Ok(false);
    };
    if matches!(
        discriminant_type.shape(),
        SemanticTypeShapeV1::Scalar(SemanticScalarTypeV1::Bool)
    ) {
        return Ok(targets.values().len() == 2
            && targets.values().iter().any(|target| target.value() == 0)
            && targets.values().iter().any(|target| target.value() == 1));
    }
    let Some(local) = simple_operand_local(discriminant) else {
        return Ok(false);
    };
    let index = local.index() as usize;
    if inventory.counts.get(index).copied() != Some(1)
        || inventory.address_escaped.get(index).copied() != Some(false)
        || function.locals().get(index).map(|local| local.ty()) != Some(discriminant.ty())
    {
        return Ok(false);
    }
    let Some(site) = inventory.assignments.get(index).copied().flatten() else {
        return Ok(false);
    };
    if site.block != block_index {
        return Ok(false);
    }
    let Some(statement) = block.statements().get(site.statement) else {
        return Ok(false);
    };
    let SemanticStatementKindV1::Assign(assignment) = statement.kind() else {
        return Ok(false);
    };
    if assignment.destination().local() != local
        || !assignment.destination().projections().is_empty()
        || assignment.destination().ty() != discriminant.ty()
        || assignment.value().result_type() != discriminant.ty()
    {
        return Ok(false);
    }
    let SemanticRvalueKindV1::Discriminant(source) = assignment.value().kind() else {
        return Ok(false);
    };
    if !source.projections().is_empty()
        || function
            .locals()
            .get(source.local().index() as usize)
            .map(|local| local.ty())
            != Some(source.ty())
    {
        return Ok(false);
    }
    let Some(SemanticTypeShapeV1::Enum {
        discriminant: enum_discriminant,
        variants,
    }) = types
        .get(source.ty().index() as usize)
        .map(SemanticTypeDeclV1::shape)
    else {
        return Ok(false);
    };
    if *enum_discriminant != discriminant.ty() {
        return Ok(false);
    }
    project_loop_graph_charge_v1(work, block.statements().len() - site.statement - 1)?;
    if block.statements()[site.statement + 1..]
        .iter()
        .any(|statement| {
            matches!(statement.kind(), SemanticStatementKindV1::StorageLive(candidate)
            | SemanticStatementKindV1::StorageDead(candidate) if *candidate == local)
        })
    {
        return Ok(false);
    }
    // The current captured tag has the enum's valid domain, not an arbitrary integer domain.
    // The original enum may change later without changing this unescaped scalar snapshot.
    project_loop_graph_charge_v1(
        work,
        variants
            .len()
            .saturating_mul(targets.values().len().saturating_add(2)),
    )?;
    Ok(switch_targets_exhaust_inhabited_variants_v1(
        variants, targets,
    ))
}
