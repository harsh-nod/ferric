//! One-shot pathname listener for the aggregate protected verifier.

use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use fe2o3_worker_v3_verification_service::{
    WorkerV3VerificationAcceptedServiceAdmissionFailureV2,
    WorkerV3VerificationAcceptedServiceEndpointV2, WorkerV3VerificationServiceErrorV1,
    prepare_worker_v3_verification_receiver_v1,
};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::fs::{AtFlags, FileType, Mode, OFlags, ResolveFlags};
use rustix::net::{AddressFamily, SocketAddrUnix, SocketFlags, SocketType};

use crate::service::{
    AbsoluteSessionDeadlineV1, FerricProtectedVerifierServiceConfigV1,
    FerricProtectedVerifierServiceFailureV1, FerricProtectedVerifierServiceOutcomeV1,
    IndependentCheckerProviderV1, ProtectedCompilerCurrentRecordProviderV1,
    ProtectedReceiptSignerProviderV1, ServiceCallerPolicyV1,
    run_ferric_protected_verifier_accepted_session_until_v2,
};

const SOCKET_MODE_V2: Mode = Mode::RUSR.union(Mode::WUSR);

/// Kernel-reported identity of one connecting protected-verifier client.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FerricProtectedVerifierPeerCredentialsV2 {
    pid: u32,
    uid: u32,
    gid: u32,
}

impl FerricProtectedVerifierPeerCredentialsV2 {
    /// Returns the positive connection-time process identifier.
    #[must_use]
    pub const fn pid(self) -> u32 {
        self.pid
    }

    /// Returns the connection-time effective user identifier.
    #[must_use]
    pub const fn uid(self) -> u32 {
        self.uid
    }

    /// Returns the connection-time effective group identifier.
    #[must_use]
    pub const fn gid(self) -> u32 {
        self.gid
    }
}

/// Stable stage for a one-shot listener failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FerricProtectedVerifierListenerFailureReasonV2 {
    /// The configured timeout could not produce an absolute deadline.
    DeadlineOverflow,
    /// The pathname was not an exact canonical absolute filesystem socket path.
    InvalidServicePath,
    /// The canonical parent could not be retained without traversing a symbolic link.
    ParentAdmission,
    /// A pathname already existed before bind.
    PathAlreadyExists,
    /// Socket creation, binding, permission setting, listening, polling, or acceptance failed.
    ListenerIo,
    /// The bound path no longer named the exact captured socket inode in the captured parent.
    PathIdentityChanged,
    /// The socket path did not retain exact owner-read/write-only mode.
    SocketModeMismatch,
    /// fe2o3 receiver preparation failed before the listener became reachable.
    ReceiverPreparation,
    /// The one absolute deadline expired before an endpoint was accepted.
    DeadlineExpired,
    /// The accepted endpoint's `SO_PEERCRED` could not be admitted.
    PeerCredentials,
    /// The accepted endpoint did not have the exact configured PID, UID, and GID.
    PeerIdentityMismatch,
    /// fe2o3 rejected the accepted endpoint shape or exact local pathname.
    EndpointAdmission,
    /// The endpoint was admitted and the existing Ferric session then failed.
    Session,
}

enum ListenerFailureSourceV2 {
    None,
    Io(io::Error),
    Receiver(WorkerV3VerificationServiceErrorV1),
    AcceptedIo {
        source: io::Error,
        accepted: OwnedFd,
    },
    Accepted {
        accepted: OwnedFd,
        observed: Option<FerricProtectedVerifierPeerCredentialsV2>,
    },
    Endpoint(WorkerV3VerificationAcceptedServiceAdmissionFailureV2),
    Session(FerricProtectedVerifierServiceFailureV1),
}

/// Typed failure from the one-shot protected-verifier listener.
///
/// Failures before fe2o3 endpoint admission retain the accepted descriptor when one exists.
/// Endpoint-admission rejection exposes fe2o3's exact retained descriptor. Once admission
/// succeeds, session failure has exactly the existing Ferric/fe2o3 custody semantics; this type
/// does not claim stronger recovery after Begin consumes the endpoint.
pub struct FerricProtectedVerifierListenerFailureV2 {
    reason: FerricProtectedVerifierListenerFailureReasonV2,
    source: ListenerFailureSourceV2,
}

