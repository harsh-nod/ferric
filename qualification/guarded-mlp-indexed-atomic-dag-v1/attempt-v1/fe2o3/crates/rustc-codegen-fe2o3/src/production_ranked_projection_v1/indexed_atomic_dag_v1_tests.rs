fn atomic_dag_test_graph_v1(successors: Vec<Vec<usize>>) -> ProjectedLoopCfgV1 {
    let count = successors.len();
    let mut predecessors = vec![Vec::new(); count];
    for (block, targets) in successors.iter().enumerate() {
        for &target in targets {
            if target < count {
                predecessors[target].push(block);
            }
        }
    }
    let mut reachable = vec![false; count];
    let mut pending = if count == 0 { Vec::new() } else { vec![0] };
    while let Some(block) = pending.pop() {
        if block >= count || reachable[block] {
            continue;
        }
        reachable[block] = true;
        pending.extend(successors[block].iter().copied());
    }
    ProjectedLoopCfgV1 {
        successors,
        predecessors,
        reachable,
        entry: 0,
        elided_switch_fallbacks: vec![false; count],
    }
}

fn atomic_dag_reference_v1(successors: &[Vec<usize>]) -> Vec<bool> {
    let count = successors.len();
    let mut paths = vec![vec![false; count]; count];
    for (source, targets) in successors.iter().enumerate() {
        for &target in targets {
            paths[source][target] = true;
        }
    }
    for middle in 0..count {
        for source in 0..count {
            for target in 0..count {
                let via_middle = paths[source][middle] && paths[middle][target];
                paths[source][target] |= via_middle;
            }
        }
    }
    (0..count).map(|block| !paths[block][block]).collect()
}

#[test]
fn indexed_atomic_dag_certificate_accepts_chains_and_diamonds() {
    for successors in [
        vec![vec![]],
        vec![vec![1], vec![2], vec![]],
        vec![vec![1, 2], vec![3], vec![3], vec![]],
        vec![vec![1, 1], vec![]],
    ] {
        let graph = atomic_dag_test_graph_v1(successors);
        let mut cache = None;
        let mut work = 0;
        for definition in 0..graph.successors.len() {
            assert!(indexed_atomic_block_acyclic_with_cache_v1(
                &graph, definition, &mut cache, &mut work,
            )
            .unwrap());
            assert_eq!(cache, Some(true));
        }
    }
}

#[test]
fn indexed_atomic_dag_certificate_matches_reference_on_all_small_graphs() {
    let mut queries = 0;
    for count in 1..=3 {
        for mask in 0_usize..(1 << (count * count)) {
            let successors = (0..count)
                .map(|source| {
                    (0..count)
                        .filter(|&target| mask & (1 << (source * count + target)) != 0)
                        .collect()
                })
                .collect::<Vec<_>>();
            let expected = atomic_dag_reference_v1(&successors);
            let graph = atomic_dag_test_graph_v1(successors);
            let mut cache = None;
            let mut work = 0;
            for (definition, expected) in expected.iter().copied().enumerate() {
                assert_eq!(
                    indexed_atomic_block_acyclic_with_cache_v1(
                        &graph, definition, &mut cache, &mut work,
                    )
                    .unwrap(),
                    expected,
                    "count={count} mask={mask} definition={definition}"
                );
                queries += 1;
            }
        }
    }
    assert_eq!(queries, 1570);
}

#[test]
fn indexed_atomic_dag_certificate_cycles_keep_per_definition_fallback() {
    let graph = atomic_dag_test_graph_v1(vec![vec![1], vec![2], vec![1], vec![0]]);
    let mut cache = None;
    let mut work = 0;
    for (definition, expected) in [true, false, false, true].into_iter().enumerate() {
        let before = work;
        assert_eq!(
            indexed_atomic_block_acyclic_with_cache_v1(
                &graph, definition, &mut cache, &mut work,
            )
            .unwrap(),
            expected
        );
        assert_eq!(cache, Some(false));
        assert!(work > before + 1, "a cyclic graph must retain charged fallback");
    }
}

#[test]
fn indexed_atomic_dag_certificate_unreachable_cycles_do_not_grant_global_certificate() {
    let graph = atomic_dag_test_graph_v1(vec![vec![1], vec![], vec![2]]);
    assert_eq!(graph.reachable, [true, true, false]);
    let mut cache = None;
    let mut work = 0;
    assert!(indexed_atomic_block_acyclic_with_cache_v1(&graph, 0, &mut cache, &mut work).unwrap());
    assert_eq!(cache, Some(false));
    assert!(!indexed_atomic_block_acyclic_with_cache_v1(&graph, 2, &mut cache, &mut work).unwrap());
}

#[test]
fn indexed_atomic_dag_certificate_rejects_invalid_query_before_cache_hit() {
    let graph = atomic_dag_test_graph_v1(vec![vec![]]);
    for initial in [None, Some(false), Some(true)] {
        let mut cache = initial;
        let mut work = 0;
        assert!(indexed_atomic_block_acyclic_with_cache_v1(
            &graph, 1, &mut cache, &mut work,
        )
        .is_err());
        assert_eq!(cache, initial);
        assert_eq!(work, 0);
    }
}

