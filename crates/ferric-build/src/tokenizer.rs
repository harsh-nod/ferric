use crate::json::Value;
use crate::sha256::Sha256;
use crate::{
    decode_hex_32, hash_field, ArtifactDigest, ADDED_TOKENS, QWEN3_TOKENIZER_BYTES,
    QWEN3_TOKENIZER_SHA256,
};
use ferric_spec::Qwen3ModelRole;
use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Read};
use std::sync::Arc;

use crate::tokenizer_execution::{
    SpecialTokenDecodePolicy, SpecialTokenEncodePolicy, TokenizerExecutionError,
    TokenizerExecutionLimits, TokenizerProgram, TokenizerProgramConstructionError,
    QWEN3_SPLIT_REGEX,
};

const TOKENIZER_JSON_BYTES: usize = 11_422_654;
const TOKENIZER_JSON_BYTES_U64: u64 = QWEN3_TOKENIZER_BYTES;
const MAX_TOKENIZER_PARSE_BYTES: usize = TOKENIZER_JSON_BYTES + 4_096;
const STREAM_BUFFER_BYTES: usize = 64 * 1_024;
const BASE_VOCABULARY_SIZE: u32 = 151_643;
const MERGE_COUNT: usize = 151_387;
type OrderedMerges = Vec<(String, String)>;
const TOKENIZER_JSON_SHA256: [u8; 32] = QWEN3_TOKENIZER_SHA256;
const VOCABULARY_SEMANTIC_SHA256: [u8; 32] =
    decode_hex_32(b"d42824870d58ccbf38bc6d29b312cc4550c8543f448c45fe644dd041f3eff638");
const MERGES_SEMANTIC_SHA256: [u8; 32] =
    decode_hex_32(b"1f8c784c660c1659a981d03c46deea0abcbf3fb4f6e85938e27281869890734f");

/// Evidence that one model role's exact pinned `tokenizer.json` stream was
/// hashed through EOF and admitted against the closed Qwen3 tokenizer schema.
///
/// Private fields and the private seal prevent descriptor-only construction.
/// The value is intentionally neither [`Clone`] nor [`Copy`].
pub struct AuthenticatedTokenizer {
    role: Qwen3ModelRole,
    descriptor: ArtifactDigest,
    vocabulary_semantic_sha256: [u8; 32],
    merges_semantic_sha256: [u8; 32],
    program: Arc<TokenizerProgram>,
    _seal: AuthenticatedTokenizerSeal,
}

#[derive(Debug, PartialEq, Eq)]
struct AuthenticatedTokenizerSeal;

impl fmt::Debug for AuthenticatedTokenizer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthenticatedTokenizer")
            .field("role", &self.role)
            .field("descriptor", &self.descriptor)
            .field(
                "vocabulary_semantic_sha256",
                &self.vocabulary_semantic_sha256,
            )
            .field("merges_semantic_sha256", &self.merges_semantic_sha256)
            .finish_non_exhaustive()
    }
}

impl AuthenticatedTokenizer {
    /// Returns the model role whose tokenizer stream was authenticated.
    #[must_use]
    pub const fn role(&self) -> Qwen3ModelRole {
        self.role
    }

    /// Encodes one bounded UTF-8 input with the authenticated tokenizer.
    ///
    /// The exact pinned NFC implementation and fixed Qwen3 Split expression
    /// are retained privately by the authenticated authority.
    ///
    /// # Errors
    ///
    /// Returns [`TokenizerExecutionError`] when a bound is invalid or exceeded,
    /// a forbidden special token is present, or the finite normalization,
    /// split, or BPE work budget is exhausted.
    pub fn encode(
        &self,
        input: &str,
        limits: TokenizerExecutionLimits,
        special_tokens: SpecialTokenEncodePolicy,
    ) -> Result<Vec<u32>, TokenizerExecutionError> {
        self.program.encode(input, limits, special_tokens)
    }

    /// Decodes bounded token IDs to exact bytes with the authenticated tokenizer.
    ///
    /// Bytes are returned directly because an arbitrary token slice need not be
    /// valid UTF-8. Added-token bytes are either preserved exactly or, for the
    /// fixed special-token subset, skipped according to `special_tokens`.
    ///
    /// # Errors
    ///
    /// Returns [`TokenizerExecutionError`] when a bound is invalid or exceeded,
    /// a token ID is outside the pinned vocabulary, or an admitted vocabulary
    /// token cannot be reversed through the pinned byte-level alphabet.
    pub fn decode_to_bytes(
        &self,
        token_ids: &[u32],
        limits: TokenizerExecutionLimits,
        special_tokens: SpecialTokenDecodePolicy,
    ) -> Result<Vec<u8>, TokenizerExecutionError> {
        self.program
            .execution
            .decode_to_bytes(token_ids, limits, special_tokens)
    }

