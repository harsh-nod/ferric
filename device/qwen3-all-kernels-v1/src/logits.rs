#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel macro emits undocumented helper modules.

//! Attributed Rust source for Ferric's exact Qwen3 K7 device roots.
//!
//! This source carries no artifact, launch, completion, numerical-
//! qualification, or M1 authority. Production integration remains fail-closed
//! until the final compiler pin extracts and verifies replacement artifacts.

use fe2o3_device::{Bf16, Index1D, RowStriped2D, WriteOnlyDisjointSlice, kernel, memory, thread};

pub const QWEN3_LOGITS_ARGMAX_KERNEL_SYMBOL_V1: &str = "ferric_qwen3_lowest_id_argmax_bf16_v1";
pub const QWEN3_LOGITS_COMPACT_KERNEL_SYMBOL_V1: &str = "ferric_qwen3_compact_completion_v1";
pub const QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_KERNEL_SYMBOL_V1: &str =
    "ferric_qwen3_speculative_token_assembly_v1";
pub const QWEN3_LOGITS_WORKGROUP_V1: [u32; 3] = [64, 1, 1];
pub const QWEN3_LOGITS_ARGMAX_MAX_GRID_WORKGROUPS_V1: u32 = 2_048;
pub const QWEN3_LOGITS_COMPACT_MAX_GRID_WORKGROUPS_V1: u32 = 32;
pub const QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_MAX_GRID_WORKGROUPS_V1: u32 = 8;
pub const QWEN3_LOGITS_VOCABULARY_V1: usize = 151_936;
pub const QWEN3_LOGITS_MAX_SPECULATIVE_K_V1: usize = 16;
pub const QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1: usize = 120;
pub const QWEN3_LOGITS_COMPACT_RECORD_WORDS_V1: usize = 30;
pub const QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1: usize = 17;
pub const QWEN3_LOGITS_ARGMAX_EXPLICIT_KERNARG_BYTES_V1: usize = 40;
pub const QWEN3_LOGITS_COMPACT_EXPLICIT_KERNARG_BYTES_V1: usize = 144;
pub const QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_EXPLICIT_KERNARG_BYTES_V1: usize = 56;

pub const QWEN3_LOGITS_ARGMAX_ROWS_V1: [usize; 13] =
    [1, 4, 5, 8, 9, 16, 17, 32, 40, 128, 512, 1_024, 2_048];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3LogitsCompactProfileV1 {
    pub sequences: usize,
    pub active_tokens: usize,
    pub speculative_k: usize,
}

