//! Descriptor-only IPC client for an externally protected antirollback head store.
//!
//! One client is pinned to one `(policy, kind, namespace)` and one measured
//! provider. It consumes an already-connected Unix `SOCK_SEQPACKET` endpoint,
//! uses fixed canonical packets, and permits only initialize-at-zero or exact
//! single-record compare-and-advance transitions. An ambiguous outcome after a
//! send attempt closes the endpoint and permanently poisons the client.

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

use rustix::fs::{FileType, OFlags};
use rustix::io::FdFlags;
use sha2::{Digest, Sha256};

use crate::{
    AbsoluteSessionDeadlineV1, ProtectedLedgerExternalHeadV1, ProtectedLedgerHeadStoreFailureV1,
    ProtectedLedgerHeadStoreV1, ProtectedLedgerKindV1,
};

const MAGIC_REQUEST_V1: [u8; 8] = *b"F3HSRQ1\0";
const MAGIC_RESPONSE_V1: [u8; 8] = *b"F3HSRS1\0";
const VERSION_V1: u16 = 1;
const HEADER_BYTES: usize = 20;
const IDENTITY_BYTES: usize = 32;
const KIND_BLOCK_BYTES: usize = 16;
const HEAD_BYTES: usize = 100;
const INVALID_LINUX_ID: u32 = u32::MAX;
const MAX_OPERATION_TIMEOUT: Duration = Duration::from_mins(1);
const REQUEST_IDENTITY_DOMAIN_V1: &[u8] = b"FERRIC/PROTECTED-HEAD-STORE/REQUEST/V1\0";
const RESPONSE_IDENTITY_DOMAIN_V1: &[u8] = b"FERRIC/PROTECTED-HEAD-STORE/RESPONSE/V1\0";
const REQUEST_DECLARED_BYTES_V1: u32 = 396;
const RESPONSE_DECLARED_BYTES_V1: u32 = 328;

/// Exact byte length of one protected head-store request packet.
pub const PREOPENED_PROTECTED_HEAD_STORE_REQUEST_BYTES_V1: usize =
    HEADER_BYTES + (5 * IDENTITY_BYTES) + KIND_BLOCK_BYTES + (2 * HEAD_BYTES);

/// Exact byte length of one protected head-store response packet.
pub const PREOPENED_PROTECTED_HEAD_STORE_RESPONSE_BYTES_V1: usize =
    HEADER_BYTES + (6 * IDENTITY_BYTES) + KIND_BLOCK_BYTES + HEAD_BYTES;

const _: () = assert!(PREOPENED_PROTECTED_HEAD_STORE_REQUEST_BYTES_V1 == 396);
const _: () = assert!(PREOPENED_PROTECTED_HEAD_STORE_RESPONSE_BYTES_V1 == 328);

/// Supervisor-pinned identity of one preopened head-store socket endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtectedHeadStoreEndpointIdentityV1 {
    device: u64,
    inode: u64,
}

impl ProtectedHeadStoreEndpointIdentityV1 {
    /// Constructs one nonzero supervisor-pinned socket identity.
    ///
    /// # Errors
    ///
    /// Returns a typed error if either coordinate is zero.
    pub const fn new(
        device: u64,
        inode: u64,
    ) -> Result<Self, ProtectedHeadStoreEndpointIdentityErrorV1> {
        if device == 0 {
            return Err(ProtectedHeadStoreEndpointIdentityErrorV1::ZeroDevice);
        }
        if inode == 0 {
            return Err(ProtectedHeadStoreEndpointIdentityErrorV1::ZeroInode);
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

    /// Endpoint metadata grants no antirollback or verifier authority.
    pub const fn grants_authority(self) -> bool {
        false
    }
}

/// Invalid supervisor-pinned head-store endpoint identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedHeadStoreEndpointIdentityErrorV1 {
    /// Socket device number was zero.
    ZeroDevice,
    /// Socket inode number was zero.
    ZeroInode,
}

impl fmt::Display for ProtectedHeadStoreEndpointIdentityErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid protected head-store endpoint: {self:?}")
    }
}

impl Error for ProtectedHeadStoreEndpointIdentityErrorV1 {}

/// Supervisor-pinned kernel credentials for the protected head-store peer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtectedHeadStorePeerIdentityV1 {
    pid: u32,
    uid: u32,
    gid: u32,
}

impl ProtectedHeadStorePeerIdentityV1 {
    /// Constructs one positive, dedicated non-root provider identity.
    ///
    /// # Errors
    ///
    /// Returns a typed error for zero/root or Linux invalid-ID coordinates.
    pub const fn new(
        pid: u32,
        uid: u32,
        gid: u32,
    ) -> Result<Self, ProtectedHeadStorePeerIdentityErrorV1> {
        if pid == 0 {
            return Err(ProtectedHeadStorePeerIdentityErrorV1::InvalidPid);
        }
        if uid == 0 || uid == INVALID_LINUX_ID {
            return Err(ProtectedHeadStorePeerIdentityErrorV1::InvalidUid);
        }
        if gid == 0 || gid == INVALID_LINUX_ID {
            return Err(ProtectedHeadStorePeerIdentityErrorV1::InvalidGid);
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

    /// Peer metadata grants no antirollback or verifier authority.
    pub const fn grants_authority(self) -> bool {
        false
    }
}

/// Invalid protected head-store peer identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedHeadStorePeerIdentityErrorV1 {
    /// PID was not positive.
    InvalidPid,
    /// UID was root or the Linux invalid-ID sentinel.
    InvalidUid,
    /// GID was root or the Linux invalid-ID sentinel.
    InvalidGid,
}

impl fmt::Display for ProtectedHeadStorePeerIdentityErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid protected head-store peer: {self:?}")
    }
}

impl Error for ProtectedHeadStorePeerIdentityErrorV1 {}

/// Sole operation admitted by a protected head-store request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedHeadStoreOperationV1 {
    /// Load the current durable head.
    Load,
    /// Install the initial zero-record head if the namespace never existed.
    Initialize,
    /// Replace exactly one current head with its one-record successor.
    CompareAndAdvance,
}

impl ProtectedHeadStoreOperationV1 {
    const fn code(self) -> u16 {
        match self {
            Self::Load => 1,
            Self::Initialize => 2,
            Self::CompareAndAdvance => 3,
        }
    }

    fn decode(code: u16) -> Result<Self, ProtectedHeadStoreProtocolErrorV1> {
        match code {
            1 => Ok(Self::Load),
            2 => Ok(Self::Initialize),
            3 => Ok(Self::CompareAndAdvance),
            actual => Err(ProtectedHeadStoreProtocolErrorV1::Operation { actual }),
        }
    }
}

/// Exact fixed-width protected head-store request.
#[derive(Clone, Eq, PartialEq)]
pub struct ProtectedHeadStoreRequestV1 {
    operation: ProtectedHeadStoreOperationV1,
    request_identity: [u8; 32],
    protocol_identity: [u8; 32],
    provider_identity: [u8; 32],
    namespace_identity: [u8; 32],
    policy_identity: [u8; 32],
    kind: ProtectedLedgerKindV1,
    request_sequence: u64,
    current: Option<ProtectedLedgerExternalHeadV1>,
    next: Option<ProtectedLedgerExternalHeadV1>,
    canonical_bytes: [u8; PREOPENED_PROTECTED_HEAD_STORE_REQUEST_BYTES_V1],
}

impl ProtectedHeadStoreRequestV1 {
    /// Constructs a canonical load request.
    ///
    /// # Errors
    ///
    /// Returns a typed error for any zero context coordinate.
    pub fn load(
        protocol_identity: [u8; 32],
        provider_identity: [u8; 32],
        namespace_identity: [u8; 32],
        policy_identity: [u8; 32],
        kind: ProtectedLedgerKindV1,
        request_sequence: u64,
    ) -> Result<Self, ProtectedHeadStoreProtocolErrorV1> {
        Self::new(
            ProtectedHeadStoreOperationV1::Load,
            protocol_identity,
            provider_identity,
            namespace_identity,
            policy_identity,
            kind,
            request_sequence,
            None,
            None,
        )
    }

    /// Constructs a canonical initialize-at-zero request.
    ///
    /// # Errors
    ///
    /// Returns a typed error unless `initial` is an empty head under the pinned policy.
    pub fn initialize(
        protocol_identity: [u8; 32],
        provider_identity: [u8; 32],
        namespace_identity: [u8; 32],
        policy_identity: [u8; 32],
        kind: ProtectedLedgerKindV1,
        request_sequence: u64,
        initial: ProtectedLedgerExternalHeadV1,
    ) -> Result<Self, ProtectedHeadStoreProtocolErrorV1> {
        Self::new(
            ProtectedHeadStoreOperationV1::Initialize,
            protocol_identity,
            provider_identity,
            namespace_identity,
            policy_identity,
            kind,
            request_sequence,
            None,
            Some(initial),
        )
    }

