use crate::safetensors::{
    classify_tensor_name, parse_header, parse_target_index, require_shard_name,
    validate_offset_coverage, validate_roster, validate_shard_mapping, FilePin, ParsedShard,
    ParsedTensor, SafetensorsError, SafetensorsSource, DRAFT_FILE_PIN,
    MAX_SAFETENSORS_HEADER_BYTES, TARGET_SHARD_PINS,
};
use crate::sha256::Sha256;
use crate::{
    WeightDescriptor, QWEN3_DRAFT_TENSOR_DATA_BYTES, QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
    QWEN3_DRAFT_WEIGHT_SHA256, QWEN3_TARGET_TENSOR_DATA_BYTES, QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
    QWEN3_TARGET_WEIGHT_SET_SHA256,
};
use ferric_spec::{Qwen3ModelRole, TensorDType};
use std::fmt;
use std::io::{self, Read, Write};

const STREAM_BUFFER_BYTES: usize = 64 * 1_024;
const MAX_MANIFEST_BYTES: usize = 256 * 1_024;
const SECTION_ALIGNMENT: u64 = 2;
const MANIFEST_DOMAIN: &[u8] = b"ferric.prepacked-weight-sections.v1\0";

/// Canonical manifest format version for prepacked weight sections.
pub const PREPACKED_WEIGHT_MANIFEST_VERSION: u32 = 1;

/// The only data transform admitted by this foundation slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeightTransform {
    /// Copy BF16 row-major tensor bytes without conversion or reordering.
    Bf16RowMajorIdentityV1,
}

impl WeightTransform {
    /// Returns the stable transform identifier bound into the manifest.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Bf16RowMajorIdentityV1 => "bf16-row-major-identity-v1",
        }
    }
}

/// One immutable tensor section in the canonical prepacked manifest.
#[derive(Debug, PartialEq, Eq)]
pub struct WeightSection {
    tensor_name: String,
    role: Qwen3ModelRole,
    dtype: TensorDType,
    rank: u32,
    dimension_0: u32,
    dimension_1: u32,
    source_artifact: String,
    source_offset: u64,
    source_length: u64,
    destination_offset: u64,
    destination_length: u64,
    alignment: u64,
    transform: WeightTransform,
    sha256: [u8; 32],
}

impl WeightSection {
    /// Returns the exact canonical safetensors tensor name.
    #[must_use]
    pub fn tensor_name(&self) -> &str {
        &self.tensor_name
    }

    /// Returns the exact model role.
    #[must_use]
    pub const fn role(&self) -> Qwen3ModelRole {
        self.role
    }

    /// Returns the admitted tensor data type.
    #[must_use]
    pub const fn dtype(&self) -> TensorDType {
        self.dtype
    }

    /// Returns `(rank, dimension_0, dimension_1)` from the executable schema.
    #[must_use]
    pub const fn shape(&self) -> (u32, u32, u32) {
        (self.rank, self.dimension_0, self.dimension_1)
    }

    /// Returns the exact source artifact filename.
    #[must_use]
    pub fn source_artifact(&self) -> &str {
        &self.source_artifact
    }

    /// Returns the absolute full-file source byte offset and length.
    #[must_use]
    pub const fn source_range(&self) -> (u64, u64) {
        (self.source_offset, self.source_length)
    }

    /// Returns the output byte offset and length.
    #[must_use]
    pub const fn destination_range(&self) -> (u64, u64) {
        (self.destination_offset, self.destination_length)
    }

    /// Returns the required destination alignment in bytes.
    #[must_use]
    pub const fn alignment(&self) -> u64 {
        self.alignment
    }

    /// Returns the lossless transform applied to the section.
    #[must_use]
    pub const fn transform(&self) -> WeightTransform {
        self.transform
    }

    /// Returns SHA-256 of the exact emitted section bytes.
    #[must_use]
    pub const fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
}

/// Immutable, versioned manifest for a complete prepacked Qwen3 weight set.
///
/// `canonical_bytes` is the domain-separated binary record hashed to produce
/// `aggregate_id`. It binds the source weight-set identity and every section
/// field and digest. The record does not claim a kernel schedule layout.
#[derive(Debug, PartialEq, Eq)]
pub struct WeightSectionManifest {
    version: u32,
    role: Qwen3ModelRole,
    source_weights_id: [u8; 32],
    source_artifact_bytes: u64,
    tensor_data_bytes: u64,
    output_bytes: u64,
    sections: Vec<WeightSection>,
    canonical_bytes: Box<[u8]>,
    aggregate_id: [u8; 32],
}

impl WeightSectionManifest {
    /// Returns the canonical manifest format version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Returns the exact model role.
    #[must_use]
    pub const fn role(&self) -> Qwen3ModelRole {
        self.role
    }

    /// Returns the pinned source weight-set identity.
    #[must_use]
    pub const fn source_weights_id(&self) -> [u8; 32] {
        self.source_weights_id
    }

    /// Returns complete source safetensors bytes, including headers.
    #[must_use]
    pub const fn source_artifact_bytes(&self) -> u64 {
        self.source_artifact_bytes
    }

    /// Returns source tensor-data bytes excluding safetensors headers.
    #[must_use]
    pub const fn tensor_data_bytes(&self) -> u64 {
        self.tensor_data_bytes
    }

    /// Returns the exact bounded output length.
    #[must_use]
    pub const fn output_bytes(&self) -> u64 {
        self.output_bytes
    }

    /// Returns all tensor sections in source-file and source-offset order.
    #[must_use]
    pub fn sections(&self) -> &[WeightSection] {
        &self.sections
    }

