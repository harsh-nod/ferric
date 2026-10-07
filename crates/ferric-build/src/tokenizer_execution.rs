//! Bounded execution of the exact parsed Qwen3 tokenizer program.
//!
//! Verus directly verifies the retained numeric execution bodies for the
//! byte/codepoint bijection, added-token search and special policy, bounded BPE
//! selection/application/termination, and exact byte decode. Construction of
//! this numeric program from authenticated strings, Rust `char` conversion,
//! allocation, NFC, and Oniguruma execution remain contracted runtime inputs;
//! these proofs do not claim exhaustive Hugging Face tokenizer equivalence.

use crate::ADDED_TOKENS;
use onig::Regex;
use std::collections::BTreeMap;
use std::fmt;
use unicode_normalization_alignments::UnicodeNormalization;
use vstd::prelude::*;

verus! {

const BASE_VOCABULARY_SIZE: usize = 151_643;
const ORDERED_LOOKUP_COMPARISONS: usize = 128;

/// Hard upper bound for one tokenizer input.
pub const MAX_TOKENIZER_INPUT_BYTES: usize = 32 * 1_024;
/// Hard upper bound for tokens produced or consumed by one tokenizer call.
pub const MAX_TOKENIZER_OUTPUT_TOKENS: usize = 8_192;
/// Hard upper bound for bytes produced by one decode call.
pub const MAX_TOKENIZER_OUTPUT_BYTES: usize = 128 * 1_024;
/// Hard upper bound for charged finite tokenizer operations in one call.
pub const MAX_TOKENIZER_WORK: usize = 16 * 1_024 * 1_024;

} // verus!

const MAX_TOKENIZER_NORMALIZED_BYTES: usize = 128 * 1_024;

pub(super) const QWEN3_SPLIT_REGEX: &str = "(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\\r\\n\\p{L}\\p{N}]?\\p{L}+|\\p{N}| ?[^\\s\\p{L}\\p{N}]+[\\r\\n]*|\\s*[\\r\\n]+|\\s+(?!\\S)|\\s+";

verus! {

/// Caller-selected bounds for one deterministic tokenizer call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenizerExecutionLimits {
    /// Maximum accepted UTF-8 input bytes for encode.
    pub input_bytes: usize,
    /// Maximum token IDs produced by encode or consumed by decode.
    pub tokens: usize,
    /// Maximum exact bytes produced by decode.
    pub output_bytes: usize,
    /// Maximum charged pretokenizer and BPE operations.
    pub work: usize,
}

#[derive(Debug)]
pub(super) struct AddedTokenProgram {
    pub(super) bytes: Vec<u8>,
    pub(super) special: bool,
    pub(super) token_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AddedTokenMatch {
    start: usize,
    offset: usize,
    token_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MergeSelection {
    rank: usize,
    left: u32,
    right: u32,
    result_id: u32,
}

#[derive(Debug)]
pub(super) struct TokenizerExecutionProgram {
    pub(super) byte_token_ids: [u32; 256],
    pub(super) vocabulary_bytes: Vec<Vec<u8>>,
    pub(super) merge_ranks: BTreeMap<(u32, u32), (usize, u32)>,
    pub(super) added_tokens: Vec<AddedTokenProgram>,
}

impl TokenizerExecutionLimits {
    pub closed spec fn valid(self) -> bool {
        &&& 0 < self.input_bytes <= MAX_TOKENIZER_INPUT_BYTES
        &&& 0 < self.tokens <= MAX_TOKENIZER_OUTPUT_TOKENS
        &&& 0 < self.output_bytes <= MAX_TOKENIZER_OUTPUT_BYTES
        &&& 0 < self.work <= MAX_TOKENIZER_WORK
    }

    /// The closed M1 tokenizer execution envelope.
    #[must_use]
    pub const fn m1() -> (limits: Self)
        ensures limits.valid(),
    {
        Self {
            input_bytes: MAX_TOKENIZER_INPUT_BYTES,
            tokens: MAX_TOKENIZER_OUTPUT_TOKENS,
            output_bytes: MAX_TOKENIZER_OUTPUT_BYTES,
            work: MAX_TOKENIZER_WORK,
        }
    }

    fn validate(self) -> (result: Result<(), TokenizerExecutionError>)
        ensures result.is_ok() == self.valid(),
    {
        if self.input_bytes == 0
            || self.input_bytes > MAX_TOKENIZER_INPUT_BYTES
            || self.tokens == 0
            || self.tokens > MAX_TOKENIZER_OUTPUT_TOKENS
            || self.output_bytes == 0
            || self.output_bytes > MAX_TOKENIZER_OUTPUT_BYTES
            || self.work == 0
            || self.work > MAX_TOKENIZER_WORK
        {
            return Err(TokenizerExecutionError::InvalidLimits);
        }
        Ok(())
    }
}

/// Policy for exact special-token strings encountered during encode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecialTokenEncodePolicy {
    /// Reject input containing any of the fixed special added tokens.
    Reject,
    /// Encode fixed special added tokens by their exact added-token IDs.
    Allow,
}

/// Policy for fixed special-token IDs encountered during decode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecialTokenDecodePolicy {
    /// Preserve every added token as its exact ASCII content bytes.
    Preserve,
    /// Omit only the fixed added tokens whose authenticated `special` bit is true.
    Skip,
}

/// Fail-closed deterministic tokenizer execution failure.
#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenizerExecutionError {
    /// At least one caller bound was zero or exceeded the closed M1 ceiling.
    InvalidLimits,
    /// Input bytes exceeded the caller-selected encode bound.
    InputTooLarge { limit: usize, actual: usize },
    /// NFC expansion exceeded the closed normalized-input ceiling.
    NormalizedInputTooLarge { limit: usize, actual: usize },
    /// A fixed special token occurred while special-token encoding was disabled.
    SpecialTokenForbidden { token_id: u32 },
    /// Token output or decode input exceeded the caller-selected token bound.
    TokenLimit { limit: usize, actual: usize },
    /// Decoded bytes exceeded the caller-selected byte bound.
    OutputByteLimit { limit: usize },
    /// The finite charged operation budget was exhausted.
    WorkLimit { limit: usize },
    /// Checked tokenizer index or length arithmetic overflowed.
    ArithmeticOverflow,
    /// A token ID is outside the exact pinned base and added vocabulary.
    UnknownTokenId(u32),
    /// A pinned vocabulary token did not map through the `ByteLevel` alphabet.
    UnsupportedVocabularySymbol(char),
    /// An allocation within the caller-selected bounds failed.
    AllocationFailed,
}

} // verus!

impl fmt::Display for TokenizerExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => formatter.write_str("tokenizer execution limits are invalid"),
            Self::InputTooLarge { limit, actual } => {
                write!(
                    formatter,
                    "tokenizer input is {actual} bytes, limit is {limit}"
                )
            }
            Self::NormalizedInputTooLarge { limit, actual } => write!(
                formatter,
                "normalized tokenizer input is {actual} bytes, limit is {limit}"
            ),
            Self::SpecialTokenForbidden { token_id } => {
                write!(
                    formatter,
                    "special tokenizer token ID {token_id} is forbidden"
                )
            }
            Self::TokenLimit { limit, actual } => {
                write!(formatter, "token count is {actual}, limit is {limit}")
            }
            Self::OutputByteLimit { limit } => {
                write!(formatter, "decoded tokenizer bytes exceed limit {limit}")
            }
            Self::WorkLimit { limit } => {
                write!(formatter, "tokenizer work exceeds limit {limit}")
            }
            Self::ArithmeticOverflow => formatter.write_str("tokenizer arithmetic overflowed"),
            Self::UnknownTokenId(id) => write!(formatter, "unknown tokenizer token ID {id}"),
            Self::UnsupportedVocabularySymbol(symbol) => write!(
                formatter,
                "tokenizer vocabulary symbol {symbol:?} is outside ByteLevel"
            ),
            Self::AllocationFailed => formatter.write_str("bounded tokenizer allocation failed"),
        }
    }
}

impl std::error::Error for TokenizerExecutionError {}

#[derive(Debug)]
pub(super) enum TokenizerProgramConstructionError {
    ArithmeticOverflow,
    AllocationFailed,
    InvalidVocabulary,
    Regex,
}

#[derive(Debug)]
pub(super) struct TokenizerProgram {
    pub(super) execution: TokenizerExecutionProgram,
    split_regex: Regex,
}

