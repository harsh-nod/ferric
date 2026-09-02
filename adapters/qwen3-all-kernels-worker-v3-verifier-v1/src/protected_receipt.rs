//! Canonical aggregate receipt wire for a future protected verifier service.
//!
//! Decoding proves canonical structure only. Authentication proves that an
//! exact receipt was signed under one caller-provisioned policy. The wire
//! carries the complete coordinate set needed by a later protected
//! backend integration. This module does not construct fe2o3 protected
//! evidence or grant verification, load, launch, or inference authority.

#![allow(
    clippy::must_use_candidate,
    reason = "every public value is an inert receipt or coordinate accessor; authority-bearing transitions remain fallible"
)]
#![forbid(unsafe_code)]

use core::fmt;
use std::error::Error;

use ed25519_dalek::{Signature, VerifyingKey};
use fe2o3_host::WorkerV3SafetyPropertiesV1;
use sha2::{Digest, Sha256};

use crate::M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1;

const SHA256_BYTES: usize = 32;
const SIGNATURE_BYTES: usize = 64;
const HEADER_BYTES: usize = 32;
const ENTRY_BYTES: usize = 200;
const COMMON_BYTES: usize = 1_056;
const UNSIGNED_BYTES: usize =
    HEADER_BYTES + COMMON_BYTES + (M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1 * ENTRY_BYTES);
const TARGET_BYTES: usize = 16;
const TARGET: &[u8] = b"gfx942:xnack-";
const CODE_OBJECT_VERSION: u16 = 6;
const MAGIC: [u8; 8] = *b"FRW3PR1\0";
const VERSION: u16 = 1;
const SIGNING_DOMAIN: &[u8] = b"FERRIC/M1/ALL-KERNELS/PROTECTED-VERIFIER/RECEIPT-SIGNATURE/V1\0";
const RECEIPT_IDENTITY_DOMAIN: &[u8] =
    b"FERRIC/M1/ALL-KERNELS/PROTECTED-VERIFIER/RECEIPT-IDENTITY/V1\0";
const POLICY_IDENTITY_DOMAIN: &[u8] = b"FERRIC/M1/ALL-KERNELS/PROTECTED-VERIFIER/TRUST-POLICY/V1\0";

/// Exact canonical aggregate protected-receipt byte length.
pub const M1_ALL_KERNELS_PROTECTED_RECEIPT_BYTES_V1: usize = UNSIGNED_BYTES + SIGNATURE_BYTES;

/// Exact canonical unsigned aggregate protected-receipt byte length.
pub const M1_ALL_KERNELS_PROTECTED_RECEIPT_UNSIGNED_BYTES_V1: usize = UNSIGNED_BYTES;

/// Field containing an invalid all-zero identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1AllKernelsProtectedReceiptIdentityFieldV1 {
    /// Caller-provisioned protected-verifier trust policy.
    TrustPolicy,
    /// Exact host aggregate verification challenge.
    Challenge,
    /// Exact canonical aggregate roster.
    Roster,
    /// Exact aggregate host lineage.
    HostLineage,
    /// Independently replayed finalizer derivation.
    FinalizerDerivation,
    /// Neutral compiler module in the source pin.
    CompilerModule,
    /// Nested compiler handoff in the source pin.
    CompilerHandoff,
    /// Compiler symbol manifest in the source pin.
    SymbolManifest,
    /// Compiler-execution subject.
    CompilerExecutionSubject,
    /// Complete compiler-execution carriage.
    CompilerExecutionCarriage,
    /// Compiler-execution issuer policy.
    CompilerExecutionPolicy,
    /// Compiler-execution issuer journal.
    CompilerExecutionIssuerJournal,
    /// Compiler occurrence.
    CompilerOccurrence,
    /// Signed compiler-execution receipt.
    CompilerExecutionReceipt,
    /// Compiler-execution publication.
    CompilerExecutionPublication,
    /// Worker acknowledgment.
    CompilerExecutionAcknowledgment,
    /// Protected Worker ledger record.
    CompilerExecutionWorkerLedgerRecord,
    /// Current rollback anchor.
    CompilerExecutionCurrentRollbackAnchor,
    /// Signed current-record verification.
    CurrentRecordVerification,
    /// Fresh current-record attestation.
    CurrentRecordAttestation,
    /// Protected compiler-policy verification result.
    ProtectedCompilerPolicyVerification,
    /// Protected Worker-ledger verification result.
    ProtectedWorkerLedgerVerification,
    /// External rollback verification result.
    ExternalRollbackVerification,
    /// Exact semantic capsule.
    Capsule,
    /// Exact formal-memory receipt.
    FormalMemoryReceipt,
    /// Exact proof-binding receipt.
    ProofBindingReceipt,
    /// Exact finalized HSACO.
    FinalizedHsaco,
    /// Measured protected-verifier closure.
    VerifierMeasurement,
    /// Measured independent checker closure.
    CheckerMeasurement,
    /// Complete protected verification transcript.
    VerificationTranscript,
    /// Per-entry host lineage.
    EntryLineage,
    /// Per-entry compiler-generated marker binding.
    EntryMarkerBinding,
    /// Per-entry generated host contract.
    EntryGeneratedHostContract,
    /// Per-entry proof-to-executable theorem.
    EntryProofExecutableBinding,
    /// Per-entry Rust type-layout theorem.
    EntryRustTypeLayoutContract,
    /// Per-entry Rust effect theorem.
    EntryRustEffectContract,
}

/// Length field rejected because it is zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1AllKernelsProtectedReceiptLengthFieldV1 {
    /// Neutral compiler module byte length.
    CompilerModule,
    /// Nested compiler handoff byte length.
    CompilerHandoff,
    /// Compiler symbol-manifest byte length.
    SymbolManifest,
    /// Finalized HSACO byte length.
    FinalizedHsaco,
}

/// Duplicate coordinate rejected across the canonical entry roster.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1AllKernelsProtectedReceiptDuplicateFieldV1 {
    /// Host lineage identity.
    Lineage,
    /// Compiler-generated marker binding identity.
    MarkerBinding,
}

/// Canonical receipt, trust-policy, authentication, or binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1AllKernelsProtectedReceiptErrorV1 {
    /// Wire length is not the one exact V1 length.
    InvalidLength {
        /// Required byte length.
        expected: usize,
        /// Observed byte length.
        actual: usize,
    },
    /// Wire magic differs from the V1 schema.
    InvalidMagic,
    /// Wire version is unsupported.
    UnsupportedVersion {
        /// Observed version.
        actual: u16,
    },
    /// Entry count is not exactly the aggregate M1 cardinality.
    InvalidEntryCount {
        /// Observed entry count.
        actual: u16,
    },
    /// Header-size declaration is noncanonical.
    InvalidHeaderLength {
        /// Observed header length.
        actual: u16,
    },
    /// Entry-size declaration is noncanonical.
    InvalidEntryLength {
        /// Observed entry length.
        actual: u16,
    },
    /// Total-size declaration is noncanonical.
    InvalidDeclaredLength {
        /// Observed total length.
        actual: u32,
    },
    /// A flags or reserved field is nonzero.
    NonzeroReserved,
    /// A required identity is the all-zero sentinel.
    ZeroIdentity {
        /// Rejected identity field.
        field: M1AllKernelsProtectedReceiptIdentityFieldV1,
        /// Entry ordinal for an entry-scoped field.
        ordinal: Option<u16>,
    },
    /// A required content length is zero.
    ZeroLength {
        /// Rejected length field.
        field: M1AllKernelsProtectedReceiptLengthFieldV1,
    },
    /// Compiler-execution rollback sequence is zero.
    ZeroCompilerExecutionSequence,
    /// Prior and current rollback anchors are identical.
    UnadvancedCompilerExecutionRollbackAnchor,
    /// The target bytes are not the exact canonical gfx942 target.
    InvalidTarget,
    /// The code-object version is not V6.
    InvalidCodeObjectVersion {
        /// Observed version.
        actual: u16,
    },
    /// Entry ordinal differs from its canonical descriptor-table position.
    InvalidEntryOrdinal {
        /// Required ordinal.
        expected: u16,
        /// Observed ordinal.
        actual: u16,
    },
    /// One entry lineage or marker identity occurs more than once.
    DuplicateEntryIdentity {
        /// Duplicated coordinate.
        field: M1AllKernelsProtectedReceiptDuplicateFieldV1,
        /// Second ordinal carrying the duplicate.
        ordinal: u16,
    },
    /// Entry does not claim the complete V1 safety-property set.
    MissingRequiredSafetyProperties {
        /// Rejected entry ordinal.
        ordinal: u16,
        /// Observed property bits.
        actual: u8,
    },
    /// Verifier and checker measurements must describe distinct closures.
    AliasedVerifierAndCheckerMeasurements,
    /// Caller-supplied Ed25519 verifying key is not canonical.
    InvalidVerifyingKey,
    /// Caller-supplied Ed25519 verifying key is weak.
    WeakVerifyingKey,
    /// Receipt names a different trust-policy identity.
    TrustPolicyMismatch,
    /// Receipt names a different verifier measurement.
    VerifierMeasurementMismatch,
    /// Receipt names a different checker measurement.
    CheckerMeasurementMismatch,
    /// Strict Ed25519 verification rejected the signature.
    SignatureRejected,
}

impl fmt::Display for M1AllKernelsProtectedReceiptErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "aggregate M1 protected receipt rejected: {self:?}"
        )
    }
}

impl Error for M1AllKernelsProtectedReceiptErrorV1 {}

/// Identity of one exact caller-provisioned trust policy.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct M1AllKernelsProtectedVerifierTrustPolicyIdentityV1([u8; SHA256_BYTES]);

impl M1AllKernelsProtectedVerifierTrustPolicyIdentityV1 {
    /// Returns the exact domain-separated identity bytes.
    pub const fn as_bytes(&self) -> &[u8; SHA256_BYTES] {
        &self.0
    }
}

