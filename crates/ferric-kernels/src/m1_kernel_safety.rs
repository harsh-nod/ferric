//! Executable admission for caller-supplied M1 K1-K7 safety certificates.
//!
//! The certificate describes a complete finite source-level schedule, buffer
//! roster, initialized input ranges, and modeled byte effects. This module
//! validates that supplied model; it does not derive effects from kernel
//! source. Success remains conditional on the exact compiler, runtime, target,
//! ABI, proof-set, and TCB identities retained by the structural catalog.
//! Those identities are framing premises, not authentication. No result here
//! grants artifact, compilation, load, launch, GPU, numerical, completion,
//! performance, qualification, or hardware-correctness authority.

use crate::{
    validate_kernel_catalog_input, KernelCatalogInput, KernelCatalogValidationError, KernelFamily,
    ValidatedKernelCatalogInput,
};
use vstd::prelude::*;

verus! {

/// Maximum caller-supplied modeled accesses in one certificate.
pub const M1_KERNEL_MAX_MODELED_ACCESSES_V1: usize = 4_096;

/// Maximum caller-supplied initialized-range records in one certificate.
pub const M1_KERNEL_MAX_INITIALIZED_RANGES_V1: usize = 4_096;

/// Direction of one modeled half-open byte range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1KernelModeledAccessModeV1 {
    /// The workitem only observes the range.
    ReadOnly,
    /// The workitem initializes or replaces the range.
    WriteOnly,
    /// The workitem observes and replaces the range.
    ReadWrite,
}

/// One modeled half-open access by one workitem in one source-level phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1KernelModeledAccessV1 {
    /// Zero-based modeled phase.
    pub phase: u32,
    /// Zero-based flattened modeled workitem.
    pub workitem: u32,
    /// Zero-based logical buffer.
    pub buffer: u16,
    /// Inclusive byte offset.
    pub start: u64,
    /// Exclusive byte offset.
    pub end: u64,
    /// Read/write direction.
    pub mode: M1KernelModeledAccessModeV1,
}

/// One caller-supplied initialized half-open byte range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1KernelInitializedRangeV1 {
    /// Zero-based logical buffer.
    pub buffer: u16,
    /// Inclusive byte offset.
    pub start: u64,
    /// Exclusive byte offset.
    pub end: u64,
}

/// Complete finite source-level certificate candidate.
pub struct M1KernelSafetyCertificateInputV1<'a> {
    /// Previously validated inert structural-catalog metadata.
    pub catalog: &'a ValidatedKernelCatalogInput,
    /// Exact independently supplied catalog expectation.
    pub expected: KernelCatalogInput,
    /// Flattened modeled workitem count.
    pub workitems: u32,
    /// Decreasing source-level control rank in phase order.
    pub ranks: &'a [u32],
    /// Logical byte extent of every modeled buffer.
    pub buffer_extents: &'a [u64],
    /// Complete caller-supplied modeled access roster.
    pub accesses: &'a [M1KernelModeledAccessV1],
    /// Complete caller-supplied initialized input-range roster.
    pub initialized_before: &'a [M1KernelInitializedRangeV1],
}

/// Fail-closed source-level certificate rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1KernelSafetyCertificateErrorV1 {
    /// Structural catalog metadata no longer matches the exact expectation.
    Catalog(KernelCatalogValidationError),
    /// The modeled workitem count is zero or above its family bound.
    ResourceWorkitems,
    /// The modeled control-rank roster is empty, too large, or not decreasing.
    ResourceControlRanks,
    /// Checked access-limit arithmetic overflowed.
    ResourceArithmeticOverflow,
    /// A finite input roster exceeds its explicit admitted bound.
    ResourceRosterBound,
    /// The logical buffer roster is empty, too large, or contains zero extent.
    ResourceBufferRoster,
    /// An initialized range is malformed or outside its logical buffer.
    MemoryInitializedRange,
    /// Initialized ranges overlap, duplicate, or ambiguously cover one buffer.
    MemoryAmbiguousInitialization,
    /// A modeled access is malformed or outside its phase/workitem/buffer bound.
    MemoryAccessRange,
    /// A modeled read is not covered by exactly one initialized input range.
    MemoryUninitializedRead,
    /// Distinct workitems have a same-phase overlapping access where one writes.
    RaceConflictingAccess,
}

