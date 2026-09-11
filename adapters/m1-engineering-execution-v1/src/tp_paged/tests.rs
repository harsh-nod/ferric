use super::*;

#[test]
fn large_kv_limits_are_explicit_and_do_not_change_logical_or_legacy_bounds() {
    for pages in [1, 512, 513, 8192, 8704, 16384] {
        let limits = EngineeringTpPagedLimitsV1::new_large_kv32(8192, 32, pages, 100).unwrap();
        assert_eq!(limits.context_tokens(), 8192);
        assert_eq!(limits.page_table_stride(), 512);
        assert_eq!(limits.physical_token_capacity().unwrap(), pages * 16);
        assert_eq!(
            limits.target_kv_payload_bytes().unwrap(),
            u64::from(pages) * 2_359_296
        );
        assert!(EngineeringTpPagedPoolV1::new(scope(), limits).is_err());
        assert!(EngineeringTpPagedPoolV1::new_wide32(scope(), limits).is_err());
        let pool = EngineeringTpPagedPoolV1::new_large_kv32_recording(scope(), limits).unwrap();
        assert_eq!(pool.row_capacity(), 32);
        assert_eq!(
            pool.limits().profile(),
            EngineeringTpKvPoolProfileV1::LargeV9
        );
        pool.check_invariants().unwrap();
        if pages > 512 {
            assert!(EngineeringTpPagedLimitsV1::new(8192, 32, pages, 100).is_err());
        }
    }
    assert_eq!(
        EngineeringTpPagedLimitsV1::new_large_kv32(8192, 32, 16384, 100)
            .unwrap()
            .target_kv_payload_bytes(),
        Ok(38_654_705_664)
    );
    for (context, sequences, pages, ttl) in [
        (0, 1, 1, 1),
        (8193, 1, 1, 1),
        (8192, 33, 1, 1),
        (1, 1, 0, 1),
        (1, 1, 16385, 1),
        (1, 1, u32::MAX, 1),
        (1, 1, 1, 0),
    ] {
        assert!(
            EngineeringTpPagedLimitsV1::new_large_kv32(context, sequences, pages, ttl).is_err()
        );
    }
    let legacy = EngineeringTpPagedLimitsV1::new(8192, 32, 512, 100).unwrap();
    assert!(EngineeringTpPagedPoolV1::new_large_kv32_recording(scope(), legacy).is_err());
}

#[test]
fn large_kv_actual_reservations_cross_page_512_and_remain_fail_atomic() {
    let limits = EngineeringTpPagedLimitsV1::new_large_kv32(288, 32, 544, 100).unwrap();
    let mut pool = EngineeringTpPagedPoolV1::new_large_kv32_recording(scope(), limits).unwrap();
    let mut sequences = Vec::new();
    for token in 1..=32 {
        let sequence = pool
            .open_sequence(scope(), &[token; 272], 0)
            .unwrap()
            .sequence();
        append(&mut pool, sequence, &[token; 272]);
        sequences.push(sequence);
    }
    assert_eq!(
        pool.state.sequences[&32].pages,
        (527..544).collect::<Vec<_>>()
    );
    assert_eq!(
        pool.state
            .pages
            .iter()
            .filter(|page| page.refs != 0)
            .count(),
        544
    );
    pool.check_invariants().unwrap();
    let before = pool.state.clone();
    assert_eq!(
        pool.reserve_batch(&[row(sequences[31], 272, 1)])
            .unwrap_err(),
        Error::OutOfPages
    );
    assert_eq!(pool.state, before);
    for sequence in sequences {
        pool.cancel_sequence(sequence).unwrap();
    }
    assert!(pool.state.pages.iter().all(|page| page.refs == 0));
    pool.check_invariants().unwrap();
}