/// Identity of one exact canonical signed receipt.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct M1AllKernelsProtectedVerifierReceiptIdentityV1([u8; SHA256_BYTES]);

impl M1AllKernelsProtectedVerifierReceiptIdentityV1 {
    /// Returns the exact domain-separated identity bytes.
    pub const fn as_bytes(&self) -> &[u8; SHA256_BYTES] {
        &self.0
    }

    /// Recomputes this identity after strict canonical decoding.
    #[must_use]
    pub fn matches_canonical_bytes(self, bytes: &[u8]) -> bool {
        M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(bytes)
            .is_ok_and(|receipt| receipt.identity == self)
    }
}

/// Six exact compiler source coordinates for the aggregate artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1AllKernelsProtectedReceiptSourcePinV1 {
    compiler_module_sha256: [u8; SHA256_BYTES],
    compiler_module_length: u64,
    compiler_handoff_sha256: [u8; SHA256_BYTES],
    compiler_handoff_length: u64,
    symbol_manifest_sha256: [u8; SHA256_BYTES],
    symbol_manifest_length: u64,
}

impl M1AllKernelsProtectedReceiptSourcePinV1 {
    /// Constructs an inert, nonzero source-pin coordinate set.
    ///
    /// # Errors
    ///
    /// Returns a typed error when any digest or byte length is zero.
    pub fn new(
        compiler_module_sha256: [u8; SHA256_BYTES],
        compiler_module_length: u64,
        compiler_handoff_sha256: [u8; SHA256_BYTES],
        compiler_handoff_length: u64,
        symbol_manifest_sha256: [u8; SHA256_BYTES],
        symbol_manifest_length: u64,
    ) -> Result<Self, M1AllKernelsProtectedReceiptErrorV1> {
        require_identity(
            compiler_module_sha256,
            M1AllKernelsProtectedReceiptIdentityFieldV1::CompilerModule,
            None,
        )?;
        require_length(
            compiler_module_length,
            M1AllKernelsProtectedReceiptLengthFieldV1::CompilerModule,
        )?;
        require_identity(
            compiler_handoff_sha256,
            M1AllKernelsProtectedReceiptIdentityFieldV1::CompilerHandoff,
            None,
        )?;
        require_length(
            compiler_handoff_length,
            M1AllKernelsProtectedReceiptLengthFieldV1::CompilerHandoff,
        )?;
        require_identity(
            symbol_manifest_sha256,
            M1AllKernelsProtectedReceiptIdentityFieldV1::SymbolManifest,
            None,
        )?;
        require_length(
            symbol_manifest_length,
            M1AllKernelsProtectedReceiptLengthFieldV1::SymbolManifest,
        )?;
        Ok(Self {
            compiler_module_sha256,
            compiler_module_length,
            compiler_handoff_sha256,
            compiler_handoff_length,
            symbol_manifest_sha256,
            symbol_manifest_length,
        })
    }

    /// Returns the neutral compiler-module digest.
    pub const fn compiler_module_sha256(&self) -> [u8; SHA256_BYTES] {
        self.compiler_module_sha256
    }

    /// Returns the neutral compiler-module byte length.
    pub const fn compiler_module_length(&self) -> u64 {
        self.compiler_module_length
    }

    /// Returns the nested compiler-handoff digest.
    pub const fn compiler_handoff_sha256(&self) -> [u8; SHA256_BYTES] {
        self.compiler_handoff_sha256
    }

    /// Returns the nested compiler-handoff byte length.
    pub const fn compiler_handoff_length(&self) -> u64 {
        self.compiler_handoff_length
    }

    /// Returns the compiler symbol-manifest digest.
    pub const fn symbol_manifest_sha256(&self) -> [u8; SHA256_BYTES] {
        self.symbol_manifest_sha256
    }

    /// Returns the compiler symbol-manifest byte length.
    pub const fn symbol_manifest_length(&self) -> u64 {
        self.symbol_manifest_length
    }

    fn encode_into(&self, writer: &mut Writer) {
        writer.identity(self.compiler_module_sha256);
        writer.u64(self.compiler_module_length);
        writer.identity(self.compiler_handoff_sha256);
        writer.u64(self.compiler_handoff_length);
        writer.identity(self.symbol_manifest_sha256);
        writer.u64(self.symbol_manifest_length);
    }

    fn decode(reader: &mut Reader<'_>) -> Result<Self, M1AllKernelsProtectedReceiptErrorV1> {
        Self::new(
            reader.identity()?,
            reader.u64()?,
            reader.identity()?,
            reader.u64()?,
            reader.identity()?,
            reader.u64()?,
        )
    }
}

/// Host-known common request coordinates covered by the protected receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1AllKernelsProtectedReceiptRequestClaimsV1 {
    challenge_identity: [u8; SHA256_BYTES],
    roster_identity: [u8; SHA256_BYTES],
    host_lineage_identity: [u8; SHA256_BYTES],
    finalizer_derivation_sha256: [u8; SHA256_BYTES],
    source_pin: M1AllKernelsProtectedReceiptSourcePinV1,
    capsule_sha256: [u8; SHA256_BYTES],
    formal_memory_receipt_sha256: [u8; SHA256_BYTES],
    proof_binding_receipt_sha256: [u8; SHA256_BYTES],
    finalized_hsaco_sha256: [u8; SHA256_BYTES],
    finalized_hsaco_length: u64,
}

impl M1AllKernelsProtectedReceiptRequestClaimsV1 {
    /// Constructs inert host-known receipt claims.
    ///
    /// # Errors
    ///
    /// Returns a typed error when any required identity or finalized-artifact length is zero.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        challenge_identity: [u8; SHA256_BYTES],
        roster_identity: [u8; SHA256_BYTES],
        host_lineage_identity: [u8; SHA256_BYTES],
        finalizer_derivation_sha256: [u8; SHA256_BYTES],
        source_pin: M1AllKernelsProtectedReceiptSourcePinV1,
        capsule_sha256: [u8; SHA256_BYTES],
        formal_memory_receipt_sha256: [u8; SHA256_BYTES],
        proof_binding_receipt_sha256: [u8; SHA256_BYTES],
        finalized_hsaco_sha256: [u8; SHA256_BYTES],
        finalized_hsaco_length: u64,
    ) -> Result<Self, M1AllKernelsProtectedReceiptErrorV1> {
        for (identity, field) in [
            (
                challenge_identity,
                M1AllKernelsProtectedReceiptIdentityFieldV1::Challenge,
            ),
            (
                roster_identity,
                M1AllKernelsProtectedReceiptIdentityFieldV1::Roster,
            ),
            (
                host_lineage_identity,
                M1AllKernelsProtectedReceiptIdentityFieldV1::HostLineage,
            ),
            (
                finalizer_derivation_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::FinalizerDerivation,
            ),
            (
                capsule_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::Capsule,
            ),
            (
                formal_memory_receipt_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::FormalMemoryReceipt,
            ),
            (
                proof_binding_receipt_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::ProofBindingReceipt,
            ),
            (
                finalized_hsaco_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::FinalizedHsaco,
            ),
        ] {
            require_identity(identity, field, None)?;
        }
        require_length(
            finalized_hsaco_length,
            M1AllKernelsProtectedReceiptLengthFieldV1::FinalizedHsaco,
        )?;
        Ok(Self {
            challenge_identity,
            roster_identity,
            host_lineage_identity,
            finalizer_derivation_sha256,
            source_pin,
            capsule_sha256,
            formal_memory_receipt_sha256,
            proof_binding_receipt_sha256,
            finalized_hsaco_sha256,
            finalized_hsaco_length,
        })
    }

    /// Returns the exact host challenge.
    pub const fn challenge_identity(&self) -> [u8; SHA256_BYTES] {
        self.challenge_identity
    }

    /// Returns the exact aggregate roster identity.
    pub const fn roster_identity(&self) -> [u8; SHA256_BYTES] {
        self.roster_identity
    }

    /// Returns the aggregate host-lineage identity.
    pub const fn host_lineage_identity(&self) -> [u8; SHA256_BYTES] {
        self.host_lineage_identity
    }

    /// Returns the independently replayed finalizer identity.
    pub const fn finalizer_derivation_sha256(&self) -> [u8; SHA256_BYTES] {
        self.finalizer_derivation_sha256
    }

    /// Returns the exact compiler source pin.
    pub const fn source_pin(&self) -> M1AllKernelsProtectedReceiptSourcePinV1 {
        self.source_pin
    }

    /// Returns the exact semantic-capsule digest.
    pub const fn capsule_sha256(&self) -> [u8; SHA256_BYTES] {
        self.capsule_sha256
    }

    /// Returns the exact formal-memory receipt digest.
    pub const fn formal_memory_receipt_sha256(&self) -> [u8; SHA256_BYTES] {
        self.formal_memory_receipt_sha256
    }

    /// Returns the exact proof-binding receipt digest.
    pub const fn proof_binding_receipt_sha256(&self) -> [u8; SHA256_BYTES] {
        self.proof_binding_receipt_sha256
    }

    /// Returns the exact finalized-HSACO digest.
    pub const fn finalized_hsaco_sha256(&self) -> [u8; SHA256_BYTES] {
        self.finalized_hsaco_sha256
    }

    /// Returns the exact finalized-HSACO byte length.
    pub const fn finalized_hsaco_length(&self) -> u64 {
        self.finalized_hsaco_length
    }

    fn encode_prefix_into(&self, writer: &mut Writer) {
        writer.identity(self.challenge_identity);
        writer.identity(self.roster_identity);
        writer.identity(self.host_lineage_identity);
        writer.identity(self.finalizer_derivation_sha256);
        self.source_pin.encode_into(writer);
    }

    fn encode_suffix_into(&self, writer: &mut Writer) {
        writer.identity(self.capsule_sha256);
        writer.identity(self.formal_memory_receipt_sha256);
        writer.identity(self.proof_binding_receipt_sha256);
        writer.identity(self.finalized_hsaco_sha256);
        writer.u64(self.finalized_hsaco_length);
    }
}