    /// Returns the canonical domain-separated manifest record.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Returns SHA-256 of `canonical_bytes`.
    #[must_use]
    pub const fn aggregate_id(&self) -> [u8; 32] {
        self.aggregate_id
    }
}

/// Sealed authority for a completely emitted and authenticated prepacked set.
///
/// This value is returned only after exact source EOF and full-file SHA-256,
/// schema and layout validation, every successful output write, and manifest
/// construction. It is intentionally not `Clone`. Callers must write into a
/// staging destination and publish that destination atomically only after this
/// authority is returned.
#[derive(Debug, PartialEq, Eq)]
pub struct PrepackedWeightSet {
    role: Qwen3ModelRole,
    descriptor: WeightDescriptor,
    manifest: WeightSectionManifest,
    seal: PrepackedSeal,
}

#[derive(Debug, PartialEq, Eq)]
struct PrepackedSeal;

struct SourceStreamState {
    hasher: Sha256,
    total: u64,
    buffer: Box<[u8]>,
}

impl PrepackedWeightSet {
    /// Returns the exact model role.
    #[must_use]
    pub const fn role(&self) -> Qwen3ModelRole {
        self.role
    }

    /// Returns the immutable canonical section manifest.
    #[must_use]
    pub const fn manifest(&self) -> &WeightSectionManifest {
        &self.manifest
    }

    pub(super) fn into_parts(self) -> (WeightDescriptor, WeightSectionManifest) {
        (self.descriptor, self.manifest)
    }
}

/// Fail-closed prepacked weight streaming or manifest error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WeightStreamError {
    /// Exact source authentication or schema admission failed.
    Source(SafetensorsError),
    /// A checked offset, length, or manifest bound overflowed.
    ArithmeticOverflow(&'static str),
    /// A parsed section plan violated the canonical identity layout.
    InvalidLayout(&'static str),
    /// The bounded output writer failed before authority publication.
    OutputIo {
        /// Tensor whose output was incomplete.
        tensor: String,
        /// Stable I/O error category.
        kind: io::ErrorKind,
    },
    /// The canonical manifest exceeded its hard byte bound.
    ManifestTooLarge,
}

impl fmt::Display for WeightStreamError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => write!(formatter, "source authentication failed: {error}"),
            Self::ArithmeticOverflow(field) => write!(formatter, "{field} arithmetic overflowed"),
            Self::InvalidLayout(reason) => write!(formatter, "invalid prepacked layout: {reason}"),
            Self::OutputIo { tensor, kind } => {
                write!(formatter, "output failed for tensor {tensor:?}: {kind:?}")
            }
            Self::ManifestTooLarge => formatter.write_str("canonical weight manifest is too large"),
        }
    }
}

impl std::error::Error for WeightStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            _ => None,
        }
    }
}

impl From<SafetensorsError> for WeightStreamError {
    fn from(error: SafetensorsError) -> Self {
        Self::Source(error)
    }
}