pub const QWEN3_LOGITS_COMPACT_PROFILES_V1: [Qwen3LogitsCompactProfileV1; 11] = [
    Qwen3LogitsCompactProfileV1 {
        sequences: 1,
        active_tokens: 128,
        speculative_k: 0,
    },
    Qwen3LogitsCompactProfileV1 {
        sequences: 8,
        active_tokens: 128,
        speculative_k: 0,
    },
    Qwen3LogitsCompactProfileV1 {
        sequences: 1,
        active_tokens: 512,
        speculative_k: 0,
    },
    Qwen3LogitsCompactProfileV1 {
        sequences: 1,
        active_tokens: 2_048,
        speculative_k: 0,
    },
    Qwen3LogitsCompactProfileV1 {
        sequences: 1,
        active_tokens: 1,
        speculative_k: 0,
    },
    Qwen3LogitsCompactProfileV1 {
        sequences: 8,
        active_tokens: 1,
        speculative_k: 0,
    },
    Qwen3LogitsCompactProfileV1 {
        sequences: 32,
        active_tokens: 1,
        speculative_k: 0,
    },
    Qwen3LogitsCompactProfileV1 {
        sequences: 1,
        active_tokens: 5,
        speculative_k: 4,
    },
    Qwen3LogitsCompactProfileV1 {
        sequences: 8,
        active_tokens: 5,
        speculative_k: 4,
    },
    Qwen3LogitsCompactProfileV1 {
        sequences: 1,
        active_tokens: 9,
        speculative_k: 8,
    },
    Qwen3LogitsCompactProfileV1 {
        sequences: 1,
        active_tokens: 17,
        speculative_k: 16,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3SpeculativeTokenAssemblyProfileV1 {
    pub sequences: usize,
    pub speculative_k: usize,
}

pub const QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_PROFILES_V1: [Qwen3SpeculativeTokenAssemblyProfileV1;
    4] = [
    Qwen3SpeculativeTokenAssemblyProfileV1 {
        sequences: 1,
        speculative_k: 4,
    },
    Qwen3SpeculativeTokenAssemblyProfileV1 {
        sequences: 8,
        speculative_k: 4,
    },
    Qwen3SpeculativeTokenAssemblyProfileV1 {
        sequences: 1,
        speculative_k: 8,
    },
    Qwen3SpeculativeTokenAssemblyProfileV1 {
        sequences: 1,
        speculative_k: 16,
    },
];

macro_rules! qwen3_logits_rows_are_admitted_expr_v1 {
    ($rows:expr) => {
        $rows == 1
            || $rows == 4
            || $rows == 5
            || $rows == 8
            || $rows == 9
            || $rows == 16
            || $rows == 17
            || $rows == 32
            || $rows == 40
            || $rows == 128
            || $rows == 512
            || $rows == 1_024
            || $rows == 2_048
    };
}

macro_rules! qwen3_logits_compact_profile_is_admitted_expr_v1 {
    ($sequences:expr, $active_tokens:expr, $speculative_k:expr) => {
        ($speculative_k == 0
            && (($sequences == 1
                && ($active_tokens == 1
                    || $active_tokens == 128
                    || $active_tokens == 512
                    || $active_tokens == 2_048))
                || ($sequences == 8 && ($active_tokens == 1 || $active_tokens == 128))
                || ($sequences == 32 && $active_tokens == 1)))
            || ($speculative_k == 4 && $active_tokens == 5 && ($sequences == 1 || $sequences == 8))
            || ($speculative_k == 8 && $active_tokens == 9 && $sequences == 1)
            || ($speculative_k == 16 && $active_tokens == 17 && $sequences == 1)
    };
}

macro_rules! qwen3_speculative_token_assembly_profile_is_admitted_expr_v1 {
    ($sequences:expr, $speculative_k:expr) => {
        ($speculative_k == 4 && ($sequences == 1 || $sequences == 8))
            || ($sequences == 1 && ($speculative_k == 8 || $speculative_k == 16))
    };
}

#[must_use]
pub const fn qwen3_logits_rows_are_admitted_v1(rows: usize) -> bool {
    qwen3_logits_rows_are_admitted_expr_v1!(rows)
}

#[must_use]
pub const fn qwen3_logits_compact_profile_is_admitted_v1(
    sequences: usize,
    active_tokens: usize,
    speculative_k: usize,
) -> bool {
    qwen3_logits_compact_profile_is_admitted_expr_v1!(sequences, active_tokens, speculative_k)
}

#[must_use]
pub const fn qwen3_speculative_token_assembly_profile_is_admitted_v1(
    sequences: usize,
    speculative_k: usize,
) -> bool {
    qwen3_speculative_token_assembly_profile_is_admitted_expr_v1!(sequences, speculative_k)
}

/// Selects the lowest token ID attaining the maximum finite BF16 logit.
///
/// One Wave64 workgroup owns each flattened row. Lane zero performs the exact
/// ascending scan and owns the sole row output; the remaining lanes are inert.
#[kernel(
    typed,
    launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [2048, 1, 1]),
    control_flow(loop_bounds(151936))
)]
pub fn ferric_qwen3_lowest_id_argmax_bf16_v1(
    logits: &[u16],
    mut choices: WriteOnlyDisjointSlice<u32, RowStriped2D<Index1D, 64, 1>>,
    rows: u32,
    vocabulary: u32,
) {
    let rows = rows as usize;
    let vocabulary = vocabulary as usize;
    if !(rows == 1
        || rows == 4
        || rows == 5
        || rows == 8
        || rows == 9
        || rows == 16
        || rows == 17
        || rows == 32
        || rows == 40
        || rows == 128
        || rows == 512
        || rows == 1_024
        || rows == 2_048)
        || vocabulary != QWEN3_LOGITS_VOCABULARY_V1
        || logits.len() != rows * vocabulary
        || choices.len() != rows
        || thread::block_dim_x() as usize != 64
        || thread::grid_dim_x() as usize != rows
    {
        fe2o3_device::trap();
    }

    let invocation = thread::index_1d();
    let raw = invocation.get();
    let lane = raw % 64;
    let row = raw / 64;
    if row >= rows {
        fe2o3_device::trap();
    }
    if lane != 0 {
        return;
    }
    let Some(choice_row) = invocation.checked_row_striped_2d::<64, 1>() else {
        fe2o3_device::trap();
    };

    let row_base = row * vocabulary;
    let first = Bf16::from_bits(memory::volatile_load(logits, row_base));
    if !first.is_finite() {
        fe2o3_device::trap();
    }
    let mut winner_token = 0_u32;
    let mut winner_value = first.to_f32();
    let mut token = 1;
    while token < vocabulary {
        let candidate = Bf16::from_bits(memory::volatile_load(logits, row_base + token));
        if !candidate.is_finite() {
            fe2o3_device::trap();
        }
        let candidate_value = candidate.to_f32();
        if candidate_value > winner_value {
            winner_token = token as u32;
            winner_value = candidate_value;
        }
        token += 1;
    }

    if !choices.write_row_striped_2d(&choice_row, 0, rows, 1, 1, winner_token) {
        fe2o3_device::trap();
    }
}

