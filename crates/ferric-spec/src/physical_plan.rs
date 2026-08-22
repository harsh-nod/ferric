//! Fail-closed, inert physical-plan declaration validation.
//!
//! This module can compare a caller-supplied physical packet declaration with
//! a separately supplied exact expectation. Both inputs are data. Validation
//! does not authenticate either input, prove a fusion premise, construct an
//! AQL packet, or grant artifact, descriptor, address, queue, publication,
//! completion, hardware, performance, or qualification authority.
//!
//! The V1 dependency rule is intentionally conservative: packet zero has no
//! predecessors and every later packet must depend on its immediate predecessor.
//! Additional earlier predecessors are allowed. This makes the final packet a
//! declared completion-dominating node while preserving the sequential logical
//! plan order. No claim is made that a runtime implements those dependencies.
//!
//! This inert prerequisite is not an M1 property/path foundation registry row:
//! the current finite registry has no physical-declaration validation
//! association, and inventing one here would overstate closure ownership.

use crate::{plan_step_count, Identity, Qwen3PlanError, Qwen3PlanSelection};
use vstd::prelude::*;

verus! {

/// Canonical inert physical-plan declaration version.
pub const M1_PHYSICAL_PLAN_DECLARATION_VERSION: u32 = 1;
/// Exact maximum of the currently reviewed batch-arithmetic declaration.
pub const M1_REVIEWED_BATCH_PACKET_CAPACITY_V1: u32 = 256;
/// Exact maximum of the reviewed fixed-batch V2 declaration.
pub const M1_REVIEWED_BATCH_PACKET_CAPACITY_V2: u32 = 1_024;
/// Smallest ring capacity named by the reviewed arithmetic profile.
pub const M1_MIN_DECLARED_RING_PACKETS_V1: u32 = 64;
/// Largest ring capacity named by the reviewed arithmetic profile.
pub const M1_MAX_DECLARED_RING_PACKETS_V1: u32 = 33_554_432;
/// Absolute source-level bound for a future untrusted packet-capacity expectation.
pub const M1_MAX_UNTRUSTED_PACKET_CAPACITY_V1: u32 = 1_024;

/// Provenance class of a separately supplied capacity expectation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicalCapacitySource {
    /// Declares the current reviewed arithmetic maximum of 256 packets.
    ///
    /// This tag and its identity are not fe2o3 authentication or support
    /// evidence.
    ReviewedBatchArithmeticV1,
    /// Declares the reviewed fixed-batch V2 maximum of 1,024 packets.
    ///
    /// This tag is a Ferric structural expectation only. It is not an
    /// authenticated fe2o3 build, runtime observation, or support receipt.
    ReviewedBatchArithmeticV2,
    /// Explicitly untrusted future capacity, usable only for structural tests
    /// and future integration validation.
    FutureUntrusted,
}

/// Separate, caller-supplied capacity expectation.
///
/// A future descriptor is deliberately labeled untrusted. Acceptance under
/// that descriptor cannot be reported as fe2o3 capability or evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalCapacityExpectation {
    pub source: PhysicalCapacitySource,
    pub descriptor_id: Identity,
    pub batch_packet_capacity: u32,
    pub ring_packet_capacity: u32,
}

/// Exact per-packet identities expected from a future authenticated build.
///
/// The validator checks presence, role separation, and exact equality with the
/// expectation. It does not authenticate the bytes behind any identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalPacketIdentityBinding {
    pub kernel_contract_id: Identity,
    pub artifact_id: Identity,
    pub descriptor_id: Identity,
    pub geometry_id: Identity,
    pub kernarg_layout_id: Identity,
    pub buffer_layout_id: Identity,
    pub effect_contract_id: Identity,
}

/// Explicit unproved premise required when one packet covers multiple logical
/// operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeclaredFusionRefinementPremise {
    /// Identity of the claimed logical-span-to-packet refinement relation.
    pub relation_id: Identity,
    /// Identity of the evidence obligation that must eventually discharge it.
    pub evidence_requirement_id: Identity,
}

/// One physical packet position and its exact contiguous logical span.
#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalPacketSpanDeclaration {
    pub packet_index: u32,
    pub logical_start: u32,
    pub logical_count: u32,
    pub identities: PhysicalPacketIdentityBinding,
    /// Strictly increasing packet indices, all earlier than `packet_index`.
    pub predecessors: Vec<u32>,
    /// Required exactly when `logical_count > 1`.
    pub fusion: Option<DeclaredFusionRefinementPremise>,
}

/// Exact batch-level publication declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalPublicationDeclaration {
    pub contract_id: Identity,
    pub reservation_count: u8,
    pub reserved_packet_count: u32,
    pub release_header_count: u32,
    pub doorbell_count: u8,
    pub doorbell_packet_index: u32,
}

/// Exact declared completion-dominance boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalCompletionDeclaration {
    pub contract_id: Identity,
    pub completion_packet_index: u32,
    pub completion_signal_count: u32,
    pub declared_dominated_packet_count: u32,
}

/// Complete inert physical-plan candidate.
#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalPlanDeclaration {
    pub version: u32,
    pub declaration_id: Identity,
    pub source_closure_id: Identity,
    pub logical_plan_id: Identity,
    pub selection: Qwen3PlanSelection,
    pub logical_operation_count: u32,
    pub capacity_descriptor_id: Identity,
    pub declared_batch_packet_capacity: u32,
    pub declared_ring_packet_capacity: u32,
    pub packets: Vec<PhysicalPacketSpanDeclaration>,
    pub publication: PhysicalPublicationDeclaration,
    pub completion: PhysicalCompletionDeclaration,
}

/// Separately supplied exact expectation for one candidate.
///
/// This wrapper is not trusted and is not embedded with a production mapping.
#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalPlanExpectation {
    pub expected: PhysicalPlanDeclaration,
    pub capacity: PhysicalCapacityExpectation,
}

/// Identity role used by fail-closed diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicalIdentityRole {
    Declaration,
    SourceClosure,
    LogicalPlan,
    CapacityDescriptor,
    PublicationContract,
    CompletionContract,
    KernelContract,
    Artifact,
    Descriptor,
    Geometry,
    KernargLayout,
    BufferLayout,
    EffectContract,
    FusionRelation,
    FusionEvidenceRequirement,
}

/// Fail-closed physical-plan declaration error.
#[derive(Debug, PartialEq, Eq)]
pub enum PhysicalPlanError {
    InvalidExpectation,
    UnsupportedVersion,
    MissingIdentity {
        role: PhysicalIdentityRole,
        packet_index: Option<u32>,
    },
    ReusedIdentity {
        packet_index: Option<u32>,
    },
    Selection(Qwen3PlanError),
    LogicalOperationCount {
        expected: u32,
        actual: u32,
    },
    CapacityDescriptorDrift,
    InvalidCapacity,
    PacketCountExceedsCapacity {
        packet_count: u32,
        capacity: u32,
    },
    PacketCountExceedsRing {
        packet_count: u32,
        ring_capacity: u32,
    },
    EmptyPacketSet,
    PacketIndexDrift {
        expected: u32,
        actual: u32,
    },
    InvalidLogicalSpan {
        packet_index: u32,
    },
    LogicalCoverageDrift {
        packet_index: u32,
        expected_start: u32,
        actual_start: u32,
    },
    LogicalCoverageEnd {
        expected: u32,
        actual: u32,
    },
    DirectSpanHasFusionPremise {
        packet_index: u32,
    },
    FusionPremiseMissing {
        packet_index: u32,
    },
    InvalidFusionPremise {
        packet_index: u32,
    },
    InvalidPredecessor {
        packet_index: u32,
    },
    CompletionDependencyMissing {
        packet_index: u32,
        required_predecessor: u32,
    },
    PublicationDrift,
    CompletionDrift,
    ExpectedHeaderDrift,
    ExpectedPacketDrift {
        packet_index: u32,
    },
}

/// Structurally checked declaration retained under its untrusted expectation.
///
/// This value is intentionally not `Clone`. It grants no runtime or evidence
/// authority and has no packet-construction or submission operation.
#[derive(Debug, PartialEq, Eq)]
pub struct StructurallyValidatedPhysicalPlan {
    declaration: PhysicalPlanDeclaration,
    capacity_source: PhysicalCapacitySource,
}

impl StructurallyValidatedPhysicalPlan {
    pub closed spec fn declaration_spec(&self) -> PhysicalPlanDeclaration {
        self.declaration
    }

    pub closed spec fn capacity_source_spec(&self) -> PhysicalCapacitySource {
        self.capacity_source
    }

    /// Returns the inert declaration for identity binding and later review.
    #[must_use]
    pub const fn declaration(&self) -> (declaration: &PhysicalPlanDeclaration)
        ensures *declaration == self.declaration_spec(),
    {
        &self.declaration
    }