impl FerricProtectedVerifierListenerFailureV2 {
    fn plain(reason: FerricProtectedVerifierListenerFailureReasonV2) -> Self {
        Self {
            reason,
            source: ListenerFailureSourceV2::None,
        }
    }

    fn io(source: impl Into<io::Error>) -> Self {
        Self {
            reason: FerricProtectedVerifierListenerFailureReasonV2::ListenerIo,
            source: ListenerFailureSourceV2::Io(source.into()),
        }
    }

    fn accepted_io(source: impl Into<io::Error>, accepted: OwnedFd) -> Self {
        Self {
            reason: FerricProtectedVerifierListenerFailureReasonV2::PeerCredentials,
            source: ListenerFailureSourceV2::AcceptedIo {
                source: source.into(),
                accepted,
            },
        }
    }

    fn accepted(reason: FerricProtectedVerifierListenerFailureReasonV2, accepted: OwnedFd) -> Self {
        Self {
            reason,
            source: ListenerFailureSourceV2::Accepted {
                accepted,
                observed: None,
            },
        }
    }

    fn peer_identity_mismatch(
        accepted: OwnedFd,
        observed: FerricProtectedVerifierPeerCredentialsV2,
    ) -> Self {
        Self {
            reason: FerricProtectedVerifierListenerFailureReasonV2::PeerIdentityMismatch,
            source: ListenerFailureSourceV2::Accepted {
                accepted,
                observed: Some(observed),
            },
        }
    }

    /// Returns the stable failure stage without releasing retained custody.
    #[must_use]
    pub const fn reason(&self) -> FerricProtectedVerifierListenerFailureReasonV2 {
        self.reason
    }

    /// Returns admitted kernel peer credentials for an exact-identity mismatch.
    #[must_use]
    pub const fn observed_peer_credentials(
        &self,
    ) -> Option<FerricProtectedVerifierPeerCredentialsV2> {
        match &self.source {
            ListenerFailureSourceV2::Accepted { observed, .. } => *observed,
            ListenerFailureSourceV2::None
            | ListenerFailureSourceV2::Io(_)
            | ListenerFailureSourceV2::Receiver(_)
            | ListenerFailureSourceV2::AcceptedIo { .. }
            | ListenerFailureSourceV2::Endpoint(_)
            | ListenerFailureSourceV2::Session(_) => None,
        }
    }

    /// Returns the exact accepted endpoint for a pre-handoff or endpoint-admission rejection.
    #[must_use]
    pub fn into_accepted_endpoint(self) -> Option<OwnedFd> {
        match self.source {
            ListenerFailureSourceV2::AcceptedIo { accepted, .. }
            | ListenerFailureSourceV2::Accepted { accepted, .. } => Some(accepted),
            ListenerFailureSourceV2::Endpoint(failure) => Some(failure.into_control()),
            ListenerFailureSourceV2::None
            | ListenerFailureSourceV2::Io(_)
            | ListenerFailureSourceV2::Receiver(_)
            | ListenerFailureSourceV2::Session(_) => None,
        }
    }

    /// Returns the existing post-admission session failure, when that stage was reached.
    #[must_use]
    pub fn into_session_failure(self) -> Option<FerricProtectedVerifierServiceFailureV1> {
        match self.source {
            ListenerFailureSourceV2::Session(failure) => Some(failure),
            ListenerFailureSourceV2::None
            | ListenerFailureSourceV2::Io(_)
            | ListenerFailureSourceV2::Receiver(_)
            | ListenerFailureSourceV2::AcceptedIo { .. }
            | ListenerFailureSourceV2::Accepted { .. }
            | ListenerFailureSourceV2::Endpoint(_) => None,
        }
    }
}

