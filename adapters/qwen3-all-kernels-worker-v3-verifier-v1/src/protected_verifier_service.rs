//! Canonical packets for one aggregate protected-verifier exchange.
//!
//! The request binds caller-provisioned policy identity, the exact aggregate
//! challenge, source pin, compiler currentness, finalized artifact, and all 12
//! host-known entry coordinates. The response carries one exact signed receipt
//! and correlates it to that request. Packet identities detect transport
//! corruption; only caller-side receipt authentication establishes signer
//! authenticity.
//!
//! This V1 transports coordinates, not their evidence payloads. A conforming
//! protected service must already hold, or authentically reacquire, the exact
//! receipt-bearing Worker V3 V2 envelope, finalized HSACO, semantic and proof
//! inputs, and protected current-record evidence named by the request. It must
//! verify those payloads rather than sign a hash echo. It must also atomically
//! consume the challenge and validate sequence and rollback anchor against
//! protected live current-ledger state shared across instances and restarts.
//! Canonical packets, request correlation, and a one-shot client do not by
//! themselves establish evidence custody, freshness, or currentness. These
//! inert packets grant no verifier or runtime authority.

#![allow(
    clippy::must_use_candidate,
    reason = "public accessors expose inert packet coordinates and authority-free state"
)]
#![forbid(unsafe_code)]

use core::fmt;
use std::error::Error;

use sha2::{Digest, Sha256};

use crate::M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1;
use crate::protected_receipt::{
    M1_ALL_KERNELS_PROTECTED_RECEIPT_BYTES_V1, M1AllKernelsProtectedReceiptCompilerClaimsV1,
    M1AllKernelsProtectedReceiptEntryV1, M1AllKernelsProtectedReceiptErrorV1,
    M1AllKernelsProtectedReceiptRequestClaimsV1, M1AllKernelsProtectedReceiptSourcePinV1,
    M1AllKernelsProtectedVerifierReceiptV1, M1AllKernelsProtectedVerifierTrustPolicyIdentityV1,
};

const SHA256_BYTES: usize = 32;
const HEADER_BYTES: usize = 24;
const TARGET_BLOCK_BYTES: usize = 24;
const REQUEST_CLAIMS_BYTES: usize = 384;
const COMPILER_CLAIMS_BYTES: usize = 520;
const ENTRY_COORDINATES_BYTES: usize = 104;
const VERSION: u16 = 1;
const REQUEST_KIND: u16 = 1;
const RESPONSE_KIND: u16 = 1;
const TARGET_BYTES: usize = 16;
const TARGET: &[u8] = b"gfx942:xnack-";
const CODE_OBJECT_VERSION: u16 = 6;
const REQUEST_MAGIC: [u8; 8] = *b"FRW3VSQ1";
const RESPONSE_MAGIC: [u8; 8] = *b"FRW3VSP1";
const REQUEST_IDENTITY_DOMAIN: &[u8] =
    b"FERRIC/M1/ALL-KERNELS/PROTECTED-VERIFIER/SERVICE-REQUEST/V1\0";
const RESPONSE_IDENTITY_DOMAIN: &[u8] =
    b"FERRIC/M1/ALL-KERNELS/PROTECTED-VERIFIER/SERVICE-RESPONSE/V1\0";

const REQUEST_PREIMAGE_BYTES: usize = HEADER_BYTES
    + SHA256_BYTES
    + 8
    + SHA256_BYTES
    + TARGET_BLOCK_BYTES
    + REQUEST_CLAIMS_BYTES
    + COMPILER_CLAIMS_BYTES
    + (M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1 * ENTRY_COORDINATES_BYTES);

/// Exact byte length of one aggregate protected-verifier request packet.
pub const M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1: usize =
    REQUEST_PREIMAGE_BYTES + SHA256_BYTES;

const RESPONSE_PREIMAGE_BYTES: usize = HEADER_BYTES
    + SHA256_BYTES
    + SHA256_BYTES
    + 8
    + SHA256_BYTES
    + TARGET_BLOCK_BYTES
    + SHA256_BYTES
    + M1_ALL_KERNELS_PROTECTED_RECEIPT_BYTES_V1;

/// Exact byte length of one aggregate protected-verifier response packet.
pub const M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1: usize =
    RESPONSE_PREIMAGE_BYTES + SHA256_BYTES;

/// Maximum request packet accepted by the aggregate protected verifier.
pub const MAX_M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1: usize =
    M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1;

/// Maximum response packet accepted by the aggregate protected-verifier client.
pub const MAX_M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1: usize =
    M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1;

const _: [(); 2_304] = [(); M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1];
const _: [(); 3_768] = [(); M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1];

/// Domain-separated identity of one exact canonical request packet.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct M1AllKernelsProtectedVerifierServiceRequestIdentityV1([u8; SHA256_BYTES]);

impl M1AllKernelsProtectedVerifierServiceRequestIdentityV1 {
    /// Returns the exact request identity bytes.
    pub const fn as_bytes(&self) -> &[u8; SHA256_BYTES] {
        &self.0
    }
}

impl fmt::Debug for M1AllKernelsProtectedVerifierServiceRequestIdentityV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("M1AllKernelsProtectedVerifierServiceRequestIdentityV1")
            .field(&self.0)
            .finish()
    }
}

/// Domain-separated identity of one exact canonical response packet.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct M1AllKernelsProtectedVerifierServiceResponseIdentityV1([u8; SHA256_BYTES]);

impl M1AllKernelsProtectedVerifierServiceResponseIdentityV1 {
    /// Returns the exact response identity bytes.
    pub const fn as_bytes(&self) -> &[u8; SHA256_BYTES] {
        &self.0
    }
}

