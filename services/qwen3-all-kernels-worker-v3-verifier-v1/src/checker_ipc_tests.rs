//! Protocol fixtures are not theorem, compiler, or hardware evidence.

use std::thread;
use std::time::Duration;

use fe2o3_runtime_protocol::WorkerV3LoadEnvelopeWireV2;
use fe2o3_worker_v3_verification_protocol::{
    WorkerV3VerificationEntryCoordinateV1, WorkerV3VerificationFdPayloadDescriptorV1,
    WorkerV3VerificationFreshChallengeV1, WorkerV3VerificationMeasurementIdentityV1,
    WorkerV3VerificationPolicyIdentityV1, WorkerV3VerificationRosterIdentityV1,
};
use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_receipt::{
    M1AllKernelsProtectedReceiptRequestClaimsV1, M1AllKernelsProtectedVerifierReceiptV1,
};
use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_verifier_service::M1AllKernelsProtectedVerifierServiceEntryV1;
use rustix::net::{AddressFamily, SocketFlags, SocketType, socketpair};

use super::*;
use crate::AuthenticatedCompilerCurrentRecordV1;

const ENVELOPE: &[u8] = include_bytes!("../tests/fixtures/valid-envelope-v2.bin");
const HSACO: &[u8] = include_bytes!("../tests/fixtures/valid-finalized-hsaco.bin");
const RECEIPT: &[u8] = include_bytes!("../tests/fixtures/valid-protected-receipt-v1.bin");

