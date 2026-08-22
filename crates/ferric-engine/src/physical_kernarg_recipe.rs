//! Zero-pointer COV6 kernarg images for exact M1 physical dispatch recipes.
//!
//! The images contain every inspected explicit length and scalar value, while
//! every explicit device pointer and the complete 256-byte hidden suffix stay
//! zero. Generic fe2 runtime code remains responsible for private pointer and
//! hidden-argument injection. These inert bytes do not bind buffers, construct
//! packets, grant queue or execution authority, report hardware behavior, or
//! prove operator refinement.

use ferric_kernels::KernelProfileDescriptor;
use ferric_qwen_kernels::{gemm, logits, paged_decode, prefill, rmsnorm, rope_kv, swiglu};
use ferric_spec::{
    expected_step, Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3Operator, Qwen3PlanBucket,
    Qwen3PlanSelection,
};

use crate::{
    AddresslessM1PhysicalDispatchRecipeV1, M1OperationDispatchKind, M1PhysicalDispatchKindV1,
    M1PhysicalDispatchProfileV1, M1PhysicalDispatchRecipeRowV1, M1PhysicalProfileFamilyV1,
    M1PhysicalProgramV1, M1StepDispatchStage, M1_PHYSICAL_DISPATCH_RECIPE_VERSION_V1,
};

/// Zero-pointer physical-kernarg recipe format.
pub const M1_PHYSICAL_KERNARG_RECIPE_VERSION_V1: u32 = 2;
/// Exact byte count reserved for COV6 hidden arguments in every image.
pub const M1_COV6_HIDDEN_KERNARG_BYTES_V1: usize = 256;

/// One complete zero-pointer COV6 kernarg image.
///
/// This inert owner intentionally does not implement `Clone`.
#[derive(Debug, Eq, PartialEq)]
pub struct M1PhysicalKernargImageV1 {
    dispatch_index: u32,
    selection: Qwen3PlanSelection,
    profile_id: Identity,
    program: M1PhysicalProgramV1,
    explicit_bytes: usize,
    bytes: Box<[u8]>,
}

impl M1PhysicalKernargImageV1 {
    /// Global dispatch row bound to these bytes.
    #[must_use]
    pub const fn dispatch_index(&self) -> u32 {
        self.dispatch_index
    }

    /// Exact finite selection bound to these bytes.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Exact canonical profile identity bound to these bytes.
    #[must_use]
    pub const fn profile_id(&self) -> Identity {
        self.profile_id
    }

    /// Exact physical program bound to these bytes.
    #[must_use]
    pub const fn program(&self) -> M1PhysicalProgramV1 {
        self.program
    }

    /// Inspected explicit-argument byte count.
    #[must_use]
    pub const fn explicit_bytes(&self) -> usize {
        self.explicit_bytes
    }

    /// Complete explicit plus COV6 hidden image bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Entire zero suffix reserved for generic fe2 private injection.
    #[must_use]
    pub fn cov6_hidden_suffix(&self) -> &[u8] {
        &self.bytes[self.explicit_bytes..]
    }

    /// These bytes bind no device buffers.
    #[must_use]
    pub const fn binds_device_buffers(&self) -> bool {
        false
    }

    /// These bytes construct no packet and grant no queue authority.
    #[must_use]
    pub const fn grants_packet_or_queue_authority(&self) -> bool {
        false
    }

    /// These bytes report no hardware result and prove no refinement.
    #[must_use]
    pub const fn proves_hardware_or_operator_refinement(&self) -> bool {
        false
    }

    /// Consumes the inert image and returns its complete byte allocation.
    #[must_use]
    pub fn into_bytes(self) -> Box<[u8]> {
        self.bytes
    }
}

/// Move-only custody of a source physical recipe and all derived images.
///
/// ```compile_fail
/// use ferric_engine::AddresslessM1PhysicalKernargRecipeV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<AddresslessM1PhysicalKernargRecipeV1>();
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct AddresslessM1PhysicalKernargRecipeV1 {
    version: u32,
    source: AddresslessM1PhysicalDispatchRecipeV1,
    images: Box<[M1PhysicalKernargImageV1]>,
}

impl AddresslessM1PhysicalKernargRecipeV1 {
    /// Kernarg-recipe format version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Exact retained physical dispatch recipe.
    #[must_use]
    pub const fn source_recipe(&self) -> &AddresslessM1PhysicalDispatchRecipeV1 {
        &self.source
    }

    /// Exact zero-pointer images in global dispatch order.
    #[must_use]
    pub fn images(&self) -> &[M1PhysicalKernargImageV1] {
        &self.images
    }

    /// These inert images bind no device buffers.
    #[must_use]
    pub const fn binds_device_buffers(&self) -> bool {
        false
    }

    /// These inert images construct no packets and grant no queue authority.
    #[must_use]
    pub const fn grants_packet_or_queue_authority(&self) -> bool {
        false
    }

    /// These inert images report no hardware result and prove no refinement.
    #[must_use]
    pub const fn proves_hardware_or_operator_refinement(&self) -> bool {
        false
    }

    /// Consumes the owner and returns the retained source plus inert images.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        AddresslessM1PhysicalDispatchRecipeV1,
        Box<[M1PhysicalKernargImageV1]>,
    ) {
        (self.source, self.images)
    }
}

/// Fail-closed zero-pointer kernarg derivation error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalKernargRecipeErrorV1 {
    /// The source physical-recipe format version drifted.
    SourceVersion { expected: u32, actual: u32 },
    /// The retained row count and declared count differed.
    DispatchCount { expected: u32, actual: usize },
    /// A row was not in exact global order.
    DispatchOrder { expected: u32, actual: u32 },
    /// A retained generated profile differed from the canonical step.
    ProfileDescriptor { dispatch_index: u32 },
    /// A canonical finite profile catalog could not be reconstructed.
    CanonicalProfileCatalog(M1PhysicalProfileFamilyV1),
    /// The retained identity was absent from the exact required catalog entry.
    ProfileIdentity {
        dispatch_index: u32,
        operator: Qwen3Operator,
    },
    /// The infrastructure profile identity or target selection drifted.
    InfrastructureProfileIdentity { dispatch_index: u32 },
    /// The physical subdispatch kind did not match the exact program.
    DispatchKind {
        dispatch_index: u32,
        kind: M1PhysicalDispatchKindV1,
    },
    /// The row selected a different physical program.
    Program {
        dispatch_index: u32,
        expected: M1PhysicalProgramV1,
        actual: M1PhysicalProgramV1,
    },
    /// The retained launch geometry differed from its canonical profile.
    Geometry { dispatch_index: u32 },
    /// The retained total kernarg byte count drifted.
    KernargLength {
        dispatch_index: u32,
        expected: u64,
        actual: u64,
    },
    /// An inspected explicit/total layout did not contain exactly 256 hidden bytes.
    Cov6Layout {
        dispatch_index: u32,
        explicit_bytes: u64,
        total_bytes: u64,
    },
    /// A byte length could not be represented by this host.
    HostLength { dispatch_index: u32, bytes: u64 },
    /// A checked little-endian write exceeded the inspected image.
    WriteBounds {
        dispatch_index: u32,
        offset: usize,
        width: usize,
        image_bytes: usize,
    },
    /// A write touched a device-pointer field that must remain zero.
    PointerNotZero { dispatch_index: u32, offset: usize },
    /// A write touched the COV6 suffix reserved for generic private injection.
    HiddenSuffixNotZero { dispatch_index: u32 },
    /// Checked profile arithmetic overflowed.
    ArithmeticOverflow { dispatch_index: u32 },
}

/// Retry-safe rejection retaining the unchanged source recipe.
///
/// This owner intentionally does not implement `Clone`.
#[derive(Debug, Eq, PartialEq)]
pub struct M1PhysicalKernargRecipeFailureV1 {
    error: M1PhysicalKernargRecipeErrorV1,
    source: AddresslessM1PhysicalDispatchRecipeV1,
}

impl M1PhysicalKernargRecipeFailureV1 {
    /// Exact fail-closed diagnostic.
    #[must_use]
    pub const fn error(&self) -> M1PhysicalKernargRecipeErrorV1 {
        self.error
    }

    /// Recovers the diagnostic and unchanged source recipe.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        M1PhysicalKernargRecipeErrorV1,
        AddresslessM1PhysicalDispatchRecipeV1,
    ) {
        (self.error, self.source)
    }
}

#[derive(Clone, Copy)]
struct KernargRowInput {
    dispatch_index: u32,
    stage: M1StepDispatchStage,
    selection: Qwen3PlanSelection,
    logical_ordinal: Option<u32>,
    profile: Option<KernelProfileDescriptor>,
    assembly_profile: Option<logits::Qwen3SpeculativeTokenAssemblyProfileV1>,
    kind: M1PhysicalDispatchKindV1,
    profile_id: Identity,
    program: M1PhysicalProgramV1,
    grid: [u32; 3],
    workgroup: [u16; 3],
    kernarg_bytes: u64,
    dynamic_group_segment_bytes: u32,
}