impl fmt::Debug for M1AllKernelsProtectedVerifierServiceResponseIdentityV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("M1AllKernelsProtectedVerifierServiceResponseIdentityV1")
            .field(&self.0)
            .finish()
    }
}

/// Host-known coordinates for one canonical aggregate roster entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1AllKernelsProtectedVerifierServiceEntryV1 {
    ordinal: u16,
    lineage_identity: [u8; SHA256_BYTES],
    marker_binding_identity: [u8; SHA256_BYTES],
    generated_host_contract_identity: [u8; SHA256_BYTES],
}

impl M1AllKernelsProtectedVerifierServiceEntryV1 {
    /// Constructs one exact, nonzero entry-coordinate row.
    ///
    /// # Errors
    ///
    /// Returns a typed error when any required identity is zero.
    pub fn new(
        ordinal: u16,
        lineage_identity: [u8; SHA256_BYTES],
        marker_binding_identity: [u8; SHA256_BYTES],
        generated_host_contract_identity: [u8; SHA256_BYTES],
    ) -> Result<Self, M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
        for (identity, field) in [
            (
                lineage_identity,
                M1AllKernelsProtectedVerifierServiceIdentityFieldV1::EntryLineage,
            ),
            (
                marker_binding_identity,
                M1AllKernelsProtectedVerifierServiceIdentityFieldV1::EntryMarkerBinding,
            ),
            (
                generated_host_contract_identity,
                M1AllKernelsProtectedVerifierServiceIdentityFieldV1::EntryGeneratedHostContract,
            ),
        ] {
            require_identity(identity, field, Some(ordinal))?;
        }
        Ok(Self {
            ordinal,
            lineage_identity,
            marker_binding_identity,
            generated_host_contract_identity,
        })
    }

    /// Projects the host-known coordinates from one decoded signed result.
    pub fn from_receipt_entry(entry: &M1AllKernelsProtectedReceiptEntryV1) -> Self {
        Self {
            ordinal: entry.ordinal(),
            lineage_identity: entry.lineage_identity(),
            marker_binding_identity: entry.marker_binding_identity(),
            generated_host_contract_identity: entry.generated_host_contract_identity(),
        }
    }

    /// Returns the canonical descriptor-table ordinal.
    pub const fn ordinal(&self) -> u16 {
        self.ordinal
    }

    /// Returns the exact host lineage identity.
    pub const fn lineage_identity(&self) -> [u8; SHA256_BYTES] {
        self.lineage_identity
    }

    /// Returns the exact compiler-generated marker binding.
    pub const fn marker_binding_identity(&self) -> [u8; SHA256_BYTES] {
        self.marker_binding_identity
    }

    /// Returns the exact generated host-contract identity.
    pub const fn generated_host_contract_identity(&self) -> [u8; SHA256_BYTES] {
        self.generated_host_contract_identity
    }

    fn matches_receipt(&self, receipt: &M1AllKernelsProtectedReceiptEntryV1) -> bool {
        self.ordinal == receipt.ordinal()
            && self.lineage_identity == receipt.lineage_identity()
            && self.marker_binding_identity == receipt.marker_binding_identity()
            && self.generated_host_contract_identity == receipt.generated_host_contract_identity()
    }

    fn encode_into(&self, writer: &mut Writer) {
        writer.u16(self.ordinal);
        writer.zeros(6);
        writer.identity(self.lineage_identity);
        writer.identity(self.marker_binding_identity);
        writer.identity(self.generated_host_contract_identity);
    }

    fn decode(
        reader: &mut Reader<'_>,
    ) -> Result<Self, M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
        let ordinal = reader.u16()?;
        reader.require_zero(6)?;
        Self::new(
            ordinal,
            reader.identity()?,
            reader.identity()?,
            reader.identity()?,
        )
    }
}

/// Canonical request for one exact signed aggregate protected-verifier receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct M1AllKernelsProtectedVerifierServiceRequestV1 {
    trust_policy_identity: M1AllKernelsProtectedVerifierTrustPolicyIdentityV1,
    expected_sequence: u64,
    expected_current_rollback_anchor: [u8; SHA256_BYTES],
    request_claims: M1AllKernelsProtectedReceiptRequestClaimsV1,
    compiler_claims: M1AllKernelsProtectedReceiptCompilerClaimsV1,
    entries: [M1AllKernelsProtectedVerifierServiceEntryV1; M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1],
    identity: M1AllKernelsProtectedVerifierServiceRequestIdentityV1,
    canonical_bytes: [u8; M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1],
}

