//! Bounded full-byte IPC for the separately measured independent checker.
//!
//! The codec and payload receiver are inert. Only the explicitly unsafe
//! supervisor admission connects an authenticated peer result to the existing
//! service provider contract. No theorem checker, key, or deployment is supplied.

#![allow(
    clippy::must_use_candidate,
    reason = "accessors expose inert protocol data"
)]

use core::fmt;
use std::error::Error;
use std::io;
use std::mem;
use std::os::fd::{AsRawFd, OwnedFd};
use std::time::Instant;

use fe2o3_host::WorkerV3SafetyPropertiesV1;
use fe2o3_worker_v3_verification_protocol::{
    MAX_WORKER_V3_VERIFICATION_ENVELOPE_FD_BYTES_V1, MAX_WORKER_V3_VERIFICATION_HSACO_FD_BYTES_V1,
    MAX_WORKER_V3_VERIFICATION_REQUEST_BYTES_V1, WorkerV3VerificationRequestV1,
};
use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_receipt::{
    M1AllKernelsProtectedReceiptCompilerClaimsV1, M1AllKernelsProtectedReceiptEntryV1,
};
use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_verifier_service::{
    M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1,
    M1AllKernelsProtectedVerifierServiceRequestV1,
};
use sha2::{Digest, Sha256};

use crate::{
    AbsoluteSessionDeadlineV1, IndependentCheckerInputV1, IndependentCheckerProviderV1,
    IndependentCheckerVerifiedClaimsV1, ProtectedCompilerCurrentClientAdmissionErrorV1,
    ProtectedCompilerCurrentEndpointIdentityV1, ProtectedCompilerCurrentPeerIdentityV1,
};

const MAGIC: &[u8; 8] = b"F3CKIPC1";
const VERSION: u16 = 1;
const HEADER: usize = 56;
const REQUEST: u16 = 1;
const PAYLOAD: u16 = 2;
const VERIFIED: u16 = 3;
const REJECTED: u16 = 4;
const CLAIM_BYTES: usize = 520;
const REQUEST_FIXED: usize = HEADER + 128 + CLAIM_BYTES;
const RESULT_BYTES: usize =
    32 + M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1 + 12 * 97;
const REQUEST_DOMAIN: &[u8] = b"FERRIC/INDEPENDENT-CHECKER/REQUEST/V1\0";
const RESPONSE_DOMAIN: &[u8] = b"FERRIC/INDEPENDENT-CHECKER/RESPONSE/V1\0";

/// Maximum data bytes in one ordered payload packet (not including its header).
pub const INDEPENDENT_CHECKER_CHUNK_BYTES_V1: usize = 16 * 1024;
/// Maximum complete packet accepted by this protocol.
pub const INDEPENDENT_CHECKER_MAX_PACKET_BYTES_V1: usize = 32 * 1024;
const _: () = assert!(REQUEST_FIXED < INDEPENDENT_CHECKER_MAX_PACKET_BYTES_V1);
const _: () = assert!(HEADER + RESULT_BYTES + 32 <= INDEPENDENT_CHECKER_MAX_PACKET_BYTES_V1);

/// Identity of the exact full-byte, ordered-packet protocol implemented here.
pub fn independent_checker_protocol_identity_v1() -> [u8; 32] {
    hash(&[
        b"ferric.independent-checker.full-byte-seqpacket.v1\0",
        &VERSION.to_le_bytes(),
        &(INDEPENDENT_CHECKER_CHUNK_BYTES_V1 as u64).to_le_bytes(),
        &MAX_WORKER_V3_VERIFICATION_ENVELOPE_FD_BYTES_V1.to_le_bytes(),
        &MAX_WORKER_V3_VERIFICATION_HSACO_FD_BYTES_V1.to_le_bytes(),
    ])
}

/// Supervisor-provisioned socket object and dedicated checker credentials.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndependentCheckerEndpointV1 {
    object: ProtectedCompilerCurrentEndpointIdentityV1,
    peer: ProtectedCompilerCurrentPeerIdentityV1,
}

impl IndependentCheckerEndpointV1 {
    /// Constructs inert nonzero object metadata and positive non-root credentials.
    ///
    /// # Errors
    /// Rejects zero object coordinates and invalid Linux credentials.
    pub fn new(
        device: u64,
        inode: u64,
        pid: u32,
        uid: u32,
        gid: u32,
    ) -> Result<Self, IndependentCheckerErrorV1> {
        Ok(Self {
            object: ProtectedCompilerCurrentEndpointIdentityV1::new(device, inode)
                .map_err(|_| IndependentCheckerErrorV1::Context)?,
            peer: ProtectedCompilerCurrentPeerIdentityV1::new(pid, uid, gid)
                .map_err(|_| IndependentCheckerErrorV1::Context)?,
        })
    }
}

/// Inert pinned checker measurement, trust policy, and never-reused admission session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndependentCheckerContextV1 {
    measurement: [u8; 32],
    policy: [u8; 32],
    session: [u8; 32],
}

impl IndependentCheckerContextV1 {
    /// Checks the exact protocol and nonzero independently provisioned coordinates.
    ///
    /// # Errors
    /// Rejects unknown protocols and zero identities; this does not authenticate them.
    pub fn new(
        protocol: [u8; 32],
        measurement: [u8; 32],
        policy: [u8; 32],
        session: [u8; 32],
    ) -> Result<Self, IndependentCheckerErrorV1> {
        if protocol != independent_checker_protocol_identity_v1()
            || [measurement, policy, session].contains(&[0; 32])
        {
            return Err(IndependentCheckerErrorV1::Context);
        }
        Ok(Self {
            measurement,
            policy,
            session,
        })
    }

