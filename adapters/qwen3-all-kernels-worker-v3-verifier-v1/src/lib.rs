//! Ferric adapters for the aggregate Qwen3 Worker V3 verifier boundary.
//!
//! The zero-state backend remains unconditionally fail-closed. The separately
//! configured backend owns caller-admitted one-shot protected-verifier and
//! compiler-current clients, derives an exact request from local move-only
//! owners, and promotes only a correlated signature-authenticated 12-entry
//! receipt after the caller has assumed the external deployment obligations.
//! Neither backend embeds deployment keys, receipts, or authority.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::os::fd::OwnedFd;

use fe2o3_host::{
    BlockSizeV1, CompilerGeneratedKernelExpectationRosterEntryV1,
    CompilerGeneratedKernelExpectationRosterV1, InheritedWorkerV3CompilerCurrentRecordAuditorV1,
    WorkerV3CompilerCurrentRecordAuditErrorV1, WorkerV3CompilerCurrentRecordAuditV1,
    WorkerV3CompilerExecutionEvidenceErrorV1, WorkerV3CompilerExecutionVerificationV1,
    WorkerV3ProtectedRosterEntryEvidenceV1, WorkerV3ProtectedRosterVerificationEvidenceV1,
    WorkerV3ProtectedRosterVerifierBackendV1, WorkerV3RosterVerificationRequestV1,
};
use fe2o3_hsaco_finalize::{
    FinalizedDescriptorInspection, RevalidatedProtectedWorkerV3FinalizerDerivationV1,
};
use fe2o3_verifier::{
    ValidatedCompilerMultiRootProofInputsV1, ValidatedCompilerMultiRootTargetLineageV1,
};
use fe2o3_worker_v3_verification_protocol::{
    WorkerV3VerificationEntryCoordinateV1, WorkerV3VerificationFdPayloadDescriptorV1,
    WorkerV3VerificationMeasurementIdentityV1, WorkerV3VerificationPolicyIdentityV1,
    WorkerV3VerificationProtocolErrorV1, WorkerV3VerificationRequestV1,
    WorkerV3VerificationRosterIdentityV1,
};
use ferric_qwen3_all_kernels_device_v1::M1AllKernelsWorkerV3RosterV1;
use ferric_qwen3_all_kernels_worker_v3_source_pin_v1::{
    M1AggregateSourcePinErrorV1, project_m1_aggregate_module_handoff_v1,
};

use crate::protected_receipt::{
    M1AllKernelsAuthenticatedProtectedVerifierReceiptV1,
    M1AllKernelsProtectedReceiptCompilerClaimsV1, M1AllKernelsProtectedReceiptEntryV1,
    M1AllKernelsProtectedReceiptErrorV1, M1AllKernelsProtectedReceiptRequestClaimsV1,
    M1AllKernelsProtectedReceiptSourcePinV1, M1AllKernelsProtectedVerifierTrustPolicyV1,
};
use crate::protected_verifier_client::{
    M1AllKernelsProtectedVerifierBeginChallengeV2, M1AllKernelsProtectedVerifierClientErrorV2,
    M1AllKernelsProtectedVerifierClientV2,
};
use crate::protected_verifier_service::{
    M1AllKernelsProtectedVerifierServiceEntryV1,
    M1AllKernelsProtectedVerifierServiceProtocolErrorV1,
    M1AllKernelsProtectedVerifierServiceRequestV1,
};
use rustix::fs::{MemfdFlags, Mode, SealFlags};
use sha2::{Digest, Sha256};

/// Canonical protected-receipt wire and caller-provisioned trust policy.
pub mod protected_receipt;

/// One-shot bounded client for an externally supervised protected verifier.
pub mod protected_verifier_client;

/// Canonical aggregate protected-verifier request and response packets.
pub mod protected_verifier_service;

#[cfg(test)]
mod protected_verifier_test_support;

/// Exact number of markers in Ferric's current aggregate M1 roster.
pub const M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1: usize = M1AllKernelsWorkerV3RosterV1::ENTRIES.len();

const M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1: usize = 12;
const _: [(); M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1] = [(); M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1];

// These private values are an inert observation of the host-owned request.
// They deliberately have no public constructor or conversion to evidence.
#[allow(dead_code)]
struct M1AllKernelsPendingDescriptorProjectionV1 {
    kernel_id: [u8; 32],
    logical_name: String,
    entry_name: String,
    descriptor_symbol: String,
    source_evidence_identity: [u8; 32],
    source_evidence_digest: [u8; 32],
    executable_ir_evidence_identity: [u8; 32],
    executable_ir_evidence_digest: [u8; 32],
    explicit_argument_size: u32,
    kernarg_segment_size: u32,
    kernarg_segment_alignment: u32,
    launch_block_size: BlockSizeV1,
    capability_count: usize,
    logical_argument_count: usize,
}

#[allow(dead_code)]
struct M1AllKernelsPendingDescriptorBindingProjectionV1 {
    kernel_index: usize,
    descriptor_address: u64,
    descriptor_file_offset: u64,
    entry_address: u64,
    entry_file_offset: u64,
    entry_size: u64,
    group_segment_fixed_size: u32,
    private_segment_fixed_size: u32,
    kernarg_size: u32,
    kernel_code_entry_byte_offset: i64,
    compute_pgm_rsrc3: u32,
    compute_pgm_rsrc1: u32,
    compute_pgm_rsrc2: u32,
    kernel_code_properties: u16,
    kernarg_preload: u16,
}

#[allow(dead_code)]
struct M1AllKernelsPendingPhysicalKernelProjectionV1 {
    name: String,
    symbol: String,
    kernarg_segment_size: u64,
    kernarg_segment_alignment: u64,
    group_segment_fixed_size: u64,
    private_segment_fixed_size: u64,
    wavefront_size: u32,
    sgpr_count: u16,
    vgpr_count: u16,
    agpr_count: Option<u32>,
    sgpr_spill_count: Option<u32>,
    vgpr_spill_count: Option<u32>,
    max_flat_workgroup_size: u32,
    required_workgroup_size: Option<[u32; 3]>,
    max_workgroups: [Option<u32>; 3],
    cluster_dims: Option<[u32; 3]>,
    uniform_work_group_size: Option<bool>,
    uses_dynamic_stack: Option<bool>,
    workgroup_processor_mode: Option<bool>,
    implicit_argument_offset: Option<u64>,
    implicit_argument_size: u64,
    explicit_argument_count: usize,
    hidden_argument_count: usize,
}

#[allow(dead_code)]
struct M1AllKernelsPendingEntryProjectionV1 {
    ordinal: usize,
    logical_name: &'static str,
    export_name: &'static str,
    marker_binding_identity: [u8; 32],
    generated_host_contract_identity: [u8; 32],
    lineage_identity: Option<[u8; 32]>,
    descriptor: Option<M1AllKernelsPendingDescriptorProjectionV1>,
    descriptor_binding: Option<M1AllKernelsPendingDescriptorBindingProjectionV1>,
    physical_kernel: Option<M1AllKernelsPendingPhysicalKernelProjectionV1>,
}

#[allow(dead_code)]
struct M1AllKernelsPendingRequestProjectionV1 {
    challenge_identity: [u8; 32],
    roster_identity: [u8; 32],
    lineage_identity: [u8; 32],
    finalizer_derivation_sha256: [u8; 32],
    compiler_execution_subject_sha256: [u8; 32],
    compiler_execution_carriage_sha256: [u8; 32],
    compiler_execution_policy_sha256: [u8; 32],
    compiler_execution_issuer_journal_sha256: [u8; 32],
    compiler_occurrence_sha256: [u8; 32],
    compiler_execution_receipt_sha256: [u8; 32],
    compiler_execution_publication_sha256: [u8; 32],
    compiler_execution_acknowledgment_sha256: [u8; 32],
    compiler_execution_worker_ledger_record_sha256: [u8; 32],
    compiler_execution_sequence: u64,
    compiler_execution_prior_rollback_anchor: [u8; 32],
    compiler_execution_current_rollback_anchor: [u8; 32],
    capsule_sha256: [u8; 32],
    formal_memory_receipt_sha256: [u8; 32],
    proof_binding_receipt_sha256: [u8; 32],
    finalized_hsaco_sha256: [u8; 32],
    finalized_hsaco_length: u64,
    target: String,
    code_object_version: u8,
    entries: [M1AllKernelsPendingEntryProjectionV1; M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1],
}