    /// Constructs a canonical exact single-record compare-and-advance request.
    ///
    /// # Errors
    ///
    /// Returns a typed error unless policy and header are unchanged and `next`
    /// advances the record count by exactly one with a nonzero new record identity.
    pub fn compare_and_advance(
        protocol_identity: [u8; 32],
        provider_identity: [u8; 32],
        namespace_identity: [u8; 32],
        policy_identity: [u8; 32],
        kind: ProtectedLedgerKindV1,
        request_sequence: u64,
        current: ProtectedLedgerExternalHeadV1,
        next: ProtectedLedgerExternalHeadV1,
    ) -> Result<Self, ProtectedHeadStoreProtocolErrorV1> {
        Self::new(
            ProtectedHeadStoreOperationV1::CompareAndAdvance,
            protocol_identity,
            provider_identity,
            namespace_identity,
            policy_identity,
            kind,
            request_sequence,
            Some(current),
            Some(next),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        operation: ProtectedHeadStoreOperationV1,
        protocol_identity: [u8; 32],
        provider_identity: [u8; 32],
        namespace_identity: [u8; 32],
        policy_identity: [u8; 32],
        kind: ProtectedLedgerKindV1,
        request_sequence: u64,
        current: Option<ProtectedLedgerExternalHeadV1>,
        next: Option<ProtectedLedgerExternalHeadV1>,
    ) -> Result<Self, ProtectedHeadStoreProtocolErrorV1> {
        validate_context(
            protocol_identity,
            provider_identity,
            namespace_identity,
            policy_identity,
        )?;
        if request_sequence == 0 {
            return Err(ProtectedHeadStoreProtocolErrorV1::ZeroRequestSequence);
        }
        validate_transition(operation, policy_identity, current, next)?;
        let current_bytes = encode_optional_head(current);
        let next_bytes = encode_optional_head(next);
        let request_identity = request_identity(
            operation,
            protocol_identity,
            provider_identity,
            namespace_identity,
            policy_identity,
            kind,
            request_sequence,
            &current_bytes,
            &next_bytes,
        );
        let canonical_bytes = encode_request(
            operation,
            request_identity,
            protocol_identity,
            provider_identity,
            namespace_identity,
            policy_identity,
            kind,
            request_sequence,
            &current_bytes,
            &next_bytes,
        );
        Ok(Self {
            operation,
            request_identity,
            protocol_identity,
            provider_identity,
            namespace_identity,
            policy_identity,
            kind,
            request_sequence,
            current,
            next,
            canonical_bytes,
        })
    }

    /// Strictly decodes one complete canonical request packet.
    ///
    /// # Errors
    ///
    /// Returns a typed error for any framing, context, transition, or identity mismatch.
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self, ProtectedHeadStoreProtocolErrorV1> {
        if bytes.len() != PREOPENED_PROTECTED_HEAD_STORE_REQUEST_BYTES_V1 {
            return Err(ProtectedHeadStoreProtocolErrorV1::RequestLength {
                expected: PREOPENED_PROTECTED_HEAD_STORE_REQUEST_BYTES_V1,
                actual: bytes.len(),
            });
        }
        let mut reader = ReaderV1::new(bytes);
        if reader.array::<8>()? != MAGIC_REQUEST_V1 || reader.u16()? != VERSION_V1 {
            return Err(ProtectedHeadStoreProtocolErrorV1::RequestHeader);
        }
        let operation = ProtectedHeadStoreOperationV1::decode(reader.u16()?)?;
        if reader.u16()? != 0
            || reader.u16()? != 0
            || reader.u32()? as usize != PREOPENED_PROTECTED_HEAD_STORE_REQUEST_BYTES_V1
        {
            return Err(ProtectedHeadStoreProtocolErrorV1::RequestHeader);
        }
        let encoded_identity = reader.array()?;
        let protocol_identity = reader.array()?;
        let provider_identity = reader.array()?;
        let namespace_identity = reader.array()?;
        let policy_identity = reader.array()?;
        let kind = ProtectedLedgerKindV1::from_tag(reader.u16()?)
            .ok_or(ProtectedHeadStoreProtocolErrorV1::LedgerKind)?;
        let request_sequence = reader.u64()?;
        if reader.array::<6>()? != [0; 6] {
            return Err(ProtectedHeadStoreProtocolErrorV1::ReservedBytes);
        }
        let current_bytes = reader.array()?;
        let next_bytes = reader.array()?;
        if !reader.is_finished() {
            return Err(ProtectedHeadStoreProtocolErrorV1::TrailingBytes);
        }
        let current = decode_optional_head(current_bytes)?;
        let next = decode_optional_head(next_bytes)?;
        let request = Self::new(
            operation,
            protocol_identity,
            provider_identity,
            namespace_identity,
            policy_identity,
            kind,
            request_sequence,
            current,
            next,
        )?;
        if request.request_identity != encoded_identity {
            return Err(ProtectedHeadStoreProtocolErrorV1::RequestIdentity);
        }
        if request.canonical_bytes.as_slice() != bytes {
            return Err(ProtectedHeadStoreProtocolErrorV1::NoncanonicalRequest);
        }
        Ok(request)
    }

    /// Returns the requested operation.
    pub const fn operation(&self) -> ProtectedHeadStoreOperationV1 {
        self.operation
    }

    /// Returns the identity binding the complete request.
    pub const fn request_identity(&self) -> [u8; 32] {
        self.request_identity
    }

    /// Returns the pinned protocol identity.
    pub const fn protocol_identity(&self) -> [u8; 32] {
        self.protocol_identity
    }

    /// Returns the pinned provider identity.
    pub const fn provider_identity(&self) -> [u8; 32] {
        self.provider_identity
    }

    /// Returns the pinned durable namespace identity.
    pub const fn namespace_identity(&self) -> [u8; 32] {
        self.namespace_identity
    }

    /// Returns the pinned trust-policy identity.
    pub const fn policy_identity(&self) -> [u8; 32] {
        self.policy_identity
    }

    /// Returns the pinned ledger kind.
    pub const fn kind(&self) -> ProtectedLedgerKindV1 {
        self.kind
    }

    /// Returns the nonzero per-client sequence correlated by the response.
    pub const fn request_sequence(&self) -> u64 {
        self.request_sequence
    }

    /// Returns the expected current head for compare-and-advance.
    pub const fn current(&self) -> Option<ProtectedLedgerExternalHeadV1> {
        self.current
    }

    /// Returns the initial or successor head.
    pub const fn next(&self) -> Option<ProtectedLedgerExternalHeadV1> {
        self.next
    }

    /// Borrows the exact canonical packet bytes.
    pub const fn encode_canonical(&self) -> &[u8; PREOPENED_PROTECTED_HEAD_STORE_REQUEST_BYTES_V1] {
        &self.canonical_bytes
    }

    /// Protocol framing alone grants no antirollback or verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

impl fmt::Debug for ProtectedHeadStoreRequestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProtectedHeadStoreRequestV1")
            .field("operation", &self.operation)
            .field("request_identity", &self.request_identity)
            .field("protocol_identity", &self.protocol_identity)
            .field("provider_identity", &self.provider_identity)
            .field("namespace_identity", &self.namespace_identity)
            .field("policy_identity", &self.policy_identity)
            .field("kind", &self.kind)
            .field("request_sequence", &self.request_sequence)
            .field("current", &self.current)
            .field("next", &self.next)
            .field("authority", &"none")
            .finish_non_exhaustive()
    }
}

/// Terminal status of one correlated head-store response.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedHeadStoreResponseStatusV1 {
    /// A load returned the durable head.
    Loaded,
    /// A load found a never-provisioned namespace.
    Absent,
    /// The protected peer claims it externally durably committed the requested next head.
    ///
    /// The peer must emit this only after durability; the client treats a missing
    /// response after send as ambiguous and poisons rather than inferring success.
    Advanced,
    /// Initialize or compare-and-advance observed a conflict and made no change.
    Conflict,
    /// The protected provider explicitly rejected the request and made no change.
    Rejected,
}

impl ProtectedHeadStoreResponseStatusV1 {
    const fn code(self) -> u16 {
        match self {
            Self::Loaded => 1,
            Self::Absent => 2,
            Self::Advanced => 3,
            Self::Conflict => 4,
            Self::Rejected => 5,
        }
    }

    fn decode(code: u16) -> Result<Self, ProtectedHeadStoreProtocolErrorV1> {
        match code {
            1 => Ok(Self::Loaded),
            2 => Ok(Self::Absent),
            3 => Ok(Self::Advanced),
            4 => Ok(Self::Conflict),
            5 => Ok(Self::Rejected),
            actual => Err(ProtectedHeadStoreProtocolErrorV1::ResponseStatus { actual }),
        }
    }
}

/// Exact fixed-width terminal response from the protected head store.
#[derive(Clone, Eq, PartialEq)]
pub struct ProtectedHeadStoreResponseV1 {
    operation: ProtectedHeadStoreOperationV1,
    status: ProtectedHeadStoreResponseStatusV1,
    response_identity: [u8; 32],
    request_identity: [u8; 32],
    protocol_identity: [u8; 32],
    provider_identity: [u8; 32],
    namespace_identity: [u8; 32],
    policy_identity: [u8; 32],
    kind: ProtectedLedgerKindV1,
    request_sequence: u64,
    head: Option<ProtectedLedgerExternalHeadV1>,
    canonical_bytes: [u8; PREOPENED_PROTECTED_HEAD_STORE_RESPONSE_BYTES_V1],
}

impl ProtectedHeadStoreResponseV1 {
    /// Constructs a canonical response returning one loaded head.
    ///
    /// # Errors
    ///
    /// Returns a typed error unless `request` is a load and the head matches its policy.
    pub fn loaded(
        request: &ProtectedHeadStoreRequestV1,
        head: ProtectedLedgerExternalHeadV1,
    ) -> Result<Self, ProtectedHeadStoreProtocolErrorV1> {
        Self::new(
            request,
            ProtectedHeadStoreResponseStatusV1::Loaded,
            Some(head),
        )
    }

    /// Constructs a canonical absent response for one load.
    ///
    /// # Errors
    ///
    /// Returns a typed error unless `request` is a load.
    pub fn absent(
        request: &ProtectedHeadStoreRequestV1,
    ) -> Result<Self, ProtectedHeadStoreProtocolErrorV1> {
        Self::new(request, ProtectedHeadStoreResponseStatusV1::Absent, None)
    }

    /// Constructs a canonical durable-advance response.
    ///
    /// # Errors
    ///
    /// Returns a typed error unless `request` is initialize or compare-and-advance.
    pub fn advanced(
        request: &ProtectedHeadStoreRequestV1,
    ) -> Result<Self, ProtectedHeadStoreProtocolErrorV1> {
        Self::new(
            request,
            ProtectedHeadStoreResponseStatusV1::Advanced,
            request.next,
        )
    }

    /// Constructs a canonical no-change conflict response.
    ///
    /// # Errors
    ///
    /// Returns a typed error unless `request` mutates a head.
    pub fn conflict(
        request: &ProtectedHeadStoreRequestV1,
    ) -> Result<Self, ProtectedHeadStoreProtocolErrorV1> {
        Self::new(request, ProtectedHeadStoreResponseStatusV1::Conflict, None)
    }

    /// Constructs a canonical explicit rejection response with no claimed state.
    ///
    /// # Errors
    ///
    /// Returns an error only if the supplied request is not internally canonical.
    pub fn rejected(
        request: &ProtectedHeadStoreRequestV1,
    ) -> Result<Self, ProtectedHeadStoreProtocolErrorV1> {
        Self::new(request, ProtectedHeadStoreResponseStatusV1::Rejected, None)
    }

    fn new(
        request: &ProtectedHeadStoreRequestV1,
        status: ProtectedHeadStoreResponseStatusV1,
        head: Option<ProtectedLedgerExternalHeadV1>,
    ) -> Result<Self, ProtectedHeadStoreProtocolErrorV1> {
        validate_response_semantics(request, status, head)?;
        let head_bytes = encode_optional_head(head);
        let response_identity = response_identity(request, status, &head_bytes);
        let canonical_bytes = encode_response(request, status, response_identity, &head_bytes);
        Ok(Self {
            operation: request.operation,
            status,
            response_identity,
            request_identity: request.request_identity,
            protocol_identity: request.protocol_identity,
            provider_identity: request.provider_identity,
            namespace_identity: request.namespace_identity,
            policy_identity: request.policy_identity,
            kind: request.kind,
            request_sequence: request.request_sequence,
            head,
            canonical_bytes,
        })
    }

    /// Strictly decodes one complete canonical response packet.
    ///
    /// # Errors
    ///
    /// Returns a typed error for any framing, semantics, or identity mismatch.
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self, ProtectedHeadStoreProtocolErrorV1> {
        if bytes.len() != PREOPENED_PROTECTED_HEAD_STORE_RESPONSE_BYTES_V1 {
            return Err(ProtectedHeadStoreProtocolErrorV1::ResponseLength {
                expected: PREOPENED_PROTECTED_HEAD_STORE_RESPONSE_BYTES_V1,
                actual: bytes.len(),
            });
        }
        let mut reader = ReaderV1::new(bytes);
        if reader.array::<8>()? != MAGIC_RESPONSE_V1 || reader.u16()? != VERSION_V1 {
            return Err(ProtectedHeadStoreProtocolErrorV1::ResponseHeader);
        }
        let operation = ProtectedHeadStoreOperationV1::decode(reader.u16()?)?;
        let status = ProtectedHeadStoreResponseStatusV1::decode(reader.u16()?)?;
        if reader.u16()? != 0
            || reader.u32()? as usize != PREOPENED_PROTECTED_HEAD_STORE_RESPONSE_BYTES_V1
        {
            return Err(ProtectedHeadStoreProtocolErrorV1::ResponseHeader);
        }
        let encoded_identity = reader.array()?;
        let request_identity = reader.array()?;
        let protocol_identity = reader.array()?;
        let provider_identity = reader.array()?;
        let namespace_identity = reader.array()?;
        let policy_identity = reader.array()?;
        let kind = ProtectedLedgerKindV1::from_tag(reader.u16()?)
            .ok_or(ProtectedHeadStoreProtocolErrorV1::LedgerKind)?;
        let request_sequence = reader.u64()?;
        if request_sequence == 0 {
            return Err(ProtectedHeadStoreProtocolErrorV1::ZeroRequestSequence);
        }
        if reader.array::<6>()? != [0; 6] {
            return Err(ProtectedHeadStoreProtocolErrorV1::ReservedBytes);
        }
        let head_bytes = reader.array()?;
        if !reader.is_finished() {
            return Err(ProtectedHeadStoreProtocolErrorV1::TrailingBytes);
        }
        let head = decode_optional_head(head_bytes)?;
        validate_context(
            protocol_identity,
            provider_identity,
            namespace_identity,
            policy_identity,
        )?;
        validate_decoded_response_shape(operation, status, policy_identity, head)?;
        let expected_identity = response_identity_fields(
            operation,
            status,
            request_identity,
            protocol_identity,
            provider_identity,
            namespace_identity,
            policy_identity,
            kind,
            request_sequence,
            &head_bytes,
        );
        if expected_identity != encoded_identity {
            return Err(ProtectedHeadStoreProtocolErrorV1::ResponseIdentity);
        }
        let response = Self {
            operation,
            status,
            response_identity: expected_identity,
            request_identity,
            protocol_identity,
            provider_identity,
            namespace_identity,
            policy_identity,
            kind,
            request_sequence,
            head,
            canonical_bytes: encode_response_fields(
                operation,
                status,
                expected_identity,
                request_identity,
                protocol_identity,
                provider_identity,
                namespace_identity,
                policy_identity,
                kind,
                request_sequence,
                &head_bytes,
            ),
        };
        if response.canonical_bytes.as_slice() != bytes {
            return Err(ProtectedHeadStoreProtocolErrorV1::NoncanonicalResponse);
        }
        Ok(response)
    }

    /// Returns the terminal status.
    pub const fn status(&self) -> ProtectedHeadStoreResponseStatusV1 {
        self.status
    }

