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

use crate::{plan_step_count, Identity, Qwen3PlanError, Qwen3PlanSelection};
use vstd::prelude::*;

verus! {

/// Canonical inert physical-plan declaration version.
pub const M1_PHYSICAL_PLAN_DECLARATION_VERSION: u32 = 1;
/// Exact maximum of the currently reviewed batch-arithmetic declaration.
pub const M1_REVIEWED_BATCH_PACKET_CAPACITY_V1: u32 = 256;
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
    /// Returns the inert declaration for identity binding and later review.
    #[must_use]
    pub const fn declaration(&self) -> &PhysicalPlanDeclaration {
        &self.declaration
    }

    /// Returns the retained expectation class. `FutureUntrusted` never means
    /// fe2o3 support, and the reviewed tag is not authentication evidence.
    #[must_use]
    pub const fn capacity_source(&self) -> PhysicalCapacitySource {
        self.capacity_source
    }
}

fn identity_present(
    identity: &Identity,
    role: PhysicalIdentityRole,
    packet_index: Option<u32>,
) -> Result<(), PhysicalPlanError> {
    if !identity.is_present() {
        return Err(PhysicalPlanError::MissingIdentity { role, packet_index });
    }
    Ok(())
}

fn identities_are_distinct(identities: &[Identity]) -> bool {
    let mut index = 0usize;
    while index < identities.len()
        invariant
            index <= identities@.len(),
        decreases identities@.len() - index,
    {
        let mut prior = 0usize;
        while prior < index
            invariant
                prior <= index,
                index < identities@.len(),
            decreases index - prior,
        {
            if identities[index].equals(&identities[prior]) {
                return false;
            }
            prior += 1;
        }
        index += 1;
    }
    true
}

