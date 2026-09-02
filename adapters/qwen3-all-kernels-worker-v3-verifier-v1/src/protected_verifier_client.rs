//! One-shot bounded client for the aggregate protected-verifier service.
//!
//! The client admits only a connected unnamed Unix `SOCK_SEQPACKET` peer with
//! caller-pinned, dedicated service credentials. One absolute monotonic
//! deadline covers the entire exchange. The client authenticates the returned
//! receipt under the caller-provisioned trust policy after exact packet
//! correlation and closes the peer on every terminal path. Consuming one
//! client prevents reuse of that local session object, not replay across new
//! sessions: global freshness requires service-side atomic challenge
//! consumption and protected live current-ledger validation.

#![allow(
    clippy::must_use_candidate,
    reason = "public accessors expose inert identities or retained ownership"
)]

use core::fmt;
use std::error::Error;
use std::io;
use std::mem;
use std::os::fd::{AsRawFd, OwnedFd};
use std::time::{Duration, Instant};

use fe2o3_runtime_protocol::{
    COMPILER_EXECUTION_CURRENT_RECORD_ATTESTATION_BYTES_V3,
    COMPILER_EXECUTION_CURRENT_RECORD_VERIFICATION_BYTES_V3,
};
use fe2o3_worker_v3_verification_client::{
    PendingWorkerV3VerificationClientV2, WorkerV3VerificationBeginOutcomeV2,
    WorkerV3VerificationClientErrorV1, WorkerV3VerificationClientErrorV2,
    WorkerV3VerificationClientV2, WorkerV3VerificationCurrentRecordChallengeV2,
    WorkerV3VerificationPayloadSnapshotsV1,
};
use fe2o3_worker_v3_verification_protocol::{
    WorkerV3VerificationFreshChallengeV1, WorkerV3VerificationProtocolErrorV1,
    WorkerV3VerificationRequestV1, WorkerV3VerificationTerminalDispositionV2,
};

use crate::protected_receipt::{
    M1AllKernelsAuthenticatedProtectedVerifierReceiptV1, M1AllKernelsProtectedReceiptErrorV1,
    M1AllKernelsProtectedVerifierTrustPolicyV1,
};
use crate::protected_verifier_service::{
    M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1,
    M1AllKernelsProtectedVerifierServiceProtocolErrorV1,
    M1AllKernelsProtectedVerifierServiceRequestV1, M1AllKernelsProtectedVerifierServiceResponseV1,
};

const INVALID_ID: u32 = u32::MAX;

/// One deployment-reserved, move-only generic Begin challenge.
///
/// Ferric does not generate this challenge: production deployment code must use an unpredictable
/// source and durably exclude the exact value from every prior and future Begin transaction before
/// constructing this token. The deterministic host roster challenge is a different coordinate and
/// must never be substituted here.
///
/// ```compile_fail
/// use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_verifier_client::
///     M1AllKernelsProtectedVerifierBeginChallengeV2;
/// fn duplicate(value: M1AllKernelsProtectedVerifierBeginChallengeV2) {
///     let _again = value.clone();
/// }
/// ```
pub struct M1AllKernelsProtectedVerifierBeginChallengeV2 {
    challenge: WorkerV3VerificationFreshChallengeV1,
}

impl M1AllKernelsProtectedVerifierBeginChallengeV2 {
    /// Admits a deployment-generated challenge after durable reservation.
    ///
    /// # Safety
    ///
    /// Before this call, the deployment must generate `bytes` unpredictably, atomically reserve it
    /// in durable replay state shared by every verifier instance, and guarantee that the value is
    /// never admitted for another Begin transaction, including after disconnects or restarts.
    ///
    /// # Errors
    ///
    /// Returns a protocol error for the forbidden all-zero challenge.
    pub unsafe fn from_durable_reservation(
        bytes: [u8; 32],
    ) -> Result<Self, WorkerV3VerificationProtocolErrorV1> {
        Ok(Self {
            challenge: WorkerV3VerificationFreshChallengeV1::new(bytes)?,
        })
    }

    pub(crate) fn into_protocol_challenge(self) -> WorkerV3VerificationFreshChallengeV1 {
        self.challenge
    }

    /// Reservation of a caller nonce alone grants no verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

impl fmt::Debug for M1AllKernelsProtectedVerifierBeginChallengeV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AllKernelsProtectedVerifierBeginChallengeV2")
            .field("challenge", &"redacted")
            .field("durable_reservation", &"caller obligation")
            .field("authority", &"none")
            .finish()
    }
}

/// Caller-pinned kernel credential identity for the protected-verifier peer.
///
/// The UID and GID must identify a dedicated non-root service. This value
/// carries no socket pathname, key, measurement, policy, or verifier authority.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct M1AllKernelsProtectedVerifierServiceIdentityV1 {
    uid: u32,
    gid: u32,
}

impl M1AllKernelsProtectedVerifierServiceIdentityV1 {
    /// Constructs one dedicated non-root service identity.
    ///
    /// # Errors
    ///
    /// Returns a typed error for root or Linux invalid-ID sentinels.
    pub const fn new(
        uid: u32,
        gid: u32,
    ) -> Result<Self, M1AllKernelsProtectedVerifierServiceIdentityErrorV1> {
        if uid == 0 || uid == INVALID_ID {
            return Err(M1AllKernelsProtectedVerifierServiceIdentityErrorV1::InvalidUid);
        }
        if gid == 0 || gid == INVALID_ID {
            return Err(M1AllKernelsProtectedVerifierServiceIdentityErrorV1::InvalidGid);
        }
        Ok(Self { uid, gid })
    }

    /// Returns the expected effective service UID from `SO_PEERCRED`.
    pub const fn uid(self) -> u32 {
        self.uid
    }

    /// Returns the expected effective service GID from `SO_PEERCRED`.
    pub const fn gid(self) -> u32 {
        self.gid
    }

    /// A kernel credential expectation grants no verifier authority.
    pub const fn grants_authority(self) -> bool {
        false
    }
}

impl fmt::Debug for M1AllKernelsProtectedVerifierServiceIdentityV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AllKernelsProtectedVerifierServiceIdentityV1")
            .field("uid", &self.uid)
            .field("gid", &self.gid)
            .field("authority", &"none")
            .finish()
    }
}

/// Invalid dedicated protected-verifier service credentials.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1AllKernelsProtectedVerifierServiceIdentityErrorV1 {
    /// UID is root or the Linux invalid-ID sentinel.
    InvalidUid,
    /// GID is root or the Linux invalid-ID sentinel.
    InvalidGid,
}

impl fmt::Display for M1AllKernelsProtectedVerifierServiceIdentityErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidUid => "invalid aggregate protected-verifier service UID",
            Self::InvalidGid => "invalid aggregate protected-verifier service GID",
        })
    }
}

impl Error for M1AllKernelsProtectedVerifierServiceIdentityErrorV1 {}

/// Ownership-retaining failure from service-peer admission.
///
/// The caller can inspect the stable reason and recover the exact `OwnedFd`.
/// No bytes have been sent when this value is returned.
pub struct M1AllKernelsProtectedVerifierClientAdmissionFailureV1 {
    error: M1AllKernelsProtectedVerifierClientErrorV1,
    peer: OwnedFd,
}

impl M1AllKernelsProtectedVerifierClientAdmissionFailureV1 {
    /// Returns the stable admission error without releasing the peer.
    pub const fn error(&self) -> &M1AllKernelsProtectedVerifierClientErrorV1 {
        &self.error
    }

    /// Returns the exact owned peer to the caller.
    pub fn into_peer(self) -> OwnedFd {
        self.peer
    }

    /// Returns both the stable error and the exact owned peer.
    pub fn into_parts(self) -> (M1AllKernelsProtectedVerifierClientErrorV1, OwnedFd) {
        (self.error, self.peer)
    }
}

impl fmt::Debug for M1AllKernelsProtectedVerifierClientAdmissionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AllKernelsProtectedVerifierClientAdmissionFailureV1")
            .field("error", &self.error)
            .field("retains_peer", &true)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for M1AllKernelsProtectedVerifierClientAdmissionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl Error for M1AllKernelsProtectedVerifierClientAdmissionFailureV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error.source()
    }
}

/// One owned, bounded connection to an externally supervised protected verifier.
///
/// The value is deliberately move-only and does not expose its descriptor. A
/// successful exchange consumes it, preventing a second request from replaying
/// the same session state.
pub struct M1AllKernelsProtectedVerifierClientV1 {
    peer: OwnedFd,
    expected_service: M1AllKernelsProtectedVerifierServiceIdentityV1,
    peer_pid: u32,
    deadline: Instant,
}

impl fmt::Debug for M1AllKernelsProtectedVerifierClientV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AllKernelsProtectedVerifierClientV1")
            .field("expected_service", &self.expected_service)
            .field("peer_pid", &self.peer_pid)
            .field("deadline", &self.deadline)
            .field("authority", &"none")
            .finish_non_exhaustive()
    }
}

impl M1AllKernelsProtectedVerifierClientV1 {
    /// Admits one caller-owned, connected, unnamed Unix `SOCK_SEQPACKET` peer.
    ///
    /// Production admission requires the peer's `SO_PEERCRED` UID to differ
    /// from the client effective UID. Failure returns ownership of the original
    /// peer and never transmits bytes.
    ///
    /// # Errors
    ///
    /// Returns an ownership-retaining failure for invalid timeouts, descriptor
    /// shape, endpoint addresses, peer credentials, or expired admission.
    pub fn admit(
        peer: OwnedFd,
        expected_service: M1AllKernelsProtectedVerifierServiceIdentityV1,
        timeout: Duration,
    ) -> Result<Self, M1AllKernelsProtectedVerifierClientAdmissionFailureV1> {
        Self::admit_inner::<true>(peer, expected_service, timeout)
    }