struct Fixture {
    begin: WorkerV3VerificationRequestV1,
    envelope: WorkerV3LoadEnvelopeWireV2,
    current: AuthenticatedCompilerCurrentRecordV1,
    receipt: M1AllKernelsProtectedVerifierReceiptV1,
    context: IndependentCheckerContextV1,
}
impl Fixture {
    fn new() -> Self {
        let receipt = M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(RECEIPT).unwrap();
        let context = IndependentCheckerContextV1::new(
            independent_checker_protocol_identity_v1(),
            receipt.checker_measurement_sha256(),
            *receipt.trust_policy_identity().as_bytes(),
            [0xa1; 32],
        )
        .unwrap();
        let begin = WorkerV3VerificationRequestV1::new(
            WorkerV3VerificationFreshChallengeV1::new(
                receipt.request_claims().challenge_identity(),
            )
            .unwrap(),
            WorkerV3VerificationRosterIdentityV1::new(receipt.request_claims().roster_identity())
                .unwrap(),
            WorkerV3VerificationPolicyIdentityV1::new(context.policy).unwrap(),
            WorkerV3VerificationMeasurementIdentityV1::new(receipt.verifier_measurement_sha256())
                .unwrap(),
            WorkerV3VerificationFdPayloadDescriptorV1::load_envelope_v2(
                ENVELOPE.len() as u64,
                hash(&[ENVELOPE]),
            )
            .unwrap(),
            WorkerV3VerificationFdPayloadDescriptorV1::finalized_hsaco(
                HSACO.len() as u64,
                hash(&[HSACO]),
            )
            .unwrap(),
            receipt
                .entries()
                .iter()
                .map(|entry| {
                    WorkerV3VerificationEntryCoordinateV1::new(
                        u32::from(entry.ordinal()),
                        format!("fixture_{}", entry.ordinal()),
                        format!("export_{}", entry.ordinal()),
                        entry.lineage_identity(),
                        entry.marker_binding_identity(),
                        entry.generated_host_contract_identity(),
                    )
                    .unwrap()
                })
                .collect(),
        )
        .unwrap();
        // SAFETY: inert test-only transcript, never used to publish/load/launch anything.
        let current = unsafe {
            AuthenticatedCompilerCurrentRecordV1::from_independent_authentication([0xa2; 32])
        }
        .unwrap();
        Self {
            begin,
            envelope: WorkerV3LoadEnvelopeWireV2::decode_canonical(ENVELOPE).unwrap(),
            current,
            receipt,
            context,
        }
    }
    fn input(&self, deadline: AbsoluteSessionDeadlineV1) -> IndependentCheckerInputV1<'_> {
        crate::service::checker_input_for_test(
            &self.begin,
            ENVELOPE,
            &self.envelope,
            HSACO,
            self.receipt.compiler_claims(),
            &self.current,
            deadline,
        )
    }
    fn request(&self, sequence: u64) -> IndependentCheckerRequestV1 {
        IndependentCheckerRequestV1::new(self.context, sequence, &self.input(deadline())).unwrap()
    }
    fn response(&self, request: &IndependentCheckerRequestV1) -> IndependentCheckerResponseV1 {
        let old = self.receipt.request_claims();
        let claims = M1AllKernelsProtectedReceiptRequestClaimsV1::new(
            *request.begin.challenge().as_bytes(),
            *request.begin.roster_identity().as_bytes(),
            old.host_lineage_identity(),
            old.finalizer_derivation_sha256(),
            old.source_pin(),
            old.capsule_sha256(),
            old.formal_memory_receipt_sha256(),
            old.proof_binding_receipt_sha256(),
            hash(&[HSACO]),
            HSACO.len() as u64,
        )
        .unwrap();
        let service = M1AllKernelsProtectedVerifierServiceRequestV1::new(
            self.receipt.trust_policy_identity(),
            claims,
            *self.receipt.compiler_claims(),
            self.receipt.entries().map(|entry| {
                M1AllKernelsProtectedVerifierServiceEntryV1::from_receipt_entry(&entry)
            }),
        )
        .unwrap();
        IndependentCheckerResponseV1::verified(
            request,
            service,
            *self.receipt.entries(),
            [0xa3; 32],
        )
        .unwrap()
    }
}
fn deadline() -> AbsoluteSessionDeadlineV1 {
    AbsoluteSessionDeadlineV1::after(Duration::from_secs(5)).unwrap()
}
fn clone_request(request: &IndependentCheckerRequestV1) -> IndependentCheckerRequestV1 {
    IndependentCheckerRequestV1::decode_canonical(request.canonical_bytes()).unwrap()
}
fn packets(request: &IndependentCheckerRequestV1) -> Vec<Vec<u8>> {
    [ENVELOPE, HSACO]
        .into_iter()
        .enumerate()
        .flat_map(|(kind, bytes)| {
            bytes
                .chunks(INDEPENDENT_CHECKER_CHUNK_BYTES_V1)
                .enumerate()
                .map(move |(index, bytes)| {
                    payload_packet(
                        request,
                        kind,
                        index * INDEPENDENT_CHECKER_CHUNK_BYTES_V1,
                        bytes,
                    )
                    .unwrap()
                })
        })
        .collect()
}
fn pair() -> (OwnedFd, OwnedFd) {
    socketpair(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .unwrap()
}
fn endpoint(peer: &OwnedFd) -> IndependentCheckerEndpointV1 {
    let stat = rustix::fs::fstat(peer).unwrap();
    IndependentCheckerEndpointV1::new(
        stat.st_dev,
        stat.st_ino,
        std::process::id(),
        rustix::process::geteuid().as_raw(),
        rustix::process::getegid().as_raw(),
    )
    .unwrap()
}
fn client(
    peer: OwnedFd,
    context: IndependentCheckerContextV1,
) -> PreopenedIndependentCheckerClientV1 {
    let endpoint = endpoint(&peer);
    PreopenedIndependentCheckerClientV1::admit_inner::<false>(peer, endpoint, context).unwrap()
}
fn receive_input(peer: &OwnedFd) -> IndependentCheckerRequestV1 {
    let deadline = deadline();
    let metadata = receive_packet(peer, deadline).unwrap();
    let request = IndependentCheckerRequestV1::decode_canonical(&metadata).unwrap();
    let count = request
        .payload_lengths()
        .unwrap()
        .into_iter()
        .map(|length| length.div_ceil(INDEPENDENT_CHECKER_CHUNK_BYTES_V1))
        .sum::<usize>();
    let mut receiver =
        IndependentCheckerPayloadReceiverV1::new(request, deadline.instant()).unwrap();
    for _ in 0..count {
        receiver
            .push(&receive_packet(peer, deadline).unwrap())
            .unwrap();
    }
    let (request, envelope, hsaco) = receiver.finish().unwrap();
    assert_eq!(envelope, ENVELOPE);
    assert_eq!(hsaco, HSACO);
    request
}

#[test]
fn checker_full_byte_roundtrip_preserves_every_input_and_twelve_entries() {
    let fixture = Fixture::new();
    let request = fixture.request(1);
    let mut receiver =
        IndependentCheckerPayloadReceiverV1::new(clone_request(&request), deadline().instant())
            .unwrap();
    for packet in packets(&request) {
        receiver.push(&packet).unwrap();
    }
    let (decoded, envelope, hsaco) = receiver.finish().unwrap();
    assert_eq!(decoded.identity(), request.identity());
    assert_eq!(
        decoded.current_transcript(),
        fixture.current.transcript_identity()
    );
    assert_eq!(decoded.compiler_claims(), fixture.receipt.compiler_claims());
    assert_eq!(decoded.begin().entries().len(), 12);
    assert_eq!(envelope, ENVELOPE);
    assert_eq!(hsaco, HSACO);
    let result = fixture.response(&request);
    let result =
        IndependentCheckerResponseV1::decode_canonical(result.canonical_bytes(), &request).unwrap();
    assert_eq!(result.entries.unwrap(), *fixture.receipt.entries());
    assert!(!decoded.grants_authority());
}

#[test]
fn checker_payload_rejects_duplicate_reordered_empty_trailing_and_mutated_bytes() {
    let fixture = Fixture::new();
    let request = fixture.request(1);
    let packets = packets(&request);
    assert!(packets.len() >= 2);
    for mutation in 0..7 {
        let mut receiver =
            IndependentCheckerPayloadReceiverV1::new(clone_request(&request), deadline().instant())
                .unwrap();
        let mut first = packets[0].clone();
        match mutation {
            0 => {
                receiver.push(&first).unwrap();
                assert!(receiver.push(&first).is_err());
            }
            1 => {
                assert!(receiver.push(&packets[1]).is_err());
            }
            2 => {
                first.truncate(HEADER + 16);
                finalize_length(&mut first).unwrap();
                assert!(receiver.push(&first).is_err());
            }
            3 => {
                first[HEADER + 8] ^= 1;
                assert!(receiver.push(&first).is_err());
            }
            4 => {
                first[24] ^= 1;
                assert!(receiver.push(&first).is_err());
            }
            5 => {
                for packet in &packets {
                    receiver.push(packet).unwrap();
                }
                assert!(receiver.push(&first).is_err());
            }
            _ => {
                first[HEADER + 16] ^= 1;
                receiver.push(&first).unwrap();
                for packet in &packets[1..] {
                    receiver.push(packet).unwrap();
                }
            }
        }
        assert!(receiver.finish().is_err());
    }
    assert!(
        IndependentCheckerPayloadReceiverV1::new(clone_request(&request), deadline().instant())
            .unwrap()
            .finish()
            .is_err()
    );
}

#[test]
fn checker_request_and_result_substitution_are_rejected() {
    let fixture = Fixture::new();
    let request = fixture.request(1);
    let response = fixture.response(&request);
    for offset in [
        0,
        8,
        10,
        12,
        16,
        24,
        HEADER,
        HEADER + 32,
        HEADER + 64,
        REQUEST_FIXED,
    ] {
        let mut changed = request.canonical_bytes().to_vec();
        changed[offset] ^= 1;
        assert!(IndependentCheckerRequestV1::decode_canonical(&changed).is_err());
    }
    let mut trailing = request.canonical_bytes().to_vec();
    trailing.push(0);
    assert!(IndependentCheckerRequestV1::decode_canonical(&trailing).is_err());
    for offset in [
        0,
        8,
        10,
        12,
        16,
        24,
        HEADER,
        HEADER + 32,
        response.canonical_bytes().len() - 1,
    ] {
        let mut changed = response.canonical_bytes().to_vec();
        changed[offset] ^= 1;
        assert!(IndependentCheckerResponseV1::decode_canonical(&changed, &request).is_err());
    }
    assert!(
        IndependentCheckerResponseV1::decode_canonical(
            response.canonical_bytes(),
            &fixture.request(2)
        )
        .is_err()
    );
    let mut foreign = clone_request(&request);
    foreign.context.session = [0xee; 32];
    foreign.identity[0] ^= 1;
    assert!(
        IndependentCheckerResponseV1::decode_canonical(response.canonical_bytes(), &foreign)
            .is_err()
    );
    let mut entries = *fixture.receipt.entries();
    entries.swap(0, 1);
    let service = response.service.unwrap();
    assert!(
        IndependentCheckerResponseV1::verified(&request, service, entries, [0xa3; 32]).is_err()
    );
}

#[test]
fn checker_safe_codecs_never_supply_production_admission() {
    let fixture = Fixture::new();
    assert!(
        IndependentCheckerContextV1::new(
            [0; 32],
            fixture.context.measurement,
            fixture.context.policy,
            fixture.context.session
        )
        .is_err()
    );
    for axis in 0..3 {
        let mut ids = [
            fixture.context.measurement,
            fixture.context.policy,
            fixture.context.session,
        ];
        ids[axis] = [0; 32];
        assert!(
            IndependentCheckerContextV1::new(
                independent_checker_protocol_identity_v1(),
                ids[0],
                ids[1],
                ids[2]
            )
            .is_err()
        );
    }
    let (peer, _server) = pair();
    let identity = endpoint(&peer);
    let failure =
        PreopenedIndependentCheckerClientV1::admit_inner::<true>(peer, identity, fixture.context)
            .err()
            .unwrap();
    let (error, retained) = failure.into_parts();
    assert!(matches!(
        error,
        IndependentCheckerErrorV1::Endpoint(
            ProtectedCompilerCurrentClientAdmissionErrorV1::ProviderAndVerifierUidMatch
        )
    ));
    assert_eq!(
        rustix::fs::fstat(&retained).unwrap().st_ino,
        identity.object.inode()
    );
    is_actual_provider::<PreopenedIndependentCheckerClientV1>();
}

fn is_actual_provider<T: IndependentCheckerProviderV1>() {}

#[test]
fn checker_actual_provider_core_transfers_bytes_and_reuses_after_explicit_rejection() {
    let fixture = Fixture::new();
    let (peer, server) = pair();
    let mut client = client(peer, fixture.context);
    let (release, hold) = std::sync::mpsc::channel();
    let task = thread::spawn(move || {
        let first = receive_input(&server);
        assert_eq!(first.sequence(), 1);
        send_packet(
            &server,
            IndependentCheckerResponseV1::rejected(&first).canonical_bytes(),
            deadline(),
        )
        .unwrap();
        let second = receive_input(&server);
        assert_eq!(second.sequence(), 2);
        let fixture = Fixture::new();
        send_packet(
            &server,
            fixture.response(&second).canonical_bytes(),
            deadline(),
        )
        .unwrap();
        hold.recv_timeout(Duration::from_secs(5)).unwrap();
    });
    let failure = client
        .verify_inner::<false>(&fixture.input(deadline()))
        .err()
        .unwrap();
    assert_eq!(failure.custody(), IndependentCheckerCustodyV1::Retained);
    assert!(matches!(
        failure.error(),
        IndependentCheckerErrorV1::Rejected
    ));
    let _claims = client
        .verify_inner::<false>(&fixture.input(deadline()))
        .unwrap();
    assert!(!client.is_poisoned());
    release.send(()).unwrap();
    task.join().unwrap();
}

#[test]
fn checker_deadlines_preserve_pre_send_and_poison_post_send_custody() {
    let fixture = Fixture::new();
    let (peer, server) = pair();
    let mut client = client(peer, fixture.context);
    let expired = AbsoluteSessionDeadlineV1::after(Duration::ZERO).unwrap();
    let failure = client
        .verify_inner::<false>(&fixture.input(expired))
        .err()
        .unwrap();
    assert!(matches!(
        failure.error(),
        IndependentCheckerErrorV1::Deadline
    ));
    assert_eq!(failure.custody(), IndependentCheckerCustodyV1::Retained);
    assert_eq!(client.next_sequence, 1);
    require_empty(&server).unwrap();
    let task = thread::spawn(move || {
        let _request = receive_input(&server);
        thread::sleep(Duration::from_millis(250));
    });
    let deadline = AbsoluteSessionDeadlineV1::after(Duration::from_millis(100)).unwrap();
    let failure = client
        .verify_inner::<false>(&fixture.input(deadline))
        .err()
        .unwrap();
    assert!(matches!(
        failure.error(),
        IndependentCheckerErrorV1::Deadline
    ));
    assert_eq!(failure.custody(), IndependentCheckerCustodyV1::Poisoned);
    assert!(client.is_poisoned());
    task.join().unwrap();
}

#[test]
fn checker_corrupt_replayed_truncated_and_ancillary_replies_poison() {
    for mode in 0..5 {
        let fixture = Fixture::new();
        let (peer, server) = pair();
        let mut client = client(peer, fixture.context);
        let task = thread::spawn(move || {
            let request = receive_input(&server);
            let fixture = Fixture::new();
            let response = fixture.response(&request);
            let mut bytes = response.canonical_bytes().to_vec();
            match mode {
                0 => {
                    bytes[24] ^= 1;
                }
                1 => {
                    bytes.pop();
                }
                2 => {
                    bytes = vec![1; INDEPENDENT_CHECKER_MAX_PACKET_BYTES_V1 + 1];
                }
                3 => {
                    bytes[16] ^= 1;
                }
                _ => {
                    send_ancillary(&server, &bytes);
                    thread::sleep(Duration::from_millis(30));
                    return;
                }
            }
            send_packet(&server, &bytes, deadline()).unwrap();
            thread::sleep(Duration::from_millis(30));
        });
        let failure = client
            .verify_inner::<false>(&fixture.input(deadline()))
            .err()
            .unwrap();
        assert_eq!(failure.custody(), IndependentCheckerCustodyV1::Poisoned);
        assert!(client.is_poisoned());
        task.join().unwrap();
    }
}

fn send_ancillary(peer: &OwnedFd, bytes: &[u8]) {
    let mut iov = libc::iovec {
        iov_base: bytes.as_ptr().cast_mut().cast(),
        iov_len: bytes.len(),
    };
    let mut control = [0_usize; 8];
    // SAFETY: initialized message and aligned control storage remain live; the
    // test transfers a duplicate of its already-owned socket, never native authority.
    unsafe {
        let mut message: libc::msghdr = mem::zeroed();
        message.msg_iov = &raw mut iov;
        message.msg_iovlen = 1;
        message.msg_control = control.as_mut_ptr().cast();
        let descriptor_length = u32::try_from(mem::size_of::<i32>()).unwrap();
        message.msg_controllen = usize::try_from(libc::CMSG_SPACE(descriptor_length)).unwrap();
        let cmsg = libc::CMSG_FIRSTHDR(&raw const message);
        (*cmsg).cmsg_level = libc::SOL_SOCKET;
        (*cmsg).cmsg_type = libc::SCM_RIGHTS;
        (*cmsg).cmsg_len = usize::try_from(libc::CMSG_LEN(descriptor_length)).unwrap();
        let descriptor = peer.as_raw_fd().to_ne_bytes();
        std::ptr::copy_nonoverlapping(descriptor.as_ptr(), libc::CMSG_DATA(cmsg), descriptor.len());
        assert_eq!(
            libc::sendmsg(
                peer.as_raw_fd(),
                &raw const message,
                libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL
            ),
            isize::try_from(bytes.len()).unwrap()
        );
    }
}

#[test]
fn checker_prequeued_endpoint_and_fixed_receiver_deadline_reject() {
    let fixture = Fixture::new();
    let (peer, server) = pair();
    send_packet(&server, b"unexpected", deadline()).unwrap();
    let identity = endpoint(&peer);
    let failure =
        PreopenedIndependentCheckerClientV1::admit_inner::<false>(peer, identity, fixture.context)
            .err()
            .unwrap();
    assert!(matches!(
        failure.into_parts().0,
        IndependentCheckerErrorV1::UnexpectedData
    ));
    let request = fixture.request(1);
    assert!(
        IndependentCheckerPayloadReceiverV1::new(clone_request(&request), Instant::now()).is_err()
    );
    let mut receiver = IndependentCheckerPayloadReceiverV1::new(
        clone_request(&request),
        Instant::now() + Duration::from_millis(1),
    )
    .unwrap();
    thread::sleep(Duration::from_millis(5));
    assert!(matches!(
        receiver.push(&packets(&request)[0]),
        Err(IndependentCheckerErrorV1::Deadline)
    ));
    assert!(receiver.finish().is_err());
}

fn redigest_response(bytes: &mut [u8]) {
    let offset = bytes.len() - 32;
    let digest = hash(&[RESPONSE_DOMAIN, &bytes[..offset]]);
    bytes[offset..].copy_from_slice(&digest);
}

fn changed_claims(
    old: &M1AllKernelsProtectedReceiptRequestClaimsV1,
    axis: usize,
) -> M1AllKernelsProtectedReceiptRequestClaimsV1 {
    M1AllKernelsProtectedReceiptRequestClaimsV1::new(
        if axis == 0 {
            [0xd9; 32]
        } else {
            old.challenge_identity()
        },
        if axis == 1 {
            [0xd9; 32]
        } else {
            old.roster_identity()
        },
        old.host_lineage_identity(),
        old.finalizer_derivation_sha256(),
        old.source_pin(),
        old.capsule_sha256(),
        old.formal_memory_receipt_sha256(),
        old.proof_binding_receipt_sha256(),
        if axis == 2 {
            [0xd9; 32]
        } else {
            old.finalized_hsaco_sha256()
        },
        old.finalized_hsaco_length() + u64::from(axis == 3),
    )
    .unwrap()
}

fn changed_compiler(
    old: &M1AllKernelsProtectedReceiptCompilerClaimsV1,
) -> M1AllKernelsProtectedReceiptCompilerClaimsV1 {
    M1AllKernelsProtectedReceiptCompilerClaimsV1::new(
        old.subject_sha256(),
        old.carriage_sha256(),
        old.policy_sha256(),
        old.issuer_journal_sha256(),
        old.compiler_occurrence_sha256(),
        old.receipt_sha256(),
        old.publication_sha256(),
        old.acknowledgment_sha256(),
        old.worker_ledger_record_sha256(),
        old.sequence() + 1,
        old.prior_rollback_anchor(),
        old.current_rollback_anchor(),
        old.current_record_verification_sha256(),
        old.current_record_attestation_sha256(),
        old.protected_policy_verification_sha256(),
        old.protected_worker_ledger_verification_sha256(),
        old.external_rollback_verification_sha256(),
    )
    .unwrap()
}

#[test]
fn checker_redigested_semantic_results_reject_policy_compiler_claims_and_every_entry_join() {
    let fixture = Fixture::new();
    let request = fixture.request(1);
    let response = fixture.response(&request);
    let service = response.service.as_ref().unwrap();
    let key = ed25519_dalek::SigningKey::from_bytes(&[0xd8; 32]);
    let other_policy = ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_receipt::M1AllKernelsProtectedVerifierTrustPolicyV1::new(key.verifying_key().to_bytes(), [0xd7; 32], [0xd6; 32]).unwrap();
    for axis in 0..42 {
        let policy = if axis == 0 {
            other_policy.identity()
        } else {
            service.trust_policy_identity()
        };
        let compiler = if axis == 1 {
            changed_compiler(service.compiler_claims())
        } else {
            *service.compiler_claims()
        };
        let claims = if (2..6).contains(&axis) {
            changed_claims(service.request_claims(), axis - 2)
        } else {
            *service.request_claims()
        };
        let mut entries = *service.entries();
        if axis >= 6 {
            let ordinal = (axis - 6) / 3;
            let field = (axis - 6) % 3;
            let old = entries[ordinal];
            entries[ordinal] = M1AllKernelsProtectedVerifierServiceEntryV1::new(
                old.ordinal(),
                if field == 0 {
                    [0xd9; 32]
                } else {
                    old.lineage_identity()
                },
                if field == 1 {
                    [0xd9; 32]
                } else {
                    old.marker_binding_identity()
                },
                if field == 2 {
                    [0xd9; 32]
                } else {
                    old.generated_host_contract_identity()
                },
            )
            .unwrap();
        }
        let changed =
            M1AllKernelsProtectedVerifierServiceRequestV1::new(policy, claims, compiler, entries)
                .unwrap();
        let mut bytes = response.canonical_bytes().to_vec();
        bytes
            [HEADER + 32..HEADER + 32 + M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1]
            .copy_from_slice(changed.canonical_bytes());
        redigest_response(&mut bytes);
        assert!(
            matches!(
                IndependentCheckerResponseV1::decode_canonical(&bytes, &request),
                Err(IndependentCheckerErrorV1::Association)
            ),
            "semantic axis {axis} must reach association checking"
        );
    }
}

#[test]
fn checker_redigested_results_require_nonzero_theorems_and_all_safety_bits_but_do_not_prove_them() {
    let fixture = Fixture::new();
    let request = fixture.request(1);
    let response = fixture.response(&request);
    let result_start = HEADER + 32 + M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1;
    for ordinal in 0..12 {
        for field in 0..4 {
            let mut bytes = response.canonical_bytes().to_vec();
            let offset = result_start + ordinal * 97 + field * 32;
            if field == 3 {
                bytes[offset] = 0;
            } else {
                bytes[offset..offset + 32].fill(0);
            }
            redigest_response(&mut bytes);
            assert!(matches!(
                IndependentCheckerResponseV1::decode_canonical(&bytes, &request),
                Err(IndependentCheckerErrorV1::Packet)
            ));
        }
        let mut bytes = response.canonical_bytes().to_vec();
        bytes[result_start + ordinal * 97] ^= 1;
        redigest_response(&mut bytes);
        let decoded = IndependentCheckerResponseV1::decode_canonical(&bytes, &request).unwrap();
        assert_ne!(
            decoded.entries.unwrap()[ordinal].proof_executable_binding_sha256(),
            fixture.receipt.entries()[ordinal].proof_executable_binding_sha256()
        );
        // The codec checks identity structure, not the theorem denoted by a
        // nonzero digest. Only the separately measured checker may assert it.
    }
}

#[test]
fn checker_exact_packet_chunk_and_sequence_bounds_are_enforced() {
    let fixture = Fixture::new();
    let request = fixture.request(1);
    assert!(matches!(
        IndependentCheckerRequestV1::new(fixture.context, 0, &fixture.input(deadline())),
        Err(IndependentCheckerErrorV1::Context)
    ));
    let mut packet = header(PAYLOAD, 1, request.identity());
    packet.resize(INDEPENDENT_CHECKER_MAX_PACKET_BYTES_V1, 0);
    finalize_length(&mut packet).unwrap();
    check_header(&packet, PAYLOAD).unwrap();
    packet.push(0);
    assert!(matches!(
        finalize_length(&mut packet),
        Err(IndependentCheckerErrorV1::Bound)
    ));
    assert!(check_header(&packet, PAYLOAD).is_err());
    let lengths = request.payload_lengths().unwrap();
    for (kind, bytes) in [ENVELOPE, HSACO].into_iter().enumerate() {
        let count = lengths[kind].div_ceil(INDEPENDENT_CHECKER_CHUNK_BYTES_V1);
        let mut observed = 0;
        for (index, chunk) in bytes.chunks(INDEPENDENT_CHECKER_CHUNK_BYTES_V1).enumerate() {
            let offset = index * INDEPENDENT_CHECKER_CHUNK_BYTES_V1;
            let packet = payload_packet(&request, kind, offset, chunk).unwrap();
            assert_eq!(packet.len(), HEADER + 16 + chunk.len());
            assert!(packet.len() <= INDEPENDENT_CHECKER_MAX_PACKET_BYTES_V1);
            assert!(payload_packet(&request, kind, offset, &chunk[..chunk.len() - 1]).is_err());
            let mut oversized = chunk.to_vec();
            oversized.push(0);
            assert!(payload_packet(&request, kind, offset, &oversized).is_err());
            observed += 1;
        }
        assert_eq!(observed, count);
        assert!(payload_packet(&request, kind, lengths[kind], &[]).is_err());
    }
    assert!(payload_packet(&request, 2, 0, &[0]).is_err());
    let (peer, server) = pair();
    let mut client = client(peer, fixture.context);
    client.next_sequence = 0;
    let failure = client
        .verify_inner::<false>(&fixture.input(deadline()))
        .err()
        .unwrap();
    assert!(matches!(failure.error(), IndependentCheckerErrorV1::Bound));
    assert_eq!(failure.custody(), IndependentCheckerCustodyV1::Retained);
    require_empty(&server).unwrap();
}
