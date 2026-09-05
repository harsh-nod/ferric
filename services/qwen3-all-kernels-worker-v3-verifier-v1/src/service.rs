#![allow(
    clippy::must_use_candidate,
    reason = "public values are inert inputs, identities, or diagnostic accessors"
)]

use std::error::Error;
use std::fmt;
use std::os::fd::{AsFd, OwnedFd};
use std::time::{Duration, Instant};

use fe2o3_runtime_protocol::WorkerV3LoadEnvelopeWireV2;
use fe2o3_worker_v3_verification_protocol::{
    WorkerV3VerificationCurrentRecordFrameV2, WorkerV3VerificationFdPayloadKindV1,
    WorkerV3VerificationMeasurementIdentityV1, WorkerV3VerificationPolicyIdentityV1,
    WorkerV3VerificationRequestV1,
};
use fe2o3_worker_v3_verification_service::{
    CompletedWorkerV3VerificationSessionV2, PendingRejectedWorkerV3VerificationTerminalSessionV2,
    PendingWorkerV3VerificationTerminalSessionV2, RejectedWorkerV3VerificationBeginV2,
    RetainedWorkerV3VerificationPayloadV1, WorkerV3VerificationBeginOutcomeV2,
    WorkerV3VerificationCallerV1, WorkerV3VerificationCurrentRecordOutcomeV2,
    WorkerV3VerificationMeasurementResolverV1, WorkerV3VerificationPolicyResolverV1,
    WorkerV3VerificationRejectedSendFailureV2, WorkerV3VerificationServiceErrorV2,
    WorkerV3VerificationTerminalSendFailureV2, begin_worker_v3_verification_session_until_v2,
};
use ferric_qwen3_all_kernels_worker_v3_verifier_v1::M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1;
use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_receipt::{
    M1AllKernelsProtectedReceiptCompilerClaimsV1, M1AllKernelsProtectedReceiptEntryV1,
    M1AllKernelsProtectedReceiptRequestClaimsV1, M1AllKernelsProtectedVerifierTrustPolicyV1,
    M1AllKernelsUnsignedProtectedVerifierReceiptV1,
};
use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_verifier_service::{
    M1AllKernelsProtectedVerifierServiceEntryV1,
    M1AllKernelsProtectedVerifierServiceProtocolErrorV1,
    M1AllKernelsProtectedVerifierServiceRequestV1, M1AllKernelsProtectedVerifierServiceResponseV1,
};
use sha2::{Digest, Sha256};

use crate::{DurableReplayGuardV1, DurableReservationProviderV2};

const REQUIRED_ROSTER_ENTRIES: usize = 12;
const _: [(); REQUIRED_ROSTER_ENTRIES] = [(); M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1];

/// One immutable absolute deadline shared by every application phase.
#[derive(Clone, Copy, Debug)]
pub struct AbsoluteSessionDeadlineV1(Instant);

impl AbsoluteSessionDeadlineV1 {
    fn after(timeout: Duration) -> Option<Self> {
        Instant::now().checked_add(timeout).map(Self)
    }

    /// Returns the remaining duration without extending the deadline.
    pub fn remaining(self) -> Option<Duration> {
        let remaining = self.0.saturating_duration_since(Instant::now());
        (!remaining.is_zero()).then_some(remaining)
    }

    /// Returns the exact monotonic deadline without deriving a new duration.
    pub fn instant(self) -> Instant {
        self.0
    }

    fn require_live(self) -> Result<(), ServiceApplicationRejectionV1> {
        self.remaining()
            .map(|_| ())
            .ok_or(ServiceApplicationRejectionV1::DeadlineExpired)
    }
}

/// Exact kernel-reported caller and generic selection admitted by the service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServiceCallerPolicyV1 {
    pid: u32,
    uid: u32,
    gid: u32,
    roster_identity: [u8; 32],
}

impl ServiceCallerPolicyV1 {
    /// Pins one exact process credential tuple and aggregate roster identity.
    ///
    /// # Errors
    ///
    /// Returns a typed error if the PID or roster identity is zero.
    pub fn new(
        pid: u32,
        uid: u32,
        gid: u32,
        roster_identity: [u8; 32],
    ) -> Result<Self, FerricProtectedVerifierServiceConfigErrorV1> {
        if pid == 0 {
            return Err(FerricProtectedVerifierServiceConfigErrorV1::ZeroCallerPid);
        }
        if roster_identity == [0; 32] {
            return Err(FerricProtectedVerifierServiceConfigErrorV1::ZeroRosterIdentity);
        }
        Ok(Self {
            pid,
            uid,
            gid,
            roster_identity,
        })
    }

    fn matches_caller(self, caller: WorkerV3VerificationCallerV1) -> bool {
        caller.pid() == self.pid && caller.uid() == self.uid && caller.gid() == self.gid
    }
}

#[derive(Clone, Copy)]
struct AdmissionResolversV1 {
    caller: ServiceCallerPolicyV1,
    policy: WorkerV3VerificationPolicyIdentityV1,
    measurement: WorkerV3VerificationMeasurementIdentityV1,
}

impl WorkerV3VerificationPolicyResolverV1 for AdmissionResolversV1 {
    fn resolve_expected_policy(
        &mut self,
        caller: WorkerV3VerificationCallerV1,
        _request: &WorkerV3VerificationRequestV1,
    ) -> Option<WorkerV3VerificationPolicyIdentityV1> {
        self.caller.matches_caller(caller).then_some(self.policy)
    }
}

impl WorkerV3VerificationMeasurementResolverV1 for AdmissionResolversV1 {
    fn resolve_expected_measurement(
        &mut self,
        caller: WorkerV3VerificationCallerV1,
        policy: WorkerV3VerificationPolicyIdentityV1,
        _request: &WorkerV3VerificationRequestV1,
    ) -> Option<WorkerV3VerificationMeasurementIdentityV1> {
        (self.caller.matches_caller(caller) && policy == self.policy).then_some(self.measurement)
    }
}

/// Input to the separately protected compiler-current-record authenticator.
pub struct ProtectedCompilerCurrentRecordInputV1<'a> {
    request: &'a WorkerV3VerificationRequestV1,
    envelope: &'a WorkerV3LoadEnvelopeWireV2,
    current_record: &'a WorkerV3VerificationCurrentRecordFrameV2,
    deadline: AbsoluteSessionDeadlineV1,
}

impl ProtectedCompilerCurrentRecordInputV1<'_> {
    /// Returns the exact canonical Begin request.
    pub const fn request(&self) -> &WorkerV3VerificationRequestV1 {
        self.request
    }

    /// Returns the independently decoded exact envelope.
    pub const fn envelope(&self) -> &WorkerV3LoadEnvelopeWireV2 {
        self.envelope
    }

    /// Returns the canonically decoded challenge-bound current record.
    pub const fn current_record(&self) -> &WorkerV3VerificationCurrentRecordFrameV2 {
        self.current_record
    }

    /// Returns the exact canonical current-record byte array authenticated by the provider.
    pub const fn current_record_bytes(&self) -> &[u8] {
        self.current_record.encode_canonical()
    }

    /// Returns the sole absolute deadline.
    pub const fn deadline(&self) -> AbsoluteSessionDeadlineV1 {
        self.deadline
    }
}

