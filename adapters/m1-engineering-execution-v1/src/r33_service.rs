//! Supervised Unix service foundation for the authority-free Ferric R33 adapter.
//!
//! The short-lived collector frontend never spawns this service. An external
//! supervisor prelaunches it under an independent process hierarchy. The
//! service plan, workload, peer credentials, socket identity, slot, and backend
//! instance are checked on every exchange. This module deliberately stops
//! before authenticated artifact selection or physical serving is joined.

use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use rustix::fs::{CWD, FileType, Mode, OFlags, ResolveFlags, Stat, fstat, openat2};
use rustix::net::sockopt::socket_peercred;
use rustix::process::geteuid;
use rustix::rand::{GetRandomFlags, getrandom};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::r33_wire::{
    M1_R33_MAX_REQUESTS_PER_WINDOW_V1, M1_R33_SERVICE_PLAN_ENV_V1,
    M1_R33_SERVICE_PLAN_SHA256_ENV_V1, M1_R33_TARGET_V1, M1_R33_WINDOWS_PER_START_V1,
    M1R33ActionV1, M1R33CollectorContextV1, M1R33CollectorRowV1, M1R33FrameKindV1,
    M1R33MeasurementReportV1, M1R33SlotV1, M1R33WireAckV1, M1R33WireErrorV1, M1R33WireReportV1,
    M1R33WireRequestV1, M1R33WireResponseV1, M1R33WireStatusV1, M1R33WorkloadRequestV1,
    decode_canonical_json_v1, encode_canonical_json_v1, read_frame_open_v1, read_frame_v1,
    require_sha256, sha256_hex, write_frame_v1,
};

/// Exact held service-plan schema.
pub const M1_R33_SERVICE_PLAN_FORMAT_V1: &str = "FERRIC-M1-R33-SERVICE-PLAN-V1";
/// Exact held pretokenized-workload schema.
pub const M1_R33_WORKLOAD_FORMAT_V1: &str = "FERRIC-M1-R33-PRETOKENIZED-WORKLOAD-V1";
/// Explicit non-authority label for service inputs.
pub const M1_R33_SERVICE_AUTHORITY_V1: &str = "none";

const MAX_SERVICE_PLAN_BYTES_V1: usize = 1024 * 1024;
const MAX_WORKLOAD_BYTES_V1: usize = 64 * 1024 * 1024;
const MIN_IO_TIMEOUT_MS_V1: u64 = 10;
const MAX_IO_TIMEOUT_MS_V1: u64 = 10 * 60 * 1000;
const MAX_SOCKET_PATH_BYTES_V1: usize = 100;

/// Exact action-command identities frozen by the collector plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct M1R33CommandIdentitiesV1 {
    pub measure: String,
    pub ready: String,
    pub start: String,
    pub stop: String,
}

impl M1R33CommandIdentitiesV1 {
    fn validate(&self) -> Result<(), M1R33ServiceErrorV1> {
        for identity in [&self.measure, &self.ready, &self.start, &self.stop] {
            require_sha256(identity, "service command")?;
        }
        Ok(())
    }

    fn for_action(&self, action: M1R33ActionV1) -> &str {
        match action {
            M1R33ActionV1::Start => &self.start,
            M1R33ActionV1::Ready => &self.ready,
            M1R33ActionV1::Measure => &self.measure,
            M1R33ActionV1::Stop => &self.stop,
        }
    }
}

/// Canonical service plan held by both frontend and supervised daemon.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct M1R33ServicePlanDocumentV1 {
    pub authority: String,
    pub commands: M1R33CommandIdentitiesV1,
    pub expected_client_uid: u32,
    pub expected_daemon_uid: u32,
    pub format: String,
    pub implementation: Value,
    pub io_timeout_ms: u64,
    pub policy_sha256: String,
    pub service_id: String,
    pub slot: M1R33SlotV1,
    pub slot_gpu_ids: Vec<u32>,
    pub socket_path: String,
    pub target: String,
    pub workload_path: String,
    pub workload_sha256: String,
}

impl M1R33ServicePlanDocumentV1 {
    fn validate(&self) -> Result<(), M1R33ServiceErrorV1> {
        if self.authority != M1_R33_SERVICE_AUTHORITY_V1
            || self.format != M1_R33_SERVICE_PLAN_FORMAT_V1
            || self.target != M1_R33_TARGET_V1
            || self.io_timeout_ms < MIN_IO_TIMEOUT_MS_V1
            || self.io_timeout_ms > MAX_IO_TIMEOUT_MS_V1
        {
            return Err(M1R33ServiceErrorV1::Plan("fixed fields"));
        }
        self.commands.validate()?;
        self.slot.validate()?;
        if self.slot.target != self.target {
            return Err(M1R33ServiceErrorV1::Plan("slot target"));
        }
        for (identity, name) in [
            (&self.policy_sha256, "service policy"),
            (&self.service_id, "service identity"),
            (&self.workload_sha256, "service workload"),
        ] {
            require_sha256(identity, name)?;
        }
        if self.slot_gpu_ids.is_empty()
            || self.slot_gpu_ids.len() > 8
            || self.slot_gpu_ids.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(M1R33ServiceErrorV1::Plan("slot GPU roster"));
        }
        if !encode_canonical_json_v1(&self.implementation)?.is_ascii() {
            return Err(M1R33ServiceErrorV1::Plan("implementation ASCII"));
        }
        validate_socket_path(Path::new(&self.socket_path))?;
        validate_canonical_input_path(Path::new(&self.workload_path), "workload path")?;
        Ok(())
    }

    fn timeout(&self) -> Duration {
        Duration::from_millis(self.io_timeout_ms)
    }
}

/// One held workload row containing exact tokenizer output.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct M1R33WorkloadWindowV1 {
    pub requests: Vec<M1R33WorkloadRequestV1>,
    pub row: M1R33CollectorRowV1,
}

impl M1R33WorkloadWindowV1 {
    fn validate(&self) -> Result<(), M1R33ServiceErrorV1> {
        self.row.validate()?;
        if self.requests.is_empty()
            || self.requests.len() > M1_R33_MAX_REQUESTS_PER_WINDOW_V1
            || self.requests.len() as u64 != self.row.expected_work.successful_requests
        {
            return Err(M1R33ServiceErrorV1::Workload("request roster"));
        }
        let mut input_tokens = 0_u64;
        let mut output_tokens = 0_u64;
        for (ordinal, request) in self.requests.iter().enumerate() {
            request.validate(ordinal)?;
            input_tokens = input_tokens
                .checked_add(request.prompt_tokens.len() as u64)
                .ok_or(M1R33ServiceErrorV1::Workload("input work overflow"))?;
            output_tokens = output_tokens
                .checked_add(request.expected_output_tokens)
                .ok_or(M1R33ServiceErrorV1::Workload("output work overflow"))?;
        }
        if input_tokens != self.row.expected_work.input_tokens
            || output_tokens != self.row.expected_work.output_tokens
        {
            return Err(M1R33ServiceErrorV1::Workload("aggregate work"));
        }
        Ok(())
    }
}

/// Canonical complete workload held for all three R33 starts.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct M1R33WorkloadDocumentV1 {
    pub authority: String,
    pub format: String,
    pub policy_sha256: String,
    pub rows: Vec<M1R33WorkloadWindowV1>,
    pub service_id: String,
    pub target: String,
}

impl M1R33WorkloadDocumentV1 {
    fn validate(&self) -> Result<(), M1R33ServiceErrorV1> {
        if self.authority != M1_R33_SERVICE_AUTHORITY_V1
            || self.format != M1_R33_WORKLOAD_FORMAT_V1
            || self.target != M1_R33_TARGET_V1
            || self.rows.len() != 3 * M1_R33_WINDOWS_PER_START_V1
        {
            return Err(M1R33ServiceErrorV1::Workload("fixed fields"));
        }
        require_sha256(&self.policy_sha256, "workload policy")?;
        require_sha256(&self.service_id, "workload service")?;
        for (ordinal, row) in self.rows.iter().enumerate() {
            row.validate()?;
            if row.row.ordinal != ordinal as u64 {
                return Err(M1R33ServiceErrorV1::Workload("row order"));
            }
        }
        Ok(())
    }

    fn row(&self, server_start: u64, within_start: usize) -> Option<&M1R33WorkloadWindowV1> {
        usize::try_from(server_start)
            .ok()?
            .checked_mul(M1_R33_WINDOWS_PER_START_V1)?
            .checked_add(within_start)
            .and_then(|ordinal| self.rows.get(ordinal))
    }
}