#[test]
fn large_kv_final_physical_slot_preserves_logical_8192_and_quarantine() {
    let limits = EngineeringTpPagedLimitsV1::new_large_kv32(8192, 32, 16384, 100).unwrap();
    let mut pool = EngineeringTpPagedPoolV1::new_large_kv32_recording(scope(), limits).unwrap();
    // Construct a complete invariant-checked boundary state without 8192 setup transactions.
    for serial in 1_u64..=32 {
        let first = u32::try_from(serial - 1).unwrap() * 512;
        pool.state.sequences.insert(
            serial,
            Sequence {
                tokens: vec![7; if serial == 32 { 8191 } else { 8192 }],
                pages: (first..first + 512).collect(),
            },
        );
    }
    pool.state.next_sequence = 33;
    for page in &mut pool.state.pages {
        page.refs = 1;
    }
    pool.check_invariants().unwrap();
    let sequence = EngineeringTpSequenceIdV1 {
        pool: pool.identity,
        serial: 32,
    };
    let before = pool.state.clone();
    let batch = pool.reserve_batch(&[row(sequence, 8191, 8)]).unwrap();
    assert_eq!(batch.rows()[0].writable_physical_page(), 16383);
    assert_eq!(batch.rows()[0].writable_token_offset(), 15);
    assert_eq!(batch.rows()[0].physical_pages().len(), 512);
    pool.abort_batch(&batch).unwrap();
    assert_eq!(pool.state, before);
    let batch = pool.reserve_batch(&[row(sequence, 8191, 8)]).unwrap();
    pool.begin_submission(&batch).unwrap();
    pool.commit_batch(
        &batch,
        EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
    )
    .unwrap();
    assert_eq!(pool.committed_position(sequence), Ok(8192));
    assert_eq!(
        pool.reserve_batch(&[row(sequence, 8192, 8)]).unwrap_err(),
        Error::InvalidRows
    );
    pool.check_invariants().unwrap();
    pool.cancel_sequence(sequence).unwrap();
    let fresh = pool.open_sequence(scope(), &[9], 1).unwrap().sequence();
    let batch = pool.reserve_batch(&[row(fresh, 0, 9)]).unwrap();
    pool.begin_submission(&batch).unwrap();
    assert!(pool.abort_batch(&batch).is_err());
    pool.quarantine_batch(&batch).unwrap();
    assert_eq!(
        pool.open_sequence(scope(), &[9], 2).unwrap_err(),
        Error::Poisoned
    );
}

#[test]
fn large_kv_high_cached_page_is_immutable_and_partial_pages_stay_exclusive() {
    let limits = EngineeringTpPagedLimitsV1::new_large_kv32(64, 32, 16384, 100).unwrap();
    let mut pool = EngineeringTpPagedPoolV1::new_large_kv32_recording(scope(), limits).unwrap();
    pool.state.pages[16383].cached = true;
    pool.state.pages[16383].refs = 1;
    pool.state.nodes[16383] = Some(RadixNode {
        parent: None,
        edge: [7; 16],
        page: 16383,
        touched: 0,
        expires: 100,
    });
    pool.check_invariants().unwrap();
    let a = pool.open_sequence(scope(), &[7; 17], 1).unwrap();
    let b = pool.open_sequence(scope(), &[7; 17], 2).unwrap();
    assert_eq!(a.physical_pages(), [16383]);
    assert_eq!(b.physical_pages(), [16383]);
    assert_eq!(pool.state.pages[16383].refs, 3);
    let batch = pool
        .reserve_batch(&[row(a.sequence(), 16, 7), row(b.sequence(), 16, 7)])
        .unwrap();
    assert_eq!(batch.rows()[0].physical_pages(), [16383, 0]);
    assert_eq!(batch.rows()[1].physical_pages(), [16383, 1]);
    assert_ne!(
        batch.rows()[0].writable_physical_page(),
        batch.rows()[1].writable_physical_page()
    );
    let mut foreign = EngineeringTpPagedPoolV1::new_large_kv32_recording(scope(), limits).unwrap();
    assert_eq!(foreign.begin_submission(&batch), Err(Error::UnknownBatch));
    pool.begin_submission(&batch).unwrap();
    pool.commit_batch(
        &batch,
        EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
    )
    .unwrap();
    pool.cancel_sequence(a.sequence()).unwrap();
    assert_eq!(pool.state.pages[16383].refs, 2);
    pool.retire_sequence(b.sequence(), true, 3).unwrap();
    assert_eq!(pool.state.pages[16383].refs, 1);
    assert!(pool.state.pages[16383].cached);
    pool.open_sequence(scope(), &[9], 104).unwrap();
    assert!(!pool.state.pages[16383].cached);
    pool.check_invariants().unwrap();
}