    /// Returns the pinned checker measurement.
    pub const fn measurement(self) -> [u8; 32] {
        self.measurement
    }
    /// Returns the complete protected-verifier trust-policy identity.
    pub const fn policy(self) -> [u8; 32] {
        self.policy
    }
    /// Returns the admission identity which deployment must never reuse.
    pub const fn session(self) -> [u8; 32] {
        self.session
    }
}

/// Fail-closed protocol, resource, endpoint, and transport errors.
#[derive(Debug)]
#[non_exhaustive]
pub enum IndependentCheckerErrorV1 {
    /// Invalid pinned coordinates, unsupported protocol, or wrong context.
    Context,
    /// Noncanonical or incorrectly sized packet.
    Packet,
    /// Wrong request, payload, compiler claim, or ordered result association.
    Association,
    /// A declared length, allocation, sequence, or packet-count bound was exceeded.
    Bound,
    /// Input is incomplete, duplicated, reordered, trailing, or has wrong bytes.
    Payload,
    /// The sole absolute deadline expired.
    Deadline,
    /// The separately measured checker explicitly rejected this exact request.
    Rejected,
    /// The client has permanently closed its endpoint.
    Poisoned,
    /// The endpoint had data before a new request or after its terminal reply.
    UnexpectedData,
    /// The endpoint failed the existing exact Linux object/credential checks.
    Endpoint(ProtectedCompilerCurrentClientAdmissionErrorV1),
    /// A syscall failed without exposing artifact contents.
    Io(io::Error),
}

impl fmt::Display for IndependentCheckerErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "independent checker: {self:?}")
    }
}
impl Error for IndependentCheckerErrorV1 {}

/// Endpoint custody after a bounded provider call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndependentCheckerCustodyV1 {
    /// No ambiguous exchange occurred; the exact client still owns its endpoint.
    Retained,
    /// The endpoint was closed and cannot be reused.
    Poisoned,
}

/// Typed provider failure, retaining the endpoint in the client when stated.
#[derive(Debug)]
pub struct IndependentCheckerFailureV1 {
    error: IndependentCheckerErrorV1,
    custody: IndependentCheckerCustodyV1,
}
impl IndependentCheckerFailureV1 {
    /// Returns the failure category.
    pub const fn error(&self) -> &IndependentCheckerErrorV1 {
        &self.error
    }
    /// Returns whether the client retained or closed its endpoint.
    pub const fn custody(&self) -> IndependentCheckerCustodyV1 {
        self.custody
    }
}
impl fmt::Display for IndependentCheckerFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ({:?})", self.error, self.custody)
    }
}
impl Error for IndependentCheckerFailureV1 {}

/// Rejected admission retaining the untouched supplied descriptor.
#[derive(Debug)]
pub struct IndependentCheckerAdmissionFailureV1 {
    error: IndependentCheckerErrorV1,
    peer: OwnedFd,
}
impl IndependentCheckerAdmissionFailureV1 {
    /// Recovers both the rejection and the exact original endpoint.
    pub fn into_parts(self) -> (IndependentCheckerErrorV1, OwnedFd) {
        (self.error, self.peer)
    }
}

/// Complete canonical metadata for one inert checker request.
#[derive(Debug)]
pub struct IndependentCheckerRequestV1 {
    context: IndependentCheckerContextV1,
    sequence: u64,
    current_transcript: [u8; 32],
    compiler: M1AllKernelsProtectedReceiptCompilerClaimsV1,
    begin: WorkerV3VerificationRequestV1,
    identity: [u8; 32],
    bytes: Vec<u8>,
}

impl IndependentCheckerRequestV1 {
    fn new(
        context: IndependentCheckerContextV1,
        sequence: u64,
        input: &IndependentCheckerInputV1<'_>,
    ) -> Result<Self, IndependentCheckerErrorV1> {
        if input.request().encode_canonical().len()
            > INDEPENDENT_CHECKER_MAX_PACKET_BYTES_V1 - REQUEST_FIXED
        {
            return Err(IndependentCheckerErrorV1::Bound);
        }
        let mut bytes = header(REQUEST, sequence, [0; 32]);
        bytes.extend_from_slice(&context.measurement);
        bytes.extend_from_slice(&context.policy);
        bytes.extend_from_slice(&context.session);
        bytes.extend_from_slice(&input.current_authentication().transcript_identity());
        encode_compiler(&mut bytes, input.compiler_claims());
        bytes.extend_from_slice(input.request().encode_canonical());
        finalize_length(&mut bytes)?;
        let identity = hash(&[REQUEST_DOMAIN, &bytes]);
        bytes[24..HEADER].copy_from_slice(&identity);
        let request = Self::decode_canonical(&bytes)?;
        validate_payload_bytes(
            &request,
            input.envelope_bytes(),
            input.hsaco_bytes(),
            input.deadline().instant(),
        )?;
        if input
            .envelope()
            .encode_canonical()
            .map_err(|_| IndependentCheckerErrorV1::Association)?
            .as_slice()
            != input.envelope_bytes()
        {
            return Err(IndependentCheckerErrorV1::Association);
        }
        require_live(input.deadline())?;
        Ok(request)
    }