    /// Returns the identity binding the complete response.
    pub const fn response_identity(&self) -> [u8; 32] {
        self.response_identity
    }

    /// Returns the correlated request identity.
    pub const fn request_identity(&self) -> [u8; 32] {
        self.request_identity
    }

    /// Returns the loaded or durably committed head, if present.
    pub const fn head(&self) -> Option<ProtectedLedgerExternalHeadV1> {
        self.head
    }

    /// Returns whether every response coordinate names `request` exactly.
    pub fn matches_request(&self, request: &ProtectedHeadStoreRequestV1) -> bool {
        self.operation == request.operation
            && self.request_identity == request.request_identity
            && self.protocol_identity == request.protocol_identity
            && self.provider_identity == request.provider_identity
            && self.namespace_identity == request.namespace_identity
            && self.policy_identity == request.policy_identity
            && self.kind == request.kind
            && self.request_sequence == request.request_sequence
            && validate_response_semantics(request, self.status, self.head).is_ok()
    }

    /// Borrows the exact canonical packet bytes.
    pub const fn encode_canonical(
        &self,
    ) -> &[u8; PREOPENED_PROTECTED_HEAD_STORE_RESPONSE_BYTES_V1] {
        &self.canonical_bytes
    }

    /// Protocol framing alone grants no antirollback or verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

impl fmt::Debug for ProtectedHeadStoreResponseV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProtectedHeadStoreResponseV1")
            .field("operation", &self.operation)
            .field("status", &self.status)
            .field("response_identity", &self.response_identity)
            .field("request_identity", &self.request_identity)
            .field("head", &self.head)
            .field("authority", &"none")
            .finish_non_exhaustive()
    }
}

/// Canonical protected head-store protocol failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedHeadStoreProtocolErrorV1 {
    /// Protocol identity was zero.
    ZeroProtocolIdentity,
    /// Provider identity was zero.
    ZeroProviderIdentity,
    /// Namespace identity was zero.
    ZeroNamespaceIdentity,
    /// Trust-policy identity was zero.
    ZeroPolicyIdentity,
    /// Request operation code was unsupported.
    Operation {
        /// Unsupported operation code.
        actual: u16,
    },
    /// Ledger kind code was unsupported.
    LedgerKind,
    /// Reserved bytes were not zero.
    ReservedBytes,
    /// Request sequence was zero.
    ZeroRequestSequence,
    /// One encoded head was structurally invalid.
    InvalidHead,
    /// A head used a policy other than the pinned policy.
    HeadPolicyMismatch,
    /// Initialize did not contain exactly one empty initial head.
    InvalidInitialization,
    /// Compare-and-advance was not an exact monotonic single-record transition.
    InvalidAdvance,
    /// Request packet length was not exact.
    RequestLength {
        /// Required length.
        expected: usize,
        /// Observed length.
        actual: usize,
    },
    /// Request header, version, flags, reserved bytes, or declared length differed.
    RequestHeader,
    /// Request identity did not bind every request coordinate.
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
    /// Response header, version, reserved bytes, or declared length differed.
    ResponseHeader,
    /// Response status code was unsupported.
    ResponseStatus {
        /// Unsupported status code.
        actual: u16,
    },
    /// Response status/head combination was invalid for its operation.
    ResponseSemantics,
    /// Response identity did not bind every response coordinate.
    ResponseIdentity,
    /// Response did not exactly round-trip through the canonical codec.
    NoncanonicalResponse,
    /// Fixed-width decoding unexpectedly exhausted its input.
    Truncated,
    /// Fixed-width decoding left trailing input.
    TrailingBytes,
}

impl fmt::Display for ProtectedHeadStoreProtocolErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "protected head-store protocol rejected: {self:?}"
        )
    }
}

impl Error for ProtectedHeadStoreProtocolErrorV1 {}

/// Descriptor or identity rejection before any head-store protocol byte is sent.
#[derive(Debug)]
#[non_exhaustive]
pub enum ProtectedHeadStoreClientAdmissionErrorV1 {
    /// Protocol identity was zero.
    ZeroProtocolIdentity,
    /// Provider identity was zero.
    ZeroProviderIdentity,
    /// Namespace identity was zero.
    ZeroNamespaceIdentity,
    /// Trust-policy identity was zero.
    ZeroPolicyIdentity,
    /// Operation timeout was below one millisecond or above the hard ceiling.
    InvalidOperationTimeout,
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
    HeadStoreAndVerifierUidMatch,
}

impl fmt::Display for ProtectedHeadStoreClientAdmissionErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "protected head-store endpoint rejected: {self:?}"
        )
    }
}

impl Error for ProtectedHeadStoreClientAdmissionErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Descriptor(source) | Self::PeerCredentials(source) => Some(source),
            _ => None,
        }
    }
}

/// Ownership-retaining head-store endpoint admission failure.
///
/// No protocol byte has been sent when this value is returned.
pub struct ProtectedHeadStoreClientAdmissionFailureV1 {
    error: ProtectedHeadStoreClientAdmissionErrorV1,
    peer: OwnedFd,
}

impl ProtectedHeadStoreClientAdmissionFailureV1 {
    /// Returns the stable admission error while retaining the endpoint.
    pub const fn error(&self) -> &ProtectedHeadStoreClientAdmissionErrorV1 {
        &self.error
    }

    /// Returns the exact caller-owned endpoint.
    pub fn into_peer(self) -> OwnedFd {
        self.peer
    }

    /// Returns the error and exact caller-owned endpoint.
    pub fn into_parts(self) -> (ProtectedHeadStoreClientAdmissionErrorV1, OwnedFd) {
        (self.error, self.peer)
    }
}

impl fmt::Debug for ProtectedHeadStoreClientAdmissionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProtectedHeadStoreClientAdmissionFailureV1")
            .field("error", &self.error)
            .field("custody", &ProtectedHeadStoreClientCustodyV1::Retained)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for ProtectedHeadStoreClientAdmissionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl Error for ProtectedHeadStoreClientAdmissionFailureV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error.source()
    }
}

/// Endpoint custody after a head-store operation fails.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedHeadStoreClientCustodyV1 {
    /// No request could have committed; the synchronized endpoint remains retained.
    Retained,
    /// The outcome may be ambiguous or the endpoint changed; the endpoint was closed.
    Poisoned,
}

/// Bounded head-store admission, transport, or correlation error.
#[derive(Debug)]
#[non_exhaustive]
pub enum ProtectedHeadStoreClientErrorV1 {
    /// A prior ambiguous exchange permanently closed the endpoint.
    Poisoned,
    /// Fixed protocol construction or decoding failed.
    Protocol(ProtectedHeadStoreProtocolErrorV1),
    /// Endpoint identity or peer credentials changed before or during exchange.
    EndpointRevalidation(ProtectedHeadStoreClientAdmissionErrorV1),
    /// The configured absolute operation deadline expired.
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
    /// A monotonic deadline could not be constructed.
    DeadlineOverflow,
    /// The nonzero per-client request sequence was exhausted.
    RequestSequenceExhausted,
    /// A synchronized response contradicted an already observed durable head.
    RollbackEvidence,
}

impl fmt::Display for ProtectedHeadStoreClientErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "protected head-store client failed: {self:?}")
    }
}

impl Error for ProtectedHeadStoreClientErrorV1 {
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
pub struct ProtectedHeadStoreClientFailureV1 {
    error: ProtectedHeadStoreClientErrorV1,
    custody: ProtectedHeadStoreClientCustodyV1,
}

impl ProtectedHeadStoreClientFailureV1 {
    /// Returns the operation error.
    pub const fn error(&self) -> &ProtectedHeadStoreClientErrorV1 {
        &self.error
    }

    /// Returns the exact endpoint custody after failure.
    pub const fn custody(&self) -> ProtectedHeadStoreClientCustodyV1 {
        self.custody
    }

    /// Splits the failure into its error and custody coordinates.
    pub fn into_parts(
        self,
    ) -> (
        ProtectedHeadStoreClientErrorV1,
        ProtectedHeadStoreClientCustodyV1,
    ) {
        (self.error, self.custody)
    }
}

impl fmt::Display for ProtectedHeadStoreClientFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl Error for ProtectedHeadStoreClientFailureV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error.source()
    }
}

/// Concrete descriptor-only provider for one protected antirollback namespace.
///
/// The provider is move-only. It performs fixed-size stack exchanges on success;
/// boxing occurs only when the legacy deployment trait returns an error.
pub struct PreopenedProtectedHeadStoreClientV1 {
    peer: Option<OwnedFd>,
    endpoint: ProtectedHeadStoreEndpointIdentityV1,
    expected_peer: ProtectedHeadStorePeerIdentityV1,
    protocol_identity: [u8; 32],
    provider_identity: [u8; 32],
    namespace_identity: [u8; 32],
    policy_identity: [u8; 32],
    kind: ProtectedLedgerKindV1,
    operation_timeout: Duration,
    next_request_sequence: u64,
    observed_existing: bool,
    last_observed_head: Option<ProtectedLedgerExternalHeadV1>,
}

impl fmt::Debug for PreopenedProtectedHeadStoreClientV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreopenedProtectedHeadStoreClientV1")
            .field("available", &self.peer.is_some())
            .field("endpoint", &self.endpoint)
            .field("expected_peer", &self.expected_peer)
            .field("protocol_identity", &self.protocol_identity)
            .field("provider_identity", &self.provider_identity)
            .field("namespace_identity", &self.namespace_identity)
            .field("policy_identity", &self.policy_identity)
            .field("kind", &self.kind)
            .field("observed_existing", &self.observed_existing)
            .field("authority", &"unsafe supervisor boundary")
            .finish_non_exhaustive()
    }
}

