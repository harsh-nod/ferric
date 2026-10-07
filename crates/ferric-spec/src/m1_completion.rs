//! Deterministic M1 argmax and compact completion-record semantics.

use crate::completion::CompletionEpoch;
use crate::{Identity, RequestId, TokenId, QWEN3_VOCABULARY_SIZE};
use vstd::prelude::*;

verus! {

/// Maximum tokens published by one `K <= 16` greedy speculative round.
pub const M1_MAX_COMPLETION_TOKENS: usize = 17;

/// Untrusted compact result written by the final logits/completion device graph.
///
/// The record is accepted only after [`validate_compact_completion`] binds it
/// to the exact request generation, completion epoch, plan identity, and draft
/// length. Slots beyond `emitted_token_count` must be zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompactCompletionRecord {
    pub request: RequestId,
    pub epoch: CompletionEpoch,
    pub plan_id: Identity,
    pub accepted_draft_tokens: u8,
    pub emitted_token_count: u8,
    pub emitted_tokens: [TokenId; M1_MAX_COMPLETION_TOKENS],
}

impl CompactCompletionRecord {
    pub closed spec fn emitted_spec(&self) -> Seq<TokenId> {
        self.emitted_tokens@
    }
}

pub proof fn compact_completion_emitted_view(record: &CompactCompletionRecord)
    ensures record.emitted_spec() == record.emitted_tokens@,
{
}

/// Fail-closed rejection for deterministic M1 result publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompactCompletionError {
    ExpectedPlanIdentityAbsent,
    RequestMismatch,
    EpochMismatch,
    PlanIdentityMismatch,
    DraftLengthOutOfRange,
    AcceptedLengthOutOfRange,
    EmittedLengthMismatch,
    TokenOutOfRange,
    NonzeroUnusedToken,
    ScoreCountMismatch,
}

pub open spec fn identity_present(identity: Identity) -> bool {
    exists|index: int|
        0 <= index < identity.bytes_spec().len()
            && identity.bytes_spec()[index] != 0
}

/// Scalar identity and length header of one compact completion record.
pub open spec fn compact_completion_header_matches(
    record: CompactCompletionRecord,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: Identity,
    draft_token_count: u8,
) -> bool {
    identity_present(expected_plan_id)
        && record.request.slot_spec() == expected_request.slot_spec()
        && record.request.generation_spec() == expected_request.generation_spec()
        && record.epoch.value == expected_epoch.value
        && record.plan_id.bytes_spec() == expected_plan_id.bytes_spec()
        && draft_token_count <= 16
        && record.accepted_draft_tokens <= draft_token_count
        && record.emitted_token_count as int == record.accepted_draft_tokens as int + 1
        && record.emitted_token_count as int <= M1_MAX_COMPLETION_TOKENS as int
}

pub open spec fn compact_completion_live_tokens_match(
    record: CompactCompletionRecord,
) -> bool {
    forall|index: int|
        0 <= index < record.emitted_token_count as int
            ==> record.emitted_spec()[index] < QWEN3_VOCABULARY_SIZE
}

pub open spec fn compact_completion_unused_tokens_match(
    record: CompactCompletionRecord,
) -> bool {
    forall|index: int|
        record.emitted_token_count as int <= index < M1_MAX_COMPLETION_TOKENS as int
            ==> record.emitted_spec()[index] == 0
}

/// Mathematical acceptance relation for one compact completion record.
pub open spec fn compact_completion_matches(
    record: CompactCompletionRecord,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: Identity,
    draft_token_count: u8,
) -> bool {
    compact_completion_header_matches(
        record,
        expected_request,
        expected_epoch,
        expected_plan_id,
        draft_token_count,
    ) && compact_completion_live_tokens_match(record)
        && compact_completion_unused_tokens_match(record)
}

