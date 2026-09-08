//! Descriptor-only IPC client for an externally protected compiler-current authenticator.
//!
//! One client consumes one supervisor-preopened connected Unix `SOCK_SEQPACKET`
//! endpoint. Fixed canonical packets bind the complete Begin request, the exact
//! compiler receipt carriage selected from the decoded V2 envelope, the complete
//! current-record frame, the envelope digest and length, and every pinned
//! admission coordinate. This client does not implement the protected policy,
//! ledger lookup, external-currentness authority, daemon, or launcher.

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

use fe2o3_runtime_protocol::{
    COMPILER_EXECUTION_RECEIPT_CARRIAGE_BYTES_V1, CompilerExecutionReceiptCarriageV1,
    WorkerV3LoadEnvelopeWireV2,
};
use fe2o3_worker_v3_verification_protocol::{
    MAX_WORKER_V3_VERIFICATION_REQUEST_BYTES_V1, WORKER_V3_VERIFICATION_CURRENT_RECORD_BYTES_V2,
    WorkerV3VerificationCurrentRecordFrameV2, WorkerV3VerificationRequestV1,
};
use rustix::fs::{FileType, OFlags};
use rustix::io::FdFlags;
use sha2::{Digest, Sha256};

use crate::{
    AbsoluteSessionDeadlineV1, AuthenticatedCompilerCurrentRecordV1,
    ProtectedCompilerCurrentRecordInputV1, ProtectedCompilerCurrentRecordProviderV1,
};

const MAGIC_REQUEST_V1: [u8; 8] = *b"F3CRRQ1\0";
const MAGIC_RESPONSE_V1: [u8; 8] = *b"F3CRRS1\0";
const VERSION_V1: u16 = 1;
const AUTHENTICATE_OPERATION_V1: u16 = 1;
const HEADER_BYTES: usize = 24;
const IDENTITY_BYTES: usize = 32;
const INVALID_LINUX_ID: u32 = u32::MAX;
const CURRENT_RECORD_REQUEST_IDENTITY_OFFSET: usize = 24;
const REQUEST_IDENTITY_DOMAIN_V1: &[u8] = b"FERRIC/PROTECTED-COMPILER-CURRENT-RECORD/REQUEST/V1\0";
const RESPONSE_IDENTITY_DOMAIN_V1: &[u8] =
    b"FERRIC/PROTECTED-COMPILER-CURRENT-RECORD/RESPONSE/V1\0";

/// Exact byte length of one protected compiler-current request packet.
pub const PREOPENED_PROTECTED_COMPILER_CURRENT_REQUEST_BYTES_V1: usize = HEADER_BYTES
    + (10 * IDENTITY_BYTES)
    + 8
    + 4
    + 8
    + 4
    + MAX_WORKER_V3_VERIFICATION_REQUEST_BYTES_V1
    + COMPILER_EXECUTION_RECEIPT_CARRIAGE_BYTES_V1
    + WORKER_V3_VERIFICATION_CURRENT_RECORD_BYTES_V2;

/// Exact byte length of one protected compiler-current response packet.
pub const PREOPENED_PROTECTED_COMPILER_CURRENT_RESPONSE_BYTES_V1: usize =
    HEADER_BYTES + (12 * IDENTITY_BYTES) + 8;

/// Supervisor-pinned identity of one preopened current-record socket endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtectedCompilerCurrentEndpointIdentityV1 {
    device: u64,
    inode: u64,
}

impl ProtectedCompilerCurrentEndpointIdentityV1 {
    /// Constructs one nonzero supervisor-pinned socket identity.
    ///
    /// # Errors
    ///
    /// Returns a typed error if either coordinate is zero.
    pub const fn new(
        device: u64,
        inode: u64,
    ) -> Result<Self, ProtectedCompilerCurrentEndpointIdentityErrorV1> {
        if device == 0 {
            return Err(ProtectedCompilerCurrentEndpointIdentityErrorV1::ZeroDevice);
        }
        if inode == 0 {
            return Err(ProtectedCompilerCurrentEndpointIdentityErrorV1::ZeroInode);
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

    /// Endpoint metadata grants no compiler or verifier authority.
    pub const fn grants_authority(self) -> bool {
        false
    }
}

/// Invalid supervisor-pinned current-record endpoint identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedCompilerCurrentEndpointIdentityErrorV1 {
    /// Socket device number was zero.
    ZeroDevice,
    /// Socket inode number was zero.
    ZeroInode,
}

impl fmt::Display for ProtectedCompilerCurrentEndpointIdentityErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid protected compiler-current endpoint: {self:?}"
        )
    }
}

impl Error for ProtectedCompilerCurrentEndpointIdentityErrorV1 {}

/// Supervisor-pinned kernel credentials for the protected current-record peer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtectedCompilerCurrentPeerIdentityV1 {
    pid: u32,
    uid: u32,
    gid: u32,
}

impl ProtectedCompilerCurrentPeerIdentityV1 {
    /// Constructs one positive, dedicated non-root provider identity.
    ///
    /// # Errors
    ///
    /// Returns a typed error for zero/root or Linux invalid-ID coordinates.
    pub const fn new(
        pid: u32,
        uid: u32,
        gid: u32,
    ) -> Result<Self, ProtectedCompilerCurrentPeerIdentityErrorV1> {
        if pid == 0 {
            return Err(ProtectedCompilerCurrentPeerIdentityErrorV1::InvalidPid);
        }
        if uid == 0 || uid == INVALID_LINUX_ID {
            return Err(ProtectedCompilerCurrentPeerIdentityErrorV1::InvalidUid);
        }
        if gid == 0 || gid == INVALID_LINUX_ID {
            return Err(ProtectedCompilerCurrentPeerIdentityErrorV1::InvalidGid);
        }
        Ok(Self { pid, uid, gid })
    }

    /// Returns the pinned provider PID.
    pub const fn pid(self) -> u32 {
        self.pid
    }

    /// Returns the pinned provider UID.
    pub const fn uid(self) -> u32 {
        self.uid
    }

    /// Returns the pinned provider GID.
    pub const fn gid(self) -> u32 {
        self.gid
    }

    /// Credential metadata grants no compiler or verifier authority.
    pub const fn grants_authority(self) -> bool {
        false
    }
}

/// Invalid protected current-record peer identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedCompilerCurrentPeerIdentityErrorV1 {
    /// PID was not positive.
    InvalidPid,
    /// UID was root or the Linux invalid-ID sentinel.
    InvalidUid,
    /// GID was root or the Linux invalid-ID sentinel.
    InvalidGid,
}

impl fmt::Display for ProtectedCompilerCurrentPeerIdentityErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid protected compiler-current peer: {self:?}"
        )
    }
}

impl Error for ProtectedCompilerCurrentPeerIdentityErrorV1 {}

/// Exact fixed-width request sent to the protected current-record authenticator.
#[derive(Clone, Eq, PartialEq)]
pub struct ProtectedCompilerCurrentRequestV1 {
    request_identity: [u8; 32],
    protocol_identity: [u8; 32],
    provider_measurement: [u8; 32],
    compiler_policy_identity: [u8; 32],
    admission_session_identity: [u8; 32],
    request_sequence: u64,
    begin_request_identity: [u8; 32],
    begin_request_sha256: [u8; 32],
    envelope_sha256: [u8; 32],
    envelope_length: u64,
    carriage_identity: [u8; 32],
    current_record_sha256: [u8; 32],
    begin_request_length: u32,
    begin_request_bytes: Box<[u8; MAX_WORKER_V3_VERIFICATION_REQUEST_BYTES_V1]>,
    carriage_bytes: [u8; COMPILER_EXECUTION_RECEIPT_CARRIAGE_BYTES_V1],
    current_record_bytes: [u8; WORKER_V3_VERIFICATION_CURRENT_RECORD_BYTES_V2],
    canonical_bytes: Box<[u8; PREOPENED_PROTECTED_COMPILER_CURRENT_REQUEST_BYTES_V1]>,
}

impl ProtectedCompilerCurrentRequestV1 {
    /// Constructs one complete authentication request from the service's exact inputs.
    ///
    /// # Errors
    ///
    /// Returns a typed error for a zero coordinate, envelope encoding failure,
    /// or any request/envelope/current-record association mismatch.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        protocol_identity: [u8; 32],
        provider_measurement: [u8; 32],
        compiler_policy_identity: [u8; 32],
        admission_session_identity: [u8; 32],
        request_sequence: u64,
        begin_request: &WorkerV3VerificationRequestV1,
        envelope: &WorkerV3LoadEnvelopeWireV2,
        current_record: &WorkerV3VerificationCurrentRecordFrameV2,
    ) -> Result<Self, ProtectedCompilerCurrentProtocolErrorV1> {
        let envelope_bytes = envelope
            .encode_canonical()
            .map_err(|_| ProtectedCompilerCurrentProtocolErrorV1::EnvelopeEncoding)?;
        let envelope_length = u64::try_from(envelope_bytes.len())
            .map_err(|_| ProtectedCompilerCurrentProtocolErrorV1::EnvelopeLength)?;
        let envelope_sha256 = sha256(&envelope_bytes);
        Self::from_parts(
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
            request_sequence,
            begin_request,
            envelope_length,
            envelope_sha256,
            envelope.compiler_execution_receipt(),
            current_record,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_parts(
        protocol_identity: [u8; 32],
        provider_measurement: [u8; 32],
        compiler_policy_identity: [u8; 32],
        admission_session_identity: [u8; 32],
        request_sequence: u64,
        begin_request: &WorkerV3VerificationRequestV1,
        envelope_length: u64,
        envelope_sha256: [u8; 32],
        carriage: &CompilerExecutionReceiptCarriageV1,
        current_record: &WorkerV3VerificationCurrentRecordFrameV2,
    ) -> Result<Self, ProtectedCompilerCurrentProtocolErrorV1> {
        validate_context(
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
        )?;
        if request_sequence == 0 {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::ZeroRequestSequence);
        }
        if envelope_length == 0 || envelope_sha256 == [0; 32] {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::EnvelopeLength);
        }
        let begin = begin_request.encode_canonical();
        let begin_request_length = u32::try_from(begin.len())
            .map_err(|_| ProtectedCompilerCurrentProtocolErrorV1::BeginRequestLength)?;
        let mut begin_request_bytes = boxed_zero_array();
        begin_request_bytes[..begin.len()].copy_from_slice(begin);
        let carriage_bytes = *carriage.canonical_bytes();
        let current_record_bytes = *current_record.encode_canonical();
        validate_association(
            begin_request,
            envelope_length,
            envelope_sha256,
            carriage,
            current_record,
        )?;
        if *carriage.policy().identity().as_bytes() != compiler_policy_identity {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::CompilerPolicyMismatch);
        }
        let begin_request_identity = *begin_request.identity().as_bytes();
        let begin_request_sha256 = sha256(begin);
        let carriage_identity = *carriage.identity().as_bytes();
        let current_record_sha256 = sha256(&current_record_bytes);
        let request_identity = request_identity(
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
            request_sequence,
            begin_request_identity,
            begin_request_sha256,
            envelope_sha256,
            envelope_length,
            carriage_identity,
            current_record_sha256,
            begin,
            &carriage_bytes,
            &current_record_bytes,
        );
        let canonical_bytes = encode_request(
            request_identity,
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
            request_sequence,
            begin_request_identity,
            begin_request_sha256,
            envelope_sha256,
            envelope_length,
            carriage_identity,
            current_record_sha256,
            begin_request_length,
            &begin_request_bytes,
            &carriage_bytes,
            &current_record_bytes,
        );
        Ok(Self {
            request_identity,
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
            request_sequence,
            begin_request_identity,
            begin_request_sha256,
            envelope_sha256,
            envelope_length,
            carriage_identity,
            current_record_sha256,
            begin_request_length,
            begin_request_bytes,
            carriage_bytes,
            current_record_bytes,
            canonical_bytes,
        })
    }

    /// Strictly decodes one complete canonical request packet.
    ///
    /// # Errors
    ///
    /// Returns a typed error for any framing, padding, nested record,
    /// association, coordinate, digest, or identity mismatch.
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self, ProtectedCompilerCurrentProtocolErrorV1> {
        if bytes.len() != PREOPENED_PROTECTED_COMPILER_CURRENT_REQUEST_BYTES_V1 {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::RequestLength {
                expected: PREOPENED_PROTECTED_COMPILER_CURRENT_REQUEST_BYTES_V1,
                actual: bytes.len(),
            });
        }
        let mut reader = ReaderV1::new(bytes);
        require_header(
            &mut reader,
            MAGIC_REQUEST_V1,
            AUTHENTICATE_OPERATION_V1,
            PREOPENED_PROTECTED_COMPILER_CURRENT_REQUEST_BYTES_V1,
        )?;
        let encoded_identity = reader.array()?;
        let protocol_identity = reader.array()?;
        let provider_measurement = reader.array()?;
        let compiler_policy_identity = reader.array()?;
        let admission_session_identity = reader.array()?;
        let request_sequence = reader.u64()?;
        let begin_request_identity = reader.array()?;
        let begin_request_sha256 = reader.array()?;
        let envelope_sha256 = reader.array()?;
        let envelope_length = reader.u64()?;
        let carriage_identity = reader.array()?;
        let current_record_sha256 = reader.array()?;
        let begin_request_length = reader.u32()?;
        if reader.array::<4>()? != [0; 4] {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::ReservedBytes);
        }
        let begin_request_bytes =
            reader.boxed_array::<MAX_WORKER_V3_VERIFICATION_REQUEST_BYTES_V1>()?;
        let carriage_bytes: [u8; COMPILER_EXECUTION_RECEIPT_CARRIAGE_BYTES_V1] = reader.array()?;
        let current_record_bytes: [u8; WORKER_V3_VERIFICATION_CURRENT_RECORD_BYTES_V2] =
            reader.array()?;
        if !reader.is_finished() {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::TrailingBytes);
        }
        let begin_length = usize::try_from(begin_request_length)
            .map_err(|_| ProtectedCompilerCurrentProtocolErrorV1::BeginRequestLength)?;
        if begin_length == 0
            || begin_length > begin_request_bytes.len()
            || begin_request_bytes[begin_length..]
                .iter()
                .any(|byte| *byte != 0)
        {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::BeginRequestPadding);
        }
        let begin_request =
            WorkerV3VerificationRequestV1::decode_canonical(&begin_request_bytes[..begin_length])
                .map_err(|_| ProtectedCompilerCurrentProtocolErrorV1::BeginRequest)?;
        let carriage = CompilerExecutionReceiptCarriageV1::decode(&carriage_bytes)
            .map_err(|_| ProtectedCompilerCurrentProtocolErrorV1::Carriage)?;
        let current_record =
            WorkerV3VerificationCurrentRecordFrameV2::decode_canonical(&current_record_bytes)
                .map_err(|_| ProtectedCompilerCurrentProtocolErrorV1::CurrentRecord)?;
        let decoded = Self::from_parts(
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
            request_sequence,
            &begin_request,
            envelope_length,
            envelope_sha256,
            &carriage,
            &current_record,
        )?;
        if decoded.request_identity != encoded_identity {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::RequestIdentity);
        }
        if decoded.begin_request_identity != begin_request_identity
            || decoded.begin_request_sha256 != begin_request_sha256
            || decoded.carriage_identity != carriage_identity
            || decoded.current_record_sha256 != current_record_sha256
        {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::NestedIdentity);
        }
        if decoded.canonical_bytes.as_slice() != bytes {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::NoncanonicalRequest);
        }
        Ok(decoded)
    }

    /// Returns the identity binding every request coordinate and nested byte string.
    pub const fn request_identity(&self) -> [u8; 32] {
        self.request_identity
    }

    /// Returns the pinned protocol identity.
    pub const fn protocol_identity(&self) -> [u8; 32] {
        self.protocol_identity
    }

    /// Returns the pinned protected-provider measurement.
    pub const fn provider_measurement(&self) -> [u8; 32] {
        self.provider_measurement
    }

    /// Returns the pinned compiler issuer-policy identity.
    pub const fn compiler_policy_identity(&self) -> [u8; 32] {
        self.compiler_policy_identity
    }

    /// Returns the fresh supervisor-provisioned admission identity.
    pub const fn admission_session_identity(&self) -> [u8; 32] {
        self.admission_session_identity
    }

    /// Returns the nonzero monotonic per-client request sequence.
    pub const fn request_sequence(&self) -> u64 {
        self.request_sequence
    }

    /// Returns the exact Begin request's domain-separated protocol identity.
    pub const fn begin_request_identity(&self) -> [u8; 32] {
        self.begin_request_identity
    }

    /// Returns the SHA-256 digest of the complete canonical Begin request.
    pub const fn begin_request_sha256(&self) -> [u8; 32] {
        self.begin_request_sha256
    }

    /// Returns the SHA-256 digest of the complete canonical decoded envelope.
    pub const fn envelope_sha256(&self) -> [u8; 32] {
        self.envelope_sha256
    }

    /// Returns the complete canonical decoded-envelope length.
    pub const fn envelope_length(&self) -> u64 {
        self.envelope_length
    }

    /// Returns the exact decoded-envelope compiler carriage identity.
    pub const fn carriage_identity(&self) -> [u8; 32] {
        self.carriage_identity
    }

    /// Returns the SHA-256 digest of the complete canonical current-record frame.
    pub const fn current_record_sha256(&self) -> [u8; 32] {
        self.current_record_sha256
    }

    /// Borrows the complete canonical Begin request without its zero padding.
    pub fn begin_request_bytes(&self) -> &[u8] {
        &self.begin_request_bytes[..self.begin_request_length as usize]
    }

    /// Borrows the exact canonical compiler receipt carriage from the decoded envelope.
    pub const fn carriage_bytes(&self) -> &[u8; COMPILER_EXECUTION_RECEIPT_CARRIAGE_BYTES_V1] {
        &self.carriage_bytes
    }

    /// Borrows the complete canonical current-record frame.
    pub const fn current_record_bytes(
        &self,
    ) -> &[u8; WORKER_V3_VERIFICATION_CURRENT_RECORD_BYTES_V2] {
        &self.current_record_bytes
    }

    /// Borrows the exact fixed-width packet bytes.
    pub fn encode_canonical(&self) -> &[u8; PREOPENED_PROTECTED_COMPILER_CURRENT_REQUEST_BYTES_V1] {
        &self.canonical_bytes
    }

    /// Protocol framing alone grants no compiler or verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

