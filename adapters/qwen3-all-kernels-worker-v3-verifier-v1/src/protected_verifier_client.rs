//! One-shot bounded client for the aggregate protected-verifier service.
//!
//! The client admits only a connected unnamed Unix `SOCK_SEQPACKET` peer with
//! caller-pinned, dedicated service credentials. One absolute monotonic
//! deadline covers the entire exchange. The client authenticates the returned
//! receipt under the caller-provisioned trust policy after exact packet
//! correlation and closes the peer on every terminal path. Consuming one
//! client prevents reuse of that local session object, not replay across new
//! sessions: global freshness requires service-side atomic challenge
//! consumption and protected live current-ledger validation.

#![allow(
    clippy::must_use_candidate,
    reason = "public accessors expose inert identities or retained ownership"
)]

use core::fmt;
use std::error::Error;
use std::io;
use std::mem;
use std::os::fd::{AsRawFd, OwnedFd};
use std::time::{Duration, Instant};

use crate::protected_receipt::{
    M1AllKernelsAuthenticatedProtectedVerifierReceiptV1, M1AllKernelsProtectedReceiptErrorV1,
    M1AllKernelsProtectedVerifierTrustPolicyV1,
};
use crate::protected_verifier_service::{
    M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1,
    M1AllKernelsProtectedVerifierServiceProtocolErrorV1,
    M1AllKernelsProtectedVerifierServiceRequestV1, M1AllKernelsProtectedVerifierServiceResponseV1,
};

const INVALID_ID: u32 = u32::MAX;

/// Caller-pinned kernel credential identity for the protected-verifier peer.
///
/// The UID and GID must identify a dedicated non-root service. This value
/// carries no socket pathname, key, measurement, policy, or verifier authority.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct M1AllKernelsProtectedVerifierServiceIdentityV1 {
    uid: u32,
    gid: u32,
}

impl M1AllKernelsProtectedVerifierServiceIdentityV1 {
    /// Constructs one dedicated non-root service identity.
    ///
    /// # Errors
    ///
    /// Returns a typed error for root or Linux invalid-ID sentinels.
    pub const fn new(
        uid: u32,
        gid: u32,
    ) -> Result<Self, M1AllKernelsProtectedVerifierServiceIdentityErrorV1> {
        if uid == 0 || uid == INVALID_ID {
            return Err(M1AllKernelsProtectedVerifierServiceIdentityErrorV1::InvalidUid);
        }
        if gid == 0 || gid == INVALID_ID {
            return Err(M1AllKernelsProtectedVerifierServiceIdentityErrorV1::InvalidGid);
        }
        Ok(Self { uid, gid })
    }

    /// Returns the expected effective service UID from `SO_PEERCRED`.
    pub const fn uid(self) -> u32 {
        self.uid
    }

    /// Returns the expected effective service GID from `SO_PEERCRED`.
    pub const fn gid(self) -> u32 {
        self.gid
    }

    /// A kernel credential expectation grants no verifier authority.
    pub const fn grants_authority(self) -> bool {
        false
    }
}

impl fmt::Debug for M1AllKernelsProtectedVerifierServiceIdentityV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AllKernelsProtectedVerifierServiceIdentityV1")
            .field("uid", &self.uid)
            .field("gid", &self.gid)
            .field("authority", &"none")
            .finish()
    }
}

/// Invalid dedicated protected-verifier service credentials.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1AllKernelsProtectedVerifierServiceIdentityErrorV1 {
    /// UID is root or the Linux invalid-ID sentinel.
    InvalidUid,
    /// GID is root or the Linux invalid-ID sentinel.
    InvalidGid,
}

impl fmt::Display for M1AllKernelsProtectedVerifierServiceIdentityErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidUid => "invalid aggregate protected-verifier service UID",
            Self::InvalidGid => "invalid aggregate protected-verifier service GID",
        })
    }
}

impl Error for M1AllKernelsProtectedVerifierServiceIdentityErrorV1 {}

/// Ownership-retaining failure from service-peer admission.
///
/// The caller can inspect the stable reason and recover the exact `OwnedFd`.
/// No bytes have been sent when this value is returned.
pub struct M1AllKernelsProtectedVerifierClientAdmissionFailureV1 {
    error: M1AllKernelsProtectedVerifierClientErrorV1,
    peer: OwnedFd,
}

impl M1AllKernelsProtectedVerifierClientAdmissionFailureV1 {
    /// Returns the stable admission error without releasing the peer.
    pub const fn error(&self) -> &M1AllKernelsProtectedVerifierClientErrorV1 {
        &self.error
    }

    /// Returns the exact owned peer to the caller.
    pub fn into_peer(self) -> OwnedFd {
        self.peer
    }

