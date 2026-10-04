//! Inert, lossless diagnostics for an exact retained numerical wave worker.
//! Canonical KIR bytes retain complete operations, not a second reduced opcode
//! schema. Locations below refer only into that exact decoded inert module.

use super::{
    ProductionCanonicalKernelIrIdentityV1 as KirIdentity,
    ProductionCanonicalKernelIrVersionV1 as KirVersion, ProductionWaveTaskFormalMemoryErrorV1,
    WaveTaskFormalCoreV2 as Formal,
};
use fe2o3_kernel_ir as kir;
use fe2o3_kernel_ir::{
    CanonicalKernelIrVerificationResourceBudgetV1 as Budget,
    CanonicalKernelIrVerificationResourceErrorV1, FormalMemoryIncompleteReason as Reason,
    FunctionOperationLocation as Location,
    InertCanonicalFormalMemoryObligationReceiptV1 as RawReceipt, Operation, OperationKind,
    SemanticWaveTaskProfileV2 as LegacyProfile, SemanticWaveTaskProfileV3 as Profile,
    SemanticWaveTaskRoleV1 as Role,
};
use fe2o3_pliron::{
    InertProductionMiddleEndWaveTaskEvidenceV1 as InertMiddle,
    ProductionMiddleEndWaveTaskEvidenceV1 as Middle,
    ProductionWaveTaskConditionalRankedInputV1 as Ranked,
    ProductionWaveTaskUnresolvedRequirementV1 as Requirement,
};
use sha2::{Digest, Sha256};
use std::{fmt, mem::size_of, ops::Range};

const MAGIC: &[u8; 8] = b"F2FMWT1\0";
const DOMAIN: &[u8] = b"FE2O3/INERT-WAVE-TASK-FORMAL-MEMORY-EVIDENCE/V1\0";
const QKV_MAGIC: &[u8; 8] = b"F2FMWQ2\0";
const QKV_DOMAIN: &[u8] = b"FE2O3/INERT-WAVE-QKV-TASK-FORMAL-MEMORY-EVIDENCE/V2\0";
const POST_MAGIC: &[u8; 8] = b"F2FMWP3\0";
const POST_DOMAIN: &[u8] = b"FE2O3/INERT-WAVE-QKV-POST-TASK-FORMAL-MEMORY-EVIDENCE/V3\0";
const ATTENTION_MAGIC: &[u8; 8] = b"F2FMWA4\0";
const ATTENTION_DOMAIN: &[u8] = b"FE2O3/INERT-WAVE-QKV-ATTENTION-TASK-FORMAL-MEMORY-EVIDENCE/V4\0";
const ATTENTION_OUTPUT_MAGIC: &[u8; 8] = b"F2FMWO5\0";
const MLP_MAGIC: &[u8; 8] = b"F2FMWM1\0";
const MLP_DOMAIN: &[u8] = b"FE2O3/INERT-WAVE-MLP-TASK-FORMAL-MEMORY-EVIDENCE/V1\0";
const MLP_TILE_MAGIC: &[u8; 8] = b"F2FMWM2\0";
const MULTIWAVE_MAGIC: &[u8; 8] = b"F2FMMW1\0";
const MULTIWAVE_DOMAIN: &[u8] = b"FE2O3/INERT-MULTIWAVE-JOIN-FORMAL-MEMORY-EVIDENCE/V1\0";
const ATTENTION_OUTPUT_TILE_MAGIC: &[u8; 8] = b"F2FMWO6\0";
const ATTENTION_OUTPUT_TILE_DOMAIN: &[u8] =
    b"FE2O3/INERT-WAVE-QKV-ATTENTION-OUTPUT-TILE-FORMAL-MEMORY-EVIDENCE/V6\0";
const MLP_TILE_DOMAIN: &[u8] = b"FE2O3/INERT-WAVE-MLP-TILE-FORMAL-MEMORY-EVIDENCE/V2\0";
const ATTENTION_OUTPUT_DOMAIN: &[u8] =
    b"FE2O3/INERT-WAVE-QKV-ATTENTION-OUTPUT-TASK-FORMAL-MEMORY-EVIDENCE/V5\0";