    /// Decodes metadata only; the payload receiver must separately receive every byte.
    ///
    /// # Errors
    /// Rejects noncanonical, over-bound, zero-sequence, or wrong-roster metadata.
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self, IndependentCheckerErrorV1> {
        let (sequence, identity) = check_header(bytes, REQUEST)?;
        if bytes.len() <= REQUEST_FIXED
            || bytes.len() > REQUEST_FIXED + MAX_WORKER_V3_VERIFICATION_REQUEST_BYTES_V1
        {
            return Err(IndependentCheckerErrorV1::Bound);
        }
        let mut canonical = bytes.to_vec();
        canonical[24..HEADER].fill(0);
        if hash(&[REQUEST_DOMAIN, &canonical]) != identity {
            return Err(IndependentCheckerErrorV1::Association);
        }
        let mut reader = Reader::new(&bytes[HEADER..]);
        let context = IndependentCheckerContextV1::new(
            independent_checker_protocol_identity_v1(),
            reader.array()?,
            reader.array()?,
            reader.array()?,
        )?;
        let current_transcript = reader.array()?;
        if current_transcript == [0; 32] {
            return Err(IndependentCheckerErrorV1::Context);
        }
        let compiler = decode_compiler(&mut reader)?;
        let begin = WorkerV3VerificationRequestV1::decode_canonical(reader.rest())
            .map_err(|_| IndependentCheckerErrorV1::Packet)?;
        if begin.entries().len() != 12 || *begin.policy_identity().as_bytes() != context.policy {
            return Err(IndependentCheckerErrorV1::Association);
        }
        let request = Self {
            context,
            sequence,
            current_transcript,
            compiler,
            begin,
            identity,
            bytes: bytes.to_vec(),
        };
        request.payload_lengths()?;
        Ok(request)
    }

    /// Returns exact canonical metadata bytes, not a proof or authentication token.
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Returns the complete Begin frame including the ordered twelve entries.
    pub const fn begin(&self) -> &WorkerV3VerificationRequestV1 {
        &self.begin
    }
    /// Returns all compiler/currentness claims supplied by the authenticated service.
    pub const fn compiler_claims(&self) -> &M1AllKernelsProtectedReceiptCompilerClaimsV1 {
        &self.compiler
    }
    /// Returns the protected current-record transcript bound into this request.
    pub const fn current_transcript(&self) -> [u8; 32] {
        self.current_transcript
    }
    /// Returns the exact pinned context.
    pub const fn context(&self) -> IndependentCheckerContextV1 {
        self.context
    }
    /// Returns the nonzero per-admission sequence.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
    /// Returns the complete request identity, including both payload descriptors.
    pub const fn identity(&self) -> [u8; 32] {
        self.identity
    }
    /// Decoding has no authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }

    fn payload_lengths(&self) -> Result<[usize; 2], IndependentCheckerErrorV1> {
        let payloads = self.begin.payloads();
        let lengths = [payloads[0].byte_len(), payloads[1].byte_len()];
        if lengths[0] == 0
            || lengths[0] > MAX_WORKER_V3_VERIFICATION_ENVELOPE_FD_BYTES_V1
            || lengths[1] == 0
            || lengths[1] > MAX_WORKER_V3_VERIFICATION_HSACO_FD_BYTES_V1
        {
            return Err(IndependentCheckerErrorV1::Bound);
        }
        Ok([
            usize::try_from(lengths[0]).map_err(|_| IndependentCheckerErrorV1::Bound)?,
            usize::try_from(lengths[1]).map_err(|_| IndependentCheckerErrorV1::Bound)?,
        ])
    }
}

/// Inert bounded full-byte handoff assembler for an external checker implementation.
///
/// It accepts exactly the envelope followed by the HSACO, in canonical fixed-size
/// chunks. It never establishes theorem, compiler, currentness, or verifier authority.
pub struct IndependentCheckerPayloadReceiverV1 {
    request: IndependentCheckerRequestV1,
    lengths: [usize; 2],
    payloads: [Vec<u8>; 2],
    active: usize,
    poisoned: bool,
    deadline: Instant,
}
impl IndependentCheckerPayloadReceiverV1 {
    /// Preallocates exactly the two already-bounded byte buffers.
    ///
    /// # Errors
    /// Rejects unsupported sizes or allocation failure before accepting chunks.
    pub fn new(
        request: IndependentCheckerRequestV1,
        deadline: Instant,
    ) -> Result<Self, IndependentCheckerErrorV1> {
        require_instant_live(deadline)?;
        let lengths = request.payload_lengths()?;
        let mut payloads = [Vec::new(), Vec::new()];
        for (payload, length) in payloads.iter_mut().zip(lengths) {
            payload
                .try_reserve_exact(length)
                .map_err(|_| IndependentCheckerErrorV1::Bound)?;
        }
        Ok(Self {
            request,
            lengths,
            payloads,
            active: 0,
            poisoned: false,
            deadline,
        })
    }

    /// Consumes exactly one next packet under the caller's unchanged absolute deadline.
    ///
    /// # Errors
    /// Every rejection permanently poisons this assembler; trailing packets reject too.
    pub fn push(&mut self, packet: &[u8]) -> Result<(), IndependentCheckerErrorV1> {
        let result = self.push_inner(packet);
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }

    fn push_inner(&mut self, packet: &[u8]) -> Result<(), IndependentCheckerErrorV1> {
        require_instant_live(self.deadline)?;
        if self.poisoned || self.active == 2 {
            return Err(IndependentCheckerErrorV1::Payload);
        }
        let (sequence, identity) = check_header(packet, PAYLOAD)?;
        if sequence != self.request.sequence
            || identity != self.request.identity
            || packet.len() < HEADER + 16
        {
            return Err(IndependentCheckerErrorV1::Association);
        }
        let mut reader = Reader::new(&packet[HEADER..]);
        let kind = reader.u64()?;
        let offset = reader.u64()?;
        let payload = &mut self.payloads[self.active];
        let expected_length =
            (self.lengths[self.active] - payload.len()).min(INDEPENDENT_CHECKER_CHUNK_BYTES_V1);
        if kind != self.active as u64
            || offset != payload.len() as u64
            || reader.rest().len() != expected_length
            || expected_length == 0
        {
            return Err(IndependentCheckerErrorV1::Payload);
        }
        payload.extend_from_slice(reader.rest());
        if payload.len() == self.lengths[self.active] {
            self.active += 1;
        }
        Ok(())
    }

    /// Returns all exact input bytes only after complete length and digest validation.
    ///
    /// # Errors
    /// Rejects incomplete/poisoned input, expiry, and any payload substitution.
    pub fn finish(
        self,
    ) -> Result<(IndependentCheckerRequestV1, Vec<u8>, Vec<u8>), IndependentCheckerErrorV1> {
        if self.poisoned || self.active != 2 {
            return Err(IndependentCheckerErrorV1::Payload);
        }
        let [envelope, hsaco] = self.payloads;
        validate_payload_bytes(&self.request, &envelope, &hsaco, self.deadline)?;
        Ok((self.request, envelope, hsaco))
    }
}

/// Canonical inert terminal result. Constructing one never grants theorem authority.
pub struct IndependentCheckerResponseV1 {
    service: Option<M1AllKernelsProtectedVerifierServiceRequestV1>,
    entries: Option<[M1AllKernelsProtectedReceiptEntryV1; 12]>,
    transcript: [u8; 32],
    bytes: Vec<u8>,
}
impl IndependentCheckerResponseV1 {
    /// Encodes a measured checker's asserted result after exact request/result association.
    ///
    /// # Errors
    /// Rejects wrong policy/compiler/request/entry identities or a zero transcript.
    pub fn verified(
        request: &IndependentCheckerRequestV1,
        service: M1AllKernelsProtectedVerifierServiceRequestV1,
        entries: [M1AllKernelsProtectedReceiptEntryV1; 12],
        transcript: [u8; 32],
    ) -> Result<Self, IndependentCheckerErrorV1> {
        validate_result(request, &service, &entries, transcript)?;
        let mut bytes = header(VERIFIED, request.sequence, request.identity);
        bytes.extend_from_slice(&transcript);
        bytes.extend_from_slice(service.canonical_bytes());
        for entry in &entries {
            bytes.extend_from_slice(&entry.proof_executable_binding_sha256());
            bytes.extend_from_slice(&entry.rust_type_layout_contract_sha256());
            bytes.extend_from_slice(&entry.rust_effect_contract_sha256());
            bytes.push(entry.safety_properties().bits());
        }
        finish_response(&mut bytes)?;
        Ok(Self {
            service: Some(service),
            entries: Some(entries),
            transcript,
            bytes,
        })
    }

    /// Encodes an explicit rejection correlated to exactly one complete request.
    ///
    /// # Panics
    /// Panics if an internal fixed-length protocol invariant is violated.
    pub fn rejected(request: &IndependentCheckerRequestV1) -> Self {
        let mut bytes = header(REJECTED, request.sequence, request.identity);
        finish_response(&mut bytes).expect("fixed rejected response length is bounded");
        Self {
            service: None,
            entries: None,
            transcript: [0; 32],
            bytes,
        }
    }

    /// Strictly decodes and correlates one exact terminal result.
    ///
    /// # Errors
    /// Rejects unknown statuses, trailing bytes, malformed claims, and substitutions.
    pub fn decode_canonical(
        bytes: &[u8],
        request: &IndependentCheckerRequestV1,
    ) -> Result<Self, IndependentCheckerErrorV1> {
        let kind = packet_kind(bytes)?;
        let (sequence, identity) = check_header(bytes, kind)?;
        if sequence != request.sequence || identity != request.identity || bytes.len() < HEADER + 32
        {
            return Err(IndependentCheckerErrorV1::Association);
        }
        let digest_offset = bytes.len() - 32;
        if bytes[digest_offset..] != hash(&[RESPONSE_DOMAIN, &bytes[..digest_offset]]) {
            return Err(IndependentCheckerErrorV1::Association);
        }
        if kind == REJECTED {
            if bytes.len() != HEADER + 32 {
                return Err(IndependentCheckerErrorV1::Packet);
            }
            return Ok(Self::rejected(request));
        }
        if kind != VERIFIED || bytes.len() != HEADER + RESULT_BYTES + 32 {
            return Err(IndependentCheckerErrorV1::Packet);
        }
        let mut reader = Reader::new(&bytes[HEADER..digest_offset]);
        let transcript = reader.array()?;
        let service = M1AllKernelsProtectedVerifierServiceRequestV1::decode(
            reader.take(M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_REQUEST_BYTES_V1)?,
        )
        .map_err(|_| IndependentCheckerErrorV1::Packet)?;
        let mut entries = Vec::with_capacity(12);
        for expected in service.entries() {
            let proof = reader.array()?;
            let layout = reader.array()?;
            let effect = reader.array()?;
            let safety = WorkerV3SafetyPropertiesV1::new(reader.array::<1>()?[0])
                .ok_or(IndependentCheckerErrorV1::Packet)?;
            entries.push(
                M1AllKernelsProtectedReceiptEntryV1::new(
                    expected.ordinal(),
                    expected.lineage_identity(),
                    expected.marker_binding_identity(),
                    expected.generated_host_contract_identity(),
                    proof,
                    layout,
                    effect,
                    safety,
                )
                .map_err(|_| IndependentCheckerErrorV1::Packet)?,
            );
        }
        let entries = entries
            .try_into()
            .map_err(|_| IndependentCheckerErrorV1::Packet)?;
        if !reader.rest().is_empty() {
            return Err(IndependentCheckerErrorV1::Packet);
        }
        Self::verified(request, service, entries, transcript)
    }