    /// Returns the retained expectation class. `FutureUntrusted` never means
    /// fe2o3 support, and the reviewed tag is not authentication evidence.
    #[must_use]
    pub const fn capacity_source(&self) -> (source: PhysicalCapacitySource)
        ensures source == self.capacity_source_spec(),
    {
        self.capacity_source
    }
}

closed spec fn physical_identity_is_present(identity: Identity) -> bool {
    exists|index: int|
        0 <= index < identity.bytes_spec().len()
            && identity.bytes_spec()[index] != 0
}

closed spec fn physical_identities_are_distinct(identities: Seq<Identity>) -> bool {
    forall|left: int, right: int|
        0 <= left < right < identities.len()
            ==> identities[left].bytes_spec() != identities[right].bytes_spec()
}

closed spec fn global_identity_sequence(
    declaration: &PhysicalPlanDeclaration,
) -> Seq<Identity> {
    Seq::empty()
        .push(declaration.declaration_id)
        .push(declaration.source_closure_id)
        .push(declaration.logical_plan_id)
        .push(declaration.capacity_descriptor_id)
        .push(declaration.publication.contract_id)
        .push(declaration.completion.contract_id)
}

closed spec fn packet_identity_sequence(
    identities: PhysicalPacketIdentityBinding,
) -> Seq<Identity> {
    Seq::empty()
        .push(identities.kernel_contract_id)
        .push(identities.artifact_id)
        .push(identities.descriptor_id)
        .push(identities.geometry_id)
        .push(identities.kernarg_layout_id)
        .push(identities.buffer_layout_id)
        .push(identities.effect_contract_id)
}

closed spec fn fused_identity_sequence(
    identities: PhysicalPacketIdentityBinding,
    premise: DeclaredFusionRefinementPremise,
) -> Seq<Identity> {
    packet_identity_sequence(identities)
        .push(premise.relation_id)
        .push(premise.evidence_requirement_id)
}

closed spec fn declared_ring_capacity_is_valid(value: u32) -> bool {
    match value {
        64 | 128 | 256 | 512 | 1_024 | 2_048 | 4_096 | 8_192 | 16_384 | 32_768
        | 65_536 | 131_072 | 262_144 | 524_288 | 1_048_576 | 2_097_152
        | 4_194_304 | 8_388_608 | 16_777_216 | 33_554_432 => true,
        _ => false,
    }
}

/// Complete structural validity relation for an untrusted capacity record.
pub closed spec fn physical_capacity_expectation_is_structurally_valid(
    capacity: PhysicalCapacityExpectation,
) -> bool {
    physical_identity_is_present(capacity.descriptor_id)
        && 0 < capacity.batch_packet_capacity
        && capacity.batch_packet_capacity <= M1_MAX_UNTRUSTED_PACKET_CAPACITY_V1
        && declared_ring_capacity_is_valid(capacity.ring_packet_capacity)
        && capacity.batch_packet_capacity <= capacity.ring_packet_capacity
        && match capacity.source {
            PhysicalCapacitySource::ReviewedBatchArithmeticV1 => {
                capacity.batch_packet_capacity == M1_REVIEWED_BATCH_PACKET_CAPACITY_V1
            },
            PhysicalCapacitySource::ReviewedBatchArithmeticV2 => {
                capacity.batch_packet_capacity == M1_REVIEWED_BATCH_PACKET_CAPACITY_V2
            },
            PhysicalCapacitySource::FutureUntrusted => true,
        }
}

closed spec fn packet_identity_binding_is_structurally_valid(
    identities: PhysicalPacketIdentityBinding,
) -> bool {
    physical_identity_is_present(identities.kernel_contract_id)
        && physical_identity_is_present(identities.artifact_id)
        && physical_identity_is_present(identities.descriptor_id)
        && physical_identity_is_present(identities.geometry_id)
        && physical_identity_is_present(identities.kernarg_layout_id)
        && physical_identity_is_present(identities.buffer_layout_id)
        && physical_identity_is_present(identities.effect_contract_id)
        && physical_identities_are_distinct(packet_identity_sequence(identities))
}

closed spec fn packet_fusion_declaration_is_structurally_valid(
    logical_count: u32,
    identities: PhysicalPacketIdentityBinding,
    fusion: Option<DeclaredFusionRefinementPremise>,
) -> bool {
    match (logical_count, fusion) {
        (1, None) => true,
        (1, Some(_)) | (_, None) => false,
        (_, Some(premise)) => {
            physical_identity_is_present(premise.relation_id)
                && physical_identity_is_present(premise.evidence_requirement_id)
                && physical_identities_are_distinct(fused_identity_sequence(identities, premise))
        },
    }
}

closed spec fn packet_predecessors_are_structurally_valid(
    packet_index: u32,
    predecessors: Seq<u32>,
) -> bool {
    &&& forall|index: int|
        0 <= index < predecessors.len() ==> predecessors[index] < packet_index
    &&& forall|index: int|
        0 < index < predecessors.len()
            ==> predecessors[index - 1] < #[trigger] predecessors[index]
    &&& if packet_index == 0 {
        predecessors.len() == 0
    } else {
        0 < predecessors.len()
            && predecessors[predecessors.len() - 1] == packet_index - 1
    }
}

closed spec fn packet_logical_end(packet: PhysicalPacketSpanDeclaration) -> int {
    packet.logical_start as int + packet.logical_count as int
}

closed spec fn physical_packet_is_structurally_valid_at(
    declaration: &PhysicalPlanDeclaration,
    index: int,
) -> bool {
    let packet = declaration.packets@[index];
    &&& packet.packet_index as int == index
    &&& 0 < packet.logical_count
    &&& packet_logical_end(packet) <= u32::MAX as int
    &&& packet_logical_end(packet) <= declaration.logical_operation_count as int
    &&& if index == 0 {
        packet.logical_start == 0
    } else {
        packet.logical_start as int == packet_logical_end(declaration.packets@[index - 1])
    }
    &&& packet_identity_binding_is_structurally_valid(packet.identities)
    &&& packet_fusion_declaration_is_structurally_valid(
        packet.logical_count,
        packet.identities,
        packet.fusion,
    )
    &&& packet_predecessors_are_structurally_valid(
        packet.packet_index,
        packet.predecessors@,
    )
}

closed spec fn global_identities_are_structurally_valid(
    declaration: &PhysicalPlanDeclaration,
) -> bool {
    physical_identity_is_present(declaration.declaration_id)
        && physical_identity_is_present(declaration.source_closure_id)
        && physical_identity_is_present(declaration.logical_plan_id)
        && physical_identity_is_present(declaration.capacity_descriptor_id)
        && physical_identity_is_present(declaration.publication.contract_id)
        && physical_identity_is_present(declaration.completion.contract_id)
        && physical_identities_are_distinct(global_identity_sequence(declaration))
}

/// Complete structural validity relation for one declaration under one
/// separately supplied capacity record.
pub closed spec fn physical_plan_declaration_is_structurally_valid(
    declaration: &PhysicalPlanDeclaration,
    capacity: PhysicalCapacityExpectation,
) -> bool {
    let packet_count = declaration.packets@.len();
    &&& declaration.version == M1_PHYSICAL_PLAN_DECLARATION_VERSION
    &&& declaration.selection.valid()
    &&& declaration.logical_operation_count
        == crate::graph::plan_step_count_spec(declaration.selection.role)
    &&& global_identities_are_structurally_valid(declaration)
    &&& declaration.capacity_descriptor_id.bytes_spec()
        == capacity.descriptor_id.bytes_spec()
    &&& declaration.declared_batch_packet_capacity == capacity.batch_packet_capacity
    &&& declaration.declared_ring_packet_capacity == capacity.ring_packet_capacity
    &&& 0 < packet_count
    &&& packet_count <= capacity.batch_packet_capacity as int
    &&& packet_count <= capacity.ring_packet_capacity as int
    &&& forall|index: int|
        0 <= index < packet_count
            ==> physical_packet_is_structurally_valid_at(declaration, index)
    &&& packet_logical_end(declaration.packets@[packet_count - 1])
        == declaration.logical_operation_count as int
    &&& declaration.publication.reservation_count == 1
    &&& declaration.publication.reserved_packet_count as int == packet_count
    &&& declaration.publication.release_header_count as int == packet_count
    &&& declaration.publication.doorbell_count == 1
    &&& declaration.publication.doorbell_packet_index as int == packet_count - 1
    &&& declaration.completion.completion_packet_index as int == packet_count - 1
    &&& declaration.completion.completion_signal_count as int == packet_count
    &&& declaration.completion.declared_dominated_packet_count as int == packet_count
}