    /// Returns both the stable error and the exact owned peer.
    pub fn into_parts(self) -> (M1AllKernelsProtectedVerifierClientErrorV1, OwnedFd) {
        (self.error, self.peer)
    }
}

impl fmt::Debug for M1AllKernelsProtectedVerifierClientAdmissionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AllKernelsProtectedVerifierClientAdmissionFailureV1")
            .field("error", &self.error)
            .field("retains_peer", &true)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for M1AllKernelsProtectedVerifierClientAdmissionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl Error for M1AllKernelsProtectedVerifierClientAdmissionFailureV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error.source()
    }
}

/// One owned, bounded connection to an externally supervised protected verifier.
///
/// The value is deliberately move-only and does not expose its descriptor. A
/// successful exchange consumes it, preventing a second request from replaying
/// the same session state.
pub struct M1AllKernelsProtectedVerifierClientV1 {
    peer: OwnedFd,
    expected_service: M1AllKernelsProtectedVerifierServiceIdentityV1,
    peer_pid: u32,
    deadline: Instant,
}

impl fmt::Debug for M1AllKernelsProtectedVerifierClientV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AllKernelsProtectedVerifierClientV1")
            .field("expected_service", &self.expected_service)
            .field("peer_pid", &self.peer_pid)
            .field("deadline", &self.deadline)
            .field("authority", &"none")
            .finish_non_exhaustive()
    }
}

impl M1AllKernelsProtectedVerifierClientV1 {
    /// Admits one caller-owned, connected, unnamed Unix `SOCK_SEQPACKET` peer.
    ///
    /// Production admission requires the peer's `SO_PEERCRED` UID to differ
    /// from the client effective UID. Failure returns ownership of the original
    /// peer and never transmits bytes.
    ///
    /// # Errors
    ///
    /// Returns an ownership-retaining failure for invalid timeouts, descriptor
    /// shape, endpoint addresses, peer credentials, or expired admission.
    pub fn admit(
        peer: OwnedFd,
        expected_service: M1AllKernelsProtectedVerifierServiceIdentityV1,
        timeout: Duration,
    ) -> Result<Self, M1AllKernelsProtectedVerifierClientAdmissionFailureV1> {
        Self::admit_inner::<true>(peer, expected_service, timeout)
    }

    fn admit_inner<const REQUIRE_DISTINCT_UID: bool>(
        peer: OwnedFd,
        expected_service: M1AllKernelsProtectedVerifierServiceIdentityV1,
        timeout: Duration,
    ) -> Result<Self, M1AllKernelsProtectedVerifierClientAdmissionFailureV1> {
        match admit_peer::<REQUIRE_DISTINCT_UID>(&peer, expected_service, timeout) {
            Ok((peer_pid, deadline)) => Ok(Self {
                peer,
                expected_service,
                peer_pid,
                deadline,
            }),
            Err(error) => {
                Err(M1AllKernelsProtectedVerifierClientAdmissionFailureV1 { error, peer })
            }
        }
    }

    /// Sends one exact request and authenticates one exact correlated receipt.
    ///
    /// The trust policy is caller-provisioned and must exactly match the request
    /// identity. Packet framing and response identities detect corruption; the
    /// Ed25519 receipt verification authenticates the protected result. The
    /// result remains authority-free until a later reviewed boundary binds it
    /// to locally retained request, evidence-custody, and audit owners.
    ///
    /// # Errors
    ///
    /// Returns a typed error for transport, peer-continuity, protocol,
    /// correlation, policy, or signature-authentication failure.
    pub fn request_receipt(
        self,
        policy: &M1AllKernelsProtectedVerifierTrustPolicyV1,
        request: &M1AllKernelsProtectedVerifierServiceRequestV1,
    ) -> Result<
        M1AllKernelsAuthenticatedProtectedVerifierReceiptV1,
        M1AllKernelsProtectedVerifierClientErrorV1,
    > {
        if policy.identity() != request.trust_policy_identity() {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::TrustPolicyMismatch);
        }
        revalidate_peer(&self.peer, self.expected_service, self.peer_pid)?;
        send_packet(&self.peer, request.canonical_bytes(), self.deadline)?;
        let received = receive_packet(&self.peer, self.deadline)?;
        revalidate_peer(&self.peer, self.expected_service, self.peer_pid)?;
        let response = M1AllKernelsProtectedVerifierServiceResponseV1::decode(&received)?;
        if !response.matches_request(request) {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::ResponseRequestMismatch);
        }
        let receipt = response.into_receipt();
        let authenticated = policy
            .authenticate_canonical(receipt.encode_canonical())
            .map_err(M1AllKernelsProtectedVerifierClientErrorV1::ReceiptAuthentication)?;
        require_deadline(self.deadline)?;
        Ok(authenticated)
    }

    /// This transport client itself grants no protected-verifier authority.
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

