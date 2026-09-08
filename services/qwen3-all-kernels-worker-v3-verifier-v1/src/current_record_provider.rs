//! Descriptor-only server for one externally protected compiler-current authority.
//!
//! The server owns one supervisor-preopened connected Unix `SOCK_SEQPACKET`
//! endpoint and one measured authority implementation. It revalidates the fixed
//! request protocol before the authority sees any input and emits only the
//! canonical correlated response format consumed by `current_record_ipc`.
//! Filesystem discovery, key material, durable session state, and authority
//! provisioning remain outside this module.

#![allow(
    clippy::must_use_candidate,
    reason = "public accessors expose only inert protocol coordinates"
)]

use core::fmt;
use std::error::Error;
use std::io;
use std::mem;
use std::os::fd::{AsRawFd, OwnedFd};
use std::time::{Duration, Instant};

use fe2o3_runtime_protocol::CompilerExecutionReceiptCarriageV1;
use fe2o3_worker_v3_verification_protocol::{
    WorkerV3VerificationCurrentRecordFrameV2, WorkerV3VerificationRequestV1,
};
use rustix::fs::{FileType, OFlags};
use rustix::io::FdFlags;
use sha2::{Digest, Sha256};

use crate::{
    PREOPENED_PROTECTED_COMPILER_CURRENT_REQUEST_BYTES_V1,
    ProtectedCompilerCurrentEndpointIdentityV1, ProtectedCompilerCurrentPeerIdentityV1,
    ProtectedCompilerCurrentProtocolErrorV1, ProtectedCompilerCurrentRequestV1,
    ProtectedCompilerCurrentResponseV1,
};

const INVALID_LINUX_ID: u32 = u32::MAX;
const PROVIDER_TRANSCRIPT_DOMAIN_V1: &[u8] =
    b"FERRIC/PROTECTED-COMPILER-CURRENT-RECORD/PROVIDER-TRANSCRIPT/V1\0";

/// Complete independently decoded handoff to one measured compiler-current authority.
pub struct ProtectedCompilerCurrentAuthorityInputV1<'a> {
    wire_request: &'a ProtectedCompilerCurrentRequestV1,
    begin_request: &'a WorkerV3VerificationRequestV1,
    carriage: &'a CompilerExecutionReceiptCarriageV1,
    current_record: &'a WorkerV3VerificationCurrentRecordFrameV2,
    deadline: Instant,
}

impl ProtectedCompilerCurrentAuthorityInputV1<'_> {
    /// Returns the complete canonical provider request and all bound coordinates.
    pub const fn wire_request(&self) -> &ProtectedCompilerCurrentRequestV1 {
        self.wire_request
    }

    /// Returns the independently decoded complete canonical Begin request.
    pub const fn begin_request(&self) -> &WorkerV3VerificationRequestV1 {
        self.begin_request
    }

    /// Returns the independently decoded exact receipt carriage.
    pub const fn carriage(&self) -> &CompilerExecutionReceiptCarriageV1 {
        self.carriage
    }

    /// Returns the independently decoded and signature-verified current-record frame.
    pub const fn current_record(&self) -> &WorkerV3VerificationCurrentRecordFrameV2 {
        self.current_record
    }

    /// Returns the caller-supplied sole absolute operation deadline.
    pub const fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Returns the complete envelope length committed by the Begin descriptor.
    pub const fn envelope_length(&self) -> u64 {
        self.wire_request.envelope_length()
    }

    /// Returns the complete envelope digest committed by the Begin descriptor.
    pub const fn envelope_sha256(&self) -> [u8; 32] {
        self.wire_request.envelope_sha256()
    }
}

/// Move-only authentication result minted only by a reviewed protected authority.
///
/// The server binds this authority transcript to the exact canonical request
/// before returning it to the verifier client.
pub struct ProtectedCompilerCurrentAuthorityAuthenticationV1 {
    transcript_identity: [u8; 32],
}

impl ProtectedCompilerCurrentAuthorityAuthenticationV1 {
    /// Mints a nonzero transcript after external policy/currentness authentication.
    ///
    /// # Safety
    ///
    /// The caller must have authenticated every field in the supplied authority
    /// input against independently protected issuer policy, exact durable Worker
    /// ledger state, the external monotonic rollback authority, and a durable
    /// non-reuse/session store. It must also have verified the complete envelope
    /// identified by its length and digest, or rejected when those retained bytes
    /// were unavailable. The transcript must uniquely identify those checks and
    /// the exact request.
    ///
    /// # Errors
    ///
    /// Returns a typed error if `transcript_identity` is zero.
    pub unsafe fn from_protected_authority(
        transcript_identity: [u8; 32],
    ) -> Result<Self, ProtectedCompilerCurrentAuthorityClaimErrorV1> {
        if transcript_identity == [0; 32] {
            return Err(ProtectedCompilerCurrentAuthorityClaimErrorV1::ZeroTranscript);
        }
        Ok(Self {
            transcript_identity,
        })
    }