/// Opaque acceptance token issued only by a reviewed protected authenticator.
///
/// ```compile_fail
/// use ferric_qwen3_all_kernels_worker_v3_verifier_service_v1::
///     AuthenticatedCompilerCurrentRecordV1;
/// fn duplicate(value: AuthenticatedCompilerCurrentRecordV1) {
///     let _second = value.clone();
/// }
/// ```
pub struct AuthenticatedCompilerCurrentRecordV1 {
    transcript_identity: [u8; 32],
}

impl AuthenticatedCompilerCurrentRecordV1 {
    /// Records the protected provider's nonzero authentication transcript.
    ///
    /// # Safety
    ///
    /// The caller must be the independently reviewed provider implementation.
    /// It must have authenticated the exact canonical verification and
    /// attestation in its input under pinned compiler policy, live Worker
    /// ledger state, and the external rollback authority. It must also have
    /// bound the reserved challenge and exact envelope carriage.
    ///
    /// # Errors
    ///
    /// Returns [`ProtectedProviderClaimErrorV1::ZeroTranscript`] for a zero transcript.
    pub unsafe fn from_independent_authentication(
        transcript_identity: [u8; 32],
    ) -> Result<Self, ProtectedProviderClaimErrorV1> {
        if transcript_identity == [0; 32] {
            return Err(ProtectedProviderClaimErrorV1::ZeroTranscript);
        }
        Ok(Self {
            transcript_identity,
        })
    }

    /// Returns the provider's authentication transcript identity.
    pub const fn transcript_identity(&self) -> [u8; 32] {
        self.transcript_identity
    }
}

/// Protected provider for independent compiler-current-record authentication.
///
/// # Safety
///
/// Implementations must communicate with an authenticated, measured protected
/// endpoint and may return acceptance only after satisfying the obligations on
/// [`AuthenticatedCompilerCurrentRecordV1::from_independent_authentication`]. IPC
/// must enforce the supplied absolute deadline and support transport cancellation;
/// a return after that deadline is rejected, but this synchronous call cannot
/// recover a thread from a hung implementation.
pub unsafe trait ProtectedCompilerCurrentRecordProviderV1 {
    /// Concrete provider failure.
    type Error: Error + Send + Sync + 'static;

    /// Returns the measurement identity pinned by the supervisor.
    fn measurement_identity(&self) -> [u8; 32];

    /// Independently authenticates the complete compiler-current record.
    ///
    /// # Errors
    ///
    /// Returns the provider's concrete transport, authentication, or policy error.
    fn authenticate_current_record(
        &mut self,
        input: ProtectedCompilerCurrentRecordInputV1<'_>,
    ) -> Result<AuthenticatedCompilerCurrentRecordV1, Self::Error>;
}

/// Exact checker handoff retaining every byte and authenticated current claim.
pub struct IndependentCheckerInputV1<'a> {
    request: &'a WorkerV3VerificationRequestV1,
    envelope_bytes: &'a [u8],
    envelope: &'a WorkerV3LoadEnvelopeWireV2,
    hsaco_bytes: &'a [u8],
    compiler_claims: &'a M1AllKernelsProtectedReceiptCompilerClaimsV1,
    current_authentication: &'a AuthenticatedCompilerCurrentRecordV1,
    deadline: AbsoluteSessionDeadlineV1,
}

impl IndependentCheckerInputV1<'_> {
    /// Returns the exact canonical Begin request with all 12 entries.
    pub const fn request(&self) -> &WorkerV3VerificationRequestV1 {
        self.request
    }

    /// Returns the exact retained canonical V2 envelope bytes.
    pub const fn envelope_bytes(&self) -> &[u8] {
        self.envelope_bytes
    }

    /// Returns the independently decoded V2 envelope.
    pub const fn envelope(&self) -> &WorkerV3LoadEnvelopeWireV2 {
        self.envelope
    }

    /// Returns the exact retained finalized HSACO bytes.
    pub const fn hsaco_bytes(&self) -> &[u8] {
        self.hsaco_bytes
    }

    /// Returns the service-reconstructed compiler/currentness claims.
    pub const fn compiler_claims(&self) -> &M1AllKernelsProtectedReceiptCompilerClaimsV1 {
        self.compiler_claims
    }

    /// Returns the move-only protected current-record acceptance token.
    pub const fn current_authentication(&self) -> &AuthenticatedCompilerCurrentRecordV1 {
        self.current_authentication
    }

    /// Returns the sole absolute deadline.
    pub const fn deadline(&self) -> AbsoluteSessionDeadlineV1 {
        self.deadline
    }
}

/// Complete claim set returned by the independently measured theorem checker.
///
/// ```compile_fail
/// use ferric_qwen3_all_kernels_worker_v3_verifier_service_v1::
///     IndependentCheckerVerifiedClaimsV1;
/// fn duplicate(value: IndependentCheckerVerifiedClaimsV1) {
///     let _second = value.clone();
/// }
/// ```
pub struct IndependentCheckerVerifiedClaimsV1 {
    request_claims: M1AllKernelsProtectedReceiptRequestClaimsV1,
    entries: [M1AllKernelsProtectedReceiptEntryV1; REQUIRED_ROSTER_ENTRIES],
    transcript_identity: [u8; 32],
}

impl IndependentCheckerVerifiedClaimsV1 {
    /// Constructs the exact claim set asserted by the independent checker.
    ///
    /// # Safety
    ///
    /// The caller must be the reviewed checker provider. It must have decoded
    /// the exact envelope and HSACO, replayed finalization, validated all proof
    /// inputs, and proved every required safety property for exactly the 12
    /// ordered entries in the supplied request.
    ///
    /// # Errors
    ///
    /// Returns [`ProtectedProviderClaimErrorV1::ZeroTranscript`] for a zero transcript.
    pub unsafe fn from_independent_checker(
        request_claims: M1AllKernelsProtectedReceiptRequestClaimsV1,
        entries: [M1AllKernelsProtectedReceiptEntryV1; REQUIRED_ROSTER_ENTRIES],
        transcript_identity: [u8; 32],
    ) -> Result<Self, ProtectedProviderClaimErrorV1> {
        if transcript_identity == [0; 32] {
            return Err(ProtectedProviderClaimErrorV1::ZeroTranscript);
        }
        Ok(Self {
            request_claims,
            entries,
            transcript_identity,
        })
    }
}

/// Separate measured theorem-checker provider contract.
///
/// # Safety
///
/// Implementations must use authenticated bounded IPC to an independently
/// measured checker and uphold the constructor obligations of
/// [`IndependentCheckerVerifiedClaimsV1`]. IPC must enforce the supplied absolute
/// deadline and support cancellation; a late return is always rejected.
pub unsafe trait IndependentCheckerProviderV1 {
    /// Concrete checker failure.
    type Error: Error + Send + Sync + 'static;

    /// Returns the checker measurement bound into the receipt trust policy.
    fn measurement_identity(&self) -> [u8; 32];

    /// Verifies the exact 12-entry artifact/roster handoff.
    ///
    /// # Errors
    ///
    /// Returns the checker's concrete IPC, proof, or policy error.
    fn verify_all_kernels(
        &mut self,
        input: IndependentCheckerInputV1<'_>,
    ) -> Result<IndependentCheckerVerifiedClaimsV1, Self::Error>;
}

/// Bounded signing request sent to the external signing provider.
pub struct ProtectedReceiptSignerInputV1<'a> {
    signing_bytes: &'a [u8],
    policy_identity: [u8; 32],
    deadline: AbsoluteSessionDeadlineV1,
}