impl M1AllKernelsProtectedVerifierServiceRequestV1 {
    /// Constructs one fixed-width receipt request from caller-owned coordinates.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the policy, rollback position, or ordered
    /// entry coordinates are invalid.
    pub fn new(
        trust_policy_identity: M1AllKernelsProtectedVerifierTrustPolicyIdentityV1,
        request_claims: M1AllKernelsProtectedReceiptRequestClaimsV1,
        compiler_claims: M1AllKernelsProtectedReceiptCompilerClaimsV1,
        entries: [M1AllKernelsProtectedVerifierServiceEntryV1;
            M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1],
    ) -> Result<Self, M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
        Self::encode(
            trust_policy_identity,
            compiler_claims.sequence(),
            compiler_claims.current_rollback_anchor(),
            &request_claims,
            &compiler_claims,
            &entries,
        )
    }

    /// Strictly decodes one complete request packet.
    ///
    /// # Errors
    ///
    /// Returns a typed error for every noncanonical, truncated, extended, or
    /// internally inconsistent request packet.
    pub fn decode(
        bytes: &[u8],
    ) -> Result<Self, M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
        if bytes.len() != M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1 {
            return Err(
                M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidRequestLength {
                    expected: M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1,
                    actual: bytes.len(),
                },
            );
        }
        let mut reader = Reader::new(bytes);
        decode_header(&mut reader, REQUEST_MAGIC, REQUEST_KIND, bytes.len())?;
        let policy_bytes = reader.identity()?;
        require_identity(
            policy_bytes,
            M1AllKernelsProtectedVerifierServiceIdentityFieldV1::TrustPolicy,
            None,
        )?;
        let trust_policy_identity =
            M1AllKernelsProtectedVerifierTrustPolicyIdentityV1::from_protocol_bytes(policy_bytes);
        let expected_sequence = reader.u64()?;
        let expected_current_rollback_anchor = reader.identity()?;
        decode_target_block(&mut reader)?;
        let request_claims = decode_request_claims(&mut reader)?;
        let compiler_claims = decode_compiler_claims(&mut reader)?;
        let mut entries = Vec::with_capacity(M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1);
        for _ in 0..M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1 {
            entries.push(M1AllKernelsProtectedVerifierServiceEntryV1::decode(
                &mut reader,
            )?);
        }
        let entries = entries
            .try_into()
            .map_err(|_| M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidEntryCount)?;
        let declared_identity = reader.identity()?;
        if !reader.is_finished() {
            return Err(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::TrailingBytes);
        }
        let decoded = Self::encode(
            trust_policy_identity,
            expected_sequence,
            expected_current_rollback_anchor,
            &request_claims,
            &compiler_claims,
            &entries,
        )?;
        if declared_identity != decoded.identity.0 || decoded.canonical_bytes != bytes {
            return Err(
                M1AllKernelsProtectedVerifierServiceProtocolErrorV1::RequestIdentityMismatch,
            );
        }
        Ok(decoded)
    }

    fn encode(
        trust_policy_identity: M1AllKernelsProtectedVerifierTrustPolicyIdentityV1,
        expected_sequence: u64,
        expected_current_rollback_anchor: [u8; SHA256_BYTES],
        request_claims: &M1AllKernelsProtectedReceiptRequestClaimsV1,
        compiler_claims: &M1AllKernelsProtectedReceiptCompilerClaimsV1,
        entries: &[M1AllKernelsProtectedVerifierServiceEntryV1;
             M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1],
    ) -> Result<Self, M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
        validate_request(
            trust_policy_identity,
            expected_sequence,
            expected_current_rollback_anchor,
            compiler_claims,
            entries,
        )?;
        let mut writer = Writer::new();
        encode_header(
            &mut writer,
            REQUEST_MAGIC,
            REQUEST_KIND,
            M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1,
        );
        writer.bytes(trust_policy_identity.as_bytes());
        writer.u64(expected_sequence);
        writer.identity(expected_current_rollback_anchor);
        encode_target_block(&mut writer);
        encode_request_claims(request_claims, &mut writer);
        encode_compiler_claims(compiler_claims, &mut writer);
        for entry in entries {
            entry.encode_into(&mut writer);
        }
        debug_assert_eq!(writer.position(), REQUEST_PREIMAGE_BYTES);
        let identity = M1AllKernelsProtectedVerifierServiceRequestIdentityV1(hash_parts(&[
            REQUEST_IDENTITY_DOMAIN,
            writer.written(),
        ]));
        writer.identity(identity.0);
        let canonical_bytes = writer.finish_request();
        Ok(Self {
            trust_policy_identity,
            expected_sequence,
            expected_current_rollback_anchor,
            request_claims: *request_claims,
            compiler_claims: *compiler_claims,
            entries: *entries,
            identity,
            canonical_bytes,
        })
    }

    /// Returns the caller-provisioned trust-policy identity.
    pub const fn trust_policy_identity(
        &self,
    ) -> M1AllKernelsProtectedVerifierTrustPolicyIdentityV1 {
        self.trust_policy_identity
    }

    /// Returns the exact compiler-execution sequence expected by this request.
    pub const fn expected_sequence(&self) -> u64 {
        self.expected_sequence
    }

    /// Returns the exact current compiler rollback anchor expected by this request.
    pub const fn expected_current_rollback_anchor(&self) -> [u8; SHA256_BYTES] {
        self.expected_current_rollback_anchor
    }

    /// Returns the complete host-known request coordinates.
    pub const fn request_claims(&self) -> &M1AllKernelsProtectedReceiptRequestClaimsV1 {
        &self.request_claims
    }

    /// Returns the complete compiler/currentness coordinates.
    pub const fn compiler_claims(&self) -> &M1AllKernelsProtectedReceiptCompilerClaimsV1 {
        &self.compiler_claims
    }

    /// Returns the exact 12 ordered host entry coordinates.
    pub const fn entries(
        &self,
    ) -> &[M1AllKernelsProtectedVerifierServiceEntryV1; M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1] {
        &self.entries
    }

    /// Returns the domain-separated identity of the complete request packet.
    pub const fn identity(&self) -> M1AllKernelsProtectedVerifierServiceRequestIdentityV1 {
        self.identity
    }

    /// Returns the exact canonical request bytes.
    pub const fn canonical_bytes(
        &self,
    ) -> &[u8; M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1] {
        &self.canonical_bytes
    }

    /// Checks every signed caller-known response coordinate against this request.
    pub fn matches_receipt(&self, receipt: &M1AllKernelsProtectedVerifierReceiptV1) -> bool {
        receipt.trust_policy_identity() == self.trust_policy_identity
            && receipt.request_claims() == &self.request_claims
            && receipt.compiler_claims() == &self.compiler_claims
            && self
                .entries
                .iter()
                .zip(receipt.entries())
                .all(|(expected, actual)| expected.matches_receipt(actual))
    }

    /// Transport correlation alone grants no protected-verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