#[derive(Debug)]
struct HeldCanonicalFileV1<T> {
    bytes: Vec<u8>,
    file: File,
    initial: Stat,
    path: PathBuf,
    value: T,
}

impl<T: for<'de> Deserialize<'de>> HeldCanonicalFileV1<T> {
    fn open(
        path: &Path,
        maximum: usize,
        description: &'static str,
    ) -> Result<Self, M1R33ServiceErrorV1> {
        validate_canonical_input_path(path, description)?;
        let descriptor = openat2(
            CWD,
            path,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|source| M1R33ServiceErrorV1::Io(io::Error::from(source)))?;
        let initial = fstat(&descriptor)
            .map_err(|source| M1R33ServiceErrorV1::Io(io::Error::from(source)))?;
        if FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile {
            return Err(M1R33ServiceErrorV1::HeldFile(description));
        }
        let length = usize::try_from(initial.st_size)
            .ok()
            .filter(|length| *length != 0 && *length <= maximum)
            .ok_or(M1R33ServiceErrorV1::HeldFile(description))?;
        let mut file = File::from(descriptor);
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length.saturating_add(1))
            .map_err(|_| M1R33ServiceErrorV1::HeldFile(description))?;
        Read::by_ref(&mut file)
            .take(maximum.saturating_add(1) as u64)
            .read_to_end(&mut bytes)?;
        let after =
            fstat(&file).map_err(|source| M1R33ServiceErrorV1::Io(io::Error::from(source)))?;
        if bytes.len() != length || !same_snapshot(&initial, &after) {
            return Err(M1R33ServiceErrorV1::HeldFile(description));
        }
        let value = decode_canonical_json_v1(&bytes)?;
        Ok(Self {
            bytes,
            file,
            initial,
            path: path.to_path_buf(),
            value,
        })
    }

    fn revalidate(&self, description: &'static str) -> Result<(), M1R33ServiceErrorV1> {
        let held =
            fstat(&self.file).map_err(|source| M1R33ServiceErrorV1::Io(io::Error::from(source)))?;
        let reopened = openat2(
            CWD,
            &self.path,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|source| M1R33ServiceErrorV1::Io(io::Error::from(source)))?;
        let path =
            fstat(&reopened).map_err(|source| M1R33ServiceErrorV1::Io(io::Error::from(source)))?;
        if !same_snapshot(&self.initial, &held) || !same_snapshot(&self.initial, &path) {
            return Err(M1R33ServiceErrorV1::HeldFile(description));
        }
        Ok(())
    }
}

/// Move-only held plan and workload used by one daemon or frontend action.
#[must_use = "service inputs must remain held for the complete exchange"]
#[derive(Debug)]
pub struct HeldM1R33ServiceBundleV1 {
    plan: HeldCanonicalFileV1<M1R33ServicePlanDocumentV1>,
    plan_sha256: String,
    workload: HeldCanonicalFileV1<M1R33WorkloadDocumentV1>,
}

impl HeldM1R33ServiceBundleV1 {
    /// Opens, canonically decodes, cross-binds, and retains both documents.
    ///
    /// # Errors
    ///
    /// Rejects symlinked/noncanonical/replaced files, schema drift, digest drift,
    /// or a service/policy/target mismatch.
    pub fn open(plan_path: impl AsRef<Path>) -> Result<Self, M1R33ServiceErrorV1> {
        let plan: HeldCanonicalFileV1<M1R33ServicePlanDocumentV1> = HeldCanonicalFileV1::open(
            plan_path.as_ref(),
            MAX_SERVICE_PLAN_BYTES_V1,
            "service plan",
        )?;
        plan.value.validate()?;
        let workload: HeldCanonicalFileV1<M1R33WorkloadDocumentV1> = HeldCanonicalFileV1::open(
            Path::new(&plan.value.workload_path),
            MAX_WORKLOAD_BYTES_V1,
            "pretokenized workload",
        )?;
        workload.value.validate()?;
        if sha256_hex(&workload.bytes) != plan.value.workload_sha256
            || workload.value.policy_sha256 != plan.value.policy_sha256
            || workload.value.service_id != plan.value.service_id
            || workload.value.target != plan.value.target
        {
            return Err(M1R33ServiceErrorV1::Plan("workload binding"));
        }
        let plan_sha256 = sha256_hex(&plan.bytes);
        Ok(Self {
            plan,
            plan_sha256,
            workload,
        })
    }

    /// Revalidates both retained descriptors and their path bindings.
    ///
    /// # Errors
    ///
    /// Rejects any replacement or in-place mutation since admission.
    pub fn revalidate(&self) -> Result<(), M1R33ServiceErrorV1> {
        self.plan.revalidate("service plan")?;
        self.workload.revalidate("pretokenized workload")
    }

    /// Exact decoded service plan.
    #[must_use]
    pub const fn plan(&self) -> &M1R33ServicePlanDocumentV1 {
        &self.plan.value
    }

    /// SHA-256 of exact canonical held service-plan bytes.
    #[must_use]
    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    /// Exact decoded pretokenized workload.
    #[must_use]
    pub const fn workload(&self) -> &M1R33WorkloadDocumentV1 {
        &self.workload.value
    }

    fn validate_context(
        &self,
        context: &M1R33CollectorContextV1,
    ) -> Result<(), M1R33ServiceErrorV1> {
        context.validate()?;
        let plan = self.plan();
        if context.command_sha256 != plan.commands.for_action(context.action)
            || context.implementation != plan.implementation
            || context.policy_sha256 != plan.policy_sha256
            || context.slot != plan.slot
            || context.target != plan.target
        {
            return Err(M1R33ServiceErrorV1::Binding("collector context"));
        }
        Ok(())
    }
}

/// Linux peer credentials observed on one accepted service socket.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1R33PeerCredentialsV1 {
    pub gid: u32,
    pub pid: u32,
    pub uid: u32,
}

/// Handler notified whether the complete response was delivered.
pub trait M1R33WireHandlerV1 {
    /// Handles one already framed, peer-checked, service-bound request.
    fn handle_request(
        &mut self,
        peer: M1R33PeerCredentialsV1,
        request: &M1R33WireRequestV1,
    ) -> M1R33WireResponseV1;

    /// Commits the response-visible lifecycle transition.
    fn response_delivered(&mut self, request_sha256: &str);

    /// Faults a post-mutation transition whose response could not be delivered.
    fn response_abandoned(&mut self, request_sha256: &str);
}

struct BoundSocketPathGuardV1 {
    armed: bool,
    socket_device: u64,
    socket_inode: u64,
    socket_path: PathBuf,
}

impl BoundSocketPathGuardV1 {
    fn new(socket_path: PathBuf) -> Result<Self, M1R33ServiceErrorV1> {
        let identity = fs::symlink_metadata(&socket_path)?;
        Ok(Self {
            armed: true,
            socket_device: identity.dev(),
            socket_inode: identity.ino(),
            socket_path,
        })
    }

    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for BoundSocketPathGuardV1 {
    fn drop(&mut self) {
        if self.armed {
            remove_matching_socket_path_v1(
                &self.socket_path,
                self.socket_device,
                self.socket_inode,
            );
        }
    }
}

/// One bound Unix listener owned by the external supervisor's daemon process.
#[must_use = "the listener owns the exact socket path"]
pub struct M1R33UnixServerV1 {
    expected_client_uid: u32,
    listener: UnixListener,
    plan_sha256: String,
    service_id: String,
    socket_device: u64,
    socket_inode: u64,
    socket_path: PathBuf,
    timeout: Duration,
}

impl fmt::Debug for M1R33UnixServerV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1R33UnixServerV1")
            .field("service_id", &self.service_id)
            .field("socket_path", &self.socket_path)
            .finish_non_exhaustive()
    }
}

impl M1R33UnixServerV1 {
    /// Binds a new socket and rejects stale paths or the wrong daemon UID.
    ///
    /// # Errors
    ///
    /// Returns a binding or filesystem error; it never removes an existing path.
    pub fn bind(bundle: &HeldM1R33ServiceBundleV1) -> Result<Self, M1R33ServiceErrorV1> {
        bundle.revalidate()?;
        let plan = bundle.plan();
        if geteuid().as_raw() != plan.expected_daemon_uid {
            return Err(M1R33ServiceErrorV1::Peer("daemon uid"));
        }
        let path = PathBuf::from(&plan.socket_path);
        let listener = UnixListener::bind(&path)?;
        let path_guard = BoundSocketPathGuardV1::new(path.clone())?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.file_type().is_socket()
            || metadata.dev() != path_guard.socket_device
            || metadata.ino() != path_guard.socket_inode
        {
            return Err(M1R33ServiceErrorV1::Socket("bound path identity"));
        }
        listener.set_nonblocking(true)?;
        let socket_device = path_guard.socket_device;
        let socket_inode = path_guard.socket_inode;
        path_guard.disarm();
        Ok(Self {
            expected_client_uid: plan.expected_client_uid,
            listener,
            plan_sha256: bundle.plan_sha256().to_owned(),
            service_id: plan.service_id.clone(),
            socket_device,
            socket_inode,
            socket_path: path,
            timeout: plan.timeout(),
        })
    }