/// Exact header equality used for separately supplied expectation matching.
pub closed spec fn physical_plan_headers_match_exactly(
    declaration: &PhysicalPlanDeclaration,
    expected: &PhysicalPlanDeclaration,
) -> bool {
    &&& declaration.version == expected.version
    &&& declaration.declaration_id.bytes_spec() == expected.declaration_id.bytes_spec()
    &&& declaration.source_closure_id.bytes_spec() == expected.source_closure_id.bytes_spec()
    &&& declaration.logical_plan_id.bytes_spec() == expected.logical_plan_id.bytes_spec()
    &&& declaration.selection == expected.selection
    &&& declaration.logical_operation_count == expected.logical_operation_count
    &&& declaration.capacity_descriptor_id.bytes_spec()
        == expected.capacity_descriptor_id.bytes_spec()
    &&& declaration.declared_batch_packet_capacity == expected.declared_batch_packet_capacity
    &&& declaration.declared_ring_packet_capacity == expected.declared_ring_packet_capacity
    &&& declaration.publication.contract_id.bytes_spec()
        == expected.publication.contract_id.bytes_spec()
    &&& declaration.publication.reservation_count == expected.publication.reservation_count
    &&& declaration.publication.reserved_packet_count
        == expected.publication.reserved_packet_count
    &&& declaration.publication.release_header_count
        == expected.publication.release_header_count
    &&& declaration.publication.doorbell_count == expected.publication.doorbell_count
    &&& declaration.publication.doorbell_packet_index
        == expected.publication.doorbell_packet_index
    &&& declaration.completion.contract_id.bytes_spec()
        == expected.completion.contract_id.bytes_spec()
    &&& declaration.completion.completion_packet_index
        == expected.completion.completion_packet_index
    &&& declaration.completion.completion_signal_count
        == expected.completion.completion_signal_count
    &&& declaration.completion.declared_dominated_packet_count
        == expected.completion.declared_dominated_packet_count
}

closed spec fn packet_identity_bindings_match_exactly(
    declaration: PhysicalPacketIdentityBinding,
    expected: PhysicalPacketIdentityBinding,
) -> bool {
    &&& declaration.kernel_contract_id.bytes_spec()
        == expected.kernel_contract_id.bytes_spec()
    &&& declaration.artifact_id.bytes_spec() == expected.artifact_id.bytes_spec()
    &&& declaration.descriptor_id.bytes_spec() == expected.descriptor_id.bytes_spec()
    &&& declaration.geometry_id.bytes_spec() == expected.geometry_id.bytes_spec()
    &&& declaration.kernarg_layout_id.bytes_spec()
        == expected.kernarg_layout_id.bytes_spec()
    &&& declaration.buffer_layout_id.bytes_spec() == expected.buffer_layout_id.bytes_spec()
    &&& declaration.effect_contract_id.bytes_spec()
        == expected.effect_contract_id.bytes_spec()
}

closed spec fn fusion_declarations_match_exactly(
    declaration: Option<DeclaredFusionRefinementPremise>,
    expected: Option<DeclaredFusionRefinementPremise>,
) -> bool {
    match (declaration, expected) {
        (None, None) => true,
        (Some(declaration), Some(expected)) => {
            declaration.relation_id.bytes_spec() == expected.relation_id.bytes_spec()
                && declaration.evidence_requirement_id.bytes_spec()
                    == expected.evidence_requirement_id.bytes_spec()
        },
        _ => false,
    }
}

/// Exact equality relation for one expected packet declaration.
pub closed spec fn physical_packet_declarations_match_exactly(
    declaration: &PhysicalPacketSpanDeclaration,
    expected: &PhysicalPacketSpanDeclaration,
) -> bool {
    &&& declaration.packet_index == expected.packet_index
    &&& declaration.logical_start == expected.logical_start
    &&& declaration.logical_count == expected.logical_count
    &&& packet_identity_bindings_match_exactly(declaration.identities, expected.identities)
    &&& declaration.predecessors@ == expected.predecessors@
    &&& fusion_declarations_match_exactly(declaration.fusion, expected.fusion)
}

/// Exact equality relation for the complete expected packet sequence.
pub closed spec fn physical_plan_packets_match_exactly(
    declaration: &PhysicalPlanDeclaration,
    expected: &PhysicalPlanDeclaration,
) -> bool {
    &&& declaration.packets@.len() == expected.packets@.len()
    &&& forall|index: int|
        0 <= index < declaration.packets@.len()
            ==> #[trigger] physical_packet_declarations_match_exactly(
                &declaration.packets@[index],
                &expected.packets@[index],
            )
}

/// Complete exact candidate/expectation validation relation.
pub closed spec fn physical_plan_matches_expectation_exactly(
    declaration: &PhysicalPlanDeclaration,
    expectation: &PhysicalPlanExpectation,
) -> bool {
    &&& physical_capacity_expectation_is_structurally_valid(expectation.capacity)
    &&& physical_plan_declaration_is_structurally_valid(
        &expectation.expected,
        expectation.capacity,
    )
    &&& physical_plan_declaration_is_structurally_valid(declaration, expectation.capacity)
    &&& physical_plan_headers_match_exactly(declaration, &expectation.expected)
    &&& physical_plan_packets_match_exactly(declaration, &expectation.expected)
}

fn identity_present(
    identity: &Identity,
    role: PhysicalIdentityRole,
    packet_index: Option<u32>,
) -> (result: Result<(), PhysicalPlanError>)
    ensures result.is_ok() == physical_identity_is_present(*identity),
{
    proof {
        reveal(physical_identity_is_present);
    }
    if !identity.is_present() {
        return Err(PhysicalPlanError::MissingIdentity { role, packet_index });
    }
    Ok(())
}

fn identities_are_distinct(identities: &[Identity]) -> (distinct: bool)
    ensures distinct == physical_identities_are_distinct(identities@),
{
    let mut index = 0usize;
    while index < identities.len()
        invariant
            index <= identities@.len(),
            forall|left: int, right: int|
                0 <= left < right < index
                    ==> identities@[left].bytes_spec() != identities@[right].bytes_spec(),
        decreases identities@.len() - index,
    {
        let mut prior = 0usize;
        while prior < index
            invariant
                prior <= index,
                index < identities@.len(),
                forall|left: int, right: int|
                    0 <= left < right < index
                        ==> identities@[left].bytes_spec()
                            != identities@[right].bytes_spec(),
                forall|left: int|
                    0 <= left < prior
                        ==> identities@[left].bytes_spec()
                            != identities@[index as int].bytes_spec(),
            decreases index - prior,
        {
            if identities[index].equals(&identities[prior]) {
                assert(!physical_identities_are_distinct(identities@)) by {
                    reveal(physical_identities_are_distinct);
                }
                return false;
            }
            prior += 1;
        }
        assert forall|left: int, right: int|
            0 <= left < right < index + 1
                implies identities@[left].bytes_spec()
                    != identities@[right].bytes_spec() by {
            if right < index {
                assert(identities@[left].bytes_spec()
                    != identities@[right].bytes_spec());
            } else {
                assert(right == index);
                assert(identities@[left].bytes_spec()
                    != identities@[index as int].bytes_spec());
            }
        }
        index += 1;
    }
    assert(physical_identities_are_distinct(identities@)) by {
        reveal(physical_identities_are_distinct);
    }
    true
}

fn declared_ring_capacity_valid(value: u32) -> (valid: bool)
    ensures valid == declared_ring_capacity_is_valid(value),
{
    proof {
        reveal(declared_ring_capacity_is_valid);
    }
    matches!(
        value,
        64 | 128 | 256 | 512 | 1_024 | 2_048 | 4_096 | 8_192 | 16_384 | 32_768
            | 65_536
            | 131_072
            | 262_144
            | 524_288
            | 1_048_576
            | 2_097_152
            | 4_194_304
            | 8_388_608
            | 16_777_216
            | 33_554_432
    )
}

fn validate_capacity_expectation(
    capacity: PhysicalCapacityExpectation,
) -> (result: Result<(), PhysicalPlanError>)
    ensures
        result.is_ok() == physical_capacity_expectation_is_structurally_valid(capacity),
{
    proof {
        reveal(physical_capacity_expectation_is_structurally_valid);
    }
    identity_present(
        &capacity.descriptor_id,
        PhysicalIdentityRole::CapacityDescriptor,
        None,
    )?;
    if capacity.batch_packet_capacity == 0
        || capacity.batch_packet_capacity > M1_MAX_UNTRUSTED_PACKET_CAPACITY_V1
        || !declared_ring_capacity_valid(capacity.ring_packet_capacity)
        || capacity.batch_packet_capacity > capacity.ring_packet_capacity
    {
        return Err(PhysicalPlanError::InvalidCapacity);
    }
    let reviewed_capacity = match capacity.source {
        PhysicalCapacitySource::ReviewedBatchArithmeticV1 => {
            Some(M1_REVIEWED_BATCH_PACKET_CAPACITY_V1)
        }
        PhysicalCapacitySource::ReviewedBatchArithmeticV2 => {
            Some(M1_REVIEWED_BATCH_PACKET_CAPACITY_V2)
        }
        PhysicalCapacitySource::FutureUntrusted => None,
    };
    match reviewed_capacity {
        Some(expected) if capacity.batch_packet_capacity != expected => {
            return Err(PhysicalPlanError::InvalidCapacity);
        }
        _ => {}
    }
    Ok(())
}