impl fmt::Debug for ProtectedCompilerCurrentRequestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProtectedCompilerCurrentRequestV1")
            .field("request_identity", &self.request_identity)
            .field("protocol_identity", &self.protocol_identity)
            .field("provider_measurement", &self.provider_measurement)
            .field("compiler_policy_identity", &self.compiler_policy_identity)
            .field(
                "admission_session_identity",
                &self.admission_session_identity,
            )
            .field("request_sequence", &self.request_sequence)
            .field("begin_request_identity", &self.begin_request_identity)
            .field("envelope_sha256", &self.envelope_sha256)
            .field("carriage_identity", &self.carriage_identity)
            .field("current_record_sha256", &self.current_record_sha256)
            .field("nested_bytes", &"redacted")
            .field("authority", &"none")
            .finish_non_exhaustive()
    }
}

/// Terminal status of one correlated current-record authentication response.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedCompilerCurrentResponseStatusV1 {
    /// The protected provider authenticated the exact request transcript.
    Authenticated,
    /// The protected provider explicitly rejected the exact request transcript.
    Rejected,
}

impl ProtectedCompilerCurrentResponseStatusV1 {
    const fn code(self) -> u16 {
        match self {
            Self::Authenticated => 1,
            Self::Rejected => 2,
        }
    }

    fn decode(code: u16) -> Result<Self, ProtectedCompilerCurrentProtocolErrorV1> {
        match code {
            1 => Ok(Self::Authenticated),
            2 => Ok(Self::Rejected),
            actual => Err(ProtectedCompilerCurrentProtocolErrorV1::ResponseStatus { actual }),
        }
    }
}

/// Exact fixed-width terminal response from the protected authenticator.
#[derive(Clone, Eq, PartialEq)]
pub struct ProtectedCompilerCurrentResponseV1 {
    status: ProtectedCompilerCurrentResponseStatusV1,
    response_identity: [u8; 32],
    request_identity: [u8; 32],
    protocol_identity: [u8; 32],
    provider_measurement: [u8; 32],
    compiler_policy_identity: [u8; 32],
    admission_session_identity: [u8; 32],
    request_sequence: u64,
    begin_request_identity: [u8; 32],
    begin_request_sha256: [u8; 32],
    envelope_sha256: [u8; 32],
    carriage_identity: [u8; 32],
    current_record_sha256: [u8; 32],
    transcript_identity: [u8; 32],
    canonical_bytes: [u8; PREOPENED_PROTECTED_COMPILER_CURRENT_RESPONSE_BYTES_V1],
}

impl ProtectedCompilerCurrentResponseV1 {
    /// Constructs an authenticated response carrying a nonzero protected transcript.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the transcript is zero.
    pub fn authenticated(
        request: &ProtectedCompilerCurrentRequestV1,
        transcript_identity: [u8; 32],
    ) -> Result<Self, ProtectedCompilerCurrentProtocolErrorV1> {
        if transcript_identity == [0; 32] {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::TranscriptShape);
        }
        Ok(Self::from_request(
            request,
            ProtectedCompilerCurrentResponseStatusV1::Authenticated,
            transcript_identity,
        ))
    }

    /// Constructs an exact explicit rejection with no authority transcript.
    pub fn rejected(request: &ProtectedCompilerCurrentRequestV1) -> Self {
        Self::from_request(
            request,
            ProtectedCompilerCurrentResponseStatusV1::Rejected,
            [0; 32],
        )
    }

    fn from_request(
        request: &ProtectedCompilerCurrentRequestV1,
        status: ProtectedCompilerCurrentResponseStatusV1,
        transcript_identity: [u8; 32],
    ) -> Self {
        Self::from_fields(
            status,
            request.request_identity,
            request.protocol_identity,
            request.provider_measurement,
            request.compiler_policy_identity,
            request.admission_session_identity,
            request.request_sequence,
            request.begin_request_identity,
            request.begin_request_sha256,
            request.envelope_sha256,
            request.carriage_identity,
            request.current_record_sha256,
            transcript_identity,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_fields(
        status: ProtectedCompilerCurrentResponseStatusV1,
        request_identity: [u8; 32],
        protocol_identity: [u8; 32],
        provider_measurement: [u8; 32],
        compiler_policy_identity: [u8; 32],
        admission_session_identity: [u8; 32],
        request_sequence: u64,
        begin_request_identity: [u8; 32],
        begin_request_sha256: [u8; 32],
        envelope_sha256: [u8; 32],
        carriage_identity: [u8; 32],
        current_record_sha256: [u8; 32],
        transcript_identity: [u8; 32],
    ) -> Self {
        let response_identity = response_identity_fields(
            status,
            request_identity,
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
            request_sequence,
            begin_request_identity,
            begin_request_sha256,
            envelope_sha256,
            carriage_identity,
            current_record_sha256,
            transcript_identity,
        );
        let canonical_bytes = encode_response(
            status,
            response_identity,
            request_identity,
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
            request_sequence,
            begin_request_identity,
            begin_request_sha256,
            envelope_sha256,
            carriage_identity,
            current_record_sha256,
            transcript_identity,
        );
        Self {
            status,
            response_identity,
            request_identity,
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
            request_sequence,
            begin_request_identity,
            begin_request_sha256,
            envelope_sha256,
            carriage_identity,
            current_record_sha256,
            transcript_identity,
            canonical_bytes,
        }
    }

    /// Strictly decodes one complete canonical response packet.
    ///
    /// # Errors
    ///
    /// Returns a typed error for any length, header, status, transcript-shape,
    /// canonical-identity, or nonzero-coordinate mismatch.
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self, ProtectedCompilerCurrentProtocolErrorV1> {
        if bytes.len() != PREOPENED_PROTECTED_COMPILER_CURRENT_RESPONSE_BYTES_V1 {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::ResponseLength {
                expected: PREOPENED_PROTECTED_COMPILER_CURRENT_RESPONSE_BYTES_V1,
                actual: bytes.len(),
            });
        }
        let mut reader = ReaderV1::new(bytes);
        let status_code = require_response_header(&mut reader)?;
        let status = ProtectedCompilerCurrentResponseStatusV1::decode(status_code)?;
        let encoded_identity = reader.array()?;
        let request_identity = reader.array()?;
        let protocol_identity = reader.array()?;
        let provider_measurement = reader.array()?;
        let compiler_policy_identity = reader.array()?;
        let admission_session_identity = reader.array()?;
        let request_sequence = reader.u64()?;
        let begin_request_identity = reader.array()?;
        let begin_request_sha256 = reader.array()?;
        let envelope_sha256 = reader.array()?;
        let carriage_identity = reader.array()?;
        let current_record_sha256 = reader.array()?;
        let transcript_identity = reader.array()?;
        if !reader.is_finished() {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::TrailingBytes);
        }
        validate_context(
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
        )?;
        if request_identity == [0; 32]
            || request_sequence == 0
            || begin_request_identity == [0; 32]
            || begin_request_sha256 == [0; 32]
            || envelope_sha256 == [0; 32]
            || carriage_identity == [0; 32]
            || current_record_sha256 == [0; 32]
        {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::ResponseCoordinates);
        }
        match (status, transcript_identity == [0; 32]) {
            (ProtectedCompilerCurrentResponseStatusV1::Authenticated, false)
            | (ProtectedCompilerCurrentResponseStatusV1::Rejected, true) => {}
            _ => return Err(ProtectedCompilerCurrentProtocolErrorV1::TranscriptShape),
        }
        let decoded = Self::from_fields(
            status,
            request_identity,
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
            request_sequence,
            begin_request_identity,
            begin_request_sha256,
            envelope_sha256,
            carriage_identity,
            current_record_sha256,
            transcript_identity,
        );
        if decoded.response_identity != encoded_identity {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::ResponseIdentity);
        }
        if decoded.canonical_bytes.as_slice() != bytes {
            return Err(ProtectedCompilerCurrentProtocolErrorV1::NoncanonicalResponse);
        }
        Ok(decoded)
    }

    /// Returns the terminal provider status.
    pub const fn status(&self) -> ProtectedCompilerCurrentResponseStatusV1 {
        self.status
    }

    /// Returns the identity binding the complete response.
    pub const fn response_identity(&self) -> [u8; 32] {
        self.response_identity
    }

    /// Returns the nonzero protected transcript on authentication success.
    pub const fn transcript_identity(&self) -> [u8; 32] {
        self.transcript_identity
    }

    /// Checks that every response coordinate names `request` exactly.
    pub fn matches_request(&self, request: &ProtectedCompilerCurrentRequestV1) -> bool {
        self.request_identity == request.request_identity
            && self.protocol_identity == request.protocol_identity
            && self.provider_measurement == request.provider_measurement
            && self.compiler_policy_identity == request.compiler_policy_identity
            && self.admission_session_identity == request.admission_session_identity
            && self.request_sequence == request.request_sequence
            && self.begin_request_identity == request.begin_request_identity
            && self.begin_request_sha256 == request.begin_request_sha256
            && self.envelope_sha256 == request.envelope_sha256
            && self.carriage_identity == request.carriage_identity
            && self.current_record_sha256 == request.current_record_sha256
    }

    /// Borrows the exact canonical packet bytes.
    pub const fn encode_canonical(
        &self,
    ) -> &[u8; PREOPENED_PROTECTED_COMPILER_CURRENT_RESPONSE_BYTES_V1] {
        &self.canonical_bytes
    }

    /// Protocol framing alone grants no compiler or verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