#[test]
fn explicit_wide_pool_commits_thirty_two_distinct_causal_slots_once() {
    let limits = EngineeringTpPagedLimitsV1::new(64, 32, 4, 100).unwrap();
    let mut wide = EngineeringTpPagedPoolV1::new_wide32(scope(), limits).unwrap();
    let prompt = (0..32).collect::<Vec<_>>();
    let sequence = wide.open_sequence(scope(), &prompt, 0).unwrap().sequence();
    let rows = prompt
        .iter()
        .enumerate()
        .map(|(index, &token)| row(sequence, u32::try_from(index).unwrap(), token))
        .collect::<Vec<_>>();
    let prepared = wide.reserve_batch(&rows).unwrap();
    assert_eq!(prepared.rows().len(), 32);
    assert_eq!(
        prepared
            .rows()
            .iter()
            .map(|row| (row.writable_physical_page(), row.writable_token_offset()))
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        32
    );
    assert_eq!(wide.committed_position(sequence).unwrap(), 0);
    wide.begin_submission(&prepared).unwrap();
    wide.commit_batch(
        &prepared,
        EngineeringTpBatchCompletionV1::after_all_ranks(&prepared),
    )
    .unwrap();
    assert_eq!(wide.committed_position(sequence).unwrap(), 32);
    wide.check_invariants().unwrap();

    let mut legacy = pool(4);
    let old_sequence = legacy
        .open_sequence(scope(), &prompt, 0)
        .unwrap()
        .sequence();
    let old_rows = prompt
        .iter()
        .enumerate()
        .map(|(index, &token)| row(old_sequence, u32::try_from(index).unwrap(), token))
        .collect::<Vec<_>>();
    assert_eq!(
        legacy.reserve_batch(&old_rows).unwrap_err(),
        Error::InvalidRows
    );
    assert_eq!(legacy.committed_position(old_sequence).unwrap(), 0);
    assert!(wide.reserve_batch(&vec![row(sequence, 32, 1); 33]).is_err());
}

fn scope() -> EngineeringTpPoolScopeV1 {
    EngineeringTpPoolScopeV1 {
        model: [1; 32],
        session: [2; 32],
    }
}
fn pool(pages: u32) -> EngineeringTpPagedPoolV1 {
    EngineeringTpPagedPoolV1::new(
        scope(),
        EngineeringTpPagedLimitsV1::new(128, 32, pages, 100).unwrap(),
    )
    .unwrap()
}
fn row(sequence: EngineeringTpSequenceIdV1, position: u32, token: u32) -> EngineeringTpPageRowV1 {
    EngineeringTpPageRowV1 {
        sequence,
        token,
        position,
    }
}
fn append(
    pool: &mut EngineeringTpPagedPoolV1,
    sequence: EngineeringTpSequenceIdV1,
    tokens: &[u32],
) {
    for chunk in tokens.chunks(16) {
        let start = pool.committed_position(sequence).unwrap();
        let rows: Vec<_> = chunk
            .iter()
            .enumerate()
            .map(|(i, token)| row(sequence, start + count(i), *token))
            .collect();
        let prepared = pool.reserve_batch(&rows).unwrap();
        assert_eq!(pool.committed_position(sequence).unwrap(), start);
        pool.check_invariants().unwrap();
        pool.begin_submission(&prepared).unwrap();
        let completion = EngineeringTpBatchCompletionV1::after_all_ranks(&prepared);
        pool.commit_batch(&prepared, completion).unwrap();
        pool.check_invariants().unwrap();
    }
}
fn publish(pool: &mut EngineeringTpPagedPoolV1, tokens: &[u32], now: u64) {
    let hit = pool.open_sequence(scope(), tokens, now).unwrap();
    append(pool, hit.sequence(), &tokens[hit.hit_tokens() as usize..]);
    pool.retire_sequence(hit.sequence(), true, now).unwrap();
    pool.check_invariants().unwrap();
}

#[test]
fn limits_identity_and_fresh_driver_binding() {
    for values in [
        (0, 1, 1, 1),
        (8193, 1, 1, 1),
        (16, 0, 1, 1),
        (16, 33, 1, 1),
        (16, 1, 0, 1),
        (16, 1, 513, 1),
        (16, 1, 1, 0),
    ] {
        assert_eq!(
            EngineeringTpPagedLimitsV1::new(values.0, values.1, values.2, values.3),
            Err(Error::InvalidLimits)
        );
    }
    let limits = EngineeringTpPagedLimitsV1::new(17, 2, 3, 10).unwrap();
    assert_eq!(limits.page_table_stride(), 2);
    assert_eq!(limits.physical_page_count(), 3);
    assert_eq!(limits.max_sequences(), 2);
    assert_eq!(limits.cache_ttl(), 10);
    assert_eq!(limits.context_tokens(), 17);
    assert!(
        EngineeringTpPagedPoolV1::new(
            EngineeringTpPoolScopeV1 {
                model: [0; 32],
                session: [1; 32]
            },
            limits
        )
        .is_err()
    );
    let mut a = pool(4);
    let b = pool(4);
    assert_ne!(a.identity(), b.identity());
    assert!(a.is_empty());
    let sequence = a.open_sequence(scope(), &[1], 0).unwrap().sequence();
    a.cancel_sequence(sequence).unwrap();
    assert!(!a.is_empty());
}

