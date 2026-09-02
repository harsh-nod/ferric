#![allow(
    clippy::must_use_candidate,
    reason = "public values are inert identities or diagnostic accessors"
)]

use std::error::Error;
use std::fmt;
use std::io;
use std::os::fd::{AsFd, OwnedFd};

use fe2o3_worker_v3_verification_protocol::{
    WorkerV3VerificationChallengeReservationV2, WorkerV3VerificationFreshChallengeV1,
    WorkerV3VerificationPolicyIdentityV1, WorkerV3VerificationRequestV1,
};
use fe2o3_worker_v3_verification_service::{
    WorkerV3VerificationCallerV1, WorkerV3VerificationChallengeReplayGuardV1,
    WorkerV3VerificationChallengeReservationProviderV2,
};
use rustix::fs::{FlockOperation, flock};
use sha2::{Digest, Sha256};

const HEADER_MAGIC: [u8; 8] = *b"FRV2LED1";
const RECORD_MAGIC: [u8; 8] = *b"FRV2REC1";
const VERSION: u16 = 1;
const HEADER_BYTES: usize = 80;
const RECORD_BYTES: usize = 224;
const RECORD_BYTES_U32: u32 = 224;
const REPLAY_KIND: u16 = 1;
const RESERVATION_KIND: u16 = 2;
const HEADER_DOMAIN: &[u8] = b"FERRIC/M1/PROTECTED-VERIFIER/DURABLE-LEDGER-HEADER/V1\0";
const RECORD_DOMAIN: &[u8] = b"FERRIC/M1/PROTECTED-VERIFIER/DURABLE-LEDGER-RECORD/V1\0";
const MAX_ENTROPY_ATTEMPTS: usize = 16;

/// Explicit supervisor-pinned identity of one preopened durable ledger object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LedgerObjectIdentityV1 {
    device: u64,
    inode: u64,
    uid: u32,
    gid: u32,
}

impl LedgerObjectIdentityV1 {
    /// Constructs an identity captured by the provisioning authority.
    pub const fn new(device: u64, inode: u64, uid: u32, gid: u32) -> Self {
        Self {
            device,
            inode,
            uid,
            gid,
        }
    }

    fn matches(self, file: &impl AsFd) -> Result<bool, DurableLedgerErrorV1> {
        let stat = rustix::fs::fstat(file)
            .map_err(|source| DurableLedgerErrorV1::Inspect(source.into()))?;
        let descriptor_flags = rustix::io::fcntl_getfd(file)
            .map_err(|source| DurableLedgerErrorV1::Inspect(source.into()))?;
        let status = rustix::fs::fcntl_getfl(file)
            .map_err(|source| DurableLedgerErrorV1::Inspect(source.into()))?;
        Ok(stat.st_dev == self.device
            && stat.st_ino == self.inode
            && stat.st_uid == self.uid
            && stat.st_gid == self.gid
            && stat.st_nlink != 0
            && rustix::fs::FileType::from_raw_mode(stat.st_mode)
                == rustix::fs::FileType::RegularFile
            && descriptor_flags.contains(rustix::io::FdFlags::CLOEXEC)
            && status & rustix::fs::OFlags::ACCMODE == rustix::fs::OFlags::RDWR
            && !status.intersects(
                rustix::fs::OFlags::APPEND
                    | rustix::fs::OFlags::ASYNC
                    | rustix::fs::OFlags::DIRECT
                    | rustix::fs::OFlags::PATH,
            ))
    }
}

/// Explicit supervisor-pinned identity of the preopened nonblocking CSPRNG character device.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntropyObjectIdentityV1 {
    device: u64,
    inode: u64,
    raw_device: u64,
    mode: u32,
    uid: u32,
    gid: u32,
}

impl EntropyObjectIdentityV1 {
    /// Constructs an exact identity captured by the provisioning authority.
    pub const fn new(
        device: u64,
        inode: u64,
        raw_device: u64,
        mode: u32,
        uid: u32,
        gid: u32,
    ) -> Self {
        Self {
            device,
            inode,
            raw_device,
            mode,
            uid,
            gid,
        }
    }

    fn matches(self, file: &impl AsFd) -> Result<bool, DurableLedgerErrorV1> {
        let stat = rustix::fs::fstat(file)
            .map_err(|source| DurableLedgerErrorV1::Inspect(source.into()))?;
        let descriptor_flags = rustix::io::fcntl_getfd(file)
            .map_err(|source| DurableLedgerErrorV1::Inspect(source.into()))?;
        let status = rustix::fs::fcntl_getfl(file)
            .map_err(|source| DurableLedgerErrorV1::Inspect(source.into()))?;
        Ok(stat.st_dev == self.device
            && stat.st_ino == self.inode
            && stat.st_rdev == self.raw_device
            && stat.st_mode == self.mode
            && stat.st_uid == self.uid
            && stat.st_gid == self.gid
            && rustix::fs::FileType::from_raw_mode(stat.st_mode)
                == rustix::fs::FileType::CharacterDevice
            && descriptor_flags.contains(rustix::io::FdFlags::CLOEXEC)
            && status & rustix::fs::OFlags::ACCMODE == rustix::fs::OFlags::RDONLY
            && status.contains(rustix::fs::OFlags::NONBLOCK)
            && !status.intersects(
                rustix::fs::OFlags::APPEND
                    | rustix::fs::OFlags::ASYNC
                    | rustix::fs::OFlags::DIRECT
                    | rustix::fs::OFlags::PATH,
            ))
    }
}

