#![forbid(unsafe_code)]

//! Standalone integer model of the v13 sharded-argmax indexing contract.
//!
//! The model covers integer ownership, finite loops, buffer-index bounds and
//! integer token keys. It does not establish source/compiler refinement, FP32
//! comparisons or conversions, subgroup convergence, ABI, physical aliasing,
//! dispatch completion/order, stale-request rejection, GPU behavior or timing.
//! It is not a runtime certificate or an M1 qualification root.

use vstd::prelude::*;

verus! {

pub open spec fn v13_active_owner(shard: int, lane: int, step: int) -> bool {
    &&& 0 <= shard < 64
    &&& 0 <= lane < 64
    &&& 0 <= step < 38
    &&& 64 * step + shard < 2374
}

pub open spec fn v13_token(shard: int, lane: int, step: int) -> int {
    4096 * step + 64 * shard + lane
}

pub open spec fn v13_shard_steps(shard: int) -> int {
    if shard < 6 { 38 } else { 37 }
}

pub open spec fn v13_scratch_index(row: int, shard: int) -> int {
    64 * row + shard
}

pub open spec fn v13_token_key(token: int) -> int {
    151936 - token
}

// Fixed divisors keep these decomposition obligations in linear arithmetic.
proof fn fixed_division(value: int, quotient: int, remainder: int, divisor: int)
    requires
        divisor == 64 || divisor == 4096,
        value >= 0,
        quotient >= 0,
        0 <= remainder < divisor,
        value == divisor * quotient + remainder,
    ensures
        value / divisor == quotient,
        value % divisor == remainder,
{
    if divisor == 64 {
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod(value, 64);
        vstd::arithmetic::div_mod::lemma_mod_bound(value, 64);
        vstd::arithmetic::div_mod::lemma_remainder_lower(value, 64);
    } else {
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod(value, 4096);
        vstd::arithmetic::div_mod::lemma_mod_bound(value, 4096);
        vstd::arithmetic::div_mod::lemma_remainder_lower(value, 4096);
    }
}

/// Every admitted owner reads one in-range token with this exact inverse.
pub proof fn v13_owner_bounds_and_inverse(shard: int, lane: int, step: int)
    requires v13_active_owner(shard, lane, step),
    ensures
        0 <= v13_token(shard, lane, step) < 151936,
        v13_token(shard, lane, step) % 64 == lane,
        (v13_token(shard, lane, step) / 64) % 64 == shard,
        v13_token(shard, lane, step) / 4096 == step,
{
    let stripe = 64 * step + shard;
    let token = v13_token(shard, lane, step);
    fixed_division(token, stripe, lane, 64);
    fixed_division(stripe, step, shard, 64);
    fixed_division(token, step, 64 * shard + lane, 4096);
}

/// Every token has an active owner, including the final six partial stripes.
pub proof fn v13_every_token_has_an_owner(token: int)
    requires 0 <= token < 151936,
    ensures
        v13_active_owner((token / 64) % 64, token % 64, token / 4096),
        v13_token((token / 64) % 64, token % 64, token / 4096) == token,
{
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(token, 64);
    vstd::arithmetic::div_mod::lemma_mod_bound(token, 64);
    vstd::arithmetic::div_mod::lemma_remainder_lower(token, 64);
    let stripe = token / 64;
    let lane = token % 64;
    assert(0 <= stripe < 2374);
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(stripe, 64);
    vstd::arithmetic::div_mod::lemma_mod_bound(stripe, 64);
    vstd::arithmetic::div_mod::lemma_remainder_lower(stripe, 64);
    let step = stripe / 64;
    let shard = stripe % 64;
    assert(0 <= step < 38);
    assert(token == 4096 * step + 64 * shard + lane);
    fixed_division(token, step, 64 * shard + lane, 4096);
}

/// Equal token indices cannot have distinct admitted shard/lane/step owners.
pub proof fn v13_token_owner_is_unique(
    shard_a: int, lane_a: int, step_a: int,
    shard_b: int, lane_b: int, step_b: int,
)
    requires
        v13_active_owner(shard_a, lane_a, step_a),
        v13_active_owner(shard_b, lane_b, step_b),
        v13_token(shard_a, lane_a, step_a) == v13_token(shard_b, lane_b, step_b),
    ensures shard_a == shard_b && lane_a == lane_b && step_a == step_b,
{
    v13_owner_bounds_and_inverse(shard_a, lane_a, step_a);
    v13_owner_bounds_and_inverse(shard_b, lane_b, step_b);
}

pub proof fn v13_last_step_and_shard_extent(shard: int, step: int)
    requires 0 <= shard < 64, 0 <= step < 38,
    ensures
        (64 * step + shard < 2374) <==> step < v13_shard_steps(shard),
        (64 * step + shard < 2374) <==> (step < 37 || shard < 6),
        (v13_shard_steps(shard) == 38) <==> shard < 6,
{
}

pub open spec fn v13_long_shards_in_prefix(shards: int) -> int
    decreases shards,
{
    if shards <= 0 { 0 } else {
        v13_long_shards_in_prefix(shards - 1) + if shards <= 6 { 1int } else { 0int }
    }
}

pub open spec fn v13_reads_in_prefix(shards: int) -> int
    decreases shards,
{
    if shards <= 0 { 0 } else {
        v13_reads_in_prefix(shards - 1) + 64 * v13_shard_steps(shards - 1)
    }
}

pub proof fn v13_prefix_counts(shards: int)
    requires 0 <= shards <= 64,
    ensures
        v13_long_shards_in_prefix(shards) == if shards <= 6 { shards } else { 6 },
        v13_reads_in_prefix(shards) == if shards <= 6 {
            shards * 64 * 38
        } else {
            6 * 64 * 38 + (shards - 6) * 64 * 37
        },
    decreases shards,
{
    if shards > 0 {
        v13_prefix_counts(shards - 1);
    }
}

/// Exactly six long shards and 58 short shards cover the fixed vocabulary.
pub proof fn v13_complete_partition_counts()
    ensures
        v13_long_shards_in_prefix(64) == 6,
        v13_reads_in_prefix(64) == 151936,
        6 * 64 * 38 + 58 * 64 * 37 == 151936,
{
    v13_prefix_counts(64);
}

/// Lane zero of exactly one producer group owns each active scratch pair.
pub proof fn v13_scratch_writer_bounds_and_inverse(rows: int, row: int, shard: int)
    requires 1 <= rows <= 32, 0 <= row < rows, 0 <= shard < 64,
    ensures
        0 <= v13_scratch_index(row, shard) < rows * 64 <= 2048,
        v13_scratch_index(row, shard) / 64 == row,
        v13_scratch_index(row, shard) % 64 == shard,
        0 <= 64 * v13_scratch_index(row, shard) < rows * 4096 <= 131072,
        (64 * v13_scratch_index(row, shard)) % 64 == 0,
        (64 * v13_scratch_index(row, shard)) / 64 == v13_scratch_index(row, shard),
{
    let index = v13_scratch_index(row, shard);
    fixed_division(index, row, shard, 64);
    fixed_division(64 * index, index, 0, 64);
}

pub proof fn v13_scratch_writer_is_unique(
    rows: int, row_a: int, shard_a: int, row_b: int, shard_b: int,
)
    requires
        1 <= rows <= 32,
        0 <= row_a < rows, 0 <= row_b < rows,
        0 <= shard_a < 64, 0 <= shard_b < 64,
        v13_scratch_index(row_a, shard_a) == v13_scratch_index(row_b, shard_b),
    ensures row_a == row_b && shard_a == shard_b,
{
    v13_scratch_writer_bounds_and_inverse(rows, row_a, shard_a);
    v13_scratch_writer_bounds_and_inverse(rows, row_b, shard_b);
}

/// One finalizer lane-zero invocation owns each active choice; tails are disjoint.
pub proof fn v13_choice_writer_bounds_and_inverse(rows: int, row: int)
    requires 1 <= rows <= 32, 0 <= row < rows,
    ensures
        0 <= 64 * row < rows * 64 <= 2048,
        (64 * row) % 64 == 0,
        (64 * row) / 64 == row,
        0 <= row < 32,
        0 <= 4 * row < 4 * row + 4 <= 4 * rows <= 128,
{
    fixed_division(64 * row, row, 0, 64);
}

pub proof fn v13_choice_writer_is_unique(rows: int, raw_a: int, raw_b: int)
    requires
        1 <= rows <= 32,
        0 <= raw_a < rows * 64, 0 <= raw_b < rows * 64,
        raw_a % 64 == 0, raw_b % 64 == 0,
        raw_a / 64 == raw_b / 64,
    ensures raw_a == raw_b,
{
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(raw_a, 64);
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(raw_b, 64);
}

pub proof fn v13_capacity_tails_are_disjoint(
    rows: int, row: int, shard: int, scratch_tail: int, choice_tail: int,
)
    requires
        1 <= rows <= 32, 0 <= row < rows, 0 <= shard < 64,
        rows * 64 <= scratch_tail < 2048,
        rows <= choice_tail < 32,
    ensures v13_scratch_index(row, shard) != scratch_tail, row != choice_tail,
{
    v13_scratch_writer_bounds_and_inverse(rows, row, shard);
}

/// Active logit, scratch and choice byte ranges stay inside their integer extents.
pub proof fn v13_active_memory_index_bounds(
    rows: int, row: int, shard: int, lane: int, step: int,
)
    requires
        1 <= rows <= 32, 0 <= row < rows,
        v13_active_owner(shard, lane, step),
    ensures
        0 <= row * 151936 + v13_token(shard, lane, step) < rows * 151936 <= 4861952,
        0 <= 4 * (row * 151936 + v13_token(shard, lane, step)),
        4 * (row * 151936 + v13_token(shard, lane, step)) + 4
            <= rows * 151936 * 4 <= 19447808 < 4294967296,
        0 <= 4 * v13_scratch_index(row, shard),
        4 * v13_scratch_index(row, shard) + 4 <= rows * 64 * 4 <= 8192,
        0 <= 4 * row < 4 * row + 4 <= 4 * rows <= 128,
{
    v13_owner_bounds_and_inverse(shard, lane, step);
    v13_scratch_writer_bounds_and_inverse(rows, row, shard);
}

/// These integer bounds fit u32 and thus a 64-bit usize; no machine execution is modeled.
pub proof fn v13_loop_rank_and_machine_index_bounds(
    rows: int, shard: int, lane: int, step: int,
)
    requires
        1 <= rows <= 32,
        0 <= shard < 64, 0 <= lane < 64,
        1 <= step < 38,
    ensures
        0 <= 38 - (step + 1) < 38 - step <= 37,
        step + 1 <= 38,
        0 <= 64 * step + shard <= 2431,
        0 <= v13_token(shard, lane, step) <= 155647 < 4294967296,
        rows * 64 <= 2048,
        rows * 4096 <= 131072 < 4294967296,
        rows * 151936 <= 4861952,
        rows * 151936 * 4 <= 19447808 < 4294967296,
        2 * 2048 * 4 == 16384,
{
}

/// Key arithmetic is over mathematical integers, not a claim about FP32 conversion.
pub proof fn v13_key_bounds_and_inverse(token: int)
    requires 0 <= token < 151936,
    ensures
        1 <= v13_token_key(token) <= 151936 < 16777216,
        151936 - v13_token_key(token) == token,
{
}

pub proof fn v13_key_order_is_reversed(token_a: int, token_b: int)
    requires 0 <= token_a < 151936, 0 <= token_b < 151936,
    ensures
        (token_a < token_b) <==> (v13_token_key(token_a) > v13_token_key(token_b)),
        (token_a == token_b) <==> (v13_token_key(token_a) == v13_token_key(token_b)),
{
}

} // verus!
