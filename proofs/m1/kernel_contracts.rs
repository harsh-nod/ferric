#![forbid(unsafe_code)]

//! M1 K1-K7 finite modeled-kernel contract theorem.
//!
//! This module joins an independently validated structural-catalog operation
//! to a caller-supplied, finite schedule and byte-range effect certificate.
//! The certificate model proves finite workitem/phase bounds, initialized
//! reads, exact modeled write effects, and absence of conflicting overlapping
//! accesses by distinct workitems in one modeled phase. Its decreasing rank
//! is control termination only, not numerical or optimization convergence.
//!
//! No theorem here establishes that Ferric LLVM source, a compiler result, an
//! object, a loader, a driver, firmware, or hardware implements the supplied
//! certificate. It also grants no numerical-correctness, launch, completion,
//! timing, throughput, or performance authority.

#[allow(unused_imports)]
use ferric_kernels::{KernelCatalogInput, KernelFamily, ValidatedKernelCatalogInput};
#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

/// Direction of one byte range in the source-level access certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1KernelModeledAccessModeV1 {
    /// The workitem only observes the range.
    ReadOnly,
    /// The workitem only initializes or replaces the range.
    WriteOnly,
    /// The workitem observes and replaces the range.
    ReadWrite,
}

/// One half-open byte range touched by one workitem in one modeled phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1KernelModeledAccessV1 {
    /// Zero-based modeled phase.
    pub phase: u32,
    /// Zero-based workitem within the finite admitted launch.
    pub workitem: u32,
    /// Zero-based logical buffer in the certificate's extent sequence.
    pub buffer: u16,
    /// Inclusive byte offset.
    pub start: u64,
    /// Exclusive byte offset.
    pub end: u64,
    /// Read/write direction.
    pub mode: M1KernelModeledAccessModeV1,
}

/// Conservative maximum flattened workitem count for each M1 family contract.
///
/// These are certificate admission limits. They are not device-capacity or
/// launch-qualification claims.
pub open spec fn m1_family_workitem_bound_spec(family: KernelFamily) -> nat {
    match family {
        KernelFamily::K1GemmGemv => 77_791_232,
        KernelFamily::K2RmsNormResidual => 4_194_304,
        KernelFamily::K3RopePagedKv => 131_072,
        KernelFamily::K4GqaPrefill => 4_194_304,
        KernelFamily::K5PagedGqaDecode => 81_920,
        KernelFamily::K6SwiGlu => 3_145_728,
        KernelFamily::K7LogitsCompact => 131_072,
    }
}

/// Conservative maximum decreasing control rank for each M1 family contract.
///
/// The rank bounds source-level modeled loops only. They do not bound machine
/// instructions, cycles, retries, queue latency, or execution time.
pub open spec fn m1_family_control_rank_bound_spec(family: KernelFamily) -> nat {
    match family {
        KernelFamily::K1GemmGemv => 3_072,
        KernelFamily::K2RmsNormResidual => 64,
        KernelFamily::K3RopePagedKv => 8_192,
        KernelFamily::K4GqaPrefill => 8_192,
        KernelFamily::K5PagedGqaDecode => 8_192,
        KernelFamily::K6SwiGlu => 8,
        KernelFamily::K7LogitsCompact => 2_374,
    }
}

/// Every K1-K7 family has positive finite certificate bounds.
pub proof fn m1_family_contract_bounds_are_finite(family: KernelFamily)
    ensures
        0 < m1_family_workitem_bound_spec(family) <= 77_791_232,
        0 < m1_family_control_rank_bound_spec(family) <= 8_192,
{
    reveal(m1_family_workitem_bound_spec);
    reveal(m1_family_control_rank_bound_spec);
}

/// A finite control trace whose natural-valued rank reaches zero exactly.
pub open spec fn m1_finite_ranked_schedule(
    family: KernelFamily,
    workitems: nat,
    ranks: Seq<nat>,
) -> bool {
    &&& 0 < workitems <= m1_family_workitem_bound_spec(family)
    &&& 0 < ranks.len()
    &&& ranks.len() <= m1_family_control_rank_bound_spec(family) + 1
    &&& forall|phase: int| 0 <= phase < ranks.len()
        ==> ranks[phase] == ranks.len() - 1 - phase
}

