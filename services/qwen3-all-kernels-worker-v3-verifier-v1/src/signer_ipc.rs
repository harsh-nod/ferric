//! Descriptor-only IPC client for an externally protected receipt signer.
//!
//! The client consumes one already-connected Unix `SOCK_SEQPACKET` endpoint. It
//! never discovers or connects a path and never accepts private-key material.
//! Admission pins the exact socket object, peer credentials, policy, provider,
//! and public key supplied by the protected supervisor. A fixed canonical
//! request/response protocol binds the complete signing input and response
//! status. Any ambiguous outcome after a send attempt closes the endpoint and
//! permanently poisons the client.

#![allow(
    clippy::must_use_candidate,
    reason = "public accessors expose only inert protocol coordinates"
)]

use core::fmt;
use std::error::Error;
use std::io;
use std::mem;
use std::os::fd::{AsRawFd, OwnedFd};
use std::time::Duration;

use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_receipt::{
    M1_ALL_KERNELS_PROTECTED_RECEIPT_SIGNING_BYTES_V1,
    M1_ALL_KERNELS_PROTECTED_RECEIPT_SIGNING_DOMAIN_V1,
};
use rustix::fs::{FileType, OFlags};
use rustix::io::FdFlags;
use sha2::{Digest, Sha256};

use crate::{
    AbsoluteSessionDeadlineV1, ProtectedReceiptSignerInputV1, ProtectedReceiptSignerProviderV1,
};

const MAGIC_REQUEST_V1: [u8; 8] = *b"F3SGRQ1\0";
const MAGIC_RESPONSE_V1: [u8; 8] = *b"F3SGRS1\0";
const VERSION_V1: u16 = 1;
const HEADER_BYTES_V1: usize = 20;
const IDENTITY_BYTES: usize = 32;
const SIGNATURE_BYTES: usize = 64;
const INVALID_LINUX_ID: u32 = u32::MAX;
const REQUEST_IDENTITY_DOMAIN_V1: &[u8] = b"FERRIC/PROTECTED-RECEIPT-SIGNER/REQUEST/V1\0";
const RESPONSE_IDENTITY_DOMAIN_V1: &[u8] = b"FERRIC/PROTECTED-RECEIPT-SIGNER/RESPONSE/V1\0";
const REQUEST_DECLARED_BYTES_V1: u32 = 3_730;
const RESPONSE_DECLARED_BYTES_V1: u32 = 276;

/// Exact byte length of one protected receipt-signer request packet.
pub const PREOPENED_PROTECTED_RECEIPT_SIGNER_REQUEST_BYTES_V1: usize =
    HEADER_BYTES_V1 + (5 * IDENTITY_BYTES) + M1_ALL_KERNELS_PROTECTED_RECEIPT_SIGNING_BYTES_V1;

/// Exact byte length of one protected receipt-signer response packet.
pub const PREOPENED_PROTECTED_RECEIPT_SIGNER_RESPONSE_BYTES_V1: usize =
    HEADER_BYTES_V1 + (6 * IDENTITY_BYTES) + SIGNATURE_BYTES;

const _: () = assert!(M1_ALL_KERNELS_PROTECTED_RECEIPT_SIGNING_BYTES_V1 == 3_550);
const _: () = assert!(PREOPENED_PROTECTED_RECEIPT_SIGNER_REQUEST_BYTES_V1 == 3_730);
const _: () = assert!(PREOPENED_PROTECTED_RECEIPT_SIGNER_RESPONSE_BYTES_V1 == 276);

/// Supervisor-pinned identity of one preopened signer socket endpoint.
///
/// The numbers are inert metadata. Production callers must obtain them from a
/// separately admitted supervisor manifest rather than deriving an expectation
/// from the untrusted endpoint being admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtectedReceiptSignerEndpointIdentityV1 {
    device: u64,
    inode: u64,
}

impl ProtectedReceiptSignerEndpointIdentityV1 {
    /// Constructs one nonzero supervisor-pinned socket identity.
    ///
    /// # Errors
    ///
    /// Returns a typed error if either coordinate is zero.
    pub const fn new(
        device: u64,
        inode: u64,
    ) -> Result<Self, ProtectedReceiptSignerEndpointIdentityErrorV1> {
        if device == 0 {
            return Err(ProtectedReceiptSignerEndpointIdentityErrorV1::ZeroDevice);
        }
        if inode == 0 {
            return Err(ProtectedReceiptSignerEndpointIdentityErrorV1::ZeroInode);
        }
        Ok(Self { device, inode })
    }

    /// Returns the pinned socket device number.
    pub const fn device(self) -> u64 {
        self.device
    }

    /// Returns the pinned socket inode number.
    pub const fn inode(self) -> u64 {
        self.inode
    }

    /// Socket identity metadata grants no signing or verifier authority.
    pub const fn grants_authority(self) -> bool {
        false
    }
}

/// Invalid supervisor-pinned signer endpoint identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedReceiptSignerEndpointIdentityErrorV1 {
    /// Socket device number was zero.
    ZeroDevice,
    /// Socket inode number was zero.
    ZeroInode,
}

impl fmt::Display for ProtectedReceiptSignerEndpointIdentityErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid protected signer endpoint identity: {self:?}"
        )
    }
}

impl Error for ProtectedReceiptSignerEndpointIdentityErrorV1 {}

/// Supervisor-pinned kernel credentials for the protected signer peer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtectedReceiptSignerPeerIdentityV1 {
    pid: u32,
    uid: u32,
    gid: u32,
}

impl ProtectedReceiptSignerPeerIdentityV1 {
    /// Constructs one positive, dedicated non-root signer identity.
    ///
    /// # Errors
    ///
    /// Returns a typed error for zero/root or Linux invalid-ID coordinates.
    pub const fn new(
        pid: u32,
        uid: u32,
        gid: u32,
    ) -> Result<Self, ProtectedReceiptSignerPeerIdentityErrorV1> {
        if pid == 0 {
            return Err(ProtectedReceiptSignerPeerIdentityErrorV1::InvalidPid);
        }
        if uid == 0 || uid == INVALID_LINUX_ID {
            return Err(ProtectedReceiptSignerPeerIdentityErrorV1::InvalidUid);
        }
        if gid == 0 || gid == INVALID_LINUX_ID {
            return Err(ProtectedReceiptSignerPeerIdentityErrorV1::InvalidGid);
        }
        Ok(Self { pid, uid, gid })
    }

    /// Returns the pinned signer PID.
    pub const fn pid(self) -> u32 {
        self.pid
    }

    /// Returns the pinned signer UID.
    pub const fn uid(self) -> u32 {
        self.uid
    }

    /// Returns the pinned signer GID.
    pub const fn gid(self) -> u32 {
        self.gid
    }

    /// Peer credential metadata grants no signing or verifier authority.
    pub const fn grants_authority(self) -> bool {
        false
    }
}

/// Invalid protected signer peer identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedReceiptSignerPeerIdentityErrorV1 {
    /// PID was not positive.
    InvalidPid,
    /// UID was root or the Linux invalid-ID sentinel.
    InvalidUid,
    /// GID was root or the Linux invalid-ID sentinel.
    InvalidGid,
}

impl fmt::Display for ProtectedReceiptSignerPeerIdentityErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid protected signer peer identity: {self:?}"
        )
    }
}

impl Error for ProtectedReceiptSignerPeerIdentityErrorV1 {}

/// Exact fixed-width request sent to the protected signer.
#[derive(Clone, Eq, PartialEq)]
pub struct ProtectedReceiptSignerRequestV1 {
    request_identity: [u8; IDENTITY_BYTES],
    policy_identity: [u8; IDENTITY_BYTES],
    provider_identity: [u8; IDENTITY_BYTES],
    verifying_key: [u8; IDENTITY_BYTES],
    signing_input_sha256: [u8; IDENTITY_BYTES],
    signing_input: [u8; M1_ALL_KERNELS_PROTECTED_RECEIPT_SIGNING_BYTES_V1],
    canonical_bytes: [u8; PREOPENED_PROTECTED_RECEIPT_SIGNER_REQUEST_BYTES_V1],
}

impl ProtectedReceiptSignerRequestV1 {
    /// Constructs one request from the complete domain-separated signing input.
    ///
    /// # Errors
    ///
    /// Returns a typed error for an incorrect signing length/domain or zero
    /// policy, provider, or public-key coordinate.
    pub fn new(
        signing_input: &[u8],
        policy_identity: [u8; IDENTITY_BYTES],
        provider_identity: [u8; IDENTITY_BYTES],
        verifying_key: [u8; IDENTITY_BYTES],
    ) -> Result<Self, ProtectedReceiptSignerProtocolErrorV1> {
        let signing_input: [u8; M1_ALL_KERNELS_PROTECTED_RECEIPT_SIGNING_BYTES_V1] =
            signing_input.try_into().map_err(|_| {
                ProtectedReceiptSignerProtocolErrorV1::SigningInputLength {
                    expected: M1_ALL_KERNELS_PROTECTED_RECEIPT_SIGNING_BYTES_V1,
                    actual: signing_input.len(),
                }
            })?;
        if !signing_input.starts_with(M1_ALL_KERNELS_PROTECTED_RECEIPT_SIGNING_DOMAIN_V1) {
            return Err(ProtectedReceiptSignerProtocolErrorV1::SigningInputDomain);
        }
        validate_nonzero_coordinates(policy_identity, provider_identity, verifying_key)?;
        let signing_input_sha256 = sha256(&signing_input);
        let request_identity = request_identity(
            policy_identity,
            provider_identity,
            verifying_key,
            signing_input_sha256,
            &signing_input,
        );
        let canonical_bytes = encode_request(
            request_identity,
            policy_identity,
            provider_identity,
            verifying_key,
            signing_input_sha256,
            &signing_input,
        );
        Ok(Self {
            request_identity,
            policy_identity,
            provider_identity,
            verifying_key,
            signing_input_sha256,
            signing_input,
            canonical_bytes,
        })
    }