    /// Returns the protected authority's nonzero transcript identity.
    pub const fn transcript_identity(&self) -> [u8; 32] {
        self.transcript_identity
    }
}

impl fmt::Debug for ProtectedCompilerCurrentAuthorityAuthenticationV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProtectedCompilerCurrentAuthorityAuthenticationV1")
            .field("transcript_identity", &self.transcript_identity)
            .field("authority", &"external measured authority")
            .finish_non_exhaustive()
    }
}

/// Invalid claim from a protected compiler-current authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedCompilerCurrentAuthorityClaimErrorV1 {
    /// The authority supplied no authentication transcript.
    ZeroTranscript,
}

impl fmt::Display for ProtectedCompilerCurrentAuthorityClaimErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid protected compiler-current authority claim: {self:?}"
        )
    }
}

impl Error for ProtectedCompilerCurrentAuthorityClaimErrorV1 {}

/// Narrow boundary implemented by the independently protected currentness authority.
///
/// # Safety
///
/// Implementations may return authentication only after satisfying
/// [`ProtectedCompilerCurrentAuthorityAuthenticationV1::from_protected_authority`].
/// The implementation must durably prevent reuse of an admission-session identity
/// for its measurement and policy, require exact monotonically increasing request
/// sequences, and retain that state across daemon and supervisor restarts. It must
/// use only the supplied absolute deadline and support cancellation; a synchronous
/// implementation that does not return remains an unsafe deployment obligation.
pub unsafe trait ProtectedCompilerCurrentAuthorityV1 {
    /// Concrete authority rejection or operational failure.
    type Error: Error + Send + Sync + 'static;

    /// Returns the provider measurement pinned by the protected supervisor.
    fn provider_measurement(&self) -> [u8; 32];

    /// Returns the exact independently provisioned compiler policy identity.
    fn compiler_policy_identity(&self) -> [u8; 32];

    /// Authenticates one exact canonical compiler-current request.
    ///
    /// # Errors
    ///
    /// Returns a concrete error for any policy, ledger, rollback, session,
    /// envelope, signature, or operational rejection. The wire peer receives one
    /// generic correlated rejection and cannot distinguish these causes.
    fn authenticate_current_record(
        &mut self,
        input: ProtectedCompilerCurrentAuthorityInputV1<'_>,
    ) -> Result<ProtectedCompilerCurrentAuthorityAuthenticationV1, Self::Error>;
}

/// Admission failure for a preopened protected provider endpoint.
#[derive(Debug)]
#[non_exhaustive]
pub enum ProtectedCompilerCurrentServerAdmissionErrorV1 {
    /// Protocol identity was zero.
    ZeroProtocolIdentity,
    /// Provider measurement was zero.
    ZeroProviderMeasurement,
    /// Compiler policy identity was zero.
    ZeroCompilerPolicyIdentity,
    /// Admission-session identity was zero.
    ZeroAdmissionSessionIdentity,
    /// The owned descriptor could not be inspected.
    Descriptor(io::Error),
    /// Descriptor type, access mode, or flags were not exact.
    DescriptorShape,
    /// Descriptor device/inode did not match the supervisor pin.
    EndpointIdentityMismatch,
    /// Endpoint was not a connected, non-listening Unix `SOCK_SEQPACKET`.
    SocketShape,
    /// A socket option returned a scalar with the wrong length.
    SocketOptionLength {
        /// Required scalar byte length.
        expected: usize,
        /// Observed scalar byte length.
        actual: usize,
    },
    /// The socket carried a pending kernel error.
    PendingSocketError {
        /// Raw operating-system error.
        raw: i32,
    },
    /// Peer credentials could not be read.
    PeerCredentials(io::Error),
    /// Peer credentials contained a root, invalid, or nonpositive coordinate.
    InvalidPeerCredentials,
    /// Kernel peer credentials did not match the supervisor pin.
    PeerIdentityMismatch,
    /// Production provider and verifier UIDs were not distinct.
    ProviderAndVerifierUidMatch,
    /// The supervisor did not supply a fresh empty connection.
    PrequeuedPacket,
    /// The measured authority did not match the pinned provider measurement.
    AuthorityMeasurementMismatch,
    /// The authority did not expose the exact pinned compiler policy.
    AuthorityPolicyMismatch,
}

impl fmt::Display for ProtectedCompilerCurrentServerAdmissionErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "protected compiler-current server admission failed: {self:?}"
        )
    }
}

impl Error for ProtectedCompilerCurrentServerAdmissionErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Descriptor(source) | Self::PeerCredentials(source) => Some(source),
            _ => None,
        }
    }
}

/// Ownership-retaining server admission failure.
pub struct ProtectedCompilerCurrentServerAdmissionFailureV1<A> {
    error: ProtectedCompilerCurrentServerAdmissionErrorV1,
    peer: OwnedFd,
    authority: A,
}

impl<A> ProtectedCompilerCurrentServerAdmissionFailureV1<A> {
    /// Returns the exact admission error.
    pub const fn error(&self) -> &ProtectedCompilerCurrentServerAdmissionErrorV1 {
        &self.error
    }

