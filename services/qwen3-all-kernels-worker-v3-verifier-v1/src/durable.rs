#![allow(
    clippy::must_use_candidate,
    reason = "public values are inert identities or diagnostic accessors"
)]

use std::collections::BTreeSet;
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

const HEADER_MAGIC: [u8; 8] = *b"FRV2LED2";
const RECORD_MAGIC: [u8; 8] = *b"FRV2REC2";
const VERSION: u16 = 2;
const HEADER_BYTES: usize = 96;
const HEADER_BYTES_U64: u64 = 96;
const HEADER_BYTES_U32: u32 = 96;
const RECORD_BYTES: usize = 224;
const RECORD_BYTES_U64: u64 = 224;
const RECORD_BYTES_U32: u32 = 224;
const REPLAY_KIND: u16 = 1;
const RESERVATION_KIND: u16 = 2;
const HEADER_DOMAIN: &[u8] = b"FERRIC/M1/PROTECTED-VERIFIER/DURABLE-LEDGER-HEADER/V2\0";
const RECORD_DOMAIN: &[u8] = b"FERRIC/M1/PROTECTED-VERIFIER/DURABLE-LEDGER-RECORD/V2\0";
const MAX_ENTROPY_ATTEMPTS: usize = 16;
const REQUIRED_LEDGER_PERMISSION_BITS: u32 = 0o600;

/// Hard implementation ceiling for one durable ledger epoch.
pub const MAX_DURABLE_LEDGER_RECORDS_V1: u32 = 1_048_576;

/// Boxed failure returned by a deployment-owned protected head store.
pub type ProtectedLedgerHeadStoreFailureV1 = Box<dyn Error + Send + Sync + 'static>;

/// Exact supervisor-pinned identity and mode of one durable ledger object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LedgerObjectIdentityV1 {
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    gid: u32,
}

impl LedgerObjectIdentityV1 {
    /// Constructs an identity captured by the protected provisioning authority.
    pub const fn new(device: u64, inode: u64, mode: u32, uid: u32, gid: u32) -> Self {
        Self {
            device,
            inode,
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
            && stat.st_mode == self.mode
            && stat.st_uid == self.uid
            && stat.st_gid == self.gid
            && stat.st_nlink == 1
            && rustix::fs::FileType::from_raw_mode(stat.st_mode)
                == rustix::fs::FileType::RegularFile
            && stat.st_mode & 0o7777 == REQUIRED_LEDGER_PERMISSION_BITS
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

/// Exact supervisor-pinned identity of the preopened nonblocking CSPRNG character device.
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

/// Logical purpose of a protected ledger and its external head.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProtectedLedgerKindV1 {
    /// Caller challenge replay exclusion.
    Replay,
    /// Service challenge and reservation non-reuse.
    Reservation,
}

impl ProtectedLedgerKindV1 {
    const fn tag(self) -> u16 {
        match self {
            Self::Replay => REPLAY_KIND,
            Self::Reservation => RESERVATION_KIND,
        }
    }
}

/// Externally anchored exact state of one protected ledger epoch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtectedLedgerExternalHeadV1 {
    header_identity: [u8; 32],
    epoch: u64,
    record_count: u32,
    record_identity: [u8; 32],
}

impl ProtectedLedgerExternalHeadV1 {
    /// Reconstructs one authenticated head loaded by a protected store.
    ///
    /// The ledger capability subsequently requires exact equality with its own
    /// independently validated state; construction alone grants no authority.
    ///
    /// # Errors
    ///
    /// Returns an error for zero identities, epoch zero, or an inconsistent
    /// empty/nonempty record identity.
    pub fn new(
        header_identity: [u8; 32],
        epoch: u64,
        record_count: u32,
        record_identity: [u8; 32],
    ) -> Result<Self, DurableLedgerErrorV1> {
        if header_identity == [0; 32] {
            return Err(DurableLedgerErrorV1::ExternalHeadMismatch);
        }
        if epoch == 0 {
            return Err(DurableLedgerErrorV1::InvalidEpoch);
        }
        if (record_count == 0) != (record_identity == [0; 32]) {
            return Err(DurableLedgerErrorV1::ExternalHeadMismatch);
        }
        Ok(Self {
            header_identity,
            epoch,
            record_count,
            record_identity,
        })
    }

    /// Returns the exact header identity covering namespace, kind, epoch, and capacity.
    pub const fn header_identity(&self) -> [u8; 32] {
        self.header_identity
    }

    /// Returns the monotonically increasing supervisor epoch.
    pub const fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Returns the exact committed record count.
    pub const fn record_count(&self) -> u32 {
        self.record_count
    }

    /// Returns the final record identity, or zero for an empty epoch.
    pub const fn record_identity(&self) -> [u8; 32] {
        self.record_identity
    }
}

/// Deployment-owned durable antirollback head store.
///
/// # Safety
///
/// Implementations must authenticate one exclusively controlled protected store,
/// durably serialize every initialize/CAS across processes and restarts, prevent
/// rollback or deletion of a committed head, and return success only after the
/// new head is externally durable. Provider IPC must be bounded and cancellable;
/// this synchronous interface cannot recover a thread from a hung implementation.
/// A SHA chain in the ledger file is not enough.
pub unsafe trait ProtectedLedgerHeadStoreV1: Send {
    /// Returns the exact measured/pinned provider identity.
    fn provider_identity(&self) -> [u8; 32];

    /// Loads the durable head, returning `None` only for a never-provisioned store.
    ///
    /// # Errors
    ///
    /// Returns an authenticated store, transport, or durability failure.
    fn load_head(
        &mut self,
    ) -> Result<Option<ProtectedLedgerExternalHeadV1>, ProtectedLedgerHeadStoreFailureV1>;