#[test]
fn all_row_mappings_are_exclusive_contiguous_and_bounded() {
    let mut pool = pool(4);
    let a = pool.open_sequence(scope(), &[1; 17], 0).unwrap().sequence();
    let b = pool.open_sequence(scope(), &[2; 17], 0).unwrap().sequence();
    append(&mut pool, a, &[1; 15]);
    append(&mut pool, b, &[2; 15]);
    let batch = pool
        .reserve_batch(&[row(a, 15, 1), row(b, 15, 2), row(a, 16, 3), row(b, 16, 4)])
        .unwrap();
    assert_eq!(batch.pool_identity(), pool.identity());
    assert_eq!(batch.scope(), pool.scope());
    assert_eq!(batch.limits(), pool.limits());
    assert_eq!(batch.context_tokens(), 128);
    assert_eq!(batch.physical_page_count(), 4);
    assert_eq!(batch.page_table_stride(), 8);
    let mut slots = std::collections::BTreeSet::new();
    for prepared in batch.rows() {
        assert_eq!(prepared.physical_pages().len(), 2);
        assert_eq!(
            prepared.writable_physical_page(),
            prepared.physical_pages()[prepared.position() as usize / 16]
        );
        assert_eq!(prepared.writable_token_offset(), prepared.position() % 16);
        assert!(slots.insert((
            prepared.writable_physical_page(),
            prepared.writable_token_offset()
        )));
        assert!(prepared.writable_physical_page() < 4);
    }
    pool.begin_submission(&batch).unwrap();
    pool.commit_batch(
        &batch,
        EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
    )
    .unwrap();
    assert_eq!(pool.committed_position(a), Ok(17));
    assert_eq!(pool.committed_position(b), Ok(17));
    pool.check_invariants().unwrap();
}

#[test]
fn oom_and_invalid_rows_are_fail_atomic() {
    let mut pool = pool(1);
    let a = pool.open_sequence(scope(), &[1], 0).unwrap().sequence();
    let b = pool.open_sequence(scope(), &[2], 0).unwrap().sequence();
    let original = pool.state.clone();
    let serial = pool.next_batch;
    assert!(matches!(
        pool.reserve_batch(&[row(a, 0, 1), row(b, 0, 2)]),
        Err(Error::OutOfPages)
    ));
    assert_eq!(pool.state, original);
    assert_eq!(pool.next_batch, serial);
    assert!(matches!(
        pool.reserve_batch(&[row(a, 0, 1), row(a, 0, 2)]),
        Err(Error::InvalidRows)
    ));
    assert_eq!(pool.state, original);
    assert!(matches!(pool.reserve_batch(&[]), Err(Error::InvalidRows)));
    assert!(matches!(
        pool.reserve_batch(&[row(a, 0, 1); 17]),
        Err(Error::InvalidRows)
    ));
    assert!(matches!(
        pool.reserve_batch(&[row(a, 128, 1)]),
        Err(Error::InvalidRows)
    ));
    assert_eq!(pool.state, original);
}

#[test]
fn abort_preserves_state_and_stale_batch_cannot_commit() {
    let mut pool = pool(2);
    let sequence = pool.open_sequence(scope(), &[1; 17], 0).unwrap().sequence();
    append(&mut pool, sequence, &[1; 15]);
    let original = pool.state.clone();
    let before = pool.stats();
    let first = pool
        .reserve_batch(&[row(sequence, 15, 2), row(sequence, 16, 3)])
        .unwrap();
    assert_eq!(pool.stats().free_pages, 0);
    assert_eq!(pool.cancel_sequence(sequence), Err(Error::Busy));
    assert!(matches!(
        pool.open_sequence(scope(), &[1], 0),
        Err(Error::Busy)
    ));
    assert_eq!(pool.expire(1), Err(Error::Busy));
    pool.abort_batch(&first).unwrap();
    assert_eq!(pool.state, original);
    assert_eq!(pool.stats(), before);
    let second = pool.reserve_batch(&[row(sequence, 15, 2)]).unwrap();
    assert!(second.id() > first.id());
    assert_eq!(pool.begin_submission(&first), Err(Error::UnknownBatch));
    pool.begin_submission(&second).unwrap();
    assert_eq!(
        pool.commit_batch(
            &second,
            EngineeringTpBatchCompletionV1::after_all_ranks(&first)
        ),
        Err(Error::UnknownBatch)
    );
    pool.commit_batch(
        &second,
        EngineeringTpBatchCompletionV1::after_all_ranks(&second),
    )
    .unwrap();
    assert_eq!(
        pool.commit_batch(
            &second,
            EngineeringTpBatchCompletionV1::after_all_ranks(&second)
        ),
        Err(Error::UnknownBatch)
    );
}