    fn admit_inner<const REQUIRE_DISTINCT_UID: bool>(
        peer: OwnedFd,
        expected_service: M1AllKernelsProtectedVerifierServiceIdentityV1,
        timeout: Duration,
    ) -> Result<Self, M1AllKernelsProtectedVerifierClientAdmissionFailureV1> {
        match admit_peer::<REQUIRE_DISTINCT_UID>(&peer, expected_service, timeout) {
            Ok((peer_pid, deadline)) => Ok(Self {
                peer,
                expected_service,
                peer_pid,
                deadline,
            }),
            Err(error) => {
                Err(M1AllKernelsProtectedVerifierClientAdmissionFailureV1 { error, peer })
            }
        }
    }

    /// Sends one exact request and authenticates one exact correlated receipt.
    ///
    /// The trust policy is caller-provisioned and must exactly match the request
    /// identity. Packet framing and response identities detect corruption; the
    /// Ed25519 receipt verification authenticates the protected result. The
    /// result remains authority-free until a later reviewed boundary binds it
    /// to locally retained request, evidence-custody, and audit owners.
    ///
    /// # Errors
    ///
    /// Returns a typed error for transport, peer-continuity, protocol,
    /// correlation, policy, or signature-authentication failure.
    pub fn request_receipt(
        self,
        policy: &M1AllKernelsProtectedVerifierTrustPolicyV1,
        request: &M1AllKernelsProtectedVerifierServiceRequestV1,
    ) -> Result<
        M1AllKernelsAuthenticatedProtectedVerifierReceiptV1,
        M1AllKernelsProtectedVerifierClientErrorV1,
    > {
        if policy.identity() != request.trust_policy_identity() {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::TrustPolicyMismatch);
        }
        revalidate_peer(&self.peer, self.expected_service, self.peer_pid)?;
        send_packet(&self.peer, request.canonical_bytes(), self.deadline)?;
        let received = receive_packet(&self.peer, self.deadline)?;
        revalidate_peer(&self.peer, self.expected_service, self.peer_pid)?;
        let authenticated = authenticate_application_response_v1(policy, request, &received)
            .map_err(M1AllKernelsProtectedVerifierClientErrorV1::from_application_error)?;
        require_deadline(self.deadline)?;
        Ok(authenticated)
    }

    /// This transport client itself grants no protected-verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

/// Caller-admitted V2 transport after Ferric has pinned the dedicated service peer.
///
/// The generic transport deliberately does not authenticate its peer. This wrapper first applies
/// Ferric's dedicated non-root UID/GID admission and only then transfers the same descriptor into
/// the generic multi-phase client. The value is move-only and grants no verifier authority.
///
/// ```compile_fail
/// use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_verifier_client::
///     M1AllKernelsProtectedVerifierClientV2;
/// fn duplicate(value: M1AllKernelsProtectedVerifierClientV2) { let _again = value.clone(); }
/// ```
pub struct M1AllKernelsProtectedVerifierClientV2 {
    inner: WorkerV3VerificationClientV2,
    expected_service: M1AllKernelsProtectedVerifierServiceIdentityV1,
    peer_pid: u32,
}

impl fmt::Debug for M1AllKernelsProtectedVerifierClientV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AllKernelsProtectedVerifierClientV2")
            .field("expected_service", &self.expected_service)
            .field("peer_pid", &self.peer_pid)
            .field("authority", &"none")
            .finish_non_exhaustive()
    }
}

impl M1AllKernelsProtectedVerifierClientV2 {
    /// Admits one connected unnamed `SOCK_SEQPACKET` peer under dedicated non-root credentials.
    ///
    /// # Errors
    ///
    /// Returns a typed error if Ferric peer admission or generic V2 admission fails.
    pub fn admit(
        peer: OwnedFd,
        expected_service: M1AllKernelsProtectedVerifierServiceIdentityV1,
        timeout: Duration,
    ) -> Result<Self, M1AllKernelsProtectedVerifierClientErrorV2> {
        Self::admit_inner::<true>(peer, expected_service, timeout)
    }

    fn admit_inner<const REQUIRE_DISTINCT_UID: bool>(
        peer: OwnedFd,
        expected_service: M1AllKernelsProtectedVerifierServiceIdentityV1,
        timeout: Duration,
    ) -> Result<Self, M1AllKernelsProtectedVerifierClientErrorV2> {
        let (peer_pid, _deadline) =
            admit_peer::<REQUIRE_DISTINCT_UID>(&peer, expected_service, timeout)
                .map_err(M1AllKernelsProtectedVerifierClientErrorV2::PeerAdmission)?;
        revalidate_peer(&peer, expected_service, peer_pid)
            .map_err(M1AllKernelsProtectedVerifierClientErrorV2::PeerAdmission)?;
        let inner = WorkerV3VerificationClientV2::admit(peer, timeout)
            .map_err(M1AllKernelsProtectedVerifierClientErrorV2::Transport)?;
        debug_assert!(!inner.authenticates_peer());
        Ok(Self {
            inner,
            expected_service,
            peer_pid,
        })
    }

    /// Sends one generic Begin request and its exact immutable payload snapshots.
    ///
    /// Ferric converts a generic Begin rejection into an explicit terminal error. A successful
    /// result retains the generic pending session separately from the move-only service challenge.
    ///
    /// # Errors
    ///
    /// Returns a typed error when snapshot admission, transport, session correlation, or the
    /// service's Begin disposition fails closed.
    pub fn begin(
        self,
        request: WorkerV3VerificationRequestV1,
        descriptors: Vec<OwnedFd>,
    ) -> Result<
        M1AllKernelsReservedProtectedVerifierSessionV2,
        M1AllKernelsProtectedVerifierClientErrorV2,
    > {
        let snapshots = WorkerV3VerificationPayloadSnapshotsV1::admit(&request, descriptors)
            .map_err(M1AllKernelsProtectedVerifierClientErrorV2::Snapshot)?;
        match self
            .inner
            .begin(request, snapshots)
            .map_err(M1AllKernelsProtectedVerifierClientErrorV2::Transport)?
        {
            WorkerV3VerificationBeginOutcomeV2::Reserved(reserved) => {
                let (challenge, pending) = reserved.into_parts();
                Ok(M1AllKernelsReservedProtectedVerifierSessionV2 {
                    challenge,
                    pending: M1AllKernelsPendingProtectedVerifierClientV2 { pending },
                })
            }
            WorkerV3VerificationBeginOutcomeV2::Rejected(rejected) => {
                Err(M1AllKernelsProtectedVerifierClientErrorV2::BeginRejected {
                    request_identity: *rejected.request().identity().as_bytes(),
                })
            }
            _ => Err(M1AllKernelsProtectedVerifierClientErrorV2::UnexpectedBeginOutcome),
        }
    }

    /// Socket admission and protocol framing alone grant no protected-verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

/// Successful Begin result with separate move-only challenge and pending-session custody.
///
/// ```compile_fail
/// use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_verifier_client::
///     M1AllKernelsReservedProtectedVerifierSessionV2;
/// fn duplicate(value: M1AllKernelsReservedProtectedVerifierSessionV2) {
///     let _again = value.clone();
/// }
/// ```
pub struct M1AllKernelsReservedProtectedVerifierSessionV2 {
    challenge: WorkerV3VerificationCurrentRecordChallengeV2,
    pending: M1AllKernelsPendingProtectedVerifierClientV2,
}

impl M1AllKernelsReservedProtectedVerifierSessionV2 {
    /// Separates the service challenge from the pending terminal transport.
    pub fn into_parts(
        self,
    ) -> (
        WorkerV3VerificationCurrentRecordChallengeV2,
        M1AllKernelsPendingProtectedVerifierClientV2,
    ) {
        (self.challenge, self.pending)
    }

    /// A reserved generic session grants no protected-verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

impl fmt::Debug for M1AllKernelsReservedProtectedVerifierSessionV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AllKernelsReservedProtectedVerifierSessionV2")
            .field("challenge", &self.challenge)
            .field("pending", &self.pending)
            .field("authority", &"none")
            .finish()
    }
}

/// Pending Ferric V2 session after the service has released its current-record challenge.
///
/// ```compile_fail
/// use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_verifier_client::
///     M1AllKernelsPendingProtectedVerifierClientV2;
/// fn duplicate(value: M1AllKernelsPendingProtectedVerifierClientV2) {
///     let _again = value.clone();
/// }
/// ```
pub struct M1AllKernelsPendingProtectedVerifierClientV2 {
    pending: PendingWorkerV3VerificationClientV2,
}

impl M1AllKernelsPendingProtectedVerifierClientV2 {
    /// Submits the exact current-record arrays and authenticates the sole application response.
    ///
    /// Generic V2 correlates the terminal frame to the Begin request, service challenge, and
    /// reservation. Ferric separately requires an exact V1 application-response length and a
    /// signature-authenticated receipt correlated to `service_request`.
    ///
    /// # Errors
    ///
    /// Returns a typed error for policy mismatch, transport or terminal-correlation failure,
    /// malformed application bytes, request mismatch, or receipt-authentication failure.
    pub fn submit_current_record(
        self,
        verification: [u8; COMPILER_EXECUTION_CURRENT_RECORD_VERIFICATION_BYTES_V3],
        attestation: [u8; COMPILER_EXECUTION_CURRENT_RECORD_ATTESTATION_BYTES_V3],
        policy: &M1AllKernelsProtectedVerifierTrustPolicyV1,
        service_request: &M1AllKernelsProtectedVerifierServiceRequestV1,
    ) -> Result<
        M1AllKernelsAuthenticatedProtectedVerifierReceiptV1,
        M1AllKernelsProtectedVerifierClientErrorV2,
    > {
        if policy.identity() != service_request.trust_policy_identity() {
            return Err(M1AllKernelsProtectedVerifierClientErrorV2::TrustPolicyMismatch);
        }
        let terminal = self
            .pending
            .submit_current_record(verification, attestation)
            .map_err(M1AllKernelsProtectedVerifierClientErrorV2::Transport)?;
        match terminal.disposition() {
            WorkerV3VerificationTerminalDispositionV2::ApplicationResponse => {}
            WorkerV3VerificationTerminalDispositionV2::Rejected => {
                return Err(M1AllKernelsProtectedVerifierClientErrorV2::TerminalRejected);
            }
            _ => {
                return Err(
                    M1AllKernelsProtectedVerifierClientErrorV2::UnexpectedTerminalDisposition,
                );
            }
        }
        authenticate_application_response_v1(
            policy,
            service_request,
            terminal.application_response_bytes(),
        )
        .map_err(M1AllKernelsProtectedVerifierClientErrorV2::from_application_error)
    }

