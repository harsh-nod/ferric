//! Ferric-owned one-packet diagnostics for the M1 service queue path.
//!
//! These descriptions retain the exact K7 S1K4 token-assembly and first K1
//! target S1T128 token-embedding shapes. They construct inert, zero-pointer
//! COV6 kernarg images only. Generic fe2o3 code still owns allocation, private
//! pointer injection, queue publication, completion, readback, and teardown.

use core::fmt;

use fe2o3_aql::{AqlDispatchGeometryV1, AqlGeometryError};
use ferric_qwen_kernels::{gemm, logits};

use crate::M1PhysicalProgramV1;

/// SHA-256 of `ferric.m1.packet-diagnostic-content.v1`.
pub const M1_PACKET_DIAGNOSTIC_CONTENT_ROLE_IDENTITY_V1: [u8; 32] = [
    0xe5, 0x63, 0x4f, 0x22, 0xfa, 0x4e, 0xfa, 0x22, 0x08, 0x2b, 0xdc, 0xa8, 0xd2, 0xa9, 0x06, 0x36,
    0x42, 0x13, 0x53, 0x31, 0xdb, 0xd6, 0x0e, 0x67, 0x5d, 0x9b, 0x9a, 0xfe, 0x43, 0x79, 0xca, 0x83,
];

/// Fixed service queue ring size used by the M1 production first publication.
pub const M1_PACKET_DIAGNOSTIC_RING_BYTES_V1: u32 = 1 << 20;

/// One closed Ferric diagnostic packet shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PacketDiagnosticKindV1 {
    /// K7 target-token assembly for one sequence and four draft tokens.
    K7SpeculativeTokenAssemblyS1K4,
    /// The exact first K1 target prefill S1T128 token-embedding packet.
    K1TargetTokenEmbeddingPrefillS1T128,
}

/// Inspected access expected for one explicit global-buffer argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PacketDiagnosticBufferAccessV1 {
    /// The kernel reads the complete range.
    ReadOnly,
    /// The kernel writes the complete range.
    WriteOnly,
}

/// Addressless exact buffer contract for one diagnostic packet argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1PacketDiagnosticBufferV1 {
    explicit_argument_index: usize,
    byte_len: u64,
    alignment: u64,
    access: M1PacketDiagnosticBufferAccessV1,
}

impl M1PacketDiagnosticBufferV1 {
    /// Explicit pointer-argument ordinal in inspected metadata order.
    #[must_use]
    pub const fn explicit_argument_index(self) -> usize {
        self.explicit_argument_index
    }

    /// Exact byte extent of the bound range.
    #[must_use]
    pub const fn byte_len(self) -> u64 {
        self.byte_len
    }

    /// Minimum alignment required by the kernel ABI.
    #[must_use]
    pub const fn alignment(self) -> u64 {
        self.alignment
    }

    /// Inspected access expected for the range.
    #[must_use]
    pub const fn access(self) -> M1PacketDiagnosticBufferAccessV1 {
        self.access
    }
}

/// Complete inert description of one exact diagnostic packet.
#[derive(Debug, Eq, PartialEq)]
pub struct M1PacketDiagnosticSpecV1 {
    kind: M1PacketDiagnosticKindV1,
    program: M1PhysicalProgramV1,
    geometry: AqlDispatchGeometryV1,
    dynamic_group_segment_bytes: u32,
    explicit_kernarg_bytes: usize,
    kernarg_bytes: Box<[u8]>,
    buffers: [M1PacketDiagnosticBufferV1; 3],
}

impl M1PacketDiagnosticSpecV1 {
    /// Closed Ferric diagnostic shape.
    #[must_use]
    pub const fn kind(&self) -> M1PacketDiagnosticKindV1 {
        self.kind
    }

    /// Stable program selected from the exact M1 catalog.
    #[must_use]
    pub const fn program(&self) -> M1PhysicalProgramV1 {
        self.program
    }

    /// Exact AQL grid and workgroup geometry.
    #[must_use]
    pub const fn geometry(&self) -> AqlDispatchGeometryV1 {
        self.geometry
    }

    /// Exact dynamic group-segment request.
    #[must_use]
    pub const fn dynamic_group_segment_bytes(&self) -> u32 {
        self.dynamic_group_segment_bytes
    }

    /// Inspected explicit kernarg byte count before the COV6 suffix.
    #[must_use]
    pub const fn explicit_kernarg_bytes(&self) -> usize {
        self.explicit_kernarg_bytes
    }

    /// Complete explicit plus COV6 hidden zero-pointer image.
    #[must_use]
    pub fn kernarg_bytes(&self) -> &[u8] {
        &self.kernarg_bytes
    }

