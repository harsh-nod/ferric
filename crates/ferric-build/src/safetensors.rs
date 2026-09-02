use crate::json::Value;
use crate::sha256::Sha256;
use crate::{
    decode_hex_32, WeightDescriptor, QWEN3_DRAFT_TENSOR_DATA_BYTES,
    QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES, QWEN3_DRAFT_WEIGHT_SHA256, QWEN3_TARGET_TENSOR_DATA_BYTES,
    QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES, QWEN3_TARGET_WEIGHT_SET_SHA256,
};
use ferric_spec::{
    Qwen3ModelRole, Qwen3TensorError, Qwen3TensorKind, Qwen3TensorMetadata, TensorDType,
    QWEN3_NO_LAYER,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Read};

pub(super) const MAX_SAFETENSORS_HEADER_BYTES: u64 = 64 * 1_024;
const MAX_INDEX_BYTES: usize = 64 * 1_024;
const STREAM_BUFFER_BYTES: usize = 64 * 1_024;
pub(super) const TARGET_INDEX_BYTES: usize = 32_878;
const TARGET_INDEX_SHA256: [u8; 32] =
    decode_hex_32(b"f9fdbcb91c23971c13ec5d5f2573d2349e8f61f2f049371ec699281748fdb1bc");

pub(super) const TARGET_SHARD_PINS: [FilePin; 5] = [
    FilePin {
        name: "model-00001-of-00005.safetensors",
        sha256: decode_hex_32(b"31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f"),
        file_bytes: 3_996_250_744,
        header_bytes: 9_328,
        tensor_data_bytes: 3_996_241_408,
    },
    FilePin {
        name: "model-00002-of-00005.safetensors",
        sha256: decode_hex_32(b"5991236cea6fe21f3d43cab0f0e84448734fbbe0789816202989f2ddc9d18282"),
        file_bytes: 3_993_160_032,
        header_bytes: 13_144,
        tensor_data_bytes: 3_993_146_880,
    },
    FilePin {
        name: "model-00003-of-00005.safetensors",
        sha256: decode_hex_32(b"c5185c4794be2d8a9784d5753c9922db38df478ce11f9ed0b415b7304d896836"),
        file_bytes: 3_959_604_768,
        header_bytes: 12_824,
        tensor_data_bytes: 3_959_591_936,
    },
    FilePin {
        name: "model-00004-of-00005.safetensors",
        sha256: decode_hex_32(b"b5ee7de71fbf17db3d5704e0c8f2bc7d005ca9e1d7ca2aeb19827b0cfcaa917a"),
        file_bytes: 3_187_841_392,
        header_bytes: 10_600,
        tensor_data_bytes: 3_187_830_784,
    },
    FilePin {
        name: "model-00005-of-00005.safetensors",
        sha256: decode_hex_32(b"20c2d6366ab85c90786ccdd829cd2b9e7d30ef3b2ebbb998280e7e4014b542ff"),
        file_bytes: 1_244_659_840,
        header_bytes: 120,
        tensor_data_bytes: 1_244_659_712,
    },
];

pub(super) const DRAFT_FILE_PIN: FilePin = FilePin {
    name: "model.safetensors",
    sha256: QWEN3_DRAFT_WEIGHT_SHA256,
    file_bytes: QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
    header_bytes: 35_552,
    tensor_data_bytes: QWEN3_DRAFT_TENSOR_DATA_BYTES,
};

/// A named, forward-only safetensors byte stream.
pub struct SafetensorsSource<'a, R> {
    /// Exact canonical artifact filename.
    pub name: &'a str,
    /// Reader positioned at the first byte of the eight-byte header length.
    pub reader: R,
}

/// Evidence that every byte of an exact pinned Qwen3 weight set was streamed,
/// hashed, schema-checked, and followed immediately by EOF.
///
/// The private seal prevents descriptor-only code from constructing this
/// typestate. It is produced only by the streaming authentication functions.
#[derive(Debug, PartialEq, Eq)]
pub struct AuthenticatedWeightSet {
    role: Qwen3ModelRole,
    descriptor: WeightDescriptor,
    seal: AuthenticatedSeal,
}

#[derive(Debug, PartialEq, Eq)]
struct AuthenticatedSeal;

impl AuthenticatedWeightSet {
    /// Returns the exact authenticated model role.
    #[must_use]
    pub const fn role(&self) -> Qwen3ModelRole {
        self.role
    }

    pub(crate) fn into_descriptor(self) -> WeightDescriptor {
        self.descriptor
    }
}

/// Fail-closed safetensors schema or streaming authentication error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SafetensorsError {
    /// A bounded JSON artifact was empty or too large.
    ArtifactSize(String),
    /// JSON syntax was invalid, including any duplicate object field.
    InvalidJson {
        /// Artifact being parsed.
        artifact: String,
        /// Strict-parser byte offset.
        offset: usize,
        /// Stable parser reason.
        reason: String,
    },
    /// A closed-schema field was missing.
    MissingField { artifact: String, field: String },
    /// A closed-schema field was not recognized.
    UnknownField { artifact: String, field: String },
    /// A closed-schema value had the wrong JSON type.
    WrongType { artifact: String, field: String },
    /// A closed-schema value differed from the pinned value.
    UnexpectedValue { artifact: String, field: String },
    /// A tensor name did not belong to the exact Qwen3 schema.
    InvalidTensorName(String),
    /// A tensor was not BF16.
    UnexpectedDType(String),
    /// The executable Qwen3 schema rejected a tensor rank, shape, or layer.
    TensorSchema {
        /// Canonical tensor name.
        tensor: String,
        /// Executable specification failure.
        error: Qwen3TensorError,
    },
    /// A checked shape byte product overflowed.
    TensorSizeOverflow(String),
    /// A tensor range length differed from its checked BF16 shape size.
    TensorByteLength(String),
    /// A tensor offset range was empty or reversed.
    InvalidOffsetRange(String),
    /// Sorted tensor offsets left an uncovered byte range.
    OffsetGap { expected: u64, actual: u64 },
    /// Sorted tensor offsets overlapped an earlier tensor.
    OffsetOverlap { expected: u64, actual: u64 },
    /// The final tensor offset differed from the declared tensor-data bytes.
    TensorDataBytes { expected: u64, actual: u64 },
    /// The exact role tensor count differed.
    TensorCount { expected: u32, actual: u32 },
    /// A canonical tensor ordinal was missing or repeated.
    TensorOrder { expected: u32, actual: u32 },
    /// A tensor appeared more than once across shards.
    DuplicateTensor(String),
    /// A target index mapped a tensor to the wrong shard.
    ShardMapping(String),
    /// A supplied stream name or order differed from the pinned shard.
    ShardName { expected: String, actual: String },
    /// The eight-byte little-endian header length exceeded the hard bound.
    HeaderTooLarge { artifact: String, actual: u64 },
    /// The header length differed from the pinned file.
    HeaderLength {
        artifact: String,
        expected: u64,
        actual: u64,
    },
    /// The reader returned an I/O error.
    Io {
        artifact: String,
        kind: io::ErrorKind,
    },
    /// EOF arrived before the pinned file length.
    EarlyEof {
        artifact: String,
        expected: u64,
        actual: u64,
    },
    /// A byte followed the exact pinned file length.
    TrailingData(String),
    /// The streamed byte count differed from the pinned file size.
    FileSize {
        artifact: String,
        expected: u64,
        actual: u64,
    },
    /// A complete stream differed from its pinned full-file SHA-256.
    DigestMismatch(String),
    /// The exact target index bytes differed from the pinned SHA-256.
    IndexDigestMismatch,
}