#[test]
fn no_reuse_or_retirement_before_all_rank_completion() {
    let mut pool = pool(2);
    let sequence = pool.open_sequence(scope(), &[1], 0).unwrap().sequence();
    let batch = pool.reserve_batch(&[row(sequence, 0, 1)]).unwrap();
    assert_eq!(
        pool.commit_batch(
            &batch,
            EngineeringTpBatchCompletionV1::after_all_ranks(&batch)
        ),
        Err(Error::SubmissionPhase)
    );
    pool.begin_submission(&batch).unwrap();
    assert_eq!(pool.begin_submission(&batch), Err(Error::SubmissionPhase));
    assert_eq!(pool.abort_batch(&batch), Err(Error::SubmissionPhase));
    assert_eq!(pool.retire_sequence(sequence, true, 0), Err(Error::Busy));
    assert_eq!(pool.cancel_sequence(sequence), Err(Error::Busy));
    assert_eq!(pool.evict_unused(2), Err(Error::Busy));
    assert_eq!(pool.committed_position(sequence), Ok(0));
    pool.commit_batch(
        &batch,
        EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
    )
    .unwrap();
    pool.cancel_sequence(sequence).unwrap();
    assert_eq!(pool.stats().free_pages, 2);
}

#[test]
fn submitted_failure_quarantines_even_unused_slots_forever() {
    let mut pool = pool(4);
    let sequence = pool.open_sequence(scope(), &[1], 0).unwrap().sequence();
    let batch = pool.reserve_batch(&[row(sequence, 0, 1)]).unwrap();
    assert_eq!(pool.quarantine_batch(&batch), Err(Error::SubmissionPhase));
    pool.begin_submission(&batch).unwrap();
    pool.quarantine_batch(&batch).unwrap();
    let stats = pool.stats();
    assert_eq!(stats.quarantined_pages, 4);
    assert_eq!(stats.free_pages, 0);
    assert_eq!(stats.cached_pages, 0);
    assert_eq!(pool.check_invariants(), Err(Error::Poisoned));
    assert_eq!(pool.abort_batch(&batch), Err(Error::Poisoned));
    assert_eq!(
        pool.commit_batch(
            &batch,
            EngineeringTpBatchCompletionV1::after_all_ranks(&batch)
        ),
        Err(Error::Poisoned)
    );
    assert_eq!(pool.cancel_sequence(sequence), Err(Error::Poisoned));
    assert_eq!(pool.expire(200), Err(Error::Poisoned));
    assert!(matches!(
        pool.open_sequence(scope(), &[1], 200),
        Err(Error::Poisoned)
    ));
    assert!(!pool.is_empty());
}

#[test]
fn complete_pages_persist_after_retirement_and_share_with_exact_hits() {
    let mut pool = pool(6);
    let tokens: Vec<u32> = (0..33).collect();
    publish(&mut pool, &tokens, 0);
    assert_eq!(pool.stats().sequences, 0);
    assert_eq!(pool.stats().cached_pages, 2);
    assert_eq!(pool.stats().free_pages, 4);
    let a = pool.open_sequence(scope(), &tokens, 1).unwrap();
    let b = pool.open_sequence(scope(), &tokens, 2).unwrap();
    assert_eq!((a.hit_tokens(), a.hit_pages()), (32, 2));
    assert_eq!(a.physical_pages(), b.physical_pages());
    assert_eq!(pool.stats().prefix_hits, 2);
    assert_eq!(pool.stats().hit_tokens, 64);
    assert_eq!(pool.stats().hit_pages, 4);
    let prepared = pool
        .reserve_batch(&[row(a.sequence(), 32, 32), row(b.sequence(), 32, 32)])
        .unwrap();
    assert_ne!(
        prepared.rows()[0].writable_physical_page(),
        prepared.rows()[1].writable_physical_page()
    );
    assert!(
        !a.physical_pages()
            .contains(&prepared.rows()[0].writable_physical_page())
    );
    pool.abort_batch(&prepared).unwrap();
    pool.cancel_sequence(a.sequence()).unwrap();
    assert_eq!(pool.stats().retained_pages, 2);
    pool.cancel_sequence(b.sequence()).unwrap();
    assert_eq!(pool.stats().cached_pages, 2);
    pool.check_invariants().unwrap();
}

#[test]
fn final_prompt_token_is_never_satisfied_by_kv_alone() {
    let mut pool = pool(5);
    publish(&mut pool, &[7; 32], 0);
    for (length, expected) in [
        (1, 0),
        (15, 0),
        (16, 0),
        (17, 16),
        (31, 16),
        (32, 16),
        (33, 32),
    ] {
        let hit = pool.open_sequence(scope(), &vec![7; length], 1).unwrap();
        assert_eq!(hit.hit_tokens(), expected);
        assert!(hit.hit_tokens() < count(length));
        pool.cancel_sequence(hit.sequence()).unwrap();
    }
}