/// One admitted source-level K1-K7 safety certificate.
///
/// This owner deliberately does not implement `Clone` or expose its catalog
/// premise as artifact or execution authority.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ValidatedM1KernelSafetyCertificateV1<'a> {
    candidate: KernelCatalogInput,
    expected: KernelCatalogInput,
    workitems: u32,
    ranks: &'a [u32],
    buffer_extents: &'a [u64],
    accesses: &'a [M1KernelModeledAccessV1],
    initialized_before: &'a [M1KernelInitializedRangeV1],
}

/// Conservative maximum flattened workitem count for one family certificate.
pub open spec fn m1_family_workitem_bound_spec(family: KernelFamily) -> nat {
    match family {
        KernelFamily::K1GemmGemv => 77_791_232,
        KernelFamily::K2RmsNormResidual | KernelFamily::K4GqaPrefill => 4_194_304,
        KernelFamily::K3RopePagedKv | KernelFamily::K7LogitsCompact => 131_072,
        KernelFamily::K5PagedGqaDecode => 81_920,
        KernelFamily::K6SwiGlu => 3_145_728,
    }
}

/// Conservative maximum decreasing source-level control rank by family.
pub open spec fn m1_family_control_rank_bound_spec(family: KernelFamily) -> nat {
    match family {
        KernelFamily::K1GemmGemv => 3_072,
        KernelFamily::K2RmsNormResidual => 64,
        KernelFamily::K3RopePagedKv
        | KernelFamily::K4GqaPrefill
        | KernelFamily::K5PagedGqaDecode => 8_192,
        KernelFamily::K6SwiGlu => 8,
        KernelFamily::K7LogitsCompact => 2_374,
    }
}

fn family_workitem_bound(family: KernelFamily) -> (bound: u32)
    ensures bound as nat == m1_family_workitem_bound_spec(family),
{
    match family {
        KernelFamily::K1GemmGemv => 77_791_232,
        KernelFamily::K2RmsNormResidual | KernelFamily::K4GqaPrefill => 4_194_304,
        KernelFamily::K3RopePagedKv | KernelFamily::K7LogitsCompact => 131_072,
        KernelFamily::K5PagedGqaDecode => 81_920,
        KernelFamily::K6SwiGlu => 3_145_728,
    }
}

fn family_control_rank_bound(family: KernelFamily) -> (bound: u32)
    ensures bound as nat == m1_family_control_rank_bound_spec(family),
{
    match family {
        KernelFamily::K1GemmGemv => 3_072,
        KernelFamily::K2RmsNormResidual => 64,
        KernelFamily::K3RopePagedKv
        | KernelFamily::K4GqaPrefill
        | KernelFamily::K5PagedGqaDecode => 8_192,
        KernelFamily::K6SwiGlu => 8,
        KernelFamily::K7LogitsCompact => 2_374,
    }
}

/// Whether one modeled access observes its range.
pub open spec fn m1_modeled_access_reads(access: M1KernelModeledAccessV1) -> bool {
    access.mode == M1KernelModeledAccessModeV1::ReadOnly
        || access.mode == M1KernelModeledAccessModeV1::ReadWrite
}

/// Whether one modeled access replaces its range.
pub open spec fn m1_modeled_access_writes(access: M1KernelModeledAccessV1) -> bool {
    access.mode == M1KernelModeledAccessModeV1::WriteOnly
        || access.mode == M1KernelModeledAccessModeV1::ReadWrite
}

/// Whether two half-open ranges overlap in one logical buffer.
pub open spec fn m1_modeled_accesses_overlap(
    left: M1KernelModeledAccessV1,
    right: M1KernelModeledAccessV1,
) -> bool {
    &&& left.buffer == right.buffer
    &&& left.start < right.end
    &&& right.start < left.end
}

pub open spec fn initialized_ranges_overlap(
    left: M1KernelInitializedRangeV1,
    right: M1KernelInitializedRangeV1,
) -> bool {
    &&& left.buffer == right.buffer
    &&& left.start < right.end
    &&& right.start < left.end
}

