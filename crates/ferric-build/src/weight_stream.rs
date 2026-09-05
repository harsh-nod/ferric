use crate::safetensors::{
    classify_tensor_name, parse_header, parse_target_index, require_shard_name,
    validate_offset_coverage, validate_roster, validate_shard_mapping, FilePin, ParsedShard,
    ParsedTensor, SafetensorsError, SafetensorsSource, DRAFT_FILE_PIN,
    MAX_SAFETENSORS_HEADER_BYTES, TARGET_SHARD_PINS,
};
use crate::sha256::Sha256;
use crate::{
    ManifestCommitment, WeightDescriptor, QWEN3_DRAFT_TENSOR_DATA_BYTES,
    QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES, QWEN3_DRAFT_WEIGHT_SHA256, QWEN3_TARGET_TENSOR_DATA_BYTES,
    QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES, QWEN3_TARGET_WEIGHT_SET_SHA256,
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
/// SHA-256 of the canonical manifest produced from the exact pinned Qwen3-8B sources.
pub const QWEN3_TARGET_PREPACKED_MANIFEST_SHA256: [u8; 32] = [
    0xd6, 0xd8, 0xb5, 0x5c, 0x73, 0x59, 0x56, 0x45, 0xde, 0xde, 0xad, 0xe0, 0x05, 0xd4, 0xe8,
    0x53, 0x09, 0x8c, 0xf3, 0xa2, 0x2f, 0xaf, 0x1c, 0x04, 0x59, 0x22, 0xd0, 0x03, 0x67, 0xec,
    0xff, 0x7e,
];
/// Canonical Qwen3-8B prepacked manifest length.
pub const QWEN3_TARGET_PREPACKED_MANIFEST_BYTES: u32 = 77_591;
/// SHA-256 of the canonical manifest produced from the exact pinned Qwen3-0.6B source.
pub const QWEN3_DRAFT_PREPACKED_MANIFEST_SHA256: [u8; 32] = [
    0xcd, 0x97, 0xbd, 0xa6, 0x5e, 0x4a, 0x40, 0xd4, 0x9a, 0xe8, 0x85, 0x44, 0x4c, 0xc6, 0xa1,
    0x5a, 0x47, 0x13, 0x5d, 0x10, 0x73, 0x07, 0x5d, 0xda, 0x10, 0x4b, 0x36, 0x00, 0x68, 0xe4,
    0x74, 0x1e,
];
/// Canonical Qwen3-0.6B prepacked manifest length.
pub const QWEN3_DRAFT_PREPACKED_MANIFEST_BYTES: u32 = 55_798;

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
/// This value is returned only after either exact source EOF/full-file SHA-256
/// authentication or an exact match to the compiled canonical-manifest trust
/// anchor followed by every section digest and output EOF. It is intentionally
/// not `Clone`. Callers must publish only the byte snapshot authenticated by
/// this authority.
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

#[cfg(test)]
impl WeightSection {
    pub(crate) fn test_set_role(&mut self, role: Qwen3ModelRole) {
        self.role = role;
    }

    pub(crate) fn test_set_tensor_name(&mut self, name: &str) {
        self.tensor_name = name.to_owned();
    }

    pub(crate) fn test_increment_dimension_0(&mut self) {
        self.dimension_0 += 1;
    }

    pub(crate) fn test_increment_destination_offset(&mut self) {
        self.destination_offset += SECTION_ALIGNMENT;
    }

    pub(crate) fn test_decrement_destination_length(&mut self) {
        self.destination_length -= SECTION_ALIGNMENT;
    }
}

#[cfg(test)]
impl WeightSectionManifest {
    pub(crate) fn test_sections_mut(&mut self) -> &mut Vec<WeightSection> {
        &mut self.sections
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

/// Failure while reopening a persisted prepacked weight artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersistedPrepackedWeightError {
    /// The admission-record commitment does not describe the exact pinned role.
    InvalidCommitment(&'static str),
    /// The bounded canonical manifest record is malformed or noncanonical.
    InvalidManifest(&'static str),
    /// A manifest string field is not valid UTF-8.
    InvalidManifestUtf8(&'static str),
    /// Parsed section metadata does not form the exact admitted Qwen3 roster.
    InvalidSectionLayout(&'static str),
    /// Reconstructing the canonical typed manifest failed.
    Manifest(WeightStreamError),
    /// The persisted weight reader returned an I/O error.
    WeightIo(io::ErrorKind),
    /// EOF arrived before the complete committed prepacked image.
    WeightEarlyEof {
        /// Exact committed image length.
        expected: u64,
        /// Bytes observed before EOF.
        actual: u64,
    },
    /// A byte followed the complete committed prepacked image.
    WeightTrailingData,
    /// One persisted tensor section differed from its committed digest.
    SectionDigestMismatch {
        /// Zero-based section position in the canonical manifest.
        section: usize,
    },
}

impl fmt::Display for PersistedPrepackedWeightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "persisted prepacked weights rejected: {self:?}")
    }
}

impl std::error::Error for PersistedPrepackedWeightError {}

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

struct PersistedManifestReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> PersistedManifestReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], PersistedPrepackedWeightError> {
        let end = self.offset.checked_add(length).ok_or(
            PersistedPrepackedWeightError::InvalidManifest("field extent overflow"),
        )?;
        let value = self.bytes.get(self.offset..end).ok_or(
            PersistedPrepackedWeightError::InvalidManifest("truncated field"),
        )?;
        self.offset = end;
        Ok(value)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], PersistedPrepackedWeightError> {
        self.take(N)?
            .try_into()
            .map_err(|_| PersistedPrepackedWeightError::InvalidManifest("fixed field"))
    }

    fn u8(&mut self) -> Result<u8, PersistedPrepackedWeightError> {
        Ok(self.array::<1>()?[0])
    }

    fn u32(&mut self) -> Result<u32, PersistedPrepackedWeightError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, PersistedPrepackedWeightError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn string(&mut self, field: &'static str) -> Result<String, PersistedPrepackedWeightError> {
        let length = usize::try_from(self.u32()?).map_err(|_| {
            PersistedPrepackedWeightError::InvalidManifest("string length overflow")
        })?;
        let bytes = self.take(length)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| PersistedPrepackedWeightError::InvalidManifestUtf8(field))
    }

    fn finish(self) -> Result<(), PersistedPrepackedWeightError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(PersistedPrepackedWeightError::InvalidManifest(
                "trailing manifest bytes",
            ))
        }
    }
}