/// Durable ledger validation or update failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum DurableLedgerErrorV1 {
    /// The pinned descriptor identity or file type did not match.
    ObjectIdentityMismatch,
    /// The namespace identity used the forbidden zero sentinel.
    ZeroNamespace,
    /// Descriptor inspection failed.
    Inspect(io::Error),
    /// The descriptor could not be exclusively locked.
    Lock(io::Error),
    /// The complete ledger could not be read.
    Read(io::Error),
    /// A durable append or initial provisioning write failed.
    Write(io::Error),
    /// The durable update could not be synchronized.
    Synchronize(io::Error),
    /// The descriptor did not contain the exact expected ledger schema.
    Corrupt(&'static str),
    /// The descriptor was not empty during initial provisioning.
    AlreadyProvisioned,
    /// The entropy descriptor failed before a complete candidate was read.
    Entropy(io::Error),
    /// No unique nonzero challenge/reservation pair was obtained within the fixed bound.
    EntropyExhausted,
}

impl fmt::Display for DurableLedgerErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObjectIdentityMismatch => {
                formatter.write_str("preopened durable ledger identity mismatch")
            }
            Self::ZeroNamespace => formatter.write_str("durable ledger namespace is zero"),
            Self::Inspect(source) => write!(formatter, "inspect durable ledger: {source}"),
            Self::Lock(source) => write!(formatter, "lock durable ledger: {source}"),
            Self::Read(source) => write!(formatter, "read durable ledger: {source}"),
            Self::Write(source) => write!(formatter, "write durable ledger: {source}"),
            Self::Synchronize(source) => write!(formatter, "synchronize durable ledger: {source}"),
            Self::Corrupt(field) => write!(formatter, "durable ledger is corrupt: {field}"),
            Self::AlreadyProvisioned => {
                formatter.write_str("durable ledger is already provisioned")
            }
            Self::Entropy(source) => write!(formatter, "read preopened entropy source: {source}"),
            Self::EntropyExhausted => formatter
                .write_str("preopened entropy source did not produce a unique nonzero reservation"),
        }
    }
}

impl Error for DurableLedgerErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Inspect(source)
            | Self::Lock(source)
            | Self::Read(source)
            | Self::Write(source)
            | Self::Synchronize(source)
            | Self::Entropy(source) => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LedgerKindV1 {
    Replay,
    Reservation,
}

impl LedgerKindV1 {
    const fn tag(self) -> u16 {
        match self {
            Self::Replay => REPLAY_KIND,
            Self::Reservation => RESERVATION_KIND,
        }
    }
}

struct DurableLedgerV1 {
    file: OwnedFd,
    object: LedgerObjectIdentityV1,
    namespace: [u8; 32],
    kind: LedgerKindV1,
}

impl DurableLedgerV1 {
    fn from_preopened(
        file: OwnedFd,
        object: LedgerObjectIdentityV1,
        namespace: [u8; 32],
        kind: LedgerKindV1,
    ) -> Result<Self, DurableLedgerErrorV1> {
        require_namespace(namespace)?;
        if !object.matches(&file)? {
            return Err(DurableLedgerErrorV1::ObjectIdentityMismatch);
        }
        let ledger = Self {
            file,
            object,
            namespace,
            kind,
        };
        {
            let _lock = LedgerLockV1::acquire(&ledger.file)?;
            ledger.read_validated_records()?;
        }
        Ok(ledger)
    }

    fn append_if_absent(&self, candidate: LedgerRecordV1) -> Result<bool, DurableLedgerErrorV1> {
        if !self.object.matches(&self.file)? {
            return Err(DurableLedgerErrorV1::ObjectIdentityMismatch);
        }
        let _lock = LedgerLockV1::acquire(&self.file)?;
        let records = self.read_validated_records()?;
        if records.iter().any(|existing| existing.conflicts(candidate)) {
            return Ok(false);
        }
        let previous = records.last().map_or([0; 32], |record| record.identity);
        let bytes = candidate.encode(previous);
        let records_bytes = records
            .len()
            .checked_mul(RECORD_BYTES)
            .ok_or(DurableLedgerErrorV1::Corrupt("record offset overflow"))?;
        let offset = HEADER_BYTES
            .checked_add(records_bytes)
            .ok_or(DurableLedgerErrorV1::Corrupt("record offset overflow"))?;
        pwrite_all(&self.file, &bytes, offset as u64)?;
        rustix::fs::fdatasync(&self.file)
            .map_err(|source| DurableLedgerErrorV1::Synchronize(source.into()))?;
        Ok(true)
    }