fn validate_packet_identities(
    packet: &PhysicalPacketSpanDeclaration,
) -> (result: Result<(), PhysicalPlanError>)
    ensures
        result.is_ok()
            == packet_identity_binding_is_structurally_valid(packet.identities),
{
    proof {
        reveal(packet_identity_binding_is_structurally_valid);
        reveal(packet_identity_sequence);
    }
    let packet_index = Some(packet.packet_index);
    identity_present(
        &packet.identities.kernel_contract_id,
        PhysicalIdentityRole::KernelContract,
        packet_index,
    )?;
    identity_present(
        &packet.identities.artifact_id,
        PhysicalIdentityRole::Artifact,
        packet_index,
    )?;
    identity_present(
        &packet.identities.descriptor_id,
        PhysicalIdentityRole::Descriptor,
        packet_index,
    )?;
    identity_present(
        &packet.identities.geometry_id,
        PhysicalIdentityRole::Geometry,
        packet_index,
    )?;
    identity_present(
        &packet.identities.kernarg_layout_id,
        PhysicalIdentityRole::KernargLayout,
        packet_index,
    )?;
    identity_present(
        &packet.identities.buffer_layout_id,
        PhysicalIdentityRole::BufferLayout,
        packet_index,
    )?;
    identity_present(
        &packet.identities.effect_contract_id,
        PhysicalIdentityRole::EffectContract,
        packet_index,
    )?;
    let identity_values = [
        packet.identities.kernel_contract_id,
        packet.identities.artifact_id,
        packet.identities.descriptor_id,
        packet.identities.geometry_id,
        packet.identities.kernarg_layout_id,
        packet.identities.buffer_layout_id,
        packet.identities.effect_contract_id,
    ];
    assert(identity_values@ =~= packet_identity_sequence(packet.identities));
    if !identities_are_distinct(&identity_values) {
        return Err(PhysicalPlanError::ReusedIdentity { packet_index });
    }
    Ok(())
}

fn validate_fusion(
    packet: &PhysicalPacketSpanDeclaration,
) -> (result: Result<(), PhysicalPlanError>)
    ensures
        result.is_ok() == packet_fusion_declaration_is_structurally_valid(
            packet.logical_count,
            packet.identities,
            packet.fusion,
        ),
{
    proof {
        reveal(packet_fusion_declaration_is_structurally_valid);
        reveal(fused_identity_sequence);
        reveal(packet_identity_sequence);
    }
    match (packet.logical_count, packet.fusion) {
        (1, None) => Ok(()),
        (1, Some(_)) => Err(PhysicalPlanError::DirectSpanHasFusionPremise {
            packet_index: packet.packet_index,
        }),
        (_, None) => Err(PhysicalPlanError::FusionPremiseMissing {
            packet_index: packet.packet_index,
        }),
        (_, Some(premise)) => {
            identity_present(
                &premise.relation_id,
                PhysicalIdentityRole::FusionRelation,
                Some(packet.packet_index),
            )?;
            identity_present(
                &premise.evidence_requirement_id,
                PhysicalIdentityRole::FusionEvidenceRequirement,
                Some(packet.packet_index),
            )?;
            let identities = [
                packet.identities.kernel_contract_id,
                packet.identities.artifact_id,
                packet.identities.descriptor_id,
                packet.identities.geometry_id,
                packet.identities.kernarg_layout_id,
                packet.identities.buffer_layout_id,
                packet.identities.effect_contract_id,
                premise.relation_id,
                premise.evidence_requirement_id,
            ];
            assert(identities@ =~= fused_identity_sequence(packet.identities, premise));
            if !identities_are_distinct(&identities) {
                return Err(PhysicalPlanError::InvalidFusionPremise {
                    packet_index: packet.packet_index,
                });
            }
            Ok(())
        }
    }
}

fn validate_predecessors(
    packet: &PhysicalPacketSpanDeclaration,
) -> (result: Result<(), PhysicalPlanError>)
    ensures
        result.is_ok() == packet_predecessors_are_structurally_valid(
            packet.packet_index,
            packet.predecessors@,
        ),
{
    proof {
        reveal(packet_predecessors_are_structurally_valid);
    }
    let mut index = 0usize;
    while index < packet.predecessors.len()
        invariant
            index <= packet.predecessors@.len(),
            forall|prior: int|
                0 <= prior < index
                    ==> packet.predecessors@[prior] < packet.packet_index,
            forall|prior: int|
                0 < prior < index
                    ==> packet.predecessors@[prior - 1]
                        < #[trigger] packet.predecessors@[prior],
        decreases packet.predecessors@.len() - index,
    {
        let predecessor = packet.predecessors[index];
        let regressed = index > 0
            && predecessor <= packet.predecessors[index - 1];
        if predecessor >= packet.packet_index || regressed {
            return Err(PhysicalPlanError::InvalidPredecessor {
                packet_index: packet.packet_index,
            });
        }
        index += 1;
    }
    if packet.packet_index == 0 {
        if !packet.predecessors.is_empty() {
            return Err(PhysicalPlanError::InvalidPredecessor { packet_index: 0 });
        }
    } else {
        let required = packet.packet_index - 1;
        let last_predecessor = if packet.predecessors.is_empty() {
            None
        } else {
            Some(packet.predecessors[packet.predecessors.len() - 1])
        };
        if last_predecessor != Some(required) {
            return Err(PhysicalPlanError::CompletionDependencyMissing {
                packet_index: packet.packet_index,
                required_predecessor: required,
            });
        }
    }
    Ok(())
}

proof fn invalid_packet_refutes_physical_plan(
    declaration: &PhysicalPlanDeclaration,
    capacity: PhysicalCapacityExpectation,
    index: int,
)
    requires
        0 <= index < declaration.packets@.len(),
        !physical_packet_is_structurally_valid_at(declaration, index),
    ensures
        !physical_plan_declaration_is_structurally_valid(declaration, capacity),
{
    reveal(physical_plan_declaration_is_structurally_valid);
    if physical_plan_declaration_is_structurally_valid(declaration, capacity) {
        assert(physical_packet_is_structurally_valid_at(declaration, index));
        assert(false);
    }
}