impl ProtectedReceiptSignerInputV1<'_> {
    /// Returns the exact domain-separated unsigned receipt bytes.
    pub const fn signing_bytes(&self) -> &[u8] {
        self.signing_bytes
    }

    /// Returns the exact pinned trust-policy identity.
    pub const fn policy_identity(&self) -> [u8; 32] {
        self.policy_identity
    }

    /// Returns the sole absolute deadline.
    pub const fn deadline(&self) -> AbsoluteSessionDeadlineV1 {
        self.deadline
    }
}

/// External protected signer provider. It never exposes private-key material.
/// Implementations must use authenticated deadline-aware, cancellable IPC. A
/// return after the supplied deadline is always rejected by the service.
pub trait ProtectedReceiptSignerProviderV1 {
    /// Concrete signer transport or policy failure.
    type Error: Error + Send + Sync + 'static;

    /// Returns the public key served by the pinned signing endpoint.
    fn verifying_key_bytes(&self) -> [u8; 32];

    /// Returns the separately supervisor-pinned signer endpoint identity.
    fn provider_identity(&self) -> [u8; 32];

    /// Signs the one exact domain-separated receipt message.
    ///
    /// # Errors
    ///
    /// Returns the external signer's concrete IPC or signing-policy error.
    fn sign_receipt(
        &mut self,
        input: ProtectedReceiptSignerInputV1<'_>,
    ) -> Result<[u8; 64], Self::Error>;
}

/// Invalid claim emitted by a protected provider implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtectedProviderClaimErrorV1 {
    /// A required provider transcript was zero.
    ZeroTranscript,
}

impl fmt::Display for ProtectedProviderClaimErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroTranscript => formatter.write_str("protected provider transcript is zero"),
        }
    }
}

impl Error for ProtectedProviderClaimErrorV1 {}

/// Path-free supervisor configuration for one protected service process.
pub struct FerricProtectedVerifierServiceConfigV1<C, K, S> {
    caller: ServiceCallerPolicyV1,
    trust_policy: M1AllKernelsProtectedVerifierTrustPolicyV1,
    replay_guard: DurableReplayGuardV1,
    reservations: DurableReservationProviderV2,
    current_provider: C,
    checker: K,
    signer: S,
    expected_current_provider_measurement: [u8; 32],
    expected_signer_provider_identity: [u8; 32],
    timeout: Duration,
}

impl<C, K, S> FerricProtectedVerifierServiceConfigV1<C, K, S>
where
    C: ProtectedCompilerCurrentRecordProviderV1,
    K: IndependentCheckerProviderV1,
    S: ProtectedReceiptSignerProviderV1,
{
    /// Joins only preopened state owners, provider instances, and pinned identities.
    ///
    /// # Errors
    ///
    /// Returns a typed error for a zero timeout/identity or substituted provider.
    pub fn new(
        caller: ServiceCallerPolicyV1,
        trust_policy: M1AllKernelsProtectedVerifierTrustPolicyV1,
        replay_guard: DurableReplayGuardV1,
        reservations: DurableReservationProviderV2,
        current_provider: C,
        checker: K,
        signer: S,
        expected_current_provider_measurement: [u8; 32],
        expected_signer_provider_identity: [u8; 32],
        timeout: Duration,
    ) -> Result<Self, FerricProtectedVerifierServiceConfigErrorV1> {
        if timeout.is_zero() {
            return Err(FerricProtectedVerifierServiceConfigErrorV1::ZeroTimeout);
        }
        if expected_current_provider_measurement == [0; 32]
            || expected_signer_provider_identity == [0; 32]
        {
            return Err(FerricProtectedVerifierServiceConfigErrorV1::ZeroProviderIdentity);
        }
        let service_policy =
            WorkerV3VerificationPolicyIdentityV1::new(*trust_policy.identity().as_bytes())
                .map_err(|_| {
                    FerricProtectedVerifierServiceConfigErrorV1::InvalidProtocolIdentity
                })?;
        if replay_guard.policy_identity() != service_policy
            || reservations.policy_identity() != service_policy
        {
            return Err(FerricProtectedVerifierServiceConfigErrorV1::LedgerPolicyMismatch);
        }
        validate_provider_identities(
            &trust_policy,
            &current_provider,
            &checker,
            &signer,
            expected_current_provider_measurement,
            expected_signer_provider_identity,
        )?;
        Ok(Self {
            caller,
            trust_policy,
            replay_guard,
            reservations,
            current_provider,
            checker,
            signer,
            expected_current_provider_measurement,
            expected_signer_provider_identity,
            timeout,
        })
    }

    /// Configuration custody alone grants no verification, load, or launch authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

/// Configuration rejection before any connection is admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FerricProtectedVerifierServiceConfigErrorV1 {
    /// The pinned peer PID used the forbidden zero value.
    ZeroCallerPid,
    /// The aggregate roster identity was zero.
    ZeroRosterIdentity,
    /// The session timeout was zero.
    ZeroTimeout,
    /// A protected-provider identity was zero.
    ZeroProviderIdentity,
    /// The current-record provider did not have the pinned measurement.
    CurrentProviderSubstitution,
    /// The checker did not have the trust-policy measurement.
    CheckerSubstitution,
    /// The signer endpoint identity or public key was substituted.
    SignerSubstitution,
    /// A replay or reservation ledger was bound to another trust-policy identity.
    LedgerPolicyMismatch,
    /// A pinned identity could not be represented by the generic protocol.
    InvalidProtocolIdentity,
}

impl fmt::Display for FerricProtectedVerifierServiceConfigErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "protected-verifier service configuration rejected: {self:?}"
        )
    }
}

impl Error for FerricProtectedVerifierServiceConfigErrorV1 {}

/// Stable application stage that caused a fail-closed terminal rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ServiceApplicationRejectionV1 {
    /// The single absolute deadline expired.
    DeadlineExpired,
    /// The pinned aggregate roster or exact cardinality mismatched.
    RosterMismatch,
    /// The second bounded retained-payload read or digest check failed.
    PayloadRevalidation,
    /// The exact V2 envelope failed canonical decoding or re-encoding.
    EnvelopeDecode,
    /// The current-record provider was substituted.
    CurrentProviderSubstitution,
    /// Independent current-record authentication failed.
    CurrentRecordAuthentication,
    /// Authenticated current fields did not bind the envelope and carriage.
    CurrentRecordAssociation,
    /// The independent checker was substituted or rejected the handoff.
    CheckerRejected,
    /// Checker claims did not bind every known request/artifact/entry coordinate.
    CheckerAssociation,
    /// The external signer was substituted or unavailable.
    SignerRejected,
    /// The returned signature failed local public-key authentication.
    SignatureRejected,
    /// Ferric's unchanged V1 request/response schema rejected an internal join.
    FerricReceiptAssembly,
    /// The generic current-record phase rejected its input.
    GenericCurrentRecord,
}

/// Completed disposition retaining fe2o3's exact terminal session custody.
#[non_exhaustive]
pub enum FerricProtectedVerifierServiceOutcomeV1 {
    /// A signed, locally authenticated Ferric V1 response was sent.
    Completed(CompletedWorkerV3VerificationSessionV2),
    /// A generic terminal rejection was sent for this application stage.
    Rejected {
        /// Stable fail-closed application reason.
        reason: ServiceApplicationRejectionV1,
        /// Complete generic session custody after the rejection send.
        session: CompletedWorkerV3VerificationSessionV2,
    },
    /// Begin admission rejected before any service reservation was released.
    BeginRejected(RejectedWorkerV3VerificationBeginV2),
}