    /// Recovers the unchanged descriptor and authority after admission rejection.
    pub fn into_parts(self) -> (ProtectedCompilerCurrentServerAdmissionErrorV1, OwnedFd, A) {
        (self.error, self.peer, self.authority)
    }
}

impl<A> fmt::Debug for ProtectedCompilerCurrentServerAdmissionFailureV1<A> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProtectedCompilerCurrentServerAdmissionFailureV1")
            .field("error", &self.error)
            .field("retains_peer", &true)
            .field("retains_authority", &true)
            .finish()
    }
}

impl<A> fmt::Display for ProtectedCompilerCurrentServerAdmissionFailureV1<A> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<A: 'static> Error for ProtectedCompilerCurrentServerAdmissionFailureV1<A> {}

/// Endpoint custody after a bounded provider operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedCompilerCurrentServerCustodyV1 {
    /// No request bytes were consumed and the server still owns the endpoint.
    Retained,
    /// The server permanently closed an ambiguous or desynchronized endpoint.
    Poisoned,
}

/// Bounded server transport or protocol failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum ProtectedCompilerCurrentServerErrorV1 {
    /// The endpoint was already poisoned.
    Poisoned,
    /// The sole absolute deadline expired.
    DeadlineExpired,
    /// Endpoint identity or credentials changed.
    EndpointRevalidation(ProtectedCompilerCurrentServerAdmissionErrorV1),
    /// Request receive failed.
    Receive(io::Error),
    /// The peer closed without a request.
    PeerClosed,
    /// Ancillary descriptor or credential data was attached.
    AncillaryData,
    /// The seqpacket was larger than the fixed request buffer.
    PacketTruncated,
    /// The packet did not have the exact request length.
    RequestLength {
        /// Required request length.
        expected: usize,
        /// Observed request length.
        actual: usize,
    },
    /// The fixed request was not canonical or internally associated.
    Protocol(ProtectedCompilerCurrentProtocolErrorV1),
    /// A request did not match the admitted protocol, measurement, policy, or session.
    RequestCoordinateMismatch,
    /// The request was not the next exact session sequence.
    RequestSequenceMismatch,
    /// The authority identity changed after admission.
    AuthoritySubstitution,
    /// Response send failed.
    Send(io::Error),
    /// The atomic seqpacket response send was partial.
    PartialSend,
}

impl fmt::Display for ProtectedCompilerCurrentServerErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "protected compiler-current server failed: {self:?}"
        )
    }
}

impl Error for ProtectedCompilerCurrentServerErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EndpointRevalidation(source) => Some(source),
            Self::Receive(source) | Self::Send(source) => Some(source),
            Self::Protocol(source) => Some(source),
            _ => None,
        }
    }
}

/// Typed server failure preserving endpoint custody.
#[derive(Debug)]
pub struct ProtectedCompilerCurrentServerFailureV1 {
    error: ProtectedCompilerCurrentServerErrorV1,
    custody: ProtectedCompilerCurrentServerCustodyV1,
}

impl ProtectedCompilerCurrentServerFailureV1 {
    /// Returns the exact failure.
    pub const fn error(&self) -> &ProtectedCompilerCurrentServerErrorV1 {
        &self.error
    }

    /// Returns whether the endpoint remains synchronized and retained.
    pub const fn custody(&self) -> ProtectedCompilerCurrentServerCustodyV1 {
        self.custody
    }
}

impl fmt::Display for ProtectedCompilerCurrentServerFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl Error for ProtectedCompilerCurrentServerFailureV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

/// Completed disposition of one exactly correlated provider exchange.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedCompilerCurrentServerOutcomeV1 {
    /// The measured authority authenticated the exact request.
    Authenticated {
        /// Exact request identity.
        request_identity: [u8; 32],
        /// Exact canonical response identity.
        response_identity: [u8; 32],
        /// Request-bound provider transcript returned to the client.
        transcript_identity: [u8; 32],
    },
    /// The measured authority explicitly rejected the exact request.
    Rejected {
        /// Exact request identity.
        request_identity: [u8; 32],
        /// Exact canonical response identity.
        response_identity: [u8; 32],
    },
}

/// One descriptor-only protected compiler-current provider endpoint.
pub struct PreopenedProtectedCompilerCurrentServerV1<A> {
    peer: Option<OwnedFd>,
    endpoint: ProtectedCompilerCurrentEndpointIdentityV1,
    expected_peer: ProtectedCompilerCurrentPeerIdentityV1,
    protocol_identity: [u8; 32],
    provider_measurement: [u8; 32],
    compiler_policy_identity: [u8; 32],
    admission_session_identity: [u8; 32],
    next_request_sequence: u64,
    authority: A,
}

impl<A> fmt::Debug for PreopenedProtectedCompilerCurrentServerV1<A> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreopenedProtectedCompilerCurrentServerV1")
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
            .field("next_request_sequence", &self.next_request_sequence)
            .field("authority", &"owned measured authority")
            .finish_non_exhaustive()
    }
}