impl fmt::Debug for FerricProtectedVerifierListenerFailureV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("FerricProtectedVerifierListenerFailureV2");
        debug.field("reason", &self.reason);
        match &self.source {
            ListenerFailureSourceV2::Io(source)
            | ListenerFailureSourceV2::AcceptedIo { source, .. } => {
                debug.field("source", source);
            }
            ListenerFailureSourceV2::Receiver(source) => {
                debug.field("source", source);
            }
            ListenerFailureSourceV2::Endpoint(source) => {
                debug.field("source", source);
            }
            ListenerFailureSourceV2::Session(source) => {
                debug.field("source", source);
            }
            ListenerFailureSourceV2::None | ListenerFailureSourceV2::Accepted { .. } => {}
        }
        debug
            .field(
                "accepted_endpoint_retained",
                &matches!(
                    self.source,
                    ListenerFailureSourceV2::AcceptedIo { .. }
                        | ListenerFailureSourceV2::Accepted { .. }
                        | ListenerFailureSourceV2::Endpoint(_)
                ),
            )
            .finish_non_exhaustive()
    }
}

impl fmt::Display for FerricProtectedVerifierListenerFailureV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Ferric protected-verifier listener failed at {:?}",
            self.reason
        )
    }
}

impl Error for FerricProtectedVerifierListenerFailureV2 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.source {
            ListenerFailureSourceV2::Io(source)
            | ListenerFailureSourceV2::AcceptedIo { source, .. } => Some(source),
            ListenerFailureSourceV2::Receiver(source) => Some(source),
            ListenerFailureSourceV2::Endpoint(source) => Some(source),
            ListenerFailureSourceV2::Session(source) => Some(source),
            ListenerFailureSourceV2::None | ListenerFailureSourceV2::Accepted { .. } => None,
        }
    }
}

#[derive(Clone, Copy)]
struct NodeIdentityV2 {
    device: u64,
    inode: u64,
}

impl NodeIdentityV2 {
    const fn from_stat(stat: &rustix::fs::Stat) -> Self {
        Self {
            device: stat.st_dev,
            inode: stat.st_ino,
        }
    }

    const fn matches(self, stat: &rustix::fs::Stat) -> bool {
        self.device == stat.st_dev && self.inode == stat.st_ino
    }
}

struct BoundSocketPathV2 {
    parent: OwnedFd,
    parent_path: PathBuf,
    parent_identity: NodeIdentityV2,
    basename: OsString,
    socket: NodeIdentityV2,
    armed: bool,
}

enum CleanupErrorV2 {
    IdentityChanged,
    Io(io::Error),
}