struct ParsedPersistedManifest {
    role: Qwen3ModelRole,
    source_weights_id: [u8; 32],
    source_artifact_bytes: u64,
    tensor_data_bytes: u64,
    output_bytes: u64,
    sections: Vec<WeightSection>,
}

fn exact_weight_descriptor(role: Qwen3ModelRole) -> WeightDescriptor {
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

fn exact_persisted_manifest_identity(role: Qwen3ModelRole) -> ([u8; 32], u32) {
    match role {
        Qwen3ModelRole::Target8B => (
            QWEN3_TARGET_PREPACKED_MANIFEST_SHA256,
            QWEN3_TARGET_PREPACKED_MANIFEST_BYTES,
        ),
        Qwen3ModelRole::Draft06B => (
            QWEN3_DRAFT_PREPACKED_MANIFEST_SHA256,
            QWEN3_DRAFT_PREPACKED_MANIFEST_BYTES,
        ),
    }
}

fn validate_persisted_commitment(
    role: Qwen3ModelRole,
    commitment: ManifestCommitment,
    manifest_bytes: &[u8],
) -> Result<WeightDescriptor, PersistedPrepackedWeightError> {
    let descriptor = exact_weight_descriptor(role);
    let (manifest_identity, manifest_bytes_exact) = exact_persisted_manifest_identity(role);
    if commitment.role != role {
        return Err(PersistedPrepackedWeightError::InvalidCommitment("role"));
    }
    if commitment.version != PREPACKED_WEIGHT_MANIFEST_VERSION {
        return Err(PersistedPrepackedWeightError::InvalidCommitment("version"));
    }
    if commitment.source_weights_id != descriptor.weights_id {
        return Err(PersistedPrepackedWeightError::InvalidCommitment(
            "source identity",
        ));
    }
    if commitment.source_artifact_bytes != descriptor.artifact_bytes
        || commitment.tensor_data_bytes != descriptor.tensor_data_bytes
        || commitment.output_bytes != descriptor.tensor_data_bytes
    {
        return Err(PersistedPrepackedWeightError::InvalidCommitment(
            "byte count",
        ));
    }
    if commitment.section_count != role.tensor_count() {
        return Err(PersistedPrepackedWeightError::InvalidCommitment(
            "section count",
        ));
    }
    let manifest_length = u32::try_from(manifest_bytes.len())
        .map_err(|_| PersistedPrepackedWeightError::InvalidCommitment("manifest byte count"))?;
    if manifest_bytes.is_empty()
        || manifest_bytes.len() > MAX_MANIFEST_BYTES
        || commitment.canonical_manifest_bytes != manifest_length
        || commitment.canonical_manifest_bytes != manifest_bytes_exact
    {
        return Err(PersistedPrepackedWeightError::InvalidCommitment(
            "manifest byte count",
        ));
    }
    if commitment.aggregate_id != manifest_identity
        || crate::sha256::digest(manifest_bytes) != manifest_identity
    {
        return Err(PersistedPrepackedWeightError::InvalidCommitment(
            "manifest identity",
        ));
    }
    Ok(descriptor)
}

fn parse_persisted_manifest(
    bytes: &[u8],
) -> Result<ParsedPersistedManifest, PersistedPrepackedWeightError> {
    if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES {
        return Err(PersistedPrepackedWeightError::InvalidManifest(
            "manifest size",
        ));
    }
    let mut reader = PersistedManifestReader::new(bytes);
    if reader.array::<36>()? != MANIFEST_DOMAIN {
        return Err(PersistedPrepackedWeightError::InvalidManifest("magic"));
    }
    if reader.u32()? != PREPACKED_WEIGHT_MANIFEST_VERSION {
        return Err(PersistedPrepackedWeightError::InvalidManifest("version"));
    }
    let role = match reader.u8()? {
        1 => Qwen3ModelRole::Target8B,
        2 => Qwen3ModelRole::Draft06B,
        _ => return Err(PersistedPrepackedWeightError::InvalidManifest("role")),
    };
    let source_weights_id = reader.array()?;
    let source_artifact_bytes = reader.u64()?;
    let tensor_data_bytes = reader.u64()?;
    let output_bytes = reader.u64()?;
    let section_count = reader.u32()?;
    if section_count != role.tensor_count() {
        return Err(PersistedPrepackedWeightError::InvalidManifest(
            "section count",
        ));
    }
    let section_count = usize::try_from(section_count)
        .map_err(|_| PersistedPrepackedWeightError::InvalidManifest("section count overflow"))?;
    let mut sections = Vec::new();
    sections
        .try_reserve_exact(section_count)
        .map_err(|_| PersistedPrepackedWeightError::InvalidManifest("section allocation"))?;
    for _ in 0..section_count {
        let tensor_name = reader.string("tensor name")?;
        let section_role = match reader.u8()? {
            1 => Qwen3ModelRole::Target8B,
            2 => Qwen3ModelRole::Draft06B,
            _ => {
                return Err(PersistedPrepackedWeightError::InvalidManifest(
                    "section role",
                ));
            }
        };
        if reader.u8()? != 1 {
            return Err(PersistedPrepackedWeightError::InvalidManifest(
                "section dtype",
            ));
        }
        let rank = reader.u32()?;
        let dimension_0 = reader.u32()?;
        let dimension_1 = reader.u32()?;
        let source_artifact = reader.string("source artifact")?;
        let source_offset = reader.u64()?;
        let source_length = reader.u64()?;
        let destination_offset = reader.u64()?;
        let destination_length = reader.u64()?;
        let alignment = reader.u64()?;
        let transform = reader.string("transform")?;
        if transform.as_bytes() != TRANSFORM_ID_BYTES {
            return Err(PersistedPrepackedWeightError::InvalidManifest(
                "section transform",
            ));
        }
        let sha256 = reader.array()?;
        sections.push(WeightSection {
            tensor_name,
            role: section_role,
            dtype: TensorDType::Bf16,
            rank,
            dimension_0,
            dimension_1,
            source_artifact,
            source_offset,
            source_length,
            destination_offset,
            destination_length,
            alignment,
            transform: WeightTransform::Bf16RowMajorIdentityV1,
            sha256,
        });
    }
    reader.finish()?;
    Ok(ParsedPersistedManifest {
        role,
        source_weights_id,
        source_artifact_bytes,
        tensor_data_bytes,
        output_bytes,
        sections,
    })
}

fn persisted_source_pin(role: Qwen3ModelRole, name: &str) -> Option<(usize, FilePin)> {
    match role {
        Qwen3ModelRole::Target8B => TARGET_SHARD_PINS
            .iter()
            .copied()
            .enumerate()
            .find(|(_, pin)| pin.name == name),
        Qwen3ModelRole::Draft06B if name == DRAFT_FILE_PIN.name => Some((0, DRAFT_FILE_PIN)),
        Qwen3ModelRole::Draft06B => None,
    }
}

fn validate_persisted_sections(
    manifest: &WeightSectionManifest,
) -> Result<(), PersistedPrepackedWeightError> {
    let role = manifest.role();
    let mut seen = [false; Qwen3ModelRole::Target8B.tensor_count() as usize];
    let mut source_cursors = [0_u64; TARGET_SHARD_PINS.len()];
    let mut expected_target_source = 0_usize;
    match role {
        Qwen3ModelRole::Target8B => {
            for (cursor, pin) in source_cursors.iter_mut().zip(TARGET_SHARD_PINS) {
                *cursor = 8_u64.checked_add(pin.header_bytes).ok_or(
                    PersistedPrepackedWeightError::InvalidSectionLayout("source header"),
                )?;
            }
        }
        Qwen3ModelRole::Draft06B => {
            source_cursors[0] = 8_u64.checked_add(DRAFT_FILE_PIN.header_bytes).ok_or(
                PersistedPrepackedWeightError::InvalidSectionLayout("source header"),
            )?;
        }
    }
    for section in manifest.sections() {
        if section.role != role
            || section.dtype != TensorDType::Bf16
            || section.transform != WeightTransform::Bf16RowMajorIdentityV1
            || section.alignment != SECTION_ALIGNMENT
            || section.sha256 == [0; 32]
        {
            return Err(PersistedPrepackedWeightError::InvalidSectionLayout(
                "section authority",
            ));
        }
        let (_, ordinal) = section
            .qwen3_metadata()
            .map_err(|_| PersistedPrepackedWeightError::InvalidSectionLayout("tensor schema"))?;
        let ordinal = usize::try_from(ordinal)
            .map_err(|_| PersistedPrepackedWeightError::InvalidSectionLayout("tensor ordinal"))?;
        let seen_entry =
            seen.get_mut(ordinal)
                .ok_or(PersistedPrepackedWeightError::InvalidSectionLayout(
                    "tensor ordinal",
                ))?;
        if *seen_entry {
            return Err(PersistedPrepackedWeightError::InvalidSectionLayout(
                "duplicate tensor ordinal",
            ));
        }
        *seen_entry = true;
        let expected_length = u64::from(section.dimension_0)
            .checked_mul(u64::from(section.dimension_1))
            .and_then(|elements| elements.checked_mul(2))
            .ok_or(PersistedPrepackedWeightError::InvalidSectionLayout(
                "tensor byte arithmetic",
            ))?;
        if section.source_length != expected_length || section.destination_length != expected_length
        {
            return Err(PersistedPrepackedWeightError::InvalidSectionLayout(
                "tensor byte length",
            ));
        }
        let (source_index, source_pin) = persisted_source_pin(role, &section.source_artifact)
            .ok_or(PersistedPrepackedWeightError::InvalidSectionLayout(
                "source artifact",
            ))?;
        if role == Qwen3ModelRole::Target8B && source_index != expected_target_source {
            return Err(PersistedPrepackedWeightError::InvalidSectionLayout(
                "source shard order",
            ));
        }
        let source_cursor = source_cursors.get_mut(source_index).ok_or(
            PersistedPrepackedWeightError::InvalidSectionLayout("source artifact index"),
        )?;
        if section.source_offset != *source_cursor
            || !section.source_offset.is_multiple_of(SECTION_ALIGNMENT)
        {
            return Err(PersistedPrepackedWeightError::InvalidSectionLayout(
                "source coverage",
            ));
        }
        *source_cursor = section
            .source_offset
            .checked_add(section.source_length)
            .ok_or(PersistedPrepackedWeightError::InvalidSectionLayout(
                "source range",
            ))?;
        if *source_cursor > source_pin.file_bytes {
            return Err(PersistedPrepackedWeightError::InvalidSectionLayout(
                "source range",
            ));
        }
        if role == Qwen3ModelRole::Target8B && *source_cursor == source_pin.file_bytes {
            expected_target_source = expected_target_source.checked_add(1).ok_or(
                PersistedPrepackedWeightError::InvalidSectionLayout("source shard order"),
            )?;
        }
    }
    let required = role.tensor_count() as usize;
    if manifest.sections().len() != required || seen[..required].iter().any(|entry| !entry) {
        return Err(PersistedPrepackedWeightError::InvalidSectionLayout(
            "complete tensor roster",
        ));
    }
    let source_coverage_complete = match role {
        Qwen3ModelRole::Target8B => source_cursors
            .iter()
            .zip(TARGET_SHARD_PINS)
            .all(|(cursor, pin)| *cursor == pin.file_bytes),
        Qwen3ModelRole::Draft06B => source_cursors[0] == DRAFT_FILE_PIN.file_bytes,
    };
    if !source_coverage_complete {
        return Err(PersistedPrepackedWeightError::InvalidSectionLayout(
            "complete source coverage",
        ));
    }
    Ok(())
}

fn verify_persisted_weight_bytes<R: Read>(
    reader: &mut R,
    manifest: &WeightSectionManifest,
) -> Result<(), PersistedPrepackedWeightError> {
    let mut buffer = vec![0; STREAM_BUFFER_BYTES].into_boxed_slice();
    let mut total = 0_u64;
    for (section_index, section) in manifest.sections().iter().enumerate() {
        let mut hasher = Sha256::new();
        let mut remaining = section.destination_length;
        while remaining != 0 {
            let chunk = usize::try_from(remaining.min(STREAM_BUFFER_BYTES as u64))
                .map_err(|_| PersistedPrepackedWeightError::InvalidSectionLayout("stream chunk"))?;
            let mut filled = 0;
            while filled < chunk {
                match reader.read(&mut buffer[filled..chunk]) {
                    Ok(0) => {
                        return Err(PersistedPrepackedWeightError::WeightEarlyEof {
                            expected: manifest.output_bytes(),
                            actual: total,
                        });
                    }
                    Ok(read) => {
                        hasher.update(&buffer[filled..filled + read]);
                        filled += read;
                        total = total
                            .checked_add(u64::try_from(read).map_err(|_| {
                                PersistedPrepackedWeightError::InvalidSectionLayout(
                                    "stream byte count",
                                )
                            })?)
                            .ok_or(PersistedPrepackedWeightError::InvalidSectionLayout(
                                "stream byte count",
                            ))?;
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(error) => {
                        return Err(PersistedPrepackedWeightError::WeightIo(error.kind()));
                    }
                }
            }
            remaining = remaining
                .checked_sub(u64::try_from(chunk).map_err(|_| {
                    PersistedPrepackedWeightError::InvalidSectionLayout("stream chunk")
                })?)
                .ok_or(PersistedPrepackedWeightError::InvalidSectionLayout(
                    "stream byte count",
                ))?;
        }
        if hasher.finish() != section.sha256 {
            return Err(PersistedPrepackedWeightError::SectionDigestMismatch {
                section: section_index,
            });
        }
    }
    if total != manifest.output_bytes() {
        return Err(PersistedPrepackedWeightError::WeightEarlyEof {
            expected: manifest.output_bytes(),
            actual: total,
        });
    }
    let mut trailing = [0; 1];
    loop {
        match reader.read(&mut trailing) {
            Ok(0) => break,
            Ok(_) => return Err(PersistedPrepackedWeightError::WeightTrailingData),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(PersistedPrepackedWeightError::WeightIo(error.kind())),
        }
    }
    Ok(())
}

fn reopen_persisted_qwen3_weight_manifest_authority(
    role: Qwen3ModelRole,
    commitment: ManifestCommitment,
    manifest_bytes: &[u8],
) -> Result<PrepackedWeightSet, PersistedPrepackedWeightError> {
    let descriptor = validate_persisted_commitment(role, commitment, manifest_bytes)?;
    reopen_persisted_qwen3_weight_manifest_after_commitment(
        role,
        descriptor,
        commitment.aggregate_id,
        manifest_bytes,
    )
}

fn reopen_persisted_qwen3_weight_manifest_after_commitment(
    role: Qwen3ModelRole,
    descriptor: WeightDescriptor,
    aggregate_id: [u8; 32],
    manifest_bytes: &[u8],
) -> Result<PrepackedWeightSet, PersistedPrepackedWeightError> {
    let parsed = parse_persisted_manifest(manifest_bytes)?;
    if parsed.role != role
        || parsed.source_weights_id != descriptor.weights_id
        || parsed.source_artifact_bytes != descriptor.artifact_bytes
        || parsed.tensor_data_bytes != descriptor.tensor_data_bytes
        || parsed.output_bytes != descriptor.tensor_data_bytes
    {
        return Err(PersistedPrepackedWeightError::InvalidManifest(
            "manifest header commitment",
        ));
    }
    let prepacked = finish_prepacked(role, descriptor, parsed.output_bytes, parsed.sections)
        .map_err(PersistedPrepackedWeightError::Manifest)?;
    if prepacked.manifest().canonical_bytes() != manifest_bytes
        || prepacked.manifest().aggregate_id() != aggregate_id
    {
        return Err(PersistedPrepackedWeightError::InvalidManifest(
            "noncanonical re-encoding",
        ));
    }
    validate_persisted_sections(prepacked.manifest())?;
    Ok(prepacked)
}

/// Reopens one canonical Qwen3 prepacked manifest under an admission commitment.
///
/// This validates the fixed role commitment, canonical encoding, complete tensor
/// schema, and exact section roster without reading the separately stored weight
/// image. The returned manifest authenticates its section digests and layout; it
/// does not claim that any section bytes have been read or matched. Callers must
/// independently hash every section they consume.
///
/// # Errors
///
/// Returns [`PersistedPrepackedWeightError`] for commitment, canonical-record,
/// tensor-schema, section-layout, or roster drift.
pub fn reopen_persisted_qwen3_weight_manifest(
    role: Qwen3ModelRole,
    commitment: ManifestCommitment,
    manifest_bytes: &[u8],
) -> Result<WeightSectionManifest, PersistedPrepackedWeightError> {
    let prepacked =
        reopen_persisted_qwen3_weight_manifest_authority(role, commitment, manifest_bytes)?;
    Ok(prepacked.into_parts().1)
}

/// Reopens one persisted prepacked Qwen3 weight image under an admission commitment.
///
/// The canonical manifest must exactly match the supplied admission-record
/// commitment and the pinned role descriptor. Every manifest section is
/// reparsed, canonically re-encoded, schema checked, and matched to the closed
/// Qwen3 tensor roster before the output image is streamed through exact
/// per-section SHA-256 checks and EOF. Success returns the same non-clone
/// [`PrepackedWeightSet`] typestate used by fresh safetensors prepacking.
///
/// `commitment` is decoded identity data, not signature authority. Reopening
/// succeeds only when its manifest length and SHA-256 equal the trust anchors
/// compiled from a fresh exact-source prepack. Signatures and independent
/// validation remain separate M1 admission and qualification boundaries.
///
/// # Errors
///
/// Returns [`PersistedPrepackedWeightError`] for commitment, canonical-record,
/// tensor-schema, section-digest, I/O, truncation, or trailing-data drift.
pub fn reopen_persisted_qwen3_weights<R: Read>(
    role: Qwen3ModelRole,
    commitment: ManifestCommitment,
    manifest_bytes: &[u8],
    mut weights: R,
) -> Result<PrepackedWeightSet, PersistedPrepackedWeightError> {
    let prepacked =
        reopen_persisted_qwen3_weight_manifest_authority(role, commitment, manifest_bytes)?;
    verify_persisted_weight_bytes(&mut weights, prepacked.manifest())?;
    Ok(prepacked)
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

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) mod test_fixtures {
    use super::{
        finish_prepacked, parse_header, PrepackedWeightSet, WeightSection, WeightTransform,
        DRAFT_FILE_PIN, SECTION_ALIGNMENT, TARGET_SHARD_PINS,
    };
    use crate::{
        sha256, WeightDescriptor, QWEN3_DRAFT_TENSOR_DATA_BYTES, QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
        QWEN3_DRAFT_WEIGHT_SHA256, QWEN3_TARGET_TENSOR_DATA_BYTES,
        QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES, QWEN3_TARGET_WEIGHT_SET_SHA256,
    };
    use ferric_spec::Qwen3ModelRole;

    const TARGET_HEADERS: [&[u8]; 5] = [
        include_bytes!("fixtures/safetensors/qwen3-8b-00001.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00002.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00003.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00004.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00005.header.json"),
    ];
    const DRAFT_HEADER: &[u8] = include_bytes!("fixtures/safetensors/qwen3-06b.header.json");

    const fn descriptor(role: Qwen3ModelRole) -> WeightDescriptor {
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
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        exact_weight_descriptor, finish_prepacked, flush_output, parse_header,
        parse_persisted_manifest, reopen_persisted_qwen3_weight_manifest_after_commitment,
        require_shard_name, stream_file, validate_persisted_commitment,
        validate_persisted_sections, validate_stream_plan, verify_persisted_weight_bytes, FilePin,
        ParsedShard, ParsedTensor, PersistedPrepackedWeightError, PrepackedWeightSet,
        WeightSection, WeightSectionManifest, WeightStreamError, WeightTransform, DRAFT_FILE_PIN,
        PREPACKED_WEIGHT_MANIFEST_VERSION, SECTION_ALIGNMENT, TARGET_SHARD_PINS,
    };
    use crate::tokenizer::tests::{authenticated_assets, test_tokenizer};
    use crate::{
        build_prepacked_deployment_bundle, sha256, BuildError, ManifestCommitment,
        SafetensorsError, WeightDescriptor, QWEN3_DRAFT_TENSOR_DATA_BYTES,
        QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES, QWEN3_DRAFT_WEIGHT_SHA256,
        QWEN3_TARGET_TENSOR_DATA_BYTES, QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
        QWEN3_TARGET_WEIGHT_SET_SHA256,
    };
    use ferric_spec::{
        Qwen3ModelRole, Qwen3TensorKind, Qwen3TensorMetadata, TensorDType, QWEN3_NO_LAYER,
    };
    use std::io::{self, Cursor, Read, Write};

    const WEIGHT_STREAM_SOURCE: &str = include_str!("weight_stream.rs");
    const HEADER: &[u8] = b"{}";
    const TARGET_HEADERS: [&[u8]; 5] = [
        include_bytes!("fixtures/safetensors/qwen3-8b-00001.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00002.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00003.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00004.header.json"),
        include_bytes!("fixtures/safetensors/qwen3-8b-00005.header.json"),
    ];
    const DRAFT_HEADER: &[u8] = include_bytes!("fixtures/safetensors/qwen3-06b.header.json");

    fn unique_source_offset(source: &str, needle: &str) -> usize {
        let mut matches = source.match_indices(needle);
        let Some((offset, _)) = matches.next() else {
            panic!("source-policy anchor is absent: {needle}");
        };
        assert!(
            matches.next().is_none(),
            "source-policy anchor is not unique: {needle}"
        );
        offset
    }

    /// Syntactic placement guard only; this is not independent proof evidence.
    #[test]
    fn source_policy_pins_role_byte_in_section_and_manifest_header() {
        let tests_marker = ["#[cfg(test)]", "pub(crate) mod tests {"].join("\n");
        let tests_start = unique_source_offset(WEIGHT_STREAM_SOURCE, &tests_marker);
        let production = &WEIGHT_STREAM_SOURCE[..tests_start];
        let role_code = unique_source_offset(production, "const fn role_code(");
        let section_start = unique_source_offset(production, "fn append_manifest_section(");
        let section_end = unique_source_offset(production, "fn append_manifest_sections(");
        let header_start = unique_source_offset(production, "fn encode_manifest_record_verified(");
        let header_end = unique_source_offset(production, "fn build_weight_manifest_verified(");
        let section = &production[section_start..section_end];
        let header = &production[header_start..header_end];

        let tensor_name = unique_source_offset(
            section,
            "append_length_prefixed_bytes(record, tensor_name.as_bytes())",
        );
        let section_role = unique_source_offset(section, "record.push(role_code(section.role));");
        let section_dtype =
            unique_source_offset(section, "record.push(dtype_code(section.dtype));");
        let version = unique_source_offset(
            header,
            "append_u32(&mut record, PREPACKED_WEIGHT_MANIFEST_VERSION);",
        );
        let header_role = unique_source_offset(header, "record.push(role_code(role));");
        let weights =
            unique_source_offset(header, "append_bytes(&mut record, &descriptor.weights_id);");

        assert!(role_code < section_start);
        assert!(role_code < header_start);
        assert!(tensor_name < section_role);
        assert!(section_role < section_dtype);
        assert!(version < header_role);
        assert!(header_role < weights);
    }

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

    fn commitment(manifest: &WeightSectionManifest) -> ManifestCommitment {
        ManifestCommitment {
            role: manifest.role(),
            version: manifest.version(),
            source_weights_id: manifest.source_weights_id(),
            aggregate_id: manifest.aggregate_id(),
            source_artifact_bytes: manifest.source_artifact_bytes(),
            tensor_data_bytes: manifest.tensor_data_bytes(),
            output_bytes: manifest.output_bytes(),
            section_count: u32::try_from(manifest.sections().len()).unwrap(),
            canonical_manifest_bytes: u32::try_from(manifest.canonical_bytes().len()).unwrap(),
        }
    }

    #[test]
    fn official_persisted_manifests_parse_reencode_and_validate_exactly() {
        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            let original = test_prepacked(role);
            let manifest = original.manifest();
            let descriptor = exact_weight_descriptor(role);
            let parsed = parse_persisted_manifest(manifest.canonical_bytes())
                .expect("canonical manifest parses");
            assert_eq!(parsed.role, role);
            let reopened = finish_prepacked(role, descriptor, parsed.output_bytes, parsed.sections)
                .expect("parsed manifest finalizes");
            assert_eq!(reopened.manifest(), manifest);
            validate_persisted_sections(reopened.manifest())
                .expect("official tensor roster validates");

            assert_eq!(
                validate_persisted_commitment(
                    role,
                    commitment(manifest),
                    manifest.canonical_bytes(),
                ),
                Err(PersistedPrepackedWeightError::InvalidCommitment(
                    "manifest identity"
                ))
            );
        }
    }

    #[test]
    fn manifest_only_reopen_accepts_canonical_bytes_and_rejects_drift() {
        let original = test_prepacked(Qwen3ModelRole::Target8B);
        let descriptor = exact_weight_descriptor(Qwen3ModelRole::Target8B);
        let aggregate_id = original.manifest().aggregate_id();
        let canonical = original.manifest().canonical_bytes().to_vec();

        let reopened = reopen_persisted_qwen3_weight_manifest_after_commitment(
            Qwen3ModelRole::Target8B,
            descriptor,
            aggregate_id,
            &canonical,
        )
        .expect("canonical manifest reopens without weight bytes");
        assert_eq!(reopened.manifest(), original.manifest());

        let mut changed_commitment = aggregate_id;
        changed_commitment[0] ^= 1;
        assert_eq!(
            reopen_persisted_qwen3_weight_manifest_after_commitment(
                Qwen3ModelRole::Target8B,
                descriptor,
                changed_commitment,
                &canonical,
            ),
            Err(PersistedPrepackedWeightError::InvalidManifest(
                "noncanonical re-encoding"
            ))
        );

        let mut changed_manifest = canonical;
        let last = changed_manifest
            .last_mut()
            .expect("canonical manifest is nonempty");
        *last ^= 1;
        assert!(reopen_persisted_qwen3_weight_manifest_after_commitment(
            Qwen3ModelRole::Target8B,
            descriptor,
            aggregate_id,
            &changed_manifest,
        )
        .is_err());
    }

    #[test]
    fn persisted_manifest_rejects_source_coverage_gaps() {
        let mut prepacked = test_prepacked(Qwen3ModelRole::Draft06B);
        prepacked.manifest.sections[0].source_offset += SECTION_ALIGNMENT;
        assert_eq!(
            validate_persisted_sections(prepacked.manifest()),
            Err(PersistedPrepackedWeightError::InvalidSectionLayout(
                "source coverage"
            ))
        );
    }

    #[test]
    fn persisted_manifest_rejects_target_shard_permutation() {
        let mut prepacked = test_prepacked(Qwen3ModelRole::Target8B);
        let second_shard = prepacked
            .manifest
            .sections
            .iter()
            .position(|section| section.source_artifact == TARGET_SHARD_PINS[1].name)
            .expect("second pinned shard is present");
        prepacked.manifest.sections.rotate_left(second_shard);
        assert_eq!(
            validate_persisted_sections(prepacked.manifest()),
            Err(PersistedPrepackedWeightError::InvalidSectionLayout(
                "source shard order"
            ))
        );
    }

    fn tiny_persisted_manifest(payload: &[u8]) -> WeightSectionManifest {
        let digest = sha256::digest(payload);
        WeightSectionManifest {
            version: PREPACKED_WEIGHT_MANIFEST_VERSION,
            role: Qwen3ModelRole::Draft06B,
            source_weights_id: QWEN3_DRAFT_WEIGHT_SHA256,
            source_artifact_bytes: payload.len() as u64,
            tensor_data_bytes: payload.len() as u64,
            output_bytes: payload.len() as u64,
            sections: vec![WeightSection {
                tensor_name: "tiny".to_owned(),
                role: Qwen3ModelRole::Draft06B,
                dtype: TensorDType::Bf16,
                rank: 1,
                dimension_0: u32::try_from(payload.len() / 2).unwrap(),
                dimension_1: 1,
                source_artifact: "tiny".to_owned(),
                source_offset: 0,
                source_length: payload.len() as u64,
                destination_offset: 0,
                destination_length: payload.len() as u64,
                alignment: SECTION_ALIGNMENT,
                transform: WeightTransform::Bf16RowMajorIdentityV1,
                sha256: digest,
            }],
            canonical_bytes: vec![1],
            aggregate_id: digest,
        }
    }

    #[test]
    fn persisted_section_stream_rejects_mutation_truncation_and_trailing_bytes() {
        let payload = b"abcdefgh";
        let manifest = tiny_persisted_manifest(payload);
        verify_persisted_weight_bytes(&mut Cursor::new(payload), &manifest)
            .expect("exact persisted bytes");

        let mut changed = payload.to_vec();
        changed[3] ^= 1;
        assert_eq!(
            verify_persisted_weight_bytes(&mut Cursor::new(changed), &manifest),
            Err(PersistedPrepackedWeightError::SectionDigestMismatch { section: 0 })
        );
        assert_eq!(
            verify_persisted_weight_bytes(
                &mut Cursor::new(&payload[..payload.len() - 1]),
                &manifest,
            ),
            Err(PersistedPrepackedWeightError::WeightEarlyEof {
                expected: payload.len() as u64,
                actual: (payload.len() - 1) as u64,
            })
        );
        let mut trailing = payload.to_vec();
        trailing.push(0);
        assert_eq!(
            verify_persisted_weight_bytes(&mut Cursor::new(trailing), &manifest),
            Err(PersistedPrepackedWeightError::WeightTrailingData)
        );
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
