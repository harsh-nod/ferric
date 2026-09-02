//! Complete positive V2 socket/session proof with real signed compiler and Ferric records.

use std::fs::File;
use std::io::Write;
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ed25519_dalek::{Signer, SigningKey};
use fe2o3_compiler_execution_protocol::{
    CompilerExecutionAttestationChallengeV1, CompilerExecutionAttestationReceiptV1,
    CompilerExecutionAttestationRequestV1, CompilerExecutionCurrentRecordAttestationV3,
    CompilerExecutionCurrentRecordVerificationV3, CompilerExecutionExternalAnchorTransactionV1,
    CompilerExecutionIssuerMeasurementV1, CompilerExecutionIssuerPolicyV1,
    CompilerExecutionReceiptCarriageV1, CompilerExecutionReceiptPublicationAckV1,
    CompilerExecutionReceiptPublicationV1,
};
use fe2o3_external_anchor_protocol::{
    AnchorPositionV1, AnchorTransitionReceiptV1, AnchoredStateV1, CallerNonceV1, HashChainHeadV1,
    PinnedAnchorKeyV1, UnsignedAnchorObservationV1,
};
use fe2o3_runtime_protocol::WorkerV3LoadEnvelopeWireV2;
use fe2o3_worker_v3_verification_client::{
    WorkerV3VerificationBeginOutcomeV2 as ClientBeginOutcomeV2, WorkerV3VerificationClientV2,
    WorkerV3VerificationPayloadSnapshotsV1,
};
use fe2o3_worker_v3_verification_protocol::{
    WorkerV3VerificationEntryCoordinateV1, WorkerV3VerificationFdPayloadDescriptorV1,
    WorkerV3VerificationFreshChallengeV1, WorkerV3VerificationMeasurementIdentityV1,
    WorkerV3VerificationPolicyIdentityV1, WorkerV3VerificationRequestV1,
    WorkerV3VerificationRosterIdentityV1, WorkerV3VerificationTerminalDispositionV2,
};
use fe2o3_worker_v3_verification_service::prepare_worker_v3_verification_receiver_v1;
use ferric_qwen3_all_kernels_worker_v3_verifier_service_v1::{
    AuthenticatedCompilerCurrentRecordV1, DurableReplayGuardV1, DurableReservationProviderV2,
    EntropyObjectIdentityV1, FerricProtectedVerifierServiceConfigErrorV1,
    FerricProtectedVerifierServiceConfigV1, FerricProtectedVerifierServiceFailureV1,
    FerricProtectedVerifierServiceOutcomeV1, IndependentCheckerInputV1,
    IndependentCheckerProviderV1, IndependentCheckerVerifiedClaimsV1, LedgerObjectIdentityV1,
    ProtectedCompilerCurrentRecordInputV1, ProtectedCompilerCurrentRecordProviderV1,
    ProtectedLedgerExternalHeadV1, ProtectedLedgerHeadStoreFailureV1, ProtectedLedgerHeadStoreV1,
    ProtectedLedgerKindV1, ProtectedLedgerReplacementAuthorizationV1,
    ProtectedLedgerStorageCapabilityV1, ProtectedPolicyRevocationV1, ProtectedReceiptSignerInputV1,
    ProtectedReceiptSignerProviderV1, ServiceApplicationRejectionV1, ServiceCallerPolicyV1,
    run_ferric_protected_verifier_session_v2,
};
use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_receipt::{
    M1AllKernelsProtectedReceiptRequestClaimsV1, M1AllKernelsProtectedReceiptSourcePinV1,
    M1AllKernelsProtectedVerifierReceiptV1, M1AllKernelsProtectedVerifierTrustPolicyV1,
};
use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_verifier_service::{
    M1AllKernelsProtectedVerifierServiceEntryV1, M1AllKernelsProtectedVerifierServiceRequestV1,
    M1AllKernelsProtectedVerifierServiceResponseV1,
};
use rustix::fs::{MemfdFlags, Mode, OFlags, SealFlags};
use rustix::net::{AddressFamily, SocketFlags, SocketType, socketpair};
use sha2::{Digest, Sha256};

