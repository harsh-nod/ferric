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
            .finish_non_exhaustive()
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
        self.serve_one_inner::<true>(deadline)
    }

    fn serve_one_inner<const REQUIRE_DISTINCT_UID: bool>(
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
        if let Err(error) =
            validate_endpoint::<REQUIRE_DISTINCT_UID>(&peer, self.endpoint, self.expected_peer)
        {
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
        if let Err(error) =
            validate_endpoint::<REQUIRE_DISTINCT_UID>(&peer, self.endpoint, self.expected_peer)
        {
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
        let (response, outcome) = if let Ok(authentication) = authority_result {
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
        } else {
            let response = ProtectedCompilerCurrentResponseV1::rejected(&request);
            let outcome = ProtectedCompilerCurrentServerOutcomeV1::Rejected {
                request_identity: request.request_identity(),
                response_identity: response.response_identity(),
            };
            (response, outcome)
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

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    use ed25519_dalek::{Signer, SigningKey};
    use fe2o3_external_anchor_protocol::{
        AnchorPositionV1, AnchorTransitionReceiptV1, AnchoredStateV1, CallerNonceV1,
        HashChainHeadV1, PinnedAnchorKeyV1, UnsignedAnchorObservationV1,
    };
    use fe2o3_runtime_protocol::{
        CompilerExecutionCurrentRecordAttestationV3, CompilerExecutionCurrentRecordVerificationV3,
        CompilerExecutionExternalAnchorTransactionV1, WorkerV3LoadEnvelopeWireV2,
    };
    use fe2o3_worker_v3_verification_protocol::{
        WorkerV3VerificationChallengeReservationV2, WorkerV3VerificationEntryCoordinateV1,
        WorkerV3VerificationFdPayloadDescriptorV1, WorkerV3VerificationFreshChallengeV1,
        WorkerV3VerificationMeasurementIdentityV1, WorkerV3VerificationPolicyIdentityV1,
        WorkerV3VerificationRosterIdentityV1,
    };
    use rustix::net::{AddressFamily, SocketFlags, SocketType, socketpair};

    use super::*;

    const ENVELOPE: &[u8] = include_bytes!("../tests/fixtures/valid-envelope-v2.bin");
    const PROTOCOL: [u8; 32] = [0xa1; 32];
    const PROVIDER: [u8; 32] = [0xa2; 32];
    const SESSION: [u8; 32] = [0xa3; 32];
    const AUTHORITY_TRANSCRIPT: [u8; 32] = [0xa4; 32];

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

        fn request(
            &self,
            protocol: [u8; 32],
            provider: [u8; 32],
            session: [u8; 32],
            sequence: u64,
        ) -> ProtectedCompilerCurrentRequestV1 {
            ProtectedCompilerCurrentRequestV1::new(
                protocol,
                provider,
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

    #[derive(Clone, Copy)]
    enum AuthorityMode {
        Accept,
        RejectFirst,
        SubstituteAfterCall,
        Delay(Duration),
    }

    #[derive(Debug)]
    struct MockAuthorityError;

    impl fmt::Display for MockAuthorityError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("mock authority rejection")
        }
    }

    impl Error for MockAuthorityError {}

    struct MockAuthority {
        measurement: Cell<[u8; 32]>,
        policy: [u8; 32],
        calls: Arc<AtomicUsize>,
        mode: AuthorityMode,
    }

    // SAFETY: this implementation exists only under cfg(test); acceptance asserts every
    // structural input exposed by the production boundary before minting a fixture transcript.
    unsafe impl ProtectedCompilerCurrentAuthorityV1 for MockAuthority {
        type Error = MockAuthorityError;

        fn provider_measurement(&self) -> [u8; 32] {
            self.measurement.get()
        }

        fn compiler_policy_identity(&self) -> [u8; 32] {
            self.policy
        }

        fn authenticate_current_record(
            &mut self,
            input: ProtectedCompilerCurrentAuthorityInputV1<'_>,
        ) -> Result<ProtectedCompilerCurrentAuthorityAuthenticationV1, Self::Error> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(input.wire_request().provider_measurement(), PROVIDER);
            assert_eq!(input.wire_request().compiler_policy_identity(), self.policy);
            assert_eq!(
                input.wire_request().begin_request_identity(),
                *input.begin_request().identity().as_bytes()
            );
            assert_eq!(
                input.wire_request().carriage_identity(),
                *input.carriage().identity().as_bytes()
            );
            assert_eq!(
                input.current_record().verification().carriage_identity(),
                *input.carriage().identity().as_bytes()
            );
            assert!(input.deadline() > Instant::now());
            match self.mode {
                AuthorityMode::RejectFirst if call == 0 => return Err(MockAuthorityError),
                AuthorityMode::SubstituteAfterCall => self.measurement.set([0xee; 32]),
                AuthorityMode::Delay(duration) => thread::sleep(duration),
                _ => {}
            }
            // SAFETY: the cfg(test) assertions above define this fixture authority.
            unsafe {
                ProtectedCompilerCurrentAuthorityAuthenticationV1::from_protected_authority(
                    AUTHORITY_TRANSCRIPT,
                )
            }
            .map_err(|_| MockAuthorityError)
        }
    }

    fn authority(policy: [u8; 32], mode: AuthorityMode) -> MockAuthority {
        MockAuthority {
            measurement: Cell::new(PROVIDER),
            policy,
            calls: Arc::new(AtomicUsize::new(0)),
            mode,
        }
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

    fn admit_test_server(
        peer: OwnedFd,
        policy: [u8; 32],
        authority: MockAuthority,
    ) -> PreopenedProtectedCompilerCurrentServerV1<MockAuthority> {
        let identity = endpoint(&peer);
        PreopenedProtectedCompilerCurrentServerV1::admit_inner::<false>(
            peer,
            identity,
            expected_peer(),
            PROTOCOL,
            PROVIDER,
            policy,
            SESSION,
            authority,
        )
        .unwrap()
    }

    fn deadline(duration: Duration) -> Instant {
        Instant::now().checked_add(duration).unwrap()
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

    fn receive_response(peer: &OwnedFd) -> ProtectedCompilerCurrentResponseV1 {
        wait_for_peer(peer, libc::POLLIN, deadline(Duration::from_secs(2))).unwrap();
        let mut bytes = [0_u8; crate::PREOPENED_PROTECTED_COMPILER_CURRENT_RESPONSE_BYTES_V1];
        // SAFETY: bytes is writable for the exact fixed response.
        let received = unsafe {
            libc::recv(
                peer.as_raw_fd(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
                libc::MSG_DONTWAIT,
            )
        };
        assert_eq!(usize::try_from(received).unwrap(), bytes.len());
        ProtectedCompilerCurrentResponseV1::decode_canonical(&bytes).unwrap()
    }

    fn assert_no_response(peer: &OwnedFd) {
        let mut byte = 0_u8;
        // SAFETY: byte is writable for one nonblocking receive.
        let received = unsafe {
            libc::recv(
                peer.as_raw_fd(),
                (&raw mut byte).cast(),
                1,
                libc::MSG_DONTWAIT,
            )
        };
        if received < 0 {
            assert_eq!(io::Error::last_os_error().kind(), io::ErrorKind::WouldBlock);
        } else {
            assert_eq!(received, 0, "a poisoned server emitted response bytes");
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
    fn authenticated_exchange_is_exactly_correlated_and_retained() {
        let fixture = Fixture::new();
        let (server_peer, client_peer) = pair(SocketType::SEQPACKET);
        let calls = Arc::new(AtomicUsize::new(0));
        let authority = MockAuthority {
            measurement: Cell::new(PROVIDER),
            policy: fixture.policy,
            calls: Arc::clone(&calls),
            mode: AuthorityMode::Accept,
        };
        let mut server = admit_test_server(server_peer, fixture.policy, authority);
        let request = fixture.request(PROTOCOL, PROVIDER, SESSION, 1);
        send_bytes(&client_peer, request.encode_canonical());
        let outcome = server
            .serve_one_inner::<false>(deadline(Duration::from_secs(2)))
            .unwrap();
        let response = receive_response(&client_peer);
        assert!(response.matches_request(&request));
        assert!(matches!(
            outcome,
            ProtectedCompilerCurrentServerOutcomeV1::Authenticated {
                request_identity,
                response_identity,
                transcript_identity,
            } if request_identity == request.request_identity()
                && response_identity == response.response_identity()
                && transcript_identity == response.transcript_identity()
                && transcript_identity != AUTHORITY_TRANSCRIPT
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(!server.is_poisoned());
    }

    #[test]
    fn authority_rejection_is_correlated_retained_and_advances_sequence() {
        let fixture = Fixture::new();
        let (server_peer, client_peer) = pair(SocketType::SEQPACKET);
        let mut server = admit_test_server(
            server_peer,
            fixture.policy,
            authority(fixture.policy, AuthorityMode::RejectFirst),
        );
        let first = fixture.request(PROTOCOL, PROVIDER, SESSION, 1);
        send_bytes(&client_peer, first.encode_canonical());
        let outcome = server
            .serve_one_inner::<false>(deadline(Duration::from_secs(2)))
            .unwrap();
        let response = receive_response(&client_peer);
        assert!(matches!(
            outcome,
            ProtectedCompilerCurrentServerOutcomeV1::Rejected { .. }
        ));
        assert!(response.matches_request(&first));
        assert_eq!(response.transcript_identity(), [0; 32]);
        assert!(!server.is_poisoned());

        let second = fixture.request(PROTOCOL, PROVIDER, SESSION, 2);
        send_bytes(&client_peer, second.encode_canonical());
        assert!(matches!(
            server
                .serve_one_inner::<false>(deadline(Duration::from_secs(2)))
                .unwrap(),
            ProtectedCompilerCurrentServerOutcomeV1::Authenticated { .. }
        ));
        assert!(receive_response(&client_peer).matches_request(&second));
    }

    #[test]
    fn substituted_context_is_correlated_rejected_and_poisoned() {
        let fixture = Fixture::new();
        for (protocol, provider, session) in [
            ([0xe1; 32], PROVIDER, SESSION),
            (PROTOCOL, [0xe2; 32], SESSION),
            (PROTOCOL, PROVIDER, [0xe3; 32]),
        ] {
            let (server_peer, client_peer) = pair(SocketType::SEQPACKET);
            let mut server = admit_test_server(
                server_peer,
                fixture.policy,
                authority(fixture.policy, AuthorityMode::Accept),
            );
            let request = fixture.request(protocol, provider, session, 1);
            send_bytes(&client_peer, request.encode_canonical());
            let failure = server
                .serve_one_inner::<false>(deadline(Duration::from_secs(2)))
                .unwrap_err();
            let response = receive_response(&client_peer);
            assert!(response.matches_request(&request));
            assert_eq!(response.transcript_identity(), [0; 32]);
            assert!(matches!(
                failure.error(),
                ProtectedCompilerCurrentServerErrorV1::RequestCoordinateMismatch
            ));
            assert_eq!(
                failure.custody(),
                ProtectedCompilerCurrentServerCustodyV1::Poisoned
            );
            assert!(server.is_poisoned());
        }
    }

    #[test]
    fn replayed_or_skipped_sequence_is_rejected_and_poisoned() {
        let fixture = Fixture::new();
        for sequence in [2, u64::MAX] {
            let (server_peer, client_peer) = pair(SocketType::SEQPACKET);
            let mut server = admit_test_server(
                server_peer,
                fixture.policy,
                authority(fixture.policy, AuthorityMode::Accept),
            );
            let request = fixture.request(PROTOCOL, PROVIDER, SESSION, sequence);
            send_bytes(&client_peer, request.encode_canonical());
            let failure = server
                .serve_one_inner::<false>(deadline(Duration::from_secs(2)))
                .unwrap_err();
            assert!(receive_response(&client_peer).matches_request(&request));
            assert!(matches!(
                failure.error(),
                ProtectedCompilerCurrentServerErrorV1::RequestSequenceMismatch
            ));
            assert!(server.is_poisoned());
        }
    }

    #[test]
    fn malformed_short_and_ancillary_requests_poison_without_authority() {
        let fixture = Fixture::new();
        let canonical = fixture.request(PROTOCOL, PROVIDER, SESSION, 1);
        for case in 0..3 {
            let (server_peer, client_peer) = pair(SocketType::SEQPACKET);
            let calls = Arc::new(AtomicUsize::new(0));
            let authority = MockAuthority {
                measurement: Cell::new(PROVIDER),
                policy: fixture.policy,
                calls: Arc::clone(&calls),
                mode: AuthorityMode::Accept,
            };
            let mut server = admit_test_server(server_peer, fixture.policy, authority);
            match case {
                0 => {
                    let mut bytes = canonical.encode_canonical().to_vec();
                    bytes[24] ^= 1;
                    send_bytes(&client_peer, &bytes);
                }
                1 => send_bytes(&client_peer, &[0x55; 32]),
                2 => send_ancillary(&client_peer, canonical.encode_canonical()),
                _ => unreachable!(),
            }
            let failure = server
                .serve_one_inner::<false>(deadline(Duration::from_secs(2)))
                .unwrap_err();
            assert_eq!(
                failure.custody(),
                ProtectedCompilerCurrentServerCustodyV1::Poisoned
            );
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert!(server.is_poisoned());
            assert_no_response(&client_peer);
        }
    }

    #[test]
    fn expired_before_receive_retains_the_fresh_endpoint() {
        let fixture = Fixture::new();
        let (server_peer, client_peer) = pair(SocketType::SEQPACKET);
        let mut server = admit_test_server(
            server_peer,
            fixture.policy,
            authority(fixture.policy, AuthorityMode::Accept),
        );
        let failure = server.serve_one_inner::<false>(Instant::now()).unwrap_err();
        assert_eq!(
            failure.custody(),
            ProtectedCompilerCurrentServerCustodyV1::Retained
        );
        assert!(!server.is_poisoned());
        let request = fixture.request(PROTOCOL, PROVIDER, SESSION, 1);
        send_bytes(&client_peer, request.encode_canonical());
        server
            .serve_one_inner::<false>(deadline(Duration::from_secs(2)))
            .unwrap();
        assert!(receive_response(&client_peer).matches_request(&request));
    }

    #[test]
    fn authority_overrun_never_emits_a_response() {
        let fixture = Fixture::new();
        let (server_peer, client_peer) = pair(SocketType::SEQPACKET);
        let mut server = admit_test_server(
            server_peer,
            fixture.policy,
            authority(
                fixture.policy,
                AuthorityMode::Delay(Duration::from_millis(20)),
            ),
        );
        let request = fixture.request(PROTOCOL, PROVIDER, SESSION, 1);
        send_bytes(&client_peer, request.encode_canonical());
        let failure = server
            .serve_one_inner::<false>(deadline(Duration::from_millis(1)))
            .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedCompilerCurrentServerErrorV1::DeadlineExpired
        ));
        assert!(server.is_poisoned());
        assert_no_response(&client_peer);
    }

    #[test]
    fn post_call_authority_substitution_rejects_and_poisons() {
        let fixture = Fixture::new();
        let (server_peer, client_peer) = pair(SocketType::SEQPACKET);
        let mut server = admit_test_server(
            server_peer,
            fixture.policy,
            authority(fixture.policy, AuthorityMode::SubstituteAfterCall),
        );
        let request = fixture.request(PROTOCOL, PROVIDER, SESSION, 1);
        send_bytes(&client_peer, request.encode_canonical());
        let failure = server
            .serve_one_inner::<false>(deadline(Duration::from_secs(2)))
            .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedCompilerCurrentServerErrorV1::AuthoritySubstitution
        ));
        let response = receive_response(&client_peer);
        assert!(response.matches_request(&request));
        assert_eq!(response.transcript_identity(), [0; 32]);
        assert!(server.is_poisoned());
    }

    #[test]
    fn admission_rejects_prequeued_stream_same_uid_and_authority_substitution() {
        let fixture = Fixture::new();
        let request = fixture.request(PROTOCOL, PROVIDER, SESSION, 1);
        let (server_peer, client_peer) = pair(SocketType::SEQPACKET);
        send_bytes(&client_peer, request.encode_canonical());
        let identity = endpoint(&server_peer);
        let failure = PreopenedProtectedCompilerCurrentServerV1::admit_inner::<false>(
            server_peer,
            identity,
            expected_peer(),
            PROTOCOL,
            PROVIDER,
            fixture.policy,
            SESSION,
            authority(fixture.policy, AuthorityMode::Accept),
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedCompilerCurrentServerAdmissionErrorV1::PrequeuedPacket
        ));
        let (_, _peer, _authority) = failure.into_parts();

        let (stream, _peer) = pair(SocketType::STREAM);
        let identity = endpoint(&stream);
        let failure = PreopenedProtectedCompilerCurrentServerV1::admit_inner::<false>(
            stream,
            identity,
            expected_peer(),
            PROTOCOL,
            PROVIDER,
            fixture.policy,
            SESSION,
            authority(fixture.policy, AuthorityMode::Accept),
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedCompilerCurrentServerAdmissionErrorV1::SocketShape
        ));

        let (server_peer, _client_peer) = pair(SocketType::SEQPACKET);
        let identity = endpoint(&server_peer);
        // SAFETY: this is the negative test for the production distinct-UID check.
        let failure = unsafe {
            PreopenedProtectedCompilerCurrentServerV1::admit_from_supervisor(
                server_peer,
                identity,
                expected_peer(),
                PROTOCOL,
                PROVIDER,
                fixture.policy,
                SESSION,
                authority(fixture.policy, AuthorityMode::Accept),
            )
        }
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedCompilerCurrentServerAdmissionErrorV1::ProviderAndVerifierUidMatch
        ));

        let (server_peer, _client_peer) = pair(SocketType::SEQPACKET);
        let identity = endpoint(&server_peer);
        let wrong_authority = authority(fixture.policy, AuthorityMode::Accept);
        wrong_authority.measurement.set([0xef; 32]);
        let failure = PreopenedProtectedCompilerCurrentServerV1::admit_inner::<false>(
            server_peer,
            identity,
            expected_peer(),
            PROTOCOL,
            PROVIDER,
            fixture.policy,
            SESSION,
            wrong_authority,
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            ProtectedCompilerCurrentServerAdmissionErrorV1::AuthorityMeasurementMismatch
        ));
    }

    #[test]
    fn transcript_constructor_rejects_zero_and_poll_conversion_is_bounded() {
        // SAFETY: zero is intentionally supplied to exercise the claim guard.
        assert!(
            unsafe {
                ProtectedCompilerCurrentAuthorityAuthenticationV1::from_protected_authority([0; 32])
            }
            .is_err()
        );
        assert_eq!(duration_to_poll_millis(Duration::ZERO), 1);
        assert_eq!(duration_to_poll_millis(Duration::from_nanos(1)), 1);
        assert_eq!(duration_to_poll_millis(Duration::MAX), i32::MAX);
    }
}
