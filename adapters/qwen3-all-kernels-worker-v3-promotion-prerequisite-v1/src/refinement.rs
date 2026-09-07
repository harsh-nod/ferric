//! Direct refinement model for a successful promotion-prerequisite collection.
//!
//! The model starts only after the unverified operating-system, filesystem,
//! hashing, decoding, and cryptographic bodies have produced their individual
//! boolean observations. It proves that the final executable acceptance gate
//! requires every observation, all 12 ordered roster positions, and an
//! authority-free result. It does not prove those external operations.

#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

/// Exact ordered all-12 entry correlation observed by the collector.
// Each Boolean is an independent ordered roster coordinate in the refinement theorem.
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct M1AllKernelsOrderedEntryRefinementV1 {
    pub(super) entry_00: bool,
    pub(super) entry_01: bool,
    pub(super) entry_02: bool,
    pub(super) entry_03: bool,
    pub(super) entry_04: bool,
    pub(super) entry_05: bool,
    pub(super) entry_06: bool,
    pub(super) entry_07: bool,
    pub(super) entry_08: bool,
    pub(super) entry_09: bool,
    pub(super) entry_10: bool,
    pub(super) entry_11: bool,
}

impl M1AllKernelsOrderedEntryRefinementV1 {
    pub(crate) open spec fn all_match_spec(&self) -> bool {
        &&& self.entry_00
        &&& self.entry_01
        &&& self.entry_02
        &&& self.entry_03
        &&& self.entry_04
        &&& self.entry_05
        &&& self.entry_06
        &&& self.entry_07
        &&& self.entry_08
        &&& self.entry_09
        &&& self.entry_10
        &&& self.entry_11
    }

    pub(crate) fn all_match(&self) -> (matches: bool)
        ensures matches == self.all_match_spec(),
    {
        self.entry_00
            && self.entry_01
            && self.entry_02
            && self.entry_03
            && self.entry_04
            && self.entry_05
            && self.entry_06
            && self.entry_07
            && self.entry_08
            && self.entry_09
            && self.entry_10
            && self.entry_11
    }
}

/// Every independently checked coordinate needed for collector success.
// Keeping the coordinates explicit makes omission visible in the proof conjunction.
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct M1AllKernelsCollectorRefinementV1 {
    pub(super) protected_receipt_authenticated: bool,
    pub(super) protected_service_request: bool,
    pub(super) recovered_current_publication: bool,
    pub(super) source_pin: bool,
    pub(super) finalized_hsaco_sha256: bool,
    pub(super) finalized_hsaco_length: bool,
    pub(super) compiler_issuer_policy: bool,
    pub(super) current_verification_transport: bool,
    pub(super) current_attestation: bool,
    pub(super) compiler_subject: bool,
    pub(super) compiler_carriage: bool,
    pub(super) compiler_policy: bool,
    pub(super) compiler_issuer_journal: bool,
    pub(super) compiler_occurrence: bool,
    pub(super) compiler_receipt: bool,
    pub(super) compiler_publication: bool,
    pub(super) compiler_acknowledgment: bool,
    pub(super) compiler_worker_ledger: bool,
    pub(super) compiler_sequence: bool,
    pub(super) compiler_prior_rollback_anchor: bool,
    pub(super) compiler_current_rollback_anchor: bool,
    pub(super) current_verification_identity: bool,
    pub(super) current_attestation_identity: bool,
    pub(super) protected_policy_verification: bool,
    pub(super) protected_worker_ledger_verification: bool,
    pub(super) external_rollback_verification: bool,
    pub(super) current_token_lease_binding: bool,
    pub(super) locked_currentness_revalidated: bool,
    pub(super) ordered_entries: M1AllKernelsOrderedEntryRefinementV1,
}

impl M1AllKernelsCollectorRefinementV1 {
    pub(crate) open spec fn accepts_spec(&self) -> bool {
        &&& self.protected_receipt_authenticated
        &&& self.protected_service_request
        &&& self.recovered_current_publication
        &&& self.source_pin
        &&& self.finalized_hsaco_sha256
        &&& self.finalized_hsaco_length
        &&& self.compiler_issuer_policy
        &&& self.current_verification_transport
        &&& self.current_attestation
        &&& self.compiler_subject
        &&& self.compiler_carriage
        &&& self.compiler_policy
        &&& self.compiler_issuer_journal
        &&& self.compiler_occurrence
        &&& self.compiler_receipt
        &&& self.compiler_publication
        &&& self.compiler_acknowledgment
        &&& self.compiler_worker_ledger
        &&& self.compiler_sequence
        &&& self.compiler_prior_rollback_anchor
        &&& self.compiler_current_rollback_anchor
        &&& self.current_verification_identity
        &&& self.current_attestation_identity
        &&& self.protected_policy_verification
        &&& self.protected_worker_ledger_verification
        &&& self.external_rollback_verification
        &&& self.current_token_lease_binding
        &&& self.locked_currentness_revalidated
        &&& self.ordered_entries.all_match_spec()
    }
}