const ENVELOPE: &[u8] = include_bytes!("fixtures/valid-envelope-v2.bin");
const HSACO: &[u8] = include_bytes!("fixtures/valid-finalized-hsaco.bin");
const ENTRY_RECEIPT: &[u8] = include_bytes!("fixtures/valid-protected-receipt-v1.bin");
const SEALS: SealFlags = SealFlags::WRITE
    .union(SealFlags::GROW)
    .union(SealFlags::SHRINK)
    .union(SealFlags::SEAL);

#[derive(Clone)]
struct MemoryHeadStore {
    identity: [u8; 32],
    head: Arc<Mutex<Option<ProtectedLedgerExternalHeadV1>>>,
}

// Test-only serialization models the unsafe contract without satisfying deployment closure.
unsafe impl ProtectedLedgerHeadStoreV1 for MemoryHeadStore {
    fn provider_identity(&self) -> [u8; 32] {
        self.identity
    }

    fn load_head(
        &mut self,
    ) -> Result<Option<ProtectedLedgerExternalHeadV1>, ProtectedLedgerHeadStoreFailureV1> {
        Ok(*self.head.lock().unwrap())
    }

    fn initialize_head(
        &mut self,
        initial: ProtectedLedgerExternalHeadV1,
    ) -> Result<bool, ProtectedLedgerHeadStoreFailureV1> {
        let mut head = self.head.lock().unwrap();
        if head.is_some() {
            return Ok(false);
        }
        *head = Some(initial);
        Ok(true)
    }

    fn compare_exchange_head(
        &mut self,
        current: ProtectedLedgerExternalHeadV1,
        next: ProtectedLedgerExternalHeadV1,
    ) -> Result<bool, ProtectedLedgerHeadStoreFailureV1> {
        let mut head = self.head.lock().unwrap();
        if *head != Some(current) {
            return Ok(false);
        }
        *head = Some(next);
        Ok(true)
    }
}

struct AcceptingCurrent {
    measurement: [u8; 32],
}

// Test-only acceptance is constrained to the generic service's already-canonical current frame.
unsafe impl ProtectedCompilerCurrentRecordProviderV1 for AcceptingCurrent {
    type Error = std::io::Error;

    fn measurement_identity(&self) -> [u8; 32] {
        self.measurement
    }

    fn authenticate_current_record(
        &mut self,
        input: ProtectedCompilerCurrentRecordInputV1<'_>,
    ) -> Result<AuthenticatedCompilerCurrentRecordV1, Self::Error> {
        assert_eq!(
            input.current_record_bytes(),
            input.current_record().encode_canonical()
        );
        assert_eq!(
            input.envelope().encode_canonical().unwrap(),
            input.envelope().encode_canonical().unwrap()
        );
        // SAFETY: this test mock has checked the exact canonical objects supplied by the service.
        Ok(unsafe {
            AuthenticatedCompilerCurrentRecordV1::from_independent_authentication([0xc1; 32])
                .unwrap()
        })
    }
}

struct AcceptingChecker {
    measurement: [u8; 32],
}

// Test-only acceptance constructs exact request/artifact/entry claims for the exercised handoff.
unsafe impl IndependentCheckerProviderV1 for AcceptingChecker {
    type Error = std::io::Error;

    fn measurement_identity(&self) -> [u8; 32] {
        self.measurement
    }

    fn verify_all_kernels(
        &mut self,
        input: IndependentCheckerInputV1<'_>,
    ) -> Result<IndependentCheckerVerifiedClaimsV1, Self::Error> {
        assert_eq!(input.envelope_bytes(), ENVELOPE);
        assert_eq!(input.hsaco_bytes(), HSACO);
        assert_ne!(
            input.current_authentication().transcript_identity(),
            [0; 32]
        );
        let source = M1AllKernelsProtectedReceiptSourcePinV1::new(
            [0xd1; 32], 1, [0xd2; 32], 2, [0xd3; 32], 3,
        )
        .unwrap();
        let claims = M1AllKernelsProtectedReceiptRequestClaimsV1::new(
            *input.request().challenge().as_bytes(),
            *input.request().roster_identity().as_bytes(),
            [0xd4; 32],
            [0xd5; 32],
            source,
            [0xd6; 32],
            [0xd7; 32],
            [0xd8; 32],
            sha256(input.hsaco_bytes()),
            input.hsaco_bytes().len() as u64,
        )
        .unwrap();
        let fixture =
            M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(ENTRY_RECEIPT).unwrap();
        let entries = *fixture.entries();
        // SAFETY: this test checker binds all asserted values to the exact handoff above.
        Ok(unsafe {
            IndependentCheckerVerifiedClaimsV1::from_independent_checker(
                claims, entries, [0xc2; 32],
            )
            .unwrap()
        })
    }
}