    fn read_validated_records(&self) -> Result<Vec<DecodedRecordV1>, DurableLedgerErrorV1> {
        let stat = rustix::fs::fstat(&self.file)
            .map_err(|source| DurableLedgerErrorV1::Inspect(source.into()))?;
        let length = usize::try_from(stat.st_size)
            .map_err(|_| DurableLedgerErrorV1::Corrupt("negative or oversized length"))?;
        if length < HEADER_BYTES || !(length - HEADER_BYTES).is_multiple_of(RECORD_BYTES) {
            return Err(DurableLedgerErrorV1::Corrupt("torn length"));
        }
        let mut bytes = vec![0_u8; length];
        pread_all(&self.file, &mut bytes, 0)?;
        validate_header(&bytes[..HEADER_BYTES], self.kind, self.namespace)?;
        let mut previous = [0; 32];
        let mut records: Vec<DecodedRecordV1> =
            Vec::with_capacity((length - HEADER_BYTES) / RECORD_BYTES);
        for chunk in bytes[HEADER_BYTES..].chunks_exact(RECORD_BYTES) {
            let record = decode_record(chunk, self.kind, previous)?;
            if records.iter().any(|prior| prior.conflicts_decoded(record)) {
                return Err(DurableLedgerErrorV1::Corrupt(
                    "duplicate durable coordinate",
                ));
            }
            previous = record.identity;
            records.push(record);
        }
        Ok(records)
    }
}

struct LedgerLockV1<'a> {
    file: &'a OwnedFd,
}

impl<'a> LedgerLockV1<'a> {
    fn acquire(file: &'a OwnedFd) -> Result<Self, DurableLedgerErrorV1> {
        flock(file, FlockOperation::NonBlockingLockExclusive)
            .map_err(|source| DurableLedgerErrorV1::Lock(source.into()))?;
        Ok(Self { file })
    }
}

impl Drop for LedgerLockV1<'_> {
    fn drop(&mut self) {
        let _ = flock(self.file, FlockOperation::Unlock);
    }
}

#[derive(Clone, Copy)]
struct LedgerRecordV1 {
    caller: [u32; 3],
    policy: [u8; 32],
    primary: [u8; 32],
    secondary: [u8; 32],
    request: [u8; 32],
    kind: LedgerKindV1,
}

impl LedgerRecordV1 {
    fn encode(self, previous: [u8; 32]) -> [u8; RECORD_BYTES] {
        let mut bytes = [0_u8; RECORD_BYTES];
        let mut offset = 0;
        put(&mut bytes, &mut offset, &RECORD_MAGIC);
        put(&mut bytes, &mut offset, &VERSION.to_le_bytes());
        put(&mut bytes, &mut offset, &self.kind.tag().to_le_bytes());
        put(&mut bytes, &mut offset, &RECORD_BYTES_U32.to_le_bytes());
        put(&mut bytes, &mut offset, &self.caller[0].to_le_bytes());
        put(&mut bytes, &mut offset, &self.caller[1].to_le_bytes());
        put(&mut bytes, &mut offset, &self.caller[2].to_le_bytes());
        put(&mut bytes, &mut offset, &0_u32.to_le_bytes());
        put(&mut bytes, &mut offset, &self.policy);
        put(&mut bytes, &mut offset, &self.primary);
        put(&mut bytes, &mut offset, &self.secondary);
        put(&mut bytes, &mut offset, &self.request);
        put(&mut bytes, &mut offset, &previous);
        let identity = hash_parts(&[RECORD_DOMAIN, &bytes[..offset]]);
        put(&mut bytes, &mut offset, &identity);
        debug_assert_eq!(offset, RECORD_BYTES);
        bytes
    }
}

#[derive(Clone, Copy)]
struct DecodedRecordV1 {
    caller: [u32; 3],
    policy: [u8; 32],
    primary: [u8; 32],
    secondary: [u8; 32],
    request: [u8; 32],
    identity: [u8; 32],
    kind: LedgerKindV1,
}

impl DecodedRecordV1 {
    fn conflicts(self, candidate: LedgerRecordV1) -> bool {
        match candidate.kind {
            LedgerKindV1::Replay => {
                self.kind == LedgerKindV1::Replay
                    && self.caller == candidate.caller
                    && self.policy == candidate.policy
                    && self.primary == candidate.primary
            }
            LedgerKindV1::Reservation => {
                self.kind == LedgerKindV1::Reservation
                    && (self.primary == candidate.primary
                        || self.secondary == candidate.secondary
                        || self.request == candidate.request)
            }
        }
    }

