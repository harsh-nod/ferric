use ferric_qwen3_logits_device_v1::{
    QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1, QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1,
    QWEN3_LOGITS_VOCABULARY_V1,
};

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
) -> Option<[u8; QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1]> {
    if live == 0 {
        return Some([0; QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1]);
    }
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
            let draft_token = *draft.get(accepted.checked_mul(sequences)? + sequence)?;
            let target_token = *choices.get(choice_base + accepted)?;
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
    let correction = if speculative_k == 0 {
        *choices.get(choice_base + live - 1)?
    } else {
        *choices.get(choice_base + accepted)?
    };
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
        record[offset..offset + 4]
            .copy_from_slice(&draft[token * sequences + sequence].to_le_bytes());
    }
    let correction_offset = 52 + accepted * 4;
    record[correction_offset..correction_offset + 4].copy_from_slice(&correction.to_le_bytes());
    Some(record)
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