    /// Exact explicit global-buffer roster in pointer-argument order.
    #[must_use]
    pub const fn buffers(&self) -> &[M1PacketDiagnosticBufferV1; 3] {
        &self.buffers
    }

    /// Consumes the inert description into service-packet construction parts.
    #[must_use]
    pub fn into_packet_parts(
        self,
    ) -> (
        M1PhysicalProgramV1,
        AqlDispatchGeometryV1,
        u32,
        Box<[u8]>,
        [M1PacketDiagnosticBufferV1; 3],
    ) {
        (
            self.program,
            self.geometry,
            self.dynamic_group_segment_bytes,
            self.kernarg_bytes,
            self.buffers,
        )
    }
}

/// Pure diagnostic-spec construction failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PacketDiagnosticSpecErrorV1 {
    /// The canonical profile catalog could not be reconstructed.
    ProfileCatalog,
    /// The requested exact profile was absent from the canonical catalog.
    Profile,
    /// Exact extent arithmetic overflowed.
    Arithmetic,
    /// The exact profile geometry was rejected by the AQL contract.
    Geometry(AqlGeometryError),
}

impl fmt::Display for M1PacketDiagnosticSpecErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 packet diagnostic spec rejected: {self:?}")
    }
}

impl std::error::Error for M1PacketDiagnosticSpecErrorV1 {}

/// Builds the smallest existing Ferric copy-style packet through the M1 catalog.
///
/// The selected K7 profile consumes one U32 anchor and four U32 draft choices,
/// and emits five U32 target tokens. This is inert structural data and does not
/// claim prior hardware qualification.
///
/// # Errors
///
/// Returns an error if the canonical profile, extent arithmetic, or exact AQL
/// geometry cannot be reconstructed.
pub fn m1_k7_s1k4_packet_diagnostic_spec_v1(
) -> Result<M1PacketDiagnosticSpecV1, M1PacketDiagnosticSpecErrorV1> {
    let profile = logits::Qwen3SpeculativeTokenAssemblyProfileV1::for_bucket(
        logits::Qwen3LogitsBucketKindV1::SpeculativeS1K4C8192,
    )
    .ok_or(M1PacketDiagnosticSpecErrorV1::Profile)?;
    let [anchor_elements, draft_elements, target_elements] = profile.storage_extents();
    let byte_lengths = [
        anchor_elements
            .checked_mul(4)
            .ok_or(M1PacketDiagnosticSpecErrorV1::Arithmetic)?,
        draft_elements
            .checked_mul(4)
            .ok_or(M1PacketDiagnosticSpecErrorV1::Arithmetic)?,
        target_elements
            .checked_mul(4)
            .ok_or(M1PacketDiagnosticSpecErrorV1::Arithmetic)?,
    ];
    let geometry =
        AqlDispatchGeometryV1::new(profile.grid_workitems(), logits::QWEN3_LOGITS_WORKGROUP_V1)
            .map_err(M1PacketDiagnosticSpecErrorV1::Geometry)?;
    let mut kernarg = zeroed_kernarg(
        logits::QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_EXPLICIT_KERNARG_BYTES_V1,
        logits::QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_TOTAL_KERNARG_BYTES_V1,
    )?;
    for (offset, elements) in
        [8, 24, 40]
            .into_iter()
            .zip([anchor_elements, draft_elements, target_elements])
    {
        write_u64(&mut kernarg, offset, elements)?;
    }
    write_u32(&mut kernarg, 48, profile.sequences())?;
    write_u32(&mut kernarg, 52, profile.speculative_k())?;
    finish_spec(
        M1PacketDiagnosticKindV1::K7SpeculativeTokenAssemblyS1K4,
        M1PhysicalProgramV1::SpeculativeTokenAssembly,
        geometry,
        logits::QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_EXPLICIT_KERNARG_BYTES_V1,
        kernarg,
        byte_lengths,
        [4, 4, 4],
    )
}