#[test]
fn partial_pages_are_never_cached_or_aliased() {
    let mut pool = pool(4);
    publish(&mut pool, &[3; 31], 0);
    assert_eq!(pool.stats().cached_pages, 1);
    let a = pool.open_sequence(scope(), &[3; 32], 1).unwrap();
    let b = pool.open_sequence(scope(), &[3; 32], 1).unwrap();
    assert_eq!(a.hit_tokens(), 16);
    let batch = pool
        .reserve_batch(&[row(a.sequence(), 16, 3), row(b.sequence(), 16, 3)])
        .unwrap();
    assert_ne!(
        batch.rows()[0].writable_physical_page(),
        batch.rows()[1].writable_physical_page()
    );
    pool.begin_submission(&batch).unwrap();
    pool.commit_batch(
        &batch,
        EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
    )
    .unwrap();
    pool.retire_sequence(a.sequence(), true, 2).unwrap();
    pool.retire_sequence(b.sequence(), true, 2).unwrap();
    assert_eq!(pool.stats().cached_pages, 1);
    assert_eq!(pool.stats().free_pages, 3);
}

#[test]
fn exact_prefix_tree_rejects_same_suffix_different_ancestor() {
    let mut pool = pool(8);
    let first: Vec<u32> = [vec![1; 16], vec![2; 16], vec![3]].concat();
    publish(&mut pool, &first, 0);
    let changed_root: Vec<u32> = [vec![9; 16], vec![2; 16], vec![3]].concat();
    let miss = pool.open_sequence(scope(), &changed_root, 1).unwrap();
    assert_eq!(miss.hit_tokens(), 0);
    pool.cancel_sequence(miss.sequence()).unwrap();
    let changed_second: Vec<u32> = [vec![1; 16], vec![8; 16], vec![3]].concat();
    let partial = pool.open_sequence(scope(), &changed_second, 1).unwrap();
    assert_eq!(partial.hit_tokens(), 16);
    append(&mut pool, partial.sequence(), &changed_second[16..]);
    pool.retire_sequence(partial.sequence(), true, 2).unwrap();
    assert_eq!(pool.stats().cached_pages, 3);
    let original = pool.open_sequence(scope(), &first, 2).unwrap();
    let branch = pool.open_sequence(scope(), &changed_second, 2).unwrap();
    assert_eq!(original.hit_tokens(), 32);
    assert_eq!(branch.hit_tokens(), 32);
    assert_eq!(original.physical_pages()[0], branch.physical_pages()[0]);
    assert_ne!(original.physical_pages()[1], branch.physical_pages()[1]);
    pool.cancel_sequence(original.sequence()).unwrap();
    pool.cancel_sequence(branch.sequence()).unwrap();
    let other_root: Vec<u32> = [vec![9; 16], vec![4; 16], vec![3]].concat();
    publish(&mut pool, &other_root, 2);
    let wrong_parent = pool.open_sequence(scope(), &changed_root, 2).unwrap();
    assert_eq!(wrong_parent.hit_tokens(), 16);
    pool.cancel_sequence(wrong_parent.sequence()).unwrap();
    pool.check_invariants().unwrap();
}

#[test]
fn scope_and_instance_bound_handles_reject_cross_request_false_hits() {
    let mut a = pool(4);
    let mut b = pool(4);
    let first_a = a.open_sequence(scope(), &[7], 0).unwrap().sequence();
    let first_b = b.open_sequence(scope(), &[7], 0).unwrap().sequence();
    assert_eq!(first_a.serial, first_b.serial);
    let b_before = b.state.clone();
    assert_eq!(b.cancel_sequence(first_a), Err(Error::UnknownSequence));
    assert_eq!(b.state, b_before);
    a.cancel_sequence(first_a).unwrap();
    b.cancel_sequence(first_b).unwrap();
    publish(&mut a, &[1; 17], 0);
    let mut wrong = scope();
    wrong.model[0] ^= 1;
    let original = a.state.clone();
    assert!(matches!(
        a.open_sequence(wrong, &[1; 17], 1),
        Err(Error::ScopeMismatch)
    ));
    wrong = scope();
    wrong.session[0] ^= 1;
    assert!(matches!(
        a.open_sequence(wrong, &[1; 17], 1),
        Err(Error::ScopeMismatch)
    ));
    assert_eq!(a.state, original);
    let request_a = a.open_sequence(scope(), &[1; 17], 1).unwrap();
    let request_b = b.open_sequence(scope(), &[1; 17], 1).unwrap();
    assert_eq!(request_b.hit_tokens(), 0);
    assert_eq!(
        b.cancel_sequence(request_a.sequence()),
        Err(Error::UnknownSequence)
    );
    let batch_a = a
        .reserve_batch(&[row(request_a.sequence(), 16, 1)])
        .unwrap();
    let batch_b = b.reserve_batch(&[row(request_b.sequence(), 0, 1)]).unwrap();
    assert_eq!(b.begin_submission(&batch_a), Err(Error::UnknownBatch));
    b.begin_submission(&batch_b).unwrap();
    assert_eq!(
        b.commit_batch(
            &batch_b,
            EngineeringTpBatchCompletionV1::after_all_ranks(&batch_a)
        ),
        Err(Error::UnknownBatch)
    );
}