/// Canonical response carrying one exact signed aggregate receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct M1AllKernelsProtectedVerifierServiceResponseV1 {
    request_identity: M1AllKernelsProtectedVerifierServiceRequestIdentityV1,
    trust_policy_identity: M1AllKernelsProtectedVerifierTrustPolicyIdentityV1,
    sequence: u64,
    current_rollback_anchor: [u8; SHA256_BYTES],
    receipt: M1AllKernelsProtectedVerifierReceiptV1,
    identity: M1AllKernelsProtectedVerifierServiceResponseIdentityV1,
    canonical_bytes: [u8; M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1],
}

impl M1AllKernelsProtectedVerifierServiceResponseV1 {
    /// Constructs a correlated response only when the receipt matches every request coordinate.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the receipt names any different request,
    /// compiler, policy, artifact, or entry coordinate.
    pub fn new(
        request: &M1AllKernelsProtectedVerifierServiceRequestV1,
        receipt: M1AllKernelsProtectedVerifierReceiptV1,
    ) -> Result<Self, M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
        if !request.matches_receipt(&receipt) {
            return Err(
                M1AllKernelsProtectedVerifierServiceProtocolErrorV1::ReceiptRequestMismatch,
            );
        }
        Ok(Self::encode(
            request.identity,
            request.trust_policy_identity,
            request.expected_sequence,
            request.expected_current_rollback_anchor,
            receipt,
        ))
    }

    /// Strictly decodes one complete response packet and embedded canonical receipt.
    ///
    /// # Errors
    ///
    /// Returns a typed error for every noncanonical, truncated, extended, or
    /// internally inconsistent response or embedded receipt.
    pub fn decode(
        bytes: &[u8],
    ) -> Result<Self, M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
        if bytes.len() != M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1 {
            return Err(
                M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidResponseLength {
                    expected: M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1,
                    actual: bytes.len(),
                },
            );
        }
        let mut reader = Reader::new(bytes);
        decode_header(&mut reader, RESPONSE_MAGIC, RESPONSE_KIND, bytes.len())?;
        let request_identity =
            M1AllKernelsProtectedVerifierServiceRequestIdentityV1(reader.identity()?);
        require_identity(
            request_identity.0,
            M1AllKernelsProtectedVerifierServiceIdentityFieldV1::Request,
            None,
        )?;
        let policy_bytes = reader.identity()?;
        require_identity(
            policy_bytes,
            M1AllKernelsProtectedVerifierServiceIdentityFieldV1::TrustPolicy,
            None,
        )?;
        let trust_policy_identity =
            M1AllKernelsProtectedVerifierTrustPolicyIdentityV1::from_protocol_bytes(policy_bytes);
        let sequence = reader.u64()?;
        let current_rollback_anchor = reader.identity()?;
        validate_position(sequence, current_rollback_anchor)?;
        decode_target_block(&mut reader)?;
        let declared_receipt_identity = reader.identity()?;
        let receipt = M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(
            reader.take(M1_ALL_KERNELS_PROTECTED_RECEIPT_BYTES_V1)?,
        )?;
        if declared_receipt_identity != *receipt.identity().as_bytes() {
            return Err(
                M1AllKernelsProtectedVerifierServiceProtocolErrorV1::ReceiptIdentityMismatch,
            );
        }
        if receipt.trust_policy_identity() != trust_policy_identity
            || receipt.compiler_claims().sequence() != sequence
            || receipt.compiler_claims().current_rollback_anchor() != current_rollback_anchor
        {
            return Err(
                M1AllKernelsProtectedVerifierServiceProtocolErrorV1::ReceiptRequestMismatch,
            );
        }
        let declared_response_identity = reader.identity()?;
        if !reader.is_finished() {
            return Err(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::TrailingBytes);
        }
        let decoded = Self::encode(
            request_identity,
            trust_policy_identity,
            sequence,
            current_rollback_anchor,
            receipt,
        );
        if declared_response_identity != decoded.identity.0 || decoded.canonical_bytes != bytes {
            return Err(
                M1AllKernelsProtectedVerifierServiceProtocolErrorV1::ResponseIdentityMismatch,
            );
        }
        Ok(decoded)
    }

    fn encode(
        request_identity: M1AllKernelsProtectedVerifierServiceRequestIdentityV1,
        trust_policy_identity: M1AllKernelsProtectedVerifierTrustPolicyIdentityV1,
        sequence: u64,
        current_rollback_anchor: [u8; SHA256_BYTES],
        receipt: M1AllKernelsProtectedVerifierReceiptV1,
    ) -> Self {
        let mut writer = Writer::new();
        encode_header(
            &mut writer,
            RESPONSE_MAGIC,
            RESPONSE_KIND,
            M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1,
        );
        writer.identity(request_identity.0);
        writer.bytes(trust_policy_identity.as_bytes());
        writer.u64(sequence);
        writer.identity(current_rollback_anchor);
        encode_target_block(&mut writer);
        writer.bytes(receipt.identity().as_bytes());
        writer.bytes(receipt.encode_canonical());
        debug_assert_eq!(writer.position(), RESPONSE_PREIMAGE_BYTES);
        let identity = M1AllKernelsProtectedVerifierServiceResponseIdentityV1(hash_parts(&[
            RESPONSE_IDENTITY_DOMAIN,
            writer.written(),
        ]));
        writer.identity(identity.0);
        let canonical_bytes = writer.finish_response();
        Self {
            request_identity,
            trust_policy_identity,
            sequence,
            current_rollback_anchor,
            receipt,
            identity,
            canonical_bytes,
        }
    }

    /// Returns the exact request identity named by this response.
    pub const fn request_identity(&self) -> M1AllKernelsProtectedVerifierServiceRequestIdentityV1 {
        self.request_identity
    }

    /// Returns the exact caller-provisioned trust-policy identity.
    pub const fn trust_policy_identity(
        &self,
    ) -> M1AllKernelsProtectedVerifierTrustPolicyIdentityV1 {
        self.trust_policy_identity
    }

    /// Returns the compiler-execution sequence repeated from the signed receipt.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Returns the current rollback anchor repeated from the signed receipt.
    pub const fn current_rollback_anchor(&self) -> [u8; SHA256_BYTES] {
        self.current_rollback_anchor
    }

    /// Returns the structurally decoded, not yet caller-authenticated receipt.
    pub const fn receipt(&self) -> &M1AllKernelsProtectedVerifierReceiptV1 {
        &self.receipt
    }

    /// Consumes the response and returns its structurally decoded receipt.
    pub fn into_receipt(self) -> M1AllKernelsProtectedVerifierReceiptV1 {
        self.receipt
    }

    /// Returns the domain-separated identity of the complete response packet.
    pub const fn identity(&self) -> M1AllKernelsProtectedVerifierServiceResponseIdentityV1 {
        self.identity
    }

    /// Returns the exact canonical response bytes.
    pub const fn canonical_bytes(
        &self,
    ) -> &[u8; M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1] {
        &self.canonical_bytes
    }

    /// Checks request identity, policy, current position, and every receipt coordinate.
    pub fn matches_request(&self, request: &M1AllKernelsProtectedVerifierServiceRequestV1) -> bool {
        self.request_identity == request.identity
            && self.trust_policy_identity == request.trust_policy_identity
            && self.sequence == request.expected_sequence
            && self.current_rollback_anchor == request.expected_current_rollback_anchor
            && request.matches_receipt(&self.receipt)
    }

    /// A structurally correlated response grants no protected-verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

