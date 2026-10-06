// The cache belongs to one immutable SemanticAssertProofsV1 graph. A failed
// construction/query publishes no new certificate; cycles use the old query.
fn indexed_atomic_block_acyclic_with_cache_v1(
    graph: &ProjectedLoopCfgV1,
    definition: usize,
    cache: &mut Option<bool>,
    work: &mut usize,
) -> Result<bool, ProductionRankedProjectionErrorV1> {
    if definition >= graph.successors.len() {
        return Err(ProductionRankedProjectionErrorV1::Unsupported(
            "indexed atomic definition is outside the semantic CFG",
        ));
    }
    project_loop_graph_charge_v1(work, 1)?;
    let whole_graph_acyclic = match *cache {
        Some(acyclic) => acyclic,
        None => indexed_atomic_cfg_is_dag_v1(&graph.successors, work)?,
    };
    let result = if whole_graph_acyclic {
        true
    } else {
        indexed_atomic_block_acyclic_fallback_v1(graph, definition, work)?
    };
    *cache = Some(whole_graph_acyclic);
    Ok(result)
}

fn indexed_atomic_cfg_is_dag_v1(
    successors: &[Vec<usize>],
    work: &mut usize,
) -> Result<bool, ProductionRankedProjectionErrorV1> {
    let count = successors.len();
    if count == 0 || count > MAX_RANKED_BOUNDS_BLOCKS {
        return Err(ProductionRankedProjectionErrorV1::Unsupported(
            "indexed atomic DAG certificate exceeds the ranked block limit",
        ));
    }
    // Charge both bounded scratch arrays before reserving or initializing them.
    project_loop_graph_charge_v1(work, count)?;
    project_loop_graph_charge_v1(work, count)?;
    let mut indegrees = Vec::new();
    indegrees.try_reserve_exact(count).map_err(|_| {
        ProductionRankedProjectionErrorV1::Unsupported(
            "indexed atomic DAG indegree storage cannot be reserved",
        )
    })?;
    indegrees.resize(count, 0_usize);
    let mut ready = Vec::new();
    ready.try_reserve_exact(count).map_err(|_| {
        ProductionRankedProjectionErrorV1::Unsupported(
            "indexed atomic DAG worklist storage cannot be reserved",
        )
    })?;
    let mut edges = 0_usize;
    for targets in successors {
        project_loop_graph_charge_v1(work, 1)?;
        for &target in targets {
            project_loop_graph_charge_v1(work, 1)?;
            if target >= count {
                return Err(ProductionRankedProjectionErrorV1::Unsupported(
                    "indexed atomic DAG edge is outside the semantic CFG",
                ));
            }
            edges = edges.checked_add(1).ok_or(
                ProductionRankedProjectionErrorV1::Unsupported(
                    "indexed atomic DAG edge count overflow",
                ),
            )?;
            if edges > MAX_RANKED_BOUNDS_EDGES {
                return Err(ProductionRankedProjectionErrorV1::Unsupported(
                    "indexed atomic DAG certificate exceeds the ranked edge limit",
                ));
            }
            indegrees[target] = indegrees[target].checked_add(1).ok_or(
                ProductionRankedProjectionErrorV1::Unsupported(
                    "indexed atomic DAG indegree overflow",
                ),
            )?;
        }
    }
    for (block, &degree) in indegrees.iter().enumerate() {
        project_loop_graph_charge_v1(work, 1)?;
        if degree == 0 {
            ready.push(block);
        }
    }
    let mut cursor = 0_usize;
    while cursor < ready.len() {
        project_loop_graph_charge_v1(work, 1)?;
        let block = ready[cursor];
        cursor += 1;
        for &target in &successors[block] {
            project_loop_graph_charge_v1(work, 1)?;
            indegrees[target] = indegrees[target].checked_sub(1).ok_or(
                ProductionRankedProjectionErrorV1::Unsupported(
                    "indexed atomic DAG indegree underflow",
                ),
            )?;
            if indegrees[target] == 0 {
                if ready.len() == count {
                    return Err(ProductionRankedProjectionErrorV1::Unsupported(
                        "indexed atomic DAG worklist exceeds the ranked block count",
                    ));
                }
                ready.push(target);
            }
        }
    }
    Ok(cursor == count)
}

fn indexed_atomic_block_acyclic_fallback_v1(
    graph: &ProjectedLoopCfgV1,
    definition: usize,
    work: &mut usize,
) -> Result<bool, ProductionRankedProjectionErrorV1> {
    let count = graph.successors.len();
    project_loop_graph_charge_v1(work, count)?;
    let mut seen = vec![false; count];
    let mut pending = VecDeque::new();
    for next in graph.successors[definition].clone() {
        project_loop_graph_charge_v1(work, 1)?;
        if next == definition {
            return Ok(false);
        }
        if !seen[next] {
            seen[next] = true;
            pending.push_back(next);
        }
    }
    while let Some(block) = pending.pop_front() {
        project_loop_graph_charge_v1(work, 1)?;
        for next in graph.successors[block].clone() {
            project_loop_graph_charge_v1(work, 1)?;
            if next == definition {
                return Ok(false);
            }
            if !seen[next] {
                seen[next] = true;
                pending.push_back(next);
            }
        }
    }
    Ok(true)
}