#[test]
fn expiry_removes_cache_pins_but_preserves_live_sequence_refs() {
    let mut pool = pool(4);
    publish(&mut pool, &[5; 33], 0);
    let hit = pool.open_sequence(scope(), &[5; 33], 1).unwrap();
    assert_eq!(hit.hit_pages(), 2);
    pool.expire(100).unwrap();
    assert_eq!(pool.stats().cached_pages, 2);
    pool.expire(101).unwrap();
    assert_eq!(pool.stats().cached_pages, 0);
    assert_eq!(pool.stats().retained_pages, 2);
    assert_eq!(pool.committed_position(hit.sequence()), Ok(32));
    let miss = pool.open_sequence(scope(), &[5; 33], 101).unwrap();
    assert_eq!(miss.hit_tokens(), 0);
    pool.cancel_sequence(miss.sequence()).unwrap();
    pool.cancel_sequence(hit.sequence()).unwrap();
    assert_eq!(pool.stats().free_pages, 4);
    assert_eq!(pool.stats().evicted_pages, 2);
}

#[test]
fn eviction_is_lru_leaf_first_atomic_and_never_reclaims_a_live_page() {
    let mut pool = pool(3);
    publish(&mut pool, &[1; 17], 0);
    publish(&mut pool, &[2; 17], 1);
    let pinned = pool.open_sequence(scope(), &[1; 17], 2).unwrap();
    let original = pool.state.clone();
    assert_eq!(pool.evict_unused(3), Err(Error::OutOfPages));
    assert_eq!(pool.state, original);
    pool.evict_unused(2).unwrap();
    assert_eq!(pool.stats().cached_pages, 1);
    assert_eq!(pool.stats().free_pages, 2);
    let miss = pool.open_sequence(scope(), &[2; 17], 2).unwrap();
    assert_eq!(miss.hit_tokens(), 0);
    pool.cancel_sequence(miss.sequence()).unwrap();
    pool.cancel_sequence(pinned.sequence()).unwrap();
    pool.evict_unused(3).unwrap();
    assert_eq!(pool.stats().free_pages, 3);
}

#[test]
fn cancellation_stale_id_and_duplicate_publication_do_not_leak_refs() {
    let mut pool = pool(4);
    let a = pool.open_sequence(scope(), &[6; 32], 0).unwrap().sequence();
    let b = pool.open_sequence(scope(), &[6; 32], 0).unwrap().sequence();
    append(&mut pool, a, &[6; 32]);
    append(&mut pool, b, &[6; 32]);
    pool.retire_sequence(a, true, 1).unwrap();
    pool.retire_sequence(b, true, 1).unwrap();
    assert_eq!(pool.stats().cached_pages, 2);
    assert_eq!(pool.stats().free_pages, 2);
    assert_eq!(pool.cancel_sequence(a), Err(Error::UnknownSequence));
    let next = pool.open_sequence(scope(), &[1], 1).unwrap().sequence();
    assert_ne!(next, a);
    assert_eq!(pool.committed_position(a), Err(Error::UnknownSequence));
    append(&mut pool, next, &[9]);
    pool.cancel_sequence(next).unwrap();
    assert_eq!(pool.stats().cached_pages, 2);
    assert_eq!(pool.stats().free_pages, 2);
    pool.check_invariants().unwrap();
}

#[test]
fn monotone_clock_capacity_and_counter_exhaustion_fail_atomically() {
    let mut pool = EngineeringTpPagedPoolV1::new(
        scope(),
        EngineeringTpPagedLimitsV1::new(17, 1, 2, 10).unwrap(),
    )
    .unwrap();
    let a = pool.open_sequence(scope(), &[1], 5).unwrap().sequence();
    let original = pool.state.clone();
    assert!(matches!(
        pool.open_sequence(scope(), &[1], 5),
        Err(Error::SequenceCapacity)
    ));
    assert_eq!(pool.expire(4), Err(Error::ClockRegression));
    assert_eq!(
        pool.retire_sequence(a, true, u64::MAX),
        Err(Error::Exhausted)
    );
    assert_eq!(pool.state, original);
    pool.cancel_sequence(a).unwrap();
    pool.state.next_sequence = u64::MAX;
    let original = pool.state.clone();
    assert!(matches!(
        pool.open_sequence(scope(), &[1], 5),
        Err(Error::Exhausted)
    ));
    assert_eq!(pool.state, original);
}