/// Independent bounded archive, never an ordinary formal-memory certificate.
pub const MAX_WAVE_TASK_FORMAL_MEMORY_EVIDENCE_BYTES_V1: usize = 4 * 1024 * 1024;
/// The policy retains all obligations and establishes no runtime premise.
pub const WAVE_TASK_FORMAL_MEMORY_EVIDENCE_POLICY_V1: u16 = 1;
/// QKV-specific archive policy; retains every unresolved requirement.
pub const WAVE_QKV_TASK_FORMAL_MEMORY_EVIDENCE_POLICY_V2: u16 = 2;
/// Twelve-root QKV post archive policy; no runtime premise is discharged.
pub const WAVE_QKV_POST_TASK_FORMAL_MEMORY_EVIDENCE_POLICY_V3: u16 = 3;
/// Thirteen-root attention policy; no runtime premise is discharged.
pub const WAVE_QKV_ATTENTION_TASK_FORMAL_MEMORY_EVIDENCE_POLICY_V4: u16 = 4;
/// Fixed fifteen-root schema; retains all unresolved premises without authorizing execution.
pub const WAVE_QKV_ATTENTION_OUTPUT_TASK_FORMAL_MEMORY_EVIDENCE_POLICY_V5: u16 = 5;
/// Fixed eleven-root MLP schema; no unresolved runtime premise is discharged.
pub const WAVE_MLP_TASK_FORMAL_MEMORY_EVIDENCE_POLICY_V1: u16 = 1;
/// Fixed eleven-root tiled MLP schema; no unresolved runtime premise is discharged.
pub const WAVE_MLP_TILE_FORMAL_MEMORY_EVIDENCE_POLICY_V2: u16 = 2;
/// Fixed three-root WG128 schema, retaining all unresolved runtime requirements.
pub const MULTIWAVE_JOIN_FORMAL_MEMORY_EVIDENCE_POLICY_V1: u16 = 1;
/// Fixed fifteen-root prefix-tile schema; no runtime premise is discharged.
pub const WAVE_QKV_ATTENTION_OUTPUT_TILE_FORMAL_MEMORY_EVIDENCE_POLICY_V6: u16 = 6;
type E = WaveTaskFormalMemoryEvidenceErrorV1;
type ResultV1<T> = Result<T, E>;

fn trace_join_work_v1(stage: &str, budget: &Budget<'_>, charge: usize) {
    if std::env::var_os("FE2O3_KIR_JOIN_WORK_DIAGNOSTIC_V1").as_deref()
        == Some(std::ffi::OsStr::new("1"))
    {
        eprintln!(
            "kir-join-work-v1 stage={} work={} remaining={} storage={} peak={} charge={}",
            stage,
            budget.work(),
            budget.remaining_work(),
            budget.storage(),
            budget.peak_storage(),
            charge
        );
    }
}


/// Construction or inert decoding failed without issuing any authority.
#[derive(Debug)]
pub enum WaveTaskFormalMemoryEvidenceErrorV1 {
    /// Checked length overflow or aggregate archive cap exceeded.
    Size,
    /// Host allocation could not retain the precharged exact payload.
    Allocation,
    /// The fixed wire schema, tag, count or reserved field changed.
    Encoding,
    /// Exact domain-separated canonical bytes and their label disagree.
    Identity,
    /// Required source, KIR, raw report or per-operation association changed.
    Correspondence,
    /// The live formal owner failed its existing bounded replay.
    Formal(ProductionWaveTaskFormalMemoryErrorV1),
    /// The complete middle-end parent failed construction or decoding.
    Middle(fe2o3_pliron::ProductionMiddleEndWaveTaskEvidenceErrorV1),
    /// The original lossless formal-obligation receipt was rejected.
    Receipt(kir::FormalMemoryReceiptErrorV1),
    /// The inert published-KIR reader rejected exact canonical bytes.
    KernelIr(kir::KernelIrDecodeError),
    /// Caller-ledger failure, preserving the original resource diagnostic.
    Resource(CanonicalKernelIrVerificationResourceErrorV1),
}
impl From<CanonicalKernelIrVerificationResourceErrorV1> for E {
    fn from(value: CanonicalKernelIrVerificationResourceErrorV1) -> Self {
        Self::Resource(value)
    }
}
impl fmt::Display for E {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "inert wave-task formal evidence rejected: {self:?}")
    }
}
impl std::error::Error for E {}