pub open spec fn initialized_range_is_in_bounds(
    extents: Seq<u64>,
    range: M1KernelInitializedRangeV1,
) -> bool {
    &&& (range.buffer as nat) < extents.len()
    &&& range.start < range.end
    &&& range.end <= extents[range.buffer as int]
}

pub open spec fn access_is_in_bounds(
    workitems: u32,
    phases: nat,
    extents: Seq<u64>,
    access: M1KernelModeledAccessV1,
) -> bool {
    &&& (access.phase as nat) < phases
    &&& access.workitem < workitems
    &&& (access.buffer as nat) < extents.len()
    &&& access.start < access.end
    &&& access.end <= extents[access.buffer as int]
}

pub open spec fn initialized_range_covers_access(
    range: M1KernelInitializedRangeV1,
    access: M1KernelModeledAccessV1,
) -> bool {
    &&& range.buffer == access.buffer
    &&& range.start <= access.start
    &&& access.end <= range.end
}

pub open spec fn catalog_premise(input: &M1KernelSafetyCertificateInputV1<'_>) -> bool {
    crate::validation::kernel_catalog_input_matches_exactly(
        input.catalog.input_spec(),
        input.expected,
    )
}

/// Exact source-level resource-bounds success predicate.
pub open spec fn m1_kernel_resource_bounded(
    input: &M1KernelSafetyCertificateInputV1<'_>,
) -> bool {
    &&& catalog_premise(input)
    &&& 0 < input.workitems as nat
    &&& input.workitems as nat
        <= m1_family_workitem_bound_spec(input.expected.profile.family)
    &&& 0 < input.ranks@.len()
    &&& input.ranks@.len()
        <= m1_family_control_rank_bound_spec(input.expected.profile.family) + 1
    &&& forall|index: int| 0 <= index < input.ranks@.len()
        ==> input.ranks@[index] as nat == input.ranks@.len() - 1 - index
    &&& 0 < input.buffer_extents@.len() <= 16
    &&& forall|index: int| 0 <= index < input.buffer_extents@.len()
        ==> input.buffer_extents@[index] > 0
    &&& input.accesses@.len()
        <= (input.workitems as nat) * input.ranks@.len() * 8
    &&& input.accesses@.len() <= M1_KERNEL_MAX_MODELED_ACCESSES_V1
    &&& input.initialized_before@.len() <= M1_KERNEL_MAX_INITIALIZED_RANGES_V1
}

/// Exact initialized-range and access-range structural predicate.
pub open spec fn m1_kernel_memory_ranges_safe(
    input: &M1KernelSafetyCertificateInputV1<'_>,
) -> bool {
    &&& forall|index: int| 0 <= index < input.initialized_before@.len()
        ==> initialized_range_is_in_bounds(
            input.buffer_extents@,
            input.initialized_before@[index],
        )
    &&& forall|left: int, right: int|
        0 <= left < right < input.initialized_before@.len()
            ==> !initialized_ranges_overlap(
                input.initialized_before@[left],
                input.initialized_before@[right],
            )
    &&& forall|index: int| 0 <= index < input.accesses@.len() ==> {
        let access = #[trigger] input.accesses@[index];
        access_is_in_bounds(
            input.workitems,
            input.ranks@.len(),
            input.buffer_extents@,
            access,
        )
    }
}

/// Exact initialized-read coverage predicate.
pub open spec fn m1_kernel_reads_are_initialized(
    input: &M1KernelSafetyCertificateInputV1<'_>,
) -> bool {
    forall|index: int| 0 <= index < input.accesses@.len() ==> {
        let access = #[trigger] input.accesses@[index];
        m1_modeled_access_reads(access) ==> exists|range: int|
            0 <= range < input.initialized_before@.len()
                && initialized_range_covers_access(
                    input.initialized_before@[range],
                    access,
                )
    }
}

/// Exact source-level initialized/in-bounds memory success predicate.
pub open spec fn m1_kernel_memory_safe(
    input: &M1KernelSafetyCertificateInputV1<'_>,
) -> bool {
    &&& catalog_premise(input)
    &&& m1_kernel_memory_ranges_safe(input)
    &&& m1_kernel_reads_are_initialized(input)
}