    /// Accepts and completes exactly one request/response exchange.
    ///
    /// # Errors
    ///
    /// Rejects accept/read/write timeout, peer mismatch, partial/trailing input,
    /// cross-service requests, and partial output. A partial output notifies the
    /// handler so a post-mutation lifecycle cannot continue.
    pub fn serve_one(
        &self,
        handler: &mut impl M1R33WireHandlerV1,
    ) -> Result<(), M1R33ServiceErrorV1> {
        let deadline = Instant::now()
            .checked_add(self.timeout)
            .ok_or(M1R33ServiceErrorV1::Timeout)?;
        let (mut stream, _) = loop {
            match self.listener.accept() {
                Ok(value) => break value,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err(M1R33ServiceErrorV1::Timeout);
                    }
                    thread::sleep(Duration::from_millis(1));
                }
                Err(error) => return Err(M1R33ServiceErrorV1::Io(error)),
            }
        };
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;
        let credentials = socket_peercred(&stream)
            .map_err(|source| M1R33ServiceErrorV1::Io(io::Error::from(source)))?;
        let peer = M1R33PeerCredentialsV1 {
            gid: credentials.gid.as_raw(),
            pid: credentials.pid.as_raw_nonzero().get().cast_unsigned(),
            uid: credentials.uid.as_raw(),
        };
        if peer.uid != self.expected_client_uid {
            return Err(M1R33ServiceErrorV1::Peer("client uid"));
        }
        let request: M1R33WireRequestV1 =
            read_frame_open_v1(&mut stream, M1R33FrameKindV1::Request)?;
        request.validate()?;
        if request.service_id != self.service_id || request.service_plan_sha256 != self.plan_sha256
        {
            return Err(M1R33ServiceErrorV1::Binding("wire service"));
        }
        let response = handler.handle_request(peer, &request);
        let delivery = (|| -> Result<(), M1R33ServiceErrorV1> {
            response.validate()?;
            if response.request_sha256 != request.request_sha256
                || response.service_id != request.service_id
                || response.service_plan_sha256 != request.service_plan_sha256
            {
                return Err(M1R33ServiceErrorV1::Binding("handler response"));
            }
            let response_sha256 = sha256_hex(&encode_canonical_json_v1(&response)?);
            write_frame_v1(&mut stream, M1R33FrameKindV1::Response, &response)?;
            stream.shutdown(Shutdown::Write)?;
            let acknowledgement: M1R33WireAckV1 =
                read_frame_v1(&mut stream, M1R33FrameKindV1::Ack)?;
            acknowledgement.validate()?;
            if acknowledgement.request_sha256 != request.request_sha256
                || acknowledgement.response_sha256 != response_sha256
                || acknowledgement.service_id != request.service_id
                || acknowledgement.service_plan_sha256 != request.service_plan_sha256
            {
                return Err(M1R33ServiceErrorV1::Binding("response acknowledgement"));
            }
            Ok(())
        })();
        match delivery {
            Ok(()) => {
                handler.response_delivered(&request.request_sha256);
                Ok(())
            }
            Err(error) => {
                handler.response_abandoned(&request.request_sha256);
                Err(error)
            }
        }
    }
}

impl Drop for M1R33UnixServerV1 {
    fn drop(&mut self) {
        remove_matching_socket_path_v1(&self.socket_path, self.socket_device, self.socket_inode);
    }
}

/// Performs one exact exchange with the prelaunched service.
///
/// # Errors
///
/// Rejects the wrong local UID, stale/non-socket path, wrong daemon peer UID,
/// timeout, disconnect, framing drift, and response binding drift.
pub fn exchange_with_supervised_service_v1(
    bundle: &HeldM1R33ServiceBundleV1,
    request: &M1R33WireRequestV1,
) -> Result<M1R33WireResponseV1, M1R33ServiceErrorV1> {
    bundle.revalidate()?;
    bundle.validate_context(&request.context)?;
    request.validate()?;
    let plan = bundle.plan();
    if request.service_id != plan.service_id
        || request.service_plan_sha256 != bundle.plan_sha256()
        || geteuid().as_raw() != plan.expected_client_uid
    {
        return Err(M1R33ServiceErrorV1::Binding("client service"));
    }
    let metadata = fs::symlink_metadata(&plan.socket_path)?;
    if !metadata.file_type().is_socket() {
        return Err(M1R33ServiceErrorV1::Socket("service path type"));
    }
    let mut stream = UnixStream::connect(&plan.socket_path)?;
    stream.set_read_timeout(Some(plan.timeout()))?;
    stream.set_write_timeout(Some(plan.timeout()))?;
    let peer = socket_peercred(&stream)
        .map_err(|source| M1R33ServiceErrorV1::Io(io::Error::from(source)))?;
    if peer.uid.as_raw() != plan.expected_daemon_uid {
        return Err(M1R33ServiceErrorV1::Peer("daemon peer uid"));
    }
    write_frame_v1(&mut stream, M1R33FrameKindV1::Request, request)?;
    let response: M1R33WireResponseV1 = read_frame_v1(&mut stream, M1R33FrameKindV1::Response)?;
    response.validate()?;
    if response.request_sha256 != request.request_sha256
        || response.service_id != request.service_id
        || response.service_plan_sha256 != request.service_plan_sha256
    {
        return Err(M1R33ServiceErrorV1::Binding("client response"));
    }
    bundle.revalidate()?;
    let _ = request.context.collector_result(&response)?;
    let acknowledgement = M1R33WireAckV1 {
        format: crate::r33_wire::M1_R33_WIRE_ACK_FORMAT_V1.to_owned(),
        request_sha256: request.request_sha256.clone(),
        response_sha256: sha256_hex(&encode_canonical_json_v1(&response)?),
        service_id: request.service_id.clone(),
        service_plan_sha256: request.service_plan_sha256.clone(),
    };
    write_frame_v1(&mut stream, M1R33FrameKindV1::Ack, &acknowledgement)?;
    stream.shutdown(Shutdown::Write)?;
    bundle.revalidate()?;
    Ok(response)
}

mod sealed {
    pub trait Sealed {}
}

/// Sealed authority-free boundary for a future physical serving join.
///
/// No value in this interface grants authenticated artifact, load,
/// queue-allocation, or launch custody.
pub trait M1R33AuthorityFreeBackendV1: sealed::Sealed {
    /// Creates one backend instance without changing its service identity.
    ///
    /// # Errors
    ///
    /// Returns a stable fault after any partial backend mutation.
    fn start(
        &mut self,
        instance_sha256: &str,
        server_start: u64,
        workload: &M1R33WorkloadDocumentV1,
        deadline: M1R33OperationDeadlineV1,
    ) -> Result<(), M1R33BackendFaultV1>;

    /// Checks that the exact instance can accept its first window.
    ///
    /// # Errors
    ///
    /// Returns a stable fault after any backend access.
    fn ready(
        &mut self,
        instance_sha256: &str,
        deadline: M1R33OperationDeadlineV1,
    ) -> Result<(), M1R33BackendFaultV1>;

    /// Executes one exact preadmitted bounded window.
    ///
    /// # Errors
    ///
    /// Returns a stable fault and no measurement after partial execution.
    fn measure(
        &mut self,
        instance_sha256: &str,
        window: &M1R33WorkloadWindowV1,
        deadline: M1R33OperationDeadlineV1,
    ) -> Result<M1R33MeasurementReportV1, M1R33BackendFaultV1>;

    /// Destroys the exact instance, including a faulted instance.
    ///
    /// # Errors
    ///
    /// Returns a stable fault when destruction cannot be confirmed.
    fn stop(
        &mut self,
        instance_sha256: &str,
        deadline: M1R33OperationDeadlineV1,
    ) -> Result<(), M1R33BackendFaultV1>;
}

/// Cooperative absolute deadline supplied to every backend operation.
#[derive(Clone, Copy, Debug)]
pub struct M1R33OperationDeadlineV1(Instant);

impl M1R33OperationDeadlineV1 {
    /// Whether the operation deadline has elapsed.
    #[must_use]
    pub fn expired(self) -> bool {
        Instant::now() >= self.0
    }
}