impl fmt::Debug for ProtectedCompilerCurrentResponseV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProtectedCompilerCurrentResponseV1")
            .field("status", &self.status)
            .field("response_identity", &self.response_identity)
            .field("request_identity", &self.request_identity)
            .field("request_sequence", &self.request_sequence)
            .field("transcript_identity", &self.transcript_identity)
            .field("authority", &"none")
            .finish_non_exhaustive()
    }
}

/// Canonical protected compiler-current protocol failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedCompilerCurrentProtocolErrorV1 {
    /// Protocol identity was zero.
    ZeroProtocolIdentity,
    /// Protected provider measurement was zero.
    ZeroProviderMeasurement,
    /// Compiler issuer-policy identity was zero.
    ZeroCompilerPolicyIdentity,
    /// Supervisor admission-session identity was zero.
    ZeroAdmissionSessionIdentity,
    /// Request sequence was zero.
    ZeroRequestSequence,
    /// The exact decoded envelope could not be encoded canonically.
    EnvelopeEncoding,
    /// Envelope length was zero, unrepresentable, or inconsistent.
    EnvelopeLength,
    /// Envelope digest did not match the Begin payload descriptor.
    EnvelopeDigest,
    /// Begin request length was invalid.
    BeginRequestLength,
    /// Begin request decoding failed.
    BeginRequest,
    /// Fixed Begin request padding was not canonical zero.
    BeginRequestPadding,
    /// Exact compiler receipt carriage decoding failed.
    Carriage,
    /// Exact current-record frame decoding failed.
    CurrentRecord,
    /// Pinned compiler policy differed from the decoded carriage policy.
    CompilerPolicyMismatch,
    /// Current-record request, carriage, policy, verification, or attestation association failed.
    CurrentRecordAssociation,
    /// Reserved bytes were not canonical zeroes.
    ReservedBytes,
    /// Request packet length was not exact.
    RequestLength {
        /// Required byte length.
        expected: usize,
        /// Observed byte length.
        actual: usize,
    },
    /// Request header, version, operation, flags, or declared length differed.
    RequestHeader,
    /// Request identity did not bind every exact field.
    RequestIdentity,
    /// Redundant nested identities or digests differed from exact bytes.
    NestedIdentity,
    /// Request did not exactly round-trip through the canonical codec.
    NoncanonicalRequest,
    /// Response packet length was not exact.
    ResponseLength {
        /// Required byte length.
        expected: usize,
        /// Observed byte length.
        actual: usize,
    },
    /// Response header, version, flags, or declared length differed.
    ResponseHeader,
    /// Response status code was unsupported.
    ResponseStatus {
        /// Unsupported status code.
        actual: u16,
    },
    /// Response contained a zero required coordinate.
    ResponseCoordinates,
    /// Success/rejection transcript shape was invalid.
    TranscriptShape,
    /// Response identity did not bind every exact field and status.
    ResponseIdentity,
    /// Response did not exactly round-trip through the canonical codec.
    NoncanonicalResponse,
    /// Fixed-width decoding unexpectedly exhausted its input.
    Truncated,
    /// Fixed-width decoding left trailing input.
    TrailingBytes,
}

impl fmt::Display for ProtectedCompilerCurrentProtocolErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "protected compiler-current protocol rejected: {self:?}"
        )
    }
}

impl Error for ProtectedCompilerCurrentProtocolErrorV1 {}

/// Descriptor or identity rejection before any current-record byte is sent.
#[derive(Debug)]
#[non_exhaustive]
pub enum ProtectedCompilerCurrentClientAdmissionErrorV1 {
    /// Protocol identity was zero.
    ZeroProtocolIdentity,
    /// Protected provider measurement was zero.
    ZeroProviderMeasurement,
    /// Compiler issuer-policy identity was zero.
    ZeroCompilerPolicyIdentity,
    /// Supervisor admission-session identity was zero.
    ZeroAdmissionSessionIdentity,
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
    /// Production provider and verifier service used the same effective UID.
    ProviderAndVerifierUidMatch,
}

impl fmt::Display for ProtectedCompilerCurrentClientAdmissionErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "protected compiler-current endpoint rejected: {self:?}"
        )
    }
}

impl Error for ProtectedCompilerCurrentClientAdmissionErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Descriptor(source) | Self::PeerCredentials(source) => Some(source),
            _ => None,
        }
    }
}

/// Ownership-retaining current-record endpoint admission failure.
///
/// No protocol byte has been sent when this value is returned.
pub struct ProtectedCompilerCurrentClientAdmissionFailureV1 {
    error: ProtectedCompilerCurrentClientAdmissionErrorV1,
    peer: OwnedFd,
}

impl ProtectedCompilerCurrentClientAdmissionFailureV1 {
    /// Returns the stable admission error while retaining the endpoint.
    pub const fn error(&self) -> &ProtectedCompilerCurrentClientAdmissionErrorV1 {
        &self.error
    }

    /// Returns the exact caller-owned endpoint.
    pub fn into_peer(self) -> OwnedFd {
        self.peer
    }

    /// Returns the error and exact caller-owned endpoint.
    pub fn into_parts(self) -> (ProtectedCompilerCurrentClientAdmissionErrorV1, OwnedFd) {
        (self.error, self.peer)
    }
}

impl fmt::Debug for ProtectedCompilerCurrentClientAdmissionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProtectedCompilerCurrentClientAdmissionFailureV1")
            .field("error", &self.error)
            .field(
                "custody",
                &ProtectedCompilerCurrentClientCustodyV1::Retained,
            )
            .finish_non_exhaustive()
    }
}

impl fmt::Display for ProtectedCompilerCurrentClientAdmissionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl Error for ProtectedCompilerCurrentClientAdmissionFailureV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error.source()
    }
}

/// Endpoint custody after a current-record authentication attempt fails.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedCompilerCurrentClientCustodyV1 {
    /// No ambiguous send occurred and the synchronized endpoint remains in the client.
    Retained,
    /// The outcome may be ambiguous or the endpoint changed; the endpoint was closed.
    Poisoned,
}

/// Bounded current-record admission, transport, or correlation error.
#[derive(Debug)]
#[non_exhaustive]
pub enum ProtectedCompilerCurrentClientErrorV1 {
    /// A prior ambiguous exchange permanently closed the endpoint.
    Poisoned,
    /// Fixed protocol construction or decoding failed.
    Protocol(ProtectedCompilerCurrentProtocolErrorV1),
    /// Endpoint identity or peer credentials changed before or during exchange.
    EndpointRevalidation(ProtectedCompilerCurrentClientAdmissionErrorV1),
    /// The sole service absolute deadline expired.
    DeadlineExpired,
    /// Polling the provider endpoint failed.
    Poll(io::Error),
    /// Sending the exact request packet failed.
    Send(io::Error),
    /// The atomic seqpacket send was partial.
    PartialSend,
    /// Receiving the response packet failed.
    Receive(io::Error),
    /// The provider closed before a terminal packet arrived.
    PeerClosed,
    /// The response exceeded the sole fixed packet bound.
    PacketTruncated,
    /// The provider attempted to transfer ancillary data.
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
    /// The provider returned a correlated explicit rejection.
    ProviderRejected,
    /// Poll reported an invalid endpoint.
    InvalidPeer,
    /// Poll reported an asynchronous endpoint error.
    PeerFailed,
    /// The nonzero per-client request sequence was exhausted.
    RequestSequenceExhausted,
    /// The protected success transcript was unexpectedly invalid.
    InvalidProviderClaim,
}

impl fmt::Display for ProtectedCompilerCurrentClientErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "protected compiler-current client failed: {self:?}"
        )
    }
}

impl Error for ProtectedCompilerCurrentClientErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Protocol(source) => Some(source),
            Self::EndpointRevalidation(source) => Some(source),
            Self::Poll(source) | Self::Send(source) | Self::Receive(source) => Some(source),
            _ => None,
        }
    }
}

/// Typed failure preserving whether the endpoint remains usable.
#[derive(Debug)]
pub struct ProtectedCompilerCurrentClientFailureV1 {
    error: ProtectedCompilerCurrentClientErrorV1,
    custody: ProtectedCompilerCurrentClientCustodyV1,
}

impl ProtectedCompilerCurrentClientFailureV1 {
    /// Returns the authentication error.
    pub const fn error(&self) -> &ProtectedCompilerCurrentClientErrorV1 {
        &self.error
    }

    /// Returns the exact endpoint custody after failure.
    pub const fn custody(&self) -> ProtectedCompilerCurrentClientCustodyV1 {
        self.custody
    }

    /// Splits the failure into its error and custody coordinates.
    pub fn into_parts(
        self,
    ) -> (
        ProtectedCompilerCurrentClientErrorV1,
        ProtectedCompilerCurrentClientCustodyV1,
    ) {
        (self.error, self.custody)
    }
}

impl fmt::Display for ProtectedCompilerCurrentClientFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl Error for ProtectedCompilerCurrentClientFailureV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error.source()
    }
}

/// Concrete descriptor-only protected compiler-current provider.
///
/// The provider is move-only. A legacy trait error retains its custody
/// coordinate; callers needing typed custody can use the bounded method.
pub struct PreopenedProtectedCompilerCurrentClientV1 {
    peer: Option<OwnedFd>,
    endpoint: ProtectedCompilerCurrentEndpointIdentityV1,
    expected_peer: ProtectedCompilerCurrentPeerIdentityV1,
    protocol_identity: [u8; 32],
    provider_measurement: [u8; 32],
    compiler_policy_identity: [u8; 32],
    admission_session_identity: [u8; 32],
    next_request_sequence: u64,
}

impl fmt::Debug for PreopenedProtectedCompilerCurrentClientV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreopenedProtectedCompilerCurrentClientV1")
            .field("available", &self.peer.is_some())
            .field("endpoint", &self.endpoint)
            .field("expected_peer", &self.expected_peer)
            .field("protocol_identity", &self.protocol_identity)
            .field("provider_measurement", &self.provider_measurement)
            .field("compiler_policy_identity", &self.compiler_policy_identity)
            .field(
                "admission_session_identity",
                &self.admission_session_identity,
            )
            .field("authority", &"unsafe supervisor boundary")
            .finish_non_exhaustive()
    }
}