impl M1AllKernelsPendingRequestProjectionV1 {
    fn entry_from_request(
        request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
        ordinal: usize,
        marker: &CompilerGeneratedKernelExpectationRosterEntryV1,
    ) -> M1AllKernelsPendingEntryProjectionV1 {
        let lineage = request
            .entry_lineage_identity(ordinal)
            .map(|identity| *identity.as_bytes());
        let descriptor = request.descriptor(ordinal).map(|descriptor| {
            let source_evidence = descriptor.source_evidence();
            let executable_ir_evidence = descriptor.executable_ir_evidence();
            let abi = descriptor.abi_layout();
            M1AllKernelsPendingDescriptorProjectionV1 {
                kernel_id: *descriptor.kernel_id().as_bytes(),
                logical_name: descriptor.logical_name().as_str().to_owned(),
                entry_name: descriptor.entry_name().as_str().to_owned(),
                descriptor_symbol: descriptor.descriptor_symbol().as_str().to_owned(),
                source_evidence_identity: *source_evidence.identity().as_bytes(),
                source_evidence_digest: *source_evidence.digest().as_bytes(),
                executable_ir_evidence_identity: *executable_ir_evidence.identity().as_bytes(),
                executable_ir_evidence_digest: *executable_ir_evidence.digest().as_bytes(),
                explicit_argument_size: abi.explicit_argument_size(),
                kernarg_segment_size: abi.kernarg_segment_size(),
                kernarg_segment_alignment: abi.kernarg_segment_alignment(),
                launch_block_size: descriptor.launch().block_size(),
                capability_count: descriptor.capabilities().len(),
                logical_argument_count: descriptor.arguments().len(),
            }
        });
        let descriptor_binding = request.descriptor_binding(ordinal).map(|binding| {
            let descriptor = binding.descriptor();
            M1AllKernelsPendingDescriptorBindingProjectionV1 {
                kernel_index: binding.kernel_index(),
                descriptor_address: binding.descriptor_address(),
                descriptor_file_offset: binding.descriptor_file_offset(),
                entry_address: binding.entry_address(),
                entry_file_offset: binding.entry_file_offset(),
                entry_size: binding.entry_size(),
                group_segment_fixed_size: descriptor.group_segment_fixed_size(),
                private_segment_fixed_size: descriptor.private_segment_fixed_size(),
                kernarg_size: descriptor.kernarg_size(),
                kernel_code_entry_byte_offset: descriptor.kernel_code_entry_byte_offset(),
                compute_pgm_rsrc3: descriptor.compute_pgm_rsrc3(),
                compute_pgm_rsrc1: descriptor.compute_pgm_rsrc1(),
                compute_pgm_rsrc2: descriptor.compute_pgm_rsrc2(),
                kernel_code_properties: descriptor.kernel_code_properties(),
                kernarg_preload: descriptor.kernarg_preload(),
            }
        });
        let physical_kernel = request.physical_kernel(ordinal).map(|physical| {
            M1AllKernelsPendingPhysicalKernelProjectionV1 {
                name: physical.name().to_owned(),
                symbol: physical.symbol().to_owned(),
                kernarg_segment_size: physical.kernarg_segment_size(),
                kernarg_segment_alignment: physical.kernarg_segment_alignment(),
                group_segment_fixed_size: physical.group_segment_fixed_size(),
                private_segment_fixed_size: physical.private_segment_fixed_size(),
                wavefront_size: physical.wavefront_size(),
                sgpr_count: physical.sgpr_count(),
                vgpr_count: physical.vgpr_count(),
                agpr_count: physical.agpr_count(),
                sgpr_spill_count: physical.sgpr_spill_count(),
                vgpr_spill_count: physical.vgpr_spill_count(),
                max_flat_workgroup_size: physical.max_flat_workgroup_size(),
                required_workgroup_size: physical.required_workgroup_size(),
                max_workgroups: physical.max_workgroups(),
                cluster_dims: physical.cluster_dims(),
                uniform_work_group_size: physical.uniform_work_group_size_declaration(),
                uses_dynamic_stack: physical.uses_dynamic_stack_declaration(),
                workgroup_processor_mode: physical.workgroup_processor_mode(),
                implicit_argument_offset: physical.implicit_argument_offset(),
                implicit_argument_size: physical.implicit_argument_size(),
                explicit_argument_count: physical.explicit_arguments().len(),
                hidden_argument_count: physical.hidden_arguments().len(),
            }
        });
        M1AllKernelsPendingEntryProjectionV1 {
            ordinal,
            logical_name: marker.logical_name(),
            export_name: marker.export_name(),
            marker_binding_identity: marker.kernel_binding_id(),
            generated_host_contract_identity: marker.generated_host_contract_identity(),
            lineage_identity: lineage,
            descriptor,
            descriptor_binding,
            physical_kernel,
        }
    }

    fn from_request(
        request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
    ) -> Option<Self> {
        let entries = request
            .marker_entries()
            .iter()
            .enumerate()
            .map(|(ordinal, marker)| Self::entry_from_request(request, ordinal, marker))
            .collect::<Vec<_>>()
            .try_into()
            .ok()?;
        Some(Self {
            challenge_identity: *request.challenge_identity().as_bytes(),
            roster_identity: *request.roster_identity().as_bytes(),
            lineage_identity: *request.lineage_identity().as_bytes(),
            finalizer_derivation_sha256: request.finalizer_derivation_sha256(),
            compiler_execution_subject_sha256: request.compiler_execution_subject_sha256(),
            compiler_execution_carriage_sha256: request.compiler_execution_carriage_sha256(),
            compiler_execution_policy_sha256: request.compiler_execution_policy_sha256(),
            compiler_execution_issuer_journal_sha256: request
                .compiler_execution_issuer_journal_sha256(),
            compiler_occurrence_sha256: request.compiler_occurrence_sha256(),
            compiler_execution_receipt_sha256: request.compiler_execution_receipt_sha256(),
            compiler_execution_publication_sha256: request.compiler_execution_publication_sha256(),
            compiler_execution_acknowledgment_sha256: request
                .compiler_execution_acknowledgment_sha256(),
            compiler_execution_worker_ledger_record_sha256: request
                .compiler_execution_worker_ledger_record_sha256(),
            compiler_execution_sequence: request.compiler_execution_sequence(),
            compiler_execution_prior_rollback_anchor: request
                .compiler_execution_prior_rollback_anchor(),
            compiler_execution_current_rollback_anchor: request
                .compiler_execution_current_rollback_anchor(),
            capsule_sha256: request.capsule_sha256(),
            formal_memory_receipt_sha256: request.formal_memory_receipt_sha256(),
            proof_binding_receipt_sha256: request.proof_binding_receipt_sha256(),
            finalized_hsaco_sha256: request.finalized_hsaco_sha256(),
            finalized_hsaco_length: request.finalized_hsaco_length(),
            target: request.target().to_string(),
            code_object_version: request.code_object_version().number(),
            entries,
        })
    }
}