/// Stable authority-free backend fault code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1R33BackendFaultV1 {
    code: &'static str,
}

impl M1R33BackendFaultV1 {
    pub(crate) const fn new(code: &'static str) -> Self {
        Self { code }
    }
}

#[derive(Clone, Debug)]
enum StableLifecycleV1 {
    Idle,
    AwaitReady {
        instance: String,
        start: u64,
    },
    BetweenWindows {
        instance: String,
        next: usize,
        start: u64,
    },
    AwaitStop {
        instance: String,
        start: u64,
    },
    Faulted {
        instance: String,
        start: u64,
    },
    StopReplay {
        context: Box<M1R33CollectorContextV1>,
        instance: String,
    },
}

#[derive(Clone, Debug)]
struct PendingLifecycleV1 {
    abandoned: StableLifecycleV1,
    delivered: StableLifecycleV1,
    request_sha256: String,
}

#[derive(Clone, Debug)]
enum LifecycleV1 {
    Stable(StableLifecycleV1),
    Pending(Box<PendingLifecycleV1>),
}

/// Top-level 20-window service controller over one long-lived backend instance.
///
/// Its successful path enforces `start -> ready -> 20 ordered bounded measures
/// -> stop`; the exact instance-bound `stop` is also admitted for cleanup after
/// a collector abort. Any backend failure or undelivered post-mutation response
/// enters a fault state in which only that stop is admitted. It does not claim
/// that the current Ferric physical queue can yet recycle all 20 windows.
pub struct M1R33DaemonCoordinatorV1<'a, B: M1R33AuthorityFreeBackendV1> {
    backend: B,
    bundle: &'a HeldM1R33ServiceBundleV1,
    state: LifecycleV1,
}

impl<B: M1R33AuthorityFreeBackendV1> fmt::Debug for M1R33DaemonCoordinatorV1<'_, B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1R33DaemonCoordinatorV1")
            .field("service_id", &self.bundle.plan().service_id)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl<'a, B: M1R33AuthorityFreeBackendV1> M1R33DaemonCoordinatorV1<'a, B> {
    /// Creates an idle coordinator retaining the held service inputs.
    #[must_use]
    pub fn new(bundle: &'a HeldM1R33ServiceBundleV1, backend: B) -> Self {
        Self {
            backend,
            bundle,
            state: LifecycleV1::Stable(StableLifecycleV1::Idle),
        }
    }

    fn deadline(&self) -> Result<M1R33OperationDeadlineV1, M1R33ServiceErrorV1> {
        Instant::now()
            .checked_add(self.bundle.plan().timeout())
            .map(M1R33OperationDeadlineV1)
            .ok_or(M1R33ServiceErrorV1::Timeout)
    }

    fn base_response(
        request: &M1R33WireRequestV1,
        status: M1R33WireStatusV1,
        instance: Option<String>,
        report: Option<M1R33WireReportV1>,
        error_code: Option<String>,
    ) -> M1R33WireResponseV1 {
        M1R33WireResponseV1 {
            action: request.context.action,
            error_code,
            format: crate::r33_wire::M1_R33_WIRE_RESPONSE_FORMAT_V1.to_owned(),
            policy_sha256: request.context.policy_sha256.clone(),
            reported: report,
            request_sha256: request.request_sha256.clone(),
            server_instance_sha256: instance,
            server_start: request.context.server_start,
            service_id: request.service_id.clone(),
            service_plan_sha256: request.service_plan_sha256.clone(),
            slot_id: request.context.slot.id.clone(),
            status,
        }
    }

    fn fault_response(
        request: &M1R33WireRequestV1,
        instance: Option<String>,
        code: &'static str,
    ) -> M1R33WireResponseV1 {
        Self::base_response(
            request,
            M1R33WireStatusV1::Fault,
            instance,
            None,
            Some(code.to_owned()),
        )
    }

    fn passed_response(
        request: &M1R33WireRequestV1,
        instance: String,
        report: M1R33WireReportV1,
    ) -> M1R33WireResponseV1 {
        Self::base_response(
            request,
            M1R33WireStatusV1::Passed,
            Some(instance),
            Some(report),
            None,
        )
    }

    fn set_pending(
        &mut self,
        request: &M1R33WireRequestV1,
        delivered: StableLifecycleV1,
        abandoned: StableLifecycleV1,
    ) {
        self.state = LifecycleV1::Pending(Box::new(PendingLifecycleV1 {
            abandoned,
            delivered,
            request_sha256: request.request_sha256.clone(),
        }));
    }

    fn backend_failure(
        &mut self,
        request: &M1R33WireRequestV1,
        instance: String,
        fault: M1R33BackendFaultV1,
    ) -> M1R33WireResponseV1 {
        self.state = LifecycleV1::Stable(StableLifecycleV1::Faulted {
            instance: instance.clone(),
            start: request.context.server_start,
        });
        Self::fault_response(request, Some(instance), fault.code)
    }

    fn dispatch(&mut self, request: &M1R33WireRequestV1) -> M1R33WireResponseV1 {
        if self.bundle.revalidate().is_err()
            || self.bundle.validate_context(&request.context).is_err()
            || request.validate().is_err()
            || request.service_id != self.bundle.plan().service_id
            || request.service_plan_sha256 != self.bundle.plan_sha256()
        {
            return Self::fault_response(request, None, "binding-rejected");
        }
        let stable = match &self.state {
            LifecycleV1::Stable(state) => state.clone(),
            LifecycleV1::Pending(_) => {
                return Self::fault_response(request, None, "response-pending");
            }
        };
        match stable {
            StableLifecycleV1::Idle => self.dispatch_start(request),
            StableLifecycleV1::AwaitReady { instance, start } => {
                if exact_instance_action(request, M1R33ActionV1::Stop, &instance, start) {
                    self.dispatch_stop(request, instance, start)
                } else {
                    self.dispatch_ready(request, instance, start)
                }
            }
            StableLifecycleV1::BetweenWindows {
                instance,
                next,
                start,
            } => {
                if exact_instance_action(request, M1R33ActionV1::Stop, &instance, start) {
                    self.dispatch_stop(request, instance, start)
                } else {
                    self.dispatch_measure(request, instance, next, start)
                }
            }
            StableLifecycleV1::AwaitStop { instance, start }
            | StableLifecycleV1::Faulted { instance, start } => {
                self.dispatch_stop(request, instance, start)
            }
            StableLifecycleV1::StopReplay { context, instance } => {
                if request.context == *context {
                    let response = Self::passed_response(
                        request,
                        instance.clone(),
                        M1R33WireReportV1::Lifecycle,
                    );
                    self.set_pending(
                        request,
                        StableLifecycleV1::Idle,
                        StableLifecycleV1::StopReplay { context, instance },
                    );
                    response
                } else {
                    Self::fault_response(request, Some(instance), "only-exact-stop-admitted")
                }
            }
        }
    }

    fn dispatch_start(&mut self, request: &M1R33WireRequestV1) -> M1R33WireResponseV1 {
        if request.context.action != M1R33ActionV1::Start {
            return Self::fault_response(request, None, "start-required");
        }
        let Ok(instance) = issue_instance_identity(self.bundle, request.context.server_start)
        else {
            return Self::fault_response(request, None, "instance-identity-failed");
        };
        let Ok(deadline) = self.deadline() else {
            return Self::fault_response(request, Some(instance), "deadline-failed");
        };
        match self.backend.start(
            &instance,
            request.context.server_start,
            self.bundle.workload(),
            deadline,
        ) {
            Ok(()) if !deadline.expired() => {
                let response =
                    Self::passed_response(request, instance.clone(), M1R33WireReportV1::Lifecycle);
                let faulted = StableLifecycleV1::Faulted {
                    instance: instance.clone(),
                    start: request.context.server_start,
                };
                self.set_pending(
                    request,
                    StableLifecycleV1::AwaitReady {
                        instance,
                        start: request.context.server_start,
                    },
                    faulted,
                );
                response
            }
            Ok(()) => self.backend_failure(
                request,
                instance,
                M1R33BackendFaultV1::new("backend-timeout"),
            ),
            Err(fault) => self.backend_failure(request, instance, fault),
        }
    }

    fn dispatch_ready(
        &mut self,
        request: &M1R33WireRequestV1,
        instance: String,
        start: u64,
    ) -> M1R33WireResponseV1 {
        if !exact_instance_action(request, M1R33ActionV1::Ready, &instance, start) {
            return Self::fault_response(request, Some(instance), "only-exact-ready-admitted");
        }
        let Ok(deadline) = self.deadline() else {
            return self.backend_failure(
                request,
                instance,
                M1R33BackendFaultV1::new("deadline-failed"),
            );
        };
        match self.backend.ready(&instance, deadline) {
            Ok(()) if !deadline.expired() => {
                let response =
                    Self::passed_response(request, instance.clone(), M1R33WireReportV1::Lifecycle);
                let faulted = StableLifecycleV1::Faulted {
                    instance: instance.clone(),
                    start,
                };
                self.set_pending(
                    request,
                    StableLifecycleV1::BetweenWindows {
                        instance,
                        next: 0,
                        start,
                    },
                    faulted,
                );
                response
            }
            Ok(()) => self.backend_failure(
                request,
                instance,
                M1R33BackendFaultV1::new("backend-timeout"),
            ),
            Err(fault) => self.backend_failure(request, instance, fault),
        }
    }

    fn dispatch_measure(
        &mut self,
        request: &M1R33WireRequestV1,
        instance: String,
        next: usize,
        start: u64,
    ) -> M1R33WireResponseV1 {
        let Some(window) = self.bundle.workload().row(start, next) else {
            return self.backend_failure(
                request,
                instance,
                M1R33BackendFaultV1::new("workload-row-absent"),
            );
        };
        if !exact_instance_action(request, M1R33ActionV1::Measure, &instance, start)
            || request.context.row.as_ref() != Some(&window.row)
        {
            return Self::fault_response(request, Some(instance), "only-next-window-admitted");
        }
        let Ok(deadline) = self.deadline() else {
            return self.backend_failure(
                request,
                instance,
                M1R33BackendFaultV1::new("deadline-failed"),
            );
        };
        match self.backend.measure(&instance, window, deadline) {
            Ok(report)
                if !deadline.expired()
                    && report.validate_against(&window.row.expected_work).is_ok() =>
            {
                let response = Self::passed_response(
                    request,
                    instance.clone(),
                    M1R33WireReportV1::Measurement(report),
                );
                let delivered = if next + 1 == M1_R33_WINDOWS_PER_START_V1 {
                    StableLifecycleV1::AwaitStop {
                        instance: instance.clone(),
                        start,
                    }
                } else {
                    StableLifecycleV1::BetweenWindows {
                        instance: instance.clone(),
                        next: next + 1,
                        start,
                    }
                };
                self.set_pending(
                    request,
                    delivered,
                    StableLifecycleV1::Faulted { instance, start },
                );
                response
            }
            Ok(_) => self.backend_failure(
                request,
                instance,
                M1R33BackendFaultV1::new("measurement-rejected"),
            ),
            Err(fault) => self.backend_failure(request, instance, fault),
        }
    }

    fn dispatch_stop(
        &mut self,
        request: &M1R33WireRequestV1,
        instance: String,
        start: u64,
    ) -> M1R33WireResponseV1 {
        if !exact_instance_action(request, M1R33ActionV1::Stop, &instance, start) {
            return Self::fault_response(request, Some(instance), "only-exact-stop-admitted");
        }
        let Ok(deadline) = self.deadline() else {
            return self.backend_failure(
                request,
                instance,
                M1R33BackendFaultV1::new("deadline-failed"),
            );
        };
        match self.backend.stop(&instance, deadline) {
            Ok(()) if !deadline.expired() => {
                let response =
                    Self::passed_response(request, instance.clone(), M1R33WireReportV1::Lifecycle);
                self.set_pending(
                    request,
                    StableLifecycleV1::Idle,
                    StableLifecycleV1::StopReplay {
                        context: Box::new(request.context.clone()),
                        instance,
                    },
                );
                response
            }
            Ok(()) => self.backend_failure(
                request,
                instance,
                M1R33BackendFaultV1::new("backend-timeout"),
            ),
            Err(fault) => self.backend_failure(request, instance, fault),
        }
    }
}