fn validate_structure(
    declaration: &PhysicalPlanDeclaration,
    capacity: PhysicalCapacityExpectation,
) -> (result: Result<(), PhysicalPlanError>)
    ensures
        result.is_ok()
            == physical_plan_declaration_is_structurally_valid(declaration, capacity),
{
    proof {
        reveal(physical_plan_declaration_is_structurally_valid);
        reveal(global_identities_are_structurally_valid);
        reveal(global_identity_sequence);
        reveal(physical_packet_is_structurally_valid_at);
        reveal(packet_logical_end);
    }
    if declaration.version != M1_PHYSICAL_PLAN_DECLARATION_VERSION {
        return Err(PhysicalPlanError::UnsupportedVersion);
    }
    if let Err(error) = declaration.selection.validate() {
        return Err(PhysicalPlanError::Selection(error));
    }
    let expected_operations = plan_step_count(declaration.selection.role);
    if declaration.logical_operation_count != expected_operations {
        return Err(PhysicalPlanError::LogicalOperationCount {
            expected: expected_operations,
            actual: declaration.logical_operation_count,
        });
    }
    identity_present(
        &declaration.declaration_id,
        PhysicalIdentityRole::Declaration,
        None,
    )?;
    identity_present(
        &declaration.source_closure_id,
        PhysicalIdentityRole::SourceClosure,
        None,
    )?;
    identity_present(
        &declaration.logical_plan_id,
        PhysicalIdentityRole::LogicalPlan,
        None,
    )?;
    identity_present(
        &declaration.capacity_descriptor_id,
        PhysicalIdentityRole::CapacityDescriptor,
        None,
    )?;
    identity_present(
        &declaration.publication.contract_id,
        PhysicalIdentityRole::PublicationContract,
        None,
    )?;
    identity_present(
        &declaration.completion.contract_id,
        PhysicalIdentityRole::CompletionContract,
        None,
    )?;
    let global_identity_values = [
        declaration.declaration_id,
        declaration.source_closure_id,
        declaration.logical_plan_id,
        declaration.capacity_descriptor_id,
        declaration.publication.contract_id,
        declaration.completion.contract_id,
    ];
    assert(global_identity_values@ =~= global_identity_sequence(declaration));
    if !identities_are_distinct(&global_identity_values) {
        return Err(PhysicalPlanError::ReusedIdentity { packet_index: None });
    }
    if !declaration
        .capacity_descriptor_id
        .equals(&capacity.descriptor_id)
        || declaration.declared_batch_packet_capacity != capacity.batch_packet_capacity
        || declaration.declared_ring_packet_capacity != capacity.ring_packet_capacity
    {
        return Err(PhysicalPlanError::CapacityDescriptorDrift);
    }
    if declaration.packets.is_empty() {
        return Err(PhysicalPlanError::EmptyPacketSet);
    }
    let packet_count = match u32::try_from(declaration.packets.len()) {
        Ok(count) => count,
        Err(_) => return Err(PhysicalPlanError::InvalidCapacity),
    };
    if packet_count > capacity.batch_packet_capacity {
        return Err(PhysicalPlanError::PacketCountExceedsCapacity {
            packet_count,
            capacity: capacity.batch_packet_capacity,
        });
    }
    if packet_count > capacity.ring_packet_capacity {
        return Err(PhysicalPlanError::PacketCountExceedsRing {
            packet_count,
            ring_capacity: capacity.ring_packet_capacity,
        });
    }

    let mut expected_start = 0u32;
    let mut index = 0usize;
    while index < declaration.packets.len()
        invariant
            index <= declaration.packets@.len(),
            declaration.packets@.len() <= capacity.batch_packet_capacity as int,
            declaration.packets@.len() <= capacity.ring_packet_capacity as int,
            expected_start <= declaration.logical_operation_count,
            index == 0 ==> expected_start == 0,
            index > 0 ==> expected_start as int
                == packet_logical_end(declaration.packets@[index as int - 1]),
            forall|prior: int|
                0 <= prior < index
                    ==> physical_packet_is_structurally_valid_at(declaration, prior),
        decreases declaration.packets@.len() - index,
    {
        let packet = &declaration.packets[index];
        let expected_index = match u32::try_from(index) {
            Ok(expected_index) => expected_index,
            Err(_) => {
                assert(packet.packet_index as int != index as int);
                assert(!physical_packet_is_structurally_valid_at(
                    declaration,
                    index as int,
                ));
                proof {
                    invalid_packet_refutes_physical_plan(declaration, capacity, index as int);
                }
                return Err(PhysicalPlanError::InvalidCapacity);
            },
        };
        if packet.packet_index != expected_index {
            assert(!physical_packet_is_structurally_valid_at(
                declaration,
                index as int,
            ));
            proof {
                invalid_packet_refutes_physical_plan(declaration, capacity, index as int);
            }
            return Err(PhysicalPlanError::PacketIndexDrift {
                expected: expected_index,
                actual: packet.packet_index,
            });
        }
        if packet.logical_count == 0 {
            assert(!physical_packet_is_structurally_valid_at(
                declaration,
                index as int,
            ));
            proof {
                invalid_packet_refutes_physical_plan(declaration, capacity, index as int);
            }
            return Err(PhysicalPlanError::InvalidLogicalSpan {
                packet_index: packet.packet_index,
            });
        }
        if packet.logical_start != expected_start {
            assert(!physical_packet_is_structurally_valid_at(
                declaration,
                index as int,
            ));
            proof {
                invalid_packet_refutes_physical_plan(declaration, capacity, index as int);
            }
            return Err(PhysicalPlanError::LogicalCoverageDrift {
                packet_index: packet.packet_index,
                expected_start,
                actual_start: packet.logical_start,
            });
        }
        expected_start = match packet.logical_start.checked_add(packet.logical_count) {
            Some(next) => next,
            None => {
                assert(!physical_packet_is_structurally_valid_at(
                    declaration,
                    index as int,
                ));
                proof {
                    invalid_packet_refutes_physical_plan(declaration, capacity, index as int);
                }
                return Err(PhysicalPlanError::InvalidLogicalSpan {
                    packet_index: packet.packet_index,
                });
            },
        };
        if expected_start > declaration.logical_operation_count {
            assert(!physical_packet_is_structurally_valid_at(
                declaration,
                index as int,
            ));
            proof {
                invalid_packet_refutes_physical_plan(declaration, capacity, index as int);
            }
            return Err(PhysicalPlanError::InvalidLogicalSpan {
                packet_index: packet.packet_index,
            });
        }
        if let Err(error) = validate_packet_identities(packet) {
            assert(!physical_packet_is_structurally_valid_at(
                declaration,
                index as int,
            ));
            proof {
                invalid_packet_refutes_physical_plan(declaration, capacity, index as int);
            }
            return Err(error);
        }
        if let Err(error) = validate_fusion(packet) {
            assert(!physical_packet_is_structurally_valid_at(
                declaration,
                index as int,
            ));
            proof {
                invalid_packet_refutes_physical_plan(declaration, capacity, index as int);
            }
            return Err(error);
        }
        if let Err(error) = validate_predecessors(packet) {
            assert(!physical_packet_is_structurally_valid_at(
                declaration,
                index as int,
            ));
            proof {
                invalid_packet_refutes_physical_plan(declaration, capacity, index as int);
            }
            return Err(error);
        }
        assert(physical_packet_is_structurally_valid_at(
            declaration,
            index as int,
        ));
        index += 1;
    }
    if expected_start != declaration.logical_operation_count {
        return Err(PhysicalPlanError::LogicalCoverageEnd {
            expected: declaration.logical_operation_count,
            actual: expected_start,
        });
    }

    let final_packet = packet_count - 1;
    if declaration.publication.reservation_count != 1
        || declaration.publication.reserved_packet_count != packet_count
        || declaration.publication.release_header_count != packet_count
        || declaration.publication.doorbell_count != 1
        || declaration.publication.doorbell_packet_index != final_packet
    {
        return Err(PhysicalPlanError::PublicationDrift);
    }
    if declaration.completion.completion_packet_index != final_packet
        || declaration.completion.completion_signal_count != packet_count
        || declaration.completion.declared_dominated_packet_count != packet_count
    {
        return Err(PhysicalPlanError::CompletionDrift);
    }
    Ok(())
}

fn validate_expected_header(
    declaration: &PhysicalPlanDeclaration,
    expected: &PhysicalPlanDeclaration,
) -> (result: Result<(), PhysicalPlanError>)
    ensures result.is_ok() == physical_plan_headers_match_exactly(declaration, expected),
{
    proof {
        reveal(physical_plan_headers_match_exactly);
    }
    if declaration.version != expected.version
        || !declaration.declaration_id.equals(&expected.declaration_id)
        || !declaration
            .source_closure_id
            .equals(&expected.source_closure_id)
        || !declaration.logical_plan_id.equals(&expected.logical_plan_id)
        || !declaration.selection.matches(expected.selection)
        || declaration.logical_operation_count != expected.logical_operation_count
        || !declaration
            .capacity_descriptor_id
            .equals(&expected.capacity_descriptor_id)
        || declaration.declared_batch_packet_capacity
            != expected.declared_batch_packet_capacity
        || declaration.declared_ring_packet_capacity != expected.declared_ring_packet_capacity
        || !declaration
            .publication
            .contract_id
            .equals(&expected.publication.contract_id)
        || declaration.publication.reservation_count != expected.publication.reservation_count
        || declaration.publication.reserved_packet_count
            != expected.publication.reserved_packet_count
        || declaration.publication.release_header_count
            != expected.publication.release_header_count
        || declaration.publication.doorbell_count != expected.publication.doorbell_count
        || declaration.publication.doorbell_packet_index
            != expected.publication.doorbell_packet_index
        || !declaration
            .completion
            .contract_id
            .equals(&expected.completion.contract_id)
        || declaration.completion.completion_packet_index
            != expected.completion.completion_packet_index
        || declaration.completion.completion_signal_count
            != expected.completion.completion_signal_count
        || declaration.completion.declared_dominated_packet_count
            != expected.completion.declared_dominated_packet_count
    {
        return Err(PhysicalPlanError::ExpectedHeaderDrift);
    }
    Ok(())
}