    /// Strictly decodes one complete canonical request packet.
    ///
    /// # Errors
    ///
    /// Returns a typed error for any length, header, coordinate, digest, or
    /// canonical-identity mismatch.
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self, ProtectedReceiptSignerProtocolErrorV1> {
        if bytes.len() != PREOPENED_PROTECTED_RECEIPT_SIGNER_REQUEST_BYTES_V1 {
            return Err(ProtectedReceiptSignerProtocolErrorV1::RequestLength {
                expected: PREOPENED_PROTECTED_RECEIPT_SIGNER_REQUEST_BYTES_V1,
                actual: bytes.len(),
            });
        }
        let mut reader = ReaderV1::new(bytes);
        if reader.array::<8>()? != MAGIC_REQUEST_V1
            || reader.u16()? != VERSION_V1
            || reader.u16()? != 0
            || reader.u32()? != 0
            || reader.u32()? as usize != PREOPENED_PROTECTED_RECEIPT_SIGNER_REQUEST_BYTES_V1
        {
            return Err(ProtectedReceiptSignerProtocolErrorV1::RequestHeader);
        }
        let encoded_request_identity = reader.array()?;
        let policy_identity = reader.array()?;
        let provider_identity = reader.array()?;
        let verifying_key = reader.array()?;
        let encoded_signing_input_sha256 = reader.array()?;
        let signing_input: [u8; M1_ALL_KERNELS_PROTECTED_RECEIPT_SIGNING_BYTES_V1] =
            reader.array()?;
        if !reader.is_finished() {
            return Err(ProtectedReceiptSignerProtocolErrorV1::TrailingBytes);
        }
        let request = Self::new(
            &signing_input,
            policy_identity,
            provider_identity,
            verifying_key,
        )?;
        if request.signing_input_sha256 != encoded_signing_input_sha256 {
            return Err(ProtectedReceiptSignerProtocolErrorV1::SigningInputDigest);
        }
        if request.request_identity != encoded_request_identity {
            return Err(ProtectedReceiptSignerProtocolErrorV1::RequestIdentity);
        }
        if request.canonical_bytes.as_slice() != bytes {
            return Err(ProtectedReceiptSignerProtocolErrorV1::NoncanonicalRequest);
        }
        Ok(request)
    }

    /// Returns the domain-separated request identity.
    pub const fn request_identity(&self) -> [u8; IDENTITY_BYTES] {
        self.request_identity
    }

    /// Returns the exact trust-policy identity.
    pub const fn policy_identity(&self) -> [u8; IDENTITY_BYTES] {
        self.policy_identity
    }

    /// Returns the supervisor-pinned signer provider identity.
    pub const fn provider_identity(&self) -> [u8; IDENTITY_BYTES] {
        self.provider_identity
    }

    /// Returns the expected signer public key.
    pub const fn verifying_key(&self) -> [u8; IDENTITY_BYTES] {
        self.verifying_key
    }

    /// Returns the signing-input digest bound into the request identity.
    pub const fn signing_input_sha256(&self) -> [u8; IDENTITY_BYTES] {
        self.signing_input_sha256
    }

    /// Borrows the complete domain-separated signing input.
    pub const fn signing_input(&self) -> &[u8; M1_ALL_KERNELS_PROTECTED_RECEIPT_SIGNING_BYTES_V1] {
        &self.signing_input
    }

    /// Borrows the exact canonical packet bytes.
    pub const fn encode_canonical(
        &self,
    ) -> &[u8; PREOPENED_PROTECTED_RECEIPT_SIGNER_REQUEST_BYTES_V1] {
        &self.canonical_bytes
    }

    /// Request framing alone grants no signing or verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

impl fmt::Debug for ProtectedReceiptSignerRequestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProtectedReceiptSignerRequestV1")
            .field("request_identity", &self.request_identity)
            .field("policy_identity", &self.policy_identity)
            .field("provider_identity", &self.provider_identity)
            .field("signing_input_sha256", &self.signing_input_sha256)
            .field("signing_input", &"redacted")
            .field("authority", &"none")
            .finish_non_exhaustive()
    }
}

/// Terminal status of one correlated signer response.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedReceiptSignerResponseStatusV1 {
    /// The response contains one candidate Ed25519 signature.
    Signed,
    /// The external signer explicitly rejected the correlated request.
    Rejected,
}

impl ProtectedReceiptSignerResponseStatusV1 {
    const fn code(self) -> u16 {
        match self {
            Self::Signed => 1,
            Self::Rejected => 2,
        }
    }

    fn decode(code: u16) -> Result<Self, ProtectedReceiptSignerProtocolErrorV1> {
        match code {
            1 => Ok(Self::Signed),
            2 => Ok(Self::Rejected),
            actual => Err(ProtectedReceiptSignerProtocolErrorV1::ResponseStatus { actual }),
        }
    }
}

/// Exact fixed-width terminal response from the protected signer.
#[derive(Clone, Eq, PartialEq)]
pub struct ProtectedReceiptSignerResponseV1 {
    status: ProtectedReceiptSignerResponseStatusV1,
    response_identity: [u8; IDENTITY_BYTES],
    request_identity: [u8; IDENTITY_BYTES],
    policy_identity: [u8; IDENTITY_BYTES],
    provider_identity: [u8; IDENTITY_BYTES],
    verifying_key: [u8; IDENTITY_BYTES],
    signing_input_sha256: [u8; IDENTITY_BYTES],
    signature: [u8; SIGNATURE_BYTES],
    canonical_bytes: [u8; PREOPENED_PROTECTED_RECEIPT_SIGNER_RESPONSE_BYTES_V1],
}

impl ProtectedReceiptSignerResponseV1 {
    /// Constructs one signed terminal response bound to `request`.
    pub fn signed(request: &ProtectedReceiptSignerRequestV1, signature: [u8; 64]) -> Self {
        Self::new(
            request,
            ProtectedReceiptSignerResponseStatusV1::Signed,
            signature,
        )
    }

    /// Constructs one canonical explicit rejection bound to `request`.
    pub fn rejected(request: &ProtectedReceiptSignerRequestV1) -> Self {
        Self::new(
            request,
            ProtectedReceiptSignerResponseStatusV1::Rejected,
            [0; SIGNATURE_BYTES],
        )
    }

    fn new(
        request: &ProtectedReceiptSignerRequestV1,
        status: ProtectedReceiptSignerResponseStatusV1,
        signature: [u8; SIGNATURE_BYTES],
    ) -> Self {
        let response_identity = response_identity(
            status,
            request.request_identity,
            request.policy_identity,
            request.provider_identity,
            request.verifying_key,
            request.signing_input_sha256,
            signature,
        );
        let canonical_bytes = encode_response(
            status,
            response_identity,
            request.request_identity,
            request.policy_identity,
            request.provider_identity,
            request.verifying_key,
            request.signing_input_sha256,
            signature,
        );
        Self {
            status,
            response_identity,
            request_identity: request.request_identity,
            policy_identity: request.policy_identity,
            provider_identity: request.provider_identity,
            verifying_key: request.verifying_key,
            signing_input_sha256: request.signing_input_sha256,
            signature,
            canonical_bytes,
        }
    }

    /// Strictly decodes one complete canonical response packet.
    ///
    /// # Errors
    ///
    /// Returns a typed error for any length, header, status, coordinate, or
    /// canonical response-identity mismatch.
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self, ProtectedReceiptSignerProtocolErrorV1> {
        if bytes.len() != PREOPENED_PROTECTED_RECEIPT_SIGNER_RESPONSE_BYTES_V1 {
            return Err(ProtectedReceiptSignerProtocolErrorV1::ResponseLength {
                expected: PREOPENED_PROTECTED_RECEIPT_SIGNER_RESPONSE_BYTES_V1,
                actual: bytes.len(),
            });
        }
        let mut reader = ReaderV1::new(bytes);
        if reader.array::<8>()? != MAGIC_RESPONSE_V1 || reader.u16()? != VERSION_V1 {
            return Err(ProtectedReceiptSignerProtocolErrorV1::ResponseHeader);
        }
        let status = ProtectedReceiptSignerResponseStatusV1::decode(reader.u16()?)?;
        if reader.u16()? != 0
            || reader.u16()? != 0
            || reader.u32()? as usize != PREOPENED_PROTECTED_RECEIPT_SIGNER_RESPONSE_BYTES_V1
        {
            return Err(ProtectedReceiptSignerProtocolErrorV1::ResponseHeader);
        }
        let encoded_response_identity = reader.array()?;
        let request_identity = reader.array()?;
        let policy_identity = reader.array()?;
        let provider_identity = reader.array()?;
        let verifying_key = reader.array()?;
        let signing_input_sha256 = reader.array()?;
        let signature = reader.array()?;
        if !reader.is_finished() {
            return Err(ProtectedReceiptSignerProtocolErrorV1::TrailingBytes);
        }
        if status == ProtectedReceiptSignerResponseStatusV1::Rejected
            && signature != [0; SIGNATURE_BYTES]
        {
            return Err(ProtectedReceiptSignerProtocolErrorV1::RejectedSignature);
        }
        let expected_response_identity = response_identity(
            status,
            request_identity,
            policy_identity,
            provider_identity,
            verifying_key,
            signing_input_sha256,
            signature,
        );
        if encoded_response_identity != expected_response_identity {
            return Err(ProtectedReceiptSignerProtocolErrorV1::ResponseIdentity);
        }
        let response = Self {
            status,
            response_identity: expected_response_identity,
            request_identity,
            policy_identity,
            provider_identity,
            verifying_key,
            signing_input_sha256,
            signature,
            canonical_bytes: encode_response(
                status,
                expected_response_identity,
                request_identity,
                policy_identity,
                provider_identity,
                verifying_key,
                signing_input_sha256,
                signature,
            ),
        };
        if response.canonical_bytes.as_slice() != bytes {
            return Err(ProtectedReceiptSignerProtocolErrorV1::NoncanonicalResponse);
        }
        Ok(response)
    }