/// Compiler-execution input and signed current-record result coordinates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1AllKernelsProtectedReceiptCompilerClaimsV1 {
    subject_sha256: [u8; SHA256_BYTES],
    carriage_sha256: [u8; SHA256_BYTES],
    policy_sha256: [u8; SHA256_BYTES],
    issuer_journal_sha256: [u8; SHA256_BYTES],
    compiler_occurrence_sha256: [u8; SHA256_BYTES],
    receipt_sha256: [u8; SHA256_BYTES],
    publication_sha256: [u8; SHA256_BYTES],
    acknowledgment_sha256: [u8; SHA256_BYTES],
    worker_ledger_record_sha256: [u8; SHA256_BYTES],
    sequence: u64,
    prior_rollback_anchor: [u8; SHA256_BYTES],
    current_rollback_anchor: [u8; SHA256_BYTES],
    current_record_verification_sha256: [u8; SHA256_BYTES],
    current_record_attestation_sha256: [u8; SHA256_BYTES],
    protected_policy_verification_sha256: [u8; SHA256_BYTES],
    protected_worker_ledger_verification_sha256: [u8; SHA256_BYTES],
    external_rollback_verification_sha256: [u8; SHA256_BYTES],
}

impl M1AllKernelsProtectedReceiptCompilerClaimsV1 {
    /// Constructs inert compiler current-record claims.
    ///
    /// # Errors
    ///
    /// Returns a typed error for a missing identity, zero sequence, or unadvanced rollback anchor.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subject_sha256: [u8; SHA256_BYTES],
        carriage_sha256: [u8; SHA256_BYTES],
        policy_sha256: [u8; SHA256_BYTES],
        issuer_journal_sha256: [u8; SHA256_BYTES],
        compiler_occurrence_sha256: [u8; SHA256_BYTES],
        receipt_sha256: [u8; SHA256_BYTES],
        publication_sha256: [u8; SHA256_BYTES],
        acknowledgment_sha256: [u8; SHA256_BYTES],
        worker_ledger_record_sha256: [u8; SHA256_BYTES],
        sequence: u64,
        prior_rollback_anchor: [u8; SHA256_BYTES],
        current_rollback_anchor: [u8; SHA256_BYTES],
        current_record_verification_sha256: [u8; SHA256_BYTES],
        current_record_attestation_sha256: [u8; SHA256_BYTES],
        protected_policy_verification_sha256: [u8; SHA256_BYTES],
        protected_worker_ledger_verification_sha256: [u8; SHA256_BYTES],
        external_rollback_verification_sha256: [u8; SHA256_BYTES],
    ) -> Result<Self, M1AllKernelsProtectedReceiptErrorV1> {
        for (identity, field) in [
            (
                subject_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::CompilerExecutionSubject,
            ),
            (
                carriage_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::CompilerExecutionCarriage,
            ),
            (
                policy_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::CompilerExecutionPolicy,
            ),
            (
                issuer_journal_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::CompilerExecutionIssuerJournal,
            ),
            (
                compiler_occurrence_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::CompilerOccurrence,
            ),
            (
                receipt_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::CompilerExecutionReceipt,
            ),
            (
                publication_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::CompilerExecutionPublication,
            ),
            (
                acknowledgment_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::CompilerExecutionAcknowledgment,
            ),
            (
                worker_ledger_record_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::CompilerExecutionWorkerLedgerRecord,
            ),
            (
                current_rollback_anchor,
                M1AllKernelsProtectedReceiptIdentityFieldV1::CompilerExecutionCurrentRollbackAnchor,
            ),
            (
                current_record_verification_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::CurrentRecordVerification,
            ),
            (
                current_record_attestation_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::CurrentRecordAttestation,
            ),
            (
                protected_policy_verification_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::ProtectedCompilerPolicyVerification,
            ),
            (
                protected_worker_ledger_verification_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::ProtectedWorkerLedgerVerification,
            ),
            (
                external_rollback_verification_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::ExternalRollbackVerification,
            ),
        ] {
            require_identity(identity, field, None)?;
        }
        if sequence == 0 {
            return Err(M1AllKernelsProtectedReceiptErrorV1::ZeroCompilerExecutionSequence);
        }
        if prior_rollback_anchor == current_rollback_anchor {
            return Err(
                M1AllKernelsProtectedReceiptErrorV1::UnadvancedCompilerExecutionRollbackAnchor,
            );
        }
        Ok(Self {
            subject_sha256,
            carriage_sha256,
            policy_sha256,
            issuer_journal_sha256,
            compiler_occurrence_sha256,
            receipt_sha256,
            publication_sha256,
            acknowledgment_sha256,
            worker_ledger_record_sha256,
            sequence,
            prior_rollback_anchor,
            current_rollback_anchor,
            current_record_verification_sha256,
            current_record_attestation_sha256,
            protected_policy_verification_sha256,
            protected_worker_ledger_verification_sha256,
            external_rollback_verification_sha256,
        })
    }

    /// Returns the compiler-execution subject digest.
    pub const fn subject_sha256(&self) -> [u8; SHA256_BYTES] {
        self.subject_sha256
    }
    /// Returns the compiler-execution carriage digest.
    pub const fn carriage_sha256(&self) -> [u8; SHA256_BYTES] {
        self.carriage_sha256
    }
    /// Returns the compiler-execution policy digest.
    pub const fn policy_sha256(&self) -> [u8; SHA256_BYTES] {
        self.policy_sha256
    }
    /// Returns the compiler-execution issuer-journal digest.
    pub const fn issuer_journal_sha256(&self) -> [u8; SHA256_BYTES] {
        self.issuer_journal_sha256
    }
    /// Returns the compiler-occurrence digest.
    pub const fn compiler_occurrence_sha256(&self) -> [u8; SHA256_BYTES] {
        self.compiler_occurrence_sha256
    }
    /// Returns the signed compiler-execution receipt digest.
    pub const fn receipt_sha256(&self) -> [u8; SHA256_BYTES] {
        self.receipt_sha256
    }
    /// Returns the compiler-execution publication digest.
    pub const fn publication_sha256(&self) -> [u8; SHA256_BYTES] {
        self.publication_sha256
    }
    /// Returns the Worker acknowledgment digest.
    pub const fn acknowledgment_sha256(&self) -> [u8; SHA256_BYTES] {
        self.acknowledgment_sha256
    }
    /// Returns the protected Worker-ledger record digest.
    pub const fn worker_ledger_record_sha256(&self) -> [u8; SHA256_BYTES] {
        self.worker_ledger_record_sha256
    }
    /// Returns the rollback sequence.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
    /// Returns the prior rollback anchor, which may be the initial zero anchor.
    pub const fn prior_rollback_anchor(&self) -> [u8; SHA256_BYTES] {
        self.prior_rollback_anchor
    }
    /// Returns the current rollback anchor.
    pub const fn current_rollback_anchor(&self) -> [u8; SHA256_BYTES] {
        self.current_rollback_anchor
    }
    /// Returns the signed current-record verification digest.
    pub const fn current_record_verification_sha256(&self) -> [u8; SHA256_BYTES] {
        self.current_record_verification_sha256
    }
    /// Returns the fresh current-record attestation digest.
    pub const fn current_record_attestation_sha256(&self) -> [u8; SHA256_BYTES] {
        self.current_record_attestation_sha256
    }
    /// Returns the protected compiler-policy check digest.
    pub const fn protected_policy_verification_sha256(&self) -> [u8; SHA256_BYTES] {
        self.protected_policy_verification_sha256
    }
    /// Returns the protected Worker-ledger check digest.
    pub const fn protected_worker_ledger_verification_sha256(&self) -> [u8; SHA256_BYTES] {
        self.protected_worker_ledger_verification_sha256
    }
    /// Returns the external rollback check digest.
    pub const fn external_rollback_verification_sha256(&self) -> [u8; SHA256_BYTES] {
        self.external_rollback_verification_sha256
    }

    fn encode_into(&self, writer: &mut Writer) {
        for identity in [
            self.subject_sha256,
            self.carriage_sha256,
            self.policy_sha256,
            self.issuer_journal_sha256,
            self.compiler_occurrence_sha256,
            self.receipt_sha256,
            self.publication_sha256,
            self.acknowledgment_sha256,
            self.worker_ledger_record_sha256,
        ] {
            writer.identity(identity);
        }
        writer.u64(self.sequence);
        writer.identity(self.prior_rollback_anchor);
        writer.identity(self.current_rollback_anchor);
        for identity in [
            self.current_record_verification_sha256,
            self.current_record_attestation_sha256,
            self.protected_policy_verification_sha256,
            self.protected_worker_ledger_verification_sha256,
            self.external_rollback_verification_sha256,
        ] {
            writer.identity(identity);
        }
    }

    fn decode(reader: &mut Reader<'_>) -> Result<Self, M1AllKernelsProtectedReceiptErrorV1> {
        Self::new(
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.u64()?,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
        )
    }
}

/// One signed protected result at one canonical descriptor-table ordinal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1AllKernelsProtectedReceiptEntryV1 {
    ordinal: u16,
    lineage_identity: [u8; SHA256_BYTES],
    marker_binding_identity: [u8; SHA256_BYTES],
    generated_host_contract_identity: [u8; SHA256_BYTES],
    proof_executable_binding_sha256: [u8; SHA256_BYTES],
    rust_type_layout_contract_sha256: [u8; SHA256_BYTES],
    rust_effect_contract_sha256: [u8; SHA256_BYTES],
    safety_properties: WorkerV3SafetyPropertiesV1,
}