/// Failure returned by the aggregate M1 protected-verifier scaffold.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1AllKernelsProtectedVerifierErrorV1 {
    /// The exact finalizer replay did not independently revalidate.
    FinalizerDerivationRevalidationFailed,
    /// The common multi-root compiler proof inputs did not validate.
    CompilerMultiRootProofInputsValidationFailed,
    /// The common multi-root target lineage did not validate.
    CompilerMultiRootTargetLineageValidationFailed,
    /// The exact borrowed finalized HSACO failed descriptor-table and digest verification.
    ExactFinalizedHsacoVerificationFailed,
    /// Finalizer custody did not match the request's exact artifact coordinates.
    FinalizerArtifactAssociationFailed,
    /// Compiler target custody did not match the finalizer, handoff, or target.
    CompilerTargetAssociationFailed,
    /// The reinspection target, COV, cardinality, load layout, or per-entry association drifted.
    RosterEntryAssociationFailed,
    /// No independently authenticated protected-verification receipt exists.
    MissingProtectedVerificationReceipt {
        /// Number of ordered marker results the missing receipt must cover.
        expected_roster_entries: usize,
    },
}

impl fmt::Display for M1AllKernelsProtectedVerifierErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FinalizerDerivationRevalidationFailed => {
                formatter.write_str("independent finalizer derivation revalidation failed")
            }
            Self::CompilerMultiRootProofInputsValidationFailed => {
                formatter.write_str("common multi-root compiler proof input validation failed")
            }
            Self::CompilerMultiRootTargetLineageValidationFailed => {
                formatter.write_str("common multi-root compiler target-lineage validation failed")
            }
            Self::ExactFinalizedHsacoVerificationFailed => {
                formatter.write_str("exact finalized HSACO verification failed")
            }
            Self::FinalizerArtifactAssociationFailed => {
                formatter.write_str("finalizer and exact artifact association failed")
            }
            Self::CompilerTargetAssociationFailed => {
                formatter.write_str("compiler module and target association failed")
            }
            Self::RosterEntryAssociationFailed => {
                formatter.write_str("aggregate roster entry association failed")
            }
            Self::MissingProtectedVerificationReceipt {
                expected_roster_entries,
            } => {
                formatter.write_str("missing protected verification receipt for all ")?;
                formatter.write_str(&expected_roster_entries.to_string())?;
                formatter.write_str(" aggregate M1 roster entries")
            }
        }
    }
}

impl Error for M1AllKernelsProtectedVerifierErrorV1 {}

struct M1AllKernelsLocallyRevalidatedOwnersV1 {
    finalizer: RevalidatedProtectedWorkerV3FinalizerDerivationV1,
    proof_inputs: ValidatedCompilerMultiRootProofInputsV1,
    target_lineage: ValidatedCompilerMultiRootTargetLineageV1,
    hsaco_reinspection: FinalizedDescriptorInspection,
}

fn locally_revalidate_request_v1(
    request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
    pending: &M1AllKernelsPendingRequestProjectionV1,
) -> Result<M1AllKernelsLocallyRevalidatedOwnersV1, M1AllKernelsProtectedVerifierErrorV1> {
    let hsaco_reinspection = ::fe2o3_hsaco_finalize::verify_finalized(
        request.finalized_hsaco_bytes(),
    )
    .map_err(|_| M1AllKernelsProtectedVerifierErrorV1::ExactFinalizedHsacoVerificationFailed)?;
    let finalizer = request
        .independently_revalidate_finalizer_derivation()
        .map_err(|_| M1AllKernelsProtectedVerifierErrorV1::FinalizerDerivationRevalidationFailed)?;
    let proof_inputs = request
        .validate_compiler_multi_root_proof_inputs_v1()
        .map_err(|_| {
            M1AllKernelsProtectedVerifierErrorV1::CompilerMultiRootProofInputsValidationFailed
        })?;
    let target_lineage = request
        .validate_compiler_multi_root_target_lineage_v1(&proof_inputs)
        .map_err(|_| {
            M1AllKernelsProtectedVerifierErrorV1::CompilerMultiRootTargetLineageValidationFailed
        })?;
    let owners = M1AllKernelsLocallyRevalidatedOwnersV1 {
        finalizer,
        proof_inputs,
        target_lineage,
        hsaco_reinspection,
    };
    validate_local_request_associations_v1(request, pending, &owners)?;
    Ok(owners)
}

#[allow(clippy::too_many_lines)]
fn validate_local_request_associations_v1(
    request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
    pending: &M1AllKernelsPendingRequestProjectionV1,
    owners: &M1AllKernelsLocallyRevalidatedOwnersV1,
) -> Result<(), M1AllKernelsProtectedVerifierErrorV1> {
    let finalizer_identity = owners.finalizer.identity();
    let finalized_hsaco = owners.finalizer.finalized_hsaco_identity();
    (finalizer_identity.as_bytes() == &pending.finalizer_derivation_sha256
        && finalized_hsaco.sha256() == &pending.finalized_hsaco_sha256
        && finalized_hsaco.byte_len() == pending.finalized_hsaco_length)
        .then_some(())
        .ok_or(M1AllKernelsProtectedVerifierErrorV1::FinalizerArtifactAssociationFailed)?;

    let final_llvm = owners.target_lineage.final_llvm_identity();
    let finalizer_module = owners.finalizer.compiler_module_identity();
    let semantic_module = request
        .semantic_compiler_handoff()
        .module_handoff()
        .module_identity();
    let target_binding = owners.target_lineage.target_binding();
    (final_llvm.sha256() == *finalizer_module.sha256()
        && final_llvm.byte_len() == finalizer_module.byte_len()
        && final_llvm.sha256() == *semantic_module.sha256()
        && final_llvm.byte_len() == semantic_module.byte_len()
        && target_binding.configured_target() == pending.target
        && target_binding.code_object_version() == u16::from(pending.code_object_version)
        && pending.target == "gfx942:xnack-"
        && pending.code_object_version == 6)
        .then_some(())
        .ok_or(M1AllKernelsProtectedVerifierErrorV1::CompilerTargetAssociationFailed)?;

    let markers = request.marker_entries();
    let proof_roots = owners.proof_inputs.roots();
    let reinspected = owners.hsaco_reinspection.hsaco();
    let reinspected_kernels = reinspected.kernels();
    let reinspected_bindings = owners.hsaco_reinspection.kernel_bindings().bindings();
    (markers.len() == M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1
        && proof_roots.len() == M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1
        && target_binding.root_count() == M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1
        && owners.hsaco_reinspection.descriptor_table() == request.descriptor_table()
        && reinspected.target() == request.target()
        && reinspected.code_object_version().number() == request.code_object_version().number()
        && reinspected_kernels.len() == M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1
        && reinspected_bindings.len() == M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1
        && owners
            .hsaco_reinspection
            .kernel_bindings()
            .load_layout()
            .is_some())
    .then_some(())
    .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;

    (markers.iter().all(|marker| {
        proof_roots
            .iter()
            .filter(|root| root.kernel_binding() == &marker.kernel_binding_id())
            .count()
            == 1
    }) && proof_roots.iter().all(|root| {
        markers
            .iter()
            .filter(|marker| marker.kernel_binding_id() == *root.kernel_binding())
            .count()
            == 1
    }))
    .then_some(())
    .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;

    let mut matched_reinspected_kernels = [false; M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1];
    pending.entries.iter().try_for_each(|entry| {
        let (root_index, root) = proof_roots
            .iter()
            .enumerate()
            .find(|(_, root)| root.kernel_binding() == &entry.marker_binding_identity)
            .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;
        let _lineage = entry
            .lineage_identity
            .as_ref()
            .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;
        let descriptor = entry
            .descriptor
            .as_ref()
            .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;
        let descriptor_binding = entry
            .descriptor_binding
            .as_ref()
            .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;
        let physical = entry
            .physical_kernel
            .as_ref()
            .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;
        let mut matching_reinspected_kernels =
            reinspected_kernels
                .iter()
                .enumerate()
                .filter(|(_, kernel)| {
                    kernel.name() == entry.export_name
                        && kernel.symbol() == descriptor.descriptor_symbol
                });
        let (reinspected_index, reinspected_physical) = matching_reinspected_kernels
            .next()
            .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;
        matching_reinspected_kernels
            .next()
            .is_none()
            .then_some(())
            .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;
        let matched = matched_reinspected_kernels
            .get_mut(reinspected_index)
            .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;
        (!*matched)
            .then_some(())
            .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;
        *matched = true;
        let reinspected_binding = reinspected_bindings
            .get(reinspected_index)
            .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;
        let request_physical = request
            .physical_kernel(entry.ordinal)
            .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;
        let request_binding = request
            .descriptor_binding(entry.ordinal)
            .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;
        let target_workgroup = target_binding
            .workgroup(root_index)
            .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;
        let descriptor_workgroup = match descriptor.launch_block_size {
            BlockSizeV1::Exact(dimensions) => {
                Some([dimensions.x(), dimensions.y(), dimensions.z()])
            }
            BlockSizeV1::Any | BlockSizeV1::AtMost(_) => None,
        }
        .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;

        (entry.logical_name == root.logical_name()
            && entry.export_name == root.export_symbol()
            && descriptor.kernel_id == entry.marker_binding_identity
            && descriptor.kernel_id == *root.kernel_binding()
            && descriptor.logical_name == root.logical_name()
            && descriptor.entry_name == root.export_symbol()
            && physical.name == root.export_symbol()
            && physical.symbol == descriptor.descriptor_symbol
            && reinspected_physical == request_physical
            && *reinspected_binding == request_binding
            && reinspected_binding.kernel_index() == reinspected_index
            && request_binding.kernel_index() == reinspected_index
            && descriptor_binding.kernel_index == reinspected_index
            && target_workgroup.kernel() == root.kernel_id()
            && target_workgroup.workgroup() == root.workgroup()
            && descriptor_workgroup == root.workgroup())
        .then_some(())
        .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)
    })?;
    matched_reinspected_kernels
        .iter()
        .all(|matched| *matched)
        .then_some(())
        .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)
}