impl PreopenedProtectedCompilerCurrentClientV1 {
    /// Admits one exact supervisor-preopened protected authenticator endpoint.
    ///
    /// The endpoint must already be freshly connected, nonblocking, and
    /// close-on-exec. Production requires a dedicated non-root UID distinct
    /// from the verifier. This function performs no path discovery or connect.
    ///
    /// # Safety
    ///
    /// The protected supervisor must independently authenticate that `peer` is
    /// the measured implementation named by `provider_measurement`, implements
    /// exactly `protocol_identity`, and enforces exactly
    /// `compiler_policy_identity`. It must verify the complete packet's Begin,
    /// compiler carriage, current-record verification and attestation, live
    /// Worker ledger state, protected policy, external rollback authority, and
    /// challenge association before returning `Authenticated`.
    /// `admission_session_identity` must be nonzero, freshly generated, and
    /// never reused for this provider, including across supervisor, verifier,
    /// and provider restarts. The supervisor must give this client exclusive
    /// custody of a fresh connection with no prequeued packets and retain no
    /// duplicate descriptor. These obligations are not established by socket
    /// metadata or the wire codec.
    ///
    /// # Errors
    ///
    /// Returns an ownership-retaining failure before sending any bytes.
    pub unsafe fn admit_from_supervisor(
        peer: OwnedFd,
        endpoint: ProtectedCompilerCurrentEndpointIdentityV1,
        expected_peer: ProtectedCompilerCurrentPeerIdentityV1,
        protocol_identity: [u8; 32],
        provider_measurement: [u8; 32],
        compiler_policy_identity: [u8; 32],
        admission_session_identity: [u8; 32],
    ) -> Result<Self, ProtectedCompilerCurrentClientAdmissionFailureV1> {
        Self::admit_inner::<true>(
            peer,
            endpoint,
            expected_peer,
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn admit_inner<const REQUIRE_DISTINCT_UID: bool>(
        peer: OwnedFd,
        endpoint: ProtectedCompilerCurrentEndpointIdentityV1,
        expected_peer: ProtectedCompilerCurrentPeerIdentityV1,
        protocol_identity: [u8; 32],
        provider_measurement: [u8; 32],
        compiler_policy_identity: [u8; 32],
        admission_session_identity: [u8; 32],
    ) -> Result<Self, ProtectedCompilerCurrentClientAdmissionFailureV1> {
        let admission = validate_admission_coordinates(
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
        )
        .and_then(|()| validate_endpoint::<REQUIRE_DISTINCT_UID>(&peer, endpoint, expected_peer));
        if let Err(error) = admission {
            return Err(ProtectedCompilerCurrentClientAdmissionFailureV1 { error, peer });
        }
        Ok(Self {
            peer: Some(peer),
            endpoint,
            expected_peer,
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
            next_request_sequence: 1,
        })
    }

    /// Returns whether an ambiguous or terminal failure permanently closed the endpoint.
    pub const fn is_poisoned(&self) -> bool {
        self.peer.is_none()
    }

    /// Returns the protected provider measurement exposed to the service contract.
    pub const fn measurement_identity(&self) -> [u8; 32] {
        self.provider_measurement
    }

    /// Descriptor admission and framing grant no compiler or verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }

    /// Authenticates one exact service input under its sole absolute deadline.
    ///
    /// # Errors
    ///
    /// Returns a typed failure carrying retained or poisoned endpoint custody.
    pub fn authenticate_current_record_bounded(
        &mut self,
        input: &ProtectedCompilerCurrentRecordInputV1<'_>,
    ) -> Result<AuthenticatedCompilerCurrentRecordV1, ProtectedCompilerCurrentClientFailureV1> {
        self.authenticate_inner::<true>(input)
    }

    fn authenticate_inner<const REQUIRE_DISTINCT_UID: bool>(
        &mut self,
        input: &ProtectedCompilerCurrentRecordInputV1<'_>,
    ) -> Result<AuthenticatedCompilerCurrentRecordV1, ProtectedCompilerCurrentClientFailureV1> {
        self.authenticate_parts::<REQUIRE_DISTINCT_UID>(
            input.request(),
            input.envelope(),
            input.current_record(),
            input.deadline(),
        )
    }

    fn authenticate_parts<const REQUIRE_DISTINCT_UID: bool>(
        &mut self,
        begin_request: &WorkerV3VerificationRequestV1,
        envelope: &WorkerV3LoadEnvelopeWireV2,
        current_record: &WorkerV3VerificationCurrentRecordFrameV2,
        deadline: AbsoluteSessionDeadlineV1,
    ) -> Result<AuthenticatedCompilerCurrentRecordV1, ProtectedCompilerCurrentClientFailureV1> {
        if deadline.remaining().is_none() {
            return Err(
                self.pre_send_failure(ProtectedCompilerCurrentClientErrorV1::DeadlineExpired)
            );
        }
        let request_sequence = self.take_request_sequence()?;
        let request = ProtectedCompilerCurrentRequestV1::new(
            self.protocol_identity,
            self.provider_measurement,
            self.compiler_policy_identity,
            self.admission_session_identity,
            request_sequence,
            begin_request,
            envelope,
            current_record,
        )
        .map_err(|error| {
            self.pre_send_failure(ProtectedCompilerCurrentClientErrorV1::Protocol(error))
        })?;
        let response = self.exchange::<REQUIRE_DISTINCT_UID>(&request, deadline)?;
        match response.status {
            ProtectedCompilerCurrentResponseStatusV1::Authenticated => {
                // SAFETY: only an exact correlated success from the unsafe-admitted protected
                // provider reaches this branch, and the codec requires a nonzero transcript.
                unsafe {
                    AuthenticatedCompilerCurrentRecordV1::from_independent_authentication(
                        response.transcript_identity,
                    )
                }
                .map_err(|_| {
                    self.poisoned_failure(
                        ProtectedCompilerCurrentClientErrorV1::InvalidProviderClaim,
                    )
                })
            }
            ProtectedCompilerCurrentResponseStatusV1::Rejected => Err(Self::retained_failure(
                ProtectedCompilerCurrentClientErrorV1::ProviderRejected,
            )),
        }
    }

    fn exchange<const REQUIRE_DISTINCT_UID: bool>(
        &mut self,
        request: &ProtectedCompilerCurrentRequestV1,
        deadline: AbsoluteSessionDeadlineV1,
    ) -> Result<ProtectedCompilerCurrentResponseV1, ProtectedCompilerCurrentClientFailureV1> {
        if deadline.remaining().is_none() {
            return Err(
                self.pre_send_failure(ProtectedCompilerCurrentClientErrorV1::DeadlineExpired)
            );
        }
        let Some(peer) = self.peer.take() else {
            return Err(self.poisoned_failure(ProtectedCompilerCurrentClientErrorV1::Poisoned));
        };
        if let Err(error) =
            validate_endpoint::<REQUIRE_DISTINCT_UID>(&peer, self.endpoint, self.expected_peer)
        {
            return Err(ProtectedCompilerCurrentClientFailureV1 {
                error: ProtectedCompilerCurrentClientErrorV1::EndpointRevalidation(error),
                custody: ProtectedCompilerCurrentClientCustodyV1::Poisoned,
            });
        }
        match send_packet(&peer, request.encode_canonical(), deadline) {
            Ok(()) => {}
            Err(failure) => {
                if !failure.poison {
                    self.peer = Some(peer);
                }
                return Err(ProtectedCompilerCurrentClientFailureV1 {
                    error: failure.error,
                    custody: if failure.poison {
                        ProtectedCompilerCurrentClientCustodyV1::Poisoned
                    } else {
                        ProtectedCompilerCurrentClientCustodyV1::Retained
                    },
                });
            }
        }
        let response = receive_correlated_response::<REQUIRE_DISTINCT_UID>(
            &peer,
            self.endpoint,
            self.expected_peer,
            request,
            deadline,
        )
        .map_err(|error| ProtectedCompilerCurrentClientFailureV1 {
            error,
            custody: ProtectedCompilerCurrentClientCustodyV1::Poisoned,
        })?;
        self.peer = Some(peer);
        Ok(response)
    }

    fn take_request_sequence(&mut self) -> Result<u64, ProtectedCompilerCurrentClientFailureV1> {
        let sequence = self.next_request_sequence;
        if sequence == 0 {
            return Err(self.pre_send_failure(
                ProtectedCompilerCurrentClientErrorV1::RequestSequenceExhausted,
            ));
        }
        self.next_request_sequence = sequence.checked_add(1).unwrap_or(0);
        Ok(sequence)
    }

    fn pre_send_failure(
        &self,
        error: ProtectedCompilerCurrentClientErrorV1,
    ) -> ProtectedCompilerCurrentClientFailureV1 {
        ProtectedCompilerCurrentClientFailureV1 {
            error,
            custody: if self.peer.is_some() {
                ProtectedCompilerCurrentClientCustodyV1::Retained
            } else {
                ProtectedCompilerCurrentClientCustodyV1::Poisoned
            },
        }
    }

    fn retained_failure(
        error: ProtectedCompilerCurrentClientErrorV1,
    ) -> ProtectedCompilerCurrentClientFailureV1 {
        ProtectedCompilerCurrentClientFailureV1 {
            error,
            custody: ProtectedCompilerCurrentClientCustodyV1::Retained,
        }
    }

    fn poisoned_failure(
        &mut self,
        error: ProtectedCompilerCurrentClientErrorV1,
    ) -> ProtectedCompilerCurrentClientFailureV1 {
        self.peer = None;
        ProtectedCompilerCurrentClientFailureV1 {
            error,
            custody: ProtectedCompilerCurrentClientCustodyV1::Poisoned,
        }
    }
}

// SAFETY: construction is restricted to the documented unsafe supervisor boundary. Every
// request and response binds the exact measured provider, compiler policy, fresh admission
// session, monotonic sequence, Begin frame, envelope carriage, and current-record frame. Only
// exact correlated authentication mints a nonzero token; every ambiguous post-send outcome
// closes and poisons the endpoint.
unsafe impl ProtectedCompilerCurrentRecordProviderV1 for PreopenedProtectedCompilerCurrentClientV1 {
    type Error = ProtectedCompilerCurrentClientFailureV1;

    fn measurement_identity(&self) -> [u8; 32] {
        self.provider_measurement
    }

    fn authenticate_current_record(
        &mut self,
        input: ProtectedCompilerCurrentRecordInputV1<'_>,
    ) -> Result<AuthenticatedCompilerCurrentRecordV1, Self::Error> {
        self.authenticate_current_record_bounded(&input)
    }
}

fn validate_context(
    protocol_identity: [u8; 32],
    provider_measurement: [u8; 32],
    compiler_policy_identity: [u8; 32],
    admission_session_identity: [u8; 32],
) -> Result<(), ProtectedCompilerCurrentProtocolErrorV1> {
    if protocol_identity == [0; 32] {
        return Err(ProtectedCompilerCurrentProtocolErrorV1::ZeroProtocolIdentity);
    }
    if provider_measurement == [0; 32] {
        return Err(ProtectedCompilerCurrentProtocolErrorV1::ZeroProviderMeasurement);
    }
    if compiler_policy_identity == [0; 32] {
        return Err(ProtectedCompilerCurrentProtocolErrorV1::ZeroCompilerPolicyIdentity);
    }
    if admission_session_identity == [0; 32] {
        return Err(ProtectedCompilerCurrentProtocolErrorV1::ZeroAdmissionSessionIdentity);
    }
    Ok(())
}

fn validate_association(
    begin_request: &WorkerV3VerificationRequestV1,
    envelope_length: u64,
    envelope_sha256: [u8; 32],
    carriage: &CompilerExecutionReceiptCarriageV1,
    current: &WorkerV3VerificationCurrentRecordFrameV2,
) -> Result<(), ProtectedCompilerCurrentProtocolErrorV1> {
    let envelope_descriptor = &begin_request.payloads()[0];
    if envelope_descriptor.byte_len() != envelope_length {
        return Err(ProtectedCompilerCurrentProtocolErrorV1::EnvelopeLength);
    }
    if *envelope_descriptor.sha256() != envelope_sha256 {
        return Err(ProtectedCompilerCurrentProtocolErrorV1::EnvelopeDigest);
    }
    let current_bytes = current.encode_canonical();
    let request_end = CURRENT_RECORD_REQUEST_IDENTITY_OFFSET + IDENTITY_BYTES;
    if current_bytes.get(CURRENT_RECORD_REQUEST_IDENTITY_OFFSET..request_end)
        != Some(begin_request.identity().as_bytes().as_slice())
    {
        return Err(ProtectedCompilerCurrentProtocolErrorV1::CurrentRecordAssociation);
    }
    let verification = current.verification();
    if verification.policy_identity() != *carriage.policy().identity().as_bytes()
        || verification.subject_identity() != *carriage.request().subject().identity().sha256()
        || verification.carriage_identity() != *carriage.identity().as_bytes()
        || verification.issuer_journal_identity()
            != carriage.acknowledgment().issuer_journal_identity()
        || verification.worker_ledger_record_identity()
            != carriage.acknowledgment().worker_ledger_record_identity()
        || verification.sequence() != carriage.acknowledgment().sequence()
        || verification.prior_rollback_anchor()
            != carriage.publication().receipt().prior_rollback_anchor()
        || verification.current_rollback_anchor()
            != carriage.acknowledgment().current_rollback_anchor()
        || current.attestation().verification().canonical_bytes() != verification.canonical_bytes()
    {
        return Err(ProtectedCompilerCurrentProtocolErrorV1::CurrentRecordAssociation);
    }
    current
        .attestation()
        .clone()
        .verify(
            carriage.policy(),
            carriage,
            current.attestation().challenge(),
        )
        .map_err(|_| ProtectedCompilerCurrentProtocolErrorV1::CurrentRecordAssociation)?;
    Ok(())
}

fn validate_admission_coordinates(
    protocol_identity: [u8; 32],
    provider_measurement: [u8; 32],
    compiler_policy_identity: [u8; 32],
    admission_session_identity: [u8; 32],
) -> Result<(), ProtectedCompilerCurrentClientAdmissionErrorV1> {
    if protocol_identity == [0; 32] {
        return Err(ProtectedCompilerCurrentClientAdmissionErrorV1::ZeroProtocolIdentity);
    }
    if provider_measurement == [0; 32] {
        return Err(ProtectedCompilerCurrentClientAdmissionErrorV1::ZeroProviderMeasurement);
    }
    if compiler_policy_identity == [0; 32] {
        return Err(ProtectedCompilerCurrentClientAdmissionErrorV1::ZeroCompilerPolicyIdentity);
    }
    if admission_session_identity == [0; 32] {
        return Err(ProtectedCompilerCurrentClientAdmissionErrorV1::ZeroAdmissionSessionIdentity);
    }
    Ok(())
}

fn validate_endpoint<const REQUIRE_DISTINCT_UID: bool>(
    peer: &OwnedFd,
    endpoint: ProtectedCompilerCurrentEndpointIdentityV1,
    expected_peer: ProtectedCompilerCurrentPeerIdentityV1,
) -> Result<(), ProtectedCompilerCurrentClientAdmissionErrorV1> {
    let stat = rustix::fs::fstat(peer).map_err(|source| {
        ProtectedCompilerCurrentClientAdmissionErrorV1::Descriptor(source.into())
    })?;
    let descriptor_flags = rustix::io::fcntl_getfd(peer).map_err(|source| {
        ProtectedCompilerCurrentClientAdmissionErrorV1::Descriptor(source.into())
    })?;
    let status = rustix::fs::fcntl_getfl(peer).map_err(|source| {
        ProtectedCompilerCurrentClientAdmissionErrorV1::Descriptor(source.into())
    })?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::Socket
        || !descriptor_flags.contains(FdFlags::CLOEXEC)
        || !status.contains(OFlags::NONBLOCK)
        || status & OFlags::ACCMODE != OFlags::RDWR
        || status.intersects(OFlags::APPEND | OFlags::ASYNC | OFlags::DIRECT | OFlags::PATH)
    {
        return Err(ProtectedCompilerCurrentClientAdmissionErrorV1::DescriptorShape);
    }
    if stat.st_dev != endpoint.device || stat.st_ino != endpoint.inode {
        return Err(ProtectedCompilerCurrentClientAdmissionErrorV1::EndpointIdentityMismatch);
    }
    if socket_option(peer, libc::SO_DOMAIN)? != libc::AF_UNIX
        || socket_option(peer, libc::SO_TYPE)? != libc::SOCK_SEQPACKET
        || socket_option(peer, libc::SO_ACCEPTCONN)? != 0
        || !is_connected_unix(peer)?
    {
        return Err(ProtectedCompilerCurrentClientAdmissionErrorV1::SocketShape);
    }
    let pending = socket_option(peer, libc::SO_ERROR)?;
    if pending != 0 {
        return Err(
            ProtectedCompilerCurrentClientAdmissionErrorV1::PendingSocketError { raw: pending },
        );
    }
    let observed = peer_credentials(peer)?;
    if observed != expected_peer {
        return Err(ProtectedCompilerCurrentClientAdmissionErrorV1::PeerIdentityMismatch);
    }
    if REQUIRE_DISTINCT_UID && observed.uid == rustix::process::geteuid().as_raw() {
        return Err(ProtectedCompilerCurrentClientAdmissionErrorV1::ProviderAndVerifierUidMatch);
    }
    Ok(())
}

fn socket_option(
    peer: &OwnedFd,
    option: i32,
) -> Result<i32, ProtectedCompilerCurrentClientAdmissionErrorV1> {
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
        return Err(ProtectedCompilerCurrentClientAdmissionErrorV1::Descriptor(
            io::Error::last_os_error(),
        ));
    }
    let actual = usize::try_from(length).expect("socklen_t fits usize");
    if actual != mem::size_of::<i32>() {
        return Err(
            ProtectedCompilerCurrentClientAdmissionErrorV1::SocketOptionLength {
                expected: mem::size_of::<i32>(),
                actual,
            },
        );
    }
    Ok(value)
}

fn is_connected_unix(
    peer: &OwnedFd,
) -> Result<bool, ProtectedCompilerCurrentClientAdmissionErrorV1> {
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
        return Err(ProtectedCompilerCurrentClientAdmissionErrorV1::Descriptor(
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
) -> Result<ProtectedCompilerCurrentPeerIdentityV1, ProtectedCompilerCurrentClientAdmissionErrorV1>
{
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
            ProtectedCompilerCurrentClientAdmissionErrorV1::PeerCredentials(
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
        return Err(ProtectedCompilerCurrentClientAdmissionErrorV1::InvalidPeerCredentials);
    }
    Ok(ProtectedCompilerCurrentPeerIdentityV1 {
        pid: pid.expect("positive peer PID checked"),
        uid: credentials.uid,
        gid: credentials.gid,
    })
}

struct SendPacketFailureV1 {
    error: ProtectedCompilerCurrentClientErrorV1,
    poison: bool,
}

fn send_packet(
    peer: &OwnedFd,
    bytes: &[u8; PREOPENED_PROTECTED_COMPILER_CURRENT_REQUEST_BYTES_V1],
    deadline: AbsoluteSessionDeadlineV1,
) -> Result<(), SendPacketFailureV1> {
    if let Err(error) = wait_for_peer(peer, libc::POLLOUT, deadline) {
        let poison = !matches!(
            error,
            ProtectedCompilerCurrentClientErrorV1::DeadlineExpired
        );
        return Err(SendPacketFailureV1 { error, poison });
    }
    // SAFETY: bytes names the complete readable packet and peer remains uniquely owned.
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
            error: ProtectedCompilerCurrentClientErrorV1::Send(io::Error::last_os_error()),
            poison: true,
        });
    }
    if usize::try_from(sent).ok() != Some(bytes.len()) {
        return Err(SendPacketFailureV1 {
            error: ProtectedCompilerCurrentClientErrorV1::PartialSend,
            poison: true,
        });
    }
    Ok(())
}