    fn conflicts_decoded(self, candidate: Self) -> bool {
        match candidate.kind {
            LedgerKindV1::Replay => {
                self.kind == LedgerKindV1::Replay
                    && self.caller == candidate.caller
                    && self.policy == candidate.policy
                    && self.primary == candidate.primary
            }
            LedgerKindV1::Reservation => {
                self.kind == LedgerKindV1::Reservation
                    && (self.primary == candidate.primary
                        || self.secondary == candidate.secondary
                        || self.request == candidate.request)
            }
        }
    }
}

/// Durable caller-challenge replay guard backed only by a preopened descriptor.
pub struct DurableReplayGuardV1 {
    ledger: DurableLedgerV1,
    last_error: Option<DurableLedgerErrorV1>,
}

impl DurableReplayGuardV1 {
    /// Opens and fully validates an existing replay ledger.
    ///
    /// # Errors
    ///
    /// Returns a typed error if the descriptor identity, schema, chain, or lock is invalid.
    pub fn from_preopened(
        file: OwnedFd,
        object: LedgerObjectIdentityV1,
        namespace: [u8; 32],
    ) -> Result<Self, DurableLedgerErrorV1> {
        Ok(Self {
            ledger: DurableLedgerV1::from_preopened(file, object, namespace, LedgerKindV1::Replay)?,
            last_error: None,
        })
    }

    /// Returns the most recent fail-closed storage error, if any.
    pub const fn last_error(&self) -> Option<&DurableLedgerErrorV1> {
        self.last_error.as_ref()
    }
}

impl WorkerV3VerificationChallengeReplayGuardV1 for DurableReplayGuardV1 {
    fn admit_fresh_challenge(
        &mut self,
        caller: WorkerV3VerificationCallerV1,
        policy: WorkerV3VerificationPolicyIdentityV1,
        challenge: WorkerV3VerificationFreshChallengeV1,
    ) -> bool {
        let record = LedgerRecordV1 {
            caller: [caller.pid(), caller.uid(), caller.gid()],
            policy: *policy.as_bytes(),
            primary: *challenge.as_bytes(),
            secondary: [0; 32],
            request: [0; 32],
            kind: LedgerKindV1::Replay,
        };
        match self.ledger.append_if_absent(record) {
            Ok(admitted) => admitted,
            Err(error) => {
                self.last_error = Some(error);
                false
            }
        }
    }
}

/// Durable service-challenge provider using a supervisor-supplied entropy descriptor.
pub struct DurableReservationProviderV2 {
    ledger: DurableLedgerV1,
    entropy: OwnedFd,
    last_error: Option<DurableLedgerErrorV1>,
}

impl DurableReservationProviderV2 {
    /// Opens an existing reservation ledger and takes the preopened entropy source.
    /// The supervisor must pin a deployment-reviewed kernel CSPRNG device; regular
    /// files, blocking descriptors, and substituted device identities are rejected.
    ///
    /// # Errors
    ///
    /// Returns a typed error if either descriptor identity or the durable ledger is invalid.
    pub fn from_preopened(
        ledger: OwnedFd,
        object: LedgerObjectIdentityV1,
        namespace: [u8; 32],
        entropy: OwnedFd,
        entropy_identity: EntropyObjectIdentityV1,
    ) -> Result<Self, DurableLedgerErrorV1> {
        if !entropy_identity.matches(&entropy)? {
            return Err(DurableLedgerErrorV1::ObjectIdentityMismatch);
        }
        Ok(Self {
            ledger: DurableLedgerV1::from_preopened(
                ledger,
                object,
                namespace,
                LedgerKindV1::Reservation,
            )?,
            entropy,
            last_error: None,
        })
    }

    /// Returns the most recent fail-closed storage or entropy error, if any.
    pub const fn last_error(&self) -> Option<&DurableLedgerErrorV1> {
        self.last_error.as_ref()
    }

    fn try_reserve(
        &mut self,
        caller: WorkerV3VerificationCallerV1,
        request: &WorkerV3VerificationRequestV1,
    ) -> Result<WorkerV3VerificationChallengeReservationV2, DurableLedgerErrorV1> {
        self.try_reserve_coordinates(
            [caller.pid(), caller.uid(), caller.gid()],
            *request.policy_identity().as_bytes(),
            *request.identity().as_bytes(),
        )
    }