    pub(crate) fn into_descriptor(self) -> ArtifactDigest {
        self.descriptor
    }

    pub(crate) fn compatible_with(&self, other: &Self) -> bool {
        self.descriptor == other.descriptor
            && self.vocabulary_semantic_sha256 == other.vocabulary_semantic_sha256
            && self.merges_semantic_sha256 == other.merges_semantic_sha256
    }
}

/// Fail-closed tokenizer streaming or semantic-admission failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenizerError {
    /// The tokenizer JSON was empty or exceeded the hard parse bound.
    ArtifactSize(usize),
    /// The tokenizer JSON was syntactically invalid, including duplicate keys.
    InvalidJson {
        /// Strict-parser byte offset.
        offset: usize,
        /// Stable parser reason.
        reason: String,
    },
    /// A closed-schema field was missing.
    MissingField(String),
    /// A closed-schema field was unknown.
    UnknownField(String),
    /// A closed-schema field had the wrong JSON type.
    WrongType(String),
    /// A closed-schema value differed from the pinned Qwen3 tokenizer.
    UnexpectedValue(String),
    /// The base vocabulary entry count differed.
    VocabularyCount { expected: u32, actual: u32 },
    /// A base-vocabulary token ID was out of range or appeared more than once.
    DuplicateOrInvalidTokenId(u32),
    /// A base-vocabulary token ID was absent.
    MissingTokenId(u32),
    /// The exhaustive ordered token/ID identity differed.
    VocabularyDigestMismatch,
    /// The ordered BPE merge count differed.
    MergeCount { expected: usize, actual: usize },
    /// The exhaustive ordered BPE merge identity differed.
    MergeDigestMismatch,
    /// The reader returned an I/O error.
    Io(io::ErrorKind),
    /// EOF arrived before the exact pinned byte length.
    EarlyEof { expected: usize, actual: usize },
    /// A byte followed the exact pinned tokenizer length.
    TrailingData,
    /// The complete streamed bytes differed from the pinned SHA-256.
    DigestMismatch,
    /// The fixed authenticated Split expression could not be compiled.
    RegexConstruction,
    /// The authenticated numeric tokenizer execution program could not be constructed.
    ExecutionProgramConstruction,
}

impl fmt::Display for TokenizerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArtifactSize(actual) => {
                write!(formatter, "tokenizer.json size {actual} violates its bound")
            }
            Self::InvalidJson { offset, reason } => {
                write!(
                    formatter,
                    "invalid tokenizer.json at byte {offset}: {reason}"
                )
            }
            Self::MissingField(field) => write!(formatter, "missing tokenizer field {field:?}"),
            Self::UnknownField(field) => write!(formatter, "unknown tokenizer field {field:?}"),
            Self::WrongType(field) => {
                write!(formatter, "tokenizer field {field:?} has the wrong type")
            }
            Self::UnexpectedValue(field) => {
                write!(formatter, "tokenizer field {field:?} is not canonical")
            }
            Self::VocabularyCount { expected, actual } => {
                write!(
                    formatter,
                    "base vocabulary count is {actual}, expected {expected}"
                )
            }
            Self::DuplicateOrInvalidTokenId(id) => {
                write!(
                    formatter,
                    "base vocabulary token ID {id} is invalid or repeated"
                )
            }
            Self::MissingTokenId(id) => {
                write!(formatter, "base vocabulary token ID {id} is absent")
            }
            Self::VocabularyDigestMismatch => {
                formatter.write_str("base vocabulary token/ID identity mismatched")
            }
            Self::MergeCount { expected, actual } => {
                write!(
                    formatter,
                    "BPE merge count is {actual}, expected {expected}"
                )
            }
            Self::MergeDigestMismatch => {
                formatter.write_str("ordered BPE merge identity mismatched")
            }
            Self::Io(kind) => write!(formatter, "I/O error reading tokenizer.json: {kind}"),
            Self::EarlyEof { expected, actual } => write!(
                formatter,
                "tokenizer.json ended at byte {actual}, expected {expected} bytes"
            ),
            Self::TrailingData => formatter.write_str("tokenizer.json has trailing data"),
            Self::DigestMismatch => {
                formatter.write_str("tokenizer.json full-file SHA-256 mismatched")
            }
            Self::RegexConstruction => {
                formatter.write_str("authenticated tokenizer Split regex construction failed")
            }
            Self::ExecutionProgramConstruction => {
                formatter.write_str("authenticated numeric tokenizer program construction failed")
            }
        }
    }
}

impl std::error::Error for TokenizerError {}