/// Ferric's current aggregate M1 protected-verifier backend.
///
/// This zero-state scaffold owns no verifier service, protected receipt,
/// compiler authority, load authority, or launch authority. Until a reviewed
/// protected backend replaces it, every request fails closed.
#[derive(Clone, Copy, Debug, Default)]
pub struct M1AllKernelsProtectedVerifierV1;

impl M1AllKernelsProtectedVerifierV1 {
    /// Constructs the fail-closed backend.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn reject_missing_protected_receipt<
        FinalizerOwner,
        ProofInputsOwner,
        TargetLineageOwner,
        HsacoReinspectionOwner,
    >(
        _request: &M1AllKernelsPendingRequestProjectionV1,
        _finalizer_owner: FinalizerOwner,
        _proof_inputs_owner: ProofInputsOwner,
        _target_lineage_owner: TargetLineageOwner,
        _hsaco_reinspection_owner: HsacoReinspectionOwner,
    ) -> Result<WorkerV3ProtectedRosterVerificationEvidenceV1, M1AllKernelsProtectedVerifierErrorV1>
    {
        Err(missing_protected_verification_receipt_v1())
    }
}

const fn missing_protected_verification_receipt_v1() -> M1AllKernelsProtectedVerifierErrorV1 {
    M1AllKernelsProtectedVerifierErrorV1::MissingProtectedVerificationReceipt {
        expected_roster_entries: M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1,
    }
}

// SAFETY: this backend cannot claim any of the trait's protected-verification
// obligations because it never constructs or returns verification evidence.
// Every request terminates with the explicit missing-receipt error below.
unsafe impl WorkerV3ProtectedRosterVerifierBackendV1<M1AllKernelsWorkerV3RosterV1>
    for M1AllKernelsProtectedVerifierV1
{
    type Error = M1AllKernelsProtectedVerifierErrorV1;

    unsafe fn verify_protected_roster(
        &mut self,
        request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
    ) -> Result<WorkerV3ProtectedRosterVerificationEvidenceV1, Self::Error> {
        let pending_request = M1AllKernelsPendingRequestProjectionV1::from_request(request)
            .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;
        let owners = locally_revalidate_request_v1(request, &pending_request)?;
        Self::reject_missing_protected_receipt(
            &pending_request,
            owners.finalizer,
            owners.proof_inputs,
            owners.target_lineage,
            owners.hsaco_reinspection,
        )
    }
}

/// Failure returned by the configured aggregate production verifier binder.
#[derive(Debug)]
#[non_exhaustive]
pub enum M1AllKernelsProductionProtectedVerifierErrorV1 {
    /// Local finalizer, proof, target-lineage, or exact-HSACO preflight failed.
    LocalRevalidation(M1AllKernelsProtectedVerifierErrorV1),
    /// The typed compiler handoff did not satisfy Ferric's exact aggregate source policy.
    SourcePin(M1AggregateSourcePinErrorV1),
    /// The inherited FD195 current-record audit failed or was already consumed.
    CompilerCurrentRecordAudit(WorkerV3CompilerCurrentRecordAuditErrorV1),
    /// The signed current-record audit did not bind to the exact subject and carriage.
    CompilerExecutionBinding(WorkerV3CompilerExecutionEvidenceErrorV1),
    /// Bound compiler coordinates differed from the exact borrowed roster request.
    CompilerExecutionAssociationFailed,
    /// Canonical protected-receipt claims could not be constructed.
    ReceiptClaims(M1AllKernelsProtectedReceiptErrorV1),
    /// Canonical protected-verifier service request construction failed.
    ServiceRequest(M1AllKernelsProtectedVerifierServiceProtocolErrorV1),
    /// Generic V2 request or coordinate construction failed.
    GenericRequest(WorkerV3VerificationProtocolErrorV1),
    /// An exact sealed memfd snapshot could not be constructed.
    PayloadSnapshot {
        /// Snapshot operation that failed.
        operation: &'static str,
        /// Underlying operating-system failure.
        source: io::Error,
    },
    /// The typed roster request lacked one exact canonical entry coordinate.
    RosterRequestAssociationFailed,
    /// The configured protected-verifier client was already consumed.
    ProtectedVerifierClientAlreadyConsumed,
    /// The deployment-reserved generic Begin challenge was already consumed.
    BeginChallengeAlreadyConsumed,
    /// The service challenge could not be transferred into the compiler-current client.
    CompilerCurrentRecordChallenge(
        fe2o3_runtime_protocol::CompilerExecutionCurrentRecordVerificationErrorV3,
    ),
    /// Protected-verifier transport, correlation, or authentication failed.
    ProtectedVerifierClient(M1AllKernelsProtectedVerifierClientErrorV2),
    /// Authenticated receipt coordinates differed during the final local join.
    AuthenticatedReceiptAssociationFailed,
}

impl fmt::Display for M1AllKernelsProductionProtectedVerifierErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalRevalidation(error) => {
                ::core::write!(
                    formatter,
                    "local aggregate request revalidation failed: {error}"
                )
            }
            Self::SourcePin(error) => {
                ::core::write!(
                    formatter,
                    "aggregate compiler source-pin projection failed: {error}"
                )
            }
            Self::CompilerCurrentRecordAudit(error) => {
                ::core::write!(formatter, "compiler current-record audit failed: {error}")
            }
            Self::CompilerExecutionBinding(error) => {
                ::core::write!(formatter, "compiler current-record binding failed: {error}")
            }
            Self::CompilerExecutionAssociationFailed => formatter
                .write_str("bound compiler execution differs from the aggregate roster request"),
            Self::ReceiptClaims(error) => {
                ::core::write!(
                    formatter,
                    "protected receipt claim construction failed: {error}"
                )
            }
            Self::ServiceRequest(error) => {
                ::core::write!(
                    formatter,
                    "protected-verifier request construction failed: {error}"
                )
            }
            Self::GenericRequest(error) => {
                ::core::write!(formatter, "generic V2 request construction failed: {error}")
            }
            Self::PayloadSnapshot { operation, source } => {
                ::core::write!(
                    formatter,
                    "protected-verifier payload {operation} failed: {source}"
                )
            }
            Self::RosterRequestAssociationFailed => {
                formatter.write_str("aggregate roster request entry association failed")
            }
            Self::ProtectedVerifierClientAlreadyConsumed => {
                formatter.write_str("protected-verifier client was already consumed")
            }
            Self::BeginChallengeAlreadyConsumed => {
                formatter.write_str("protected-verifier Begin challenge was already consumed")
            }
            Self::CompilerCurrentRecordChallenge(error) => ::core::write!(
                formatter,
                "service current-record challenge transfer failed: {error}"
            ),
            Self::ProtectedVerifierClient(error) => {
                ::core::write!(formatter, "protected-verifier exchange failed: {error}")
            }
            Self::AuthenticatedReceiptAssociationFailed => formatter
                .write_str("authenticated protected receipt failed the final local association"),
        }
    }
}