/// Transport failure retaining terminal custody whenever fe2o3 exposes it.
///
/// ```compile_fail
/// use ferric_qwen3_all_kernels_worker_v3_verifier_service_v1::
///     FerricProtectedVerifierServiceFailureV1;
/// fn duplicate_terminal_custody(failure: FerricProtectedVerifierServiceFailureV1) {
///     if let FerricProtectedVerifierServiceFailureV1::ReadyRejectionSend {
///         failure, ..
///     } = failure {
///         let _first = failure.into_session();
///         let _second = failure.into_session();
///     }
/// }
/// ```
#[non_exhaustive]
pub enum FerricProtectedVerifierServiceFailureV1 {
    /// The deadline could not be represented.
    DeadlineOverflow,
    /// A pinned trust-policy identity could not be represented by fe2o3.
    InvalidPinnedProtocolIdentity,
    /// A future generic transport returned a disposition this version cannot retain.
    UnsupportedTransportDisposition,
    /// Generic Begin admission or transport failed before recoverable terminal custody.
    Begin(WorkerV3VerificationServiceErrorV2),
    /// Current-record receive failed after reservation; the reservation remains burned.
    CurrentRecord(WorkerV3VerificationServiceErrorV2),
    /// A ready-session rejection could not be sent; custody is recoverable.
    ReadyRejectionSend {
        /// Application stage that caused rejection.
        reason: ServiceApplicationRejectionV1,
        /// Recoverable exact ready-session custody.
        failure: WorkerV3VerificationTerminalSendFailureV2,
    },
    /// A generic phase rejection could not be sent; custody is recoverable.
    PendingRejectionSend(WorkerV3VerificationRejectedSendFailureV2),
    /// A completed Ferric response could not be sent; custody is recoverable.
    ResponseSend(WorkerV3VerificationTerminalSendFailureV2),
}

impl fmt::Debug for FerricProtectedVerifierServiceFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeadlineOverflow => formatter.write_str("DeadlineOverflow"),
            Self::InvalidPinnedProtocolIdentity => {
                formatter.write_str("InvalidPinnedProtocolIdentity")
            }
            Self::UnsupportedTransportDisposition => {
                formatter.write_str("UnsupportedTransportDisposition")
            }
            Self::Begin(source) => formatter.debug_tuple("Begin").field(source).finish(),
            Self::CurrentRecord(source) => formatter
                .debug_tuple("CurrentRecord")
                .field(source)
                .finish(),
            Self::ReadyRejectionSend { reason, failure } => formatter
                .debug_struct("ReadyRejectionSend")
                .field("reason", reason)
                .field("failure", failure)
                .finish(),
            Self::PendingRejectionSend(failure) => formatter
                .debug_tuple("PendingRejectionSend")
                .field(failure)
                .finish(),
            Self::ResponseSend(failure) => formatter
                .debug_tuple("ResponseSend")
                .field(failure)
                .finish(),
        }
    }
}

impl fmt::Display for FerricProtectedVerifierServiceFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Ferric protected-verifier session failed: {self:?}"
        )
    }
}

impl Error for FerricProtectedVerifierServiceFailureV1 {}