fn admit_peer<const REQUIRE_DISTINCT_UID: bool>(
    peer: &OwnedFd,
    expected_service: M1AllKernelsProtectedVerifierServiceIdentityV1,
    timeout: Duration,
) -> Result<(u32, Instant), M1AllKernelsProtectedVerifierClientErrorV1> {
    if timeout.is_zero() {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::InvalidTimeout);
    }
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or(M1AllKernelsProtectedVerifierClientErrorV1::DeadlineOverflow)?;
    set_close_on_exec(peer)?;
    validate_seqpacket_peer(peer)?;
    let credentials = peer_credentials(peer)?;
    if credentials.uid != expected_service.uid || credentials.gid != expected_service.gid {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::PeerCredentialsMismatch);
    }
    // SAFETY: geteuid reads one process credential and has no pointer arguments.
    let client_uid = unsafe { libc::geteuid() };
    if REQUIRE_DISTINCT_UID && credentials.uid == client_uid {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::ClientAndServiceUidMatch);
    }
    require_deadline(deadline)?;
    Ok((credentials.pid, deadline))
}

fn set_close_on_exec(peer: &OwnedFd) -> Result<(), M1AllKernelsProtectedVerifierClientErrorV1> {
    // SAFETY: the peer is live and F_GETFD has no pointer arguments.
    let flags = unsafe { libc::fcntl(peer.as_raw_fd(), libc::F_GETFD) };
    if flags < 0 {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::Descriptor(
            io::Error::last_os_error(),
        ));
    }
    // SAFETY: F_SETFD writes only descriptor flags for the retained peer.
    if unsafe { libc::fcntl(peer.as_raw_fd(), libc::F_SETFD, flags | libc::FD_CLOEXEC) } != 0 {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::Descriptor(
            io::Error::last_os_error(),
        ));
    }
    Ok(())
}

fn validate_seqpacket_peer(
    peer: &OwnedFd,
) -> Result<(), M1AllKernelsProtectedVerifierClientErrorV1> {
    let mut socket_type = 0_i32;
    let mut socket_type_len = libc::socklen_t::try_from(mem::size_of::<i32>())
        .expect("socket-type scalar length fits socklen_t");
    // SAFETY: output pointers name initialized writable scalar storage of the declared length.
    if unsafe {
        libc::getsockopt(
            peer.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_TYPE,
            (&raw mut socket_type).cast(),
            &raw mut socket_type_len,
        )
    } != 0
    {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::Descriptor(
            io::Error::last_os_error(),
        ));
    }
    if usize::try_from(socket_type_len).ok() != Some(mem::size_of::<i32>())
        || socket_type != libc::SOCK_SEQPACKET
    {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::NotSeqpacket);
    }
    validate_unnamed_address(peer, false)?;
    validate_unnamed_address(peer, true)
}

fn validate_unnamed_address(
    peer: &OwnedFd,
    remote: bool,
) -> Result<(), M1AllKernelsProtectedVerifierClientErrorV1> {
    // SAFETY: zero is a valid initial sockaddr_storage and the kernel initializes the result.
    let mut address = unsafe { mem::zeroed::<libc::sockaddr_storage>() };
    let mut length = libc::socklen_t::try_from(mem::size_of::<libc::sockaddr_storage>())
        .expect("socket-address length fits socklen_t");
    // SAFETY: the address and length pointers name writable storage of the declared capacity.
    let result = unsafe {
        if remote {
            libc::getpeername(peer.as_raw_fd(), (&raw mut address).cast(), &raw mut length)
        } else {
            libc::getsockname(peer.as_raw_fd(), (&raw mut address).cast(), &raw mut length)
        }
    };
    if result != 0 {
        let error = io::Error::last_os_error();
        if remote && error.raw_os_error() == Some(libc::ENOTCONN) {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::NamedOrNonUnixPeer);
        }
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::Descriptor(
            error,
        ));
    }
    if i32::from(address.ss_family) != libc::AF_UNIX
        || usize::try_from(length).ok() != Some(mem::size_of::<libc::sa_family_t>())
    {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::NamedOrNonUnixPeer);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PeerCredentials {
    pid: u32,
    uid: u32,
    gid: u32,
}