impl Error for M1AllKernelsProductionProtectedVerifierErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::LocalRevalidation(error) => Some(error),
            Self::SourcePin(error) => Some(error),
            Self::CompilerCurrentRecordAudit(error) => Some(error),
            Self::CompilerExecutionBinding(error) => Some(error),
            Self::ReceiptClaims(error) => Some(error),
            Self::ServiceRequest(error) => Some(error),
            Self::GenericRequest(error) => Some(error),
            Self::PayloadSnapshot { source, .. } => Some(source),
            Self::CompilerCurrentRecordChallenge(error) => Some(error),
            Self::ProtectedVerifierClient(error) => Some(error),
            Self::CompilerExecutionAssociationFailed
            | Self::RosterRequestAssociationFailed
            | Self::ProtectedVerifierClientAlreadyConsumed
            | Self::BeginChallengeAlreadyConsumed
            | Self::AuthenticatedReceiptAssociationFailed => None,
        }
    }
}

/// Move-only configured aggregate verifier binder for a supervised deployment.
///
/// Construction requires a previously admitted one-shot protected-verifier
/// client, a caller-provisioned signature and measurement trust policy, and a
/// previously admitted inherited FD195 compiler-current auditor. No endpoint,
/// key, measurement, `CURRENT` record, receipt, or authority is embedded here.
/// The deployment must independently arrange and review all three inputs.
///
/// ```compile_fail
/// use ferric_qwen3_all_kernels_worker_v3_verifier_v1::
///     M1AllKernelsProductionProtectedVerifierV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1AllKernelsProductionProtectedVerifierV1>();
/// ```
///
/// The exact current-record byte view cannot escape its move-only audit owner:
///
/// ```compile_fail
/// use fe2o3_host::{
///     WorkerV3CompilerCurrentRecordAuditV1, WorkerV3CompilerCurrentRecordEvidenceViewV1,
/// };
/// fn escape(
///     owner: &WorkerV3CompilerCurrentRecordAuditV1,
/// ) -> WorkerV3CompilerCurrentRecordEvidenceViewV1<'static> {
///     owner.canonical_evidence_view()
/// }
/// ```
pub struct M1AllKernelsProductionProtectedVerifierV1 {
    client: Option<M1AllKernelsProtectedVerifierClientV2>,
    begin_challenge: Option<M1AllKernelsProtectedVerifierBeginChallengeV2>,
    trust_policy: M1AllKernelsProtectedVerifierTrustPolicyV1,
    current_auditor: InheritedWorkerV3CompilerCurrentRecordAuditorV1,
}

impl fmt::Debug for M1AllKernelsProductionProtectedVerifierV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AllKernelsProductionProtectedVerifierV1")
            .field("client_available", &self.client.is_some())
            .field("begin_challenge_available", &self.begin_challenge.is_some())
            .field("trust_policy_identity", &self.trust_policy.identity())
            .field("current_auditor", &self.current_auditor)
            .finish_non_exhaustive()
    }
}

impl M1AllKernelsProductionProtectedVerifierV1 {
    /// Takes ownership of all caller-admitted one-shot deployment inputs.
    ///
    /// # Safety
    ///
    /// The caller must have independently reviewed the verifier and independent
    /// checker implementations identified by `client` and `trust_policy`. The
    /// supplied Begin challenge must already be unpredictably generated and
    /// durably reserved against replay across all instances and restarts. The
    /// protected signing key must be bound to the exact admitted verifier and
    /// checker measurements. For every request, the deployment must hold
    /// pre-provisioned, or authentically reacquire over a separately reviewed
    /// bounded channel, the exact:
    ///
    /// - Worker V3 V2 envelope;
    /// - finalized HSACO;
    /// - semantic and proof-input payloads; and
    /// - protected compiler-current-record payload.
    ///
    /// The deployment must verify those payloads instead of signing coordinate
    /// echoes. It must atomically consume every fresh challenge and exclude
    /// replay across all service instances. It must check the exact live Worker
    /// ledger and rollback currentness against durable state that survives
    /// service restarts. Every signed theorem, type-layout, effect, and safety
    /// result must be correct for every concrete invocation covered by its
    /// corresponding marker contract. These external properties are required
    /// by the unsafe protected-roster backend contract implemented below.
    #[must_use]
    pub unsafe fn new(
        client: M1AllKernelsProtectedVerifierClientV2,
        begin_challenge: M1AllKernelsProtectedVerifierBeginChallengeV2,
        trust_policy: M1AllKernelsProtectedVerifierTrustPolicyV1,
        current_auditor: InheritedWorkerV3CompilerCurrentRecordAuditorV1,
    ) -> Self {
        Self {
            client: Some(client),
            begin_challenge: Some(begin_challenge),
            trust_policy,
            current_auditor,
        }
    }

    /// Configuration alone grants no verification, load, or launch authority.
    #[must_use]
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

fn protected_service_request_v1(
    request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
    pending: &M1AllKernelsPendingRequestProjectionV1,
    compiler: &WorkerV3CompilerExecutionVerificationV1,
    trust_policy: &M1AllKernelsProtectedVerifierTrustPolicyV1,
) -> Result<
    M1AllKernelsProtectedVerifierServiceRequestV1,
    M1AllKernelsProductionProtectedVerifierErrorV1,
> {
    (compiler.subject_sha256() == pending.compiler_execution_subject_sha256
        && compiler.carriage_sha256() == pending.compiler_execution_carriage_sha256
        && compiler.policy_sha256() == pending.compiler_execution_policy_sha256
        && compiler.issuer_journal_sha256() == pending.compiler_execution_issuer_journal_sha256
        && compiler.compiler_occurrence_sha256() == pending.compiler_occurrence_sha256
        && compiler.receipt_sha256() == pending.compiler_execution_receipt_sha256
        && compiler.publication_sha256() == pending.compiler_execution_publication_sha256
        && compiler.acknowledgment_sha256() == pending.compiler_execution_acknowledgment_sha256
        && compiler.worker_ledger_record_sha256()
            == pending.compiler_execution_worker_ledger_record_sha256
        && compiler.sequence() == pending.compiler_execution_sequence
        && compiler.prior_rollback_anchor() == pending.compiler_execution_prior_rollback_anchor
        && compiler.current_rollback_anchor()
            == pending.compiler_execution_current_rollback_anchor
        && compiler.authenticates_signed_currentness_evidence())
    .then_some(())
    .ok_or(M1AllKernelsProductionProtectedVerifierErrorV1::CompilerExecutionAssociationFailed)?;
    let compiler_claims = M1AllKernelsProtectedReceiptCompilerClaimsV1::new(
        compiler.subject_sha256(),
        compiler.carriage_sha256(),
        compiler.policy_sha256(),
        compiler.issuer_journal_sha256(),
        compiler.compiler_occurrence_sha256(),
        compiler.receipt_sha256(),
        compiler.publication_sha256(),
        compiler.acknowledgment_sha256(),
        compiler.worker_ledger_record_sha256(),
        compiler.sequence(),
        compiler.prior_rollback_anchor(),
        compiler.current_rollback_anchor(),
        compiler.current_record_verification_sha256(),
        compiler.current_record_attestation_sha256(),
        compiler.protected_policy_verification_sha256(),
        compiler.protected_worker_ledger_verification_sha256(),
        compiler.external_rollback_verification_sha256(),
    )
    .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::ReceiptClaims)?;
    protected_service_request_with_compiler_claims_v1(
        request,
        pending,
        &compiler_claims,
        trust_policy,
    )
}