impl fmt::Display for SafetensorsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArtifactSize(artifact) => write!(formatter, "{artifact} violates its size bound"),
            Self::InvalidJson {
                artifact,
                offset,
                reason,
            } => write!(
                formatter,
                "invalid {artifact} JSON at byte {offset}: {reason}"
            ),
            Self::MissingField { artifact, field } => {
                write!(formatter, "{artifact} is missing field {field:?}")
            }
            Self::UnknownField { artifact, field } => {
                write!(formatter, "{artifact} has unknown field {field:?}")
            }
            Self::WrongType { artifact, field } => {
                write!(formatter, "{artifact} field {field:?} has the wrong type")
            }
            Self::UnexpectedValue { artifact, field } => {
                write!(formatter, "{artifact} field {field:?} is not canonical")
            }
            Self::InvalidTensorName(tensor) => write!(formatter, "invalid tensor name {tensor:?}"),
            Self::UnexpectedDType(tensor) => write!(formatter, "tensor {tensor:?} is not BF16"),
            Self::TensorSchema { tensor, error } => {
                write!(
                    formatter,
                    "tensor {tensor:?} violates Qwen3 schema: {error}"
                )
            }
            Self::TensorSizeOverflow(tensor) => {
                write!(formatter, "tensor {tensor:?} byte size overflows")
            }
            Self::TensorByteLength(tensor) => {
                write!(
                    formatter,
                    "tensor {tensor:?} byte range does not match its shape"
                )
            }
            Self::InvalidOffsetRange(tensor) => {
                write!(formatter, "tensor {tensor:?} has an invalid offset range")
            }
            Self::OffsetGap { expected, actual } => {
                write!(
                    formatter,
                    "tensor offsets contain a gap at {expected}, next is {actual}"
                )
            }
            Self::OffsetOverlap { expected, actual } => write!(
                formatter,
                "tensor offsets overlap at {actual}, prior end is {expected}"
            ),
            Self::TensorDataBytes { expected, actual } => write!(
                formatter,
                "tensor-data length is {actual}, expected {expected}"
            ),
            Self::TensorCount { expected, actual } => {
                write!(formatter, "tensor count is {actual}, expected {expected}")
            }
            Self::TensorOrder { expected, actual } => write!(
                formatter,
                "tensor ordinal is {actual}, expected canonical ordinal {expected}"
            ),
            Self::DuplicateTensor(tensor) => write!(formatter, "duplicate tensor {tensor:?}"),
            Self::ShardMapping(tensor) => {
                write!(
                    formatter,
                    "target index maps tensor {tensor:?} to the wrong shard"
                )
            }
            Self::ShardName { expected, actual } => {
                write!(formatter, "shard is {actual:?}, expected {expected:?}")
            }
            Self::HeaderTooLarge { artifact, actual } => {
                write!(
                    formatter,
                    "{artifact} header length {actual} exceeds the bound"
                )
            }
            Self::HeaderLength {
                artifact,
                expected,
                actual,
            } => write!(
                formatter,
                "{artifact} header length is {actual}, expected {expected}"
            ),
            Self::Io { artifact, kind } => {
                write!(formatter, "I/O error reading {artifact}: {kind}")
            }
            Self::EarlyEof {
                artifact,
                expected,
                actual,
            } => write!(
                formatter,
                "{artifact} ended at byte {actual}, expected {expected} bytes"
            ),
            Self::TrailingData(artifact) => write!(formatter, "{artifact} has trailing data"),
            Self::FileSize {
                artifact,
                expected,
                actual,
            } => write!(
                formatter,
                "{artifact} has {actual} bytes, expected {expected}"
            ),
            Self::DigestMismatch(artifact) => {
                write!(formatter, "{artifact} full-file SHA-256 mismatched")
            }
            Self::IndexDigestMismatch => formatter.write_str("target index SHA-256 mismatched"),
        }
    }
}

impl std::error::Error for SafetensorsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TensorSchema { error, .. } => Some(error),
            _ => None,
        }
    }
}