fn peer_credentials(
    peer: &OwnedFd,
) -> Result<PeerCredentials, M1AllKernelsProtectedVerifierClientErrorV1> {
    // SAFETY: zero initializes every scalar field of Linux ucred.
    let mut credentials = unsafe { mem::zeroed::<libc::ucred>() };
    let mut length = libc::socklen_t::try_from(mem::size_of::<libc::ucred>())
        .expect("peer-credential length fits socklen_t");
    // SAFETY: output pointers name initialized writable ucred storage of the declared length.
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
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::PeerCredentials(
            io::Error::last_os_error(),
        ));
    }
    let pid = u32::try_from(credentials.pid)
        .ok()
        .filter(|pid| *pid != 0)
        .ok_or(M1AllKernelsProtectedVerifierClientErrorV1::InvalidPeerPid)?;
    if usize::try_from(length).ok() != Some(mem::size_of::<libc::ucred>())
        || credentials.uid == INVALID_ID
        || credentials.gid == INVALID_ID
    {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::InvalidPeerCredentials);
    }
    Ok(PeerCredentials {
        pid,
        uid: credentials.uid,
        gid: credentials.gid,
    })
}

fn revalidate_peer(
    peer: &OwnedFd,
    expected_service: M1AllKernelsProtectedVerifierServiceIdentityV1,
    expected_pid: u32,
) -> Result<(), M1AllKernelsProtectedVerifierClientErrorV1> {
    validate_seqpacket_peer(peer)?;
    let current = peer_credentials(peer)?;
    if current.pid != expected_pid
        || current.uid != expected_service.uid
        || current.gid != expected_service.gid
    {
        return Err(M1AllKernelsProtectedVerifierClientErrorV1::PeerIdentityChanged);
    }
    Ok(())
}

fn send_packet(
    peer: &OwnedFd,
    bytes: &[u8],
    deadline: Instant,
) -> Result<(), M1AllKernelsProtectedVerifierClientErrorV1> {
    loop {
        wait_for_peer(peer, libc::POLLOUT, deadline)?;
        // SAFETY: bytes is readable for its complete length and peer remains owned.
        let sent = unsafe {
            libc::send(
                peer.as_raw_fd(),
                bytes.as_ptr().cast(),
                bytes.len(),
                libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
            )
        };
        if sent < 0 {
            let error = io::Error::last_os_error();
            if matches!(
                error.kind(),
                io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
            ) {
                continue;
            }
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::Send(error));
        }
        if usize::try_from(sent).ok() != Some(bytes.len()) {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::PartialSend);
        }
        return Ok(());
    }
}

fn receive_packet(
    peer: &OwnedFd,
    deadline: Instant,
) -> Result<
    [u8; M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1],
    M1AllKernelsProtectedVerifierClientErrorV1,
> {
    let mut bytes = [0_u8; M1_ALL_KERNELS_PROTECTED_VERIFIER_SERVICE_RESPONSE_BYTES_V1];
    loop {
        wait_for_peer(peer, libc::POLLIN, deadline)?;
        let mut vector = libc::iovec {
            iov_base: bytes.as_mut_ptr().cast(),
            iov_len: bytes.len(),
        };
        // SAFETY: zero is a valid empty msghdr; the live iovec is installed below.
        let mut header = unsafe { mem::zeroed::<libc::msghdr>() };
        header.msg_iov = &raw mut vector;
        header.msg_iovlen = 1;
        // SAFETY: header names the live output buffer and no ancillary buffer.
        let received = unsafe {
            libc::recvmsg(
                peer.as_raw_fd(),
                &raw mut header,
                libc::MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC,
            )
        };
        if received < 0 {
            let error = io::Error::last_os_error();
            if matches!(
                error.kind(),
                io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
            ) {
                continue;
            }
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::Receive(error));
        }
        if header.msg_flags & libc::MSG_CTRUNC != 0 {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::AncillaryData);
        }
        if header.msg_flags & libc::MSG_TRUNC != 0 {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::PacketTruncated);
        }
        if header.msg_controllen != 0 {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::AncillaryData);
        }
        let received = usize::try_from(received)
            .map_err(|_| M1AllKernelsProtectedVerifierClientErrorV1::PacketTruncated)?;
        if received == 0 {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::PeerClosed);
        }
        if received != bytes.len() {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::ResponseLength {
                expected: bytes.len(),
                actual: received,
            });
        }
        return Ok(bytes);
    }
}