impl TokenizerProgram {
    pub(super) fn new(
        vocabulary: Vec<String>,
        merges: Vec<(String, String)>,
    ) -> Result<Self, TokenizerProgramConstructionError> {
        let token_ids: BTreeMap<String, u32> = vocabulary
            .iter()
            .enumerate()
            .map(|(id, token)| {
                u32::try_from(id)
                    .map(|id| (token.clone(), id))
                    .map_err(|_| TokenizerProgramConstructionError::ArithmeticOverflow)
            })
            .collect::<Result<_, _>>()?;

        let mut byte_token_ids = [0_u32; 256];
        for byte in u8::MIN..=u8::MAX {
            let symbol = byte_to_unicode(byte)
                .ok_or(TokenizerProgramConstructionError::InvalidVocabulary)?
                .to_string();
            byte_token_ids[usize::from(byte)] = token_ids
                .get(&symbol)
                .copied()
                .ok_or(TokenizerProgramConstructionError::InvalidVocabulary)?;
        }

        let mut vocabulary_bytes = Vec::new();
        vocabulary_bytes
            .try_reserve_exact(vocabulary.len())
            .map_err(|_| TokenizerProgramConstructionError::AllocationFailed)?;
        for token in &vocabulary {
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(token.len())
                .map_err(|_| TokenizerProgramConstructionError::AllocationFailed)?;
            for symbol in token.chars() {
                bytes.push(
                    unicode_to_byte(symbol)
                        .ok_or(TokenizerProgramConstructionError::InvalidVocabulary)?,
                );
            }
            vocabulary_bytes.push(bytes);
        }

        let mut merge_ranks = BTreeMap::new();
        for (rank, (left, right)) in merges.into_iter().enumerate() {
            let left_id = token_ids
                .get(&left)
                .copied()
                .ok_or(TokenizerProgramConstructionError::InvalidVocabulary)?;
            let right_id = token_ids
                .get(&right)
                .copied()
                .ok_or(TokenizerProgramConstructionError::InvalidVocabulary)?;
            let capacity = left
                .len()
                .checked_add(right.len())
                .ok_or(TokenizerProgramConstructionError::ArithmeticOverflow)?;
            let mut combined = String::new();
            combined
                .try_reserve_exact(capacity)
                .map_err(|_| TokenizerProgramConstructionError::AllocationFailed)?;
            combined.push_str(&left);
            combined.push_str(&right);
            let result_id = token_ids
                .get(&combined)
                .copied()
                .ok_or(TokenizerProgramConstructionError::InvalidVocabulary)?;
            merge_ranks.insert((left_id, right_id), (rank, result_id));
        }

        let mut added_tokens = Vec::new();
        added_tokens
            .try_reserve_exact(ADDED_TOKENS.len())
            .map_err(|_| TokenizerProgramConstructionError::AllocationFailed)?;
        for (offset, (content, special)) in ADDED_TOKENS.iter().enumerate() {
            let token_id = u32::try_from(BASE_VOCABULARY_SIZE)
                .ok()
                .and_then(|base| base.checked_add(u32::try_from(offset).ok()?))
                .ok_or(TokenizerProgramConstructionError::ArithmeticOverflow)?;
            added_tokens.push(AddedTokenProgram {
                bytes: content.as_bytes().to_vec(),
                special: *special,
                token_id,
            });
        }

        Ok(Self {
            execution: TokenizerExecutionProgram {
                byte_token_ids,
                vocabulary_bytes,
                merge_ranks,
                added_tokens,
            },
            split_regex: Regex::new(QWEN3_SPLIT_REGEX)
                .map_err(|_| TokenizerProgramConstructionError::Regex)?,
        })
    }

    pub(super) fn encode(
        &self,
        input: &str,
        limits: TokenizerExecutionLimits,
        special_tokens: SpecialTokenEncodePolicy,
    ) -> Result<Vec<u32>, TokenizerExecutionError> {
        limits.validate()?;
        if input.len() > limits.input_bytes {
            return Err(TokenizerExecutionError::InputTooLarge {
                limit: limits.input_bytes,
                actual: input.len(),
            });
        }
        let mut work = WorkBudget::new(limits.work);
        if special_tokens == SpecialTokenEncodePolicy::Reject {
            reject_forbidden_special(input.as_bytes(), &self.execution.added_tokens, &mut work)?;
        }
        let mut output = Vec::new();
        output
            .try_reserve(input.len().min(limits.tokens))
            .map_err(|_| TokenizerExecutionError::AllocationFailed)?;
        let mut cursor = 0;
        while cursor < input.len() {
            let next = find_added_token(
                input.as_bytes(),
                cursor,
                &self.execution.added_tokens,
                &mut work,
            )?;
            let normal_end = next.map_or(input.len(), |match_| match_.start);
            self.encode_normal_text(&input[cursor..normal_end], limits, &mut work, &mut output)?;
            let Some(match_) = next else {
                break;
            };
            let added = &self.execution.added_tokens[match_.offset];
            let added_len = emit_added_token(added, special_tokens, &mut output, limits.tokens)?;
            cursor = match_
                .start
                .checked_add(added_len)
                .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
        }
        Ok(output)
    }

    fn encode_normal_text(
        &self,
        input: &str,
        limits: TokenizerExecutionLimits,
        work: &mut WorkBudget,
        output: &mut Vec<u32>,
    ) -> Result<(), TokenizerExecutionError> {
        let mut normalized = String::new();
        normalized
            .try_reserve(input.len())
            .map_err(|_| TokenizerExecutionError::AllocationFailed)?;
        for (symbol, _) in input.nfc() {
            work.charge(1)?;
            let actual = normalized
                .len()
                .checked_add(symbol.len_utf8())
                .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
            if actual > MAX_TOKENIZER_NORMALIZED_BYTES {
                return Err(TokenizerExecutionError::NormalizedInputTooLarge {
                    limit: MAX_TOKENIZER_NORMALIZED_BYTES,
                    actual,
                });
            }
            normalized.push(symbol);
        }

        let mut previous = 0;
        for (start, end) in self.split_regex.find_iter(&normalized) {
            work.charge(
                end.checked_sub(previous)
                    .ok_or(TokenizerExecutionError::ArithmeticOverflow)?,
            )?;
            if previous != start {
                self.execution.encode_piece(
                    &normalized.as_bytes()[previous..start],
                    limits,
                    work,
                    output,
                )?;
            }
            self.execution.encode_piece(
                &normalized.as_bytes()[start..end],
                limits,
                work,
                output,
            )?;
            previous = end;
        }
        if previous != normalized.len() {
            work.charge(
                normalized
                    .len()
                    .checked_sub(previous)
                    .ok_or(TokenizerExecutionError::ArithmeticOverflow)?,
            )?;
            self.execution.encode_piece(
                &normalized.as_bytes()[previous..],
                limits,
                work,
                output,
            )?;
        }
        Ok(())
    }
}