/// Exact modeled-access race-freedom predicate without catalog framing.
pub open spec fn m1_kernel_access_roster_race_free(
    input: &M1KernelSafetyCertificateInputV1<'_>,
) -> bool {
    forall|left: int, right: int| 0 <= left < right < input.accesses@.len()
        ==> {
            let left_access = #[trigger] input.accesses@[left];
            let right_access = #[trigger] input.accesses@[right];
            left_access.phase != right_access.phase
                || left_access.workitem == right_access.workitem
                || (!m1_modeled_access_writes(left_access)
                    && !m1_modeled_access_writes(right_access))
                || !m1_modeled_accesses_overlap(left_access, right_access)
        }
}

/// Exact source-level same-phase data-race-freedom success predicate.
pub open spec fn m1_kernel_race_free(
    input: &M1KernelSafetyCertificateInputV1<'_>,
) -> bool {
    &&& catalog_premise(input)
    &&& m1_kernel_access_roster_race_free(input)
}

impl ValidatedM1KernelSafetyCertificateV1<'_> {
    /// Exposes the admitted resource-bounds predicate to proof composition.
    pub closed spec fn resource_bounded_spec(&self) -> bool {
        crate::validation::kernel_catalog_input_matches_exactly(self.candidate, self.expected)
            && 0 < self.workitems as nat
            && self.workitems as nat
                <= m1_family_workitem_bound_spec(self.expected.profile.family)
            && 0 < self.ranks@.len()
            && self.ranks@.len()
                <= m1_family_control_rank_bound_spec(self.expected.profile.family) + 1
            && forall|index: int| 0 <= index < self.ranks@.len()
                ==> self.ranks@[index] as nat == self.ranks@.len() - 1 - index
            && 0 < self.buffer_extents@.len() <= 16
            && forall|index: int| 0 <= index < self.buffer_extents@.len()
                ==> self.buffer_extents@[index] > 0
            && self.accesses@.len()
                <= (self.workitems as nat) * self.ranks@.len() * 8
            && self.accesses@.len() <= M1_KERNEL_MAX_MODELED_ACCESSES_V1
            && self.initialized_before@.len() <= M1_KERNEL_MAX_INITIALIZED_RANGES_V1
    }

    /// Exposes the admitted initialized/in-bounds predicate to proof composition.
    pub closed spec fn memory_safe_spec(&self) -> bool {
        crate::validation::kernel_catalog_input_matches_exactly(self.candidate, self.expected)
            && forall|index: int| 0 <= index < self.initialized_before@.len()
                ==> initialized_range_is_in_bounds(
                    self.buffer_extents@,
                    self.initialized_before@[index],
                )
            && forall|left: int, right: int|
                0 <= left < right < self.initialized_before@.len()
                    ==> !initialized_ranges_overlap(
                        self.initialized_before@[left],
                        self.initialized_before@[right],
                    )
            && forall|index: int| 0 <= index < self.accesses@.len() ==> {
                let access = #[trigger] self.accesses@[index];
                &&& access_is_in_bounds(
                    self.workitems,
                    self.ranks@.len(),
                    self.buffer_extents@,
                    access,
                )
                &&& m1_modeled_access_reads(access) ==> exists|range: int|
                    0 <= range < self.initialized_before@.len()
                        && initialized_range_covers_access(
                            self.initialized_before@[range],
                            access,
                        )
            }
    }

    /// Exposes the admitted modeled race-freedom predicate to proof composition.
    pub closed spec fn race_free_spec(&self) -> bool {
        crate::validation::kernel_catalog_input_matches_exactly(self.candidate, self.expected)
            && forall|left: int, right: int| 0 <= left < right < self.accesses@.len()
                ==> {
                    let left_access = #[trigger] self.accesses@[left];
                    let right_access = #[trigger] self.accesses@[right];
                    left_access.phase != right_access.phase
                        || left_access.workitem == right_access.workitem
                        || (!m1_modeled_access_writes(left_access)
                            && !m1_modeled_access_writes(right_access))
                        || !m1_modeled_accesses_overlap(left_access, right_access)
                }
    }
}