fn wait_for_peer(
    peer: &OwnedFd,
    wanted: i16,
    deadline: Instant,
) -> Result<(), M1AllKernelsProtectedVerifierClientErrorV1> {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::Timeout);
        }
        let mut descriptor = libc::pollfd {
            fd: peer.as_raw_fd(),
            events: wanted | libc::POLLERR | libc::POLLHUP,
            revents: 0,
        };
        // SAFETY: descriptor is a live one-element pollfd array for the complete call.
        let result =
            unsafe { libc::poll(&raw mut descriptor, 1, duration_to_poll_millis(remaining)) };
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::Poll(error));
        }
        if result == 0 || deadline.saturating_duration_since(Instant::now()).is_zero() {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::Timeout);
        }
        if descriptor.revents & libc::POLLNVAL != 0 {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::InvalidPeer);
        }
        if descriptor.revents & wanted != 0 {
            return Ok(());
        }
        if descriptor.revents & libc::POLLERR != 0 {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::PeerFailed);
        }
        if descriptor.revents & libc::POLLHUP != 0 {
            return Err(M1AllKernelsProtectedVerifierClientErrorV1::PeerClosed);
        }
    }
}

fn require_deadline(deadline: Instant) -> Result<(), M1AllKernelsProtectedVerifierClientErrorV1> {
    if deadline.saturating_duration_since(Instant::now()).is_zero() {
        Err(M1AllKernelsProtectedVerifierClientErrorV1::Timeout)
    } else {
        Ok(())
    }
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

/// Bounded client admission, transport, correlation, or authentication failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum M1AllKernelsProtectedVerifierClientErrorV1 {
    /// Session timeout must be nonzero.
    InvalidTimeout,
    /// Absolute deadline overflowed.
    DeadlineOverflow,
    /// Descriptor inspection or mutation failed.
    Descriptor(io::Error),
    /// Peer is not a Unix `SOCK_SEQPACKET` endpoint.
    NotSeqpacket,
    /// Peer is named, non-Unix, or disconnected.
    NamedOrNonUnixPeer,
    /// `SO_PEERCRED` inspection failed.
    PeerCredentials(io::Error),
    /// Peer PID is not positive.
    InvalidPeerPid,
    /// Peer credentials contain an invalid sentinel.
    InvalidPeerCredentials,
    /// Peer credentials differ from the caller-pinned identity.
    PeerCredentialsMismatch,
    /// Production client and protected service use the same effective UID.
    ClientAndServiceUidMatch,
    /// Peer process or credentials changed during the exchange.
    PeerIdentityChanged,
    /// Polling the peer failed.
    Poll(io::Error),
    /// Sending the request failed.
    Send(io::Error),
    /// Receiving the response failed.
    Receive(io::Error),
    /// Absolute session deadline expired.
    Timeout,
    /// Peer descriptor became invalid.
    InvalidPeer,
    /// Peer reported an asynchronous socket error.
    PeerFailed,
    /// Peer closed before completing the response.
    PeerClosed,
    /// Response exceeded its sole exact packet bound.
    PacketTruncated,
    /// Peer attempted to transfer ancillary data.
    AncillaryData,
    /// Atomic seqpacket send was partial.
    PartialSend,
    /// Response packet length was not exact.
    ResponseLength {
        /// Required byte length.
        expected: usize,
        /// Observed byte length.
        actual: usize,
    },
    /// Request names another caller-provisioned trust policy.
    TrustPolicyMismatch,
    /// Response does not correlate to the exact request and signed coordinates.
    ResponseRequestMismatch,
    /// Canonical service packet was invalid.
    Protocol(M1AllKernelsProtectedVerifierServiceProtocolErrorV1),
    /// Signed receipt did not authenticate under the caller-provisioned policy.
    ReceiptAuthentication(M1AllKernelsProtectedReceiptErrorV1),
}