    /// Returns the exact complete canonical result packet.
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Codec results are not protected verification evidence.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

fn validate_result(
    request: &IndependentCheckerRequestV1,
    service: &M1AllKernelsProtectedVerifierServiceRequestV1,
    entries: &[M1AllKernelsProtectedReceiptEntryV1; 12],
    transcript: [u8; 32],
) -> Result<(), IndependentCheckerErrorV1> {
    let claims = service.request_claims();
    let hsaco = &request.begin.payloads()[1];
    if transcript == [0; 32]
        || *service.trust_policy_identity().as_bytes() != request.context.policy
        || service.compiler_claims() != &request.compiler
        || claims.challenge_identity() != *request.begin.challenge().as_bytes()
        || claims.roster_identity() != *request.begin.roster_identity().as_bytes()
        || claims.finalized_hsaco_sha256() != *hsaco.sha256()
        || claims.finalized_hsaco_length() != hsaco.byte_len()
    {
        return Err(IndependentCheckerErrorV1::Association);
    }
    for ((expected, declared), result) in request
        .begin
        .entries()
        .iter()
        .zip(service.entries())
        .zip(entries)
    {
        if u32::from(result.ordinal()) != expected.ordinal()
            || result.ordinal() != declared.ordinal()
            || result.lineage_identity() != *expected.lineage_identity()
            || result.lineage_identity() != declared.lineage_identity()
            || result.marker_binding_identity() != *expected.marker_binding_identity()
            || result.marker_binding_identity() != declared.marker_binding_identity()
            || result.generated_host_contract_identity()
                != *expected.generated_host_contract_identity()
            || result.generated_host_contract_identity()
                != declared.generated_host_contract_identity()
            || result.safety_properties() != WorkerV3SafetyPropertiesV1::required()
        {
            return Err(IndependentCheckerErrorV1::Association);
        }
    }
    Ok(())
}

/// Move-only descriptor client implementing the actual protected service checker contract.
pub struct PreopenedIndependentCheckerClientV1 {
    peer: Option<OwnedFd>,
    endpoint: IndependentCheckerEndpointV1,
    context: IndependentCheckerContextV1,
    next_sequence: u64,
}
impl PreopenedIndependentCheckerClientV1 {
    /// Admits one fresh, exclusive, connected nonblocking/CLOEXEC checker endpoint.
    ///
    /// # Safety
    /// The supervisor must independently authenticate the exact measured checker,
    /// protocol, trust policy and PID/UID/GID. The checker must receive and verify
    /// every exact input byte, replay finalization, validate all proof inputs, and
    /// prove every required safety property for all twelve ordered entries before
    /// returning Verified. It must authenticate its verifier caller and enforce
    /// fresh durable admission identities and exact sequences across restarts.
    /// This client must exclusively own a fresh connection with no queued traffic
    /// or retained duplicates. Session identities must never be reused. Peer loss
    /// must cancel the checker's pending operation; the sole caller deadline is
    /// never extended. Socket metadata and inert codecs establish none of these
    /// measurement, theorem, durable freshness, or process isolation obligations.
    ///
    /// # Errors
    /// Rejected admission returns the exact supplied descriptor without sending.
    pub unsafe fn admit_from_supervisor(
        peer: OwnedFd,
        endpoint: IndependentCheckerEndpointV1,
        context: IndependentCheckerContextV1,
    ) -> Result<Self, IndependentCheckerAdmissionFailureV1> {
        Self::admit_inner::<true>(peer, endpoint, context)
    }

    fn admit_inner<const DISTINCT: bool>(
        peer: OwnedFd,
        endpoint: IndependentCheckerEndpointV1,
        context: IndependentCheckerContextV1,
    ) -> Result<Self, IndependentCheckerAdmissionFailureV1> {
        let result =
            validate_endpoint::<DISTINCT>(&peer, endpoint).and_then(|()| require_empty(&peer));
        if let Err(error) = result {
            return Err(IndependentCheckerAdmissionFailureV1 { error, peer });
        }
        Ok(Self {
            peer: Some(peer),
            endpoint,
            context,
            next_sequence: 1,
        })
    }

    /// Reports whether ambiguous traffic permanently closed this client.
    pub const fn is_poisoned(&self) -> bool {
        self.peer.is_none()
    }

    /// Executes the existing service input under its original absolute deadline.
    ///
    /// # Errors
    /// Returns typed retained/poisoned custody; no artifact data is included in errors.
    pub fn verify_all_kernels_bounded(
        &mut self,
        input: &IndependentCheckerInputV1<'_>,
    ) -> Result<IndependentCheckerVerifiedClaimsV1, IndependentCheckerFailureV1> {
        self.verify_inner::<true>(input)
    }