impl BoundSocketPathV2 {
    fn cleanup(&mut self) -> Result<(), CleanupErrorV2> {
        if !self.armed {
            return Ok(());
        }
        let parent_changed = rustix::fs::statat(
            rustix::fs::ABS,
            &self.parent_path,
            AtFlags::SYMLINK_NOFOLLOW,
        )
        .map_or(true, |current| {
            !self.parent_identity.matches(&current)
                || FileType::from_raw_mode(current.st_mode) != FileType::Directory
        });
        let current = rustix::fs::statat(&self.parent, &self.basename, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(|source| {
            if source == rustix::io::Errno::NOENT {
                CleanupErrorV2::IdentityChanged
            } else {
                CleanupErrorV2::Io(source.into())
            }
        })?;
        if !self.socket.matches(&current)
            || FileType::from_raw_mode(current.st_mode) != FileType::Socket
        {
            return Err(CleanupErrorV2::IdentityChanged);
        }
        rustix::fs::unlinkat(&self.parent, &self.basename, AtFlags::empty())
            .map_err(|source| CleanupErrorV2::Io(source.into()))?;
        self.armed = false;
        if parent_changed {
            Err(CleanupErrorV2::IdentityChanged)
        } else {
            Ok(())
        }
    }
}

impl Drop for BoundSocketPathV2 {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Binds, accepts, and serves one exact filesystem-path protected-verifier connection.
///
/// One absolute monotonic deadline covers parent/path admission, bind, accept, endpoint admission,
/// and the complete existing Ferric session. The parent must be an exact `0700` directory owned by
/// the service effective UID and is opened with `openat2` without symbolic links. The new socket
/// must remain the exact captured device/inode at mode `0600`, receiver credential stamping is
/// enabled before backlog-one `listen`, and cleanup unlinks only that exact captured node through
/// the retained parent descriptor. The connecting PID, UID, and GID must equal the service
/// configuration before the accepted descriptor is transferred into fe2o3.
///
/// This is transport supervision, not verifier authority. It provides no checker, signer,
/// antirollback store, process isolation, privilege transition, restart loop, or Qwen server.
///
/// # Errors
///
/// Returns a typed failure. Credential and endpoint-admission failures retain the accepted
/// descriptor. Post-admission failures preserve only the existing Ferric/fe2o3 terminal custody.
pub fn run_ferric_protected_verifier_listener_session_v2<C, K, S>(
    expected_service_path: &Path,
    config: &mut FerricProtectedVerifierServiceConfigV1<C, K, S>,
) -> Result<FerricProtectedVerifierServiceOutcomeV1, FerricProtectedVerifierListenerFailureV2>
where
    C: ProtectedCompilerCurrentRecordProviderV1,
    K: IndependentCheckerProviderV1,
    S: ProtectedReceiptSignerProviderV1,
{
    let deadline = AbsoluteSessionDeadlineV1::after(config.timeout()).ok_or_else(|| {
        FerricProtectedVerifierListenerFailureV2::plain(
            FerricProtectedVerifierListenerFailureReasonV2::DeadlineOverflow,
        )
    })?;
    let (parent_path, basename, address) = canonical_service_path(expected_service_path)
        .ok_or_else(|| {
            FerricProtectedVerifierListenerFailureV2::plain(
                FerricProtectedVerifierListenerFailureReasonV2::InvalidServicePath,
            )
        })?;
    require_live(deadline)?;

    let parent = rustix::fs::openat2(
        rustix::fs::ABS,
        parent_path,
        OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|source| FerricProtectedVerifierListenerFailureV2 {
        reason: FerricProtectedVerifierListenerFailureReasonV2::ParentAdmission,
        source: ListenerFailureSourceV2::Io(source.into()),
    })?;
    let parent_identity = NodeIdentityV2::from_stat(
        &rustix::fs::fstat(&parent).map_err(FerricProtectedVerifierListenerFailureV2::io)?,
    );
    verify_private_parent(&parent, parent_path, parent_identity)?;
    match rustix::fs::statat(&parent, basename, AtFlags::SYMLINK_NOFOLLOW) {
        Err(rustix::io::Errno::NOENT) => {}
        Ok(_) => {
            return Err(FerricProtectedVerifierListenerFailureV2::plain(
                FerricProtectedVerifierListenerFailureReasonV2::PathAlreadyExists,
            ));
        }
        Err(source) => return Err(FerricProtectedVerifierListenerFailureV2::io(source)),
    }
    require_live(deadline)?;

    let listener = rustix::net::socket_with(
        AddressFamily::UNIX,
        SocketType::SEQPACKET,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .map_err(FerricProtectedVerifierListenerFailureV2::io)?;
    verify_private_parent(&parent, parent_path, parent_identity)?;
    rustix::net::bind(&listener, &address).map_err(FerricProtectedVerifierListenerFailureV2::io)?;

    let socket_stat = rustix::fs::statat(&parent, basename, AtFlags::SYMLINK_NOFOLLOW)
        .map_err(FerricProtectedVerifierListenerFailureV2::io)?;
    let socket_identity = NodeIdentityV2::from_stat(&socket_stat);
    if FileType::from_raw_mode(socket_stat.st_mode) != FileType::Socket {
        return Err(FerricProtectedVerifierListenerFailureV2::plain(
            FerricProtectedVerifierListenerFailureReasonV2::PathIdentityChanged,
        ));
    }
    let mut bound_path = BoundSocketPathV2 {
        parent,
        parent_path: parent_path.to_path_buf(),
        parent_identity,
        basename: basename.to_os_string(),
        socket: socket_identity,
        armed: true,
    };
    let ambient_socket_stat = match rustix::fs::statat(
        rustix::fs::ABS,
        expected_service_path,
        AtFlags::SYMLINK_NOFOLLOW,
    ) {
        Ok(stat) => stat,
        Err(source) => {
            let failure = FerricProtectedVerifierListenerFailureV2::io(source);
            return Err(cleanup_before_return(&mut bound_path, failure, None));
        }
    };
    if !socket_identity.matches(&ambient_socket_stat)
        || FileType::from_raw_mode(ambient_socket_stat.st_mode) != FileType::Socket
    {
        let failure = FerricProtectedVerifierListenerFailureV2::plain(
            FerricProtectedVerifierListenerFailureReasonV2::PathIdentityChanged,
        );
        return Err(cleanup_before_return(&mut bound_path, failure, None));
    }
    if let Err(failure) = verify_private_parent(&bound_path.parent, parent_path, parent_identity) {
        return Err(cleanup_before_return(&mut bound_path, failure, None));
    }
    if let Err(source) = rustix::fs::chmodat(
        &bound_path.parent,
        &bound_path.basename,
        SOCKET_MODE_V2,
        AtFlags::empty(),
    ) {
        let failure = FerricProtectedVerifierListenerFailureV2::io(source);
        return Err(cleanup_before_return(&mut bound_path, failure, None));
    }
    if let Err(failure) = verify_socket_path(&bound_path, SOCKET_MODE_V2) {
        return Err(cleanup_before_return(&mut bound_path, failure, None));
    }
    if let Err(source) = prepare_worker_v3_verification_receiver_v1(&listener) {
        let failure = FerricProtectedVerifierListenerFailureV2 {
            reason: FerricProtectedVerifierListenerFailureReasonV2::ReceiverPreparation,
            source: ListenerFailureSourceV2::Receiver(source),
        };
        return Err(cleanup_before_return(&mut bound_path, failure, None));
    }
    if let Err(source) = rustix::net::listen(&listener, 1) {
        let failure = FerricProtectedVerifierListenerFailureV2::io(source);
        return Err(cleanup_before_return(&mut bound_path, failure, None));
    }
    if let Err(failure) = require_live(deadline) {
        return Err(cleanup_before_return(&mut bound_path, failure, None));
    }

    let accepted = match accept_until(&listener, deadline.instant()) {
        Ok(accepted) => accepted,
        Err(failure) => return Err(cleanup_before_return(&mut bound_path, failure, None)),
    };
    drop(listener);
    if let Err(cleanup) = bound_path.cleanup() {
        return Err(cleanup_failure(cleanup, Some(accepted)));
    }
    require_live_with_accepted(deadline, accepted).and_then(|accepted| {
        let observed = match accepted_peer_credentials(&accepted) {
            Ok(observed) => observed,
            Err(source) => {
                return Err(FerricProtectedVerifierListenerFailureV2::accepted_io(
                    source, accepted,
                ));
            }
        };
        if !credentials_match(config.caller_policy(), observed) {
            return Err(
                FerricProtectedVerifierListenerFailureV2::peer_identity_mismatch(
                    accepted, observed,
                ),
            );
        }
        let endpoint =
            WorkerV3VerificationAcceptedServiceEndpointV2::admit(accepted, expected_service_path)
                .map_err(|failure| FerricProtectedVerifierListenerFailureV2 {
                reason: FerricProtectedVerifierListenerFailureReasonV2::EndpointAdmission,
                source: ListenerFailureSourceV2::Endpoint(failure),
            })?;
        run_ferric_protected_verifier_accepted_session_until_v2(endpoint, deadline, config).map_err(
            |failure| FerricProtectedVerifierListenerFailureV2 {
                reason: FerricProtectedVerifierListenerFailureReasonV2::Session,
                source: ListenerFailureSourceV2::Session(failure),
            },
        )
    })
}

fn canonical_service_path(path: &Path) -> Option<(&Path, &OsStr, SocketAddrUnix)> {
    let bytes = path.as_os_str().as_bytes();
    if bytes.is_empty() || bytes.first() != Some(&b'/') || bytes.last() == Some(&b'/') {
        return None;
    }
    let mut components = bytes.split(|byte| *byte == b'/');
    if components.next() != Some(&[][..])
        || !components
            .all(|component| !component.is_empty() && component != b"." && component != b"..")
    {
        return None;
    }
    Some((
        path.parent()?,
        path.file_name()?,
        SocketAddrUnix::new(path).ok()?,
    ))
}

fn verify_private_parent(
    parent: &OwnedFd,
    path: &Path,
    identity: NodeIdentityV2,
) -> Result<(), FerricProtectedVerifierListenerFailureV2> {
    let retained =
        rustix::fs::fstat(parent).map_err(FerricProtectedVerifierListenerFailureV2::io)?;
    let current = rustix::fs::statat(rustix::fs::ABS, path, AtFlags::SYMLINK_NOFOLLOW)
        .map_err(FerricProtectedVerifierListenerFailureV2::io)?;
    if !identity.matches(&retained)
        || !identity.matches(&current)
        || FileType::from_raw_mode(retained.st_mode) != FileType::Directory
        || FileType::from_raw_mode(current.st_mode) != FileType::Directory
        || retained.st_uid != rustix::process::geteuid().as_raw()
        || retained.st_mode & 0o7777 != Mode::RWXU.bits()
    {
        return Err(FerricProtectedVerifierListenerFailureV2::plain(
            FerricProtectedVerifierListenerFailureReasonV2::ParentAdmission,
        ));
    }
    Ok(())
}

fn verify_socket_path(
    path: &BoundSocketPathV2,
    mode: Mode,
) -> Result<(), FerricProtectedVerifierListenerFailureV2> {
    let current = rustix::fs::statat(&path.parent, &path.basename, AtFlags::SYMLINK_NOFOLLOW)
        .map_err(FerricProtectedVerifierListenerFailureV2::io)?;
    if !path.socket.matches(&current)
        || FileType::from_raw_mode(current.st_mode) != FileType::Socket
    {
        return Err(FerricProtectedVerifierListenerFailureV2::plain(
            FerricProtectedVerifierListenerFailureReasonV2::PathIdentityChanged,
        ));
    }
    if current.st_mode & 0o7777 != mode.bits() {
        return Err(FerricProtectedVerifierListenerFailureV2::plain(
            FerricProtectedVerifierListenerFailureReasonV2::SocketModeMismatch,
        ));
    }
    Ok(())
}

fn accept_until(
    listener: &OwnedFd,
    deadline: Instant,
) -> Result<OwnedFd, FerricProtectedVerifierListenerFailureV2> {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(FerricProtectedVerifierListenerFailureV2::plain(
                FerricProtectedVerifierListenerFailureReasonV2::DeadlineExpired,
            ));
        }
        let timeout = Timespec {
            tv_sec: i64::try_from(remaining.as_secs()).unwrap_or(i64::MAX),
            tv_nsec: i64::from(remaining.subsec_nanos()),
        };
        let mut descriptors = [PollFd::new(
            listener,
            PollFlags::IN | PollFlags::ERR | PollFlags::HUP,
        )];
        match poll(&mut descriptors, Some(&timeout)) {
            Ok(0) => {
                return Err(FerricProtectedVerifierListenerFailureV2::plain(
                    FerricProtectedVerifierListenerFailureReasonV2::DeadlineExpired,
                ));
            }
            Ok(_) if descriptors[0].revents().contains(PollFlags::NVAL) => {
                return Err(FerricProtectedVerifierListenerFailureV2::plain(
                    FerricProtectedVerifierListenerFailureReasonV2::ListenerIo,
                ));
            }
            Ok(_) if descriptors[0].revents().intersects(PollFlags::IN) => {
                match rustix::net::accept_with(
                    listener,
                    SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
                ) {
                    Ok(accepted) => return Ok(accepted),
                    Err(rustix::io::Errno::INTR | rustix::io::Errno::AGAIN) => {}
                    Err(source) => {
                        return Err(FerricProtectedVerifierListenerFailureV2::io(source));
                    }
                }
            }
            Ok(_) => {
                return Err(FerricProtectedVerifierListenerFailureV2::plain(
                    FerricProtectedVerifierListenerFailureReasonV2::ListenerIo,
                ));
            }
            Err(rustix::io::Errno::INTR) => {}
            Err(source) => return Err(FerricProtectedVerifierListenerFailureV2::io(source)),
        }
    }
}

fn accepted_peer_credentials(
    accepted: &OwnedFd,
) -> Result<FerricProtectedVerifierPeerCredentialsV2, io::Error> {
    let credentials = rustix::net::sockopt::socket_peercred(accepted)?;
    let pid = u32::try_from(credentials.pid.as_raw_nonzero().get())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "peer PID does not fit u32"))?;
    Ok(FerricProtectedVerifierPeerCredentialsV2 {
        pid,
        uid: credentials.uid.as_raw(),
        gid: credentials.gid.as_raw(),
    })
}