impl<A: ProtectedCompilerCurrentAuthorityV1> PreopenedProtectedCompilerCurrentServerV1<A> {
    /// Admits one exact supervisor-preopened verifier connection and measured authority.
    ///
    /// The descriptor must be a fresh, empty, connected, nonblocking,
    /// close-on-exec Unix `SOCK_SEQPACKET`. This function performs no discovery,
    /// connection, key loading, or authority construction.
    ///
    /// # Safety
    ///
    /// The supervisor must establish that `authority` is the independently
    /// protected measured implementation named by `provider_measurement`, is
    /// provisioned exclusively for `compiler_policy_identity`, and satisfies the
    /// trait's durable policy/ledger/rollback/session obligations. The supervisor
    /// must allocate `admission_session_identity` freshly and never reuse it for
    /// this provider, including across verifier, daemon, and supervisor restarts.
    /// It must transfer exclusive descriptor custody with no duplicate or
    /// prequeued packet and provision a verifier UID distinct from the provider.
    ///
    /// # Errors
    ///
    /// Returns the unchanged descriptor and authority on any admission failure.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn admit_from_supervisor(
        peer: OwnedFd,
        endpoint: ProtectedCompilerCurrentEndpointIdentityV1,
        expected_peer: ProtectedCompilerCurrentPeerIdentityV1,
        protocol_identity: [u8; 32],
        provider_measurement: [u8; 32],
        compiler_policy_identity: [u8; 32],
        admission_session_identity: [u8; 32],
        authority: A,
    ) -> Result<Self, ProtectedCompilerCurrentServerAdmissionFailureV1<A>> {
        Self::admit_inner::<true>(
            peer,
            endpoint,
            expected_peer,
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
            authority,
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
        authority: A,
    ) -> Result<Self, ProtectedCompilerCurrentServerAdmissionFailureV1<A>> {
        let admission = validate_admission_coordinates(
            protocol_identity,
            provider_measurement,
            compiler_policy_identity,
            admission_session_identity,
        )
        .and_then(|()| validate_endpoint::<REQUIRE_DISTINCT_UID>(&peer, endpoint, expected_peer))
        .and_then(|()| require_empty_connection(&peer))
        .and_then(|()| {
            if authority.provider_measurement() != provider_measurement {
                Err(ProtectedCompilerCurrentServerAdmissionErrorV1::AuthorityMeasurementMismatch)
            } else if authority.compiler_policy_identity() != compiler_policy_identity {
                Err(ProtectedCompilerCurrentServerAdmissionErrorV1::AuthorityPolicyMismatch)
            } else {
                Ok(())
            }
        });
        if let Err(error) = admission {
            return Err(ProtectedCompilerCurrentServerAdmissionFailureV1 {
                error,
                peer,
                authority,
            });
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
            authority,
        })
    }

    /// Returns whether a terminal or ambiguous failure permanently closed the endpoint.
    pub const fn is_poisoned(&self) -> bool {
        self.peer.is_none()
    }

    /// Returns the provider measurement bound into every request and response.
    pub const fn provider_measurement(&self) -> [u8; 32] {
        self.provider_measurement
    }

    /// Descriptor admission and wire framing grant no currentness authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }

    /// Receives, authenticates, and answers one fixed request before `deadline`.
    ///
    /// # Errors
    ///
    /// Returns typed retained/poisoned endpoint custody. A valid authority
    /// rejection is a successful [`ProtectedCompilerCurrentServerOutcomeV1::Rejected`]
    /// exchange, not a transport failure.
    pub fn serve_one_until(
        &mut self,
        deadline: Instant,
    ) -> Result<ProtectedCompilerCurrentServerOutcomeV1, ProtectedCompilerCurrentServerFailureV1>
    {
        if remaining(deadline).is_none() {
            return Err(
                self.pre_receive_failure(ProtectedCompilerCurrentServerErrorV1::DeadlineExpired)
            );
        }
        let Some(peer) = self.peer.take() else {
            return Err(self.poisoned_failure(ProtectedCompilerCurrentServerErrorV1::Poisoned));
        };
        if let Err(error) = validate_endpoint::<true>(&peer, self.endpoint, self.expected_peer) {
            return Err(ProtectedCompilerCurrentServerFailureV1 {
                error: ProtectedCompilerCurrentServerErrorV1::EndpointRevalidation(error),
                custody: ProtectedCompilerCurrentServerCustodyV1::Poisoned,
            });
        }
        let packet = match receive_request_packet(&peer, deadline) {
            Ok(packet) => packet,
            Err(failure) => {
                if !failure.poison {
                    self.peer = Some(peer);
                }
                return Err(ProtectedCompilerCurrentServerFailureV1 {
                    error: failure.error,
                    custody: if failure.poison {
                        ProtectedCompilerCurrentServerCustodyV1::Poisoned
                    } else {
                        ProtectedCompilerCurrentServerCustodyV1::Retained
                    },
                });
            }
        };
        if let Err(error) = validate_endpoint::<true>(&peer, self.endpoint, self.expected_peer) {
            return Err(ProtectedCompilerCurrentServerFailureV1 {
                error: ProtectedCompilerCurrentServerErrorV1::EndpointRevalidation(error),
                custody: ProtectedCompilerCurrentServerCustodyV1::Poisoned,
            });
        }
        if remaining(deadline).is_none() {
            return Err(ProtectedCompilerCurrentServerFailureV1 {
                error: ProtectedCompilerCurrentServerErrorV1::DeadlineExpired,
                custody: ProtectedCompilerCurrentServerCustodyV1::Poisoned,
            });
        }
        let request = ProtectedCompilerCurrentRequestV1::decode_canonical(packet.as_slice())
            .map_err(|source| ProtectedCompilerCurrentServerFailureV1 {
                error: ProtectedCompilerCurrentServerErrorV1::Protocol(source),
                custody: ProtectedCompilerCurrentServerCustodyV1::Poisoned,
            })?;
        if !self.matches_request_context(&request) {
            let response = ProtectedCompilerCurrentResponseV1::rejected(&request);
            let _ = send_response(&peer, &response, deadline);
            return Err(ProtectedCompilerCurrentServerFailureV1 {
                error: ProtectedCompilerCurrentServerErrorV1::RequestCoordinateMismatch,
                custody: ProtectedCompilerCurrentServerCustodyV1::Poisoned,
            });
        }
        if request.request_sequence() != self.next_request_sequence {
            let response = ProtectedCompilerCurrentResponseV1::rejected(&request);
            let _ = send_response(&peer, &response, deadline);
            return Err(ProtectedCompilerCurrentServerFailureV1 {
                error: ProtectedCompilerCurrentServerErrorV1::RequestSequenceMismatch,
                custody: ProtectedCompilerCurrentServerCustodyV1::Poisoned,
            });
        }
        self.next_request_sequence = self.next_request_sequence.checked_add(1).unwrap_or(0);
        let begin_request =
            WorkerV3VerificationRequestV1::decode_canonical(request.begin_request_bytes())
                .map_err(|_| {
                    self.protocol_failure(ProtectedCompilerCurrentProtocolErrorV1::BeginRequest)
                })?;
        let carriage = CompilerExecutionReceiptCarriageV1::decode(request.carriage_bytes())
            .map_err(|_| {
                self.protocol_failure(ProtectedCompilerCurrentProtocolErrorV1::Carriage)
            })?;
        let current_record = WorkerV3VerificationCurrentRecordFrameV2::decode_canonical(
            request.current_record_bytes(),
        )
        .map_err(|_| {
            self.protocol_failure(ProtectedCompilerCurrentProtocolErrorV1::CurrentRecord)
        })?;
        independently_validate_nested(&request, &begin_request, &carriage, &current_record)
            .map_err(|source| self.protocol_failure(source))?;
        if remaining(deadline).is_none() {
            return Err(
                self.poisoned_failure(ProtectedCompilerCurrentServerErrorV1::DeadlineExpired)
            );
        }
        if !self.authority_matches_pin() {
            let response = ProtectedCompilerCurrentResponseV1::rejected(&request);
            let _ = send_response(&peer, &response, deadline);
            return Err(
                self.poisoned_failure(ProtectedCompilerCurrentServerErrorV1::AuthoritySubstitution)
            );
        }
        let authority_result =
            self.authority
                .authenticate_current_record(ProtectedCompilerCurrentAuthorityInputV1 {
                    wire_request: &request,
                    begin_request: &begin_request,
                    carriage: &carriage,
                    current_record: &current_record,
                    deadline,
                });
        if remaining(deadline).is_none() {
            return Err(
                self.poisoned_failure(ProtectedCompilerCurrentServerErrorV1::DeadlineExpired)
            );
        }
        if !self.authority_matches_pin() {
            let response = ProtectedCompilerCurrentResponseV1::rejected(&request);
            let _ = send_response(&peer, &response, deadline);
            return Err(
                self.poisoned_failure(ProtectedCompilerCurrentServerErrorV1::AuthoritySubstitution)
            );
        }
        let (response, outcome) = match authority_result {
            Ok(authentication) => {
                let transcript_identity =
                    bind_provider_transcript(&request, authentication.transcript_identity());
                if transcript_identity == [0; 32] {
                    let response = ProtectedCompilerCurrentResponseV1::rejected(&request);
                    let outcome = ProtectedCompilerCurrentServerOutcomeV1::Rejected {
                        request_identity: request.request_identity(),
                        response_identity: response.response_identity(),
                    };
                    (response, outcome)
                } else {
                    let response = ProtectedCompilerCurrentResponseV1::authenticated(
                        &request,
                        transcript_identity,
                    )
                    .map_err(|source| self.protocol_failure(source))?;
                    let outcome = ProtectedCompilerCurrentServerOutcomeV1::Authenticated {
                        request_identity: request.request_identity(),
                        response_identity: response.response_identity(),
                        transcript_identity,
                    };
                    (response, outcome)
                }
            }
            Err(_) => {
                let response = ProtectedCompilerCurrentResponseV1::rejected(&request);
                let outcome = ProtectedCompilerCurrentServerOutcomeV1::Rejected {
                    request_identity: request.request_identity(),
                    response_identity: response.response_identity(),
                };
                (response, outcome)
            }
        };
        send_response(&peer, &response, deadline).map_err(|error| self.poisoned_failure(error))?;
        if remaining(deadline).is_none() {
            return Err(
                self.poisoned_failure(ProtectedCompilerCurrentServerErrorV1::DeadlineExpired)
            );
        }
        self.peer = Some(peer);
        Ok(outcome)
    }

    fn matches_request_context(&self, request: &ProtectedCompilerCurrentRequestV1) -> bool {
        request.protocol_identity() == self.protocol_identity
            && request.provider_measurement() == self.provider_measurement
            && request.compiler_policy_identity() == self.compiler_policy_identity
            && request.admission_session_identity() == self.admission_session_identity
    }

    fn authority_matches_pin(&self) -> bool {
        self.authority.provider_measurement() == self.provider_measurement
            && self.authority.compiler_policy_identity() == self.compiler_policy_identity
    }

    fn pre_receive_failure(
        &self,
        error: ProtectedCompilerCurrentServerErrorV1,
    ) -> ProtectedCompilerCurrentServerFailureV1 {
        ProtectedCompilerCurrentServerFailureV1 {
            error,
            custody: if self.peer.is_some() {
                ProtectedCompilerCurrentServerCustodyV1::Retained
            } else {
                ProtectedCompilerCurrentServerCustodyV1::Poisoned
            },
        }
    }

    fn protocol_failure(
        &mut self,
        source: ProtectedCompilerCurrentProtocolErrorV1,
    ) -> ProtectedCompilerCurrentServerFailureV1 {
        self.poisoned_failure(ProtectedCompilerCurrentServerErrorV1::Protocol(source))
    }

    fn poisoned_failure(
        &mut self,
        error: ProtectedCompilerCurrentServerErrorV1,
    ) -> ProtectedCompilerCurrentServerFailureV1 {
        self.peer = None;
        ProtectedCompilerCurrentServerFailureV1 {
            error,
            custody: ProtectedCompilerCurrentServerCustodyV1::Poisoned,
        }
    }
}