impl M1AllKernelsProtectedReceiptEntryV1 {
    /// Constructs one inert result claim. All eight V1 safety bits are mandatory.
    ///
    /// # Errors
    ///
    /// Returns a typed error for a zero theorem coordinate or incomplete safety claim.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ordinal: u16,
        lineage_identity: [u8; SHA256_BYTES],
        marker_binding_identity: [u8; SHA256_BYTES],
        generated_host_contract_identity: [u8; SHA256_BYTES],
        proof_executable_binding_sha256: [u8; SHA256_BYTES],
        rust_type_layout_contract_sha256: [u8; SHA256_BYTES],
        rust_effect_contract_sha256: [u8; SHA256_BYTES],
        safety_properties: WorkerV3SafetyPropertiesV1,
    ) -> Result<Self, M1AllKernelsProtectedReceiptErrorV1> {
        for (identity, field) in [
            (
                lineage_identity,
                M1AllKernelsProtectedReceiptIdentityFieldV1::EntryLineage,
            ),
            (
                marker_binding_identity,
                M1AllKernelsProtectedReceiptIdentityFieldV1::EntryMarkerBinding,
            ),
            (
                generated_host_contract_identity,
                M1AllKernelsProtectedReceiptIdentityFieldV1::EntryGeneratedHostContract,
            ),
            (
                proof_executable_binding_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::EntryProofExecutableBinding,
            ),
            (
                rust_type_layout_contract_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::EntryRustTypeLayoutContract,
            ),
            (
                rust_effect_contract_sha256,
                M1AllKernelsProtectedReceiptIdentityFieldV1::EntryRustEffectContract,
            ),
        ] {
            require_identity(identity, field, Some(ordinal))?;
        }
        if safety_properties != WorkerV3SafetyPropertiesV1::required() {
            return Err(
                M1AllKernelsProtectedReceiptErrorV1::MissingRequiredSafetyProperties {
                    ordinal,
                    actual: safety_properties.bits(),
                },
            );
        }
        Ok(Self {
            ordinal,
            lineage_identity,
            marker_binding_identity,
            generated_host_contract_identity,
            proof_executable_binding_sha256,
            rust_type_layout_contract_sha256,
            rust_effect_contract_sha256,
            safety_properties,
        })
    }

    /// Returns the canonical descriptor-table ordinal.
    pub const fn ordinal(&self) -> u16 {
        self.ordinal
    }
    /// Returns the exact host-lineage identity.
    pub const fn lineage_identity(&self) -> [u8; SHA256_BYTES] {
        self.lineage_identity
    }
    /// Returns the compiler-generated marker binding.
    pub const fn marker_binding_identity(&self) -> [u8; SHA256_BYTES] {
        self.marker_binding_identity
    }
    /// Returns the generated host-contract identity.
    pub const fn generated_host_contract_identity(&self) -> [u8; SHA256_BYTES] {
        self.generated_host_contract_identity
    }
    /// Returns the protected proof-to-executable theorem digest.
    pub const fn proof_executable_binding_sha256(&self) -> [u8; SHA256_BYTES] {
        self.proof_executable_binding_sha256
    }
    /// Returns the protected Rust type-layout theorem digest.
    pub const fn rust_type_layout_contract_sha256(&self) -> [u8; SHA256_BYTES] {
        self.rust_type_layout_contract_sha256
    }
    /// Returns the protected Rust effect theorem digest.
    pub const fn rust_effect_contract_sha256(&self) -> [u8; SHA256_BYTES] {
        self.rust_effect_contract_sha256
    }
    /// Returns the complete required safety-property set.
    pub const fn safety_properties(&self) -> WorkerV3SafetyPropertiesV1 {
        self.safety_properties
    }

    fn encode_into(&self, writer: &mut Writer) {
        writer.u16(self.ordinal);
        writer.u8(self.safety_properties.bits());
        writer.u8(0);
        writer.u32(0);
        for identity in [
            self.lineage_identity,
            self.marker_binding_identity,
            self.generated_host_contract_identity,
            self.proof_executable_binding_sha256,
            self.rust_type_layout_contract_sha256,
            self.rust_effect_contract_sha256,
        ] {
            writer.identity(identity);
        }
    }

    fn decode(reader: &mut Reader<'_>) -> Result<Self, M1AllKernelsProtectedReceiptErrorV1> {
        let ordinal = reader.u16()?;
        let safety_bits = reader.u8()?;
        if reader.u8()? != 0 || reader.u32()? != 0 {
            return Err(M1AllKernelsProtectedReceiptErrorV1::NonzeroReserved);
        }
        let safety_properties = WorkerV3SafetyPropertiesV1::new(safety_bits).ok_or(
            M1AllKernelsProtectedReceiptErrorV1::MissingRequiredSafetyProperties {
                ordinal,
                actual: safety_bits,
            },
        )?;
        Self::new(
            ordinal,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            safety_properties,
        )
    }
}

/// Canonical unsigned receipt prepared for an external protected signer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct M1AllKernelsUnsignedProtectedVerifierReceiptV1 {
    trust_policy_identity: M1AllKernelsProtectedVerifierTrustPolicyIdentityV1,
    request: M1AllKernelsProtectedReceiptRequestClaimsV1,
    compiler: M1AllKernelsProtectedReceiptCompilerClaimsV1,
    verifier_measurement_sha256: [u8; SHA256_BYTES],
    checker_measurement_sha256: [u8; SHA256_BYTES],
    verification_transcript_sha256: [u8; SHA256_BYTES],
    entries: [M1AllKernelsProtectedReceiptEntryV1; M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1],
    canonical_bytes: [u8; UNSIGNED_BYTES],
}

impl M1AllKernelsUnsignedProtectedVerifierReceiptV1 {
    /// Constructs one canonical inert claim set for an external signer.
    ///
    /// # Errors
    ///
    /// Returns a typed error for a zero identity, aliased measurements, noncanonical entry order,
    /// duplicate entry identity, or incomplete safety claim.
    pub fn new(
        trust_policy_identity: M1AllKernelsProtectedVerifierTrustPolicyIdentityV1,
        request: M1AllKernelsProtectedReceiptRequestClaimsV1,
        compiler: M1AllKernelsProtectedReceiptCompilerClaimsV1,
        verifier_measurement_sha256: [u8; SHA256_BYTES],
        checker_measurement_sha256: [u8; SHA256_BYTES],
        verification_transcript_sha256: [u8; SHA256_BYTES],
        entries: [M1AllKernelsProtectedReceiptEntryV1; M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1],
    ) -> Result<Self, M1AllKernelsProtectedReceiptErrorV1> {
        require_identity(
            *trust_policy_identity.as_bytes(),
            M1AllKernelsProtectedReceiptIdentityFieldV1::TrustPolicy,
            None,
        )?;
        validate_measurements(verifier_measurement_sha256, checker_measurement_sha256)?;
        require_identity(
            verification_transcript_sha256,
            M1AllKernelsProtectedReceiptIdentityFieldV1::VerificationTranscript,
            None,
        )?;
        validate_entries(&entries)?;
        let mut receipt = Self {
            trust_policy_identity,
            request,
            compiler,
            verifier_measurement_sha256,
            checker_measurement_sha256,
            verification_transcript_sha256,
            entries,
            canonical_bytes: [0; UNSIGNED_BYTES],
        };
        receipt.canonical_bytes = receipt.encode();
        Ok(receipt)
    }

    /// Returns the exact canonical unsigned bytes.
    pub const fn encode_canonical(&self) -> &[u8; UNSIGNED_BYTES] {
        &self.canonical_bytes
    }

    /// Returns the exact domain-separated signing message.
    #[must_use]
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(SIGNING_DOMAIN.len() + UNSIGNED_BYTES);
        bytes.extend_from_slice(SIGNING_DOMAIN);
        bytes.extend_from_slice(&self.canonical_bytes);
        bytes
    }

    /// Attaches externally produced Ed25519 bytes without authenticating them.
    #[must_use]
    pub fn attach_signature(
        self,
        signature: [u8; SIGNATURE_BYTES],
    ) -> M1AllKernelsProtectedVerifierReceiptV1 {
        M1AllKernelsProtectedVerifierReceiptV1::from_unsigned(self, signature)
    }

    fn encode(&self) -> [u8; UNSIGNED_BYTES] {
        let mut writer = Writer::new();
        writer.bytes(&MAGIC);
        writer.u16(VERSION);
        writer.u16(12);
        writer.u16(32);
        writer.u16(200);
        writer.u32(3_552);
        writer.u32(0);
        writer.u64(0);
        writer.identity(*self.trust_policy_identity.as_bytes());
        self.request.encode_prefix_into(&mut writer);
        self.compiler.encode_into(&mut writer);
        self.request.encode_suffix_into(&mut writer);
        let mut target = [0_u8; TARGET_BYTES];
        target[..TARGET.len()].copy_from_slice(TARGET);
        writer.bytes(&target);
        writer.u16(CODE_OBJECT_VERSION);
        writer.u16(0);
        writer.u32(0);
        writer.identity(self.verifier_measurement_sha256);
        writer.identity(self.checker_measurement_sha256);
        writer.identity(self.verification_transcript_sha256);
        for entry in &self.entries {
            entry.encode_into(&mut writer);
        }
        writer.finish()
    }
}

/// Strictly decoded canonical signed receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct M1AllKernelsProtectedVerifierReceiptV1 {
    unsigned: M1AllKernelsUnsignedProtectedVerifierReceiptV1,
    signature: [u8; SIGNATURE_BYTES],
    identity: M1AllKernelsProtectedVerifierReceiptIdentityV1,
    canonical_bytes: [u8; M1_ALL_KERNELS_PROTECTED_RECEIPT_BYTES_V1],
}