/// Validates an untrusted device completion before any token or state is
/// published to the logical engine.
///
/// # Errors
///
/// Returns [`CompactCompletionError`] unless every identity, bound, live
/// token, and canonical unused slot matches the exact expected round.
pub fn validate_compact_completion(
    record: &CompactCompletionRecord,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: &Identity,
    draft_token_count: u8,
) -> (result: Result<(), CompactCompletionError>)
    ensures
        result.is_ok() == compact_completion_matches(
            *record,
            expected_request,
            expected_epoch,
            *expected_plan_id,
            draft_token_count,
        ),
{
    if !expected_plan_id.is_present() {
        return Err(CompactCompletionError::ExpectedPlanIdentityAbsent);
    }
    if record.request.slot() != expected_request.slot()
        || record.request.generation() != expected_request.generation()
    {
        return Err(CompactCompletionError::RequestMismatch);
    }
    if record.epoch.value != expected_epoch.value {
        return Err(CompactCompletionError::EpochMismatch);
    }
    if !record.plan_id.equals(expected_plan_id) {
        return Err(CompactCompletionError::PlanIdentityMismatch);
    }
    if draft_token_count > 16 {
        return Err(CompactCompletionError::DraftLengthOutOfRange);
    }
    if record.accepted_draft_tokens > draft_token_count {
        return Err(CompactCompletionError::AcceptedLengthOutOfRange);
    }
    let expected_emitted = record.accepted_draft_tokens + 1;
    if record.emitted_token_count != expected_emitted
        || record.emitted_token_count as usize > M1_MAX_COMPLETION_TOKENS
    {
        return Err(CompactCompletionError::EmittedLengthMismatch);
    }
    assert(compact_completion_header_matches(
        *record,
        expected_request,
        expected_epoch,
        *expected_plan_id,
        draft_token_count,
    ));

    let mut index = 0;
    while index < record.emitted_token_count as usize
        invariant
            compact_completion_header_matches(
                *record,
                expected_request,
                expected_epoch,
                *expected_plan_id,
                draft_token_count,
            ),
            index <= record.emitted_token_count as int,
            record.emitted_token_count as int <= M1_MAX_COMPLETION_TOKENS as int,
            forall|prior: int|
                0 <= prior < index
                    ==> record.emitted_spec()[prior] < QWEN3_VOCABULARY_SIZE,
        decreases record.emitted_token_count as int - index,
    {
        if record.emitted_tokens[index] >= QWEN3_VOCABULARY_SIZE {
            assert(record.emitted_spec()[index as int] == record.emitted_tokens[index as int]);
            assert(!compact_completion_matches(
                *record,
                expected_request,
                expected_epoch,
                *expected_plan_id,
                draft_token_count,
            )) by {
                assert(!(forall|position: int|
                    0 <= position < record.emitted_token_count as int
                        ==> record.emitted_spec()[position] < QWEN3_VOCABULARY_SIZE));
            }
            return Err(CompactCompletionError::TokenOutOfRange);
        }
        index += 1;
    }
    while index < M1_MAX_COMPLETION_TOKENS
        invariant
            compact_completion_header_matches(
                *record,
                expected_request,
                expected_epoch,
                *expected_plan_id,
                draft_token_count,
            ),
            record.emitted_token_count as int <= index <= M1_MAX_COMPLETION_TOKENS as int,
            forall|prior: int|
                0 <= prior < record.emitted_token_count as int
                    ==> record.emitted_spec()[prior] < QWEN3_VOCABULARY_SIZE,
            forall|prior: int|
                record.emitted_token_count as int <= prior < index
                    ==> record.emitted_spec()[prior] == 0,
        decreases M1_MAX_COMPLETION_TOKENS - index,
    {
        if record.emitted_tokens[index] != 0 {
            assert(record.emitted_spec()[index as int] == record.emitted_tokens[index as int]);
            assert(record.emitted_token_count as int <= index as int);
            assert((index as int) < M1_MAX_COMPLETION_TOKENS as int);
            assert(record.emitted_spec()[index as int] != 0);
            assert(!compact_completion_unused_tokens_match(*record)) by {
                reveal(compact_completion_unused_tokens_match);
                assert(!(forall|position: int|
                    record.emitted_token_count as int
                            <= position < M1_MAX_COMPLETION_TOKENS as int
                        ==> record.emitted_spec()[position] == 0)) by {
                    if forall|position: int|
                        record.emitted_token_count as int
                                <= position < M1_MAX_COMPLETION_TOKENS as int
                            ==> record.emitted_spec()[position] == 0
                    {
                        assert(record.emitted_spec()[index as int] == 0);
                        assert(false);
                    }
                }
            }
            assert(!compact_completion_matches(
                *record,
                expected_request,
                expected_epoch,
                *expected_plan_id,
                draft_token_count,
            )) by {
                reveal(compact_completion_matches);
            }
            return Err(CompactCompletionError::NonzeroUnusedToken);
        }
        index += 1;
    }
    Ok(())
}