    /// Durably installs the initial epoch only when no head has ever existed.
    ///
    /// # Errors
    ///
    /// Returns an authenticated store, transport, or durability failure.
    fn initialize_head(
        &mut self,
        initial: ProtectedLedgerExternalHeadV1,
    ) -> Result<bool, ProtectedLedgerHeadStoreFailureV1>;

    /// Durably replaces exactly `current` with `next`, or reports a CAS conflict.
    ///
    /// # Errors
    ///
    /// Returns an authenticated store, transport, or durability failure.
    fn compare_exchange_head(
        &mut self,
        current: ProtectedLedgerExternalHeadV1,
        next: ProtectedLedgerExternalHeadV1,
    ) -> Result<bool, ProtectedLedgerHeadStoreFailureV1>;
}

/// Move-only authority to use one externally anchored, exclusively modified ledger.
///
/// ```compile_fail
/// use ferric_qwen3_all_kernels_worker_v3_verifier_service_v1::
///     ProtectedLedgerStorageCapabilityV1;
/// fn duplicate(value: ProtectedLedgerStorageCapabilityV1) {
///     let _again = value.clone();
/// }
/// ```
pub struct ProtectedLedgerStorageCapabilityV1 {
    file: OwnedFd,
    object: LedgerObjectIdentityV1,
    namespace: [u8; 32],
    kind: ProtectedLedgerKindV1,
    epoch: u64,
    max_records: u32,
    head_store: Box<dyn ProtectedLedgerHeadStoreV1>,
    head_store_identity: [u8; 32],
    state: ValidatedLedgerStateV1,
}

impl ProtectedLedgerStorageCapabilityV1 {
    /// Provisions a never-before-used ledger and initializes its external head.
    ///
    /// # Safety
    ///
    /// The caller must be the protected supervisor. It must guarantee that `file`
    /// is new and cannot be replaced, rolled back, linked, or modified by another
    /// principal, and that the pinned owner is the protected service principal.
    /// `head_store` must satisfy its unsafe trait contract and be dedicated to this
    /// logical ledger. Namespace and epoch must never be reused.
    ///
    /// # Errors
    ///
    /// Returns a typed error if any invariant or external durable transition fails.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn provision_new_from_supervisor(
        file: OwnedFd,
        object: LedgerObjectIdentityV1,
        namespace: [u8; 32],
        kind: ProtectedLedgerKindV1,
        epoch: u64,
        max_records: u32,
        mut head_store: Box<dyn ProtectedLedgerHeadStoreV1>,
        head_store_identity: [u8; 32],
    ) -> Result<Self, DurableLedgerErrorV1> {
        validate_configuration(namespace, epoch, max_records, head_store_identity)?;
        validate_file(&file, object)?;
        validate_head_store_identity(&*head_store, head_store_identity)?;
        let provision_lock = LedgerLockV1::acquire(&file)?;
        let stat = rustix::fs::fstat(&file)
            .map_err(|source| DurableLedgerErrorV1::Inspect(source.into()))?;
        if stat.st_size != 0 {
            return Err(DurableLedgerErrorV1::AlreadyProvisioned);
        }
        let header = encode_header(kind, namespace, epoch, max_records);
        pwrite_all(&file, &header, 0)?;
        rustix::fs::fdatasync(&file)
            .map_err(|source| DurableLedgerErrorV1::Synchronize(source.into()))?;
        let initial = ProtectedLedgerExternalHeadV1 {
            header_identity: header_identity(&header),
            epoch,
            record_count: 0,
            record_identity: [0; 32],
        };
        if !head_store
            .initialize_head(initial)
            .map_err(DurableLedgerErrorV1::ExternalHead)?
        {
            return Err(DurableLedgerErrorV1::ExternalHeadConflict);
        }
        drop(provision_lock);
        Ok(Self {
            file,
            object,
            namespace,
            kind,
            epoch,
            max_records,
            head_store,
            head_store_identity,
            state: ValidatedLedgerStateV1::empty(initial),
        })
    }