/// Original source component and its physical entry parameter, not noalias.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InertWaveTaskFormalRootV1 {
    /// Original source argument ordinal.
    pub source_argument: u32,
    /// Original MIR local ordinal holding the carrier.
    pub source_local: u32,
    /// Original semantic pointer type ordinal, not a type-based issuer.
    pub component_type: u32,
    /// Original role in the exact profile's source carrier.
    pub role: Role,
    /// Physical entry parameter ordinal.
    pub parameter_index: u32,
    /// Actual entry parameter value identity.
    pub parameter: kir::ValueId,
    /// Logical role origin only; never physical noalias authority.
    pub allocation_origin: u64,
}

/// Exact memory-census association. The full opcode and pointer type remain in
/// the archived Module and are returned by operation()/pointer_type().
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InertWaveTaskFormalMemoryBindingV1 {
    /// Exact operation in the selected archived function.
    pub location: Location,
    /// Original operation-local modeled-access ordinal.
    pub access_ordinal: u32,
    /// Original pointer SSA value, with its full type in the archived Module.
    pub pointer: kir::ValueId,
    /// Read=0, write=1, atomic read=2, atomic write=3, atomic RMW=4.
    pub access_tag: u8,
    /// Global=0, Workgroup=1, Private=2.
    pub memory_space_tag: u8,
    /// Existing normalized order/scope/optional failure-order metadata.
    pub atomic_contract: Option<(u8, u8, Option<u8>)>,
    /// Original semantic block, optional statement and unfiltered ordinal.
    pub source_site: Option<(u32, Option<u32>, u32)>,
    /// Original ranked block and operation, absent only for Private rows.
    pub ranked_site: Option<(u32, u32)>,
}
/// Exact actual synchronization operation and its ranked association.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InertWaveTaskFormalSyncBindingV1 {
    /// Original selected-function operation, preserving full synchronization.
    pub location: Location,
    /// Original ranked block and operation.
    pub ranked_site: (u32, u32),
}