fn validate_request(
    trust_policy_identity: M1AllKernelsProtectedVerifierTrustPolicyIdentityV1,
    expected_sequence: u64,
    expected_current_rollback_anchor: [u8; SHA256_BYTES],
    compiler_claims: &M1AllKernelsProtectedReceiptCompilerClaimsV1,
    entries: &[M1AllKernelsProtectedVerifierServiceEntryV1; M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1],
) -> Result<(), M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
    require_identity(
        *trust_policy_identity.as_bytes(),
        M1AllKernelsProtectedVerifierServiceIdentityFieldV1::TrustPolicy,
        None,
    )?;
    validate_position(expected_sequence, expected_current_rollback_anchor)?;
    if expected_sequence != compiler_claims.sequence()
        || expected_current_rollback_anchor != compiler_claims.current_rollback_anchor()
    {
        return Err(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::CompilerPositionMismatch);
    }
    validate_entries(entries)
}

fn validate_position(
    sequence: u64,
    current_rollback_anchor: [u8; SHA256_BYTES],
) -> Result<(), M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
    if sequence == 0 || current_rollback_anchor == [0; SHA256_BYTES] {
        return Err(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidRollbackPosition);
    }
    Ok(())
}

fn validate_entries(
    entries: &[M1AllKernelsProtectedVerifierServiceEntryV1; M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1],
) -> Result<(), M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
    for (index, entry) in entries.iter().enumerate() {
        let expected = u16::try_from(index)
            .map_err(|_| M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidEntryCount)?;
        if entry.ordinal != expected {
            return Err(
                M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidEntryOrdinal {
                    expected,
                    actual: entry.ordinal,
                },
            );
        }
        for prior in &entries[..index] {
            if prior.lineage_identity == entry.lineage_identity {
                return Err(
                    M1AllKernelsProtectedVerifierServiceProtocolErrorV1::DuplicateEntryIdentity {
                        field: M1AllKernelsProtectedVerifierServiceDuplicateFieldV1::Lineage,
                        ordinal: entry.ordinal,
                    },
                );
            }
            if prior.marker_binding_identity == entry.marker_binding_identity {
                return Err(
                    M1AllKernelsProtectedVerifierServiceProtocolErrorV1::DuplicateEntryIdentity {
                        field: M1AllKernelsProtectedVerifierServiceDuplicateFieldV1::MarkerBinding,
                        ordinal: entry.ordinal,
                    },
                );
            }
        }
    }
    Ok(())
}

fn encode_request_claims(
    claims: &M1AllKernelsProtectedReceiptRequestClaimsV1,
    writer: &mut Writer,
) {
    writer.identity(claims.challenge_identity());
    writer.identity(claims.roster_identity());
    writer.identity(claims.host_lineage_identity());
    writer.identity(claims.finalizer_derivation_sha256());
    let source = claims.source_pin();
    writer.identity(source.compiler_module_sha256());
    writer.u64(source.compiler_module_length());
    writer.identity(source.compiler_handoff_sha256());
    writer.u64(source.compiler_handoff_length());
    writer.identity(source.symbol_manifest_sha256());
    writer.u64(source.symbol_manifest_length());
    writer.identity(claims.capsule_sha256());
    writer.identity(claims.formal_memory_receipt_sha256());
    writer.identity(claims.proof_binding_receipt_sha256());
    writer.identity(claims.finalized_hsaco_sha256());
    writer.u64(claims.finalized_hsaco_length());
}

fn decode_request_claims(
    reader: &mut Reader<'_>,
) -> Result<
    M1AllKernelsProtectedReceiptRequestClaimsV1,
    M1AllKernelsProtectedVerifierServiceProtocolErrorV1,
> {
    let challenge = reader.identity()?;
    let roster = reader.identity()?;
    let lineage = reader.identity()?;
    let finalizer = reader.identity()?;
    let source = M1AllKernelsProtectedReceiptSourcePinV1::new(
        reader.identity()?,
        reader.u64()?,
        reader.identity()?,
        reader.u64()?,
        reader.identity()?,
        reader.u64()?,
    )?;
    Ok(M1AllKernelsProtectedReceiptRequestClaimsV1::new(
        challenge,
        roster,
        lineage,
        finalizer,
        source,
        reader.identity()?,
        reader.identity()?,
        reader.identity()?,
        reader.identity()?,
        reader.u64()?,
    )?)
}