fn validate_catalog_premise(
    input: &M1KernelSafetyCertificateInputV1<'_>,
) -> (result: Result<(), M1KernelSafetyCertificateErrorV1>)
    ensures result.is_ok() ==> catalog_premise(input),
{
    let candidate = *input.catalog.input();
    match validate_kernel_catalog_input(candidate, input.expected) {
        Ok(_validated) => Ok(()),
        Err(error) => Err(M1KernelSafetyCertificateErrorV1::Catalog(error)),
    }
}

/// Checks the exact finite K1-K7 schedule and certificate roster bounds.
///
/// # Errors
///
/// Rejects catalog drift, zero or excessive work, malformed decreasing ranks,
/// checked access-limit overflow, an excessive access/initialization roster,
/// or an empty/oversized/zero-extent buffer roster.
pub fn validate_m1_kernel_resource_bounds(
    input: &M1KernelSafetyCertificateInputV1<'_>,
) -> (result: Result<(), M1KernelSafetyCertificateErrorV1>)
    ensures result.is_ok() ==> m1_kernel_resource_bounded(input),
{
    validate_catalog_premise(input)?;
    let workitem_bound = family_workitem_bound(input.expected.profile.family);
    if input.workitems == 0 || input.workitems > workitem_bound {
        return Err(M1KernelSafetyCertificateErrorV1::ResourceWorkitems);
    }
    let rank_bound = family_control_rank_bound(input.expected.profile.family);
    let rank_bound = match usize::try_from(rank_bound) {
        Ok(bound) => bound,
        Err(_) => {
            return Err(M1KernelSafetyCertificateErrorV1::ResourceArithmeticOverflow);
        },
    };
    let rank_limit = rank_bound
        .checked_add(1)
        .ok_or(M1KernelSafetyCertificateErrorV1::ResourceArithmeticOverflow)?;
    if input.ranks.is_empty() || input.ranks.len() > rank_limit {
        return Err(M1KernelSafetyCertificateErrorV1::ResourceControlRanks);
    }
    let mut rank = 0usize;
    while rank < input.ranks.len()
        invariant
            rank <= input.ranks@.len(),
            forall|prior: int| 0 <= prior < rank
                ==> input.ranks@[prior] as nat == input.ranks@.len() - 1 - prior,
        decreases input.ranks@.len() - rank,
    {
        let expected_rank = input.ranks.len() - 1 - rank;
        if input.ranks[rank] as usize != expected_rank {
            return Err(M1KernelSafetyCertificateErrorV1::ResourceControlRanks);
        }
        rank += 1;
    }
    if input.buffer_extents.is_empty() || input.buffer_extents.len() > 16 {
        return Err(M1KernelSafetyCertificateErrorV1::ResourceBufferRoster);
    }
    let mut buffer = 0usize;
    while buffer < input.buffer_extents.len()
        invariant
            buffer <= input.buffer_extents@.len(),
            forall|prior: int| 0 <= prior < buffer
                ==> input.buffer_extents@[prior] > 0,
        decreases input.buffer_extents@.len() - buffer,
    {
        if input.buffer_extents[buffer] == 0 {
            return Err(M1KernelSafetyCertificateErrorV1::ResourceBufferRoster);
        }
        buffer += 1;
    }
    let workitems = match usize::try_from(input.workitems) {
        Ok(workitems) => workitems,
        Err(_) => {
            return Err(M1KernelSafetyCertificateErrorV1::ResourceArithmeticOverflow);
        },
    };
    let phase_workitems = workitems
        .checked_mul(input.ranks.len())
        .ok_or(M1KernelSafetyCertificateErrorV1::ResourceArithmeticOverflow)?;
    let access_limit = phase_workitems
        .checked_mul(8)
        .ok_or(M1KernelSafetyCertificateErrorV1::ResourceArithmeticOverflow)?;
    if input.accesses.len() > access_limit
        || input.accesses.len() > M1_KERNEL_MAX_MODELED_ACCESSES_V1
        || input.initialized_before.len() > M1_KERNEL_MAX_INITIALIZED_RANGES_V1
    {
        return Err(M1KernelSafetyCertificateErrorV1::ResourceRosterBound);
    }
    Ok(())
}

