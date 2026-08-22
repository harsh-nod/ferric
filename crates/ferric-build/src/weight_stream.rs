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
use ferric_spec::{Qwen3ModelRole, Qwen3TensorMetadata, TensorDType};
use std::fmt;
use std::io::{self, Read, Write};
use vstd::bytes::{u32_to_le_bytes, u64_to_le_bytes};
use vstd::prelude::*;

const STREAM_BUFFER_BYTES: usize = 64 * 1_024;

verus! {

const MAX_MANIFEST_BYTES: usize = 256 * 1_024;
const SECTION_ALIGNMENT: u64 = 2;
const MANIFEST_DOMAIN: [u8; 36] = [
    102, 101, 114, 114, 105, 99, 46, 112, 114, 101, 112, 97, 99, 107, 101, 100, 45, 119, 101,
    105, 103, 104, 116, 45, 115, 101, 99, 116, 105, 111, 110, 115, 46, 118, 49, 0,
];
const TRANSFORM_ID_BYTES: [u8; 26] = [
    98, 102, 49, 54, 45, 114, 111, 119, 45, 109, 97, 106, 111, 114, 45, 105, 100, 101, 110,
    116, 105, 116, 121, 45, 118, 49,
];

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
    pub closed spec fn tensor_name_spec(&self) -> Seq<char> { self.tensor_name@ }
    pub closed spec fn role_spec(&self) -> Qwen3ModelRole { self.role }
    pub closed spec fn dtype_spec(&self) -> TensorDType { self.dtype }
    pub closed spec fn shape_spec(&self) -> (u32, u32, u32) {
        (self.rank, self.dimension_0, self.dimension_1)
    }
    pub closed spec fn source_artifact_spec(&self) -> Seq<char> { self.source_artifact@ }
    pub closed spec fn source_range_spec(&self) -> (u64, u64) {
        (self.source_offset, self.source_length)
    }
    pub closed spec fn destination_range_spec(&self) -> (u64, u64) {
        (self.destination_offset, self.destination_length)
    }
    pub closed spec fn alignment_spec(&self) -> u64 { self.alignment }
    pub closed spec fn transform_spec(&self) -> WeightTransform { self.transform }
    pub closed spec fn sha256_spec(&self) -> Seq<u8> { self.sha256@ }

    /// Returns the exact canonical safetensors tensor name.
    #[must_use]
    pub fn tensor_name(&self) -> (name: &str)
        ensures name@ == self.tensor_name_spec(),
    {
        &self.tensor_name
    }

    /// Returns the exact model role.
    #[must_use]
    pub const fn role(&self) -> (role: Qwen3ModelRole)
        ensures role == self.role_spec(),
    {
        self.role
    }

    /// Returns the admitted tensor data type.
    #[must_use]
    pub const fn dtype(&self) -> (dtype: TensorDType)
        ensures dtype == self.dtype_spec(),
    {
        self.dtype
    }

    /// Returns `(rank, dimension_0, dimension_1)` from the executable schema.
    #[must_use]
    pub const fn shape(&self) -> (shape: (u32, u32, u32))
        ensures shape == self.shape_spec(),
    {
        (self.rank, self.dimension_0, self.dimension_1)
    }

    /// Returns the exact source artifact filename.
    #[must_use]
    pub fn source_artifact(&self) -> (artifact: &str)
        ensures artifact@ == self.source_artifact_spec(),
    {
        &self.source_artifact
    }

    /// Returns the absolute full-file source byte offset and length.
    #[must_use]
    pub const fn source_range(&self) -> (range: (u64, u64))
        ensures range == self.source_range_spec(),
    {
        (self.source_offset, self.source_length)
    }

    /// Returns the output byte offset and length.
    #[must_use]
    pub const fn destination_range(&self) -> (range: (u64, u64))
        ensures range == self.destination_range_spec(),
    {
        (self.destination_offset, self.destination_length)
    }

    /// Returns the required destination alignment in bytes.
    #[must_use]
    pub const fn alignment(&self) -> (alignment: u64)
        ensures alignment == self.alignment_spec(),
    {
        self.alignment
    }

    /// Returns the lossless transform applied to the section.
    #[must_use]
    pub const fn transform(&self) -> (transform: WeightTransform)
        ensures transform == self.transform_spec(),
    {
        self.transform
    }

    /// Returns SHA-256 of the exact emitted section bytes.
    #[must_use]
    pub const fn sha256(&self) -> (digest: [u8; 32])
        ensures digest@ == self.sha256_spec(),
    {
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
    canonical_bytes: Vec<u8>,
    aggregate_id: [u8; 32],
}

impl WeightSectionManifest {
    pub closed spec fn version_spec(&self) -> u32 { self.version }
    pub closed spec fn role_spec(&self) -> Qwen3ModelRole { self.role }
    pub closed spec fn source_weights_id_spec(&self) -> Seq<u8> { self.source_weights_id@ }
    pub closed spec fn source_artifact_bytes_spec(&self) -> u64 { self.source_artifact_bytes }
    pub closed spec fn tensor_data_bytes_spec(&self) -> u64 { self.tensor_data_bytes }
    pub closed spec fn output_bytes_spec(&self) -> u64 { self.output_bytes }
    pub closed spec fn sections_spec(&self) -> Seq<WeightSection> { self.sections@ }
    pub closed spec fn canonical_bytes_spec(&self) -> Seq<u8> { self.canonical_bytes@ }
    pub closed spec fn aggregate_id_spec(&self) -> Seq<u8> { self.aggregate_id@ }

    /// Exact relation proved by the pure manifest finalizer.
    pub closed spec fn valid_commitment(&self) -> bool {
        &&& self.version == PREPACKED_WEIGHT_MANIFEST_VERSION
        &&& destination_layout_spec(self.role, self.output_bytes, self.sections@)
        &&& self.canonical_bytes@ == manifest_record_spec(
            self.role,
            self.source_weights_id@,
            self.source_artifact_bytes,
            self.tensor_data_bytes,
            self.output_bytes,
            self.sections@,
        )
        &&& self.aggregate_id@ == crate::sha256::digest_spec(self.canonical_bytes@)
    }

    /// Returns the canonical manifest format version.
    #[must_use]
    pub const fn version(&self) -> (version: u32)
        ensures version == self.version_spec(),
    {
        self.version
    }

    /// Returns the exact model role.
    #[must_use]
    pub const fn role(&self) -> (role: Qwen3ModelRole)
        ensures role == self.role_spec(),
    {
        self.role
    }

    /// Returns the pinned source weight-set identity.
    #[must_use]
    pub const fn source_weights_id(&self) -> (identity: [u8; 32])
        ensures identity@ == self.source_weights_id_spec(),
    {
        self.source_weights_id
    }

    /// Returns complete source safetensors bytes, including headers.
    #[must_use]
    pub const fn source_artifact_bytes(&self) -> (bytes: u64)
        ensures bytes == self.source_artifact_bytes_spec(),
    {
        self.source_artifact_bytes
    }

    /// Returns source tensor-data bytes excluding safetensors headers.
    #[must_use]
    pub const fn tensor_data_bytes(&self) -> (bytes: u64)
        ensures bytes == self.tensor_data_bytes_spec(),
    {
        self.tensor_data_bytes
    }

    /// Returns the exact bounded output length.
    #[must_use]
    pub const fn output_bytes(&self) -> (bytes: u64)
        ensures bytes == self.output_bytes_spec(),
    {
        self.output_bytes
    }

    /// Returns all tensor sections in source-file and source-offset order.
    #[must_use]
    pub fn sections(&self) -> (sections: &[WeightSection])
        ensures sections@ == self.sections_spec(),
    {
        &self.sections
    }

    /// Returns the canonical domain-separated manifest record.
    #[must_use]
    pub fn canonical_bytes(&self) -> (bytes: &[u8])
        ensures bytes@ == self.canonical_bytes_spec(),
    {
        &self.canonical_bytes
    }

    /// Returns SHA-256 of `canonical_bytes`.
    #[must_use]
    pub const fn aggregate_id(&self) -> (identity: [u8; 32])
        ensures identity@ == self.aggregate_id_spec(),
    {
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

impl PrepackedWeightSet {
    pub closed spec fn role_spec(&self) -> Qwen3ModelRole { self.role }
    pub closed spec fn descriptor_spec(&self) -> WeightDescriptor { self.descriptor }
    pub closed spec fn manifest_spec(&self) -> WeightSectionManifest { self.manifest }

    /// Returns the exact model role.
    #[must_use]
    pub const fn role(&self) -> (role: Qwen3ModelRole)
        ensures role == self.role_spec(),
    {
        self.role
    }

    /// Returns the immutable canonical section manifest.
    #[must_use]
    pub const fn manifest(&self) -> (manifest: &WeightSectionManifest)
        ensures *manifest == self.manifest_spec(),
    {
        &self.manifest
    }

    pub(crate) fn into_parts(self) -> (parts: (WeightDescriptor, WeightSectionManifest))
        ensures parts == (self.descriptor_spec(), self.manifest_spec()),
    {
        (self.descriptor, self.manifest)
    }
}

closed spec fn role_code_spec(role: Qwen3ModelRole) -> u8 {
    match role {
        Qwen3ModelRole::Target8B => 1,
        Qwen3ModelRole::Draft06B => 2,
    }
}

closed spec fn dtype_code_spec(dtype: TensorDType) -> u8 {
    match dtype {
        TensorDType::Bf16 => 1,
    }
}

closed spec fn string_record_spec(bytes: Seq<u8>) -> Seq<u8> {
    vstd::bytes::spec_u32_to_le_bytes(bytes.len() as u32) + bytes
}

closed spec fn section_record_spec(section: WeightSection) -> Seq<u8> {
    string_record_spec(vstd::utf8::encode_utf8(section.tensor_name@))
        + seq![role_code_spec(section.role)]
        + seq![dtype_code_spec(section.dtype)]
        + vstd::bytes::spec_u32_to_le_bytes(section.rank)
        + vstd::bytes::spec_u32_to_le_bytes(section.dimension_0)
        + vstd::bytes::spec_u32_to_le_bytes(section.dimension_1)
        + string_record_spec(vstd::utf8::encode_utf8(section.source_artifact@))
        + vstd::bytes::spec_u64_to_le_bytes(section.source_offset)
        + vstd::bytes::spec_u64_to_le_bytes(section.source_length)
        + vstd::bytes::spec_u64_to_le_bytes(section.destination_offset)
        + vstd::bytes::spec_u64_to_le_bytes(section.destination_length)
        + vstd::bytes::spec_u64_to_le_bytes(section.alignment)
        + string_record_spec(TRANSFORM_ID_BYTES@)
        + section.sha256@
}

closed spec fn section_records_from_spec(
    sections: Seq<WeightSection>,
    index: nat,
) -> Seq<u8>
    recommends index <= sections.len(),
    decreases sections.len() - index,
{
    if index < sections.len() {
        section_record_spec(sections[index as int])
            + section_records_from_spec(sections, index + 1)
    } else {
        Seq::empty()
    }
}

closed spec fn section_records_spec(sections: Seq<WeightSection>) -> Seq<u8> {
    section_records_from_spec(sections, 0)
}

closed spec fn manifest_record_spec(
    role: Qwen3ModelRole,
    source_weights_id: Seq<u8>,
    source_artifact_bytes: u64,
    tensor_data_bytes: u64,
    output_bytes: u64,
    sections: Seq<WeightSection>,
) -> Seq<u8> {
    MANIFEST_DOMAIN@
        + vstd::bytes::spec_u32_to_le_bytes(PREPACKED_WEIGHT_MANIFEST_VERSION)
        + seq![role_code_spec(role)]
        + source_weights_id
        + vstd::bytes::spec_u64_to_le_bytes(source_artifact_bytes)
        + vstd::bytes::spec_u64_to_le_bytes(tensor_data_bytes)
        + vstd::bytes::spec_u64_to_le_bytes(output_bytes)
        + vstd::bytes::spec_u32_to_le_bytes(sections.len() as u32)
        + section_records_spec(sections)
}

closed spec fn destination_layout_from_spec(
    role: Qwen3ModelRole,
    output_bytes: u64,
    sections: Seq<WeightSection>,
    index: nat,
    expected: u64,
) -> bool
    recommends index <= sections.len(),
    decreases sections.len() - index,
{
    if index < sections.len() {
        let section = sections[index as int];
        &&& destination_section_spec(section, role, expected)
        &&& expected <= u64::MAX - section.destination_length
        &&& destination_layout_from_spec(
            role,
            output_bytes,
            sections,
            index + 1,
            (expected + section.destination_length) as u64,
        )
    } else {
        expected == output_bytes
    }
}

pub open spec fn destination_section_spec(
    section: WeightSection,
    role: Qwen3ModelRole,
    expected: u64,
) -> bool {
    &&& section.role_spec() == role
    &&& section.dtype_spec() == TensorDType::Bf16
    &&& section.transform_spec() == WeightTransform::Bf16RowMajorIdentityV1
    &&& section.alignment_spec() == 2
    &&& section.destination_range_spec().0 == expected
    &&& section.destination_range_spec().1 == section.source_range_spec().1
    &&& section.destination_range_spec().0 % 2 == 0
    &&& section.destination_range_spec().1 % 2 == 0
}

pub closed spec fn destination_layout_spec(
    role: Qwen3ModelRole,
    output_bytes: u64,
    sections: Seq<WeightSection>,
) -> bool {
    destination_layout_from_spec(role, output_bytes, sections, 0, 0)
}

}

impl WeightSection {
    /// Resolves the canonical tensor name and retained shape to a typed Qwen3
    /// coordinate and its role-local ordinal.
    ///
    /// This uses the same closed name classifier as safetensors admission. It
    /// does not authenticate bytes or grant allocation or execution authority.
    ///
    /// # Errors
    ///
    /// Returns [`SafetensorsError`] if the retained name, layer, rank, or shape
    /// is not an exact member of the admitted Qwen3 role schema.
    pub fn qwen3_metadata(&self) -> Result<(Qwen3TensorMetadata, u32), SafetensorsError> {
        let (kind, layer, ordinal) = classify_tensor_name(self.role, &self.tensor_name)?;
        let metadata = Qwen3TensorMetadata {
            role: self.role,
            kind,
            layer,
            dtype: self.dtype,
            rank: self.rank,
            dimension_0: self.dimension_0,
            dimension_1: self.dimension_1,
        };
        metadata
            .validate()
            .map_err(|error| SafetensorsError::TensorSchema {
                tensor: self.tensor_name.clone(),
                error,
            })?;
        Ok((metadata, ordinal))
    }
}

struct SourceStreamState {
    hasher: Sha256,
    total: u64,
    buffer: Box<[u8]>,
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

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ManifestBuildError {
    CompleteOutput,
    DestinationLayout,
    DestinationCoverage,
    DestinationOverflow,
    ManifestCapacity,
    ManifestSectionCount,
    ManifestStringLength,
    ManifestTooLarge,
}

const fn role_code(role: Qwen3ModelRole) -> (code: u8)
    ensures code == role_code_spec(role),
{
    match role {
        Qwen3ModelRole::Target8B => 1,
        Qwen3ModelRole::Draft06B => 2,
    }
}

const fn dtype_code(dtype: TensorDType) -> (code: u8)
    ensures code == dtype_code_spec(dtype),
{
    match dtype {
        TensorDType::Bf16 => 1,
    }
}

fn roles_equal(left: Qwen3ModelRole, right: Qwen3ModelRole) -> (equal: bool)
    ensures equal == (left == right),
{
    match left {
        Qwen3ModelRole::Target8B => match right {
            Qwen3ModelRole::Target8B => true,
            Qwen3ModelRole::Draft06B => false,
        },
        Qwen3ModelRole::Draft06B => match right {
            Qwen3ModelRole::Target8B => false,
            Qwen3ModelRole::Draft06B => true,
        },
    }
}

fn dtypes_equal(left: TensorDType, right: TensorDType) -> (equal: bool)
    ensures equal == (left == right),
{
    match (left, right) {
        (TensorDType::Bf16, TensorDType::Bf16) => true,
    }
}

fn transforms_equal(left: WeightTransform, right: WeightTransform) -> (equal: bool)
    ensures equal == (left == right),
{
    match (left, right) {
        (WeightTransform::Bf16RowMajorIdentityV1, WeightTransform::Bf16RowMajorIdentityV1) => true,
    }
}

fn append_bytes(record: &mut Vec<u8>, bytes: &[u8])
    ensures final(record)@ == old(record)@ + bytes@,
{
    let ghost original = record@;
    let mut index = 0;
    while index < bytes.len()
        invariant
            index <= bytes.len(),
            record@ == original + bytes@.subrange(0, index as int),
        decreases bytes.len() - index,
    {
        record.push(bytes[index]);
        index += 1;
    }
    assert(bytes@.subrange(0, bytes@.len() as int) == bytes@);
}

fn append_u32(record: &mut Vec<u8>, value: u32)
    ensures
        final(record)@ == old(record)@ + vstd::bytes::spec_u32_to_le_bytes(value),
        final(record)@.len() == old(record)@.len() + 4,
{
    let encoded = u32_to_le_bytes(value);
    append_bytes(record, &encoded);
}

fn append_u64(record: &mut Vec<u8>, value: u64)
    ensures
        final(record)@ == old(record)@ + vstd::bytes::spec_u64_to_le_bytes(value),
        final(record)@.len() == old(record)@.len() + 8,
{
    let encoded = u64_to_le_bytes(value);
    append_bytes(record, &encoded);
}

fn append_length_prefixed_bytes(
    record: &mut Vec<u8>,
    bytes: &[u8],
) -> (result: Result<(), ManifestBuildError>)
    ensures
        result.is_ok() ==> final(record)@ == old(record)@ + string_record_spec(bytes@),
        result.is_ok() ==> final(record)@.len() <= MAX_MANIFEST_BYTES,
{
    if bytes.len() > u32::MAX as usize {
        return Err(ManifestBuildError::ManifestStringLength);
    }
    if record.len() > MAX_MANIFEST_BYTES - 4 {
        return Err(ManifestBuildError::ManifestTooLarge);
    }
    let with_length = record.len() + 4;
    if bytes.len() > MAX_MANIFEST_BYTES - with_length {
        return Err(ManifestBuildError::ManifestTooLarge);
    }
    let ghost original = record@;
    let length = match u32::try_from(bytes.len()) {
        Ok(length) => length,
        Err(_) => return Err(ManifestBuildError::ManifestStringLength),
    };
    append_u32(record, length);
    append_bytes(record, bytes);
    assert(record@ == original + string_record_spec(bytes@));
    assert(record@.len() == original.len() + 4 + bytes@.len());
    assert(record@.len() <= MAX_MANIFEST_BYTES);
    Ok(())
}

fn append_manifest_section(
    record: &mut Vec<u8>,
    section: &WeightSection,
) -> (result: Result<(), ManifestBuildError>)
    requires old(record)@.len() <= MAX_MANIFEST_BYTES,
    ensures
        result.is_ok() ==> final(record)@ == old(record)@ + section_record_spec(*section),
        result.is_ok() ==> final(record)@.len() <= MAX_MANIFEST_BYTES,
{
    let ghost original = record@;
    let tensor_name: &str = &section.tensor_name;
    match append_length_prefixed_bytes(record, tensor_name.as_bytes()) {
        Ok(()) => {},
        Err(error) => return Err(error),
    }
    record.push(role_code(section.role));
    record.push(dtype_code(section.dtype));
    append_u32(record, section.rank);
    append_u32(record, section.dimension_0);
    append_u32(record, section.dimension_1);
    let source_artifact: &str = &section.source_artifact;
    match append_length_prefixed_bytes(record, source_artifact.as_bytes()) {
        Ok(()) => {},
        Err(error) => return Err(error),
    }
    append_u64(record, section.source_offset);
    append_u64(record, section.source_length);
    append_u64(record, section.destination_offset);
    append_u64(record, section.destination_length);
    append_u64(record, section.alignment);
    match append_length_prefixed_bytes(record, &TRANSFORM_ID_BYTES) {
        Ok(()) => {},
        Err(error) => return Err(error),
    }
    append_bytes(record, &section.sha256);
    if record.len() > MAX_MANIFEST_BYTES {
        return Err(ManifestBuildError::ManifestTooLarge);
    }
    assert(record@ == original + section_record_spec(*section));
    Ok(())
}

fn append_manifest_sections(
    record: &mut Vec<u8>,
    sections: &[WeightSection],
    index: usize,
) -> (result: Result<(), ManifestBuildError>)
    requires
        index <= sections.len(),
        old(record)@.len() <= MAX_MANIFEST_BYTES,
    ensures
        result.is_ok() ==> final(record)@
            == old(record)@ + section_records_from_spec(sections@, index as nat),
        result.is_ok() ==> final(record)@.len() <= MAX_MANIFEST_BYTES,
    decreases sections.len() - index,
{
    reveal(section_records_from_spec);
    if index == sections.len() {
        return Ok(());
    }
    match append_manifest_section(record, &sections[index]) {
        Ok(()) => {},
        Err(error) => return Err(error),
    }
    match append_manifest_sections(record, sections, index + 1) {
        Ok(()) => Ok(()),
        Err(error) => Err(error),
    }
}

fn destination_section_valid(
    section: &WeightSection,
    role: Qwen3ModelRole,
    expected: u64,
) -> (valid: bool)
    ensures valid ==> destination_section_spec(*section, role, expected),
{
    reveal(destination_section_spec);
    reveal(WeightSection::role_spec);
    reveal(WeightSection::dtype_spec);
    reveal(WeightSection::transform_spec);
    reveal(WeightSection::alignment_spec);
    reveal(WeightSection::destination_range_spec);
    reveal(WeightSection::source_range_spec);
    if !roles_equal(section.role, role) {
        return false;
    }
    if !dtypes_equal(section.dtype, TensorDType::Bf16) {
        return false;
    }
    if !transforms_equal(section.transform, WeightTransform::Bf16RowMajorIdentityV1) {
        return false;
    }
    if section.alignment != SECTION_ALIGNMENT {
        return false;
    }
    if section.destination_offset != expected {
        return false;
    }
    if section.destination_length != section.source_length {
        return false;
    }
    if !section.destination_offset.is_multiple_of(SECTION_ALIGNMENT) {
        return false;
    }
    if !section.destination_length.is_multiple_of(SECTION_ALIGNMENT) {
        return false;
    }
    proof {
        assert(SECTION_ALIGNMENT == 2);
        assert(section.destination_offset % 2 == 0);
        assert(section.destination_length % 2 == 0);
        assert(section.role_spec() == section.role);
        assert(section.dtype_spec() == section.dtype);
        assert(section.transform_spec() == section.transform);
        assert(section.alignment_spec() == section.alignment);
        assert(section.destination_range_spec() == (
            section.destination_offset,
            section.destination_length,
        ));
        assert(section.source_range_spec() == (section.source_offset, section.source_length));
        assert(section.role_spec() == role);
        assert(section.dtype_spec() == TensorDType::Bf16);
        assert(section.transform_spec() == WeightTransform::Bf16RowMajorIdentityV1);
        assert(section.alignment_spec() == 2);
        assert(section.destination_range_spec().0 == expected);
        assert(section.destination_range_spec().1 == section.source_range_spec().1);
        assert(section.destination_range_spec().0 % 2 == 0);
        assert(section.destination_range_spec().1 % 2 == 0);
        assert(destination_section_spec(*section, role, expected));
    }
    true
}

fn validate_destination_from(
    role: Qwen3ModelRole,
    output_bytes: u64,
    sections: &[WeightSection],
    index: usize,
    expected: u64,
) -> (result: Result<(), ManifestBuildError>)
    requires index <= sections.len(),
    ensures
        result.is_ok() ==> destination_layout_from_spec(
            role,
            output_bytes,
            sections@,
            index as nat,
            expected,
        ),
    decreases sections.len() - index,
{
    reveal(destination_layout_from_spec);
    if index == sections.len() {
        return if expected == output_bytes {
            Ok(())
        } else {
            Err(ManifestBuildError::DestinationCoverage)
        };
    }
    let section = &sections[index];
    assert(*section == sections@[index as int]);
    if !destination_section_valid(section, role, expected) {
        return Err(ManifestBuildError::DestinationLayout);
    }
    let next = match expected.checked_add(section.destination_length) {
        Some(next) => next,
        None => return Err(ManifestBuildError::DestinationOverflow),
    };
    assert(expected <= u64::MAX - section.destination_length);
    assert(next == (expected + section.destination_length) as u64);
    validate_destination_from(role, output_bytes, sections, index + 1, next)
}

fn validate_destination_coverage_verified(
    role: Qwen3ModelRole,
    output_bytes: u64,
    sections: &[WeightSection],
) -> (result: Result<(), ManifestBuildError>)
    ensures result.is_ok() ==> destination_layout_spec(role, output_bytes, sections@),
{
    reveal(destination_layout_spec);
    validate_destination_from(role, output_bytes, sections, 0, 0)
}

fn encode_manifest_record_verified(
    role: Qwen3ModelRole,
    descriptor: WeightDescriptor,
    output_bytes: u64,
    sections: &[WeightSection],
) -> (result: Result<Vec<u8>, ManifestBuildError>)
    ensures match result {
        Ok(record) => {
            &&& record@ == manifest_record_spec(
                role,
                descriptor.weights_id@,
                descriptor.artifact_bytes,
                descriptor.tensor_data_bytes,
                output_bytes,
                sections@,
            )
            &&& record@.len() <= MAX_MANIFEST_BYTES
        },
        Err(_) => true,
    },
{
    let scaled = match sections.len().checked_mul(160) {
        Some(scaled) => scaled,
        None => return Err(ManifestBuildError::ManifestCapacity),
    };
    let capacity = match scaled.checked_add(64) {
        Some(capacity) => capacity,
        None => return Err(ManifestBuildError::ManifestCapacity),
    };
    if capacity > MAX_MANIFEST_BYTES {
        return Err(ManifestBuildError::ManifestTooLarge);
    }
    if sections.len() > u32::MAX as usize {
        return Err(ManifestBuildError::ManifestSectionCount);
    }

    let mut record = Vec::with_capacity(capacity);
    append_bytes(&mut record, &MANIFEST_DOMAIN);
    append_u32(&mut record, PREPACKED_WEIGHT_MANIFEST_VERSION);
    record.push(role_code(role));
    append_bytes(&mut record, &descriptor.weights_id);
    append_u64(&mut record, descriptor.artifact_bytes);
    append_u64(&mut record, descriptor.tensor_data_bytes);
    append_u64(&mut record, output_bytes);
    let section_count = match u32::try_from(sections.len()) {
        Ok(section_count) => section_count,
        Err(_) => return Err(ManifestBuildError::ManifestSectionCount),
    };
    append_u32(&mut record, section_count);
    assert(record@.len() == 101);
    assert(record@.len() <= MAX_MANIFEST_BYTES);
    match append_manifest_sections(&mut record, sections, 0) {
        Ok(()) => {},
        Err(error) => return Err(error),
    }
    assert(record@ == manifest_record_spec(
        role,
        descriptor.weights_id@,
        descriptor.artifact_bytes,
        descriptor.tensor_data_bytes,
        output_bytes,
        sections@,
    ));
    Ok(record)
}

fn build_weight_manifest_verified(
    role: Qwen3ModelRole,
    descriptor: WeightDescriptor,
    output_bytes: u64,
    sections: Vec<WeightSection>,
) -> (result: Result<WeightSectionManifest, ManifestBuildError>)
    ensures match result {
        Ok(manifest) => {
            &&& manifest.valid_commitment()
            &&& manifest.role_spec() == role
            &&& manifest.source_weights_id_spec() == descriptor.weights_id@
            &&& manifest.source_artifact_bytes_spec() == descriptor.artifact_bytes
            &&& manifest.tensor_data_bytes_spec() == descriptor.tensor_data_bytes
            &&& manifest.output_bytes_spec() == output_bytes
            &&& manifest.sections_spec() == sections@
        },
        Err(_) => true,
    },
{
    if output_bytes != descriptor.tensor_data_bytes
        || output_bytes != role.tensor_data_bytes()
        || sections.len() != role.tensor_count() as usize
    {
        return Err(ManifestBuildError::CompleteOutput);
    }
    match validate_destination_coverage_verified(role, output_bytes, &sections) {
        Ok(()) => {},
        Err(error) => return Err(error),
    }
    let canonical_bytes =
        encode_manifest_record_verified(role, descriptor, output_bytes, &sections)?;
    assert(canonical_bytes@.len() <= MAX_MANIFEST_BYTES);
    assert(canonical_bytes@.len() <= u64::MAX / 8);
    let aggregate_id = crate::sha256::digest(&canonical_bytes);
    let manifest = WeightSectionManifest {
        version: PREPACKED_WEIGHT_MANIFEST_VERSION,
        role,
        source_weights_id: descriptor.weights_id,
        source_artifact_bytes: descriptor.artifact_bytes,
        tensor_data_bytes: descriptor.tensor_data_bytes,
        output_bytes,
        sections,
        canonical_bytes,
        aggregate_id,
    };
    assert(manifest.valid_commitment());
    Ok(manifest)
}

}

fn finish_prepacked(
    role: Qwen3ModelRole,
    descriptor: WeightDescriptor,
    output_bytes: u64,
    sections: Vec<WeightSection>,
) -> Result<PrepackedWeightSet, WeightStreamError> {
    let manifest = build_weight_manifest_verified(role, descriptor, output_bytes, sections)
        .map_err(map_manifest_build_error)?;
    Ok(PrepackedWeightSet {
        role,
        descriptor,
        manifest,
        seal: PrepackedSeal,
    })
}

const fn map_manifest_build_error(error: ManifestBuildError) -> WeightStreamError {
    match error {
        ManifestBuildError::CompleteOutput => {
            WeightStreamError::InvalidLayout("complete output count or length drifted")
        }
        ManifestBuildError::DestinationLayout => {
            WeightStreamError::InvalidLayout("destination sections are not canonical and complete")
        }
        ManifestBuildError::DestinationCoverage => {
            WeightStreamError::InvalidLayout("destination sections do not cover output")
        }
        ManifestBuildError::DestinationOverflow => {
            WeightStreamError::ArithmeticOverflow("destination coverage")
        }
        ManifestBuildError::ManifestCapacity => {
            WeightStreamError::ArithmeticOverflow("manifest capacity")
        }
        ManifestBuildError::ManifestSectionCount => {
            WeightStreamError::ArithmeticOverflow("manifest section count")
        }
        ManifestBuildError::ManifestStringLength => {
            WeightStreamError::ArithmeticOverflow("manifest string length")
        }
        ManifestBuildError::ManifestTooLarge => WeightStreamError::ManifestTooLarge,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        finish_prepacked, flush_output, parse_header, require_shard_name, stream_file,
        validate_stream_plan, FilePin, ParsedShard, ParsedTensor, PrepackedWeightSet,
        WeightSection, WeightStreamError, WeightTransform, DRAFT_FILE_PIN, SECTION_ALIGNMENT,
        TARGET_SHARD_PINS,
    };
    use crate::tokenizer::tests::{authenticated_assets, test_tokenizer};
    use crate::{
        build_prepacked_deployment_bundle, sha256, BuildError, SafetensorsError, WeightDescriptor,
        QWEN3_DRAFT_TENSOR_DATA_BYTES, QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
        QWEN3_DRAFT_WEIGHT_SHA256, QWEN3_TARGET_TENSOR_DATA_BYTES,
        QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES, QWEN3_TARGET_WEIGHT_SET_SHA256,
    };
    use ferric_spec::{
        Qwen3ModelRole, Qwen3TensorKind, Qwen3TensorMetadata, TensorDType, QWEN3_NO_LAYER,
    };
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

    fn validate_destination_coverage(
        role: Qwen3ModelRole,
        output_bytes: u64,
        sections: &[WeightSection],
    ) -> Result<(), WeightStreamError> {
        super::validate_destination_coverage_verified(role, output_bytes, sections)
            .map_err(super::map_manifest_build_error)
    }

    fn encode_manifest_record(
        role: Qwen3ModelRole,
        descriptor: WeightDescriptor,
        output_bytes: u64,
        sections: &[WeightSection],
    ) -> Result<Vec<u8>, WeightStreamError> {
        super::encode_manifest_record_verified(role, descriptor, output_bytes, sections)
            .map_err(super::map_manifest_build_error)
    }

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

    fn tiny_sections() -> Vec<WeightSection> {
        let data = tiny_data();
        let (bytes, pin) = synthetic_file(&data);
        stream_tiny(&mut Cursor::new(bytes), &mut Vec::new(), pin, tiny_plan())
            .expect("complete tiny stream")
            .1
    }

    fn tiny_manifest_record(sections: &[WeightSection]) -> Vec<u8> {
        encode_manifest_record(
            Qwen3ModelRole::Draft06B,
            descriptor(Qwen3ModelRole::Draft06B),
            TOTAL_BYTES,
            sections,
        )
        .expect("bounded tiny manifest")
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
        finish_prepacked(role, descriptor, destination, sections)
            .expect("test sections satisfy the verified production finalizer")
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
    fn destination_manifest_layout_mutations_fail_closed() {
        let mut gap = tiny_sections();
        gap[1].destination_offset += SECTION_ALIGNMENT;
        assert!(
            validate_destination_coverage(Qwen3ModelRole::Draft06B, TOTAL_BYTES, &gap).is_err()
        );

        let mut overlap = tiny_sections();
        overlap[1].destination_offset -= SECTION_ALIGNMENT;
        assert!(
            validate_destination_coverage(Qwen3ModelRole::Draft06B, TOTAL_BYTES, &overlap,)
                .is_err()
        );

        let mut reordered = tiny_sections();
        reordered.swap(0, 1);
        assert!(
            validate_destination_coverage(Qwen3ModelRole::Draft06B, TOTAL_BYTES, &reordered,)
                .is_err()
        );

        let mut short = tiny_sections();
        short.pop();
        assert!(
            validate_destination_coverage(Qwen3ModelRole::Draft06B, TOTAL_BYTES, &short).is_err()
        );

        let mut wrong_role = tiny_sections();
        wrong_role[0].role = Qwen3ModelRole::Target8B;
        assert!(
            validate_destination_coverage(Qwen3ModelRole::Draft06B, TOTAL_BYTES, &wrong_role,)
                .is_err()
        );

        let mut wrong_alignment = tiny_sections();
        wrong_alignment[0].alignment = 4;
        assert!(validate_destination_coverage(
            Qwen3ModelRole::Draft06B,
            TOTAL_BYTES,
            &wrong_alignment,
        )
        .is_err());

        let mut length_mismatch = tiny_sections();
        length_mismatch[0].destination_length -= SECTION_ALIGNMENT;
        assert!(validate_destination_coverage(
            Qwen3ModelRole::Draft06B,
            TOTAL_BYTES,
            &length_mismatch,
        )
        .is_err());

        let mut overflow = tiny_sections();
        overflow[0].source_length = u64::MAX - 1;
        overflow[0].destination_length = u64::MAX - 1;
        overflow[1].source_length = SECTION_ALIGNMENT;
        overflow[1].destination_offset = u64::MAX - 1;
        overflow[1].destination_length = SECTION_ALIGNMENT;
        assert_eq!(
            validate_destination_coverage(Qwen3ModelRole::Draft06B, 0, &overflow),
            Err(WeightStreamError::ArithmeticOverflow(
                "destination coverage"
            ))
        );
    }

    #[test]
    fn canonical_manifest_framing_binds_header_and_section_fields() {
        let baseline_sections = tiny_sections();
        let baseline = tiny_manifest_record(&baseline_sections);
        let baseline_digest = sha256::digest(&baseline);
        assert!(baseline.starts_with(b"ferric.prepacked-weight-sections.v1\0"));
        assert_eq!(
            &baseline[36..40],
            &super::PREPACKED_WEIGHT_MANIFEST_VERSION.to_le_bytes()
        );

        let assert_changed = |sections: &[WeightSection]| {
            let changed = tiny_manifest_record(sections);
            assert_ne!(changed, baseline);
            assert_ne!(sha256::digest(&changed), baseline_digest);
        };

        let mut changed = tiny_sections();
        changed[0].tensor_name.push_str(".mutated");
        assert_changed(&changed);
        let mut changed = tiny_sections();
        changed[0].role = Qwen3ModelRole::Target8B;
        assert_changed(&changed);
        let mut changed = tiny_sections();
        changed[0].rank += 1;
        assert_changed(&changed);
        let mut changed = tiny_sections();
        changed[0].dimension_0 += 1;
        assert_changed(&changed);
        let mut changed = tiny_sections();
        changed[0].dimension_1 += 1;
        assert_changed(&changed);
        let mut changed = tiny_sections();
        changed[0].source_artifact.push_str(".mutated");
        assert_changed(&changed);
        let mut changed = tiny_sections();
        changed[0].source_offset += SECTION_ALIGNMENT;
        assert_changed(&changed);
        let mut changed = tiny_sections();
        changed[0].source_length += SECTION_ALIGNMENT;
        assert_changed(&changed);
        let mut changed = tiny_sections();
        changed[0].destination_offset += SECTION_ALIGNMENT;
        assert_changed(&changed);
        let mut changed = tiny_sections();
        changed[0].destination_length += SECTION_ALIGNMENT;
        assert_changed(&changed);
        let mut changed = tiny_sections();
        changed[0].alignment += SECTION_ALIGNMENT;
        assert_changed(&changed);
        let mut changed = tiny_sections();
        changed[0].sha256[0] ^= 1;
        assert_changed(&changed);

        let descriptor = descriptor(Qwen3ModelRole::Draft06B);
        let header_variants = [
            WeightDescriptor {
                weights_id: [0; 32],
                ..descriptor
            },
            WeightDescriptor {
                artifact_bytes: descriptor.artifact_bytes + 1,
                ..descriptor
            },
            WeightDescriptor {
                tensor_data_bytes: descriptor.tensor_data_bytes + 1,
                ..descriptor
            },
        ];
        for changed_descriptor in header_variants {
            let changed = encode_manifest_record(
                Qwen3ModelRole::Draft06B,
                changed_descriptor,
                TOTAL_BYTES,
                &baseline_sections,
            )
            .expect("bounded changed header");
            assert_ne!(changed, baseline);
            assert_ne!(sha256::digest(&changed), baseline_digest);
        }
        let changed_output = encode_manifest_record(
            Qwen3ModelRole::Draft06B,
            descriptor,
            TOTAL_BYTES + SECTION_ALIGNMENT,
            &baseline_sections,
        )
        .expect("bounded changed output length");
        assert_ne!(changed_output, baseline);
        assert_ne!(sha256::digest(&changed_output), baseline_digest);

        let changed_role = encode_manifest_record(
            Qwen3ModelRole::Target8B,
            descriptor,
            TOTAL_BYTES,
            &baseline_sections,
        )
        .expect("bounded changed role");
        assert_ne!(changed_role, baseline);
        assert_ne!(sha256::digest(&changed_role), baseline_digest);

        let changed_count = tiny_manifest_record(&baseline_sections[..1]);
        assert_ne!(changed_count, baseline);
        assert_ne!(sha256::digest(&changed_count), baseline_digest);
    }

    #[test]
    fn verified_finalizer_commits_complete_official_layouts() {
        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            let prepacked = test_prepacked(role);
            let manifest = prepacked.manifest();
            assert_eq!(manifest.role(), role);
            assert_eq!(manifest.sections().len(), role.tensor_count() as usize);
            assert_eq!(manifest.output_bytes(), role.tensor_data_bytes());
            assert_eq!(
                manifest.aggregate_id(),
                sha256::digest(manifest.canonical_bytes())
            );

            let mut seen = vec![false; role.tensor_count() as usize];
            for section in manifest.sections() {
                let (metadata, ordinal) = section
                    .qwen3_metadata()
                    .expect("sealed section resolves through the admission classifier");
                assert_eq!(metadata.role, role);
                assert!((ordinal as usize) < seen.len());
                assert!(!seen[ordinal as usize]);
                seen[ordinal as usize] = true;
            }
            assert!(seen.into_iter().all(|present| present));
        }
    }

    #[test]
    fn typed_section_metadata_preserves_global_and_layer_coordinates() {
        let target = test_prepacked(Qwen3ModelRole::Target8B);
        let embedding = target
            .manifest()
            .sections()
            .iter()
            .find(|section| section.tensor_name() == "model.embed_tokens.weight")
            .expect("target embedding section");
        let (metadata, ordinal) = embedding.qwen3_metadata().unwrap();
        assert_eq!(metadata.kind, Qwen3TensorKind::TokenEmbedding);
        assert_eq!(metadata.layer, QWEN3_NO_LAYER);
        assert_eq!(ordinal, 0);

        let draft = test_prepacked(Qwen3ModelRole::Draft06B);
        let query = draft
            .manifest()
            .sections()
            .iter()
            .find(|section| section.tensor_name() == "model.layers.27.self_attn.q_proj.weight")
            .expect("last draft query projection section");
        let (metadata, ordinal) = query.qwen3_metadata().unwrap();
        assert_eq!(metadata.kind, Qwen3TensorKind::QueryProjection);
        assert_eq!(metadata.layer, 27);
        assert_eq!(ordinal, 2 + 27 * 11 + 9);
    }

    #[test]
    fn typed_section_metadata_rejects_name_and_shape_drift() {
        let mut invalid_name = tiny_sections();
        invalid_name[0].tensor_name.push_str(".drift");
        assert!(matches!(
            invalid_name[0].qwen3_metadata(),
            Err(SafetensorsError::InvalidTensorName(_))
        ));

        let mut invalid_shape = tiny_sections();
        invalid_shape[0].dimension_0 += 1;
        assert!(matches!(
            invalid_shape[0].qwen3_metadata(),
            Err(SafetensorsError::TensorSchema { .. })
        ));
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