fn encode_compiler_claims(
    claims: &M1AllKernelsProtectedReceiptCompilerClaimsV1,
    writer: &mut Writer,
) {
    for identity in [
        claims.subject_sha256(),
        claims.carriage_sha256(),
        claims.policy_sha256(),
        claims.issuer_journal_sha256(),
        claims.compiler_occurrence_sha256(),
        claims.receipt_sha256(),
        claims.publication_sha256(),
        claims.acknowledgment_sha256(),
        claims.worker_ledger_record_sha256(),
    ] {
        writer.identity(identity);
    }
    writer.u64(claims.sequence());
    writer.identity(claims.prior_rollback_anchor());
    writer.identity(claims.current_rollback_anchor());
    for identity in [
        claims.current_record_verification_sha256(),
        claims.current_record_attestation_sha256(),
        claims.protected_policy_verification_sha256(),
        claims.protected_worker_ledger_verification_sha256(),
        claims.external_rollback_verification_sha256(),
    ] {
        writer.identity(identity);
    }
}

fn decode_compiler_claims(
    reader: &mut Reader<'_>,
) -> Result<
    M1AllKernelsProtectedReceiptCompilerClaimsV1,
    M1AllKernelsProtectedVerifierServiceProtocolErrorV1,
> {
    Ok(M1AllKernelsProtectedReceiptCompilerClaimsV1::new(
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
    )?)
}

fn encode_header(writer: &mut Writer, magic: [u8; 8], kind: u16, total: usize) {
    writer.bytes(&magic);
    writer.u16(VERSION);
    writer.u16(kind);
    writer.u16(u16::try_from(HEADER_BYTES).expect("fixed header length"));
    writer.u16(0);
    writer.u32(u32::try_from(total).expect("bounded packet length"));
    writer.u32(0);
}

fn decode_header(
    reader: &mut Reader<'_>,
    magic: [u8; 8],
    expected_kind: u16,
    actual_total: usize,
) -> Result<(), M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
    if reader.fixed::<8>()? != magic {
        return Err(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidMagic);
    }
    let version = reader.u16()?;
    if version != VERSION {
        return Err(
            M1AllKernelsProtectedVerifierServiceProtocolErrorV1::UnsupportedVersion {
                actual: version,
            },
        );
    }
    let kind = reader.u16()?;
    if kind != expected_kind {
        return Err(
            M1AllKernelsProtectedVerifierServiceProtocolErrorV1::UnknownPacketKind { actual: kind },
        );
    }
    let header = reader.u16()?;
    if usize::from(header) != HEADER_BYTES {
        return Err(
            M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidHeaderLength {
                actual: header,
            },
        );
    }
    if reader.u16()? != 0 {
        return Err(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::NonzeroReserved);
    }
    let declared = reader.u32()?;
    if usize::try_from(declared).ok() != Some(actual_total) {
        return Err(
            M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidDeclaredLength {
                actual: declared,
            },
        );
    }
    if reader.u32()? != 0 {
        return Err(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::NonzeroReserved);
    }
    Ok(())
}

fn encode_target_block(writer: &mut Writer) {
    let mut target = [0_u8; TARGET_BYTES];
    target[..TARGET.len()].copy_from_slice(TARGET);
    writer.bytes(&target);
    writer.u16(CODE_OBJECT_VERSION);
    writer.u16(u16::try_from(M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1).expect("fixed roster"));
    writer.u32(0);
}

fn decode_target_block(
    reader: &mut Reader<'_>,
) -> Result<(), M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
    let target = reader.fixed::<TARGET_BYTES>()?;
    if &target[..TARGET.len()] != TARGET || target[TARGET.len()..].iter().any(|byte| *byte != 0) {
        return Err(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidTarget);
    }
    let code_object_version = reader.u16()?;
    if code_object_version != CODE_OBJECT_VERSION {
        return Err(
            M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidCodeObjectVersion {
                actual: code_object_version,
            },
        );
    }
    if usize::from(reader.u16()?) != M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1 {
        return Err(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidEntryCount);
    }
    if reader.u32()? != 0 {
        return Err(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::NonzeroReserved);
    }
    Ok(())
}

fn require_identity(
    identity: [u8; SHA256_BYTES],
    field: M1AllKernelsProtectedVerifierServiceIdentityFieldV1,
    ordinal: Option<u16>,
) -> Result<(), M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
    if identity == [0; SHA256_BYTES] {
        return Err(
            M1AllKernelsProtectedVerifierServiceProtocolErrorV1::ZeroIdentity { field, ordinal },
        );
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

/// Identity field rejected because it is the all-zero sentinel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1AllKernelsProtectedVerifierServiceIdentityFieldV1 {
    /// Caller-provisioned trust policy.
    TrustPolicy,
    /// Correlated request identity in a response.
    Request,
    /// Per-entry host lineage.
    EntryLineage,
    /// Per-entry compiler marker binding.
    EntryMarkerBinding,
    /// Per-entry generated host contract.
    EntryGeneratedHostContract,
}

/// Duplicate entry coordinate rejected by the request decoder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1AllKernelsProtectedVerifierServiceDuplicateFieldV1 {
    /// Duplicate per-entry host lineage.
    Lineage,
    /// Duplicate per-entry compiler marker binding.
    MarkerBinding,
}