/// Streams and authenticates the exact shared Qwen3 `tokenizer.json` for one
/// target or draft role.
///
/// The input is read forward in bounded chunks. The complete tokenizer JSON is
/// retained in one exactly bounded allocation for strict semantic parsing; no
/// model weight bytes are involved.
///
/// # Errors
///
/// Returns [`TokenizerError`] unless the stream has the exact pinned size and
/// SHA-256, ends immediately at EOF, and satisfies every closed tokenizer
/// schema and semantic identity check.
pub fn authenticate_qwen3_tokenizer<R: Read>(
    role: Qwen3ModelRole,
    mut reader: R,
) -> Result<AuthenticatedTokenizer, TokenizerError> {
    let mut bytes = vec![0; TOKENIZER_JSON_BYTES];
    let mut hasher = Sha256::new();
    let mut total = 0_usize;
    for chunk in bytes.chunks_mut(STREAM_BUFFER_BYTES) {
        read_exact_hashed(&mut reader, chunk, &mut hasher, &mut total)?;
    }
    let mut trailing = [0; 1];
    loop {
        match reader.read(&mut trailing) {
            Ok(0) => break,
            Ok(_) => return Err(TokenizerError::TrailingData),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(TokenizerError::Io(error.kind())),
        }
    }
    if hasher.finish() != TOKENIZER_JSON_SHA256 {
        return Err(TokenizerError::DigestMismatch);
    }
    let semantics = parse_tokenizer_json(&bytes)?;
    Ok(AuthenticatedTokenizer {
        role,
        descriptor: ArtifactDigest {
            sha256: TOKENIZER_JSON_SHA256,
            byte_len: TOKENIZER_JSON_BYTES_U64,
        },
        vocabulary_semantic_sha256: semantics.vocabulary_semantic_sha256,
        merges_semantic_sha256: semantics.merges_semantic_sha256,
        program: semantics.program,
        _seal: AuthenticatedTokenizerSeal,
    })
}

fn read_exact_hashed<R: Read>(
    reader: &mut R,
    buffer: &mut [u8],
    hasher: &mut Sha256,
    total: &mut usize,
) -> Result<(), TokenizerError> {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => {
                return Err(TokenizerError::EarlyEof {
                    expected: TOKENIZER_JSON_BYTES,
                    actual: *total,
                });
            }
            Ok(read) => {
                hasher.update(&buffer[filled..filled + read]);
                filled += read;
                *total = total
                    .checked_add(read)
                    .ok_or(TokenizerError::ArtifactSize(usize::MAX))?;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(TokenizerError::Io(error.kind())),
        }
    }
    Ok(())
}

#[derive(Debug)]
struct TokenizerSemantics {
    vocabulary_semantic_sha256: [u8; 32],
    merges_semantic_sha256: [u8; 32],
    program: Arc<TokenizerProgram>,
}

fn parse_tokenizer_json(bytes: &[u8]) -> Result<TokenizerSemantics, TokenizerError> {
    if bytes.is_empty() || bytes.len() > MAX_TOKENIZER_PARSE_BYTES {
        return Err(TokenizerError::ArtifactSize(bytes.len()));
    }
    let value = crate::json::parse(bytes).map_err(|error| TokenizerError::InvalidJson {
        offset: error.offset,
        reason: error.kind.to_string(),
    })?;
    let mut root = Fields::new("$", object("$", value)?);
    root.expect_string("version", "1.0")?;
    root.expect_null("truncation")?;
    root.expect_null("padding")?;
    validate_added_tokens(root.take("added_tokens")?)?;
    validate_normalizer(root.take("normalizer")?)?;
    validate_pre_tokenizer(root.take("pre_tokenizer")?)?;
    validate_byte_level("post_processor", root.take("post_processor")?)?;
    validate_byte_level("decoder", root.take("decoder")?)?;
    let semantics = validate_model(root.take("model")?)?;
    root.finish()?;
    Ok(semantics)
}

fn validate_added_tokens(value: Value) -> Result<(), TokenizerError> {
    let Value::Array(tokens) = value else {
        return Err(wrong_type("$.added_tokens"));
    };
    if tokens.len() != ADDED_TOKENS.len() {
        return Err(unexpected("$.added_tokens"));
    }
    for (offset, (token, (expected_content, expected_special))) in
        tokens.into_iter().zip(ADDED_TOKENS).enumerate()
    {
        let path = format!("$.added_tokens[{offset}]");
        let mut fields = Fields::new(&path, object(&path, token)?);
        fields.expect_u32(
            "id",
            BASE_VOCABULARY_SIZE
                .checked_add(u32::try_from(offset).expect("added token offset fits u32"))
                .expect("added token ID fits u32"),
        )?;
        fields.expect_string("content", expected_content)?;
        fields.expect_bool("single_word", false)?;
        fields.expect_bool("lstrip", false)?;
        fields.expect_bool("rstrip", false)?;
        fields.expect_bool("normalized", false)?;
        fields.expect_bool("special", expected_special)?;
        fields.finish()?;
    }
    Ok(())
}