impl<B: M1R33AuthorityFreeBackendV1> M1R33WireHandlerV1 for M1R33DaemonCoordinatorV1<'_, B> {
    fn handle_request(
        &mut self,
        _peer: M1R33PeerCredentialsV1,
        request: &M1R33WireRequestV1,
    ) -> M1R33WireResponseV1 {
        self.dispatch(request)
    }

    fn response_delivered(&mut self, request_sha256: &str) {
        let LifecycleV1::Pending(pending) = &self.state else {
            return;
        };
        let next = if pending.request_sha256 == request_sha256 {
            pending.delivered.clone()
        } else {
            pending.abandoned.clone()
        };
        self.state = LifecycleV1::Stable(next);
    }

    fn response_abandoned(&mut self, _request_sha256: &str) {
        let LifecycleV1::Pending(pending) = &self.state else {
            return;
        };
        self.state = LifecycleV1::Stable(pending.abandoned.clone());
    }
}

fn exact_instance_action(
    request: &M1R33WireRequestV1,
    action: M1R33ActionV1,
    instance: &str,
    start: u64,
) -> bool {
    request.context.action == action
        && request.context.server_start == start
        && request.context.server_instance_sha256.as_deref() == Some(instance)
}

fn issue_instance_identity(
    bundle: &HeldM1R33ServiceBundleV1,
    server_start: u64,
) -> Result<String, M1R33ServiceErrorV1> {
    let mut nonce = [0_u8; 32];
    let _ = getrandom(&mut nonce, GetRandomFlags::empty())
        .map_err(|source| M1R33ServiceErrorV1::Io(io::Error::from(source)))?;
    let mut identity = Vec::with_capacity(32 + 64 + 8);
    identity.extend_from_slice(&nonce);
    identity.extend_from_slice(bundle.plan_sha256().as_bytes());
    identity.extend_from_slice(&server_start.to_be_bytes());
    Ok(sha256_hex(&identity))
}