impl M1AllKernelsProtectedVerifierReceiptV1 {
    /// Strictly decodes one complete canonical V1 receipt.
    ///
    /// # Errors
    ///
    /// Returns a typed error for any noncanonical header, field, entry, target, or length.
    #[allow(clippy::too_many_lines)]
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self, M1AllKernelsProtectedReceiptErrorV1> {
        if bytes.len() != M1_ALL_KERNELS_PROTECTED_RECEIPT_BYTES_V1 {
            return Err(M1AllKernelsProtectedReceiptErrorV1::InvalidLength {
                expected: M1_ALL_KERNELS_PROTECTED_RECEIPT_BYTES_V1,
                actual: bytes.len(),
            });
        }
        let mut reader = Reader::new(&bytes[..UNSIGNED_BYTES]);
        if reader.fixed::<8>()? != MAGIC {
            return Err(M1AllKernelsProtectedReceiptErrorV1::InvalidMagic);
        }
        let version = reader.u16()?;
        if version != VERSION {
            return Err(M1AllKernelsProtectedReceiptErrorV1::UnsupportedVersion {
                actual: version,
            });
        }
        let entry_count = reader.u16()?;
        if usize::from(entry_count) != M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1 {
            return Err(M1AllKernelsProtectedReceiptErrorV1::InvalidEntryCount {
                actual: entry_count,
            });
        }
        let header_length = reader.u16()?;
        if usize::from(header_length) != HEADER_BYTES {
            return Err(M1AllKernelsProtectedReceiptErrorV1::InvalidHeaderLength {
                actual: header_length,
            });
        }
        let entry_length = reader.u16()?;
        if usize::from(entry_length) != ENTRY_BYTES {
            return Err(M1AllKernelsProtectedReceiptErrorV1::InvalidEntryLength {
                actual: entry_length,
            });
        }
        let declared_length = reader.u32()?;
        if declared_length != 3_552 {
            return Err(M1AllKernelsProtectedReceiptErrorV1::InvalidDeclaredLength {
                actual: declared_length,
            });
        }
        if reader.u32()? != 0 || reader.u64()? != 0 {
            return Err(M1AllKernelsProtectedReceiptErrorV1::NonzeroReserved);
        }
        let trust_policy_identity =
            M1AllKernelsProtectedVerifierTrustPolicyIdentityV1(reader.identity()?);
        let challenge_identity = reader.identity()?;
        let roster_identity = reader.identity()?;
        let host_lineage_identity = reader.identity()?;
        let finalizer_derivation_sha256 = reader.identity()?;
        let source_pin = M1AllKernelsProtectedReceiptSourcePinV1::decode(&mut reader)?;
        let compiler = M1AllKernelsProtectedReceiptCompilerClaimsV1::decode(&mut reader)?;
        let request = M1AllKernelsProtectedReceiptRequestClaimsV1::new(
            challenge_identity,
            roster_identity,
            host_lineage_identity,
            finalizer_derivation_sha256,
            source_pin,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
            reader.u64()?,
        )?;
        let mut expected_target = [0_u8; TARGET_BYTES];
        expected_target[..TARGET.len()].copy_from_slice(TARGET);
        if reader.fixed::<TARGET_BYTES>()? != expected_target {
            return Err(M1AllKernelsProtectedReceiptErrorV1::InvalidTarget);
        }
        let code_object_version = reader.u16()?;
        if code_object_version != CODE_OBJECT_VERSION {
            return Err(
                M1AllKernelsProtectedReceiptErrorV1::InvalidCodeObjectVersion {
                    actual: code_object_version,
                },
            );
        }
        if reader.u16()? != 0 || reader.u32()? != 0 {
            return Err(M1AllKernelsProtectedReceiptErrorV1::NonzeroReserved);
        }
        let verifier_measurement_sha256 = reader.identity()?;
        let checker_measurement_sha256 = reader.identity()?;
        let verification_transcript_sha256 = reader.identity()?;
        let mut entries = Vec::with_capacity(M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1);
        for _ in 0..M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1 {
            entries.push(M1AllKernelsProtectedReceiptEntryV1::decode(&mut reader)?);
        }
        if !reader.is_finished() {
            return Err(M1AllKernelsProtectedReceiptErrorV1::InvalidLength {
                expected: UNSIGNED_BYTES,
                actual: reader.position(),
            });
        }
        let entries: [M1AllKernelsProtectedReceiptEntryV1; M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1] =
            entries.try_into().map_err(|entries: Vec<_>| {
                M1AllKernelsProtectedReceiptErrorV1::InvalidEntryCount {
                    actual: u16::try_from(entries.len()).unwrap_or(u16::MAX),
                }
            })?;
        let unsigned = M1AllKernelsUnsignedProtectedVerifierReceiptV1::new(
            trust_policy_identity,
            request,
            compiler,
            verifier_measurement_sha256,
            checker_measurement_sha256,
            verification_transcript_sha256,
            entries,
        )?;
        if unsigned.encode_canonical() != &bytes[..UNSIGNED_BYTES] {
            return Err(M1AllKernelsProtectedReceiptErrorV1::NonzeroReserved);
        }
        let mut signature = [0_u8; SIGNATURE_BYTES];
        signature.copy_from_slice(&bytes[UNSIGNED_BYTES..]);
        Ok(Self::from_unsigned(unsigned, signature))
    }

    /// Returns the exact canonical signed bytes.
    pub const fn encode_canonical(&self) -> &[u8; M1_ALL_KERNELS_PROTECTED_RECEIPT_BYTES_V1] {
        &self.canonical_bytes
    }

    /// Returns the domain-separated identity of the complete signed bytes.
    pub const fn identity(&self) -> M1AllKernelsProtectedVerifierReceiptIdentityV1 {
        self.identity
    }

    /// Returns the caller-provisioned trust-policy identity named by the receipt.
    pub const fn trust_policy_identity(
        &self,
    ) -> M1AllKernelsProtectedVerifierTrustPolicyIdentityV1 {
        self.unsigned.trust_policy_identity
    }

    /// Returns the protected verifier measurement claim.
    pub const fn verifier_measurement_sha256(&self) -> [u8; SHA256_BYTES] {
        self.unsigned.verifier_measurement_sha256
    }

    /// Returns the independent checker measurement claim.
    pub const fn checker_measurement_sha256(&self) -> [u8; SHA256_BYTES] {
        self.unsigned.checker_measurement_sha256
    }

    /// Returns the complete protected transcript claim.
    pub const fn verification_transcript_sha256(&self) -> [u8; SHA256_BYTES] {
        self.unsigned.verification_transcript_sha256
    }

    /// Returns the host-known request claims.
    pub const fn request_claims(&self) -> &M1AllKernelsProtectedReceiptRequestClaimsV1 {
        &self.unsigned.request
    }

    /// Returns the compiler current-record claims.
    pub const fn compiler_claims(&self) -> &M1AllKernelsProtectedReceiptCompilerClaimsV1 {
        &self.unsigned.compiler
    }

    /// Returns all 12 ordered entry results.
    pub const fn entries(
        &self,
    ) -> &[M1AllKernelsProtectedReceiptEntryV1; M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1] {
        &self.unsigned.entries
    }

    fn from_unsigned(
        unsigned: M1AllKernelsUnsignedProtectedVerifierReceiptV1,
        signature: [u8; SIGNATURE_BYTES],
    ) -> Self {
        let mut canonical_bytes = [0_u8; M1_ALL_KERNELS_PROTECTED_RECEIPT_BYTES_V1];
        canonical_bytes[..UNSIGNED_BYTES].copy_from_slice(unsigned.encode_canonical());
        canonical_bytes[UNSIGNED_BYTES..].copy_from_slice(&signature);
        let identity = M1AllKernelsProtectedVerifierReceiptIdentityV1(hash_parts(&[
            RECEIPT_IDENTITY_DOMAIN,
            &canonical_bytes,
        ]));
        Self {
            unsigned,
            signature,
            identity,
            canonical_bytes,
        }
    }

    fn signing_bytes(&self) -> Vec<u8> {
        self.unsigned.signing_bytes()
    }
}

/// Caller-provisioned Ed25519 trust policy for one verifier/checker closure pair.
///
/// This type has no default and contains no embedded key or measurement. Its
/// identity covers the exact public key, both distinct measurements, the
/// receipt version, roster cardinality, target, code-object version, and the
/// complete required safety-property mask.
#[derive(Debug)]
pub struct M1AllKernelsProtectedVerifierTrustPolicyV1 {
    verifying_key: VerifyingKey,
    verifier_measurement_sha256: [u8; SHA256_BYTES],
    checker_measurement_sha256: [u8; SHA256_BYTES],
    identity: M1AllKernelsProtectedVerifierTrustPolicyIdentityV1,
}

impl M1AllKernelsProtectedVerifierTrustPolicyV1 {
    /// Admits one exact externally provisioned key and measurement pair.
    ///
    /// # Errors
    ///
    /// Returns a typed error for an invalid or weak key, zero measurement, or aliased
    /// verifier/checker measurements.
    pub fn new(
        verifying_key: [u8; 32],
        verifier_measurement_sha256: [u8; SHA256_BYTES],
        checker_measurement_sha256: [u8; SHA256_BYTES],
    ) -> Result<Self, M1AllKernelsProtectedReceiptErrorV1> {
        validate_measurements(verifier_measurement_sha256, checker_measurement_sha256)?;
        let verifying_key = VerifyingKey::from_bytes(&verifying_key)
            .map_err(|_| M1AllKernelsProtectedReceiptErrorV1::InvalidVerifyingKey)?;
        if verifying_key.is_weak() {
            return Err(M1AllKernelsProtectedReceiptErrorV1::WeakVerifyingKey);
        }
        let identity = M1AllKernelsProtectedVerifierTrustPolicyIdentityV1(hash_parts(&[
            POLICY_IDENTITY_DOMAIN,
            &VERSION.to_le_bytes(),
            &12_u16.to_le_bytes(),
            verifying_key.as_bytes(),
            &verifier_measurement_sha256,
            &checker_measurement_sha256,
            TARGET,
            &CODE_OBJECT_VERSION.to_le_bytes(),
            &[WorkerV3SafetyPropertiesV1::required().bits()],
        ]));
        Ok(Self {
            verifying_key,
            verifier_measurement_sha256,
            checker_measurement_sha256,
            identity,
        })
    }