verus! {

closed spec fn bytes_match_at_spec(input: Seq<u8>, start: int, pattern: Seq<u8>) -> bool {
    &&& 0 <= start
    &&& start + pattern.len() <= input.len()
    &&& input.subrange(start, start + pattern.len() as int) == pattern
}

closed spec fn added_program_valid(added_tokens: Seq<AddedTokenProgram>) -> bool {
    &&& forall|offset: int| 0 <= offset < added_tokens.len() ==> {
        &&& added_tokens[offset].bytes@.len() > 0
        &&& added_tokens[offset].bytes@[0] == 0x3c
        &&& added_tokens[offset].token_id as int == BASE_VOCABULARY_SIZE + offset
    }
}

closed spec fn added_match_at(
    input: Seq<u8>,
    added_tokens: Seq<AddedTokenProgram>,
    start: int,
    offset: int,
) -> bool {
    &&& 0 <= offset < added_tokens.len()
    &&& bytes_match_at_spec(input, start, added_tokens[offset].bytes@)
}

fn bytes_match_at(input: &[u8], start: usize, pattern: &[u8]) -> (matches: bool)
    requires start <= input@.len(),
    ensures matches == bytes_match_at_spec(input@, start as int, pattern@),
{
    if pattern.len() > input.len() - start {
        return false;
    }
    let mut index = 0;
    while index < pattern.len()
        invariant
            start <= input@.len(),
            pattern@.len() <= input@.len() - start,
            index <= pattern@.len(),
            start + index <= input@.len(),
            index <= usize::MAX - start,
            forall|prior: int| 0 <= prior < index ==>
                input@[start as int + prior] == pattern@[prior],
        decreases pattern.len() - index,
    {
        let position = start + index;
        if input[position] != pattern[index] {
            assert(input@.subrange(start as int, start as int + pattern@.len() as int)
                != pattern@);
            return false;
        }
        index += 1;
    }
    assert(input@.subrange(start as int, start as int + pattern@.len() as int) =~= pattern@);
    true
}

fn reject_forbidden_special(
    input: &[u8],
    added_tokens: &[AddedTokenProgram],
    work: &mut WorkBudget,
) -> (result: Result<(), TokenizerExecutionError>)
    requires
        old(work).valid(),
        added_program_valid(added_tokens@),
    ensures
        final(work).valid(),
        result.is_ok() ==> forall|start: int, offset: int|
                0 <= start < input@.len() && 0 <= offset < added_tokens@.len()
                    && added_tokens@[offset].special
                    ==> !added_match_at(input@, added_tokens@, start, offset),
        match result {
            Err(TokenizerExecutionError::SpecialTokenForbidden { token_id }) =>
                exists|start: int, offset: int|
                    0 <= start < input@.len()
                        && 0 <= offset < added_tokens@.len()
                        && added_tokens@[offset].special
                        && added_tokens@[offset].token_id == token_id
                        && added_match_at(input@, added_tokens@, start, offset),
            Ok(()) => true,
            Err(_) => true,
        },
{
    let mut start = 0;
    while start < input.len()
        invariant
            start <= input@.len(),
            added_program_valid(added_tokens@),
            work.valid(),
            forall|prior: int, offset: int|
                0 <= prior < start && 0 <= offset < added_tokens@.len()
                    && added_tokens@[offset].special
                    ==> !added_match_at(input@, added_tokens@, prior, offset),
        decreases input.len() - start,
    {
        match work.charge(1) {
            Ok(()) => {},
            Err(error) => return Err(error),
        }
        if input[start] != 0x3c {
            assert forall|offset: int| 0 <= offset < added_tokens@.len()
                && added_tokens@[offset].special implies
                !added_match_at(input@, added_tokens@, start as int, offset) by {
                assert(added_tokens@[offset].bytes@[0] == 0x3c);
            }
            start += 1;
            continue;
        }
        let mut offset = 0;
        while offset < added_tokens.len()
            invariant
                start < input@.len(),
                input@[start as int] == 0x3c,
                offset <= added_tokens@.len(),
                added_program_valid(added_tokens@),
                work.valid(),
                forall|prior: int| 0 <= prior < offset
                    && added_tokens@[prior].special ==>
                    !added_match_at(input@, added_tokens@, start as int, prior),
            decreases added_tokens.len() - offset,
        {
            let added = &added_tokens[offset];
            if !added.special {
                offset += 1;
                continue;
            }
            match work.charge(added.bytes.len()) {
                Ok(()) => {},
                Err(error) => return Err(error),
            }
            if bytes_match_at(input, start, &added.bytes) {
                assert(added_match_at(
                    input@,
                    added_tokens@,
                    start as int,
                    offset as int,
                ));
                return Err(TokenizerExecutionError::SpecialTokenForbidden {
                    token_id: added.token_id,
                });
            }
            offset += 1;
        }
        start += 1;
    }
    Ok(())
}

fn find_added_token(
    input: &[u8],
    cursor: usize,
    added_tokens: &[AddedTokenProgram],
    work: &mut WorkBudget,
) -> (result: Result<Option<AddedTokenMatch>, TokenizerExecutionError>)
    requires
        cursor <= input@.len(),
        old(work).valid(),
        added_program_valid(added_tokens@),
    ensures
        final(work).valid(),
        result.is_ok() ==> match result.unwrap() {
            Some(found) => {
                &&& cursor <= found.start < input@.len()
                &&& found.offset < added_tokens@.len()
                &&& found.token_id == added_tokens@[found.offset as int].token_id
                &&& added_match_at(
                    input@,
                    added_tokens@,
                    found.start as int,
                    found.offset as int,
                )
                &&& forall|prior_start: int, offset: int|
                    cursor <= prior_start < found.start
                        && 0 <= offset < added_tokens@.len()
                        ==> !added_match_at(input@, added_tokens@, prior_start, offset)
                &&& forall|offset: int|
                    0 <= offset < added_tokens@.len()
                        && added_match_at(input@, added_tokens@, found.start as int, offset)
                        ==> {
                            let candidate_len = added_tokens@[offset].bytes@.len();
                            let found_len = added_tokens@[found.offset as int].bytes@.len();
                            candidate_len < found_len
                                || candidate_len == found_len && found.offset as int <= offset
                        }
            },
            None => forall|start: int, offset: int|
                cursor <= start < input@.len() && 0 <= offset < added_tokens@.len()
                    ==> !added_match_at(input@, added_tokens@, start, offset),
        },
{
    let mut start = cursor;
    while start < input.len()
        invariant
            cursor <= start <= input@.len(),
            added_program_valid(added_tokens@),
            work.valid(),
            forall|prior_start: int, offset: int|
                cursor <= prior_start < start && 0 <= offset < added_tokens@.len()
                    ==> !added_match_at(input@, added_tokens@, prior_start, offset),
        decreases input.len() - start,
    {
        match work.charge(1) {
            Ok(()) => {},
            Err(error) => return Err(error),
        }
        if input[start] == 0x3c {
            let mut best: Option<AddedTokenMatch> = None;
            let mut offset = 0;
            while offset < added_tokens.len()
                invariant
                    start < input@.len(),
                    input@[start as int] == 0x3c,
                    offset <= added_tokens@.len(),
                    added_program_valid(added_tokens@),
                    work.valid(),
                    match best {
                        Some(found) => {
                            &&& found.start == start
                            &&& found.offset < offset
                            &&& found.token_id
                                == added_tokens@[found.offset as int].token_id
                            &&& added_match_at(
                                input@,
                                added_tokens@,
                                start as int,
                                found.offset as int,
                            )
                            &&& forall|prior: int| 0 <= prior < offset
                                && added_match_at(input@, added_tokens@, start as int, prior)
                                ==> {
                                    let candidate_len = added_tokens@[prior].bytes@.len();
                                    let found_len =
                                        added_tokens@[found.offset as int].bytes@.len();
                                    candidate_len < found_len
                                        || candidate_len == found_len
                                            && found.offset as int <= prior
                                }
                        },
                        None => forall|prior: int| 0 <= prior < offset ==>
                            !added_match_at(input@, added_tokens@, start as int, prior),
                    },
                decreases added_tokens.len() - offset,
            {
                let added = &added_tokens[offset];
                match work.charge(added.bytes.len()) {
                    Ok(()) => {},
                    Err(error) => return Err(error),
                }
                if !bytes_match_at(input, start, &added.bytes) {
                    offset += 1;
                    continue;
                }
                let candidate = AddedTokenMatch {
                    start,
                    offset,
                    token_id: added.token_id,
                };
                let replace = match best {
                    None => true,
                    Some(current) => {
                        added.bytes.len() > added_tokens[current.offset].bytes.len()
                            || (added.bytes.len() == added_tokens[current.offset].bytes.len()
                                && offset < current.offset)
                    }
                };
                if replace {
                    best = Some(candidate);
                }
                offset += 1;
            }
            if best.is_some() {
                return Ok(best);
            }
        } else {
            assert forall|offset: int| 0 <= offset < added_tokens@.len() implies
                !added_match_at(input@, added_tokens@, start as int, offset) by {
                assert(added_tokens@[offset].bytes@[0] == 0x3c);
            }
        }
        start += 1;
    }
    Ok(None)
}

} // verus!