    /// Returns the terminal response status.
    pub const fn status(&self) -> ProtectedReceiptSignerResponseStatusV1 {
        self.status
    }

    /// Returns the identity binding response status and all returned fields.
    pub const fn response_identity(&self) -> [u8; IDENTITY_BYTES] {
        self.response_identity
    }

    /// Returns the echoed request identity.
    pub const fn request_identity(&self) -> [u8; IDENTITY_BYTES] {
        self.request_identity
    }

    /// Returns the echoed trust-policy identity.
    pub const fn policy_identity(&self) -> [u8; IDENTITY_BYTES] {
        self.policy_identity
    }

    /// Returns the echoed signer provider identity.
    pub const fn provider_identity(&self) -> [u8; IDENTITY_BYTES] {
        self.provider_identity
    }

    /// Returns the echoed signer public key.
    pub const fn verifying_key(&self) -> [u8; IDENTITY_BYTES] {
        self.verifying_key
    }

    /// Returns the echoed signing-input digest.
    pub const fn signing_input_sha256(&self) -> [u8; IDENTITY_BYTES] {
        self.signing_input_sha256
    }

    /// Returns the candidate signature, or zeroes for a canonical rejection.
    pub const fn signature(&self) -> [u8; SIGNATURE_BYTES] {
        self.signature
    }

    /// Borrows the exact canonical response packet.
    pub const fn encode_canonical(
        &self,
    ) -> &[u8; PREOPENED_PROTECTED_RECEIPT_SIGNER_RESPONSE_BYTES_V1] {
        &self.canonical_bytes
    }

    /// Returns whether every echoed coordinate names `request` exactly.
    pub fn matches_request(&self, request: &ProtectedReceiptSignerRequestV1) -> bool {
        self.request_identity == request.request_identity
            && self.policy_identity == request.policy_identity
            && self.provider_identity == request.provider_identity
            && self.verifying_key == request.verifying_key
            && self.signing_input_sha256 == request.signing_input_sha256
    }

    /// Response framing alone grants no signing or verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

impl fmt::Debug for ProtectedReceiptSignerResponseV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProtectedReceiptSignerResponseV1")
            .field("status", &self.status)
            .field("response_identity", &self.response_identity)
            .field("request_identity", &self.request_identity)
            .field("provider_identity", &self.provider_identity)
            .field("signature", &"redacted")
            .field("authority", &"none")
            .finish_non_exhaustive()
    }
}

/// Canonical signer protocol failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedReceiptSignerProtocolErrorV1 {
    /// Signing input did not have the sole exact length.
    SigningInputLength {
        /// Required length.
        expected: usize,
        /// Observed length.
        actual: usize,
    },
    /// Signing input did not start with Ferric's exact receipt-signature domain.
    SigningInputDomain,
    /// Trust-policy identity was zero.
    ZeroPolicyIdentity,
    /// Signer provider identity was zero.
    ZeroProviderIdentity,
    /// Signer public key was zero.
    ZeroVerifyingKey,
    /// Request packet length was not exact.
    RequestLength {
        /// Required length.
        expected: usize,
        /// Observed length.
        actual: usize,
    },
    /// Request header, version, flags, reserved bytes, or declared length differed.
    RequestHeader,
    /// Request's signing-input digest differed.
    SigningInputDigest,
    /// Request identity did not bind the complete canonical signing input.
    RequestIdentity,
    /// Request did not exactly round-trip through the canonical codec.
    NoncanonicalRequest,
    /// Response packet length was not exact.
    ResponseLength {
        /// Required length.
        expected: usize,
        /// Observed length.
        actual: usize,
    },
    /// Response header, version, flags, reserved bytes, or declared length differed.
    ResponseHeader,
    /// Response status was not supported.
    ResponseStatus {
        /// Unsupported status code.
        actual: u16,
    },
    /// A rejected response carried nonzero signature bytes.
    RejectedSignature,
    /// Response identity did not bind its status and complete response coordinates.
    ResponseIdentity,
    /// Response did not exactly round-trip through the canonical codec.
    NoncanonicalResponse,
    /// Fixed-width decoding unexpectedly exhausted its input.
    Truncated,
    /// Fixed-width decoding left trailing input.
    TrailingBytes,
}

impl fmt::Display for ProtectedReceiptSignerProtocolErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "protected receipt-signer protocol rejected: {self:?}"
        )
    }
}

impl Error for ProtectedReceiptSignerProtocolErrorV1 {}

/// Descriptor or identity rejection before any signer protocol byte is sent.
#[derive(Debug)]
#[non_exhaustive]
pub enum ProtectedReceiptSignerClientAdmissionErrorV1 {
    /// Expected trust-policy identity was zero.
    ZeroPolicyIdentity,
    /// Expected provider identity was zero.
    ZeroProviderIdentity,
    /// Expected public key was zero.
    ZeroVerifyingKey,
    /// Descriptor inspection failed.
    Descriptor(io::Error),
    /// A fixed-width socket option returned an unexpected scalar length.
    SocketOptionLength {
        /// Required scalar byte length.
        expected: usize,
        /// Observed scalar byte length.
        actual: usize,
    },
    /// Descriptor was not a read-write, nonblocking, close-on-exec socket.
    DescriptorShape,
    /// Socket object differed from the supervisor-pinned device/inode.
    EndpointIdentityMismatch,
    /// Socket was not a connected, non-listening Unix `SOCK_SEQPACKET`.
    SocketShape,
    /// Socket reported a pending asynchronous error.
    PendingSocketError {
        /// Raw pending errno.
        raw: i32,
    },
    /// `SO_PEERCRED` inspection failed.
    PeerCredentials(io::Error),
    /// Kernel peer credentials were invalid.
    InvalidPeerCredentials,
    /// Kernel peer credentials differed from the supervisor-pinned tuple.
    PeerIdentityMismatch,
    /// Production signer and verifier service used the same effective UID.
    SignerAndVerifierUidMatch,
}

impl fmt::Display for ProtectedReceiptSignerClientAdmissionErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "protected receipt-signer endpoint rejected: {self:?}"
        )
    }
}

impl Error for ProtectedReceiptSignerClientAdmissionErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Descriptor(source) | Self::PeerCredentials(source) => Some(source),
            _ => None,
        }
    }
}

/// Ownership-retaining signer endpoint admission failure.
///
/// No protocol byte has been sent when this value is returned.
pub struct ProtectedReceiptSignerClientAdmissionFailureV1 {
    error: ProtectedReceiptSignerClientAdmissionErrorV1,
    peer: OwnedFd,
}

impl ProtectedReceiptSignerClientAdmissionFailureV1 {
    /// Returns the stable admission error while retaining the endpoint.
    pub const fn error(&self) -> &ProtectedReceiptSignerClientAdmissionErrorV1 {
        &self.error
    }

    /// Returns the exact caller-owned endpoint.
    pub fn into_peer(self) -> OwnedFd {
        self.peer
    }

    /// Returns the error and exact caller-owned endpoint.
    pub fn into_parts(self) -> (ProtectedReceiptSignerClientAdmissionErrorV1, OwnedFd) {
        (self.error, self.peer)
    }
}

impl fmt::Debug for ProtectedReceiptSignerClientAdmissionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProtectedReceiptSignerClientAdmissionFailureV1")
            .field("error", &self.error)
            .field("retains_peer", &true)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for ProtectedReceiptSignerClientAdmissionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl Error for ProtectedReceiptSignerClientAdmissionFailureV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error.source()
    }
}

/// Concrete descriptor-only provider for an external protected signer.
///
/// The provider is move-only. A poisoned provider retains no endpoint and can
/// only return [`ProtectedReceiptSignerClientErrorV1::Poisoned`]. Even a valid
/// response remains a candidate signature: the enclosing verifier service
/// independently authenticates it under the caller-provisioned trust policy.
pub struct PreopenedProtectedReceiptSignerClientV1 {
    peer: Option<OwnedFd>,
    endpoint: ProtectedReceiptSignerEndpointIdentityV1,
    expected_peer: ProtectedReceiptSignerPeerIdentityV1,
    policy_identity: [u8; IDENTITY_BYTES],
    provider_identity: [u8; IDENTITY_BYTES],
    verifying_key: [u8; IDENTITY_BYTES],
}

impl PreopenedProtectedReceiptSignerClientV1 {
    /// Admits one exact supervisor-preopened protected signer endpoint.
    ///
    /// Production admission requires an independently running signer under a
    /// dedicated non-root UID different from this verifier process. The endpoint
    /// must already be connected and configured nonblocking and close-on-exec.
    /// This function performs no path discovery or connection.
    ///
    /// # Errors
    ///
    /// Returns an ownership-retaining failure before sending any bytes.
    pub fn admit(
        peer: OwnedFd,
        endpoint: ProtectedReceiptSignerEndpointIdentityV1,
        expected_peer: ProtectedReceiptSignerPeerIdentityV1,
        policy_identity: [u8; IDENTITY_BYTES],
        provider_identity: [u8; IDENTITY_BYTES],
        verifying_key: [u8; IDENTITY_BYTES],
    ) -> Result<Self, ProtectedReceiptSignerClientAdmissionFailureV1> {
        Self::admit_inner::<true>(
            peer,
            endpoint,
            expected_peer,
            policy_identity,
            provider_identity,
            verifying_key,
        )
    }

