//! Bounded execution of the exact parsed Qwen3 tokenizer program.
//!
//! Encode is intentionally restricted to ASCII until Unicode normalization and
//! property-regex behavior have their own admitted implementation.

use crate::ADDED_TOKENS;
use std::collections::BTreeMap;
use std::fmt;

const BASE_VOCABULARY_SIZE: usize = 151_643;
const TOTAL_VOCABULARY_SIZE: usize = BASE_VOCABULARY_SIZE + ADDED_TOKENS.len();
const ORDERED_LOOKUP_COMPARISONS: usize = 128;

/// Hard upper bound for one tokenizer input.
pub const MAX_TOKENIZER_INPUT_BYTES: usize = 32 * 1_024;
/// Hard upper bound for tokens produced or consumed by one tokenizer call.
pub const MAX_TOKENIZER_OUTPUT_TOKENS: usize = 8_192;
/// Hard upper bound for bytes produced by one decode call.
pub const MAX_TOKENIZER_OUTPUT_BYTES: usize = 128 * 1_024;
/// Hard upper bound for charged finite tokenizer operations in one call.
pub const MAX_TOKENIZER_WORK: usize = 16 * 1_024 * 1_024;

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

impl TokenizerExecutionLimits {
    /// The closed M1 tokenizer execution envelope.
    #[must_use]
    pub const fn m1() -> Self {
        Self {
            input_bytes: MAX_TOKENIZER_INPUT_BYTES,
            tokens: MAX_TOKENIZER_OUTPUT_TOKENS,
            output_bytes: MAX_TOKENIZER_OUTPUT_BYTES,
            work: MAX_TOKENIZER_WORK,
        }
    }