verus! {

broadcast use {
    vstd::laws_cmp::group_laws_cmp,
    vstd::std_specs::btree::group_btree_axioms,
};

closed spec fn merge_rule_at(
    symbols: Seq<u32>,
    merge_ranks: Map<(u32, u32), (usize, u32)>,
    index: int,
) -> Option<MergeSelection> {
    if 0 <= index && index + 1 < symbols.len()
        && merge_ranks.contains_key((symbols[index], symbols[index + 1]))
    {
        let key = (symbols[index], symbols[index + 1]);
        let rule = merge_ranks[key];
        Some(MergeSelection {
            rank: rule.0,
            left: key.0,
            right: key.1,
            result_id: rule.1,
        })
    } else {
        None
    }
}

closed spec fn prefer_merge(
    current: Option<MergeSelection>,
    candidate: Option<MergeSelection>,
) -> Option<MergeSelection> {
    match (current, candidate) {
        (None, candidate) => candidate,
        (current, None) => current,
        (Some(current), Some(candidate)) => {
            if candidate.rank < current.rank {
                Some(candidate)
            } else {
                Some(current)
            }
        },
    }
}

closed spec fn select_merge_prefix(
    symbols: Seq<u32>,
    merge_ranks: Map<(u32, u32), (usize, u32)>,
    count: nat,
) -> Option<MergeSelection>
    recommends count <= symbols.len(),
    decreases count,
{
    if count == 0 {
        None
    } else {
        prefer_merge(
            select_merge_prefix(symbols, merge_ranks, (count - 1) as nat),
            merge_rule_at(symbols, merge_ranks, count as int - 1),
        )
    }
}

closed spec fn selected_merge(
    symbols: Seq<u32>,
    merge_ranks: Map<(u32, u32), (usize, u32)>,
) -> Option<MergeSelection> {
    if symbols.len() <= 1 {
        None
    } else {
        select_merge_prefix(symbols, merge_ranks, (symbols.len() - 1) as nat)
    }
}

fn selection_from_rule(
    key: (u32, u32),
    rule: Option<&(usize, u32)>,
) -> (selection: Option<MergeSelection>)
    ensures selection == match rule {
        Some(rule) => Some(MergeSelection {
            rank: rule.0,
            left: key.0,
            right: key.1,
            result_id: rule.1,
        }),
        None => None,
    },
{
    let rule = rule?;
    Some(MergeSelection {
        rank: rule.0,
        left: key.0,
        right: key.1,
        result_id: rule.1,
    })
}

fn select_merge(
    symbols: &[u32],
    merge_ranks: &BTreeMap<(u32, u32), (usize, u32)>,
    work: &mut WorkBudget,
) -> (result: Result<Option<MergeSelection>, TokenizerExecutionError>)
    requires old(work).valid(),
    ensures
        final(work).valid(),
        result.is_ok() ==> result.unwrap() == selected_merge(symbols@, merge_ranks@),
{
    if symbols.len() <= 1 {
        return Ok(None);
    }
    let mut best: Option<MergeSelection> = None;
    let mut index = 0;
    while index < symbols.len() - 1
        invariant
            symbols@.len() > 1,
            index <= symbols@.len() - 1,
            work.valid(),
            best == select_merge_prefix(symbols@, merge_ranks@, index as nat),
        decreases symbols.len() - 1 - index,
    {
        match charge_ordered_lookup(work, 4, 4) {
            Ok(()) => {},
            Err(error) => return Err(error),
        }
        let key = (symbols[index], symbols[index + 1]);
        let rule = merge_ranks.get(&key);
        let candidate = selection_from_rule(key, rule);
        proof {
            reveal(merge_rule_at);
            assert(candidate == merge_rule_at(symbols@, merge_ranks@, index as int));
        }
        best = match (best, candidate) {
            (None, candidate) => candidate,
            (current, None) => current,
            (Some(current), Some(candidate)) => {
                if candidate.rank < current.rank {
                    Some(candidate)
                } else {
                    Some(current)
                }
            },
        };
        index += 1;
    }
    Ok(best)
}

proof fn select_merge_prefix_has_witness(
    symbols: Seq<u32>,
    merge_ranks: Map<(u32, u32), (usize, u32)>,
    count: nat,
)
    requires count <= symbols.len(),
    ensures match select_merge_prefix(symbols, merge_ranks, count) {
        Some(selection) => {
            &&& has_adjacent_pair(symbols, selection.left, selection.right)
            &&& merge_ranks.contains_key((selection.left, selection.right))
            &&& merge_ranks[(selection.left, selection.right)]
                == (selection.rank, selection.result_id)
        },
        None => true,
    },
    decreases count,
{
    if count > 0 {
        select_merge_prefix_has_witness(symbols, merge_ranks, (count - 1) as nat);
        let candidate = merge_rule_at(symbols, merge_ranks, count as int - 1);
        if candidate.is_some() {
            let selection = candidate.unwrap();
            assert(has_adjacent_pair(symbols, selection.left, selection.right)) by {
                let index = count as int - 1;
                assert(symbols[index] == selection.left);
                assert(symbols[index + 1] == selection.right);
            }
        }
    }
}

proof fn selected_merge_has_witness(
    symbols: Seq<u32>,
    merge_ranks: Map<(u32, u32), (usize, u32)>,
)
    ensures match selected_merge(symbols, merge_ranks) {
        Some(selection) => {
            &&& has_adjacent_pair(symbols, selection.left, selection.right)
            &&& merge_ranks.contains_key((selection.left, selection.right))
            &&& merge_ranks[(selection.left, selection.right)]
                == (selection.rank, selection.result_id)
        },
        None => true,
    },
{
    if symbols.len() > 1 {
        select_merge_prefix_has_witness(
            symbols,
            merge_ranks,
            (symbols.len() - 1) as nat,
        );
    }
}

closed spec fn has_adjacent_pair(symbols: Seq<u32>, left: u32, right: u32) -> bool {
    exists|index: int| #![trigger symbols[index]] 0 <= index && index + 1 < symbols.len()
        && symbols[index] == left && symbols[index + 1] == right
}

closed spec fn apply_merge_spec(
    symbols: Seq<u32>,
    left: u32,
    right: u32,
    result_id: u32,
) -> Seq<u32>
    decreases symbols.len(),
{
    if symbols.len() == 0 {
        Seq::empty()
    } else if symbols.len() >= 2 && symbols[0] == left && symbols[1] == right {
        seq![result_id] + apply_merge_spec(
            symbols.subrange(2, symbols.len() as int),
            left,
            right,
            result_id,
        )
    } else {
        seq![symbols[0]] + apply_merge_spec(
            symbols.subrange(1, symbols.len() as int),
            left,
            right,
            result_id,
        )
    }
}

proof fn apply_merge_spec_length(
    symbols: Seq<u32>,
    left: u32,
    right: u32,
    result_id: u32,
)
    ensures
        apply_merge_spec(symbols, left, right, result_id).len() <= symbols.len(),
        has_adjacent_pair(symbols, left, right) ==>
            apply_merge_spec(symbols, left, right, result_id).len() < symbols.len(),
    decreases symbols.len(),
{
    if symbols.len() == 0 {
    } else if symbols.len() >= 2 && symbols[0] == left && symbols[1] == right {
        apply_merge_spec_length(
            symbols.subrange(2, symbols.len() as int),
            left,
            right,
            result_id,
        );
    } else {
        let tail = symbols.subrange(1, symbols.len() as int);
        if has_adjacent_pair(symbols, left, right) {
            let index = choose|index: int| #![trigger symbols[index]] 0 <= index && index + 1 < symbols.len()
                && symbols[index] == left && symbols[index + 1] == right;
            assert(index > 0);
            assert(has_adjacent_pair(tail, left, right)) by {
                let tail_index = index - 1;
                assert(tail[tail_index] == symbols[index]);
                assert(tail[tail_index + 1] == symbols[index + 1]);
            }
        }
        apply_merge_spec_length(tail, left, right, result_id);
    }
}

fn apply_merge(
    symbols: &[u32],
    selection: MergeSelection,
    work: &mut WorkBudget,
) -> (result: Result<Vec<u32>, TokenizerExecutionError>)
    requires
        old(work).valid(),
        has_adjacent_pair(symbols@, selection.left, selection.right),
    ensures
        final(work).valid(),
        result.is_ok() ==> {
            &&& result.unwrap()@ == apply_merge_spec(
                symbols@,
                selection.left,
                selection.right,
                selection.result_id,
            )
            &&& result.unwrap()@.len() < symbols@.len()
        },
{
    proof {
        reveal(apply_merge_spec);
    }
    let mut merged = Vec::new();
    match merged.try_reserve(symbols.len()) {
        Ok(()) => {},
        Err(_) => return Err(TokenizerExecutionError::AllocationFailed),
    }
    let mut index = 0;
    assert(symbols@.subrange(0, symbols@.len() as int) == symbols@);
    assert(merged@ == Seq::<u32>::empty());
    while index < symbols.len()
        invariant
            index <= symbols@.len(),
            work.valid(),
            merged@ + apply_merge_spec(
                symbols@.subrange(index as int, symbols@.len() as int),
                selection.left,
                selection.right,
                selection.result_id,
            ) == apply_merge_spec(
                symbols@,
                selection.left,
                selection.right,
                selection.result_id,
            ),
        decreases symbols.len() - index,
    {
        let ghost remaining = symbols@.subrange(index as int, symbols@.len() as int);
        match work.charge(1) {
            Ok(()) => {},
            Err(error) => return Err(error),
        }
        if index + 1 < symbols.len()
            && symbols[index] == selection.left
            && symbols[index + 1] == selection.right
        {
            assert(remaining.len() >= 2);
            assert(remaining[0] == selection.left);
            assert(remaining[1] == selection.right);
            merged.push(selection.result_id);
            index += 2;
            assert(remaining.subrange(2, remaining.len() as int)
                == symbols@.subrange(index as int, symbols@.len() as int));
        } else {
            assert(remaining.len() > 0);
            assert(!(remaining.len() >= 2
                && remaining[0] == selection.left
                && remaining[1] == selection.right));
            merged.push(symbols[index]);
            index += 1;
            assert(remaining.subrange(1, remaining.len() as int)
                == symbols@.subrange(index as int, symbols@.len() as int));
        }
    }
    proof {
        apply_merge_spec_length(
            symbols@,
            selection.left,
            selection.right,
            selection.result_id,
        );
    }
    Ok(merged)
}

closed spec fn bpe_step(
    before: Seq<u32>,
    after: Seq<u32>,
    merge_ranks: Map<(u32, u32), (usize, u32)>,
) -> bool {
    match selected_merge(before, merge_ranks) {
        Some(selection) => {
            &&& has_adjacent_pair(before, selection.left, selection.right)
            &&& after == apply_merge_spec(
                before,
                selection.left,
                selection.right,
                selection.result_id,
            )
            &&& after.len() < before.len()
        },
        None => false,
    }
}

closed spec fn bpe_trace_valid(
    trace: Seq<Seq<u32>>,
    merge_ranks: Map<(u32, u32), (usize, u32)>,
) -> bool {
    &&& trace.len() > 0
    &&& forall|index: int| #![trigger trace[index]] 0 <= index && index + 1 < trace.len()
        ==> bpe_step(trace[index], trace[index + 1], merge_ranks)
}

closed spec fn bpe_output(
    input: Seq<u32>,
    output: Seq<u32>,
    merge_ranks: Map<(u32, u32), (usize, u32)>,
) -> bool {
    &&& selected_merge(output, merge_ranks).is_none()
    &&& exists|trace: Seq<Seq<u32>>| {
        &&& bpe_trace_valid(trace, merge_ranks)
        &&& trace[0] == input
        &&& trace[trace.len() - 1] == output
    }
}

fn run_bpe(
    initial: &[u32],
    merge_ranks: &BTreeMap<(u32, u32), (usize, u32)>,
    work: &mut WorkBudget,
) -> (result: Result<Vec<u32>, TokenizerExecutionError>)
    requires old(work).valid(),
    ensures
        final(work).valid(),
        result.is_ok() ==> bpe_output(initial@, result.unwrap()@, merge_ranks@),
{
    proof {
        reveal(bpe_output);
    }
    let ghost original = initial@;
    let mut symbols = Vec::new();
    match symbols.try_reserve(initial.len()) {
        Ok(()) => {},
        Err(_) => return Err(TokenizerExecutionError::AllocationFailed),
    }
    let mut initial_index = 0;
    while initial_index < initial.len()
        invariant
            initial_index <= initial@.len(),
            original == initial@,
            symbols@ == initial@.subrange(0, initial_index as int),
            work.valid(),
        decreases initial.len() - initial_index,
    {
        symbols.push(initial[initial_index]);
        initial_index += 1;
    }
    assert(symbols@ == initial@);
    let ghost mut trace = seq![symbols@];
    while symbols.len() > 1
        invariant
            work.valid(),
            original == initial@,
            trace.len() > 0,
            trace[0] == original,
            trace[trace.len() - 1] == symbols@,
            bpe_trace_valid(trace, merge_ranks@),
        decreases symbols.len(),
    {
        let selection = select_merge(&symbols, merge_ranks, work)?;
        let Some(selection) = selection else {
            assert(selected_merge(symbols@, merge_ranks@).is_none());
            assert(exists|candidate_trace: Seq<Seq<u32>>| {
                &&& bpe_trace_valid(candidate_trace, merge_ranks@)
                &&& candidate_trace[0] == original
                &&& candidate_trace[candidate_trace.len() - 1] == symbols@
            }) by {
                assert(bpe_trace_valid(trace, merge_ranks@));
                assert(trace[0] == original);
            }
            assert(bpe_output(initial@, symbols@, merge_ranks@));
            return Ok(symbols);
        };
        proof {
            selected_merge_has_witness(symbols@, merge_ranks@);
        }
        let ghost before = symbols@;
        symbols = apply_merge(&symbols, selection, work)?;
        proof {
            assert(bpe_step(before, symbols@, merge_ranks@));
            trace = trace.push(symbols@);
        }
    }
    assert(selected_merge(symbols@, merge_ranks@).is_none());
    assert(exists|candidate_trace: Seq<Seq<u32>>| {
        &&& bpe_trace_valid(candidate_trace, merge_ranks@)
        &&& candidate_trace[0] == original
        &&& candidate_trace[candidate_trace.len() - 1] == symbols@
    }) by {
        assert(bpe_trace_valid(trace, merge_ranks@));
        assert(trace[0] == original);
    }
    assert(bpe_output(initial@, symbols@, merge_ranks@));
    Ok(symbols)
}

} // verus!