fn independently_validate_nested(
    request: &ProtectedCompilerCurrentRequestV1,
    begin: &WorkerV3VerificationRequestV1,
    carriage: &CompilerExecutionReceiptCarriageV1,
    current: &WorkerV3VerificationCurrentRecordFrameV2,
) -> Result<(), ProtectedCompilerCurrentProtocolErrorV1> {
    let envelope = &begin.payloads()[0];
    let verification = current.verification();
    if request.begin_request_identity() != *begin.identity().as_bytes()
        || request.begin_request_sha256() != sha256(begin.encode_canonical())
        || request.envelope_length() != envelope.byte_len()
        || request.envelope_sha256() != *envelope.sha256()
        || request.carriage_identity() != *carriage.identity().as_bytes()
        || request.current_record_sha256() != sha256(current.encode_canonical())
        || request.compiler_policy_identity() != *carriage.policy().identity().as_bytes()
        || verification.policy_identity() != *carriage.policy().identity().as_bytes()
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
) -> Result<(), ProtectedCompilerCurrentServerAdmissionErrorV1> {
    if protocol_identity == [0; 32] {
        return Err(ProtectedCompilerCurrentServerAdmissionErrorV1::ZeroProtocolIdentity);
    }
    if provider_measurement == [0; 32] {
        return Err(ProtectedCompilerCurrentServerAdmissionErrorV1::ZeroProviderMeasurement);
    }
    if compiler_policy_identity == [0; 32] {
        return Err(ProtectedCompilerCurrentServerAdmissionErrorV1::ZeroCompilerPolicyIdentity);
    }
    if admission_session_identity == [0; 32] {
        return Err(ProtectedCompilerCurrentServerAdmissionErrorV1::ZeroAdmissionSessionIdentity);
    }
    Ok(())
}