    fn admit_inner<const REQUIRE_DISTINCT_UID: bool>(
        peer: OwnedFd,
        endpoint: ProtectedReceiptSignerEndpointIdentityV1,
        expected_peer: ProtectedReceiptSignerPeerIdentityV1,
        policy_identity: [u8; IDENTITY_BYTES],
        provider_identity: [u8; IDENTITY_BYTES],
        verifying_key: [u8; IDENTITY_BYTES],
    ) -> Result<Self, ProtectedReceiptSignerClientAdmissionFailureV1> {
        let admission =
            validate_admission_coordinates(policy_identity, provider_identity, verifying_key)
                .and_then(|()| {
                    validate_endpoint::<REQUIRE_DISTINCT_UID>(&peer, endpoint, expected_peer)
                });
        if let Err(error) = admission {
            return Err(ProtectedReceiptSignerClientAdmissionFailureV1 { error, peer });
        }
        Ok(Self {
            peer: Some(peer),
            endpoint,
            expected_peer,
            policy_identity,
            provider_identity,
            verifying_key,
        })
    }

    /// Returns whether an ambiguous exchange permanently dropped the endpoint.
    pub const fn is_poisoned(&self) -> bool {
        self.peer.is_none()
    }

    /// Descriptor admission and protocol framing grant no signing or verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }

    fn sign(
        &mut self,
        input: ProtectedReceiptSignerInputV1<'_>,
    ) -> Result<[u8; SIGNATURE_BYTES], ProtectedReceiptSignerClientErrorV1> {
        self.sign_inner::<true>(input)
    }

    fn sign_inner<const REQUIRE_DISTINCT_UID: bool>(
        &mut self,
        input: ProtectedReceiptSignerInputV1<'_>,
    ) -> Result<[u8; SIGNATURE_BYTES], ProtectedReceiptSignerClientErrorV1> {
        if input.policy_identity() != self.policy_identity {
            return Err(ProtectedReceiptSignerClientErrorV1::PolicyIdentityMismatch);
        }
        if input.deadline().remaining().is_none() {
            return Err(ProtectedReceiptSignerClientErrorV1::DeadlineExpired);
        }
        let request = ProtectedReceiptSignerRequestV1::new(
            input.signing_bytes(),
            input.policy_identity(),
            self.provider_identity,
            self.verifying_key,
        )
        .map_err(ProtectedReceiptSignerClientErrorV1::Protocol)?;
        let Some(peer) = self.peer.take() else {
            return Err(ProtectedReceiptSignerClientErrorV1::Poisoned);
        };
        if let Err(error) =
            validate_endpoint::<REQUIRE_DISTINCT_UID>(&peer, self.endpoint, self.expected_peer)
        {
            return Err(ProtectedReceiptSignerClientErrorV1::EndpointRevalidation(
                error,
            ));
        }
        match send_packet(&peer, request.encode_canonical(), input.deadline()) {
            Ok(()) => {}
            Err(failure) => {
                if !failure.poison {
                    self.peer = Some(peer);
                }
                return Err(failure.error);
            }
        }
        let response = receive_correlated_response::<REQUIRE_DISTINCT_UID>(
            &peer,
            self.endpoint,
            self.expected_peer,
            &request,
            input.deadline(),
        )?;
        self.peer = Some(peer);
        match response.status() {
            ProtectedReceiptSignerResponseStatusV1::Signed => Ok(response.signature()),
            ProtectedReceiptSignerResponseStatusV1::Rejected => {
                Err(ProtectedReceiptSignerClientErrorV1::SignerRejected)
            }
        }
    }
}

impl fmt::Debug for PreopenedProtectedReceiptSignerClientV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreopenedProtectedReceiptSignerClientV1")
            .field("available", &self.peer.is_some())
            .field("endpoint", &self.endpoint)
            .field("expected_peer", &self.expected_peer)
            .field("policy_identity", &self.policy_identity)
            .field("provider_identity", &self.provider_identity)
            .field("verifying_key", &"public key pinned")
            .field("authority", &"none")
            .finish_non_exhaustive()
    }
}

impl ProtectedReceiptSignerProviderV1 for PreopenedProtectedReceiptSignerClientV1 {
    type Error = ProtectedReceiptSignerClientErrorV1;

    fn verifying_key_bytes(&self) -> [u8; 32] {
        self.verifying_key
    }

    fn provider_identity(&self) -> [u8; 32] {
        self.provider_identity
    }

    fn sign_receipt(
        &mut self,
        input: ProtectedReceiptSignerInputV1<'_>,
    ) -> Result<[u8; 64], Self::Error> {
        self.sign(input)
    }
}

/// Bounded signer-client admission, transport, or correlation failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum ProtectedReceiptSignerClientErrorV1 {
    /// A prior ambiguous exchange permanently dropped the endpoint.
    Poisoned,
    /// The service supplied a trust-policy identity different from admission.
    PolicyIdentityMismatch,
    /// Fixed signer protocol construction or decoding failed.
    Protocol(ProtectedReceiptSignerProtocolErrorV1),
    /// Endpoint identity or peer credentials changed before or during exchange.
    EndpointRevalidation(ProtectedReceiptSignerClientAdmissionErrorV1),
    /// The sole absolute service deadline expired.
    DeadlineExpired,
    /// Polling the signer endpoint failed.
    Poll(io::Error),
    /// Sending the exact request packet failed.
    Send(io::Error),
    /// The atomic seqpacket send was partial.
    PartialSend,
    /// Receiving the response packet failed.
    Receive(io::Error),
    /// The signer closed before a terminal packet arrived.
    PeerClosed,
    /// The response exceeded the sole fixed packet bound.
    PacketTruncated,
    /// The signer attempted to transfer ancillary data.
    AncillaryData,
    /// The received response packet length was not exact.
    ResponseLength {
        /// Required response length.
        expected: usize,
        /// Observed response length.
        actual: usize,
    },
    /// Response coordinates did not correlate to the exact request.
    ResponseRequestMismatch,
    /// The signer returned a correlated explicit rejection.
    SignerRejected,
    /// Poll reported an invalid endpoint.
    InvalidPeer,
    /// Poll reported an asynchronous endpoint error.
    PeerFailed,
}

impl fmt::Display for ProtectedReceiptSignerClientErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "protected receipt-signer client failed: {self:?}"
        )
    }
}

impl Error for ProtectedReceiptSignerClientErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Protocol(source) => Some(source),
            Self::EndpointRevalidation(source) => Some(source),
            Self::Poll(source) | Self::Send(source) | Self::Receive(source) => Some(source),
            _ => None,
        }
    }
}

fn validate_nonzero_coordinates(
    policy_identity: [u8; IDENTITY_BYTES],
    provider_identity: [u8; IDENTITY_BYTES],
    verifying_key: [u8; IDENTITY_BYTES],
) -> Result<(), ProtectedReceiptSignerProtocolErrorV1> {
    if policy_identity == [0; IDENTITY_BYTES] {
        return Err(ProtectedReceiptSignerProtocolErrorV1::ZeroPolicyIdentity);
    }
    if provider_identity == [0; IDENTITY_BYTES] {
        return Err(ProtectedReceiptSignerProtocolErrorV1::ZeroProviderIdentity);
    }
    if verifying_key == [0; IDENTITY_BYTES] {
        return Err(ProtectedReceiptSignerProtocolErrorV1::ZeroVerifyingKey);
    }
    Ok(())
}

fn validate_admission_coordinates(
    policy_identity: [u8; IDENTITY_BYTES],
    provider_identity: [u8; IDENTITY_BYTES],
    verifying_key: [u8; IDENTITY_BYTES],
) -> Result<(), ProtectedReceiptSignerClientAdmissionErrorV1> {
    if policy_identity == [0; IDENTITY_BYTES] {
        return Err(ProtectedReceiptSignerClientAdmissionErrorV1::ZeroPolicyIdentity);
    }
    if provider_identity == [0; IDENTITY_BYTES] {
        return Err(ProtectedReceiptSignerClientAdmissionErrorV1::ZeroProviderIdentity);
    }
    if verifying_key == [0; IDENTITY_BYTES] {
        return Err(ProtectedReceiptSignerClientAdmissionErrorV1::ZeroVerifyingKey);
    }
    Ok(())
}