fn packet_identity_bindings_match(
    declaration: PhysicalPacketIdentityBinding,
    expected: PhysicalPacketIdentityBinding,
) -> (matches: bool)
    ensures matches == packet_identity_bindings_match_exactly(declaration, expected),
{
    proof {
        reveal(packet_identity_bindings_match_exactly);
    }
    declaration
        .kernel_contract_id
        .equals(&expected.kernel_contract_id)
        && declaration.artifact_id.equals(&expected.artifact_id)
        && declaration.descriptor_id.equals(&expected.descriptor_id)
        && declaration.geometry_id.equals(&expected.geometry_id)
        && declaration
            .kernarg_layout_id
            .equals(&expected.kernarg_layout_id)
        && declaration
            .buffer_layout_id
            .equals(&expected.buffer_layout_id)
        && declaration
            .effect_contract_id
            .equals(&expected.effect_contract_id)
}

fn fusion_declarations_match(
    declaration: Option<DeclaredFusionRefinementPremise>,
    expected: Option<DeclaredFusionRefinementPremise>,
) -> (matches: bool)
    ensures matches == fusion_declarations_match_exactly(declaration, expected),
{
    proof {
        reveal(fusion_declarations_match_exactly);
    }
    match (declaration, expected) {
        (None, None) => true,
        (Some(declaration), Some(expected)) => {
            declaration.relation_id.equals(&expected.relation_id)
                && declaration
                    .evidence_requirement_id
                    .equals(&expected.evidence_requirement_id)
        },
        _ => false,
    }
}

fn predecessor_sequences_match(
    declaration: &[u32],
    expected: &[u32],
) -> (matches: bool)
    ensures matches == (declaration@ == expected@),
{
    if declaration.len() != expected.len() {
        return false;
    }
    let mut index = 0usize;
    while index < declaration.len()
        invariant
            index <= declaration@.len(),
            declaration@.len() == expected@.len(),
            forall|prior: int|
                0 <= prior < index ==> declaration@[prior] == expected@[prior],
        decreases declaration@.len() - index,
    {
        if declaration[index] != expected[index] {
            assert(declaration@ != expected@) by {
                if declaration@ == expected@ {
                    assert(declaration@[index as int] == expected@[index as int]);
                    assert(false);
                }
            }
            return false;
        }
        index += 1;
    }
    assert(declaration@ =~= expected@);
    true
}

fn physical_packet_declarations_match(
    declaration: &PhysicalPacketSpanDeclaration,
    expected: &PhysicalPacketSpanDeclaration,
) -> (matches: bool)
    ensures matches == physical_packet_declarations_match_exactly(declaration, expected),
{
    proof {
        reveal(physical_packet_declarations_match_exactly);
    }
    declaration.packet_index == expected.packet_index
        && declaration.logical_start == expected.logical_start
        && declaration.logical_count == expected.logical_count
        && packet_identity_bindings_match(declaration.identities, expected.identities)
        && predecessor_sequences_match(&declaration.predecessors, &expected.predecessors)
        && fusion_declarations_match(declaration.fusion, expected.fusion)
}

fn validate_expected_packets(
    declaration: &PhysicalPlanDeclaration,
    expected: &PhysicalPlanDeclaration,
) -> (result: Result<(), PhysicalPlanError>)
    ensures result.is_ok() == physical_plan_packets_match_exactly(declaration, expected),
{
    proof {
        reveal(physical_plan_packets_match_exactly);
    }
    if declaration.packets.len() != expected.packets.len() {
        return Err(PhysicalPlanError::ExpectedHeaderDrift);
    }
    let mut index = 0usize;
    while index < declaration.packets.len()
        invariant
            index <= declaration.packets@.len(),
            declaration.packets@.len() == expected.packets@.len(),
            forall|prior: int|
                0 <= prior < index
                    ==> physical_packet_declarations_match_exactly(
                        &declaration.packets@[prior],
                        &expected.packets@[prior],
                    ),
        decreases declaration.packets@.len() - index,
    {
        let actual = &declaration.packets[index];
        let expected_packet = &expected.packets[index];
        if !physical_packet_declarations_match(actual, expected_packet) {
            assert(!physical_plan_packets_match_exactly(declaration, expected)) by {
                if physical_plan_packets_match_exactly(declaration, expected) {
                    assert(physical_packet_declarations_match_exactly(
                        &declaration.packets@[index as int],
                        &expected.packets@[index as int],
                    ));
                    assert(false);
                }
            }
            let packet_index = match u32::try_from(index) {
                Ok(packet_index) => packet_index,
                Err(_) => return Err(PhysicalPlanError::InvalidCapacity),
            };
            return Err(PhysicalPlanError::ExpectedPacketDrift { packet_index });
        }
        index += 1;
    }
    Ok(())
}

/// Validates an inert candidate against one separately supplied expectation.
///
/// # Errors
///
/// Returns [`PhysicalPlanError`] unless both records are structurally valid,
/// all exact fields match, logical spans form an ordered total partition,
/// every fused span names two distinct unproved premise identities, the final
/// packet is dependency-dominating under the conservative chain rule, and the
/// declaration names exactly one reservation and one doorbell.
///
/// The postcondition covers structural acceptance only. It does not prove any
/// declared fusion premise or grant physical execution authority.
pub fn validate_physical_plan_declaration(
    declaration: PhysicalPlanDeclaration,
    expectation: &PhysicalPlanExpectation,
) -> (result: Result<StructurallyValidatedPhysicalPlan, PhysicalPlanError>)
    ensures
        result.is_ok() == physical_plan_matches_expectation_exactly(
            &declaration,
            expectation,
        ),
        match result {
            Ok(validated) => {
                &&& validated.declaration_spec() == declaration
                &&& validated.capacity_source_spec() == expectation.capacity.source
            },
            Err(_) => true,
        },
{
    proof {
        reveal(physical_plan_matches_expectation_exactly);
    }
    validate_capacity_expectation(expectation.capacity)?;
    if validate_structure(&expectation.expected, expectation.capacity).is_err() {
        return Err(PhysicalPlanError::InvalidExpectation);
    }
    validate_structure(&declaration, expectation.capacity)?;
    validate_expected_header(&declaration, &expectation.expected)?;
    validate_expected_packets(&declaration, &expectation.expected)?;
    Ok(StructurallyValidatedPhysicalPlan {
        declaration,
        capacity_source: expectation.capacity.source,
    })
}