fn validate_endpoint<const REQUIRE_DISTINCT_UID: bool>(
    peer: &OwnedFd,
    endpoint: ProtectedCompilerCurrentEndpointIdentityV1,
    expected_peer: ProtectedCompilerCurrentPeerIdentityV1,
) -> Result<(), ProtectedCompilerCurrentServerAdmissionErrorV1> {
    let stat = rustix::fs::fstat(peer).map_err(|source| {
        ProtectedCompilerCurrentServerAdmissionErrorV1::Descriptor(source.into())
    })?;
    let descriptor_flags = rustix::io::fcntl_getfd(peer).map_err(|source| {
        ProtectedCompilerCurrentServerAdmissionErrorV1::Descriptor(source.into())
    })?;
    let status = rustix::fs::fcntl_getfl(peer).map_err(|source| {
        ProtectedCompilerCurrentServerAdmissionErrorV1::Descriptor(source.into())
    })?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::Socket
        || !descriptor_flags.contains(FdFlags::CLOEXEC)
        || !status.contains(OFlags::NONBLOCK)
        || status & OFlags::ACCMODE != OFlags::RDWR
        || status.intersects(OFlags::APPEND | OFlags::ASYNC | OFlags::DIRECT | OFlags::PATH)
    {
        return Err(ProtectedCompilerCurrentServerAdmissionErrorV1::DescriptorShape);
    }
    if stat.st_dev != endpoint.device() || stat.st_ino != endpoint.inode() {
        return Err(ProtectedCompilerCurrentServerAdmissionErrorV1::EndpointIdentityMismatch);
    }
    if socket_option(peer, libc::SO_DOMAIN)? != libc::AF_UNIX
        || socket_option(peer, libc::SO_TYPE)? != libc::SOCK_SEQPACKET
        || socket_option(peer, libc::SO_ACCEPTCONN)? != 0
        || !is_connected_unix(peer)?
    {
        return Err(ProtectedCompilerCurrentServerAdmissionErrorV1::SocketShape);
    }
    let pending = socket_option(peer, libc::SO_ERROR)?;
    if pending != 0 {
        return Err(
            ProtectedCompilerCurrentServerAdmissionErrorV1::PendingSocketError { raw: pending },
        );
    }
    let observed = peer_credentials(peer)?;
    if observed != expected_peer {
        return Err(ProtectedCompilerCurrentServerAdmissionErrorV1::PeerIdentityMismatch);
    }
    if REQUIRE_DISTINCT_UID && observed.uid() == rustix::process::geteuid().as_raw() {
        return Err(ProtectedCompilerCurrentServerAdmissionErrorV1::ProviderAndVerifierUidMatch);
    }
    Ok(())
}