    fn try_reserve_coordinates(
        &mut self,
        caller: [u32; 3],
        policy: [u8; 32],
        request: [u8; 32],
    ) -> Result<WorkerV3VerificationChallengeReservationV2, DurableLedgerErrorV1> {
        for _ in 0..MAX_ENTROPY_ATTEMPTS {
            let mut entropy = [0_u8; 64];
            read_all(&self.entropy, &mut entropy)?;
            let challenge: [u8; 32] = entropy[..32].try_into().expect("fixed entropy prefix");
            let reservation: [u8; 32] = entropy[32..].try_into().expect("fixed entropy suffix");
            if challenge == [0; 32] || reservation == [0; 32] {
                continue;
            }
            let record = LedgerRecordV1 {
                caller,
                policy,
                primary: challenge,
                secondary: reservation,
                request,
                kind: LedgerKindV1::Reservation,
            };
            if self.ledger.append_if_absent(record)? {
                return WorkerV3VerificationChallengeReservationV2::new(challenge, reservation)
                    .map_err(|_| DurableLedgerErrorV1::EntropyExhausted);
            }
        }
        Err(DurableLedgerErrorV1::EntropyExhausted)
    }
}

impl WorkerV3VerificationChallengeReservationProviderV2 for DurableReservationProviderV2 {
    fn reserve_current_record_challenge(
        &mut self,
        caller: WorkerV3VerificationCallerV1,
        request: &WorkerV3VerificationRequestV1,
    ) -> Option<WorkerV3VerificationChallengeReservationV2> {
        match self.try_reserve(caller, request) {
            Ok(reservation) => Some(reservation),
            Err(error) => {
                self.last_error = Some(error);
                None
            }
        }
    }
}

/// Initializes one empty replay ledger on an explicitly pinned preopened file.
///
/// # Errors
///
/// Returns a typed error if the descriptor is substituted, nonempty, or cannot be synchronized.
pub fn provision_empty_replay_ledger_v1(
    file: &OwnedFd,
    object: LedgerObjectIdentityV1,
    namespace: [u8; 32],
) -> Result<(), DurableLedgerErrorV1> {
    provision_empty(file, object, namespace, LedgerKindV1::Replay)
}

/// Initializes one empty reservation ledger on an explicitly pinned preopened file.
///
/// # Errors
///
/// Returns a typed error if the descriptor is substituted, nonempty, or cannot be synchronized.
pub fn provision_empty_reservation_ledger_v2(
    file: &OwnedFd,
    object: LedgerObjectIdentityV1,
    namespace: [u8; 32],
) -> Result<(), DurableLedgerErrorV1> {
    provision_empty(file, object, namespace, LedgerKindV1::Reservation)
}

fn provision_empty(
    file: &OwnedFd,
    object: LedgerObjectIdentityV1,
    namespace: [u8; 32],
    kind: LedgerKindV1,
) -> Result<(), DurableLedgerErrorV1> {
    require_namespace(namespace)?;
    if !object.matches(file)? {
        return Err(DurableLedgerErrorV1::ObjectIdentityMismatch);
    }
    let _lock = LedgerLockV1::acquire(file)?;
    let stat =
        rustix::fs::fstat(file).map_err(|source| DurableLedgerErrorV1::Inspect(source.into()))?;
    if stat.st_size != 0 {
        return Err(DurableLedgerErrorV1::AlreadyProvisioned);
    }
    let header = encode_header(kind, namespace);
    pwrite_all(file, &header, 0)?;
    rustix::fs::fdatasync(file).map_err(|source| DurableLedgerErrorV1::Synchronize(source.into()))
}

fn encode_header(kind: LedgerKindV1, namespace: [u8; 32]) -> [u8; HEADER_BYTES] {
    let mut bytes = [0_u8; HEADER_BYTES];
    let mut offset = 0;
    put(&mut bytes, &mut offset, &HEADER_MAGIC);
    put(&mut bytes, &mut offset, &VERSION.to_le_bytes());
    put(&mut bytes, &mut offset, &kind.tag().to_le_bytes());
    put(&mut bytes, &mut offset, &RECORD_BYTES_U32.to_le_bytes());
    put(&mut bytes, &mut offset, &namespace);
    let digest = hash_parts(&[HEADER_DOMAIN, &bytes[..offset]]);
    put(&mut bytes, &mut offset, &digest);
    debug_assert_eq!(offset, HEADER_BYTES);
    bytes
}

fn validate_header(
    bytes: &[u8],
    kind: LedgerKindV1,
    namespace: [u8; 32],
) -> Result<(), DurableLedgerErrorV1> {
    if bytes.len() != HEADER_BYTES || bytes[..8] != HEADER_MAGIC {
        return Err(DurableLedgerErrorV1::Corrupt("header magic or length"));
    }
    if u16::from_le_bytes(bytes[8..10].try_into().expect("header version")) != VERSION
        || u16::from_le_bytes(bytes[10..12].try_into().expect("header kind")) != kind.tag()
        || u32::from_le_bytes(bytes[12..16].try_into().expect("record length")) != RECORD_BYTES_U32
        || bytes[16..48] != namespace
        || bytes[48..80] != hash_parts(&[HEADER_DOMAIN, &bytes[..48]])
    {
        return Err(DurableLedgerErrorV1::Corrupt("header binding"));
    }
    Ok(())
}