/// All raw incomplete reasons, including variants outside the currently
/// admitted worker subset. IDs remain inert ordinals; no issuer is fabricated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InertWaveTaskFormalReasonV1<'a> {
    /// The extractor did not admit the retained index width.
    UnsupportedIndexWidth(kir::FormalIndexWidth),
    /// No exact launch extent was available.
    LaunchExtentUnknown,
    /// The retained launch extent contains zero.
    LaunchExtentZero,
    /// The extractor did not support this raw rank.
    LaunchRankUnsupported(u8),
    /// The domain rank and supplied extent rank differ.
    LaunchRankMismatch {
        #[doc = "Original launch-domain rank."]
        domain_rank: u8,
        #[doc = "Original supplied extent rank."]
        extent_rank: u8,
    },
    /// Unused axes did not have the required shape.
    LaunchExtentShapeMismatch {
        #[doc = "Original rank, including unsupported values."]
        rank: u8,
        #[doc = "All original extents, without normalization."]
        extents: [u64; 3],
    },
    /// The retained extent product overflowed.
    LaunchExtentOverflow {
        #[doc = "Original rank, including unsupported values."]
        rank: u8,
        #[doc = "All original extents, without normalization."]
        extents: [u64; 3],
    },
    /// A static one-dimensional extent differed.
    StaticLaunchExtentMismatch {
        #[doc = "Original static expected extent."]
        expected: u32,
        #[doc = "Original supplied extent."]
        actual: u64,
    },
    /// A static extent differed on one named axis.
    StaticLaunchAxisExtentMismatch {
        #[doc = "Original named axis."]
        axis: kir::Axis,
        #[doc = "Original static expected extent."]
        expected: u32,
        #[doc = "Original supplied extent."]
        actual: u64,
    },
    /// The extractor lacked complete effects for this call.
    CallEffectsUnavailable {
        #[doc = "Exact original operation location."]
        location: Location,
        #[doc = "Original unavailable callee identifier."]
        callee: &'a str,
    },
    /// An exact operation had an unsupported memory effect.
    UnsupportedMemoryEffect(Location),
    /// A guarded access requires an unresolved ranked proof.
    GuardedAccessRequiresRankedProof(Location),
    /// The entry block carried unsupported parameters.
    UnsupportedEntryBlockParameters(kir::BlockId),
    /// The extractor could not resolve this pointer.
    UnsupportedPointerDerivation {
        #[doc = "Exact original operation location."]
        location: Location,
        #[doc = "Original pointer SSA value."]
        pointer: kir::ValueId,
    },
    /// The extractor could not represent this index expression.
    UnsupportedIndexExpression {
        #[doc = "Exact original operation location."]
        location: Location,
        #[doc = "Original index SSA value."]
        index: kir::ValueId,
        #[doc = "Original formal allocation parameter ordinal."]
        allocation: u32,
    },
    /// An exact pointer had no available element width.
    ElementWidthUnavailable {
        #[doc = "Exact original operation location."]
        location: Location,
        #[doc = "Original pointer SSA value."]
        pointer: kir::ValueId,
    },
    /// Address arithmetic overflowed at this operation.
    AddressArithmeticOverflow(Location),
}
/// An original incomplete diagnostic and any exact operation it names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InertWaveTaskFormalReasonRecordV1<'a> {
    /// Complete typed raw diagnostic, not a successful discharge.
    pub reason: InertWaveTaskFormalReasonV1<'a>,
    /// Original operation for location-bearing diagnostics.
    pub covered_operation: Option<Location>,
}

/// Move-only inert content. Decoding reconstructs no verified/live owner.
#[derive(Debug, Eq, PartialEq)]
struct WaveArchiveCoreV2<const N: usize = 6> {
    profile: Profile,
    bytes: Box<[u8]>,
    identity: [u8; 32],
    kir_identity: KirIdentity,
    middle: InertMiddle,
    module: kir::Module,
    function: usize,
    status_complete: bool,
    rank: u8,
    global: [u64; 3],
    workgroup: [u64; 3],
    roots: [InertWaveTaskFormalRootV1; N],
    receipt_range: Range<usize>,
    receipt_identity: [u8; 32],
    kir_range: Range<usize>,
    reasons: Range<usize>,
    reason_count: usize,
    allocations: Range<usize>,
    allocation_count: usize,
    memory: Range<usize>,
    memory_count: usize,
    sync: Range<usize>,
    sync_count: usize,
    retained_storage: usize,
}