/// Runs one complete V2 connection and emits one unchanged Ferric V1 response payload.
///
/// # Errors
///
/// Returns a custody-preserving transport failure or an unrecoverable Begin failure.
pub fn run_ferric_protected_verifier_session_v2<C, K, S>(
    control: OwnedFd,
    config: &mut FerricProtectedVerifierServiceConfigV1<C, K, S>,
) -> Result<FerricProtectedVerifierServiceOutcomeV1, FerricProtectedVerifierServiceFailureV1>
where
    C: ProtectedCompilerCurrentRecordProviderV1,
    K: IndependentCheckerProviderV1,
    S: ProtectedReceiptSignerProviderV1,
{
    let deadline = AbsoluteSessionDeadlineV1::after(config.timeout)
        .ok_or(FerricProtectedVerifierServiceFailureV1::DeadlineOverflow)?;
    let policy =
        WorkerV3VerificationPolicyIdentityV1::new(*config.trust_policy.identity().as_bytes())
            .map_err(|_| FerricProtectedVerifierServiceFailureV1::InvalidPinnedProtocolIdentity)?;
    let measurement = WorkerV3VerificationMeasurementIdentityV1::new(
        config.trust_policy.verifier_measurement_sha256(),
    )
    .map_err(|_| FerricProtectedVerifierServiceFailureV1::InvalidPinnedProtocolIdentity)?;
    let resolvers = AdmissionResolversV1 {
        caller: config.caller,
        policy,
        measurement,
    };
    let mut policy_resolver = resolvers;
    let mut measurement_resolver = resolvers;
    let begin = begin_worker_v3_verification_session_until_v2(
        control,
        deadline.instant(),
        &mut policy_resolver,
        &mut measurement_resolver,
        &mut config.replay_guard,
        &mut config.reservations,
    )
    .map_err(FerricProtectedVerifierServiceFailureV1::Begin)?;
    let pending = match begin {
        WorkerV3VerificationBeginOutcomeV2::Reserved(pending) => pending,
        WorkerV3VerificationBeginOutcomeV2::Rejected(rejected) => {
            return Ok(FerricProtectedVerifierServiceOutcomeV1::BeginRejected(
                rejected,
            ));
        }
        _ => {
            return Err(FerricProtectedVerifierServiceFailureV1::UnsupportedTransportDisposition);
        }
    };
    debug_assert_eq!(pending.deadline(), deadline.instant());

    let mut early_rejection = deadline
        .require_live()
        .err()
        .or_else(|| validate_roster(pending.request(), config.caller).err());
    let payloads = match read_and_decode_payloads(&pending) {
        Ok(payloads) => Some(payloads),
        Err(reason) => {
            early_rejection.get_or_insert(reason);
            None
        }
    };
    let current = pending
        .receive_current_record()
        .map_err(FerricProtectedVerifierServiceFailureV1::CurrentRecord)?;
    let ready = match current {
        WorkerV3VerificationCurrentRecordOutcomeV2::Ready(ready) => ready,
        WorkerV3VerificationCurrentRecordOutcomeV2::Rejected(rejected) => {
            return reject_pending_current(rejected);
        }
        _ => {
            return Err(FerricProtectedVerifierServiceFailureV1::UnsupportedTransportDisposition);
        }
    };
    debug_assert_eq!(ready.deadline(), deadline.instant());
    if let Some(reason) = early_rejection {
        return reject_ready(ready, reason);
    }
    let Some((envelope_bytes, envelope, hsaco_bytes)) = payloads else {
        return reject_ready(ready, ServiceApplicationRejectionV1::PayloadRevalidation);
    };
    if deadline.require_live().is_err() {
        return reject_ready(ready, ServiceApplicationRejectionV1::DeadlineExpired);
    }
    let current_provider_measurement = config.current_provider.measurement_identity();
    if deadline.require_live().is_err() {
        return reject_ready(ready, ServiceApplicationRejectionV1::DeadlineExpired);
    }
    if current_provider_measurement != config.expected_current_provider_measurement {
        return reject_ready(
            ready,
            ServiceApplicationRejectionV1::CurrentProviderSubstitution,
        );
    }
    let current_record =
        match decode_current_record_bytes(ready.current_record().encode_canonical()) {
            Ok(current_record) => current_record,
            Err(reason) => return reject_ready(ready, reason),
        };
    let current_authentication_result =
        current_provider_result(config.current_provider.authenticate_current_record(
            ProtectedCompilerCurrentRecordInputV1 {
                request: ready.request(),
                envelope: &envelope,
                current_record: &current_record,
                deadline,
            },
        ));
    if deadline.require_live().is_err() {
        return reject_ready(ready, ServiceApplicationRejectionV1::DeadlineExpired);
    }
    let current_authentication = match current_authentication_result {
        Ok(authentication) => authentication,
        Err(reason) => return reject_ready(ready, reason),
    };
    let Ok(compiler_claims) = build_compiler_claims(&envelope, &current_record) else {
        return reject_ready(
            ready,
            ServiceApplicationRejectionV1::CurrentRecordAssociation,
        );
    };
    if deadline.require_live().is_err() {
        return reject_ready(ready, ServiceApplicationRejectionV1::DeadlineExpired);
    }
    let checker_measurement = config.checker.measurement_identity();
    if deadline.require_live().is_err() {
        return reject_ready(ready, ServiceApplicationRejectionV1::DeadlineExpired);
    }
    if checker_measurement != config.trust_policy.checker_measurement_sha256() {
        return reject_ready(ready, ServiceApplicationRejectionV1::CheckerRejected);
    }
    let checked_result = checker_provider_result(config.checker.verify_all_kernels(
        IndependentCheckerInputV1 {
            request: ready.request(),
            envelope_bytes: &envelope_bytes,
            envelope: &envelope,
            hsaco_bytes: &hsaco_bytes,
            compiler_claims: &compiler_claims,
            current_authentication: &current_authentication,
            deadline,
        },
    ));
    if deadline.require_live().is_err() {
        return reject_ready(ready, ServiceApplicationRejectionV1::DeadlineExpired);
    }
    let checked = match checked_result {
        Ok(checked) => checked,
        Err(reason) => return reject_ready(ready, reason),
    };
    if !checker_claims_associate(ready.request(), &hsaco_bytes, &checked) {
        return reject_ready(ready, ServiceApplicationRejectionV1::CheckerAssociation);
    }
    let Ok(service_entries) = service_entries(ready.request()) else {
        return reject_ready(ready, ServiceApplicationRejectionV1::CheckerAssociation);
    };
    let Ok(service_request) = M1AllKernelsProtectedVerifierServiceRequestV1::new(
        config.trust_policy.identity(),
        checked.request_claims,
        compiler_claims,
        service_entries,
    ) else {
        return reject_ready(ready, ServiceApplicationRejectionV1::FerricReceiptAssembly);
    };
    let Ok(unsigned) = M1AllKernelsUnsignedProtectedVerifierReceiptV1::new(
        config.trust_policy.identity(),
        checked.request_claims,
        compiler_claims,
        config.trust_policy.verifier_measurement_sha256(),
        config.trust_policy.checker_measurement_sha256(),
        checked.transcript_identity,
        checked.entries,
    ) else {
        return reject_ready(ready, ServiceApplicationRejectionV1::FerricReceiptAssembly);
    };
    if deadline.require_live().is_err() {
        return reject_ready(ready, ServiceApplicationRejectionV1::DeadlineExpired);
    }
    let signing_bytes = unsigned.signing_bytes();
    let signature = match request_receipt_signature(
        &mut config.signer,
        config.expected_signer_provider_identity,
        config.trust_policy.verifying_key_bytes(),
        &signing_bytes,
        *config.trust_policy.identity().as_bytes(),
        deadline,
    ) {
        Ok(signature) => signature,
        Err(reason) => return reject_ready(ready, reason),
    };
    let receipt = unsigned.attach_signature(signature);
    if config
        .trust_policy
        .authenticate_canonical(receipt.encode_canonical())
        .is_err()
    {
        return reject_ready(ready, ServiceApplicationRejectionV1::SignatureRejected);
    }
    let Ok(response) =
        M1AllKernelsProtectedVerifierServiceResponseV1::new(&service_request, receipt)
    else {
        return reject_ready(ready, ServiceApplicationRejectionV1::FerricReceiptAssembly);
    };
    if deadline.require_live().is_err() {
        return reject_ready(ready, ServiceApplicationRejectionV1::DeadlineExpired);
    }
    let completed = ready
        .send_application_response(response.canonical_bytes().to_vec())
        .map_err(FerricProtectedVerifierServiceFailureV1::ResponseSend)?;
    Ok(FerricProtectedVerifierServiceOutcomeV1::Completed(
        completed,
    ))
}

fn validate_roster(
    request: &WorkerV3VerificationRequestV1,
    policy: ServiceCallerPolicyV1,
) -> Result<(), ServiceApplicationRejectionV1> {
    if request.entries().len() != REQUIRED_ROSTER_ENTRIES
        || request.roster_identity().as_bytes() != &policy.roster_identity
    {
        return Err(ServiceApplicationRejectionV1::RosterMismatch);
    }
    Ok(())
}

fn read_and_decode_payloads(
    pending: &impl PendingPayloadViewV1,
) -> Result<(Vec<u8>, WorkerV3LoadEnvelopeWireV2, Vec<u8>), ServiceApplicationRejectionV1> {
    let envelope = read_retained_payload(
        pending.payload(WorkerV3VerificationFdPayloadKindV1::LoadEnvelopeV2),
    )?;
    let hsaco = read_retained_payload(
        pending.payload(WorkerV3VerificationFdPayloadKindV1::FinalizedHsaco),
    )?;
    let decoded = WorkerV3LoadEnvelopeWireV2::decode_canonical(&envelope)
        .map_err(|_| ServiceApplicationRejectionV1::EnvelopeDecode)?;
    if match decoded.encode_canonical() {
        Ok(canonical) => canonical != envelope,
        Err(_) => true,
    } {
        return Err(ServiceApplicationRejectionV1::EnvelopeDecode);
    }
    if decoded
        .published_claim()
        .worker_v3_binding()
        .finalized_output_sha256()
        != sha256(&hsaco)
        || decoded
            .published_claim()
            .worker_v3_binding()
            .finalized_output_length()
            != hsaco.len() as u64
    {
        return Err(ServiceApplicationRejectionV1::PayloadRevalidation);
    }
    Ok((envelope, decoded, hsaco))
}

trait PendingPayloadViewV1 {
    fn payload(
        &self,
        kind: WorkerV3VerificationFdPayloadKindV1,
    ) -> &RetainedWorkerV3VerificationPayloadV1;
}

impl PendingPayloadViewV1
    for fe2o3_worker_v3_verification_service::PendingWorkerV3VerificationCurrentRecordSessionV2
{
    fn payload(
        &self,
        kind: WorkerV3VerificationFdPayloadKindV1,
    ) -> &RetainedWorkerV3VerificationPayloadV1 {
        self.payload(kind)
    }
}

fn read_retained_payload(
    payload: &RetainedWorkerV3VerificationPayloadV1,
) -> Result<Vec<u8>, ServiceApplicationRejectionV1> {
    read_exact_preopened_payload(
        payload,
        payload.byte_len(),
        payload.kind().maximum_byte_len(),
        *payload.sha256(),
    )
}