impl PreopenedProtectedHeadStoreClientV1 {
    /// Admits one exact supervisor-preopened protected head-store endpoint.
    ///
    /// The endpoint must already be connected, nonblocking, and close-on-exec.
    /// Production requires a dedicated non-root UID distinct from the verifier.
    /// This function performs no path discovery or connection. Descriptor and
    /// protocol checks alone cannot establish the peer's durability properties.
    ///
    /// # Safety
    ///
    /// The protected supervisor must independently authenticate that `peer` is
    /// the dedicated measured implementation for `protocol_identity` and
    /// `provider_identity`; that it owns an antirollback store exclusively for
    /// `(policy_identity, kind, namespace_identity)`; that it durably serializes
    /// every operation across processes and restarts; and that it emits
    /// [`ProtectedHeadStoreResponseStatusV1::Advanced`] only after the requested
    /// head is externally durable. The namespace may never be reset, deleted, or
    /// reused, including after policy revocation or capacity exhaustion.
    ///
    /// # Errors
    ///
    /// Returns an ownership-retaining failure before sending any bytes.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn admit_from_supervisor(
        peer: OwnedFd,
        endpoint: ProtectedHeadStoreEndpointIdentityV1,
        expected_peer: ProtectedHeadStorePeerIdentityV1,
        protocol_identity: [u8; 32],
        provider_identity: [u8; 32],
        namespace_identity: [u8; 32],
        policy_identity: [u8; 32],
        kind: ProtectedLedgerKindV1,
        operation_timeout: Duration,
    ) -> Result<Self, ProtectedHeadStoreClientAdmissionFailureV1> {
        Self::admit_inner::<true>(
            peer,
            endpoint,
            expected_peer,
            protocol_identity,
            provider_identity,
            namespace_identity,
            policy_identity,
            kind,
            operation_timeout,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn admit_inner<const REQUIRE_DISTINCT_UID: bool>(
        peer: OwnedFd,
        endpoint: ProtectedHeadStoreEndpointIdentityV1,
        expected_peer: ProtectedHeadStorePeerIdentityV1,
        protocol_identity: [u8; 32],
        provider_identity: [u8; 32],
        namespace_identity: [u8; 32],
        policy_identity: [u8; 32],
        kind: ProtectedLedgerKindV1,
        operation_timeout: Duration,
    ) -> Result<Self, ProtectedHeadStoreClientAdmissionFailureV1> {
        let admission = validate_admission_coordinates(
            protocol_identity,
            provider_identity,
            namespace_identity,
            policy_identity,
            operation_timeout,
        )
        .and_then(|()| validate_endpoint::<REQUIRE_DISTINCT_UID>(&peer, endpoint, expected_peer));
        if let Err(error) = admission {
            return Err(ProtectedHeadStoreClientAdmissionFailureV1 { error, peer });
        }
        Ok(Self {
            peer: Some(peer),
            endpoint,
            expected_peer,
            protocol_identity,
            provider_identity,
            namespace_identity,
            policy_identity,
            kind,
            operation_timeout,
            next_request_sequence: 1,
            observed_existing: false,
            last_observed_head: None,
        })
    }

    /// Returns whether a terminal or ambiguous failure permanently closed the endpoint.
    pub const fn is_poisoned(&self) -> bool {
        self.peer.is_none()
    }

    /// Returns the measured/pinned provider identity exposed to the ledger.
    pub const fn provider_identity(&self) -> [u8; 32] {
        self.provider_identity
    }

    /// Descriptor admission and framing grant no antirollback or verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }

    /// Loads the head under one newly computed absolute operation deadline.
    ///
    /// # Errors
    ///
    /// Returns a typed failure carrying retained/poisoned endpoint custody.
    pub fn load_head_bounded(
        &mut self,
    ) -> Result<Option<ProtectedLedgerExternalHeadV1>, ProtectedHeadStoreClientFailureV1> {
        self.load_head_inner::<true>()
    }

    fn load_head_inner<const REQUIRE_DISTINCT_UID: bool>(
        &mut self,
    ) -> Result<Option<ProtectedLedgerExternalHeadV1>, ProtectedHeadStoreClientFailureV1> {
        let request_sequence = self.take_request_sequence()?;
        let request = ProtectedHeadStoreRequestV1::load(
            self.protocol_identity,
            self.provider_identity,
            self.namespace_identity,
            self.policy_identity,
            self.kind,
            request_sequence,
        )
        .map_err(|error| self.pre_send_failure(ProtectedHeadStoreClientErrorV1::Protocol(error)))?;
        let response = self.exchange::<REQUIRE_DISTINCT_UID>(&request)?;
        match response.status {
            ProtectedHeadStoreResponseStatusV1::Loaded => {
                let head = response.head.expect("loaded response has a head");
                self.observe_loaded_head(head)?;
                Ok(Some(head))
            }
            ProtectedHeadStoreResponseStatusV1::Absent if !self.observed_existing => Ok(None),
            ProtectedHeadStoreResponseStatusV1::Absent => {
                Err(self.poisoned_failure(ProtectedHeadStoreClientErrorV1::RollbackEvidence))
            }
            ProtectedHeadStoreResponseStatusV1::Rejected => Err(Self::retained_failure(
                ProtectedHeadStoreClientErrorV1::ProviderRejected,
            )),
            _ => {
                Err(self.poisoned_failure(ProtectedHeadStoreClientErrorV1::ResponseRequestMismatch))
            }
        }
    }

    /// Initializes the exact empty head under one absolute operation deadline.
    ///
    /// # Errors
    ///
    /// Returns a typed failure carrying retained/poisoned endpoint custody.
    pub fn initialize_head_bounded(
        &mut self,
        initial: ProtectedLedgerExternalHeadV1,
    ) -> Result<bool, ProtectedHeadStoreClientFailureV1> {
        self.initialize_head_inner::<true>(initial)
    }

    fn initialize_head_inner<const REQUIRE_DISTINCT_UID: bool>(
        &mut self,
        initial: ProtectedLedgerExternalHeadV1,
    ) -> Result<bool, ProtectedHeadStoreClientFailureV1> {
        let request_sequence = self.take_request_sequence()?;
        let request = ProtectedHeadStoreRequestV1::initialize(
            self.protocol_identity,
            self.provider_identity,
            self.namespace_identity,
            self.policy_identity,
            self.kind,
            request_sequence,
            initial,
        )
        .map_err(|error| self.pre_send_failure(ProtectedHeadStoreClientErrorV1::Protocol(error)))?;
        self.exchange_mutation::<REQUIRE_DISTINCT_UID>(&request)
    }

    /// Compares and advances by exactly one record under one absolute operation deadline.
    ///
    /// # Errors
    ///
    /// Returns a typed failure carrying retained/poisoned endpoint custody.
    pub fn compare_and_advance_head_bounded(
        &mut self,
        current: ProtectedLedgerExternalHeadV1,
        next: ProtectedLedgerExternalHeadV1,
    ) -> Result<bool, ProtectedHeadStoreClientFailureV1> {
        self.compare_and_advance_head_inner::<true>(current, next)
    }

    fn compare_and_advance_head_inner<const REQUIRE_DISTINCT_UID: bool>(
        &mut self,
        current: ProtectedLedgerExternalHeadV1,
        next: ProtectedLedgerExternalHeadV1,
    ) -> Result<bool, ProtectedHeadStoreClientFailureV1> {
        let request_sequence = self.take_request_sequence()?;
        let request = ProtectedHeadStoreRequestV1::compare_and_advance(
            self.protocol_identity,
            self.provider_identity,
            self.namespace_identity,
            self.policy_identity,
            self.kind,
            request_sequence,
            current,
            next,
        )
        .map_err(|error| self.pre_send_failure(ProtectedHeadStoreClientErrorV1::Protocol(error)))?;
        self.exchange_mutation::<REQUIRE_DISTINCT_UID>(&request)
    }

    fn exchange_mutation<const REQUIRE_DISTINCT_UID: bool>(
        &mut self,
        request: &ProtectedHeadStoreRequestV1,
    ) -> Result<bool, ProtectedHeadStoreClientFailureV1> {
        let response = self.exchange::<REQUIRE_DISTINCT_UID>(request)?;
        match response.status {
            ProtectedHeadStoreResponseStatusV1::Advanced => {
                let head = response.head.expect("advanced response has a head");
                self.observe_loaded_head(head)?;
                Ok(true)
            }
            ProtectedHeadStoreResponseStatusV1::Conflict => {
                self.observed_existing = true;
                Ok(false)
            }
            ProtectedHeadStoreResponseStatusV1::Rejected => Err(Self::retained_failure(
                ProtectedHeadStoreClientErrorV1::ProviderRejected,
            )),
            _ => {
                Err(self.poisoned_failure(ProtectedHeadStoreClientErrorV1::ResponseRequestMismatch))
            }
        }
    }

    fn exchange<const REQUIRE_DISTINCT_UID: bool>(
        &mut self,
        request: &ProtectedHeadStoreRequestV1,
    ) -> Result<ProtectedHeadStoreResponseV1, ProtectedHeadStoreClientFailureV1> {
        let deadline =
            AbsoluteSessionDeadlineV1::after(self.operation_timeout).ok_or_else(|| {
                self.pre_send_failure(ProtectedHeadStoreClientErrorV1::DeadlineOverflow)
            })?;
        let Some(peer) = self.peer.take() else {
            return Err(self.poisoned_failure(ProtectedHeadStoreClientErrorV1::Poisoned));
        };
        if let Err(error) =
            validate_endpoint::<REQUIRE_DISTINCT_UID>(&peer, self.endpoint, self.expected_peer)
        {
            return Err(ProtectedHeadStoreClientFailureV1 {
                error: ProtectedHeadStoreClientErrorV1::EndpointRevalidation(error),
                custody: ProtectedHeadStoreClientCustodyV1::Poisoned,
            });
        }
        match send_packet(&peer, request.encode_canonical(), deadline) {
            Ok(()) => {}
            Err(failure) => {
                if !failure.poison {
                    self.peer = Some(peer);
                }
                return Err(ProtectedHeadStoreClientFailureV1 {
                    error: failure.error,
                    custody: if failure.poison {
                        ProtectedHeadStoreClientCustodyV1::Poisoned
                    } else {
                        ProtectedHeadStoreClientCustodyV1::Retained
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
        .map_err(|error| ProtectedHeadStoreClientFailureV1 {
            error,
            custody: ProtectedHeadStoreClientCustodyV1::Poisoned,
        })?;
        self.peer = Some(peer);
        Ok(response)
    }

    fn pre_send_failure(
        &self,
        error: ProtectedHeadStoreClientErrorV1,
    ) -> ProtectedHeadStoreClientFailureV1 {
        ProtectedHeadStoreClientFailureV1 {
            error,
            custody: if self.peer.is_some() {
                ProtectedHeadStoreClientCustodyV1::Retained
            } else {
                ProtectedHeadStoreClientCustodyV1::Poisoned
            },
        }
    }

    fn take_request_sequence(&mut self) -> Result<u64, ProtectedHeadStoreClientFailureV1> {
        let sequence = self.next_request_sequence;
        if sequence == 0 {
            return Err(
                self.pre_send_failure(ProtectedHeadStoreClientErrorV1::RequestSequenceExhausted)
            );
        }
        self.next_request_sequence = sequence.checked_add(1).unwrap_or(0);
        Ok(sequence)
    }

    fn observe_loaded_head(
        &mut self,
        head: ProtectedLedgerExternalHeadV1,
    ) -> Result<(), ProtectedHeadStoreClientFailureV1> {
        if let Some(prior) = self.last_observed_head {
            let regressed = head.header_identity() != prior.header_identity()
                || head.record_count() < prior.record_count()
                || (head.record_count() == prior.record_count()
                    && head.record_identity() != prior.record_identity());
            if regressed {
                return Err(
                    self.poisoned_failure(ProtectedHeadStoreClientErrorV1::RollbackEvidence)
                );
            }
        }
        self.observed_existing = true;
        self.last_observed_head = Some(head);
        Ok(())
    }

    fn retained_failure(
        error: ProtectedHeadStoreClientErrorV1,
    ) -> ProtectedHeadStoreClientFailureV1 {
        ProtectedHeadStoreClientFailureV1 {
            error,
            custody: ProtectedHeadStoreClientCustodyV1::Retained,
        }
    }

    fn poisoned_failure(
        &mut self,
        error: ProtectedHeadStoreClientErrorV1,
    ) -> ProtectedHeadStoreClientFailureV1 {
        self.peer = None;
        ProtectedHeadStoreClientFailureV1 {
            error,
            custody: ProtectedHeadStoreClientCustodyV1::Poisoned,
        }
    }
}

// SAFETY: the only public constructor is the documented unsafe supervisor
// boundary. After admission, every request remains pinned to that one provider
// and namespace; mutations are initialize-at-zero or exact single-record CAS;
// exchanges are bounded; and every ambiguous post-send outcome poisons.
unsafe impl ProtectedLedgerHeadStoreV1 for PreopenedProtectedHeadStoreClientV1 {
    fn provider_identity(&self) -> [u8; 32] {
        self.provider_identity
    }

    fn load_head(
        &mut self,
    ) -> Result<Option<ProtectedLedgerExternalHeadV1>, ProtectedLedgerHeadStoreFailureV1> {
        self.load_head_bounded()
            .map_err(|failure| Box::new(failure) as ProtectedLedgerHeadStoreFailureV1)
    }

    fn initialize_head(
        &mut self,
        initial: ProtectedLedgerExternalHeadV1,
    ) -> Result<bool, ProtectedLedgerHeadStoreFailureV1> {
        self.initialize_head_bounded(initial)
            .map_err(|failure| Box::new(failure) as ProtectedLedgerHeadStoreFailureV1)
    }

    fn compare_exchange_head(
        &mut self,
        current: ProtectedLedgerExternalHeadV1,
        next: ProtectedLedgerExternalHeadV1,
    ) -> Result<bool, ProtectedLedgerHeadStoreFailureV1> {
        self.compare_and_advance_head_bounded(current, next)
            .map_err(|failure| Box::new(failure) as ProtectedLedgerHeadStoreFailureV1)
    }
}

fn validate_context(
    protocol_identity: [u8; 32],
    provider_identity: [u8; 32],
    namespace_identity: [u8; 32],
    policy_identity: [u8; 32],
) -> Result<(), ProtectedHeadStoreProtocolErrorV1> {
    if protocol_identity == [0; 32] {
        return Err(ProtectedHeadStoreProtocolErrorV1::ZeroProtocolIdentity);
    }
    if provider_identity == [0; 32] {
        return Err(ProtectedHeadStoreProtocolErrorV1::ZeroProviderIdentity);
    }
    if namespace_identity == [0; 32] {
        return Err(ProtectedHeadStoreProtocolErrorV1::ZeroNamespaceIdentity);
    }
    if policy_identity == [0; 32] {
        return Err(ProtectedHeadStoreProtocolErrorV1::ZeroPolicyIdentity);
    }
    Ok(())
}

fn validate_transition(
    operation: ProtectedHeadStoreOperationV1,
    policy_identity: [u8; 32],
    current: Option<ProtectedLedgerExternalHeadV1>,
    next: Option<ProtectedLedgerExternalHeadV1>,
) -> Result<(), ProtectedHeadStoreProtocolErrorV1> {
    for head in [current, next].into_iter().flatten() {
        if head.policy_identity() != policy_identity {
            return Err(ProtectedHeadStoreProtocolErrorV1::HeadPolicyMismatch);
        }
    }
    match (operation, current, next) {
        (ProtectedHeadStoreOperationV1::Load, None, None) => Ok(()),
        (ProtectedHeadStoreOperationV1::Initialize, None, Some(initial))
            if initial.record_count() == 0 && initial.record_identity() == [0; 32] =>
        {
            Ok(())
        }
        (ProtectedHeadStoreOperationV1::Initialize, _, _) => {
            Err(ProtectedHeadStoreProtocolErrorV1::InvalidInitialization)
        }
        (ProtectedHeadStoreOperationV1::CompareAndAdvance, Some(current), Some(next))
            if next.header_identity() == current.header_identity()
                && next.record_count() == current.record_count().checked_add(1).unwrap_or(0)
                && next.record_identity() != [0; 32] =>
        {
            Ok(())
        }
        (ProtectedHeadStoreOperationV1::CompareAndAdvance, _, _) => {
            Err(ProtectedHeadStoreProtocolErrorV1::InvalidAdvance)
        }
        (ProtectedHeadStoreOperationV1::Load, _, _) => {
            Err(ProtectedHeadStoreProtocolErrorV1::ResponseSemantics)
        }
    }
}

fn validate_response_semantics(
    request: &ProtectedHeadStoreRequestV1,
    status: ProtectedHeadStoreResponseStatusV1,
    head: Option<ProtectedLedgerExternalHeadV1>,
) -> Result<(), ProtectedHeadStoreProtocolErrorV1> {
    if head.is_some_and(|value| value.policy_identity() != request.policy_identity) {
        return Err(ProtectedHeadStoreProtocolErrorV1::HeadPolicyMismatch);
    }
    match (request.operation, status, head) {
        (
            ProtectedHeadStoreOperationV1::Initialize
            | ProtectedHeadStoreOperationV1::CompareAndAdvance,
            ProtectedHeadStoreResponseStatusV1::Advanced,
            Some(value),
        ) if Some(value) == request.next => Ok(()),
        (
            ProtectedHeadStoreOperationV1::Load,
            ProtectedHeadStoreResponseStatusV1::Loaded,
            Some(_),
        )
        | (ProtectedHeadStoreOperationV1::Load, ProtectedHeadStoreResponseStatusV1::Absent, None)
        | (_, ProtectedHeadStoreResponseStatusV1::Rejected, None)
        | (
            ProtectedHeadStoreOperationV1::Initialize
            | ProtectedHeadStoreOperationV1::CompareAndAdvance,
            ProtectedHeadStoreResponseStatusV1::Conflict,
            None,
        ) => Ok(()),
        _ => Err(ProtectedHeadStoreProtocolErrorV1::ResponseSemantics),
    }
}

fn validate_decoded_response_shape(
    operation: ProtectedHeadStoreOperationV1,
    status: ProtectedHeadStoreResponseStatusV1,
    policy_identity: [u8; 32],
    head: Option<ProtectedLedgerExternalHeadV1>,
) -> Result<(), ProtectedHeadStoreProtocolErrorV1> {
    if head.is_some_and(|value| value.policy_identity() != policy_identity) {
        return Err(ProtectedHeadStoreProtocolErrorV1::HeadPolicyMismatch);
    }
    match (operation, status, head) {
        (
            ProtectedHeadStoreOperationV1::Load,
            ProtectedHeadStoreResponseStatusV1::Loaded,
            Some(_),
        )
        | (ProtectedHeadStoreOperationV1::Load, ProtectedHeadStoreResponseStatusV1::Absent, None)
        | (_, ProtectedHeadStoreResponseStatusV1::Rejected, None)
        | (
            ProtectedHeadStoreOperationV1::Initialize
            | ProtectedHeadStoreOperationV1::CompareAndAdvance,
            ProtectedHeadStoreResponseStatusV1::Advanced,
            Some(_),
        )
        | (
            ProtectedHeadStoreOperationV1::Initialize
            | ProtectedHeadStoreOperationV1::CompareAndAdvance,
            ProtectedHeadStoreResponseStatusV1::Conflict,
            None,
        ) => Ok(()),
        _ => Err(ProtectedHeadStoreProtocolErrorV1::ResponseSemantics),
    }
}

fn validate_admission_coordinates(
    protocol_identity: [u8; 32],
    provider_identity: [u8; 32],
    namespace_identity: [u8; 32],
    policy_identity: [u8; 32],
    operation_timeout: Duration,
) -> Result<(), ProtectedHeadStoreClientAdmissionErrorV1> {
    if protocol_identity == [0; 32] {
        return Err(ProtectedHeadStoreClientAdmissionErrorV1::ZeroProtocolIdentity);
    }
    if provider_identity == [0; 32] {
        return Err(ProtectedHeadStoreClientAdmissionErrorV1::ZeroProviderIdentity);
    }
    if namespace_identity == [0; 32] {
        return Err(ProtectedHeadStoreClientAdmissionErrorV1::ZeroNamespaceIdentity);
    }
    if policy_identity == [0; 32] {
        return Err(ProtectedHeadStoreClientAdmissionErrorV1::ZeroPolicyIdentity);
    }
    if operation_timeout < Duration::from_millis(1) || operation_timeout > MAX_OPERATION_TIMEOUT {
        return Err(ProtectedHeadStoreClientAdmissionErrorV1::InvalidOperationTimeout);
    }
    Ok(())
}

fn validate_endpoint<const REQUIRE_DISTINCT_UID: bool>(
    peer: &OwnedFd,
    endpoint: ProtectedHeadStoreEndpointIdentityV1,
    expected_peer: ProtectedHeadStorePeerIdentityV1,
) -> Result<(), ProtectedHeadStoreClientAdmissionErrorV1> {
    let stat = rustix::fs::fstat(peer)
        .map_err(|source| ProtectedHeadStoreClientAdmissionErrorV1::Descriptor(source.into()))?;
    let descriptor_flags = rustix::io::fcntl_getfd(peer)
        .map_err(|source| ProtectedHeadStoreClientAdmissionErrorV1::Descriptor(source.into()))?;
    let status = rustix::fs::fcntl_getfl(peer)
        .map_err(|source| ProtectedHeadStoreClientAdmissionErrorV1::Descriptor(source.into()))?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::Socket
        || !descriptor_flags.contains(FdFlags::CLOEXEC)
        || !status.contains(OFlags::NONBLOCK)
        || status & OFlags::ACCMODE != OFlags::RDWR
        || status.intersects(OFlags::APPEND | OFlags::ASYNC | OFlags::DIRECT | OFlags::PATH)
    {
        return Err(ProtectedHeadStoreClientAdmissionErrorV1::DescriptorShape);
    }
    if stat.st_dev != endpoint.device || stat.st_ino != endpoint.inode {
        return Err(ProtectedHeadStoreClientAdmissionErrorV1::EndpointIdentityMismatch);
    }
    if socket_option(peer, libc::SO_DOMAIN)? != libc::AF_UNIX
        || socket_option(peer, libc::SO_TYPE)? != libc::SOCK_SEQPACKET
        || socket_option(peer, libc::SO_ACCEPTCONN)? != 0
        || !is_connected_unix(peer)?
    {
        return Err(ProtectedHeadStoreClientAdmissionErrorV1::SocketShape);
    }
    let pending = socket_option(peer, libc::SO_ERROR)?;
    if pending != 0 {
        return Err(ProtectedHeadStoreClientAdmissionErrorV1::PendingSocketError { raw: pending });
    }
    let observed = peer_credentials(peer)?;
    if observed != expected_peer {
        return Err(ProtectedHeadStoreClientAdmissionErrorV1::PeerIdentityMismatch);
    }
    if REQUIRE_DISTINCT_UID && observed.uid == rustix::process::geteuid().as_raw() {
        return Err(ProtectedHeadStoreClientAdmissionErrorV1::HeadStoreAndVerifierUidMatch);
    }
    Ok(())
}

fn socket_option(
    peer: &OwnedFd,
    option: i32,
) -> Result<i32, ProtectedHeadStoreClientAdmissionErrorV1> {
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
        return Err(ProtectedHeadStoreClientAdmissionErrorV1::Descriptor(
            io::Error::last_os_error(),
        ));
    }
    let actual = usize::try_from(length).expect("socklen_t fits usize");
    if actual != mem::size_of::<i32>() {
        return Err(
            ProtectedHeadStoreClientAdmissionErrorV1::SocketOptionLength {
                expected: mem::size_of::<i32>(),
                actual,
            },
        );
    }
    Ok(value)
}

fn is_connected_unix(peer: &OwnedFd) -> Result<bool, ProtectedHeadStoreClientAdmissionErrorV1> {
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
        return Err(ProtectedHeadStoreClientAdmissionErrorV1::Descriptor(source));
    }
    Ok(i32::from(address.ss_family) == libc::AF_UNIX
        && usize::try_from(length).ok().is_some_and(|value| {
            value >= mem::size_of::<libc::sa_family_t>()
                && value <= mem::size_of::<libc::sockaddr_un>()
        }))
}

fn peer_credentials(
    peer: &OwnedFd,
) -> Result<ProtectedHeadStorePeerIdentityV1, ProtectedHeadStoreClientAdmissionErrorV1> {
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
        return Err(ProtectedHeadStoreClientAdmissionErrorV1::PeerCredentials(
            io::Error::last_os_error(),
        ));
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
        return Err(ProtectedHeadStoreClientAdmissionErrorV1::InvalidPeerCredentials);
    }
    Ok(ProtectedHeadStorePeerIdentityV1 {
        pid: pid.expect("positive peer PID checked"),
        uid: credentials.uid,
        gid: credentials.gid,
    })
}

struct SendPacketFailureV1 {
    error: ProtectedHeadStoreClientErrorV1,
    poison: bool,
}

fn send_packet(
    peer: &OwnedFd,
    bytes: &[u8; PREOPENED_PROTECTED_HEAD_STORE_REQUEST_BYTES_V1],
    deadline: AbsoluteSessionDeadlineV1,
) -> Result<(), SendPacketFailureV1> {
    if let Err(error) = wait_for_peer(peer, libc::POLLOUT, deadline) {
        let poison = !matches!(error, ProtectedHeadStoreClientErrorV1::DeadlineExpired);
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
            error: ProtectedHeadStoreClientErrorV1::Send(io::Error::last_os_error()),
            poison: true,
        });
    }
    if usize::try_from(sent).ok() != Some(bytes.len()) {
        return Err(SendPacketFailureV1 {
            error: ProtectedHeadStoreClientErrorV1::PartialSend,
            poison: true,
        });
    }
    Ok(())
}

fn receive_correlated_response<const REQUIRE_DISTINCT_UID: bool>(
    peer: &OwnedFd,
    endpoint: ProtectedHeadStoreEndpointIdentityV1,
    expected_peer: ProtectedHeadStorePeerIdentityV1,
    request: &ProtectedHeadStoreRequestV1,
    deadline: AbsoluteSessionDeadlineV1,
) -> Result<ProtectedHeadStoreResponseV1, ProtectedHeadStoreClientErrorV1> {
    validate_endpoint::<REQUIRE_DISTINCT_UID>(peer, endpoint, expected_peer)
        .map_err(ProtectedHeadStoreClientErrorV1::EndpointRevalidation)?;
    let bytes = receive_packet(peer, deadline)?;
    validate_endpoint::<REQUIRE_DISTINCT_UID>(peer, endpoint, expected_peer)
        .map_err(ProtectedHeadStoreClientErrorV1::EndpointRevalidation)?;
    if deadline.remaining().is_none() {
        return Err(ProtectedHeadStoreClientErrorV1::DeadlineExpired);
    }
    let response = ProtectedHeadStoreResponseV1::decode_canonical(&bytes)
        .map_err(ProtectedHeadStoreClientErrorV1::Protocol)?;
    if !response.matches_request(request) {
        return Err(ProtectedHeadStoreClientErrorV1::ResponseRequestMismatch);
    }
    Ok(response)
}

fn receive_packet(
    peer: &OwnedFd,
    deadline: AbsoluteSessionDeadlineV1,
) -> Result<[u8; PREOPENED_PROTECTED_HEAD_STORE_RESPONSE_BYTES_V1], ProtectedHeadStoreClientErrorV1>
{
    wait_for_peer(peer, libc::POLLIN, deadline)?;
    let mut bytes = [0_u8; PREOPENED_PROTECTED_HEAD_STORE_RESPONSE_BYTES_V1];
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
        return Err(ProtectedHeadStoreClientErrorV1::Receive(
            io::Error::last_os_error(),
        ));
    }
    if header.msg_flags & libc::MSG_CTRUNC != 0 || header.msg_controllen != 0 {
        return Err(ProtectedHeadStoreClientErrorV1::AncillaryData);
    }
    if header.msg_flags & libc::MSG_TRUNC != 0 {
        return Err(ProtectedHeadStoreClientErrorV1::PacketTruncated);
    }
    let received =
        usize::try_from(received).map_err(|_| ProtectedHeadStoreClientErrorV1::PacketTruncated)?;
    if received == 0 {
        return Err(ProtectedHeadStoreClientErrorV1::PeerClosed);
    }
    if received != bytes.len() {
        return Err(ProtectedHeadStoreClientErrorV1::ResponseLength {
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
) -> Result<(), ProtectedHeadStoreClientErrorV1> {
    loop {
        let remaining = deadline
            .remaining()
            .ok_or(ProtectedHeadStoreClientErrorV1::DeadlineExpired)?;
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
            return Err(ProtectedHeadStoreClientErrorV1::Poll(source));
        }
        if result == 0 {
            continue;
        }
        if deadline.remaining().is_none() {
            return Err(ProtectedHeadStoreClientErrorV1::DeadlineExpired);
        }
        if descriptor.revents & libc::POLLNVAL != 0 {
            return Err(ProtectedHeadStoreClientErrorV1::InvalidPeer);
        }
        if descriptor.revents & wanted != 0 {
            return Ok(());
        }
        if descriptor.revents & libc::POLLERR != 0 {
            return Err(ProtectedHeadStoreClientErrorV1::PeerFailed);
        }
        if descriptor.revents & libc::POLLHUP != 0 {
            return Err(ProtectedHeadStoreClientErrorV1::PeerClosed);
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
    operation: ProtectedHeadStoreOperationV1,
    protocol_identity: [u8; 32],
    provider_identity: [u8; 32],
    namespace_identity: [u8; 32],
    policy_identity: [u8; 32],
    kind: ProtectedLedgerKindV1,
    request_sequence: u64,
    current: &[u8; HEAD_BYTES],
    next: &[u8; HEAD_BYTES],
) -> [u8; 32] {
    hash_parts(&[
        REQUEST_IDENTITY_DOMAIN_V1,
        &VERSION_V1.to_le_bytes(),
        &operation.code().to_le_bytes(),
        &REQUEST_DECLARED_BYTES_V1.to_le_bytes(),
        &protocol_identity,
        &provider_identity,
        &namespace_identity,
        &policy_identity,
        &kind.tag().to_le_bytes(),
        &request_sequence.to_le_bytes(),
        current,
        next,
    ])
}

fn response_identity(
    request: &ProtectedHeadStoreRequestV1,
    status: ProtectedHeadStoreResponseStatusV1,
    head: &[u8; HEAD_BYTES],
) -> [u8; 32] {
    response_identity_fields(
        request.operation,
        status,
        request.request_identity,
        request.protocol_identity,
        request.provider_identity,
        request.namespace_identity,
        request.policy_identity,
        request.kind,
        request.request_sequence,
        head,
    )
}

#[allow(clippy::too_many_arguments)]
fn response_identity_fields(
    operation: ProtectedHeadStoreOperationV1,
    status: ProtectedHeadStoreResponseStatusV1,
    request_identity: [u8; 32],
    protocol_identity: [u8; 32],
    provider_identity: [u8; 32],
    namespace_identity: [u8; 32],
    policy_identity: [u8; 32],
    kind: ProtectedLedgerKindV1,
    request_sequence: u64,
    head: &[u8; HEAD_BYTES],
) -> [u8; 32] {
    hash_parts(&[
        RESPONSE_IDENTITY_DOMAIN_V1,
        &VERSION_V1.to_le_bytes(),
        &operation.code().to_le_bytes(),
        &status.code().to_le_bytes(),
        &RESPONSE_DECLARED_BYTES_V1.to_le_bytes(),
        &request_identity,
        &protocol_identity,
        &provider_identity,
        &namespace_identity,
        &policy_identity,
        &kind.tag().to_le_bytes(),
        &request_sequence.to_le_bytes(),
        head,
    ])
}

#[allow(clippy::too_many_arguments)]
fn encode_request(
    operation: ProtectedHeadStoreOperationV1,
    request_identity: [u8; 32],
    protocol_identity: [u8; 32],
    provider_identity: [u8; 32],
    namespace_identity: [u8; 32],
    policy_identity: [u8; 32],
    kind: ProtectedLedgerKindV1,
    request_sequence: u64,
    current: &[u8; HEAD_BYTES],
    next: &[u8; HEAD_BYTES],
) -> [u8; PREOPENED_PROTECTED_HEAD_STORE_REQUEST_BYTES_V1] {
    let mut bytes = [0_u8; PREOPENED_PROTECTED_HEAD_STORE_REQUEST_BYTES_V1];
    let mut offset = 0;
    put(&mut bytes, &mut offset, &MAGIC_REQUEST_V1);
    put(&mut bytes, &mut offset, &VERSION_V1.to_le_bytes());
    put(&mut bytes, &mut offset, &operation.code().to_le_bytes());
    put(&mut bytes, &mut offset, &0_u16.to_le_bytes());
    put(&mut bytes, &mut offset, &0_u16.to_le_bytes());
    put(
        &mut bytes,
        &mut offset,
        &REQUEST_DECLARED_BYTES_V1.to_le_bytes(),
    );
    put(&mut bytes, &mut offset, &request_identity);
    put(&mut bytes, &mut offset, &protocol_identity);
    put(&mut bytes, &mut offset, &provider_identity);
    put(&mut bytes, &mut offset, &namespace_identity);
    put(&mut bytes, &mut offset, &policy_identity);
    put(&mut bytes, &mut offset, &kind.tag().to_le_bytes());
    put(&mut bytes, &mut offset, &request_sequence.to_le_bytes());
    put(&mut bytes, &mut offset, &[0; 6]);
    put(&mut bytes, &mut offset, current);
    put(&mut bytes, &mut offset, next);
    debug_assert_eq!(offset, bytes.len());
    bytes
}

fn encode_response(
    request: &ProtectedHeadStoreRequestV1,
    status: ProtectedHeadStoreResponseStatusV1,
    response_identity: [u8; 32],
    head: &[u8; HEAD_BYTES],
) -> [u8; PREOPENED_PROTECTED_HEAD_STORE_RESPONSE_BYTES_V1] {
    encode_response_fields(
        request.operation,
        status,
        response_identity,
        request.request_identity,
        request.protocol_identity,
        request.provider_identity,
        request.namespace_identity,
        request.policy_identity,
        request.kind,
        request.request_sequence,
        head,
    )
}

#[allow(clippy::too_many_arguments)]
fn encode_response_fields(
    operation: ProtectedHeadStoreOperationV1,
    status: ProtectedHeadStoreResponseStatusV1,
    response_identity: [u8; 32],
    request_identity: [u8; 32],
    protocol_identity: [u8; 32],
    provider_identity: [u8; 32],
    namespace_identity: [u8; 32],
    policy_identity: [u8; 32],
    kind: ProtectedLedgerKindV1,
    request_sequence: u64,
    head: &[u8; HEAD_BYTES],
) -> [u8; PREOPENED_PROTECTED_HEAD_STORE_RESPONSE_BYTES_V1] {
    let mut bytes = [0_u8; PREOPENED_PROTECTED_HEAD_STORE_RESPONSE_BYTES_V1];
    let mut offset = 0;
    put(&mut bytes, &mut offset, &MAGIC_RESPONSE_V1);
    put(&mut bytes, &mut offset, &VERSION_V1.to_le_bytes());
    put(&mut bytes, &mut offset, &operation.code().to_le_bytes());
    put(&mut bytes, &mut offset, &status.code().to_le_bytes());
    put(&mut bytes, &mut offset, &0_u16.to_le_bytes());
    put(
        &mut bytes,
        &mut offset,
        &RESPONSE_DECLARED_BYTES_V1.to_le_bytes(),
    );
    put(&mut bytes, &mut offset, &response_identity);
    put(&mut bytes, &mut offset, &request_identity);
    put(&mut bytes, &mut offset, &protocol_identity);
    put(&mut bytes, &mut offset, &provider_identity);
    put(&mut bytes, &mut offset, &namespace_identity);
    put(&mut bytes, &mut offset, &policy_identity);
    put(&mut bytes, &mut offset, &kind.tag().to_le_bytes());
    put(&mut bytes, &mut offset, &request_sequence.to_le_bytes());
    put(&mut bytes, &mut offset, &[0; 6]);
    put(&mut bytes, &mut offset, head);
    debug_assert_eq!(offset, bytes.len());
    bytes
}

fn encode_optional_head(head: Option<ProtectedLedgerExternalHeadV1>) -> [u8; HEAD_BYTES] {
    let mut bytes = [0_u8; HEAD_BYTES];
    if let Some(head) = head {
        let mut offset = 0;
        put(&mut bytes, &mut offset, &head.policy_identity());
        put(&mut bytes, &mut offset, &head.header_identity());
        put(&mut bytes, &mut offset, &head.record_count().to_le_bytes());
        put(&mut bytes, &mut offset, &head.record_identity());
        debug_assert_eq!(offset, HEAD_BYTES);
    }
    bytes
}

fn decode_optional_head(
    bytes: [u8; HEAD_BYTES],
) -> Result<Option<ProtectedLedgerExternalHeadV1>, ProtectedHeadStoreProtocolErrorV1> {
    if bytes == [0; HEAD_BYTES] {
        return Ok(None);
    }
    let mut reader = ReaderV1::new(&bytes);
    let policy_identity = reader.array()?;
    let header_identity = reader.array()?;
    let record_count = reader.u32()?;
    let record_identity = reader.array()?;
    ProtectedLedgerExternalHeadV1::new(
        policy_identity,
        header_identity,
        record_count,
        record_identity,
    )
    .map(Some)
    .map_err(|_| ProtectedHeadStoreProtocolErrorV1::InvalidHead)
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

    fn array<const N: usize>(&mut self) -> Result<[u8; N], ProtectedHeadStoreProtocolErrorV1> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(ProtectedHeadStoreProtocolErrorV1::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(ProtectedHeadStoreProtocolErrorV1::Truncated)?
            .try_into()
            .map_err(|_| ProtectedHeadStoreProtocolErrorV1::Truncated)?;
        self.offset = end;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, ProtectedHeadStoreProtocolErrorV1> {
        self.array().map(u16::from_le_bytes)
    }

    fn u32(&mut self) -> Result<u32, ProtectedHeadStoreProtocolErrorV1> {
        self.array().map(u32::from_le_bytes)
    }

    fn u64(&mut self) -> Result<u64, ProtectedHeadStoreProtocolErrorV1> {
        self.array().map(u64::from_le_bytes)
    }

    const fn is_finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

#[cfg(test)]
impl PreopenedProtectedHeadStoreClientV1 {
    #[allow(clippy::too_many_arguments)]
    fn admit_same_uid_for_test(
        peer: OwnedFd,
        endpoint: ProtectedHeadStoreEndpointIdentityV1,
        expected_peer: ProtectedHeadStorePeerIdentityV1,
        protocol_identity: [u8; 32],
        provider_identity: [u8; 32],
        namespace_identity: [u8; 32],
        policy_identity: [u8; 32],
        kind: ProtectedLedgerKindV1,
        operation_timeout: Duration,
    ) -> Result<Self, ProtectedHeadStoreClientAdmissionFailureV1> {
        Self::admit_inner::<false>(
            peer,
            endpoint,
            expected_peer,
            protocol_identity,
            provider_identity,
            namespace_identity,
            policy_identity,
            kind,
            operation_timeout,
        )
    }

    fn load_head_same_uid_for_test(
        &mut self,
    ) -> Result<Option<ProtectedLedgerExternalHeadV1>, ProtectedHeadStoreClientFailureV1> {
        self.load_head_inner::<false>()
    }

    fn initialize_head_same_uid_for_test(
        &mut self,
        initial: ProtectedLedgerExternalHeadV1,
    ) -> Result<bool, ProtectedHeadStoreClientFailureV1> {
        self.initialize_head_inner::<false>(initial)
    }

    fn compare_and_advance_same_uid_for_test(
        &mut self,
        current: ProtectedLedgerExternalHeadV1,
        next: ProtectedLedgerExternalHeadV1,
    ) -> Result<bool, ProtectedHeadStoreClientFailureV1> {
        self.compare_and_advance_head_inner::<false>(current, next)
    }
}

#[cfg(test)]
mod tests {
    use std::os::fd::{AsRawFd, OwnedFd};
    use std::thread;

    use rustix::net::{AddressFamily, SocketFlags, SocketType, socketpair};

    use super::*;

    const PROTOCOL: [u8; 32] = [0x11; 32];
    const PROVIDER: [u8; 32] = [0x22; 32];
    const NAMESPACE: [u8; 32] = [0x33; 32];
    const POLICY: [u8; 32] = [0x44; 32];

    fn initial_head() -> ProtectedLedgerExternalHeadV1 {
        ProtectedLedgerExternalHeadV1::new(POLICY, [0x55; 32], 0, [0; 32]).unwrap()
    }

    fn next_head() -> ProtectedLedgerExternalHeadV1 {
        ProtectedLedgerExternalHeadV1::new(POLICY, [0x55; 32], 1, [0x66; 32]).unwrap()
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

    fn endpoint_identity(peer: &OwnedFd) -> ProtectedHeadStoreEndpointIdentityV1 {
        let stat = rustix::fs::fstat(peer).unwrap();
        ProtectedHeadStoreEndpointIdentityV1::new(stat.st_dev, stat.st_ino).unwrap()
    }

    fn current_peer_identity(peer: &OwnedFd) -> ProtectedHeadStorePeerIdentityV1 {
        peer_credentials(peer).unwrap()
    }

    fn admit_test_client(peer: OwnedFd) -> PreopenedProtectedHeadStoreClientV1 {
        let endpoint = endpoint_identity(&peer);
        let expected_peer = current_peer_identity(&peer);
        PreopenedProtectedHeadStoreClientV1::admit_same_uid_for_test(
            peer,
            endpoint,
            expected_peer,
            PROTOCOL,
            PROVIDER,
            NAMESPACE,
            POLICY,
            ProtectedLedgerKindV1::Replay,
            Duration::from_secs(2),
        )
        .unwrap()
    }

    fn deadline(duration: Duration) -> AbsoluteSessionDeadlineV1 {
        AbsoluteSessionDeadlineV1::after(duration).unwrap()
    }

    fn receive_request(peer: &OwnedFd) -> ProtectedHeadStoreRequestV1 {
        wait_for_peer(peer, libc::POLLIN, deadline(Duration::from_secs(2))).unwrap();
        let mut bytes = [0_u8; PREOPENED_PROTECTED_HEAD_STORE_REQUEST_BYTES_V1];
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
        ProtectedHeadStoreRequestV1::decode_canonical(&bytes).unwrap()
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
        F: FnOnce(&ProtectedHeadStoreRequestV1) -> Vec<u8> + Send + 'static,
    {
        thread::spawn(move || {
            let request = receive_request(&peer);
            send_bytes(&peer, &response(&request));
        })
    }

    #[test]
    fn canonical_protocol_binds_namespace_kind_and_monotonic_transition() {
        let load = ProtectedHeadStoreRequestV1::load(
            PROTOCOL,
            PROVIDER,
            NAMESPACE,
            POLICY,
            ProtectedLedgerKindV1::Replay,
            1,
        )
        .unwrap();
        assert_eq!(
            ProtectedHeadStoreRequestV1::decode_canonical(load.encode_canonical()).unwrap(),
            load
        );
        assert!(!load.grants_authority());

        let initialize = ProtectedHeadStoreRequestV1::initialize(
            PROTOCOL,
            PROVIDER,
            NAMESPACE,
            POLICY,
            ProtectedLedgerKindV1::Replay,
            2,
            initial_head(),
        )
        .unwrap();
        let advance = ProtectedHeadStoreRequestV1::compare_and_advance(
            PROTOCOL,
            PROVIDER,
            NAMESPACE,
            POLICY,
            ProtectedLedgerKindV1::Replay,
            3,
            initial_head(),
            next_head(),
        )
        .unwrap();
        assert_ne!(load.request_identity(), initialize.request_identity());
        assert_ne!(initialize.request_identity(), advance.request_identity());

        let advanced = ProtectedHeadStoreResponseV1::advanced(&advance).unwrap();
        assert_eq!(advanced.head(), Some(next_head()));
        assert!(advanced.matches_request(&advance));
        assert_eq!(
            ProtectedHeadStoreResponseV1::decode_canonical(advanced.encode_canonical()).unwrap(),
            advanced
        );
        assert!(!advanced.grants_authority());

        let loaded = ProtectedHeadStoreResponseV1::loaded(&load, initial_head()).unwrap();
        for offset in [10, 12, 52, 84, 116, 148, 180, 212, 214, 260, 292, 296] {
            let mut mutation = *loaded.encode_canonical();
            mutation[offset] ^= 1;
            assert!(
                ProtectedHeadStoreResponseV1::decode_canonical(&mutation).is_err(),
                "hostile response field at offset {offset} was accepted"
            );
        }
        let head_bytes = encode_optional_head(Some(initial_head()));
        let forged_advanced_identity = response_identity_fields(
            load.operation,
            ProtectedHeadStoreResponseStatusV1::Advanced,
            load.request_identity,
            load.protocol_identity,
            load.provider_identity,
            load.namespace_identity,
            load.policy_identity,
            load.kind,
            load.request_sequence,
            &head_bytes,
        );
        let forged_advanced = encode_response_fields(
            load.operation,
            ProtectedHeadStoreResponseStatusV1::Advanced,
            forged_advanced_identity,
            load.request_identity,
            load.protocol_identity,
            load.provider_identity,
            load.namespace_identity,
            load.policy_identity,
            load.kind,
            load.request_sequence,
            &head_bytes,
        );
        assert!(matches!(
            ProtectedHeadStoreResponseV1::decode_canonical(&forged_advanced),
            Err(ProtectedHeadStoreProtocolErrorV1::ResponseSemantics)
        ));
        let empty_head = [0; HEAD_BYTES];
        let forged_absent_identity = response_identity_fields(
            initialize.operation,
            ProtectedHeadStoreResponseStatusV1::Absent,
            initialize.request_identity,
            initialize.protocol_identity,
            initialize.provider_identity,
            initialize.namespace_identity,
            initialize.policy_identity,
            initialize.kind,
            initialize.request_sequence,
            &empty_head,
        );
        let forged_absent = encode_response_fields(
            initialize.operation,
            ProtectedHeadStoreResponseStatusV1::Absent,
            forged_absent_identity,
            initialize.request_identity,
            initialize.protocol_identity,
            initialize.provider_identity,
            initialize.namespace_identity,
            initialize.policy_identity,
            initialize.kind,
            initialize.request_sequence,
            &empty_head,
        );
        assert!(matches!(
            ProtectedHeadStoreResponseV1::decode_canonical(&forged_absent),
            Err(ProtectedHeadStoreProtocolErrorV1::ResponseSemantics)
        ));

        let skipped =
            ProtectedLedgerExternalHeadV1::new(POLICY, [0x55; 32], 2, [0x77; 32]).unwrap();
        assert!(matches!(
            ProtectedHeadStoreRequestV1::compare_and_advance(
                PROTOCOL,
                PROVIDER,
                NAMESPACE,
                POLICY,
                ProtectedLedgerKindV1::Replay,
                4,
                initial_head(),
                skipped,
            ),
            Err(ProtectedHeadStoreProtocolErrorV1::InvalidAdvance)
        ));
        let wrong_header =
            ProtectedLedgerExternalHeadV1::new(POLICY, [0x88; 32], 1, [0x66; 32]).unwrap();
        assert!(matches!(
            ProtectedHeadStoreRequestV1::compare_and_advance(
                PROTOCOL,
                PROVIDER,
                NAMESPACE,
                POLICY,
                ProtectedLedgerKindV1::Replay,
                5,
                initial_head(),
                wrong_header,
            ),
            Err(ProtectedHeadStoreProtocolErrorV1::InvalidAdvance)
        ));

        for offset in [52, 84, 116, 148, 180] {
            let mut mutation = *load.encode_canonical();
            mutation[offset] ^= 1;
            assert!(ProtectedHeadStoreRequestV1::decode_canonical(&mutation).is_err());
        }
        let mut kind_mutation = *load.encode_canonical();
        kind_mutation[180..182]
            .copy_from_slice(&ProtectedLedgerKindV1::Reservation.tag().to_le_bytes());
        assert!(matches!(
            ProtectedHeadStoreRequestV1::decode_canonical(&kind_mutation),
            Err(ProtectedHeadStoreProtocolErrorV1::RequestIdentity)
        ));
    }

    #[test]
    fn admission_rejects_substitution_and_retains_exact_endpoint() {
        let (peer, _store) = pair(SocketType::SEQPACKET);
        let raw = peer.as_raw_fd();
        let stat = rustix::fs::fstat(&peer).unwrap();
        let endpoint =
            ProtectedHeadStoreEndpointIdentityV1::new(stat.st_dev, stat.st_ino + 1).unwrap();
        let expected_peer = current_peer_identity(&peer);
        // SAFETY: the call is expected to fail on endpoint identity before authority is minted.
        let failure = unsafe {
            PreopenedProtectedHeadStoreClientV1::admit_from_supervisor(
                peer,
                endpoint,
                expected_peer,
                PROTOCOL,
                PROVIDER,
                NAMESPACE,
                POLICY,
                ProtectedLedgerKindV1::Replay,
                Duration::from_secs(2),
            )
        }
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedHeadStoreClientAdmissionErrorV1::EndpointIdentityMismatch
        ));
        assert_eq!(failure.into_peer().as_raw_fd(), raw);

        for coordinate in 0..3 {
            let (peer, _store) = pair(SocketType::SEQPACKET);
            let raw = peer.as_raw_fd();
            let endpoint = endpoint_identity(&peer);
            let mut expected_peer = current_peer_identity(&peer);
            match coordinate {
                0 => expected_peer.pid = expected_peer.pid.saturating_add(1),
                1 => expected_peer.uid = expected_peer.uid.saturating_add(1),
                _ => expected_peer.gid = expected_peer.gid.saturating_add(1),
            }
            let failure = PreopenedProtectedHeadStoreClientV1::admit_same_uid_for_test(
                peer,
                endpoint,
                expected_peer,
                PROTOCOL,
                PROVIDER,
                NAMESPACE,
                POLICY,
                ProtectedLedgerKindV1::Replay,
                Duration::from_secs(2),
            )
            .unwrap_err();
            assert!(matches!(
                failure.error(),
                ProtectedHeadStoreClientAdmissionErrorV1::PeerIdentityMismatch
            ));
            assert_eq!(failure.into_peer().as_raw_fd(), raw);
        }

        let (peer, _store) = pair(SocketType::STREAM);
        let raw = peer.as_raw_fd();
        let endpoint = endpoint_identity(&peer);
        let expected_peer = current_peer_identity(&peer);
        // SAFETY: the call is expected to fail on socket type before authority is minted.
        let failure = unsafe {
            PreopenedProtectedHeadStoreClientV1::admit_from_supervisor(
                peer,
                endpoint,
                expected_peer,
                PROTOCOL,
                PROVIDER,
                NAMESPACE,
                POLICY,
                ProtectedLedgerKindV1::Replay,
                Duration::from_secs(2),
            )
        }
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedHeadStoreClientAdmissionErrorV1::SocketShape
        ));
        assert_eq!(failure.into_peer().as_raw_fd(), raw);

        for timeout in [Duration::ZERO, Duration::from_nanos(999_999)] {
            let (peer, _store) = pair(SocketType::SEQPACKET);
            let raw = peer.as_raw_fd();
            let endpoint = endpoint_identity(&peer);
            let expected_peer = current_peer_identity(&peer);
            let failure = PreopenedProtectedHeadStoreClientV1::admit_same_uid_for_test(
                peer,
                endpoint,
                expected_peer,
                PROTOCOL,
                PROVIDER,
                NAMESPACE,
                POLICY,
                ProtectedLedgerKindV1::Replay,
                timeout,
            )
            .unwrap_err();
            assert!(matches!(
                failure.error(),
                ProtectedHeadStoreClientAdmissionErrorV1::InvalidOperationTimeout
            ));
            assert_eq!(failure.into_peer().as_raw_fd(), raw);
        }
    }

    #[test]
    fn production_admission_requires_a_distinct_uid() {
        let (peer, _store) = pair(SocketType::SEQPACKET);
        let endpoint = endpoint_identity(&peer);
        let expected_peer = current_peer_identity(&peer);
        // SAFETY: the call is expected to reject this same-UID test endpoint.
        let failure = unsafe {
            PreopenedProtectedHeadStoreClientV1::admit_from_supervisor(
                peer,
                endpoint,
                expected_peer,
                PROTOCOL,
                PROVIDER,
                NAMESPACE,
                POLICY,
                ProtectedLedgerKindV1::Replay,
                Duration::from_secs(2),
            )
        }
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedHeadStoreClientAdmissionErrorV1::HeadStoreAndVerifierUidMatch
        ));
    }

    #[test]
    fn load_initialize_advance_and_conflict_stay_synchronized() {
        let (peer, store) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client(peer);
        let server = thread::spawn(move || {
            let load_absent = receive_request(&store);
            send_bytes(
                &store,
                ProtectedHeadStoreResponseV1::absent(&load_absent)
                    .unwrap()
                    .encode_canonical(),
            );
            let initialize = receive_request(&store);
            send_bytes(
                &store,
                ProtectedHeadStoreResponseV1::advanced(&initialize)
                    .unwrap()
                    .encode_canonical(),
            );
            let load_present = receive_request(&store);
            send_bytes(
                &store,
                ProtectedHeadStoreResponseV1::loaded(&load_present, initial_head())
                    .unwrap()
                    .encode_canonical(),
            );
            let advance = receive_request(&store);
            send_bytes(
                &store,
                ProtectedHeadStoreResponseV1::advanced(&advance)
                    .unwrap()
                    .encode_canonical(),
            );
            let conflict = receive_request(&store);
            send_bytes(
                &store,
                ProtectedHeadStoreResponseV1::conflict(&conflict)
                    .unwrap()
                    .encode_canonical(),
            );
        });
        assert_eq!(client.load_head_same_uid_for_test().unwrap(), None);
        assert!(
            client
                .initialize_head_same_uid_for_test(initial_head())
                .unwrap()
        );
        assert_eq!(
            client.load_head_same_uid_for_test().unwrap(),
            Some(initial_head())
        );
        assert!(
            client
                .compare_and_advance_same_uid_for_test(initial_head(), next_head())
                .unwrap()
        );
        assert!(
            !client
                .compare_and_advance_same_uid_for_test(initial_head(), next_head())
                .unwrap()
        );
        assert!(!client.is_poisoned());
        assert!(!client.grants_authority());
        server.join().unwrap();
    }

    #[test]
    fn pre_send_invalid_advance_retains_channel_and_rejection_is_synchronized() {
        let (peer, store) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client(peer);
        let skipped =
            ProtectedLedgerExternalHeadV1::new(POLICY, [0x55; 32], 2, [0x77; 32]).unwrap();
        let failure = client
            .compare_and_advance_same_uid_for_test(initial_head(), skipped)
            .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedHeadStoreClientErrorV1::Protocol(
                ProtectedHeadStoreProtocolErrorV1::InvalidAdvance
            )
        ));
        assert_eq!(
            failure.custody(),
            ProtectedHeadStoreClientCustodyV1::Retained
        );
        assert!(!client.is_poisoned());

        let server = thread::spawn(move || {
            let rejected = receive_request(&store);
            send_bytes(
                &store,
                ProtectedHeadStoreResponseV1::rejected(&rejected)
                    .unwrap()
                    .encode_canonical(),
            );
            let load = receive_request(&store);
            send_bytes(
                &store,
                ProtectedHeadStoreResponseV1::absent(&load)
                    .unwrap()
                    .encode_canonical(),
            );
        });
        let failure = client.load_head_same_uid_for_test().unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedHeadStoreClientErrorV1::ProviderRejected
        ));
        assert_eq!(
            failure.custody(),
            ProtectedHeadStoreClientCustodyV1::Retained
        );
        assert_eq!(client.load_head_same_uid_for_test().unwrap(), None);
        assert!(!client.is_poisoned());
        server.join().unwrap();
    }

    #[test]
    fn uncorrelated_and_malformed_post_send_responses_poison() {
        for malformed in [false, true] {
            let (peer, store) = pair(SocketType::SEQPACKET);
            let mut client = admit_test_client(peer);
            let server = spawn_response(store, move |request| {
                if malformed {
                    let mut bytes = ProtectedHeadStoreResponseV1::absent(request)
                        .unwrap()
                        .encode_canonical()
                        .to_vec();
                    bytes[0] ^= 1;
                    bytes
                } else {
                    let other = ProtectedHeadStoreRequestV1::load(
                        PROTOCOL,
                        PROVIDER,
                        [0x99; 32],
                        POLICY,
                        ProtectedLedgerKindV1::Replay,
                        request.request_sequence(),
                    )
                    .unwrap();
                    ProtectedHeadStoreResponseV1::absent(&other)
                        .unwrap()
                        .encode_canonical()
                        .to_vec()
                }
            });
            let failure = client.load_head_same_uid_for_test().unwrap_err();
            if malformed {
                assert!(matches!(
                    failure.error(),
                    ProtectedHeadStoreClientErrorV1::Protocol(
                        ProtectedHeadStoreProtocolErrorV1::ResponseHeader
                    )
                ));
            } else {
                assert!(matches!(
                    failure.error(),
                    ProtectedHeadStoreClientErrorV1::ResponseRequestMismatch
                ));
            }
            assert_eq!(
                failure.custody(),
                ProtectedHeadStoreClientCustodyV1::Poisoned
            );
            assert!(client.is_poisoned());
            assert!(matches!(
                client.load_head_same_uid_for_test().unwrap_err().error(),
                ProtectedHeadStoreClientErrorV1::Poisoned
            ));
            server.join().unwrap();
        }
    }

    #[test]
    fn replayed_response_sequence_is_rejected_and_poisons() {
        let (peer, store) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client(peer);
        let server = thread::spawn(move || {
            let first = receive_request(&store);
            let first_response = *ProtectedHeadStoreResponseV1::absent(&first)
                .unwrap()
                .encode_canonical();
            send_bytes(&store, &first_response);
            let second = receive_request(&store);
            assert_ne!(first.request_sequence(), second.request_sequence());
            send_bytes(&store, &first_response);
        });
        assert_eq!(client.load_head_same_uid_for_test().unwrap(), None);
        let failure = client.load_head_same_uid_for_test().unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedHeadStoreClientErrorV1::ResponseRequestMismatch
        ));
        assert_eq!(
            failure.custody(),
            ProtectedHeadStoreClientCustodyV1::Poisoned
        );
        assert!(client.is_poisoned());
        server.join().unwrap();
    }

    #[test]
    fn absent_or_stale_head_after_existence_evidence_poisons() {
        for stale_kind in 0..4 {
            let (peer, store) = pair(SocketType::SEQPACKET);
            let mut client = admit_test_client(peer);
            let server = thread::spawn(move || {
                let initialize = receive_request(&store);
                send_bytes(
                    &store,
                    ProtectedHeadStoreResponseV1::advanced(&initialize)
                        .unwrap()
                        .encode_canonical(),
                );
                if stale_kind != 0 {
                    let advance = receive_request(&store);
                    send_bytes(
                        &store,
                        ProtectedHeadStoreResponseV1::advanced(&advance)
                            .unwrap()
                            .encode_canonical(),
                    );
                }
                let load = receive_request(&store);
                let response = match stale_kind {
                    0 => ProtectedHeadStoreResponseV1::absent(&load).unwrap(),
                    1 => ProtectedHeadStoreResponseV1::loaded(&load, initial_head()).unwrap(),
                    2 => ProtectedHeadStoreResponseV1::loaded(
                        &load,
                        ProtectedLedgerExternalHeadV1::new(POLICY, [0x55; 32], 1, [0x77; 32])
                            .unwrap(),
                    )
                    .unwrap(),
                    _ => ProtectedHeadStoreResponseV1::loaded(
                        &load,
                        ProtectedLedgerExternalHeadV1::new(POLICY, [0x88; 32], 1, [0x66; 32])
                            .unwrap(),
                    )
                    .unwrap(),
                };
                send_bytes(&store, response.encode_canonical());
            });
            assert!(
                client
                    .initialize_head_same_uid_for_test(initial_head())
                    .unwrap()
            );
            if stale_kind != 0 {
                assert!(
                    client
                        .compare_and_advance_same_uid_for_test(initial_head(), next_head())
                        .unwrap()
                );
            }
            let failure = client.load_head_same_uid_for_test().unwrap_err();
            assert!(matches!(
                failure.error(),
                ProtectedHeadStoreClientErrorV1::RollbackEvidence
            ));
            assert_eq!(
                failure.custody(),
                ProtectedHeadStoreClientCustodyV1::Poisoned
            );
            assert!(client.is_poisoned());
            server.join().unwrap();
        }
    }

    #[test]
    fn short_oversize_and_malformed_responses_poison() {
        for response_kind in 0..3 {
            let (peer, store) = pair(SocketType::SEQPACKET);
            let mut client = admit_test_client(peer);
            let server = spawn_response(store, move |request| {
                let mut response = ProtectedHeadStoreResponseV1::absent(request)
                    .unwrap()
                    .encode_canonical()
                    .to_vec();
                match response_kind {
                    0 => {
                        response.pop();
                    }
                    1 => response.push(0),
                    _ => response[0] ^= 1,
                }
                response
            });
            let failure = client.load_head_same_uid_for_test().unwrap_err();
            match response_kind {
                0 => assert!(matches!(
                    failure.error(),
                    ProtectedHeadStoreClientErrorV1::ResponseLength { .. }
                )),
                1 => assert!(matches!(
                    failure.error(),
                    ProtectedHeadStoreClientErrorV1::PacketTruncated
                )),
                _ => assert!(matches!(
                    failure.error(),
                    ProtectedHeadStoreClientErrorV1::Protocol(
                        ProtectedHeadStoreProtocolErrorV1::ResponseHeader
                    )
                )),
            }
            assert_eq!(
                failure.custody(),
                ProtectedHeadStoreClientCustodyV1::Poisoned
            );
            assert!(client.is_poisoned());
            server.join().unwrap();
        }
    }

    #[test]
    fn committed_then_transport_failure_is_ambiguous_and_poisons() {
        let (peer, store) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client(peer);
        let server = thread::spawn(move || {
            let committed = receive_request(&store);
            assert_eq!(
                committed.operation(),
                ProtectedHeadStoreOperationV1::Initialize
            );
            assert_eq!(committed.next(), Some(initial_head()));
            // Model a peer that made the state durable and died before its acknowledgement.
            drop(store);
        });
        let failure = client
            .initialize_head_same_uid_for_test(initial_head())
            .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedHeadStoreClientErrorV1::PeerClosed
                | ProtectedHeadStoreClientErrorV1::Receive(_)
        ));
        assert_eq!(
            failure.custody(),
            ProtectedHeadStoreClientCustodyV1::Poisoned
        );
        assert!(client.is_poisoned());
        server.join().unwrap();
    }

    #[test]
    fn timeout_and_peer_close_after_send_are_ambiguous_and_poison() {
        for close in [false, true] {
            let (peer, store) = pair(SocketType::SEQPACKET);
            let mut client = admit_test_client(peer);
            client.operation_timeout = Duration::from_millis(40);
            let server = thread::spawn(move || {
                let _request = receive_request(&store);
                if !close {
                    thread::sleep(Duration::from_millis(100));
                }
            });
            let failure = client.load_head_same_uid_for_test().unwrap_err();
            if close {
                assert!(matches!(
                    failure.error(),
                    ProtectedHeadStoreClientErrorV1::PeerClosed
                        | ProtectedHeadStoreClientErrorV1::Receive(_)
                ));
            } else {
                assert!(matches!(
                    failure.error(),
                    ProtectedHeadStoreClientErrorV1::DeadlineExpired
                ));
            }
            assert_eq!(
                failure.custody(),
                ProtectedHeadStoreClientCustodyV1::Poisoned
            );
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
        let (peer, store) = pair(SocketType::SEQPACKET);
        let mut client = admit_test_client(peer);
        let server = thread::spawn(move || {
            let request = receive_request(&store);
            let response = ProtectedHeadStoreResponseV1::absent(&request).unwrap();
            send_ancillary(&store, response.encode_canonical());
        });
        let failure = client.load_head_same_uid_for_test().unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedHeadStoreClientErrorV1::AncillaryData
        ));
        assert_eq!(
            failure.custody(),
            ProtectedHeadStoreClientCustodyV1::Poisoned
        );
        assert!(client.is_poisoned());
        server.join().unwrap();
    }
}