    /// Opens an existing ledger only when its complete state equals the external head.
    ///
    /// # Safety
    ///
    /// The protected supervisor must uphold the same exclusive-modification and
    /// antirollback contract as [`Self::provision_new_from_supervisor`], including
    /// continuity of the exact protected head store across restarts.
    ///
    /// # Errors
    ///
    /// Returns a typed error if the object, bounded ledger, or external head differs.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn open_existing_from_supervisor(
        file: OwnedFd,
        object: LedgerObjectIdentityV1,
        namespace: [u8; 32],
        kind: ProtectedLedgerKindV1,
        epoch: u64,
        max_records: u32,
        head_store: Box<dyn ProtectedLedgerHeadStoreV1>,
        head_store_identity: [u8; 32],
    ) -> Result<Self, DurableLedgerErrorV1> {
        validate_configuration(namespace, epoch, max_records, head_store_identity)?;
        validate_file(&file, object)?;
        validate_head_store_identity(&*head_store, head_store_identity)?;
        let state = {
            let _lock = LedgerLockV1::acquire(&file)?;
            read_validated_state(&file, kind, namespace, epoch, max_records)?
        };
        let mut capability = Self {
            file,
            object,
            namespace,
            kind,
            epoch,
            max_records,
            head_store,
            head_store_identity,
            state,
        };
        capability.validate_current_head()?;
        Ok(capability)
    }

    /// Rotates a full epoch to a new empty object without weakening antirollback.
    ///
    /// No automatic compaction exists. Capacity exhaustion remains terminal until
    /// the protected supervisor quiesces admission and performs this transition.
    ///
    /// # Safety
    ///
    /// The supervisor must quiesce every old user, protect the new file under the
    /// initial-provisioning contract, and never reuse `next_epoch`. The protected
    /// head-store CAS is the commit point: before it the old epoch is authoritative;
    /// after it the already-synchronized new epoch is authoritative.
    ///
    /// # Errors
    ///
    /// Returns an error unless the old epoch is full/current and the next epoch,
    /// new object, and external CAS all satisfy the protected contract.
    pub unsafe fn rotate_full_epoch_from_supervisor(
        mut self,
        new_file: OwnedFd,
        new_object: LedgerObjectIdentityV1,
        next_epoch: u64,
        next_max_records: u32,
    ) -> Result<Self, DurableLedgerErrorV1> {
        if next_epoch <= self.epoch {
            return Err(DurableLedgerErrorV1::InvalidEpoch);
        }
        validate_configuration(
            self.namespace,
            next_epoch,
            next_max_records,
            self.head_store_identity,
        )?;
        let old = self.validate_current_head()?;
        if old.record_count != self.max_records {
            return Err(DurableLedgerErrorV1::RotationBeforeCapacity);
        }
        validate_file(&new_file, new_object)?;
        let lock = LedgerLockV1::acquire(&new_file)?;
        let stat = rustix::fs::fstat(&new_file)
            .map_err(|source| DurableLedgerErrorV1::Inspect(source.into()))?;
        if stat.st_size != 0 {
            return Err(DurableLedgerErrorV1::AlreadyProvisioned);
        }
        let header = encode_header(self.kind, self.namespace, next_epoch, next_max_records);
        pwrite_all(&new_file, &header, 0)?;
        rustix::fs::fdatasync(&new_file)
            .map_err(|source| DurableLedgerErrorV1::Synchronize(source.into()))?;
        let next = ProtectedLedgerExternalHeadV1 {
            header_identity: header_identity(&header),
            epoch: next_epoch,
            record_count: 0,
            record_identity: [0; 32],
        };
        if !self
            .head_store
            .compare_exchange_head(old, next)
            .map_err(DurableLedgerErrorV1::ExternalHead)?
        {
            return Err(DurableLedgerErrorV1::ExternalHeadConflict);
        }
        drop(lock);
        self.file = new_file;
        self.object = new_object;
        self.epoch = next_epoch;
        self.max_records = next_max_records;
        self.state = ValidatedLedgerStateV1::empty(next);
        Ok(self)
    }

    fn validate_current_head(
        &mut self,
    ) -> Result<ProtectedLedgerExternalHeadV1, DurableLedgerErrorV1> {
        validate_file(&self.file, self.object)?;
        validate_head_store_identity(&*self.head_store, self.head_store_identity)?;
        let _lock = LedgerLockV1::acquire(&self.file)?;
        validate_cached_file_state(
            &self.file,
            self.kind,
            self.namespace,
            self.epoch,
            self.max_records,
            &self.state,
        )?;
        let anchored = self
            .head_store
            .load_head()
            .map_err(DurableLedgerErrorV1::ExternalHead)?
            .ok_or(DurableLedgerErrorV1::ExternalHeadMissing)?;
        if anchored != self.state.external_head {
            return Err(DurableLedgerErrorV1::ExternalHeadMismatch);
        }
        Ok(self.state.external_head)
    }
}

/// Durable ledger validation or update failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum DurableLedgerErrorV1 {
    /// The pinned descriptor identity/mode/owner/link/type did not match.
    ObjectIdentityMismatch,
    /// The namespace used the forbidden zero sentinel.
    ZeroNamespace,
    /// The epoch was zero or did not increase during rotation.
    InvalidEpoch,
    /// Capacity was zero or exceeded the hard implementation ceiling.
    InvalidCapacity,
    /// No more records may be appended in this epoch.
    CapacityExhausted,
    /// Rotation was attempted before exact capacity.
    RotationBeforeCapacity,
    /// The external head-store identity was zero or substituted.
    ExternalHeadIdentityMismatch,
    /// The external head was absent for an existing ledger.
    ExternalHeadMissing,
    /// The external head differed from the exact bounded file state.
    ExternalHeadMismatch,
    /// External initialization/CAS reported a conflict.
    ExternalHeadConflict,
    /// The protected head store failed.
    ExternalHead(ProtectedLedgerHeadStoreFailureV1),
    /// Descriptor inspection failed.
    Inspect(io::Error),
    /// The descriptor could not be exclusively locked.
    Lock(io::Error),
    /// The ledger could not be read completely.
    Read(io::Error),
    /// A durable write failed.
    Write(io::Error),
    /// A durable update could not be synchronized.
    Synchronize(io::Error),
    /// The descriptor did not contain the exact schema.
    Corrupt(&'static str),
    /// Provisioning or rotation received a nonempty file.
    AlreadyProvisioned,
    /// The entropy descriptor failed before a complete candidate was read.
    Entropy(io::Error),
    /// No unique nonzero challenge/reservation pair was obtained within the bound.
    EntropyExhausted,
}