    /// Pending transport custody alone grants no protected-verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

impl fmt::Debug for M1AllKernelsPendingProtectedVerifierClientV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AllKernelsPendingProtectedVerifierClientV2")
            .field("pending", &self.pending)
            .field("authority", &"none")
            .finish()
    }
}

#[derive(Clone, Copy, Debug)]
enum M1AllKernelsApplicationResponseErrorV1 {
    Length { expected: usize, actual: usize },
    Protocol(M1AllKernelsProtectedVerifierServiceProtocolErrorV1),
    RequestMismatch,
    ReceiptAuthentication(M1AllKernelsProtectedReceiptErrorV1),
}

fn authenticate_application_response_v1(
    policy: &M1AllKernelsProtectedVerifierTrustPolicyV1,
    request: &M1AllKernelsProtectedVerifierServiceRequestV1,
    bytes: &[u8],
) -> Result<
    M1AllKernelsAuthenticatedProtectedVerifierReceiptV1,
    M1AllKernelsApplicationResponseErrorV1,
> {
    if bytes.len() != M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1 {
        return Err(M1AllKernelsApplicationResponseErrorV1::Length {
            expected: M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1,
            actual: bytes.len(),
        });
    }
    let response = M1AllKernelsProtectedVerifierServiceResponseV1::decode(bytes)
        .map_err(M1AllKernelsApplicationResponseErrorV1::Protocol)?;
    if !response.matches_request(request) {
        return Err(M1AllKernelsApplicationResponseErrorV1::RequestMismatch);
    }
    let receipt = response.into_receipt();
    policy
        .authenticate_canonical(receipt.encode_canonical())
        .map_err(M1AllKernelsApplicationResponseErrorV1::ReceiptAuthentication)
}

/// Ferric peer-admission, V2 phase, snapshot, terminal, or receipt-authentication failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum M1AllKernelsProtectedVerifierClientErrorV2 {
    /// Ferric's dedicated UID/GID or peer-shape admission rejected the descriptor.
    PeerAdmission(M1AllKernelsProtectedVerifierClientErrorV1),
    /// Exact immutable payload-snapshot admission failed.
    Snapshot(WorkerV3VerificationClientErrorV1),
    /// Generic V2 transport, framing, or session correlation failed.
    Transport(WorkerV3VerificationClientErrorV2),
    /// The service rejected a structurally valid Begin request.
    BeginRejected {
        /// Exact generic request identity named by the correlated rejection.
        request_identity: [u8; 32],
    },
    /// The generic client returned a future Begin outcome not understood by this Ferric revision.
    UnexpectedBeginOutcome,
    /// The service returned the correlated generic terminal rejection.
    TerminalRejected,
    /// The generic terminal used a disposition not understood by this Ferric revision.
    UnexpectedTerminalDisposition,
    /// The application payload did not have Ferric's exact V1 response length.
    ApplicationResponseLength {
        /// Required Ferric response length.
        expected: usize,
        /// Received application payload length.
        actual: usize,
    },
    /// The Ferric application response was not strict canonical V1 bytes.
    ApplicationProtocol(M1AllKernelsProtectedVerifierServiceProtocolErrorV1),
    /// The Ferric response or receipt named another protected-verifier request.
    ApplicationRequestMismatch,
    /// The caller policy did not match the application service request.
    TrustPolicyMismatch,
    /// Strict Ed25519 receipt authentication under the caller policy failed.
    ReceiptAuthentication(M1AllKernelsProtectedReceiptErrorV1),
}

impl M1AllKernelsProtectedVerifierClientErrorV2 {
    fn from_application_error(error: M1AllKernelsApplicationResponseErrorV1) -> Self {
        match error {
            M1AllKernelsApplicationResponseErrorV1::Length { expected, actual } => {
                Self::ApplicationResponseLength { expected, actual }
            }
            M1AllKernelsApplicationResponseErrorV1::Protocol(source) => {
                Self::ApplicationProtocol(source)
            }
            M1AllKernelsApplicationResponseErrorV1::RequestMismatch => {
                Self::ApplicationRequestMismatch
            }
            M1AllKernelsApplicationResponseErrorV1::ReceiptAuthentication(source) => {
                Self::ReceiptAuthentication(source)
            }
        }
    }
}

impl fmt::Display for M1AllKernelsProtectedVerifierClientErrorV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PeerAdmission(source) => {
                write!(formatter, "protected-verifier V2 peer admission failed: {source}")
            }
            Self::Snapshot(source) => {
                write!(formatter, "protected-verifier V2 snapshot admission failed: {source}")
            }
            Self::Transport(source) => {
                write!(formatter, "protected-verifier V2 transport failed: {source}")
            }
            Self::BeginRejected { request_identity } => write!(
                formatter,
                "protected-verifier V2 Begin was rejected for request {request_identity:02x?}"
            ),
            Self::UnexpectedBeginOutcome => formatter
                .write_str("protected-verifier V2 Begin used an unsupported outcome"),
            Self::TerminalRejected => {
                formatter.write_str("protected-verifier V2 terminal rejected the current record")
            }
            Self::UnexpectedTerminalDisposition => formatter
                .write_str("protected-verifier V2 terminal used an unsupported disposition"),
            Self::ApplicationResponseLength { expected, actual } => write!(
                formatter,
                "protected-verifier V2 application response length {actual} differs from {expected}"
            ),
            Self::ApplicationProtocol(source) => {
                write!(formatter, "protected-verifier V2 application response failed: {source}")
            }
            Self::ApplicationRequestMismatch => formatter.write_str(
                "protected-verifier V2 application response names another request or coordinate set",
            ),
            Self::TrustPolicyMismatch => formatter
                .write_str("protected-verifier V2 request names another trust policy"),
            Self::ReceiptAuthentication(source) => write!(
                formatter,
                "protected-verifier V2 receipt authentication failed: {source}"
            ),
        }
    }
}

impl Error for M1AllKernelsProtectedVerifierClientErrorV2 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PeerAdmission(source) => Some(source),
            Self::Snapshot(source) => Some(source),
            Self::Transport(source) => Some(source),
            Self::ApplicationProtocol(source) => Some(source),
            Self::ReceiptAuthentication(source) => Some(source),
            Self::BeginRejected { .. }
            | Self::UnexpectedBeginOutcome
            | Self::TerminalRejected
            | Self::UnexpectedTerminalDisposition
            | Self::ApplicationResponseLength { .. }
            | Self::ApplicationRequestMismatch
            | Self::TrustPolicyMismatch => None,
        }
    }
}

fn admit_peer<const REQUIRE_DISTINCT_UID: bool>(
    peer: &OwnedFd,
    expected_service: M1AllKernelsProtectedVerifierServiceIdentityV1,
    timeout: Duration,
) -> Result<(u32, Instant), M1AllKernelsProtectedVerifierClientErrorV1> {
    if timeout.is_zero() {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::InvalidTimeout);
    }
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or(M1AllKernelsProtectedVerifierClientErrorV1::DeadlineOverflow)?;
    set_close_on_exec(peer)?;
    validate_seqpacket_peer(peer)?;
    let credentials = peer_credentials(peer)?;
    if credentials.uid != expected_service.uid || credentials.gid != expected_service.gid {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::PeerCredentialsMismatch);
    }
    // SAFETY: geteuid reads one process credential and has no pointer arguments.
    let client_uid = unsafe { libc::geteuid() };
    if REQUIRE_DISTINCT_UID && credentials.uid == client_uid {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::ClientAndServiceUidMatch);
    }
    require_deadline(deadline)?;
    Ok((credentials.pid, deadline))
}

fn set_close_on_exec(peer: &OwnedFd) -> Result<(), M1AllKernelsProtectedVerifierClientErrorV1> {
    // SAFETY: the peer is live and F_GETFD has no pointer arguments.
    let flags = unsafe { libc::fcntl(peer.as_raw_fd(), libc::F_GETFD) };
    if flags < 0 {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::Descriptor(
            io::Error::last_os_error(),
        ));
    }
    // SAFETY: F_SETFD writes only descriptor flags for the retained peer.
    if unsafe { libc::fcntl(peer.as_raw_fd(), libc::F_SETFD, flags | libc::FD_CLOEXEC) } != 0 {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::Descriptor(
            io::Error::last_os_error(),
        ));
    }
    Ok(())
}

fn validate_seqpacket_peer(
    peer: &OwnedFd,
) -> Result<(), M1AllKernelsProtectedVerifierClientErrorV1> {
    let mut socket_type = 0_i32;
    let mut socket_type_len = libc::socklen_t::try_from(mem::size_of::<i32>())
        .expect("socket-type scalar length fits socklen_t");
    // SAFETY: output pointers name initialized writable scalar storage of the declared length.
    if unsafe {
        libc::getsockopt(
            peer.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_TYPE,
            (&raw mut socket_type).cast(),
            &raw mut socket_type_len,
        )
    } != 0
    {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::Descriptor(
            io::Error::last_os_error(),
        ));
    }
    if usize::try_from(socket_type_len).ok() != Some(mem::size_of::<i32>())
        || socket_type != libc::SOCK_SEQPACKET
    {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::NotSeqpacket);
    }
    validate_unnamed_address(peer, false)?;
    validate_unnamed_address(peer, true)
}