struct SigningProvider {
    key: SigningKey,
    identity: [u8; 32],
    delay: Duration,
    overrun_deadline: bool,
    sign_wrong_message: bool,
    called: Arc<AtomicBool>,
}

impl ProtectedReceiptSignerProviderV1 for SigningProvider {
    type Error = std::io::Error;

    fn verifying_key_bytes(&self) -> [u8; 32] {
        self.key.verifying_key().to_bytes()
    }

    fn provider_identity(&self) -> [u8; 32] {
        self.identity
    }

    fn sign_receipt(
        &mut self,
        input: ProtectedReceiptSignerInputV1<'_>,
    ) -> Result<[u8; 64], Self::Error> {
        self.called.store(true, Ordering::SeqCst);
        let delay = if self.overrun_deadline {
            input.deadline().remaining().unwrap_or_default() + Duration::from_millis(20)
        } else {
            self.delay
        };
        thread::sleep(delay);
        let message = if self.sign_wrong_message {
            b"wrong-message".as_slice()
        } else {
            input.signing_bytes()
        };
        Ok(self.key.sign(message).to_bytes())
    }
}

#[derive(Clone)]
struct CurrentFixture {
    issuer: SigningKey,
    anchor: SigningKey,
    carriage: CompilerExecutionReceiptCarriageV1,
}

impl CurrentFixture {
    fn from_envelope() -> Self {
        let envelope = WorkerV3LoadEnvelopeWireV2::decode_canonical(ENVELOPE).unwrap();
        Self {
            issuer: SigningKey::from_bytes(&[0x51; 32]),
            anchor: SigningKey::from_bytes(&[0x52; 32]),
            carriage: envelope.compiler_execution_receipt().clone(),
        }
    }