impl<const N: usize> WaveArchiveCoreV2<N> {
    /// Replays the live formal owner and complete parent report under one
    /// caller ledger. Incoming reservations must cover both borrowed owners.
    /// Success retains only this archive's additional payload; errors restore
    /// the incoming floor without refunding accepted work.
    pub fn from_live_owner(
        formal: &Formal,
        middle: &Middle,
        budget: &mut Budget<'_>,
    ) -> ResultV1<Self> {
        let floor = budget.storage();
        let result = (|| {
            if N != formal.profile().roles().len() {
                return Err(E::Correspondence);
            }
            if floor < add(formal.retained_storage(), middle.retained_storage())? {
                return Err(E::Resource(
                    CanonicalKernelIrVerificationResourceErrorV1::Accounting,
                ));
            }
            require_formal_roots(formal)?;
            formal
                .verify_equivalence_with_budget(budget)
                .map_err(E::Formal)?;
            let [checks] = formal.semantic_kir().generic_checks.as_ref() else {
                return Err(E::Correspondence);
            };
            let ranked = checks
                .lowering
                .conditional_wave_tasks()
                .ok_or(E::Correspondence)?;
            let replay = Middle::try_new(
                formal.semantic_kir().semantic(),
                ranked,
                middle.as_inert().ranked_ir(),
                budget,
            )
            .map_err(E::Middle)?;
            let replay_storage = replay.retained_storage();
            let compared = budget
                .charge_work(replay.canonical_bytes().len())
                .map_err(E::Resource)
                .and_then(|()| {
                    if replay.canonical_bytes() == middle.canonical_bytes() {
                        Ok(())
                    } else {
                        Err(E::Correspondence)
                    }
                });
            drop(replay);
            budget.release_storage(replay_storage)?;
            compared?;
            if formal.unresolved_requirements() != middle.as_inert().unresolved_requirements()
                || ranked.function_ordinal() != middle.as_inert().function_ordinal() as usize
            {
                return Err(E::Correspondence);
            }

            let receipt_len = raw_receipt_length(formal.raw_obligations())?;
            let receipt_frame = receipt_frame(receipt_len)?;
            budget.charge_work(receipt_work(receipt_len)?)?;
            budget.reserve_storage(receipt_frame)?;
            let result = (|| {
                let receipt =
                    RawReceipt::from_obligations(formal.raw_obligations()).map_err(E::Receipt)?;
                if receipt.canonical_bytes().len() != receipt_len {
                    return Err(E::Encoding);
                }
                // Charge before sizing/traversing variable reasons and exact
                // operation equality. Existing canonical bytes bound payloads.
                let source_bytes = formal.semantic_kir().canonical_kernel_ir_bytes().len();
                budget.charge_work(mul(add(source_bytes, receipt_len)?, 16)?)?;
                validate_live_indices(formal, ranked, budget)?;
                let mut size = Writer::sizing();
                encode_live(&mut size, formal, middle, ranked, &receipt)?;
                let length = add(size.len, 32)?;
                bounded(length)?;
                budget.charge_work(mul(length, 4)?)?;
                budget.reserve_storage(length)?;
                let encoded = (|| {
                    let mut writer = Writer::owning(length)?;
                    encode_live(&mut writer, formal, middle, ranked, &receipt)?;
                    let digest = archive_identity_for_profile(writer.as_bytes()?, formal.profile());
                    writer.bytes(&digest)?;
                    let bytes = writer.finish()?;
                    let archive = Self::decode_with_budget(&bytes, formal.profile(), budget)?;
                    let comparison = budget
                        .charge_work(mul(source_bytes, 4)?)
                        .map_err(E::Resource)
                        .and_then(|()| {
                            if &archive.module == formal.semantic_kir().module() {
                                Ok(())
                            } else {
                                Err(E::Correspondence)
                            }
                        });
                    if let Err(error) = comparison {
                        let retained = archive.retained_storage();
                        drop(archive);
                        budget.release_storage(retained)?;
                        return Err(error);
                    }
                    Ok(archive)
                })();
                budget.release_storage(length)?;
                encoded
            })();
            budget.release_storage(receipt_frame)?;
            result
        })();
        if result.is_err() {
            release_to(budget, floor)?;
        }
        result
    }