impl From<M1PhysicalDispatchRecipeRowV1> for KernargRowInput {
    fn from(row: M1PhysicalDispatchRecipeRowV1) -> Self {
        Self {
            dispatch_index: row.dispatch_index(),
            stage: row.stage(),
            selection: row.selection(),
            logical_ordinal: row.logical_ordinal(),
            profile: match row.profile() {
                M1PhysicalDispatchProfileV1::Model { descriptor, .. } => Some(descriptor),
                M1PhysicalDispatchProfileV1::SpeculativeTokenAssembly(_) => None,
            },
            assembly_profile: match row.profile() {
                M1PhysicalDispatchProfileV1::Model { .. } => None,
                M1PhysicalDispatchProfileV1::SpeculativeTokenAssembly(profile) => Some(profile),
            },
            kind: row.kind(),
            profile_id: row.profile_id(),
            program: row.program(),
            grid: row.geometry().grid(),
            workgroup: row.geometry().workgroup(),
            kernarg_bytes: row.kernarg_bytes(),
            dynamic_group_segment_bytes: row.dynamic_group_segment_bytes(),
        }
    }
}

struct CanonicalKernargProfiles {
    gemm: gemm::Qwen3GemmProfileCatalogV1,
    embedding: gemm::Qwen3TokenEmbeddingProfileCatalogV1,
    rmsnorm: rmsnorm::Qwen3RmsNormProfileCatalogV1,
    rope_kv: rope_kv::Qwen3RopeKvProfileCatalogV1,
    prefill: prefill::Qwen3PrefillProfileCatalogV1,
    paged_decode: paged_decode::Qwen3PagedDecodeProfileCatalogV1,
    swiglu: swiglu::Qwen3SwiGluProfileCatalogV1,
    logits: logits::Qwen3LogitsProfileCatalogV1,
}

impl CanonicalKernargProfiles {
    fn new() -> Result<Self, M1PhysicalKernargRecipeErrorV1> {
        Ok(Self {
            gemm: gemm::Qwen3GemmProfileCatalogV1::canonical().map_err(|_| {
                M1PhysicalKernargRecipeErrorV1::CanonicalProfileCatalog(
                    M1PhysicalProfileFamilyV1::Gemm,
                )
            })?,
            embedding: gemm::Qwen3TokenEmbeddingProfileCatalogV1::canonical().map_err(|_| {
                M1PhysicalKernargRecipeErrorV1::CanonicalProfileCatalog(
                    M1PhysicalProfileFamilyV1::TokenEmbedding,
                )
            })?,
            rmsnorm: rmsnorm::Qwen3RmsNormProfileCatalogV1::canonical().map_err(|_| {
                M1PhysicalKernargRecipeErrorV1::CanonicalProfileCatalog(
                    M1PhysicalProfileFamilyV1::RmsNorm,
                )
            })?,
            rope_kv: rope_kv::Qwen3RopeKvProfileCatalogV1::canonical().map_err(|_| {
                M1PhysicalKernargRecipeErrorV1::CanonicalProfileCatalog(
                    M1PhysicalProfileFamilyV1::RopeKv,
                )
            })?,
            prefill: prefill::Qwen3PrefillProfileCatalogV1::canonical().map_err(|_| {
                M1PhysicalKernargRecipeErrorV1::CanonicalProfileCatalog(
                    M1PhysicalProfileFamilyV1::Prefill,
                )
            })?,
            paged_decode: paged_decode::Qwen3PagedDecodeProfileCatalogV1::canonical().map_err(
                |_| {
                    M1PhysicalKernargRecipeErrorV1::CanonicalProfileCatalog(
                        M1PhysicalProfileFamilyV1::PagedDecode,
                    )
                },
            )?,
            swiglu: swiglu::Qwen3SwiGluProfileCatalogV1::canonical().map_err(|_| {
                M1PhysicalKernargRecipeErrorV1::CanonicalProfileCatalog(
                    M1PhysicalProfileFamilyV1::SwiGlu,
                )
            })?,
            logits: logits::Qwen3LogitsProfileCatalogV1::canonical().map_err(|_| {
                M1PhysicalKernargRecipeErrorV1::CanonicalProfileCatalog(
                    M1PhysicalProfileFamilyV1::Logits,
                )
            })?,
        })
    }
}

#[derive(Debug)]
struct ImageWriter {
    dispatch_index: u32,
    explicit_bytes: usize,
    pointer_offsets: &'static [usize],
    bytes: Vec<u8>,
}

impl ImageWriter {
    fn new(
        dispatch_index: u32,
        explicit_bytes: u64,
        total_bytes: u64,
        pointer_offsets: &'static [usize],
    ) -> Result<Self, M1PhysicalKernargRecipeErrorV1> {
        if total_bytes.checked_sub(explicit_bytes) != Some(M1_COV6_HIDDEN_KERNARG_BYTES_V1 as u64) {
            return Err(M1PhysicalKernargRecipeErrorV1::Cov6Layout {
                dispatch_index,
                explicit_bytes,
                total_bytes,
            });
        }
        let explicit_bytes = usize::try_from(explicit_bytes).map_err(|_| {
            M1PhysicalKernargRecipeErrorV1::HostLength {
                dispatch_index,
                bytes: explicit_bytes,
            }
        })?;
        let total = usize::try_from(total_bytes).map_err(|_| {
            M1PhysicalKernargRecipeErrorV1::HostLength {
                dispatch_index,
                bytes: total_bytes,
            }
        })?;
        Ok(Self {
            dispatch_index,
            explicit_bytes,
            pointer_offsets,
            bytes: vec![0; total],
        })
    }

    fn write_u32(
        &mut self,
        offset: usize,
        value: u32,
    ) -> Result<(), M1PhysicalKernargRecipeErrorV1> {
        self.write(offset, &value.to_le_bytes())
    }

    fn write_u64(
        &mut self,
        offset: usize,
        value: u64,
    ) -> Result<(), M1PhysicalKernargRecipeErrorV1> {
        self.write(offset, &value.to_le_bytes())
    }

    fn write(&mut self, offset: usize, value: &[u8]) -> Result<(), M1PhysicalKernargRecipeErrorV1> {
        let Some(end) = offset.checked_add(value.len()) else {
            return Err(M1PhysicalKernargRecipeErrorV1::WriteBounds {
                dispatch_index: self.dispatch_index,
                offset,
                width: value.len(),
                image_bytes: self.bytes.len(),
            });
        };
        let image_bytes = self.bytes.len();
        let Some(target) = self.bytes.get_mut(offset..end) else {
            return Err(M1PhysicalKernargRecipeErrorV1::WriteBounds {
                dispatch_index: self.dispatch_index,
                offset,
                width: value.len(),
                image_bytes,
            });
        };
        target.copy_from_slice(value);
        Ok(())
    }

    fn finish(
        self,
        row: &KernargRowInput,
    ) -> Result<M1PhysicalKernargImageV1, M1PhysicalKernargRecipeErrorV1> {
        for &offset in self.pointer_offsets {
            let Some(pointer) = self.bytes.get(offset..offset + 8) else {
                return Err(M1PhysicalKernargRecipeErrorV1::WriteBounds {
                    dispatch_index: self.dispatch_index,
                    offset,
                    width: 8,
                    image_bytes: self.bytes.len(),
                });
            };
            if pointer != [0; 8] {
                return Err(M1PhysicalKernargRecipeErrorV1::PointerNotZero {
                    dispatch_index: self.dispatch_index,
                    offset,
                });
            }
        }
        if self.bytes[self.explicit_bytes..]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(M1PhysicalKernargRecipeErrorV1::HiddenSuffixNotZero {
                dispatch_index: self.dispatch_index,
            });
        }
        Ok(M1PhysicalKernargImageV1 {
            dispatch_index: row.dispatch_index,
            selection: row.selection,
            profile_id: row.profile_id,
            program: row.program,
            explicit_bytes: self.explicit_bytes,
            bytes: self.bytes.into_boxed_slice(),
        })
    }
}

/// Consumes an exact physical recipe and derives its complete zero-pointer images.
///
/// On rejection, [`M1PhysicalKernargRecipeFailureV1::into_parts`] recovers the
/// unchanged source recipe.
///
/// # Errors
///
/// Returns a retry-safe failure for source version/count/order drift, generated
/// descriptor or canonical profile drift, physical program/geometry/ABI-size
/// drift, arithmetic or host-size overflow, an out-of-bounds write, or any
/// nonzero explicit pointer or COV6 hidden byte.
pub fn derive_m1_physical_kernarg_recipe_v1(
    source: AddresslessM1PhysicalDispatchRecipeV1,
) -> Result<AddresslessM1PhysicalKernargRecipeV1, M1PhysicalKernargRecipeFailureV1> {
    let result = derive_images(&source);
    match result {
        Ok(images) => Ok(AddresslessM1PhysicalKernargRecipeV1 {
            version: M1_PHYSICAL_KERNARG_RECIPE_VERSION_V1,
            source,
            images,
        }),
        Err(error) => Err(M1PhysicalKernargRecipeFailureV1 { error, source }),
    }
}