fn receive_correlated_response<const REQUIRE_DISTINCT_UID: bool>(
    peer: &OwnedFd,
    endpoint: ProtectedCompilerCurrentEndpointIdentityV1,
    expected_peer: ProtectedCompilerCurrentPeerIdentityV1,
    request: &ProtectedCompilerCurrentRequestV1,
    deadline: AbsoluteSessionDeadlineV1,
) -> Result<ProtectedCompilerCurrentResponseV1, ProtectedCompilerCurrentClientErrorV1> {
    validate_endpoint::<REQUIRE_DISTINCT_UID>(peer, endpoint, expected_peer)
        .map_err(ProtectedCompilerCurrentClientErrorV1::EndpointRevalidation)?;
    let bytes = receive_packet(peer, deadline)?;
    validate_endpoint::<REQUIRE_DISTINCT_UID>(peer, endpoint, expected_peer)
        .map_err(ProtectedCompilerCurrentClientErrorV1::EndpointRevalidation)?;
    if deadline.remaining().is_none() {
        return Err(ProtectedCompilerCurrentClientErrorV1::DeadlineExpired);
    }
    let response = ProtectedCompilerCurrentResponseV1::decode_canonical(&bytes)
        .map_err(ProtectedCompilerCurrentClientErrorV1::Protocol)?;
    if !response.matches_request(request) {
        return Err(ProtectedCompilerCurrentClientErrorV1::ResponseRequestMismatch);
    }
    Ok(response)
}

fn receive_packet(
    peer: &OwnedFd,
    deadline: AbsoluteSessionDeadlineV1,
) -> Result<
    [u8; PREOPENED_PROTECTED_COMPILER_CURRENT_RESPONSE_BYTES_V1],
    ProtectedCompilerCurrentClientErrorV1,