fn validate_unnamed_address(
    peer: &OwnedFd,
    remote: bool,
) -> Result<(), M1AllKernelsProtectedVerifierClientErrorV1> {
    // SAFETY: zero is a valid initial sockaddr_storage and the kernel initializes the result.
    let mut address = unsafe { mem::zeroed::<libc::sockaddr_storage>() };
    let mut length = libc::socklen_t::try_from(mem::size_of::<libc::sockaddr_storage>())
        .expect("socket-address length fits socklen_t");
    // SAFETY: the address and length pointers name writable storage of the declared capacity.
    let result = unsafe {
        if remote {
            libc::getpeername(peer.as_raw_fd(), (&raw mut address).cast(), &raw mut length)
        } else {
            libc::getsockname(peer.as_raw_fd(), (&raw mut address).cast(), &raw mut length)
        }
    };
    if result != 0 {
        let error = io::Error::last_os_error();
        if remote && error.raw_os_error() == Some(libc::ENOTCONN) {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::NamedOrNonUnixPeer);
        }
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::Descriptor(
            error,
        ));
    }
    if i32::from(address.ss_family) != libc::AF_UNIX
        || usize::try_from(length).ok() != Some(mem::size_of::<libc::sa_family_t>())
    {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::NamedOrNonUnixPeer);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PeerCredentials {
    pid: u32,
    uid: u32,
    gid: u32,
}

fn peer_credentials(
    peer: &OwnedFd,
) -> Result<PeerCredentials, M1AllKernelsProtectedVerifierClientErrorV1> {
    // SAFETY: zero initializes every scalar field of Linux ucred.
    let mut credentials = unsafe { mem::zeroed::<libc::ucred>() };
    let mut length = libc::socklen_t::try_from(mem::size_of::<libc::ucred>())
        .expect("peer-credential length fits socklen_t");
    // SAFETY: output pointers name initialized writable ucred storage of the declared length.
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
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::PeerCredentials(
            io::Error::last_os_error(),
        ));
    }
    let pid = u32::try_from(credentials.pid)
        .ok()
        .filter(|pid| *pid != 0)
        .ok_or(M1AllKernelsProtectedVerifierClientErrorV1::InvalidPeerPid)?;
    if usize::try_from(length).ok() != Some(mem::size_of::<libc::ucred>())
        || credentials.uid == INVALID_ID
        || credentials.gid == INVALID_ID
    {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::InvalidPeerCredentials);
    }
    Ok(PeerCredentials {
        pid,
        uid: credentials.uid,
        gid: credentials.gid,
    })
}

fn revalidate_peer(
    peer: &OwnedFd,
    expected_service: M1AllKernelsProtectedVerifierServiceIdentityV1,
    expected_pid: u32,
) -> Result<(), M1AllKernelsProtectedVerifierClientErrorV1> {
    validate_seqpacket_peer(peer)?;
    let current = peer_credentials(peer)?;
    if current.pid != expected_pid
        || current.uid != expected_service.uid
        || current.gid != expected_service.gid
    {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::PeerIdentityChanged);
    }
    Ok(())
}

fn send_packet(
    peer: &OwnedFd,
    bytes: &[u8],
    deadline: Instant,
) -> Result<(), M1AllKernelsProtectedVerifierClientErrorV1> {
    loop {
        wait_for_peer(peer, libc::POLLOUT, deadline)?;
        // SAFETY: bytes is readable for its complete length and peer remains owned.
        let sent = unsafe {
            libc::send(
                peer.as_raw_fd(),
                bytes.as_ptr().cast(),
                bytes.len(),
                libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
            )
        };
        if sent < 0 {
            let error = io::Error::last_os_error();
            if matches!(
                error.kind(),
                io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
            ) {
                continue;
            }
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::Send(error));
        }
        if usize::try_from(sent).ok() != Some(bytes.len()) {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::PartialSend);
        }
        return Ok(());
    }
}

fn receive_packet(
    peer: &OwnedFd,
    deadline: Instant,
) -> Result<
    [u8; M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1],
    M1AllKernelsProtectedVerifierClientErrorV1,
> {
    let mut bytes = [0_u8; M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1];
    loop {
        wait_for_peer(peer, libc::POLLIN, deadline)?;
        let mut vector = libc::iovec {
            iov_base: bytes.as_mut_ptr().cast(),
            iov_len: bytes.len(),
        };
        // SAFETY: zero is a valid empty msghdr; the live iovec is installed below.
        let mut header = unsafe { mem::zeroed::<libc::msghdr>() };
        header.msg_iov = &raw mut vector;
        header.msg_iovlen = 1;
        // SAFETY: header names the live output buffer and no ancillary buffer.
        let received = unsafe {
            libc::recvmsg(
                peer.as_raw_fd(),
                &raw mut header,
                libc::MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC,
            )
        };
        if received < 0 {
            let error = io::Error::last_os_error();
            if matches!(
                error.kind(),
                io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
            ) {
                continue;
            }
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::Receive(error));
        }
        if header.msg_flags & libc::MSG_CTRUNC != 0 {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::AncillaryData);
        }
        if header.msg_flags & libc::MSG_TRUNC != 0 {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::PacketTruncated);
        }
        if header.msg_controllen != 0 {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::AncillaryData);
        }
        let received = usize::try_from(received)
            .map_err(|_| M1AllKernelsProtectedVerifierClientErrorV1::PacketTruncated)?;
        if received == 0 {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::PeerClosed);
        }
        if received != bytes.len() {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::ResponseLength {
                expected: bytes.len(),
                actual: received,
            });
        }
        return Ok(bytes);
    }
}

fn wait_for_peer(
    peer: &OwnedFd,
    wanted: i16,
    deadline: Instant,
) -> Result<(), M1AllKernelsProtectedVerifierClientErrorV1> {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::Timeout);
        }
        let mut descriptor = libc::pollfd {
            fd: peer.as_raw_fd(),
            events: wanted | libc::POLLERR | libc::POLLHUP,
            revents: 0,
        };
        // SAFETY: descriptor is a live one-element pollfd array for the complete call.
        let result =
            unsafe { libc::poll(&raw mut descriptor, 1, duration_to_poll_millis(remaining)) };
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::Poll(error));
        }
        if result == 0 || deadline.saturating_duration_since(Instant::now()).is_zero() {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::Timeout);
        }
        if descriptor.revents & libc::POLLNVAL != 0 {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::InvalidPeer);
        }
        if descriptor.revents & wanted != 0 {
            return Ok(());
        }
        if descriptor.revents & libc::POLLERR != 0 {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::PeerFailed);
        }
        if descriptor.revents & libc::POLLHUP != 0 {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::PeerClosed);
        }
    }
}

fn require_deadline(deadline: Instant) -> Result<(), M1AllKernelsProtectedVerifierClientErrorV1> {
    if deadline.saturating_duration_since(Instant::now()).is_zero() {
        Err(M1AllKernelsProtectedVerifierClientErrorV1::Timeout)
    } else {
        Ok(())
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

/// Bounded client admission, transport, correlation, or authentication failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum M1AllKernelsProtectedVerifierClientErrorV1 {
    /// Session timeout must be nonzero.
    InvalidTimeout,
    /// Absolute deadline overflowed.
    DeadlineOverflow,
    /// Descriptor inspection or mutation failed.
    Descriptor(io::Error),
    /// Peer is not a Unix `SOCK_SEQPACKET` endpoint.
    NotSeqpacket,
    /// Peer is named, non-Unix, or disconnected.
    NamedOrNonUnixPeer,
    /// `SO_PEERCRED` inspection failed.
    PeerCredentials(io::Error),
    /// Peer PID is not positive.
    InvalidPeerPid,
    /// Peer credentials contain an invalid sentinel.
    InvalidPeerCredentials,
    /// Peer credentials differ from the caller-pinned identity.
    PeerCredentialsMismatch,
    /// Production client and protected service use the same effective UID.
    ClientAndServiceUidMatch,
    /// Peer process or credentials changed during the exchange.
    PeerIdentityChanged,
    /// Polling the peer failed.
    Poll(io::Error),
    /// Sending the request failed.
    Send(io::Error),
    /// Receiving the response failed.
    Receive(io::Error),
    /// Absolute session deadline expired.
    Timeout,
    /// Peer descriptor became invalid.
    InvalidPeer,
    /// Peer reported an asynchronous socket error.
    PeerFailed,
    /// Peer closed before completing the response.
    PeerClosed,
    /// Response exceeded its sole exact packet bound.
    PacketTruncated,
    /// Peer attempted to transfer ancillary data.
    AncillaryData,
    /// Atomic seqpacket send was partial.
    PartialSend,
    /// Response packet length was not exact.
    ResponseLength {
        /// Required byte length.
        expected: usize,
        /// Observed byte length.
        actual: usize,
    },
    /// Request names another caller-provisioned trust policy.
    TrustPolicyMismatch,
    /// Response does not correlate to the exact request and signed coordinates.
    ResponseRequestMismatch,
    /// Canonical service packet was invalid.
    Protocol(M1AllKernelsProtectedVerifierServiceProtocolErrorV1),
    /// Signed receipt did not authenticate under the caller-provisioned policy.
    ReceiptAuthentication(M1AllKernelsProtectedReceiptErrorV1),
}

impl M1AllKernelsProtectedVerifierClientErrorV1 {
    fn from_application_error(error: M1AllKernelsApplicationResponseErrorV1) -> Self {
        match error {
            M1AllKernelsApplicationResponseErrorV1::Length { expected, actual } => {
                Self::ResponseLength { expected, actual }
            }
            M1AllKernelsApplicationResponseErrorV1::Protocol(source) => Self::Protocol(source),
            M1AllKernelsApplicationResponseErrorV1::RequestMismatch => {
                Self::ResponseRequestMismatch
            }
            M1AllKernelsApplicationResponseErrorV1::ReceiptAuthentication(source) => {
                Self::ReceiptAuthentication(source)
            }
        }
    }
}

impl fmt::Display for M1AllKernelsProtectedVerifierClientErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTimeout => {
                formatter.write_str("protected-verifier timeout must be nonzero")
            }
            Self::DeadlineOverflow => formatter.write_str("protected-verifier deadline overflowed"),
            Self::Descriptor(error) => {
                write!(formatter, "protected-verifier peer is invalid: {error}")
            }
            Self::NotSeqpacket => {
                formatter.write_str("protected-verifier peer is not SOCK_SEQPACKET")
            }
            Self::NamedOrNonUnixPeer => formatter
                .write_str("protected-verifier peer is not a connected unnamed Unix socket"),
            Self::PeerCredentials(error) => write!(
                formatter,
                "protected-verifier peer credentials failed: {error}"
            ),
            Self::InvalidPeerPid => formatter.write_str("protected-verifier peer PID is invalid"),
            Self::InvalidPeerCredentials => {
                formatter.write_str("protected-verifier peer credentials are invalid")
            }
            Self::PeerCredentialsMismatch => formatter.write_str(
                "protected-verifier peer credentials differ from the pinned service identity",
            ),
            Self::ClientAndServiceUidMatch => {
                formatter.write_str("protected-verifier client and service UIDs must differ")
            }
            Self::PeerIdentityChanged => {
                formatter.write_str("protected-verifier peer identity changed during the exchange")
            }
            Self::Poll(error) => write!(formatter, "protected-verifier poll failed: {error}"),
            Self::Send(error) => write!(formatter, "protected-verifier send failed: {error}"),
            Self::Receive(error) => write!(formatter, "protected-verifier receive failed: {error}"),
            Self::Timeout => formatter.write_str("protected-verifier absolute deadline expired"),
            Self::InvalidPeer => {
                formatter.write_str("protected-verifier peer descriptor became invalid")
            }
            Self::PeerFailed => formatter.write_str("protected-verifier peer reported an error"),
            Self::PeerClosed => formatter.write_str("protected-verifier peer closed"),
            Self::PacketTruncated => {
                formatter.write_str("protected-verifier response packet was truncated")
            }
            Self::AncillaryData => {
                formatter.write_str("protected-verifier response carried ancillary data")
            }
            Self::PartialSend => {
                formatter.write_str("protected-verifier request packet send was partial")
            }
            Self::ResponseLength { expected, actual } => write!(
                formatter,
                "protected-verifier response length {actual} differs from exact length {expected}"
            ),
            Self::TrustPolicyMismatch => {
                formatter.write_str("protected-verifier request names another trust policy")
            }
            Self::ResponseRequestMismatch => formatter
                .write_str("protected-verifier response names another request or coordinate set"),
            Self::Protocol(error) => {
                write!(formatter, "protected-verifier protocol failed: {error}")
            }
            Self::ReceiptAuthentication(error) => write!(
                formatter,
                "protected-verifier receipt authentication failed: {error}"
            ),
        }
    }
}