fn derive_images(
    source: &AddresslessM1PhysicalDispatchRecipeV1,
) -> Result<Box<[M1PhysicalKernargImageV1]>, M1PhysicalKernargRecipeErrorV1> {
    if source.version() != M1_PHYSICAL_DISPATCH_RECIPE_VERSION_V1 {
        return Err(M1PhysicalKernargRecipeErrorV1::SourceVersion {
            expected: M1_PHYSICAL_DISPATCH_RECIPE_VERSION_V1,
            actual: source.version(),
        });
    }
    if usize::try_from(source.dispatch_count()).ok() != Some(source.rows().len()) {
        return Err(M1PhysicalKernargRecipeErrorV1::DispatchCount {
            expected: source.dispatch_count(),
            actual: source.rows().len(),
        });
    }
    let catalogs = CanonicalKernargProfiles::new()?;
    let rows = source
        .rows()
        .iter()
        .copied()
        .map(KernargRowInput::from)
        .collect::<Vec<_>>();
    derive_input_images(&catalogs, &rows)
}

fn derive_input_images(
    catalogs: &CanonicalKernargProfiles,
    rows: &[KernargRowInput],
) -> Result<Box<[M1PhysicalKernargImageV1]>, M1PhysicalKernargRecipeErrorV1> {
    let mut images = Vec::with_capacity(rows.len());
    for (expected, row) in rows.iter().enumerate() {
        let expected =
            u32::try_from(expected).map_err(|_| M1PhysicalKernargRecipeErrorV1::DispatchCount {
                expected: u32::MAX,
                actual: rows.len(),
            })?;
        if row.dispatch_index != expected {
            return Err(M1PhysicalKernargRecipeErrorV1::DispatchOrder {
                expected,
                actual: row.dispatch_index,
            });
        }
        validate_descriptor(row)?;
        images.push(derive_image(catalogs, row)?);
    }
    Ok(images.into_boxed_slice())
}

fn validate_descriptor(row: &KernargRowInput) -> Result<(), M1PhysicalKernargRecipeErrorV1> {
    if let Some(profile) = row.assembly_profile {
        let expected_profile = assembly_profile_for_selection(row.selection);
        if row.kind != M1PhysicalDispatchKindV1::SpeculativeTokenAssembly
            || row.logical_ordinal.is_some()
            || row.profile.is_some()
            || expected_profile != Some(profile)
            || !matches!(
                row.stage,
                M1StepDispatchStage::TargetVerification { draft_iterations }
                    if u32::from(draft_iterations) == profile.speculative_k()
            )
            || row.dynamic_group_segment_bytes != 0
        {
            return Err(M1PhysicalKernargRecipeErrorV1::ProfileDescriptor {
                dispatch_index: row.dispatch_index,
            });
        }
        return Ok(());
    }
    let (Some(logical_ordinal), Some(profile), M1PhysicalDispatchKindV1::Model(_)) =
        (row.logical_ordinal, row.profile, row.kind)
    else {
        return Err(M1PhysicalKernargRecipeErrorV1::ProfileDescriptor {
            dispatch_index: row.dispatch_index,
        });
    };
    let Some(expected) = expected_step(
        row.selection.role,
        row.selection.mode,
        row.selection.bucket,
        logical_ordinal,
    ) else {
        return Err(M1PhysicalKernargRecipeErrorV1::ProfileDescriptor {
            dispatch_index: row.dispatch_index,
        });
    };
    let Some(dimensions) = row
        .selection
        .bucket
        .dimensions(row.selection.role, row.selection.mode)
    else {
        return Err(M1PhysicalKernargRecipeErrorV1::ProfileDescriptor {
            dispatch_index: row.dispatch_index,
        });
    };
    if profile.selection != row.selection
        || profile.step != expected
        || profile.sequences != dimensions.sequences
        || profile.active_tokens != dimensions.active_tokens
        || profile.context_tokens != dimensions.context_tokens
        || row.dynamic_group_segment_bytes != 0
    {
        return Err(M1PhysicalKernargRecipeErrorV1::ProfileDescriptor {
            dispatch_index: row.dispatch_index,
        });
    }
    Ok(())
}

fn derive_image(
    catalogs: &CanonicalKernargProfiles,
    row: &KernargRowInput,
) -> Result<M1PhysicalKernargImageV1, M1PhysicalKernargRecipeErrorV1> {
    if row.kind == M1PhysicalDispatchKindV1::SpeculativeTokenAssembly {
        return encode_speculative_token_assembly(row);
    }
    let profile = model_descriptor(row)?;
    match profile.step.operator {
        Qwen3Operator::TokenEmbedding => encode_embedding(catalogs, row),
        Qwen3Operator::QueryProjection
        | Qwen3Operator::KeyProjection
        | Qwen3Operator::ValueProjection
        | Qwen3Operator::AttentionOutputResidual
        | Qwen3Operator::GateProjection
        | Qwen3Operator::UpProjection
        | Qwen3Operator::DownResidual
        | Qwen3Operator::LogitsProjection => encode_gemm(catalogs, row),
        Qwen3Operator::InputRmsNorm
        | Qwen3Operator::QueryRmsNorm
        | Qwen3Operator::KeyRmsNorm
        | Qwen3Operator::PostAttentionRmsNorm
        | Qwen3Operator::FinalRmsNorm => encode_rmsnorm(catalogs, row),
        Qwen3Operator::Rope | Qwen3Operator::KvWrite => encode_rope_kv(catalogs, row),
        Qwen3Operator::Attention => match row.selection.mode {
            Qwen3ExecutionMode::Prefill => encode_prefill(catalogs, row),
            Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => {
                encode_paged_decode(catalogs, row)
            }
        },
        Qwen3Operator::SwiGlu => encode_swiglu(catalogs, row),
        Qwen3Operator::ArgmaxCompactCompletion => encode_logits(catalogs, row),
    }
}

fn unresolved(row: &KernargRowInput) -> M1PhysicalKernargRecipeErrorV1 {
    M1PhysicalKernargRecipeErrorV1::ProfileIdentity {
        dispatch_index: row.dispatch_index,
        operator: row
            .profile
            .map_or(Qwen3Operator::ArgmaxCompactCompletion, |profile| {
                profile.step.operator
            }),
    }
}

fn model_descriptor(
    row: &KernargRowInput,
) -> Result<KernelProfileDescriptor, M1PhysicalKernargRecipeErrorV1> {
    row.profile
        .ok_or(M1PhysicalKernargRecipeErrorV1::ProfileDescriptor {
            dispatch_index: row.dispatch_index,
        })
}

fn model_kind(
    row: &KernargRowInput,
) -> Result<M1OperationDispatchKind, M1PhysicalKernargRecipeErrorV1> {
    match row.kind {
        M1PhysicalDispatchKindV1::Model(kind) => Ok(kind),
        M1PhysicalDispatchKindV1::SpeculativeTokenAssembly => {
            Err(M1PhysicalKernargRecipeErrorV1::DispatchKind {
                dispatch_index: row.dispatch_index,
                kind: row.kind,
            })
        }
    }
}

fn check_row(
    row: &KernargRowInput,
    expected_program: M1PhysicalProgramV1,
    expected_grid: [u32; 3],
    expected_workgroup: [u32; 3],
    explicit_bytes: u64,
    total_bytes: u64,
) -> Result<ImageWriter, M1PhysicalKernargRecipeErrorV1> {
    if row.program != expected_program {
        return Err(M1PhysicalKernargRecipeErrorV1::Program {
            dispatch_index: row.dispatch_index,
            expected: expected_program,
            actual: row.program,
        });
    }
    let [workgroup_x, workgroup_y, workgroup_z] = expected_workgroup;
    let expected_workgroup = [workgroup_x, workgroup_y, workgroup_z].map(|value| {
        u16::try_from(value).map_err(|_| M1PhysicalKernargRecipeErrorV1::Geometry {
            dispatch_index: row.dispatch_index,
        })
    });
    let [workgroup_x, workgroup_y, workgroup_z] = expected_workgroup;
    let expected_workgroup = [workgroup_x?, workgroup_y?, workgroup_z?];
    if row.grid != expected_grid || row.workgroup != expected_workgroup {
        return Err(M1PhysicalKernargRecipeErrorV1::Geometry {
            dispatch_index: row.dispatch_index,
        });
    }
    if row.kernarg_bytes != total_bytes {
        return Err(M1PhysicalKernargRecipeErrorV1::KernargLength {
            dispatch_index: row.dispatch_index,
            expected: total_bytes,
            actual: row.kernarg_bytes,
        });
    }
    let pointer_offsets = match expected_program {
        M1PhysicalProgramV1::GemmReference
        | M1PhysicalProgramV1::GemmVectorized
        | M1PhysicalProgramV1::TokenEmbedding
        | M1PhysicalProgramV1::SwiGlu
        | M1PhysicalProgramV1::SpeculativeTokenAssembly => &[0, 16, 32][..],
        M1PhysicalProgramV1::RmsNorm | M1PhysicalProgramV1::GqaPrefill => &[0, 16, 32, 48, 64][..],
        M1PhysicalProgramV1::Rope => &[0, 16, 32, 48, 64, 80, 96][..],
        M1PhysicalProgramV1::PagedKvWrite | M1PhysicalProgramV1::PagedGqaDecode => {
            &[0, 16, 32, 48, 64, 80][..]
        }
        M1PhysicalProgramV1::LogitsArgmax => &[0, 16][..],
        M1PhysicalProgramV1::LogitsCompact => &[0, 16, 32, 48, 64, 80, 96, 112][..],
    };
    ImageWriter::new(
        row.dispatch_index,
        explicit_bytes,
        total_bytes,
        pointer_offsets,
    )
}