    fn validate(self) -> Result<(), TokenizerExecutionError> {
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenizerExecutionError {
    /// At least one caller bound was zero or exceeded the closed M1 ceiling.
    InvalidLimits,
    /// Input bytes exceeded the caller-selected encode bound.
    InputTooLarge { limit: usize, actual: usize },
    /// Input requires Unicode normalization or property classification not yet admitted.
    UnsupportedUnicode { byte_offset: usize },
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
            Self::UnsupportedUnicode { byte_offset } => write!(
                formatter,
                "tokenizer input requires unsupported Unicode behavior at byte {byte_offset}"
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

#[derive(Debug, PartialEq, Eq)]
pub(super) struct TokenizerProgram {
    vocabulary: Vec<String>,
    token_ids: BTreeMap<String, u32>,
    merge_ranks: BTreeMap<String, BTreeMap<String, usize>>,
}

impl TokenizerProgram {
    pub(super) fn new(vocabulary: Vec<String>, merges: Vec<(String, String)>) -> Self {
        debug_assert_eq!(vocabulary.len(), BASE_VOCABULARY_SIZE);
        let token_ids = vocabulary
            .iter()
            .enumerate()
            .map(|(id, token)| {
                (
                    token.clone(),
                    u32::try_from(id).expect("pinned vocabulary ID fits u32"),
                )
            })
            .collect();
        let mut merge_ranks: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
        for (rank, (left, right)) in merges.into_iter().enumerate() {
            let prior = merge_ranks.entry(left).or_default().insert(right, rank);
            debug_assert!(prior.is_none(), "pinned merge pairs are unique");
        }
        Self {
            vocabulary,
            token_ids,
            merge_ranks,
        }
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
        if let Some((byte_offset, _)) = input.char_indices().find(|(_, symbol)| !symbol.is_ascii())
        {
            return Err(TokenizerExecutionError::UnsupportedUnicode { byte_offset });
        }

        let mut output = Vec::new();
        output
            .try_reserve(input.len().min(limits.tokens))
            .map_err(|_| TokenizerExecutionError::AllocationFailed)?;
        let mut work = WorkBudget::new(limits.work);
        let mut cursor = 0;
        while cursor < input.len() {
            let next = find_added_token(input, cursor, &mut work)?;
            let normal_end = next.map_or(input.len(), |match_| match_.start);
            self.encode_normal_ascii(&input[cursor..normal_end], limits, &mut work, &mut output)?;
            let Some(match_) = next else {
                break;
            };
            let (content, special) = ADDED_TOKENS[match_.offset];
            if special && special_tokens == SpecialTokenEncodePolicy::Reject {
                return Err(TokenizerExecutionError::SpecialTokenForbidden {
                    token_id: match_.token_id,
                });
            }
            push_token(&mut output, match_.token_id, limits.tokens)?;
            cursor = match_
                .start
                .checked_add(content.len())
                .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
        }
        Ok(output)
    }

    fn encode_normal_ascii(
        &self,
        input: &str,
        limits: TokenizerExecutionLimits,
        work: &mut WorkBudget,
        output: &mut Vec<u32>,
    ) -> Result<(), TokenizerExecutionError> {
        let mut cursor = 0;
        while cursor < input.len() {
            let end = next_ascii_split(input.as_bytes(), cursor, work)?;
            self.encode_piece(&input.as_bytes()[cursor..end], limits, work, output)?;
            cursor = end;
        }
        Ok(())
    }

    fn encode_piece(
        &self,
        piece: &[u8],
        limits: TokenizerExecutionLimits,
        work: &mut WorkBudget,
        output: &mut Vec<u32>,
    ) -> Result<(), TokenizerExecutionError> {
        let mut symbols = Vec::new();
        symbols
            .try_reserve_exact(piece.len())
            .map_err(|_| TokenizerExecutionError::AllocationFailed)?;
        for byte in piece {
            work.charge(1)?;
            symbols.push(
                byte_to_unicode(*byte)
                    .ok_or(TokenizerExecutionError::ArithmeticOverflow)?
                    .to_string(),
            );
        }

        while symbols.len() > 1 {
            let mut best: Option<(usize, &str, &str)> = None;
            for pair in symbols.windows(2) {
                charge_ordered_lookup(work, pair[0].len(), pair[1].len())?;
                if let Some(rank) = self
                    .merge_ranks
                    .get(pair[0].as_str())
                    .and_then(|rights| rights.get(pair[1].as_str()))
                {
                    if best.is_none_or(|(best_rank, _, _)| *rank < best_rank) {
                        best = Some((*rank, pair[0].as_str(), pair[1].as_str()));
                    }
                }
            }
            let Some((_, left, right)) = best else {
                break;
            };
            let left = left.to_owned();
            let right = right.to_owned();
            let mut merged = Vec::new();
            merged
                .try_reserve_exact(symbols.len())
                .map_err(|_| TokenizerExecutionError::AllocationFailed)?;
            let mut index = 0;
            while index < symbols.len() {
                work.charge(1)?;
                let next = index
                    .checked_add(1)
                    .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
                if next < symbols.len() && symbols[index] == left && symbols[next] == right {
                    let capacity = left
                        .len()
                        .checked_add(right.len())
                        .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
                    let mut combined = String::new();
                    combined
                        .try_reserve_exact(capacity)
                        .map_err(|_| TokenizerExecutionError::AllocationFailed)?;
                    combined.push_str(&left);
                    combined.push_str(&right);
                    merged.push(combined);
                    index = index
                        .checked_add(2)
                        .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
                } else {
                    merged.push(symbols[index].clone());
                    index = next;
                }
            }
            symbols = merged;
        }

        for symbol in symbols {
            charge_ordered_lookup(work, symbol.len(), 0)?;
            let id = self.token_ids.get(&symbol).copied().ok_or(
                TokenizerExecutionError::UnsupportedVocabularySymbol(
                    symbol.chars().next().unwrap_or('\0'),
                ),
            )?;
            push_token(output, id, limits.tokens)?;
        }
        Ok(())
    }

    pub(super) fn decode_to_bytes(
        &self,
        token_ids: &[u32],
        limits: TokenizerExecutionLimits,
        special_tokens: SpecialTokenDecodePolicy,
    ) -> Result<Vec<u8>, TokenizerExecutionError> {
        limits.validate()?;
        if token_ids.len() > limits.tokens {
            return Err(TokenizerExecutionError::TokenLimit {
                limit: limits.tokens,
                actual: token_ids.len(),
            });
        }
        let mut output = Vec::new();
        output
            .try_reserve(token_ids.len().min(limits.output_bytes))
            .map_err(|_| TokenizerExecutionError::AllocationFailed)?;
        let mut work = WorkBudget::new(limits.work);
        for id in token_ids {
            work.charge(1)?;
            let index =
                usize::try_from(*id).map_err(|_| TokenizerExecutionError::UnknownTokenId(*id))?;
            if index >= TOTAL_VOCABULARY_SIZE {
                return Err(TokenizerExecutionError::UnknownTokenId(*id));
            }
            if index < BASE_VOCABULARY_SIZE {
                for symbol in self.vocabulary[index].chars() {
                    work.charge(1)?;
                    let byte = unicode_to_byte(symbol)
                        .ok_or(TokenizerExecutionError::UnsupportedVocabularySymbol(symbol))?;
                    push_byte(&mut output, byte, limits.output_bytes)?;
                }
                continue;
            }
            let offset = index
                .checked_sub(BASE_VOCABULARY_SIZE)
                .ok_or(TokenizerExecutionError::UnknownTokenId(*id))?;
            let Some((content, special)) = ADDED_TOKENS.get(offset) else {
                return Err(TokenizerExecutionError::UnknownTokenId(*id));
            };
            if *special && special_tokens == SpecialTokenDecodePolicy::Skip {
                continue;
            }
            for byte in content.as_bytes() {
                work.charge(1)?;
                push_byte(&mut output, *byte, limits.output_bytes)?;
            }
        }
        Ok(output)
    }
}

#[derive(Clone, Copy)]
struct AddedTokenMatch {
    start: usize,
    offset: usize,
    token_id: u32,
}

fn find_added_token(
    input: &str,
    cursor: usize,
    work: &mut WorkBudget,
) -> Result<Option<AddedTokenMatch>, TokenizerExecutionError> {
    let mut best: Option<AddedTokenMatch> = None;
    let mut start = cursor;
    while start < input.len() {
        work.charge(1)?;
        if input.as_bytes()[start] == b'<' {
            for (offset, (content, _)) in ADDED_TOKENS.iter().enumerate() {
                work.charge(content.len())?;
                if !input[start..].starts_with(content) {
                    continue;
                }
                let token_id = u32::try_from(BASE_VOCABULARY_SIZE)
                    .ok()
                    .and_then(|base| base.checked_add(u32::try_from(offset).ok()?))
                    .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
                let candidate = AddedTokenMatch {
                    start,
                    offset,
                    token_id,
                };
                if best.is_none_or(|current| {
                    content.len() > ADDED_TOKENS[current.offset].0.len()
                        || (content.len() == ADDED_TOKENS[current.offset].0.len()
                            && offset < current.offset)
                }) {
                    best = Some(candidate);
                }
            }
            if best.is_some() {
                return Ok(best);
            }
        }
        start = start
            .checked_add(1)
            .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
    }
    Ok(None)
}

fn next_ascii_split(
    bytes: &[u8],
    start: usize,
    work: &mut WorkBudget,
) -> Result<usize, TokenizerExecutionError> {
    work.charge(1)?;
    if bytes[start] == b'\'' {
        for contraction in [b"re".as_slice(), b"ve", b"ll", b"s", b"t", b"m", b"d"] {
            work.charge(contraction.len())?;
            let contraction_bytes = 1_usize
                .checked_add(contraction.len())
                .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
            let end = start
                .checked_add(contraction_bytes)
                .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
            let content_start = start
                .checked_add(1)
                .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
            if end <= bytes.len()
                && bytes[content_start..end]
                    .iter()
                    .zip(contraction)
                    .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected))
            {
                return Ok(end);
            }
        }
    }

    let mut letters = start;
    if !is_ascii_letter(bytes[letters]) {
        let next = letters
            .checked_add(1)
            .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
        if bytes[letters] != b'\r'
            && bytes[letters] != b'\n'
            && !is_ascii_digit(bytes[letters])
            && next < bytes.len()
            && is_ascii_letter(bytes[next])
        {
            letters = next;
        }
    }
    if is_ascii_letter(bytes[letters]) {
        let mut end = letters;
        while end < bytes.len() && is_ascii_letter(bytes[end]) {
            work.charge(1)?;
            end = end
                .checked_add(1)
                .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
        }
        return Ok(end);
    }

    if is_ascii_digit(bytes[start]) {
        return start
            .checked_add(1)
            .ok_or(TokenizerExecutionError::ArithmeticOverflow);
    }

    let punctuation = if bytes[start] == b' ' {
        start.checked_add(1).and_then(|next| {
            (next < bytes.len() && is_ascii_punctuation_class(bytes[next])).then_some(next)
        })
    } else if is_ascii_punctuation_class(bytes[start]) {
        Some(start)
    } else {
        None
    };
    if let Some(mut end) = punctuation {
        while end < bytes.len() && is_ascii_punctuation_class(bytes[end]) {
            work.charge(1)?;
            end = end
                .checked_add(1)
                .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
        }
        while end < bytes.len() && matches!(bytes[end], b'\r' | b'\n') {
            work.charge(1)?;
            end = end
                .checked_add(1)
                .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
        }
        return Ok(end);
    }

    if is_ascii_whitespace(bytes[start]) {
        let mut run_end = start;
        let mut last_newline_end = None;
        while run_end < bytes.len() && is_ascii_whitespace(bytes[run_end]) {
            work.charge(1)?;
            let inspected = run_end;
            run_end = run_end
                .checked_add(1)
                .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
            if matches!(bytes[inspected], b'\r' | b'\n') {
                last_newline_end = Some(run_end);
            }
        }
        if let Some(newline_end) = last_newline_end {
            return Ok(newline_end);
        }
        let run_length = run_end
            .checked_sub(start)
            .ok_or(TokenizerExecutionError::ArithmeticOverflow)?;
        if run_end < bytes.len() && run_length > 1 {
            return run_end
                .checked_sub(1)
                .ok_or(TokenizerExecutionError::ArithmeticOverflow);
        }
        return Ok(run_end);
    }

    start
        .checked_add(1)
        .ok_or(TokenizerExecutionError::ArithmeticOverflow)
}

const fn is_ascii_letter(byte: u8) -> bool {
    byte.is_ascii_alphabetic()
}

const fn is_ascii_digit(byte: u8) -> bool {
    byte.is_ascii_digit()
}

const fn is_ascii_whitespace(byte: u8) -> bool {
    byte.is_ascii_whitespace()
}

const fn is_ascii_punctuation_class(byte: u8) -> bool {
    !is_ascii_whitespace(byte) && !is_ascii_letter(byte) && !is_ascii_digit(byte)
}

fn byte_to_unicode(byte: u8) -> Option<char> {
    let codepoint = if matches!(byte, b'!'..=b'~' | 0xA1..=0xAC | 0xAE..=0xFF) {
        u32::from(byte)
    } else if byte <= b' ' {
        256_u32.checked_add(u32::from(byte))?
    } else if (0x7F..=0xA0).contains(&byte) {
        u32::from(byte).checked_add(162)?
    } else {
        debug_assert_eq!(byte, 0xAD);
        323
    };
    char::from_u32(codepoint)
}

fn unicode_to_byte(symbol: char) -> Option<u8> {
    let codepoint = u32::from(symbol);
    if let Ok(byte) = u8::try_from(codepoint) {
        if matches!(byte, b'!'..=b'~' | 0xA1..=0xAC | 0xAE..=0xFF) {
            return Some(byte);
        }
    }
    match codepoint {
        256..=288 => u8::try_from(codepoint.checked_sub(256)?).ok(),
        289..=322 => u8::try_from(codepoint.checked_sub(162)?).ok(),
        323 => Some(0xAD),
        _ => None,
    }
}

fn charge_ordered_lookup(
    work: &mut WorkBudget,
    left_bytes: usize,
    right_bytes: usize,
) -> Result<(), TokenizerExecutionError> {
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
) -> Result<(), TokenizerExecutionError> {
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

fn push_byte(output: &mut Vec<u8>, byte: u8, limit: usize) -> Result<(), TokenizerExecutionError> {
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
    const fn new(limit: usize) -> Self {
        Self { used: 0, limit }
    }

    fn charge(&mut self, amount: usize) -> Result<(), TokenizerExecutionError> {
        self.used = self
            .used
            .checked_add(amount)
            .ok_or(TokenizerExecutionError::WorkLimit { limit: self.limit })?;
        if self.used > self.limit {
            return Err(TokenizerExecutionError::WorkLimit { limit: self.limit });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        byte_to_unicode, next_ascii_split, unicode_to_byte, SpecialTokenDecodePolicy,
        SpecialTokenEncodePolicy, TokenizerExecutionError, TokenizerExecutionLimits, WorkBudget,
        MAX_TOKENIZER_INPUT_BYTES, MAX_TOKENIZER_WORK,
    };
    use crate::tokenizer::tests::test_tokenizer;
    use crate::ADDED_TOKENS;
    use ferric_spec::Qwen3ModelRole;

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
    fn pinned_ascii_encode_fixtures_are_exact_and_deterministic() {
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
        ];
        for (input, expected) in fixtures {
            let first = tokenizer
                .encode(
                    input,
                    TokenizerExecutionLimits::m1(),
                    SpecialTokenEncodePolicy::Reject,
                )
                .expect("pinned ASCII fixture");
            let second = tokenizer
                .encode(
                    input,
                    TokenizerExecutionLimits::m1(),
                    SpecialTokenEncodePolicy::Reject,
                )
                .expect("repeat pinned ASCII fixture");
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
    fn ascii_split_boundaries_match_the_pinned_regex_domain() {
        for (input, expected) in [
            ("Hello world", vec!["Hello", " world"]),
            ("hello   world", vec!["hello", "  ", " world"]),
            ("I'RE ready", vec!["I", "'RE", " ready"]),
            ("1 23\nnext", vec!["1", " ", "2", "3", "\n", "next"]),
            ("tabs\twork", vec!["tabs", "\twork"]),
            ("line\r\nend", vec!["line", "\r\n", "end"]),
            (" punctuation!?", vec![" punctuation", "!?"]),
            ("trailing   ", vec!["trailing", "   "]),
        ] {
            let mut budget = WorkBudget::new(MAX_TOKENIZER_WORK);
            let mut cursor = 0;
            let mut actual = Vec::new();
            while cursor < input.len() {
                let end = next_ascii_split(input.as_bytes(), cursor, &mut budget)
                    .expect("bounded ASCII split");
                actual.push(&input[cursor..end]);
                cursor = end;
            }
            assert_eq!(actual, expected, "split fixture {input:?}");
        }
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
    fn unsupported_unicode_and_every_execution_bound_fail_closed() {
        let tokenizer = tokenizer();
        assert_eq!(
            tokenizer
                .encode(
                    "aé",
                    TokenizerExecutionLimits::m1(),
                    SpecialTokenEncodePolicy::Reject,
                )
                .expect_err("Unicode is outside the proved execution domain"),
            TokenizerExecutionError::UnsupportedUnicode { byte_offset: 1 }
        );
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
                    tight_limits(32, 1, 32, 1_024),
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
}