impl fmt::Display for DurableLedgerErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObjectIdentityMismatch => formatter.write_str("protected ledger object mismatch"),
            Self::ZeroNamespace => formatter.write_str("protected ledger namespace is zero"),
            Self::InvalidEpoch => formatter.write_str("protected ledger epoch is invalid"),
            Self::InvalidCapacity => formatter.write_str("protected ledger capacity is invalid"),
            Self::CapacityExhausted => formatter.write_str("protected ledger epoch is full"),
            Self::RotationBeforeCapacity => {
                formatter.write_str("rotation requested before capacity")
            }
            Self::ExternalHeadIdentityMismatch => {
                formatter.write_str("head-store identity mismatch")
            }
            Self::ExternalHeadMissing => formatter.write_str("protected ledger head is missing"),
            Self::ExternalHeadMismatch => formatter.write_str("protected ledger head mismatch"),
            Self::ExternalHeadConflict => formatter.write_str("protected ledger head CAS conflict"),
            Self::ExternalHead(source) => write!(formatter, "protected head store: {source}"),
            Self::Inspect(source) => write!(formatter, "inspect protected ledger: {source}"),
            Self::Lock(source) => write!(formatter, "lock protected ledger: {source}"),
            Self::Read(source) => write!(formatter, "read protected ledger: {source}"),
            Self::Write(source) => write!(formatter, "write protected ledger: {source}"),
            Self::Synchronize(source) => {
                write!(formatter, "synchronize protected ledger: {source}")
            }
            Self::Corrupt(field) => write!(formatter, "protected ledger is corrupt: {field}"),
            Self::AlreadyProvisioned => {
                formatter.write_str("protected ledger is already provisioned")
            }
            Self::Entropy(source) => write!(formatter, "read preopened entropy source: {source}"),
            Self::EntropyExhausted => {
                formatter.write_str("entropy did not yield a unique reservation")
            }
        }
    }
}

impl Error for DurableLedgerErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ExternalHead(source) => Some(&**source),
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

struct DurableLedgerV1 {
    storage: ProtectedLedgerStorageCapabilityV1,
}

impl DurableLedgerV1 {
    const fn from_capability(storage: ProtectedLedgerStorageCapabilityV1) -> Self {
        Self { storage }
    }

    fn append_if_absent(
        &mut self,
        candidate: LedgerRecordV1,
    ) -> Result<bool, DurableLedgerErrorV1> {
        validate_file(&self.storage.file, self.storage.object)?;
        validate_head_store_identity(&*self.storage.head_store, self.storage.head_store_identity)?;
        let _lock = LedgerLockV1::acquire(&self.storage.file)?;
        validate_cached_file_state(
            &self.storage.file,
            self.storage.kind,
            self.storage.namespace,
            self.storage.epoch,
            self.storage.max_records,
            &self.storage.state,
        )?;
        let anchored = self
            .storage
            .head_store
            .load_head()
            .map_err(DurableLedgerErrorV1::ExternalHead)?
            .ok_or(DurableLedgerErrorV1::ExternalHeadMissing)?;
        if anchored != self.storage.state.external_head {
            return Err(DurableLedgerErrorV1::ExternalHeadMismatch);
        }
        if self.storage.state.conflicts(candidate) {
            return Ok(false);
        }
        if self.storage.state.external_head.record_count >= self.storage.max_records {
            return Err(DurableLedgerErrorV1::CapacityExhausted);
        }
        let bytes = candidate.encode(self.storage.state.external_head.record_identity);
        pwrite_all(
            &self.storage.file,
            &bytes,
            record_offset(self.storage.state.external_head.record_count)?,
        )?;
        rustix::fs::fdatasync(&self.storage.file)
            .map_err(|source| DurableLedgerErrorV1::Synchronize(source.into()))?;
        let next = ProtectedLedgerExternalHeadV1 {
            header_identity: self.storage.state.external_head.header_identity,
            epoch: self.storage.state.external_head.epoch,
            record_count: self
                .storage
                .state
                .external_head
                .record_count
                .checked_add(1)
                .ok_or(DurableLedgerErrorV1::CapacityExhausted)?,
            record_identity: bytes[192..224].try_into().expect("fixed record identity"),
        };
        if !self
            .storage
            .head_store
            .compare_exchange_head(self.storage.state.external_head, next)
            .map_err(DurableLedgerErrorV1::ExternalHead)?
        {
            return Err(DurableLedgerErrorV1::ExternalHeadConflict);
        }
        self.storage.state.insert(candidate)?;
        self.storage.state.external_head = next;
        Ok(true)
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
    kind: ProtectedLedgerKindV1,
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
}

type ReplayCoordinateV1 = ([u32; 3], [u8; 32], [u8; 32]);

struct ValidatedLedgerStateV1 {
    external_head: ProtectedLedgerExternalHeadV1,
    replay: BTreeSet<ReplayCoordinateV1>,
    challenges: BTreeSet<[u8; 32]>,
    reservations: BTreeSet<[u8; 32]>,
    requests: BTreeSet<[u8; 32]>,
}

impl ValidatedLedgerStateV1 {
    fn empty(external_head: ProtectedLedgerExternalHeadV1) -> Self {
        Self {
            external_head,
            replay: BTreeSet::new(),
            challenges: BTreeSet::new(),
            reservations: BTreeSet::new(),
            requests: BTreeSet::new(),
        }
    }

    fn conflicts(&self, candidate: LedgerRecordV1) -> bool {
        match candidate.kind {
            ProtectedLedgerKindV1::Replay => {
                self.replay
                    .contains(&(candidate.caller, candidate.policy, candidate.primary))
            }
            ProtectedLedgerKindV1::Reservation => {
                self.challenges.contains(&candidate.primary)
                    || self.reservations.contains(&candidate.secondary)
                    || self.requests.contains(&candidate.request)
            }
        }
    }