fn modeled_access_reads(access: M1KernelModeledAccessV1) -> (reads: bool)
    ensures reads == m1_modeled_access_reads(access),
{
    matches!(
        access.mode,
        M1KernelModeledAccessModeV1::ReadOnly | M1KernelModeledAccessModeV1::ReadWrite
    )
}

fn initialized_range_in_bounds(
    extents: &[u64],
    range: M1KernelInitializedRangeV1,
) -> (valid: bool)
    ensures valid == initialized_range_is_in_bounds(extents@, range),
{
    let buffer = usize::from(range.buffer);
    if buffer >= extents.len() {
        return false;
    }
    range.start < range.end && range.end <= extents[buffer]
}

fn modeled_access_in_bounds(
    input: &M1KernelSafetyCertificateInputV1<'_>,
    access: M1KernelModeledAccessV1,
) -> (valid: bool)
    ensures valid == access_is_in_bounds(
        input.workitems,
        input.ranks@.len(),
        input.buffer_extents@,
        access,
    ),
{
    let phase = match usize::try_from(access.phase) {
        Ok(phase) => phase,
        Err(_) => return false,
    };
    let buffer = usize::from(access.buffer);
    if phase >= input.ranks.len()
        || access.workitem >= input.workitems
        || buffer >= input.buffer_extents.len()
    {
        return false;
    }
    access.start < access.end && access.end <= input.buffer_extents[buffer]
}

fn initialized_overlap(
    left: M1KernelInitializedRangeV1,
    right: M1KernelInitializedRangeV1,
) -> (overlap: bool)
    ensures overlap == initialized_ranges_overlap(left, right),
{
    left.buffer == right.buffer && left.start < right.end && right.start < left.end
}

fn initialized_covers_access(
    range: M1KernelInitializedRangeV1,
    access: M1KernelModeledAccessV1,
) -> (covers: bool)
    ensures covers == initialized_range_covers_access(range, access),
{
    range.buffer == access.buffer && range.start <= access.start && access.end <= range.end
}

fn read_has_one_initialized_range(
    initialized: &[M1KernelInitializedRangeV1],
    access: M1KernelModeledAccessV1,
) -> (covered: bool)
    ensures covered == exists|range: int| 0 <= range < initialized@.len()
        && initialized_range_covers_access(initialized@[range], access),
{
    let mut range = 0usize;
    while range < initialized.len()
        invariant
            range <= initialized@.len(),
            forall|prior: int| 0 <= prior < range
                ==> !initialized_range_covers_access(initialized@[prior], access),
        decreases initialized@.len() - range,
    {
        if initialized_covers_access(initialized[range], access) {
            return true;
        }
        range += 1;
    }
    false
}