fn declared_ring_capacity_valid(value: u32) -> bool {
    matches!(
        value,
        64 | 128 | 256 | 512 | 1_024 | 2_048 | 4_096 | 8_192 | 16_384 | 32_768 | 65_536
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
) -> Result<(), PhysicalPlanError> {
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
    if matches!(
        capacity.source,
        PhysicalCapacitySource::ReviewedBatchArithmeticV1
    ) && capacity.batch_packet_capacity != M1_REVIEWED_BATCH_PACKET_CAPACITY_V1
    {
        return Err(PhysicalPlanError::InvalidCapacity);
    }
    Ok(())
}

fn validate_packet_identities(
    packet: &PhysicalPacketSpanDeclaration,
) -> Result<(), PhysicalPlanError> {
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
    if !identities_are_distinct(&identity_values) {
        return Err(PhysicalPlanError::ReusedIdentity { packet_index });
    }
    Ok(())
}

fn validate_fusion(
    packet: &PhysicalPacketSpanDeclaration,
) -> Result<(), PhysicalPlanError> {
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
) -> Result<(), PhysicalPlanError> {
    let mut prior = None;
    for predecessor in &packet.predecessors {
        let regressed = match prior {
            Some(value) => *predecessor <= value,
            None => false,
        };
        if *predecessor >= packet.packet_index || regressed {
            return Err(PhysicalPlanError::InvalidPredecessor {
                packet_index: packet.packet_index,
            });
        }
        prior = Some(*predecessor);
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

fn validate_structure(
    declaration: &PhysicalPlanDeclaration,
    capacity: PhysicalCapacityExpectation,
) -> Result<(), PhysicalPlanError> {
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
        decreases declaration.packets@.len() - index,
    {
        let packet = &declaration.packets[index];
        let expected_index = match u32::try_from(index) {
            Ok(expected_index) => expected_index,
            Err(_) => return Err(PhysicalPlanError::InvalidCapacity),
        };
        if packet.packet_index != expected_index {
            return Err(PhysicalPlanError::PacketIndexDrift {
                expected: expected_index,
                actual: packet.packet_index,
            });
        }
        if packet.logical_count == 0 {
            return Err(PhysicalPlanError::InvalidLogicalSpan {
                packet_index: packet.packet_index,
            });
        }
        if packet.logical_start != expected_start {
            return Err(PhysicalPlanError::LogicalCoverageDrift {
                packet_index: packet.packet_index,
                expected_start,
                actual_start: packet.logical_start,
            });
        }
        expected_start = packet
            .logical_start
            .checked_add(packet.logical_count)
            .ok_or(PhysicalPlanError::InvalidLogicalSpan {
                packet_index: packet.packet_index,
            })?;
        if expected_start > declaration.logical_operation_count {
            return Err(PhysicalPlanError::InvalidLogicalSpan {
                packet_index: packet.packet_index,
            });
        }
        validate_packet_identities(packet)?;
        validate_fusion(packet)?;
        validate_predecessors(packet)?;
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
/// This executable body has no semantic specification postcondition and
/// therefore contributes no physical-plan refinement proof authority.
pub fn validate_physical_plan_declaration(
    declaration: PhysicalPlanDeclaration,
    expectation: &PhysicalPlanExpectation,
) -> Result<StructurallyValidatedPhysicalPlan, PhysicalPlanError> {
    validate_capacity_expectation(expectation.capacity)?;
    if validate_structure(&expectation.expected, expectation.capacity).is_err() {
        return Err(PhysicalPlanError::InvalidExpectation);
    }
    validate_structure(&declaration, expectation.capacity)?;

    if declaration.version != expectation.expected.version
        || !declaration
            .declaration_id
            .equals(&expectation.expected.declaration_id)
        || !declaration
            .source_closure_id
            .equals(&expectation.expected.source_closure_id)
        || !declaration
            .logical_plan_id
            .equals(&expectation.expected.logical_plan_id)
        || !declaration.selection.matches(expectation.expected.selection)
        || declaration.logical_operation_count != expectation.expected.logical_operation_count
        || !declaration
            .capacity_descriptor_id
            .equals(&expectation.expected.capacity_descriptor_id)
        || declaration.declared_batch_packet_capacity
            != expectation.expected.declared_batch_packet_capacity
        || declaration.declared_ring_packet_capacity
            != expectation.expected.declared_ring_packet_capacity
        || declaration.publication != expectation.expected.publication
        || declaration.completion != expectation.expected.completion
    {
        return Err(PhysicalPlanError::ExpectedHeaderDrift);
    }
    if declaration.packets.len() != expectation.expected.packets.len() {
        return Err(PhysicalPlanError::ExpectedHeaderDrift);
    }
    let mut index = 0usize;
    while index < declaration.packets.len()
        invariant
            index <= declaration.packets@.len(),
            declaration.packets@.len() == expectation.expected.packets@.len(),
        decreases declaration.packets@.len() - index,
    {
        let actual = &declaration.packets[index];
        let expected = &expectation.expected.packets[index];
        if actual != expected {
            let packet_index = match u32::try_from(index) {
                Ok(packet_index) => packet_index,
                Err(_) => return Err(PhysicalPlanError::InvalidCapacity),
            };
            return Err(PhysicalPlanError::ExpectedPacketDrift { packet_index });
        }
        index += 1;
    }
    Ok(StructurallyValidatedPhysicalPlan {
        declaration,
        capacity_source: expectation.capacity.source,
    })
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{
        validate_physical_plan_declaration, DeclaredFusionRefinementPremise,
        PhysicalCapacityExpectation, PhysicalCapacitySource, PhysicalCompletionDeclaration,
        PhysicalIdentityRole, PhysicalPacketIdentityBinding, PhysicalPacketSpanDeclaration,
        PhysicalPlanDeclaration, PhysicalPlanError, PhysicalPlanExpectation,
        PhysicalPublicationDeclaration, M1_PHYSICAL_PLAN_DECLARATION_VERSION,
        M1_REVIEWED_BATCH_PACKET_CAPACITY_V1,
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
}