/// Streams and authenticates all five exact Qwen3-8B weight shards.
///
/// # Errors
///
/// Returns [`SafetensorsError`] unless the exact pinned index and every shard
/// pass closed-schema validation, full-file SHA-256, exact size, and EOF.
pub fn authenticate_qwen3_target_weights<R: Read>(
    index_json: &[u8],
    mut shards: [SafetensorsSource<'_, R>; 5],
) -> Result<AuthenticatedWeightSet, SafetensorsError> {
    let index = parse_target_index(index_json, true)?;
    let mut parsed = Vec::with_capacity(TARGET_SHARD_PINS.len());
    for (shard_index, (source, pin)) in shards.iter_mut().zip(TARGET_SHARD_PINS).enumerate() {
        require_shard_name(source.name, pin.name)?;
        let shard = authenticate_file(&mut source.reader, pin, |header, tensor_data_bytes| {
            let shard = parse_header(
                Qwen3ModelRole::Target8B,
                pin.name,
                header,
                tensor_data_bytes,
            )?;
            validate_shard_mapping(&index, shard_index, &shard)?;
            Ok(shard)
        })?;
        parsed.push(shard);
    }
    validate_roster(Qwen3ModelRole::Target8B, &parsed)?;
    Ok(AuthenticatedWeightSet {
        role: Qwen3ModelRole::Target8B,
        descriptor: WeightDescriptor {
            weights_id: QWEN3_TARGET_WEIGHT_SET_SHA256,
            artifact_bytes: QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
            tensor_data_bytes: QWEN3_TARGET_TENSOR_DATA_BYTES,
            sections: 5,
        },
        seal: AuthenticatedSeal,
    })
}

/// Streams and authenticates the exact Qwen3-0.6B weight file.
///
/// # Errors
///
/// Returns [`SafetensorsError`] unless the complete pinned file passes
/// closed-schema validation, full-file SHA-256, exact size, and EOF.
pub fn authenticate_qwen3_draft_weights<R: Read>(
    mut source: SafetensorsSource<'_, R>,
) -> Result<AuthenticatedWeightSet, SafetensorsError> {
    require_shard_name(source.name, DRAFT_FILE_PIN.name)?;
    let parsed = authenticate_file(
        &mut source.reader,
        DRAFT_FILE_PIN,
        |header, tensor_data_bytes| {
            parse_header(
                Qwen3ModelRole::Draft06B,
                DRAFT_FILE_PIN.name,
                header,
                tensor_data_bytes,
            )
        },
    )?;
    validate_roster(Qwen3ModelRole::Draft06B, &[parsed])?;
    Ok(AuthenticatedWeightSet {
        role: Qwen3ModelRole::Draft06B,
        descriptor: WeightDescriptor {
            weights_id: QWEN3_DRAFT_WEIGHT_SHA256,
            artifact_bytes: QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
            tensor_data_bytes: QWEN3_DRAFT_TENSOR_DATA_BYTES,
            sections: 1,
        },
        seal: AuthenticatedSeal,
    })
}

#[derive(Clone, Copy)]
pub(super) struct FilePin {
    pub(super) name: &'static str,
    pub(super) sha256: [u8; 32],
    pub(super) file_bytes: u64,
    pub(super) header_bytes: u64,
    pub(super) tensor_data_bytes: u64,
}

pub(super) struct ParsedTensor {
    pub(super) name: String,
    pub(super) start: u64,
    pub(super) end: u64,
    pub(super) ordinal: u32,
    pub(super) metadata: Qwen3TensorMetadata,
}

pub(super) struct ParsedShard {
    pub(super) tensors: Vec<ParsedTensor>,
    pub(super) tensor_data_bytes: u64,
}

#[derive(Debug)]
pub(super) struct TargetIndex {
    shard_by_tensor: BTreeMap<String, usize>,
}

fn authenticate_file<R, F>(
    reader: &mut R,
    pin: FilePin,
    validate_header: F,
) -> Result<ParsedShard, SafetensorsError>
where
    R: Read,
    F: FnOnce(&[u8], u64) -> Result<ParsedShard, SafetensorsError>,
{
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut length_bytes = [0; 8];
    read_exact_hashed(
        reader,
        pin.name,
        &mut length_bytes,
        &mut hasher,
        &mut total,
        pin.file_bytes,
    )?;
    let header_bytes = u64::from_le_bytes(length_bytes);
    if header_bytes > MAX_SAFETENSORS_HEADER_BYTES {
        return Err(SafetensorsError::HeaderTooLarge {
            artifact: pin.name.to_owned(),
            actual: header_bytes,
        });
    }
    if header_bytes != pin.header_bytes {
        return Err(SafetensorsError::HeaderLength {
            artifact: pin.name.to_owned(),
            expected: pin.header_bytes,
            actual: header_bytes,
        });
    }
    let header_len =
        usize::try_from(header_bytes).map_err(|_| SafetensorsError::HeaderTooLarge {
            artifact: pin.name.to_owned(),
            actual: header_bytes,
        })?;
    let mut header = vec![0; header_len];
    read_exact_hashed(
        reader,
        pin.name,
        &mut header,
        &mut hasher,
        &mut total,
        pin.file_bytes,
    )?;
    let parsed = validate_header(&header, pin.tensor_data_bytes)?;

    let mut remaining = pin.tensor_data_bytes;
    let mut buffer = vec![0; STREAM_BUFFER_BYTES].into_boxed_slice();
    while remaining != 0 {
        let chunk = usize::try_from(remaining.min(STREAM_BUFFER_BYTES as u64))
            .expect("bounded stream chunk fits usize");
        read_exact_hashed(
            reader,
            pin.name,
            &mut buffer[..chunk],
            &mut hasher,
            &mut total,
            pin.file_bytes,
        )?;
        remaining -= u64::try_from(chunk).expect("stream chunk fits u64");
    }
    let mut trailing = [0; 1];
    match reader.read(&mut trailing) {
        Ok(0) => {}
        Ok(read) => {
            hasher.update(&trailing[..read]);
            return Err(SafetensorsError::TrailingData(pin.name.to_owned()));
        }
        Err(error) => return Err(io_error(pin.name, error)),
    }
    if total != pin.file_bytes {
        return Err(SafetensorsError::FileSize {
            artifact: pin.name.to_owned(),
            expected: pin.file_bytes,
            actual: total,
        });
    }
    if hasher.finish() != pin.sha256 {
        return Err(SafetensorsError::DigestMismatch(pin.name.to_owned()));
    }
    Ok(parsed)
}

fn read_exact_hashed<R: Read>(
    reader: &mut R,
    artifact: &str,
    buffer: &mut [u8],
    hasher: &mut Sha256,
    total: &mut u64,
    expected_total: u64,
) -> Result<(), SafetensorsError> {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => {
                return Err(SafetensorsError::EarlyEof {
                    artifact: artifact.to_owned(),
                    expected: expected_total,
                    actual: *total,
                });
            }
            Ok(read) => {
                hasher.update(&buffer[filled..filled + read]);
                filled += read;
                *total = total
                    .checked_add(u64::try_from(read).expect("read length fits u64"))
                    .ok_or_else(|| SafetensorsError::FileSize {
                        artifact: artifact.to_owned(),
                        expected: expected_total,
                        actual: u64::MAX,
                    })?;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(io_error(artifact, error)),
        }
    }
    Ok(())
}

fn io_error(artifact: &str, error: io::Error) -> SafetensorsError {
    SafetensorsError::Io {
        artifact: artifact.to_owned(),
        kind: error.kind(),
    }
}

pub(super) fn parse_target_index(
    bytes: &[u8],
    authenticate_bytes: bool,
) -> Result<TargetIndex, SafetensorsError> {
    let artifact = "model.safetensors.index.json";
    let value = parse_json(artifact, bytes, MAX_INDEX_BYTES)?;
    let mut root = Fields::new(artifact, object(artifact, "$", value)?);
    let mut metadata = Fields::new(
        artifact,
        object(artifact, "metadata", root.take("metadata")?)?,
    );
    metadata.expect_u64("total_size", QWEN3_TARGET_TENSOR_DATA_BYTES)?;
    metadata.finish()?;
    let weight_map = object(artifact, "weight_map", root.take("weight_map")?)?;
    root.finish()?;

    let expected_count = Qwen3ModelRole::Target8B.tensor_count();
    let actual_count = u32::try_from(weight_map.len()).unwrap_or(u32::MAX);
    if actual_count != expected_count {
        return Err(SafetensorsError::TensorCount {
            expected: expected_count,
            actual: actual_count,
        });
    }
    let mut ordinals = BTreeSet::new();
    let mut shard_by_tensor = BTreeMap::new();
    for (tensor, value) in weight_map {
        let (_, _, ordinal) = classify_tensor_name(Qwen3ModelRole::Target8B, &tensor)?;
        if !ordinals.insert(ordinal) {
            return Err(SafetensorsError::DuplicateTensor(tensor));
        }
        let Value::String(shard_name) = value else {
            return Err(wrong_type(artifact, &format!("weight_map.{tensor}")));
        };
        let shard_index = TARGET_SHARD_PINS
            .iter()
            .position(|pin| pin.name == shard_name)
            .ok_or_else(|| unexpected(artifact, &format!("weight_map.{tensor}")))?;
        shard_by_tensor.insert(tensor, shard_index);
    }
    validate_ordinals(&ordinals, expected_count)?;
    if authenticate_bytes
        && (bytes.len() != TARGET_INDEX_BYTES
            || crate::sha256::digest(bytes) != TARGET_INDEX_SHA256)
    {
        return Err(SafetensorsError::IndexDigestMismatch);
    }
    Ok(TargetIndex { shard_by_tensor })
}