fn role_matches(selection: Qwen3PlanSelection, target: bool) -> bool {
    matches!(
        (selection.role, target),
        (Qwen3ModelRole::Target8B, true) | (Qwen3ModelRole::Draft06B, false)
    )
}

fn dimensions_match(selection: Qwen3PlanSelection, sequences: u32, active_tokens: u32) -> bool {
    selection
        .bucket
        .dimensions(selection.role, selection.mode)
        .is_some_and(|dimensions| {
            dimensions.sequences == sequences && dimensions.active_tokens == active_tokens
        })
}

fn assembly_profile_for_selection(
    selection: Qwen3PlanSelection,
) -> Option<logits::Qwen3SpeculativeTokenAssemblyProfileV1> {
    if selection.role != Qwen3ModelRole::Target8B
        || selection.mode != Qwen3ExecutionMode::Speculative
    {
        return None;
    }
    let bucket = match selection.bucket {
        Qwen3PlanBucket::SpeculativeS1K4C8192 => {
            logits::Qwen3LogitsBucketKindV1::SpeculativeS1K4C8192
        }
        Qwen3PlanBucket::SpeculativeS8K4C8192 => {
            logits::Qwen3LogitsBucketKindV1::SpeculativeS8K4C8192
        }
        Qwen3PlanBucket::SpeculativeS1K8C8192 => {
            logits::Qwen3LogitsBucketKindV1::SpeculativeS1K8C8192
        }
        Qwen3PlanBucket::SpeculativeS1K16C8192 => {
            logits::Qwen3LogitsBucketKindV1::SpeculativeS1K16C8192
        }
        _ => return None,
    };
    logits::Qwen3SpeculativeTokenAssemblyProfileV1::for_bucket(bucket)
}

fn encode_embedding(
    catalogs: &CanonicalKernargProfiles,
    row: &KernargRowInput,
) -> Result<M1PhysicalKernargImageV1, M1PhysicalKernargRecipeErrorV1> {
    let profile = catalogs
        .embedding
        .profiles()
        .iter()
        .copied()
        .find(|profile| profile.identity().as_bytes() == row.profile_id.as_bytes())
        .filter(|profile| {
            let [sequences, active_tokens] = profile.bucket().sequence_and_active_tokens();
            dimensions_match(row.selection, sequences, active_tokens)
                && role_matches(
                    row.selection,
                    matches!(
                        profile.bucket().role(),
                        gemm::Qwen3GemmModelRoleV1::Target8B
                    ),
                )
        })
        .ok_or_else(|| unresolved(row))?;
    if model_kind(row)? != M1OperationDispatchKind::WholeOperation {
        return Err(M1PhysicalKernargRecipeErrorV1::DispatchKind {
            dispatch_index: row.dispatch_index,
            kind: row.kind,
        });
    }
    let mut image = check_row(
        row,
        M1PhysicalProgramV1::TokenEmbedding,
        profile.aql_grid_workitems(),
        gemm::QWEN3_GEMM_WORKGROUP_V1,
        gemm::QWEN3_TOKEN_EMBEDDING_EXPLICIT_KERNARG_BYTES_V1,
        gemm::QWEN3_TOKEN_EMBEDDING_TOTAL_KERNARG_BYTES_V1,
    )?;
    let lengths = profile.storage_elements();
    for (offset, length) in [8, 24, 40].into_iter().zip(lengths) {
        image.write_u64(offset, length)?;
    }
    image.write_u32(48, profile.rows())?;
    image.write_u32(52, profile.hidden_size())?;
    image.write_u32(56, gemm::QWEN3_VOCABULARY_SIZE_V1)?;
    image.finish(row)
}

fn gemm_operation_matches(operator: Qwen3Operator, operation: gemm::Qwen3GemmOperationV1) -> bool {
    matches!(
        (operator, operation),
        (
            Qwen3Operator::QueryProjection,
            gemm::Qwen3GemmOperationV1::QueryProjection
        ) | (
            Qwen3Operator::KeyProjection,
            gemm::Qwen3GemmOperationV1::KeyProjection
        ) | (
            Qwen3Operator::ValueProjection,
            gemm::Qwen3GemmOperationV1::ValueProjection
        ) | (
            Qwen3Operator::AttentionOutputResidual,
            gemm::Qwen3GemmOperationV1::AttentionOutputResidual
        ) | (
            Qwen3Operator::GateProjection,
            gemm::Qwen3GemmOperationV1::GateProjection
        ) | (
            Qwen3Operator::UpProjection,
            gemm::Qwen3GemmOperationV1::UpProjection
        ) | (
            Qwen3Operator::DownResidual,
            gemm::Qwen3GemmOperationV1::DownResidual
        ) | (
            Qwen3Operator::LogitsProjection,
            gemm::Qwen3GemmOperationV1::LogitsProjection
        )
    )
}

fn encode_gemm(
    catalogs: &CanonicalKernargProfiles,
    row: &KernargRowInput,
) -> Result<M1PhysicalKernargImageV1, M1PhysicalKernargRecipeErrorV1> {
    let descriptor = model_descriptor(row)?;
    let profile = catalogs
        .gemm
        .profiles()
        .iter()
        .copied()
        .find(|profile| profile.identity().as_bytes() == row.profile_id.as_bytes())
        .filter(|profile| {
            let [sequences, active_tokens] = profile.bucket().sequence_and_active_tokens();
            dimensions_match(row.selection, sequences, active_tokens)
                && role_matches(
                    row.selection,
                    matches!(
                        profile.bucket().role(),
                        gemm::Qwen3GemmModelRoleV1::Target8B
                    ),
                )
                && gemm_operation_matches(descriptor.step.operator, profile.operation())
        })
        .ok_or_else(|| unresolved(row))?;
    if model_kind(row)? != M1OperationDispatchKind::WholeOperation {
        return Err(M1PhysicalKernargRecipeErrorV1::DispatchKind {
            dispatch_index: row.dispatch_index,
            kind: row.kind,
        });
    }
    let program = match profile.schedule() {
        gemm::Qwen3GemmScheduleV1::ReferenceWave64V1 => M1PhysicalProgramV1::GemmReference,
        gemm::Qwen3GemmScheduleV1::VectorizedA4Wave64V1 => M1PhysicalProgramV1::GemmVectorized,
    };
    let mut image = check_row(
        row,
        program,
        profile.aql_grid_workitems(),
        gemm::QWEN3_GEMM_WORKGROUP_V1,
        gemm::QWEN3_GEMM_EXPLICIT_KERNARG_BYTES_V1,
        gemm::QWEN3_GEMM_TOTAL_KERNARG_BYTES_V1,
    )?;
    for (offset, length) in [8, 24, 40].into_iter().zip(profile.storage_elements()) {
        image.write_u64(offset, length)?;
    }
    let [m, n, k] = profile.dimensions();
    image.write_u32(48, m)?;
    image.write_u32(52, n)?;
    image.write_u32(56, k)?;
    image.write_u32(60, profile.beta_bits())?;
    image.finish(row)
}

fn rmsnorm_operation_matches(
    operator: Qwen3Operator,
    operation: rmsnorm::Qwen3RmsNormOperationV1,
) -> bool {
    matches!(
        (operator, operation),
        (
            Qwen3Operator::InputRmsNorm,
            rmsnorm::Qwen3RmsNormOperationV1::InputRmsNorm
        ) | (
            Qwen3Operator::QueryRmsNorm,
            rmsnorm::Qwen3RmsNormOperationV1::QueryRmsNorm
        ) | (
            Qwen3Operator::KeyRmsNorm,
            rmsnorm::Qwen3RmsNormOperationV1::KeyRmsNorm
        ) | (
            Qwen3Operator::PostAttentionRmsNorm,
            rmsnorm::Qwen3RmsNormOperationV1::PostAttentionRmsNorm
        ) | (
            Qwen3Operator::FinalRmsNorm,
            rmsnorm::Qwen3RmsNormOperationV1::FinalRmsNorm
        )
    )
}