/// The exact-rank certificate reaches zero and decreases by one per phase.
pub proof fn m1_ranked_schedule_converges(
    family: KernelFamily,
    workitems: nat,
    ranks: Seq<nat>,
)
    requires m1_finite_ranked_schedule(family, workitems, ranks),
    ensures
        0 < workitems <= m1_family_workitem_bound_spec(family),
        0 < ranks.len() <= m1_family_control_rank_bound_spec(family) + 1,
        ranks[0] == ranks.len() - 1,
        ranks[ranks.len() - 1] == 0,
        forall|phase: int| 0 <= phase < ranks.len() - 1
            ==> #[trigger] ranks[phase] == ranks[phase + 1] + 1,
{
    reveal(m1_finite_ranked_schedule);
    assert(ranks[0] == ranks.len() - 1 - 0);
    assert(ranks[ranks.len() - 1] == ranks.len() - 1 - (ranks.len() - 1));
    assert forall|phase: int| 0 <= phase < ranks.len() - 1
        implies #[trigger] ranks[phase] == ranks[phase + 1] + 1 by {
        assert(ranks[phase] == ranks.len() - 1 - phase);
        assert(ranks[phase + 1] == ranks.len() - 1 - (phase + 1));
    }
}

/// Whether this access observes its range.
pub open spec fn m1_modeled_access_reads(access: M1KernelModeledAccessV1) -> bool {
    access.mode == M1KernelModeledAccessModeV1::ReadOnly
        || access.mode == M1KernelModeledAccessModeV1::ReadWrite
}

/// Whether this access replaces or initializes its range.
pub open spec fn m1_modeled_access_writes(access: M1KernelModeledAccessV1) -> bool {
    access.mode == M1KernelModeledAccessModeV1::WriteOnly
        || access.mode == M1KernelModeledAccessModeV1::ReadWrite
}

/// Whether one access contains one logical byte location.
pub open spec fn m1_modeled_access_contains(
    access: M1KernelModeledAccessV1,
    buffer: nat,
    offset: nat,
) -> bool {
    &&& access.buffer as nat == buffer
    &&& access.start as nat <= offset
    &&& offset < access.end as nat
}

/// Whether two half-open ranges overlap in the same logical buffer.
pub open spec fn m1_modeled_accesses_overlap(
    left: M1KernelModeledAccessV1,
    right: M1KernelModeledAccessV1,
) -> bool {
    &&& left.buffer == right.buffer
    &&& left.start < right.end
    &&& right.start < left.end
}

/// Every modeled access names an in-bounds phase, workitem, buffer, and
/// nonempty half-open byte range.
pub open spec fn m1_modeled_accesses_are_in_bounds(
    workitems: nat,
    phases: nat,
    buffer_extents: Seq<nat>,
    accesses: Seq<M1KernelModeledAccessV1>,
) -> bool {
    &&& 0 < buffer_extents.len() <= 16
    &&& forall|buffer: int| 0 <= buffer < buffer_extents.len()
        ==> 0 < buffer_extents[buffer]
    &&& forall|index: int| 0 <= index < accesses.len() ==> {
        let access = #[trigger] accesses[index];
        &&& (access.phase as nat) < phases
        &&& (access.workitem as nat) < workitems
        &&& (access.buffer as nat) < buffer_extents.len()
        &&& access.start < access.end
        &&& (access.end as nat) <= buffer_extents[access.buffer as int]
    }
}

/// Every cell named by an initialization set is inside a declared buffer.
pub open spec fn m1_modeled_initialization_is_in_bounds(
    buffer_extents: Seq<nat>,
    initialized: ISet<(nat, nat)>,
) -> bool {
    forall|cell: (nat, nat)| initialized.contains(cell) ==> {
        &&& cell.0 < buffer_extents.len()
        &&& cell.1 < buffer_extents[cell.0 as int]
    }
}

/// Every modeled byte read is initialized before the certified kernel step.
pub open spec fn m1_modeled_reads_are_initialized(
    accesses: Seq<M1KernelModeledAccessV1>,
    initialized_before: ISet<(nat, nat)>,
) -> bool {
    forall|index: int, offset: nat|
        0 <= index < accesses.len()
            && m1_modeled_access_reads(accesses[index])
            && accesses[index].start as nat <= offset
            && offset < accesses[index].end as nat
        ==> initialized_before.contains((accesses[index].buffer as nat, offset))
}

/// The post-initialization set is exactly the prestate plus modeled writes.
pub open spec fn m1_modeled_effect_is_exact(
    buffer_extents: Seq<nat>,
    accesses: Seq<M1KernelModeledAccessV1>,
    initialized_before: ISet<(nat, nat)>,
    initialized_after: ISet<(nat, nat)>,
) -> bool {
    &&& m1_modeled_initialization_is_in_bounds(buffer_extents, initialized_before)
    &&& m1_modeled_initialization_is_in_bounds(buffer_extents, initialized_after)
    &&& forall|buffer: nat, offset: nat|
        buffer < buffer_extents.len() && offset < buffer_extents[buffer as int]
        ==> initialized_after.contains((buffer, offset)) == (
            initialized_before.contains((buffer, offset))
            || exists|index: int| 0 <= index < accesses.len()
                && m1_modeled_access_writes(accesses[index])
                && m1_modeled_access_contains(accesses[index], buffer, offset)
        )
}