verus! {

closed spec fn byte_to_unicode_codepoint_spec(byte: u8) -> u32 {
    if 0x21 <= byte <= 0x7e || 0xa1 <= byte <= 0xac || 0xae <= byte {
        byte as u32
    } else if byte <= 0x20 {
        (256 + byte as int) as u32
    } else if 0x7f <= byte <= 0xa0 {
        (byte as int + 162) as u32
    } else {
        323
    }
}

fn byte_to_unicode_codepoint(byte: u8) -> (codepoint: u32)
    ensures
        codepoint == byte_to_unicode_codepoint_spec(byte),
        codepoint <= 323,
        !(0xd800 <= codepoint <= 0xdfff),
{
    if (0x21..=0x7e).contains(&byte)
        || (0xa1..=0xac).contains(&byte)
        || 0xae <= byte
    {
        u32::from(byte)
    } else if byte <= 0x20 {
        256_u32 + u32::from(byte)
    } else if (0x7F..=0xA0).contains(&byte) {
        u32::from(byte) + 162
    } else {
        323
    }
}

closed spec fn unicode_codepoint_to_byte_spec(codepoint: u32) -> Option<u8> {
    if codepoint <= 0xff
        && (0x21 <= codepoint <= 0x7e
            || 0xa1 <= codepoint <= 0xac
            || 0xae <= codepoint)
    {
        Some(codepoint as u8)
    } else if 256 <= codepoint <= 288 {
        Some((codepoint - 256) as u8)
    } else if 289 <= codepoint <= 322 {
        Some((codepoint - 162) as u8)
    } else if codepoint == 323 {
        Some(0xad)
    } else {
        None
    }
}

fn unicode_codepoint_to_byte(codepoint: u32) -> (byte: Option<u8>)
    ensures byte == unicode_codepoint_to_byte_spec(codepoint),
{
    if codepoint <= 0xff
        && ((0x21..=0x7e).contains(&codepoint)
            || (0xa1..=0xac).contains(&codepoint)
            || 0xae <= codepoint)
    {
        u8::try_from(codepoint).ok()
    } else if (256..=288).contains(&codepoint) {
        u8::try_from(codepoint - 256).ok()
    } else if (289..=322).contains(&codepoint) {
        u8::try_from(codepoint - 162).ok()
    } else if codepoint == 323 {
        Some(0xad)
    } else {
        None
    }
}

proof fn byte_level_codepoint_bijection(byte: u8)
    ensures
        unicode_codepoint_to_byte_spec(byte_to_unicode_codepoint_spec(byte)) == Some(byte),
{
}

} // verus!

fn byte_to_unicode(byte: u8) -> Option<char> {
    char::from_u32(byte_to_unicode_codepoint(byte))
}

fn unicode_to_byte(symbol: char) -> Option<u8> {
    unicode_codepoint_to_byte(u32::from(symbol))
}

verus! {

fn charge_ordered_lookup(
    work: &mut WorkBudget,
    left_bytes: usize,
    right_bytes: usize,
) -> (result: Result<(), TokenizerExecutionError>)
    requires old(work).valid(),
    ensures final(work).valid(),
{
    let pair_bytes = left_bytes
        .checked_add(right_bytes)
        .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
    let charged = pair_bytes
        .max(1)
        .checked_mul(ORDERED_LOOKUP_COMPARISONS)
        .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
    work.charge(charged)
}

fn push_token(
    output: &mut Vec<u32>,
    token: u32,
    limit: usize,
) -> (result: Result<(), TokenizerExecutionError>)
    ensures
        match result {
            Ok(()) => {
                &&& final(output)@ == old(output)@.push(token)
                &&& final(output)@.len() <= limit
            },
            Err(_) => final(output)@ == old(output)@,
        },
{
    let actual = output
        .len()
        .checked_add(1)
        .ok_or(TokenizerExecutionError::TokenLimit {
            limit,
            actual: usize::MAX,
        })?;
    if actual > limit {
        return Err(TokenizerExecutionError::TokenLimit { limit, actual });
    }
    output.push(token);
    Ok(())
}

closed spec fn added_token_allowed(
    added: AddedTokenProgram,
    policy: SpecialTokenEncodePolicy,
) -> bool {
    match policy {
        SpecialTokenEncodePolicy::Reject => !added.special,
        SpecialTokenEncodePolicy::Allow => true,
    }
}

fn emit_added_token(
    added: &AddedTokenProgram,
    policy: SpecialTokenEncodePolicy,
    output: &mut Vec<u32>,
    limit: usize,
) -> (result: Result<usize, TokenizerExecutionError>)
    ensures
        !added_token_allowed(*added, policy) ==> {
            &&& result == Err(TokenizerExecutionError::SpecialTokenForbidden {
                token_id: added.token_id,
            })
            &&& final(output)@ == old(output)@
        },
        match result {
            Ok(length) => {
                &&& added_token_allowed(*added, policy)
                &&& length == added.bytes@.len()
                &&& final(output)@ == old(output)@.push(added.token_id)
                &&& final(output)@.len() <= limit
            },
            Err(_) => final(output)@ == old(output)@,
        },
{
    proof {
        reveal(added_token_allowed);
    }
    match policy {
        SpecialTokenEncodePolicy::Reject => {
            if added.special {
                return Err(TokenizerExecutionError::SpecialTokenForbidden {
                    token_id: added.token_id,
                });
            }
        },
        SpecialTokenEncodePolicy::Allow => {},
    }
    assert(added_token_allowed(*added, policy));
    match push_token(output, added.token_id, limit) {
        Ok(()) => Ok(added.bytes.len()),
        Err(error) => Err(error),
    }
}

fn push_byte(
    output: &mut Vec<u8>,
    byte: u8,
    limit: usize,
) -> (result: Result<(), TokenizerExecutionError>)
    ensures
        match result {
            Ok(()) => {
                &&& final(output)@ == old(output)@.push(byte)
                &&& final(output)@.len() <= limit
            },
            Err(_) => final(output)@ == old(output)@,
        },
{
    if output.len() >= limit {
        return Err(TokenizerExecutionError::OutputByteLimit { limit });
    }
    output.push(byte);
    Ok(())
}

struct WorkBudget {
    used: usize,
    limit: usize,
}

impl WorkBudget {
    closed spec fn valid(&self) -> bool {
        self.used <= self.limit
    }

    const fn new(limit: usize) -> (budget: Self)
        ensures
            budget.valid(),
            budget.used == 0,
            budget.limit == limit,
    {
        Self { used: 0, limit }
    }

    fn charge(&mut self, amount: usize) -> (result: Result<(), TokenizerExecutionError>)
        requires old(self).valid(),
        ensures
            final(self).valid(),
            final(self).limit == old(self).limit,
            match result {
                Ok(()) => {
                    &&& final(self).used as nat == old(self).used as nat + amount as nat
                    &&& final(self).used <= final(self).limit
                },
                Err(error) => {
                    &&& final(self).used == old(self).used
                    &&& error == TokenizerExecutionError::WorkLimit { limit: old(self).limit }
                },
            },
    {
        let used = self
            .used
            .checked_add(amount)
            .ok_or(TokenizerExecutionError::WorkLimit { limit: self.limit })?;
        if used > self.limit {
            return Err(TokenizerExecutionError::WorkLimit { limit: self.limit });
        }
        self.used = used;
        Ok(())
    }
}

} // verus!