    /// Standalone inert decoding has fixed archive-specific resource limits.
    /// Connected compiler callers use decode_with_budget instead.
    pub fn decode(bytes: &[u8], profile: Profile) -> ResultV1<Self> {
        let mut work = kir::CanonicalKernelIrWorkBudgetV1::new(256 * 1024 * 1024);
        let mut budget = Budget::new(&mut work, 128 * 1024 * 1024);
        Self::decode_with_budget(bytes, profile, &mut budget)
    }
    /// Validates and retains inert content under the existing caller ledger.
    pub fn decode_with_budget(
        bytes: &[u8],
        profile: Profile,
        budget: &mut Budget<'_>,
    ) -> ResultV1<Self> {
        let floor = budget.storage();
        let result = (|| {
            bounded(bytes.len())?;
            let charge = mul(bytes.len(), 16)?;
            trace_join_work_v1("formal-archive-before", budget, charge);
            budget.charge_work(charge)?;
            trace_join_work_v1("formal-archive-after", budget, 0);
            let layout = parse_layout::<N>(bytes, profile)?;
            let raw = &bytes[layout.receipt.clone()];
            let frame = receipt_frame(raw.len())?;
            let charge = receipt_work(raw.len())?;
            trace_join_work_v1("formal-receipt-before", budget, charge);
            budget.charge_work(charge)?;
            trace_join_work_v1("formal-receipt-after", budget, 0);
            budget.reserve_storage(frame)?;
            let raw_result: ResultV1<_> = (|| {
                let copy = copy_bytes(raw)?;
                let receipt = RawReceipt::from_canonical_bytes(copy).map_err(E::Receipt)?;
                Ok((
                    *receipt.identity().digest(),
                    receipt.kernel_id().len(),
                    receipt.entry_id().len(),
                ))
            })();
            budget.release_storage(frame)?;
            let (receipt_identity, _, _) = raw_result?;
            let kir_bytes = &bytes[layout.kir.clone()];
            trace_join_work_v1("formal-kir-before", budget, 0);
            let (module, module_storage) =
                kir::decode_inert_published_kernel_ir_with_budget_v1(kir_bytes, budget)
                    .map_err(E::KernelIr)?;
            trace_join_work_v1("formal-kir-after", budget, 0);
            budget.reserve_storage(module_storage.retained_storage())?;
            let kir_identity = published_identity(kir_bytes)?;
            if kir_identity != layout.kir_identity {
                return Err(E::Identity);
            }
            trace_join_work_v1("formal-middle-before", budget, 0);
            let middle = InertMiddle::decode_with_budget(&bytes[layout.middle.clone()], budget)
                .map_err(E::Middle)?;
            trace_join_work_v1("formal-middle-after", budget, 0);
            if middle.function_ordinal() != layout.function {
                return Err(E::Correspondence);
            }
            trace_join_work_v1("formal-layout-before", budget, 0);
            validate_layout_module(bytes, &layout, &module, budget)?;
            trace_join_work_v1("formal-layout-after", budget, 0);
            let own = add(bytes.len(), size_of::<Self>())?;
            budget.reserve_storage(own)?;
            let bytes = copy_bytes(bytes)?.into_boxed_slice();
            let retained_storage = budget.storage().checked_sub(floor).ok_or(E::Size)?;
            Ok(Self {
                profile,
                bytes,
                identity: layout.identity,
                kir_identity,
                middle,
                module,
                function: layout.function as usize,
                status_complete: layout.status_complete,
                rank: layout.rank,
                global: layout.global,
                workgroup: layout.workgroup,
                roots: layout.roots,
                receipt_range: layout.receipt,
                receipt_identity,
                kir_range: layout.kir,
                reasons: layout.reasons,
                reason_count: layout.reason_count,
                allocations: layout.allocations,
                allocation_count: layout.allocation_count,
                memory: layout.memory,
                memory_count: layout.memory_count,
                sync: layout.sync,
                sync_count: layout.sync_count,
                retained_storage,
            })
        })();
        if result.is_err() {
            release_to(budget, floor)?;
        }
        result
    }
    /// Returns the exact complete canonical archive bytes.
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Returns this archive's domain-separated inert content digest.
    pub const fn identity(&self) -> &[u8; 32] {
        &self.identity
    }
    /// Returns the published V8/V9/V11 identity, independent of pre-ranked V12.
    pub const fn canonical_kernel_ir_identity(&self) -> KirIdentity {
        self.kir_identity
    }
    /// Borrows all three raw report archives and independent source/ranked/V12 labels.
    pub const fn middle_end_evidence(&self) -> &InertMiddle {
        &self.middle
    }
    /// Returns the retained actual launch rank.
    pub const fn rank(&self) -> u8 {
        self.rank
    }
    /// Returns the actual global invocation extents, not a fixed finite-join shape.
    pub const fn witness_extents(&self) -> [u64; 3] {
        self.global
    }
    /// Returns the actual workgroup extents.
    pub const fn workgroup_extents(&self) -> [u64; 3] {
        self.workgroup
    }
    /// Returns all six original entry roots, separately from Private/LDS allocations.
    pub const fn roots(&self) -> &[InertWaveTaskFormalRootV1; N] {
        &self.roots
    }
    /// Reports extraction completeness only, never runtime safety.
    pub const fn raw_analysis_is_complete(&self) -> bool {
        self.status_complete
    }
    /// Returns all additional logical payload retained by this inert record.
    pub const fn retained_storage(&self) -> usize {
        self.retained_storage
    }
    /// Returns all eight retained, undischarged wave-task requirements.
    pub const fn unresolved_requirements(&self) -> &[Requirement; 8] {
        self.middle.unresolved_requirements()
    }
    /// Returns every raw allocation, access, bound, alias and conflict record.
    pub fn formal_obligation_receipt_bytes(&self) -> &[u8] {
        &self.bytes[self.receipt_range.clone()]
    }
    /// Returns the exact nested raw-obligation receipt identity.
    pub const fn formal_obligation_receipt_identity(&self) -> &[u8; 32] {
        &self.receipt_identity
    }
    /// Returns complete published KIR bytes including original operation payloads.
    pub fn canonical_kernel_ir_bytes(&self) -> &[u8] {
        &self.bytes[self.kir_range.clone()]
    }
    /// The plain Module is inert wire content, not a verified-module token.
    pub const fn module(&self) -> &kir::Module {
        &self.module
    }
    /// Borrows an exact original operation from the inert selected function.
    pub fn operation(&self, location: Location) -> Option<&Operation> {
        operation_at(&self.module, self.function, location).ok()
    }
    /// Borrows the original SSA pointer type from the inert selected function.
    pub fn pointer_type(&self, pointer: kir::ValueId) -> Option<&kir::Type> {
        let function = self.module.functions.get(self.function)?;
        pointer_type(function, pointer).ok()
    }
    /// Iterates complete raw incomplete-analysis diagnostics in original order.
    pub fn reasons(&self) -> impl ExactSizeIterator<Item = InertWaveTaskFormalReasonRecordV1<'_>> {
        let mut reader = Reader::new(&self.bytes[self.reasons.clone()]);
        (0..self.reason_count)
            .map(move |_| read_reason_record(&mut reader).expect("validated reason record"))
    }
    /// Iterates every original Private/LDS allocation location.
    pub fn internal_allocations(&self) -> impl ExactSizeIterator<Item = Location> + '_ {
        let mut reader = Reader::new(&self.bytes[self.allocations.clone()]);
        (0..self.allocation_count)
            .map(move |_| reader.location().expect("validated allocation location"))
    }
    /// Iterates complete ordered memory associations, including Private rows.
    pub fn memory_census(
        &self,
    ) -> impl ExactSizeIterator<Item = InertWaveTaskFormalMemoryBindingV1> + '_ {
        let mut reader = Reader::new(&self.bytes[self.memory.clone()]);
        (0..self.memory_count)
            .map(move |_| read_memory(&mut reader).expect("validated memory record"))
    }
    /// Iterates complete ordered synchronization associations.
    pub fn synchronization_bindings(
        &self,
    ) -> impl ExactSizeIterator<Item = InertWaveTaskFormalSyncBindingV1> + '_ {
        let mut reader = Reader::new(&self.bytes[self.sync.clone()]);
        (0..self.sync_count).map(move |_| read_sync(&mut reader).expect("validated sync record"))
    }
    /// Always false: neither construction nor decoding grants live authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
    /// Always false: raw diagnostics do not establish compiler refinement.
    pub const fn grants_compiler_refinement_authority(&self) -> bool {
        false
    }
    /// Always false: this content never authorizes an artifact or launch.
    pub const fn grants_artifact_or_launch_authority(&self) -> bool {
        false
    }
}

