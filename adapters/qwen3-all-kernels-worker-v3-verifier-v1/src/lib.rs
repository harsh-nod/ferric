//! Fail-closed Ferric adapter for the aggregate Qwen3 Worker V3 verifier boundary.
//!
//! M1 does not yet possess an independently produced protected-verification
//! receipt for the aggregate 12-marker artifact. This backend therefore makes
//! the integration boundary explicit while refusing every verification request.
//! Before that refusal, it independently reacquires the common finalizer and
//! compiler proof owners carried by the exact typed request.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::error::Error;
use std::fmt;

use fe2o3_host::{
    CompilerGeneratedKernelExpectationRosterV1, WorkerV3ProtectedRosterVerificationEvidenceV1,
    WorkerV3ProtectedRosterVerifierBackendV1, WorkerV3RosterVerificationRequestV1,
};
use ferric_qwen3_all_kernels_device_v1::M1AllKernelsWorkerV3RosterV1;

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
    ) -> M1AllKernelsPendingEntryProjectionV1 {
        let marker_entries = request.marker_entries();
        let marker = &marker_entries[ordinal];
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
    ) -> Self {
        let entries = std::array::from_fn(|ordinal| Self::entry_from_request(request, ordinal));
        Self {
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
        }
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

    fn reject_missing_protected_receipt<FinalizerOwner, ProofInputsOwner, TargetLineageOwner>(
        _request: &M1AllKernelsPendingRequestProjectionV1,
        _finalizer_owner: FinalizerOwner,
        _proof_inputs_owner: ProofInputsOwner,
        _target_lineage_owner: TargetLineageOwner,
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
        let pending_request = M1AllKernelsPendingRequestProjectionV1::from_request(request);
        let finalizer_owner = request
            .independently_revalidate_finalizer_derivation()
            .map_err(|_| {
                M1AllKernelsProtectedVerifierErrorV1::FinalizerDerivationRevalidationFailed
            })?;
        let proof_inputs_owner = request
            .validate_compiler_multi_root_proof_inputs_v1()
            .map_err(|_| {
                M1AllKernelsProtectedVerifierErrorV1::CompilerMultiRootProofInputsValidationFailed
            })?;
        let target_lineage_owner = request
            .validate_compiler_multi_root_target_lineage_v1(&proof_inputs_owner)
            .map_err(|_| {
                M1AllKernelsProtectedVerifierErrorV1::CompilerMultiRootTargetLineageValidationFailed
            })?;
        Self::reject_missing_protected_receipt(
            &pending_request,
            finalizer_owner,
            proof_inputs_owner,
            target_lineage_owner,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
