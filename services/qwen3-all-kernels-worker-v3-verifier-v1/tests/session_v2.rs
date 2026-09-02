//! Complete positive V2 socket/session proof with real signed compiler and Ferric records.

use std::fs::File;
use std::io::Write;
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ed25519_dalek::{Signer, SigningKey};
use fe2o3_compiler_execution_protocol::{
    CompilerExecutionCurrentRecordAttestationV3, CompilerExecutionCurrentRecordVerificationV3,
    CompilerExecutionExternalAnchorTransactionV1, CompilerExecutionReceiptCarriageV1,
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
    EntropyObjectIdentityV1, FerricProtectedVerifierServiceConfigV1,
    FerricProtectedVerifierServiceOutcomeV1, IndependentCheckerInputV1,
    IndependentCheckerProviderV1, IndependentCheckerVerifiedClaimsV1, LedgerObjectIdentityV1,
    ProtectedCompilerCurrentRecordInputV1, ProtectedCompilerCurrentRecordProviderV1,
    ProtectedLedgerExternalHeadV1, ProtectedLedgerHeadStoreFailureV1, ProtectedLedgerHeadStoreV1,
    ProtectedLedgerKindV1, ProtectedLedgerStorageCapabilityV1, ProtectedReceiptSignerInputV1,
    ProtectedReceiptSignerProviderV1, ServiceCallerPolicyV1,
    run_ferric_protected_verifier_session_v2,
};
use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_receipt::{
    M1AllKernelsProtectedReceiptRequestClaimsV1, M1AllKernelsProtectedReceiptSourcePinV1,
    M1AllKernelsProtectedVerifierReceiptV1, M1AllKernelsProtectedVerifierTrustPolicyV1,
};
use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_verifier_service::M1AllKernelsProtectedVerifierServiceResponseV1;
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
        Ok(self.key.sign(input.signing_bytes()).to_bytes())
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
    M1AllKernelsProtectedVerifierTrustPolicyV1::new(
        SigningKey::from_bytes(&[0x91; 32])
            .verifying_key()
            .to_bytes(),
        [0xa1; 32],
        [0xa2; 32],
    )
    .unwrap()
}

fn request() -> WorkerV3VerificationRequestV1 {
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
        WorkerV3VerificationPolicyIdentityV1::new(*trust_policy().identity().as_bytes()).unwrap(),
        WorkerV3VerificationMeasurementIdentityV1::new([0xa1; 32]).unwrap(),
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
    namespace: [u8; 32],
    kind: ProtectedLedgerKindV1,
    head_identity: [u8; 32],
) -> ProtectedLedgerStorageCapabilityV1 {
    let store = MemoryHeadStore {
        identity: head_identity,
        head: Arc::new(Mutex::new(None)),
    };
    // SAFETY: this test retains the private 0600 file and serialized mock head exclusively.
    unsafe {
        ProtectedLedgerStorageCapabilityV1::provision_new_from_supervisor(
            rustix::io::fcntl_dupfd_cloexec(file, 0).unwrap(),
            ledger_identity(file),
            namespace,
            kind,
            1,
            8,
            Box::new(store),
            head_identity,
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

#[test]
fn full_v2_socket_session_returns_a_valid_ed25519_signed_ferric_response() {
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
        [0xe1; 32],
        ProtectedLedgerKindV1::Replay,
        [0xf1; 32],
    ))
    .unwrap();
    let (entropy, entropy_identity) = entropy();
    let reservations = DurableReservationProviderV2::from_protected_storage(
        provision(
            reservation_file.as_file(),
            [0xe2; 32],
            ProtectedLedgerKindV1::Reservation,
            [0xf2; 32],
        ),
        entropy,
        entropy_identity,
    )
    .unwrap();
    let request = request();
    let caller = ServiceCallerPolicyV1::new(
        std::process::id(),
        rustix::process::getuid().as_raw(),
        rustix::process::getgid().as_raw(),
        *request.roster_identity().as_bytes(),
    )
    .unwrap();
    let signing_key = SigningKey::from_bytes(&[0x91; 32]);
    let mut config = FerricProtectedVerifierServiceConfigV1::new(
        caller,
        trust_policy(),
        replay,
        reservations,
        AcceptingCurrent {
            measurement: [0xa3; 32],
        },
        AcceptingChecker {
            measurement: [0xa2; 32],
        },
        SigningProvider {
            key: signing_key,
            identity: [0xa4; 32],
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
    assert!(!response.grants_authority());
}