    /// Returns the exact domain-separated policy identity.
    pub const fn identity(&self) -> M1AllKernelsProtectedVerifierTrustPolicyIdentityV1 {
        self.identity
    }

    /// Returns the exact externally provisioned public-key bytes.
    pub fn verifying_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Returns the required protected-verifier measurement.
    pub const fn verifier_measurement_sha256(&self) -> [u8; SHA256_BYTES] {
        self.verifier_measurement_sha256
    }

    /// Returns the required independent-checker measurement.
    pub const fn checker_measurement_sha256(&self) -> [u8; SHA256_BYTES] {
        self.checker_measurement_sha256
    }

    /// Strictly decodes and authenticates one canonical receipt.
    ///
    /// # Errors
    ///
    /// Returns a typed canonical-decoding, policy, measurement, or signature error.
    pub fn authenticate_canonical(
        &self,
        bytes: &[u8],
    ) -> Result<
        M1AllKernelsAuthenticatedProtectedVerifierReceiptV1,
        M1AllKernelsProtectedReceiptErrorV1,
    > {
        let receipt = M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(bytes)?;
        if receipt.trust_policy_identity() != self.identity {
            return Err(M1AllKernelsProtectedReceiptErrorV1::TrustPolicyMismatch);
        }
        if receipt.verifier_measurement_sha256() != self.verifier_measurement_sha256 {
            return Err(M1AllKernelsProtectedReceiptErrorV1::VerifierMeasurementMismatch);
        }
        if receipt.checker_measurement_sha256() != self.checker_measurement_sha256 {
            return Err(M1AllKernelsProtectedReceiptErrorV1::CheckerMeasurementMismatch);
        }
        let signature = Signature::from_bytes(&receipt.signature);
        self.verifying_key
            .verify_strict(&receipt.signing_bytes(), &signature)
            .map_err(|_| M1AllKernelsProtectedReceiptErrorV1::SignatureRejected)?;
        Ok(M1AllKernelsAuthenticatedProtectedVerifierReceiptV1 {
            receipt,
            policy_identity: self.identity,
        })
    }
}

/// Signature-authenticated aggregate protected-verifier receipt.
#[derive(Debug)]
pub struct M1AllKernelsAuthenticatedProtectedVerifierReceiptV1 {
    receipt: M1AllKernelsProtectedVerifierReceiptV1,
    policy_identity: M1AllKernelsProtectedVerifierTrustPolicyIdentityV1,
}

impl M1AllKernelsAuthenticatedProtectedVerifierReceiptV1 {
    /// Returns the authenticated canonical receipt.
    pub const fn receipt(&self) -> &M1AllKernelsProtectedVerifierReceiptV1 {
        &self.receipt
    }

    /// Returns the independently supplied policy identity used for authentication.
    pub const fn policy_identity(&self) -> M1AllKernelsProtectedVerifierTrustPolicyIdentityV1 {
        self.policy_identity
    }

    /// Signature authentication alone does not grant fe2o3 verifier authority.
    pub const fn grants_verifier_authority(&self) -> bool {
        false
    }
    /// Signature authentication alone does not grant GPU load authority.
    pub const fn grants_load_authority(&self) -> bool {
        false
    }
    /// Signature authentication alone does not grant GPU launch authority.
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

fn validate_measurements(
    verifier: [u8; SHA256_BYTES],
    checker: [u8; SHA256_BYTES],
) -> Result<(), M1AllKernelsProtectedReceiptErrorV1> {
    require_identity(
        verifier,
        M1AllKernelsProtectedReceiptIdentityFieldV1::VerifierMeasurement,
        None,
    )?;
    require_identity(
        checker,
        M1AllKernelsProtectedReceiptIdentityFieldV1::CheckerMeasurement,
        None,
    )?;
    if verifier == checker {
        return Err(M1AllKernelsProtectedReceiptErrorV1::AliasedVerifierAndCheckerMeasurements);
    }
    Ok(())
}

fn validate_entries(
    entries: &[M1AllKernelsProtectedReceiptEntryV1; M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1],
) -> Result<(), M1AllKernelsProtectedReceiptErrorV1> {
    for (index, entry) in entries.iter().enumerate() {
        let expected = u16::try_from(index).map_err(|_| {
            M1AllKernelsProtectedReceiptErrorV1::InvalidEntryCount { actual: u16::MAX }
        })?;
        if entry.ordinal != expected {
            return Err(M1AllKernelsProtectedReceiptErrorV1::InvalidEntryOrdinal {
                expected,
                actual: entry.ordinal,
            });
        }
        for prior in &entries[..index] {
            if prior.lineage_identity == entry.lineage_identity {
                return Err(
                    M1AllKernelsProtectedReceiptErrorV1::DuplicateEntryIdentity {
                        field: M1AllKernelsProtectedReceiptDuplicateFieldV1::Lineage,
                        ordinal: entry.ordinal,
                    },
                );
            }
            if prior.marker_binding_identity == entry.marker_binding_identity {
                return Err(
                    M1AllKernelsProtectedReceiptErrorV1::DuplicateEntryIdentity {
                        field: M1AllKernelsProtectedReceiptDuplicateFieldV1::MarkerBinding,
                        ordinal: entry.ordinal,
                    },
                );
            }
        }
    }
    Ok(())
}

fn require_identity(
    identity: [u8; SHA256_BYTES],
    field: M1AllKernelsProtectedReceiptIdentityFieldV1,
    ordinal: Option<u16>,
) -> Result<(), M1AllKernelsProtectedReceiptErrorV1> {
    if identity == [0; SHA256_BYTES] {
        return Err(M1AllKernelsProtectedReceiptErrorV1::ZeroIdentity { field, ordinal });
    }
    Ok(())
}

fn require_length(
    length: u64,
    field: M1AllKernelsProtectedReceiptLengthFieldV1,
) -> Result<(), M1AllKernelsProtectedReceiptErrorV1> {
    if length == 0 {
        return Err(M1AllKernelsProtectedReceiptErrorV1::ZeroLength { field });
    }
    Ok(())
}

fn hash_parts(parts: &[&[u8]]) -> [u8; SHA256_BYTES] {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update(part);
    }
    digest.finalize().into()
}

struct Writer {
    bytes: [u8; UNSIGNED_BYTES],
    position: usize,
}

impl Writer {
    const fn new() -> Self {
        Self {
            bytes: [0; UNSIGNED_BYTES],
            position: 0,
        }
    }

    fn bytes(&mut self, value: &[u8]) {
        let end = self.position + value.len();
        self.bytes[self.position..end].copy_from_slice(value);
        self.position = end;
    }

    fn u8(&mut self, value: u8) {
        self.bytes(&[value]);
    }
    fn u16(&mut self, value: u16) {
        self.bytes(&value.to_le_bytes());
    }
    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }
    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }
    fn identity(&mut self, value: [u8; SHA256_BYTES]) {
        self.bytes(&value);
    }

    fn finish(self) -> [u8; UNSIGNED_BYTES] {
        assert_eq!(
            self.position, UNSIGNED_BYTES,
            "fixed receipt encoder length"
        );
        self.bytes
    }
}

struct Reader<'bytes> {
    bytes: &'bytes [u8],
    position: usize,
}