/// Integer ordering key for a finite BF16 encoding, merging the two zeros.
///
/// The exponent-all-ones encodings are outside this finite score domain.
pub open spec fn finite_bf16_order_key_spec(bits: u16) -> Option<i64> {
    let magnitude = bits as int % 32_768;
    if magnitude >= 32_640 {
        None
    } else if bits < 32_768 {
        Some(magnitude as i64)
    } else {
        Some((-magnitude) as i64)
    }
}

/// Maps finite BF16 bits into the integer score order used by M1 argmax.
///
/// Both signed zeros map to zero. Negative finite encodings reverse magnitude
/// order; positive encodings preserve it. NaNs and infinities return `None`.
/// This is an encoded-value contract, not a GPU arithmetic or model-accuracy
/// claim. Its exact-result postcondition requires a separate Verus result.
#[must_use]
#[allow(clippy::cast_lossless, reason = "Verus models widening casts directly but the pinned From call is opaque")]
pub fn finite_bf16_order_key(bits: u16) -> (key: Option<i64>)
    ensures key == finite_bf16_order_key_spec(bits),
{
    let magnitude = bits % 32_768;
    if magnitude >= 32_640 {
        None
    } else if bits < 32_768 {
        Some(magnitude as i64)
    } else {
        Some(-(magnitude as i64))
    }
}

/// Failure while scanning one complete little-endian BF16 vocabulary row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FiniteBf16ArgmaxError {
    RowExtent,
    NonFinite { token: TokenId },
}

pub open spec fn bf16_row_bits(bytes: Seq<u8>, token: int) -> u16 {
    (bytes[2 * token] as int + 256 * bytes[2 * token + 1] as int) as u16
}

pub open spec fn bf16_row_key(bytes: Seq<u8>, token: int) -> Option<i64> {
    finite_bf16_order_key_spec(bf16_row_bits(bytes, token))
}

/// Finite encoded-value maximum with the lowest token ID among equal keys.
pub open spec fn is_lowest_finite_bf16_argmax(bytes: Seq<u8>, token: TokenId) -> bool {
    bytes.len() == 2 * QWEN3_VOCABULARY_SIZE
        && token < QWEN3_VOCABULARY_SIZE
        && bf16_row_key(bytes, token as int).is_some()
        && (forall|index: int| 0 <= index < QWEN3_VOCABULARY_SIZE
            ==> bf16_row_key(bytes, index).is_some())
        && (forall|index: int| 0 <= index < QWEN3_VOCABULARY_SIZE
            ==> bf16_row_key(bytes, token as int).unwrap() >= bf16_row_key(bytes, index).unwrap())
        && (forall|index: int| 0 <= index < token as int
            ==> bf16_row_key(bytes, index).unwrap() < bf16_row_key(bytes, token as int).unwrap())
}