fn validate_normalizer(value: Value) -> Result<(), TokenizerError> {
    let mut fields = Fields::new("$.normalizer", object("$.normalizer", value)?);
    fields.expect_string("type", "NFC")?;
    fields.finish()
}

fn validate_pre_tokenizer(value: Value) -> Result<(), TokenizerError> {
    let mut fields = Fields::new("$.pre_tokenizer", object("$.pre_tokenizer", value)?);
    fields.expect_string("type", "Sequence")?;
    let Value::Array(mut stages) = fields.take("pretokenizers")? else {
        return Err(wrong_type("$.pre_tokenizer.pretokenizers"));
    };
    fields.finish()?;
    if stages.len() != 2 {
        return Err(unexpected("$.pre_tokenizer.pretokenizers"));
    }
    let byte_level = stages.pop().expect("two preprocessing stages");
    let split = stages.pop().expect("two preprocessing stages");

    let mut split_fields = Fields::new(
        "$.pre_tokenizer.pretokenizers[0]",
        object("$.pre_tokenizer.pretokenizers[0]", split)?,
    );
    split_fields.expect_string("type", "Split")?;
    let mut pattern = Fields::new(
        "$.pre_tokenizer.pretokenizers[0].pattern",
        object(
            "$.pre_tokenizer.pretokenizers[0].pattern",
            split_fields.take("pattern")?,
        )?,
    );
    pattern.expect_string("Regex", QWEN3_SPLIT_REGEX)?;
    pattern.finish()?;
    split_fields.expect_string("behavior", "Isolated")?;
    split_fields.expect_bool("invert", false)?;
    split_fields.finish()?;
    validate_byte_level("pre_tokenizer.pretokenizers[1]", byte_level)
}

fn validate_byte_level(path: &str, value: Value) -> Result<(), TokenizerError> {
    let path = format!("$.{path}");
    let mut fields = Fields::new(&path, object(&path, value)?);
    fields.expect_string("type", "ByteLevel")?;
    fields.expect_bool("add_prefix_space", false)?;
    fields.expect_bool("trim_offsets", false)?;
    fields.expect_bool("use_regex", false)?;
    fields.finish()
}

fn validate_model(value: Value) -> Result<TokenizerSemantics, TokenizerError> {
    let mut fields = Fields::new("$.model", object("$.model", value)?);
    fields.expect_string("type", "BPE")?;
    fields.expect_null("dropout")?;
    fields.expect_null("unk_token")?;
    fields.expect_string("continuing_subword_prefix", "")?;
    fields.expect_string("end_of_word_suffix", "")?;
    fields.expect_bool("fuse_unk", false)?;
    fields.expect_bool("byte_fallback", false)?;
    fields.expect_bool("ignore_merges", false)?;
    let (vocabulary_semantic_sha256, vocabulary) = validate_vocabulary(fields.take("vocab")?)?;
    let (merges_semantic_sha256, merges) = validate_merges(fields.take("merges")?)?;
    fields.finish()?;
    Ok(TokenizerSemantics {
        vocabulary_semantic_sha256,
        merges_semantic_sha256,
        program: Arc::new(TokenizerProgram::new(vocabulary, merges).map_err(
            |error| match error {
                TokenizerProgramConstructionError::Regex => TokenizerError::RegexConstruction,
                TokenizerProgramConstructionError::ArithmeticOverflow
                | TokenizerProgramConstructionError::AllocationFailed
                | TokenizerProgramConstructionError::InvalidVocabulary => {
                    TokenizerError::ExecutionProgramConstruction
                }
            },
        )?),
    })
}