fn encode_rmsnorm(
    catalogs: &CanonicalKernargProfiles,
    row: &KernargRowInput,
) -> Result<M1PhysicalKernargImageV1, M1PhysicalKernargRecipeErrorV1> {
    let descriptor = model_descriptor(row)?;
    let profile = catalogs
        .rmsnorm
        .profiles()
        .iter()
        .copied()
        .find(|profile| profile.identity().as_bytes() == row.profile_id.as_bytes())
        .filter(|profile| {
            let [sequences, active_tokens] = profile.bucket().sequence_and_active_tokens();
            dimensions_match(row.selection, sequences, active_tokens)
                && role_matches(
                    row.selection,
                    matches!(
                        profile.bucket().role(),
                        rmsnorm::Qwen3RmsNormModelRoleV1::Target8B
                    ),
                )
                && rmsnorm_operation_matches(descriptor.step.operator, profile.operation())
        })
        .ok_or_else(|| unresolved(row))?;
    if model_kind(row)? != M1OperationDispatchKind::WholeOperation {
        return Err(M1PhysicalKernargRecipeErrorV1::DispatchKind {
            dispatch_index: row.dispatch_index,
            kind: row.kind,
        });
    }
    let mut image = check_row(
        row,
        M1PhysicalProgramV1::RmsNorm,
        profile.aql_grid_work_items(),
        rmsnorm::QWEN3_RMSNORM_WORKGROUP_V1,
        rmsnorm::QWEN3_RMSNORM_EXPLICIT_KERNARG_BYTES_V1,
        rmsnorm::QWEN3_RMSNORM_TOTAL_KERNARG_BYTES_V1,
    )?;
    let residual = match profile.behavior() {
        rmsnorm::Qwen3RmsNormBehaviorV1::Pure => 0,
        rmsnorm::Qwen3RmsNormBehaviorV1::ResidualFused => profile.row_elements(),
    };
    for (offset, length) in [8, 24, 40, 56, 72].into_iter().zip([
        profile.row_elements(),
        residual,
        profile.weight_elements(),
        residual,
        profile.row_elements(),
    ]) {
        image.write_u64(offset, length)?;
    }
    image.write_u32(80, profile.rows())?;
    image.write_u32(84, profile.width())?;
    image.write_u32(88, profile.epsilon_bits())?;
    image.write_u32(92, profile.behavior() as u32)?;
    image.finish(row)
}

fn encode_rope_kv(
    catalogs: &CanonicalKernargProfiles,
    row: &KernargRowInput,
) -> Result<M1PhysicalKernargImageV1, M1PhysicalKernargRecipeErrorV1> {
    let descriptor = model_descriptor(row)?;
    let profile = catalogs
        .rope_kv
        .profiles()
        .iter()
        .copied()
        .find(|profile| profile.identity().as_bytes() == row.profile_id.as_bytes())
        .filter(|profile| {
            let [sequences, active_tokens] = profile.bucket().sequence_and_active_tokens();
            dimensions_match(row.selection, sequences, active_tokens)
                && role_matches(
                    row.selection,
                    matches!(
                        profile.bucket().role(),
                        rope_kv::Qwen3RopeKvModelRoleV1::Target8B
                    ),
                )
                && matches!(
                    (descriptor.step.operator, profile.operation()),
                    (Qwen3Operator::Rope, rope_kv::Qwen3RopeKvOperationV1::Rope)
                        | (
                            Qwen3Operator::KvWrite,
                            rope_kv::Qwen3RopeKvOperationV1::PagedKvWrite
                        )
                )
        })
        .ok_or_else(|| unresolved(row))?;
    if model_kind(row)? != M1OperationDispatchKind::WholeOperation {
        return Err(M1PhysicalKernargRecipeErrorV1::DispatchKind {
            dispatch_index: row.dispatch_index,
            kind: row.kind,
        });
    }
    let [sequences, active_tokens] = profile.bucket().sequence_and_active_tokens();
    match profile.operation() {
        rope_kv::Qwen3RopeKvOperationV1::Rope => {
            let mut image = check_row(
                row,
                M1PhysicalProgramV1::Rope,
                profile.aql_grid_work_items(),
                rope_kv::QWEN3_ROPE_KV_WORKGROUP_V1,
                rope_kv::QWEN3_ROPE_EXPLICIT_KERNARG_BYTES_V1,
                rope_kv::QWEN3_ROPE_TOTAL_KERNARG_BYTES_V1,
            )?;
            for (offset, length) in [8, 24, 40, 56, 72, 88, 104].into_iter().zip([
                profile.query_elements(),
                profile.kv_elements(),
                u64::from(profile.base_rows()),
                rope_kv::QWEN3_ROPE_TRIG_TABLE_ELEMENTS_V1,
                rope_kv::QWEN3_ROPE_TRIG_TABLE_ELEMENTS_V1,
                profile.query_elements(),
                profile.kv_elements(),
            ]) {
                image.write_u64(offset, length)?;
            }
            image.write_u32(112, active_tokens)?;
            image.write_u32(116, sequences)?;
            image.write_u32(120, profile.query_heads())?;
            image.write_u32(124, profile.context_tokens())?;
            image.finish(row)
        }
        rope_kv::Qwen3RopeKvOperationV1::PagedKvWrite => {
            let page_indices = u64::from(sequences)
                .checked_mul(u64::from(rope_kv::QWEN3_KV_PAGE_TABLE_ENTRIES_V1))
                .ok_or(M1PhysicalKernargRecipeErrorV1::ArithmeticOverflow {
                    dispatch_index: row.dispatch_index,
                })?;
            let mut image = check_row(
                row,
                M1PhysicalProgramV1::PagedKvWrite,
                profile.aql_grid_work_items(),
                rope_kv::QWEN3_ROPE_KV_WORKGROUP_V1,
                rope_kv::QWEN3_KV_WRITE_EXPLICIT_KERNARG_BYTES_V1,
                rope_kv::QWEN3_KV_WRITE_TOTAL_KERNARG_BYTES_V1,
            )?;
            for (offset, length) in [8, 24, 40, 56, 72, 88].into_iter().zip([
                profile.kv_elements(),
                profile.kv_elements(),
                u64::from(sequences),
                page_indices,
                rope_kv::QWEN3_KV_CACHE_ELEMENTS_V1,
                rope_kv::QWEN3_KV_CACHE_ELEMENTS_V1,
            ]) {
                image.write_u64(offset, length)?;
            }
            image.write_u32(96, active_tokens)?;
            image.write_u32(100, sequences)?;
            image.write_u32(104, profile.context_tokens())?;
            image.finish(row)
        }
    }
}

fn encode_prefill(
    catalogs: &CanonicalKernargProfiles,
    row: &KernargRowInput,
) -> Result<M1PhysicalKernargImageV1, M1PhysicalKernargRecipeErrorV1> {
    let profile = catalogs
        .prefill
        .profiles()
        .iter()
        .copied()
        .find(|profile| profile.identity().as_bytes() == row.profile_id.as_bytes())
        .filter(|profile| {
            dimensions_match(row.selection, profile.sequences(), profile.tokens())
                && role_matches(
                    row.selection,
                    matches!(profile.role(), prefill::Qwen3PrefillModelRoleV1::Target8B),
                )
        })
        .ok_or_else(|| unresolved(row))?;
    if model_kind(row)? != M1OperationDispatchKind::WholeOperation {
        return Err(M1PhysicalKernargRecipeErrorV1::DispatchKind {
            dispatch_index: row.dispatch_index,
            kind: row.kind,
        });
    }
    let mut image = check_row(
        row,
        M1PhysicalProgramV1::GqaPrefill,
        profile.launch_workitems(),
        prefill::QWEN3_PREFILL_WORKGROUP_V1,
        prefill::QWEN3_PREFILL_EXPLICIT_KERNARG_BYTES_V1,
        prefill::QWEN3_PREFILL_TOTAL_KERNARG_BYTES_V1,
    )?;
    for (offset, length) in [8, 24, 40, 56, 72].into_iter().zip([
        profile.query_elements(),
        profile.cache_elements_each(),
        profile.cache_elements_each(),
        profile.page_table_elements(),
        profile.query_elements(),
    ]) {
        image.write_u64(offset, length)?;
    }
    image.finish(row)
}