    fn hostile_for_same_subject() -> Self {
        let envelope = WorkerV3LoadEnvelopeWireV2::decode_canonical(ENVELOPE).unwrap();
        let subject = envelope
            .reconstructed_compiler_execution_subject_v1()
            .unwrap();
        let issuer = SigningKey::from_bytes(&[0x71; 32]);
        let anchor = SigningKey::from_bytes(&[0x72; 32]);
        let policy = CompilerExecutionIssuerPolicyV1::new(
            1,
            CompilerExecutionIssuerMeasurementV1::new([0x73; 32], 12_345).unwrap(),
            CompilerExecutionIssuerMeasurementV1::new([0x74; 32], 67_890).unwrap(),
            issuer.verifying_key().to_bytes(),
            anchor.verifying_key().to_bytes(),
        )
        .unwrap();
        let challenge =
            CompilerExecutionAttestationChallengeV1::new(&policy, &subject, [0x75; 32], 1, [0; 32])
                .unwrap();
        let request = CompilerExecutionAttestationRequestV1::new(challenge, subject).unwrap();
        let receipt =
            CompilerExecutionAttestationReceiptV1::issue(&policy, &request, &issuer).unwrap();
        let publication =
            CompilerExecutionReceiptPublicationV1::new([0x76; 32], [0x77; 32], receipt).unwrap();
        let acknowledgment =
            CompilerExecutionReceiptPublicationAckV1::new(&publication, [0x78; 32]).unwrap();
        let carriage =
            CompilerExecutionReceiptCarriageV1::new(policy, request, publication, acknowledgment)
                .unwrap();
        Self {
            issuer,
            anchor,
            carriage,
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
        let pending = AnchoredStateV1::from_local_state(0, HashChainHeadV1::from_bytes([0; 32]))
            .prepare(transaction.external_anchor_digest(), &anchor_key)
            .unwrap()
            .begin_advance(CallerNonceV1::from_bytes([0x67; 32]), &anchor_key)
            .unwrap();
        let unsigned = UnsignedAnchorObservationV1::from_challenge(
            pending.challenge(),
            AnchorPositionV1::Proposed,
        );
        let signature = self.anchor.sign(&unsigned.signing_bytes()).to_bytes();
        let commit = AnchorTransitionReceiptV1::new(
            pending.challenge().clone(),
            &unsigned.attach_signature(signature),
            &anchor_key,
        )
        .unwrap();
        let current_challenge =
            CompilerExecutionCurrentRecordVerificationV3::external_anchor_currentness_challenge(
                &self.carriage,
                &commit,
                challenge,
            )
            .unwrap();
        let unsigned = UnsignedAnchorObservationV1::from_challenge(
            &current_challenge,
            AnchorPositionV1::Proposed,
        );
        let signature = self.anchor.sign(&unsigned.signing_bytes()).to_bytes();
        let current = AnchorTransitionReceiptV1::new(
            current_challenge,
            &unsigned.attach_signature(signature),
            &anchor_key,
        )
        .unwrap();
        let verification = CompilerExecutionCurrentRecordVerificationV3::new(
            &self.carriage,
            commit,
            current,
            challenge,
            [0x91; 32],
            [0x92; 32],
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

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn trust_policy() -> M1AllKernelsProtectedVerifierTrustPolicyV1 {
    trust_policy_with_signing_seed(0x91)
}

fn trust_policy_with_signing_seed(seed: u8) -> M1AllKernelsProtectedVerifierTrustPolicyV1 {
    M1AllKernelsProtectedVerifierTrustPolicyV1::new(
        SigningKey::from_bytes(&[seed; 32])
            .verifying_key()
            .to_bytes(),
        [0xa1; 32],
        [0xa2; 32],
    )
    .unwrap()
}

fn request() -> WorkerV3VerificationRequestV1 {
    request_for_trust_policy(&trust_policy())
}

fn request_for_trust_policy(
    trust: &M1AllKernelsProtectedVerifierTrustPolicyV1,
) -> WorkerV3VerificationRequestV1 {
    let fixture = M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(ENTRY_RECEIPT).unwrap();
    let entries = fixture
        .entries()
        .iter()
        .map(|entry| {
            WorkerV3VerificationEntryCoordinateV1::new(
                u32::from(entry.ordinal()),
                format!("logical_{}", entry.ordinal()),
                format!("export_{}", entry.ordinal()),
                entry.lineage_identity(),
                entry.marker_binding_identity(),
                entry.generated_host_contract_identity(),
            )
            .unwrap()
        })
        .collect();
    WorkerV3VerificationRequestV1::new(
        WorkerV3VerificationFreshChallengeV1::new([0x11; 32]).unwrap(),
        WorkerV3VerificationRosterIdentityV1::new([0x22; 32]).unwrap(),
        WorkerV3VerificationPolicyIdentityV1::new(*trust.identity().as_bytes()).unwrap(),
        WorkerV3VerificationMeasurementIdentityV1::new(trust.verifier_measurement_sha256())
            .unwrap(),
        WorkerV3VerificationFdPayloadDescriptorV1::load_envelope_v2(
            ENVELOPE.len() as u64,
            sha256(ENVELOPE),
        )
        .unwrap(),
        WorkerV3VerificationFdPayloadDescriptorV1::finalized_hsaco(
            HSACO.len() as u64,
            sha256(HSACO),
        )
        .unwrap(),
        entries,
    )
    .unwrap()
}

fn sealed_read_only(bytes: &[u8]) -> File {
    let descriptor = rustix::fs::memfd_create(
        "ferric-v2-service-test",
        MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
    )
    .unwrap();
    let mut writer = File::from(descriptor);
    rustix::fs::fchmod(&writer, Mode::RUSR).unwrap();
    writer.write_all(bytes).unwrap();
    writer.flush().unwrap();
    rustix::fs::fcntl_add_seals(&writer, SEALS).unwrap();
    let path = format!("/proc/self/fd/{}", writer.as_raw_fd());
    let retained = File::from(
        rustix::fs::open(path, OFlags::RDONLY | OFlags::CLOEXEC, Mode::empty()).unwrap(),
    );
    drop(writer);
    retained
}

fn snapshots(request: &WorkerV3VerificationRequestV1) -> WorkerV3VerificationPayloadSnapshotsV1 {
    WorkerV3VerificationPayloadSnapshotsV1::admit(
        request,
        vec![
            sealed_read_only(ENVELOPE).into(),
            sealed_read_only(HSACO).into(),
        ],
    )
    .unwrap()
}

fn ledger_identity(file: &impl AsFd) -> LedgerObjectIdentityV1 {
    let stat = rustix::fs::fstat(file).unwrap();
    LedgerObjectIdentityV1::new(
        stat.st_dev,
        stat.st_ino,
        stat.st_mode,
        stat.st_uid,
        stat.st_gid,
    )
}

fn provision(
    file: &File,
    policy: WorkerV3VerificationPolicyIdentityV1,
    kind: ProtectedLedgerKindV1,
    head_identity: [u8; 32],
) -> ProtectedLedgerStorageCapabilityV1 {
    let store = MemoryHeadStore {
        identity: head_identity,
        head: Arc::new(Mutex::new(None)),
    };
    // SAFETY: this test retains the private 0600 file and serialized mock head exclusively.
    unsafe {
        ProtectedLedgerStorageCapabilityV1::provision_initial_from_supervisor(
            rustix::io::fcntl_dupfd_cloexec(file, 0).unwrap(),
            ledger_identity(file),
            policy,
            kind,
            8,
            Box::new(store),
            head_identity,
        )
    }
    .unwrap()
}

fn provision_replacement(
    file: &File,
    policy: WorkerV3VerificationPolicyIdentityV1,
    kind: ProtectedLedgerKindV1,
    head_identity: [u8; 32],
    authorization: ProtectedLedgerReplacementAuthorizationV1,
) -> ProtectedLedgerStorageCapabilityV1 {
    let store = MemoryHeadStore {
        identity: head_identity,
        head: Arc::new(Mutex::new(None)),
    };
    // SAFETY: this models a supervisor-provisioned new-policy object after revocation.
    unsafe {
        ProtectedLedgerStorageCapabilityV1::provision_replacement_from_supervisor(
            rustix::io::fcntl_dupfd_cloexec(file, 0).unwrap(),
            ledger_identity(file),
            policy,
            kind,
            8,
            Box::new(store),
            head_identity,
            authorization,
        )
    }
    .unwrap()
}

fn entropy() -> (OwnedFd, EntropyObjectIdentityV1) {
    let file = rustix::fs::open(
        "/dev/urandom",
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .unwrap();
    let stat = rustix::fs::fstat(&file).unwrap();
    let identity = EntropyObjectIdentityV1::new(
        stat.st_dev,
        stat.st_ino,
        stat.st_rdev,
        stat.st_mode,
        stat.st_uid,
        stat.st_gid,
    );
    (file, identity)
}

type TestServiceConfig =
    FerricProtectedVerifierServiceConfigV1<AcceptingCurrent, AcceptingChecker, SigningProvider>;

fn initial_config(
    request: &WorkerV3VerificationRequestV1,
    trust: M1AllKernelsProtectedVerifierTrustPolicyV1,
    ledger_policy: WorkerV3VerificationPolicyIdentityV1,
    signing_seed: u8,
    signer_delay: Duration,
    signer_overrun: bool,
    sign_wrong_message: bool,
    timeout: Duration,
) -> Result<
    (
        TestServiceConfig,
        tempfile::NamedTempFile,
        tempfile::NamedTempFile,
        Arc<AtomicBool>,
    ),
    FerricProtectedVerifierServiceConfigErrorV1,
> {
    let replay_file = tempfile::NamedTempFile::new().unwrap();
    let reservation_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::set_permissions(replay_file.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::set_permissions(
        reservation_file.path(),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let replay = DurableReplayGuardV1::from_protected_storage(provision(
        replay_file.as_file(),
        ledger_policy,
        ProtectedLedgerKindV1::Replay,
        [0xf1; 32],
    ))
    .unwrap();
    let (entropy, entropy_identity) = entropy();
    let reservations = DurableReservationProviderV2::from_protected_storage(
        provision(
            reservation_file.as_file(),
            ledger_policy,
            ProtectedLedgerKindV1::Reservation,
            [0xf2; 32],
        ),
        entropy,
        entropy_identity,
    )
    .unwrap();
    let caller = ServiceCallerPolicyV1::new(
        std::process::id(),
        rustix::process::getuid().as_raw(),
        rustix::process::getgid().as_raw(),
        *request.roster_identity().as_bytes(),
    )
    .unwrap();
    let checker_measurement = trust.checker_measurement_sha256();
    let signer_called = Arc::new(AtomicBool::new(false));
    let config = FerricProtectedVerifierServiceConfigV1::new(
        caller,
        trust,
        replay,
        reservations,
        AcceptingCurrent {
            measurement: [0xa3; 32],
        },
        AcceptingChecker {
            measurement: checker_measurement,
        },
        SigningProvider {
            key: SigningKey::from_bytes(&[signing_seed; 32]),
            identity: [0xa4; 32],
            delay: signer_delay,
            overrun_deadline: signer_overrun,
            sign_wrong_message,
            called: Arc::clone(&signer_called),
        },
        [0xa3; 32],
        [0xa4; 32],
        timeout,
    )?;
    Ok((config, replay_file, reservation_file, signer_called))
}

#[test]
fn full_v2_socket_session_returns_a_valid_ed25519_signed_ferric_response() {
    let request = request();
    let (mut config, _replay_file, _reservation_file, _signer_called) = initial_config(
        &request,
        trust_policy(),
        request.policy_identity(),
        0x91,
        Duration::ZERO,
        false,
        false,
        Duration::from_secs(5),
    )
    .unwrap();
    let (service, client) = socketpair(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .unwrap();
    prepare_worker_v3_verification_receiver_v1(&service).unwrap();
    let client_request = request.clone();
    let client_thread = thread::spawn(move || {
        let begin = WorkerV3VerificationClientV2::admit(client, Duration::from_secs(5))
            .unwrap()
            .begin(client_request.clone(), snapshots(&client_request))
            .unwrap();
        let ClientBeginOutcomeV2::Reserved(begin) = begin else {
            panic!("valid Begin was rejected");
        };
        let (challenge, pending) = begin.into_parts();
        let fixture = CurrentFixture::from_envelope();
        let (verification, attestation) = fixture.records(challenge.into_bytes());
        pending
            .submit_current_record(
                *verification.canonical_bytes(),
                *attestation.canonical_bytes(),
            )
            .unwrap()
    });
    let outcome = run_ferric_protected_verifier_session_v2(service, &mut config).unwrap();
    assert!(matches!(
        outcome,
        FerricProtectedVerifierServiceOutcomeV1::Completed(_)
    ));
    let terminal = client_thread.join().unwrap();
    assert_eq!(
        terminal.disposition(),
        WorkerV3VerificationTerminalDispositionV2::ApplicationResponse
    );
    let response = M1AllKernelsProtectedVerifierServiceResponseV1::decode(
        terminal.application_response_bytes(),
    )
    .unwrap();
    trust_policy()
        .authenticate_canonical(response.receipt().encode_canonical())
        .unwrap();
    let correlated_entries: [M1AllKernelsProtectedVerifierServiceEntryV1; 12] = request
        .entries()
        .iter()
        .map(|entry| {
            M1AllKernelsProtectedVerifierServiceEntryV1::new(
                u16::try_from(entry.ordinal()).unwrap(),
                *entry.lineage_identity(),
                *entry.marker_binding_identity(),
                *entry.generated_host_contract_identity(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let correlated_request = M1AllKernelsProtectedVerifierServiceRequestV1::new(
        response.trust_policy_identity(),
        *response.receipt().request_claims(),
        *response.receipt().compiler_claims(),
        correlated_entries,
    )
    .unwrap();
    assert_eq!(response.request_identity(), correlated_request.identity());
    assert_eq!(response.sequence(), correlated_request.expected_sequence());
    assert_eq!(
        response.current_rollback_anchor(),
        correlated_request.expected_current_rollback_anchor()
    );
    assert!(!response.grants_authority());
}

#[test]
fn safe_configuration_rejects_a_ledger_bound_to_another_policy() {
    let request = request();
    let result = initial_config(
        &request,
        trust_policy(),
        WorkerV3VerificationPolicyIdentityV1::new([0xee; 32]).unwrap(),
        0x91,
        Duration::ZERO,
        false,
        false,
        Duration::from_secs(5),
    );
    assert!(matches!(
        result,
        Err(FerricProtectedVerifierServiceConfigErrorV1::LedgerPolicyMismatch)
    ));
}

#[test]
fn independently_valid_current_for_another_carriage_is_rejected() {
    let request = request();
    let (mut config, _replay_file, _reservation_file, _signer_called) = initial_config(
        &request,
        trust_policy(),
        request.policy_identity(),
        0x91,
        Duration::ZERO,
        false,
        false,
        Duration::from_secs(5),
    )
    .unwrap();
    let (service, client) = socketpair(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .unwrap();
    prepare_worker_v3_verification_receiver_v1(&service).unwrap();
    let client_thread = thread::spawn(move || {
        let begin = WorkerV3VerificationClientV2::admit(client, Duration::from_secs(5))
            .unwrap()
            .begin(request.clone(), snapshots(&request))
            .unwrap();
        let ClientBeginOutcomeV2::Reserved(begin) = begin else {
            panic!("valid Begin was rejected");
        };
        let (challenge, pending) = begin.into_parts();
        let (verification, attestation) =
            CurrentFixture::hostile_for_same_subject().records(challenge.into_bytes());
        pending
            .submit_current_record(
                *verification.canonical_bytes(),
                *attestation.canonical_bytes(),
            )
            .unwrap()
    });
    let outcome = run_ferric_protected_verifier_session_v2(service, &mut config).unwrap();
    assert!(matches!(
        outcome,
        FerricProtectedVerifierServiceOutcomeV1::Rejected {
            reason: ServiceApplicationRejectionV1::CurrentRecordAssociation,
            ..
        }
    ));
    assert_eq!(
        client_thread.join().unwrap().disposition(),
        WorkerV3VerificationTerminalDispositionV2::Rejected
    );
}

#[test]
fn signer_signature_over_the_wrong_message_is_terminally_rejected() {
    let request = request();
    let (mut config, _replay_file, _reservation_file, _signer_called) = initial_config(
        &request,
        trust_policy(),
        request.policy_identity(),
        0x91,
        Duration::ZERO,
        false,
        true,
        Duration::from_secs(5),
    )
    .unwrap();
    let (service, client) = socketpair(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .unwrap();
    prepare_worker_v3_verification_receiver_v1(&service).unwrap();
    let client_thread = thread::spawn(move || {
        let begin = WorkerV3VerificationClientV2::admit(client, Duration::from_secs(5))
            .unwrap()
            .begin(request.clone(), snapshots(&request))
            .unwrap();
        let ClientBeginOutcomeV2::Reserved(begin) = begin else {
            panic!("valid Begin was rejected");
        };
        let (challenge, pending) = begin.into_parts();
        let (verification, attestation) =
            CurrentFixture::from_envelope().records(challenge.into_bytes());
        pending
            .submit_current_record(
                *verification.canonical_bytes(),
                *attestation.canonical_bytes(),
            )
            .unwrap()
    });
    let outcome = run_ferric_protected_verifier_session_v2(service, &mut config).unwrap();
    assert!(matches!(
        outcome,
        FerricProtectedVerifierServiceOutcomeV1::Rejected {
            reason: ServiceApplicationRejectionV1::SignatureRejected,
            ..
        }
    ));
    assert_eq!(
        client_thread.join().unwrap().disposition(),
        WorkerV3VerificationTerminalDispositionV2::Rejected
    );
}

#[test]
fn full_terminal_signer_overrun_never_emits_an_application_response() {
    let request = request();
    let (mut config, _replay_file, _reservation_file, signer_called) = initial_config(
        &request,
        trust_policy(),
        request.policy_identity(),
        0x91,
        Duration::ZERO,
        true,
        false,
        Duration::from_secs(2),
    )
    .unwrap();
    let (service, client) = socketpair(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .unwrap();
    prepare_worker_v3_verification_receiver_v1(&service).unwrap();
    let client_thread = thread::spawn(move || {
        let begin = WorkerV3VerificationClientV2::admit(client, Duration::from_secs(2))?
            .begin(request.clone(), snapshots(&request))?;
        let ClientBeginOutcomeV2::Reserved(begin) = begin else {
            panic!("valid Begin was rejected before the signer phase");
        };
        let (challenge, pending) = begin.into_parts();
        let (verification, attestation) =
            CurrentFixture::from_envelope().records(challenge.into_bytes());
        pending.submit_current_record(
            *verification.canonical_bytes(),
            *attestation.canonical_bytes(),
        )
    });
    let Err(failure) = run_ferric_protected_verifier_session_v2(service, &mut config) else {
        panic!("an overrun session emitted a terminal disposition");
    };
    assert!(signer_called.load(Ordering::SeqCst));
    assert!(matches!(
        failure,
        FerricProtectedVerifierServiceFailureV1::ReadyRejectionSend {
            reason: ServiceApplicationRejectionV1::DeadlineExpired,
            ..
        }
    ));
    drop(failure);
    assert!(client_thread.join().unwrap().is_err());
}

#[test]
fn captured_old_begin_is_rejected_after_new_policy_replacement() {
    let old_trust = trust_policy_with_signing_seed(0x91);
    let new_trust = trust_policy_with_signing_seed(0x92);
    let captured_request = request_for_trust_policy(&old_trust);
    let old_policy = captured_request.policy_identity();
    let new_policy =
        WorkerV3VerificationPolicyIdentityV1::new(*new_trust.identity().as_bytes()).unwrap();
    // SAFETY: this test models completed global old-policy revocation.
    let revocation = unsafe {
        ProtectedPolicyRevocationV1::from_supervisor_revocation(old_policy, new_policy).unwrap()
    };
    let (replay_authorization, reservation_authorization) = revocation.into_ledger_authorizations();
    let replay_file = tempfile::NamedTempFile::new().unwrap();
    let reservation_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::set_permissions(replay_file.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::set_permissions(
        reservation_file.path(),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let replay = DurableReplayGuardV1::from_protected_storage(provision_replacement(
        replay_file.as_file(),
        new_policy,
        ProtectedLedgerKindV1::Replay,
        [0xe1; 32],
        replay_authorization,
    ))
    .unwrap();
    let (entropy, entropy_identity) = entropy();
    let reservations = DurableReservationProviderV2::from_protected_storage(
        provision_replacement(
            reservation_file.as_file(),
            new_policy,
            ProtectedLedgerKindV1::Reservation,
            [0xe2; 32],
            reservation_authorization,
        ),
        entropy,
        entropy_identity,
    )
    .unwrap();
    let caller = ServiceCallerPolicyV1::new(
        std::process::id(),
        rustix::process::getuid().as_raw(),
        rustix::process::getgid().as_raw(),
        *captured_request.roster_identity().as_bytes(),
    )
    .unwrap();
    let mut config = FerricProtectedVerifierServiceConfigV1::new(
        caller,
        new_trust,
        replay,
        reservations,
        AcceptingCurrent {
            measurement: [0xa3; 32],
        },
        AcceptingChecker {
            measurement: [0xa2; 32],
        },
        SigningProvider {
            key: SigningKey::from_bytes(&[0x92; 32]),
            identity: [0xa4; 32],
            delay: Duration::ZERO,
            overrun_deadline: false,
            sign_wrong_message: false,
            called: Arc::new(AtomicBool::new(false)),
        },
        [0xa3; 32],
        [0xa4; 32],
        Duration::from_secs(5),
    )
    .unwrap();
    let (service, client) = socketpair(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .unwrap();
    prepare_worker_v3_verification_receiver_v1(&service).unwrap();
    let client_thread = thread::spawn(move || {
        WorkerV3VerificationClientV2::admit(client, Duration::from_secs(5))
            .unwrap()
            .begin(captured_request.clone(), snapshots(&captured_request))
            .unwrap()
    });
    assert!(matches!(
        run_ferric_protected_verifier_session_v2(service, &mut config).unwrap(),
        FerricProtectedVerifierServiceOutcomeV1::BeginRejected(_)
    ));
    assert!(matches!(
        client_thread.join().unwrap(),
        ClientBeginOutcomeV2::Rejected(_)
    ));
}