    fn verify_inner<const DISTINCT: bool>(
        &mut self,
        input: &IndependentCheckerInputV1<'_>,
    ) -> Result<IndependentCheckerVerifiedClaimsV1, IndependentCheckerFailureV1> {
        if self.peer.is_none() {
            return Err(self.failure(IndependentCheckerErrorV1::Poisoned));
        }
        if self.next_sequence == 0 {
            return Err(self.failure(IndependentCheckerErrorV1::Bound));
        }
        require_live(input.deadline()).map_err(|error| self.failure(error))?;
        let request = IndependentCheckerRequestV1::new(self.context, self.next_sequence, input)
            .map_err(|error| self.failure(error))?;
        let response = self.exchange::<DISTINCT>(
            &request,
            input.envelope_bytes(),
            input.hsaco_bytes(),
            input.deadline(),
        )?;
        let (Some(service), Some(entries)) = (response.service, response.entries) else {
            return Err(self.failure(IndependentCheckerErrorV1::Rejected));
        };
        // SAFETY: this is the sole authority transition, after full-byte transfer and
        // an exact correlated result from the explicitly unsafe-admitted checker.
        // Its separately measured theorem obligations are stated at admission.
        unsafe {
            IndependentCheckerVerifiedClaimsV1::from_independent_checker(
                *service.request_claims(),
                entries,
                response.transcript,
            )
        }
        .map_err(|_| {
            self.peer = None;
            self.failure(IndependentCheckerErrorV1::Association)
        })
    }

    fn exchange<const DISTINCT: bool>(
        &mut self,
        request: &IndependentCheckerRequestV1,
        envelope: &[u8],
        hsaco: &[u8],
        deadline: AbsoluteSessionDeadlineV1,
    ) -> Result<IndependentCheckerResponseV1, IndependentCheckerFailureV1> {
        require_live(deadline).map_err(|error| self.failure(error))?;
        let Some(peer) = self.peer.take() else {
            return Err(self.failure(IndependentCheckerErrorV1::Poisoned));
        };
        let preflight =
            validate_endpoint::<DISTINCT>(&peer, self.endpoint).and_then(|()| require_empty(&peer));
        if let Err(error) = preflight {
            return Err(self.failure(error));
        }
        if let Err(error) = require_live(deadline) {
            self.peer = Some(peer);
            return Err(self.failure(error));
        }
        self.next_sequence = self.next_sequence.checked_add(1).unwrap_or(0);
        let result = (|| {
            send_packet(&peer, request.canonical_bytes(), deadline)?;
            for (kind, bytes) in [envelope, hsaco].into_iter().enumerate() {
                for (ordinal, chunk) in bytes.chunks(INDEPENDENT_CHECKER_CHUNK_BYTES_V1).enumerate()
                {
                    require_empty(&peer)?;
                    let offset = ordinal
                        .checked_mul(INDEPENDENT_CHECKER_CHUNK_BYTES_V1)
                        .ok_or(IndependentCheckerErrorV1::Bound)?;
                    let packet = payload_packet(request, kind, offset, chunk)?;
                    send_packet(&peer, &packet, deadline)?;
                }
            }
            let bytes = receive_packet(&peer, deadline)?;
            validate_endpoint::<DISTINCT>(&peer, self.endpoint)?;
            require_empty(&peer)?;
            let response = IndependentCheckerResponseV1::decode_canonical(&bytes, request)?;
            require_live(deadline)?;
            Ok(response)
        })();
        match result {
            Ok(response) => {
                self.peer = Some(peer);
                Ok(response)
            }
            Err(error) => Err(self.failure(error)),
        }
    }