    fn insert(&mut self, candidate: LedgerRecordV1) -> Result<(), DurableLedgerErrorV1> {
        let inserted = match candidate.kind {
            ProtectedLedgerKindV1::Replay => {
                self.replay
                    .insert((candidate.caller, candidate.policy, candidate.primary))
            }
            ProtectedLedgerKindV1::Reservation => {
                let challenge = self.challenges.insert(candidate.primary);
                let reservation = self.reservations.insert(candidate.secondary);
                let request = self.requests.insert(candidate.request);
                challenge && reservation && request
            }
        };
        if !inserted {
            return Err(DurableLedgerErrorV1::Corrupt(
                "duplicate durable coordinate",
            ));
        }
        Ok(())
    }
}

/// Durable caller-challenge replay guard backed by protected storage capability.
pub struct DurableReplayGuardV1 {
    ledger: DurableLedgerV1,
    last_error: Option<DurableLedgerErrorV1>,
}

impl DurableReplayGuardV1 {
    /// Consumes a move-only protected replay-ledger capability.
    ///
    /// # Errors
    ///
    /// Returns an error if the capability names a reservation ledger.
    pub fn from_protected_storage(
        storage: ProtectedLedgerStorageCapabilityV1,
    ) -> Result<Self, DurableLedgerErrorV1> {
        if storage.kind != ProtectedLedgerKindV1::Replay {
            return Err(DurableLedgerErrorV1::Corrupt("replay ledger kind"));
        }
        Ok(Self {
            ledger: DurableLedgerV1::from_capability(storage),
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
            kind: ProtectedLedgerKindV1::Replay,
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

/// Durable service-challenge provider using protected storage and a pinned CSPRNG.
pub struct DurableReservationProviderV2 {
    ledger: DurableLedgerV1,
    entropy: OwnedFd,
    entropy_identity: EntropyObjectIdentityV1,
    last_error: Option<DurableLedgerErrorV1>,
}

impl DurableReservationProviderV2 {
    /// Consumes protected reservation storage and a preopened nonblocking CSPRNG.
    ///
    /// # Errors
    ///
    /// Returns an error for a substituted descriptor or wrong ledger kind.
    pub fn from_protected_storage(
        storage: ProtectedLedgerStorageCapabilityV1,
        entropy: OwnedFd,
        entropy_identity: EntropyObjectIdentityV1,
    ) -> Result<Self, DurableLedgerErrorV1> {
        if storage.kind != ProtectedLedgerKindV1::Reservation {
            return Err(DurableLedgerErrorV1::Corrupt("reservation ledger kind"));
        }
        if !entropy_identity.matches(&entropy)? {
            return Err(DurableLedgerErrorV1::ObjectIdentityMismatch);
        }
        Ok(Self {
            ledger: DurableLedgerV1::from_capability(storage),
            entropy,
            entropy_identity,
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
        if !self.entropy_identity.matches(&self.entropy)? {
            return Err(DurableLedgerErrorV1::ObjectIdentityMismatch);
        }
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
                kind: ProtectedLedgerKindV1::Reservation,
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

fn validate_configuration(
    namespace: [u8; 32],
    epoch: u64,
    max_records: u32,
    head_store_identity: [u8; 32],
) -> Result<(), DurableLedgerErrorV1> {
    if namespace == [0; 32] {
        return Err(DurableLedgerErrorV1::ZeroNamespace);
    }
    if epoch == 0 {
        return Err(DurableLedgerErrorV1::InvalidEpoch);
    }
    if max_records == 0 || max_records > MAX_DURABLE_LEDGER_RECORDS_V1 {
        return Err(DurableLedgerErrorV1::InvalidCapacity);
    }
    if head_store_identity == [0; 32] {
        return Err(DurableLedgerErrorV1::ExternalHeadIdentityMismatch);
    }
    Ok(())
}

fn validate_file(
    file: &OwnedFd,
    object: LedgerObjectIdentityV1,
) -> Result<(), DurableLedgerErrorV1> {
    if !object.matches(file)? {
        return Err(DurableLedgerErrorV1::ObjectIdentityMismatch);
    }
    Ok(())
}

fn validate_head_store_identity(
    head_store: &dyn ProtectedLedgerHeadStoreV1,
    expected: [u8; 32],
) -> Result<(), DurableLedgerErrorV1> {
    if head_store.provider_identity() != expected {
        return Err(DurableLedgerErrorV1::ExternalHeadIdentityMismatch);
    }
    Ok(())
}

fn read_validated_state(
    file: &OwnedFd,
    kind: ProtectedLedgerKindV1,
    namespace: [u8; 32],
    epoch: u64,
    max_records: u32,
) -> Result<ValidatedLedgerStateV1, DurableLedgerErrorV1> {
    let stat =
        rustix::fs::fstat(file).map_err(|source| DurableLedgerErrorV1::Inspect(source.into()))?;
    let length = u64::try_from(stat.st_size)
        .map_err(|_| DurableLedgerErrorV1::Corrupt("negative length"))?;
    let maximum_length = u64::from(max_records)
        .checked_mul(RECORD_BYTES_U64)
        .and_then(|records| records.checked_add(HEADER_BYTES_U64))
        .ok_or(DurableLedgerErrorV1::InvalidCapacity)?;
    if length < HEADER_BYTES_U64 || length > maximum_length {
        return Err(DurableLedgerErrorV1::Corrupt(
            "length outside configured capacity",
        ));
    }
    let body_length = length - HEADER_BYTES_U64;
    if !body_length.is_multiple_of(RECORD_BYTES_U64) {
        return Err(DurableLedgerErrorV1::Corrupt("torn length"));
    }
    let record_count = u32::try_from(body_length / RECORD_BYTES_U64)
        .map_err(|_| DurableLedgerErrorV1::Corrupt("record count overflow"))?;
    if record_count > max_records {
        return Err(DurableLedgerErrorV1::Corrupt("record capacity exceeded"));
    }
    let mut header = [0_u8; HEADER_BYTES];
    pread_all(file, &mut header, 0)?;
    validate_header(&header, kind, namespace, epoch, max_records)?;
    let mut state = ValidatedLedgerStateV1 {
        external_head: ProtectedLedgerExternalHeadV1 {
            header_identity: header_identity(&header),
            epoch,
            record_count,
            record_identity: [0; 32],
        },
        replay: BTreeSet::new(),
        challenges: BTreeSet::new(),
        reservations: BTreeSet::new(),
        requests: BTreeSet::new(),
    };
    let mut previous = [0; 32];
    for index in 0..record_count {
        let mut bytes = [0_u8; RECORD_BYTES];
        pread_all(file, &mut bytes, record_offset(index)?)?;
        let record = decode_record(&bytes, kind, previous)?;
        let inserted = match kind {
            ProtectedLedgerKindV1::Replay => {
                state
                    .replay
                    .insert((record.caller, record.policy, record.primary))
            }
            ProtectedLedgerKindV1::Reservation => {
                let challenge = state.challenges.insert(record.primary);
                let reservation = state.reservations.insert(record.secondary);
                let request = state.requests.insert(record.request);
                challenge && reservation && request
            }
        };
        if !inserted {
            return Err(DurableLedgerErrorV1::Corrupt(
                "duplicate durable coordinate",
            ));
        }
        previous = record.identity;
    }
    state.external_head.record_identity = previous;
    Ok(state)
}

fn validate_cached_file_state(
    file: &OwnedFd,
    kind: ProtectedLedgerKindV1,
    namespace: [u8; 32],
    epoch: u64,
    max_records: u32,
    state: &ValidatedLedgerStateV1,
) -> Result<(), DurableLedgerErrorV1> {
    let stat =
        rustix::fs::fstat(file).map_err(|source| DurableLedgerErrorV1::Inspect(source.into()))?;
    let length = u64::try_from(stat.st_size)
        .map_err(|_| DurableLedgerErrorV1::Corrupt("negative length"))?;
    if length != record_offset(state.external_head.record_count)? {
        return Err(DurableLedgerErrorV1::ExternalHeadMismatch);
    }
    let mut header = [0_u8; HEADER_BYTES];
    pread_all(file, &mut header, 0)?;
    validate_header(&header, kind, namespace, epoch, max_records)?;
    if header_identity(&header) != state.external_head.header_identity {
        return Err(DurableLedgerErrorV1::ExternalHeadMismatch);
    }
    if state.external_head.record_count > 0 {
        let mut tail = [0_u8; RECORD_BYTES];
        pread_all(
            file,
            &mut tail,
            record_offset(state.external_head.record_count - 1)?,
        )?;
        let tail_identity: [u8; 32] = tail[192..224].try_into().expect("tail identity");
        if tail_identity != state.external_head.record_identity
            || tail_identity != hash_parts(&[RECORD_DOMAIN, &tail[..192]])
        {
            return Err(DurableLedgerErrorV1::ExternalHeadMismatch);
        }
    } else if state.external_head.record_identity != [0; 32] {
        return Err(DurableLedgerErrorV1::ExternalHeadMismatch);
    }
    Ok(())
}

fn record_offset(record_count: u32) -> Result<u64, DurableLedgerErrorV1> {
    u64::from(record_count)
        .checked_mul(RECORD_BYTES_U64)
        .and_then(|offset| offset.checked_add(HEADER_BYTES_U64))
        .ok_or(DurableLedgerErrorV1::Corrupt("record offset overflow"))
}

fn encode_header(
    kind: ProtectedLedgerKindV1,
    namespace: [u8; 32],
    epoch: u64,
    max_records: u32,
) -> [u8; HEADER_BYTES] {
    let mut bytes = [0_u8; HEADER_BYTES];
    let mut offset = 0;
    put(&mut bytes, &mut offset, &HEADER_MAGIC);
    put(&mut bytes, &mut offset, &VERSION.to_le_bytes());
    put(&mut bytes, &mut offset, &kind.tag().to_le_bytes());
    put(&mut bytes, &mut offset, &HEADER_BYTES_U32.to_le_bytes());
    put(&mut bytes, &mut offset, &RECORD_BYTES_U32.to_le_bytes());
    put(&mut bytes, &mut offset, &max_records.to_le_bytes());
    put(&mut bytes, &mut offset, &epoch.to_le_bytes());
    put(&mut bytes, &mut offset, &namespace);
    let digest = hash_parts(&[HEADER_DOMAIN, &bytes[..offset]]);
    put(&mut bytes, &mut offset, &digest);
    debug_assert_eq!(offset, HEADER_BYTES);
    bytes
}

fn header_identity(bytes: &[u8; HEADER_BYTES]) -> [u8; 32] {
    bytes[64..96].try_into().expect("fixed header identity")
}

fn validate_header(
    bytes: &[u8; HEADER_BYTES],
    kind: ProtectedLedgerKindV1,
    namespace: [u8; 32],
    epoch: u64,
    max_records: u32,
) -> Result<(), DurableLedgerErrorV1> {
    if bytes[..8] != HEADER_MAGIC
        || u16::from_le_bytes(bytes[8..10].try_into().expect("header version")) != VERSION
        || u16::from_le_bytes(bytes[10..12].try_into().expect("header kind")) != kind.tag()
        || u32::from_le_bytes(bytes[12..16].try_into().expect("header length")) != HEADER_BYTES_U32
        || u32::from_le_bytes(bytes[16..20].try_into().expect("record length")) != RECORD_BYTES_U32
        || u32::from_le_bytes(bytes[20..24].try_into().expect("record capacity")) != max_records
        || u64::from_le_bytes(bytes[24..32].try_into().expect("epoch")) != epoch
        || bytes[32..64] != namespace
        || bytes[64..96] != hash_parts(&[HEADER_DOMAIN, &bytes[..64]])
    {
        return Err(DurableLedgerErrorV1::Corrupt("header binding"));
    }
    Ok(())
}

fn decode_record(
    bytes: &[u8; RECORD_BYTES],
    kind: ProtectedLedgerKindV1,
    previous: [u8; 32],
) -> Result<DecodedRecordV1, DurableLedgerErrorV1> {
    if bytes[..8] != RECORD_MAGIC {
        return Err(DurableLedgerErrorV1::Corrupt("record magic"));
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
        || (kind == ProtectedLedgerKindV1::Replay && (secondary != [0; 32] || request != [0; 32]))
        || (kind == ProtectedLedgerKindV1::Reservation
            && (secondary == [0; 32] || request == [0; 32]))
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
    })
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
                "protected ledger write returned zero",
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
    use std::os::unix::fs::PermissionsExt;
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Clone)]
    struct TestHeadStore {
        identity: [u8; 32],
        head: Arc<Mutex<Option<ProtectedLedgerExternalHeadV1>>>,
    }

    // Test-only memory serialization models the contract without claiming deployment closure.
    unsafe impl ProtectedLedgerHeadStoreV1 for TestHeadStore {
        fn provider_identity(&self) -> [u8; 32] {
            self.identity
        }

        fn load_head(
            &mut self,
        ) -> Result<Option<ProtectedLedgerExternalHeadV1>, ProtectedLedgerHeadStoreFailureV1>
        {
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

    fn owned(file: &File) -> OwnedFd {
        rustix::io::fcntl_dupfd_cloexec(file, 0).unwrap()
    }

    fn object(file: &impl AsFd) -> LedgerObjectIdentityV1 {
        let stat = rustix::fs::fstat(file).unwrap();
        LedgerObjectIdentityV1::new(
            stat.st_dev,
            stat.st_ino,
            stat.st_mode,
            stat.st_uid,
            stat.st_gid,
        )
    }

    fn protected_file() -> tempfile::NamedTempFile {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
        file
    }

    fn test_store() -> (
        TestHeadStore,
        Arc<Mutex<Option<ProtectedLedgerExternalHeadV1>>>,
    ) {
        let head = Arc::new(Mutex::new(None));
        (
            TestHeadStore {
                identity: [0xa0; 32],
                head: Arc::clone(&head),
            },
            head,
        )
    }

    fn provision(
        file: &File,
        kind: ProtectedLedgerKindV1,
        capacity: u32,
        store: TestHeadStore,
    ) -> ProtectedLedgerStorageCapabilityV1 {
        // SAFETY: test owns the linked 0600 file and serialized mock head exclusively.
        unsafe {
            ProtectedLedgerStorageCapabilityV1::provision_new_from_supervisor(
                owned(file),
                object(file),
                [1; 32],
                kind,
                1,
                capacity,
                Box::new(store),
                [0xa0; 32],
            )
        }
        .unwrap()
    }

    fn reopen(
        file: &File,
        kind: ProtectedLedgerKindV1,
        capacity: u32,
        store: TestHeadStore,
    ) -> Result<ProtectedLedgerStorageCapabilityV1, DurableLedgerErrorV1> {
        // SAFETY: test preserves the exact protected file and mock head across restart.
        unsafe {
            ProtectedLedgerStorageCapabilityV1::open_existing_from_supervisor(
                owned(file),
                object(file),
                [1; 32],
                kind,
                1,
                capacity,
                Box::new(store),
                [0xa0; 32],
            )
        }
    }

    fn record(kind: ProtectedLedgerKindV1, seed: u8) -> LedgerRecordV1 {
        LedgerRecordV1 {
            caller: [11, 12, 13],
            policy: [21; 32],
            primary: [seed; 32],
            secondary: if kind == ProtectedLedgerKindV1::Reservation {
                [seed.wrapping_add(1); 32]
            } else {
                [0; 32]
            },
            request: if kind == ProtectedLedgerKindV1::Reservation {
                [seed.wrapping_add(2); 32]
            } else {
                [0; 32]
            },
            kind,
        }
    }

    #[test]
    fn replay_state_survives_restart_and_rejects_reuse() {
        let file = protected_file();
        let (store, _) = test_store();
        let mut ledger = DurableLedgerV1::from_capability(provision(
            file.as_file(),
            ProtectedLedgerKindV1::Replay,
            4,
            store.clone(),
        ));
        let first = record(ProtectedLedgerKindV1::Replay, 31);
        assert!(ledger.append_if_absent(first).unwrap());
        drop(ledger);
        let mut restarted = DurableLedgerV1::from_capability(
            reopen(file.as_file(), ProtectedLedgerKindV1::Replay, 4, store).unwrap(),
        );
        assert!(!restarted.append_if_absent(first).unwrap());
    }

    #[test]
    fn capacity_is_hard_and_distinct() {
        let file = protected_file();
        let (store, _) = test_store();
        let mut ledger = DurableLedgerV1::from_capability(provision(
            file.as_file(),
            ProtectedLedgerKindV1::Replay,
            1,
            store,
        ));
        assert!(
            ledger
                .append_if_absent(record(ProtectedLedgerKindV1::Replay, 41))
                .unwrap()
        );
        assert!(matches!(
            ledger.append_if_absent(record(ProtectedLedgerKindV1::Replay, 42)),
            Err(DurableLedgerErrorV1::CapacityExhausted)
        ));
    }

    #[test]
    fn rollback_and_head_substitution_fail_closed() {
        let file = protected_file();
        let (store, head) = test_store();
        let mut ledger = DurableLedgerV1::from_capability(provision(
            file.as_file(),
            ProtectedLedgerKindV1::Replay,
            2,
            store.clone(),
        ));
        let initial = head.lock().unwrap().unwrap();
        assert!(
            ledger
                .append_if_absent(record(ProtectedLedgerKindV1::Replay, 51))
                .unwrap()
        );
        let committed = head.lock().unwrap().unwrap();
        *head.lock().unwrap() = Some(initial);
        assert!(matches!(
            ledger.append_if_absent(record(ProtectedLedgerKindV1::Replay, 52)),
            Err(DurableLedgerErrorV1::ExternalHeadMismatch)
        ));
        *head.lock().unwrap() = Some(committed);
        drop(ledger);
        rustix::fs::ftruncate(file.as_file(), HEADER_BYTES as u64).unwrap();
        rustix::fs::fdatasync(file.as_file()).unwrap();
        assert!(matches!(
            reopen(
                file.as_file(),
                ProtectedLedgerKindV1::Replay,
                2,
                store.clone()
            ),
            Err(DurableLedgerErrorV1::ExternalHeadMismatch)
        ));
        let mut substituted = store;
        substituted.identity = [0xb0; 32];
        assert!(matches!(
            reopen(
                file.as_file(),
                ProtectedLedgerKindV1::Replay,
                2,
                substituted
            ),
            Err(DurableLedgerErrorV1::ExternalHeadIdentityMismatch)
        ));
    }

    #[test]
    fn crash_torn_tail_is_never_recovered_or_truncated() {
        let file = protected_file();
        let (store, _) = test_store();
        drop(provision(
            file.as_file(),
            ProtectedLedgerKindV1::Replay,
            2,
            store.clone(),
        ));
        rustix::io::pwrite(file.as_file(), &[0xaa], HEADER_BYTES as u64).unwrap();
        rustix::fs::fdatasync(file.as_file()).unwrap();
        assert!(matches!(
            reopen(file.as_file(), ProtectedLedgerKindV1::Replay, 2, store),
            Err(DurableLedgerErrorV1::Corrupt("torn length"))
        ));
    }

    #[test]
    fn corrupted_reservation_record_fails_restart_validation() {
        let file = protected_file();
        let (store, _) = test_store();
        let mut ledger = DurableLedgerV1::from_capability(provision(
            file.as_file(),
            ProtectedLedgerKindV1::Reservation,
            2,
            store.clone(),
        ));
        assert!(
            ledger
                .append_if_absent(record(ProtectedLedgerKindV1::Reservation, 71))
                .unwrap()
        );
        drop(ledger);
        rustix::io::pwrite(file.as_file(), &[0xbb], HEADER_BYTES_U64 + 40).unwrap();
        rustix::fs::fdatasync(file.as_file()).unwrap();
        assert!(matches!(
            reopen(file.as_file(), ProtectedLedgerKindV1::Reservation, 2, store,),
            Err(DurableLedgerErrorV1::Corrupt("record checksum"))
        ));
    }

    #[test]
    fn stale_concurrent_capability_cannot_append_after_external_commit() {
        let file = protected_file();
        let (store, _) = test_store();
        let first = provision(
            file.as_file(),
            ProtectedLedgerKindV1::Replay,
            2,
            store.clone(),
        );
        let stale = reopen(file.as_file(), ProtectedLedgerKindV1::Replay, 2, store).unwrap();
        let mut first = DurableLedgerV1::from_capability(first);
        let mut stale = DurableLedgerV1::from_capability(stale);
        assert!(
            first
                .append_if_absent(record(ProtectedLedgerKindV1::Replay, 81))
                .unwrap()
        );
        assert!(matches!(
            stale.append_if_absent(record(ProtectedLedgerKindV1::Replay, 82)),
            Err(DurableLedgerErrorV1::ExternalHeadMismatch)
        ));
    }

    #[test]
    fn concurrent_writer_lock_is_fail_closed() {
        let file = protected_file();
        let (store, _) = test_store();
        let mut ledger = DurableLedgerV1::from_capability(provision(
            file.as_file(),
            ProtectedLedgerKindV1::Replay,
            2,
            store,
        ));
        let independent = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(file.path())
            .unwrap();
        flock(&independent, FlockOperation::NonBlockingLockExclusive).unwrap();
        assert!(matches!(
            ledger.append_if_absent(record(ProtectedLedgerKindV1::Replay, 61)),
            Err(DurableLedgerErrorV1::Lock(_))
        ));
        flock(&independent, FlockOperation::Unlock).unwrap();
    }

    #[test]
    fn insecure_mode_and_extra_link_are_rejected() {
        let file = protected_file();
        let identity = object(file.as_file());
        std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o640)).unwrap();
        assert!(!identity.matches(file.as_file()).unwrap());
        std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(!object(file.as_file()).matches(file.as_file()).unwrap());
        std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
        let link = file.path().with_extension("link");
        std::fs::hard_link(file.path(), &link).unwrap();
        assert!(!object(file.as_file()).matches(file.as_file()).unwrap());
        std::fs::remove_file(link).unwrap();
    }
}