impl fmt::Display for M1AllKernelsProtectedVerifierClientErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTimeout => {
                formatter.write_str("protected-verifier timeout must be nonzero")
            }
            Self::DeadlineOverflow => formatter.write_str("protected-verifier deadline overflowed"),
            Self::Descriptor(error) => {
                write!(formatter, "protected-verifier peer is invalid: {error}")
            }
            Self::NotSeqpacket => {
                formatter.write_str("protected-verifier peer is not SOCK_SEQPACKET")
            }
            Self::NamedOrNonUnixPeer => formatter
                .write_str("protected-verifier peer is not a connected unnamed Unix socket"),
            Self::PeerCredentials(error) => write!(
                formatter,
                "protected-verifier peer credentials failed: {error}"
            ),
            Self::InvalidPeerPid => formatter.write_str("protected-verifier peer PID is invalid"),
            Self::InvalidPeerCredentials => {
                formatter.write_str("protected-verifier peer credentials are invalid")
            }
            Self::PeerCredentialsMismatch => formatter.write_str(
                "protected-verifier peer credentials differ from the pinned service identity",
            ),
            Self::ClientAndServiceUidMatch => {
                formatter.write_str("protected-verifier client and service UIDs must differ")
            }
            Self::PeerIdentityChanged => {
                formatter.write_str("protected-verifier peer identity changed during the exchange")
            }
            Self::Poll(error) => write!(formatter, "protected-verifier poll failed: {error}"),
            Self::Send(error) => write!(formatter, "protected-verifier send failed: {error}"),
            Self::Receive(error) => write!(formatter, "protected-verifier receive failed: {error}"),
            Self::Timeout => formatter.write_str("protected-verifier absolute deadline expired"),
            Self::InvalidPeer => {
                formatter.write_str("protected-verifier peer descriptor became invalid")
            }
            Self::PeerFailed => formatter.write_str("protected-verifier peer reported an error"),
            Self::PeerClosed => formatter.write_str("protected-verifier peer closed"),
            Self::PacketTruncated => {
                formatter.write_str("protected-verifier response packet was truncated")
            }
            Self::AncillaryData => {
                formatter.write_str("protected-verifier response carried ancillary data")
            }
            Self::PartialSend => {
                formatter.write_str("protected-verifier request packet send was partial")
            }
            Self::ResponseLength { expected, actual } => write!(
                formatter,
                "protected-verifier response length {actual} differs from exact length {expected}"
            ),
            Self::TrustPolicyMismatch => {
                formatter.write_str("protected-verifier request names another trust policy")
            }
            Self::ResponseRequestMismatch => formatter
                .write_str("protected-verifier response names another request or coordinate set"),
            Self::Protocol(error) => {
                write!(formatter, "protected-verifier protocol failed: {error}")
            }
            Self::ReceiptAuthentication(error) => write!(
                formatter,
                "protected-verifier receipt authentication failed: {error}"
            ),
        }
    }
}

impl Error for M1AllKernelsProtectedVerifierClientErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Descriptor(error)
            | Self::PeerCredentials(error)
            | Self::Poll(error)
            | Self::Send(error)
            | Self::Receive(error) => Some(error),
            Self::Protocol(error) => Some(error),
            Self::ReceiptAuthentication(error) => Some(error),
            _ => None,
        }
    }
}