/// Query-bearing wrapper for the exact inert structural validation theorem.
///
/// This wrapper is deliberately absent from the finite M1 property/path
/// registry because that registry has no physical-declaration association.
/// It does not authenticate identities, discharge fusion premises, or grant
/// packet construction, launch, publication, or completion authority.
///
/// # Errors
///
/// Returns [`PhysicalPlanError`] exactly when the closed structural validity
/// and expectation relation does not hold.
pub fn physical_plan_structural_validation_theorem(
    declaration: PhysicalPlanDeclaration,
    expectation: &PhysicalPlanExpectation,
) -> (result: Result<StructurallyValidatedPhysicalPlan, PhysicalPlanError>)
    ensures
        result.is_ok() == physical_plan_matches_expectation_exactly(
            &declaration,
            expectation,
        ),
        match result {
            Ok(validated) => {
                &&& validated.declaration_spec() == declaration
                &&& validated.capacity_source_spec() == expectation.capacity.source
            },
            Err(_) => true,
        },
{
    validate_physical_plan_declaration(declaration, expectation)
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{
        physical_plan_structural_validation_theorem, validate_physical_plan_declaration,
        DeclaredFusionRefinementPremise, PhysicalCapacityExpectation, PhysicalCapacitySource,
        PhysicalCompletionDeclaration, PhysicalIdentityRole, PhysicalPacketIdentityBinding,
        PhysicalPacketSpanDeclaration, PhysicalPlanDeclaration, PhysicalPlanError,
        PhysicalPlanExpectation, PhysicalPublicationDeclaration,
        M1_PHYSICAL_PLAN_DECLARATION_VERSION, M1_REVIEWED_BATCH_PACKET_CAPACITY_V1,
        M1_REVIEWED_BATCH_PACKET_CAPACITY_V2,
    };
    use crate::{
        Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection,
        QWEN3_DRAFT_PLAN_STEPS, QWEN3_TARGET_PLAN_STEPS,
    };

    fn identity(role: u8, index: u32) -> Identity {
        let mut bytes = [0u8; 32];
        bytes[0] = role;
        bytes[1..5].copy_from_slice(&index.to_le_bytes());
        Identity::new(bytes)
    }

    const fn target_selection() -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        }
    }

    const fn draft_selection() -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        }
    }

    fn capacity(
        source: PhysicalCapacitySource,
        maximum: u32,
        ring: u32,
    ) -> PhysicalCapacityExpectation {
        PhysicalCapacityExpectation {
            source,
            descriptor_id: identity(10, maximum),
            batch_packet_capacity: maximum,
            ring_packet_capacity: ring,
        }
    }

    fn packet(index: u32, logical_start: u32, logical_count: u32) -> PhysicalPacketSpanDeclaration {
        PhysicalPacketSpanDeclaration {
            packet_index: index,
            logical_start,
            logical_count,
            identities: PhysicalPacketIdentityBinding {
                kernel_contract_id: identity(20, index),
                artifact_id: identity(21, index),
                descriptor_id: identity(22, index),
                geometry_id: identity(23, index),
                kernarg_layout_id: identity(24, index),
                buffer_layout_id: identity(25, index),
                effect_contract_id: identity(26, index),
            },
            predecessors: if index == 0 {
                Vec::new()
            } else {
                vec![index - 1]
            },
            fusion: (logical_count > 1).then_some(DeclaredFusionRefinementPremise {
                relation_id: identity(27, index),
                evidence_requirement_id: identity(28, index),
            }),
        }
    }

    // Synthetic test data only. The generated fusion IDs name no evidence.
    fn synthetic_unproved_declaration(
        selection: Qwen3PlanSelection,
        logical_count: u32,
        span_width: u32,
        capacity: PhysicalCapacityExpectation,
    ) -> PhysicalPlanDeclaration {
        assert!(span_width > 0);
        let mut packets = Vec::new();
        let mut logical_start = 0;
        while logical_start < logical_count {
            let count = span_width.min(logical_count - logical_start);
            packets.push(packet(
                u32::try_from(packets.len()).unwrap(),
                logical_start,
                count,
            ));
            logical_start += count;
        }
        let packet_count = u32::try_from(packets.len()).unwrap();
        PhysicalPlanDeclaration {
            version: M1_PHYSICAL_PLAN_DECLARATION_VERSION,
            declaration_id: identity(1, logical_count),
            source_closure_id: identity(2, logical_count),
            logical_plan_id: identity(3, logical_count),
            selection,
            logical_operation_count: logical_count,
            capacity_descriptor_id: capacity.descriptor_id,
            declared_batch_packet_capacity: capacity.batch_packet_capacity,
            declared_ring_packet_capacity: capacity.ring_packet_capacity,
            packets,
            publication: PhysicalPublicationDeclaration {
                contract_id: identity(4, logical_count),
                reservation_count: 1,
                reserved_packet_count: packet_count,
                release_header_count: packet_count,
                doorbell_count: 1,
                doorbell_packet_index: packet_count - 1,
            },
            completion: PhysicalCompletionDeclaration {
                contract_id: identity(5, logical_count),
                completion_packet_index: packet_count - 1,
                completion_signal_count: packet_count,
                declared_dominated_packet_count: packet_count,
            },
        }
    }

    fn expectation(
        declaration: &PhysicalPlanDeclaration,
        capacity: PhysicalCapacityExpectation,
    ) -> PhysicalPlanExpectation {
        PhysicalPlanExpectation {
            expected: declaration.clone(),
            capacity,
        }
    }

    #[test]
    fn explicit_fusion_partition_is_accepted_only_structurally() {
        let capacity = capacity(
            PhysicalCapacitySource::ReviewedBatchArithmeticV1,
            M1_REVIEWED_BATCH_PACKET_CAPACITY_V1,
            256,
        );
        let declaration =
            synthetic_unproved_declaration(draft_selection(), QWEN3_DRAFT_PLAN_STEPS, 2, capacity);
        let expected = expectation(&declaration, capacity);
        let validated = validate_physical_plan_declaration(declaration, &expected).unwrap();
        assert_eq!(validated.declaration().packets.len(), 212);
        assert_eq!(
            validated.capacity_source(),
            PhysicalCapacitySource::ReviewedBatchArithmeticV1
        );
    }

    #[test]
    fn direct_target_and_draft_exceed_reviewed_capacity() {
        let capacity = capacity(
            PhysicalCapacitySource::ReviewedBatchArithmeticV1,
            M1_REVIEWED_BATCH_PACKET_CAPACITY_V1,
            1_024,
        );
        for (selection, logical_count) in [
            (target_selection(), QWEN3_TARGET_PLAN_STEPS),
            (draft_selection(), QWEN3_DRAFT_PLAN_STEPS),
        ] {
            let declaration = synthetic_unproved_declaration(selection, logical_count, 1, capacity);
            let expected = expectation(&declaration, capacity);
            assert!(matches!(
                validate_physical_plan_declaration(declaration, &expected),
                Err(PhysicalPlanError::InvalidExpectation)
            ));
        }
    }

    #[test]
    fn direct_target_and_draft_fit_reviewed_v2_without_fusion() {
        let capacity = capacity(
            PhysicalCapacitySource::ReviewedBatchArithmeticV2,
            M1_REVIEWED_BATCH_PACKET_CAPACITY_V2,
            1_024,
        );
        for (selection, logical_count) in [
            (target_selection(), QWEN3_TARGET_PLAN_STEPS),
            (draft_selection(), QWEN3_DRAFT_PLAN_STEPS),
        ] {
            let declaration = synthetic_unproved_declaration(selection, logical_count, 1, capacity);
            assert!(declaration
                .packets
                .iter()
                .all(|packet| { packet.logical_count == 1 && packet.fusion.is_none() }));
            let expected = expectation(&declaration, capacity);
            let validated = validate_physical_plan_declaration(declaration, &expected)
                .expect("reviewed V2 retains one packet per generated operation");
            assert_eq!(
                validated.declaration().packets.len(),
                logical_count as usize
            );
            assert_eq!(
                validated.capacity_source(),
                PhysicalCapacitySource::ReviewedBatchArithmeticV2
            );
        }
    }

    #[test]
    fn omission_and_overlap_fail_closed() {
        let capacity = capacity(
            PhysicalCapacitySource::ReviewedBatchArithmeticV1,
            M1_REVIEWED_BATCH_PACKET_CAPACITY_V1,
            256,
        );
        let canonical =
            synthetic_unproved_declaration(draft_selection(), QWEN3_DRAFT_PLAN_STEPS, 2, capacity);
        let expected = expectation(&canonical, capacity);
        for changed_start in [3, 1] {
            let mut candidate = canonical.clone();
            candidate.packets[1].logical_start = changed_start;
            assert!(matches!(
                validate_physical_plan_declaration(candidate, &expected),
                Err(PhysicalPlanError::LogicalCoverageDrift {
                    packet_index: 1,
                    ..
                })
            ));
        }
    }

    #[test]
    fn reordered_packets_fail_closed() {
        let capacity = capacity(
            PhysicalCapacitySource::ReviewedBatchArithmeticV1,
            M1_REVIEWED_BATCH_PACKET_CAPACITY_V1,
            256,
        );
        let canonical =
            synthetic_unproved_declaration(draft_selection(), QWEN3_DRAFT_PLAN_STEPS, 2, capacity);
        let expected = expectation(&canonical, capacity);
        let mut candidate = canonical;
        candidate.packets.swap(0, 1);
        assert!(matches!(
            validate_physical_plan_declaration(candidate, &expected),
            Err(PhysicalPlanError::PacketIndexDrift {
                expected: 0,
                actual: 1
            })
        ));
    }

    #[test]
    fn fake_or_reused_fusion_premises_fail_closed() {
        let capacity = capacity(
            PhysicalCapacitySource::ReviewedBatchArithmeticV1,
            M1_REVIEWED_BATCH_PACKET_CAPACITY_V1,
            256,
        );
        let canonical =
            synthetic_unproved_declaration(draft_selection(), QWEN3_DRAFT_PLAN_STEPS, 2, capacity);
        let expected = expectation(&canonical, capacity);

        let mut missing = canonical.clone();
        missing.packets[0].fusion = None;
        assert_eq!(
            validate_physical_plan_declaration(missing, &expected),
            Err(PhysicalPlanError::FusionPremiseMissing { packet_index: 0 })
        );

        let mut substituted = canonical.clone();
        substituted.packets[0].fusion.as_mut().unwrap().relation_id = identity(29, 999);
        assert_eq!(
            validate_physical_plan_declaration(substituted, &expected),
            Err(PhysicalPlanError::ExpectedPacketDrift { packet_index: 0 })
        );

        let mut reused = canonical;
        reused.packets[0].fusion.as_mut().unwrap().relation_id =
            reused.packets[0].identities.artifact_id;
        assert_eq!(
            validate_physical_plan_declaration(reused, &expected),
            Err(PhysicalPlanError::InvalidFusionPremise { packet_index: 0 })
        );
    }

    #[test]
    fn direct_span_cannot_smuggle_a_fusion_premise() {
        let capacity = capacity(PhysicalCapacitySource::FutureUntrusted, 1_024, 1_024);
        let canonical = synthetic_unproved_declaration(
            target_selection(),
            QWEN3_TARGET_PLAN_STEPS,
            1,
            capacity,
        );
        let expected = expectation(&canonical, capacity);
        let mut candidate = canonical;
        candidate.packets[0].fusion = Some(DeclaredFusionRefinementPremise {
            relation_id: identity(27, 0),
            evidence_requirement_id: identity(28, 0),
        });
        assert_eq!(
            validate_physical_plan_declaration(candidate, &expected),
            Err(PhysicalPlanError::DirectSpanHasFusionPremise { packet_index: 0 })
        );
    }

    #[test]
    fn missing_completion_chain_dependency_fails_closed() {
        let capacity = capacity(
            PhysicalCapacitySource::ReviewedBatchArithmeticV1,
            M1_REVIEWED_BATCH_PACKET_CAPACITY_V1,
            256,
        );
        let canonical =
            synthetic_unproved_declaration(draft_selection(), QWEN3_DRAFT_PLAN_STEPS, 2, capacity);
        let expected = expectation(&canonical, capacity);
        let mut candidate = canonical;
        candidate.packets[7].predecessors.clear();
        assert_eq!(
            validate_physical_plan_declaration(candidate, &expected),
            Err(PhysicalPlanError::CompletionDependencyMissing {
                packet_index: 7,
                required_predecessor: 6,
            })
        );
    }

    #[test]
    fn capacity_and_ring_drift_fail_closed() {
        let capacity = capacity(
            PhysicalCapacitySource::ReviewedBatchArithmeticV1,
            M1_REVIEWED_BATCH_PACKET_CAPACITY_V1,
            256,
        );
        let canonical =
            synthetic_unproved_declaration(draft_selection(), QWEN3_DRAFT_PLAN_STEPS, 2, capacity);
        let expected = expectation(&canonical, capacity);

        let mut changed = canonical.clone();
        changed.declared_batch_packet_capacity = 512;
        assert_eq!(
            validate_physical_plan_declaration(changed, &expected),
            Err(PhysicalPlanError::CapacityDescriptorDrift)
        );

        let mut changed = canonical;
        changed.declared_ring_packet_capacity = 128;
        assert_eq!(
            validate_physical_plan_declaration(changed, &expected),
            Err(PhysicalPlanError::CapacityDescriptorDrift)
        );
    }

    #[test]
    fn publication_and_completion_drift_fail_closed() {
        let capacity = capacity(
            PhysicalCapacitySource::ReviewedBatchArithmeticV1,
            M1_REVIEWED_BATCH_PACKET_CAPACITY_V1,
            256,
        );
        let canonical =
            synthetic_unproved_declaration(draft_selection(), QWEN3_DRAFT_PLAN_STEPS, 2, capacity);
        let expected = expectation(&canonical, capacity);

        let mut publication = canonical.clone();
        publication.publication.reservation_count = 2;
        assert_eq!(
            validate_physical_plan_declaration(publication, &expected),
            Err(PhysicalPlanError::PublicationDrift)
        );

        let mut completion = canonical;
        completion.completion.completion_packet_index -= 1;
        assert_eq!(
            validate_physical_plan_declaration(completion, &expected),
            Err(PhysicalPlanError::CompletionDrift)
        );
    }

    #[test]
    fn exact_per_packet_identity_drift_fails_closed() {
        let capacity = capacity(
            PhysicalCapacitySource::ReviewedBatchArithmeticV1,
            M1_REVIEWED_BATCH_PACKET_CAPACITY_V1,
            256,
        );
        let canonical =
            synthetic_unproved_declaration(draft_selection(), QWEN3_DRAFT_PLAN_STEPS, 2, capacity);
        let expected = expectation(&canonical, capacity);
        let mut candidate = canonical;
        candidate.packets[11].identities.artifact_id = identity(21, 999);
        assert_eq!(
            validate_physical_plan_declaration(candidate, &expected),
            Err(PhysicalPlanError::ExpectedPacketDrift { packet_index: 11 })
        );
    }

    #[test]
    fn missing_packet_identity_fails_with_exact_role() {
        let capacity = capacity(
            PhysicalCapacitySource::ReviewedBatchArithmeticV1,
            M1_REVIEWED_BATCH_PACKET_CAPACITY_V1,
            256,
        );
        let canonical =
            synthetic_unproved_declaration(draft_selection(), QWEN3_DRAFT_PLAN_STEPS, 2, capacity);
        let expected = expectation(&canonical, capacity);
        let mut candidate = canonical;
        candidate.packets[3].identities.geometry_id = Identity::new([0; 32]);
        assert_eq!(
            validate_physical_plan_declaration(candidate, &expected),
            Err(PhysicalPlanError::MissingIdentity {
                role: PhysicalIdentityRole::Geometry,
                packet_index: Some(3),
            })
        );
    }

    #[test]
    fn future_capacity_remains_explicitly_untrusted() {
        let capacity = capacity(PhysicalCapacitySource::FutureUntrusted, 512, 512);
        let declaration =
            synthetic_unproved_declaration(draft_selection(), QWEN3_DRAFT_PLAN_STEPS, 2, capacity);
        let expected = expectation(&declaration, capacity);
        let validated = validate_physical_plan_declaration(declaration, &expected).unwrap();
        assert_eq!(
            validated.capacity_source(),
            PhysicalCapacitySource::FutureUntrusted
        );
    }

    #[test]
    fn capacity_expectation_boundaries_fail_closed() {
        for invalid_capacity in [
            capacity(PhysicalCapacitySource::FutureUntrusted, 0, 64),
            capacity(PhysicalCapacitySource::FutureUntrusted, 1_025, 2_048),
            capacity(PhysicalCapacitySource::FutureUntrusted, 128, 192),
            capacity(PhysicalCapacitySource::FutureUntrusted, 512, 256),
            capacity(PhysicalCapacitySource::ReviewedBatchArithmeticV1, 255, 256),
            capacity(
                PhysicalCapacitySource::ReviewedBatchArithmeticV2,
                1_023,
                1_024,
            ),
        ] {
            let declaration = synthetic_unproved_declaration(
                draft_selection(),
                QWEN3_DRAFT_PLAN_STEPS,
                2,
                invalid_capacity,
            );
            let expected = expectation(&declaration, invalid_capacity);
            assert_eq!(
                validate_physical_plan_declaration(declaration, &expected),
                Err(PhysicalPlanError::InvalidCapacity)
            );
        }

        let mut missing_descriptor = capacity(PhysicalCapacitySource::FutureUntrusted, 512, 512);
        missing_descriptor.descriptor_id = Identity::new([0; 32]);
        let declaration = synthetic_unproved_declaration(
            draft_selection(),
            QWEN3_DRAFT_PLAN_STEPS,
            2,
            missing_descriptor,
        );
        let expected = expectation(&declaration, missing_descriptor);
        assert_eq!(
            validate_physical_plan_declaration(declaration, &expected),
            Err(PhysicalPlanError::MissingIdentity {
                role: PhysicalIdentityRole::CapacityDescriptor,
                packet_index: None,
            })
        );
    }

    #[test]
    fn predecessor_regression_and_exact_header_drift_fail_closed() {
        let capacity = capacity(
            PhysicalCapacitySource::ReviewedBatchArithmeticV1,
            M1_REVIEWED_BATCH_PACKET_CAPACITY_V1,
            256,
        );
        let canonical =
            synthetic_unproved_declaration(draft_selection(), QWEN3_DRAFT_PLAN_STEPS, 2, capacity);
        let expected = expectation(&canonical, capacity);

        let mut regressed = canonical.clone();
        regressed.packets[7].predecessors = vec![5, 5, 6];
        assert_eq!(
            validate_physical_plan_declaration(regressed, &expected),
            Err(PhysicalPlanError::InvalidPredecessor { packet_index: 7 })
        );

        let mut changed_header = canonical;
        changed_header.declaration_id = identity(1, 999);
        assert_eq!(
            validate_physical_plan_declaration(changed_header, &expected),
            Err(PhysicalPlanError::ExpectedHeaderDrift)
        );
    }

    #[test]
    fn theorem_wrapper_retains_the_exact_inert_declaration() {
        let capacity = capacity(
            PhysicalCapacitySource::ReviewedBatchArithmeticV1,
            M1_REVIEWED_BATCH_PACKET_CAPACITY_V1,
            256,
        );
        let declaration =
            synthetic_unproved_declaration(draft_selection(), QWEN3_DRAFT_PLAN_STEPS, 2, capacity);
        let retained = declaration.clone();
        let expected = expectation(&declaration, capacity);
        let validated =
            physical_plan_structural_validation_theorem(declaration, &expected).unwrap();

        assert_eq!(validated.declaration(), &retained);
        assert_eq!(
            validated.capacity_source(),
            PhysicalCapacitySource::ReviewedBatchArithmeticV1
        );
    }
}
