use super::*;

// Exact qualified FIFO loop; only the function name changes.
pub(super) fn validate_partial_moves_fifo_v1(
    function_id: SemanticFunctionIdV1,
    function: &SemanticFunctionDeclV1,
    types: Option<&[SemanticTypeDeclV1]>,
    plan: &SsaConstructionPlanV1,
    auxiliary_resources: SemanticSsaAuxiliaryResourcesV1,
    limits: ProductionSemanticSsaLimitsV1,
) -> Result<ProductionSemanticPartialMoveCertificateV1, ProductionSemanticSsaErrorV1> {
    let (projected_moves, _) = projected_local_move_metrics_v1(function)?;
    if projected_moves == 0 {
        return Ok(ProductionSemanticPartialMoveCertificateV1::default());
    }

    let mut budget = SemanticPartialMoveBudgetV1 {
        function: function_id,
        base_storage_words: plan
            .resources()
            .storage_words()
            .checked_add(auxiliary_resources.storage_words)
            .ok_or(ProductionSemanticSsaErrorV1::ResourceOverflow)?,
        base_work_units: plan
            .resources()
            .work_units()
            .checked_add(auxiliary_resources.work_units)
            .ok_or(ProductionSemanticSsaErrorV1::ResourceOverflow)?,
        state_entries: 0,
        work_units: 0,
        limits: limits.planner(),
    };
    let mut incoming = vec![None::<SemanticPartialMoveStateV1>; function.blocks().len()];
    let entry = function.entry().index() as usize;
    incoming[entry] = Some(BTreeMap::new());
    let mut pending = VecDeque::from([entry]);
    let mut queued = vec![false; function.blocks().len()];
    queued[entry] = true;
    let return_local = function
        .locals()
        .iter()
        .position(|local| matches!(local.role(), SemanticLocalRoleV1::Return))
        .map(|local| local as u32);

    while let Some(block_index) = pending.pop_front() {
        queued[block_index] = false;
        if !plan.is_reachable(SsaBlockIdV1::new(block_index as u32)) {
            continue;
        }
        budget.charge_work()?;
        let mut state = incoming[block_index]
            .as_ref()
            .cloned()
            .ok_or(ProductionSemanticSsaErrorV1::ReplayMismatch)?;
        let block = &function.blocks()[block_index];
        for (statement_index, statement) in block.statements().iter().enumerate() {
            let location = SemanticPartialMoveLocationV1 {
                function: function_id,
                block: block_index as u32,
                statement: Some(statement_index as u32),
            };
            validate_partial_move_statement_v1(
                function,
                types,
                statement.kind(),
                location,
                &mut state,
                &mut budget,
            )?;
        }
        let location = SemanticPartialMoveLocationV1 {
            function: function_id,
            block: block_index as u32,
            statement: None,
        };
        validate_partial_move_terminator_v1(
            function,
            types,
            block.terminator().kind(),
            return_local,
            location,
            &mut state,
            &mut budget,
        )?;

        block.terminator().kind().try_for_each_edge(|edge| {
            let target = edge.target().index() as usize;
            if !plan.is_reachable(SsaBlockIdV1::new(target as u32)) {
                return Ok(());
            }
            let mut edge_state = state.clone();
            if let SemanticTerminatorKindV1::Call(call) = block.terminator().kind()
                && let Some(destination) = call.destination()
                && destination.edge() == edge
            {
                apply_partial_move_destination_write_v1(
                    function,
                    types,
                    destination.place(),
                    location,
                    &mut edge_state,
                    &mut budget,
                )?;
            }
            let first_incoming_edge = incoming[target].is_none();
            let changed = merge_partial_move_state_v1(
                incoming[target].get_or_insert_with(BTreeMap::new),
                &edge_state,
                &mut budget,
            )?;
            if (first_incoming_edge || changed) && !queued[target] {
                queued[target] = true;
                pending.push_back(target);
            }
            Ok(())
        })?;
    }

    Ok(ProductionSemanticPartialMoveCertificateV1 {
        projected_moves,
        state_entries: budget.state_entries,
        work_units: budget.work_units,
    })
}