/// Publishes one canonical 120-byte completion record per target sequence.
///
/// `records` is the canonical byte record. Each lane owns byte `lane` and,
/// when present, byte `64 + lane`. Direct profiles retain the required non-null
/// empty `draft` slice and never read it; speculative profiles consume
/// iteration-major `[K,S]` choices.
#[kernel(
    typed,
    launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [32, 1, 1]),
    control_flow(loop_bounds(32, 16, 2))
)]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_compact_completion_v1(
    choices: &[u32],
    draft: &[u32],
    active_lengths: &[u32],
    request_slots: &[u32],
    request_generations: &[u32],
    completion_epochs: &[u64],
    plan_identities: &[u8],
    mut records: WriteOnlyDisjointSlice<u8, RowStriped2D<Index1D, 64, 2>>,
    sequences: u32,
    active_tokens: u32,
    speculative_k: u32,
) {
    let sequences = sequences as usize;
    let active_tokens = active_tokens as usize;
    let speculative_k = speculative_k as usize;
    if !((speculative_k == 0
        && ((sequences == 1
            && (active_tokens == 1
                || active_tokens == 128
                || active_tokens == 512
                || active_tokens == 2_048))
            || (sequences == 8 && (active_tokens == 1 || active_tokens == 128))
            || (sequences == 32 && active_tokens == 1)))
        || (speculative_k == 4 && active_tokens == 5 && (sequences == 1 || sequences == 8))
        || (speculative_k == 8 && active_tokens == 9 && sequences == 1)
        || (speculative_k == 16 && active_tokens == 17 && sequences == 1))
        || choices.len() != sequences * active_tokens
        || draft.len() != sequences * speculative_k
        || active_lengths.len() != sequences
        || request_slots.len() != sequences
        || request_generations.len() != sequences
        || completion_epochs.len() != sequences
        || plan_identities.len() != sequences * 32
        || records.len() != sequences * QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1
        || thread::block_dim_x() as usize != 64
        || thread::grid_dim_x() as usize != sequences
    {
        fe2o3_device::trap();
    }

    let invocation = thread::index_1d();
    let raw = invocation.get();
    let lane = raw % 64;
    let sequence = raw / 64;
    if sequence >= sequences {
        fe2o3_device::trap();
    }
    let Some(record_row) = invocation.checked_row_striped_2d::<64, 2>() else {
        fe2o3_device::trap();
    };

    let live = memory::volatile_load(active_lengths, sequence) as usize;
    let direct = speculative_k == 0;
    if live > active_tokens || (!direct && live != 0 && live != active_tokens) {
        fe2o3_device::trap();
    }
    if live == 0 {
        if !records.write_row_striped_2d(
            &record_row,
            0,
            sequences,
            QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1,
            QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1,
            0,
        ) {
            fe2o3_device::trap();
        }
        if lane < QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1 - 64
            && !records.write_row_striped_2d(
                &record_row,
                1,
                sequences,
                QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1,
                QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1,
                0,
            )
        {
            fe2o3_device::trap();
        }
        return;
    }

    let slot = memory::volatile_load(request_slots, sequence);
    let generation = memory::volatile_load(request_generations, sequence);
    if slot >= 32 || generation == 0 {
        fe2o3_device::trap();
    }
    let plan_base = sequence * 32;
    let mut plan_present = false;
    let mut plan_byte = 0;
    while plan_byte < 32 {
        if memory::volatile_load(plan_identities, plan_base + plan_byte) != 0 {
            plan_present = true;
        }
        plan_byte += 1;
    }
    if !plan_present {
        fe2o3_device::trap();
    }

    let choice_base = sequence * active_tokens;
    let mut accepted = 0;
    if !direct {
        while accepted < speculative_k {
            let draft_index = accepted * sequences + sequence;
            let target_index = choice_base + accepted;
            let draft_token = memory::volatile_load(draft, draft_index);
            let target_token = memory::volatile_load(choices, target_index);
            if draft_token as usize >= QWEN3_LOGITS_VOCABULARY_V1
                || target_token as usize >= QWEN3_LOGITS_VOCABULARY_V1
            {
                fe2o3_device::trap();
            }
            if draft_token != target_token {
                break;
            }
            accepted += 1;
        }
    }
    let correction_index = if direct {
        choice_base + live - 1
    } else {
        choice_base + accepted
    };
    let correction = memory::volatile_load(choices, correction_index);
    if correction as usize >= QWEN3_LOGITS_VOCABULARY_V1 {
        fe2o3_device::trap();
    }

    let mut component = 0;
    while component < 2 {
        let byte = component * 64 + lane;
        if byte < QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1 {
            let value = if byte < 4 {
                (slot >> (byte * 8)) as u8
            } else if byte < 8 {
                (generation >> ((byte - 4) * 8)) as u8
            } else if byte < 16 {
                (memory::volatile_load(completion_epochs, sequence) >> ((byte - 8) * 8)) as u8
            } else if byte < 48 {
                memory::volatile_load(plan_identities, plan_base + byte - 16)
            } else if byte == 48 {
                accepted as u8
            } else if byte == 49 {
                (accepted + 1) as u8
            } else if byte < 52 {
                0
            } else {
                let token_byte = byte - 52;
                let token = token_byte / 4;
                let token_value = if token < accepted {
                    memory::volatile_load(draft, token * sequences + sequence)
                } else if token == accepted {
                    correction
                } else {
                    0
                };
                (token_value >> ((token_byte % 4) * 8)) as u8
            };

            if !records.write_row_striped_2d(
                &record_row,
                component,
                sequences,
                QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1,
                QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1,
                value,
            ) {
                fe2o3_device::trap();
            }
        }
        component += 1;
    }
}

