//! Optional same-host replica release protocol. This grants no device authority.

use rustix::time::{ClockId, clock_gettime};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

const FRAME_LIMIT: usize = 4096;
const CLOCK: &str = "CLOCK_MONOTONIC_RAW";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ClockDomain {
    clock: String,
    hostname: String,
    boot_id: String,
    time_namespace_dev: u64,
    time_namespace_ino: u64,
}

impl ClockDomain {
    pub(super) fn current() -> Result<Self, String> {
        let namespace = std::fs::metadata("/proc/self/ns/time").map_err(error)?;
        let hostname = proc_text("/proc/sys/kernel/hostname")?;
        let boot_id = proc_text("/proc/sys/kernel/random/boot_id")?;
        if hostname.is_empty() || hostname.len() > 255 || boot_id.len() != 36 {
            return Err("invalid local clock domain".into());
        }
        Ok(Self {
            clock: CLOCK.into(),
            hostname,
            boot_id,
            time_namespace_dev: namespace.dev(),
            time_namespace_ino: namespace.ino(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ControlIdentity {
    nonce: String,
    replica_id: String,
    workload_sha256: String,
    requests_sha256: String,
    device_unique_ids: Vec<u64>,
    clock_domain: ClockDomain,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ControlConfig {
    schema: String,
    socket_path: PathBuf,
    launcher_pid: u32,
    identity: ControlIdentity,
    io_timeout_ms: u64,
    min_start_lead_ns: u64,
    max_start_lead_ns: u64,
    max_lateness_ns: u64,
}

#[derive(Serialize)]
struct ReplicaReady<'a> {
    schema: &'static str,
    authority: &'static str,
    identity: &'a ControlIdentity,
    pid: u32,
    ready_ns: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Start {
    schema: String,
    authority: String,
    identity: ControlIdentity,
    epoch_ns: u64,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct StartMetadata {
    schema: &'static str,
    authority: &'static str,
    identity: ControlIdentity,
    pid: u32,
    ready_ns: u64,
    start_received_ns: u64,
    epoch_ns: u64,
    started_ns: u64,
    lateness_ns: u64,
}

#[derive(Debug, Serialize)]
pub(super) struct CloseMetadata {
    schema: &'static str,
    authority: &'static str,
    identity: ControlIdentity,
    pid: u32,
    epoch_ns: u64,
    closed_ns: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CloseAck {
    schema: String,
    authority: String,
    identity: ControlIdentity,
    epoch_ns: u64,
}

pub(super) struct BenchmarkClock {
    config: ControlConfig,
    stream: UnixStream,
    metadata: StartMetadata,
}

fn error(value: impl std::fmt::Display) -> String {
    value.to_string()
}

fn proc_text(path: &str) -> Result<String, String> {
    let mut value = String::new();
    File::open(path)
        .map_err(error)?
        .take(257)
        .read_to_string(&mut value)
        .map_err(error)?;
    if value.len() > 256 || value.contains('\0') {
        return Err("oversized clock-domain field".into());
    }
    Ok(value.trim_end_matches('\n').to_owned())
}

pub(super) fn monotonic_raw_ns() -> Result<u64, String> {
    let timestamp = clock_gettime(ClockId::MonotonicRaw);
    let seconds = u64::try_from(timestamp.tv_sec).map_err(error)?;
    let nanoseconds = u64::try_from(timestamp.tv_nsec).map_err(error)?;
    if nanoseconds >= 1_000_000_000 {
        return Err("invalid clock nanoseconds".into());
    }
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|n| n.checked_add(nanoseconds))
        .ok_or_else(|| "monotonic clock overflow".into())
}

fn private_directory(path: &Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path).map_err(error)?;
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::getuid().as_raw()
        || metadata.mode() & 0o777 != 0o700
    {
        return Err("control parent must be an owned private0700 directory".into());
    }
    Ok(())
}

fn read_file(path: &Path, limit: u64, private: bool) -> Result<Vec<u8>, String> {
    let flags = i32::try_from((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits())
        .map_err(error)?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(flags)
        .open(path)
        .map_err(error)?;
    let before = file.metadata().map_err(error)?;
    if !before.is_file()
        || before.len() == 0
        || before.len() > limit
        || (private
            && (before.uid() != rustix::process::getuid().as_raw() || before.mode() & 0o077 != 0))
    {
        return Err("invalid bounded control input".into());
    }
    let mut data = Vec::new();
    (&file)
        .take(limit + 1)
        .read_to_end(&mut data)
        .map_err(error)?;
    let after = file.metadata().map_err(error)?;
    let identity = |m: &std::fs::Metadata| {
        (
            m.dev(),
            m.ino(),
            m.mode(),
            m.uid(),
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    };
    if u64::try_from(data.len()).map_err(error)? != before.len()
        || identity(&before) != identity(&after)
    {
        return Err("control input changed while reading".into());
    }
    Ok(data)
}

fn digest(data: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    Sha256::digest(data)
        .iter()
        .flat_map(|byte| {
            [
                char::from(DIGITS[usize::from(byte >> 4)]),
                char::from(DIGITS[usize::from(byte & 15)]),
            ]
        })
        .collect()
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && value.bytes().any(|b| b != b'0')
}

impl ControlConfig {
    pub(super) fn verify_loaded_requests_sha256(&self, sha: &str) -> Result<(), String> {
        if !valid_hash(sha) || sha != self.identity.requests_sha256 {
            return Err("parsed workload bytes differ from replica control identity".into());
        }
        Ok(())
    }

    pub(super) fn open(path: &Path, requests: &Path, devices: &[u64]) -> Result<Self, String> {
        if !path.is_absolute() {
            return Err("control path must be absolute".into());
        }
        private_directory(path.parent().ok_or("control path parent")?)?;
        let config: Self =
            serde_json::from_slice(&read_file(path, 16_384, true)?).map_err(error)?;
        config.validate(devices, &digest(&read_file(requests, 1_048_576, false)?))?;
        private_directory(config.socket_path.parent().ok_or("socket parent")?)?;
        let metadata = std::fs::symlink_metadata(&config.socket_path).map_err(error)?;
        if !metadata.file_type().is_socket()
            || metadata.uid() != rustix::process::getuid().as_raw()
            || metadata.mode() & 0o077 != 0
        {
            return Err("control endpoint must be an owned private Unix socket".into());
        }
        Ok(config)
    }

    fn validate(&self, devices: &[u64], requests_hash: &str) -> Result<(), String> {
        if self.schema != "FerricReplicaControlConfigV1"
            || !self.socket_path.is_absolute()
            || self.socket_path.as_os_str().len() > 100
            || self.launcher_pid == 0
            || !valid_hash(&self.identity.nonce)
            || !valid_hash(&self.identity.workload_sha256)
            || !valid_hash(&self.identity.requests_sha256)
            || self.identity.requests_sha256 != requests_hash
            || !(0..8).any(|i| self.identity.replica_id == format!("replica-{i:02}"))
            || !matches!(devices.len(), 1 | 2 | 8)
            || devices.contains(&0)
            || devices
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != devices.len()
            || self.identity.device_unique_ids != devices
            || self.identity.clock_domain != ClockDomain::current()?
            || !(1..=3_600_000).contains(&self.io_timeout_ms)
            || !(1_000_000..=1_000_000_000).contains(&self.min_start_lead_ns)
            || !(self.min_start_lead_ns..=10_000_000_000).contains(&self.max_start_lead_ns)
            || !(1..=1_000_000_000).contains(&self.max_lateness_ns)
        {
            return Err("replica control identity, clock, workload, or bounds differ".into());
        }
        Ok(())
    }

    pub(super) fn ready_and_wait(self) -> Result<BenchmarkClock, String> {
        if ClockDomain::current()? != self.identity.clock_domain {
            return Err("clock domain drift".into());
        }
        let mut stream = connect(&self.socket_path, self.io_timeout_ms)?;
        let peer = rustix::net::sockopt::socket_peercred(&stream).map_err(error)?;
        if peer.uid != rustix::process::getuid()
            || u32::try_from(peer.pid.as_raw_nonzero().get()).map_err(error)? != self.launcher_pid
        {
            return Err("control launcher peer credentials differ".into());
        }
        let ready_ns = monotonic_raw_ns()?;
        send(
            &mut stream,
            &ReplicaReady {
                schema: "FerricReplicaReadyV1",
                authority: "none",
                identity: &self.identity,
                pid: std::process::id(),
                ready_ns,
            },
            self.io_timeout_ms,
        )?;
        let start: Start = receive(&mut stream, self.io_timeout_ms)?;
        let received = monotonic_raw_ns()?;
        self.validate_start(&start, received)?;
        loop {
            let now = monotonic_raw_ns()?;
            if now >= start.epoch_ns {
                break;
            }
            std::thread::sleep(Duration::from_nanos((start.epoch_ns - now).min(1_000_000)));
        }
        let started_ns = monotonic_raw_ns()?;
        let lateness_ns = started_ns
            .checked_sub(start.epoch_ns)
            .ok_or("clock before release")?;
        if lateness_ns > self.max_lateness_ns
            || ClockDomain::current()? != self.identity.clock_domain
        {
            return Err("late release or changed clock domain".into());
        }
        let metadata = StartMetadata {
            schema: "FerricReplicaStartedV1",
            authority: "none",
            identity: self.identity.clone(),
            pid: std::process::id(),
            ready_ns,
            start_received_ns: received,
            epoch_ns: start.epoch_ns,
            started_ns,
            lateness_ns,
        };
        send(&mut stream, &metadata, self.io_timeout_ms)?;
        Ok(BenchmarkClock {
            config: self,
            stream,
            metadata,
        })
    }

    fn validate_start(&self, start: &Start, received: u64) -> Result<(), String> {
        let lead = start
            .epoch_ns
            .checked_sub(received)
            .ok_or("release epoch is not future")?;
        if start.schema != "FerricReplicaStartV1"
            || start.authority != "none"
            || start.identity != self.identity
            || !(self.min_start_lead_ns..=self.max_start_lead_ns).contains(&lead)
        {
            return Err("Start identity or future epoch differs".into());
        }
        Ok(())
    }
}

impl BenchmarkClock {
    pub(super) const fn metadata(&self) -> &StartMetadata {
        &self.metadata
    }
    pub(super) fn elapsed_ns(&self) -> Result<u64, String> {
        monotonic_raw_ns()?
            .checked_sub(self.metadata.epoch_ns)
            .ok_or_else(|| "clock precedes common epoch".into())
    }
    pub(super) fn finish(mut self) -> Result<CloseMetadata, String> {
        if ClockDomain::current()? != self.config.identity.clock_domain {
            return Err("clock domain drift on close".into());
        }
        let closed_ns = monotonic_raw_ns()?;
        if closed_ns < self.metadata.started_ns {
            return Err("noncausal replica close".into());
        }
        let closed = CloseMetadata {
            schema: "FerricReplicaClosedV1",
            authority: "none",
            identity: self.config.identity.clone(),
            pid: std::process::id(),
            epoch_ns: self.metadata.epoch_ns,
            closed_ns,
        };
        send(&mut self.stream, &closed, self.config.io_timeout_ms)?;
        let ack: CloseAck = receive(&mut self.stream, self.config.io_timeout_ms)?;
        if ack.schema != "FerricReplicaCloseAckV1"
            || ack.authority != "none"
            || ack.identity != self.config.identity
            || ack.epoch_ns != self.metadata.epoch_ns
        {
            return Err("close acknowledgement differs".into());
        }
        Ok(closed)
    }
}

fn remaining(deadline: u64) -> Result<Duration, String> {
    let ns = deadline
        .checked_sub(monotonic_raw_ns()?)
        .filter(|n| *n > 0)
        .ok_or("control deadline exceeded")?;
    Ok(Duration::from_nanos(ns))
}
fn deadline(timeout_ms: u64) -> Result<u64, String> {
    monotonic_raw_ns()?
        .checked_add(
            timeout_ms
                .checked_mul(1_000_000)
                .ok_or("timeout overflow")?,
        )
        .ok_or_else(|| "deadline overflow".into())
}
fn connect(path: &Path, timeout_ms: u64) -> Result<UnixStream, String> {
    use rustix::net::{AddressFamily, SocketAddrUnix, SocketFlags, SocketType};
    let address = SocketAddrUnix::new(path).map_err(error)?;
    let end = deadline(timeout_ms)?;
    loop {
        remaining(end)?;
        let socket = rustix::net::socket_with(
            AddressFamily::UNIX,
            SocketType::STREAM,
            SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
            None,
        )
        .map_err(error)?;
        match rustix::net::connect(&socket, &address) {
            Ok(()) => {
                let stream = UnixStream::from(socket);
                stream.set_nonblocking(false).map_err(error)?;
                return Ok(stream);
            }
            Err(rustix::io::Errno::AGAIN) => {
                std::thread::sleep(remaining(end)?.min(Duration::from_millis(1)));
            }
            Err(failure) => return Err(error(failure)),
        }
    }
}
fn send(stream: &mut UnixStream, value: &impl Serialize, timeout_ms: u64) -> Result<(), String> {
    let mut bytes = serde_json::to_vec(value).map_err(error)?;
    bytes.push(b'\n');
    if bytes.len() > FRAME_LIMIT {
        return Err("oversized control frame".into());
    }
    let end = deadline(timeout_ms)?;
    while !bytes.is_empty() {
        stream
            .set_write_timeout(Some(remaining(end)?))
            .map_err(error)?;
        let written = stream.write(&bytes).map_err(error)?;
        if written == 0 {
            return Err("closed control writer".into());
        }
        bytes.drain(..written);
    }
    Ok(())
}
fn receive<T: DeserializeOwned>(stream: &mut UnixStream, timeout_ms: u64) -> Result<T, String> {
    let end = deadline(timeout_ms)?;
    let mut frame = Vec::new();
    while frame.len() < FRAME_LIMIT {
        stream
            .set_read_timeout(Some(remaining(end)?))
            .map_err(error)?;
        let mut byte = [0];
        if stream.read(&mut byte).map_err(error)? == 0 {
            return Err("EOF before control frame".into());
        }
        if byte[0] == b'\n' {
            return serde_json::from_slice(&frame).map_err(error);
        }
        frame.push(byte[0]);
    }
    Err("oversized control frame".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    use std::os::unix::net::UnixListener;

    struct PrivateDir(PathBuf);
    impl PrivateDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "frctl-{}-{}",
                std::process::id(),
                monotonic_raw_ns().unwrap()
            ));
            std::fs::DirBuilder::new()
                .mode(0o700)
                .create(&path)
                .unwrap();
            Self(path)
        }
    }
    impl Drop for PrivateDir {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn config(path: PathBuf) -> ControlConfig {
        ControlConfig {
            schema: "FerricReplicaControlConfigV1".into(),
            socket_path: path,
            launcher_pid: std::process::id(),
            identity: ControlIdentity {
                nonce: "a".repeat(64),
                replica_id: "replica-00".into(),
                workload_sha256: "b".repeat(64),
                requests_sha256: digest(b"requests"),
                device_unique_ids: vec![1, 2],
                clock_domain: ClockDomain::current().unwrap(),
            },
            io_timeout_ms: 1000,
            min_start_lead_ns: 1_000_000,
            max_start_lead_ns: 1_000_000_000,
            max_lateness_ns: 500_000_000,
        }
    }
    #[test]
    fn checked_clock_and_exact_config_identity() {
        assert_eq!(
            digest(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let before = monotonic_raw_ns().unwrap();
        assert!(monotonic_raw_ns().unwrap() >= before);
        let mut value = config(PathBuf::from("/tmp/private/control.sock"));
        assert!(
            value
                .verify_loaded_requests_sha256(&digest(b"requests"))
                .is_ok()
        );
        assert!(
            value
                .verify_loaded_requests_sha256(&digest(b"changed"))
                .is_err()
        );
        assert!(value.verify_loaded_requests_sha256("invalid").is_err());
        assert!(value.validate(&[1, 2], &digest(b"requests")).is_ok());
        assert!(value.validate(&[2, 1], &digest(b"requests")).is_err());
        assert!(value.validate(&[1, 1], &digest(b"requests")).is_err());
        assert!(value.validate(&[1, 2], &digest(b"changed")).is_err());
        value.identity.clock_domain.time_namespace_ino += 1;
        assert!(value.validate(&[1, 2], &digest(b"requests")).is_err());
    }
    #[test]
    fn start_rejects_wrong_nonce_epoch_and_unknown_fields() {
        let config = config(PathBuf::from("/tmp/private/control.sock"));
        let mut start = Start {
            schema: "FerricReplicaStartV1".into(),
            authority: "none".into(),
            identity: config.identity.clone(),
            epoch_ns: 10_000_000,
        };
        assert!(config.validate_start(&start, 1_000_000).is_ok());
        assert!(config.validate_start(&start, start.epoch_ns).is_err());
        assert!(config.validate_start(&start, start.epoch_ns + 1).is_err());
        start.identity.nonce = "c".repeat(64);
        assert!(config.validate_start(&start, 1_000_000).is_err());
        assert!(serde_json::from_str::<Start>("{\"unexpected\":true}").is_err());
        assert!(serde_json::from_str::<Start>("{\"schema\":\"a\",\"schema\":\"a\"}").is_err());
    }
    #[test]
    fn framing_bounds_eof_timeout_and_duplicate_keys() {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        writer
            .write_all(b"{\"schema\":\"a\",\"schema\":\"a\"}\n")
            .unwrap();
        assert!(receive::<Start>(&mut reader, 10).is_err());
        assert!(receive::<Start>(&mut reader, 10).is_err());
        writer.write_all(&vec![b'x'; FRAME_LIMIT]).unwrap();
        assert!(receive::<Start>(&mut reader, 100).is_err());
        drop(writer);
        assert!(receive::<Start>(&mut reader, 10).is_err());
    }
    #[test]
    fn private_file_hash_and_socket_admission() {
        let directory = PrivateDir::new();
        let endpoint = directory.0.join("control.sock");
        let _listener = UnixListener::bind(&endpoint).unwrap();
        std::fs::set_permissions(&endpoint, std::fs::Permissions::from_mode(0o600)).unwrap();
        let value = config(endpoint);
        let path = directory.0.join("control.json");
        let requests = directory.0.join("requests.json");
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&requests)
            .unwrap()
            .write_all(b"requests")
            .unwrap();
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap()
            .write_all(&serde_json::to_vec(&value).unwrap())
            .unwrap();
        assert!(ControlConfig::open(&path, &requests, &[1, 2]).is_ok());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(ControlConfig::open(&path, &requests, &[1, 2]).is_err());
        let symlink = directory.0.join("alias");
        std::os::unix::fs::symlink(&requests, &symlink).unwrap();
        assert!(read_file(&symlink, 1024, false).is_err());
    }
    #[test]
    fn ready_future_release_and_acknowledged_close_use_one_clock() {
        roundtrip(false);
    }
    #[test]
    fn wrong_close_ack_is_a_failed_run() {
        roundtrip(true);
    }
    fn roundtrip(wrong_ack: bool) {
        let directory = PrivateDir::new();
        let endpoint = directory.0.join("control.sock");
        let listener = UnixListener::bind(&endpoint).unwrap();
        let value = config(endpoint);
        let launcher = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let ready: serde_json::Value = receive(&mut stream, 1000).unwrap();
            let epoch = monotonic_raw_ns().unwrap() + 20_000_000;
            send(
                &mut stream,
                &serde_json::json!({"schema":"FerricReplicaStartV1", "authority":"none",
                "identity":ready["identity"], "epoch_ns":epoch}),
                1000,
            )
            .unwrap();
            let started: serde_json::Value = receive(&mut stream, 1000).unwrap();
            assert_eq!(started["epoch_ns"], epoch);
            let closed: serde_json::Value = receive(&mut stream, 1000).unwrap();
            assert_eq!(closed["epoch_ns"], epoch);
            send(
                &mut stream,
                &serde_json::json!({"schema":"FerricReplicaCloseAckV1", "authority":"none",
                "identity":ready["identity"], "epoch_ns":epoch + u64::from(wrong_ack)}),
                1000,
            )
            .unwrap();
        });
        let clock = value.ready_and_wait().unwrap();
        assert!(clock.elapsed_ns().is_ok());
        assert_eq!(clock.metadata().identity.replica_id, "replica-00");
        assert_eq!(clock.finish().is_err(), wrong_ack);
        launcher.join().unwrap();
    }
}