fn validate_m1_kernel_memory_ranges(
    input: &M1KernelSafetyCertificateInputV1<'_>,
) -> (result: Result<(), M1KernelSafetyCertificateErrorV1>)
    ensures result.is_ok() ==> m1_kernel_memory_ranges_safe(input),
{
    let mut initialized = 0usize;
    while initialized < input.initialized_before.len()
        invariant
            initialized <= input.initialized_before@.len(),
            forall|prior: int| 0 <= prior < initialized
                ==> initialized_range_is_in_bounds(
                    input.buffer_extents@,
                    input.initialized_before@[prior],
                ),
            forall|left: int, right: int|
                0 <= left < right < initialized
                    ==> !initialized_ranges_overlap(
                        input.initialized_before@[left],
                        input.initialized_before@[right],
                    ),
        decreases input.initialized_before@.len() - initialized,
    {
        let current = input.initialized_before[initialized];
        if !initialized_range_in_bounds(input.buffer_extents, current) {
            return Err(M1KernelSafetyCertificateErrorV1::MemoryInitializedRange);
        }
        let mut prior = 0usize;
        while prior < initialized
            invariant
                prior <= initialized,
                initialized < input.initialized_before@.len(),
                forall|checked: int| 0 <= checked < prior
                    ==> !initialized_ranges_overlap(
                        input.initialized_before@[checked],
                        current,
                    ),
            decreases initialized - prior,
        {
            if initialized_overlap(input.initialized_before[prior], current) {
                return Err(
                    M1KernelSafetyCertificateErrorV1::MemoryAmbiguousInitialization,
                );
            }
            prior += 1;
        }
        initialized += 1;
    }
    let mut access = 0usize;
    while access < input.accesses.len()
        invariant
            access <= input.accesses@.len(),
            forall|prior: int| 0 <= prior < input.initialized_before@.len()
                ==> initialized_range_is_in_bounds(
                    input.buffer_extents@,
                    input.initialized_before@[prior],
                ),
            forall|left: int, right: int|
                0 <= left < right < input.initialized_before@.len()
                    ==> !initialized_ranges_overlap(
                        input.initialized_before@[left],
                        input.initialized_before@[right],
                    ),
            forall|prior: int| 0 <= prior < access
                ==> access_is_in_bounds(
                    input.workitems,
                    input.ranks@.len(),
                    input.buffer_extents@,
                    #[trigger] input.accesses@[prior],
                ),
        decreases input.accesses@.len() - access,
    {
        let current = input.accesses[access];
        if !modeled_access_in_bounds(input, current) {
            return Err(M1KernelSafetyCertificateErrorV1::MemoryAccessRange);
        }
        access += 1;
    }
    Ok(())
}

fn modeled_reads_are_initialized(
    input: &M1KernelSafetyCertificateInputV1<'_>,
) -> (safe: bool)
    ensures safe == m1_kernel_reads_are_initialized(input),
{
    let mut access = 0usize;
    while access < input.accesses.len()
        invariant
            access <= input.accesses@.len(),
            forall|prior: int| 0 <= prior < access ==> {
                let prior_access = #[trigger] input.accesses@[prior];
                m1_modeled_access_reads(prior_access) ==> exists|range: int|
                    0 <= range < input.initialized_before@.len()
                        && initialized_range_covers_access(
                            input.initialized_before@[range],
                            prior_access,
                        )
            },
        decreases input.accesses@.len() - access,
    {
        let current = input.accesses[access];
        if modeled_access_reads(current)
            && !read_has_one_initialized_range(input.initialized_before, current)
        {
            return false;
        }
        access += 1;
    }
    true
}

/// Checks complete modeled access bounds and unambiguous initialized reads.
///
/// # Errors
///
/// Rejects catalog drift, malformed/out-of-bounds initialized or access
/// ranges, overlapping initialization coverage, and any modeled read not
/// wholly covered by an initialized input range. Pairwise non-overlap of every
/// nonempty initialized range makes that successful coverage unique.
pub fn validate_m1_kernel_memory_safety(
    input: &M1KernelSafetyCertificateInputV1<'_>,
) -> (result: Result<(), M1KernelSafetyCertificateErrorV1>)
    ensures result.is_ok() ==> m1_kernel_memory_safe(input),
{
    validate_m1_kernel_resource_bounds(input)?;
    validate_m1_kernel_memory_ranges(input)?;
    let reads_initialized = modeled_reads_are_initialized(input);
    if !reads_initialized {
        return Err(M1KernelSafetyCertificateErrorV1::MemoryUninitializedRead);
    }
    Ok(())
}

fn modeled_access_writes(access: M1KernelModeledAccessV1) -> (writes: bool)
    ensures writes == m1_modeled_access_writes(access),
{
    matches!(
        access.mode,
        M1KernelModeledAccessModeV1::WriteOnly | M1KernelModeledAccessModeV1::ReadWrite
    )
}

fn modeled_access_overlap(
    left: M1KernelModeledAccessV1,
    right: M1KernelModeledAccessV1,
) -> (overlap: bool)
    ensures overlap == m1_modeled_accesses_overlap(left, right),
{
    left.buffer == right.buffer && left.start < right.end && right.start < left.end
}