    fn failure(&self, error: IndependentCheckerErrorV1) -> IndependentCheckerFailureV1 {
        IndependentCheckerFailureV1 {
            error,
            custody: if self.peer.is_some() {
                IndependentCheckerCustodyV1::Retained
            } else {
                IndependentCheckerCustodyV1::Poisoned
            },
        }
    }
}

// SAFETY: the only public constructor requires the complete independently
// measured checker contract. Every success transfers all bytes and binds the
// exact request, policy, session, sequence and twelve results; ambiguous I/O closes.
unsafe impl IndependentCheckerProviderV1 for PreopenedIndependentCheckerClientV1 {
    type Error = IndependentCheckerFailureV1;
    fn measurement_identity(&self) -> [u8; 32] {
        self.context.measurement
    }
    fn verify_all_kernels(
        &mut self,
        input: IndependentCheckerInputV1<'_>,
    ) -> Result<IndependentCheckerVerifiedClaimsV1, Self::Error> {
        self.verify_all_kernels_bounded(&input)
    }
}

fn validate_endpoint<const DISTINCT: bool>(
    peer: &OwnedFd,
    endpoint: IndependentCheckerEndpointV1,
) -> Result<(), IndependentCheckerErrorV1> {
    crate::current_record_ipc::validate_endpoint::<DISTINCT>(peer, endpoint.object, endpoint.peer)
        .map_err(IndependentCheckerErrorV1::Endpoint)
}

fn require_empty(peer: &OwnedFd) -> Result<(), IndependentCheckerErrorV1> {
    let mut event = libc::pollfd {
        fd: peer.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: the one initialized pollfd remains live throughout the bounded poll.
    let result = unsafe { libc::poll(&raw mut event, 1, 0) };
    if result < 0 {
        return Err(IndependentCheckerErrorV1::Io(io::Error::last_os_error()));
    }
    if event.revents != 0 {
        return Err(IndependentCheckerErrorV1::UnexpectedData);
    }
    Ok(())
}

fn wait(
    peer: &OwnedFd,
    events: i16,
    deadline: AbsoluteSessionDeadlineV1,
) -> Result<(), IndependentCheckerErrorV1> {
    loop {
        let remaining = deadline
            .remaining()
            .ok_or(IndependentCheckerErrorV1::Deadline)?;
        let millis = remaining
            .as_millis()
            .saturating_add(u128::from(remaining.subsec_nanos() % 1_000_000 != 0));
        let timeout = i32::try_from(millis).unwrap_or(i32::MAX);
        let mut event = libc::pollfd {
            fd: peer.as_raw_fd(),
            events,
            revents: 0,
        };
        // SAFETY: poll receives one initialized live event and a bounded timeout.
        let result = unsafe { libc::poll(&raw mut event, 1, timeout) };
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(IndependentCheckerErrorV1::Io(error));
        }
        require_live(deadline)?;
        if event.revents & (libc::POLLERR | libc::POLLNVAL) != 0 {
            return Err(IndependentCheckerErrorV1::Packet);
        }
        if event.revents & events != 0 {
            return Ok(());
        }
        if event.revents & libc::POLLHUP != 0 {
            return Err(IndependentCheckerErrorV1::Packet);
        }
    }
}

fn send_packet(
    peer: &OwnedFd,
    bytes: &[u8],
    deadline: AbsoluteSessionDeadlineV1,
) -> Result<(), IndependentCheckerErrorV1> {
    loop {
        wait(peer, libc::POLLOUT, deadline)?;
        // SAFETY: the initialized packet slice remains live, and the socket is nonblocking.
        let count = unsafe {
            libc::send(
                peer.as_raw_fd(),
                bytes.as_ptr().cast(),
                bytes.len(),
                libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
            )
        };
        if count < 0 {
            let error = io::Error::last_os_error();
            if matches!(
                error.kind(),
                io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
            ) {
                continue;
            }
            return Err(IndependentCheckerErrorV1::Io(error));
        }
        if usize::try_from(count).ok() != Some(bytes.len()) {
            return Err(IndependentCheckerErrorV1::Packet);
        }
        return require_live(deadline);
    }
}

fn receive_packet(
    peer: &OwnedFd,
    deadline: AbsoluteSessionDeadlineV1,
) -> Result<Vec<u8>, IndependentCheckerErrorV1> {
    let mut bytes = vec![0_u8; INDEPENDENT_CHECKER_MAX_PACKET_BYTES_V1];
    loop {
        wait(peer, libc::POLLIN, deadline)?;
        let mut iov = libc::iovec {
            iov_base: bytes.as_mut_ptr().cast(),
            iov_len: bytes.len(),
        };
        // SAFETY: all-zero msghdr has null optional pointers; iov is installed below.
        let mut message: libc::msghdr = unsafe { mem::zeroed() };
        message.msg_iov = &raw mut iov;
        message.msg_iovlen = 1;
        // No ancillary buffer: Linux closes unreturned SCM_RIGHTS descriptors and
        // sets MSG_CTRUNC, which is rejected rather than ignored.
        // SAFETY: the writable iovec and msghdr are valid for this nonblocking call.
        let count = unsafe {
            libc::recvmsg(
                peer.as_raw_fd(),
                &raw mut message,
                libc::MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC,
            )
        };
        if count < 0 {
            let error = io::Error::last_os_error();
            if matches!(
                error.kind(),
                io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
            ) {
                continue;
            }
            return Err(IndependentCheckerErrorV1::Io(error));
        }
        require_live(deadline)?;
        let length = usize::try_from(count).map_err(|_| IndependentCheckerErrorV1::Packet)?;
        if length == 0
            || length > bytes.len()
            || message.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC) != 0
            || message.msg_controllen != 0
        {
            return Err(IndependentCheckerErrorV1::Packet);
        }
        bytes.truncate(length);
        return Ok(bytes);
    }
}

fn validate_payload_bytes(
    request: &IndependentCheckerRequestV1,
    envelope: &[u8],
    hsaco: &[u8],
    deadline: Instant,
) -> Result<(), IndependentCheckerErrorV1> {
    let lengths = request.payload_lengths()?;
    for ((bytes, expected), descriptor) in [envelope, hsaco]
        .into_iter()
        .zip(lengths)
        .zip(request.begin.payloads())
    {
        if bytes.len() != expected {
            return Err(IndependentCheckerErrorV1::Payload);
        }
        let mut digest = Sha256::new();
        for chunk in bytes.chunks(INDEPENDENT_CHECKER_CHUNK_BYTES_V1) {
            require_instant_live(deadline)?;
            digest.update(chunk);
        }
        let actual: [u8; 32] = digest.finalize().into();
        if actual != *descriptor.sha256() {
            return Err(IndependentCheckerErrorV1::Payload);
        }
    }
    require_instant_live(deadline)
}

fn payload_packet(
    request: &IndependentCheckerRequestV1,
    kind: usize,
    offset: usize,
    bytes: &[u8],
) -> Result<Vec<u8>, IndependentCheckerErrorV1> {
    let lengths = request.payload_lengths()?;
    if kind >= 2
        || offset >= lengths[kind]
        || !offset.is_multiple_of(INDEPENDENT_CHECKER_CHUNK_BYTES_V1)
        || bytes.len() != (lengths[kind] - offset).min(INDEPENDENT_CHECKER_CHUNK_BYTES_V1)
    {
        return Err(IndependentCheckerErrorV1::Payload);
    }
    let mut packet = header(PAYLOAD, request.sequence, request.identity);
    packet.extend_from_slice(&(kind as u64).to_le_bytes());
    packet.extend_from_slice(&(offset as u64).to_le_bytes());
    packet.extend_from_slice(bytes);
    finalize_length(&mut packet)?;
    Ok(packet)
}