fn decode_record(
    bytes: &[u8],
    kind: LedgerKindV1,
    previous: [u8; 32],
) -> Result<DecodedRecordV1, DurableLedgerErrorV1> {
    if bytes.len() != RECORD_BYTES || bytes[..8] != RECORD_MAGIC {
        return Err(DurableLedgerErrorV1::Corrupt("record magic or length"));
    }
    let version = u16::from_le_bytes(bytes[8..10].try_into().expect("record version"));
    let actual_kind = u16::from_le_bytes(bytes[10..12].try_into().expect("record kind"));
    let length = u32::from_le_bytes(bytes[12..16].try_into().expect("record length"));
    if version != VERSION || actual_kind != kind.tag() || length != RECORD_BYTES_U32 {
        return Err(DurableLedgerErrorV1::Corrupt("record header"));
    }
    if bytes[28..32] != [0; 4] || bytes[160..192] != previous {
        return Err(DurableLedgerErrorV1::Corrupt("record chain"));
    }
    let identity: [u8; 32] = bytes[192..224].try_into().expect("record identity");
    if identity != hash_parts(&[RECORD_DOMAIN, &bytes[..192]]) {
        return Err(DurableLedgerErrorV1::Corrupt("record checksum"));
    }
    let primary = bytes[64..96].try_into().expect("primary");
    let secondary = bytes[96..128].try_into().expect("secondary");
    let request = bytes[128..160].try_into().expect("request");
    if primary == [0; 32]
        || (kind == LedgerKindV1::Replay && (secondary != [0; 32] || request != [0; 32]))
        || (kind == LedgerKindV1::Reservation && (secondary == [0; 32] || request == [0; 32]))
    {
        return Err(DurableLedgerErrorV1::Corrupt("record coordinates"));
    }
    Ok(DecodedRecordV1 {
        caller: [
            u32::from_le_bytes(bytes[16..20].try_into().expect("pid")),
            u32::from_le_bytes(bytes[20..24].try_into().expect("uid")),
            u32::from_le_bytes(bytes[24..28].try_into().expect("gid")),
        ],
        policy: bytes[32..64].try_into().expect("policy"),
        primary,
        secondary,
        request,
        identity,
        kind,
    })
}

fn require_namespace(namespace: [u8; 32]) -> Result<(), DurableLedgerErrorV1> {
    if namespace == [0; 32] {
        Err(DurableLedgerErrorV1::ZeroNamespace)
    } else {
        Ok(())
    }
}

fn pread_all(
    file: &impl AsFd,
    mut bytes: &mut [u8],
    mut offset: u64,
) -> Result<(), DurableLedgerErrorV1> {
    while !bytes.is_empty() {
        let count = rustix::io::pread(file, &mut *bytes, offset)
            .map_err(|source| DurableLedgerErrorV1::Read(source.into()))?;
        if count == 0 {
            return Err(DurableLedgerErrorV1::Corrupt("unexpected end of file"));
        }
        offset = offset
            .checked_add(count as u64)
            .ok_or(DurableLedgerErrorV1::Corrupt("read offset overflow"))?;
        bytes = &mut bytes[count..];
    }
    Ok(())
}

fn pwrite_all(
    file: &impl AsFd,
    mut bytes: &[u8],
    mut offset: u64,
) -> Result<(), DurableLedgerErrorV1> {
    while !bytes.is_empty() {
        let count = rustix::io::pwrite(file, bytes, offset)
            .map_err(|source| DurableLedgerErrorV1::Write(source.into()))?;
        if count == 0 {
            return Err(DurableLedgerErrorV1::Write(io::Error::new(
                io::ErrorKind::WriteZero,
                "durable ledger write returned zero",
            )));
        }
        offset = offset
            .checked_add(count as u64)
            .ok_or(DurableLedgerErrorV1::Corrupt("write offset overflow"))?;
        bytes = &bytes[count..];
    }
    Ok(())
}

fn read_all(file: &impl AsFd, mut bytes: &mut [u8]) -> Result<(), DurableLedgerErrorV1> {
    while !bytes.is_empty() {
        match rustix::io::read(file, &mut *bytes) {
            Ok(0) => {
                return Err(DurableLedgerErrorV1::Entropy(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "entropy descriptor reached EOF",
                )));
            }
            Ok(count) => bytes = &mut bytes[count..],
            Err(rustix::io::Errno::INTR) => {}
            Err(source) => return Err(DurableLedgerErrorV1::Entropy(source.into())),
        }
    }
    Ok(())
}