fn protected_service_request_from_current_audit_v1(
    request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
    pending: &M1AllKernelsPendingRequestProjectionV1,
    current_audit: &WorkerV3CompilerCurrentRecordAuditV1,
    trust_policy: &M1AllKernelsProtectedVerifierTrustPolicyV1,
) -> Result<
    M1AllKernelsProtectedVerifierServiceRequestV1,
    M1AllKernelsProductionProtectedVerifierErrorV1,
> {
    let verification = current_audit.verification();
    (verification.subject_identity() == pending.compiler_execution_subject_sha256
        && verification.carriage_identity() == pending.compiler_execution_carriage_sha256
        && verification.policy_identity() == pending.compiler_execution_policy_sha256
        && verification.issuer_journal_identity()
            == pending.compiler_execution_issuer_journal_sha256
        && verification.worker_ledger_record_identity()
            == pending.compiler_execution_worker_ledger_record_sha256
        && verification.sequence() == pending.compiler_execution_sequence
        && verification.prior_rollback_anchor()
            == pending.compiler_execution_prior_rollback_anchor
        && verification.current_rollback_anchor()
            == pending.compiler_execution_current_rollback_anchor
        && current_audit.authenticates_pinned_signing_key()
        && current_audit.authenticates_expected_fresh_challenge()
        && current_audit.authenticates_protected_current_record()
        && current_audit.authenticates_external_anchor_commit()
        && current_audit.authenticates_external_rollback_currentness())
    .then_some(())
    .ok_or(M1AllKernelsProductionProtectedVerifierErrorV1::CompilerExecutionAssociationFailed)?;
    let compiler_claims = M1AllKernelsProtectedReceiptCompilerClaimsV1::new(
        pending.compiler_execution_subject_sha256,
        pending.compiler_execution_carriage_sha256,
        pending.compiler_execution_policy_sha256,
        pending.compiler_execution_issuer_journal_sha256,
        pending.compiler_occurrence_sha256,
        pending.compiler_execution_receipt_sha256,
        pending.compiler_execution_publication_sha256,
        pending.compiler_execution_acknowledgment_sha256,
        pending.compiler_execution_worker_ledger_record_sha256,
        pending.compiler_execution_sequence,
        pending.compiler_execution_prior_rollback_anchor,
        pending.compiler_execution_current_rollback_anchor,
        *verification.identity().as_bytes(),
        *current_audit.attestation_identity().as_bytes(),
        verification.protected_policy_verification_identity(),
        verification.protected_worker_ledger_verification_identity(),
        current_audit.external_rollback_verification_identity(),
    )
    .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::ReceiptClaims)?;
    protected_service_request_with_compiler_claims_v1(
        request,
        pending,
        &compiler_claims,
        trust_policy,
    )
}

fn protected_service_request_with_compiler_claims_v1(
    request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
    pending: &M1AllKernelsPendingRequestProjectionV1,
    compiler_claims: &M1AllKernelsProtectedReceiptCompilerClaimsV1,
    trust_policy: &M1AllKernelsProtectedVerifierTrustPolicyV1,
) -> Result<
    M1AllKernelsProtectedVerifierServiceRequestV1,
    M1AllKernelsProductionProtectedVerifierErrorV1,
> {
    let source_pin = project_m1_aggregate_module_handoff_v1(
        request.semantic_compiler_handoff().module_handoff(),
    )
    .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::SourcePin)?
    .source_pin();
    let source_pin = M1AllKernelsProtectedReceiptSourcePinV1::new(
        source_pin.compiler_module_sha256(),
        source_pin.compiler_module_length(),
        source_pin.compiler_handoff_sha256(),
        source_pin.compiler_handoff_length(),
        source_pin.symbol_manifest_sha256(),
        source_pin.symbol_manifest_length(),
    )
    .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::ReceiptClaims)?;
    let request_claims = M1AllKernelsProtectedReceiptRequestClaimsV1::new(
        pending.challenge_identity,
        pending.roster_identity,
        pending.lineage_identity,
        pending.finalizer_derivation_sha256,
        source_pin,
        pending.capsule_sha256,
        pending.formal_memory_receipt_sha256,
        pending.proof_binding_receipt_sha256,
        pending.finalized_hsaco_sha256,
        pending.finalized_hsaco_length,
    )
    .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::ReceiptClaims)?;
    let entries = pending
        .entries
        .iter()
        .map(|entry| {
            let ordinal = u16::try_from(entry.ordinal).map_err(|_| {
                M1AllKernelsProductionProtectedVerifierErrorV1::RosterRequestAssociationFailed
            })?;
            let lineage = entry.lineage_identity.ok_or(
                M1AllKernelsProductionProtectedVerifierErrorV1::RosterRequestAssociationFailed,
            )?;
            M1AllKernelsProtectedVerifierServiceEntryV1::new(
                ordinal,
                lineage,
                entry.marker_binding_identity,
                entry.generated_host_contract_identity,
            )
            .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::ServiceRequest)
        })
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_: Vec<M1AllKernelsProtectedVerifierServiceEntryV1>| {
            M1AllKernelsProductionProtectedVerifierErrorV1::RosterRequestAssociationFailed
        })?;
    M1AllKernelsProtectedVerifierServiceRequestV1::new(
        trust_policy.identity(),
        request_claims,
        *compiler_claims,
        entries,
    )
    .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::ServiceRequest)
}

fn generic_verification_request_v1(
    request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
    pending: &M1AllKernelsPendingRequestProjectionV1,
    trust_policy: &M1AllKernelsProtectedVerifierTrustPolicyV1,
    challenge: M1AllKernelsProtectedVerifierBeginChallengeV2,
) -> Result<WorkerV3VerificationRequestV1, M1AllKernelsProductionProtectedVerifierErrorV1> {
    let envelope_view = request.load_envelope_evidence_view();
    let envelope = envelope_view.exact_canonical_bytes();
    let hsaco = request.finalized_hsaco_bytes();
    let envelope_sha256: [u8; 32] = Sha256::digest(envelope).into();
    let hsaco_sha256: [u8; 32] = Sha256::digest(hsaco).into();
    (hsaco_sha256 == pending.finalized_hsaco_sha256
        && u64::try_from(hsaco.len()).ok() == Some(pending.finalized_hsaco_length))
    .then_some(())
    .ok_or(M1AllKernelsProductionProtectedVerifierErrorV1::RosterRequestAssociationFailed)?;
    let entries = pending
        .entries
        .iter()
        .map(|entry| {
            WorkerV3VerificationEntryCoordinateV1::new(
                u32::try_from(entry.ordinal).map_err(|_| {
                    M1AllKernelsProductionProtectedVerifierErrorV1::RosterRequestAssociationFailed
                })?,
                entry.logical_name,
                entry.export_name,
                entry.lineage_identity.ok_or(
                    M1AllKernelsProductionProtectedVerifierErrorV1::RosterRequestAssociationFailed,
                )?,
                entry.marker_binding_identity,
                entry.generated_host_contract_identity,
            )
            .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::GenericRequest)
        })
        .collect::<Result<Vec<_>, _>>()?;
    WorkerV3VerificationRequestV1::new(
        challenge.into_protocol_challenge(),
        WorkerV3VerificationRosterIdentityV1::new(pending.roster_identity)
            .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::GenericRequest)?,
        WorkerV3VerificationPolicyIdentityV1::new(*trust_policy.identity().as_bytes())
            .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::GenericRequest)?,
        WorkerV3VerificationMeasurementIdentityV1::new(trust_policy.verifier_measurement_sha256())
            .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::GenericRequest)?,
        WorkerV3VerificationFdPayloadDescriptorV1::load_envelope_v2(
            u64::try_from(envelope.len()).map_err(|_| {
                M1AllKernelsProductionProtectedVerifierErrorV1::RosterRequestAssociationFailed
            })?,
            envelope_sha256,
        )
        .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::GenericRequest)?,
        WorkerV3VerificationFdPayloadDescriptorV1::finalized_hsaco(
            pending.finalized_hsaco_length,
            pending.finalized_hsaco_sha256,
        )
        .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::GenericRequest)?,
        entries,
    )
    .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::GenericRequest)
}