/// Scans the complete BF16 row without allocating or converting to floats.
///
/// Signed zeros tie. The first nonfinite encoding rejects the entire row,
/// including when a preceding finite value already attains the largest key.
/// This contract covers encoded bytes and selection, not device arithmetic.
///
/// # Errors
///
/// Rejects any extent other than two bytes per vocabulary entry, or the first
/// nonfinite entry in token order.
#[allow(clippy::cast_lossless, reason = "Verus models widening casts directly but the pinned From call is opaque")]
pub fn select_lowest_finite_bf16_argmax(
    bytes: &[u8],
) -> (result: Result<TokenId, FiniteBf16ArgmaxError>)
    ensures
        match result {
            Ok(token) => is_lowest_finite_bf16_argmax(bytes@, token),
            Err(FiniteBf16ArgmaxError::RowExtent) => bytes@.len() != 2 * QWEN3_VOCABULARY_SIZE,
            Err(FiniteBf16ArgmaxError::NonFinite { token }) => {
                bytes@.len() == 2 * QWEN3_VOCABULARY_SIZE
                    && token < QWEN3_VOCABULARY_SIZE
                    && bf16_row_key(bytes@, token as int).is_none()
                    && forall|prior: int| 0 <= prior < token as int
                        ==> bf16_row_key(bytes@, prior).is_some()
            },
        },
{
    if bytes.len() != 2 * QWEN3_VOCABULARY_SIZE as usize {
        return Err(FiniteBf16ArgmaxError::RowExtent);
    }
    let mut index = 0u32;
    let mut best = 0u32;
    let mut best_value = 0i64;
    while index < QWEN3_VOCABULARY_SIZE
        invariant
            bytes@.len() == 2 * QWEN3_VOCABULARY_SIZE,
            index <= QWEN3_VOCABULARY_SIZE,
            index == 0 ==> best == 0,
            index > 0 ==> best < index,
            index > 0 ==> bf16_row_key(bytes@, best as int) == Some(best_value),
            index > 0 ==> bf16_row_key(bytes@, 0).is_some(),
            forall|prior: int| 0 <= prior < index as int
                ==> bf16_row_key(bytes@, prior).is_some(),
            forall|prior: int| 0 <= prior < index as int
                ==> bf16_row_key(bytes@, prior).unwrap() <= best_value,
            forall|prior: int| 0 <= prior < best as int
                ==> bf16_row_key(bytes@, prior).unwrap() < best_value,
        decreases QWEN3_VOCABULARY_SIZE - index,
    {
        let offset = 2 * index as usize;
        let bits = bytes[offset] as u16 + (bytes[offset + 1] as u16) * 256;
        assert(bits == bf16_row_bits(bytes@, index as int));
        let value = match finite_bf16_order_key(bits) {
            Some(value) => value,
            None => return Err(FiniteBf16ArgmaxError::NonFinite { token: index }),
        };
        if index == 0 || value > best_value {
            best = index;
            best_value = value;
        }
        index += 1;
    }
    assert(index == QWEN3_VOCABULARY_SIZE);
    Ok(best)
}

/// Mathematical lowest-token tie-breaking argmax relation.
pub open spec fn is_lowest_argmax(scores: Seq<i64>, token: TokenId) -> bool {
    scores.len() == QWEN3_VOCABULARY_SIZE
        && token < QWEN3_VOCABULARY_SIZE
        && forall|index: int|
            0 <= index < scores.len() ==> scores[token as int] >= scores[index]
        && forall|index: int|
            0 <= index < token as int ==> scores[index] < scores[token as int]
}

