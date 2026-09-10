//! Owned-byte IPC to a separately opted-in, non-authoritative KFD process.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use fe2o3_hsaco::{
    ArgumentAccess, ExplicitArgument, ExplicitValueKind, ExplicitValueType, InspectedKernel,
};
use fe2o3_kfd::engineering_wire::{
    self as wire, BufferAccessV1, CommandV1, KernelMetadataV1, PointerFixupV1, ResponseV1,
};
use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpArtifactV1;
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTpArgumentV1, EngineeringTpBufferAccessV1, EngineeringTpDispatchV1,
    EngineeringTpRankTransportV1, TpResult,
};
use sha2::{Digest, Sha256};

const OP_TIMEOUT: Duration = Duration::from_mins(2);
const EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const DISPATCH_TIMEOUT_MS: u32 = 60_000;

struct Outgoing {
    header: CommandV1,
    payload: Vec<u8>,
}

struct Incoming {
    header: ResponseV1,
    payload: Vec<u8>,
}

struct LoadedKernel {
    id: u64,
    metadata: InspectedKernel,
}

#[derive(Eq, PartialEq)]
enum PendingRequest {
    Other,
    Dispatch,
}

pub struct Worker {
    child: Child,
    writer: Option<SyncSender<Outgoing>>,
    written: Receiver<TpResult<()>>,
    reader: Receiver<TpResult<Incoming>>,
    io_threads: Vec<JoinHandle<()>>,
    buffers: BTreeMap<u64, usize>,
    kernels: BTreeMap<String, LoadedKernel>,
    pending: Option<PendingRequest>,
    failed: bool,
    exited: bool,
    timeout: Duration,
}