fn validate_endpoint<const REQUIRE_DISTINCT_UID: bool>(
    peer: &OwnedFd,
    endpoint: ProtectedReceiptSignerEndpointIdentityV1,
    expected_peer: ProtectedReceiptSignerPeerIdentityV1,
) -> Result<(), ProtectedReceiptSignerClientAdmissionErrorV1> {
    let stat = rustix::fs::fstat(peer).map_err(|source| {
        ProtectedReceiptSignerClientAdmissionErrorV1::Descriptor(source.into())
    })?;
    let descriptor_flags = rustix::io::fcntl_getfd(peer).map_err(|source| {
        ProtectedReceiptSignerClientAdmissionErrorV1::Descriptor(source.into())
    })?;
    let status = rustix::fs::fcntl_getfl(peer).map_err(|source| {
        ProtectedReceiptSignerClientAdmissionErrorV1::Descriptor(source.into())
    })?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::Socket
        || !descriptor_flags.contains(FdFlags::CLOEXEC)
        || !status.contains(OFlags::NONBLOCK)
        || status & OFlags::ACCMODE != OFlags::RDWR
        || status.intersects(OFlags::APPEND | OFlags::ASYNC | OFlags::DIRECT | OFlags::PATH)
    {
        return Err(ProtectedReceiptSignerClientAdmissionErrorV1::DescriptorShape);
    }
    if stat.st_dev != endpoint.device || stat.st_ino != endpoint.inode {
        return Err(ProtectedReceiptSignerClientAdmissionErrorV1::EndpointIdentityMismatch);
    }
    if socket_option(peer, libc::SO_DOMAIN)? != libc::AF_UNIX
        || socket_option(peer, libc::SO_TYPE)? != libc::SOCK_SEQPACKET
        || socket_option(peer, libc::SO_ACCEPTCONN)? != 0
        || !is_connected_unix(peer)?
    {
        return Err(ProtectedReceiptSignerClientAdmissionErrorV1::SocketShape);
    }
    let pending = socket_option(peer, libc::SO_ERROR)?;
    if pending != 0 {
        return Err(
            ProtectedReceiptSignerClientAdmissionErrorV1::PendingSocketError { raw: pending },
        );
    }
    let observed = peer_credentials(peer)?;
    if observed != expected_peer {
        return Err(ProtectedReceiptSignerClientAdmissionErrorV1::PeerIdentityMismatch);
    }
    if REQUIRE_DISTINCT_UID && observed.uid == rustix::process::geteuid().as_raw() {
        return Err(ProtectedReceiptSignerClientAdmissionErrorV1::SignerAndVerifierUidMatch);
    }
    Ok(())
}

fn socket_option(
    peer: &OwnedFd,
    option: i32,
) -> Result<i32, ProtectedReceiptSignerClientAdmissionErrorV1> {
    let mut value = 0_i32;
    let mut length = libc::socklen_t::try_from(mem::size_of::<i32>())
        .expect("socket option scalar length fits socklen_t");
    // SAFETY: output pointers name initialized scalar storage of the declared length.
    if unsafe {
        libc::getsockopt(
            peer.as_raw_fd(),
            libc::SOL_SOCKET,
            option,
            (&raw mut value).cast(),
            &raw mut length,
        )
    } != 0
    {
        return Err(ProtectedReceiptSignerClientAdmissionErrorV1::Descriptor(
            io::Error::last_os_error(),
        ));
    }
    let actual = usize::try_from(length).expect("socklen_t fits usize");
    if actual != mem::size_of::<i32>() {
        return Err(
            ProtectedReceiptSignerClientAdmissionErrorV1::SocketOptionLength {
                expected: mem::size_of::<i32>(),
                actual,
            },
        );
    }
    Ok(value)
}

fn is_connected_unix(peer: &OwnedFd) -> Result<bool, ProtectedReceiptSignerClientAdmissionErrorV1> {
    // SAFETY: zero initializes all sockaddr_storage bytes.
    let mut address = unsafe { mem::zeroed::<libc::sockaddr_storage>() };
    let mut length = libc::socklen_t::try_from(mem::size_of::<libc::sockaddr_storage>())
        .expect("socket address length fits socklen_t");
    // SAFETY: address and length name writable storage of the declared capacity.
    let result =
        unsafe { libc::getpeername(peer.as_raw_fd(), (&raw mut address).cast(), &raw mut length) };
    if result != 0 {
        let source = io::Error::last_os_error();
        if source.raw_os_error() == Some(libc::ENOTCONN) {
            return Ok(false);
        }
        return Err(ProtectedReceiptSignerClientAdmissionErrorV1::Descriptor(
            source,
        ));
    }
    Ok(i32::from(address.ss_family) == libc::AF_UNIX
        && usize::try_from(length).ok().is_some_and(|value| {
            value >= mem::size_of::<libc::sa_family_t>()
                && value <= mem::size_of::<libc::sockaddr_un>()
        }))
}

fn peer_credentials(
    peer: &OwnedFd,
) -> Result<ProtectedReceiptSignerPeerIdentityV1, ProtectedReceiptSignerClientAdmissionErrorV1> {
    // SAFETY: zero initializes every scalar field of Linux ucred.
    let mut credentials = unsafe { mem::zeroed::<libc::ucred>() };
    let mut length = libc::socklen_t::try_from(mem::size_of::<libc::ucred>())
        .expect("peer credential length fits socklen_t");
    // SAFETY: output pointers name initialized ucred storage of the declared length.
    if unsafe {
        libc::getsockopt(
            peer.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&raw mut credentials).cast(),
            &raw mut length,
        )
    } != 0
    {
        return Err(
            ProtectedReceiptSignerClientAdmissionErrorV1::PeerCredentials(
                io::Error::last_os_error(),
            ),
        );
    }
    let pid = u32::try_from(credentials.pid)
        .ok()
        .filter(|value| *value != 0);
    if usize::try_from(length).ok() != Some(mem::size_of::<libc::ucred>())
        || pid.is_none()
        || credentials.uid == 0
        || credentials.uid == INVALID_LINUX_ID
        || credentials.gid == 0
        || credentials.gid == INVALID_LINUX_ID
    {
        return Err(ProtectedReceiptSignerClientAdmissionErrorV1::InvalidPeerCredentials);
    }
    Ok(ProtectedReceiptSignerPeerIdentityV1 {
        pid: pid.expect("positive peer PID checked"),
        uid: credentials.uid,
        gid: credentials.gid,
    })
}

struct SendPacketFailureV1 {
    error: ProtectedReceiptSignerClientErrorV1,
    poison: bool,
}

fn send_packet(
    peer: &OwnedFd,
    bytes: &[u8; PREOPENED_PROTECTED_RECEIPT_SIGNER_REQUEST_BYTES_V1],
    deadline: AbsoluteSessionDeadlineV1,
) -> Result<(), SendPacketFailureV1> {
    if let Err(error) = wait_for_peer(peer, libc::POLLOUT, deadline) {
        let poison = !matches!(error, ProtectedReceiptSignerClientErrorV1::DeadlineExpired);
        return Err(SendPacketFailureV1 { error, poison });
    }
    // SAFETY: bytes names the complete readable packet and peer remains uniquely owned by caller.
    let sent = unsafe {
        libc::send(
            peer.as_raw_fd(),
            bytes.as_ptr().cast(),
            bytes.len(),
            libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
        )
    };
    if sent < 0 {
        return Err(SendPacketFailureV1 {
            error: ProtectedReceiptSignerClientErrorV1::Send(io::Error::last_os_error()),
            poison: true,
        });
    }
    if usize::try_from(sent).ok() != Some(bytes.len()) {
        return Err(SendPacketFailureV1 {
            error: ProtectedReceiptSignerClientErrorV1::PartialSend,
            poison: true,
        });
    }
    Ok(())
}

fn receive_correlated_response<const REQUIRE_DISTINCT_UID: bool>(
    peer: &OwnedFd,
    endpoint: ProtectedReceiptSignerEndpointIdentityV1,
    expected_peer: ProtectedReceiptSignerPeerIdentityV1,
    request: &ProtectedReceiptSignerRequestV1,
    deadline: AbsoluteSessionDeadlineV1,
) -> Result<ProtectedReceiptSignerResponseV1, ProtectedReceiptSignerClientErrorV1> {
    validate_endpoint::<REQUIRE_DISTINCT_UID>(peer, endpoint, expected_peer)
        .map_err(ProtectedReceiptSignerClientErrorV1::EndpointRevalidation)?;
    let bytes = receive_packet(peer, deadline)?;
    validate_endpoint::<REQUIRE_DISTINCT_UID>(peer, endpoint, expected_peer)
        .map_err(ProtectedReceiptSignerClientErrorV1::EndpointRevalidation)?;
    if deadline.remaining().is_none() {
        return Err(ProtectedReceiptSignerClientErrorV1::DeadlineExpired);
    }
    let response = ProtectedReceiptSignerResponseV1::decode_canonical(&bytes)
        .map_err(ProtectedReceiptSignerClientErrorV1::Protocol)?;
    if !response.matches_request(request) {
        return Err(ProtectedReceiptSignerClientErrorV1::ResponseRequestMismatch);
    }
    Ok(response)
}

fn receive_packet(
    peer: &OwnedFd,
    deadline: AbsoluteSessionDeadlineV1,
) -> Result<
    [u8; PREOPENED_PROTECTED_RECEIPT_SIGNER_RESPONSE_BYTES_V1],
    ProtectedReceiptSignerClientErrorV1,
> {
    wait_for_peer(peer, libc::POLLIN, deadline)?;
    let mut bytes = [0_u8; PREOPENED_PROTECTED_RECEIPT_SIGNER_RESPONSE_BYTES_V1];
    let mut vector = libc::iovec {
        iov_base: bytes.as_mut_ptr().cast(),
        iov_len: bytes.len(),
    };
    // SAFETY: zero is a valid empty msghdr and the live iovec is installed below.
    let mut header = unsafe { mem::zeroed::<libc::msghdr>() };
    header.msg_iov = &raw mut vector;
    header.msg_iovlen = 1;
    // SAFETY: header names the complete output buffer and deliberately has no ancillary buffer.
    let received = unsafe {
        libc::recvmsg(
            peer.as_raw_fd(),
            &raw mut header,
            libc::MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC,
        )
    };
    if received < 0 {
        return Err(ProtectedReceiptSignerClientErrorV1::Receive(
            io::Error::last_os_error(),
        ));
    }
    if header.msg_flags & libc::MSG_CTRUNC != 0 || header.msg_controllen != 0 {
        return Err(ProtectedReceiptSignerClientErrorV1::AncillaryData);
    }
    if header.msg_flags & libc::MSG_TRUNC != 0 {
        return Err(ProtectedReceiptSignerClientErrorV1::PacketTruncated);
    }
    let received = usize::try_from(received)
        .map_err(|_| ProtectedReceiptSignerClientErrorV1::PacketTruncated)?;
    if received == 0 {
        return Err(ProtectedReceiptSignerClientErrorV1::PeerClosed);
    }
    if received != bytes.len() {
        return Err(ProtectedReceiptSignerClientErrorV1::ResponseLength {
            expected: bytes.len(),
            actual: received,
        });
    }
    Ok(bytes)
}