/// Builds the exact first K1 packet in the target-only S1T128 canary batch.
///
/// # Errors
///
/// Returns an error if the canonical target embedding profile, extent
/// arithmetic, or exact AQL geometry cannot be reconstructed.
pub fn m1_k1_target_s1t128_packet_diagnostic_spec_v1(
) -> Result<M1PacketDiagnosticSpecV1, M1PacketDiagnosticSpecErrorV1> {
    let catalog = gemm::Qwen3TokenEmbeddingProfileCatalogV1::canonical()
        .map_err(|_| M1PacketDiagnosticSpecErrorV1::ProfileCatalog)?;
    let profile = catalog
        .profile(gemm::Qwen3GemmBucketV1::new(
            gemm::Qwen3GemmModelRoleV1::Target8B,
            gemm::Qwen3GemmBucketKindV1::PrefillS1T128,
        ))
        .ok_or(M1PacketDiagnosticSpecErrorV1::Profile)?;
    let [token_elements, weight_elements, output_elements] = profile.storage_elements();
    let byte_lengths = [
        token_elements
            .checked_mul(4)
            .ok_or(M1PacketDiagnosticSpecErrorV1::Arithmetic)?,
        weight_elements
            .checked_mul(2)
            .ok_or(M1PacketDiagnosticSpecErrorV1::Arithmetic)?,
        output_elements
            .checked_mul(2)
            .ok_or(M1PacketDiagnosticSpecErrorV1::Arithmetic)?,
    ];
    let geometry =
        AqlDispatchGeometryV1::new(profile.aql_grid_workitems(), gemm::QWEN3_GEMM_WORKGROUP_V1)
            .map_err(M1PacketDiagnosticSpecErrorV1::Geometry)?;
    let mut kernarg = zeroed_kernarg(
        gemm::QWEN3_TOKEN_EMBEDDING_EXPLICIT_KERNARG_BYTES_V1,
        gemm::QWEN3_TOKEN_EMBEDDING_TOTAL_KERNARG_BYTES_V1,
    )?;
    for (offset, elements) in
        [8, 24, 40]
            .into_iter()
            .zip([token_elements, weight_elements, output_elements])
    {
        write_u64(&mut kernarg, offset, elements)?;
    }
    write_u32(&mut kernarg, 48, profile.rows())?;
    write_u32(&mut kernarg, 52, profile.hidden_size())?;
    write_u32(&mut kernarg, 56, gemm::QWEN3_VOCABULARY_SIZE_V1)?;
    finish_spec(
        M1PacketDiagnosticKindV1::K1TargetTokenEmbeddingPrefillS1T128,
        M1PhysicalProgramV1::TokenEmbedding,
        geometry,
        gemm::QWEN3_TOKEN_EMBEDDING_EXPLICIT_KERNARG_BYTES_V1,
        kernarg,
        byte_lengths,
        [4, 2, 2],
    )
}

fn zeroed_kernarg(
    explicit_bytes: u64,
    total_bytes: u64,
) -> Result<Vec<u8>, M1PacketDiagnosticSpecErrorV1> {
    if total_bytes.checked_sub(explicit_bytes) != Some(256) {
        return Err(M1PacketDiagnosticSpecErrorV1::Arithmetic);
    }
    let total =
        usize::try_from(total_bytes).map_err(|_| M1PacketDiagnosticSpecErrorV1::Arithmetic)?;
    Ok(vec![0; total])
}

fn write_u32(
    bytes: &mut [u8],
    offset: usize,
    value: u32,
) -> Result<(), M1PacketDiagnosticSpecErrorV1> {
    write_bytes(bytes, offset, &value.to_le_bytes())
}

fn write_u64(
    bytes: &mut [u8],
    offset: usize,
    value: u64,
) -> Result<(), M1PacketDiagnosticSpecErrorV1> {
    write_bytes(bytes, offset, &value.to_le_bytes())
}

fn write_bytes(
    bytes: &mut [u8],
    offset: usize,
    value: &[u8],
) -> Result<(), M1PacketDiagnosticSpecErrorV1> {
    let end = offset
        .checked_add(value.len())
        .ok_or(M1PacketDiagnosticSpecErrorV1::Arithmetic)?;
    let target = bytes
        .get_mut(offset..end)
        .ok_or(M1PacketDiagnosticSpecErrorV1::Arithmetic)?;
    target.copy_from_slice(value);
    Ok(())
}

fn finish_spec(
    kind: M1PacketDiagnosticKindV1,
    program: M1PhysicalProgramV1,
    geometry: AqlDispatchGeometryV1,
    explicit_kernarg_bytes: u64,
    kernarg_bytes: Vec<u8>,
    byte_lengths: [u64; 3],
    alignments: [u64; 3],
) -> Result<M1PacketDiagnosticSpecV1, M1PacketDiagnosticSpecErrorV1> {
    let explicit_kernarg_bytes = usize::try_from(explicit_kernarg_bytes)
        .map_err(|_| M1PacketDiagnosticSpecErrorV1::Arithmetic)?;
    if [0, 16, 32].into_iter().any(|offset| {
        kernarg_bytes
            .get(offset..offset + 8)
            .is_none_or(|pointer| pointer != [0; 8])
    }) || kernarg_bytes
        .get(explicit_kernarg_bytes..)
        .is_none_or(|suffix| suffix.iter().any(|byte| *byte != 0))
    {
        return Err(M1PacketDiagnosticSpecErrorV1::Arithmetic);
    }
    Ok(M1PacketDiagnosticSpecV1 {
        kind,
        program,
        geometry,
        dynamic_group_segment_bytes: 0,
        explicit_kernarg_bytes,
        kernarg_bytes: kernarg_bytes.into_boxed_slice(),
        buffers: core::array::from_fn(|index| M1PacketDiagnosticBufferV1 {
            explicit_argument_index: index * 2,
            byte_len: byte_lengths[index],
            alignment: alignments[index],
            access: if index == 2 {
                M1PacketDiagnosticBufferAccessV1::WriteOnly
            } else {
                M1PacketDiagnosticBufferAccessV1::ReadOnly
            },
        }),
    })
}