impl Worker {
    pub fn spawn(
        executable: &Path,
        unique_id: u64,
        artifact: &EngineeringTpArtifactV1,
    ) -> TpResult<Self> {
        let child = Command::new(executable)
            .arg("--device-unique-id")
            .arg(unique_id.to_string())
            .arg("--allow-unauthenticated-machine-code")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("spawn GPU worker: {error}"))?;
        let mut worker = Self::connect(child, unique_id, OP_TIMEOUT)?;
        worker.load_artifact(artifact)?;
        Ok(worker)
    }

    fn connect(mut child: Child, unique_id: u64, timeout: Duration) -> TpResult<Self> {
        let Some(mut input) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("worker stdin missing".into());
        };
        let Some(mut output) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("worker stdout missing".into());
        };
        let (writer, pending_frames) = mpsc::sync_channel::<Outgoing>(1);
        let (acknowledge, written) = mpsc::sync_channel(1);
        let (responses, reader) = mpsc::sync_channel(1);
        // Both pipe directions have independent bounded queues. The controller
        // never blocks in a pipe write when a child stops consuming input.
        let write_thread = thread::spawn(move || {
            while let Ok(frame) = pending_frames.recv() {
                let result = (|| {
                    wire::write_header_v1(&mut input, &frame.header)?;
                    input.write_all(&frame.payload)?;
                    input.flush()
                })()
                .map_err(|error: std::io::Error| format!("worker pipe write: {error}"));
                let failed = result.is_err();
                if acknowledge.send(result).is_err() || failed {
                    break;
                }
            }
        });
        let read_thread = thread::spawn(move || {
            loop {
                let result = read_response(&mut output);
                let failed = result.is_err();
                if responses.send(result).is_err() || failed {
                    break;
                }
            }
        });
        let mut worker = Self {
            child,
            writer: Some(writer),
            written,
            reader,
            io_threads: vec![write_thread, read_thread],
            buffers: BTreeMap::new(),
            kernels: BTreeMap::new(),
            pending: Some(PendingRequest::Other),
            failed: false,
            exited: false,
            timeout,
        };
        let ready = worker.receive()?;
        if !matches!(ready.header, ResponseV1::Ready { protocol, target, device_unique_id, authority }
            if protocol == wire::PROTOCOL_VERSION_V1 && target == wire::TARGET_V1
                && device_unique_id == unique_id && authority == "none")
        {
            return worker.reject("worker ready identity mismatch");
        }
        Ok(worker)
    }

    fn load_artifact(&mut self, artifact: &EngineeringTpArtifactV1) -> TpResult<()> {
        let hash: [u8; 32] = Sha256::digest(artifact.bytes()).into();
        for metadata in artifact.inspection().hsaco().kernels() {
            let length =
                u32::try_from(artifact.bytes().len()).map_err(|_| "HSACO length overflow")?;
            self.send(
                CommandV1::LoadKernel {
                    payload_bytes: length,
                    object_sha256: hash,
                    symbol: metadata.name().into(),
                },
                artifact.bytes().to_vec(),
            )?;
            let response = self.receive()?;
            let ResponseV1::LoadedKernel {
                kernel,
                metadata: actual,
            } = response.header
            else {
                return self.reject("worker did not load kernel");
            };
            if kernel == 0
                || self.kernels.values().any(|loaded| loaded.id == kernel)
                || !metadata_matches(metadata, &actual, hash)
            {
                return self.reject("worker kernel metadata or identity mismatch");
            }
            self.kernels.insert(
                metadata.name().into(),
                LoadedKernel {
                    id: kernel,
                    metadata: metadata.clone(),
                },
            );
        }
        Ok(())
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    fn reject<T>(&mut self, message: impl Into<String>) -> TpResult<T> {
        self.failed = true;
        let message = message.into();
        let cleanup = self.terminate();
        Err(match cleanup {
            Ok(()) => message,
            Err(error) => format!("{message}; {error}"),
        })
    }

    fn send(&mut self, header: CommandV1, payload: Vec<u8>) -> TpResult<()> {
        if self.failed || self.exited || self.pending.is_some() {
            return self.reject("worker is not ready for a request");
        }
        if header.payload_bytes().map_err(|error| error.to_string())? != payload.len() {
            return self.reject("outgoing payload length mismatch");
        }
        self.pending = Some(if matches!(header, CommandV1::Dispatch { .. }) {
            PendingRequest::Dispatch
        } else {
            PendingRequest::Other
        });
        let Some(writer) = &self.writer else {
            return self.reject("worker writer closed");
        };
        if writer.try_send(Outgoing { header, payload }).is_err() {
            return self.reject("worker writer unavailable");
        }
        match self.written.recv_timeout(self.timeout) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => self.reject(error),
            Err(error) => self.reject(format!("worker write deadline/disconnect: {error}")),
        }
    }

    fn receive(&mut self) -> TpResult<Incoming> {
        if self.pending.is_none() || self.failed || self.exited {
            return self.reject("worker has no admissible pending response");
        }
        match self.reader.recv_timeout(self.timeout) {
            Ok(Ok(Incoming {
                header: ResponseV1::Error { message, fatal },
                ..
            })) => self.reject(format!("GPU worker failed (fatal={fatal}): {message}")),
            Ok(Ok(response)) => {
                self.pending = None;
                Ok(response)
            }
            Ok(Err(error)) => self.reject(error),
            Err(error) => self.reject(format!("worker response deadline/disconnect: {error}")),
        }
    }

    fn extent(&self, buffer: u64, offset: usize, bytes: usize) -> TpResult<()> {
        let capacity = self.buffers.get(&buffer).ok_or("unknown owned buffer")?;
        let end = offset.checked_add(bytes).ok_or("buffer extent overflow")?;
        if offset > *capacity || end > *capacity {
            return Err("buffer extent outside allocation".into());
        }
        Ok(())
    }

    fn terminate(&mut self) -> TpResult<()> {
        self.writer.take();
        if self.exited {
            return Ok(());
        }
        if self
            .child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_none()
        {
            self.child
                .kill()
                .map_err(|error| format!("cannot kill worker {}: {error}", self.pid()))?;
        }
        self.await_exit(false)
    }

    fn await_exit(&mut self, successful: bool) -> TpResult<()> {
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().map_err(|error| error.to_string())? {
                self.exited = true;
                self.writer.take();
                while self.io_threads.iter().any(|thread| !thread.is_finished()) {
                    if start.elapsed() >= EXIT_TIMEOUT {
                        return Err(format!(
                            "worker {} exited but IPC threads did not stop",
                            self.pid()
                        ));
                    }
                    thread::sleep(Duration::from_millis(1));
                }
                for handle in self.io_threads.drain(..) {
                    if handle.is_finished() {
                        let _ = handle.join();
                    }
                }
                return if successful && !status.success() {
                    Err(format!("worker exited with {status}"))
                } else {
                    Ok(())
                };
            }
            if start.elapsed() >= EXIT_TIMEOUT {
                return Err(format!(
                    "worker {} did not exit; resources are not confirmed released",
                    self.pid()
                ));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl EngineeringTpRankTransportV1 for Worker {
    fn allocate(&mut self, byte_len: usize) -> TpResult<u64> {
        if byte_len == 0 {
            return self.reject("zero allocation");
        }
        self.send(
            CommandV1::Allocate {
                bytes: byte_len as u64,
            },
            vec![],
        )?;
        let response = self.receive()?;
        match response.header {
            ResponseV1::Allocated { buffer, bytes }
                if buffer != 0
                    && bytes == byte_len as u64
                    && !self.buffers.contains_key(&buffer) =>
            {
                self.buffers.insert(buffer, byte_len);
                Ok(buffer)
            }
            _ => self.reject("allocation response mismatch"),
        }
    }

    fn write(&mut self, buffer: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        self.extent(buffer, offset, bytes.len())?;
        for (index, chunk) in bytes
            .chunks(wire::MAX_TRANSFER_BYTES_V1 as usize)
            .enumerate()
        {
            let position = offset + index * wire::MAX_TRANSFER_BYTES_V1 as usize;
            self.send(
                CommandV1::Write {
                    buffer,
                    offset: position as u64,
                    payload_bytes: u32::try_from(chunk.len()).map_err(|_| "write chunk length")?,
                },
                chunk.to_vec(),
            )?;
            if !matches!(self.receive()?.header, ResponseV1::Written) {
                return self.reject("write response mismatch");
            }
        }
        Ok(())
    }

    fn read(&mut self, buffer: u64, offset: usize, bytes: &mut [u8]) -> TpResult<()> {
        self.extent(buffer, offset, bytes.len())?;
        for (index, chunk) in bytes
            .chunks_mut(wire::MAX_TRANSFER_BYTES_V1 as usize)
            .enumerate()
        {
            let position = offset + index * wire::MAX_TRANSFER_BYTES_V1 as usize;
            self.send(
                CommandV1::Read {
                    buffer,
                    offset: position as u64,
                    bytes: u32::try_from(chunk.len()).map_err(|_| "read chunk length")?,
                },
                vec![],
            )?;
            let response = self.receive()?;
            if !matches!(response.header, ResponseV1::Read { payload_bytes } if payload_bytes as usize == chunk.len())
                || response.payload.len() != chunk.len()
            {
                return self.reject("read response mismatch");
            }
            chunk.copy_from_slice(&response.payload);
        }
        Ok(())
    }

    fn submit(&mut self, dispatch: &EngineeringTpDispatchV1) -> TpResult<()> {
        let loaded = self.kernels.get(dispatch.kernel).ok_or("unloaded kernel")?;
        let (header, bytes) = pack_dispatch(loaded, dispatch, &self.buffers)?;
        self.send(header, bytes)
    }

    fn wait(&mut self) -> TpResult<()> {
        if self.pending != Some(PendingRequest::Dispatch) {
            return self.reject("no pending GPU dispatch");
        }
        if !matches!(self.receive()?.header, ResponseV1::Dispatched { .. }) {
            return self.reject("dispatch completion response mismatch");
        }
        Ok(())
    }

    fn close(&mut self) -> TpResult<()> {
        if self.exited {
            return Ok(());
        }
        if self.failed {
            return self.terminate();
        }
        if self.pending == Some(PendingRequest::Dispatch) {
            self.wait()?;
        }
        self.send(CommandV1::Close, vec![])?;
        if !matches!(self.receive()?.header, ResponseV1::Closed) {
            return self.reject("close response mismatch");
        }
        let result = self.await_exit(true);
        if result.is_err() {
            let _ = self.terminate();
        }
        result
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if !self.exited
            && let Err(error) = self.terminate()
        {
            eprintln!("GPU worker cleanup: {error}");
        }
    }
}

fn read_response(input: &mut impl Read) -> TpResult<Incoming> {
    let header = wire::read_header_v1(input)
        .map_err(|error| format!("worker pipe header: {error}"))?
        .ok_or("worker closed its response pipe")?;
    let length = match &header {
        ResponseV1::Read { payload_bytes } if *payload_bytes <= wire::MAX_TRANSFER_BYTES_V1 => {
            *payload_bytes as usize
        }
        ResponseV1::Read { .. } => return Err("oversized worker payload".into()),
        _ => 0,
    };
    let mut payload = vec![0; length];
    input
        .read_exact(&mut payload)
        .map_err(|error| format!("worker pipe payload: {error}"))?;
    Ok(Incoming { header, payload })
}

fn wire_access(access: ArgumentAccess) -> BufferAccessV1 {
    match access {
        ArgumentAccess::ReadOnly => BufferAccessV1::Read,
        ArgumentAccess::WriteOnly => BufferAccessV1::Write,
        ArgumentAccess::ReadWrite => BufferAccessV1::ReadWrite,
    }
}

fn metadata_matches(expected: &InspectedKernel, actual: &KernelMetadataV1, hash: [u8; 32]) -> bool {
    if expected.name() != actual.symbol
        || hash != actual.object_sha256
        || expected.kernarg_segment_size() != u64::from(actual.kernarg_bytes)
        || expected.kernarg_segment_alignment() != u64::from(actual.kernarg_alignment)
        || expected.group_segment_fixed_size() != u64::from(actual.group_segment_bytes)
        || expected.private_segment_fixed_size() != u64::from(actual.private_segment_bytes)
        || expected.wavefront_size() != actual.wavefront_size
        || expected.implicit_argument_offset() != actual.implicit_argument_offset.map(u64::from)
        || expected.implicit_argument_size() != u64::from(actual.implicit_argument_bytes)
        || expected.explicit_arguments().len() != actual.explicit_arguments.len()
    {
        return false;
    }
    expected
        .explicit_arguments()
        .iter()
        .zip(&actual.explicit_arguments)
        .all(|(left, right)| {
            left.offset() == u64::from(right.offset)
                && left.size() == u64::from(right.bytes)
                && (left.value_kind() == ExplicitValueKind::GlobalBuffer) == right.global_buffer
                && left.pointee_alignment() == right.pointee_alignment.map(u64::from)
                && left.access().map(wire_access) == right.access
        })
}

fn pointee_size_matches(value_type: Option<ExplicitValueType>, element_bytes: u32) -> bool {
    use ExplicitValueType::{F16, F32, F64, I8, I16, I32, I64, Struct, U8, U16, U32, U64};
    match value_type {
        None => matches!(element_bytes, 2 | 4),
        Some(I8 | U8) => element_bytes == 1,
        Some(I16 | U16 | F16) => element_bytes == 2,
        Some(I32 | U32 | F32) => element_bytes == 4,
        Some(I64 | U64 | F64) => element_bytes == 8,
        Some(Struct) => false,
    }
}

fn pack_dispatch(
    loaded: &LoadedKernel,
    dispatch: &EngineeringTpDispatchV1,
    buffers: &BTreeMap<u64, usize>,
) -> TpResult<(CommandV1, Vec<u8>)> {
    let metadata = &loaded.metadata;
    let workgroup = [dispatch.workgroup_size, 1, 1];
    let grid_x = dispatch
        .grid_workgroups
        .checked_mul(u32::from(dispatch.workgroup_size))
        .ok_or("grid overflow")?;
    if grid_x == 0
        || dispatch.workgroup_size == 0
        || u32::from(dispatch.workgroup_size) > metadata.max_flat_workgroup_size()
        || metadata
            .required_workgroup_size()
            .is_some_and(|required| required != workgroup.map(u32::from))
        || metadata
            .max_workgroups()
            .iter()
            .zip([dispatch.grid_workgroups, 1, 1])
            .any(|(limit, actual)| limit.is_some_and(|limit| actual > limit))
    {
        return Err("dispatch geometry contradicts compiler metadata".into());
    }
    let length =
        u32::try_from(metadata.kernarg_segment_size()).map_err(|_| "kernarg length overflow")?;
    if length > wire::MAX_KERNARG_BYTES_V1 {
        return Err("kernarg bound exceeded".into());
    }
    let mut bytes = vec![0; length as usize];
    let mut explicit = metadata.explicit_arguments().iter();
    let mut pointers = Vec::new();
    for argument in &dispatch.arguments {
        let field = explicit
            .next()
            .ok_or("missing explicit argument metadata")?;
        match *argument {
            EngineeringTpArgumentV1::Buffer {
                id,
                offset,
                elements,
                element_bytes,
                access,
            } => {
                let wanted_access = match access {
                    EngineeringTpBufferAccessV1::Read => BufferAccessV1::Read,
                    EngineeringTpBufferAccessV1::Write => BufferAccessV1::Write,
                    EngineeringTpBufferAccessV1::ReadWrite => BufferAccessV1::ReadWrite,
                };
                let extent = elements
                    .checked_mul(element_bytes as usize)
                    .ok_or("slice extent overflow")?;
                let end = offset.checked_add(extent).ok_or("slice offset overflow")?;
                if element_bytes == 0
                    || !offset.is_multiple_of(element_bytes as usize)
                    || !buffers
                        .get(&id)
                        .is_some_and(|capacity| offset <= *capacity && end <= *capacity)
                    || field.value_kind() != ExplicitValueKind::GlobalBuffer
                    || field.size() != 8
                    || !pointee_size_matches(field.value_type(), element_bytes)
                    || field
                        .access()
                        .map(wire_access)
                        .is_some_and(|actual| actual != wanted_access)
                    || field.pointee_alignment().is_some_and(|alignment| {
                        alignment == 0 || !(offset as u64).is_multiple_of(alignment)
                    })
                {
                    return Err("slice ownership, access or physical ABI mismatch".into());
                }
                pointers.push(PointerFixupV1 {
                    kernarg_offset: u32::try_from(field.offset())
                        .map_err(|_| "pointer offset overflow")?,
                    buffer: id,
                    buffer_offset: offset as u64,
                    extent_bytes: extent as u64,
                    access: wanted_access,
                });
                let count = explicit.next().ok_or("missing slice length ABI field")?;
                put_scalar(
                    &mut bytes,
                    count,
                    &(elements as u64).to_le_bytes(),
                    ExplicitValueType::U64,
                )?;
            }
            EngineeringTpArgumentV1::U32(value) => put_scalar(
                &mut bytes,
                field,
                &value.to_le_bytes(),
                ExplicitValueType::U32,
            )?,
            EngineeringTpArgumentV1::F32(value) => {
                if !value.is_finite() {
                    return Err("nonfinite dispatch scalar".into());
                }
                put_scalar(
                    &mut bytes,
                    field,
                    &value.to_le_bytes(),
                    ExplicitValueType::F32,
                )?;
            }
        }
    }
    if explicit.next().is_some() {
        return Err("extra explicit compiler arguments".into());
    }
    let header = CommandV1::Dispatch {
        kernel: loaded.id,
        payload_bytes: length,
        workgroup,
        grid: [grid_x, 1, 1],
        pointers,
        timeout_ms: DISPATCH_TIMEOUT_MS,
    };
    header.payload_bytes().map_err(|error| error.to_string())?;
    Ok((header, bytes))
}

fn put_scalar(
    bytes: &mut [u8],
    field: &ExplicitArgument,
    value: &[u8],
    expected_type: ExplicitValueType,
) -> TpResult<()> {
    if field.value_kind() != ExplicitValueKind::ByValue
        || field.size() != value.len() as u64
        || field.value_type().is_some_and(|kind| kind != expected_type)
    {
        return Err("scalar physical ABI mismatch".into());
    }
    let start = usize::try_from(field.offset()).map_err(|_| "scalar offset overflow")?;
    let end = start
        .checked_add(value.len())
        .ok_or("scalar end overflow")?;
    bytes
        .get_mut(start..end)
        .ok_or("scalar exceeds kernarg extent")?
        .copy_from_slice(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAKE_WORKER: &str = r#"
import json, struct, sys, time
mode = sys.argv[1]
def send(value, payload=b''):
    header = json.dumps(value).encode()
    sys.stdout.buffer.write(struct.pack('<I', len(header)) + header + payload)
    sys.stdout.buffer.flush()
send({'op':'ready','protocol':1,'target':'gfx950:xnack-',
      'device_unique_id':2 if mode == 'wrong' else 1,'authority':'none'})
if mode != 'normal':
    time.sleep(60)
buffers = {}
while True:
    prefix = sys.stdin.buffer.read(4)
    if len(prefix) != 4: sys.exit(3)
    command = json.loads(sys.stdin.buffer.read(struct.unpack('<I',prefix)[0]))
    op = command['op']
    if op == 'allocate':
        identity = len(buffers) + 1
        buffers[identity] = bytearray(command['bytes'])
        send({'op':'allocated','buffer':identity,'bytes':command['bytes']})
    elif op == 'write':
        size = command['payload_bytes']
        value = sys.stdin.buffer.read(size)
        if len(value) != size: sys.exit(4)
        start = command['offset']
        buffers[command['buffer']][start:start+size] = value
        send({'op':'written'})
    elif op == 'read':
        start, size = command['offset'], command['bytes']
        send({'op':'read','payload_bytes':size}, buffers[command['buffer']][start:start+size])
    elif op == 'close':
        send({'op':'closed'})
        sys.exit(0)
    else:
        sys.exit(5)
"#;

    fn fake(mode: &str) -> Child {
        Command::new("python3")
            .arg("-u")
            .arg("-c")
            .arg(FAKE_WORKER)
            .arg(mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap()
    }

    #[test]
    fn owned_child_chunked_roundtrip_closes_threads_and_reaps_process() {
        let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
        let size = wire::MAX_TRANSFER_BYTES_V1 as usize + 17;
        let buffer = worker.allocate(size + 1).unwrap();
        let source = vec![0xa5; size];
        worker.write(buffer, 1, &source).unwrap();
        let mut actual = vec![0; size];
        worker.read(buffer, 1, &mut actual).unwrap();
        assert_eq!(actual, source);
        let mut first = [1];
        worker.read(buffer, 0, &mut first).unwrap();
        assert_eq!(first, [0]);
        worker.close().unwrap();
        assert!(worker.exited);
        assert!(worker.writer.is_none());
        assert!(worker.io_threads.is_empty());
        assert!(worker.child.try_wait().unwrap().unwrap().success());
    }

    #[test]
    fn wrong_ready_identity_is_rejected_and_child_is_reaped() {
        let child = fake("wrong");
        let pid = child.id();
        assert!(Worker::connect(child, 1, Duration::from_secs(5)).is_err());
        assert!(!Path::new(&format!("/proc/{pid}")).exists());
    }

    #[test]
    fn missing_response_has_a_deadline_and_kills_owned_child() {
        let mut worker = Worker::connect(fake("stall"), 1, Duration::from_secs(5)).unwrap();
        worker.timeout = Duration::from_millis(100);
        let start = Instant::now();
        assert!(worker.allocate(8).is_err());
        assert!(start.elapsed() < Duration::from_secs(10));
        assert!(worker.failed && worker.exited);
        assert!(worker.io_threads.is_empty());
    }

    #[test]
    fn blocked_pipe_write_has_a_deadline_and_kills_owned_child() {
        let mut worker = Worker::connect(fake("stall"), 1, Duration::from_secs(5)).unwrap();
        worker.timeout = Duration::from_millis(100);
        worker
            .buffers
            .insert(1, wire::MAX_TRANSFER_BYTES_V1 as usize);
        let start = Instant::now();
        assert!(
            worker
                .write(1, 0, &vec![7; wire::MAX_TRANSFER_BYTES_V1 as usize])
                .is_err()
        );
        assert!(start.elapsed() < Duration::from_secs(10));
        assert!(worker.failed && worker.exited);
        assert!(worker.io_threads.is_empty());
    }

    #[test]
    fn bounded_reader_requires_exact_payload_and_rejects_oversized_frames() {
        let mut bytes = Vec::new();
        wire::write_header_v1(&mut bytes, &ResponseV1::Read { payload_bytes: 3 }).unwrap();
        bytes.extend_from_slice(&[1, 2, 3]);
        let response = read_response(&mut bytes.as_slice()).unwrap();
        assert_eq!(response.payload, [1, 2, 3]);
        bytes.pop();
        assert!(read_response(&mut bytes.as_slice()).is_err());
        bytes.clear();
        wire::write_header_v1(
            &mut bytes,
            &ResponseV1::Read {
                payload_bytes: wire::MAX_TRANSFER_BYTES_V1 + 1,
            },
        )
        .unwrap();
        assert!(read_response(&mut bytes.as_slice()).is_err());
        assert!(read_response(&mut [].as_slice()).is_err());
    }

    #[test]
    fn compiler_pointee_width_cannot_be_substituted_by_caller() {
        assert!(pointee_size_matches(Some(ExplicitValueType::U16), 2));
        assert!(!pointee_size_matches(Some(ExplicitValueType::U16), 4));
        assert!(pointee_size_matches(Some(ExplicitValueType::F32), 4));
        assert!(!pointee_size_matches(Some(ExplicitValueType::F32), 2));
        assert!(!pointee_size_matches(Some(ExplicitValueType::Struct), 4));
        assert!(!pointee_size_matches(None, 0));
    }
}