pub(super) fn parse_header(
    role: Qwen3ModelRole,
    artifact: &str,
    bytes: &[u8],
    expected_tensor_data_bytes: u64,
) -> Result<ParsedShard, SafetensorsError> {
    let value = parse_json(
        artifact,
        bytes,
        usize::try_from(MAX_SAFETENSORS_HEADER_BYTES).expect("header bound fits usize"),
    )?;
    let mut tensors = object(artifact, "$", value)?;
    let metadata_value =
        tensors
            .remove("__metadata__")
            .ok_or_else(|| SafetensorsError::MissingField {
                artifact: artifact.to_owned(),
                field: "__metadata__".to_owned(),
            })?;
    let mut metadata = Fields::new(artifact, object(artifact, "__metadata__", metadata_value)?);
    metadata.expect_string("format", "pt")?;
    metadata.finish()?;

    let mut parsed = Vec::with_capacity(tensors.len());
    for (name, value) in tensors {
        parsed.push(parse_tensor(role, artifact, name, value)?);
    }
    parsed.sort_by_key(|tensor| tensor.start);
    validate_offset_coverage(&parsed, expected_tensor_data_bytes)?;
    Ok(ParsedShard {
        tensors: parsed,
        tensor_data_bytes: expected_tensor_data_bytes,
    })
}

fn parse_tensor(
    role: Qwen3ModelRole,
    artifact: &str,
    name: String,
    value: Value,
) -> Result<ParsedTensor, SafetensorsError> {
    let (kind, layer, ordinal) = classify_tensor_name(role, &name)?;
    let mut fields = Fields::new(artifact, object(artifact, &name, value)?);
    if fields.string("dtype")? != "BF16" {
        return Err(SafetensorsError::UnexpectedDType(name));
    }
    let shape = u32_array(artifact, &format!("{name}.shape"), fields.take("shape")?)?;
    let offsets = u64_array(
        artifact,
        &format!("{name}.data_offsets"),
        fields.take("data_offsets")?,
    )?;
    fields.finish()?;
    if offsets.len() != 2 {
        return Err(unexpected(artifact, &format!("{name}.data_offsets")));
    }
    let start = offsets[0];
    let end = offsets[1];
    if end <= start {
        return Err(SafetensorsError::InvalidOffsetRange(name));
    }
    let rank = u32::try_from(shape.len()).unwrap_or(u32::MAX);
    let dimensions = (
        shape.first().copied().unwrap_or(1),
        shape.get(1).copied().unwrap_or(1),
    );
    let tensor = Qwen3TensorMetadata {
        role,
        kind,
        layer,
        dtype: TensorDType::Bf16,
        rank,
        dimension_0: dimensions.0,
        dimension_1: dimensions.1,
    };
    let expected_bytes = shape.iter().try_fold(2_u64, |bytes, dimension| {
        bytes.checked_mul(u64::from(*dimension))
    });
    let Some(expected_bytes) = expected_bytes else {
        return Err(SafetensorsError::TensorSizeOverflow(name));
    };
    tensor
        .validate()
        .map_err(|error| SafetensorsError::TensorSchema {
            tensor: name.clone(),
            error,
        })?;
    if end - start != expected_bytes {
        return Err(SafetensorsError::TensorByteLength(name));
    }
    Ok(ParsedTensor {
        name,
        start,
        end,
        ordinal,
        metadata: tensor,
    })
}

pub(super) fn classify_tensor_name(
    role: Qwen3ModelRole,
    name: &str,
) -> Result<(Qwen3TensorKind, u32, u32), SafetensorsError> {
    let global = match name {
        "lm_head.weight" => Some(Qwen3TensorKind::LanguageModelHead),
        "model.embed_tokens.weight" => Some(Qwen3TensorKind::TokenEmbedding),
        "model.norm.weight" => Some(Qwen3TensorKind::FinalNorm),
        _ => None,
    };
    if let Some(kind) = global {
        return Ok((kind, QWEN3_NO_LAYER, global_ordinal(role, kind)));
    }
    let Some(layer_suffix) = name.strip_prefix("model.layers.") else {
        return Err(SafetensorsError::InvalidTensorName(name.to_owned()));
    };
    let Some((layer_text, suffix)) = layer_suffix.split_once('.') else {
        return Err(SafetensorsError::InvalidTensorName(name.to_owned()));
    };
    let layer = layer_text
        .parse::<u32>()
        .map_err(|_| SafetensorsError::InvalidTensorName(name.to_owned()))?;
    if layer.to_string() != layer_text {
        return Err(SafetensorsError::InvalidTensorName(name.to_owned()));
    }
    let (kind, kind_ordinal) = match suffix {
        "input_layernorm.weight" => (Qwen3TensorKind::InputLayerNorm, 0),
        "mlp.down_proj.weight" => (Qwen3TensorKind::DownProjection, 1),
        "mlp.gate_proj.weight" => (Qwen3TensorKind::GateProjection, 2),
        "mlp.up_proj.weight" => (Qwen3TensorKind::UpProjection, 3),
        "post_attention_layernorm.weight" => (Qwen3TensorKind::PostAttentionLayerNorm, 4),
        "self_attn.k_norm.weight" => (Qwen3TensorKind::KeyNorm, 5),
        "self_attn.k_proj.weight" => (Qwen3TensorKind::KeyProjection, 6),
        "self_attn.o_proj.weight" => (Qwen3TensorKind::OutputProjection, 7),
        "self_attn.q_norm.weight" => (Qwen3TensorKind::QueryNorm, 8),
        "self_attn.q_proj.weight" => (Qwen3TensorKind::QueryProjection, 9),
        "self_attn.v_proj.weight" => (Qwen3TensorKind::ValueProjection, 10),
        _ => return Err(SafetensorsError::InvalidTensorName(name.to_owned())),
    };
    if layer >= role.layers() {
        return Err(SafetensorsError::TensorSchema {
            tensor: name.to_owned(),
            error: Qwen3TensorError::UnexpectedLayer,
        });
    }
    let base = match role {
        Qwen3ModelRole::Target8B => 1,
        Qwen3ModelRole::Draft06B => 2,
    };
    Ok((kind, layer, base + layer * 11 + kind_ordinal))
}

fn global_ordinal(role: Qwen3ModelRole, kind: Qwen3TensorKind) -> u32 {
    match (role, kind) {
        (Qwen3ModelRole::Target8B, Qwen3TensorKind::TokenEmbedding)
        | (Qwen3ModelRole::Draft06B, Qwen3TensorKind::LanguageModelHead) => 0,
        (Qwen3ModelRole::Target8B, Qwen3TensorKind::FinalNorm) => 397,
        (Qwen3ModelRole::Target8B, Qwen3TensorKind::LanguageModelHead) => 398,
        (Qwen3ModelRole::Draft06B, Qwen3TensorKind::TokenEmbedding) => 1,
        (Qwen3ModelRole::Draft06B, Qwen3TensorKind::FinalNorm) => 310,
        _ => u32::MAX,
    }
}