fn socket_option(
    peer: &OwnedFd,
    option: i32,
) -> Result<i32, ProtectedCompilerCurrentServerAdmissionErrorV1> {
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
        return Err(ProtectedCompilerCurrentServerAdmissionErrorV1::Descriptor(
            io::Error::last_os_error(),
        ));
    }
    let actual = usize::try_from(length).expect("socklen_t fits usize");
    if actual != mem::size_of::<i32>() {
        return Err(
            ProtectedCompilerCurrentServerAdmissionErrorV1::SocketOptionLength {
                expected: mem::size_of::<i32>(),
                actual,
            },
        );
    }
    Ok(value)
}

fn is_connected_unix(
    peer: &OwnedFd,
) -> Result<bool, ProtectedCompilerCurrentServerAdmissionErrorV1> {
    // SAFETY: zero initializes all sockaddr storage.
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
        return Err(ProtectedCompilerCurrentServerAdmissionErrorV1::Descriptor(
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
) -> Result<ProtectedCompilerCurrentPeerIdentityV1, ProtectedCompilerCurrentServerAdmissionErrorV1>
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
            ProtectedCompilerCurrentServerAdmissionErrorV1::PeerCredentials(
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
        return Err(ProtectedCompilerCurrentServerAdmissionErrorV1::InvalidPeerCredentials);
    }
    ProtectedCompilerCurrentPeerIdentityV1::new(
        pid.expect("positive peer PID checked"),
        credentials.uid,
        credentials.gid,
    )
    .map_err(|_| ProtectedCompilerCurrentServerAdmissionErrorV1::InvalidPeerCredentials)
}

fn require_empty_connection(
    peer: &OwnedFd,
) -> Result<(), ProtectedCompilerCurrentServerAdmissionErrorV1> {
    let mut byte = 0_u8;
    // SAFETY: byte names one writable byte and the call only peeks at the owned socket.
    let received = unsafe {
        libc::recv(
            peer.as_raw_fd(),
            (&raw mut byte).cast(),
            1,
            libc::MSG_DONTWAIT | libc::MSG_PEEK,
        )
    };
    if received < 0 {
        let source = io::Error::last_os_error();
        if source.kind() == io::ErrorKind::WouldBlock {
            return Ok(());
        }
        return Err(ProtectedCompilerCurrentServerAdmissionErrorV1::Descriptor(
            source,
        ));
    }
    Err(ProtectedCompilerCurrentServerAdmissionErrorV1::PrequeuedPacket)
}

struct ReceivePacketFailureV1 {
    error: ProtectedCompilerCurrentServerErrorV1,
    poison: bool,
}

fn receive_request_packet(
    peer: &OwnedFd,
    deadline: Instant,
) -> Result<Box<[u8; PREOPENED_PROTECTED_COMPILER_CURRENT_REQUEST_BYTES_V1]>, ReceivePacketFailureV1>
{
    if let Err(error) = wait_for_peer(peer, libc::POLLIN, deadline) {
        return Err(ReceivePacketFailureV1 {
            poison: !matches!(
                error,
                ProtectedCompilerCurrentServerErrorV1::DeadlineExpired
            ),
            error,
        });
    }
    let mut bytes = boxed_zero_array();
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
        return Err(ReceivePacketFailureV1 {
            error: ProtectedCompilerCurrentServerErrorV1::Receive(io::Error::last_os_error()),
            poison: true,
        });
    }
    if header.msg_flags & libc::MSG_CTRUNC != 0 || header.msg_controllen != 0 {
        return Err(ReceivePacketFailureV1 {
            error: ProtectedCompilerCurrentServerErrorV1::AncillaryData,
            poison: true,
        });
    }
    if header.msg_flags & libc::MSG_TRUNC != 0 {
        return Err(ReceivePacketFailureV1 {
            error: ProtectedCompilerCurrentServerErrorV1::PacketTruncated,
            poison: true,
        });
    }
    let received = usize::try_from(received).map_err(|_| ReceivePacketFailureV1 {
        error: ProtectedCompilerCurrentServerErrorV1::PacketTruncated,
        poison: true,
    })?;
    if received == 0 {
        return Err(ReceivePacketFailureV1 {
            error: ProtectedCompilerCurrentServerErrorV1::PeerClosed,
            poison: true,
        });
    }
    if received != bytes.len() {
        return Err(ReceivePacketFailureV1 {
            error: ProtectedCompilerCurrentServerErrorV1::RequestLength {
                expected: bytes.len(),
                actual: received,
            },
            poison: true,
        });
    }
    Ok(bytes)
}

