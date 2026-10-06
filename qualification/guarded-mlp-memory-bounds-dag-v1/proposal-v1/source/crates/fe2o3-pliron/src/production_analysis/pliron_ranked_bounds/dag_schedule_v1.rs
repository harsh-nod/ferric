/// A private, whole-function DAG schedule retained across one bounds stage.
/// The runtime rejoins both identities and the exact successor multigraph.
pub(crate) struct RankedBoundsDagScheduleV1 {
    function: pliron::context::Ptr<Operation>,
    blocks: Vec<pliron::context::Ptr<pliron::basic_block::BasicBlock>>,
    successors: Vec<Vec<usize>>,
    order: Vec<usize>,
}

pub(crate) struct RankedBoundsPreflightV1 {
    upper_bound: ProductionAnalysisResourceUpperBoundV1,
    dag: Option<RankedBoundsDagScheduleV1>,
}

impl RankedBoundsPreflightV1 {
    pub(crate) const fn upper_bound(&self) -> ProductionAnalysisResourceUpperBoundV1 {
        self.upper_bound
    }

    pub(crate) fn dag_schedule(&self) -> Option<&RankedBoundsDagScheduleV1> {
        self.dag.as_ref()
    }
}

fn ranked_bounds_dag_resources_v1(
    census: ProductionAnalysisInputCensusV1,
) -> Result<(usize, usize, usize), ProductionAnalysisResourceLimitV1> {
    let error = "memory-bounds DAG schedule upper bound";
    let discovery_work = checked_ranked_bounds_sum_v1(
        &[
            checked_ranked_bounds_product_v1(census.operations, 2, error)?,
            checked_ranked_bounds_product_v1(census.blocks, 12, error)?,
            checked_ranked_bounds_product_v1(census.successors, 8, error)?,
        ],
        error,
    )?;
    let validation_work = checked_ranked_bounds_sum_v1(
        &[
            checked_ranked_bounds_product_v1(census.blocks, 8, error)?,
            checked_ranked_bounds_product_v1(census.successors, 4, error)?,
        ],
        error,
    )?;
    // This sum covers construction scratch, retained identities/edges/order,
    // and the runtime position map even if their lifetimes overlap.
    let storage = checked_ranked_bounds_sum_v1(
        &[
            checked_ranked_bounds_product_v1(census.blocks, 12, error)?,
            checked_ranked_bounds_product_v1(census.successors, 4, error)?,
        ],
        error,
    )?;
    Ok((discovery_work, validation_work, storage))
}

/// Try a different bounded algorithm only after the old wave bound refuses.
/// Previously admitted graphs keep their exact FIFO path and resource bound.
pub(crate) fn preflight_ranked_bounds_with_dag_v1(
    context: &Context,
    function: &FuncOp,
    inventory: &crate::production_analysis::pliron_function_inventory::BoundedPlironFunctionInventoryV1,
    census: ProductionAnalysisInputCensusV1,
    limits: ProductionAnalysisResourceLimitsV1,
) -> Result<RankedBoundsPreflightV1, ProductionAnalysisResourceLimitV1> {
    let previous = match preflight_ranked_bounds_resource_upper_bound_v1(census, limits) {
        Ok(upper_bound) => return Ok(RankedBoundsPreflightV1 { upper_bound, dag: None }),
        Err(error) => error,
    };
    // Ownership may rerun the legacy FIFO analysis under the composed bound.
    // Do not supply a one-pass bound to that independently scheduled consumer.
    if previous.resource != "memory-bounds work hard limit" || census.ownership_contracts != 0 {
        return Err(previous);
    }
    let (discovery_work, validation_work, storage) = ranked_bounds_dag_resources_v1(census)?;
    let phase = ProductionAnalysisResourcePhaseV1::MemoryBounds;
    // Admission precedes every new allocation or graph traversal. An early
    // refusal consumes no schedule, and cannot enable the one-pass estimate.
    let discovery = ProductionAnalysisResourceUpperBoundV1::checked_phase(
        phase, discovery_work, 0, storage,
    )?;
    limits.require(phase, discovery)?;
    let Some(dag) = collect_ranked_bounds_dag_v1(context, function, inventory, census) else {
        return Err(previous);
    };
    let work = checked_ranked_bounds_sum_v1(
        &[discovery_work, validation_work], "memory-bounds DAG schedule upper bound",
    )?;
    let upper_bound = preflight_ranked_bounds_resource_upper_bound_impl_v1(
        census, limits, Some((work, storage)),
    )?;
    Ok(RankedBoundsPreflightV1 { upper_bound, dag: Some(dag) })
}