const fn credentials_match(
    expected: ServiceCallerPolicyV1,
    observed: FerricProtectedVerifierPeerCredentialsV2,
) -> bool {
    expected.pid() == observed.pid
        && expected.uid() == observed.uid
        && expected.gid() == observed.gid
}

fn require_live(
    deadline: AbsoluteSessionDeadlineV1,
) -> Result<(), FerricProtectedVerifierListenerFailureV2> {
    deadline.remaining().map(|_| ()).ok_or_else(|| {
        FerricProtectedVerifierListenerFailureV2::plain(
            FerricProtectedVerifierListenerFailureReasonV2::DeadlineExpired,
        )
    })
}

fn require_live_with_accepted(
    deadline: AbsoluteSessionDeadlineV1,
    accepted: OwnedFd,
) -> Result<OwnedFd, FerricProtectedVerifierListenerFailureV2> {
    if deadline.remaining().is_some() {
        Ok(accepted)
    } else {
        Err(FerricProtectedVerifierListenerFailureV2::accepted(
            FerricProtectedVerifierListenerFailureReasonV2::DeadlineExpired,
            accepted,
        ))
    }
}

fn cleanup_before_return(
    path: &mut BoundSocketPathV2,
    failure: FerricProtectedVerifierListenerFailureV2,
    accepted: Option<OwnedFd>,
) -> FerricProtectedVerifierListenerFailureV2 {
    match path.cleanup() {
        Ok(()) => match accepted {
            Some(accepted) => {
                FerricProtectedVerifierListenerFailureV2::accepted(failure.reason(), accepted)
            }
            None => failure,
        },
        Err(cleanup) => cleanup_failure(cleanup, accepted),
    }
}

fn cleanup_failure(
    cleanup: CleanupErrorV2,
    accepted: Option<OwnedFd>,
) -> FerricProtectedVerifierListenerFailureV2 {
    match (cleanup, accepted) {
        (CleanupErrorV2::IdentityChanged, Some(accepted)) => {
            FerricProtectedVerifierListenerFailureV2::accepted(
                FerricProtectedVerifierListenerFailureReasonV2::PathIdentityChanged,
                accepted,
            )
        }
        (CleanupErrorV2::IdentityChanged, None) => FerricProtectedVerifierListenerFailureV2::plain(
            FerricProtectedVerifierListenerFailureReasonV2::PathIdentityChanged,
        ),
        (CleanupErrorV2::Io(source), Some(accepted)) => FerricProtectedVerifierListenerFailureV2 {
            reason: FerricProtectedVerifierListenerFailureReasonV2::ListenerIo,
            source: ListenerFailureSourceV2::AcceptedIo { source, accepted },
        },
        (CleanupErrorV2::Io(source), None) => FerricProtectedVerifierListenerFailureV2::io(source),
    }
}