fn send_response(
    peer: &OwnedFd,
    response: &ProtectedCompilerCurrentResponseV1,
    deadline: Instant,
) -> Result<(), ProtectedCompilerCurrentServerErrorV1> {
    wait_for_peer(peer, libc::POLLOUT, deadline)?;
    if remaining(deadline).is_none() {
        return Err(ProtectedCompilerCurrentServerErrorV1::DeadlineExpired);
    }
    let bytes = response.encode_canonical();
    // SAFETY: bytes names the complete readable response and peer remains uniquely owned.
    let sent = unsafe {
        libc::send(
            peer.as_raw_fd(),
            bytes.as_ptr().cast(),
            bytes.len(),
            libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
        )
    };
    if sent < 0 {
        return Err(ProtectedCompilerCurrentServerErrorV1::Send(
            io::Error::last_os_error(),
        ));
    }
    if usize::try_from(sent).ok() != Some(bytes.len()) {
        return Err(ProtectedCompilerCurrentServerErrorV1::PartialSend);
    }
    Ok(())
}

fn wait_for_peer(
    peer: &OwnedFd,
    wanted: i16,
    deadline: Instant,
) -> Result<(), ProtectedCompilerCurrentServerErrorV1> {
    loop {
        let duration =
            remaining(deadline).ok_or(ProtectedCompilerCurrentServerErrorV1::DeadlineExpired)?;
        let mut descriptor = libc::pollfd {
            fd: peer.as_raw_fd(),
            events: wanted | libc::POLLERR | libc::POLLHUP,
            revents: 0,
        };
        // SAFETY: descriptor is a live one-element pollfd array for the complete call.
        let result =
            unsafe { libc::poll(&raw mut descriptor, 1, duration_to_poll_millis(duration)) };
        if result < 0 {
            let source = io::Error::last_os_error();
            if source.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(ProtectedCompilerCurrentServerErrorV1::Receive(source));
        }
        if result == 0 {
            continue;
        }
        if remaining(deadline).is_none() {
            return Err(ProtectedCompilerCurrentServerErrorV1::DeadlineExpired);
        }
        if descriptor.revents & libc::POLLNVAL != 0 {
            return Err(ProtectedCompilerCurrentServerErrorV1::Receive(
                io::Error::from_raw_os_error(libc::EBADF),
            ));
        }
        if descriptor.revents & wanted != 0 {
            return Ok(());
        }
        if descriptor.revents & libc::POLLERR != 0 {
            return Err(ProtectedCompilerCurrentServerErrorV1::Receive(
                io::Error::from_raw_os_error(libc::ECONNRESET),
            ));
        }
        if descriptor.revents & libc::POLLHUP != 0 {
            return Err(ProtectedCompilerCurrentServerErrorV1::PeerClosed);
        }
    }
}

fn remaining(deadline: Instant) -> Option<Duration> {
    let duration = deadline.saturating_duration_since(Instant::now());
    (!duration.is_zero()).then_some(duration)
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

fn bind_provider_transcript(
    request: &ProtectedCompilerCurrentRequestV1,
    authority_transcript_identity: [u8; 32],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(PROVIDER_TRANSCRIPT_DOMAIN_V1);
    hasher.update(request.request_identity());
    hasher.update(request.protocol_identity());
    hasher.update(request.provider_measurement());
    hasher.update(request.compiler_policy_identity());
    hasher.update(request.admission_session_identity());
    hasher.update(request.request_sequence().to_le_bytes());
    hasher.update(authority_transcript_identity);
    hasher.finalize().into()
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn boxed_zero_array<const N: usize>() -> Box<[u8; N]> {
    vec![0_u8; N]
        .into_boxed_slice()
        .try_into()
        .unwrap_or_else(|_| unreachable!("fixed zero buffer has its declared length"))
}