fn add(a: usize, b: usize) -> ResultV1<usize> {
    a.checked_add(b).ok_or(E::Size)
}
fn mul(a: usize, b: usize) -> ResultV1<usize> {
    a.checked_mul(b).ok_or(E::Size)
}
fn bounded(n: usize) -> ResultV1<()> {
    if n <= MAX_WAVE_TASK_FORMAL_MEMORY_EVIDENCE_BYTES_V1 {
        Ok(())
    } else {
        Err(E::Size)
    }
}
fn release_to(budget: &mut Budget<'_>, floor: usize) -> ResultV1<()> {
    let added = budget.storage().checked_sub(floor).ok_or(E::Size)?;
    budget.release_storage(added)?;
    Ok(())
}
fn copy_bytes(bytes: &[u8]) -> ResultV1<Vec<u8>> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len())
        .map_err(|_| E::Allocation)?;
    if copy.capacity() != bytes.len() {
        return Err(E::Allocation);
    }
    copy.extend_from_slice(bytes);
    Ok(copy)
}
fn archive_identity_for_profile(bytes: &[u8], profile: Profile) -> [u8; 32] {
    let domain = match profile {
        Profile::KeyV1 => DOMAIN,
        Profile::QkvV2 => QKV_DOMAIN,
        Profile::QkvPostV3 => POST_DOMAIN,
        Profile::QkvAttentionV4 => ATTENTION_DOMAIN,
        Profile::QkvAttentionOutputV5 => ATTENTION_OUTPUT_DOMAIN,
        Profile::MlpV1 => MLP_DOMAIN,
        Profile::MlpTilesV2 => MLP_TILE_DOMAIN,
        Profile::MultiwaveJoinV1 => MULTIWAVE_DOMAIN,
        Profile::QkvAttentionOutputTilesV6 => ATTENTION_OUTPUT_TILE_DOMAIN,
    };
    let mut hash = Sha256::new();
    hash.update((domain.len() as u32).to_le_bytes());
    hash.update(domain);
    hash.update(bytes);
    hash.finalize().into()
}
fn published_identity(bytes: &[u8]) -> ResultV1<KirIdentity> {
    let version = bytes.get(8..10).ok_or(E::Encoding)?;
    let (version, domain, policy) = match u16::from_le_bytes([version[0], version[1]]) {
        8 => (
            KirVersion::V8,
            kir::VERIFIED_CANONICAL_KERNEL_IR_V8_IDENTITY_DOMAIN_V1,
            kir::VERIFIED_CANONICAL_KERNEL_IR_V8_IDENTITY_POLICY_V1,
        ),
        9 => (
            KirVersion::V9,
            kir::VERIFIED_CANONICAL_KERNEL_IR_V9_IDENTITY_DOMAIN_V1,
            kir::VERIFIED_CANONICAL_KERNEL_IR_V9_IDENTITY_POLICY_V1,
        ),
        11 => (
            KirVersion::V11,
            kir::VERIFIED_CANONICAL_KERNEL_IR_V11_IDENTITY_DOMAIN_V1,
            kir::VERIFIED_CANONICAL_KERNEL_IR_V11_IDENTITY_POLICY_V1,
        ),
        _ => return Err(E::Encoding),
    };
    let mut hash = Sha256::new();
    hash.update((domain.len() as u32).to_le_bytes());
    hash.update(domain);
    hash.update(policy.to_le_bytes());
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
    Ok(KirIdentity::from_canonical_parts(
        version,
        hash.finalize().into(),
        bytes.len() as u64,
    ))
}

include!("wave_formal_evidence_codec_v1.rs");
include!("wave_formal_evidence_join_v1.rs");
include!("wave_formal_evidence_profiles_v2.rs");
#[cfg(test)]
#[path = "wave_formal_evidence_v1_tests.rs"]
mod tests;