/// Canonical packet structure, correlation, or embedded receipt failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1AllKernelsProtectedVerifierServiceProtocolErrorV1 {
    /// Request packet length is not exact.
    InvalidRequestLength {
        /// Required byte length.
        expected: usize,
        /// Observed byte length.
        actual: usize,
    },
    /// Response packet length is not exact.
    InvalidResponseLength {
        /// Required byte length.
        expected: usize,
        /// Observed byte length.
        actual: usize,
    },
    /// Packet magic is not the required request or response magic.
    InvalidMagic,
    /// Packet version is unsupported.
    UnsupportedVersion {
        /// Observed version.
        actual: u16,
    },
    /// Packet kind is not the sole V1 operation.
    UnknownPacketKind {
        /// Observed packet kind.
        actual: u16,
    },
    /// Declared header length is noncanonical.
    InvalidHeaderLength {
        /// Observed header byte length.
        actual: u16,
    },
    /// Declared packet length differs from the received packet.
    InvalidDeclaredLength {
        /// Observed declared total byte length.
        actual: u32,
    },
    /// A reserved or flags field is nonzero.
    NonzeroReserved,
    /// Exact target is not `gfx942:xnack-`.
    InvalidTarget,
    /// Code-object version is not V6.
    InvalidCodeObjectVersion {
        /// Observed code-object version.
        actual: u16,
    },
    /// Roster entry count is not exactly 12.
    InvalidEntryCount,
    /// Required identity is the all-zero sentinel.
    ZeroIdentity {
        /// Rejected identity field.
        field: M1AllKernelsProtectedVerifierServiceIdentityFieldV1,
        /// Entry ordinal for entry-scoped identities.
        ordinal: Option<u16>,
    },
    /// Sequence or current rollback anchor is zero.
    InvalidRollbackPosition,
    /// Header position differs from the embedded compiler claims.
    CompilerPositionMismatch,
    /// Entry ordinal differs from its canonical array position.
    InvalidEntryOrdinal {
        /// Required canonical ordinal.
        expected: u16,
        /// Observed ordinal.
        actual: u16,
    },
    /// A lineage or marker binding occurs more than once.
    DuplicateEntryIdentity {
        /// Coordinate duplicated across entries.
        field: M1AllKernelsProtectedVerifierServiceDuplicateFieldV1,
        /// Later entry at which the duplicate was observed.
        ordinal: u16,
    },
    /// Decoder did not consume the complete packet.
    TrailingBytes,
    /// Request identity or canonical re-encoding changed.
    RequestIdentityMismatch,
    /// Response identity or canonical re-encoding changed.
    ResponseIdentityMismatch,
    /// Embedded receipt identity does not name the exact receipt bytes.
    ReceiptIdentityMismatch,
    /// Receipt common or entry coordinates differ from the request.
    ReceiptRequestMismatch,
    /// Embedded protected receipt is structurally invalid.
    Receipt(M1AllKernelsProtectedReceiptErrorV1),
}

impl fmt::Display for M1AllKernelsProtectedVerifierServiceProtocolErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "aggregate protected-verifier service packet rejected: {self:?}"
        )
    }
}

impl Error for M1AllKernelsProtectedVerifierServiceProtocolErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Receipt(error) => Some(error),
            _ => None,
        }
    }
}

impl From<M1AllKernelsProtectedReceiptErrorV1>
    for M1AllKernelsProtectedVerifierServiceProtocolErrorV1
{
    fn from(error: M1AllKernelsProtectedReceiptErrorV1) -> Self {
        Self::Receipt(error)
    }
}

struct Writer {
    bytes: [u8; M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1],
    position: usize,
}

impl Writer {
    const fn new() -> Self {
        Self {
            bytes: [0; M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1],
            position: 0,
        }
    }

    fn bytes(&mut self, value: &[u8]) {
        let end = self.position + value.len();
        self.bytes[self.position..end].copy_from_slice(value);
        self.position = end;
    }

    fn zeros(&mut self, count: usize) {
        self.position += count;
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

    const fn position(&self) -> usize {
        self.position
    }

    fn written(&self) -> &[u8] {
        &self.bytes[..self.position]
    }

    fn finish_request(self) -> [u8; M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1] {
        assert_eq!(
            self.position, M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1,
            "fixed request encoder length",
        );
        self.bytes[..self.position]
            .try_into()
            .expect("fixed request array length")
    }

    fn finish_response(self) -> [u8; M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1] {
        assert_eq!(
            self.position, M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1,
            "fixed response encoder length",
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

    fn take(
        &mut self,
        length: usize,
    ) -> Result<&'bytes [u8], M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::TrailingBytes)?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::TrailingBytes)?;
        self.position = end;
        Ok(value)
    }

    fn fixed<const N: usize>(
        &mut self,
    ) -> Result<[u8; N], M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
        self.take(N)?
            .try_into()
            .map_err(|_| M1AllKernelsProtectedVerifierServiceProtocolErrorV1::TrailingBytes)
    }

    fn require_zero(
        &mut self,
        length: usize,
    ) -> Result<(), M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
        if self.take(length)?.iter().any(|byte| *byte != 0) {
            return Err(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::NonzeroReserved);
        }
        Ok(())
    }