fn read_exact_preopened_payload(
    file: &impl AsFd,
    byte_len: u64,
    maximum_byte_len: u64,
    expected_sha256: [u8; 32],
) -> Result<Vec<u8>, ServiceApplicationRejectionV1> {
    let length = usize::try_from(byte_len)
        .map_err(|_| ServiceApplicationRejectionV1::PayloadRevalidation)?;
    let expected_stat_length =
        i64::try_from(byte_len).map_err(|_| ServiceApplicationRejectionV1::PayloadRevalidation)?;
    if length == 0 || byte_len > maximum_byte_len {
        return Err(ServiceApplicationRejectionV1::PayloadRevalidation);
    }
    let before =
        rustix::fs::fstat(file).map_err(|_| ServiceApplicationRejectionV1::PayloadRevalidation)?;
    let mut bytes = vec![0_u8; length];
    let mut offset = 0_usize;
    while offset < length {
        let count = rustix::io::pread(file, &mut bytes[offset..], offset as u64)
            .map_err(|_| ServiceApplicationRejectionV1::PayloadRevalidation)?;
        if count == 0 {
            return Err(ServiceApplicationRejectionV1::PayloadRevalidation);
        }
        offset += count;
    }
    let mut trailing = [0_u8; 1];
    if rustix::io::pread(file, &mut trailing, length as u64)
        .map_err(|_| ServiceApplicationRejectionV1::PayloadRevalidation)?
        != 0
    {
        return Err(ServiceApplicationRejectionV1::PayloadRevalidation);
    }
    let after =
        rustix::fs::fstat(file).map_err(|_| ServiceApplicationRejectionV1::PayloadRevalidation)?;
    if before.st_dev != after.st_dev
        || before.st_ino != after.st_ino
        || before.st_size != after.st_size
        || before.st_mode != after.st_mode
        || before.st_nlink != after.st_nlink
        || before.st_size != expected_stat_length
        || sha256(&bytes) != expected_sha256
    {
        return Err(ServiceApplicationRejectionV1::PayloadRevalidation);
    }
    Ok(bytes)
}

fn decode_current_record_bytes(
    bytes: &[u8],
) -> Result<WorkerV3VerificationCurrentRecordFrameV2, ServiceApplicationRejectionV1> {
    let decoded = WorkerV3VerificationCurrentRecordFrameV2::decode_canonical(bytes)
        .map_err(|_| ServiceApplicationRejectionV1::CurrentRecordAuthentication)?;
    if decoded.encode_canonical().as_slice() != bytes || decoded.grants_authority() {
        return Err(ServiceApplicationRejectionV1::CurrentRecordAuthentication);
    }
    Ok(decoded)
}

fn validate_provider_identities<C, K, S>(
    trust_policy: &M1AllKernelsProtectedVerifierTrustPolicyV1,
    current_provider: &C,
    checker: &K,
    signer: &S,
    expected_current_provider_measurement: [u8; 32],
    expected_signer_provider_identity: [u8; 32],
) -> Result<(), FerricProtectedVerifierServiceConfigErrorV1>
where
    C: ProtectedCompilerCurrentRecordProviderV1,
    K: IndependentCheckerProviderV1,
    S: ProtectedReceiptSignerProviderV1,
{
    if current_provider.measurement_identity() != expected_current_provider_measurement {
        return Err(FerricProtectedVerifierServiceConfigErrorV1::CurrentProviderSubstitution);
    }
    if checker.measurement_identity() != trust_policy.checker_measurement_sha256() {
        return Err(FerricProtectedVerifierServiceConfigErrorV1::CheckerSubstitution);
    }
    if signer.provider_identity() != expected_signer_provider_identity
        || signer.verifying_key_bytes() != trust_policy.verifying_key_bytes()
    {
        return Err(FerricProtectedVerifierServiceConfigErrorV1::SignerSubstitution);
    }
    Ok(())
}

fn request_receipt_signature<S: ProtectedReceiptSignerProviderV1>(
    signer: &mut S,
    expected_provider_identity: [u8; 32],
    expected_verifying_key: [u8; 32],
    signing_bytes: &[u8],
    policy_identity: [u8; 32],
    deadline: AbsoluteSessionDeadlineV1,
) -> Result<[u8; 64], ServiceApplicationRejectionV1> {
    deadline.require_live()?;
    let provider_identity = signer.provider_identity();
    deadline.require_live()?;
    let verifying_key = signer.verifying_key_bytes();
    deadline.require_live()?;
    if provider_identity != expected_provider_identity || verifying_key != expected_verifying_key {
        return Err(ServiceApplicationRejectionV1::SignerRejected);
    }
    let signature = signer.sign_receipt(ProtectedReceiptSignerInputV1 {
        signing_bytes,
        policy_identity,
        deadline,
    });
    deadline.require_live()?;
    signature.map_err(|_| ServiceApplicationRejectionV1::SignerRejected)
}

fn current_provider_result<T, E>(result: Result<T, E>) -> Result<T, ServiceApplicationRejectionV1> {
    result.map_err(|_| ServiceApplicationRejectionV1::CurrentRecordAuthentication)
}

fn checker_provider_result<T, E>(result: Result<T, E>) -> Result<T, ServiceApplicationRejectionV1> {
    result.map_err(|_| ServiceApplicationRejectionV1::CheckerRejected)
}

fn build_compiler_claims(
    envelope: &WorkerV3LoadEnvelopeWireV2,
    current: &WorkerV3VerificationCurrentRecordFrameV2,
) -> Result<M1AllKernelsProtectedReceiptCompilerClaimsV1, ()> {
    let subject = envelope
        .reconstructed_compiler_execution_subject_v1()
        .map_err(|_| ())?;
    let carriage = envelope.compiler_execution_receipt();
    let verification = current.verification();
    if verification.subject_identity() != *subject.identity().sha256()
        || verification.carriage_identity() != *carriage.identity().as_bytes()
        || verification.policy_identity() != *carriage.policy().identity().as_bytes()
        || verification.issuer_journal_identity()
            != carriage.acknowledgment().issuer_journal_identity()
        || verification.worker_ledger_record_identity()
            != carriage.acknowledgment().worker_ledger_record_identity()
        || verification.sequence() != carriage.acknowledgment().sequence()
        || verification.prior_rollback_anchor()
            != carriage.publication().receipt().prior_rollback_anchor()
        || verification.current_rollback_anchor()
            != carriage.acknowledgment().current_rollback_anchor()
    {
        return Err(());
    }
    M1AllKernelsProtectedReceiptCompilerClaimsV1::new(
        *subject.identity().sha256(),
        *carriage.identity().as_bytes(),
        *carriage.policy().identity().as_bytes(),
        carriage.acknowledgment().issuer_journal_identity(),
        carriage.acknowledgment().compiler_occurrence_identity(),
        *carriage.acknowledgment().receipt_identity().as_bytes(),
        *carriage.acknowledgment().publication_identity().as_bytes(),
        *carriage.acknowledgment().identity().as_bytes(),
        carriage.acknowledgment().worker_ledger_record_identity(),
        carriage.acknowledgment().sequence(),
        carriage.publication().receipt().prior_rollback_anchor(),
        carriage.acknowledgment().current_rollback_anchor(),
        *verification.identity().as_bytes(),
        *current.attestation().identity().as_bytes(),
        verification.protected_policy_verification_identity(),
        verification.protected_worker_ledger_verification_identity(),
        verification.external_rollback_verification_identity(),
    )
    .map_err(|_| ())
}