#[cfg(test)]
mod tests {
    use ferric_qwen_kernels::logits::{
        assemble_qwen3_speculative_target_tokens_v1, Qwen3LogitsBucketKindV1,
        Qwen3SpeculativeTokenAssemblyProfileV1,
    };

    use super::*;

    fn read_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    fn read_u64(bytes: &[u8], offset: usize) -> u64 {
        u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
    }

    fn assert_zero_pointer_and_hidden_bytes(spec: &M1PacketDiagnosticSpecV1) {
        for offset in [0, 16, 32] {
            assert_eq!(&spec.kernarg_bytes()[offset..offset + 8], &[0; 8]);
        }
        assert!(spec.kernarg_bytes()[spec.explicit_kernarg_bytes()..]
            .iter()
            .all(|byte| *byte == 0));
    }

    #[test]
    fn k7_s1k4_spec_is_one_tiny_copy_style_packet() {
        let spec = m1_k7_s1k4_packet_diagnostic_spec_v1().unwrap();
        assert_eq!(
            spec.kind(),
            M1PacketDiagnosticKindV1::K7SpeculativeTokenAssemblyS1K4
        );
        assert_eq!(
            spec.program(),
            M1PhysicalProgramV1::SpeculativeTokenAssembly
        );
        assert_eq!(spec.program().program_index(), 11);
        assert_eq!(spec.geometry().grid(), [64, 1, 1]);
        assert_eq!(spec.geometry().workgroup(), [64, 1, 1]);
        assert_eq!(spec.explicit_kernarg_bytes(), 56);
        assert_eq!(spec.kernarg_bytes().len(), 312);
        assert_eq!(
            spec.buffers().map(M1PacketDiagnosticBufferV1::byte_len),
            [4, 16, 20]
        );
        assert_eq!(read_u64(spec.kernarg_bytes(), 8), 1);
        assert_eq!(read_u64(spec.kernarg_bytes(), 24), 4);
        assert_eq!(read_u64(spec.kernarg_bytes(), 40), 5);
        assert_eq!(read_u32(spec.kernarg_bytes(), 48), 1);
        assert_eq!(read_u32(spec.kernarg_bytes(), 52), 4);
        assert_zero_pointer_and_hidden_bytes(&spec);

        let profile = Qwen3SpeculativeTokenAssemblyProfileV1::for_bucket(
            Qwen3LogitsBucketKindV1::SpeculativeS1K4C8192,
        )
        .unwrap();
        assert_eq!(
            assemble_qwen3_speculative_target_tokens_v1(profile, &[10], &[11, 12, 13, 14]).unwrap(),
            [10, 11, 12, 13, 14]
        );
    }

    #[test]
    fn k1_spec_is_the_exact_first_target_s1t128_embedding_packet() {
        let spec = m1_k1_target_s1t128_packet_diagnostic_spec_v1().unwrap();
        assert_eq!(
            spec.kind(),
            M1PacketDiagnosticKindV1::K1TargetTokenEmbeddingPrefillS1T128
        );
        assert_eq!(spec.program(), M1PhysicalProgramV1::TokenEmbedding);
        assert_eq!(spec.program().program_index(), 2);
        assert_eq!(spec.geometry().grid(), [524_288, 1, 1]);
        assert_eq!(spec.geometry().workgroup(), [64, 1, 1]);
        assert_eq!(spec.explicit_kernarg_bytes(), 64);
        assert_eq!(spec.kernarg_bytes().len(), 320);
        assert_eq!(
            spec.buffers().map(M1PacketDiagnosticBufferV1::byte_len),
            [512, 1_244_659_712, 1_048_576]
        );
        assert_eq!(read_u64(spec.kernarg_bytes(), 8), 128);
        assert_eq!(read_u64(spec.kernarg_bytes(), 24), 622_329_856);
        assert_eq!(read_u64(spec.kernarg_bytes(), 40), 524_288);
        assert_eq!(read_u32(spec.kernarg_bytes(), 48), 128);
        assert_eq!(read_u32(spec.kernarg_bytes(), 52), 4_096);
        assert_eq!(read_u32(spec.kernarg_bytes(), 56), 151_936);
        assert_zero_pointer_and_hidden_bytes(&spec);
    }
}