verus! {

closed spec fn byte_token_sequence(piece: Seq<u8>, byte_token_ids: Seq<u32>) -> Seq<u32> {
    Seq::new(piece.len(), |index: int| byte_token_ids[piece[index] as int])
}

pub(super) closed spec fn decoded_token_bytes(
    vocabulary_bytes: Seq<Vec<u8>>,
    added_tokens: Seq<AddedTokenProgram>,
    token_id: u32,
    special_tokens: SpecialTokenDecodePolicy,
) -> Option<Seq<u8>> {
    let index = token_id as int;
    if index < BASE_VOCABULARY_SIZE as int {
        if index < vocabulary_bytes.len() {
            Some(vocabulary_bytes[index]@)
        } else {
            None
        }
    } else {
        let offset = index - BASE_VOCABULARY_SIZE as int;
        if offset < added_tokens.len() {
            let added = added_tokens[offset];
            match special_tokens {
                SpecialTokenDecodePolicy::Preserve => Some(added.bytes@),
                SpecialTokenDecodePolicy::Skip => {
                    if added.special {
                        Some(Seq::empty())
                    } else {
                        Some(added.bytes@)
                    }
                },
            }
        } else {
            None
        }
    }
}

pub(super) closed spec fn decode_from(
    prefix: Seq<u8>,
    vocabulary_bytes: Seq<Vec<u8>>,
    added_tokens: Seq<AddedTokenProgram>,
    token_ids: Seq<u32>,
    special_tokens: SpecialTokenDecodePolicy,
) -> Option<Seq<u8>>
    decreases token_ids.len(),
{
    if token_ids.len() == 0 {
        Some(prefix)
    } else {
        match decoded_token_bytes(
            vocabulary_bytes,
            added_tokens,
            token_ids[0],
            special_tokens,
        ) {
            Some(bytes) => decode_from(
                prefix + bytes,
                vocabulary_bytes,
                added_tokens,
                token_ids.subrange(1, token_ids.len() as int),
                special_tokens,
            ),
            None => None,
        }
    }
}

fn append_decoded_bytes(
    bytes: &[u8],
    work: &mut WorkBudget,
    output: &mut Vec<u8>,
    limit: usize,
) -> (result: Result<(), TokenizerExecutionError>)
    requires
        old(work).valid(),
        old(output)@.len() <= limit,
    ensures
        final(work).valid(),
        result.is_ok() ==> {
            &&& final(output)@ == old(output)@ + bytes@
            &&& final(output)@.len() <= limit
        },
{
    let ghost initial_output = output@;
    let mut index = 0;
    while index < bytes.len()
        invariant
            index <= bytes@.len(),
            work.valid(),
            output@ == initial_output + bytes@.subrange(0, index as int),
            output@.len() <= limit,
        decreases bytes.len() - index,
    {
        match work.charge(1) {
            Ok(()) => {},
            Err(error) => return Err(error),
        }
        match push_byte(output, bytes[index], limit) {
            Ok(()) => {},
            Err(error) => return Err(error),
        }
        index += 1;
    }
    Ok(())
}

impl TokenizerExecutionProgram {
    fn encode_piece(
        &self,
        piece: &[u8],
        limits: TokenizerExecutionLimits,
        work: &mut WorkBudget,
        output: &mut Vec<u32>,
    ) -> (result: Result<(), TokenizerExecutionError>)
        requires
            old(work).valid(),
            old(output)@.len() <= limits.tokens,
        ensures
            final(work).valid(),
            result.is_ok() ==> exists|encoded: Seq<u32>| {
                &&& bpe_output(
                    byte_token_sequence(piece@, self.byte_token_ids@),
                    encoded,
                    self.merge_ranks@,
                )
                &&& final(output)@ == old(output)@ + encoded
                &&& final(output)@.len() <= limits.tokens
            },
    {
        let ghost initial_output = output@;
        let mut symbols = Vec::new();
        match symbols.try_reserve(piece.len()) {
            Ok(()) => {},
            Err(_) => return Err(TokenizerExecutionError::AllocationFailed),
        }
        let mut index = 0;
        while index < piece.len()
            invariant
                index <= piece@.len(),
                work.valid(),
                symbols@ == Seq::new(
                    index as nat,
                    |offset: int| self.byte_token_ids@[piece@[offset] as int],
                ),
            decreases piece.len() - index,
        {
            match work.charge(1) {
                Ok(()) => {},
                Err(error) => return Err(error),
            }
            symbols.push(self.byte_token_ids[usize::from(piece[index])]);
            index += 1;
        }
        assert(symbols@ == byte_token_sequence(piece@, self.byte_token_ids@));

        let merged = run_bpe(&symbols, &self.merge_ranks, work)?;
        let ghost encoded = merged@;
        let mut merged_index = 0;
        while merged_index < merged.len()
            invariant
                merged_index <= merged@.len(),
                work.valid(),
                output@ == initial_output
                    + merged@.subrange(0, merged_index as int),
                output@.len() <= limits.tokens,
            decreases merged.len() - merged_index,
        {
            match push_token(output, merged[merged_index], limits.tokens) {
                Ok(()) => {},
                Err(error) => return Err(error),
            }
            merged_index += 1;
        }
        assert(output@ == initial_output + encoded);
        Ok(())
    }

    pub(super) fn decode_to_bytes(
        &self,
        token_ids: &[u32],
        limits: TokenizerExecutionLimits,
        special_tokens: SpecialTokenDecodePolicy,
    ) -> (result: Result<Vec<u8>, TokenizerExecutionError>)
        ensures result.is_ok() ==> {
            &&& decode_from(
                Seq::empty(),
                self.vocabulary_bytes@,
                self.added_tokens@,
                token_ids@,
                special_tokens,
            ) == Some(result.unwrap()@)
            &&& result.unwrap()@.len() <= limits.output_bytes
        },
    {
        match limits.validate() {
            Ok(()) => {},
            Err(error) => return Err(error),
        }
        if token_ids.len() > limits.tokens {
            return Err(TokenizerExecutionError::TokenLimit {
                limit: limits.tokens,
                actual: token_ids.len(),
            });
        }
        let mut output = Vec::new();
        match output.try_reserve(token_ids.len().min(limits.output_bytes)) {
            Ok(()) => {},
            Err(_) => return Err(TokenizerExecutionError::AllocationFailed),
        }
        let mut work = WorkBudget::new(limits.work);
        let ghost expected = decode_from(
            Seq::empty(),
            self.vocabulary_bytes@,
            self.added_tokens@,
            token_ids@,
            special_tokens,
        );
        let mut token_index = 0;
        assert(output@ == Seq::<u8>::empty());
        assert(token_ids@.subrange(0, token_ids@.len() as int) == token_ids@);
        while token_index < token_ids.len()
            invariant
                token_index <= token_ids@.len(),
                work.valid(),
                output@.len() <= limits.output_bytes,
                decode_from(
                    output@,
                    self.vocabulary_bytes@,
                    self.added_tokens@,
                    token_ids@.subrange(token_index as int, token_ids@.len() as int),
                    special_tokens,
                ) == expected,
            decreases token_ids.len() - token_index,
        {
            proof {
                reveal(decode_from);
            }
            let ghost prior_output = output@;
            let ghost remaining = token_ids@
                .subrange(token_index as int, token_ids@.len() as int);
            assert(remaining.len() > 0);
            assert(remaining[0] == token_ids@[token_index as int]);
            match work.charge(1) {
                Ok(()) => {},
                Err(error) => return Err(error),
            }
            let token_id = token_ids[token_index];
            assert(remaining[0] == token_id);
            let index = match usize::try_from(token_id) {
                Ok(index) => index,
                Err(_) => return Err(TokenizerExecutionError::UnknownTokenId(token_id)),
            };
            assert(index as int == token_id as int);
            if index < BASE_VOCABULARY_SIZE {
                let bytes = match self.vocabulary_bytes.get(index) {
                    Some(bytes) => bytes,
                    None => return Err(TokenizerExecutionError::UnknownTokenId(token_id)),
                };
                assert(decoded_token_bytes(
                    self.vocabulary_bytes@,
                    self.added_tokens@,
                    token_id,
                    special_tokens,
                ) == Some(bytes@));
                match append_decoded_bytes(bytes, &mut work, &mut output, limits.output_bytes) {
                    Ok(()) => {},
                    Err(error) => return Err(error),
                }
            } else {
                assert(token_id as int >= BASE_VOCABULARY_SIZE as int);
                let offset = index - BASE_VOCABULARY_SIZE;
                assert(offset as int
                    == token_id as int - BASE_VOCABULARY_SIZE as int);
                let added = match self.added_tokens.get(offset) {
                    Some(added) => added,
                    None => return Err(TokenizerExecutionError::UnknownTokenId(token_id)),
                };
                assert(0 <= (offset as int) < self.added_tokens@.len());
                assert(0 <= (token_id as int - BASE_VOCABULARY_SIZE as int));
                assert((token_id as int - BASE_VOCABULARY_SIZE as int)
                    < self.added_tokens@.len());
                assert(added.special == self.added_tokens@[offset as int].special);
                assert(added.bytes@ == self.added_tokens@[offset as int].bytes@);
                assert(self.added_tokens@[
                    token_id as int - BASE_VOCABULARY_SIZE as int
                ].special == added.special);
                assert(self.added_tokens@[
                    token_id as int - BASE_VOCABULARY_SIZE as int
                ].bytes@ == added.bytes@);
                proof {
                    reveal(decoded_token_bytes);
                }
                let skip = match special_tokens {
                    SpecialTokenDecodePolicy::Preserve => false,
                    SpecialTokenDecodePolicy::Skip => added.special,
                };
                if skip {
                    assert(decoded_token_bytes(
                        self.vocabulary_bytes@,
                        self.added_tokens@,
                        token_id,
                        special_tokens,
                    ) == Some(Seq::<u8>::empty()));
                } else {
                    assert(decoded_token_bytes(
                        self.vocabulary_bytes@,
                        self.added_tokens@,
                        token_id,
                        special_tokens,
                    ) == Some(added.bytes@));
                    match append_decoded_bytes(
                        &added.bytes,
                        &mut work,
                        &mut output,
                        limits.output_bytes,
                    ) {
                        Ok(()) => {},
                        Err(error) => return Err(error),
                    }
                }
            }
            token_index += 1;
            assert(remaining.subrange(1, remaining.len() as int) == token_ids@.subrange(
                token_index as int,
                token_ids@.len() as int,
            ));
            assert(decode_from(
                prior_output,
                self.vocabulary_bytes@,
                self.added_tokens@,
                remaining,
                special_tokens,
            ) == decode_from(
                output@,
                self.vocabulary_bytes@,
                self.added_tokens@,
                token_ids@.subrange(token_index as int, token_ids@.len() as int),
                special_tokens,
            ));
        }
        assert(decode_from(
            output@,
            self.vocabulary_bytes@,
            self.added_tokens@,
            Seq::empty(),
            special_tokens,
        ) == Some(output@));
        Ok(output)
    }
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{
        byte_to_unicode, find_added_token, reject_forbidden_special, run_bpe, unicode_to_byte,
        AddedTokenProgram, SpecialTokenDecodePolicy, SpecialTokenEncodePolicy,
        TokenizerExecutionError, TokenizerExecutionLimits, TokenizerExecutionProgram, WorkBudget,
        MAX_TOKENIZER_INPUT_BYTES, MAX_TOKENIZER_WORK,
    };
    use crate::tokenizer::tests::test_tokenizer;
    use crate::ADDED_TOKENS;
    use ferric_spec::Qwen3ModelRole;
    use std::collections::BTreeMap;

    const DIFFERENTIAL_CORPUS: &str =
        include_str!("fixtures/tokenizer/qwen3-tokenizer-differential.txt");

    fn tokenizer() -> crate::AuthenticatedTokenizer {
        test_tokenizer(Qwen3ModelRole::Target8B)
    }

    fn tight_limits(
        input_bytes: usize,
        tokens: usize,
        output_bytes: usize,
        work: usize,
    ) -> TokenizerExecutionLimits {
        TokenizerExecutionLimits {
            input_bytes,
            tokens,
            output_bytes,
            work,
        }
    }

    #[test]
    fn pinned_ascii_and_unicode_encode_fixtures_are_exact_and_deterministic() {
        let tokenizer = tokenizer();
        let fixtures: &[(&str, &[u32])] = &[
            ("", &[]),
            ("hello", &[14_990]),
            ("Hello world", &[9_707, 1_879]),
            ("hello   world", &[14_990, 256, 1_879]),
            ("I'm testing!", &[40, 2_776, 7_497, 0]),
            ("1 23\nnext", &[16, 220, 17, 18, 198, 3_600]),
            ("tabs\twork", &[30_993, 97_038]),
            ("line\r\nend", &[1_056, 319, 408]),
            (" punctuation!?", &[61_503, 57_390]),
            ("A.B", &[32, 1_785]),
            ("trailing   ", &[376, 14_277, 262]),
            ("I'RE", &[40, 94_153]),
            ("\0\u{7f}", &[188, 221]),
            ("e\u{301}", &[963]),
            ("\u{e9}", &[963]),
            ("\u{4e2d}\u{6587}", &[104_811]),
            ("\u{661}\u{662}", &[149, 94, 149, 95]),
        ];
        for (input, expected) in fixtures {
            let first = tokenizer
                .encode(
                    input,
                    TokenizerExecutionLimits::m1(),
                    SpecialTokenEncodePolicy::Reject,
                )
                .expect("pinned tokenizer fixture");
            let second = tokenizer
                .encode(
                    input,
                    TokenizerExecutionLimits::m1(),
                    SpecialTokenEncodePolicy::Reject,
                )
                .expect("repeat pinned tokenizer fixture");
            assert_eq!(&first, expected, "fixture {input:?}");
            assert_eq!(second, first, "determinism {input:?}");
        }
    }

    #[test]
    fn pinned_ascii_decode_fixtures_return_exact_bytes() {
        let tokenizer = tokenizer();
        for (ids, expected) in [
            (&[14_990_u32][..], &b"hello"[..]),
            (&[9_707, 1_879][..], &b"Hello world"[..]),
            (&[40, 2_776, 7_497, 0][..], &b"I'm testing!"[..]),
            (&[16, 220, 17, 18, 198, 3_600][..], &b"1 23\nnext"[..]),
            (&[30_993, 97_038][..], &b"tabs\twork"[..]),
            (&[1_056, 319, 408][..], &b"line\r\nend"[..]),
            (&[188, 222, 187][..], &[0, 128, 255][..]),
        ] {
            assert_eq!(
                tokenizer
                    .decode_to_bytes(
                        ids,
                        TokenizerExecutionLimits::m1(),
                        SpecialTokenDecodePolicy::Preserve,
                    )
                    .expect("pinned decode fixture"),
                expected
            );
        }
    }

    #[test]
    fn added_and_special_token_policy_is_exact() {
        let tokenizer = tokenizer();
        assert_eq!(
            tokenizer
                .encode(
                    "<think>ok</think>",
                    TokenizerExecutionLimits::m1(),
                    SpecialTokenEncodePolicy::Reject,
                )
                .expect("non-special added tokens remain recognized"),
            vec![151_667, 562, 151_668]
        );

        assert_eq!(
            tokenizer
                .encode(
                    "<|im_start|>user\nHello<|im_end|>",
                    TokenizerExecutionLimits::m1(),
                    SpecialTokenEncodePolicy::Reject,
                )
                .expect_err("special token must be explicit"),
            TokenizerExecutionError::SpecialTokenForbidden { token_id: 151_644 }
        );
        assert_eq!(
            tokenizer
                .encode(
                    "\u{e9} prefix <|im_end|>",
                    TokenizerExecutionLimits::m1(),
                    SpecialTokenEncodePolicy::Reject,
                )
                .expect_err("special token is rejected before Unicode execution"),
            TokenizerExecutionError::SpecialTokenForbidden { token_id: 151_645 }
        );
        let ids = tokenizer
            .encode(
                "<|im_start|>user\nHello<|im_end|>",
                TokenizerExecutionLimits::m1(),
                SpecialTokenEncodePolicy::Allow,
            )
            .expect("special tokens explicitly enabled");
        assert_eq!(ids, vec![151_644, 872, 198, 9_707, 151_645]);
        assert_eq!(
            tokenizer
                .decode_to_bytes(
                    &ids,
                    TokenizerExecutionLimits::m1(),
                    SpecialTokenDecodePolicy::Preserve,
                )
                .expect("preserve special tokens"),
            b"<|im_start|>user\nHello<|im_end|>"
        );
        assert_eq!(
            tokenizer
                .decode_to_bytes(
                    &ids,
                    TokenizerExecutionLimits::m1(),
                    SpecialTokenDecodePolicy::Skip,
                )
                .expect("skip special tokens"),
            b"user\nHello"
        );

        for (offset, (content, special)) in ADDED_TOKENS.iter().enumerate() {
            let id = 151_643_u32
                .checked_add(u32::try_from(offset).expect("added token offset fits u32"))
                .expect("added token ID fits u32");
            let encoded = tokenizer.encode(
                content,
                TokenizerExecutionLimits::m1(),
                SpecialTokenEncodePolicy::Reject,
            );
            if *special {
                assert_eq!(
                    encoded.expect_err("every special token is disabled by policy"),
                    TokenizerExecutionError::SpecialTokenForbidden { token_id: id }
                );
            } else {
                assert_eq!(
                    encoded.expect("every non-special added token remains enabled"),
                    vec![id]
                );
            }
            assert_eq!(
                tokenizer
                    .encode(
                        content,
                        TokenizerExecutionLimits::m1(),
                        SpecialTokenEncodePolicy::Allow,
                    )
                    .expect("every added token has its exact ID"),
                vec![id]
            );
            assert_eq!(
                tokenizer
                    .decode_to_bytes(
                        &[id],
                        TokenizerExecutionLimits::m1(),
                        SpecialTokenDecodePolicy::Preserve,
                    )
                    .expect("every added token preserves exact bytes"),
                content.as_bytes()
            );
            let skipped = tokenizer
                .decode_to_bytes(
                    &[id],
                    TokenizerExecutionLimits::m1(),
                    SpecialTokenDecodePolicy::Skip,
                )
                .expect("every added token obeys its authenticated special bit");
            let expected: &[u8] = if *special { &[] } else { content.as_bytes() };
            assert_eq!(skipped, expected);
        }
    }

    #[test]
    fn independent_oracle_corpus_matches_the_production_path() {
        let tokenizer = tokenizer();
        let mut cases = 0_usize;
        for (line_number, line) in DIFFERENTIAL_CORPUS.lines().enumerate() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (input_hex, ids_text) = line
                .split_once('\t')
                .unwrap_or_else(|| panic!("malformed corpus line {}", line_number + 1));
            let input_bytes = if input_hex == "-" {
                Vec::new()
            } else {
                decode_hex(input_hex)
            };
            let input = std::str::from_utf8(&input_bytes).expect("corpus input is UTF-8");
            let expected: Vec<u32> = if ids_text == "-" {
                Vec::new()
            } else {
                ids_text
                    .split(',')
                    .map(|id| id.parse().expect("corpus token ID is u32"))
                    .collect()
            };
            let actual = tokenizer
                .encode(
                    input,
                    TokenizerExecutionLimits::m1(),
                    SpecialTokenEncodePolicy::Allow,
                )
                .unwrap_or_else(|error| panic!("corpus line {} failed: {error}", line_number + 1));
            assert_eq!(actual, expected, "corpus input {input:?}");
            cases = cases.checked_add(1).expect("corpus case count fits usize");
        }
        assert!(cases >= 640, "differential corpus must remain substantial");
    }

    #[test]
    fn byte_level_alphabet_is_an_exact_bijection() {
        for byte in u8::MIN..=u8::MAX {
            let symbol = byte_to_unicode(byte).expect("all byte-level codepoints are valid");
            assert_eq!(unicode_to_byte(symbol), Some(byte));
        }
        assert_eq!(unicode_to_byte('€'), None);
    }

    #[test]
    fn hostile_bpe_rank_ties_and_simultaneous_merges_are_exact() {
        let mut ranked = BTreeMap::new();
        ranked.insert((1, 2), (10, 4));
        ranked.insert((2, 3), (1, 5));
        let mut work = WorkBudget::new(MAX_TOKENIZER_WORK);
        assert_eq!(
            run_bpe(&[1, 2, 3], &ranked, &mut work).expect("lower rank is selected"),
            vec![1, 5]
        );

        let mut tied = BTreeMap::new();
        tied.insert((1, 2), (1, 4));
        tied.insert((2, 3), (1, 5));
        let mut work = WorkBudget::new(MAX_TOKENIZER_WORK);
        assert_eq!(
            run_bpe(&[1, 2, 3], &tied, &mut work).expect("rank tie keeps earliest pair"),
            vec![4, 3]
        );

        let mut repeated = BTreeMap::new();
        repeated.insert((1, 2), (0, 9));
        let mut work = WorkBudget::new(MAX_TOKENIZER_WORK);
        assert_eq!(
            run_bpe(&[1, 2, 1, 2, 1], &repeated, &mut work)
                .expect("one BPE step merges every nonoverlapping pair"),
            vec![9, 9, 1]
        );

        let mut exhausted = WorkBudget::new(1);
        assert_eq!(
            run_bpe(&[1, 2], &repeated, &mut exhausted).expect_err("lookup is fully charged"),
            TokenizerExecutionError::WorkLimit { limit: 1 }
        );
        assert_eq!(exhausted.used, 0, "failed charge does not mutate budget");
    }

    #[test]
    fn hostile_added_token_overlap_uses_earliest_longest_lowest_offset() {
        let added = vec![
            AddedTokenProgram {
                bytes: b"<ab".to_vec(),
                special: false,
                token_id: 10,
            },
            AddedTokenProgram {
                bytes: b"<abc".to_vec(),
                special: false,
                token_id: 11,
            },
            AddedTokenProgram {
                bytes: b"<abc".to_vec(),
                special: false,
                token_id: 12,
            },
            AddedTokenProgram {
                bytes: b"<z".to_vec(),
                special: true,
                token_id: 13,
            },
        ];
        let input = b"x<abc<z";
        let mut work = WorkBudget::new(MAX_TOKENIZER_WORK);
        let first = find_added_token(input, 0, &added, &mut work)
            .expect("search fits work bound")
            .expect("overlap is found");
        assert_eq!((first.start, first.offset, first.token_id), (1, 1, 11));

        let mut work = WorkBudget::new(MAX_TOKENIZER_WORK);
        let second = find_added_token(input, 2, &added, &mut work)
            .expect("suffix search fits work bound")
            .expect("later special token is found");
        assert_eq!((second.start, second.offset, second.token_id), (5, 3, 13));

        let mut work = WorkBudget::new(MAX_TOKENIZER_WORK);
        assert_eq!(
            reject_forbidden_special(input, &added, &mut work)
                .expect_err("special match is rejected"),
            TokenizerExecutionError::SpecialTokenForbidden { token_id: 13 }
        );
    }

    #[test]
    fn hostile_numeric_decode_preserves_bytes_skips_only_special_and_rejects_holes() {
        const BASE_ID: u32 = 151_643;
        let program = TokenizerExecutionProgram {
            byte_token_ids: [0; 256],
            vocabulary_bytes: vec![vec![0, 0xff]],
            merge_ranks: BTreeMap::new(),
            added_tokens: vec![
                AddedTokenProgram {
                    bytes: b"<s>".to_vec(),
                    special: true,
                    token_id: BASE_ID,
                },
                AddedTokenProgram {
                    bytes: b"plain".to_vec(),
                    special: false,
                    token_id: BASE_ID + 1,
                },
            ],
        };
        let ids = [0, BASE_ID, BASE_ID + 1];
        let limits = tight_limits(1, 4, 32, 1_024);
        assert_eq!(
            program
                .decode_to_bytes(&ids, limits, SpecialTokenDecodePolicy::Preserve)
                .expect("preserve exact bytes"),
            b"\0\xff<s>plain"
        );
        assert_eq!(
            program
                .decode_to_bytes(&ids, limits, SpecialTokenDecodePolicy::Skip)
                .expect("skip only authenticated special bit"),
            b"\0\xffplain"
        );
        assert_eq!(
            program
                .decode_to_bytes(&[1], limits, SpecialTokenDecodePolicy::Preserve)
                .expect_err("base vocabulary hole is rejected"),
            TokenizerExecutionError::UnknownTokenId(1)
        );
        let unknown_added = BASE_ID + 2;
        assert_eq!(
            program
                .decode_to_bytes(&[unknown_added], limits, SpecialTokenDecodePolicy::Preserve,)
                .expect_err("added vocabulary hole is rejected"),
            TokenizerExecutionError::UnknownTokenId(unknown_added)
        );
    }

    #[test]
    fn every_execution_bound_fails_closed() {
        let tokenizer = tokenizer();
        assert_eq!(
            tokenizer
                .encode(
                    "hello",
                    tight_limits(4, 8, 8, 1_024),
                    SpecialTokenEncodePolicy::Reject,
                )
                .expect_err("input bound"),
            TokenizerExecutionError::InputTooLarge {
                limit: 4,
                actual: 5,
            }
        );
        assert_eq!(
            tokenizer
                .encode(
                    "hello world",
                    tight_limits(32, 1, 32, MAX_TOKENIZER_WORK),
                    SpecialTokenEncodePolicy::Reject,
                )
                .expect_err("token output bound"),
            TokenizerExecutionError::TokenLimit {
                limit: 1,
                actual: 2,
            }
        );
        assert_eq!(
            tokenizer
                .encode(
                    "hello",
                    tight_limits(32, 8, 32, 1),
                    SpecialTokenEncodePolicy::Reject,
                )
                .expect_err("work bound"),
            TokenizerExecutionError::WorkLimit { limit: 1 }
        );
        assert_eq!(
            tokenizer
                .decode_to_bytes(
                    &[14_990],
                    tight_limits(32, 8, 4, 1_024),
                    SpecialTokenDecodePolicy::Preserve,
                )
                .expect_err("decoded byte bound"),
            TokenizerExecutionError::OutputByteLimit { limit: 4 }
        );
        assert_eq!(
            tokenizer
                .decode_to_bytes(
                    &[151_669],
                    TokenizerExecutionLimits::m1(),
                    SpecialTokenDecodePolicy::Preserve,
                )
                .expect_err("unknown token ID"),
            TokenizerExecutionError::UnknownTokenId(151_669)
        );
        assert_eq!(
            tokenizer
                .encode(
                    "x",
                    tight_limits(MAX_TOKENIZER_INPUT_BYTES + 1, 1, 1, 1),
                    SpecialTokenEncodePolicy::Reject,
                )
                .expect_err("hard envelope cannot be raised"),
            TokenizerExecutionError::InvalidLimits
        );
    }

    fn decode_hex(input: &str) -> Vec<u8> {
        assert_eq!(input.len() % 2, 0, "hex input has complete bytes");
        (0..input.len())
            .step_by(2)
            .map(|offset| {
                u8::from_str_radix(&input[offset..offset + 2], 16)
                    .expect("corpus input contains lowercase hex")
            })
            .collect()
    }
}