fn collect_ranked_bounds_dag_v1(
    context: &Context,
    function: &FuncOp,
    inventory: &crate::production_analysis::pliron_function_inventory::BoundedPlironFunctionInventoryV1,
    census: ProductionAnalysisInputCensusV1,
) -> Option<RankedBoundsDagScheduleV1> {
    let blocks = inventory.blocks();
    if blocks.is_empty() || blocks.len() != census.blocks
        || blocks.len() > MAX_RANKED_BOUNDS_BLOCKS
        || inventory.operations().len() != census.operations
        || census.operations > MAX_RANKED_BOUNDS_OPERATIONS
        || census.successors > MAX_RANKED_BOUNDS_EDGES
    {
        return None;
    }
    if !ranked_bounds_inventory_matches_function_v1(context, function, blocks) {
        return None;
    }
    let mut indices = HashMap::new();
    indices.try_reserve(blocks.len()).ok()?;
    for (index, block) in blocks.iter().copied().enumerate() {
        indices.insert(block, index);
    }
    if indices.len() != blocks.len() {
        return None;
    }
    let mut successors = reserved_ranked_bounds_vec_v1(blocks.len())?;
    let mut indegrees = reserved_ranked_bounds_vec_v1(blocks.len())?;
    indegrees.resize(blocks.len(), 0_usize);
    let mut edges = 0_usize;
    for (block_index, block) in blocks.iter().enumerate() {
        let terminator = block.deref(context).get_terminator(context)?;
        if inventory.block_operations(block_index).last()?.pointer() != terminator {
            return None;
        }
        let raw = terminator.deref(context);
        edges = edges.checked_add(raw.get_num_successors())?;
        if edges > census.successors {
            return None;
        }
        let mut targets = reserved_ranked_bounds_vec_v1(raw.get_num_successors())?;
        for successor in raw.successors() {
            let target = *indices.get(&successor)?;
            indegrees[target] = indegrees[target].checked_add(1)?;
            targets.push(target);
        }
        successors.push(targets);
    }
    if edges != census.successors {
        return None;
    }
    let mut pending = VecDeque::new();
    pending.try_reserve_exact(blocks.len()).ok()?;
    for (block, indegree) in indegrees.iter().enumerate() {
        if *indegree == 0 {
            pending.push_back(block);
        }
    }
    let mut order = reserved_ranked_bounds_vec_v1(blocks.len())?;
    while let Some(block) = pending.pop_front() {
        order.push(block);
        for target in &successors[block] {
            indegrees[*target] -= 1;
            if indegrees[*target] == 0 {
                pending.push_back(*target);
            }
        }
    }
    if order.len() != blocks.len() {
        return None;
    }
    let mut retained_blocks = reserved_ranked_bounds_vec_v1(blocks.len())?;
    retained_blocks.extend_from_slice(blocks);
    Some(RankedBoundsDagScheduleV1 {
        function: function.get_operation(), blocks: retained_blocks, successors, order,
    })
}

fn reserved_ranked_bounds_vec_v1<T>(capacity: usize) -> Option<Vec<T>> {
    let mut values = Vec::new();
    values.try_reserve_exact(capacity).ok()?;
    Some(values)
}

fn ranked_bounds_inventory_matches_function_v1(
    context: &Context,
    function: &FuncOp,
    blocks: &[pliron::context::Ptr<pliron::basic_block::BasicBlock>],
) -> bool {
    use pliron::builtin::op_interfaces::OneRegionInterface;
    use pliron::linked_list::ContainsLinkedList;

    function.get_region(context).deref(context).iter(context)
        .take(blocks.len().saturating_add(1)).eq(blocks.iter().copied())
}

impl RankedBoundsDagScheduleV1 {
    fn validate_runtime(
        &self,
        context: &Context,
        function: &FuncOp,
        blocks: &[pliron::context::Ptr<pliron::basic_block::BasicBlock>],
        successors: &[Vec<usize>],
        budget: &mut RankedBoundsBudget,
    ) -> Result<(), RankedBoundsFindingV1> {
        let blocks_count = blocks.len();
        budget.work(blocks_count)?;
        let edges = successors.iter().try_fold(0_usize, |sum, targets| sum.checked_add(targets.len()))
            .ok_or(RankedBoundsFindingV1::StructuralVerificationFailed)?;
        let work = blocks_count.checked_mul(7)
            .and_then(|work| edges.checked_mul(4).and_then(|more| work.checked_add(more)))
            .ok_or(RankedBoundsFindingV1::StructuralVerificationFailed)?;
        budget.work(work)?;
        let storage = blocks_count.checked_mul(4).and_then(|items| items.checked_add(edges))
            .ok_or(RankedBoundsFindingV1::StructuralVerificationFailed)?;
        // Account the retained certificate and the temporary position map.
        budget.storage(storage)?;
        if self.function != function.get_operation() || self.blocks != blocks
            || self.successors != successors || self.order.len() != blocks_count
            || !ranked_bounds_inventory_matches_function_v1(context, function, blocks)
        {
            return Err(RankedBoundsFindingV1::StructuralVerificationFailed);
        }
        let mut positions = reserved_ranked_bounds_vec_v1(blocks_count).ok_or_else(|| {
            bounds_transport_failure_v1("analysis storage item", MAX_RANKED_BOUNDS_STORAGE_ITEMS, usize::MAX)
        })?;
        positions.resize(blocks_count, usize::MAX);
        for (position, block) in self.order.iter().copied().enumerate() {
            if block >= blocks_count || positions[block] != usize::MAX {
                return Err(RankedBoundsFindingV1::StructuralVerificationFailed);
            }
            positions[block] = position;
        }
        for (block, targets) in successors.iter().enumerate() {
            if targets.iter().any(|target| *target >= blocks_count || positions[block] >= positions[*target]) {
                return Err(RankedBoundsFindingV1::StructuralVerificationFailed);
            }
        }
        Ok(())
    }
}