/// Short-lived exact collector frontend entry point.
#[must_use]
pub fn adapter_main(arguments: &[OsString]) -> ExitCode {
    match run_adapter(arguments) {
        Ok(bytes) => {
            if let Err(error) = io::stdout().write_all(&bytes) {
                eprintln!("FAIL: cannot write canonical R33 result: {error}");
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("FAIL: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_adapter(arguments: &[OsString]) -> Result<Vec<u8>, M1R33ServiceErrorV1> {
    if !arguments.is_empty() {
        return Err(M1R33ServiceErrorV1::AdapterArguments);
    }
    let plan_path = std::env::var_os(M1_R33_SERVICE_PLAN_ENV_V1)
        .ok_or(M1R33ServiceErrorV1::MissingServicePlanEnvironment)?;
    let bundle = HeldM1R33ServiceBundleV1::open(PathBuf::from(plan_path))?;
    let expected_plan_sha256 = std::env::var(M1_R33_SERVICE_PLAN_SHA256_ENV_V1)
        .map_err(|_| M1R33ServiceErrorV1::MissingServicePlanEnvironment)?;
    require_sha256(&expected_plan_sha256, "frozen service plan")?;
    if expected_plan_sha256 != bundle.plan_sha256() {
        return Err(M1R33ServiceErrorV1::Binding("frozen service plan"));
    }
    let context = M1R33CollectorContextV1::from_current_environment()?;
    bundle.validate_context(&context)?;
    let request = M1R33WireRequestV1 {
        context,
        format: crate::r33_wire::M1_R33_WIRE_REQUEST_FORMAT_V1.to_owned(),
        request_sha256: issue_request_identity()?,
        service_id: bundle.plan().service_id.clone(),
        service_plan_sha256: bundle.plan_sha256().to_owned(),
    };
    let response = exchange_with_supervised_service_v1(&bundle, &request)?;
    let result = request.context.collector_result(&response)?;
    bundle.revalidate()?;
    encode_canonical_json_v1(&result).map_err(M1R33ServiceErrorV1::Wire)
}

fn issue_request_identity() -> Result<String, M1R33ServiceErrorV1> {
    let mut nonce = [0_u8; 32];
    let _ = getrandom(&mut nonce, GetRandomFlags::empty())
        .map_err(|source| M1R33ServiceErrorV1::Io(io::Error::from(source)))?;
    Ok(sha256_hex(&nonce))
}

/// Service ingestion, binding, transport, lifecycle, or backend error.
#[derive(Debug)]
pub enum M1R33ServiceErrorV1 {
    Plan(&'static str),
    Workload(&'static str),
    HeldFile(&'static str),
    Binding(&'static str),
    Peer(&'static str),
    Socket(&'static str),
    Timeout,
    AdapterArguments,
    MissingServicePlanEnvironment,
    Io(io::Error),
    Wire(M1R33WireErrorV1),
}

impl fmt::Display for M1R33ServiceErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Ferric M1 R33 service rejected: {self:?}")
    }
}

impl std::error::Error for M1R33ServiceErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(source) => Some(source),
            Self::Wire(source) => Some(source),
            _ => None,
        }
    }
}

impl From<io::Error> for M1R33ServiceErrorV1 {
    fn from(source: io::Error) -> Self {
        Self::Io(source)
    }
}

impl From<M1R33WireErrorV1> for M1R33ServiceErrorV1 {
    fn from(source: M1R33WireErrorV1) -> Self {
        Self::Wire(source)
    }
}

fn validate_canonical_input_path(
    path: &Path,
    description: &'static str,
) -> Result<(), M1R33ServiceErrorV1> {
    if !path.is_absolute() || !path.as_os_str().as_encoded_bytes().is_ascii() {
        return Err(M1R33ServiceErrorV1::HeldFile(description));
    }
    let canonical = path.canonicalize()?;
    if canonical != path {
        return Err(M1R33ServiceErrorV1::HeldFile(description));
    }
    Ok(())
}

fn validate_socket_path(path: &Path) -> Result<(), M1R33ServiceErrorV1> {
    if !path.is_absolute()
        || path.as_os_str().as_encoded_bytes().len() > MAX_SOCKET_PATH_BYTES_V1
        || !path.as_os_str().as_encoded_bytes().is_ascii()
    {
        return Err(M1R33ServiceErrorV1::Plan("socket path"));
    }
    let parent = path
        .parent()
        .ok_or(M1R33ServiceErrorV1::Plan("socket parent"))?;
    if parent.canonicalize()? != parent {
        return Err(M1R33ServiceErrorV1::Plan("socket parent"));
    }
    if let Ok(metadata) = fs::symlink_metadata(path)
        && !metadata.file_type().is_socket()
    {
        return Err(M1R33ServiceErrorV1::Plan("socket path type"));
    }
    Ok(())
}

fn remove_matching_socket_path_v1(path: &Path, socket_device: u64, socket_inode: u64) {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return;
    };
    if metadata.file_type().is_socket()
        && metadata.dev() == socket_device
        && metadata.ino() == socket_inode
    {
        let _ = fs::remove_file(path);
    }
}

fn same_snapshot(left: &Stat, right: &Stat) -> bool {
    left.st_dev == right.st_dev
        && left.st_ino == right.st_ino
        && left.st_mode == right.st_mode
        && left.st_nlink == right.st_nlink
        && left.st_size == right.st_size
        && left.st_mtime == right.st_mtime
        && left.st_mtime_nsec == right.st_mtime_nsec
        && left.st_ctime == right.st_ctime
        && left.st_ctime_nsec == right.st_ctime_nsec
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r33_wire::{
        M1_R33_CLOCK_V1, M1_R33_DURATION_BOUNDARY_V1, M1_R33_TIMING_BOUNDARIES_V1,
        M1R33RequestEventWireV1, M1R33WorkV1,
    };
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let sequence = NEXT_TEST.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "ferric-r33-service-test-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn digest(label: &str) -> String {
        sha256_hex(label.as_bytes())
    }

    fn work() -> crate::r33_wire::M1R33WorkV1 {
        M1R33WorkV1 {
            input_tokens: 2,
            output_tokens: 2,
            successful_requests: 1,
            total_tokens: 4,
        }
    }

    fn workload(service_id: &str, policy: &str) -> M1R33WorkloadDocumentV1 {
        let rows = (0..3 * M1_R33_WINDOWS_PER_START_V1)
            .map(|ordinal| {
                let within = ordinal % M1_R33_WINDOWS_PER_START_V1;
                let start = ordinal / M1_R33_WINDOWS_PER_START_V1;
                let phase = if within < 10 { "warmup" } else { "recorded" };
                M1R33WorkloadWindowV1 {
                    requests: vec![M1R33WorkloadRequestV1 {
                        expected_output_tokens: 2,
                        prompt_tokens: vec![1, 2],
                        request_ordinal: 0,
                    }],
                    row: M1R33CollectorRowV1 {
                        expected_work: work(),
                        id: format!("start-{start}.{phase}-{:02}", within % 10),
                        ordinal: ordinal as u64,
                        phase: phase.to_owned(),
                        server_start: start as u64,
                        window: (within % 10) as u64,
                    },
                }
            })
            .collect();
        M1R33WorkloadDocumentV1 {
            authority: M1_R33_SERVICE_AUTHORITY_V1.to_owned(),
            format: M1_R33_WORKLOAD_FORMAT_V1.to_owned(),
            policy_sha256: policy.to_owned(),
            rows,
            service_id: service_id.to_owned(),
            target: M1_R33_TARGET_V1.to_owned(),
        }
    }

    fn fixture() -> (TestDirectory, HeldM1R33ServiceBundleV1) {
        let root = TestDirectory::new();
        let service_id = digest("service");
        let policy = digest("policy");
        let workload_path = root.0.join("workload.json");
        let workload_bytes = encode_canonical_json_v1(&workload(&service_id, &policy)).unwrap();
        fs::write(&workload_path, &workload_bytes).unwrap();
        let plan = M1R33ServicePlanDocumentV1 {
            authority: M1_R33_SERVICE_AUTHORITY_V1.to_owned(),
            commands: M1R33CommandIdentitiesV1 {
                measure: digest("measure"),
                ready: digest("ready"),
                start: digest("start"),
                stop: digest("stop"),
            },
            expected_client_uid: geteuid().as_raw(),
            expected_daemon_uid: geteuid().as_raw(),
            format: M1_R33_SERVICE_PLAN_FORMAT_V1.to_owned(),
            implementation: serde_json::json!({"id": "ferric"}),
            io_timeout_ms: 100,
            policy_sha256: policy,
            service_id,
            slot: M1R33SlotV1 {
                hardware_configuration_sha256: digest("configuration"),
                hardware_sha256: digest("hardware"),
                id: "slot-0".to_owned(),
                target: M1_R33_TARGET_V1.to_owned(),
            },
            slot_gpu_ids: vec![0],
            socket_path: root.0.join("service.sock").to_str().unwrap().to_owned(),
            target: M1_R33_TARGET_V1.to_owned(),
            workload_path: workload_path.to_str().unwrap().to_owned(),
            workload_sha256: sha256_hex(&workload_bytes),
        };
        let plan_path = root.0.join("plan.json");
        fs::write(&plan_path, encode_canonical_json_v1(&plan).unwrap()).unwrap();
        let bundle = HeldM1R33ServiceBundleV1::open(&plan_path).unwrap();
        (root, bundle)
    }

    fn context(
        bundle: &HeldM1R33ServiceBundleV1,
        action: M1R33ActionV1,
        start: u64,
        instance: Option<String>,
        row: Option<M1R33CollectorRowV1>,
    ) -> M1R33CollectorContextV1 {
        M1R33CollectorContextV1 {
            action,
            command_sha256: bundle.plan().commands.for_action(action).to_owned(),
            engine: "ferric".to_owned(),
            engine_order: vec!["ferric".to_owned(), "vllm".to_owned(), "sglang".to_owned()],
            implementation: bundle.plan().implementation.clone(),
            policy_sha256: bundle.plan().policy_sha256.clone(),
            row,
            server_instance_sha256: instance,
            server_start: start,
            slot: bundle.plan().slot.clone(),
            target: M1_R33_TARGET_V1.to_owned(),
        }
    }

    fn request(
        bundle: &HeldM1R33ServiceBundleV1,
        context: M1R33CollectorContextV1,
        id: &str,
    ) -> M1R33WireRequestV1 {
        M1R33WireRequestV1 {
            context,
            format: crate::r33_wire::M1_R33_WIRE_REQUEST_FORMAT_V1.to_owned(),
            request_sha256: digest(id),
            service_id: bundle.plan().service_id.clone(),
            service_plan_sha256: bundle.plan_sha256().to_owned(),
        }
    }

    #[derive(Default)]
    struct TestBackend {
        active: Option<String>,
        fail_measure: bool,
        measures: usize,
        starts: usize,
        stops: usize,
    }

    impl sealed::Sealed for TestBackend {}

    impl M1R33AuthorityFreeBackendV1 for TestBackend {
        fn start(
            &mut self,
            instance: &str,
            _start: u64,
            _workload: &M1R33WorkloadDocumentV1,
            _deadline: M1R33OperationDeadlineV1,
        ) -> Result<(), M1R33BackendFaultV1> {
            assert!(self.active.is_none());
            self.active = Some(instance.to_owned());
            self.starts += 1;
            Ok(())
        }

        fn ready(
            &mut self,
            instance: &str,
            _deadline: M1R33OperationDeadlineV1,
        ) -> Result<(), M1R33BackendFaultV1> {
            assert_eq!(self.active.as_deref(), Some(instance));
            Ok(())
        }

        fn measure(
            &mut self,
            instance: &str,
            window: &M1R33WorkloadWindowV1,
            _deadline: M1R33OperationDeadlineV1,
        ) -> Result<M1R33MeasurementReportV1, M1R33BackendFaultV1> {
            assert_eq!(self.active.as_deref(), Some(instance));
            if self.fail_measure {
                return Err(M1R33BackendFaultV1::new("injected-measure-fault"));
            }
            self.measures += 1;
            Ok(M1R33MeasurementReportV1 {
                clock: M1_R33_CLOCK_V1.to_owned(),
                duration_boundary: M1_R33_DURATION_BOUNDARY_V1.to_owned(),
                duration_ns: 10,
                failed_requests: 0,
                input_tokens: window.row.expected_work.input_tokens,
                output_tokens: window.row.expected_work.output_tokens,
                request_events: vec![M1R33RequestEventWireV1 {
                    arrival_offset_ns: 0,
                    first_token_offset_ns: 2,
                    input_tokens: 2,
                    output_tokens: 2,
                    request_ordinal: 0,
                    terminal_offset_ns: 4,
                }],
                request_timing_boundaries: M1_R33_TIMING_BOUNDARIES_V1.to_owned(),
                successful_requests: 1,
                total_tokens: 4,
            })
        }

        fn stop(
            &mut self,
            instance: &str,
            _deadline: M1R33OperationDeadlineV1,
        ) -> Result<(), M1R33BackendFaultV1> {
            assert_eq!(self.active.as_deref(), Some(instance));
            self.active = None;
            self.stops += 1;
            Ok(())
        }
    }

    fn dispatch_delivered(
        coordinator: &mut M1R33DaemonCoordinatorV1<'_, TestBackend>,
        request: &M1R33WireRequestV1,
    ) -> M1R33WireResponseV1 {
        let response = coordinator.handle_request(
            M1R33PeerCredentialsV1 {
                gid: 0,
                pid: 1,
                uid: 0,
            },
            request,
        );
        coordinator.response_delivered(&request.request_sha256);
        response
    }

    #[test]
    fn held_bundle_rejects_workload_replacement() {
        let (_root, bundle) = fixture();
        let path = PathBuf::from(&bundle.plan().workload_path);
        fs::write(&path, b"{}\n").unwrap();
        assert!(bundle.revalidate().is_err());
    }

    #[test]
    fn lifecycle_runs_exactly_twenty_ordered_windows() {
        let (_root, bundle) = fixture();
        let mut coordinator = M1R33DaemonCoordinatorV1::new(&bundle, TestBackend::default());
        let start = request(
            &bundle,
            context(&bundle, M1R33ActionV1::Start, 0, None, None),
            "start",
        );
        let response = dispatch_delivered(&mut coordinator, &start);
        assert_eq!(response.status, M1R33WireStatusV1::Passed);
        let instance = response.server_instance_sha256.unwrap();
        let ready = request(
            &bundle,
            context(
                &bundle,
                M1R33ActionV1::Ready,
                0,
                Some(instance.clone()),
                None,
            ),
            "ready",
        );
        assert_eq!(
            dispatch_delivered(&mut coordinator, &ready).status,
            M1R33WireStatusV1::Passed
        );
        for ordinal in 0..M1_R33_WINDOWS_PER_START_V1 {
            let row = bundle.workload().row(0, ordinal).unwrap().row.clone();
            let measure = request(
                &bundle,
                context(
                    &bundle,
                    M1R33ActionV1::Measure,
                    0,
                    Some(instance.clone()),
                    Some(row),
                ),
                &format!("measure-{ordinal}"),
            );
            assert_eq!(
                dispatch_delivered(&mut coordinator, &measure).status,
                M1R33WireStatusV1::Passed
            );
        }
        let stale = request(
            &bundle,
            context(
                &bundle,
                M1R33ActionV1::Measure,
                0,
                Some(instance.clone()),
                Some(bundle.workload().row(0, 19).unwrap().row.clone()),
            ),
            "late",
        );
        assert_eq!(
            coordinator
                .handle_request(
                    M1R33PeerCredentialsV1 {
                        gid: 0,
                        pid: 1,
                        uid: 0
                    },
                    &stale
                )
                .status,
            M1R33WireStatusV1::Fault
        );
        let stop = request(
            &bundle,
            context(&bundle, M1R33ActionV1::Stop, 0, Some(instance), None),
            "stop",
        );
        assert_eq!(
            dispatch_delivered(&mut coordinator, &stop).status,
            M1R33WireStatusV1::Passed
        );
        assert_eq!(coordinator.backend.starts, 1);
        assert_eq!(coordinator.backend.measures, M1_R33_WINDOWS_PER_START_V1);
        assert_eq!(coordinator.backend.stops, 1);
    }

    #[test]
    fn cross_slot_instance_order_and_abandonment_fail_closed() {
        let (_root, bundle) = fixture();
        let mut coordinator = M1R33DaemonCoordinatorV1::new(&bundle, TestBackend::default());
        let start = request(
            &bundle,
            context(&bundle, M1R33ActionV1::Start, 0, None, None),
            "start",
        );
        let response = coordinator.handle_request(
            M1R33PeerCredentialsV1 {
                gid: 0,
                pid: 1,
                uid: 0,
            },
            &start,
        );
        let instance = response.server_instance_sha256.unwrap();
        coordinator.response_abandoned(&start.request_sha256);

        let mut wrong_slot = context(
            &bundle,
            M1R33ActionV1::Ready,
            0,
            Some(instance.clone()),
            None,
        );
        wrong_slot.slot.id = "slot-1".to_owned();
        let wrong_slot = request(&bundle, wrong_slot, "wrong-slot");
        assert_eq!(
            coordinator
                .handle_request(
                    M1R33PeerCredentialsV1 {
                        gid: 0,
                        pid: 1,
                        uid: 0
                    },
                    &wrong_slot
                )
                .status,
            M1R33WireStatusV1::Fault
        );
        let wrong_instance = request(
            &bundle,
            context(&bundle, M1R33ActionV1::Stop, 0, Some(digest("other")), None),
            "wrong-instance",
        );
        assert_eq!(
            coordinator
                .handle_request(
                    M1R33PeerCredentialsV1 {
                        gid: 0,
                        pid: 1,
                        uid: 0
                    },
                    &wrong_instance
                )
                .status,
            M1R33WireStatusV1::Fault
        );
        let stop = request(
            &bundle,
            context(&bundle, M1R33ActionV1::Stop, 0, Some(instance), None),
            "stop",
        );
        assert_eq!(
            dispatch_delivered(&mut coordinator, &stop).status,
            M1R33WireStatusV1::Passed
        );
    }

    #[test]
    fn backend_fault_admits_only_exact_stop() {
        let (_root, bundle) = fixture();
        let backend = TestBackend {
            fail_measure: true,
            ..TestBackend::default()
        };
        let mut coordinator = M1R33DaemonCoordinatorV1::new(&bundle, backend);
        let start = request(
            &bundle,
            context(&bundle, M1R33ActionV1::Start, 0, None, None),
            "start",
        );
        let instance = dispatch_delivered(&mut coordinator, &start)
            .server_instance_sha256
            .unwrap();
        let ready = request(
            &bundle,
            context(
                &bundle,
                M1R33ActionV1::Ready,
                0,
                Some(instance.clone()),
                None,
            ),
            "ready",
        );
        let _ = dispatch_delivered(&mut coordinator, &ready);
        let row = bundle.workload().row(0, 0).unwrap().row.clone();
        let measure = request(
            &bundle,
            context(
                &bundle,
                M1R33ActionV1::Measure,
                0,
                Some(instance.clone()),
                Some(row),
            ),
            "measure",
        );
        assert_eq!(
            coordinator
                .handle_request(
                    M1R33PeerCredentialsV1 {
                        gid: 0,
                        pid: 1,
                        uid: 0
                    },
                    &measure
                )
                .status,
            M1R33WireStatusV1::Fault
        );
        let ready_again = request(
            &bundle,
            context(
                &bundle,
                M1R33ActionV1::Ready,
                0,
                Some(instance.clone()),
                None,
            ),
            "ready-again",
        );
        assert_eq!(
            coordinator
                .handle_request(
                    M1R33PeerCredentialsV1 {
                        gid: 0,
                        pid: 1,
                        uid: 0
                    },
                    &ready_again
                )
                .status,
            M1R33WireStatusV1::Fault
        );
        let stop = request(
            &bundle,
            context(&bundle, M1R33ActionV1::Stop, 0, Some(instance), None),
            "stop",
        );
        assert_eq!(
            dispatch_delivered(&mut coordinator, &stop).status,
            M1R33WireStatusV1::Passed
        );
    }

    #[test]
    fn service_row_substitution_preserves_exact_cleanup_stop() {
        let (_root, bundle) = fixture();
        let mut coordinator = M1R33DaemonCoordinatorV1::new(&bundle, TestBackend::default());
        let mut cross_service = request(
            &bundle,
            context(&bundle, M1R33ActionV1::Start, 0, None, None),
            "cross-service",
        );
        cross_service.service_id = digest("other-service");
        assert_eq!(
            coordinator
                .handle_request(
                    M1R33PeerCredentialsV1 {
                        gid: 0,
                        pid: 1,
                        uid: 0
                    },
                    &cross_service
                )
                .status,
            M1R33WireStatusV1::Fault
        );

        let start = request(
            &bundle,
            context(&bundle, M1R33ActionV1::Start, 0, None, None),
            "valid-start",
        );
        let instance = dispatch_delivered(&mut coordinator, &start)
            .server_instance_sha256
            .unwrap();
        let ready = request(
            &bundle,
            context(
                &bundle,
                M1R33ActionV1::Ready,
                0,
                Some(instance.clone()),
                None,
            ),
            "valid-ready",
        );
        let _ = dispatch_delivered(&mut coordinator, &ready);
        let reordered = request(
            &bundle,
            context(
                &bundle,
                M1R33ActionV1::Measure,
                0,
                Some(instance.clone()),
                Some(bundle.workload().row(0, 1).unwrap().row.clone()),
            ),
            "reordered-row",
        );
        assert_eq!(
            coordinator
                .handle_request(
                    M1R33PeerCredentialsV1 {
                        gid: 0,
                        pid: 1,
                        uid: 0
                    },
                    &reordered
                )
                .status,
            M1R33WireStatusV1::Fault
        );
        let stop = request(
            &bundle,
            context(&bundle, M1R33ActionV1::Stop, 0, Some(instance), None),
            "cleanup-stop",
        );
        assert_eq!(
            dispatch_delivered(&mut coordinator, &stop).status,
            M1R33WireStatusV1::Passed
        );
        assert_eq!(coordinator.backend.measures, 0);
        assert_eq!(coordinator.backend.stops, 1);
    }

    #[test]
    fn unix_transport_checks_peer_and_exact_bindings() {
        let (_root, bundle) = fixture();
        let server = M1R33UnixServerV1::bind(&bundle).unwrap();
        let start = request(
            &bundle,
            context(&bundle, M1R33ActionV1::Start, 0, None, None),
            "transport-start",
        );
        let thread_bundle_path = bundle.plan.path.clone();
        let thread = std::thread::spawn(move || {
            let thread_bundle = HeldM1R33ServiceBundleV1::open(thread_bundle_path).unwrap();
            let mut coordinator =
                M1R33DaemonCoordinatorV1::new(&thread_bundle, TestBackend::default());
            server.serve_one(&mut coordinator).unwrap();
        });
        let response = exchange_with_supervised_service_v1(&bundle, &start).unwrap();
        assert_eq!(response.status, M1R33WireStatusV1::Passed);
        thread.join().unwrap();
    }

    #[test]
    fn unix_server_rejects_stale_socket_and_wrong_peer_uid() {
        let (_root, bundle) = fixture();
        let server = M1R33UnixServerV1::bind(&bundle).unwrap();
        assert!(M1R33UnixServerV1::bind(&bundle).is_err());
        let mut server = server;
        server.expected_client_uid ^= 1;
        let start = request(
            &bundle,
            context(&bundle, M1R33ActionV1::Start, 0, None, None),
            "wrong-peer",
        );
        let thread = std::thread::spawn(move || {
            let mut backend = TestBackend::default();
            let result = server.serve_one(&mut RejectBeforeHandler(&mut backend));
            assert!(matches!(
                result,
                Err(M1R33ServiceErrorV1::Peer("client uid"))
            ));
        });
        assert!(exchange_with_supervised_service_v1(&bundle, &start).is_err());
        thread.join().unwrap();
    }

    #[test]
    fn post_bind_guard_removes_only_the_exact_socket_inode() {
        let root = TestDirectory::new();
        let socket_path = root.0.join("guarded.sock");
        let _first_listener = UnixListener::bind(&socket_path).unwrap();
        let guard = BoundSocketPathGuardV1::new(socket_path.clone()).unwrap();
        drop(guard);
        assert!(!socket_path.exists());

        let replacement_path = root.0.join("replacement.sock");
        let _replacement_listener = UnixListener::bind(&socket_path).unwrap();
        let guard = BoundSocketPathGuardV1::new(socket_path.clone()).unwrap();
        fs::rename(&socket_path, &replacement_path).unwrap();
        fs::write(&socket_path, b"replacement").unwrap();
        drop(guard);
        assert_eq!(fs::read(&socket_path).unwrap(), b"replacement");
        assert!(
            fs::symlink_metadata(&replacement_path)
                .unwrap()
                .file_type()
                .is_socket()
        );
    }

    struct RejectBeforeHandler<'a>(&'a mut TestBackend);

    impl M1R33WireHandlerV1 for RejectBeforeHandler<'_> {
        fn handle_request(
            &mut self,
            _peer: M1R33PeerCredentialsV1,
            _request: &M1R33WireRequestV1,
        ) -> M1R33WireResponseV1 {
            self.0.starts += 1;
            panic!("wrong peer reached handler")
        }

        fn response_delivered(&mut self, _request_sha256: &str) {}

        fn response_abandoned(&mut self, _request_sha256: &str) {}
    }

    #[test]
    fn unix_client_rejects_response_timeout_and_partial_output() {
        for partial in [false, true] {
            let (_root, bundle) = fixture();
            let listener = UnixListener::bind(&bundle.plan().socket_path).unwrap();
            let thread = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let _: M1R33WireRequestV1 =
                    read_frame_open_v1(&mut stream, M1R33FrameKindV1::Request).unwrap();
                if partial {
                    stream.write_all(b"FRR33V1").unwrap();
                } else {
                    thread::sleep(Duration::from_millis(250));
                }
            });
            let start = request(
                &bundle,
                context(&bundle, M1R33ActionV1::Start, 0, None, None),
                if partial { "partial" } else { "timeout" },
            );
            let result = exchange_with_supervised_service_v1(&bundle, &start);
            assert!(matches!(
                result,
                Err(M1R33ServiceErrorV1::Wire(M1R33WireErrorV1::Io(_)) | M1R33ServiceErrorV1::Io(_))
            ));
            thread.join().unwrap();
        }
    }

    #[test]
    fn missing_response_ack_faults_instance_until_exact_stop() {
        let (_root, bundle) = fixture();
        let server = M1R33UnixServerV1::bind(&bundle).unwrap();
        let thread_bundle_path = bundle.plan.path.clone();
        let thread = std::thread::spawn(move || {
            let thread_bundle = HeldM1R33ServiceBundleV1::open(thread_bundle_path).unwrap();
            let mut coordinator =
                M1R33DaemonCoordinatorV1::new(&thread_bundle, TestBackend::default());
            assert!(server.serve_one(&mut coordinator).is_err());
            server.serve_one(&mut coordinator).unwrap();
            assert_eq!(coordinator.backend.starts, 1);
            assert_eq!(coordinator.backend.stops, 1);
        });

        let start = request(
            &bundle,
            context(&bundle, M1R33ActionV1::Start, 0, None, None),
            "unacknowledged-start",
        );
        let mut stream = UnixStream::connect(&bundle.plan().socket_path).unwrap();
        stream
            .set_read_timeout(Some(bundle.plan().timeout()))
            .unwrap();
        write_frame_v1(&mut stream, M1R33FrameKindV1::Request, &start).unwrap();
        let response: M1R33WireResponseV1 =
            read_frame_v1(&mut stream, M1R33FrameKindV1::Response).unwrap();
        let instance = response.server_instance_sha256.unwrap();
        drop(stream);

        let stop = request(
            &bundle,
            context(&bundle, M1R33ActionV1::Stop, 0, Some(instance), None),
            "cleanup-after-disconnect",
        );
        assert_eq!(
            exchange_with_supervised_service_v1(&bundle, &stop)
                .unwrap()
                .status,
            M1R33WireStatusV1::Passed
        );
        thread.join().unwrap();
    }
}