fn wait_for_peer(
    peer: &OwnedFd,
    wanted: i16,
    deadline: AbsoluteSessionDeadlineV1,
) -> Result<(), ProtectedReceiptSignerClientErrorV1> {
    loop {
        let remaining = deadline
            .remaining()
            .ok_or(ProtectedReceiptSignerClientErrorV1::DeadlineExpired)?;
        let mut descriptor = libc::pollfd {
            fd: peer.as_raw_fd(),
            events: wanted | libc::POLLERR | libc::POLLHUP,
            revents: 0,
        };
        // SAFETY: descriptor is a live one-element pollfd array for the complete call.
        let result =
            unsafe { libc::poll(&raw mut descriptor, 1, duration_to_poll_millis(remaining)) };
        if result < 0 {
            let source = io::Error::last_os_error();
            if source.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(ProtectedReceiptSignerClientErrorV1::Poll(source));
        }
        if result == 0 {
            continue;
        }
        if deadline.remaining().is_none() {
            return Err(ProtectedReceiptSignerClientErrorV1::DeadlineExpired);
        }
        if descriptor.revents & libc::POLLNVAL != 0 {
            return Err(ProtectedReceiptSignerClientErrorV1::InvalidPeer);
        }
        if descriptor.revents & wanted != 0 {
            return Ok(());
        }
        if descriptor.revents & libc::POLLERR != 0 {
            return Err(ProtectedReceiptSignerClientErrorV1::PeerFailed);
        }
        if descriptor.revents & libc::POLLHUP != 0 {
            return Err(ProtectedReceiptSignerClientErrorV1::PeerClosed);
        }
    }
}

fn duration_to_poll_millis(duration: Duration) -> i32 {
    let millis = duration.as_millis();
    let rounded = if duration.subsec_nanos().is_multiple_of(1_000_000) {
        millis
    } else {
        millis.saturating_add(1)
    };
    i32::try_from(rounded.clamp(1, u128::from(i32::MAX.unsigned_abs())))
        .expect("poll bound fits i32")
}

fn request_identity(
    policy_identity: [u8; IDENTITY_BYTES],
    provider_identity: [u8; IDENTITY_BYTES],
    verifying_key: [u8; IDENTITY_BYTES],
    signing_input_sha256: [u8; IDENTITY_BYTES],
    signing_input: &[u8; M1_ALL_KERNELS_PROTECTED_RECEIPT_SIGNING_BYTES_V1],
) -> [u8; IDENTITY_BYTES] {
    hash_parts(&[
        REQUEST_IDENTITY_DOMAIN_V1,
        &VERSION_V1.to_le_bytes(),
        &REQUEST_DECLARED_BYTES_V1.to_le_bytes(),
        &policy_identity,
        &provider_identity,
        &verifying_key,
        &signing_input_sha256,
        signing_input,
    ])
}

fn response_identity(
    status: ProtectedReceiptSignerResponseStatusV1,
    request_identity: [u8; IDENTITY_BYTES],
    policy_identity: [u8; IDENTITY_BYTES],
    provider_identity: [u8; IDENTITY_BYTES],
    verifying_key: [u8; IDENTITY_BYTES],
    signing_input_sha256: [u8; IDENTITY_BYTES],
    signature: [u8; SIGNATURE_BYTES],
) -> [u8; IDENTITY_BYTES] {
    hash_parts(&[
        RESPONSE_IDENTITY_DOMAIN_V1,
        &VERSION_V1.to_le_bytes(),
        &status.code().to_le_bytes(),
        &RESPONSE_DECLARED_BYTES_V1.to_le_bytes(),
        &request_identity,
        &policy_identity,
        &provider_identity,
        &verifying_key,
        &signing_input_sha256,
        &signature,
    ])
}

fn encode_request(
    request_identity: [u8; IDENTITY_BYTES],
    policy_identity: [u8; IDENTITY_BYTES],
    provider_identity: [u8; IDENTITY_BYTES],
    verifying_key: [u8; IDENTITY_BYTES],
    signing_input_sha256: [u8; IDENTITY_BYTES],
    signing_input: &[u8; M1_ALL_KERNELS_PROTECTED_RECEIPT_SIGNING_BYTES_V1],
) -> [u8; PREOPENED_PROTECTED_RECEIPT_SIGNER_REQUEST_BYTES_V1] {
    let mut bytes = [0_u8; PREOPENED_PROTECTED_RECEIPT_SIGNER_REQUEST_BYTES_V1];
    let mut offset = 0;
    put(&mut bytes, &mut offset, &MAGIC_REQUEST_V1);
    put(&mut bytes, &mut offset, &VERSION_V1.to_le_bytes());
    put(&mut bytes, &mut offset, &0_u16.to_le_bytes());
    put(&mut bytes, &mut offset, &0_u32.to_le_bytes());
    put(
        &mut bytes,
        &mut offset,
        &REQUEST_DECLARED_BYTES_V1.to_le_bytes(),
    );
    put(&mut bytes, &mut offset, &request_identity);
    put(&mut bytes, &mut offset, &policy_identity);
    put(&mut bytes, &mut offset, &provider_identity);
    put(&mut bytes, &mut offset, &verifying_key);
    put(&mut bytes, &mut offset, &signing_input_sha256);
    put(&mut bytes, &mut offset, signing_input);
    debug_assert_eq!(offset, bytes.len());
    bytes
}

#[allow(clippy::too_many_arguments)]
fn encode_response(
    status: ProtectedReceiptSignerResponseStatusV1,
    response_identity: [u8; IDENTITY_BYTES],
    request_identity: [u8; IDENTITY_BYTES],
    policy_identity: [u8; IDENTITY_BYTES],
    provider_identity: [u8; IDENTITY_BYTES],
    verifying_key: [u8; IDENTITY_BYTES],
    signing_input_sha256: [u8; IDENTITY_BYTES],
    signature: [u8; SIGNATURE_BYTES],
) -> [u8; PREOPENED_PROTECTED_RECEIPT_SIGNER_RESPONSE_BYTES_V1] {
    let mut bytes = [0_u8; PREOPENED_PROTECTED_RECEIPT_SIGNER_RESPONSE_BYTES_V1];
    let mut offset = 0;
    put(&mut bytes, &mut offset, &MAGIC_RESPONSE_V1);
    put(&mut bytes, &mut offset, &VERSION_V1.to_le_bytes());
    put(&mut bytes, &mut offset, &status.code().to_le_bytes());
    put(&mut bytes, &mut offset, &0_u16.to_le_bytes());
    put(&mut bytes, &mut offset, &0_u16.to_le_bytes());
    put(
        &mut bytes,
        &mut offset,
        &RESPONSE_DECLARED_BYTES_V1.to_le_bytes(),
    );
    put(&mut bytes, &mut offset, &response_identity);
    put(&mut bytes, &mut offset, &request_identity);
    put(&mut bytes, &mut offset, &policy_identity);
    put(&mut bytes, &mut offset, &provider_identity);
    put(&mut bytes, &mut offset, &verifying_key);
    put(&mut bytes, &mut offset, &signing_input_sha256);
    put(&mut bytes, &mut offset, &signature);
    debug_assert_eq!(offset, bytes.len());
    bytes
}

fn sha256(bytes: &[u8]) -> [u8; IDENTITY_BYTES] {
    Sha256::digest(bytes).into()
}

fn hash_parts(parts: &[&[u8]]) -> [u8; IDENTITY_BYTES] {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update(part);
    }
    digest.finalize().into()
}

fn put<const N: usize>(bytes: &mut [u8; N], offset: &mut usize, value: &[u8]) {
    let end = *offset + value.len();
    bytes[*offset..end].copy_from_slice(value);
    *offset = end;
}