fn header(kind: u16, sequence: u64, identity: [u8; 32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HEADER);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(&kind.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&sequence.to_le_bytes());
    bytes.extend_from_slice(&identity);
    bytes
}
fn finalize_length(bytes: &mut [u8]) -> Result<(), IndependentCheckerErrorV1> {
    if bytes.len() > INDEPENDENT_CHECKER_MAX_PACKET_BYTES_V1 {
        return Err(IndependentCheckerErrorV1::Bound);
    }
    let length = u32::try_from(bytes.len()).map_err(|_| IndependentCheckerErrorV1::Bound)?;
    bytes[12..16].copy_from_slice(&length.to_le_bytes());
    Ok(())
}
fn packet_kind(bytes: &[u8]) -> Result<u16, IndependentCheckerErrorV1> {
    if bytes.len() < HEADER
        || bytes.len() > INDEPENDENT_CHECKER_MAX_PACKET_BYTES_V1
        || &bytes[..8] != MAGIC
        || bytes[8..10] != VERSION.to_le_bytes()
    {
        return Err(IndependentCheckerErrorV1::Packet);
    }
    Ok(u16::from_le_bytes(
        bytes[10..12]
            .try_into()
            .map_err(|_| IndependentCheckerErrorV1::Packet)?,
    ))
}
fn check_header(bytes: &[u8], kind: u16) -> Result<(u64, [u8; 32]), IndependentCheckerErrorV1> {
    if packet_kind(bytes)? != kind {
        return Err(IndependentCheckerErrorV1::Packet);
    }
    let mut reader = Reader::new(&bytes[12..HEADER]);
    if u32::from_le_bytes(reader.array()?) as usize != bytes.len() {
        return Err(IndependentCheckerErrorV1::Packet);
    }
    let sequence = reader.u64()?;
    let identity = reader.array()?;
    if sequence == 0 || identity == [0; 32] {
        return Err(IndependentCheckerErrorV1::Context);
    }
    Ok((sequence, identity))
}
fn finish_response(bytes: &mut Vec<u8>) -> Result<(), IndependentCheckerErrorV1> {
    bytes.extend_from_slice(&[0; 32]);
    finalize_length(bytes)?;
    let offset = bytes.len() - 32;
    let digest = hash(&[RESPONSE_DOMAIN, &bytes[..offset]]);
    bytes[offset..].copy_from_slice(&digest);
    Ok(())
}
fn require_live(deadline: AbsoluteSessionDeadlineV1) -> Result<(), IndependentCheckerErrorV1> {
    deadline
        .remaining()
        .map(|_| ())
        .ok_or(IndependentCheckerErrorV1::Deadline)
}
fn require_instant_live(deadline: Instant) -> Result<(), IndependentCheckerErrorV1> {
    if Instant::now() >= deadline {
        Err(IndependentCheckerErrorV1::Deadline)
    } else {
        Ok(())
    }
}
fn hash(parts: &[&[u8]]) -> [u8; 32] {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update(part);
    }
    digest.finalize().into()
}
fn encode_compiler(bytes: &mut Vec<u8>, claims: &M1AllKernelsProtectedReceiptCompilerClaimsV1) {
    for identity in [
        claims.subject_sha256(),
        claims.carriage_sha256(),
        claims.policy_sha256(),
        claims.issuer_journal_sha256(),
        claims.compiler_occurrence_sha256(),
        claims.receipt_sha256(),
        claims.publication_sha256(),
        claims.acknowledgment_sha256(),
        claims.worker_ledger_record_sha256(),
    ] {
        bytes.extend_from_slice(&identity);
    }
    bytes.extend_from_slice(&claims.sequence().to_le_bytes());
    for identity in [
        claims.prior_rollback_anchor(),
        claims.current_rollback_anchor(),
        claims.current_record_verification_sha256(),
        claims.current_record_attestation_sha256(),
        claims.protected_policy_verification_sha256(),
        claims.protected_worker_ledger_verification_sha256(),
        claims.external_rollback_verification_sha256(),
    ] {
        bytes.extend_from_slice(&identity);
    }
}
fn decode_compiler(
    reader: &mut Reader<'_>,
) -> Result<M1AllKernelsProtectedReceiptCompilerClaimsV1, IndependentCheckerErrorV1> {
    M1AllKernelsProtectedReceiptCompilerClaimsV1::new(
        reader.array()?,
        reader.array()?,
        reader.array()?,
        reader.array()?,
        reader.array()?,
        reader.array()?,
        reader.array()?,
        reader.array()?,
        reader.array()?,
        reader.u64()?,
        reader.array()?,
        reader.array()?,
        reader.array()?,
        reader.array()?,
        reader.array()?,
        reader.array()?,
        reader.array()?,
    )
    .map_err(|_| IndependentCheckerErrorV1::Packet)
}
struct Reader<'a> {
    bytes: &'a [u8],
}
impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }
    fn take(&mut self, length: usize) -> Result<&'a [u8], IndependentCheckerErrorV1> {
        let result = self
            .bytes
            .get(..length)
            .ok_or(IndependentCheckerErrorV1::Packet)?;
        self.bytes = &self.bytes[length..];
        Ok(result)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], IndependentCheckerErrorV1> {
        self.take(N)?
            .try_into()
            .map_err(|_| IndependentCheckerErrorV1::Packet)
    }
    fn u64(&mut self) -> Result<u64, IndependentCheckerErrorV1> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    const fn rest(&self) -> &'a [u8] {
        self.bytes
    }
}

#[cfg(test)]
#[path = "checker_ipc_tests.rs"]
mod tests;