/// Selects the maximum ordered logit, retaining the lowest token ID on ties.
///
/// The input is an integer total-order abstraction. [`finite_bf16_order_key`]
/// supplies finite encoded BF16 keys for the host observer, including signed
/// zero equality and nonfinite rejection. Correspondence to device arithmetic
/// and FP32 model results remains a separate numerical contract.
///
/// # Errors
///
/// Returns [`CompactCompletionError::ScoreCountMismatch`] unless there is
/// exactly one score for every pinned Qwen3 vocabulary entry.
pub fn select_lowest_argmax(
    scores: &[i64],
) -> (result: Result<TokenId, CompactCompletionError>)
    ensures
        match result {
            Ok(token) => is_lowest_argmax(scores@, token),
            Err(CompactCompletionError::ScoreCountMismatch) => {
                scores@.len() != QWEN3_VOCABULARY_SIZE
            },
            Err(_) => false,
        },
{
    if scores.len() != QWEN3_VOCABULARY_SIZE as usize {
        return Err(CompactCompletionError::ScoreCountMismatch);
    }
    let mut best = 0u32;
    let mut index = 1u32;
    while (index as usize) < scores.len()
        invariant
            scores@.len() == QWEN3_VOCABULARY_SIZE,
            0 <= best < index <= scores@.len(),
            forall|prior: int|
                0 <= prior < index ==> scores@[best as int] >= scores@[prior],
            forall|prior: int|
                0 <= prior < best ==> scores@[prior] < scores@[best as int],
        decreases scores@.len() - index as int,
    {
        if scores[index as usize] > scores[best as usize] {
            best = index;
        }
        assert(forall|prior: int|
            0 <= prior < best ==> scores@[prior] < scores@[best as int]);
        index += 1;
    }
    assert(best < QWEN3_VOCABULARY_SIZE);
    Ok(best)
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{
        finite_bf16_order_key, select_lowest_argmax, select_lowest_finite_bf16_argmax,
        validate_compact_completion, CompactCompletionError, CompactCompletionRecord,
        FiniteBf16ArgmaxError, M1_MAX_COMPLETION_TOKENS,
    };
    use crate::completion::CompletionEpoch;
    use crate::{Identity, RequestId, QWEN3_VOCABULARY_SIZE};

    fn record() -> CompactCompletionRecord {
        let mut tokens = [0; M1_MAX_COMPLETION_TOKENS];
        tokens[0] = 7;
        tokens[1] = 11;
        tokens[2] = 13;
        CompactCompletionRecord {
            request: RequestId::new(3, 9),
            epoch: CompletionEpoch::new(12),
            plan_id: Identity::new([5; 32]),
            accepted_draft_tokens: 2,
            emitted_token_count: 3,
            emitted_tokens: tokens,
        }
    }

    #[test]
    fn exact_completion_record_is_accepted() {
        assert_eq!(
            validate_compact_completion(
                &record(),
                RequestId::new(3, 9),
                CompletionEpoch::new(12),
                &Identity::new([5; 32]),
                4,
            ),
            Ok(())
        );
    }

    #[test]
    fn stale_identity_bounds_and_unused_slots_fail_closed() {
        let expected = Identity::new([5; 32]);
        let mut changed = record();
        changed.request = RequestId::new(3, 10);
        assert_eq!(
            validate_compact_completion(
                &changed,
                RequestId::new(3, 9),
                CompletionEpoch::new(12),
                &expected,
                4,
            ),
            Err(CompactCompletionError::RequestMismatch)
        );

        changed = record();
        changed.accepted_draft_tokens = 5;
        assert_eq!(
            validate_compact_completion(
                &changed,
                RequestId::new(3, 9),
                CompletionEpoch::new(12),
                &expected,
                4,
            ),
            Err(CompactCompletionError::AcceptedLengthOutOfRange)
        );

        changed = record();
        changed.emitted_tokens[16] = 1;
        assert_eq!(
            validate_compact_completion(
                &changed,
                RequestId::new(3, 9),
                CompletionEpoch::new(12),
                &expected,
                4,
            ),
            Err(CompactCompletionError::NonzeroUnusedToken)
        );
    }

    #[test]
    fn every_completion_header_and_live_token_drift_fails_closed() {
        let expected = Identity::new([5; 32]);
        let expected_request = RequestId::new(3, 9);
        let expected_epoch = CompletionEpoch::new(12);

        assert_eq!(
            validate_compact_completion(
                &record(),
                expected_request,
                expected_epoch,
                &Identity::new([0; 32]),
                4,
            ),
            Err(CompactCompletionError::ExpectedPlanIdentityAbsent)
        );

        let mut changed = record();
        changed.epoch = CompletionEpoch::new(13);
        assert_eq!(
            validate_compact_completion(&changed, expected_request, expected_epoch, &expected, 4),
            Err(CompactCompletionError::EpochMismatch)
        );

        assert_eq!(
            validate_compact_completion(
                &record(),
                expected_request,
                expected_epoch,
                &Identity::new([6; 32]),
                4,
            ),
            Err(CompactCompletionError::PlanIdentityMismatch)
        );

        assert_eq!(
            validate_compact_completion(&record(), expected_request, expected_epoch, &expected, 17),
            Err(CompactCompletionError::DraftLengthOutOfRange)
        );

        changed = record();
        changed.emitted_token_count = 2;
        assert_eq!(
            validate_compact_completion(&changed, expected_request, expected_epoch, &expected, 4),
            Err(CompactCompletionError::EmittedLengthMismatch)
        );

        changed = record();
        changed.emitted_tokens[0] = QWEN3_VOCABULARY_SIZE;
        assert_eq!(
            validate_compact_completion(&changed, expected_request, expected_epoch, &expected, 4),
            Err(CompactCompletionError::TokenOutOfRange)
        );
    }

    #[test]
    fn finite_bf16_order_key_exhaustively_matches_cpu_f32_order() {
        let mut finite = Vec::with_capacity(65_280);
        let mut rejected = 0;
        for bits in u16::MIN..=u16::MAX {
            let value = f32::from_bits(u32::from(bits) << 16);
            match finite_bf16_order_key(bits) {
                Some(key) => {
                    assert!(value.is_finite(), "nonfinite encoding {bits:#06x}");
                    finite.push((key, value));
                }
                None => {
                    assert!(!value.is_finite(), "finite encoding {bits:#06x}");
                    rejected += 1;
                }
            }
        }
        assert_eq!(finite.len(), 65_280);
        assert_eq!(rejected, 256);
        finite.sort_unstable_by_key(|(key, _)| *key);
        let mut equal_neighbors = 0;
        for pair in finite.windows(2) {
            let numerical_order = pair[0].1.partial_cmp(&pair[1].1).unwrap();
            assert_eq!(pair[0].0.cmp(&pair[1].0), numerical_order);
            if numerical_order == core::cmp::Ordering::Equal {
                assert_eq!(pair[0].0, 0);
                equal_neighbors += 1;
            }
        }
        assert_eq!(equal_neighbors, 1);
    }

    #[test]
    fn finite_bf16_order_key_preserves_zero_subnormals_and_extremes() {
        for (bits, expected) in [
            (0x0000, 0),
            (0x8000, 0),
            (0x0001, 1),
            (0x8001, -1),
            (0x007f, 127),
            (0x807f, -127),
            (0x0080, 128),
            (0x8080, -128),
            (0x7f7f, 32_639),
            (0xff7f, -32_639),
        ] {
            assert_eq!(finite_bf16_order_key(bits), Some(expected));
        }
    }

    #[test]
    fn bf16_byte_scan_matches_independent_float_oracle() {
        let mut bytes = Vec::with_capacity(QWEN3_VOCABULARY_SIZE as usize * 2);
        let mut expected = 0;
        let mut maximum = f32::NEG_INFINITY;
        for token in 0..QWEN3_VOCABULARY_SIZE {
            let bits = u16::try_from(token % 65_536).unwrap().rotate_left(7);
            let bits = if bits & 0x7f80 == 0x7f80 { 0 } else { bits };
            bytes.extend_from_slice(&bits.to_le_bytes());
            let value = f32::from_bits(u32::from(bits) << 16);
            assert!(value.is_finite());
            if value > maximum {
                maximum = value;
                expected = token;
            }
        }
        assert_eq!(select_lowest_finite_bf16_argmax(&bytes), Ok(expected));
    }

    #[test]
    fn bf16_byte_scan_preserves_ties_zeros_and_little_endian_order() {
        let mut bytes = vec![0; QWEN3_VOCABULARY_SIZE as usize * 2];
        for pair in bytes.chunks_exact_mut(2) {
            pair.copy_from_slice(&0xff7fu16.to_le_bytes());
        }
        bytes[8..10].copy_from_slice(&0x8000u16.to_le_bytes());
        bytes[12..14].copy_from_slice(&0x0000u16.to_le_bytes());
        assert_eq!(select_lowest_finite_bf16_argmax(&bytes), Ok(4));
        bytes[8..10].copy_from_slice(&0x0000u16.to_le_bytes());
        bytes[12..14].copy_from_slice(&0x8000u16.to_le_bytes());
        assert_eq!(select_lowest_finite_bf16_argmax(&bytes), Ok(4));
        bytes[18..20].copy_from_slice(&0x3f80u16.to_le_bytes());
        bytes[26..28].copy_from_slice(&0x803fu16.to_le_bytes());
        assert_eq!(select_lowest_finite_bf16_argmax(&bytes), Ok(9));
        let last = bytes.len() - 2;
        bytes[last..].copy_from_slice(&0x7f7fu16.to_le_bytes());
        assert_eq!(
            select_lowest_finite_bf16_argmax(&bytes),
            Ok(QWEN3_VOCABULARY_SIZE - 1)
        );
        bytes[32..34].copy_from_slice(&0x7f7fu16.to_le_bytes());
        assert_eq!(select_lowest_finite_bf16_argmax(&bytes), Ok(16));
    }

    #[test]
    fn bf16_byte_scan_rejects_extent_before_reading_any_value() {
        let expected = QWEN3_VOCABULARY_SIZE as usize * 2;
        for length in [0, 1, expected - 2, expected - 1, expected + 1, expected + 2] {
            let bytes = vec![0xff; length];
            assert_eq!(
                select_lowest_finite_bf16_argmax(&bytes),
                Err(FiniteBf16ArgmaxError::RowExtent)
            );
        }
    }

    #[test]
    fn bf16_byte_scan_rejects_first_nonfinite_even_after_maximum() {
        for bits in [0x7f80u16, 0xff80, 0x7f81, 0xff81, 0x7fff, 0xffff] {
            let mut bytes = vec![0; QWEN3_VOCABULARY_SIZE as usize * 2];
            bytes[..2].copy_from_slice(&0x7f7fu16.to_le_bytes());
            let last = bytes.len() - 2;
            bytes[last..].copy_from_slice(&bits.to_le_bytes());
            assert_eq!(
                select_lowest_finite_bf16_argmax(&bytes),
                Err(FiniteBf16ArgmaxError::NonFinite {
                    token: QWEN3_VOCABULARY_SIZE - 1
                })
            );
            bytes[34..36].copy_from_slice(&bits.to_le_bytes());
            assert_eq!(
                select_lowest_finite_bf16_argmax(&bytes),
                Err(FiniteBf16ArgmaxError::NonFinite { token: 17 })
            );
            bytes[..2].copy_from_slice(&bits.to_le_bytes());
            assert_eq!(
                select_lowest_finite_bf16_argmax(&bytes),
                Err(FiniteBf16ArgmaxError::NonFinite { token: 0 })
            );
        }
    }

    #[test]
    fn lowest_token_id_wins_argmax_ties() {
        let mut scores = vec![0; QWEN3_VOCABULARY_SIZE as usize];
        scores[17] = 99;
        scores[23] = 99;
        assert_eq!(select_lowest_argmax(&scores), Ok(17));
        assert_eq!(
            select_lowest_argmax(&scores[..scores.len() - 1]),
            Err(CompactCompletionError::ScoreCountMismatch)
        );
    }
}