struct ReaderV1<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ReaderV1<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], ProtectedReceiptSignerProtocolErrorV1> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(ProtectedReceiptSignerProtocolErrorV1::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(ProtectedReceiptSignerProtocolErrorV1::Truncated)?
            .try_into()
            .map_err(|_| ProtectedReceiptSignerProtocolErrorV1::Truncated)?;
        self.offset = end;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, ProtectedReceiptSignerProtocolErrorV1> {
        self.array().map(u16::from_le_bytes)
    }

    fn u32(&mut self) -> Result<u32, ProtectedReceiptSignerProtocolErrorV1> {
        self.array().map(u32::from_le_bytes)
    }

    const fn is_finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

#[cfg(test)]
impl PreopenedProtectedReceiptSignerClientV1 {
    fn admit_same_uid_for_test(
        peer: OwnedFd,
        endpoint: ProtectedReceiptSignerEndpointIdentityV1,
        expected_peer: ProtectedReceiptSignerPeerIdentityV1,
        policy_identity: [u8; IDENTITY_BYTES],
        provider_identity: [u8; IDENTITY_BYTES],
        verifying_key: [u8; IDENTITY_BYTES],
    ) -> Result<Self, ProtectedReceiptSignerClientAdmissionFailureV1> {
        Self::admit_inner::<false>(
            peer,
            endpoint,
            expected_peer,
            policy_identity,
            provider_identity,
            verifying_key,
        )
    }

    fn sign_same_uid_for_test(
        &mut self,
        input: ProtectedReceiptSignerInputV1<'_>,
    ) -> Result<[u8; SIGNATURE_BYTES], ProtectedReceiptSignerClientErrorV1> {
        self.sign_inner::<false>(input)
    }
}

#[cfg(test)]
mod tests {
    use std::os::fd::{AsRawFd, OwnedFd};
    use std::thread;

    use ed25519_dalek::{Signer, SigningKey};
    use rustix::net::{AddressFamily, SocketFlags, SocketType, socketpair};

    use super::*;

    const POLICY: [u8; 32] = [0x11; 32];
    const PROVIDER: [u8; 32] = [0x22; 32];

    fn signing_key() -> SigningKey {
        SigningKey::from_bytes(&[0x33; 32])
    }

    fn signing_input() -> Vec<u8> {
        let mut bytes = vec![0x5a; M1_ALL_KERNELS_PROTECTED_RECEIPT_SIGNING_BYTES_V1];
        bytes[..M1_ALL_KERNELS_PROTECTED_RECEIPT_SIGNING_DOMAIN_V1.len()]
            .copy_from_slice(M1_ALL_KERNELS_PROTECTED_RECEIPT_SIGNING_DOMAIN_V1);
        bytes
    }

    fn pair(socket_type: SocketType) -> (OwnedFd, OwnedFd) {
        socketpair(
            AddressFamily::UNIX,
            socket_type,
            SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
            None,
        )
        .unwrap()
    }

    fn endpoint_identity(peer: &OwnedFd) -> ProtectedReceiptSignerEndpointIdentityV1 {
        let stat = rustix::fs::fstat(peer).unwrap();
        ProtectedReceiptSignerEndpointIdentityV1::new(stat.st_dev, stat.st_ino).unwrap()
    }

    fn current_peer_identity(peer: &OwnedFd) -> ProtectedReceiptSignerPeerIdentityV1 {
        peer_credentials(peer).unwrap()
    }

    fn admit_test_client(
        peer: OwnedFd,
        policy: [u8; 32],
        provider: [u8; 32],
        verifying_key: [u8; 32],
    ) -> PreopenedProtectedReceiptSignerClientV1 {
        let endpoint = endpoint_identity(&peer);
        let expected_peer = current_peer_identity(&peer);
        PreopenedProtectedReceiptSignerClientV1::admit_same_uid_for_test(
            peer,
            endpoint,
            expected_peer,
            policy,
            provider,
            verifying_key,
        )
        .unwrap()
    }

    fn deadline(duration: Duration) -> AbsoluteSessionDeadlineV1 {
        AbsoluteSessionDeadlineV1::after(duration).unwrap()
    }

    fn signer_input(
        bytes: &[u8],
        policy: [u8; 32],
        duration: Duration,
    ) -> ProtectedReceiptSignerInputV1<'_> {
        ProtectedReceiptSignerInputV1::new(bytes, policy, deadline(duration))
    }

    fn receive_request(peer: &OwnedFd) -> ProtectedReceiptSignerRequestV1 {
        wait_for_peer(peer, libc::POLLIN, deadline(Duration::from_secs(2))).unwrap();
        let mut bytes = [0_u8; PREOPENED_PROTECTED_RECEIPT_SIGNER_REQUEST_BYTES_V1];
        // SAFETY: bytes names the complete writable request buffer and peer is live.
        let received = unsafe {
            libc::recv(
                peer.as_raw_fd(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
                libc::MSG_DONTWAIT,
            )
        };
        assert_eq!(usize::try_from(received).unwrap(), bytes.len());
        ProtectedReceiptSignerRequestV1::decode_canonical(&bytes).unwrap()
    }

    fn send_bytes(peer: &OwnedFd, bytes: &[u8]) {
        wait_for_peer(peer, libc::POLLOUT, deadline(Duration::from_secs(2))).unwrap();
        // SAFETY: bytes names the complete readable packet and peer is live.
        let sent = unsafe {
            libc::send(
                peer.as_raw_fd(),
                bytes.as_ptr().cast(),
                bytes.len(),
                libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
            )
        };
        assert_eq!(usize::try_from(sent).unwrap(), bytes.len());
    }

    fn spawn_response<F>(peer: OwnedFd, response: F) -> thread::JoinHandle<()>
    where
        F: FnOnce(&ProtectedReceiptSignerRequestV1) -> Vec<u8> + Send + 'static,
    {
        thread::spawn(move || {
            let request = receive_request(&peer);
            send_bytes(&peer, &response(&request));
        })
    }

    fn signed_response(request: &ProtectedReceiptSignerRequestV1) -> Vec<u8> {
        let signature = signing_key().sign(request.signing_input()).to_bytes();
        ProtectedReceiptSignerResponseV1::signed(request, signature)
            .encode_canonical()
            .to_vec()
    }

    #[test]
    fn fixed_protocol_round_trips_and_binds_complete_request_and_response_status() {
        let request = ProtectedReceiptSignerRequestV1::new(
            &signing_input(),
            POLICY,
            PROVIDER,
            signing_key().verifying_key().to_bytes(),
        )
        .unwrap();
        let decoded =
            ProtectedReceiptSignerRequestV1::decode_canonical(request.encode_canonical()).unwrap();
        assert_eq!(decoded, request);
        assert!(!decoded.grants_authority());

        let signed = ProtectedReceiptSignerResponseV1::signed(&request, [0; 64]);
        let rejected = ProtectedReceiptSignerResponseV1::rejected(&request);
        assert_ne!(signed.response_identity(), rejected.response_identity());
        assert_eq!(
            ProtectedReceiptSignerResponseV1::decode_canonical(signed.encode_canonical()).unwrap(),
            signed
        );
        assert_eq!(
            ProtectedReceiptSignerResponseV1::decode_canonical(rejected.encode_canonical())
                .unwrap(),
            rejected
        );
        assert!(!signed.grants_authority());

        let mut request_mutation = *request.encode_canonical();
        *request_mutation.last_mut().unwrap() ^= 1;
        assert!(matches!(
            ProtectedReceiptSignerRequestV1::decode_canonical(&request_mutation),
            Err(ProtectedReceiptSignerProtocolErrorV1::SigningInputDigest
                | ProtectedReceiptSignerProtocolErrorV1::RequestIdentity)
        ));
        let mut status_mutation = *signed.encode_canonical();
        status_mutation[10..12].copy_from_slice(
            &ProtectedReceiptSignerResponseStatusV1::Rejected
                .code()
                .to_le_bytes(),
        );
        assert!(matches!(
            ProtectedReceiptSignerResponseV1::decode_canonical(&status_mutation),
            Err(ProtectedReceiptSignerProtocolErrorV1::ResponseIdentity)
        ));
        assert!(matches!(
            ProtectedReceiptSignerRequestV1::decode_canonical(
                &request.encode_canonical()[..request.encode_canonical().len() - 1]
            ),
            Err(ProtectedReceiptSignerProtocolErrorV1::RequestLength { .. })
        ));
    }

    #[test]
    fn admission_rejects_substitution_and_returns_the_exact_owned_endpoint() {
        let (peer, _signer) = pair(SocketType::SEQPACKET);
        let raw = peer.as_raw_fd();
        let stat = rustix::fs::fstat(&peer).unwrap();
        let wrong_endpoint =
            ProtectedReceiptSignerEndpointIdentityV1::new(stat.st_dev, stat.st_ino + 1).unwrap();
        let expected_peer = current_peer_identity(&peer);
        let failure = PreopenedProtectedReceiptSignerClientV1::admit(
            peer,
            wrong_endpoint,
            expected_peer,
            POLICY,
            PROVIDER,
            signing_key().verifying_key().to_bytes(),
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedReceiptSignerClientAdmissionErrorV1::EndpointIdentityMismatch
        ));
        assert_eq!(failure.into_peer().as_raw_fd(), raw);

        for coordinate in 0..3 {
            let (peer, _signer) = pair(SocketType::SEQPACKET);
            let raw = peer.as_raw_fd();
            let endpoint = endpoint_identity(&peer);
            let mut wrong_peer = current_peer_identity(&peer);
            match coordinate {
                0 => wrong_peer.pid = wrong_peer.pid.saturating_add(1),
                1 => wrong_peer.uid = wrong_peer.uid.saturating_add(1),
                _ => wrong_peer.gid = wrong_peer.gid.saturating_add(1),
            }
            let failure = PreopenedProtectedReceiptSignerClientV1::admit(
                peer,
                endpoint,
                wrong_peer,
                POLICY,
                PROVIDER,
                signing_key().verifying_key().to_bytes(),
            )
            .unwrap_err();
            assert!(matches!(
                failure.error(),
                ProtectedReceiptSignerClientAdmissionErrorV1::PeerIdentityMismatch
            ));
            assert_eq!(failure.into_peer().as_raw_fd(), raw);
        }

        let (peer, _signer) = pair(SocketType::STREAM);
        let raw = peer.as_raw_fd();
        let endpoint = endpoint_identity(&peer);
        let expected_peer = current_peer_identity(&peer);
        let failure = PreopenedProtectedReceiptSignerClientV1::admit(
            peer,
            endpoint,
            expected_peer,
            POLICY,
            PROVIDER,
            signing_key().verifying_key().to_bytes(),
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedReceiptSignerClientAdmissionErrorV1::SocketShape
        ));
        assert_eq!(failure.into_peer().as_raw_fd(), raw);
    }

    #[test]
    fn production_admission_rejects_same_uid_and_test_only_client_signs() {
        let key = signing_key();
        let (peer, signer) = pair(SocketType::SEQPACKET);
        let endpoint = endpoint_identity(&peer);
        let expected_peer = current_peer_identity(&peer);
        let failure = PreopenedProtectedReceiptSignerClientV1::admit(
            peer,
            endpoint,
            expected_peer,
            POLICY,
            PROVIDER,
            key.verifying_key().to_bytes(),
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedReceiptSignerClientAdmissionErrorV1::SignerAndVerifierUidMatch
        ));
        drop(failure.into_peer());
        drop(signer);

        let (peer, signer) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client(peer, POLICY, PROVIDER, key.verifying_key().to_bytes());
        let server = spawn_response(signer, signed_response);
        let bytes = signing_input();
        let signature = client
            .sign_same_uid_for_test(signer_input(&bytes, POLICY, Duration::from_secs(2)))
            .unwrap();
        assert_eq!(signature, key.sign(&bytes).to_bytes());
        assert!(!client.is_poisoned());
        assert!(!client.grants_authority());
        server.join().unwrap();
    }

    #[test]
    fn pre_send_policy_rejection_retains_the_channel() {
        let key = signing_key();
        let (peer, signer) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client(peer, POLICY, PROVIDER, key.verifying_key().to_bytes());
        let server = spawn_response(signer, signed_response);
        let bytes = signing_input();
        assert!(matches!(
            client.sign_same_uid_for_test(signer_input(&bytes, POLICY, Duration::ZERO)),
            Err(ProtectedReceiptSignerClientErrorV1::DeadlineExpired)
        ));
        assert!(!client.is_poisoned());
        assert!(matches!(
            client.sign_same_uid_for_test(signer_input(&bytes, [0x44; 32], Duration::from_secs(2))),
            Err(ProtectedReceiptSignerClientErrorV1::PolicyIdentityMismatch)
        ));
        assert!(!client.is_poisoned());
        client
            .sign_same_uid_for_test(signer_input(&bytes, POLICY, Duration::from_secs(2)))
            .unwrap();
        assert!(!client.is_poisoned());
        server.join().unwrap();
    }

    #[derive(Clone, Copy)]
    enum Mismatch {
        Policy,
        Provider,
        Key,
        SigningInput,
    }

    #[test]
    fn every_valid_but_uncorrelated_response_coordinate_poisons() {
        for mismatch in [
            Mismatch::Policy,
            Mismatch::Provider,
            Mismatch::Key,
            Mismatch::SigningInput,
        ] {
            let key = signing_key();
            let (peer, signer) = pair(SocketType::SEQPACKET);
            let mut client =
                admit_test_client(peer, POLICY, PROVIDER, key.verifying_key().to_bytes());
            let server = spawn_response(signer, move |request| {
                let mut bytes = request.signing_input().to_vec();
                let mut policy = request.policy_identity();
                let mut provider = request.provider_identity();
                let mut verifying_key = request.verifying_key();
                match mismatch {
                    Mismatch::Policy => policy[0] ^= 1,
                    Mismatch::Provider => provider[0] ^= 1,
                    Mismatch::Key => verifying_key[0] ^= 1,
                    Mismatch::SigningInput => *bytes.last_mut().unwrap() ^= 1,
                }
                let other =
                    ProtectedReceiptSignerRequestV1::new(&bytes, policy, provider, verifying_key)
                        .unwrap();
                ProtectedReceiptSignerResponseV1::signed(&other, [0x55; 64])
                    .encode_canonical()
                    .to_vec()
            });
            let bytes = signing_input();
            assert!(matches!(
                client.sign_same_uid_for_test(signer_input(&bytes, POLICY, Duration::from_secs(2))),
                Err(ProtectedReceiptSignerClientErrorV1::ResponseRequestMismatch)
            ));
            assert!(client.is_poisoned());
            assert!(matches!(
                client.sign_same_uid_for_test(signer_input(&bytes, POLICY, Duration::from_secs(2))),
                Err(ProtectedReceiptSignerClientErrorV1::Poisoned)
            ));
            server.join().unwrap();
        }
    }

    #[test]
    fn short_oversize_and_malformed_responses_poison() {
        for response_kind in 0..3 {
            let key = signing_key();
            let (peer, signer) = pair(SocketType::SEQPACKET);
            let mut client =
                admit_test_client(peer, POLICY, PROVIDER, key.verifying_key().to_bytes());
            let server = spawn_response(signer, move |request| {
                let mut response = signed_response(request);
                match response_kind {
                    0 => {
                        response.pop();
                    }
                    1 => response.push(0),
                    _ => response[0] ^= 1,
                }
                response
            });
            let bytes = signing_input();
            let error = client
                .sign_same_uid_for_test(signer_input(&bytes, POLICY, Duration::from_secs(2)))
                .unwrap_err();
            match response_kind {
                0 => assert!(matches!(
                    error,
                    ProtectedReceiptSignerClientErrorV1::ResponseLength { .. }
                )),
                1 => assert!(matches!(
                    error,
                    ProtectedReceiptSignerClientErrorV1::PacketTruncated
                )),
                _ => assert!(matches!(
                    error,
                    ProtectedReceiptSignerClientErrorV1::Protocol(
                        ProtectedReceiptSignerProtocolErrorV1::ResponseHeader
                    )
                )),
            }
            assert!(client.is_poisoned());
            server.join().unwrap();
        }
    }

    fn send_ancillary(peer: &OwnedFd, bytes: &[u8]) {
        let mut vector = libc::iovec {
            iov_base: bytes.as_ptr().cast_mut().cast(),
            iov_len: bytes.len(),
        };
        let control_bytes = usize::try_from(unsafe {
            libc::CMSG_SPACE(u32::try_from(mem::size_of::<i32>()).unwrap())
        })
        .unwrap();
        assert!(control_bytes <= 2 * mem::size_of::<libc::cmsghdr>());
        // SAFETY: all-zero storage is valid backing memory for a cmsghdr control buffer.
        let mut control: [libc::cmsghdr; 2] = unsafe { mem::zeroed() };
        // SAFETY: zero initializes a valid msghdr and all pointers are installed below.
        let mut header = unsafe { mem::zeroed::<libc::msghdr>() };
        header.msg_iov = &raw mut vector;
        header.msg_iovlen = 1;
        header.msg_control = control.as_mut_ptr().cast();
        header.msg_controllen = control_bytes;
        // SAFETY: header has a valid control buffer with room for one descriptor.
        unsafe {
            let message = libc::CMSG_FIRSTHDR(&raw const header);
            assert!(!message.is_null());
            (*message).cmsg_level = libc::SOL_SOCKET;
            (*message).cmsg_type = libc::SCM_RIGHTS;
            (*message).cmsg_len = libc::CMSG_LEN(u32::try_from(mem::size_of::<i32>()).unwrap())
                .try_into()
                .unwrap();
            std::ptr::copy_nonoverlapping(
                peer.as_raw_fd().to_ne_bytes().as_ptr(),
                libc::CMSG_DATA(message),
                mem::size_of::<i32>(),
            );
        }
        wait_for_peer(peer, libc::POLLOUT, deadline(Duration::from_secs(2))).unwrap();
        // SAFETY: header retains live data and control buffers for the complete call.
        let sent = unsafe {
            libc::sendmsg(
                peer.as_raw_fd(),
                &raw const header,
                libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
            )
        };
        assert_eq!(usize::try_from(sent).unwrap(), bytes.len());
    }

    #[test]
    fn ancillary_response_is_rejected_and_poisons() {
        let key = signing_key();
        let (peer, signer) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client(peer, POLICY, PROVIDER, key.verifying_key().to_bytes());
        let server = thread::spawn(move || {
            let request = receive_request(&signer);
            let response = signed_response(&request);
            send_ancillary(&signer, &response);
        });
        let bytes = signing_input();
        assert!(matches!(
            client.sign_same_uid_for_test(signer_input(&bytes, POLICY, Duration::from_secs(2))),
            Err(ProtectedReceiptSignerClientErrorV1::AncillaryData)
        ));
        assert!(client.is_poisoned());
        server.join().unwrap();
    }

    #[test]
    fn timeout_and_peer_close_after_send_poison() {
        for close in [false, true] {
            let key = signing_key();
            let (peer, signer) = pair(SocketType::SEQPACKET);
            let mut client =
                admit_test_client(peer, POLICY, PROVIDER, key.verifying_key().to_bytes());
            let server = thread::spawn(move || {
                let _request = receive_request(&signer);
                if !close {
                    thread::sleep(Duration::from_millis(150));
                }
            });
            let bytes = signing_input();
            let error = client
                .sign_same_uid_for_test(signer_input(&bytes, POLICY, Duration::from_millis(50)))
                .unwrap_err();
            if close {
                assert!(matches!(
                    error,
                    ProtectedReceiptSignerClientErrorV1::PeerClosed
                        | ProtectedReceiptSignerClientErrorV1::Receive(_)
                ));
            } else {
                assert!(matches!(
                    error,
                    ProtectedReceiptSignerClientErrorV1::DeadlineExpired
                ));
            }
            assert!(client.is_poisoned());
            server.join().unwrap();
        }
    }

    #[test]
    fn correlated_explicit_rejection_keeps_channel_synchronized() {
        let key = signing_key();
        let (peer, signer) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client(peer, POLICY, PROVIDER, key.verifying_key().to_bytes());
        let server = thread::spawn(move || {
            let first = receive_request(&signer);
            send_bytes(
                &signer,
                ProtectedReceiptSignerResponseV1::rejected(&first).encode_canonical(),
            );
            let second = receive_request(&signer);
            send_bytes(&signer, &signed_response(&second));
        });
        let bytes = signing_input();
        assert!(matches!(
            client.sign_same_uid_for_test(signer_input(&bytes, POLICY, Duration::from_secs(2))),
            Err(ProtectedReceiptSignerClientErrorV1::SignerRejected)
        ));
        assert!(!client.is_poisoned());
        client
            .sign_same_uid_for_test(signer_input(&bytes, POLICY, Duration::from_secs(2)))
            .unwrap();
        assert!(!client.is_poisoned());
        server.join().unwrap();
    }
}