> {
    wait_for_peer(peer, libc::POLLIN, deadline)?;
    let mut bytes = [0_u8; PREOPENED_PROTECTED_COMPILER_CURRENT_RESPONSE_BYTES_V1];
    let mut vector = libc::iovec {
        iov_base: bytes.as_mut_ptr().cast(),
        iov_len: bytes.len(),
    };
    // SAFETY: zero is a valid empty msghdr and the live iovec is installed below.
    let mut header = unsafe { mem::zeroed::<libc::msghdr>() };
    header.msg_iov = &raw mut vector;
    header.msg_iovlen = 1;
    // SAFETY: header names the output buffer and deliberately has no ancillary buffer.
    let received = unsafe {
        libc::recvmsg(
            peer.as_raw_fd(),
            &raw mut header,
            libc::MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC,
        )
    };
    if received < 0 {
        return Err(ProtectedCompilerCurrentClientErrorV1::Receive(
            io::Error::last_os_error(),
        ));
    }
    if header.msg_flags & libc::MSG_CTRUNC != 0 || header.msg_controllen != 0 {
        return Err(ProtectedCompilerCurrentClientErrorV1::AncillaryData);
    }
    if header.msg_flags & libc::MSG_TRUNC != 0 {
        return Err(ProtectedCompilerCurrentClientErrorV1::PacketTruncated);
    }
    let received = usize::try_from(received)
        .map_err(|_| ProtectedCompilerCurrentClientErrorV1::PacketTruncated)?;
    if received == 0 {
        return Err(ProtectedCompilerCurrentClientErrorV1::PeerClosed);
    }
    if received != bytes.len() {
        return Err(ProtectedCompilerCurrentClientErrorV1::ResponseLength {
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
) -> Result<(), ProtectedCompilerCurrentClientErrorV1> {
    loop {
        let remaining = deadline
            .remaining()
            .ok_or(ProtectedCompilerCurrentClientErrorV1::DeadlineExpired)?;
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
            return Err(ProtectedCompilerCurrentClientErrorV1::Poll(source));
        }
        if result == 0 {
            continue;
        }
        if deadline.remaining().is_none() {
            return Err(ProtectedCompilerCurrentClientErrorV1::DeadlineExpired);
        }
        if descriptor.revents & libc::POLLNVAL != 0 {
            return Err(ProtectedCompilerCurrentClientErrorV1::InvalidPeer);
        }
        if descriptor.revents & wanted != 0 {
            return Ok(());
        }
        if descriptor.revents & libc::POLLERR != 0 {
            return Err(ProtectedCompilerCurrentClientErrorV1::PeerFailed);
        }
        if descriptor.revents & libc::POLLHUP != 0 {
            return Err(ProtectedCompilerCurrentClientErrorV1::PeerClosed);
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

#[allow(clippy::too_many_arguments)]
fn request_identity(
    protocol_identity: [u8; 32],
    provider_measurement: [u8; 32],
    compiler_policy_identity: [u8; 32],
    admission_session_identity: [u8; 32],
    request_sequence: u64,
    begin_request_identity: [u8; 32],
    begin_request_sha256: [u8; 32],
    envelope_sha256: [u8; 32],
    envelope_length: u64,
    carriage_identity: [u8; 32],
    current_record_sha256: [u8; 32],
    begin_request: &[u8],
    carriage: &[u8; COMPILER_EXECUTION_RECEIPT_CARRIAGE_BYTES_V1],
    current_record: &[u8; WORKER_V3_VERIFICATION_CURRENT_RECORD_BYTES_V2],
) -> [u8; 32] {
    hash_parts(&[
        REQUEST_IDENTITY_DOMAIN_V1,
        &VERSION_V1.to_le_bytes(),
        &AUTHENTICATE_OPERATION_V1.to_le_bytes(),
        &(PREOPENED_PROTECTED_COMPILER_CURRENT_REQUEST_BYTES_V1 as u64).to_le_bytes(),
        &protocol_identity,
        &provider_measurement,
        &compiler_policy_identity,
        &admission_session_identity,
        &request_sequence.to_le_bytes(),
        &begin_request_identity,
        &begin_request_sha256,
        &envelope_sha256,
        &envelope_length.to_le_bytes(),
        &carriage_identity,
        &current_record_sha256,
        &u32::try_from(begin_request.len())
            .expect("bounded Begin request length fits u32")
            .to_le_bytes(),
        begin_request,
        carriage,
        current_record,
    ])
}

#[allow(clippy::too_many_arguments)]
fn response_identity_fields(
    status: ProtectedCompilerCurrentResponseStatusV1,
    request_identity: [u8; 32],
    protocol_identity: [u8; 32],
    provider_measurement: [u8; 32],
    compiler_policy_identity: [u8; 32],
    admission_session_identity: [u8; 32],
    request_sequence: u64,
    begin_request_identity: [u8; 32],
    begin_request_sha256: [u8; 32],
    envelope_sha256: [u8; 32],
    carriage_identity: [u8; 32],
    current_record_sha256: [u8; 32],
    transcript_identity: [u8; 32],
) -> [u8; 32] {
    hash_parts(&[
        RESPONSE_IDENTITY_DOMAIN_V1,
        &VERSION_V1.to_le_bytes(),
        &status.code().to_le_bytes(),
        &(PREOPENED_PROTECTED_COMPILER_CURRENT_RESPONSE_BYTES_V1 as u64).to_le_bytes(),
        &request_identity,
        &protocol_identity,
        &provider_measurement,
        &compiler_policy_identity,
        &admission_session_identity,
        &request_sequence.to_le_bytes(),
        &begin_request_identity,
        &begin_request_sha256,
        &envelope_sha256,
        &carriage_identity,
        &current_record_sha256,
        &transcript_identity,
    ])
}

#[allow(clippy::too_many_arguments)]
fn encode_request(
    request_identity: [u8; 32],
    protocol_identity: [u8; 32],
    provider_measurement: [u8; 32],
    compiler_policy_identity: [u8; 32],
    admission_session_identity: [u8; 32],
    request_sequence: u64,
    begin_request_identity: [u8; 32],
    begin_request_sha256: [u8; 32],
    envelope_sha256: [u8; 32],
    envelope_length: u64,
    carriage_identity: [u8; 32],
    current_record_sha256: [u8; 32],
    begin_request_length: u32,
    begin_request: &[u8; MAX_WORKER_V3_VERIFICATION_REQUEST_BYTES_V1],
    carriage: &[u8; COMPILER_EXECUTION_RECEIPT_CARRIAGE_BYTES_V1],
    current_record: &[u8; WORKER_V3_VERIFICATION_CURRENT_RECORD_BYTES_V2],
) -> Box<[u8; PREOPENED_PROTECTED_COMPILER_CURRENT_REQUEST_BYTES_V1]> {
    let mut bytes = boxed_zero_array();
    let mut offset = encode_header(&mut bytes, MAGIC_REQUEST_V1, AUTHENTICATE_OPERATION_V1);
    for identity in [
        request_identity,
        protocol_identity,
        provider_measurement,
        compiler_policy_identity,
        admission_session_identity,
    ] {
        put(&mut bytes, &mut offset, &identity);
    }
    put(&mut bytes, &mut offset, &request_sequence.to_le_bytes());
    for identity in [
        begin_request_identity,
        begin_request_sha256,
        envelope_sha256,
    ] {
        put(&mut bytes, &mut offset, &identity);
    }
    put(&mut bytes, &mut offset, &envelope_length.to_le_bytes());
    put(&mut bytes, &mut offset, &carriage_identity);
    put(&mut bytes, &mut offset, &current_record_sha256);
    put(&mut bytes, &mut offset, &begin_request_length.to_le_bytes());
    put(&mut bytes, &mut offset, &[0; 4]);
    put(&mut bytes, &mut offset, begin_request);
    put(&mut bytes, &mut offset, carriage);
    put(&mut bytes, &mut offset, current_record);
    debug_assert_eq!(offset, bytes.len());
    bytes
}

#[allow(clippy::too_many_arguments)]
fn encode_response(
    status: ProtectedCompilerCurrentResponseStatusV1,
    response_identity: [u8; 32],
    request_identity: [u8; 32],
    protocol_identity: [u8; 32],
    provider_measurement: [u8; 32],
    compiler_policy_identity: [u8; 32],
    admission_session_identity: [u8; 32],
    request_sequence: u64,
    begin_request_identity: [u8; 32],
    begin_request_sha256: [u8; 32],
    envelope_sha256: [u8; 32],
    carriage_identity: [u8; 32],
    current_record_sha256: [u8; 32],
    transcript_identity: [u8; 32],
) -> [u8; PREOPENED_PROTECTED_COMPILER_CURRENT_RESPONSE_BYTES_V1] {
    let mut bytes = [0_u8; PREOPENED_PROTECTED_COMPILER_CURRENT_RESPONSE_BYTES_V1];
    let mut offset = encode_header(&mut bytes, MAGIC_RESPONSE_V1, status.code());
    for identity in [
        response_identity,
        request_identity,
        protocol_identity,
        provider_measurement,
        compiler_policy_identity,
        admission_session_identity,
    ] {
        put(&mut bytes, &mut offset, &identity);
    }
    put(&mut bytes, &mut offset, &request_sequence.to_le_bytes());
    for identity in [
        begin_request_identity,
        begin_request_sha256,
        envelope_sha256,
        carriage_identity,
        current_record_sha256,
        transcript_identity,
    ] {
        put(&mut bytes, &mut offset, &identity);
    }
    debug_assert_eq!(offset, bytes.len());
    bytes
}

fn encode_header<const N: usize>(bytes: &mut [u8; N], magic: [u8; 8], code: u16) -> usize {
    let mut offset = 0;
    put(bytes, &mut offset, &magic);
    put(bytes, &mut offset, &VERSION_V1.to_le_bytes());
    put(bytes, &mut offset, &code.to_le_bytes());
    put(bytes, &mut offset, &0_u32.to_le_bytes());
    put(bytes, &mut offset, &(N as u64).to_le_bytes());
    debug_assert_eq!(offset, HEADER_BYTES);
    offset
}

fn require_header(
    reader: &mut ReaderV1<'_>,
    magic: [u8; 8],
    code: u16,
    length: usize,
) -> Result<(), ProtectedCompilerCurrentProtocolErrorV1> {
    if reader.array::<8>()? != magic
        || reader.u16()? != VERSION_V1
        || reader.u16()? != code
        || reader.u32()? != 0
        || reader.u64()? != length as u64
    {
        return Err(ProtectedCompilerCurrentProtocolErrorV1::RequestHeader);
    }
    Ok(())
}

fn require_response_header(
    reader: &mut ReaderV1<'_>,
) -> Result<u16, ProtectedCompilerCurrentProtocolErrorV1> {
    if reader.array::<8>()? != MAGIC_RESPONSE_V1 || reader.u16()? != VERSION_V1 {
        return Err(ProtectedCompilerCurrentProtocolErrorV1::ResponseHeader);
    }
    let status = reader.u16()?;
    if reader.u32()? != 0
        || reader.u64()? != PREOPENED_PROTECTED_COMPILER_CURRENT_RESPONSE_BYTES_V1 as u64
    {
        return Err(ProtectedCompilerCurrentProtocolErrorV1::ResponseHeader);
    }
    Ok(status)
}

fn put<const N: usize>(destination: &mut [u8; N], offset: &mut usize, source: &[u8]) {
    let end = offset
        .checked_add(source.len())
        .expect("fixed packet offset");
    destination[*offset..end].copy_from_slice(source);
    *offset = end;
}

fn hash_parts(parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

struct ReaderV1<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ReaderV1<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ProtectedCompilerCurrentProtocolErrorV1> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(ProtectedCompilerCurrentProtocolErrorV1::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(ProtectedCompilerCurrentProtocolErrorV1::Truncated)?;
        self.offset = end;
        Ok(value)
    }

    fn array<const N: usize>(
        &mut self,
    ) -> Result<[u8; N], ProtectedCompilerCurrentProtocolErrorV1> {
        self.take(N)?
            .try_into()
            .map_err(|_| ProtectedCompilerCurrentProtocolErrorV1::Truncated)
    }

    fn boxed_array<const N: usize>(
        &mut self,
    ) -> Result<Box<[u8; N]>, ProtectedCompilerCurrentProtocolErrorV1> {
        self.take(N)?
            .to_vec()
            .into_boxed_slice()
            .try_into()
            .map_err(|_| ProtectedCompilerCurrentProtocolErrorV1::Truncated)
    }

    fn u16(&mut self) -> Result<u16, ProtectedCompilerCurrentProtocolErrorV1> {
        self.array().map(u16::from_le_bytes)
    }

    fn u32(&mut self) -> Result<u32, ProtectedCompilerCurrentProtocolErrorV1> {
        self.array().map(u32::from_le_bytes)
    }

    fn u64(&mut self) -> Result<u64, ProtectedCompilerCurrentProtocolErrorV1> {
        self.array().map(u64::from_le_bytes)
    }

    fn is_finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

fn boxed_zero_array<const N: usize>() -> Box<[u8; N]> {
    vec![0_u8; N]
        .into_boxed_slice()
        .try_into()
        .unwrap_or_else(|_| unreachable!("fixed zero buffer has its declared length"))
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::thread;
    use std::time::Instant;

    use ed25519_dalek::{Signer, SigningKey};
    use fe2o3_external_anchor_protocol::{
        AnchorPositionV1, AnchorTransitionReceiptV1, AnchoredStateV1, CallerNonceV1,
        HashChainHeadV1, PinnedAnchorKeyV1, UnsignedAnchorObservationV1,
    };
    use fe2o3_runtime_protocol::{
        CompilerExecutionCurrentRecordAttestationV3, CompilerExecutionCurrentRecordVerificationV3,
        CompilerExecutionExternalAnchorTransactionV1, CompilerExecutionReceiptCarriageV1,
    };
    use fe2o3_worker_v3_verification_protocol::{
        WorkerV3VerificationChallengeReservationV2, WorkerV3VerificationEntryCoordinateV1,
        WorkerV3VerificationFdPayloadDescriptorV1, WorkerV3VerificationFreshChallengeV1,
        WorkerV3VerificationMeasurementIdentityV1, WorkerV3VerificationPolicyIdentityV1,
        WorkerV3VerificationRosterIdentityV1,
    };
    use rustix::fs::{MemfdFlags, Mode};
    use rustix::net::{AddressFamily, SocketFlags, SocketType, socketpair};

    use super::*;

    const ENVELOPE: &[u8] = include_bytes!("../tests/fixtures/valid-envelope-v2.bin");
    const PROTOCOL: [u8; 32] = [0xa1; 32];
    const PROVIDER: [u8; 32] = [0xa2; 32];
    const SESSION: [u8; 32] = [0xa3; 32];
    const OLD_SESSION: [u8; 32] = [0xa4; 32];
    const TRANSCRIPT: [u8; 32] = [0xa5; 32];

    struct Fixture {
        begin: WorkerV3VerificationRequestV1,
        envelope: WorkerV3LoadEnvelopeWireV2,
        current: WorkerV3VerificationCurrentRecordFrameV2,
        policy: [u8; 32],
    }

    impl Fixture {
        fn new() -> Self {
            let envelope = WorkerV3LoadEnvelopeWireV2::decode_canonical(ENVELOPE).unwrap();
            let begin = begin_request(ENVELOPE);
            let reservation =
                WorkerV3VerificationChallengeReservationV2::new([0xb1; 32], [0xb2; 32]).unwrap();
            let records = CurrentRecordsFixture::from_envelope(&envelope);
            let (verification, attestation) = records.records(*reservation.challenge_bytes());
            let current = WorkerV3VerificationCurrentRecordFrameV2::new(
                &begin,
                &reservation,
                verification.canonical_bytes(),
                attestation.canonical_bytes(),
            )
            .unwrap();
            let policy = *envelope
                .compiler_execution_receipt()
                .policy()
                .identity()
                .as_bytes();
            Self {
                begin,
                envelope,
                current,
                policy,
            }
        }

        fn request(&self, session: [u8; 32], sequence: u64) -> ProtectedCompilerCurrentRequestV1 {
            ProtectedCompilerCurrentRequestV1::new(
                PROTOCOL,
                PROVIDER,
                self.policy,
                session,
                sequence,
                &self.begin,
                &self.envelope,
                &self.current,
            )
            .unwrap()
        }
    }

    #[derive(Clone)]
    struct CurrentRecordsFixture {
        issuer: SigningKey,
        anchor: SigningKey,
        carriage: CompilerExecutionReceiptCarriageV1,
    }

    impl CurrentRecordsFixture {
        fn from_envelope(envelope: &WorkerV3LoadEnvelopeWireV2) -> Self {
            Self {
                issuer: SigningKey::from_bytes(&[0x51; 32]),
                anchor: SigningKey::from_bytes(&[0x52; 32]),
                carriage: envelope.compiler_execution_receipt().clone(),
            }
        }

        fn records(
            &self,
            challenge: [u8; 32],
        ) -> (
            CompilerExecutionCurrentRecordVerificationV3,
            CompilerExecutionCurrentRecordAttestationV3,
        ) {
            let transaction = CompilerExecutionExternalAnchorTransactionV1::new(
                self.carriage.policy().clone(),
                self.carriage.request().clone(),
                self.carriage.publication().clone(),
            )
            .unwrap();
            let anchor_key =
                PinnedAnchorKeyV1::from_bytes(self.anchor.verifying_key().to_bytes()).unwrap();
            let pending =
                AnchoredStateV1::from_local_state(0, HashChainHeadV1::from_bytes([0; 32]))
                    .prepare(transaction.external_anchor_digest(), &anchor_key)
                    .unwrap()
                    .begin_advance(CallerNonceV1::from_bytes([0xc1; 32]), &anchor_key)
                    .unwrap();
            let commit = signed_anchor_receipt(&self.anchor, &anchor_key, pending.challenge());
            let current_challenge =
                CompilerExecutionCurrentRecordVerificationV3::external_anchor_currentness_challenge(
                    &self.carriage,
                    &commit,
                    challenge,
                )
                .unwrap();
            let current = signed_anchor_receipt(&self.anchor, &anchor_key, &current_challenge);
            let verification = CompilerExecutionCurrentRecordVerificationV3::new(
                &self.carriage,
                commit,
                current,
                challenge,
                [0xc2; 32],
                [0xc3; 32],
            )
            .unwrap();
            let attestation = CompilerExecutionCurrentRecordAttestationV3::issue(
                self.carriage.policy(),
                &self.carriage,
                verification.clone(),
                challenge,
                &self.issuer,
            )
            .unwrap();
            (verification, attestation)
        }
    }

    fn signed_anchor_receipt(
        key: &SigningKey,
        pinned: &PinnedAnchorKeyV1,
        challenge: &fe2o3_external_anchor_protocol::AnchorChallengeV1,
    ) -> AnchorTransitionReceiptV1 {
        let unsigned =
            UnsignedAnchorObservationV1::from_challenge(challenge, AnchorPositionV1::Proposed);
        let signature = key.sign(&unsigned.signing_bytes()).to_bytes();
        AnchorTransitionReceiptV1::new(
            challenge.clone(),
            &unsigned.attach_signature(signature),
            pinned,
        )
        .unwrap()
    }

    fn begin_request(envelope: &[u8]) -> WorkerV3VerificationRequestV1 {
        WorkerV3VerificationRequestV1::new(
            WorkerV3VerificationFreshChallengeV1::new([0xd1; 32]).unwrap(),
            WorkerV3VerificationRosterIdentityV1::new([0xd2; 32]).unwrap(),
            WorkerV3VerificationPolicyIdentityV1::new([0xd3; 32]).unwrap(),
            WorkerV3VerificationMeasurementIdentityV1::new([0xd4; 32]).unwrap(),
            WorkerV3VerificationFdPayloadDescriptorV1::load_envelope_v2(
                envelope.len() as u64,
                sha256(envelope),
            )
            .unwrap(),
            WorkerV3VerificationFdPayloadDescriptorV1::finalized_hsaco(32, [0xd5; 32]).unwrap(),
            vec![
                WorkerV3VerificationEntryCoordinateV1::new(
                    0, "logical", "export", [0xd6; 32], [0xd7; 32], [0xd8; 32],
                )
                .unwrap(),
            ],
        )
        .unwrap()
    }

    fn pair(kind: SocketType) -> (OwnedFd, OwnedFd) {
        socketpair(
            AddressFamily::UNIX,
            kind,
            SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
            None,
        )
        .unwrap()
    }

    fn endpoint(peer: &OwnedFd) -> ProtectedCompilerCurrentEndpointIdentityV1 {
        let stat = rustix::fs::fstat(peer).unwrap();
        ProtectedCompilerCurrentEndpointIdentityV1::new(stat.st_dev, stat.st_ino).unwrap()
    }

    fn expected_peer() -> ProtectedCompilerCurrentPeerIdentityV1 {
        ProtectedCompilerCurrentPeerIdentityV1::new(
            std::process::id(),
            rustix::process::getuid().as_raw(),
            rustix::process::getgid().as_raw(),
        )
        .unwrap()
    }

    fn admit_test_client_for_session(
        peer: OwnedFd,
        session: [u8; 32],
        policy: [u8; 32],
    ) -> PreopenedProtectedCompilerCurrentClientV1 {
        let endpoint = endpoint(&peer);
        PreopenedProtectedCompilerCurrentClientV1::admit_inner::<false>(
            peer,
            endpoint,
            expected_peer(),
            PROTOCOL,
            PROVIDER,
            policy,
            session,
        )
        .unwrap()
    }

    fn deadline(duration: Duration) -> AbsoluteSessionDeadlineV1 {
        AbsoluteSessionDeadlineV1::after(duration).unwrap()
    }

    fn receive_request(peer: &OwnedFd) -> ProtectedCompilerCurrentRequestV1 {
        wait_for_peer(peer, libc::POLLIN, deadline(Duration::from_secs(2))).unwrap();
        let mut bytes = boxed_zero_array::<PREOPENED_PROTECTED_COMPILER_CURRENT_REQUEST_BYTES_V1>();
        // SAFETY: bytes is writable for the exact request bound.
        let received = unsafe {
            libc::recv(
                peer.as_raw_fd(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
                libc::MSG_DONTWAIT,
            )
        };
        assert_eq!(usize::try_from(received).unwrap(), bytes.len());
        ProtectedCompilerCurrentRequestV1::decode_canonical(bytes.as_slice()).unwrap()
    }

    fn send_bytes(peer: &OwnedFd, bytes: &[u8]) {
        wait_for_peer(peer, libc::POLLOUT, deadline(Duration::from_secs(2))).unwrap();
        // SAFETY: bytes remains readable for the complete send.
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

    fn authenticate(
        client: &mut PreopenedProtectedCompilerCurrentClientV1,
        fixture: &Fixture,
        operation_deadline: AbsoluteSessionDeadlineV1,
    ) -> Result<AuthenticatedCompilerCurrentRecordV1, ProtectedCompilerCurrentClientFailureV1> {
        client.authenticate_parts::<false>(
            &fixture.begin,
            &fixture.envelope,
            &fixture.current,
            operation_deadline,
        )
    }

    fn expect_failure(
        result: Result<
            AuthenticatedCompilerCurrentRecordV1,
            ProtectedCompilerCurrentClientFailureV1,
        >,
    ) -> ProtectedCompilerCurrentClientFailureV1 {
        match result {
            Err(failure) => failure,
            Ok(_) => panic!("protected compiler-current request unexpectedly succeeded"),
        }
    }

    fn spawn_response(
        peer: OwnedFd,
        response: impl FnOnce(&ProtectedCompilerCurrentRequestV1) -> Vec<u8> + Send + 'static,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let request = receive_request(&peer);
            send_bytes(&peer, &response(&request));
        })
    }

    #[test]
    fn fixed_packets_round_trip_and_bind_complete_nested_records() {
        let fixture = Fixture::new();
        let request = fixture.request(SESSION, 1);
        assert_eq!(
            request.encode_canonical().len(),
            PREOPENED_PROTECTED_COMPILER_CURRENT_REQUEST_BYTES_V1
        );
        assert_eq!(
            request.begin_request_bytes(),
            fixture.begin.encode_canonical()
        );
        assert_eq!(
            request.carriage_bytes(),
            fixture
                .envelope
                .compiler_execution_receipt()
                .canonical_bytes()
        );
        assert_eq!(
            request.current_record_bytes(),
            fixture.current.encode_canonical()
        );
        assert_eq!(request.envelope_length(), ENVELOPE.len() as u64);
        assert_eq!(request.envelope_sha256(), sha256(ENVELOPE));
        assert_eq!(
            ProtectedCompilerCurrentRequestV1::decode_canonical(request.encode_canonical())
                .unwrap(),
            request
        );
        let accepted =
            ProtectedCompilerCurrentResponseV1::authenticated(&request, TRANSCRIPT).unwrap();
        assert_eq!(
            accepted.encode_canonical().len(),
            PREOPENED_PROTECTED_COMPILER_CURRENT_RESPONSE_BYTES_V1
        );
        assert_eq!(
            ProtectedCompilerCurrentResponseV1::decode_canonical(accepted.encode_canonical())
                .unwrap(),
            accepted
        );
        assert!(accepted.matches_request(&request));
        assert!(!request.grants_authority());
        assert!(!accepted.grants_authority());
    }

    #[test]
    fn protocol_rejects_zero_context_bad_padding_and_nested_substitution() {
        let fixture = Fixture::new();
        for context in 0..4 {
            let mut coordinates = [PROTOCOL, PROVIDER, fixture.policy, SESSION];
            coordinates[context] = [0; 32];
            assert!(
                ProtectedCompilerCurrentRequestV1::new(
                    coordinates[0],
                    coordinates[1],
                    coordinates[2],
                    coordinates[3],
                    1,
                    &fixture.begin,
                    &fixture.envelope,
                    &fixture.current,
                )
                .is_err()
            );
        }
        assert!(
            ProtectedCompilerCurrentRequestV1::new(
                PROTOCOL,
                PROVIDER,
                fixture.policy,
                SESSION,
                0,
                &fixture.begin,
                &fixture.envelope,
                &fixture.current,
            )
            .is_err()
        );
        let request = fixture.request(SESSION, 1);
        let mut padding = *request.encode_canonical();
        let padding_offset = HEADER_BYTES + (8 * IDENTITY_BYTES) + 8 + 8 + (2 * IDENTITY_BYTES) + 4;
        padding[padding_offset] = 1;
        assert!(matches!(
            ProtectedCompilerCurrentRequestV1::decode_canonical(&padding),
            Err(ProtectedCompilerCurrentProtocolErrorV1::ReservedBytes)
        ));
        let mut carriage = *request.encode_canonical();
        let carriage_offset = PREOPENED_PROTECTED_COMPILER_CURRENT_REQUEST_BYTES_V1
            - COMPILER_EXECUTION_RECEIPT_CARRIAGE_BYTES_V1
            - WORKER_V3_VERIFICATION_CURRENT_RECORD_BYTES_V2;
        carriage[carriage_offset] ^= 1;
        assert!(ProtectedCompilerCurrentRequestV1::decode_canonical(&carriage).is_err());
        let mut current = *request.encode_canonical();
        current[PREOPENED_PROTECTED_COMPILER_CURRENT_REQUEST_BYTES_V1 - 1] ^= 1;
        assert!(ProtectedCompilerCurrentRequestV1::decode_canonical(&current).is_err());
        assert!(matches!(
            ProtectedCompilerCurrentResponseV1::authenticated(&request, [0; 32]),
            Err(ProtectedCompilerCurrentProtocolErrorV1::TranscriptShape)
        ));
    }

    #[test]
    fn admission_rejects_flags_type_peer_endpoint_and_same_uid_with_fd_retained() {
        let fixture = Fixture::new();
        let (peer, provider) = pair(SocketType::SEQPACKET);
        let raw = peer.as_raw_fd();
        let failure = unsafe {
            PreopenedProtectedCompilerCurrentClientV1::admit_from_supervisor(
                peer,
                endpoint(&provider),
                expected_peer(),
                PROTOCOL,
                PROVIDER,
                fixture.policy,
                SESSION,
            )
        }
        .unwrap_err();
        assert_eq!(failure.into_peer().as_raw_fd(), raw);

        let (stream, _other) = pair(SocketType::STREAM);
        let stream_endpoint = endpoint(&stream);
        let failure = PreopenedProtectedCompilerCurrentClientV1::admit_inner::<false>(
            stream,
            stream_endpoint,
            expected_peer(),
            PROTOCOL,
            PROVIDER,
            fixture.policy,
            SESSION,
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedCompilerCurrentClientAdmissionErrorV1::SocketShape
        ));

        let (peer, _provider) = socketpair(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();
        let identity = endpoint(&peer);
        assert!(matches!(
            PreopenedProtectedCompilerCurrentClientV1::admit_inner::<false>(
                peer,
                identity,
                expected_peer(),
                PROTOCOL,
                PROVIDER,
                fixture.policy,
                SESSION,
            )
            .unwrap_err()
            .error(),
            ProtectedCompilerCurrentClientAdmissionErrorV1::DescriptorShape
        ));

        let (peer, _provider) = pair(SocketType::SEQPACKET);
        let identity = endpoint(&peer);
        let wrong_peer = ProtectedCompilerCurrentPeerIdentityV1::new(
            expected_peer().pid().checked_add(1).unwrap(),
            expected_peer().uid(),
            expected_peer().gid(),
        )
        .unwrap();
        assert!(matches!(
            PreopenedProtectedCompilerCurrentClientV1::admit_inner::<false>(
                peer,
                identity,
                wrong_peer,
                PROTOCOL,
                PROVIDER,
                fixture.policy,
                SESSION,
            )
            .unwrap_err()
            .error(),
            ProtectedCompilerCurrentClientAdmissionErrorV1::PeerIdentityMismatch
        ));

        for coordinate in 0..4 {
            let (peer, _provider) = pair(SocketType::SEQPACKET);
            let identity = endpoint(&peer);
            let mut values = [PROTOCOL, PROVIDER, fixture.policy, SESSION];
            values[coordinate] = [0; 32];
            assert!(
                PreopenedProtectedCompilerCurrentClientV1::admit_inner::<false>(
                    peer,
                    identity,
                    expected_peer(),
                    values[0],
                    values[1],
                    values[2],
                    values[3],
                )
                .is_err()
            );
        }

        let (peer, _provider) = pair(SocketType::SEQPACKET);
        let identity = endpoint(&peer);
        let failure = unsafe {
            PreopenedProtectedCompilerCurrentClientV1::admit_from_supervisor(
                peer,
                identity,
                expected_peer(),
                PROTOCOL,
                PROVIDER,
                fixture.policy,
                SESSION,
            )
        }
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedCompilerCurrentClientAdmissionErrorV1::ProviderAndVerifierUidMatch
        ));
    }

    #[test]
    fn exact_authentication_mints_nonzero_transcript() {
        let fixture = Fixture::new();
        let (peer, provider) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client_for_session(peer, SESSION, fixture.policy);
        let server = spawn_response(provider, |request| {
            ProtectedCompilerCurrentResponseV1::authenticated(request, TRANSCRIPT)
                .unwrap()
                .encode_canonical()
                .to_vec()
        });
        let authenticated =
            authenticate(&mut client, &fixture, deadline(Duration::from_secs(2))).unwrap();
        assert_eq!(authenticated.transcript_identity(), TRANSCRIPT);
        assert!(!client.is_poisoned());
        server.join().unwrap();
    }

    #[test]
    fn correlated_rejection_retains_sync_and_next_sequence_succeeds() {
        let fixture = Fixture::new();
        let (peer, provider) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client_for_session(peer, SESSION, fixture.policy);
        let server = thread::spawn(move || {
            let first = receive_request(&provider);
            assert_eq!(first.request_sequence(), 1);
            send_bytes(
                &provider,
                ProtectedCompilerCurrentResponseV1::rejected(&first).encode_canonical(),
            );
            let second = receive_request(&provider);
            assert_eq!(second.request_sequence(), 2);
            send_bytes(
                &provider,
                ProtectedCompilerCurrentResponseV1::authenticated(&second, TRANSCRIPT)
                    .unwrap()
                    .encode_canonical(),
            );
        });
        let failure = expect_failure(authenticate(
            &mut client,
            &fixture,
            deadline(Duration::from_secs(2)),
        ));
        assert!(matches!(
            failure.error(),
            ProtectedCompilerCurrentClientErrorV1::ProviderRejected
        ));
        assert_eq!(
            failure.custody(),
            ProtectedCompilerCurrentClientCustodyV1::Retained
        );
        let success =
            authenticate(&mut client, &fixture, deadline(Duration::from_secs(2))).unwrap();
        assert_eq!(success.transcript_identity(), TRANSCRIPT);
        server.join().unwrap();
    }

    #[test]
    fn cross_admission_old_session_sequence_one_prequeue_poisons() {
        let fixture = Fixture::new();
        let (peer, provider) = pair(SocketType::SEQPACKET);
        let duplicate = rustix::io::fcntl_dupfd_cloexec(&peer, 0).unwrap();
        let old_client = admit_test_client_for_session(peer, OLD_SESSION, fixture.policy);
        let old_request = fixture.request(OLD_SESSION, 1);
        let old_response =
            ProtectedCompilerCurrentResponseV1::authenticated(&old_request, TRANSCRIPT).unwrap();
        send_bytes(&provider, old_response.encode_canonical());
        drop(old_client);
        let mut client = admit_test_client_for_session(duplicate, SESSION, fixture.policy);
        let failure = expect_failure(authenticate(
            &mut client,
            &fixture,
            deadline(Duration::from_secs(2)),
        ));
        assert!(matches!(
            failure.error(),
            ProtectedCompilerCurrentClientErrorV1::ResponseRequestMismatch
        ));
        assert_eq!(
            failure.custody(),
            ProtectedCompilerCurrentClientCustodyV1::Poisoned
        );
        assert!(client.is_poisoned());
        let queued_new_request = receive_request(&provider);
        assert_eq!(queued_new_request.admission_session_identity(), SESSION);
    }

    #[test]
    fn recomputed_response_coordinate_substitutions_poison() {
        let fixture = Fixture::new();
        for coordinate in 0..11 {
            let (peer, provider) = pair(SocketType::SEQPACKET);
            let mut client = admit_test_client_for_session(peer, SESSION, fixture.policy);
            let server = spawn_response(provider, move |request| {
                let mut fields = [
                    request.request_identity,
                    request.protocol_identity,
                    request.provider_measurement,
                    request.compiler_policy_identity,
                    request.admission_session_identity,
                    request.begin_request_identity,
                    request.begin_request_sha256,
                    request.envelope_sha256,
                    request.carriage_identity,
                    request.current_record_sha256,
                ];
                let sequence = if coordinate == 10 {
                    request.request_sequence + 1
                } else {
                    fields[coordinate] = [u8::try_from(coordinate).unwrap() + 1; 32];
                    request.request_sequence
                };
                ProtectedCompilerCurrentResponseV1::from_fields(
                    ProtectedCompilerCurrentResponseStatusV1::Authenticated,
                    fields[0],
                    fields[1],
                    fields[2],
                    fields[3],
                    fields[4],
                    sequence,
                    fields[5],
                    fields[6],
                    fields[7],
                    fields[8],
                    fields[9],
                    TRANSCRIPT,
                )
                .encode_canonical()
                .to_vec()
            });
            let failure = expect_failure(authenticate(
                &mut client,
                &fixture,
                deadline(Duration::from_secs(2)),
            ));
            assert!(matches!(
                failure.error(),
                ProtectedCompilerCurrentClientErrorV1::ResponseRequestMismatch
            ));
            assert_eq!(
                failure.custody(),
                ProtectedCompilerCurrentClientCustodyV1::Poisoned
            );
            assert!(client.is_poisoned());
            server.join().unwrap();
        }
    }

    #[test]
    fn replayed_sequence_response_and_malformed_status_poison_permanently() {
        let fixture = Fixture::new();
        let (peer, provider) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client_for_session(peer, SESSION, fixture.policy);
        let server = thread::spawn(move || {
            let first = receive_request(&provider);
            let response = *ProtectedCompilerCurrentResponseV1::authenticated(&first, TRANSCRIPT)
                .unwrap()
                .encode_canonical();
            send_bytes(&provider, &response);
            let second = receive_request(&provider);
            assert_eq!(second.request_sequence(), 2);
            send_bytes(&provider, &response);
        });
        authenticate(&mut client, &fixture, deadline(Duration::from_secs(2))).unwrap();
        let failure = expect_failure(authenticate(
            &mut client,
            &fixture,
            deadline(Duration::from_secs(2)),
        ));
        assert!(matches!(
            failure.error(),
            ProtectedCompilerCurrentClientErrorV1::ResponseRequestMismatch
        ));
        assert!(client.is_poisoned());
        assert!(matches!(
            expect_failure(authenticate(
                &mut client,
                &fixture,
                deadline(Duration::from_secs(2)),
            ))
            .error(),
            ProtectedCompilerCurrentClientErrorV1::Poisoned
        ));
        server.join().unwrap();

        let (peer, provider) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client_for_session(peer, SESSION, fixture.policy);
        let server = spawn_response(provider, |request| {
            let mut bytes =
                *ProtectedCompilerCurrentResponseV1::rejected(request).encode_canonical();
            bytes[10..12].copy_from_slice(&99_u16.to_le_bytes());
            bytes.to_vec()
        });
        assert!(matches!(
            expect_failure(authenticate(
                &mut client,
                &fixture,
                deadline(Duration::from_secs(2)),
            ))
            .error(),
            ProtectedCompilerCurrentClientErrorV1::Protocol(
                ProtectedCompilerCurrentProtocolErrorV1::ResponseStatus { .. }
            )
        ));
        assert!(client.is_poisoned());
        server.join().unwrap();
    }

    #[test]
    fn deadline_lower_upper_and_expired_paths_use_only_supplied_absolute_deadline() {
        assert_eq!(duration_to_poll_millis(Duration::from_nanos(1)), 1);
        assert_eq!(duration_to_poll_millis(Duration::from_millis(1)), 1);
        assert_eq!(duration_to_poll_millis(Duration::MAX), i32::MAX);

        let fixture = Fixture::new();
        let (peer, _provider) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client_for_session(peer, SESSION, fixture.policy);
        let expired = deadline(Duration::from_nanos(1));
        thread::sleep(Duration::from_millis(1));
        let failure = expect_failure(authenticate(&mut client, &fixture, expired));
        assert!(matches!(
            failure.error(),
            ProtectedCompilerCurrentClientErrorV1::DeadlineExpired
        ));
        assert_eq!(
            failure.custody(),
            ProtectedCompilerCurrentClientCustodyV1::Retained
        );
        assert!(!client.is_poisoned());

        let (peer, provider) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client_for_session(peer, SESSION, fixture.policy);
        let server = thread::spawn(move || {
            let _request = receive_request(&provider);
            thread::sleep(Duration::from_secs(1));
        });
        let start = Instant::now();
        let failure = expect_failure(authenticate(
            &mut client,
            &fixture,
            deadline(Duration::from_millis(500)),
        ));
        assert!(matches!(
            failure.error(),
            ProtectedCompilerCurrentClientErrorV1::DeadlineExpired
        ));
        assert!(start.elapsed() < Duration::from_secs(1));
        assert!(client.is_poisoned());
        server.join().unwrap();

        let (peer, provider) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client_for_session(peer, SESSION, fixture.policy);
        let server = spawn_response(provider, |request| {
            ProtectedCompilerCurrentResponseV1::authenticated(request, TRANSCRIPT)
                .unwrap()
                .encode_canonical()
                .to_vec()
        });
        let above_poll_bound = Duration::from_secs(u64::from(i32::MAX.unsigned_abs()) + 1);
        authenticate(&mut client, &fixture, deadline(above_poll_bound)).unwrap();
        server.join().unwrap();
    }

    #[test]
    fn post_admission_flag_peer_and_endpoint_substitution_poison() {
        let fixture = Fixture::new();
        for substitution in 0..3 {
            let (peer, _provider) = pair(SocketType::SEQPACKET);
            let mut client = admit_test_client_for_session(peer, SESSION, fixture.policy);
            match substitution {
                0 => {
                    let fd = client.peer.as_ref().unwrap();
                    rustix::fs::fcntl_setfl(fd, OFlags::RDWR).unwrap();
                }
                1 => {
                    client.expected_peer.pid = client.expected_peer.pid.checked_add(1).unwrap();
                }
                _ => {
                    client.endpoint.inode = client.endpoint.inode.checked_add(1).unwrap();
                }
            }
            let failure = expect_failure(authenticate(
                &mut client,
                &fixture,
                deadline(Duration::from_secs(2)),
            ));
            assert!(matches!(
                failure.error(),
                ProtectedCompilerCurrentClientErrorV1::EndpointRevalidation(_)
            ));
            assert_eq!(
                failure.custody(),
                ProtectedCompilerCurrentClientCustodyV1::Poisoned
            );
            assert!(client.is_poisoned());
        }
    }

    #[test]
    fn short_oversize_and_malformed_responses_poison() {
        let fixture = Fixture::new();
        for kind in 0..3 {
            let (peer, provider) = pair(SocketType::SEQPACKET);
            let mut client = admit_test_client_for_session(peer, SESSION, fixture.policy);
            let server = spawn_response(provider, move |request| {
                let mut response =
                    ProtectedCompilerCurrentResponseV1::authenticated(request, TRANSCRIPT)
                        .unwrap()
                        .encode_canonical()
                        .to_vec();
                match kind {
                    0 => {
                        response.pop();
                    }
                    1 => response.push(0),
                    _ => response[0] ^= 1,
                }
                response
            });
            let failure = expect_failure(authenticate(
                &mut client,
                &fixture,
                deadline(Duration::from_secs(2)),
            ));
            match kind {
                0 => assert!(matches!(
                    failure.error(),
                    ProtectedCompilerCurrentClientErrorV1::ResponseLength { .. }
                )),
                1 => assert!(matches!(
                    failure.error(),
                    ProtectedCompilerCurrentClientErrorV1::PacketTruncated
                )),
                _ => assert!(matches!(
                    failure.error(),
                    ProtectedCompilerCurrentClientErrorV1::Protocol(
                        ProtectedCompilerCurrentProtocolErrorV1::ResponseHeader
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
        // SAFETY: all-zero storage is valid backing memory for cmsghdr control data.
        let mut control: [libc::cmsghdr; 2] = unsafe { mem::zeroed() };
        // SAFETY: zero initializes a valid msghdr and pointers are installed below.
        let mut header = unsafe { mem::zeroed::<libc::msghdr>() };
        header.msg_iov = &raw mut vector;
        header.msg_iovlen = 1;
        header.msg_control = control.as_mut_ptr().cast();
        header.msg_controllen = control_bytes;
        // SAFETY: header has room for one descriptor control message.
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
        let fixture = Fixture::new();
        let (peer, provider) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client_for_session(peer, SESSION, fixture.policy);
        let server = thread::spawn(move || {
            let request = receive_request(&provider);
            let response =
                ProtectedCompilerCurrentResponseV1::authenticated(&request, TRANSCRIPT).unwrap();
            send_ancillary(&provider, response.encode_canonical());
        });
        let failure = expect_failure(authenticate(
            &mut client,
            &fixture,
            deadline(Duration::from_secs(2)),
        ));
        assert!(matches!(
            failure.error(),
            ProtectedCompilerCurrentClientErrorV1::AncillaryData
        ));
        assert!(client.is_poisoned());
        server.join().unwrap();
    }

    #[test]
    fn compiler_policy_and_envelope_descriptor_substitution_fail_before_send() {
        let fixture = Fixture::new();
        assert!(matches!(
            ProtectedCompilerCurrentRequestV1::new(
                PROTOCOL,
                PROVIDER,
                [0xee; 32],
                SESSION,
                1,
                &fixture.begin,
                &fixture.envelope,
                &fixture.current,
            ),
            Err(ProtectedCompilerCurrentProtocolErrorV1::CompilerPolicyMismatch)
        ));
        let wrong_begin = begin_request(b"wrong envelope bytes");
        assert!(matches!(
            ProtectedCompilerCurrentRequestV1::new(
                PROTOCOL,
                PROVIDER,
                fixture.policy,
                SESSION,
                1,
                &wrong_begin,
                &fixture.envelope,
                &fixture.current,
            ),
            Err(ProtectedCompilerCurrentProtocolErrorV1::EnvelopeLength
                | ProtectedCompilerCurrentProtocolErrorV1::EnvelopeDigest)
        ));
    }

    #[test]
    fn descriptor_peer_close_after_send_is_ambiguous_and_poisons() {
        let fixture = Fixture::new();
        let (peer, provider) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client_for_session(peer, SESSION, fixture.policy);
        let server = thread::spawn(move || {
            let _request = receive_request(&provider);
        });
        let failure = expect_failure(authenticate(
            &mut client,
            &fixture,
            deadline(Duration::from_secs(2)),
        ));
        assert!(matches!(
            failure.error(),
            ProtectedCompilerCurrentClientErrorV1::PeerClosed
                | ProtectedCompilerCurrentClientErrorV1::Receive(_)
        ));
        assert!(client.is_poisoned());
        server.join().unwrap();
    }

    #[test]
    fn non_socket_descriptor_is_rejected_with_exact_custody() {
        let descriptor =
            rustix::fs::memfd_create("current-record-admission", MemfdFlags::CLOEXEC).unwrap();
        rustix::fs::fchmod(&descriptor, Mode::RUSR | Mode::WUSR).unwrap();
        let stat = rustix::fs::fstat(&descriptor).unwrap();
        let endpoint =
            ProtectedCompilerCurrentEndpointIdentityV1::new(stat.st_dev, stat.st_ino).unwrap();
        let raw = descriptor.as_raw_fd();
        let failure = PreopenedProtectedCompilerCurrentClientV1::admit_inner::<false>(
            descriptor,
            endpoint,
            expected_peer(),
            PROTOCOL,
            PROVIDER,
            [0xee; 32],
            SESSION,
        )
        .unwrap_err();
        let retained = failure.into_peer();
        assert_eq!(retained.as_raw_fd(), raw);
        let _file = File::from(retained);
    }
}