fn protected_payload_snapshots_v2(
    request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
) -> Result<Vec<OwnedFd>, M1AllKernelsProductionProtectedVerifierErrorV1> {
    let envelope_view = request.load_envelope_evidence_view();
    let envelope = sealed_payload_snapshot_v2(
        "ferric-worker-v3-load-envelope-v2",
        envelope_view.exact_canonical_bytes(),
    )?;
    let hsaco = sealed_payload_snapshot_v2(
        "ferric-worker-v3-finalized-hsaco",
        request.finalized_hsaco_bytes(),
    )?;
    Ok(Vec::from([envelope, hsaco]))
}

fn sealed_payload_snapshot_v2(
    name: &str,
    bytes: &[u8],
) -> Result<OwnedFd, M1AllKernelsProductionProtectedVerifierErrorV1> {
    let descriptor =
        rustix::fs::memfd_create(name, MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING)
            .map_err(|source| payload_snapshot_error_v2("creation", source.into()))?;
    let mut writer = File::from(descriptor);
    rustix::fs::fchmod(&writer, Mode::RUSR)
        .map_err(|source| payload_snapshot_error_v2("permission pinning", source.into()))?;
    writer
        .write_all(bytes)
        .map_err(|source| payload_snapshot_error_v2("write", source))?;
    writer
        .flush()
        .map_err(|source| payload_snapshot_error_v2("flush", source))?;
    let seals = SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL;
    rustix::fs::fcntl_add_seals(&writer, seals)
        .map_err(|source| payload_snapshot_error_v2("sealing", source.into()))?;
    Ok(writer.into())
}

fn payload_snapshot_error_v2(
    operation: &'static str,
    source: io::Error,
) -> M1AllKernelsProductionProtectedVerifierErrorV1 {
    M1AllKernelsProductionProtectedVerifierErrorV1::PayloadSnapshot { operation, source }
}

fn authenticated_entry_coordinates_associate_v1(
    ordinal: usize,
    expected_ordinal: usize,
    expected_lineage: Option<[u8; 32]>,
    typed_lineage: [u8; 32],
    expected_marker: [u8; 32],
    expected_generated_host: [u8; 32],
    signed: &M1AllKernelsProtectedReceiptEntryV1,
) -> bool {
    usize::from(signed.ordinal()) == ordinal
        && expected_ordinal == ordinal
        && expected_lineage == Some(typed_lineage)
        && signed.lineage_identity() == typed_lineage
        && signed.marker_binding_identity() == expected_marker
        && signed.generated_host_contract_identity() == expected_generated_host
}

fn authenticated_entry_evidence_v1(
    request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
    pending: &M1AllKernelsPendingRequestProjectionV1,
    authenticated: &M1AllKernelsAuthenticatedProtectedVerifierReceiptV1,
) -> Result<
    Vec<WorkerV3ProtectedRosterEntryEvidenceV1>,
    M1AllKernelsProductionProtectedVerifierErrorV1,
> {
    let mut evidence = Vec::with_capacity(M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1);
    for (ordinal, (expected, signed)) in pending
        .entries
        .iter()
        .zip(authenticated.receipt().entries())
        .enumerate()
    {
        let lineage = request.entry_lineage_identity(ordinal).ok_or(
            M1AllKernelsProductionProtectedVerifierErrorV1::AuthenticatedReceiptAssociationFailed,
        )?;
        authenticated_entry_coordinates_associate_v1(
            ordinal,
            expected.ordinal,
            expected.lineage_identity,
            *lineage.as_bytes(),
            expected.marker_binding_identity,
            expected.generated_host_contract_identity,
            signed,
        )
        .then_some(())
        .ok_or(
            M1AllKernelsProductionProtectedVerifierErrorV1::AuthenticatedReceiptAssociationFailed,
        )?;
        // SAFETY: the one-shot client authenticated the complete signed receipt
        // under the caller policy and correlated all common and ordered entry
        // coordinates. The comparisons above repeat the typed lineage, marker,
        // host-contract, and ordinal join before mapping the signed theorem and
        // complete safety result for this exact entry.
        evidence.push(unsafe {
            WorkerV3ProtectedRosterEntryEvidenceV1::new(
                lineage,
                signed.marker_binding_identity(),
                signed.generated_host_contract_identity(),
                signed.proof_executable_binding_sha256(),
                signed.rust_type_layout_contract_sha256(),
                signed.rust_effect_contract_sha256(),
                signed.safety_properties(),
            )
        });
    }
    (evidence.len() == M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1)
        .then_some(evidence)
        .ok_or(
            M1AllKernelsProductionProtectedVerifierErrorV1::AuthenticatedReceiptAssociationFailed,
        )
}

fn authenticated_receipt_associates_v1(
    authenticated: &M1AllKernelsAuthenticatedProtectedVerifierReceiptV1,
    service_request: &M1AllKernelsProtectedVerifierServiceRequestV1,
    trust_policy: &M1AllKernelsProtectedVerifierTrustPolicyV1,
) -> bool {
    authenticated.policy_identity() == trust_policy.identity()
        && service_request.matches_receipt(authenticated.receipt())
        && authenticated.receipt().verifier_measurement_sha256()
            == trust_policy.verifier_measurement_sha256()
        && authenticated.receipt().checker_measurement_sha256()
            == trust_policy.checker_measurement_sha256()
}