impl From<M1AllKernelsProtectedVerifierServiceProtocolErrorV1>
    for M1AllKernelsProtectedVerifierClientErrorV1
{
    fn from(error: M1AllKernelsProtectedVerifierServiceProtocolErrorV1) -> Self {
        Self::Protocol(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protected_verifier_service::M1AllKernelsProtectedVerifierServiceResponseV1;
    use crate::protected_verifier_test_support::{fixture_request, signed_fixture};
    use std::fs::File;
    use std::os::fd::{FromRawFd, IntoRawFd};
    use std::os::unix::net::UnixStream;
    use std::ptr;
    use std::thread;

    fn socketpair() -> (OwnedFd, OwnedFd) {
        let mut descriptors = [-1_i32; 2];
        // SAFETY: descriptors names writable storage for exactly two returned file descriptors.
        assert_eq!(
            unsafe {
                libc::socketpair(
                    libc::AF_UNIX,
                    libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
                    0,
                    descriptors.as_mut_ptr(),
                )
            },
            0,
        );
        // SAFETY: successful socketpair returned two distinct uniquely owned descriptors.
        unsafe {
            (
                OwnedFd::from_raw_fd(descriptors[0]),
                OwnedFd::from_raw_fd(descriptors[1]),
            )
        }
    }

    fn current_service_identity() -> M1AllKernelsProtectedVerifierServiceIdentityV1 {
        // Tests exercise the same-UID internal admission path only.
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        M1AllKernelsProtectedVerifierServiceIdentityV1 { uid, gid }
    }

    fn spawn_response(peer: OwnedFd, bytes: Vec<u8>) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let mut request = [0_u8; 2_304];
            // SAFETY: request is writable and the peer descriptor is live for this thread.
            let received = unsafe {
                libc::recv(
                    peer.as_raw_fd(),
                    request.as_mut_ptr().cast(),
                    request.len(),
                    0,
                )
            };
            assert_eq!(usize::try_from(received).unwrap(), request.len());
            // SAFETY: bytes is readable for the complete requested length.
            let sent = unsafe {
                libc::send(
                    peer.as_raw_fd(),
                    bytes.as_ptr().cast(),
                    bytes.len(),
                    libc::MSG_NOSIGNAL,
                )
            };
            assert_eq!(usize::try_from(sent).unwrap(), bytes.len());
        })
    }

    fn spawn_response_with_ancillary_fd(
        peer: OwnedFd,
        mut bytes: Vec<u8>,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let mut request = [0_u8; 2_304];
            // SAFETY: request is writable and the peer descriptor is live for this thread.
            let received = unsafe {
                libc::recv(
                    peer.as_raw_fd(),
                    request.as_mut_ptr().cast(),
                    request.len(),
                    0,
                )
            };
            assert_eq!(usize::try_from(received).unwrap(), request.len());

            let carried = File::open("/dev/null").unwrap();
            let descriptor_bytes = u32::try_from(mem::size_of::<i32>()).unwrap();
            let control_len =
                usize::try_from(unsafe { libc::CMSG_SPACE(descriptor_bytes) }).unwrap();
            let mut control = vec![0_u64; control_len.div_ceil(mem::size_of::<u64>())];
            let mut vector = libc::iovec {
                iov_base: bytes.as_mut_ptr().cast(),
                iov_len: bytes.len(),
            };
            // SAFETY: zero is a valid empty msghdr; live payload and control storage are installed.
            let mut header = unsafe { mem::zeroed::<libc::msghdr>() };
            header.msg_iov = &raw mut vector;
            header.msg_iovlen = 1;
            header.msg_control = control.as_mut_ptr().cast();
            header.msg_controllen = control_len;
            // SAFETY: the control buffer is suitably aligned and has CMSG_SPACE bytes.
            let message = unsafe { libc::CMSG_FIRSTHDR(&raw const header) };
            assert!(!message.is_null());
            // SAFETY: message points inside the live control buffer with room for one descriptor.
            unsafe {
                (*message).cmsg_level = libc::SOL_SOCKET;
                (*message).cmsg_type = libc::SCM_RIGHTS;
                (*message).cmsg_len = usize::try_from(libc::CMSG_LEN(descriptor_bytes)).unwrap();
                let carried_fd = carried.as_raw_fd();
                ptr::copy_nonoverlapping(
                    (&raw const carried_fd).cast::<u8>(),
                    libc::CMSG_DATA(message),
                    mem::size_of::<i32>(),
                );
            }
            // SAFETY: header names live payload and a complete SCM_RIGHTS control message.
            let sent =
                unsafe { libc::sendmsg(peer.as_raw_fd(), &raw const header, libc::MSG_NOSIGNAL) };
            assert_eq!(usize::try_from(sent).unwrap(), bytes.len());
        })
    }

    #[test]
    fn one_shot_client_returns_only_caller_authenticated_receipt() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        let response =
            M1AllKernelsProtectedVerifierServiceResponseV1::new(&request, receipt).unwrap();
        let (client_peer, service_peer) = socketpair();
        let server = spawn_response(service_peer, response.canonical_bytes().to_vec());
        let client = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(!client.grants_authority());
        let authenticated = client.request_receipt(&policy, &request).unwrap();
        assert_eq!(
            authenticated.receipt().identity(),
            response.receipt().identity()
        );
        assert!(!authenticated.grants_verifier_authority());
        server.join().unwrap();
    }

    #[test]
    fn admission_failure_returns_the_exact_owned_peer() {
        let (stream, _other) = UnixStream::pair().unwrap();
        let raw = stream.into_raw_fd();
        // SAFETY: into_raw_fd transferred unique ownership.
        let peer = unsafe { OwnedFd::from_raw_fd(raw) };
        let original = peer.as_raw_fd();
        let failure = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            peer,
            current_service_identity(),
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            M1AllKernelsProtectedVerifierClientErrorV1::NotSeqpacket
        ));
        let peer = failure.into_peer();
        assert_eq!(peer.as_raw_fd(), original);
        assert!(unsafe { libc::fcntl(peer.as_raw_fd(), libc::F_GETFD) } >= 0);
    }

    #[test]
    fn admission_rejects_zero_timeout_and_wrong_credentials() {
        let (peer, _other) = socketpair();
        let failure = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            peer,
            current_service_identity(),
            Duration::ZERO,
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            M1AllKernelsProtectedVerifierClientErrorV1::InvalidTimeout
        ));

        let (peer, _other) = socketpair();
        let current = current_service_identity();
        let wrong = M1AllKernelsProtectedVerifierServiceIdentityV1::new(
            current.uid().saturating_add(1).max(1),
            current.gid(),
        )
        .unwrap();
        let failure = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            peer,
            wrong,
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            M1AllKernelsProtectedVerifierClientErrorV1::PeerCredentialsMismatch
        ));
    }

    #[test]
    fn public_admission_rejects_same_uid() {
        let (peer, _other) = socketpair();
        let original = peer.as_raw_fd();
        let failure = M1AllKernelsProtectedVerifierClientV1::admit(
            peer,
            current_service_identity(),
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            M1AllKernelsProtectedVerifierClientErrorV1::ClientAndServiceUidMatch
        ));
        assert_eq!(failure.into_peer().as_raw_fd(), original);
    }

    #[test]
    fn truncated_and_oversized_packets_fail_closed() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        let response =
            M1AllKernelsProtectedVerifierServiceResponseV1::new(&request, receipt).unwrap();
        for bytes in [
            response.canonical_bytes()[..response.canonical_bytes().len() - 1].to_vec(),
            {
                let mut oversized = response.canonical_bytes().to_vec();
                oversized.push(0);
                oversized
            },
        ] {
            let oversized = bytes.len() > response.canonical_bytes().len();
            let (client_peer, service_peer) = socketpair();
            let server = spawn_response(service_peer, bytes);
            let client = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
                client_peer,
                current_service_identity(),
                Duration::from_secs(2),
            )
            .unwrap();
            let error = client.request_receipt(&policy, &request).unwrap_err();
            if oversized {
                assert!(matches!(
                    error,
                    M1AllKernelsProtectedVerifierClientErrorV1::PacketTruncated
                ));
            } else {
                assert!(matches!(
                    error,
                    M1AllKernelsProtectedVerifierClientErrorV1::ResponseLength { .. }
                ));
            }
            server.join().unwrap();
        }
    }

    #[test]
    fn response_with_ancillary_descriptor_fails_closed() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        let response =
            M1AllKernelsProtectedVerifierServiceResponseV1::new(&request, receipt).unwrap();
        let (client_peer, service_peer) = socketpair();
        let server =
            spawn_response_with_ancillary_fd(service_peer, response.canonical_bytes().to_vec());
        let client = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(matches!(
            client.request_receipt(&policy, &request),
            Err(M1AllKernelsProtectedVerifierClientErrorV1::AncillaryData)
        ));
        server.join().unwrap();
    }

    #[test]
    fn caller_policy_substitution_fails_before_transport() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        let (other_policy, _) =
            crate::protected_verifier_test_support::signed_fixture_with_seed(0x92);
        let (client_peer, _service_peer) = socketpair();
        let client = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_secs(1),
        )
        .unwrap();
        assert!(matches!(
            client.request_receipt(&other_policy, &request),
            Err(M1AllKernelsProtectedVerifierClientErrorV1::TrustPolicyMismatch)
        ));
    }

    #[test]
    fn response_for_another_request_fails_correlation() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        let mut entries = *request.entries();
        entries[0] =
            crate::protected_verifier_service::M1AllKernelsProtectedVerifierServiceEntryV1::new(
                0,
                [0xee; 32],
                entries[0].marker_binding_identity(),
                entries[0].generated_host_contract_identity(),
            )
            .unwrap();
        let other_request = M1AllKernelsProtectedVerifierServiceRequestV1::new(
            policy.identity(),
            *request.request_claims(),
            *request.compiler_claims(),
            entries,
        )
        .unwrap();
        let response =
            M1AllKernelsProtectedVerifierServiceResponseV1::new(&request, receipt).unwrap();
        let (client_peer, service_peer) = socketpair();
        let server = spawn_response(service_peer, response.canonical_bytes().to_vec());
        let client = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(matches!(
            client.request_receipt(&policy, &other_request),
            Err(M1AllKernelsProtectedVerifierClientErrorV1::ResponseRequestMismatch)
        ));
        server.join().unwrap();
    }

    #[test]
    fn structurally_correlated_response_with_bad_signature_fails_authentication() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        let mut hostile_bytes = *receipt.encode_canonical();
        let signature_offset = hostile_bytes.len() - 1;
        hostile_bytes[signature_offset] ^= 1;
        let hostile_receipt =
            crate::protected_receipt::M1AllKernelsProtectedVerifierReceiptV1::decode_canonical(
                &hostile_bytes,
            )
            .unwrap();
        let response =
            M1AllKernelsProtectedVerifierServiceResponseV1::new(&request, hostile_receipt).unwrap();
        let (client_peer, service_peer) = socketpair();
        let server = spawn_response(service_peer, response.canonical_bytes().to_vec());
        let client = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(matches!(
            client.request_receipt(&policy, &request),
            Err(
                M1AllKernelsProtectedVerifierClientErrorV1::ReceiptAuthentication(
                    M1AllKernelsProtectedReceiptErrorV1::SignatureRejected
                )
            )
        ));
        server.join().unwrap();
    }

    #[test]
    fn silent_peer_hits_the_single_absolute_deadline() {
        let (policy, receipt) = signed_fixture();
        let request = fixture_request(&policy, &receipt);
        let (client_peer, _service_peer) = socketpair();
        let client = M1AllKernelsProtectedVerifierClientV1::admit_inner::<false>(
            client_peer,
            current_service_identity(),
            Duration::from_millis(20),
        )
        .unwrap();
        assert!(matches!(
            client.request_receipt(&policy, &request),
            Err(M1AllKernelsProtectedVerifierClientErrorV1::Timeout)
        ));
    }
}