    fn u16(&mut self) -> Result<u16, M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
        Ok(u16::from_le_bytes(self.fixed()?))
    }

    fn u32(&mut self) -> Result<u32, M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
        Ok(u32::from_le_bytes(self.fixed()?))
    }

    fn u64(&mut self) -> Result<u64, M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
        Ok(u64::from_le_bytes(self.fixed()?))
    }

    fn identity(
        &mut self,
    ) -> Result<[u8; SHA256_BYTES], M1AllKernelsProtectedVerifierServiceProtocolErrorV1> {
        self.fixed()
    }

    const fn is_finished(&self) -> bool {
        self.position == self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protected_verifier_test_support::{fixture_request, signed_fixture};

    #[test]
    fn request_and_response_round_trip_exact_canonical_packets() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        assert_eq!(request.canonical_bytes().len(), 2_304);
        assert_eq!(&request.canonical_bytes()[..8], &REQUEST_MAGIC);
        let decoded =
            M1AllKernelsProtectedVerifierServiceRequestV1::decode(request.canonical_bytes())
                .unwrap();
        assert_eq!(decoded, request);
        assert!(decoded.matches_receipt(&receipt));
        assert!(!decoded.grants_authority());

        let response =
            M1AllKernelsProtectedVerifierServiceResponseV1::new(&request, receipt).unwrap();
        assert_eq!(response.canonical_bytes().len(), 3_768);
        assert_eq!(&response.canonical_bytes()[..8], &RESPONSE_MAGIC);
        let decoded =
            M1AllKernelsProtectedVerifierServiceResponseV1::decode(response.canonical_bytes())
                .unwrap();
        assert_eq!(decoded, response);
        assert!(decoded.matches_request(&request));
        assert!(!decoded.grants_authority());
    }

    #[test]
    fn every_request_and_response_single_byte_mutation_is_rejected() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        for offset in 0..request.canonical_bytes().len() {
            let mut hostile = *request.canonical_bytes();
            hostile[offset] ^= 0x80;
            assert!(
                M1AllKernelsProtectedVerifierServiceRequestV1::decode(&hostile).is_err(),
                "request mutation at {offset} survived",
            );
        }
        let response =
            M1AllKernelsProtectedVerifierServiceResponseV1::new(&request, receipt).unwrap();
        for offset in 0..response.canonical_bytes().len() {
            let mut hostile = *response.canonical_bytes();
            hostile[offset] ^= 0x80;
            assert!(
                M1AllKernelsProtectedVerifierServiceResponseV1::decode(&hostile).is_err(),
                "response mutation at {offset} survived",
            );
        }
    }

    #[test]
    fn every_truncation_and_trailing_byte_is_rejected() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        for length in 0..request.canonical_bytes().len() {
            assert!(matches!(
                M1AllKernelsProtectedVerifierServiceRequestV1::decode(
                    &request.canonical_bytes()[..length]
                ),
                Err(
                    M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidRequestLength { .. }
                )
            ));
        }
        let mut request_trailing = request.canonical_bytes().to_vec();
        request_trailing.push(0);
        assert!(matches!(
            M1AllKernelsProtectedVerifierServiceRequestV1::decode(&request_trailing),
            Err(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidRequestLength { .. })
        ));

        let response =
            M1AllKernelsProtectedVerifierServiceResponseV1::new(&request, receipt).unwrap();
        for length in 0..response.canonical_bytes().len() {
            assert!(matches!(
                M1AllKernelsProtectedVerifierServiceResponseV1::decode(
                    &response.canonical_bytes()[..length]
                ),
                Err(
                    M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidResponseLength { .. }
                )
            ));
        }
        let mut response_trailing = response.canonical_bytes().to_vec();
        response_trailing.push(0);
        assert!(matches!(
            M1AllKernelsProtectedVerifierServiceResponseV1::decode(&response_trailing),
            Err(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidResponseLength { .. })
        ));
    }

    #[test]
    fn request_rejects_noncanonical_entry_sets() {
        let (policy, receipt) = signed_fixture();
        let valid = fixture_request(&policy, &receipt);
        let mut entries = *valid.entries();
        entries.swap(0, 1);
        assert!(matches!(
            M1AllKernelsProtectedVerifierServiceRequestV1::new(
                policy.identity(),
                *valid.request_claims(),
                *valid.compiler_claims(),
                entries,
            ),
            Err(M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidEntryOrdinal { .. })
        ));
        let mut entries = *valid.entries();
        entries[1].lineage_identity = entries[0].lineage_identity;
        assert!(matches!(
            M1AllKernelsProtectedVerifierServiceRequestV1::new(
                policy.identity(),
                *valid.request_claims(),
                *valid.compiler_claims(),
                entries,
            ),
            Err(
                M1AllKernelsProtectedVerifierServiceProtocolErrorV1::DuplicateEntryIdentity {
                    field: M1AllKernelsProtectedVerifierServiceDuplicateFieldV1::Lineage,
                    ..
                }
            )
        ));
        let mut entries = *valid.entries();
        entries[1].marker_binding_identity = entries[0].marker_binding_identity;
        assert!(matches!(
            M1AllKernelsProtectedVerifierServiceRequestV1::new(
                policy.identity(),
                *valid.request_claims(),
                *valid.compiler_claims(),
                entries,
            ),
            Err(
                M1AllKernelsProtectedVerifierServiceProtocolErrorV1::DuplicateEntryIdentity {
                    field: M1AllKernelsProtectedVerifierServiceDuplicateFieldV1::MarkerBinding,
                    ..
                }
            )
        ));
    }

    #[test]
    fn response_constructor_rejects_request_substitution() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        let mut entries = *request.entries();
        entries[0] = M1AllKernelsProtectedVerifierServiceEntryV1::new(
            0,
            [0xe1; 32],
            entries[0].marker_binding_identity(),
            entries[0].generated_host_contract_identity(),
        )
        .unwrap();
        let substituted = M1AllKernelsProtectedVerifierServiceRequestV1::new(
            policy.identity(),
            *request.request_claims(),
            *request.compiler_claims(),
            entries,
        )
        .unwrap();
        assert_eq!(
            M1AllKernelsProtectedVerifierServiceResponseV1::new(&substituted, receipt).unwrap_err(),
            M1AllKernelsProtectedVerifierServiceProtocolErrorV1::ReceiptRequestMismatch,
        );
    }
}