// SAFETY: this implementation promotes only after it has locally reconstructed
// and cross-bound the exact finalizer, proof inputs, target lineage, and
// finalized HSACO. It transfers exact immutable envelope/HSACO snapshots,
// obtains the service challenge before acquiring challenge-bound FD195
// current-record evidence, and submits both complete canonical arrays. It
// authenticates the correlated terminal receipt before consuming that audit
// into compiler evidence, byte-compares the pre/post-bind service requests,
// and repeats every ordered entry join before consuming the local owners into
// fe2o3 aggregate evidence.
unsafe impl WorkerV3ProtectedRosterVerifierBackendV1<M1AllKernelsWorkerV3RosterV1>
    for M1AllKernelsProductionProtectedVerifierV1
{
    type Error = M1AllKernelsProductionProtectedVerifierErrorV1;

    unsafe fn verify_protected_roster(
        &mut self,
        request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
    ) -> Result<WorkerV3ProtectedRosterVerificationEvidenceV1, Self::Error> {
        let pending = M1AllKernelsPendingRequestProjectionV1::from_request(request).ok_or(
            M1AllKernelsProductionProtectedVerifierErrorV1::RosterRequestAssociationFailed,
        )?;
        let owners = locally_revalidate_request_v1(request, &pending)
            .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::LocalRevalidation)?;
        let client = self.client.take().ok_or(
            M1AllKernelsProductionProtectedVerifierErrorV1::ProtectedVerifierClientAlreadyConsumed,
        )?;
        let begin_challenge = self
            .begin_challenge
            .take()
            .ok_or(M1AllKernelsProductionProtectedVerifierErrorV1::BeginChallengeAlreadyConsumed)?;
        let generic_request = generic_verification_request_v1(
            request,
            &pending,
            &self.trust_policy,
            begin_challenge,
        )?;
        let snapshots = protected_payload_snapshots_v2(request)?;
        let reserved = client
            .begin(generic_request, snapshots)
            .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::ProtectedVerifierClient)?;
        let (service_challenge, pending_client) = reserved.into_parts();
        let compiler_challenge = service_challenge
            .into_compiler_execution_challenge()
            .map_err(
                M1AllKernelsProductionProtectedVerifierErrorV1::CompilerCurrentRecordChallenge,
            )?;
        let current_audit = self
            .current_auditor
            .audit_roster_with_challenge(request, compiler_challenge)
            .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::CompilerCurrentRecordAudit)?;
        let service_request = protected_service_request_from_current_audit_v1(
            request,
            &pending,
            &current_audit,
            &self.trust_policy,
        )?;
        let current_record = current_audit.canonical_evidence_view();
        let exact_verification = *current_record.verification_canonical_bytes();
        let exact_attestation = *current_record.attestation_canonical_bytes();
        let authenticated = pending_client
            .submit_current_record(
                exact_verification,
                exact_attestation,
                &self.trust_policy,
                &service_request,
            )
            .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::ProtectedVerifierClient)?;
        authenticated_receipt_associates_v1(
            &authenticated,
            &service_request,
            &self.trust_policy,
        )
        .then_some(())
        .ok_or(
            M1AllKernelsProductionProtectedVerifierErrorV1::AuthenticatedReceiptAssociationFailed,
        )?;
        let compiler_execution = current_audit
            .bind_exact_compiler_execution_v1(
                request.compiler_execution_subject(),
                request.compiler_execution_receipt_carriage(),
            )
            .map_err(M1AllKernelsProductionProtectedVerifierErrorV1::CompilerExecutionBinding)?;
        let bound_service_request = protected_service_request_v1(
            request,
            &pending,
            &compiler_execution,
            &self.trust_policy,
        )?;
        (bound_service_request.canonical_bytes() == service_request.canonical_bytes())
            .then_some(())
            .ok_or(
                M1AllKernelsProductionProtectedVerifierErrorV1::CompilerExecutionAssociationFailed,
            )?;
        let entries = authenticated_entry_evidence_v1(request, &pending, &authenticated)?;
        let verifier_measurement = authenticated.receipt().verifier_measurement_sha256();
        let verification_transcript = authenticated.receipt().verification_transcript_sha256();
        let M1AllKernelsLocallyRevalidatedOwnersV1 {
            finalizer,
            proof_inputs,
            target_lineage,
            hsaco_reinspection,
        } = owners;
        drop(hsaco_reinspection);
        // SAFETY: local owner association, authenticated receipt correlation,
        // and every canonical signed entry join are established above. Moving
        // the finalizer, bound current audit, proof-input, and target-lineage
        // owners prevents reuse outside this exact aggregate evidence value.
        Ok(unsafe {
            WorkerV3ProtectedRosterVerificationEvidenceV1::new(
                finalizer,
                compiler_execution,
                proof_inputs,
                target_lineage,
                verifier_measurement,
                verification_transcript,
                entries,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt_entry(
        ordinal: u16,
        lineage: [u8; 32],
        marker: [u8; 32],
        generated_host: [u8; 32],
    ) -> M1AllKernelsProtectedReceiptEntryV1 {
        M1AllKernelsProtectedReceiptEntryV1::new(
            ordinal,
            lineage,
            marker,
            generated_host,
            [4; 32],
            [5; 32],
            [6; 32],
            fe2o3_host::WorkerV3SafetyPropertiesV1::required(),
        )
        .expect("nonzero complete signed entry")
    }

    #[test]
    fn aggregate_roster_cardinality_is_exact() {
        assert_eq!(M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1, 12);
        assert_eq!(
            M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1,
            M1AllKernelsWorkerV3RosterV1::ENTRIES.len()
        );
    }

    #[test]
    fn production_rejection_is_structured_and_unconditional() {
        let error = missing_protected_verification_receipt_v1();
        assert_eq!(
            error,
            M1AllKernelsProtectedVerifierErrorV1::MissingProtectedVerificationReceipt {
                expected_roster_entries: 12,
            }
        );
        assert_eq!(
            error.to_string(),
            "missing protected verification receipt for all 12 aggregate M1 roster entries"
        );
    }

    #[test]
    fn common_custody_preflight_failures_are_distinct() {
        assert_eq!(
            M1AllKernelsProtectedVerifierErrorV1::FinalizerDerivationRevalidationFailed.to_string(),
            "independent finalizer derivation revalidation failed"
        );
        assert_eq!(
            M1AllKernelsProtectedVerifierErrorV1::CompilerMultiRootProofInputsValidationFailed
                .to_string(),
            "common multi-root compiler proof input validation failed"
        );
        assert_eq!(
            M1AllKernelsProtectedVerifierErrorV1::CompilerMultiRootTargetLineageValidationFailed
                .to_string(),
            "common multi-root compiler target-lineage validation failed"
        );
        assert_eq!(
            M1AllKernelsProtectedVerifierErrorV1::ExactFinalizedHsacoVerificationFailed.to_string(),
            "exact finalized HSACO verification failed"
        );
        assert_eq!(
            M1AllKernelsProtectedVerifierErrorV1::FinalizerArtifactAssociationFailed.to_string(),
            "finalizer and exact artifact association failed"
        );
        assert_eq!(
            M1AllKernelsProtectedVerifierErrorV1::CompilerTargetAssociationFailed.to_string(),
            "compiler module and target association failed"
        );
        assert_eq!(
            M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed.to_string(),
            "aggregate roster entry association failed"
        );
    }

    #[test]
    fn missing_receipt_count_uses_default_decimal_formatting() {
        let error = missing_protected_verification_receipt_v1();
        let expected =
            "missing protected verification receipt for all 12 aggregate M1 roster entries";
        assert_eq!(format!("{error:010}"), expected);
        assert_eq!(format!("{error:+}"), expected);
        assert_eq!(format!("{error:>120}"), expected);
    }

    #[test]
    fn exact_authenticated_entry_coordinates_associate() {
        let signed = receipt_entry(0, [1; 32], [2; 32], [3; 32]);
        assert!(authenticated_entry_coordinates_associate_v1(
            0,
            0,
            Some([1; 32]),
            [1; 32],
            [2; 32],
            [3; 32],
            &signed,
        ));
    }

    #[test]
    fn every_authenticated_entry_coordinate_substitution_fails_closed() {
        let exact = receipt_entry(0, [1; 32], [2; 32], [3; 32]);
        let wrong_ordinal = receipt_entry(1, [1; 32], [2; 32], [3; 32]);
        let wrong_lineage = receipt_entry(0, [7; 32], [2; 32], [3; 32]);
        let wrong_marker = receipt_entry(0, [1; 32], [7; 32], [3; 32]);
        let wrong_generated_host = receipt_entry(0, [1; 32], [2; 32], [7; 32]);
        for (signed, ordinal, expected_ordinal, pending_lineage, typed_lineage) in [
            (&wrong_ordinal, 0, 0, Some([1; 32]), [1; 32]),
            (&exact, 0, 1, Some([1; 32]), [1; 32]),
            (&exact, 0, 0, None, [1; 32]),
            (&wrong_lineage, 0, 0, Some([1; 32]), [1; 32]),
            (&exact, 0, 0, Some([1; 32]), [7; 32]),
        ] {
            assert!(!authenticated_entry_coordinates_associate_v1(
                ordinal,
                expected_ordinal,
                pending_lineage,
                typed_lineage,
                [2; 32],
                [3; 32],
                signed,
            ));
        }
        assert!(!authenticated_entry_coordinates_associate_v1(
            0,
            0,
            Some([1; 32]),
            [1; 32],
            [2; 32],
            [3; 32],
            &wrong_marker,
        ));
        assert!(!authenticated_entry_coordinates_associate_v1(
            0,
            0,
            Some([1; 32]),
            [1; 32],
            [2; 32],
            [3; 32],
            &wrong_generated_host,
        ));
    }
}