/// Assembles target verification TokenIds from anchors and draft choices.
///
/// The draft input is iteration-major `[K,S]`; the write-only target is
/// sequence-major `[S,K+1]`. Lanes zero through K own one token each.
#[kernel(
    typed,
    launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [8, 1, 1])
)]
pub fn ferric_qwen3_speculative_token_assembly_v1(
    anchor_token_ids: &[u32],
    draft_choices: &[u32],
    mut target_token_ids: WriteOnlyDisjointSlice<u32, RowStriped2D<Index1D, 64, 1>>,
    sequences: u32,
    speculative_k: u32,
) {
    let sequences = sequences as usize;
    let speculative_k = speculative_k as usize;
    let width = speculative_k + 1;
    if !((speculative_k == 4 && (sequences == 1 || sequences == 8))
        || (sequences == 1 && (speculative_k == 8 || speculative_k == 16)))
        || anchor_token_ids.len() != sequences
        || draft_choices.len() != sequences * speculative_k
        || target_token_ids.len() != sequences * width
        || thread::block_dim_x() as usize != 64
        || thread::grid_dim_x() as usize != sequences
    {
        fe2o3_device::trap();
    }

    let invocation = thread::index_1d();
    let raw = invocation.get();
    let token = raw % 64;
    let sequence = raw / 64;
    if sequence >= sequences {
        fe2o3_device::trap();
    }
    if token >= width {
        return;
    }
    let Some(target_row) = invocation.checked_row_striped_2d::<64, 1>() else {
        fe2o3_device::trap();
    };
    let value = if token == 0 {
        memory::volatile_load(anchor_token_ids, sequence)
    } else {
        memory::volatile_load(draft_choices, (token - 1) * sequences + sequence)
    };
    if !target_token_ids.write_row_striped_2d(&target_row, 0, sequences, width, width, value) {
        fe2o3_device::trap();
    }
}