fn validate_vocabulary(value: Value) -> Result<([u8; 32], Vec<String>), TokenizerError> {
    let vocabulary = object("$.model.vocab", value)?;
    let actual = u32::try_from(vocabulary.len()).unwrap_or(u32::MAX);
    if actual != BASE_VOCABULARY_SIZE {
        return Err(TokenizerError::VocabularyCount {
            expected: BASE_VOCABULARY_SIZE,
            actual,
        });
    }
    let mut tokens =
        vec![None; usize::try_from(BASE_VOCABULARY_SIZE).expect("base vocabulary size fits usize")];
    for (token, value) in vocabulary {
        let id = number_u32("$.model.vocab", value)?;
        let index =
            usize::try_from(id).map_err(|_| TokenizerError::DuplicateOrInvalidTokenId(id))?;
        let Some(slot) = tokens.get_mut(index) else {
            return Err(TokenizerError::DuplicateOrInvalidTokenId(id));
        };
        if slot.replace(token).is_some() {
            return Err(TokenizerError::DuplicateOrInvalidTokenId(id));
        }
    }

    let mut hasher = Sha256::new();
    hash_field(&mut hasher, b"ferric.qwen3-tokenizer-vocab.v1");
    for (id, token) in tokens.iter().enumerate() {
        let id = u32::try_from(id).expect("base vocabulary index fits u32");
        let token = token.as_ref().ok_or(TokenizerError::MissingTokenId(id))?;
        hash_field(&mut hasher, &id.to_be_bytes());
        hash_field(&mut hasher, token.as_bytes());
    }
    let digest = hasher.finish();
    if digest != VOCABULARY_SEMANTIC_SHA256 {
        return Err(TokenizerError::VocabularyDigestMismatch);
    }
    let tokens = tokens
        .into_iter()
        .enumerate()
        .map(|(id, token)| {
            token.ok_or_else(|| {
                TokenizerError::MissingTokenId(
                    u32::try_from(id).expect("base vocabulary index fits u32"),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((digest, tokens))
}

fn validate_merges(value: Value) -> Result<([u8; 32], OrderedMerges), TokenizerError> {
    let Value::Array(merges) = value else {
        return Err(wrong_type("$.model.merges"));
    };
    if merges.len() != MERGE_COUNT {
        return Err(TokenizerError::MergeCount {
            expected: MERGE_COUNT,
            actual: merges.len(),
        });
    }
    let mut hasher = Sha256::new();
    let mut ordered_merges = Vec::new();
    ordered_merges
        .try_reserve_exact(merges.len())
        .map_err(|_| TokenizerError::ArtifactSize(TOKENIZER_JSON_BYTES))?;
    hash_field(&mut hasher, b"ferric.qwen3-tokenizer-merges.v1");
    for (ordinal, merge) in merges.into_iter().enumerate() {
        let Value::Array(pair) = merge else {
            return Err(wrong_type(&format!("$.model.merges[{ordinal}]")));
        };
        if pair.len() != 2 {
            return Err(unexpected(&format!("$.model.merges[{ordinal}]")));
        }
        let mut pair = pair.into_iter();
        let left = string(
            &format!("$.model.merges[{ordinal}][0]"),
            pair.next().expect("two merge members"),
        )?;
        let right = string(
            &format!("$.model.merges[{ordinal}][1]"),
            pair.next().expect("two merge members"),
        )?;
        hash_field(
            &mut hasher,
            &u64::try_from(ordinal)
                .expect("merge ordinal fits u64")
                .to_be_bytes(),
        );
        hash_field(&mut hasher, left.as_bytes());
        hash_field(&mut hasher, right.as_bytes());
        ordered_merges.push((left, right));
    }
    let digest = hasher.finish();
    if digest != MERGES_SEMANTIC_SHA256 {
        return Err(TokenizerError::MergeDigestMismatch);
    }
    Ok((digest, ordered_merges))
}

fn object(path: &str, value: Value) -> Result<BTreeMap<String, Value>, TokenizerError> {
    if let Value::Object(fields) = value {
        Ok(fields)
    } else {
        Err(wrong_type(path))
    }
}

fn string(path: &str, value: Value) -> Result<String, TokenizerError> {
    if let Value::String(value) = value {
        Ok(value)
    } else {
        Err(wrong_type(path))
    }
}

fn number_u32(path: &str, value: Value) -> Result<u32, TokenizerError> {
    let Value::Number(value) = value else {
        return Err(wrong_type(path));
    };
    value.parse().map_err(|_| unexpected(path))
}

struct Fields {
    path: String,
    fields: BTreeMap<String, Value>,
}

impl Fields {
    fn new(path: &str, fields: BTreeMap<String, Value>) -> Self {
        Self {
            path: path.to_owned(),
            fields,
        }
    }

    fn field_path(&self, field: &str) -> String {
        format!("{}.{}", self.path, field)
    }

    fn take(&mut self, field: &str) -> Result<Value, TokenizerError> {
        self.fields
            .remove(field)
            .ok_or_else(|| TokenizerError::MissingField(self.field_path(field)))
    }

    fn expect_null(&mut self, field: &str) -> Result<(), TokenizerError> {
        if self.take(field)? != Value::Null {
            return Err(unexpected(&self.field_path(field)));
        }
        Ok(())
    }

    fn expect_bool(&mut self, field: &str, expected: bool) -> Result<(), TokenizerError> {
        let Value::Bool(actual) = self.take(field)? else {
            return Err(wrong_type(&self.field_path(field)));
        };
        if actual != expected {
            return Err(unexpected(&self.field_path(field)));
        }
        Ok(())
    }

    fn expect_string(&mut self, field: &str, expected: &str) -> Result<(), TokenizerError> {
        let actual = string(&self.field_path(field), self.take(field)?)?;
        if actual != expected {
            return Err(unexpected(&self.field_path(field)));
        }
        Ok(())
    }

    fn expect_u32(&mut self, field: &str, expected: u32) -> Result<(), TokenizerError> {
        let actual = number_u32(&self.field_path(field), self.take(field)?)?;
        if actual != expected {
            return Err(unexpected(&self.field_path(field)));
        }
        Ok(())
    }

    fn finish(self) -> Result<(), TokenizerError> {
        if let Some(field) = self.fields.into_keys().next() {
            return Err(TokenizerError::UnknownField(format!(
                "{}.{}",
                self.path, field
            )));
        }
        Ok(())
    }
}

fn wrong_type(path: &str) -> TokenizerError {
    TokenizerError::WrongType(path.to_owned())
}

fn unexpected(path: &str) -> TokenizerError {
    TokenizerError::UnexpectedValue(path.to_owned())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        authenticate_qwen3_tokenizer, parse_tokenizer_json, AuthenticatedTokenizer,
        AuthenticatedTokenizerSeal, TokenizerError, MERGES_SEMANTIC_SHA256, TOKENIZER_JSON_BYTES,
        TOKENIZER_JSON_SHA256, VOCABULARY_SEMANTIC_SHA256,
    };
    use crate::safetensors::tests::test_authenticated_weight_set;
    use crate::{
        build_authenticated_deployment_bundle, ArtifactDigest, AuthenticatedDeploymentAssets,
        AuthenticatedModelAssets, BuildError, SpecialTokenDecodePolicy, SpecialTokenEncodePolicy,
        TokenizerExecutionLimits, DRAFT_REPOSITORY, DRAFT_REVISION, QWEN3_TARGET_WEIGHT_SET_SHA256,
        TARGET_REPOSITORY, TARGET_REVISION,
    };
    use ferric_spec::{EngineLimits, Qwen3ModelRole};
    use std::io::{Cursor, Read};
    use std::sync::{Arc, OnceLock};

    const TOKENIZER: &[u8] = include_bytes!("fixtures/tokenizer/qwen3-tokenizer.json");
    const TARGET_CONFIG: &[u8] = include_bytes!("fixtures/qwen3-8b-config.json");
    const DRAFT_CONFIG: &[u8] = include_bytes!("fixtures/qwen3-06b-config.json");
    const TOKENIZER_METADATA: &[u8] = include_bytes!("fixtures/qwen3-tokenizer-config.json");

    fn replace_once(input: &[u8], from: &str, to: &str) -> Vec<u8> {
        let input = std::str::from_utf8(input).expect("tokenizer fixture is UTF-8");
        let changed = input.replacen(from, to, 1);
        assert_ne!(changed, input);
        changed.into_bytes()
    }

    pub(crate) fn test_tokenizer(role: Qwen3ModelRole) -> AuthenticatedTokenizer {
        static PROGRAM: OnceLock<Arc<super::TokenizerProgram>> = OnceLock::new();
        AuthenticatedTokenizer {
            role,
            descriptor: ArtifactDigest {
                sha256: TOKENIZER_JSON_SHA256,
                byte_len: u64::try_from(TOKENIZER_JSON_BYTES)
                    .expect("tokenizer byte length fits u64"),
            },
            vocabulary_semantic_sha256: VOCABULARY_SEMANTIC_SHA256,
            merges_semantic_sha256: MERGES_SEMANTIC_SHA256,
            program: Arc::clone(PROGRAM.get_or_init(|| {
                parse_tokenizer_json(TOKENIZER)
                    .expect("canonical tokenizer semantics")
                    .program
            })),
            _seal: AuthenticatedTokenizerSeal,
        }
    }

    pub(crate) fn authenticated_assets() -> AuthenticatedDeploymentAssets<'static> {
        AuthenticatedDeploymentAssets {
            target: AuthenticatedModelAssets {
                repository: TARGET_REPOSITORY,
                revision: TARGET_REVISION,
                config_json: &TARGET_CONFIG[..TARGET_CONFIG.len() - 1],
                tokenizer_metadata_json: TOKENIZER_METADATA,
            },
            draft: AuthenticatedModelAssets {
                repository: DRAFT_REPOSITORY,
                revision: DRAFT_REVISION,
                config_json: &DRAFT_CONFIG[..DRAFT_CONFIG.len() - 1],
                tokenizer_metadata_json: TOKENIZER_METADATA,
            },
            limits: EngineLimits {
                max_context_tokens: 8_192,
                max_active_sequences: 32,
                kv_page_tokens: 256,
                max_draft_tokens: 16,
            },
        }
    }

    struct ChunkedReader<'a> {
        cursor: Cursor<&'a [u8]>,
        max_chunk: usize,
    }

    impl Read for ChunkedReader<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let chunk = buffer.len().min(self.max_chunk);
            self.cursor.read(&mut buffer[..chunk])
        }
    }

    #[test]
    fn canonical_tokenizer_semantics_and_chunked_stream_authenticate() {
        assert_eq!(TOKENIZER.len(), TOKENIZER_JSON_BYTES);
        assert_eq!(crate::sha256::digest(TOKENIZER), TOKENIZER_JSON_SHA256);
        let semantics = parse_tokenizer_json(TOKENIZER).expect("canonical tokenizer semantics");
        assert_eq!(
            semantics.vocabulary_semantic_sha256,
            VOCABULARY_SEMANTIC_SHA256
        );
        assert_eq!(semantics.merges_semantic_sha256, MERGES_SEMANTIC_SHA256);

        let authenticated = authenticate_qwen3_tokenizer(
            Qwen3ModelRole::Target8B,
            ChunkedReader {
                cursor: Cursor::new(TOKENIZER),
                max_chunk: 4_093,
            },
        )
        .expect("complete chunked tokenizer stream");
        assert_eq!(authenticated.role(), Qwen3ModelRole::Target8B);
        let ids = authenticated
            .encode(
                "Hello world",
                TokenizerExecutionLimits::m1(),
                SpecialTokenEncodePolicy::Reject,
            )
            .expect("execute only through complete authenticated authority");
        assert_eq!(ids, vec![9_707, 1_879]);
        assert_eq!(
            authenticated
                .decode_to_bytes(
                    &ids,
                    TokenizerExecutionLimits::m1(),
                    SpecialTokenDecodePolicy::Preserve,
                )
                .expect("decode only through complete authenticated authority"),
            b"Hello world"
        );
    }

    #[test]
    fn duplicate_unknown_and_missing_keys_fail_closed() {
        let duplicate = replace_once(
            TOKENIZER,
            r#""version": "1.0","#,
            r#""version": "1.0", "version": "1.0","#,
        );
        assert!(matches!(
            parse_tokenizer_json(&duplicate),
            Err(TokenizerError::InvalidJson { ref reason, .. })
                if reason.contains("duplicate field")
        ));

        let unknown = replace_once(
            TOKENIZER,
            r#""version": "1.0","#,
            r#""version": "1.0", "future": null,"#,
        );
        assert!(matches!(
            parse_tokenizer_json(&unknown),
            Err(TokenizerError::UnknownField(ref field)) if field == "$.future"
        ));

        let missing = replace_once(TOKENIZER, r#""normalizer": {"#, r#""renamed": {"#);
        assert!(matches!(
            parse_tokenizer_json(&missing),
            Err(TokenizerError::MissingField(ref field)) if field == "$.normalizer"
        ));
    }

    #[test]
    fn token_content_ids_and_added_token_drift_fail_closed() {
        let token = replace_once(TOKENIZER, r#""!": 0"#, r#""!drift": 0"#);
        assert_eq!(
            parse_tokenizer_json(&token).expect_err("token content drift"),
            TokenizerError::VocabularyDigestMismatch
        );

        let id = replace_once(TOKENIZER, r#""!": 0"#, r#""!": 1"#);
        assert_eq!(
            parse_tokenizer_json(&id).expect_err("token ID drift"),
            TokenizerError::DuplicateOrInvalidTokenId(1)
        );

        let added_id = replace_once(TOKENIZER, r#""id": 151643"#, r#""id": 151644"#);
        assert!(matches!(
            parse_tokenizer_json(&added_id),
            Err(TokenizerError::UnexpectedValue(ref field))
                if field == "$.added_tokens[0].id"
        ));

        let special = replace_once(
            TOKENIZER,
            r#""content": "<|endoftext|>""#,
            r#""content": "<|endoftext-drift|>""#,
        );
        assert!(matches!(
            parse_tokenizer_json(&special),
            Err(TokenizerError::UnexpectedValue(ref field))
                if field == "$.added_tokens[0].content"
        ));
    }

    #[test]
    fn merge_order_and_pipeline_drift_fail_closed() {
        let merges = replace_once(
            TOKENIZER,
            "      [\n        \"r\",\n        \"e\"\n      ],\n      [\n        \"a\",\n        \"t\"\n      ]",
            "      [\n        \"a\",\n        \"t\"\n      ],\n      [\n        \"r\",\n        \"e\"\n      ]",
        );
        assert_eq!(
            parse_tokenizer_json(&merges).expect_err("merge order drift"),
            TokenizerError::MergeDigestMismatch
        );

        for (from, to, field) in [
            (r#""type": "NFC""#, r#""type": "NFD""#, "$.normalizer.type"),
            (
                r#""behavior": "Isolated""#,
                r#""behavior": "Merged""#,
                "$.pre_tokenizer.pretokenizers[0].behavior",
            ),
            (r#""type": "BPE""#, r#""type": "WordPiece""#, "$.model.type"),
        ] {
            let changed = replace_once(TOKENIZER, from, to);
            assert!(matches!(
                parse_tokenizer_json(&changed),
                Err(TokenizerError::UnexpectedValue(ref actual)) if actual == field
            ));
        }

        let decoder = replace_once(
            TOKENIZER,
            "\"decoder\": {\n    \"type\": \"ByteLevel\"",
            "\"decoder\": {\n    \"type\": \"Metaspace\"",
        );
        assert!(matches!(
            parse_tokenizer_json(&decoder),
            Err(TokenizerError::UnexpectedValue(ref field)) if field == "$.decoder.type"
        ));
    }

    #[test]
    fn truncation_trailing_size_and_digest_mismatch_fail_closed() {
        assert!(matches!(
            authenticate_qwen3_tokenizer(
                Qwen3ModelRole::Target8B,
                Cursor::new(&TOKENIZER[..TOKENIZER.len() - 1]),
            ),
            Err(TokenizerError::EarlyEof { .. })
        ));

        let mut trailing = TOKENIZER.to_vec();
        trailing.push(b' ');
        assert_eq!(
            authenticate_qwen3_tokenizer(
                Qwen3ModelRole::Target8B,
                Cursor::new(trailing.as_slice()),
            )
            .expect_err("trailing byte"),
            TokenizerError::TrailingData
        );

        let mut flipped = TOKENIZER.to_vec();
        flipped[0] ^= 1;
        assert_eq!(
            authenticate_qwen3_tokenizer(
                Qwen3ModelRole::Target8B,
                Cursor::new(flipped.as_slice()),
            )
            .expect_err("full-file digest drift"),
            TokenizerError::DigestMismatch
        );

        let oversized = vec![b' '; super::MAX_TOKENIZER_PARSE_BYTES + 1];
        assert_eq!(
            parse_tokenizer_json(&oversized).expect_err("parser size bound"),
            TokenizerError::ArtifactSize(oversized.len())
        );
    }

    #[test]
    fn final_builder_consumes_roles_and_rejects_target_draft_mismatch() {
        let bundle = build_authenticated_deployment_bundle(
            authenticated_assets(),
            test_tokenizer(Qwen3ModelRole::Target8B),
            test_tokenizer(Qwen3ModelRole::Draft06B),
            test_authenticated_weight_set(Qwen3ModelRole::Target8B),
            test_authenticated_weight_set(Qwen3ModelRole::Draft06B),
        )
        .expect("fully authenticated bundle path");
        assert_eq!(
            bundle.target_model.weights.weights_id.as_bytes(),
            &QWEN3_TARGET_WEIGHT_SET_SHA256
        );
        assert_eq!(
            bundle.target_model.tokenizer.vocabulary_id.as_bytes(),
            &TOKENIZER_JSON_SHA256
        );

        assert_eq!(
            build_authenticated_deployment_bundle(
                authenticated_assets(),
                test_tokenizer(Qwen3ModelRole::Draft06B),
                test_tokenizer(Qwen3ModelRole::Target8B),
                test_authenticated_weight_set(Qwen3ModelRole::Target8B),
                test_authenticated_weight_set(Qwen3ModelRole::Draft06B),
            ),
            Err(BuildError::AuthenticatedTokenizerRole {
                expected: Qwen3ModelRole::Target8B,
                actual: Qwen3ModelRole::Draft06B,
            })
        );

        let chat_template_drift = replace_once(
            TOKENIZER_METADATA,
            r#""chat_template": "{%- if tools %}"#,
            r#""chat_template": "{%- if toolz %}"#,
        );
        let mut assets = authenticated_assets();
        assets.draft.tokenizer_metadata_json = &chat_template_drift;
        assert_eq!(
            build_authenticated_deployment_bundle(
                assets,
                test_tokenizer(Qwen3ModelRole::Target8B),
                test_tokenizer(Qwen3ModelRole::Draft06B),
                test_authenticated_weight_set(Qwen3ModelRole::Target8B),
                test_authenticated_weight_set(Qwen3ModelRole::Draft06B),
            ),
            Err(BuildError::DigestMismatch("tokenizer_config.json"))
        );

        let target = test_tokenizer(Qwen3ModelRole::Target8B);
        let mut draft = test_tokenizer(Qwen3ModelRole::Draft06B);
        draft.merges_semantic_sha256[0] ^= 1;
        assert_eq!(
            build_authenticated_deployment_bundle(
                authenticated_assets(),
                target,
                draft,
                test_authenticated_weight_set(Qwen3ModelRole::Target8B),
                test_authenticated_weight_set(Qwen3ModelRole::Draft06B),
            ),
            Err(BuildError::TokenizerMismatch)
        );
    }
}