#[test]
fn indexed_atomic_dag_certificate_refuses_malformed_graph_without_publishing_cache() {
    for successors in [
        vec![vec![1]],
        vec![vec![]; MAX_RANKED_BOUNDS_BLOCKS + 1],
        vec![vec![0; MAX_RANKED_BOUNDS_EDGES + 1]],
    ] {
        let graph = atomic_dag_test_graph_v1(successors);
        let mut cache = None;
        let mut work = 0;
        assert!(indexed_atomic_block_acyclic_with_cache_v1(
            &graph, 0, &mut cache, &mut work,
        )
        .is_err());
        assert_eq!(cache, None);
    }
    let mut work = 0;
    assert!(indexed_atomic_cfg_is_dag_v1(&[], &mut work).is_err());
    assert_eq!(work, 0);
}

#[test]
fn indexed_atomic_dag_certificate_charges_exact_budget_and_never_caches_failed_queries() {
    let graph = atomic_dag_test_graph_v1(vec![vec![1, 2], vec![3], vec![3], vec![]]);
    let cost = 1 + 5 * 4 + 2 * 4;
    for remaining in 0..cost {
        let mut cache = None;
        let mut work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - remaining;
        assert!(indexed_atomic_block_acyclic_with_cache_v1(
            &graph, 0, &mut cache, &mut work,
        )
        .is_err());
        assert_eq!(cache, None);
    }
    let mut cache = None;
    let mut work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - cost;
    assert!(indexed_atomic_block_acyclic_with_cache_v1(&graph, 0, &mut cache, &mut work).unwrap());
    assert_eq!(work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
    assert_eq!(cache, Some(true));
    assert!(indexed_atomic_block_acyclic_with_cache_v1(&graph, 0, &mut cache, &mut work).is_err());
    assert_eq!(cache, Some(true));

    let graph = atomic_dag_test_graph_v1(vec![vec![0]]);
    let mut cache = None;
    let mut work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - 7;
    assert!(indexed_atomic_block_acyclic_with_cache_v1(&graph, 0, &mut cache, &mut work).is_err());
    assert_eq!(cache, None, "successful negative construction plus failed fallback is not published");
    let mut work = usize::MAX;
    assert!(indexed_atomic_block_acyclic_with_cache_v1(&graph, 0, &mut cache, &mut work).is_err());
    assert_eq!(cache, None);
}

#[test]
fn indexed_atomic_dag_certificate_reuses_only_within_independent_proof_instances() {
    let function = cfg_diagnostic_function_v1(2);
    let mut first_proof = SemanticAssertProofsV1::new(&[], &function).unwrap();
    let mut second_proof = SemanticAssertProofsV1::new(&[], &function).unwrap();
    assert_eq!(first_proof.indexed_atomic_cfg_acyclic, None);
    assert_eq!(second_proof.indexed_atomic_cfg_acyclic, None);
    assert!(indexed_atomic_block_acyclic_v1(&mut first_proof, 0).unwrap());
    assert_eq!(first_proof.indexed_atomic_cfg_acyclic, Some(true));
    assert_eq!(second_proof.indexed_atomic_cfg_acyclic, None);
    let construction_work = first_proof.work;
    assert!(indexed_atomic_block_acyclic_v1(&mut first_proof, 1).unwrap());
    assert_eq!(first_proof.work, construction_work + 1);
    assert!(indexed_atomic_block_acyclic_v1(&mut second_proof, 0).unwrap());
    assert_eq!(second_proof.work, construction_work);

    let first = atomic_dag_test_graph_v1(vec![vec![1], vec![]]);
    let second = atomic_dag_test_graph_v1(vec![vec![1], vec![0]]);
    let mut first_cache = None;
    let mut second_cache = None;
    let mut first_work = 0;
    let mut second_work = 0;
    assert!(indexed_atomic_block_acyclic_with_cache_v1(
        &first, 0, &mut first_cache, &mut first_work,
    )
    .unwrap());
    let before = first_work;
    assert!(indexed_atomic_block_acyclic_with_cache_v1(
        &first, 1, &mut first_cache, &mut first_work,
    )
    .unwrap());
    assert_eq!(first_work, before + 1);
    assert!(!indexed_atomic_block_acyclic_with_cache_v1(
        &second, 0, &mut second_cache, &mut second_work,
    )
    .unwrap());
    assert_eq!(first_cache, Some(true));
    assert_eq!(second_cache, Some(false));
}

#[test]
fn indexed_atomic_dag_certificate_bounds_large_acyclic_query_work() {
    for count in [1121, MAX_RANKED_BOUNDS_BLOCKS] {
        let graph = atomic_dag_test_graph_v1(
            (0..count)
                .map(|block| if block + 1 < count { vec![block + 1] } else { vec![] })
                .collect(),
        );
        let mut cache = None;
        let mut work = 0;
        for definition in 0..count {
            assert!(indexed_atomic_block_acyclic_with_cache_v1(
                &graph, definition, &mut cache, &mut work,
            )
            .unwrap());
        }
        assert_eq!(work, 6 * count + 2 * (count - 1));
        assert!(work < MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
    }
}