pub(super) fn validate_offset_coverage(
    tensors: &[ParsedTensor],
    expected_tensor_data_bytes: u64,
) -> Result<(), SafetensorsError> {
    let mut expected = 0_u64;
    for tensor in tensors {
        if tensor.start > expected {
            return Err(SafetensorsError::OffsetGap {
                expected,
                actual: tensor.start,
            });
        }
        if tensor.start < expected {
            return Err(SafetensorsError::OffsetOverlap {
                expected,
                actual: tensor.start,
            });
        }
        expected = tensor.end;
    }
    if expected != expected_tensor_data_bytes {
        return Err(SafetensorsError::TensorDataBytes {
            expected: expected_tensor_data_bytes,
            actual: expected,
        });
    }
    Ok(())
}

pub(super) fn validate_roster(
    role: Qwen3ModelRole,
    shards: &[ParsedShard],
) -> Result<(), SafetensorsError> {
    let mut names = BTreeSet::new();
    let mut ordinals = BTreeSet::new();
    let mut tensor_data_bytes = 0_u64;
    for shard in shards {
        tensor_data_bytes = tensor_data_bytes
            .checked_add(shard.tensor_data_bytes)
            .ok_or(SafetensorsError::TensorDataBytes {
                expected: role.tensor_data_bytes(),
                actual: u64::MAX,
            })?;
        for tensor in &shard.tensors {
            if !names.insert(tensor.name.clone()) {
                return Err(SafetensorsError::DuplicateTensor(tensor.name.clone()));
            }
            if !ordinals.insert(tensor.ordinal) {
                return Err(SafetensorsError::TensorOrder {
                    expected: u32::MAX,
                    actual: tensor.ordinal,
                });
            }
        }
    }
    let actual_count = u32::try_from(names.len()).unwrap_or(u32::MAX);
    if actual_count != role.tensor_count() {
        return Err(SafetensorsError::TensorCount {
            expected: role.tensor_count(),
            actual: actual_count,
        });
    }
    validate_ordinals(&ordinals, role.tensor_count())?;
    if tensor_data_bytes != role.tensor_data_bytes() {
        return Err(SafetensorsError::TensorDataBytes {
            expected: role.tensor_data_bytes(),
            actual: tensor_data_bytes,
        });
    }
    Ok(())
}

pub(super) fn validate_shard_mapping(
    index: &TargetIndex,
    shard_index: usize,
    shard: &ParsedShard,
) -> Result<(), SafetensorsError> {
    for tensor in &shard.tensors {
        if index.shard_by_tensor.get(&tensor.name) != Some(&shard_index) {
            return Err(SafetensorsError::ShardMapping(tensor.name.clone()));
        }
    }
    Ok(())
}

fn validate_ordinals(
    ordinals: &BTreeSet<u32>,
    expected_count: u32,
) -> Result<(), SafetensorsError> {
    for expected in 0..expected_count {
        if !ordinals.contains(&expected) {
            return Err(SafetensorsError::TensorOrder {
                expected,
                actual: u32::MAX,
            });
        }
    }
    Ok(())
}

pub(super) fn require_shard_name(actual: &str, expected: &str) -> Result<(), SafetensorsError> {
    if actual != expected {
        return Err(SafetensorsError::ShardName {
            expected: expected.to_owned(),
            actual: actual.to_owned(),
        });
    }
    Ok(())
}

fn parse_json(artifact: &str, bytes: &[u8], max_bytes: usize) -> Result<Value, SafetensorsError> {
    if bytes.is_empty() || bytes.len() > max_bytes {
        return Err(SafetensorsError::ArtifactSize(artifact.to_owned()));
    }
    crate::json::parse(bytes).map_err(|error| SafetensorsError::InvalidJson {
        artifact: artifact.to_owned(),
        offset: error.offset,
        reason: error.kind.to_string(),
    })
}

fn object(
    artifact: &str,
    field: &str,
    value: Value,
) -> Result<BTreeMap<String, Value>, SafetensorsError> {
    if let Value::Object(fields) = value {
        Ok(fields)
    } else {
        Err(wrong_type(artifact, field))
    }
}

fn u32_array(artifact: &str, field: &str, value: Value) -> Result<Vec<u32>, SafetensorsError> {
    let Value::Array(values) = value else {
        return Err(wrong_type(artifact, field));
    };
    values
        .into_iter()
        .map(|value| number_u32(artifact, field, value))
        .collect()
}

fn u64_array(artifact: &str, field: &str, value: Value) -> Result<Vec<u64>, SafetensorsError> {
    let Value::Array(values) = value else {
        return Err(wrong_type(artifact, field));
    };
    values
        .into_iter()
        .map(|value| number_u64(artifact, field, value))
        .collect()
}

fn number_u32(artifact: &str, field: &str, value: Value) -> Result<u32, SafetensorsError> {
    let Value::Number(number) = value else {
        return Err(wrong_type(artifact, field));
    };
    number.parse().map_err(|_| unexpected(artifact, field))
}

fn number_u64(artifact: &str, field: &str, value: Value) -> Result<u64, SafetensorsError> {
    let Value::Number(number) = value else {
        return Err(wrong_type(artifact, field));
    };
    number.parse().map_err(|_| unexpected(artifact, field))
}

struct Fields<'a> {
    artifact: &'a str,
    fields: BTreeMap<String, Value>,
}

impl<'a> Fields<'a> {
    fn new(artifact: &'a str, fields: BTreeMap<String, Value>) -> Self {
        Self { artifact, fields }
    }

    fn take(&mut self, field: &str) -> Result<Value, SafetensorsError> {
        self.fields
            .remove(field)
            .ok_or_else(|| SafetensorsError::MissingField {
                artifact: self.artifact.to_owned(),
                field: field.to_owned(),
            })
    }

    fn string(&mut self, field: &str) -> Result<String, SafetensorsError> {
        if let Value::String(value) = self.take(field)? {
            Ok(value)
        } else {
            Err(wrong_type(self.artifact, field))
        }
    }

    fn expect_string(&mut self, field: &str, expected: &str) -> Result<(), SafetensorsError> {
        if self.string(field)? != expected {
            return Err(unexpected(self.artifact, field));
        }
        Ok(())
    }

    fn expect_u64(&mut self, field: &str, expected: u64) -> Result<(), SafetensorsError> {
        if number_u64(self.artifact, field, self.take(field)?)? != expected {
            return Err(unexpected(self.artifact, field));
        }
        Ok(())
    }

    fn finish(self) -> Result<(), SafetensorsError> {
        if let Some(field) = self.fields.into_keys().next() {
            return Err(SafetensorsError::UnknownField {
                artifact: self.artifact.to_owned(),
                field,
            });
        }
        Ok(())
    }
}

fn wrong_type(artifact: &str, field: &str) -> SafetensorsError {
    SafetensorsError::WrongType {
        artifact: artifact.to_owned(),
        field: field.to_owned(),
    }
}