fn conflicting_accesses(
    left: M1KernelModeledAccessV1,
    right: M1KernelModeledAccessV1,
) -> (conflict: bool)
    ensures conflict == !(
        left.phase != right.phase
            || left.workitem == right.workitem
            || (!m1_modeled_access_writes(left) && !m1_modeled_access_writes(right))
            || !m1_modeled_accesses_overlap(left, right)
    ),
{
    left.phase == right.phase
        && left.workitem != right.workitem
        && (modeled_access_writes(left) || modeled_access_writes(right))
        && modeled_access_overlap(left, right)
}

fn modeled_access_roster_is_race_free(
    input: &M1KernelSafetyCertificateInputV1<'_>,
) -> (safe: bool)
    ensures safe == m1_kernel_access_roster_race_free(input),
{
    let mut left = 0usize;
    while left < input.accesses.len()
        invariant
            left <= input.accesses@.len(),
            forall|prior_left: int, prior_right: int|
                0 <= prior_left < left
                    && prior_left < prior_right < input.accesses@.len()
                    ==> {
                        let left_access = #[trigger] input.accesses@[prior_left];
                        let right_access = #[trigger] input.accesses@[prior_right];
                        left_access.phase != right_access.phase
                            || left_access.workitem == right_access.workitem
                            || (!m1_modeled_access_writes(left_access)
                                && !m1_modeled_access_writes(right_access))
                            || !m1_modeled_accesses_overlap(left_access, right_access)
                    },
        decreases input.accesses@.len() - left,
    {
        let mut right = left + 1;
        while right < input.accesses.len()
            invariant
                left < input.accesses@.len(),
                left + 1 <= right <= input.accesses@.len(),
                forall|prior: int| left < prior < right ==> {
                    let left_access = #[trigger] input.accesses@[left as int];
                    let right_access = #[trigger] input.accesses@[prior];
                    left_access.phase != right_access.phase
                        || left_access.workitem == right_access.workitem
                        || (!m1_modeled_access_writes(left_access)
                            && !m1_modeled_access_writes(right_access))
                        || !m1_modeled_accesses_overlap(left_access, right_access)
                },
            decreases input.accesses@.len() - right,
        {
            if conflicting_accesses(input.accesses[left], input.accesses[right]) {
                return false;
            }
            right += 1;
        }
        left += 1;
    }
    true
}

/// Checks the complete modeled access roster for same-phase data races.
///
/// # Errors
///
/// Rejects catalog drift or any overlapping same-buffer access by distinct
/// same-phase workitems when either access writes.
pub fn validate_m1_kernel_race_freedom(
    input: &M1KernelSafetyCertificateInputV1<'_>,
) -> (result: Result<(), M1KernelSafetyCertificateErrorV1>)
    ensures result.is_ok() ==> m1_kernel_race_free(input),
{
    validate_m1_kernel_memory_safety(input)?;
    let race_free = modeled_access_roster_is_race_free(input);
    if !race_free {
        return Err(M1KernelSafetyCertificateErrorV1::RaceConflictingAccess);
    }
    Ok(())
}

/// Validates all three source-level properties and returns one non-clone owner.
///
/// The supplied access roster is treated as the caller's complete modeled
/// effect certificate; this function does not infer effects from kernel source.
///
/// # Errors
///
/// Returns the first exact fail-closed resource, memory, or race rejection.
pub fn validate_m1_kernel_safety_certificate(
    input: M1KernelSafetyCertificateInputV1<'_>,
) -> (result: Result<ValidatedM1KernelSafetyCertificateV1<'_>, M1KernelSafetyCertificateErrorV1>)
    ensures match result {
        Ok(validated) => {
            &&& validated.resource_bounded_spec()
            &&& validated.memory_safe_spec()
            &&& validated.race_free_spec()
        },
        Err(_) => true,
    },
{
    validate_m1_kernel_resource_bounds(&input)?;
    validate_m1_kernel_memory_safety(&input)?;
    validate_m1_kernel_race_freedom(&input)?;
    Ok(ValidatedM1KernelSafetyCertificateV1 {
        candidate: *input.catalog.input(),
        expected: input.expected,
        workitems: input.workitems,
        ranks: input.ranks,
        buffer_extents: input.buffer_extents,
        accesses: input.accesses,
        initialized_before: input.initialized_before,
    })
}

} // verus!