impl Error for M1AllKernelsProtectedVerifierClientErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Descriptor(error)
            | Self::PeerCredentials(error)
            | Self::Poll(error)
            | Self::Send(error)
            | Self::Receive(error) => Some(error),
            Self::Protocol(error) => Some(error),
            Self::ReceiptAuthentication(error) => Some(error),
            _ => None,
        }
    }
}

impl From<M1AllKernelsProtectedVerifierServiceProtocolErrorV1>
    for M1AllKernelsProtectedVerifierClientErrorV1
{
    fn from(error: M1AllKernelsProtectedVerifierServiceProtocolErrorV1) -> Self {
        Self::Protocol(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protected_verifier_service::M1AllKernelsProtectedVerifierServiceResponseV1;
    use crate::protected_verifier_test_support::{fixture_request, signed_fixture};
    use ed25519_dalek::{Signer, SigningKey};
    use fe2o3_artifact_transaction::{
        INERT_COMPILER_EXECUTION_SUBJECT_BYTES_V1, INERT_COMPILER_EXECUTION_SUBJECT_MAGIC_V1,
        INERT_COMPILER_EXECUTION_SUBJECT_VERSION_V1, InertCompilerExecutionSubjectV1,
    };
    use fe2o3_external_anchor_protocol::{
        AnchorPositionV1, AnchorTransitionReceiptV1, AnchoredStateV1, CallerNonceV1,
        HashChainHeadV1, PinnedAnchorKeyV1, UnsignedAnchorObservationV1,
    };
    use fe2o3_runtime_protocol::{
        CompilerExecutionAttestationChallengeV1, CompilerExecutionAttestationReceiptV1,
        CompilerExecutionAttestationRequestV1, CompilerExecutionCurrentRecordAttestationV3,
        CompilerExecutionCurrentRecordVerificationV3, CompilerExecutionExternalAnchorTransactionV1,
        CompilerExecutionIssuerMeasurementV1, CompilerExecutionIssuerPolicyV1,
        CompilerExecutionReceiptCarriageV1, CompilerExecutionReceiptPublicationAckV1,
        CompilerExecutionReceiptPublicationV1,
    };
    use fe2o3_worker_v3_verification_protocol::{
        MAX_WORKER_V3_VERIFICATION_REQUEST_BYTES_V1,
        WORKER_V3_VERIFICATION_CURRENT_RECORD_BYTES_V2, WorkerV3VerificationChallengeFrameV2,
        WorkerV3VerificationChallengeReservationV2, WorkerV3VerificationCurrentRecordFrameV2,
        WorkerV3VerificationEntryCoordinateV1, WorkerV3VerificationFdPayloadDescriptorV1,
        WorkerV3VerificationMeasurementIdentityV1, WorkerV3VerificationPolicyIdentityV1,
        WorkerV3VerificationRosterIdentityV1, WorkerV3VerificationTerminalFrameV2,
    };
    use rustix::fs::{MemfdFlags, OFlags, SealFlags};
    use rustix::net::{
        RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags, SendFlags,
    };
    use sha2::{Digest, Sha256};
    use std::fs::File;
    use std::io::{IoSliceMut, Write};
    use std::mem::MaybeUninit;
    use std::os::fd::{FromRawFd, IntoRawFd};
    use std::os::unix::net::UnixStream;
    use std::ptr;
    use std::thread;

    const SUBJECT_IDENTITY_DOMAIN: &[u8] = b"FE2O3/INERT-COMPILER-EXECUTION-SUBJECT/V1\0";
    const COMPILER_CLOSURE_IDENTITY_DOMAIN: &[u8] = b"fe2o3-compiler-closure-identity-v2\0";
    const GENERIC_ENVELOPE: &[u8] = b"ferric-exact-canonical-v2-envelope";
    const GENERIC_HSACO: &[u8] = b"ferric-exact-finalized-hsaco";

    #[derive(Clone)]
    struct CurrentRecordFixture {
        signing_key: SigningKey,
        anchor_signing_key: SigningKey,
        policy: CompilerExecutionIssuerPolicyV1,
        carriage: CompilerExecutionReceiptCarriageV1,
        publication_request: CompilerExecutionAttestationRequestV1,
        publication: CompilerExecutionReceiptPublicationV1,
    }

    impl CurrentRecordFixture {
        fn new() -> Self {
            let signing_key = SigningKey::from_bytes(&[0x51; 32]);
            let anchor_signing_key = SigningKey::from_bytes(&[0x52; 32]);
            let policy = CompilerExecutionIssuerPolicyV1::new(
                7,
                CompilerExecutionIssuerMeasurementV1::new([0x61; 32], 123).unwrap(),
                CompilerExecutionIssuerMeasurementV1::new([0x62; 32], 456).unwrap(),
                signing_key.verifying_key().to_bytes(),
                anchor_signing_key.verifying_key().to_bytes(),
            )
            .unwrap();
            let subject = compiler_subject(0x20);
            let challenge = CompilerExecutionAttestationChallengeV1::new(
                &policy, &subject, [0x63; 32], 1, [0; 32],
            )
            .unwrap();
            let publication_request =
                CompilerExecutionAttestationRequestV1::new(challenge, subject).unwrap();
            let receipt = CompilerExecutionAttestationReceiptV1::issue(
                &policy,
                &publication_request,
                &signing_key,
            )
            .unwrap();
            let publication =
                CompilerExecutionReceiptPublicationV1::new([0x64; 32], [0x65; 32], receipt)
                    .unwrap();
            let acknowledgment =
                CompilerExecutionReceiptPublicationAckV1::new(&publication, [0x66; 32]).unwrap();
            let carriage = CompilerExecutionReceiptCarriageV1::new(
                policy.clone(),
                publication_request.clone(),
                publication.clone(),
                acknowledgment,
            )
            .unwrap();
            Self {
                signing_key,
                anchor_signing_key,
                policy,
                carriage,
                publication_request,
                publication,
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
                self.policy.clone(),
                self.publication_request.clone(),
                self.publication.clone(),
            )
            .unwrap();
            let anchor_key =
                PinnedAnchorKeyV1::from_bytes(self.anchor_signing_key.verifying_key().to_bytes())
                    .unwrap();
            let pending =
                AnchoredStateV1::from_local_state(0, HashChainHeadV1::from_bytes([0; 32]))
                    .prepare(transaction.external_anchor_digest(), &anchor_key)
                    .unwrap()
                    .begin_advance(CallerNonceV1::from_bytes([0x67; 32]), &anchor_key)
                    .unwrap();
            let unsigned = UnsignedAnchorObservationV1::from_challenge(
                pending.challenge(),
                AnchorPositionV1::Proposed,
            );
            let signature = self
                .anchor_signing_key
                .sign(&unsigned.signing_bytes())
                .to_bytes();
            let commit_receipt = AnchorTransitionReceiptV1::new(
                pending.challenge().clone(),
                &unsigned.attach_signature(signature),
                &anchor_key,
            )
            .unwrap();
            let currentness_challenge =
                CompilerExecutionCurrentRecordVerificationV3::external_anchor_currentness_challenge(
                    &self.carriage,
                    &commit_receipt,
                    challenge,
                )
                .unwrap();
            let unsigned = UnsignedAnchorObservationV1::from_challenge(
                &currentness_challenge,
                AnchorPositionV1::Proposed,
            );
            let signature = self
                .anchor_signing_key
                .sign(&unsigned.signing_bytes())
                .to_bytes();
            let currentness_receipt = AnchorTransitionReceiptV1::new(
                currentness_challenge,
                &unsigned.attach_signature(signature),
                &anchor_key,
            )
            .unwrap();
            let verification = CompilerExecutionCurrentRecordVerificationV3::new(
                &self.carriage,
                commit_receipt,
                currentness_receipt,
                challenge,
                [0x91; 32],
                [0x92; 32],
            )
            .unwrap();
            let attestation = CompilerExecutionCurrentRecordAttestationV3::issue(
                &self.policy,
                &self.carriage,
                verification.clone(),
                challenge,
                &self.signing_key,
            )
            .unwrap();
            (verification, attestation)
        }
    }

    fn generic_request(challenge: u8) -> WorkerV3VerificationRequestV1 {
        generic_request_with_payloads(challenge, GENERIC_ENVELOPE, GENERIC_HSACO)
    }

    fn generic_request_with_payloads(
        challenge: u8,
        envelope: &[u8],
        hsaco: &[u8],
    ) -> WorkerV3VerificationRequestV1 {
        WorkerV3VerificationRequestV1::new(
            WorkerV3VerificationFreshChallengeV1::new([challenge; 32]).unwrap(),
            WorkerV3VerificationRosterIdentityV1::new([0x22; 32]).unwrap(),
            WorkerV3VerificationPolicyIdentityV1::new([0x23; 32]).unwrap(),
            WorkerV3VerificationMeasurementIdentityV1::new([0x24; 32]).unwrap(),
            WorkerV3VerificationFdPayloadDescriptorV1::load_envelope_v2(
                envelope.len() as u64,
                Sha256::digest(envelope).into(),
            )
            .unwrap(),
            WorkerV3VerificationFdPayloadDescriptorV1::finalized_hsaco(
                hsaco.len() as u64,
                Sha256::digest(hsaco).into(),
            )
            .unwrap(),
            vec![
                WorkerV3VerificationEntryCoordinateV1::new(
                    0,
                    "kernel",
                    "kernel_export",
                    [0x31; 32],
                    [0x32; 32],
                    [0x33; 32],
                )
                .unwrap(),
            ],
        )
        .unwrap()
    }

    fn generic_snapshots() -> Vec<OwnedFd> {
        vec![
            crate::sealed_payload_snapshot_v2("ferric-v2-test-envelope", GENERIC_ENVELOPE).unwrap(),
            crate::sealed_payload_snapshot_v2("ferric-v2-test-hsaco", GENERIC_HSACO).unwrap(),
        ]
    }

    fn test_memfd(bytes: &[u8], seals: SealFlags) -> OwnedFd {
        let descriptor = rustix::fs::memfd_create(
            "ferric-v2-hostile-payload",
            MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
        )
        .unwrap();
        let mut writer = File::from(descriptor);
        writer.write_all(bytes).unwrap();
        if !seals.is_empty() {
            rustix::fs::fcntl_add_seals(&writer, seals).unwrap();
        }
        writer.into()
    }

    fn receive_generic_begin(peer: &OwnedFd) -> (WorkerV3VerificationRequestV1, Vec<OwnedFd>) {
        let mut bytes =
            vec![0_u8; MAX_WORKER_V3_VERIFICATION_REQUEST_BYTES_V1 + 1].into_boxed_slice();
        let mut control_space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(3))];
        let mut control = RecvAncillaryBuffer::new(&mut control_space);
        let received = {
            let mut vectors = [IoSliceMut::new(&mut bytes)];
            rustix::net::recvmsg(
                peer,
                &mut vectors,
                &mut control,
                RecvFlags::CMSG_CLOEXEC | RecvFlags::TRUNC,
            )
            .unwrap()
        };
        assert!(
            !received
                .flags
                .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC)
        );
        let mut descriptors = Vec::new();
        for message in control.drain() {
            match message {
                RecvAncillaryMessage::ScmRights(rights) => descriptors.extend(rights),
                _ => panic!("unexpected Begin ancillary message"),
            }
        }
        assert_eq!(descriptors.len(), 2);
        (
            WorkerV3VerificationRequestV1::decode_canonical(&bytes[..received.bytes]).unwrap(),
            descriptors,
        )
    }

    fn read_exact_at(descriptor: &OwnedFd, len: usize) -> Vec<u8> {
        let mut bytes = vec![0; len];
        let mut offset = 0;
        while offset < len {
            let count = rustix::io::pread(descriptor, &mut bytes[offset..], offset as u64).unwrap();
            assert_ne!(count, 0);
            offset += count;
        }
        bytes
    }

    fn receive_current_record(peer: &OwnedFd) -> WorkerV3VerificationCurrentRecordFrameV2 {
        let mut bytes = [0_u8; WORKER_V3_VERIFICATION_CURRENT_RECORD_BYTES_V2];
        let (received, _address) = rustix::net::recv(peer, &mut bytes, RecvFlags::empty()).unwrap();
        assert_eq!(received, bytes.len());
        WorkerV3VerificationCurrentRecordFrameV2::decode_canonical(&bytes).unwrap()
    }

    fn send_exact(peer: &OwnedFd, bytes: &[u8]) {
        assert_eq!(
            rustix::net::send(peer, bytes, SendFlags::NOSIGNAL).unwrap(),
            bytes.len()
        );
    }

    fn compiler_subject(seed: u8) -> InertCompilerExecutionSubjectV1 {
        let closure_pins = [
            [seed; 32],
            [seed + 1; 32],
            [seed + 2; 32],
            [seed + 3; 32],
            [seed + 4; 32],
            [seed + 5; 32],
        ];
        let mut closure_digest = Sha256::new();
        closure_digest.update(COMPILER_CLOSURE_IDENTITY_DOMAIN);
        closure_digest.update(1_u16.to_le_bytes());
        for pin in closure_pins {
            closure_digest.update(pin);
        }
        let closure_identity: [u8; 32] = closure_digest.finalize().into();
        let mut bytes = [0_u8; INERT_COMPILER_EXECUTION_SUBJECT_BYTES_V1];
        let mut offset = 0;
        put_test_bytes(
            &mut bytes,
            &mut offset,
            &INERT_COMPILER_EXECUTION_SUBJECT_MAGIC_V1,
        );
        put_test_bytes(
            &mut bytes,
            &mut offset,
            &INERT_COMPILER_EXECUTION_SUBJECT_VERSION_V1.to_le_bytes(),
        );
        put_test_bytes(&mut bytes, &mut offset, &0_u16.to_le_bytes());
        put_test_bytes(
            &mut bytes,
            &mut offset,
            &(INERT_COMPILER_EXECUTION_SUBJECT_BYTES_V1 as u64).to_le_bytes(),
        );
        put_test_bytes(&mut bytes, &mut offset, &0_u32.to_le_bytes());
        put_test_bytes(&mut bytes, &mut offset, &9_u64.to_le_bytes());
        put_test_bytes(&mut bytes, &mut offset, &[seed + 6; 16]);
        put_test_bytes(&mut bytes, &mut offset, &[seed + 7; 32]);
        bytes[offset] = 0;
        offset += 8;
        put_test_bytes(&mut bytes, &mut offset, &[seed + 8; 32]);
        put_test_bytes(&mut bytes, &mut offset, &[seed + 9; 32]);
        for pin in closure_pins {
            put_test_bytes(&mut bytes, &mut offset, &pin);
        }
        put_test_bytes(&mut bytes, &mut offset, &1_u16.to_le_bytes());
        put_test_bytes(&mut bytes, &mut offset, &closure_identity);
        for axis in 0_u8..7 {
            put_test_bytes(&mut bytes, &mut offset, &[seed + 10 + axis; 32]);
            put_test_bytes(
                &mut bytes,
                &mut offset,
                &(1_000_u64 + u64::from(axis)).to_le_bytes(),
            );
        }
        let identity = test_digest(SUBJECT_IDENTITY_DOMAIN, &bytes[..offset]);
        put_test_bytes(&mut bytes, &mut offset, &identity);
        assert_eq!(offset, bytes.len());
        InertCompilerExecutionSubjectV1::decode(&bytes).unwrap()
    }

    fn test_digest(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
        let mut digest = Sha256::new();
        digest.update(domain);
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
        digest.finalize().into()
    }

    fn put_test_bytes(output: &mut [u8], offset: &mut usize, value: &[u8]) {
        let end = *offset + value.len();
        output[*offset..end].copy_from_slice(value);
        *offset = end;
    }

    fn socketpair() -> (OwnedFd, OwnedFd) {
        let mut descriptors = [-1_i32; 2];
        // SAFETY: descriptors names writable storage for exactly two returned file descriptors.
        assert_eq!(
            unsafe {
                libc::socketpair(
                    libc::AF_UNIX,
                    libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
                    0,
                    descriptors.as_mut_ptr(),
                )
            },
            0,
        );
        // SAFETY: successful socketpair returned two distinct uniquely owned descriptors.
        unsafe {
            (
                OwnedFd::from_raw_fd(descriptors[0]),
                OwnedFd::from_raw_fd(descriptors[1]),
            )
        }
    }

    fn current_service_identity() -> M1AllKernelsProtectedVerifierServiceIdentityV1 {
        // Tests exercise the same-UID internal admission path only.
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        M1AllKernelsProtectedVerifierServiceIdentityV1 { uid, gid }
    }

    fn assert_v2_snapshot_rejected(
        request: WorkerV3VerificationRequestV1,
        descriptors: Vec<OwnedFd>,
    ) {
        let (client_peer, _service_peer) = socketpair();
        let client = M1AllKernelsProtectedVerifierClientV2::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(matches!(
            client.begin(request, descriptors),
            Err(M1AllKernelsProtectedVerifierClientErrorV2::Snapshot(_))
        ));
    }

    fn spawn_response(peer: OwnedFd, bytes: Vec<u8>) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let mut request = [0_u8; 2_304];
            // SAFETY: request is writable and the peer descriptor is live for this thread.
            let received = unsafe {
                libc::recv(
                    peer.as_raw_fd(),
                    request.as_mut_ptr().cast(),
                    request.len(),
                    0,
                )
            };
            assert_eq!(usize::try_from(received).unwrap(), request.len());
            // SAFETY: bytes is readable for the complete requested length.
            let sent = unsafe {
                libc::send(
                    peer.as_raw_fd(),
                    bytes.as_ptr().cast(),
                    bytes.len(),
                    libc::MSG_NOSIGNAL,
                )
            };
            assert_eq!(usize::try_from(sent).unwrap(), bytes.len());
        })
    }

    fn spawn_response_with_ancillary_fd(
        peer: OwnedFd,
        mut bytes: Vec<u8>,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let mut request = [0_u8; 2_304];
            // SAFETY: request is writable and the peer descriptor is live for this thread.
            let received = unsafe {
                libc::recv(
                    peer.as_raw_fd(),
                    request.as_mut_ptr().cast(),
                    request.len(),
                    0,
                )
            };
            assert_eq!(usize::try_from(received).unwrap(), request.len());

            let carried = File::open("/dev/null").unwrap();
            let descriptor_bytes = u32::try_from(mem::size_of::<i32>()).unwrap();
            let control_len =
                usize::try_from(unsafe { libc::CMSG_SPACE(descriptor_bytes) }).unwrap();
            let mut control = vec![0_u64; control_len.div_ceil(mem::size_of::<u64>())];
            let mut vector = libc::iovec {
                iov_base: bytes.as_mut_ptr().cast(),
                iov_len: bytes.len(),
            };
            // SAFETY: zero is a valid empty msghdr; live payload and control storage are installed.
            let mut header = unsafe { mem::zeroed::<libc::msghdr>() };
            header.msg_iov = &raw mut vector;
            header.msg_iovlen = 1;
            header.msg_control = control.as_mut_ptr().cast();
            header.msg_controllen = control_len;
            // SAFETY: the control buffer is suitably aligned and has CMSG_SPACE bytes.
            let message = unsafe { libc::CMSG_FIRSTHDR(&raw const header) };
            assert!(!message.is_null());
            // SAFETY: message points inside the live control buffer with room for one descriptor.
            unsafe {
                (*message).cmsg_level = libc::SOL_SOCKET;
                (*message).cmsg_type = libc::SCM_RIGHTS;
                (*message).cmsg_len = usize::try_from(libc::CMSG_LEN(descriptor_bytes)).unwrap();
                let carried_fd = carried.as_raw_fd();
                ptr::copy_nonoverlapping(
                    (&raw const carried_fd).cast::<u8>(),
                    libc::CMSG_DATA(message),
                    mem::size_of::<i32>(),
                );
            }
            // SAFETY: header names live payload and a complete SCM_RIGHTS control message.
            let sent =
                unsafe { libc::sendmsg(peer.as_raw_fd(), &raw const header, libc::MSG_NOSIGNAL) };
            assert_eq!(usize::try_from(sent).unwrap(), bytes.len());
        })
    }

    #[test]
    fn one_shot_client_returns_only_caller_authenticated_receipt() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        let response =
            M1AllKernelsProtectedVerifierServiceResponseV1::new(&request, receipt).unwrap();
        let (client_peer, service_peer) = socketpair();
        let server = spawn_response(service_peer, response.canonical_bytes().to_vec());
        let client = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(!client.grants_authority());
        let authenticated = client.request_receipt(&policy, &request).unwrap();
        assert_eq!(
            authenticated.receipt().identity(),
            response.receipt().identity()
        );
        assert!(!authenticated.grants_verifier_authority());
        server.join().unwrap();
    }

    #[test]
    fn admission_failure_returns_the_exact_owned_peer() {
        let (stream, _other) = UnixStream::pair().unwrap();
        let raw = stream.into_raw_fd();
        // SAFETY: into_raw_fd transferred unique ownership.
        let peer = unsafe { OwnedFd::from_raw_fd(raw) };
        let original = peer.as_raw_fd();
        let failure = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            peer,
            current_service_identity(),
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            M1AllKernelsProtectedVerifierClientErrorV1::NotSeqpacket
        ));
        let peer = failure.into_peer();
        assert_eq!(peer.as_raw_fd(), original);
        assert!(unsafe { libc::fcntl(peer.as_raw_fd(), libc::F_GETFD) } >= 0);
    }

    #[test]
    fn admission_rejects_zero_timeout_and_wrong_credentials() {
        let (peer, _other) = socketpair();
        let failure = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            peer,
            current_service_identity(),
            Duration::ZERO,
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            M1AllKernelsProtectedVerifierClientErrorV1::InvalidTimeout
        ));

        let (peer, _other) = socketpair();
        let current = current_service_identity();
        let wrong = M1AllKernelsProtectedVerifierServiceIdentityV1::new(
            current.uid().saturating_add(1).max(1),
            current.gid(),
        )
        .unwrap();
        let failure = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            peer,
            wrong,
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            M1AllKernelsProtectedVerifierClientErrorV1::PeerCredentialsMismatch
        ));
    }

    #[test]
    fn public_admission_rejects_same_uid() {
        let (peer, _other) = socketpair();
        let original = peer.as_raw_fd();
        let failure = M1AllKernelsProtectedVerifierClientV1::admit(
            peer,
            current_service_identity(),
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            M1AllKernelsProtectedVerifierClientErrorV1::ClientAndServiceUidMatch
        ));
        assert_eq!(failure.into_peer().as_raw_fd(), original);
    }

    #[test]
    fn truncated_and_oversized_packets_fail_closed() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        let response =
            M1AllKernelsProtectedVerifierServiceResponseV1::new(&request, receipt).unwrap();
        for bytes in [
            response.canonical_bytes()[..response.canonical_bytes().len() - 1].to_vec(),
            {
                let mut oversized = response.canonical_bytes().to_vec();
                oversized.push(0);
                oversized
            },
        ] {
            let oversized = bytes.len() > response.canonical_bytes().len();
            let (client_peer, service_peer) = socketpair();
            let server = spawn_response(service_peer, bytes);
            let client = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
                client_peer,
                current_service_identity(),
                Duration::from_secs(2),
            )
            .unwrap();
            let error = client.request_receipt(&policy, &request).unwrap_err();
            if oversized {
                assert!(matches!(
                    error,
                    M1AllKernelsProtectedVerifierClientErrorV1::PacketTruncated
                ));
            } else {
                assert!(matches!(
                    error,
                    M1AllKernelsProtectedVerifierClientErrorV1::ResponseLength { .. }
                ));
            }
            server.join().unwrap();
        }
    }

    #[test]
    fn response_with_ancillary_descriptor_fails_closed() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        let response =
            M1AllKernelsProtectedVerifierServiceResponseV1::new(&request, receipt).unwrap();
        let (client_peer, service_peer) = socketpair();
        let server =
            spawn_response_with_ancillary_fd(service_peer, response.canonical_bytes().to_vec());
        let client = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(matches!(
            client.request_receipt(&policy, &request),
            Err(M1AllKernelsProtectedVerifierClientErrorV1::AncillaryData)
        ));
        server.join().unwrap();
    }

    #[test]
    fn caller_policy_substitution_fails_before_transport() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        let (other_policy, _) =
            crate::protected_verifier_test_support::signed_fixture_with_seed(0x92);
        let (client_peer, _service_peer) = socketpair();
        let client = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_secs(1),
        )
        .unwrap();
        assert!(matches!(
            client.request_receipt(&other_policy, &request),
            Err(M1AllKernelsProtectedVerifierClientErrorV1::TrustPolicyMismatch)
        ));
    }

    #[test]
    fn response_for_another_request_fails_correlation() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        let mut entries = *request.entries();
        entries[0] =
            crate::protected_verifier_service::M1AllKernelsProtectedVerifierServiceEntryV1::new(
                0,
                [0xee; 32],
                entries[0].marker_binding_identity(),
                entries[0].generated_host_contract_identity(),
            )
            .unwrap();
        let other_request = M1AllKernelsProtectedVerifierServiceRequestV1::new(
            policy.identity(),
            *request.request_claims(),
            *request.compiler_claims(),
            entries,
        )
        .unwrap();
        let response =
            M1AllKernelsProtectedVerifierServiceResponseV1::new(&request, receipt).unwrap();
        let (client_peer, service_peer) = socketpair();
        let server = spawn_response(service_peer, response.canonical_bytes().to_vec());
        let client = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(matches!(
            client.request_receipt(&policy, &other_request),
            Err(M1AllKernelsProtectedVerifierClientErrorV1::ResponseRequestMismatch)
        ));
        server.join().unwrap();
    }

    #[test]
    fn structurally_correlated_response_with_bad_signature_fails_authentication() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        let mut hostile_bytes = *receipt.encode_canonical();
        let signature_offset = hostile_bytes.len() - 1;
        hostile_bytes[signature_offset] ^= 1;
        let hostile_receipt =
            crate::protected_receipt::M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(
                &hostile_bytes,
            )
            .unwrap();
        let response =
            M1AllKernelsProtectedVerifierServiceResponseV1::new(&request, hostile_receipt).unwrap();
        let (client_peer, service_peer) = socketpair();
        let server = spawn_response(service_peer, response.canonical_bytes().to_vec());
        let client = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(matches!(
            client.request_receipt(&policy, &request),
            Err(
                M1AllKernelsProtectedVerifierClientErrorV1::ReceiptAuthentication(
                    M1AllKernelsProtectedReceiptErrorV1::SignatureRejected
                )
            )
        ));
        server.join().unwrap();
    }

    #[test]
    fn silent_peer_hits_the_single_absolute_deadline() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        let (client_peer, _service_peer) = socketpair();
        let client = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_millis(20),
        )
        .unwrap();
        assert!(matches!(
            client.request_receipt(&policy, &request),
            Err(M1AllKernelsProtectedVerifierClientErrorV1::Timeout)
        ));
    }

    #[derive(Clone, Copy)]
    enum V2TerminalCase {
        Valid,
        Rejected,
        WrongGenericSession,
        WrongApplicationLength,
        WrongApplicationRequest,
        BadReceiptSignature,
    }

    #[allow(clippy::too_many_lines)]
    fn run_v2_terminal_case(
        terminal_case: V2TerminalCase,
    ) -> Result<
        M1AllKernelsAuthenticatedProtectedVerifierReceiptV1,
        M1AllKernelsProtectedVerifierClientErrorV2,
    > {
        let generic = generic_request(0x21);
        let expected_generic = generic.clone();
        let (policy, receipt) = signed_fixture();
        let service_request = fixture_request(&policy, &receipt);
        let application = match terminal_case {
            V2TerminalCase::WrongApplicationLength => vec![0; 17],
            V2TerminalCase::WrongApplicationRequest => {
                let (other_policy, other_receipt) =
                    crate::protected_verifier_test_support::signed_fixture_with_seed(0x97);
                let other_request = fixture_request(&other_policy, &other_receipt);
                M1AllKernelsProtectedVerifierServiceResponseV1::new(&other_request, other_receipt)
                    .unwrap()
                    .canonical_bytes()
                    .to_vec()
            }
            V2TerminalCase::BadReceiptSignature => {
                let mut hostile = *receipt.encode_canonical();
                let final_byte = hostile.len() - 1;
                hostile[final_byte] ^= 1;
                let hostile =
                    crate::protected_receipt::M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(
                        &hostile,
                    )
                    .unwrap();
                M1AllKernelsProtectedVerifierServiceResponseV1::new(&service_request, hostile)
                    .unwrap()
                    .canonical_bytes()
                    .to_vec()
            }
            V2TerminalCase::Valid
            | V2TerminalCase::Rejected
            | V2TerminalCase::WrongGenericSession => {
                M1AllKernelsProtectedVerifierServiceResponseV1::new(&service_request, receipt)
                    .unwrap()
                    .canonical_bytes()
                    .to_vec()
            }
        };
        let current = CurrentRecordFixture::new();
        let service_challenge = [0x71; 32];
        let (verification, attestation) = current.records(service_challenge);
        let verification_bytes = *verification.canonical_bytes();
        let attestation_bytes = *attestation.canonical_bytes();
        let (client_peer, service_peer) = socketpair();
        let server = thread::spawn(move || {
            let (received_request, descriptors) = receive_generic_begin(&service_peer);
            assert_eq!(received_request, expected_generic);
            assert_eq!(
                read_exact_at(&descriptors[0], GENERIC_ENVELOPE.len()),
                GENERIC_ENVELOPE
            );
            assert_eq!(
                read_exact_at(&descriptors[1], GENERIC_HSACO.len()),
                GENERIC_HSACO
            );
            for descriptor in &descriptors {
                assert_eq!(
                    rustix::fs::fcntl_getfl(descriptor).unwrap() & OFlags::ACCMODE,
                    OFlags::RDONLY
                );
            }
            let reservation =
                WorkerV3VerificationChallengeReservationV2::new(service_challenge, [0x72; 32])
                    .unwrap();
            let challenge =
                WorkerV3VerificationChallengeFrameV2::reserved(&received_request, &reservation);
            send_exact(&service_peer, challenge.encode_canonical());
            let submitted = receive_current_record(&service_peer);
            assert!(submitted.matches_session(&received_request, &reservation));
            assert_eq!(
                submitted.verification().canonical_bytes(),
                &verification_bytes
            );
            assert_eq!(
                submitted.attestation().canonical_bytes(),
                &attestation_bytes
            );
            let terminal = match terminal_case {
                V2TerminalCase::Rejected => {
                    WorkerV3VerificationTerminalFrameV2::rejected(&received_request, &reservation)
                }
                V2TerminalCase::WrongGenericSession => {
                    WorkerV3VerificationTerminalFrameV2::application_response(
                        &generic_request(0x22),
                        &reservation,
                        application,
                    )
                    .unwrap()
                }
                _ => WorkerV3VerificationTerminalFrameV2::application_response(
                    &received_request,
                    &reservation,
                    application,
                )
                .unwrap(),
            };
            send_exact(&service_peer, terminal.encode_canonical());
        });
        let client = M1AllKernelsProtectedVerifierClientV2::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_secs(2),
        )
        .unwrap();
        let reserved = client.begin(generic, generic_snapshots()).unwrap();
        let (challenge, pending) = reserved.into_parts();
        assert_eq!(challenge.as_bytes(), &service_challenge);
        let _compiler_challenge = challenge.into_compiler_execution_challenge().unwrap();
        let result = pending.submit_current_record(
            verification_bytes,
            attestation_bytes,
            &policy,
            &service_request,
        );
        server.join().unwrap();
        result
    }

    #[test]
    fn v2_exchange_preserves_phase_order_exact_payloads_and_authenticates_receipt() {
        let authenticated = run_v2_terminal_case(V2TerminalCase::Valid).unwrap();
        assert!(!authenticated.grants_verifier_authority());
    }

    #[test]
    fn v2_terminal_rejection_and_generic_session_substitution_fail_closed() {
        assert!(matches!(
            run_v2_terminal_case(V2TerminalCase::Rejected),
            Err(M1AllKernelsProtectedVerifierClientErrorV2::TerminalRejected)
        ));
        assert!(matches!(
            run_v2_terminal_case(V2TerminalCase::WrongGenericSession),
            Err(M1AllKernelsProtectedVerifierClientErrorV2::Transport(
                WorkerV3VerificationClientErrorV2::SessionMismatch
            ))
        ));
    }

    #[test]
    fn v2_application_length_request_and_signature_substitution_fail_closed() {
        assert!(matches!(
            run_v2_terminal_case(V2TerminalCase::WrongApplicationLength),
            Err(M1AllKernelsProtectedVerifierClientErrorV2::ApplicationResponseLength { .. })
        ));
        assert!(matches!(
            run_v2_terminal_case(V2TerminalCase::WrongApplicationRequest),
            Err(M1AllKernelsProtectedVerifierClientErrorV2::ApplicationRequestMismatch)
        ));
        assert!(matches!(
            run_v2_terminal_case(V2TerminalCase::BadReceiptSignature),
            Err(
                M1AllKernelsProtectedVerifierClientErrorV2::ReceiptAuthentication(
                    M1AllKernelsProtectedReceiptErrorV1::SignatureRejected
                )
            )
        ));
    }

    #[test]
    fn v2_begin_rejection_is_typed_and_payload_order_is_strict() {
        let generic = generic_request(0x31);
        let expected = generic.clone();
        let (client_peer, service_peer) = socketpair();
        let server = thread::spawn(move || {
            let (received, _descriptors) = receive_generic_begin(&service_peer);
            assert_eq!(received, expected);
            let rejected = WorkerV3VerificationChallengeFrameV2::rejected(&received);
            send_exact(&service_peer, rejected.encode_canonical());
        });
        let client = M1AllKernelsProtectedVerifierClientV2::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(matches!(
            client.begin(generic, generic_snapshots()),
            Err(M1AllKernelsProtectedVerifierClientErrorV2::BeginRejected { .. })
        ));
        server.join().unwrap();

        let generic = generic_request(0x32);
        let (client_peer, _service_peer) = socketpair();
        let client = M1AllKernelsProtectedVerifierClientV2::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_secs(2),
        )
        .unwrap();
        let mut descriptors = generic_snapshots();
        descriptors.swap(0, 1);
        assert!(matches!(
            client.begin(generic, descriptors),
            Err(M1AllKernelsProtectedVerifierClientErrorV2::Snapshot(_))
        ));
    }

    #[test]
    fn v2_payload_snapshots_reject_mutation_length_digest_and_inode_aliasing() {
        let seals = SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL;
        assert_v2_snapshot_rejected(
            generic_request(0x41),
            vec![
                test_memfd(GENERIC_ENVELOPE, SealFlags::empty()),
                test_memfd(GENERIC_HSACO, seals),
            ],
        );
        let mut trailing = GENERIC_ENVELOPE.to_vec();
        trailing.push(0);
        assert_v2_snapshot_rejected(
            generic_request(0x42),
            vec![
                test_memfd(&trailing, seals),
                test_memfd(GENERIC_HSACO, seals),
            ],
        );
        let mut altered = GENERIC_ENVELOPE.to_vec();
        altered[0] ^= 1;
        assert_v2_snapshot_rejected(
            generic_request(0x43),
            vec![
                test_memfd(&altered, seals),
                test_memfd(GENERIC_HSACO, seals),
            ],
        );
        let aliased_request =
            generic_request_with_payloads(0x44, GENERIC_ENVELOPE, GENERIC_ENVELOPE);
        let first = test_memfd(GENERIC_ENVELOPE, seals);
        let second = rustix::io::dup(&first).unwrap();
        assert_v2_snapshot_rejected(aliased_request, vec![first, second]);
    }

    #[test]
    fn begin_challenge_rejects_zero_and_exposes_no_implicit_generation() {
        assert!(matches!(
            unsafe {
                M1AllKernelsProtectedVerifierBeginChallengeV2::from_durable_reservation([0; 32])
            },
            Err(WorkerV3VerificationProtocolErrorV1::ZeroIdentity(_))
        ));
        let challenge = unsafe {
            M1AllKernelsProtectedVerifierBeginChallengeV2::from_durable_reservation([0x81; 32])
        }
        .unwrap();
        assert!(!challenge.grants_authority());
    }
}