/// Distinct workitems never overlap in one phase when either access writes.
/// Read/read overlap and reuse after a modeled phase boundary are permitted.
pub open spec fn m1_modeled_accesses_are_race_free(
    accesses: Seq<M1KernelModeledAccessV1>,
) -> bool {
    forall|left: int, right: int|
        0 <= left < accesses.len()
            && 0 <= right < accesses.len()
            && accesses[left].phase == accesses[right].phase
            && accesses[left].workitem != accesses[right].workitem
            && (m1_modeled_access_writes(accesses[left])
                || m1_modeled_access_writes(accesses[right]))
        ==> !m1_modeled_accesses_overlap(accesses[left], accesses[right])
}

/// Complete premise required to admit one modeled K1-K7 contract witness.
///
/// The finite access-count clause is a proof-resource bound, not a statement
/// about compiler-generated instructions or hardware memory transactions.
pub open spec fn m1_kernel_modeled_contract_admitted(
    validated: &ValidatedKernelCatalogInput,
    expected: KernelCatalogInput,
    workitems: nat,
    ranks: Seq<nat>,
    buffer_extents: Seq<nat>,
    accesses: Seq<M1KernelModeledAccessV1>,
    initialized_before: ISet<(nat, nat)>,
    initialized_after: ISet<(nat, nat)>,
) -> bool {
    &&& validated.matches_expected_spec(expected)
    &&& m1_finite_ranked_schedule(expected.profile.family, workitems, ranks)
    &&& accesses.len() <= workitems * ranks.len() * 8
    &&& m1_modeled_accesses_are_in_bounds(
        workitems,
        ranks.len(),
        buffer_extents,
        accesses,
    )
    &&& m1_modeled_reads_are_initialized(accesses, initialized_before)
    &&& m1_modeled_effect_is_exact(
        buffer_extents,
        accesses,
        initialized_before,
        initialized_after,
    )
    &&& m1_modeled_accesses_are_race_free(accesses)
}

/// Decomposes one admitted certificate into the complete M1 modeled contract.
///
/// In particular, the structural catalog validator supplies the exact finite
/// Qwen3 dimensions and K1-K7 membership; the independent certificate supplies
/// the schedule and memory-effect premises. The result establishes control
/// convergence only for that model and makes none of the implementation or
/// machine-boundary claims listed in the module documentation.
pub proof fn m1_k1_k7_modeled_contract_properties(
    validated: &ValidatedKernelCatalogInput,
    expected: KernelCatalogInput,
    workitems: nat,
    ranks: Seq<nat>,
    buffer_extents: Seq<nat>,
    accesses: Seq<M1KernelModeledAccessV1>,
    initialized_before: ISet<(nat, nat)>,
    initialized_after: ISet<(nat, nat)>,
)
    requires m1_kernel_modeled_contract_admitted(
        validated,
        expected,
        workitems,
        ranks,
        buffer_extents,
        accesses,
        initialized_before,
        initialized_after,
    ),
    ensures
        ferric_kernels::m1_kernel_profile_is_finite(expected.profile),
        0 < workitems
            <= m1_family_workitem_bound_spec(expected.profile.family),
        0 < ranks.len()
            <= m1_family_control_rank_bound_spec(expected.profile.family) + 1,
        ranks[0] == ranks.len() - 1,
        ranks[ranks.len() - 1] == 0,
        forall|phase: int| 0 <= phase < ranks.len() - 1
            ==> #[trigger] ranks[phase] == ranks[phase + 1] + 1,
        accesses.len() <= workitems * ranks.len() * 8,
        m1_modeled_accesses_are_in_bounds(
            workitems,
            ranks.len(),
            buffer_extents,
            accesses,
        ),
        m1_modeled_reads_are_initialized(accesses, initialized_before),
        m1_modeled_effect_is_exact(
            buffer_extents,
            accesses,
            initialized_before,
            initialized_after,
        ),
        m1_modeled_accesses_are_race_free(accesses),
{
    reveal(m1_kernel_modeled_contract_admitted);
    validated.expose_m1_finite_profile(expected);
    m1_family_contract_bounds_are_finite(expected.profile.family);
    m1_ranked_schedule_converges(expected.profile.family, workitems, ranks);
}

/// Mathematical natural-number view of executable `u32` certificate values.
pub open spec fn m1_u32_certificate_values(values: Seq<u32>) -> Seq<nat> {
    Seq::new(values.len(), |index: int| values[index] as nat)
}

