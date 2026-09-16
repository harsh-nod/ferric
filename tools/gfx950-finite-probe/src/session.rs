use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use fe2o3_kfd::engineering_wire::{
    read_header_v1, write_header_v1, CommandV1, ResponseV1, MAX_TRANSFER_BYTES_V1,
};

use crate::Result;

pub const COMMAND_DEADLINE: Duration = Duration::from_secs(30);
const EXIT_DEADLINE: Duration = Duration::from_secs(3);

pub struct Packet {
    pub response: ResponseV1,
    pub payload: Vec<u8>,
}

pub trait Transport {
    fn request(&mut self, command: CommandV1, payload: Vec<u8>) -> Result<Packet>;
}

type Request = (CommandV1, Vec<u8>);

pub struct Session {
    child: Child,
    commands: Option<SyncSender<Request>>,
    replies: Receiver<Result<Packet>>,
    actor: Option<JoinHandle<()>>,
    deadline: Instant,
    command_timeout: Duration,
}

/// Bounded framing; error text deliberately excludes raw worker messages/UIDs.
pub fn receive(input: &mut impl Read) -> Result<Packet> {
    let response = read_header_v1::<ResponseV1>(input)
        .map_err(|_| "invalid worker response frame")?
        .ok_or("worker ended before its response")?;
    let bytes = match response {
        ResponseV1::Read { payload_bytes } if payload_bytes <= MAX_TRANSFER_BYTES_V1 => {
            payload_bytes
        }
        ResponseV1::Read { .. } => {
            return Err("worker response payload exceeds transfer cap".into())
        }
        ResponseV1::Error { .. } => {
            return Err("engineering worker rejected request; see private worker-stderr.log".into())
        }
        _ => 0,
    };
    let mut payload = vec![0; usize::try_from(bytes).expect("bounded wire payload")];
    input
        .read_exact(&mut payload)
        .map_err(|_| "truncated worker response payload")?;
    Ok(Packet { response, payload })
}

fn exchange(input: &mut impl Read, output: &mut impl Write, request: Request) -> Result<Packet> {
    let (command, payload) = request;
    if command
        .payload_bytes()
        .map_err(|_| "invalid command bounds")?
        != payload.len()
    {
        return Err("command payload length mismatch".into());
    }
    write_header_v1(output, &command).map_err(|_| "worker command write failed")?;
    output
        .write_all(&payload)
        .map_err(|_| "worker payload write failed")?;
    output.flush().map_err(|_| "worker command flush failed")?;
    receive(input)
}

impl Session {
    pub fn spawn(worker: &Path, unique_id: u64, stderr: File) -> Result<(Self, Packet)> {
        Self::spawn_with_deadline(worker, unique_id, stderr, COMMAND_DEADLINE)
    }

    pub(super) fn spawn_with_deadline(
        worker: &Path,
        unique_id: u64,
        stderr: File,
        command_timeout: Duration,
    ) -> Result<(Self, Packet)> {
        let mut child = Command::new(worker)
            .arg("--device-unique-id")
            .arg(unique_id.to_string())
            .arg("--allow-unauthenticated-machine-code")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()
            .map_err(|_| "engineering worker process could not be started")?;
        let mut output = child.stdin.take().expect("piped child stdin");
        let mut input = child.stdout.take().expect("piped child stdout");
        let (commands, requests) = mpsc::sync_channel::<Request>(1);
        let (replies, responses) = mpsc::sync_channel(1);
        let actor = thread::spawn(move || {
            let ready = receive(&mut input);
            let failed = ready.is_err();
            if replies.send(ready).is_err() || failed {
                return;
            }
            while let Ok(request) = requests.recv() {
                let closing = matches!(request.0, CommandV1::Close);
                let response = exchange(&mut input, &mut output, request);
                let failed = response.is_err();
                if replies.send(response).is_err() || closing || failed {
                    break;
                }
            }
        });
        let session = Self {
            child,
            commands: Some(commands),
            replies: responses,
            actor: Some(actor),
            deadline: Instant::now() + Duration::from_mins(3),
            command_timeout,
        };
        let ready = session.receive_bounded()?;
        Ok((session, ready))
    }

    fn receive_bounded(&self) -> Result<Packet> {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        self.replies
            .recv_timeout(self.command_timeout.min(remaining))
            .map_err(|_| "worker response deadline or channel failure")?
    }

    pub fn finish(mut self) -> Result<()> {
        self.commands.take();
        let deadline = Instant::now() + EXIT_DEADLINE;
        loop {
            if let Some(status) = self
                .child
                .try_wait()
                .map_err(|_| "worker exit query failed")?
            {
                if !status.success() {
                    return Err("worker exited unsuccessfully".into());
                }
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err("worker did not exit after Close".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn executable_digest(&self) -> Result<[u8; 32]> {
        let path = std::path::PathBuf::from(format!("/proc/{}/exe", self.child.id()));
        crate::read_bounded(&path, 256 * 1024 * 1024).map(|bytes| crate::artifact::digest(&bytes))
    }

    #[cfg(test)]
    pub(super) fn process_id(&self) -> u32 {
        self.child.id()
    }
}

impl Transport for Session {
    fn request(&mut self, command: CommandV1, payload: Vec<u8>) -> Result<Packet> {
        self.commands
            .as_ref()
            .ok_or("worker session is closed")?
            .try_send((command, payload))
            .map_err(|_| "worker command channel unavailable")?;
        self.receive_bounded()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.commands.take();
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            let _ = self.child.kill();
            let deadline = Instant::now() + EXIT_DEADLINE;
            while matches!(self.child.try_wait(), Ok(None)) && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(10));
            }
        }
        // Never wait indefinitely on a broken child pipe or an inherited FD.
        if self.actor.as_ref().is_some_and(JoinHandle::is_finished) {
            if let Some(actor) = self.actor.take() {
                let _ = actor.join();
            }
        }
    }
}