/// Authority-free projection returned by the executable refinement gate.
// These independent negative-authority facts are the theorem's public result coordinates.
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct M1AllKernelsCollectorRefinementOutcomeV1 {
    pub(super) accepted: bool,
    pub(super) is_current: bool,
    pub(super) grants_publication_authority: bool,
    pub(super) grants_verifier_authority: bool,
    pub(super) grants_load_authority: bool,
    pub(super) grants_launch_authority: bool,
    pub(super) requires_external_promotion_service: bool,
}

impl M1AllKernelsCollectorRefinementOutcomeV1 {
    pub(crate) open spec fn is_authority_free_spec(&self) -> bool {
        &&& !self.is_current
        &&& !self.grants_publication_authority
        &&& !self.grants_verifier_authority
        &&& !self.grants_load_authority
        &&& !self.grants_launch_authority
        &&& self.requires_external_promotion_service
    }

    pub(crate) fn is_authority_free(&self) -> (authority_free: bool)
        ensures authority_free == self.is_authority_free_spec(),
    {
        !self.is_current
            && !self.grants_publication_authority
            && !self.grants_verifier_authority
            && !self.grants_load_authority
            && !self.grants_launch_authority
            && self.requires_external_promotion_service
    }
}

/// Checks the exact collector-success conjunction and returns no authority.
pub(crate) fn validate_m1_all_kernels_collector_refinement_v1(
    model: &M1AllKernelsCollectorRefinementV1,
) -> (outcome: M1AllKernelsCollectorRefinementOutcomeV1)
    ensures
        outcome.accepted == model.accepts_spec(),
        !outcome.is_current,
        !outcome.grants_publication_authority,
        !outcome.grants_verifier_authority,
        !outcome.grants_load_authority,
        !outcome.grants_launch_authority,
        outcome.requires_external_promotion_service,
        outcome.is_authority_free_spec(),
{
    M1AllKernelsCollectorRefinementOutcomeV1 {
        accepted: model.protected_receipt_authenticated
            && model.protected_service_request
            && model.recovered_current_publication
            && model.source_pin
            && model.finalized_hsaco_sha256
            && model.finalized_hsaco_length
            && model.compiler_issuer_policy
            && model.current_verification_transport
            && model.current_attestation
            && model.compiler_subject
            && model.compiler_carriage
            && model.compiler_policy
            && model.compiler_issuer_journal
            && model.compiler_occurrence
            && model.compiler_receipt
            && model.compiler_publication
            && model.compiler_acknowledgment
            && model.compiler_worker_ledger
            && model.compiler_sequence
            && model.compiler_prior_rollback_anchor
            && model.compiler_current_rollback_anchor
            && model.current_verification_identity
            && model.current_attestation_identity
            && model.protected_policy_verification
            && model.protected_worker_ledger_verification
            && model.external_rollback_verification
            && model.current_token_lease_binding
            && model.locked_currentness_revalidated
            && model.ordered_entries.all_match(),
        is_current: false,
        grants_publication_authority: false,
        grants_verifier_authority: false,
        grants_load_authority: false,
        grants_launch_authority: false,
        requires_external_promotion_service: true,
    }
}

/// Exposes every coordinate implied by a successful refinement result.
pub(crate) proof fn successful_refinement_binds_every_coordinate_v1(
    model: &M1AllKernelsCollectorRefinementV1,
    outcome: &M1AllKernelsCollectorRefinementOutcomeV1,
)
    requires
        outcome.accepted,
        outcome.accepted == model.accepts_spec(),
    ensures
        model.protected_receipt_authenticated,
        model.protected_service_request,
        model.recovered_current_publication,
        model.source_pin,
        model.finalized_hsaco_sha256,
        model.finalized_hsaco_length,
        model.compiler_issuer_policy,
        model.current_verification_transport,
        model.current_attestation,
        model.compiler_subject,
        model.compiler_carriage,
        model.compiler_policy,
        model.compiler_issuer_journal,
        model.compiler_occurrence,
        model.compiler_receipt,
        model.compiler_publication,
        model.compiler_acknowledgment,
        model.compiler_worker_ledger,
        model.compiler_sequence,
        model.compiler_prior_rollback_anchor,
        model.compiler_current_rollback_anchor,
        model.current_verification_identity,
        model.current_attestation_identity,
        model.protected_policy_verification,
        model.protected_worker_ledger_verification,
        model.external_rollback_verification,
        model.current_token_lease_binding,
        model.locked_currentness_revalidated,
        model.ordered_entries.entry_00,
        model.ordered_entries.entry_01,
        model.ordered_entries.entry_02,
        model.ordered_entries.entry_03,
        model.ordered_entries.entry_04,
        model.ordered_entries.entry_05,
        model.ordered_entries.entry_06,
        model.ordered_entries.entry_07,
        model.ordered_entries.entry_08,
        model.ordered_entries.entry_09,
        model.ordered_entries.entry_10,
        model.ordered_entries.entry_11,
{
}

} // verus!