fn encode_paged_decode(
    catalogs: &CanonicalKernargProfiles,
    row: &KernargRowInput,
) -> Result<M1PhysicalKernargImageV1, M1PhysicalKernargRecipeErrorV1> {
    let profile = catalogs
        .paged_decode
        .profiles()
        .iter()
        .copied()
        .find(|profile| profile.identity().as_bytes() == row.profile_id.as_bytes())
        .filter(|profile| {
            dimensions_match(row.selection, profile.sequences(), profile.active_tokens())
                && role_matches(
                    row.selection,
                    matches!(
                        profile.role(),
                        paged_decode::Qwen3PagedDecodeModelRoleV1::Target8B
                    ),
                )
        })
        .ok_or_else(|| unresolved(row))?;
    if model_kind(row)? != M1OperationDispatchKind::WholeOperation {
        return Err(M1PhysicalKernargRecipeErrorV1::DispatchKind {
            dispatch_index: row.dispatch_index,
            kind: row.kind,
        });
    }
    let mut image = check_row(
        row,
        M1PhysicalProgramV1::PagedGqaDecode,
        profile.launch_workitems(),
        paged_decode::QWEN3_PAGED_DECODE_WORKGROUP_V1,
        paged_decode::QWEN3_PAGED_DECODE_EXPLICIT_KERNARG_BYTES_V1,
        paged_decode::QWEN3_PAGED_DECODE_TOTAL_KERNARG_BYTES_V1,
    )?;
    for (offset, length) in [8, 24, 40, 56, 72, 88].into_iter().zip([
        profile.query_elements(),
        profile.cache_elements_each(),
        profile.cache_elements_each(),
        profile.page_table_elements(),
        profile.context_elements(),
        profile.query_elements(),
    ]) {
        image.write_u64(offset, length)?;
    }
    image.finish(row)
}

fn encode_swiglu(
    catalogs: &CanonicalKernargProfiles,
    row: &KernargRowInput,
) -> Result<M1PhysicalKernargImageV1, M1PhysicalKernargRecipeErrorV1> {
    let profile = catalogs
        .swiglu
        .profiles()
        .iter()
        .copied()
        .find(|profile| profile.identity().as_bytes() == row.profile_id.as_bytes())
        .filter(|profile| {
            dimensions_match(row.selection, profile.sequences(), profile.active_tokens())
                && role_matches(
                    row.selection,
                    matches!(profile.role(), swiglu::Qwen3SwiGluModelRoleV1::Target8B),
                )
        })
        .ok_or_else(|| unresolved(row))?;
    if model_kind(row)? != M1OperationDispatchKind::WholeOperation {
        return Err(M1PhysicalKernargRecipeErrorV1::DispatchKind {
            dispatch_index: row.dispatch_index,
            kind: row.kind,
        });
    }
    let mut image = check_row(
        row,
        M1PhysicalProgramV1::SwiGlu,
        profile.launch_workitems(),
        swiglu::QWEN3_SWIGLU_WORKGROUP_V1,
        swiglu::QWEN3_SWIGLU_EXPLICIT_KERNARG_BYTES_V1,
        swiglu::QWEN3_SWIGLU_TOTAL_KERNARG_BYTES_V1,
    )?;
    for offset in [8, 24, 40] {
        image.write_u64(offset, profile.elements())?;
    }
    image.finish(row)
}

fn encode_logits(
    catalogs: &CanonicalKernargProfiles,
    row: &KernargRowInput,
) -> Result<M1PhysicalKernargImageV1, M1PhysicalKernargRecipeErrorV1> {
    let profile = catalogs
        .logits
        .profiles()
        .iter()
        .copied()
        .find(|profile| profile.identity().as_bytes() == row.profile_id.as_bytes())
        .filter(|profile| {
            let [sequences, active_tokens] = profile.choice_shape();
            dimensions_match(row.selection, sequences, active_tokens)
                && role_matches(
                    row.selection,
                    matches!(
                        profile.bucket().role(),
                        logits::Qwen3LogitsModelRoleV1::Target8B
                    ),
                )
        })
        .ok_or_else(|| unresolved(row))?;
    let [logits_elements, choices_elements, draft_elements, record_bytes] =
        profile.storage_extents();
    match model_kind(row)? {
        M1OperationDispatchKind::K7Argmax => {
            let mut image = check_row(
                row,
                M1PhysicalProgramV1::LogitsArgmax,
                profile.argmax_grid_workitems(),
                logits::QWEN3_LOGITS_WORKGROUP_V1,
                logits::QWEN3_LOGITS_ARGMAX_EXPLICIT_KERNARG_BYTES_V1,
                logits::QWEN3_LOGITS_ARGMAX_TOTAL_KERNARG_BYTES_V1,
            )?;
            image.write_u64(8, logits_elements)?;
            image.write_u64(24, choices_elements)?;
            image.write_u32(32, profile.bucket().rows())?;
            image.write_u32(36, logits::QWEN3_LOGITS_VOCABULARY_V1)?;
            image.finish(row)
        }
        M1OperationDispatchKind::K7Compact => {
            let grid = profile
                .compact_grid_workitems()
                .ok_or_else(|| unresolved(row))?;
            let [sequences, active_tokens] = profile.choice_shape();
            let plans = u64::from(sequences).checked_mul(32).ok_or(
                M1PhysicalKernargRecipeErrorV1::ArithmeticOverflow {
                    dispatch_index: row.dispatch_index,
                },
            )?;
            let mut image = check_row(
                row,
                M1PhysicalProgramV1::LogitsCompact,
                grid,
                logits::QWEN3_LOGITS_WORKGROUP_V1,
                logits::QWEN3_LOGITS_COMPACT_EXPLICIT_KERNARG_BYTES_V1,
                logits::QWEN3_LOGITS_COMPACT_TOTAL_KERNARG_BYTES_V1,
            )?;
            for (offset, length) in [8, 24, 40, 56, 72, 88, 104, 120].into_iter().zip([
                choices_elements,
                draft_elements,
                u64::from(sequences),
                u64::from(sequences),
                u64::from(sequences),
                u64::from(sequences),
                plans,
                record_bytes,
            ]) {
                image.write_u64(offset, length)?;
            }
            image.write_u32(128, sequences)?;
            image.write_u32(132, active_tokens)?;
            image.write_u32(136, profile.speculative_k())?;
            image.finish(row)
        }
        M1OperationDispatchKind::WholeOperation => {
            Err(M1PhysicalKernargRecipeErrorV1::DispatchKind {
                dispatch_index: row.dispatch_index,
                kind: row.kind,
            })
        }
    }
}