/// Mathematical natural-number view of executable `u64` buffer extents.
pub open spec fn m1_u64_certificate_values(values: Seq<u64>) -> Seq<nat> {
    Seq::new(values.len(), |index: int| values[index] as nat)
}

/// Mathematical initialized-byte set named by finite executable cells.
pub open spec fn m1_initialization_certificate_set(
    cells: Seq<(u16, u64)>,
) -> ISet<(nat, nat)> {
    ISet::new(|cell: (nat, nat)| exists|index: int| 0 <= index < cells.len()
        && (#[trigger] cells[index]).0 as nat == cell.0
        && cells[index].1 as nat == cell.1)
}

/// Finite executable encoding consumed by the queryable theorem wrapper.
pub struct M1KernelModeledContractInputsV1<'a> {
    /// Independently validated structural-catalog owner.
    pub validated: &'a ValidatedKernelCatalogInput,
    /// Exact independently supplied catalog expectation.
    pub expected: KernelCatalogInput,
    /// Flattened modeled workitem count.
    pub workitems: u32,
    /// Natural-valued control ranks in modeled phase order.
    pub ranks: &'a [u32],
    /// Logical byte extent for every modeled buffer.
    pub buffer_extents: &'a [u64],
    /// Complete finite modeled byte-access roster.
    pub accesses: &'a [M1KernelModeledAccessV1],
    /// Explicit initialized-byte prestate cells.
    pub initialized_before: &'a [(u16, u64)],
    /// Explicit initialized-byte poststate cells.
    pub initialized_after: &'a [(u16, u64)],
}

/// Directly queryable theorem wrapper for one finite K1-K7 certificate.
///
/// This function has no runtime effect. Its slice arguments provide finite,
/// executable encodings of the mathematical certificate so qualification can
/// select this exact compiler body. The precondition remains independent
/// proof evidence; calling this wrapper does not validate a kernel, source,
/// object, launch, completion, numerical result, hardware result, or
/// performance result.
pub fn m1_k1_k7_modeled_contract_theorem(inputs: M1KernelModeledContractInputsV1<'_>)
    requires m1_kernel_modeled_contract_admitted(
        inputs.validated,
        inputs.expected,
        inputs.workitems as nat,
        m1_u32_certificate_values(inputs.ranks@),
        m1_u64_certificate_values(inputs.buffer_extents@),
        inputs.accesses@,
        m1_initialization_certificate_set(inputs.initialized_before@),
        m1_initialization_certificate_set(inputs.initialized_after@),
    ),
    ensures
        ferric_kernels::m1_kernel_profile_is_finite(inputs.expected.profile),
        0 < inputs.workitems as nat
            <= m1_family_workitem_bound_spec(inputs.expected.profile.family),
        0 < inputs.ranks@.len()
            <= m1_family_control_rank_bound_spec(inputs.expected.profile.family) + 1,
        m1_u32_certificate_values(inputs.ranks@)[0] == inputs.ranks@.len() - 1,
        m1_u32_certificate_values(inputs.ranks@)[inputs.ranks@.len() - 1] == 0,
        forall|phase: int| 0 <= phase < inputs.ranks@.len() - 1
            ==> #[trigger] m1_u32_certificate_values(inputs.ranks@)[phase]
                == m1_u32_certificate_values(inputs.ranks@)[phase + 1] + 1,
        inputs.accesses@.len()
            <= (inputs.workitems as nat) * inputs.ranks@.len() * 8,
        m1_modeled_accesses_are_in_bounds(
            inputs.workitems as nat,
            inputs.ranks@.len(),
            m1_u64_certificate_values(inputs.buffer_extents@),
            inputs.accesses@,
        ),
        m1_modeled_reads_are_initialized(
            inputs.accesses@,
            m1_initialization_certificate_set(inputs.initialized_before@),
        ),
        m1_modeled_effect_is_exact(
            m1_u64_certificate_values(inputs.buffer_extents@),
            inputs.accesses@,
            m1_initialization_certificate_set(inputs.initialized_before@),
            m1_initialization_certificate_set(inputs.initialized_after@),
        ),
        m1_modeled_accesses_are_race_free(inputs.accesses@),
{
    let _ = &inputs;
    proof {
        m1_k1_k7_modeled_contract_properties(
            inputs.validated,
            inputs.expected,
            inputs.workitems as nat,
            m1_u32_certificate_values(inputs.ranks@),
            m1_u64_certificate_values(inputs.buffer_extents@),
            inputs.accesses@,
            m1_initialization_certificate_set(inputs.initialized_before@),
            m1_initialization_certificate_set(inputs.initialized_after@),
        );
    }
}

} // verus!