fn unexpected(artifact: &str, field: &str) -> SafetensorsError {
    SafetensorsError::UnexpectedValue {
        artifact: artifact.to_owned(),
        field: field.to_owned(),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        authenticate_file, parse_header, parse_target_index, require_shard_name, validate_roster,
        validate_shard_mapping, AuthenticatedSeal, AuthenticatedWeightSet, FilePin, ParsedShard,
        SafetensorsError, DRAFT_FILE_PIN, TARGET_SHARD_PINS,
    };
    use crate::{
        build_weight_authenticated_deployment_bundle, ArtifactDigest, BuildError,
        WeightAuthenticatedDeploymentAssets, WeightAuthenticatedModelAssets, WeightDescriptor,
        DRAFT_REPOSITORY, DRAFT_REVISION, QWEN3_DRAFT_TENSOR_DATA_BYTES,
        QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES, QWEN3_DRAFT_WEIGHT_SHA256,
        QWEN3_TARGET_TENSOR_DATA_BYTES, QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
        QWEN3_TARGET_WEIGHT_SET_SHA256, QWEN3_TOKENIZER_BYTES, QWEN3_TOKENIZER_SHA256,
        TARGET_REPOSITORY, TARGET_REVISION,
    };
    use ferric_spec::{EngineLimits, Qwen3ModelRole};
    use std::io::{Cursor, Read};

    const TARGET_INDEX: &[u8] = include_bytes!("fixtures/safetensors/qwen3-8b-index.json");
    const TARGET_HEADERS: [&[u8]; 5] = [
        include_bytes!("fixtures/safetensors/qwen3-8b-00001.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00002.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00003.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00004.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00005.header.json"),
    ];
    const DRAFT_HEADER: &[u8] = include_bytes!("fixtures/safetensors/qwen3-06b.header.json");
    const TARGET_CONFIG: &[u8] = include_bytes!("fixtures/qwen3-8b-config.json");
    const DRAFT_CONFIG: &[u8] = include_bytes!("fixtures/qwen3-06b-config.json");
    const TOKENIZER_METADATA: &[u8] = include_bytes!("fixtures/qwen3-tokenizer-config.json");
    const TARGET_HEADER_SHA256: [[u8; 32]; 5] = [
        crate::decode_hex_32(b"979bbeed365485ddaa67a1ed41d0289e15e2f3ba0b3388cb93e42d31f346d1df"),
        crate::decode_hex_32(b"347ca6985eefe87273139b97dde9c547da3ab59c782a73beac1d498556ac0b45"),
        crate::decode_hex_32(b"2c34e283f27490d335b129053f9c50504a357ba8b67a2e89fcf5d39cdd85f6f4"),
        crate::decode_hex_32(b"3f2742edfb110486c05a06b091280eefb7d2bc05252d1b78f1233c1a813a48e7"),
        crate::decode_hex_32(b"205a19ce27198b75abaeedc42f7a64387e19628b7804aff215887b255f1b8dd7"),
    ];
    const DRAFT_HEADER_SHA256: [u8; 32] =
        crate::decode_hex_32(b"399d16f500e925c7e923fe05966c6df6862ab64da60916843119e802f1801bca");

    fn padded_raw_header(fixture: &[u8], pin: FilePin) -> Vec<u8> {
        assert_eq!(fixture.last(), Some(&b'\n'));
        let mut header = fixture[..fixture.len() - 1].to_vec();
        let header_len = usize::try_from(pin.header_bytes).expect("bounded header length");
        assert!(header.len() <= header_len);
        header.resize(header_len, b' ');
        header
    }

    fn exact_raw_header(fixture: &[u8], pin: FilePin, expected_sha256: [u8; 32]) -> Vec<u8> {
        let header = padded_raw_header(fixture, pin);
        assert_eq!(crate::sha256::digest(&header), expected_sha256);
        header
    }

    fn target_headers() -> Vec<ParsedShard> {
        TARGET_HEADERS
            .iter()
            .zip(TARGET_SHARD_PINS.into_iter().zip(TARGET_HEADER_SHA256))
            .map(|(fixture, (pin, header_sha256))| {
                let header = exact_raw_header(fixture, pin, header_sha256);
                parse_header(
                    Qwen3ModelRole::Target8B,
                    pin.name,
                    &header,
                    pin.tensor_data_bytes,
                )
                .expect("official target header schema")
            })
            .collect()
    }

    fn draft_header(bytes: &[u8]) -> Result<ParsedShard, SafetensorsError> {
        parse_header(
            Qwen3ModelRole::Draft06B,
            DRAFT_FILE_PIN.name,
            bytes,
            DRAFT_FILE_PIN.tensor_data_bytes,
        )
    }

    fn replace_once(input: &[u8], from: &str, to: &str) -> Vec<u8> {
        let input = std::str::from_utf8(input).expect("fixture is UTF-8");
        let changed = input.replacen(from, to, 1);
        assert_ne!(changed, input);
        changed.into_bytes()
    }

    pub(crate) fn test_authenticated_weight_set(role: Qwen3ModelRole) -> AuthenticatedWeightSet {
        let descriptor = match role {
            Qwen3ModelRole::Target8B => WeightDescriptor {
                weights_id: QWEN3_TARGET_WEIGHT_SET_SHA256,
                artifact_bytes: QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
                tensor_data_bytes: QWEN3_TARGET_TENSOR_DATA_BYTES,
                sections: 5,
            },
            Qwen3ModelRole::Draft06B => WeightDescriptor {
                weights_id: QWEN3_DRAFT_WEIGHT_SHA256,
                artifact_bytes: QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
                tensor_data_bytes: QWEN3_DRAFT_TENSOR_DATA_BYTES,
                sections: 1,
            },
        };
        AuthenticatedWeightSet {
            role,
            descriptor,
            seal: AuthenticatedSeal,
        }
    }

    fn weight_authenticated_assets() -> WeightAuthenticatedDeploymentAssets<'static> {
        let vocabulary = ArtifactDigest {
            sha256: QWEN3_TOKENIZER_SHA256,
            byte_len: QWEN3_TOKENIZER_BYTES,
        };
        WeightAuthenticatedDeploymentAssets {
            target: WeightAuthenticatedModelAssets {
                repository: TARGET_REPOSITORY,
                revision: TARGET_REVISION,
                config_json: &TARGET_CONFIG[..TARGET_CONFIG.len() - 1],
                tokenizer_metadata_json: TOKENIZER_METADATA,
                vocabulary,
            },
            draft: WeightAuthenticatedModelAssets {
                repository: DRAFT_REPOSITORY,
                revision: DRAFT_REVISION,
                config_json: &DRAFT_CONFIG[..DRAFT_CONFIG.len() - 1],
                tokenizer_metadata_json: TOKENIZER_METADATA,
                vocabulary,
            },
            limits: EngineLimits {
                max_context_tokens: 8_192,
                max_active_sequences: 32,
                kv_page_tokens: 256,
                max_draft_tokens: 16,
            },
        }
    }

    #[test]
    fn official_index_and_headers_match_qwen3_spec() {
        let index = parse_target_index(TARGET_INDEX, true).expect("pinned target index");
        let target = target_headers();
        for (shard_index, shard) in target.iter().enumerate() {
            validate_shard_mapping(&index, shard_index, shard).expect("exact index mapping");
        }
        validate_roster(Qwen3ModelRole::Target8B, &target).expect("complete target roster");
        assert_eq!(
            target
                .iter()
                .map(|shard| shard.tensors.len())
                .sum::<usize>(),
            399
        );
        assert_eq!(
            target
                .iter()
                .map(|shard| shard.tensor_data_bytes)
                .sum::<u64>(),
            QWEN3_TARGET_TENSOR_DATA_BYTES
        );

        let draft_bytes = exact_raw_header(DRAFT_HEADER, DRAFT_FILE_PIN, DRAFT_HEADER_SHA256);
        let draft = draft_header(&draft_bytes).expect("official draft header schema");
        assert_eq!(draft.tensors.len(), 311);
        assert_eq!(draft.tensor_data_bytes, QWEN3_DRAFT_TENSOR_DATA_BYTES);
        validate_roster(Qwen3ModelRole::Draft06B, &[draft]).expect("complete draft roster");
    }

    #[test]
    fn weight_authenticated_builder_consumes_and_role_checks_authorities() {
        let bundle = build_weight_authenticated_deployment_bundle(
            weight_authenticated_assets(),
            test_authenticated_weight_set(Qwen3ModelRole::Target8B),
            test_authenticated_weight_set(Qwen3ModelRole::Draft06B),
        )
        .expect("weight-authenticated bundle");
        assert_eq!(
            bundle.target_model.weights.weights_id.as_bytes(),
            &QWEN3_TARGET_WEIGHT_SET_SHA256
        );

        assert_eq!(
            build_weight_authenticated_deployment_bundle(
                weight_authenticated_assets(),
                test_authenticated_weight_set(Qwen3ModelRole::Draft06B),
                test_authenticated_weight_set(Qwen3ModelRole::Target8B),
            ),
            Err(BuildError::AuthenticatedWeightRole {
                expected: Qwen3ModelRole::Target8B,
                actual: Qwen3ModelRole::Draft06B,
            })
        );
    }

    #[test]
    fn exact_pinned_header_lengths_and_padding_stream_to_eof() {
        for (fixture, (pin, header_sha256)) in TARGET_HEADERS
            .iter()
            .zip(TARGET_SHARD_PINS.into_iter().zip(TARGET_HEADER_SHA256))
            .chain(std::iter::once((
                &DRAFT_HEADER,
                (DRAFT_FILE_PIN, DRAFT_HEADER_SHA256),
            )))
        {
            let header = exact_raw_header(fixture, pin, header_sha256);
            let mut complete = pin.header_bytes.to_le_bytes().to_vec();
            complete.extend_from_slice(&header);
            let header_only_pin = FilePin {
                name: pin.name,
                sha256: crate::sha256::digest(&complete),
                file_bytes: 8 + pin.header_bytes,
                header_bytes: pin.header_bytes,
                tensor_data_bytes: 0,
            };
            let parsed = authenticate_file(
                &mut Cursor::new(complete),
                header_only_pin,
                |streamed_header, tensor_data_bytes| {
                    assert_eq!(streamed_header, header);
                    assert_eq!(tensor_data_bytes, 0);
                    Ok(ParsedShard {
                        tensors: Vec::new(),
                        tensor_data_bytes,
                    })
                },
            )
            .expect("exact reconstructed header stream");
            assert_eq!(parsed.tensor_data_bytes, 0);
        }
    }

    #[test]
    fn dtype_shape_name_and_layer_drift_are_rejected() {
        let dtype = replace_once(DRAFT_HEADER, r#""dtype":"BF16""#, r#""dtype":"F32""#);
        assert!(matches!(
            draft_header(&dtype),
            Err(SafetensorsError::UnexpectedDType(_))
        ));

        let shape = replace_once(
            DRAFT_HEADER,
            r#""shape":[151936,1024]"#,
            r#""shape":[151936,1025]"#,
        );
        assert!(matches!(
            draft_header(&shape),
            Err(SafetensorsError::TensorSchema { .. })
        ));

        let name = replace_once(DRAFT_HEADER, "lm_head.weight", "lm_head.bias");
        assert!(matches!(
            draft_header(&name),
            Err(SafetensorsError::InvalidTensorName(_))
        ));

        let layer = replace_once(
            DRAFT_HEADER,
            "model.layers.27.input_layernorm.weight",
            "model.layers.28.input_layernorm.weight",
        );
        assert!(matches!(
            draft_header(&layer),
            Err(SafetensorsError::TensorSchema { .. })
        ));
    }

    #[test]
    fn duplicate_missing_unknown_and_overflow_fail_closed() {
        let input = std::str::from_utf8(DRAFT_HEADER).expect("fixture is UTF-8");
        let duplicate = format!(
            r#"{{"lm_head.weight":{{"dtype":"BF16","shape":[151936,1024],"data_offsets":[0,311164928]}},{}"#,
            &input[1..]
        );
        assert!(matches!(
            draft_header(duplicate.as_bytes()),
            Err(SafetensorsError::InvalidJson { ref reason, .. })
                if reason.contains("duplicate field")
        ));

        let mut missing = draft_header(DRAFT_HEADER).expect("official draft header");
        missing.tensors.remove(0);
        assert!(matches!(
            validate_roster(Qwen3ModelRole::Draft06B, &[missing]),
            Err(SafetensorsError::TensorCount { .. })
        ));

        let unknown = replace_once(
            DRAFT_HEADER,
            r#""dtype":"BF16""#,
            r#""dtype":"BF16","future":0"#,
        );
        assert!(matches!(
            draft_header(&unknown),
            Err(SafetensorsError::UnknownField { .. })
        ));

        let overflow = replace_once(
            DRAFT_HEADER,
            r#""shape":[151936,1024]"#,
            r#""shape":[4294967295,4294967295]"#,
        );
        assert!(matches!(
            draft_header(&overflow),
            Err(SafetensorsError::TensorSizeOverflow(_))
        ));
    }

    #[test]
    fn gaps_overlaps_invalid_ranges_and_reordering_are_rejected() {
        let gap = replace_once(
            DRAFT_HEADER,
            r#""data_offsets":[0,311164928]"#,
            r#""data_offsets":[2,311164930]"#,
        );
        assert!(matches!(
            draft_header(&gap),
            Err(SafetensorsError::OffsetGap { .. })
        ));

        let overlap = replace_once(
            DRAFT_HEADER,
            r#""data_offsets":[311164928,622329856]"#,
            r#""data_offsets":[311164926,622329854]"#,
        );
        assert!(matches!(
            draft_header(&overlap),
            Err(SafetensorsError::OffsetOverlap { .. })
        ));

        let reversed = replace_once(
            DRAFT_HEADER,
            r#""data_offsets":[0,311164928]"#,
            r#""data_offsets":[2,1]"#,
        );
        assert!(matches!(
            draft_header(&reversed),
            Err(SafetensorsError::InvalidOffsetRange(_))
        ));

        let reordered = replace_once(
            DRAFT_HEADER,
            r#""data_offsets":[0,311164928]"#,
            r#""data_offsets":[9,10]"#,
        );
        let reordered = replace_once(
            &reordered,
            r#""data_offsets":[311164928,622329856]"#,
            r#""data_offsets":[0,311164928]"#,
        );
        let reordered = replace_once(
            &reordered,
            r#""data_offsets":[9,10]"#,
            r#""data_offsets":[311164928,622329856]"#,
        );
        let parsed = draft_header(&reordered).expect("offset coverage remains complete");
        validate_roster(Qwen3ModelRole::Draft06B, &[parsed])
            .expect("tensor order is authenticated by the pinned byte digest");
        let canonical_header = exact_raw_header(DRAFT_HEADER, DRAFT_FILE_PIN, DRAFT_HEADER_SHA256);
        let reordered_header = padded_raw_header(&reordered, DRAFT_FILE_PIN);
        let mut canonical_file = DRAFT_FILE_PIN.header_bytes.to_le_bytes().to_vec();
        canonical_file.extend_from_slice(&canonical_header);
        let header_only_pin = FilePin {
            name: DRAFT_FILE_PIN.name,
            sha256: crate::sha256::digest(&canonical_file),
            file_bytes: 8 + DRAFT_FILE_PIN.header_bytes,
            header_bytes: DRAFT_FILE_PIN.header_bytes,
            tensor_data_bytes: 0,
        };
        let mut reordered_file = DRAFT_FILE_PIN.header_bytes.to_le_bytes().to_vec();
        reordered_file.extend_from_slice(&reordered_header);
        assert!(matches!(
            authenticate_file(
                &mut Cursor::new(reordered_file),
                header_only_pin,
                |_, tensor_data_bytes| Ok(ParsedShard {
                    tensors: Vec::new(),
                    tensor_data_bytes,
                }),
            ),
            Err(SafetensorsError::DigestMismatch(_))
        ));
    }

    #[test]
    fn index_mapping_and_shard_order_are_exact() {
        let index_text = std::str::from_utf8(TARGET_INDEX).expect("fixture is UTF-8");
        let changed = index_text.replacen(
            r#""model.embed_tokens.weight": "model-00001-of-00005.safetensors""#,
            r#""model.embed_tokens.weight": "model-00002-of-00005.safetensors""#,
            1,
        );
        assert_ne!(changed, index_text);
        let index =
            parse_target_index(changed.as_bytes(), false).expect("schema-valid changed index");
        let target = target_headers();
        assert!(matches!(
            validate_shard_mapping(&index, 0, &target[0]),
            Err(SafetensorsError::ShardMapping(_))
        ));

        let mut swapped = target_headers();
        swapped.swap(0, 1);
        assert!(matches!(
            validate_shard_mapping(&index, 0, &swapped[0]),
            Err(SafetensorsError::ShardMapping(_))
        ));
        assert!(matches!(
            require_shard_name(TARGET_SHARD_PINS[1].name, TARGET_SHARD_PINS[0].name),
            Err(SafetensorsError::ShardName { .. })
        ));
    }

    #[test]
    fn changed_index_bytes_fail_authentication_after_schema() {
        let mut changed = TARGET_INDEX.to_vec();
        changed.extend_from_slice(b" ");
        assert_eq!(
            parse_target_index(&changed, true).expect_err("raw index identity must change"),
            SafetensorsError::IndexDigestMismatch
        );
    }

    struct ChunkedReader {
        cursor: Cursor<Vec<u8>>,
        max_chunk: usize,
    }

    impl Read for ChunkedReader {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let chunk = buffer.len().min(self.max_chunk);
            self.cursor.read(&mut buffer[..chunk])
        }
    }

    fn synthetic_file(header: &[u8], data: &[u8]) -> (Vec<u8>, FilePin) {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(
            &u64::try_from(header.len())
                .expect("test header length fits u64")
                .to_le_bytes(),
        );
        bytes.extend_from_slice(header);
        bytes.extend_from_slice(data);
        let pin = FilePin {
            name: "tiny.safetensors",
            sha256: crate::sha256::digest(&bytes),
            file_bytes: u64::try_from(bytes.len()).expect("test file length fits u64"),
            header_bytes: u64::try_from(header.len()).expect("test header length fits u64"),
            tensor_data_bytes: u64::try_from(data.len()).expect("test data length fits u64"),
        };
        (bytes, pin)
    }

    fn accept_tiny_header(
        header: &[u8],
        tensor_data_bytes: u64,
    ) -> Result<ParsedShard, SafetensorsError> {
        if header != b"{}" {
            return Err(SafetensorsError::UnexpectedValue {
                artifact: "tiny.safetensors".to_owned(),
                field: "header".to_owned(),
            });
        }
        Ok(ParsedShard {
            tensors: Vec::new(),
            tensor_data_bytes,
        })
    }

    #[test]
    fn streaming_authenticator_hashes_chunked_input_through_eof() {
        let (bytes, pin) = synthetic_file(b"{}", b"authenticated payload");
        let mut reader = ChunkedReader {
            cursor: Cursor::new(bytes),
            max_chunk: 3,
        };
        let parsed =
            authenticate_file(&mut reader, pin, accept_tiny_header).expect("complete exact stream");
        assert_eq!(parsed.tensor_data_bytes, 21);
    }

    #[test]
    fn header_length_truncation_trailing_and_bit_flip_are_rejected() {
        let (bytes, pin) = synthetic_file(b"{}", b"authenticated payload");

        let mut too_large = bytes.clone();
        too_large[..8].copy_from_slice(&(super::MAX_SAFETENSORS_HEADER_BYTES + 1).to_le_bytes());
        assert!(matches!(
            authenticate_file(&mut Cursor::new(too_large), pin, accept_tiny_header),
            Err(SafetensorsError::HeaderTooLarge { .. })
        ));

        let mut wrong_length = bytes.clone();
        wrong_length[..8].copy_from_slice(&3_u64.to_le_bytes());
        assert!(matches!(
            authenticate_file(&mut Cursor::new(wrong_length), pin, accept_tiny_header),
            Err(SafetensorsError::HeaderLength { .. })
        ));

        let header_truncated = bytes[..9].to_vec();
        assert!(matches!(
            authenticate_file(&mut Cursor::new(header_truncated), pin, accept_tiny_header),
            Err(SafetensorsError::EarlyEof { .. })
        ));

        let mut payload_truncated = bytes.clone();
        payload_truncated.pop();
        assert!(matches!(
            authenticate_file(&mut Cursor::new(payload_truncated), pin, accept_tiny_header),
            Err(SafetensorsError::EarlyEof { .. })
        ));

        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(matches!(
            authenticate_file(&mut Cursor::new(trailing), pin, accept_tiny_header),
            Err(SafetensorsError::TrailingData(_))
        ));

        let mut flipped = bytes;
        let last = flipped.len() - 1;
        flipped[last] ^= 1;
        assert!(matches!(
            authenticate_file(&mut Cursor::new(flipped), pin, accept_tiny_header),
            Err(SafetensorsError::DigestMismatch(_))
        ));
    }
}