#[test]
fn randomized_bounded_transactions_preserve_invariants_and_accounting() {
    let mut pool = pool(12);
    let mut live = Vec::new();
    let mut random = 0x1234_5678_u64;
    for now in 0..500 {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        if live.len() < 6 && random.is_multiple_of(3) {
            let hit = pool.open_sequence(scope(), &[7; 33], now).unwrap();
            live.push(hit.sequence());
        } else if let Some(sequence) = live.pop() {
            let position = pool.committed_position(sequence).unwrap();
            if position < 48 && random & 1 == 0 {
                match pool.reserve_batch(&[row(sequence, position, 7)]) {
                    Ok(batch) => {
                        if random & 4 == 0 {
                            pool.abort_batch(&batch).unwrap();
                        } else {
                            pool.begin_submission(&batch).unwrap();
                            pool.commit_batch(
                                &batch,
                                EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
                            )
                            .unwrap();
                        }
                    }
                    Err(Error::OutOfPages) => {
                        let _ = pool.evict_unused(1);
                    }
                    other => panic!("unexpected reserve result: {other:?}"),
                }
                live.push(sequence);
            } else {
                pool.retire_sequence(sequence, random & 2 == 0, now)
                    .unwrap();
            }
        }
        pool.check_invariants().unwrap();
        let stats = pool.stats();
        assert_eq!(
            stats.free_pages + stats.retained_pages + stats.quarantined_pages,
            12
        );
        assert_eq!(stats.sequences, count(live.len()));
    }
    for sequence in live {
        pool.cancel_sequence(sequence).unwrap();
    }
    pool.expire(1000).unwrap();
    assert_eq!(pool.stats().free_pages, 12);
}

#[test]
fn maximum_context_can_evict_only_the_held_back_complete_leaf() {
    let mut pool = EngineeringTpPagedPoolV1::new(
        scope(),
        EngineeringTpPagedLimitsV1::new(8192, 32, 512, 100).unwrap(),
    )
    .unwrap();
    publish(&mut pool, &vec![4; 8192], 0);
    assert_eq!(pool.stats().cached_pages, 512);
    let hit = pool.open_sequence(scope(), &vec![4; 8192], 1).unwrap();
    assert_eq!(hit.hit_tokens(), 8176);
    assert_eq!(hit.hit_pages(), 511);
    let rows: Vec<_> = (8176..8192)
        .map(|position| row(hit.sequence(), position, 4))
        .collect();
    assert!(matches!(pool.reserve_batch(&rows), Err(Error::OutOfPages)));
    pool.evict_unused(1).unwrap();
    assert_eq!(pool.stats().cached_pages, 511);
    let batch = pool.reserve_batch(&rows).unwrap();
    assert_eq!(batch.rows()[0].physical_pages().len(), 512);
    pool.begin_submission(&batch).unwrap();
    pool.commit_batch(
        &batch,
        EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
    )
    .unwrap();
    assert_eq!(pool.committed_position(hit.sequence()), Ok(8192));
    assert!(matches!(
        pool.reserve_batch(&[row(hit.sequence(), 8192, 4)]),
        Err(Error::InvalidRows)
    ));
    pool.retire_sequence(hit.sequence(), true, 2).unwrap();
    pool.expire(102).unwrap();
    assert_eq!(pool.stats().free_pages, 512);
}

#[test]
fn hit_eviction_and_batch_counter_exhaustion_preserve_state() {
    let mut pool = pool(4);
    publish(&mut pool, &[1; 17], 0);
    pool.state.hit_tokens = u64::MAX;
    let original = pool.state.clone();
    assert!(matches!(
        pool.open_sequence(scope(), &[1; 17], 1),
        Err(Error::Exhausted)
    ));
    assert_eq!(pool.state, original);
    pool.state.hit_tokens = 0;
    pool.state.evicted_pages = u64::MAX;
    let original = pool.state.clone();
    assert_eq!(pool.expire(100), Err(Error::Exhausted));
    assert_eq!(pool.state, original);
    let request = pool.open_sequence(scope(), &[2], 1).unwrap().sequence();
    pool.next_batch = u64::MAX;
    let original = pool.state.clone();
    assert!(matches!(
        pool.reserve_batch(&[row(request, 0, 2)]),
        Err(Error::Exhausted)
    ));
    assert_eq!(pool.state, original);
    assert!(pool.pending.is_none());
}