fn checker_claims_associate(
    request: &WorkerV3VerificationRequestV1,
    hsaco: &[u8],
    checked: &IndependentCheckerVerifiedClaimsV1,
) -> bool {
    checked.request_claims.challenge_identity() == *request.challenge().as_bytes()
        && checked.request_claims.roster_identity() == *request.roster_identity().as_bytes()
        && checked.request_claims.finalized_hsaco_sha256() == sha256(hsaco)
        && checked.request_claims.finalized_hsaco_length() == hsaco.len() as u64
        && request
            .entries()
            .iter()
            .zip(&checked.entries)
            .all(|(expected, actual)| {
                u32::from(actual.ordinal()) == expected.ordinal()
                    && actual.lineage_identity() == *expected.lineage_identity()
                    && actual.marker_binding_identity() == *expected.marker_binding_identity()
                    && actual.generated_host_contract_identity()
                        == *expected.generated_host_contract_identity()
            })
}

fn service_entries(
    request: &WorkerV3VerificationRequestV1,
) -> Result<
    [M1AllKernelsProtectedVerifierServiceEntryV1; REQUIRED_ROSTER_ENTRIES],
    M1AllKernelsProtectedVerifierServiceProtocolErrorV1,
> {
    request
        .entries()
        .iter()
        .map(|entry| {
            M1AllKernelsProtectedVerifierServiceEntryV1::new(
                u16::try_from(entry.ordinal()).map_err(|_| {
                    M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidEntryCount
                })?,
                *entry.lineage_identity(),
                *entry.marker_binding_identity(),
                *entry.generated_host_contract_identity(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| M1AllKernelsProtectedVerifierServiceProtocolErrorV1::InvalidEntryCount)
}

fn reject_ready(
    ready: PendingWorkerV3VerificationTerminalSessionV2,
    reason: ServiceApplicationRejectionV1,
) -> Result<FerricProtectedVerifierServiceOutcomeV1, FerricProtectedVerifierServiceFailureV1> {
    match ready.send_rejection() {
        Ok(session) => Ok(FerricProtectedVerifierServiceOutcomeV1::Rejected { reason, session }),
        Err(failure) => {
            Err(FerricProtectedVerifierServiceFailureV1::ReadyRejectionSend { reason, failure })
        }
    }
}

fn reject_pending_current(
    rejected: PendingRejectedWorkerV3VerificationTerminalSessionV2,
) -> Result<FerricProtectedVerifierServiceOutcomeV1, FerricProtectedVerifierServiceFailureV1> {
    let session = rejected
        .send_rejection()
        .map_err(FerricProtectedVerifierServiceFailureV1::PendingRejectionSend)?;
    Ok(FerricProtectedVerifierServiceOutcomeV1::Rejected {
        reason: ServiceApplicationRejectionV1::GenericCurrentRecord,
        session,
    })
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{self, Write};

    use ed25519_dalek::SigningKey;
    use fe2o3_worker_v3_verification_protocol::{
        WorkerV3VerificationEntryCoordinateV1, WorkerV3VerificationFdPayloadDescriptorV1,
        WorkerV3VerificationFreshChallengeV1, WorkerV3VerificationMeasurementIdentityV1,
        WorkerV3VerificationPolicyIdentityV1, WorkerV3VerificationRosterIdentityV1,
    };

    use super::*;

    struct MockCurrentProvider {
        measurement: [u8; 32],
    }

    // The mock always rejects and cannot issue an authentication token.
    unsafe impl ProtectedCompilerCurrentRecordProviderV1 for MockCurrentProvider {
        type Error = io::Error;

        fn measurement_identity(&self) -> [u8; 32] {
            self.measurement
        }

        fn authenticate_current_record(
            &mut self,
            _input: ProtectedCompilerCurrentRecordInputV1<'_>,
        ) -> Result<AuthenticatedCompilerCurrentRecordV1, Self::Error> {
            Err(io::Error::other("test current provider failure"))
        }
    }

    struct MockCheckerProvider {
        measurement: [u8; 32],
    }

    // The mock always rejects and cannot issue checked claims.
    unsafe impl IndependentCheckerProviderV1 for MockCheckerProvider {
        type Error = io::Error;

        fn measurement_identity(&self) -> [u8; 32] {
            self.measurement
        }

        fn verify_all_kernels(
            &mut self,
            _input: IndependentCheckerInputV1<'_>,
        ) -> Result<IndependentCheckerVerifiedClaimsV1, Self::Error> {
            Err(io::Error::other("test checker failure"))
        }
    }

    struct MockSignerProvider {
        verifying_key: [u8; 32],
        provider: [u8; 32],
        fail: bool,
        calls: usize,
        delay: Duration,
    }

    impl ProtectedReceiptSignerProviderV1 for MockSignerProvider {
        type Error = io::Error;

        fn verifying_key_bytes(&self) -> [u8; 32] {
            self.verifying_key
        }

        fn provider_identity(&self) -> [u8; 32] {
            self.provider
        }

        fn sign_receipt(
            &mut self,
            input: ProtectedReceiptSignerInputV1<'_>,
        ) -> Result<[u8; 64], Self::Error> {
            self.calls += 1;
            std::thread::sleep(self.delay);
            assert!(!input.signing_bytes().is_empty());
            assert_ne!(input.policy_identity(), [0; 32]);
            if self.fail {
                Err(io::Error::other("test signer failure"))
            } else {
                Ok([0x55; 64])
            }
        }
    }

    fn policy() -> M1AllKernelsProtectedVerifierTrustPolicyV1 {
        let signing = SigningKey::from_bytes(&[0x91; 32]);
        M1AllKernelsProtectedVerifierTrustPolicyV1::new(
            signing.verifying_key().to_bytes(),
            [0xa1; 32],
            [0xa2; 32],
        )
        .unwrap()
    }

    fn providers() -> (MockCurrentProvider, MockCheckerProvider, MockSignerProvider) {
        let policy = policy();
        (
            MockCurrentProvider {
                measurement: [0xa3; 32],
            },
            MockCheckerProvider {
                measurement: policy.checker_measurement_sha256(),
            },
            MockSignerProvider {
                verifying_key: policy.verifying_key_bytes(),
                provider: [0xa4; 32],
                fail: false,
                calls: 0,
                delay: Duration::ZERO,
            },
        )
    }

    fn request(entry_count: usize) -> WorkerV3VerificationRequestV1 {
        let entries = (0..entry_count)
            .map(|ordinal| {
                let seed = u8::try_from(ordinal).unwrap();
                WorkerV3VerificationEntryCoordinateV1::new(
                    u32::from(seed),
                    format!("logical_{seed}"),
                    format!("export_{seed}"),
                    [seed.wrapping_add(1); 32],
                    [seed.wrapping_add(21); 32],
                    [seed.wrapping_add(41); 32],
                )
                .unwrap()
            })
            .collect();
        WorkerV3VerificationRequestV1::new(
            WorkerV3VerificationFreshChallengeV1::new([1; 32]).unwrap(),
            WorkerV3VerificationRosterIdentityV1::new([2; 32]).unwrap(),
            WorkerV3VerificationPolicyIdentityV1::new([3; 32]).unwrap(),
            WorkerV3VerificationMeasurementIdentityV1::new([4; 32]).unwrap(),
            WorkerV3VerificationFdPayloadDescriptorV1::load_envelope_v2(1, [5; 32]).unwrap(),
            WorkerV3VerificationFdPayloadDescriptorV1::finalized_hsaco(1, [6; 32]).unwrap(),
            entries,
        )
        .unwrap()
    }

    #[test]
    fn exact_twelve_entry_roster_is_the_only_service_handoff() {
        let caller = ServiceCallerPolicyV1::new(1, 2, 3, [2; 32]).unwrap();
        let exact = request(REQUIRED_ROSTER_ENTRIES);
        assert_eq!(validate_roster(&exact, caller), Ok(()));
        let handed_off = service_entries(&exact).unwrap();
        for (index, entry) in handed_off.iter().enumerate() {
            assert_eq!(usize::from(entry.ordinal()), index);
            assert_eq!(
                entry.lineage_identity(),
                *exact.entries()[index].lineage_identity()
            );
            assert_eq!(
                entry.marker_binding_identity(),
                *exact.entries()[index].marker_binding_identity()
            );
            assert_eq!(
                entry.generated_host_contract_identity(),
                *exact.entries()[index].generated_host_contract_identity()
            );
        }
        assert_eq!(
            validate_roster(&request(REQUIRED_ROSTER_ENTRIES - 1), caller),
            Err(ServiceApplicationRejectionV1::RosterMismatch)
        );
    }

    #[test]
    fn corrupted_or_inexact_payload_is_rejected() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"exact-payload").unwrap();
        file.as_file_mut().sync_all().unwrap();
        let expected = sha256(b"exact-payload");
        assert_eq!(
            read_exact_preopened_payload(file.as_file(), 13, 64, expected).unwrap(),
            b"exact-payload"
        );
        assert_eq!(
            read_exact_preopened_payload(file.as_file(), 12, 64, expected),
            Err(ServiceApplicationRejectionV1::PayloadRevalidation)
        );
        assert_eq!(
            read_exact_preopened_payload(file.as_file(), 13, 64, [0xff; 32]),
            Err(ServiceApplicationRejectionV1::PayloadRevalidation)
        );
        assert_eq!(
            read_exact_preopened_payload(file.as_file(), 65, 64, expected),
            Err(ServiceApplicationRejectionV1::PayloadRevalidation)
        );
    }

    #[test]
    fn current_record_corruption_and_provider_failure_are_rejections() {
        assert!(matches!(
            decode_current_record_bytes(&[0_u8; 1]),
            Err(ServiceApplicationRejectionV1::CurrentRecordAuthentication)
        ));
        assert_eq!(
            current_provider_result::<(), _>(Err(io::Error::other("rejected"))),
            Err(ServiceApplicationRejectionV1::CurrentRecordAuthentication)
        );
        assert!(matches!(
            // SAFETY: this deliberately exercises the constructor's zero-token rejection.
            unsafe {
                AuthenticatedCompilerCurrentRecordV1::from_independent_authentication([0; 32])
            },
            Err(ProtectedProviderClaimErrorV1::ZeroTranscript)
        ));
    }

    #[test]
    fn checker_failure_and_provider_substitution_are_rejections() {
        assert_eq!(
            checker_provider_result::<(), _>(Err(io::Error::other("rejected"))),
            Err(ServiceApplicationRejectionV1::CheckerRejected)
        );
        let trust = policy();
        let (mut current, mut checker, signer) = providers();
        current.measurement = [0xb1; 32];
        assert_eq!(
            validate_provider_identities(
                &trust, &current, &checker, &signer, [0xa3; 32], [0xa4; 32],
            ),
            Err(FerricProtectedVerifierServiceConfigErrorV1::CurrentProviderSubstitution)
        );

        current.measurement = [0xa3; 32];
        checker.measurement = [0xb2; 32];
        assert_eq!(
            validate_provider_identities(
                &trust, &current, &checker, &signer, [0xa3; 32], [0xa4; 32],
            ),
            Err(FerricProtectedVerifierServiceConfigErrorV1::CheckerSubstitution)
        );
    }

    #[test]
    fn signer_substitution_failure_and_deadline_do_not_return_a_signature() {
        let trust = policy();
        let (_, _, mut signer) = providers();
        let live = AbsoluteSessionDeadlineV1::after(Duration::from_secs(1)).unwrap();
        let policy_identity = *trust.identity().as_bytes();
        let signature = request_receipt_signature(
            &mut signer,
            [0xa4; 32],
            trust.verifying_key_bytes(),
            b"receipt",
            policy_identity,
            live,
        )
        .unwrap();
        assert_eq!(signature, [0x55; 64]);
        assert_eq!(signer.calls, 1);

        signer.provider = [0xb3; 32];
        assert_eq!(
            request_receipt_signature(
                &mut signer,
                [0xa4; 32],
                trust.verifying_key_bytes(),
                b"receipt",
                policy_identity,
                live,
            ),
            Err(ServiceApplicationRejectionV1::SignerRejected)
        );
        assert_eq!(signer.calls, 1);

        signer.provider = [0xa4; 32];
        signer.fail = true;
        assert_eq!(
            request_receipt_signature(
                &mut signer,
                [0xa4; 32],
                trust.verifying_key_bytes(),
                b"receipt",
                policy_identity,
                live,
            ),
            Err(ServiceApplicationRejectionV1::SignerRejected)
        );
        assert_eq!(signer.calls, 2);

        signer.fail = false;
        let expired =
            AbsoluteSessionDeadlineV1(Instant::now().checked_sub(Duration::from_secs(1)).unwrap());
        assert_eq!(
            request_receipt_signature(
                &mut signer,
                [0xa4; 32],
                trust.verifying_key_bytes(),
                b"receipt",
                policy_identity,
                expired,
            ),
            Err(ServiceApplicationRejectionV1::DeadlineExpired)
        );
        assert_eq!(signer.calls, 2);

        signer.delay = Duration::from_millis(10);
        let overrun = AbsoluteSessionDeadlineV1::after(Duration::from_millis(1)).unwrap();
        assert_eq!(
            request_receipt_signature(
                &mut signer,
                [0xa4; 32],
                trust.verifying_key_bytes(),
                b"receipt",
                policy_identity,
                overrun,
            ),
            Err(ServiceApplicationRejectionV1::DeadlineExpired)
        );
        assert_eq!(signer.calls, 3);
    }

    #[test]
    fn signer_public_key_substitution_is_configuration_rejection() {
        let trust = policy();
        let (current, checker, mut signer) = providers();
        signer.verifying_key = SigningKey::from_bytes(&[0x92; 32])
            .verifying_key()
            .to_bytes();
        assert_eq!(
            validate_provider_identities(
                &trust, &current, &checker, &signer, [0xa3; 32], [0xa4; 32],
            ),
            Err(FerricProtectedVerifierServiceConfigErrorV1::SignerSubstitution)
        );
    }

    #[test]
    fn payload_reader_uses_descriptor_not_stream_position() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"descriptor-relative").unwrap();
        file.as_file_mut().sync_all().unwrap();
        let reader = File::open(file.path()).unwrap();
        let expected = sha256(b"descriptor-relative");
        let first = read_exact_preopened_payload(&reader, 19, 64, expected).unwrap();
        let second = read_exact_preopened_payload(&reader, 19, 64, expected).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn production_surface_has_no_ambient_configuration_or_signing_key() {
        let service = include_str!("service.rs");
        let production_service = service.split("#[cfg(test)]").next().unwrap();
        let durable = include_str!("durable.rs");
        let production_durable = durable.split("#[cfg(test)]").next().unwrap();
        for forbidden in ["SigningKey", "std::env", "env::var", "File::open", "/dev/"] {
            assert!(!production_service.contains(forbidden));
            assert!(!production_durable.contains(forbidden));
        }
        let manifest = include_str!("../Cargo.toml");
        assert!(manifest.contains("rev = \"16da71edd823e0d5c16529bfbbedb4f9dd8e70c6\""));
        assert!(!manifest.contains("branch ="));
    }
}