fn encode_speculative_token_assembly(
    row: &KernargRowInput,
) -> Result<M1PhysicalKernargImageV1, M1PhysicalKernargRecipeErrorV1> {
    let profile = row.assembly_profile.ok_or(
        M1PhysicalKernargRecipeErrorV1::InfrastructureProfileIdentity {
            dispatch_index: row.dispatch_index,
        },
    )?;
    if row.kind != M1PhysicalDispatchKindV1::SpeculativeTokenAssembly {
        return Err(M1PhysicalKernargRecipeErrorV1::DispatchKind {
            dispatch_index: row.dispatch_index,
            kind: row.kind,
        });
    }
    if assembly_profile_for_selection(row.selection) != Some(profile)
        || row.profile_id.as_bytes() != profile.identity().as_bytes()
    {
        return Err(
            M1PhysicalKernargRecipeErrorV1::InfrastructureProfileIdentity {
                dispatch_index: row.dispatch_index,
            },
        );
    }
    let mut image = check_row(
        row,
        M1PhysicalProgramV1::SpeculativeTokenAssembly,
        profile.grid_workitems(),
        logits::QWEN3_LOGITS_WORKGROUP_V1,
        logits::QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_EXPLICIT_KERNARG_BYTES_V1,
        logits::QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_TOTAL_KERNARG_BYTES_V1,
    )?;
    let [anchor, draft, target] = profile.storage_extents();
    image.write_u64(8, anchor)?;
    image.write_u64(24, draft)?;
    image.write_u64(40, target)?;
    image.write_u32(48, profile.sequences())?;
    image.write_u32(52, profile.speculative_k())?;
    image.finish(row)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use ferric_qwen_kernels::{gemm, logits, paged_decode, prefill, rmsnorm, rope_kv, swiglu};
    use ferric_spec::{
        Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection,
    };

    use super::{
        derive_image, derive_input_images, derive_m1_physical_kernarg_recipe_v1,
        validate_descriptor, CanonicalKernargProfiles, ImageWriter, KernargRowInput,
        M1PhysicalKernargRecipeErrorV1, M1_COV6_HIDDEN_KERNARG_BYTES_V1,
        M1_PHYSICAL_KERNARG_RECIPE_VERSION_V1,
    };
    use crate::operation_kernel_plan::tests::public_operation_kernel_plan_fixture;
    use crate::{
        derive_m1_physical_dispatch_recipe_v1, derive_m1_step_dispatch_plan,
        AddresslessM1PhysicalDispatchRecipeV1, M1OperationDispatchKind, M1PhysicalProgramV1,
        M1StepDispatchIntent,
    };

    const fn target(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode,
            bucket,
        }
    }

    fn complete_intents() -> [M1StepDispatchIntent; 15] {
        [
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS8T128,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T512,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T2048,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS8C8192,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS32C8192,
            )),
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128,
            )),
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS8T128,
            )),
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T512,
            )),
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T2048,
            )),
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
            )),
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS8K4C8192,
            )),
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K8C8192,
            )),
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K16C8192,
            )),
        ]
    }

    fn physical_recipe(intent: M1StepDispatchIntent) -> AddresslessM1PhysicalDispatchRecipeV1 {
        let operation_plan = public_operation_kernel_plan_fixture();
        let step = derive_m1_step_dispatch_plan(&operation_plan, intent).unwrap();
        derive_m1_physical_dispatch_recipe_v1(&step).unwrap()
    }

    fn pointer_offsets(program: M1PhysicalProgramV1) -> &'static [usize] {
        match program {
            M1PhysicalProgramV1::GemmReference
            | M1PhysicalProgramV1::GemmVectorized
            | M1PhysicalProgramV1::TokenEmbedding
            | M1PhysicalProgramV1::SwiGlu
            | M1PhysicalProgramV1::SpeculativeTokenAssembly => &[0, 16, 32],
            M1PhysicalProgramV1::RmsNorm | M1PhysicalProgramV1::GqaPrefill => &[0, 16, 32, 48, 64],
            M1PhysicalProgramV1::Rope => &[0, 16, 32, 48, 64, 80, 96],
            M1PhysicalProgramV1::PagedKvWrite | M1PhysicalProgramV1::PagedGqaDecode => {
                &[0, 16, 32, 48, 64, 80]
            }
            M1PhysicalProgramV1::LogitsArgmax => &[0, 16],
            M1PhysicalProgramV1::LogitsCompact => &[0, 16, 32, 48, 64, 80, 96, 112],
        }
    }

    fn read_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    fn read_u64(bytes: &[u8], offset: usize) -> u64 {
        u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
    }

    #[test]
    fn every_complete_intent_derives_exact_zero_pointer_images_for_all_programs() {
        let mut programs = HashSet::new();
        let mut selections = Vec::new();
        for intent in complete_intents() {
            let source = physical_recipe(intent);
            let dispatch_count = source.dispatch_count();
            let composition_id = source.composition_id();
            let recipe = derive_m1_physical_kernarg_recipe_v1(source).unwrap();
            assert_eq!(recipe.version(), M1_PHYSICAL_KERNARG_RECIPE_VERSION_V1);
            assert_eq!(recipe.source_recipe().composition_id(), composition_id);
            assert_eq!(recipe.images().len(), dispatch_count as usize);
            assert!(!recipe.binds_device_buffers());
            assert!(!recipe.grants_packet_or_queue_authority());
            assert!(!recipe.proves_hardware_or_operator_refinement());

            for (index, (row, image)) in recipe
                .source_recipe()
                .rows()
                .iter()
                .zip(recipe.images())
                .enumerate()
            {
                assert_eq!(image.dispatch_index(), u32::try_from(index).unwrap());
                assert_eq!(image.selection(), row.selection());
                assert_eq!(image.profile_id(), row.profile_id());
                assert_eq!(image.program(), row.program());
                assert_eq!(
                    image.bytes().len(),
                    usize::try_from(row.kernarg_bytes()).unwrap()
                );
                assert_eq!(
                    image.cov6_hidden_suffix().len(),
                    M1_COV6_HIDDEN_KERNARG_BYTES_V1
                );
                assert!(image.cov6_hidden_suffix().iter().all(|byte| *byte == 0));
                assert!(pointer_offsets(image.program()).iter().all(|offset| {
                    image.bytes()[*offset..*offset + 8]
                        .iter()
                        .all(|byte| *byte == 0)
                }));
                assert!(image.bytes()[..image.explicit_bytes()]
                    .iter()
                    .any(|byte| *byte != 0));
                assert!(!image.binds_device_buffers());
                assert!(!image.grants_packet_or_queue_authority());
                assert!(!image.proves_hardware_or_operator_refinement());
                programs.insert(image.program());
                if !selections.contains(&image.selection()) {
                    selections.push(image.selection());
                }
            }
        }
        assert_eq!(programs, M1PhysicalProgramV1::ALL.into_iter().collect());
        // Complete M1 intents use all eleven target selections plus four draft
        // prefill and two draft decode selections. Draft decode-S32 and draft
        // speculative profiles are not independently publishable step intents.
        assert_eq!(selections.len(), 17);
    }

    #[test]
    fn representative_images_encode_inspected_element_lengths_and_scalars_little_endian() {
        let recipe = derive_m1_physical_kernarg_recipe_v1(physical_recipe(
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
            )),
        ))
        .unwrap();

        let embedding = recipe
            .images()
            .iter()
            .find(|image| image.program() == M1PhysicalProgramV1::TokenEmbedding)
            .unwrap();
        assert_eq!(read_u64(embedding.bytes(), 8), 1);
        assert_eq!(read_u32(embedding.bytes(), 48), 1);
        assert_eq!(read_u32(embedding.bytes(), 52), 1_024);
        assert_eq!(read_u32(embedding.bytes(), 56), 151_936);

        let compact = recipe
            .images()
            .iter()
            .find(|image| image.program() == M1PhysicalProgramV1::LogitsCompact)
            .unwrap();
        assert_eq!(read_u64(compact.bytes(), 8), 5);
        assert_eq!(read_u64(compact.bytes(), 24), 4);
        assert_eq!(read_u64(compact.bytes(), 104), 32);
        assert_eq!(read_u64(compact.bytes(), 120), 120);
        assert_eq!(read_u32(compact.bytes(), 128), 1);
        assert_eq!(read_u32(compact.bytes(), 132), 5);
        assert_eq!(read_u32(compact.bytes(), 136), 4);

        let s8 = derive_m1_physical_kernarg_recipe_v1(physical_recipe(
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS8K4C8192,
            )),
        ))
        .unwrap();
        let assembly = s8
            .images()
            .iter()
            .find(|image| image.program() == M1PhysicalProgramV1::SpeculativeTokenAssembly)
            .unwrap();
        assert_eq!(assembly.explicit_bytes(), 56);
        assert_eq!(assembly.bytes().len(), 312);
        assert_eq!(read_u64(assembly.bytes(), 8), 8);
        assert_eq!(read_u64(assembly.bytes(), 24), 32);
        assert_eq!(read_u64(assembly.bytes(), 40), 40);
        assert_eq!(read_u32(assembly.bytes(), 48), 8);
        assert_eq!(read_u32(assembly.bytes(), 52), 4);
        assert!(assembly.bytes()[0..8].iter().all(|byte| *byte == 0));
        assert!(assembly.bytes()[16..24].iter().all(|byte| *byte == 0));
        assert!(assembly.bytes()[32..40].iter().all(|byte| *byte == 0));
        assert!(assembly.cov6_hidden_suffix().iter().all(|byte| *byte == 0));
    }

    fn all_finite_selections() -> Vec<Qwen3PlanSelection> {
        let buckets = [
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128),
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS8T128),
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T512),
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T2048),
            (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192),
            (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192),
            (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS32C8192),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS8K4C8192,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K8C8192,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K16C8192,
            ),
        ];
        [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B]
            .into_iter()
            .flat_map(|role| {
                buckets
                    .into_iter()
                    .map(move |(mode, bucket)| Qwen3PlanSelection { role, mode, bucket })
            })
            .collect()
    }

    #[test]
    fn all_twenty_two_finite_selections_resolve_in_every_required_profile_family() {
        let catalogs = CanonicalKernargProfiles::new().unwrap();
        let selections = all_finite_selections();
        assert_eq!(selections.len(), 22);
        for selection in selections {
            let dimensions = selection
                .bucket
                .dimensions(selection.role, selection.mode)
                .unwrap();
            let target_role = selection.role == Qwen3ModelRole::Target8B;
            assert_eq!(
                catalogs
                    .gemm
                    .profiles()
                    .iter()
                    .filter(|profile| {
                        let [sequences, active_tokens] =
                            profile.bucket().sequence_and_active_tokens();
                        sequences == dimensions.sequences
                            && active_tokens == dimensions.active_tokens
                            && matches!(
                                profile.bucket().role(),
                                gemm::Qwen3GemmModelRoleV1::Target8B
                            ) == target_role
                    })
                    .count(),
                gemm::QWEN3_GEMM_OPERATION_COUNT_V1
            );
            assert!(catalogs.embedding.profiles().iter().any(|profile| {
                let [sequences, active_tokens] = profile.bucket().sequence_and_active_tokens();
                sequences == dimensions.sequences
                    && active_tokens == dimensions.active_tokens
                    && matches!(
                        profile.bucket().role(),
                        gemm::Qwen3GemmModelRoleV1::Target8B
                    ) == target_role
            }));
            assert!(catalogs.rmsnorm.profiles().iter().any(|profile| {
                let [sequences, active_tokens] = profile.bucket().sequence_and_active_tokens();
                sequences == dimensions.sequences
                    && active_tokens == dimensions.active_tokens
                    && matches!(
                        profile.bucket().role(),
                        rmsnorm::Qwen3RmsNormModelRoleV1::Target8B
                    ) == target_role
            }));
            assert!(catalogs.rope_kv.profiles().iter().any(|profile| {
                let [sequences, active_tokens] = profile.bucket().sequence_and_active_tokens();
                sequences == dimensions.sequences
                    && active_tokens == dimensions.active_tokens
                    && profile.context_tokens() == dimensions.context_tokens
                    && matches!(
                        profile.bucket().role(),
                        rope_kv::Qwen3RopeKvModelRoleV1::Target8B
                    ) == target_role
            }));
            assert!(catalogs.swiglu.profiles().iter().any(|profile| {
                profile.sequences() == dimensions.sequences
                    && profile.active_tokens() == dimensions.active_tokens
                    && matches!(profile.role(), swiglu::Qwen3SwiGluModelRoleV1::Target8B)
                        == target_role
            }));
            assert!(catalogs.logits.profiles().iter().any(|profile| {
                profile.choice_shape() == [dimensions.sequences, dimensions.active_tokens]
                    && matches!(
                        profile.bucket().role(),
                        logits::Qwen3LogitsModelRoleV1::Target8B
                    ) == target_role
            }));
            let attention_resolves = match selection.mode {
                Qwen3ExecutionMode::Prefill => catalogs.prefill.profiles().iter().any(|profile| {
                    profile.sequences() == dimensions.sequences
                        && profile.tokens() == dimensions.active_tokens
                        && matches!(profile.role(), prefill::Qwen3PrefillModelRoleV1::Target8B)
                            == target_role
                }),
                Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => {
                    catalogs.paged_decode.profiles().iter().any(|profile| {
                        profile.sequences() == dimensions.sequences
                            && profile.active_tokens() == dimensions.active_tokens
                            && matches!(
                                profile.role(),
                                paged_decode::Qwen3PagedDecodeModelRoleV1::Target8B
                            ) == target_role
                    })
                }
            };
            assert!(attention_resolves);
        }
    }

    fn hostile_fixture() -> (CanonicalKernargProfiles, Vec<KernargRowInput>) {
        let source = physical_recipe(M1StepDispatchIntent::TargetOnly(target(
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        )));
        let rows = source
            .rows()
            .iter()
            .copied()
            .map(KernargRowInput::from)
            .collect();
        (CanonicalKernargProfiles::new().unwrap(), rows)
    }

    #[test]
    fn hostile_order_descriptor_identity_kind_program_geometry_and_length_drift_fail_closed() {
        let (catalogs, rows) = hostile_fixture();

        let mut order = rows.clone();
        order[0].dispatch_index = 1;
        assert_eq!(
            derive_input_images(&catalogs, &order).unwrap_err(),
            M1PhysicalKernargRecipeErrorV1::DispatchOrder {
                expected: 0,
                actual: 1,
            }
        );

        let mut descriptor = rows[0];
        descriptor.profile.as_mut().unwrap().selection =
            target(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192);
        assert_eq!(
            validate_descriptor(&descriptor),
            Err(M1PhysicalKernargRecipeErrorV1::ProfileDescriptor {
                dispatch_index: descriptor.dispatch_index,
            })
        );

        let mut identity = rows[0];
        identity.profile_id = Identity::new([0; 32]);
        assert!(matches!(
            derive_image(&catalogs, &identity),
            Err(M1PhysicalKernargRecipeErrorV1::ProfileIdentity { .. })
        ));

        let mut kind = rows[0];
        kind.kind = crate::M1PhysicalDispatchKindV1::Model(M1OperationDispatchKind::K7Argmax);
        assert!(matches!(
            derive_image(&catalogs, &kind),
            Err(M1PhysicalKernargRecipeErrorV1::DispatchKind { .. })
        ));

        let mut program = rows[0];
        program.program = M1PhysicalProgramV1::RmsNorm;
        assert!(matches!(
            derive_image(&catalogs, &program),
            Err(M1PhysicalKernargRecipeErrorV1::Program { .. })
        ));

        let mut geometry = rows[0];
        geometry.grid[0] += 1;
        assert!(matches!(
            derive_image(&catalogs, &geometry),
            Err(M1PhysicalKernargRecipeErrorV1::Geometry { .. })
        ));

        let mut length = rows[0];
        length.kernarg_bytes -= 1;
        assert!(matches!(
            derive_image(&catalogs, &length),
            Err(M1PhysicalKernargRecipeErrorV1::KernargLength { .. })
        ));
    }

    #[test]
    fn canonical_identity_from_another_selection_is_rejected() {
        let target_s1 = physical_recipe(M1StepDispatchIntent::TargetOnly(target(
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        )));
        let target_s8 = physical_recipe(M1StepDispatchIntent::TargetOnly(target(
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        )));
        let mut row = KernargRowInput::from(target_s1.rows()[0]);
        row.profile_id = target_s8.rows()[0].profile_id();
        assert!(matches!(
            derive_image(&CanonicalKernargProfiles::new().unwrap(), &row),
            Err(M1PhysicalKernargRecipeErrorV1::ProfileIdentity { .. })
        ));
    }

    #[test]
    fn hostile_assembly_profile_kind_program_geometry_and_abi_drift_fail_closed() {
        let catalogs = CanonicalKernargProfiles::new().unwrap();
        let source = physical_recipe(M1StepDispatchIntent::SpeculativeRound(target(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
        )));
        let row = source
            .rows()
            .iter()
            .copied()
            .find(|row| row.program() == M1PhysicalProgramV1::SpeculativeTokenAssembly)
            .map(KernargRowInput::from)
            .unwrap();

        let mut wrong_profile = row;
        wrong_profile.assembly_profile = logits::Qwen3SpeculativeTokenAssemblyProfileV1::for_bucket(
            logits::Qwen3LogitsBucketKindV1::SpeculativeS1K4C8192,
        );
        assert_eq!(
            validate_descriptor(&wrong_profile),
            Err(M1PhysicalKernargRecipeErrorV1::ProfileDescriptor {
                dispatch_index: row.dispatch_index,
            })
        );

        let mut wrong_identity = row;
        wrong_identity.profile_id = Identity::new([0; 32]);
        assert_eq!(
            derive_image(&catalogs, &wrong_identity),
            Err(
                M1PhysicalKernargRecipeErrorV1::InfrastructureProfileIdentity {
                    dispatch_index: row.dispatch_index,
                }
            )
        );

        let mut wrong_kind = row;
        wrong_kind.kind =
            crate::M1PhysicalDispatchKindV1::Model(M1OperationDispatchKind::WholeOperation);
        assert_eq!(
            derive_image(&catalogs, &wrong_kind),
            Err(M1PhysicalKernargRecipeErrorV1::ProfileDescriptor {
                dispatch_index: row.dispatch_index,
            })
        );

        let mut wrong_program = row;
        wrong_program.program = M1PhysicalProgramV1::LogitsArgmax;
        assert!(matches!(
            derive_image(&catalogs, &wrong_program),
            Err(M1PhysicalKernargRecipeErrorV1::Program { .. })
        ));

        let mut wrong_geometry = row;
        wrong_geometry.grid[0] += 64;
        assert!(matches!(
            derive_image(&catalogs, &wrong_geometry),
            Err(M1PhysicalKernargRecipeErrorV1::Geometry { .. })
        ));

        let mut wrong_abi = row;
        wrong_abi.kernarg_bytes -= 1;
        assert!(matches!(
            derive_image(&catalogs, &wrong_abi),
            Err(M1PhysicalKernargRecipeErrorV1::KernargLength { .. })
        ));
    }

    #[test]
    fn checked_writer_rejects_layout_bounds_pointer_and_hidden_suffix_corruption() {
        let (_, rows) = hostile_fixture();
        let row = rows[0];
        assert_eq!(
            ImageWriter::new(row.dispatch_index, 64, 319, &[0]).unwrap_err(),
            M1PhysicalKernargRecipeErrorV1::Cov6Layout {
                dispatch_index: row.dispatch_index,
                explicit_bytes: 64,
                total_bytes: 319,
            }
        );

        let mut bounds = ImageWriter::new(row.dispatch_index, 64, 320, &[0]).unwrap();
        assert_eq!(
            bounds.write_u64(316, 1),
            Err(M1PhysicalKernargRecipeErrorV1::WriteBounds {
                dispatch_index: row.dispatch_index,
                offset: 316,
                width: 8,
                image_bytes: 320,
            })
        );

        let mut pointer = ImageWriter::new(row.dispatch_index, 64, 320, &[0]).unwrap();
        pointer.write_u64(0, 1).unwrap();
        assert_eq!(
            pointer.finish(&row).unwrap_err(),
            M1PhysicalKernargRecipeErrorV1::PointerNotZero {
                dispatch_index: row.dispatch_index,
                offset: 0,
            }
        );

        let mut hidden = ImageWriter::new(row.dispatch_index, 64, 320, &[0]).unwrap();
        hidden.write_u32(64, 1).unwrap();
        assert_eq!(
            hidden.finish(&row).unwrap_err(),
            M1PhysicalKernargRecipeErrorV1::HiddenSuffixNotZero {
                dispatch_index: row.dispatch_index,
            }
        );
    }
}