fn hash_parts(parts: &[&[u8]]) -> [u8; 32] {
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

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::os::fd::{AsFd, OwnedFd};

    use super::*;

    fn owned(file: &File) -> OwnedFd {
        rustix::io::fcntl_dupfd_cloexec(file, 0).unwrap()
    }

    fn identity(file: &impl AsFd) -> LedgerObjectIdentityV1 {
        let stat = rustix::fs::fstat(file).unwrap();
        LedgerObjectIdentityV1::new(stat.st_dev, stat.st_ino, stat.st_uid, stat.st_gid)
    }

    fn entropy_identity(file: &impl AsFd) -> EntropyObjectIdentityV1 {
        let stat = rustix::fs::fstat(file).unwrap();
        EntropyObjectIdentityV1::new(
            stat.st_dev,
            stat.st_ino,
            stat.st_rdev,
            stat.st_mode,
            stat.st_uid,
            stat.st_gid,
        )
    }

    fn open_entropy() -> OwnedFd {
        rustix::fs::open(
            "/dev/urandom",
            rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::CLOEXEC | rustix::fs::OFlags::NONBLOCK,
            rustix::fs::Mode::empty(),
        )
        .unwrap()
    }

    fn record(kind: LedgerKindV1, coordinate: u8) -> LedgerRecordV1 {
        LedgerRecordV1 {
            caller: [11, 12, 13],
            policy: [21; 32],
            primary: [coordinate; 32],
            secondary: if kind == LedgerKindV1::Reservation {
                [coordinate.wrapping_add(1); 32]
            } else {
                [0; 32]
            },
            request: if kind == LedgerKindV1::Reservation {
                [coordinate.wrapping_add(2); 32]
            } else {
                [0; 32]
            },
            kind,
        }
    }

    #[test]
    fn torn_or_corrupt_ledger_fails_closed_after_reopen() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let file = file.as_file();
        let object = identity(&file);
        provision_empty_replay_ledger_v1(&owned(file), object, [1; 32]).unwrap();
        rustix::io::pwrite(file, &[0xff], 0).unwrap();
        rustix::fs::fdatasync(file).unwrap();
        assert!(matches!(
            DurableReplayGuardV1::from_preopened(owned(file), object, [1; 32]),
            Err(DurableLedgerErrorV1::Corrupt(_))
        ));
    }

    #[test]
    fn nonempty_unprovisioned_file_is_never_reinitialized() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let file = file.as_file();
        rustix::io::pwrite(file, &[7], 0).unwrap();
        let object = identity(&file);
        assert!(matches!(
            provision_empty_replay_ledger_v1(&owned(file), object, [2; 32]),
            Err(DurableLedgerErrorV1::AlreadyProvisioned)
        ));
    }

    #[test]
    fn replay_admission_is_atomic_and_survives_restart() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let object = identity(file.as_file());
        provision_empty_replay_ledger_v1(&owned(file.as_file()), object, [3; 32]).unwrap();

        let first = record(LedgerKindV1::Replay, 31);
        let ledger = DurableLedgerV1::from_preopened(
            owned(file.as_file()),
            object,
            [3; 32],
            LedgerKindV1::Replay,
        )
        .unwrap();
        assert!(ledger.append_if_absent(first).unwrap());
        drop(ledger);

        let restarted = DurableLedgerV1::from_preopened(
            owned(file.as_file()),
            object,
            [3; 32],
            LedgerKindV1::Replay,
        )
        .unwrap();
        assert!(!restarted.append_if_absent(first).unwrap());
        assert!(
            restarted
                .append_if_absent(record(LedgerKindV1::Replay, 32))
                .unwrap()
        );
        assert_eq!(restarted.read_validated_records().unwrap().len(), 2);
    }

    #[test]
    fn torn_record_is_not_truncated_or_recovered_on_restart() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let object = identity(file.as_file());
        provision_empty_replay_ledger_v1(&owned(file.as_file()), object, [4; 32]).unwrap();
        rustix::io::pwrite(file.as_file(), &[0xaa], HEADER_BYTES as u64).unwrap();
        rustix::fs::fdatasync(file.as_file()).unwrap();

        assert!(matches!(
            DurableLedgerV1::from_preopened(
                owned(file.as_file()),
                object,
                [4; 32],
                LedgerKindV1::Replay,
            ),
            Err(DurableLedgerErrorV1::Corrupt("torn length"))
        ));
    }

    #[test]
    fn corrupted_record_checksum_fails_closed_after_restart() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let object = identity(file.as_file());
        provision_empty_replay_ledger_v1(&owned(file.as_file()), object, [5; 32]).unwrap();
        let ledger = DurableLedgerV1::from_preopened(
            owned(file.as_file()),
            object,
            [5; 32],
            LedgerKindV1::Replay,
        )
        .unwrap();
        assert!(
            ledger
                .append_if_absent(record(LedgerKindV1::Replay, 41))
                .unwrap()
        );
        drop(ledger);
        rustix::io::pwrite(file.as_file(), &[0xbb], (HEADER_BYTES + 64) as u64).unwrap();
        rustix::fs::fdatasync(file.as_file()).unwrap();

        assert!(matches!(
            DurableLedgerV1::from_preopened(
                owned(file.as_file()),
                object,
                [5; 32],
                LedgerKindV1::Replay,
            ),
            Err(DurableLedgerErrorV1::Corrupt("record checksum"))
        ));
    }

    #[test]
    fn duplicate_historical_reservation_coordinate_is_corruption() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let object = identity(file.as_file());
        provision_empty_reservation_ledger_v2(&owned(file.as_file()), object, [6; 32]).unwrap();
        let first = record(LedgerKindV1::Reservation, 51).encode([0; 32]);
        let first_identity = first[192..224].try_into().unwrap();
        let mut duplicate = record(LedgerKindV1::Reservation, 52);
        duplicate.primary = [51; 32];
        let duplicate = duplicate.encode(first_identity);
        rustix::io::pwrite(file.as_file(), &first, HEADER_BYTES as u64).unwrap();
        rustix::io::pwrite(
            file.as_file(),
            &duplicate,
            (HEADER_BYTES + RECORD_BYTES) as u64,
        )
        .unwrap();
        rustix::fs::fdatasync(file.as_file()).unwrap();

        assert!(matches!(
            DurableLedgerV1::from_preopened(
                owned(file.as_file()),
                object,
                [6; 32],
                LedgerKindV1::Reservation,
            ),
            Err(DurableLedgerErrorV1::Corrupt(
                "duplicate durable coordinate"
            ))
        ));
    }

    #[test]
    fn live_reservation_ledger_rejects_every_reused_coordinate() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let object = identity(file.as_file());
        provision_empty_reservation_ledger_v2(&owned(file.as_file()), object, [8; 32]).unwrap();
        let ledger = DurableLedgerV1::from_preopened(
            owned(file.as_file()),
            object,
            [8; 32],
            LedgerKindV1::Reservation,
        )
        .unwrap();
        let first = record(LedgerKindV1::Reservation, 81);
        assert!(ledger.append_if_absent(first).unwrap());

        let mut duplicate_challenge = record(LedgerKindV1::Reservation, 91);
        duplicate_challenge.primary = first.primary;
        assert!(!ledger.append_if_absent(duplicate_challenge).unwrap());
        let mut duplicate_reservation = record(LedgerKindV1::Reservation, 92);
        duplicate_reservation.secondary = first.secondary;
        assert!(!ledger.append_if_absent(duplicate_reservation).unwrap());
        let mut duplicate_request = record(LedgerKindV1::Reservation, 93);
        duplicate_request.request = first.request;
        assert!(!ledger.append_if_absent(duplicate_request).unwrap());
        assert_eq!(ledger.read_validated_records().unwrap().len(), 1);
    }

    #[test]
    fn reservation_is_burned_before_release_and_survives_restart() {
        let ledger_file = tempfile::NamedTempFile::new().unwrap();
        let ledger_object = identity(ledger_file.as_file());
        provision_empty_reservation_ledger_v2(
            &owned(ledger_file.as_file()),
            ledger_object,
            [7; 32],
        )
        .unwrap();

        let entropy = open_entropy();
        let entropy_object = entropy_identity(&entropy);
        let mut provider = DurableReservationProviderV2::from_preopened(
            owned(ledger_file.as_file()),
            ledger_object,
            [7; 32],
            entropy,
            entropy_object,
        )
        .unwrap();
        let reserved = provider
            .try_reserve_coordinates([1, 2, 3], [71; 32], [72; 32])
            .unwrap();
        assert_ne!(reserved.challenge_bytes(), &[0; 32]);
        assert_eq!(provider.ledger.read_validated_records().unwrap().len(), 1);
        drop(provider);

        let entropy = open_entropy();
        let mut restarted = DurableReservationProviderV2::from_preopened(
            owned(ledger_file.as_file()),
            ledger_object,
            [7; 32],
            entropy,
            entropy_object,
        )
        .unwrap();
        assert!(matches!(
            restarted.try_reserve_coordinates([1, 2, 3], [71; 32], [72; 32]),
            Err(DurableLedgerErrorV1::EntropyExhausted)
        ));
        assert_eq!(restarted.ledger.read_validated_records().unwrap().len(), 1);

        let entropy = open_entropy();
        let mut next_request = DurableReservationProviderV2::from_preopened(
            owned(ledger_file.as_file()),
            ledger_object,
            [7; 32],
            entropy,
            entropy_object,
        )
        .unwrap();
        let next = next_request
            .try_reserve_coordinates([1, 2, 3], [71; 32], [73; 32])
            .unwrap();
        assert_ne!(next.challenge_bytes(), reserved.challenge_bytes());
        assert_eq!(
            next_request.ledger.read_validated_records().unwrap().len(),
            2
        );
    }

    #[test]
    fn descriptor_identity_substitution_is_rejected() {
        let first = tempfile::NamedTempFile::new().unwrap();
        let second = tempfile::NamedTempFile::new().unwrap();
        let first_identity = identity(first.as_file());
        assert!(!first_identity.matches(second.as_file()).unwrap());
    }
}