impl<'bytes> Reader<'bytes> {
    const fn new(bytes: &'bytes [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], M1AllKernelsProtectedReceiptErrorV1> {
        let end = self.position.checked_add(N).ok_or(
            M1AllKernelsProtectedReceiptErrorV1::InvalidLength {
                expected: UNSIGNED_BYTES,
                actual: self.bytes.len(),
            },
        )?;
        let slice = self.bytes.get(self.position..end).ok_or(
            M1AllKernelsProtectedReceiptErrorV1::InvalidLength {
                expected: UNSIGNED_BYTES,
                actual: self.bytes.len(),
            },
        )?;
        self.position = end;
        let mut value = [0_u8; N];
        value.copy_from_slice(slice);
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, M1AllKernelsProtectedReceiptErrorV1> {
        Ok(self.fixed::<1>()?[0])
    }
    fn u16(&mut self) -> Result<u16, M1AllKernelsProtectedReceiptErrorV1> {
        Ok(u16::from_le_bytes(self.fixed()?))
    }
    fn u32(&mut self) -> Result<u32, M1AllKernelsProtectedReceiptErrorV1> {
        Ok(u32::from_le_bytes(self.fixed()?))
    }
    fn u64(&mut self) -> Result<u64, M1AllKernelsProtectedReceiptErrorV1> {
        Ok(u64::from_le_bytes(self.fixed()?))
    }
    fn identity(&mut self) -> Result<[u8; SHA256_BYTES], M1AllKernelsProtectedReceiptErrorV1> {
        self.fixed()
    }
    const fn is_finished(&self) -> bool {
        self.position == self.bytes.len()
    }
    const fn position(&self) -> usize {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn identity(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    fn policy() -> (SigningKey, M1AllKernelsProtectedVerifierTrustPolicyV1) {
        let signing = SigningKey::from_bytes(&[0x91; 32]);
        let policy = M1AllKernelsProtectedVerifierTrustPolicyV1::new(
            signing.verifying_key().to_bytes(),
            identity(0xa1),
            identity(0xa2),
        )
        .unwrap();
        (signing, policy)
    }

    fn source_pin() -> M1AllKernelsProtectedReceiptSourcePinV1 {
        M1AllKernelsProtectedReceiptSourcePinV1::new(
            identity(5),
            5_001,
            identity(6),
            6_001,
            identity(7),
            7_001,
        )
        .unwrap()
    }

    fn request_claims() -> M1AllKernelsProtectedReceiptRequestClaimsV1 {
        M1AllKernelsProtectedReceiptRequestClaimsV1::new(
            identity(1),
            identity(2),
            identity(3),
            identity(4),
            source_pin(),
            identity(8),
            identity(9),
            identity(10),
            identity(11),
            11_001,
        )
        .unwrap()
    }

    fn compiler_claims() -> M1AllKernelsProtectedReceiptCompilerClaimsV1 {
        M1AllKernelsProtectedReceiptCompilerClaimsV1::new(
            identity(20),
            identity(21),
            identity(22),
            identity(23),
            identity(24),
            identity(25),
            identity(26),
            identity(27),
            identity(28),
            29,
            [0; 32],
            identity(30),
            identity(31),
            identity(32),
            identity(33),
            identity(34),
            identity(35),
        )
        .unwrap()
    }

    fn entries() -> [M1AllKernelsProtectedReceiptEntryV1; M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1] {
        core::array::from_fn(|index| {
            let seed = u8::try_from(index).unwrap();
            M1AllKernelsProtectedReceiptEntryV1::new(
                u16::from(seed),
                identity(50 + seed),
                identity(70 + seed),
                identity(90 + (seed % 3)),
                identity(110 + seed),
                identity(130 + seed),
                identity(150 + seed),
                WorkerV3SafetyPropertiesV1::required(),
            )
            .unwrap()
        })
    }

    fn signed() -> (
        M1AllKernelsProtectedVerifierTrustPolicyV1,
        M1AllKernelsProtectedVerifierReceiptV1,
    ) {
        let (signing, policy) = policy();
        let unsigned = M1AllKernelsUnsignedProtectedVerifierReceiptV1::new(
            policy.identity(),
            request_claims(),
            compiler_claims(),
            identity(0xa1),
            identity(0xa2),
            identity(0xa3),
            entries(),
        )
        .unwrap();
        let signature = signing.sign(&unsigned.signing_bytes()).to_bytes();
        (policy, unsigned.attach_signature(signature))
    }

    #[test]
    fn exact_receipt_round_trips_and_authenticates_under_supplied_policy() {
        let (policy, receipt) = signed();
        assert_eq!(
            receipt.encode_canonical().len(),
            M1_ALL_KERNELS_PROTECTED_RECEIPT_BYTES_V1
        );
        assert_eq!(receipt.encode_canonical()[..8], MAGIC);
        let decoded =
            M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(receipt.encode_canonical())
                .unwrap();
        assert_eq!(decoded, receipt);
        assert!(
            receipt
                .identity()
                .matches_canonical_bytes(receipt.encode_canonical())
        );
        let authenticated = policy
            .authenticate_canonical(receipt.encode_canonical())
            .unwrap();
        assert_eq!(authenticated.receipt().identity(), receipt.identity());
        assert_eq!(authenticated.policy_identity(), policy.identity());
        assert!(!authenticated.grants_verifier_authority());
        assert!(!authenticated.grants_load_authority());
        assert!(!authenticated.grants_launch_authority());
        assert_eq!(authenticated.receipt().entries().len(), 12);
        assert!(
            authenticated.receipt().entries().iter().all(|entry| {
                entry.safety_properties() == WorkerV3SafetyPropertiesV1::required()
            })
        );
        assert_eq!(
            *policy.identity().as_bytes(),
            [
                0xa6, 0x9e, 0xed, 0x7b, 0x2d, 0x23, 0xba, 0x56, 0xaf, 0x1b, 0x24, 0xaf, 0xc9, 0x43,
                0x0d, 0x27, 0x88, 0x74, 0xde, 0x27, 0x59, 0xfd, 0x58, 0x61, 0xec, 0xb4, 0x28, 0xe8,
                0xb3, 0x59, 0xaf, 0x36,
            ],
        );
        assert_eq!(
            *receipt.identity().as_bytes(),
            [
                0xcb, 0x9d, 0x66, 0x2a, 0x0b, 0x32, 0x25, 0x6f, 0x65, 0xee, 0x6c, 0xdd, 0x85, 0x98,
                0xfa, 0x56, 0x80, 0x0a, 0x11, 0xb0, 0xca, 0x41, 0x3d, 0xe8, 0x9e, 0x3c, 0x59, 0x2c,
                0x20, 0x9e, 0xaf, 0x6d,
            ],
        );
    }

    #[test]
    fn decoded_accessors_preserve_every_signed_coordinate() {
        let (_, receipt) = signed();
        let decoded =
            M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(receipt.encode_canonical())
                .unwrap();
        let request = decoded.request_claims();
        assert_eq!(request.challenge_identity(), identity(1));
        assert_eq!(request.roster_identity(), identity(2));
        assert_eq!(request.host_lineage_identity(), identity(3));
        assert_eq!(request.finalizer_derivation_sha256(), identity(4));
        let source = request.source_pin();
        assert_eq!(source.compiler_module_sha256(), identity(5));
        assert_eq!(source.compiler_module_length(), 5_001);
        assert_eq!(source.compiler_handoff_sha256(), identity(6));
        assert_eq!(source.compiler_handoff_length(), 6_001);
        assert_eq!(source.symbol_manifest_sha256(), identity(7));
        assert_eq!(source.symbol_manifest_length(), 7_001);
        assert_eq!(request.capsule_sha256(), identity(8));
        assert_eq!(request.formal_memory_receipt_sha256(), identity(9));
        assert_eq!(request.proof_binding_receipt_sha256(), identity(10));
        assert_eq!(request.finalized_hsaco_sha256(), identity(11));
        assert_eq!(request.finalized_hsaco_length(), 11_001);

        let compiler = decoded.compiler_claims();
        assert_eq!(compiler.subject_sha256(), identity(20));
        assert_eq!(compiler.carriage_sha256(), identity(21));
        assert_eq!(compiler.policy_sha256(), identity(22));
        assert_eq!(compiler.issuer_journal_sha256(), identity(23));
        assert_eq!(compiler.compiler_occurrence_sha256(), identity(24));
        assert_eq!(compiler.receipt_sha256(), identity(25));
        assert_eq!(compiler.publication_sha256(), identity(26));
        assert_eq!(compiler.acknowledgment_sha256(), identity(27));
        assert_eq!(compiler.worker_ledger_record_sha256(), identity(28));
        assert_eq!(compiler.sequence(), 29);
        assert_eq!(compiler.prior_rollback_anchor(), [0; 32]);
        assert_eq!(compiler.current_rollback_anchor(), identity(30));
        assert_eq!(compiler.current_record_verification_sha256(), identity(31));
        assert_eq!(compiler.current_record_attestation_sha256(), identity(32));
        assert_eq!(
            compiler.protected_policy_verification_sha256(),
            identity(33)
        );
        assert_eq!(
            compiler.protected_worker_ledger_verification_sha256(),
            identity(34)
        );
        assert_eq!(
            compiler.external_rollback_verification_sha256(),
            identity(35)
        );
        assert_eq!(decoded.verifier_measurement_sha256(), identity(0xa1));
        assert_eq!(decoded.checker_measurement_sha256(), identity(0xa2));
        assert_eq!(decoded.verification_transcript_sha256(), identity(0xa3));

        for (index, entry) in decoded.entries().iter().enumerate() {
            let seed = u8::try_from(index).unwrap();
            assert_eq!(entry.ordinal(), u16::from(seed));
            assert_eq!(entry.lineage_identity(), identity(50 + seed));
            assert_eq!(entry.marker_binding_identity(), identity(70 + seed));
            assert_eq!(
                entry.generated_host_contract_identity(),
                identity(90 + (seed % 3))
            );
            assert_eq!(
                entry.proof_executable_binding_sha256(),
                identity(110 + seed)
            );
            assert_eq!(
                entry.rust_type_layout_contract_sha256(),
                identity(130 + seed)
            );
            assert_eq!(entry.rust_effect_contract_sha256(), identity(150 + seed));
            assert_eq!(
                entry.safety_properties(),
                WorkerV3SafetyPropertiesV1::required()
            );
        }
    }

    #[test]
    fn every_single_byte_mutation_is_rejected_by_structure_or_signature() {
        let (policy, receipt) = signed();
        for offset in 0..receipt.encode_canonical().len() {
            let mut hostile = *receipt.encode_canonical();
            hostile[offset] ^= 0x80;
            assert!(
                policy.authenticate_canonical(&hostile).is_err(),
                "mutation at byte {offset} authenticated",
            );
        }
    }

    #[test]
    fn every_truncation_and_trailing_byte_is_rejected() {
        let (_, receipt) = signed();
        for length in 0..receipt.encode_canonical().len() {
            assert!(matches!(
                M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(
                    &receipt.encode_canonical()[..length]
                ),
                Err(M1AllKernelsProtectedReceiptErrorV1::InvalidLength { .. })
            ));
        }
        let mut trailing = receipt.encode_canonical().to_vec();
        trailing.push(0);
        assert!(matches!(
            M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(&trailing),
            Err(M1AllKernelsProtectedReceiptErrorV1::InvalidLength { .. })
        ));
    }

    #[test]
    fn substituted_policy_key_measurements_and_signature_fail_closed() {
        let (policy, receipt) = signed();
        let other_signing = SigningKey::from_bytes(&[0x92; 32]);
        let other_policy = M1AllKernelsProtectedVerifierTrustPolicyV1::new(
            other_signing.verifying_key().to_bytes(),
            identity(0xa1),
            identity(0xa2),
        )
        .unwrap();
        assert_eq!(
            other_policy
                .authenticate_canonical(receipt.encode_canonical())
                .unwrap_err(),
            M1AllKernelsProtectedReceiptErrorV1::TrustPolicyMismatch,
        );
        let changed_verifier = M1AllKernelsProtectedVerifierTrustPolicyV1::new(
            policy.verifying_key_bytes(),
            identity(0xb1),
            identity(0xa2),
        )
        .unwrap();
        assert_eq!(
            changed_verifier
                .authenticate_canonical(receipt.encode_canonical())
                .unwrap_err(),
            M1AllKernelsProtectedReceiptErrorV1::TrustPolicyMismatch,
        );
        let changed_checker = M1AllKernelsProtectedVerifierTrustPolicyV1::new(
            policy.verifying_key_bytes(),
            identity(0xa1),
            identity(0xb2),
        )
        .unwrap();
        assert_eq!(
            changed_checker
                .authenticate_canonical(receipt.encode_canonical())
                .unwrap_err(),
            M1AllKernelsProtectedReceiptErrorV1::TrustPolicyMismatch,
        );
        let mut signature = *receipt.encode_canonical();
        signature[UNSIGNED_BYTES] ^= 1;
        assert_eq!(
            policy.authenticate_canonical(&signature).unwrap_err(),
            M1AllKernelsProtectedReceiptErrorV1::SignatureRejected,
        );

        let unsigned_without_domain = M1AllKernelsUnsignedProtectedVerifierReceiptV1::new(
            policy.identity(),
            request_claims(),
            compiler_claims(),
            identity(0xa1),
            identity(0xa2),
            identity(0xa3),
            entries(),
        )
        .unwrap();
        let wrong_signature = SigningKey::from_bytes(&[0x91; 32])
            .sign(unsigned_without_domain.encode_canonical())
            .to_bytes();
        let wrong_domain = unsigned_without_domain.attach_signature(wrong_signature);
        assert_eq!(
            policy
                .authenticate_canonical(wrong_domain.encode_canonical())
                .unwrap_err(),
            M1AllKernelsProtectedReceiptErrorV1::SignatureRejected,
        );

        for (verifier, checker, expected) in [
            (
                identity(0xb1),
                identity(0xa2),
                M1AllKernelsProtectedReceiptErrorV1::VerifierMeasurementMismatch,
            ),
            (
                identity(0xa1),
                identity(0xb2),
                M1AllKernelsProtectedReceiptErrorV1::CheckerMeasurementMismatch,
            ),
        ] {
            let unsigned = M1AllKernelsUnsignedProtectedVerifierReceiptV1::new(
                policy.identity(),
                request_claims(),
                compiler_claims(),
                verifier,
                checker,
                identity(0xa3),
                entries(),
            )
            .unwrap();
            let signing = SigningKey::from_bytes(&[0x91; 32]);
            let signature = signing.sign(&unsigned.signing_bytes()).to_bytes();
            let receipt = unsigned.attach_signature(signature);
            assert_eq!(
                policy
                    .authenticate_canonical(receipt.encode_canonical())
                    .unwrap_err(),
                expected,
            );
        }
    }

    #[test]
    fn weak_zero_and_aliased_policy_inputs_are_rejected() {
        assert!(matches!(
            M1AllKernelsProtectedVerifierTrustPolicyV1::new([0; 32], identity(1), identity(2)),
            Err(M1AllKernelsProtectedReceiptErrorV1::WeakVerifyingKey
                | M1AllKernelsProtectedReceiptErrorV1::InvalidVerifyingKey)
        ));
        let key = SigningKey::from_bytes(&[7; 32]).verifying_key().to_bytes();
        assert_eq!(
            M1AllKernelsProtectedVerifierTrustPolicyV1::new(key, [0; 32], identity(2)).unwrap_err(),
            M1AllKernelsProtectedReceiptErrorV1::ZeroIdentity {
                field: M1AllKernelsProtectedReceiptIdentityFieldV1::VerifierMeasurement,
                ordinal: None,
            },
        );
        assert_eq!(
            M1AllKernelsProtectedVerifierTrustPolicyV1::new(key, identity(2), identity(2))
                .unwrap_err(),
            M1AllKernelsProtectedReceiptErrorV1::AliasedVerifierAndCheckerMeasurements,
        );
    }

    #[test]
    fn reordered_duplicate_incomplete_and_zero_entry_claims_are_rejected() {
        let (_, policy) = policy();
        let make = |entries| {
            M1AllKernelsUnsignedProtectedVerifierReceiptV1::new(
                policy.identity(),
                request_claims(),
                compiler_claims(),
                identity(0xa1),
                identity(0xa2),
                identity(0xa3),
                entries,
            )
        };
        let mut reordered = entries();
        reordered.swap(0, 1);
        assert!(matches!(
            make(reordered),
            Err(M1AllKernelsProtectedReceiptErrorV1::InvalidEntryOrdinal { .. })
        ));
        let mut duplicate_lineage = entries();
        duplicate_lineage[1].lineage_identity = duplicate_lineage[0].lineage_identity;
        assert_eq!(
            make(duplicate_lineage).unwrap_err(),
            M1AllKernelsProtectedReceiptErrorV1::DuplicateEntryIdentity {
                field: M1AllKernelsProtectedReceiptDuplicateFieldV1::Lineage,
                ordinal: 1,
            },
        );
        let mut duplicate_marker = entries();
        duplicate_marker[2].marker_binding_identity = duplicate_marker[0].marker_binding_identity;
        assert_eq!(
            make(duplicate_marker).unwrap_err(),
            M1AllKernelsProtectedReceiptErrorV1::DuplicateEntryIdentity {
                field: M1AllKernelsProtectedReceiptDuplicateFieldV1::MarkerBinding,
                ordinal: 2,
            },
        );
        let incomplete = M1AllKernelsProtectedReceiptEntryV1::new(
            0,
            identity(1),
            identity(2),
            identity(3),
            identity(4),
            identity(5),
            identity(6),
            WorkerV3SafetyPropertiesV1::new(0x7f).unwrap(),
        );
        assert_eq!(
            incomplete.unwrap_err(),
            M1AllKernelsProtectedReceiptErrorV1::MissingRequiredSafetyProperties {
                ordinal: 0,
                actual: 0x7f,
            },
        );
        let zero = M1AllKernelsProtectedReceiptEntryV1::new(
            0,
            identity(1),
            identity(2),
            identity(3),
            [0; 32],
            identity(5),
            identity(6),
            WorkerV3SafetyPropertiesV1::required(),
        );
        assert_eq!(
            zero.unwrap_err(),
            M1AllKernelsProtectedReceiptErrorV1::ZeroIdentity {
                field: M1AllKernelsProtectedReceiptIdentityFieldV1::EntryProofExecutableBinding,
                ordinal: Some(0),
            },
        );
    }

    #[test]
    fn header_target_and_reserved_mutations_have_typed_rejections() {
        let (_, receipt) = signed();
        let cases = [
            (0, M1AllKernelsProtectedReceiptErrorV1::InvalidMagic),
            (
                8,
                M1AllKernelsProtectedReceiptErrorV1::UnsupportedVersion { actual: 129 },
            ),
            (
                10,
                M1AllKernelsProtectedReceiptErrorV1::InvalidEntryCount { actual: 140 },
            ),
            (
                12,
                M1AllKernelsProtectedReceiptErrorV1::InvalidHeaderLength { actual: 160 },
            ),
            (
                14,
                M1AllKernelsProtectedReceiptErrorV1::InvalidEntryLength { actual: 72 },
            ),
            (
                16,
                M1AllKernelsProtectedReceiptErrorV1::InvalidDeclaredLength { actual: 3_424 },
            ),
            (20, M1AllKernelsProtectedReceiptErrorV1::NonzeroReserved),
            (24, M1AllKernelsProtectedReceiptErrorV1::NonzeroReserved),
            (968, M1AllKernelsProtectedReceiptErrorV1::InvalidTarget),
            (
                984,
                M1AllKernelsProtectedReceiptErrorV1::InvalidCodeObjectVersion { actual: 134 },
            ),
            (986, M1AllKernelsProtectedReceiptErrorV1::NonzeroReserved),
        ];
        for (offset, expected) in cases {
            let mut hostile = *receipt.encode_canonical();
            hostile[offset] ^= 0x80;
            assert_eq!(
                M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(&hostile).unwrap_err(),
                expected,
                "offset {offset}",
            );
        }
    }

    #[test]
    fn zero_lengths_sequence_theorems_and_unadvanced_anchor_reject_on_decode() {
        let (_, receipt) = signed();
        let mut zero_module_length = *receipt.encode_canonical();
        zero_module_length[224..232].fill(0);
        assert_eq!(
            M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(&zero_module_length)
                .unwrap_err(),
            M1AllKernelsProtectedReceiptErrorV1::ZeroLength {
                field: M1AllKernelsProtectedReceiptLengthFieldV1::CompilerModule,
            },
        );

        let mut zero_sequence = *receipt.encode_canonical();
        zero_sequence[600..608].fill(0);
        assert_eq!(
            M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(&zero_sequence).unwrap_err(),
            M1AllKernelsProtectedReceiptErrorV1::ZeroCompilerExecutionSequence,
        );

        let mut unadvanced = *receipt.encode_canonical();
        unadvanced[608..640].copy_from_slice(&[30; 32]);
        assert_eq!(
            M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(&unadvanced).unwrap_err(),
            M1AllKernelsProtectedReceiptErrorV1::UnadvancedCompilerExecutionRollbackAnchor,
        );

        let mut zero_theorem = *receipt.encode_canonical();
        zero_theorem[1_192..1_224].fill(0);
        assert_eq!(
            M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(&zero_theorem).unwrap_err(),
            M1AllKernelsProtectedReceiptErrorV1::ZeroIdentity {
                field: M1AllKernelsProtectedReceiptIdentityFieldV1::EntryProofExecutableBinding,
                ordinal: Some(0),
            },
        );
    }

    #[test]
    fn unsigned_and_authenticated_values_explicitly_grant_no_runtime_authority() {
        let (policy, receipt) = signed();
        let authenticated = policy
            .authenticate_canonical(receipt.encode_canonical())
            .unwrap();
        assert!(!authenticated.grants_verifier_authority());
        assert!(!authenticated.grants_load_authority());
        assert!(!authenticated.grants_launch_authority());
    }
}