/// Freshly authenticates and emits the exact Qwen3-8B tensor payloads.
///
/// The output is the header-free concatenation of tensors in pinned shard and
/// physical source-offset order. Each tensor is copied unchanged as BF16
/// row-major data. This is a deterministic foundation layout, not final
/// kernel-schedule packing. At most one 64-KiB payload buffer and one bounded
/// safetensors header are resident in addition to the manifest.
///
/// # Errors
///
/// Returns [`WeightStreamError`] without a [`PrepackedWeightSet`] unless the
/// exact pinned index and all five streams pass schema, mapping, size, EOF,
/// full-file digest, complete-output, and canonical-manifest checks.
pub fn prepack_qwen3_target_weights<R: Read, W: Write>(
    index_json: &[u8],
    mut shards: [SafetensorsSource<'_, R>; 5],
    output: &mut W,
) -> Result<PrepackedWeightSet, WeightStreamError> {
    let index = parse_target_index(index_json, true)?;
    let mut parsed_shards = Vec::with_capacity(TARGET_SHARD_PINS.len());
    let mut sections = Vec::with_capacity(Qwen3ModelRole::Target8B.tensor_count() as usize);
    let mut destination_offset = 0_u64;
    for (shard_index, (source, pin)) in shards.iter_mut().zip(TARGET_SHARD_PINS).enumerate() {
        require_shard_name(source.name, pin.name)?;
        let (parsed, mut file_sections) = stream_file(
            &mut source.reader,
            output,
            pin,
            Qwen3ModelRole::Target8B,
            &mut destination_offset,
            |header, tensor_data_bytes| {
                let shard = parse_header(
                    Qwen3ModelRole::Target8B,
                    pin.name,
                    header,
                    tensor_data_bytes,
                )?;
                validate_shard_mapping(&index, shard_index, &shard)?;
                Ok(shard)
            },
        )?;
        parsed_shards.push(parsed);
        sections.append(&mut file_sections);
    }
    validate_roster(Qwen3ModelRole::Target8B, &parsed_shards)?;
    flush_output(output)?;
    finish_prepacked(
        Qwen3ModelRole::Target8B,
        WeightDescriptor {
            weights_id: QWEN3_TARGET_WEIGHT_SET_SHA256,
            artifact_bytes: QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
            tensor_data_bytes: QWEN3_TARGET_TENSOR_DATA_BYTES,
            sections: 5,
        },
        destination_offset,
        sections,
    )
}

/// Freshly authenticates and emits the exact Qwen3-0.6B tensor payloads.
///
/// The output is the header-free source tensor data, copied unchanged as BF16
/// row-major sections. This is a deterministic foundation layout, not final
/// kernel-schedule packing. At most one 64-KiB payload buffer and the bounded
/// safetensors header are resident in addition to the manifest.
///
/// # Errors
///
/// Returns [`WeightStreamError`] without a [`PrepackedWeightSet`] unless the
/// exact pinned stream passes schema, size, EOF, full-file digest,
/// complete-output, and canonical-manifest checks.
pub fn prepack_qwen3_draft_weights<R: Read, W: Write>(
    mut source: SafetensorsSource<'_, R>,
    output: &mut W,
) -> Result<PrepackedWeightSet, WeightStreamError> {
    require_shard_name(source.name, DRAFT_FILE_PIN.name)?;
    let mut destination_offset = 0_u64;
    let (parsed, sections) = stream_file(
        &mut source.reader,
        output,
        DRAFT_FILE_PIN,
        Qwen3ModelRole::Draft06B,
        &mut destination_offset,
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
    flush_output(output)?;
    finish_prepacked(
        Qwen3ModelRole::Draft06B,
        WeightDescriptor {
            weights_id: QWEN3_DRAFT_WEIGHT_SHA256,
            artifact_bytes: QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
            tensor_data_bytes: QWEN3_DRAFT_TENSOR_DATA_BYTES,
            sections: 1,
        },
        destination_offset,
        sections,
    )
}

fn stream_file<R, W, F>(
    reader: &mut R,
    output: &mut W,
    pin: FilePin,
    role: Qwen3ModelRole,
    destination_offset: &mut u64,
    validate_header: F,
) -> Result<(ParsedShard, Vec<WeightSection>), WeightStreamError>
where
    R: Read,
    W: Write,
    F: FnOnce(&[u8], u64) -> Result<ParsedShard, SafetensorsError>,
{
    let mut source_hasher = Sha256::new();
    let mut source_total = 0_u64;
    let mut length_bytes = [0; 8];
    read_exact_source(
        reader,
        pin,
        &mut length_bytes,
        &mut source_hasher,
        &mut source_total,
    )?;
    let header_bytes = u64::from_le_bytes(length_bytes);
    if header_bytes > MAX_SAFETENSORS_HEADER_BYTES {
        return Err(SafetensorsError::HeaderTooLarge {
            artifact: pin.name.to_owned(),
            actual: header_bytes,
        }
        .into());
    }
    if header_bytes != pin.header_bytes {
        return Err(SafetensorsError::HeaderLength {
            artifact: pin.name.to_owned(),
            expected: pin.header_bytes,
            actual: header_bytes,
        }
        .into());
    }
    let header_len = usize::try_from(header_bytes)
        .map_err(|_| WeightStreamError::ArithmeticOverflow("header length"))?;
    let mut header = vec![0; header_len];
    read_exact_source(
        reader,
        pin,
        &mut header,
        &mut source_hasher,
        &mut source_total,
    )?;
    let parsed = validate_header(&header, pin.tensor_data_bytes)?;
    validate_stream_plan(role, &parsed, pin.tensor_data_bytes)?;

    let data_base = 8_u64
        .checked_add(header_bytes)
        .ok_or(WeightStreamError::ArithmeticOverflow("source data offset"))?;
    let source_end = data_base
        .checked_add(pin.tensor_data_bytes)
        .ok_or(WeightStreamError::ArithmeticOverflow("source file length"))?;
    if source_end != pin.file_bytes {
        return Err(WeightStreamError::InvalidLayout(
            "source header and tensor data do not cover the pinned file",
        ));
    }
    let mut sections = Vec::with_capacity(parsed.tensors.len());
    let mut source_state = SourceStreamState {
        hasher: source_hasher,
        total: source_total,
        buffer: vec![0; STREAM_BUFFER_BYTES].into_boxed_slice(),
    };
    for tensor in &parsed.tensors {
        let source_length = tensor
            .end
            .checked_sub(tensor.start)
            .ok_or(WeightStreamError::InvalidLayout("reversed source range"))?;
        let source_offset =
            data_base
                .checked_add(tensor.start)
                .ok_or(WeightStreamError::ArithmeticOverflow(
                    "source tensor offset",
                ))?;
        if !source_offset.is_multiple_of(SECTION_ALIGNMENT)
            || !(*destination_offset).is_multiple_of(SECTION_ALIGNMENT)
            || !source_length.is_multiple_of(SECTION_ALIGNMENT)
        {
            return Err(WeightStreamError::InvalidLayout(
                "BF16 section is not two-byte aligned",
            ));
        }
        let destination_start = *destination_offset;
        let destination_end = destination_start.checked_add(source_length).ok_or(
            WeightStreamError::ArithmeticOverflow("destination tensor offset"),
        )?;
        if destination_end > role.tensor_data_bytes() {
            return Err(WeightStreamError::InvalidLayout(
                "destination tensor exceeds the role output bound",
            ));
        }
        let section_sha256 = stream_section(
            reader,
            output,
            pin,
            tensor,
            source_length,
            &mut source_state,
        )?;
        *destination_offset = destination_end;
        sections.push(WeightSection {
            tensor_name: tensor.name.clone(),
            role,
            dtype: tensor.metadata.dtype,
            rank: tensor.metadata.rank,
            dimension_0: tensor.metadata.dimension_0,
            dimension_1: tensor.metadata.dimension_1,
            source_artifact: pin.name.to_owned(),
            source_offset,
            source_length,
            destination_offset: destination_start,
            destination_length: source_length,
            alignment: SECTION_ALIGNMENT,
            transform: WeightTransform::Bf16RowMajorIdentityV1,
            sha256: section_sha256,
        });
    }
    require_exact_source_end(reader, pin, source_state.hasher, source_state.total)?;
    Ok((parsed, sections))
}

fn validate_stream_plan(
    role: Qwen3ModelRole,
    parsed: &ParsedShard,
    expected_tensor_data_bytes: u64,
) -> Result<(), WeightStreamError> {
    if parsed.tensor_data_bytes != expected_tensor_data_bytes {
        return Err(WeightStreamError::InvalidLayout(
            "parsed tensor-data length drifted",
        ));
    }
    validate_offset_coverage(&parsed.tensors, expected_tensor_data_bytes)?;
    for tensor in &parsed.tensors {
        let (kind, layer, ordinal) = classify_tensor_name(role, &tensor.name)?;
        if tensor.metadata.role != role
            || tensor.metadata.kind != kind
            || tensor.metadata.layer != layer
            || tensor.ordinal != ordinal
            || tensor.metadata.dtype != TensorDType::Bf16
        {
            return Err(WeightStreamError::InvalidLayout(
                "tensor name, role, dtype, or ordinal drifted",
            ));
        }
        tensor
            .metadata
            .validate()
            .map_err(|_| WeightStreamError::InvalidLayout("tensor shape drifted"))?;
        let elements = u64::from(tensor.metadata.dimension_0)
            .checked_mul(u64::from(tensor.metadata.dimension_1))
            .ok_or(WeightStreamError::ArithmeticOverflow(
                "tensor element count",
            ))?;
        let expected_bytes = elements
            .checked_mul(2)
            .ok_or(WeightStreamError::ArithmeticOverflow("tensor byte length"))?;
        let actual_bytes = tensor
            .end
            .checked_sub(tensor.start)
            .ok_or(WeightStreamError::InvalidLayout("reversed source range"))?;
        if actual_bytes != expected_bytes {
            return Err(WeightStreamError::InvalidLayout(
                "tensor byte length differs from shape",
            ));
        }
    }
    Ok(())
}

fn stream_section<R: Read, W: Write>(
    reader: &mut R,
    output: &mut W,
    pin: FilePin,
    tensor: &ParsedTensor,
    length: u64,
    source: &mut SourceStreamState,
) -> Result<[u8; 32], WeightStreamError> {
    let mut section_hasher = Sha256::new();
    let mut remaining = length;
    while remaining != 0 {
        let chunk = usize::try_from(remaining.min(STREAM_BUFFER_BYTES as u64))
            .map_err(|_| WeightStreamError::ArithmeticOverflow("stream chunk"))?;
        read_exact_source(
            reader,
            pin,
            &mut source.buffer[..chunk],
            &mut source.hasher,
            &mut source.total,
        )?;
        section_hasher.update(&source.buffer[..chunk]);
        output
            .write_all(&source.buffer[..chunk])
            .map_err(|error| WeightStreamError::OutputIo {
                tensor: tensor.name.clone(),
                kind: error.kind(),
            })?;
        remaining = remaining
            .checked_sub(u64::try_from(chunk).expect("bounded chunk fits u64"))
            .ok_or(WeightStreamError::ArithmeticOverflow("remaining bytes"))?;
    }
    Ok(section_hasher.finish())
}

fn read_exact_source<R: Read>(
    reader: &mut R,
    pin: FilePin,
    buffer: &mut [u8],
    hasher: &mut Sha256,
    total: &mut u64,
) -> Result<(), WeightStreamError> {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => {
                return Err(SafetensorsError::EarlyEof {
                    artifact: pin.name.to_owned(),
                    expected: pin.file_bytes,
                    actual: *total,
                }
                .into());
            }
            Ok(read) => {
                hasher.update(&buffer[filled..filled + read]);
                filled += read;
                *total = total
                    .checked_add(u64::try_from(read).expect("read length fits u64"))
                    .ok_or(WeightStreamError::ArithmeticOverflow("source byte count"))?;
                if *total > pin.file_bytes {
                    return Err(SafetensorsError::FileSize {
                        artifact: pin.name.to_owned(),
                        expected: pin.file_bytes,
                        actual: *total,
                    }
                    .into());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                return Err(SafetensorsError::Io {
                    artifact: pin.name.to_owned(),
                    kind: error.kind(),
                }
                .into());
            }
        }
    }
    Ok(())
}

fn require_exact_source_end<R: Read>(
    reader: &mut R,
    pin: FilePin,
    hasher: Sha256,
    total: u64,
) -> Result<(), WeightStreamError> {
    let mut trailing = [0; 1];
    match reader.read(&mut trailing) {
        Ok(0) => {}
        Ok(_) => return Err(SafetensorsError::TrailingData(pin.name.to_owned()).into()),
        Err(error) => {
            return Err(SafetensorsError::Io {
                artifact: pin.name.to_owned(),
                kind: error.kind(),
            }
            .into());
        }
    }
    if total != pin.file_bytes {
        return Err(SafetensorsError::FileSize {
            artifact: pin.name.to_owned(),
            expected: pin.file_bytes,
            actual: total,
        }
        .into());
    }
    if hasher.finish() != pin.sha256 {
        return Err(SafetensorsError::DigestMismatch(pin.name.to_owned()).into());
    }
    Ok(())
}

fn flush_output<W: Write>(output: &mut W) -> Result<(), WeightStreamError> {
    output.flush().map_err(|error| WeightStreamError::OutputIo {
        tensor: "<final flush>".to_owned(),
        kind: error.kind(),
    })
}

fn finish_prepacked(
    role: Qwen3ModelRole,
    descriptor: WeightDescriptor,
    output_bytes: u64,
    sections: Vec<WeightSection>,
) -> Result<PrepackedWeightSet, WeightStreamError> {
    if output_bytes != descriptor.tensor_data_bytes
        || output_bytes != role.tensor_data_bytes()
        || sections.len() != role.tensor_count() as usize
    {
        return Err(WeightStreamError::InvalidLayout(
            "complete output count or length drifted",
        ));
    }
    validate_destination_coverage(role, output_bytes, &sections)?;
    let canonical_bytes = encode_manifest_record(role, descriptor, output_bytes, &sections)?;
    let aggregate_id = crate::sha256::digest(&canonical_bytes);
    Ok(PrepackedWeightSet {
        role,
        descriptor,
        manifest: WeightSectionManifest {
            version: PREPACKED_WEIGHT_MANIFEST_VERSION,
            role,
            source_weights_id: descriptor.weights_id,
            source_artifact_bytes: descriptor.artifact_bytes,
            tensor_data_bytes: descriptor.tensor_data_bytes,
            output_bytes,
            sections,
            canonical_bytes: canonical_bytes.into_boxed_slice(),
            aggregate_id,
        },
        seal: PrepackedSeal,
    })
}

fn validate_destination_coverage(
    role: Qwen3ModelRole,
    output_bytes: u64,
    sections: &[WeightSection],
) -> Result<(), WeightStreamError> {
    let mut expected = 0_u64;
    for section in sections {
        if section.role != role
            || section.dtype != TensorDType::Bf16
            || section.transform != WeightTransform::Bf16RowMajorIdentityV1
            || section.alignment != SECTION_ALIGNMENT
            || section.destination_offset != expected
            || section.destination_length != section.source_length
            || !section.destination_offset.is_multiple_of(section.alignment)
            || !section.destination_length.is_multiple_of(section.alignment)
        {
            return Err(WeightStreamError::InvalidLayout(
                "destination sections are not canonical and complete",
            ));
        }
        expected = expected.checked_add(section.destination_length).ok_or(
            WeightStreamError::ArithmeticOverflow("destination coverage"),
        )?;
    }
    if expected != output_bytes {
        return Err(WeightStreamError::InvalidLayout(
            "destination sections do not cover output",
        ));
    }
    Ok(())
}

fn encode_manifest_record(
    role: Qwen3ModelRole,
    descriptor: WeightDescriptor,
    output_bytes: u64,
    sections: &[WeightSection],
) -> Result<Vec<u8>, WeightStreamError> {
    let capacity = sections
        .len()
        .checked_mul(160)
        .and_then(|bytes| bytes.checked_add(64))
        .ok_or(WeightStreamError::ArithmeticOverflow("manifest capacity"))?;
    if capacity > MAX_MANIFEST_BYTES {
        return Err(WeightStreamError::ManifestTooLarge);
    }
    let mut record = Vec::with_capacity(capacity);
    record.extend_from_slice(MANIFEST_DOMAIN);
    record.extend_from_slice(&PREPACKED_WEIGHT_MANIFEST_VERSION.to_le_bytes());
    record.push(role_code(role));
    record.extend_from_slice(&descriptor.weights_id);
    record.extend_from_slice(&descriptor.artifact_bytes.to_le_bytes());
    record.extend_from_slice(&descriptor.tensor_data_bytes.to_le_bytes());
    record.extend_from_slice(&output_bytes.to_le_bytes());
    let section_count = u32::try_from(sections.len())
        .map_err(|_| WeightStreamError::ArithmeticOverflow("manifest section count"))?;
    record.extend_from_slice(&section_count.to_le_bytes());
    for section in sections {
        push_string(&mut record, &section.tensor_name)?;
        record.push(role_code(section.role));
        record.push(dtype_code(section.dtype));
        record.extend_from_slice(&section.rank.to_le_bytes());
        record.extend_from_slice(&section.dimension_0.to_le_bytes());
        record.extend_from_slice(&section.dimension_1.to_le_bytes());
        push_string(&mut record, &section.source_artifact)?;
        record.extend_from_slice(&section.source_offset.to_le_bytes());
        record.extend_from_slice(&section.source_length.to_le_bytes());
        record.extend_from_slice(&section.destination_offset.to_le_bytes());
        record.extend_from_slice(&section.destination_length.to_le_bytes());
        record.extend_from_slice(&section.alignment.to_le_bytes());
        push_string(&mut record, section.transform.id())?;
        record.extend_from_slice(&section.sha256);
        if record.len() > MAX_MANIFEST_BYTES {
            return Err(WeightStreamError::ManifestTooLarge);
        }
    }
    Ok(record)
}

fn push_string(record: &mut Vec<u8>, value: &str) -> Result<(), WeightStreamError> {
    let length = u32::try_from(value.len())
        .map_err(|_| WeightStreamError::ArithmeticOverflow("manifest string length"))?;
    record.extend_from_slice(&length.to_le_bytes());
    record.extend_from_slice(value.as_bytes());
    if record.len() > MAX_MANIFEST_BYTES {
        return Err(WeightStreamError::ManifestTooLarge);
    }
    Ok(())
}

const fn role_code(role: Qwen3ModelRole) -> u8 {
    match role {
        Qwen3ModelRole::Target8B => 1,
        Qwen3ModelRole::Draft06B => 2,
    }
}

const fn dtype_code(dtype: TensorDType) -> u8 {
    match dtype {
        TensorDType::Bf16 => 1,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        encode_manifest_record, flush_output, parse_header, require_shard_name, stream_file,
        validate_destination_coverage, validate_stream_plan, FilePin, ParsedShard, ParsedTensor,
        PrepackedSeal, PrepackedWeightSet, WeightSection, WeightSectionManifest, WeightStreamError,
        WeightTransform, DRAFT_FILE_PIN, PREPACKED_WEIGHT_MANIFEST_VERSION, SECTION_ALIGNMENT,
        TARGET_SHARD_PINS,
    };
    use crate::tokenizer::tests::{authenticated_assets, test_tokenizer};
    use crate::{
        build_prepacked_deployment_bundle, sha256, BuildError, SafetensorsError, WeightDescriptor,
        QWEN3_DRAFT_TENSOR_DATA_BYTES, QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
        QWEN3_DRAFT_WEIGHT_SHA256, QWEN3_TARGET_TENSOR_DATA_BYTES,
        QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES, QWEN3_TARGET_WEIGHT_SET_SHA256,
    };
    use ferric_spec::{Qwen3ModelRole, Qwen3TensorMetadata, TensorDType};
    use std::io::{self, Cursor, Read, Write};

    const HEADER: &[u8] = b"{}";
    const TARGET_HEADERS: [&[u8]; 5] = [
        include_bytes!("fixtures/safetensors/qwen3-8b-00001.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00002.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00003.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00004.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00005.header.json"),
    ];
    const DRAFT_HEADER: &[u8] = include_bytes!("fixtures/safetensors/qwen3-06b.header.json");
    const TENSOR_BYTES: u64 = 2_048;
    const TOTAL_BYTES: u64 = TENSOR_BYTES * 2;

    struct ChunkedReader {
        cursor: Cursor<Vec<u8>>,
        max_chunk: usize,
    }

    impl Read for ChunkedReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let chunk = buffer.len().min(self.max_chunk);
            self.cursor.read(&mut buffer[..chunk])
        }
    }

    struct FailingWriter {
        bytes: Vec<u8>,
        limit: usize,
        fail_flush: bool,
    }

    impl Write for FailingWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            if self.bytes.len() == self.limit {
                return Err(io::Error::new(io::ErrorKind::WriteZero, "staging full"));
            }
            let allowed = buffer.len().min(self.limit - self.bytes.len());
            self.bytes.extend_from_slice(&buffer[..allowed]);
            Ok(allowed)
        }

        fn flush(&mut self) -> io::Result<()> {
            if self.fail_flush {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "flush failed"))
            } else {
                Ok(())
            }
        }
    }

    fn tiny_tensor(name: &str, start: u64, end: u64) -> ParsedTensor {
        let (kind, layer, ordinal) = super::classify_tensor_name(Qwen3ModelRole::Draft06B, name)
            .expect("canonical tiny tensor name");
        ParsedTensor {
            name: name.to_owned(),
            start,
            end,
            ordinal,
            metadata: Qwen3TensorMetadata {
                role: Qwen3ModelRole::Draft06B,
                kind,
                layer,
                dtype: TensorDType::Bf16,
                rank: 1,
                dimension_0: 1_024,
                dimension_1: 1,
            },
        }
    }

    fn tiny_plan() -> ParsedShard {
        ParsedShard {
            tensors: vec![
                tiny_tensor("model.layers.0.input_layernorm.weight", 0, TENSOR_BYTES),
                tiny_tensor(
                    "model.layers.0.post_attention_layernorm.weight",
                    TENSOR_BYTES,
                    TOTAL_BYTES,
                ),
            ],
            tensor_data_bytes: TOTAL_BYTES,
        }
    }

    fn synthetic_file(data: &[u8]) -> (Vec<u8>, FilePin) {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(
            &u64::try_from(HEADER.len())
                .expect("tiny header length fits u64")
                .to_le_bytes(),
        );
        bytes.extend_from_slice(HEADER);
        bytes.extend_from_slice(data);
        let pin = FilePin {
            name: "tiny.safetensors",
            sha256: sha256::digest(&bytes),
            file_bytes: u64::try_from(bytes.len()).expect("tiny file length fits u64"),
            header_bytes: u64::try_from(HEADER.len()).expect("tiny header length fits u64"),
            tensor_data_bytes: u64::try_from(data.len()).expect("tiny data length fits u64"),
        };
        (bytes, pin)
    }

    fn stream_tiny<R: Read, W: Write>(
        reader: &mut R,
        writer: &mut W,
        pin: FilePin,
        plan: ParsedShard,
    ) -> Result<(ParsedShard, Vec<WeightSection>), WeightStreamError> {
        let mut destination_offset = 0;
        stream_file(
            reader,
            writer,
            pin,
            Qwen3ModelRole::Draft06B,
            &mut destination_offset,
            |header, tensor_data_bytes| {
                assert_eq!(header, HEADER);
                assert_eq!(tensor_data_bytes, TOTAL_BYTES);
                Ok(plan)
            },
        )
    }

    fn tiny_data() -> Vec<u8> {
        (0..TOTAL_BYTES)
            .map(|index| u8::try_from(index % 251).expect("value is below 251"))
            .collect()
    }

    fn descriptor(role: Qwen3ModelRole) -> WeightDescriptor {
        match role {
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
        }
    }

    pub(crate) fn test_prepacked(role: Qwen3ModelRole) -> PrepackedWeightSet {
        let descriptor = descriptor(role);
        let parsed = match role {
            Qwen3ModelRole::Target8B => TARGET_HEADERS
                .iter()
                .zip(TARGET_SHARD_PINS)
                .map(|(fixture, pin)| {
                    assert_eq!(fixture.last(), Some(&b'\n'));
                    let mut header = fixture[..fixture.len() - 1].to_vec();
                    header.resize(
                        usize::try_from(pin.header_bytes).expect("test header bound"),
                        b' ',
                    );
                    let shard = parse_header(role, pin.name, &header, pin.tensor_data_bytes)
                        .expect("official target header");
                    (pin, shard)
                })
                .collect::<Vec<_>>(),
            Qwen3ModelRole::Draft06B => {
                let pin = DRAFT_FILE_PIN;
                assert_eq!(DRAFT_HEADER.last(), Some(&b'\n'));
                let mut header = DRAFT_HEADER[..DRAFT_HEADER.len() - 1].to_vec();
                header.resize(
                    usize::try_from(pin.header_bytes).expect("test header bound"),
                    b' ',
                );
                vec![(
                    pin,
                    parse_header(role, pin.name, &header, pin.tensor_data_bytes)
                        .expect("official draft header"),
                )]
            }
        };
        let mut destination = 0_u64;
        let mut sections = Vec::with_capacity(role.tensor_count() as usize);
        for (pin, shard) in parsed {
            for tensor in shard.tensors {
                let length = tensor.end - tensor.start;
                let mut digest_input = tensor.name.as_bytes().to_vec();
                digest_input.extend_from_slice(&tensor.start.to_le_bytes());
                sections.push(WeightSection {
                    tensor_name: tensor.name,
                    role,
                    dtype: tensor.metadata.dtype,
                    rank: tensor.metadata.rank,
                    dimension_0: tensor.metadata.dimension_0,
                    dimension_1: tensor.metadata.dimension_1,
                    source_artifact: pin.name.to_owned(),
                    source_offset: 8 + pin.header_bytes + tensor.start,
                    source_length: length,
                    destination_offset: destination,
                    destination_length: length,
                    alignment: SECTION_ALIGNMENT,
                    transform: WeightTransform::Bf16RowMajorIdentityV1,
                    sha256: sha256::digest(&digest_input),
                });
                destination += length;
            }
        }
        assert_eq!(destination, descriptor.tensor_data_bytes);
        assert_eq!(sections.len(), role.tensor_count() as usize);
        let canonical_bytes = encode_manifest_record(role, descriptor, destination, &sections)
            .expect("test canonical manifest");
        let aggregate_id = sha256::digest(&canonical_bytes);
        PrepackedWeightSet {
            role,
            descriptor,
            manifest: WeightSectionManifest {
                version: PREPACKED_WEIGHT_MANIFEST_VERSION,
                role,
                source_weights_id: descriptor.weights_id,
                source_artifact_bytes: descriptor.artifact_bytes,
                tensor_data_bytes: descriptor.tensor_data_bytes,
                output_bytes: destination,
                sections,
                aggregate_id,
                canonical_bytes: canonical_bytes.into_boxed_slice(),
            },
            seal: PrepackedSeal,
        }
    }

    #[test]
    fn tiny_chunked_stream_round_trips_and_binds_every_section() {
        let data = tiny_data();
        let (bytes, pin) = synthetic_file(&data);
        let mut reader = ChunkedReader {
            cursor: Cursor::new(bytes),
            max_chunk: 7,
        };
        let mut output = Vec::new();
        let (_, sections) = stream_tiny(&mut reader, &mut output, pin, tiny_plan())
            .expect("complete tiny prepack stream");
        assert_eq!(output, data);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].source_range(), (10, TENSOR_BYTES));
        assert_eq!(
            sections[1].source_range(),
            (10 + TENSOR_BYTES, TENSOR_BYTES)
        );
        assert_eq!(sections[0].destination_range(), (0, TENSOR_BYTES));
        assert_eq!(
            sections[1].destination_range(),
            (TENSOR_BYTES, TENSOR_BYTES)
        );
        assert_eq!(sections[0].alignment(), SECTION_ALIGNMENT);
        assert_eq!(
            sections[0].transform(),
            WeightTransform::Bf16RowMajorIdentityV1
        );
        assert_eq!(
            sections[0].sha256(),
            sha256::digest(
                &data[..usize::try_from(TENSOR_BYTES).expect("tiny tensor length fits usize")],
            )
        );
        validate_destination_coverage(Qwen3ModelRole::Draft06B, TOTAL_BYTES, &sections)
            .expect("complete destination coverage");

        let record = encode_manifest_record(
            Qwen3ModelRole::Draft06B,
            descriptor(Qwen3ModelRole::Draft06B),
            TOTAL_BYTES,
            &sections,
        )
        .expect("bounded canonical manifest");
        let aggregate = sha256::digest(&record);
        let second = encode_manifest_record(
            Qwen3ModelRole::Draft06B,
            descriptor(Qwen3ModelRole::Draft06B),
            TOTAL_BYTES,
            &sections,
        )
        .expect("deterministic canonical manifest");
        assert_eq!(record, second);
        assert_eq!(aggregate, sha256::digest(&second));
    }

    #[test]
    fn gap_overlap_reorder_missing_name_and_shape_drift_fail_closed() {
        let mut gap = tiny_plan();
        gap.tensors[1].start += 2;
        gap.tensors[1].end += 2;
        assert!(matches!(
            validate_stream_plan(Qwen3ModelRole::Draft06B, &gap, TOTAL_BYTES),
            Err(WeightStreamError::Source(
                SafetensorsError::OffsetGap { .. }
            ))
        ));

        let mut overlap = tiny_plan();
        overlap.tensors[1].start -= 2;
        overlap.tensors[1].end -= 2;
        assert!(matches!(
            validate_stream_plan(Qwen3ModelRole::Draft06B, &overlap, TOTAL_BYTES),
            Err(WeightStreamError::Source(
                SafetensorsError::OffsetOverlap { .. }
            ))
        ));

        let mut reordered = tiny_plan();
        reordered.tensors.swap(0, 1);
        assert!(validate_stream_plan(Qwen3ModelRole::Draft06B, &reordered, TOTAL_BYTES).is_err());

        let mut missing = tiny_plan();
        missing.tensors.pop();
        assert!(matches!(
            validate_stream_plan(Qwen3ModelRole::Draft06B, &missing, TOTAL_BYTES),
            Err(WeightStreamError::Source(
                SafetensorsError::TensorDataBytes { .. }
            ))
        ));

        let mut name = tiny_plan();
        name.tensors[0].name = "model.layers.0.input_layernorm.bias".to_owned();
        assert!(matches!(
            validate_stream_plan(Qwen3ModelRole::Draft06B, &name, TOTAL_BYTES),
            Err(WeightStreamError::Source(
                SafetensorsError::InvalidTensorName(_)
            ))
        ));

        let mut shape = tiny_plan();
        shape.tensors[0].metadata.dimension_0 += 1;
        assert_eq!(
            validate_stream_plan(Qwen3ModelRole::Draft06B, &shape, TOTAL_BYTES),
            Err(WeightStreamError::InvalidLayout("tensor shape drifted"))
        );
    }

    #[test]
    fn truncation_trailing_bit_flip_and_fresh_read_toctou_are_rejected() {
        let data = tiny_data();
        let (bytes, pin) = synthetic_file(&data);

        let mut truncated = bytes.clone();
        truncated.pop();
        assert!(matches!(
            stream_tiny(
                &mut Cursor::new(truncated),
                &mut Vec::new(),
                pin,
                tiny_plan()
            ),
            Err(WeightStreamError::Source(SafetensorsError::EarlyEof { .. }))
        ));

        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(matches!(
            stream_tiny(
                &mut Cursor::new(trailing),
                &mut Vec::new(),
                pin,
                tiny_plan()
            ),
            Err(WeightStreamError::Source(SafetensorsError::TrailingData(_)))
        ));

        stream_tiny(
            &mut Cursor::new(bytes.clone()),
            &mut Vec::new(),
            pin,
            tiny_plan(),
        )
        .expect("first exact read succeeds");
        let mut changed_after_first_pass = bytes;
        let changed_index = changed_after_first_pass.len() - 1;
        changed_after_first_pass[changed_index] ^= 1;
        assert!(matches!(
            stream_tiny(
                &mut Cursor::new(changed_after_first_pass),
                &mut Vec::new(),
                pin,
                tiny_plan()
            ),
            Err(WeightStreamError::Source(SafetensorsError::DigestMismatch(
                _
            )))
        ));
    }

    #[test]
    fn partial_output_never_returns_authority_and_manifest_digest_is_sensitive() {
        let data = tiny_data();
        let (bytes, pin) = synthetic_file(&data);
        let partial_limit = usize::try_from(TENSOR_BYTES)
            .expect("tiny tensor length fits usize")
            .checked_add(17)
            .expect("tiny partial write bound fits usize");
        let mut writer = FailingWriter {
            bytes: Vec::new(),
            limit: partial_limit,
            fail_flush: false,
        };
        assert!(matches!(
            stream_tiny(
                &mut Cursor::new(bytes.clone()),
                &mut writer,
                pin,
                tiny_plan()
            ),
            Err(WeightStreamError::OutputIo { .. })
        ));
        assert_eq!(writer.bytes.len(), partial_limit);

        let mut flush_failure = FailingWriter {
            bytes: Vec::new(),
            limit: usize::MAX,
            fail_flush: true,
        };
        assert!(matches!(
            flush_output(&mut flush_failure),
            Err(WeightStreamError::OutputIo {
                kind: io::ErrorKind::BrokenPipe,
                ..
            })
        ));

        let (_, mut sections) =
            stream_tiny(&mut Cursor::new(bytes), &mut Vec::new(), pin, tiny_plan())
                .expect("complete stream");
        let before = encode_manifest_record(
            Qwen3ModelRole::Draft06B,
            descriptor(Qwen3ModelRole::Draft06B),
            TOTAL_BYTES,
            &sections,
        )
        .expect("canonical record");
        sections[0].sha256[0] ^= 1;
        let after = encode_manifest_record(
            Qwen3ModelRole::Draft06B,
            descriptor(Qwen3ModelRole::Draft06B),
            TOTAL_BYTES,
            &sections,
        )
        .expect("changed canonical record");
        assert_ne!(before, after);
        assert_ne!(sha256::digest(&before), sha256::digest(&after));
    }

    #[test]
    fn shard_and_builder_role_swaps_are_rejected() {
        assert!(matches!(
            require_shard_name(TARGET_SHARD_PINS[1].name, TARGET_SHARD_PINS[0].name),
            Err(SafetensorsError::ShardName { .. })
        ));
        let deployment = build_prepacked_deployment_bundle(
            authenticated_assets(),
            test_tokenizer(Qwen3ModelRole::Target8B),
            test_tokenizer(Qwen3ModelRole::Draft06B),
            test_prepacked(Qwen3ModelRole::Target8B),
            test_prepacked(Qwen3ModelRole::Draft06B),
        )
        .expect("prepacked builder consumes exact roles");
        assert_eq!(
            deployment
                .deployment()
                .target_model
                .weights
                .weights_id
                .as_bytes(),
            &QWEN3_TARGET_WEIGHT_SET_SHA256
        );
        assert_eq!(
            deployment.target_manifest().role(),
            Qwen3ModelRole::Target8B
        );
        assert_eq!(deployment.draft_manifest().role(), Qwen3ModelRole::Draft06B);
        assert_eq!(
            build_prepacked_deployment_bundle(
                authenticated_assets(),
                test_tokenizer(Qwen3ModelRole::Target8B),
                test_tokenizer(Qwen3ModelRole::Draft06B),
                test_prepacked(Qwen3ModelRole::Draft06B),
                test_prepacked(Qwen3ModelRole::Target8B),
            ),
            Err(BuildError::AuthenticatedWeightRole {
                expected: Qwen3ModelRole::Target8B,
                actual: Qwen3ModelRole::Draft06B,
            })
        );
    }
}
