use ferric_qwen3_logits_device_v1::{
    QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1, QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1,
    QWEN3_LOGITS_VOCABULARY_V1,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompactRead {
    Choice(usize),
    Draft(usize),
}

type CompactRecord = [u8; QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1];

fn bf16_to_f32(bits: u16) -> f32 {
    f32::from_bits(u32::from(bits) << 16)
}

fn lowest_id_argmax(logits: &[u16]) -> Option<u32> {
    let first = bf16_to_f32(*logits.first()?);
    if !first.is_finite() {
        return None;
    }
    let mut winner = 0;
    let mut maximum = first;
    for (token, bits) in logits.iter().copied().enumerate().skip(1) {
        let candidate = bf16_to_f32(bits);
        if !candidate.is_finite() {
            return None;
        }
        if candidate > maximum {
            maximum = candidate;
            winner = token as u32;
        }
    }
    Some(winner)
}

fn checked_draft_index(
    candidate: usize,
    speculative_k: usize,
    sequences: usize,
    sequence: usize,
) -> Option<usize> {
    if candidate >= speculative_k || sequence >= sequences {
        return None;
    }
    candidate.checked_mul(sequences)?.checked_add(sequence)
}

fn checked_target_index(
    sequence: usize,
    sequences: usize,
    active_tokens: usize,
    candidate: usize,
) -> Option<usize> {
    if sequence >= sequences || candidate >= active_tokens {
        return None;
    }
    sequence.checked_mul(active_tokens)?.checked_add(candidate)
}

fn checked_generation_byte(generation: u32, byte: usize) -> Option<u8> {
    if !(4..8).contains(&byte) {
        return None;
    }
    let generation_byte = byte.checked_sub(4)?;
    if generation_byte >= 4 {
        return None;
    }
    let shift = generation_byte.checked_mul(8)?;
    if shift >= 32 {
        return None;
    }
    Some((generation >> shift) as u8)
}

#[allow(clippy::too_many_arguments)]
fn compact_record(
    choices: &[u32],
    draft: &[u32],
    live: usize,
    slot: u32,
    generation: u32,
    epoch: u64,
    plan: [u8; 32],
    sequences: usize,
    sequence: usize,
    active_tokens: usize,
    speculative_k: usize,
) -> Option<CompactRecord> {
    compact_record_with_trace(
        choices,
        draft,
        live,
        slot,
        generation,
        epoch,
        plan,
        sequences,
        sequence,
        active_tokens,
        speculative_k,
    )
    .0
}

#[allow(clippy::too_many_arguments)]
fn compact_record_with_trace(
    choices: &[u32],
    draft: &[u32],
    live: usize,
    slot: u32,
    generation: u32,
    epoch: u64,
    plan: [u8; 32],
    sequences: usize,
    sequence: usize,
    active_tokens: usize,
    speculative_k: usize,
) -> (Option<CompactRecord>, Vec<CompactRead>) {
    let mut reads = Vec::new();
    let record = (|| {
        if live == 0 {
            return Some([0; QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1]);
        }
        let direct_offset = live - 1;
        if live > active_tokens
            || (speculative_k != 0 && live != active_tokens)
            || slot >= 32
            || generation == 0
            || plan.iter().all(|byte| *byte == 0)
        {
            return None;
        }
        let choice_base = sequence.checked_mul(active_tokens)?;
        let mut accepted = 0;
        if speculative_k != 0 {
            while accepted < speculative_k {
                let draft_index =
                    checked_draft_index(accepted, speculative_k, sequences, sequence)?;
                reads.push(CompactRead::Draft(draft_index));
                let draft_token = *draft.get(draft_index)?;
                let target_index =
                    checked_target_index(sequence, sequences, active_tokens, accepted)?;
                reads.push(CompactRead::Choice(target_index));
                let target_token = *choices.get(target_index)?;
                if draft_token as usize >= QWEN3_LOGITS_VOCABULARY_V1
                    || target_token as usize >= QWEN3_LOGITS_VOCABULARY_V1
                {
                    return None;
                }
                if draft_token != target_token {
                    break;
                }
                accepted += 1;
            }
        }
        if accepted >= active_tokens {
            return None;
        }
        let correction_index = if speculative_k == 0 {
            choice_base + direct_offset
        } else {
            choice_base + accepted
        };
        reads.push(CompactRead::Choice(correction_index));
        let correction = *choices.get(correction_index)?;
        if correction as usize >= QWEN3_LOGITS_VOCABULARY_V1 {
            return None;
        }

        let mut record = [0_u8; QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1];
        record[0..4].copy_from_slice(&slot.to_le_bytes());
        record[4..8].copy_from_slice(&generation.to_le_bytes());
        record[8..16].copy_from_slice(&epoch.to_le_bytes());
        record[16..48].copy_from_slice(&plan);
        record[48] = accepted as u8;
        record[49] = (accepted + 1) as u8;
        for token in 0..accepted {
            let offset = 52 + token * 4;
            if offset < 52 || token >= speculative_k {
                return None;
            }
            record[offset..offset + 4]
                .copy_from_slice(&draft[token * sequences + sequence].to_le_bytes());
        }
        let correction_offset = 52 + accepted * 4;
        record[correction_offset..correction_offset + 4].copy_from_slice(&correction.to_le_bytes());
        Some(record)
    })();
    (record, reads)
}

#[test]
fn draft_index_requires_a_current_candidate_bound_for_every_admitted_profile() {
    for (sequences, speculative_k) in [(1_usize, 4_usize), (8, 4), (1, 8), (1, 16)] {
        for sequence in 0..sequences {
            assert_eq!(
                checked_draft_index(0, speculative_k, sequences, sequence),
                Some(sequence)
            );
            assert_eq!(
                checked_draft_index(speculative_k - 1, speculative_k, sequences, sequence),
                Some(speculative_k * sequences - sequences + sequence)
            );
            assert_eq!(
                checked_draft_index(speculative_k, speculative_k, sequences, sequence),
                None
            );
            assert_eq!(
                checked_draft_index(speculative_k + 1, speculative_k, sequences, sequence),
                None
            );
        }
        assert_eq!(
            checked_draft_index(0, speculative_k, sequences, sequences),
            None
        );
    }
}

#[test]
fn target_index_requires_the_candidate_to_remain_inside_every_admitted_row() {
    for (sequences, active_tokens, speculative_k) in [
        (1_usize, 5_usize, 4_usize),
        (8, 5, 4),
        (1, 9, 8),
        (1, 17, 16),
    ] {
        assert!(speculative_k < active_tokens);
        for sequence in 0..sequences {
            assert_eq!(
                checked_target_index(sequence, sequences, active_tokens, 0),
                Some(sequence * active_tokens)
            );
            assert_eq!(
                checked_target_index(sequence, sequences, active_tokens, speculative_k - 1),
                Some(sequence * active_tokens + speculative_k - 1)
            );
            assert_eq!(
                checked_target_index(sequence, sequences, active_tokens, speculative_k),
                Some(sequence * active_tokens + speculative_k)
            );
            assert_eq!(
                checked_target_index(sequence, sequences, active_tokens, active_tokens),
                None
            );
            assert_eq!(
                checked_target_index(sequence, sequences, active_tokens, active_tokens + 1),
                None
            );
        }
        assert_eq!(
            checked_target_index(sequences, sequences, active_tokens, 0),
            None
        );
    }
}

#[test]
fn generation_byte_shift_requires_exact_field_bounds() {
    let generation = 0x8877_6655_u32;
    assert_eq!(checked_generation_byte(generation, 4), Some(0x55));
    assert_eq!(checked_generation_byte(generation, 5), Some(0x66));
    assert_eq!(checked_generation_byte(generation, 6), Some(0x77));
    assert_eq!(checked_generation_byte(generation, 7), Some(0x88));
    assert_eq!(checked_generation_byte(generation, 3), None);
    assert_eq!(checked_generation_byte(generation, 8), None);
    assert_eq!(checked_generation_byte(generation, usize::MAX), None);
}

fn record_token(record: &[u8; QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1], token: usize) -> u32 {
    let offset = 52 + token * 4;
    u32::from_le_bytes(record[offset..offset + 4].try_into().unwrap())
}

#[test]
fn argmax_is_finite_strict_and_lowest_id_on_ties() {
    assert_eq!(lowest_id_argmax(&[0xbf80, 0x4000, 0x4000, 0x3f80]), Some(1));
    assert_eq!(lowest_id_argmax(&[0x0000, 0x8000]), Some(0));
    assert_eq!(lowest_id_argmax(&[0x8000, 0x0000]), Some(0));
    assert_eq!(lowest_id_argmax(&[0xff7f, 0x7f7f]), Some(1));
    assert_eq!(lowest_id_argmax(&[]), None);
    assert_eq!(lowest_id_argmax(&[0x7f80]), None);
    assert_eq!(lowest_id_argmax(&[0x3f80, 0x7fc1]), None);
}

#[test]
fn direct_record_uses_final_live_row_and_never_needs_draft_data() {
    let mut plan = [0_u8; 32];
    plan[0] = 0x44;
    plan[31] = 0x88;
    let mut choices = vec![0_u32; 128];
    choices[0] = 17;
    choices[1] = 23;
    choices[2] = 29;
    let record = compact_record(
        &choices,
        &[],
        3,
        7,
        9,
        0x1122_3344_5566_7788,
        plan,
        1,
        0,
        128,
        0,
    )
    .unwrap();
    assert_eq!(&record[0..4], &7_u32.to_le_bytes());
    assert_eq!(&record[4..8], &9_u32.to_le_bytes());
    assert_eq!(&record[8..16], &0x1122_3344_5566_7788_u64.to_le_bytes());
    assert_eq!(&record[16..48], &plan);
    assert_eq!(record[48], 0);
    assert_eq!(record[49], 1);
    assert_eq!(&record[50..52], &[0, 0]);
    assert_eq!(record_token(&record, 0), 29);
    assert!(record[56..].iter().all(|byte| *byte == 0));
}

#[test]
fn speculative_record_emits_accepted_prefix_then_correction_or_bonus() {
    let plan = [0x5a_u8; 32];
    let mismatch = compact_record(
        &[10, 20, 30, 40, 50],
        &[10, 20, 77, 88],
        5,
        1,
        2,
        3,
        plan,
        1,
        0,
        5,
        4,
    )
    .unwrap();
    assert_eq!(&mismatch[48..52], &[2, 3, 0, 0]);
    assert_eq!(record_token(&mismatch, 0), 10);
    assert_eq!(record_token(&mismatch, 1), 20);
    assert_eq!(record_token(&mismatch, 2), 30);
    assert!(mismatch[64..].iter().all(|byte| *byte == 0));

    let bonus = compact_record(
        &[10, 20, 30, 40, 99],
        &[10, 20, 30, 40],
        5,
        1,
        2,
        3,
        plan,
        1,
        0,
        5,
        4,
    )
    .unwrap();
    assert_eq!(&bonus[48..52], &[4, 5, 0, 0]);
    assert_eq!(record_token(&bonus, 0), 10);
    assert_eq!(record_token(&bonus, 1), 20);
    assert_eq!(record_token(&bonus, 2), 30);
    assert_eq!(record_token(&bonus, 3), 40);
    assert_eq!(record_token(&bonus, 4), 99);
    assert!(bonus[72..].iter().all(|byte| *byte == 0));
    assert_eq!(QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1, 17);
}

#[test]
fn speculative_prefix_boundaries_are_byte_exact() {
    let plan = [0x5a_u8; 32];
    let choices = [10, 20, 30, 40, 99];
    for (draft, accepted) in [
        ([77, 88, 89, 90], 0_usize),
        ([10, 20, 77, 90], 2),
        ([10, 20, 30, 77], 3),
        ([10, 20, 30, 40], 4),
    ] {
        let record = compact_record(&choices, &draft, 5, 1, 2, 3, plan, 1, 0, 5, 4)
            .expect("valid prefix must produce a record");
        assert_eq!(&record[0..4], &1_u32.to_le_bytes());
        assert_eq!(&record[4..8], &2_u32.to_le_bytes());
        assert_eq!(&record[8..16], &3_u64.to_le_bytes());
        assert_eq!(&record[16..48], &plan);
        assert_eq!(&record[48..52], &[accepted as u8, accepted as u8 + 1, 0, 0]);
        for token in 0..accepted {
            assert_eq!(record_token(&record, token), draft[token]);
        }
        assert_eq!(record_token(&record, accepted), choices[accepted]);
        assert!(
            record[52 + (accepted + 1) * 4..]
                .iter()
                .all(|byte| *byte == 0)
        );
    }
}

#[test]
fn mismatch_stops_later_validation_and_volatile_reads() {
    let invalid = QWEN3_LOGITS_VOCABULARY_V1 as u32;
    let (record, reads) = compact_record_with_trace(
        &[10, 20, invalid, 40, 99],
        &[10, 77, invalid, invalid],
        5,
        1,
        2,
        3,
        [0x5a; 32],
        1,
        0,
        5,
        4,
    );
    let record = record.expect("invalid tokens after the mismatch must remain unobserved");
    assert_eq!(&record[48..52], &[1, 2, 0, 0]);
    assert_eq!(record_token(&record, 0), 10);
    assert_eq!(record_token(&record, 1), 20);
    assert!(record[60..].iter().all(|byte| *byte == 0));
    assert_eq!(
        reads,
        vec![
            CompactRead::Draft(0),
            CompactRead::Choice(0),
            CompactRead::Draft(1),
            CompactRead::Choice(1),
            CompactRead::Choice(1),
        ]
    );
}

#[test]
fn invalid_tokens_before_or_at_mismatch_trap_after_pair_loads() {
    let invalid = QWEN3_LOGITS_VOCABULARY_V1 as u32;
    for (choices, draft) in [
        ([10, invalid, 30, 40, 99], [10, 20, 77, 88]),
        ([10, 20, 30, 40, 99], [10, invalid, 77, 88]),
    ] {
        let (record, reads) =
            compact_record_with_trace(&choices, &draft, 5, 1, 2, 3, [0x5a; 32], 1, 0, 5, 4);
        assert_eq!(record, None);
        assert_eq!(
            reads,
            vec![
                CompactRead::Draft(0),
                CompactRead::Choice(0),
                CompactRead::Draft(1),
                CompactRead::Choice(1),
            ]
        );
    }
}

#[test]
fn direct_and_all_match_read_sequences_are_exact() {
    let mut direct_choices = vec![0_u32; 128];
    direct_choices[0] = 17;
    direct_choices[1] = 23;
    direct_choices[2] = 29;
    let (direct, direct_reads) =
        compact_record_with_trace(&direct_choices, &[], 3, 1, 2, 3, [0x5a; 32], 1, 0, 128, 0);
    assert!(direct.is_some());
    assert_eq!(direct_reads, vec![CompactRead::Choice(2)]);

    let (bonus, bonus_reads) = compact_record_with_trace(
        &[10, 20, 30, 40, 99],
        &[10, 20, 30, 40],
        5,
        1,
        2,
        3,
        [0x5a; 32],
        1,
        0,
        5,
        4,
    );
    assert!(bonus.is_some());
    assert_eq!(
        bonus_reads,
        vec![
            CompactRead::Draft(0),
            CompactRead::Choice(0),
            CompactRead::Draft(1),
            CompactRead::Choice(1),
            CompactRead::Draft(2),
            CompactRead::Choice(2),
            CompactRead::Draft(3),
            CompactRead::Choice(3),
            CompactRead::Choice(4),
        ]
    );
}

#[test]
fn accepted_bound_is_unreachable_for_every_admitted_compact_profile() {
    let profiles = [
        (1_usize, 1_usize, 0_usize),
        (1, 128, 0),
        (1, 512, 0),
        (1, 2_048, 0),
        (8, 1, 0),
        (8, 128, 0),
        (32, 1, 0),
        (1, 5, 4),
        (8, 5, 4),
        (1, 9, 8),
        (1, 17, 16),
    ];

    for (sequences, active_tokens, speculative_k) in profiles {
        let choices = (0..sequences * active_tokens)
            .map(|index| (100 + index) as u32)
            .collect::<Vec<_>>();
        if speculative_k == 0 {
            for sequence in 0..sequences {
                let (record, reads) = compact_record_with_trace(
                    &choices,
                    &[],
                    active_tokens,
                    1,
                    2,
                    3,
                    [0x5a; 32],
                    sequences,
                    sequence,
                    active_tokens,
                    speculative_k,
                );
                let record = record.expect("every admitted direct profile stays below the bound");
                assert_eq!(record[48], 0);
                assert_eq!(record[49], 1);
                assert!(0 < QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1);
                assert!(1 <= QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1);
                let correction_index = sequence * active_tokens + active_tokens - 1;
                assert_eq!(record_token(&record, 0), choices[correction_index]);
                assert_eq!(reads, vec![CompactRead::Choice(correction_index)]);
            }
            continue;
        }

        let mut all_match_draft = Vec::with_capacity(sequences * speculative_k);
        for candidate in 0..speculative_k {
            for sequence in 0..sequences {
                all_match_draft.push(choices[sequence * active_tokens + candidate]);
            }
        }
        let mismatch_at = speculative_k / 2;
        let mut mismatch_draft = all_match_draft.clone();
        for sequence in 0..sequences {
            mismatch_draft[mismatch_at * sequences + sequence] =
                choices[sequence * active_tokens + mismatch_at] + 1;
        }

        for sequence in 0..sequences {
            for (draft, expected_accepted) in [
                (all_match_draft.as_slice(), speculative_k),
                (mismatch_draft.as_slice(), mismatch_at),
            ] {
                let (record, reads) = compact_record_with_trace(
                    &choices,
                    draft,
                    active_tokens,
                    1,
                    2,
                    3,
                    [0x5a; 32],
                    sequences,
                    sequence,
                    active_tokens,
                    speculative_k,
                );
                let record = record
                    .expect("every admitted speculative outcome stays below the active width");
                assert!(expected_accepted < active_tokens);
                assert!(expected_accepted < QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1);
                assert!(expected_accepted + 1 <= QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1);
                assert_eq!(record[48] as usize, expected_accepted);
                assert_eq!(record[49] as usize, expected_accepted + 1);
                for candidate in 0..expected_accepted {
                    assert!(candidate < speculative_k);
                    assert_eq!(
                        record_token(&record, candidate),
                        draft[candidate * sequences + sequence]
                    );
                }
                let correction_index = sequence * active_tokens + expected_accepted;
                assert_eq!(
                    record_token(&record, expected_accepted),
                    choices[correction_index]
                );

                let mut expected_reads = Vec::new();
                let compared = if expected_accepted == speculative_k {
                    speculative_k
                } else {
                    expected_accepted + 1
                };
                for candidate in 0..compared {
                    expected_reads.push(CompactRead::Draft(candidate * sequences + sequence));
                    expected_reads.push(CompactRead::Choice(sequence * active_tokens + candidate));
                }
                expected_reads.push(CompactRead::Choice(correction_index));
                assert_eq!(reads, expected_reads);
            }
        }
    }
}

#[test]
fn inactive_record_is_all_zero_without_request_authority() {
    let record = compact_record(&[], &[], 0, u32::MAX, 0, 0, [0; 32], 1, 0, 1, 0).unwrap();
    assert_eq!(record, [0; QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1]);
}

#[test]
fn speculative_token_assembly_transposes_iteration_major_drafts() {
    let anchors = [100_u32, 101, 102, 103, 104, 105, 106, 107];
    let draft = [
        11, 12, 13, 14, 15, 16, 17, 18, 21, 22, 23, 24, 25, 26, 27, 28, 31, 32, 33, 34, 35, 36, 37,
        38, 41, 42, 43, 44, 45, 46, 47, 48,
    ];
    let sequences = 8;
    let speculative_k = 4;
    let mut target = vec![0; sequences * (speculative_k + 1)];
    for sequence in 0..sequences {
        let base = sequence * (speculative_k + 1);
        target[base] = anchors[sequence];
        for iteration in 0..speculative_k {
            target[base + iteration + 1] = draft[iteration * sequences + sequence];
        }
    }
    assert_eq!(
        target,
        [
            100, 11, 21, 31, 41, 101, 12, 22, 32, 42, 102, 13, 23, 33, 43, 103, 14, 24, 34, 44,
            104, 15, 25, 35, 45, 105, 16, 26, 36, 46, 106, 17, 27, 37, 47, 107, 18, 28, 38, 48,
        ]
    );
}
