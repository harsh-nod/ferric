//! Closed long/readiness custody and bounded transport, without native authority.
use crate::finite_forward_wire_v1::{LOGIT_BYTES, OBSERVATION_BYTES, Payload, part};
use crate::finite_guarded_mlp_decode_wire_v1::{
    CONTROL_BYTES, Control, GUARDED_IMAGE, LayerObservation,
};
use crate::finite_long_wire_v1::header_bytes;
pub use crate::finite_long_wire_v1::{Command, FrameBudget};
use crate::finite_setup_wire_v1::{Begin, Part, Scope};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

pub const PROMPT_TOKENS: usize = 2048;
pub const OUTPUT_TOKENS: usize = 256;
pub const FORWARDS: u32 = 2303;
pub const READINESS_FORWARDS: u32 = 40;
pub const RECORD_BYTES: usize = 8192;
pub const STREAM_BYTES: usize = 64 << 20;
pub const EVIDENCE_BYTES: usize = 32 << 20;
pub const CAPTURE_BYTES: usize = 4 * (CONTROL_BYTES + OBSERVATION_BYTES);
// One bounded record per forward/Close, four captures, stderr and publication reserve.
pub const MAX_RETAINED_BYTES: usize = (FORWARDS as usize + 1) * (RECORD_BYTES + 4)
    + CAPTURE_BYTES
    + (2 << 20)
    + (128 << 10)
    + (512 << 10);
pub const RESPONSE_SCHEMA: &str = "FerricGuardedMlpLongResponseV2";
const VOCABULARY: u32 = 151_936;

fn check(ok: bool, message: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(message))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    Full2303,
    Readiness40,
}
impl Profile {
    pub const fn forwards(self) -> u32 {
        match self {
            Self::Full2303 => FORWARDS,
            Self::Readiness40 => READINESS_FORWARDS,
        }
    }
    pub const fn outputs(self) -> usize {
        match self {
            Self::Full2303 => OUTPUT_TOKENS,
            Self::Readiness40 => 0,
        }
    }
    pub const fn capture_positions(self) -> [u32; 4] {
        match self {
            Self::Full2303 => [0, 2047, 2048, 2302],
            Self::Readiness40 => [0, 15, 16, 39],
        }
    }
    pub fn captures(self, position: u32) -> bool {
        self.capture_positions().contains(&position)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub protocol: u32,
    pub profile: Profile,
    pub device_ids: [u64; 2],
    pub scope: Scope,
    pub registration: [u8; 32],
    pub begin: Begin,
    pub timeout_ms: u32,
    pub prompt_tokens: Vec<u32>,
    pub prefix_image: Part,
    pub mlp_image: Part,
    pub projection_image: Part,
    pub guarded_image: Part,
}
impl Bootstrap {
    pub fn validate(&self, devices: [u64; 2], timeout_ms: u32, pid: u32) -> io::Result<()> {
        check(
            self.protocol == 1
                && self.device_ids == devices
                && !devices.contains(&0)
                && devices[0] != devices[1]
                && self.timeout_ms == timeout_ms
                && (1..=10_000).contains(&timeout_ms)
                && pid != 0
                && self.scope.child_identity == pid
                && self.scope.bundle_id != [0; 32]
                && self.scope.model_id != [0; 32]
                && self.scope.session != [0; 32]
                && self.scope.pool_identity != 0
                && self.registration != [0; 32]
                && self.begin.scope == self.scope
                && self.begin.registration.sha256 == self.registration
                && self.prompt_tokens.len() == PROMPT_TOKENS
                && self.prompt_tokens.iter().all(|t| *t < VOCABULARY),
            "guarded long bootstrap identity/prompt",
        )?;
        check(
            self.guarded_image.sha256 == GUARDED_IMAGE,
            "guarded long selected guarded image",
        )?;
        self.total_input_bytes().map(|_| ())
    }
    /// Original separately bounded model uploads are not part of this observation stream.
    pub fn total_input_bytes(&self) -> io::Result<usize> {
        let tail = self
            .begin
            .tail_image
            .ok_or_else(|| io::Error::other("guarded long tail image"))?;
        let parts = [
            self.begin.registration,
            self.begin.source_program,
            self.begin.uploads,
            self.begin.prefix_image,
            self.begin.mlp_image,
            self.begin.residual_image,
            tail,
            self.prefix_image,
            self.mlp_image,
            self.projection_image,
            self.guarded_image,
        ];
        let mut total = 0usize;
        for (index, p) in parts.iter().enumerate() {
            check(
                p.bytes > 0
                    && p.sha256 != [0; 32]
                    && p.bytes <= if index < 3 { 4 << 20 } else { 32 << 20 },
                "guarded long setup part",
            )?;
            total = total
                .checked_add(usize::try_from(p.bytes).map_err(io::Error::other)?)
                .ok_or_else(|| io::Error::other("guarded long input overflow"))?;
        }
        let reserve = 2 * (crate::finite_composition_wire::MAX_HEADER + 4)
            + (self.profile.forwards() as usize + 1) * (RECORD_BYTES + 4);
        check(
            total <= STREAM_BYTES - reserve,
            "guarded long combined input budget",
        )?;
        Ok(total)
    }
    pub fn sha256(&self) -> io::Result<[u8; 32]> {
        self.validate(self.device_ids, self.timeout_ms, self.scope.child_identity)?;
        let mut hash = Sha256::new();
        hash.update(b"ferric-prefix284-combined2208-closed-long-v2\0");
        hash.update(header_bytes(self)?);
        Ok(hash.finalize().into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol: u32,
    pub id: u64,
    pub device_ids: [u64; 2],
    pub session: [u8; 32],
    pub registration: [u8; 32],
    pub profile_sha256: [u8; 32],
    pub command: Command,
}
impl Request {
    pub fn validate(&self, profile: Profile) -> io::Result<()> {
        check(
            self.protocol == 1
                && self.id > 0
                && !self.device_ids.contains(&0)
                && self.device_ids[0] != self.device_ids[1]
                && self.session != [0; 32]
                && self.registration != [0; 32]
                && self.profile_sha256 != [0; 32],
            "guarded long envelope",
        )?;
        match &self.command {
            Command::Forward { .. } => {
                check(
                    self.id <= u64::from(profile.forwards()),
                    "guarded closed forward limit",
                )?;
                crate::finite_long_wire_v1::Request {
                    protocol: self.protocol,
                    id: self.id,
                    device_ids: self.device_ids,
                    session: self.session,
                    registration: self.registration,
                    profile_sha256: self.profile_sha256,
                    command: self.command.clone(),
                }
                .validate()?;
            }
            Command::Close => check(
                self.id == u64::from(profile.forwards()) + 1,
                "guarded closed Close id",
            )?,
        }
        check(
            header_bytes(self)?.len() <= RECORD_BYTES,
            "guarded long request bound",
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BankStep {
    pub bank: u32,
    pub local_generation: u32,
    pub retired_forward: Option<u64>,
    pub logical_page: u32,
    pub page_offset: u32,
}
impl BankStep {
    pub fn at(profile: Profile, position: u32) -> io::Result<Self> {
        check(position < profile.forwards(), "guarded closed position")?;
        Ok(Self {
            bank: position % 2,
            local_generation: position / 2 + 1,
            retired_forward: if position >= 2 {
                Some(u64::from(position) - 1)
            } else {
                None
            },
            logical_page: position / 16,
            page_offset: position % 16,
        })
    }
}

/// The AR4 terminal predicates with a distinct closed generation bound, not a retirement proof.
pub fn validate_control(control: &Control, profile: Profile, generation: u64) -> io::Result<()> {
    check(
        (1..=u64::from(profile.forwards())).contains(&generation) && control.layers.len() == 36,
        "guarded long control generation/layers",
    )?;
    let local = BankStep::at(profile, (generation - 1) as u32)?.local_generation;
    let mut previous = [(0, 0); 2];
    for layer in &control.layers {
        for rank in 0..2 {
            let p = &layer.prefix_states[rank];
            check(
                p[..4] == [1, 0, 0, 31]
                    && p[4..9] == [1, 48, 1, 16, 64]
                    && p[9..14] == [1, 48, 1, 16, 64]
                    && p[14..19] == [u32::MAX, u32::MAX, u32::MAX, u32::MAX, 3]
                    && p[19..24] == [u32::MAX, u32::MAX, u32::MAX, u32::MAX, 3]
                    && p[24..154].iter().all(|v| (1..=64).contains(v))
                    && p[154..].iter().all(|v| *v == 64),
                "guarded long Prefix284 terminal",
            )?;
            crate::finite_mlp_tiles_comparison_wire_v1::validate_tiles_state(
                &layer.mlp_prefixes[rank],
            )?;
            check(
                layer.guards[rank] == [local, 0, 1, 0],
                "guarded long local guard generation",
            )?;
            let (write, read) = layer.observed_queue_frontiers[rank];
            check(
                read <= write && write > previous[rank].0 && read >= previous[rank].1,
                "guarded long intra-forward frontiers",
            )?;
            previous[rank] = (write, read);
        }
    }
    Ok(())
}

pub fn decode_control(bytes: &[u8], profile: Profile, generation: u64) -> io::Result<Control> {
    check(
        bytes.len() == CONTROL_BYTES,
        "guarded long exact control extent",
    )?;
    fn word<const N: usize>(r: &mut io::Cursor<&[u8]>) -> io::Result<[u8; N]> {
        let mut out = [0; N];
        r.read_exact(&mut out)?;
        Ok(out)
    }
    let mut r = io::Cursor::new(bytes);
    let mut out = Control {
        embedding_ns: [0; 2],
        tail_ns: [0; 3],
        layers: (0..36)
            .map(|_| LayerObservation {
                prefix_states: [[0; 284]; 2],
                mlp_prefixes: [[0; 548]; 2],
                guards: [[0; 4]; 2],
                prefix_host_ns: [0; 2],
                segment_host_ns: 0,
                observed_queue_frontiers: [(0, 0); 2],
            })
            .collect(),
    };
    for v in &mut out.embedding_ns {
        *v = u64::from_le_bytes(word(&mut r)?);
    }
    for layer in &mut out.layers {
        for words in &mut layer.prefix_states {
            for v in words {
                *v = u32::from_le_bytes(word(&mut r)?);
            }
        }
        for words in &mut layer.mlp_prefixes {
            for v in words {
                *v = u32::from_le_bytes(word(&mut r)?);
            }
        }
        for words in &mut layer.guards {
            for v in words {
                *v = u32::from_le_bytes(word(&mut r)?);
            }
        }
        for v in &mut layer.prefix_host_ns {
            *v = u64::from_le_bytes(word(&mut r)?);
        }
        layer.segment_host_ns = u64::from_le_bytes(word(&mut r)?);
        for (write, read) in &mut layer.observed_queue_frontiers {
            *write = u64::from_le_bytes(word(&mut r)?);
            *read = u64::from_le_bytes(word(&mut r)?);
        }
    }
    for v in &mut out.tail_ns {
        *v = u64::from_le_bytes(word(&mut r)?);
    }
    check(
        r.position() as usize == bytes.len(),
        "guarded long control trailing bytes",
    )?;
    validate_control(&out, profile, generation)?;
    Ok(out)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Completion {
    pub generation: u64,
    pub position: u32,
    pub input_token: u32,
    pub output_token: u32,
    pub bank: BankStep,
    pub control: Part,
    pub observation: Part,
    pub logits: Part,
    pub captured: bool,
    pub first_frontiers: [(u64, u64); 2],
    pub final_frontiers: [(u64, u64); 2],
    pub chain: [u8; 32],
}
fn argmax(bytes: &[u8]) -> io::Result<u32> {
    check(
        bytes.len() == OBSERVATION_BYTES,
        "guarded long observation extent",
    )?;
    let mut best = f32::NEG_INFINITY;
    let mut index = 0;
    for (i, pair) in bytes.chunks_exact(2).enumerate() {
        let word = u16::from_le_bytes([pair[0], pair[1]]);
        check(word & 0x7f80 != 0x7f80, "guarded long finite BF16")?;
        if i >= (OBSERVATION_BYTES - LOGIT_BYTES) / 2 {
            let value = f32::from_bits(u32::from(word) << 16);
            if value > best {
                best = value;
                index = (i - (OBSERVATION_BYTES - LOGIT_BYTES) / 2) as u32;
            }
        }
    }
    Ok(index)
}
impl Completion {
    pub fn from_observation(
        profile: Profile,
        request: &Request,
        control: &Control,
        bytes: &[u8],
        output_token: u32,
    ) -> io::Result<Self> {
        request.validate(profile)?;
        let Command::Forward {
            generation,
            token,
            cache_metadata,
            ..
        } = &request.command
        else {
            return Err(io::Error::other("guarded long completion for Close"));
        };
        validate_control(control, profile, *generation)?;
        check(
            argmax(bytes)? == output_token,
            "guarded long observed device argmax",
        )?;
        Ok(Self {
            generation: *generation,
            position: cache_metadata[0],
            input_token: *token,
            output_token,
            bank: BankStep::at(profile, cache_metadata[0])?,
            control: part(&control.encode()),
            observation: part(bytes),
            logits: part(&bytes[OBSERVATION_BYTES - LOGIT_BYTES..]),
            captured: profile.captures(cache_metadata[0]),
            first_frontiers: control.layers[0].observed_queue_frontiers,
            final_frontiers: control.layers[35].observed_queue_frontiers,
            chain: [0; 32],
        })
    }
    fn validate(&self, profile: Profile) -> io::Result<()> {
        check(
            self.position < profile.forwards()
                && self.generation == u64::from(self.position) + 1
                && self.input_token < VOCABULARY
                && self.output_token < VOCABULARY
                && self.bank == BankStep::at(profile, self.position)?
                && self.control.bytes as usize == CONTROL_BYTES
                && self.control.sha256 != [0; 32]
                && self.observation.bytes as usize == OBSERVATION_BYTES
                && self.observation.sha256 != [0; 32]
                && self.logits.bytes as usize == LOGIT_BYTES
                && self.logits.sha256 != [0; 32]
                && self.captured == profile.captures(self.position),
            "guarded long completion shape",
        )?;
        for rank in 0..2 {
            let first = self.first_frontiers[rank];
            let last = self.final_frontiers[rank];
            check(
                first.0 > 0
                    && first.1 <= first.0
                    && last.0 > first.0
                    && last.1 >= first.1
                    && last.1 <= last.0,
                "guarded long frontier summary",
            )?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub schema: String,
    pub profile: Profile,
    pub request: Request,
    pub completion: Completion,
}
impl Frame {
    pub fn validate(&self, profile: Profile) -> io::Result<()> {
        check(
            self.schema == RESPONSE_SCHEMA && self.profile == profile,
            "guarded long response profile/schema",
        )?;
        self.request.validate(profile)?;
        self.completion.validate(profile)?;
        let Command::Forward {
            generation,
            token,
            cache_metadata,
            ..
        } = &self.request.command
        else {
            return Err(io::Error::other("guarded long frame requires Forward"));
        };
        check(
            *generation == self.completion.generation
                && *token == self.completion.input_token
                && cache_metadata[0] == self.completion.position,
            "guarded long response/request binding",
        )?;
        check(
            header_bytes(self)?.len() <= RECORD_BYTES,
            "guarded long frame header bound",
        )
    }
}

pub fn write_record(
    writer: &mut impl Write,
    budget: &mut FrameBudget,
    value: &impl Serialize,
) -> io::Result<()> {
    let bytes = header_bytes(value)?;
    check(
        !bytes.is_empty() && bytes.len() <= RECORD_BYTES,
        "guarded long record bound",
    )?;
    budget.charge(4 + bytes.len())?;
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(&bytes)
}
pub fn read_record<T: serde::de::DeserializeOwned>(
    reader: &mut impl Read,
    budget: &mut FrameBudget,
) -> io::Result<Option<T>> {
    let mut prefix = [0; 4];
    if reader.read(&mut prefix[..1])? == 0 {
        return Ok(None);
    }
    budget.charge(4)?;
    reader.read_exact(&mut prefix[1..])?;
    let length = u32::from_le_bytes(prefix) as usize;
    check(
        length > 0 && length <= RECORD_BYTES,
        "guarded long received record bound",
    )?;
    budget.charge(length)?;
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(io::Error::other)
}

pub struct Capture {
    pub control: Control,
    pub observation: Vec<u8>,
    pub parts: Payload,
}
pub fn write_frame(
    writer: &mut impl Write,
    budget: &mut FrameBudget,
    frame: &Frame,
    control: &Control,
    payload: &[u8],
) -> io::Result<()> {
    frame.validate(frame.profile)?;
    let actual = Completion::from_observation(
        frame.profile,
        &frame.request,
        control,
        payload,
        frame.completion.output_token,
    )?;
    let mut expected = frame.completion.clone();
    expected.chain = [0; 32];
    check(actual == expected, "guarded long original completion/body")?;
    if frame.completion.captured {
        budget.charge(CONTROL_BYTES + OBSERVATION_BYTES)?;
    }
    write_record(writer, budget, frame)?;
    if frame.completion.captured {
        writer.write_all(&control.encode())?;
        writer.write_all(payload)?;
    }
    writer.flush()
}
pub fn read_frame(
    reader: &mut impl Read,
    budget: &mut FrameBudget,
    profile: Profile,
) -> io::Result<Option<(Frame, Option<Capture>)>> {
    let Some(frame): Option<Frame> = read_record(reader, budget)? else {
        return Ok(None);
    };
    frame.validate(profile)?;
    let capture = if frame.completion.captured {
        budget.charge(CONTROL_BYTES + OBSERVATION_BYTES)?;
        let mut control = vec![0; CONTROL_BYTES];
        let mut payload = vec![0; OBSERVATION_BYTES];
        reader.read_exact(&mut control)?;
        reader.read_exact(&mut payload)?;
        check(
            part(&control) == frame.completion.control,
            "guarded long selected control hash",
        )?;
        let control = decode_control(&control, profile, frame.completion.generation)?;
        let actual = Completion::from_observation(
            profile,
            &frame.request,
            &control,
            &payload,
            frame.completion.output_token,
        )?;
        let mut expected = frame.completion.clone();
        expected.chain = [0; 32];
        check(
            actual == expected,
            "guarded long selected control/payload joins",
        )?;
        Some(Capture {
            control,
            parts: Payload::from_bytes(&payload)?,
            observation: payload,
        })
    } else {
        None
    };
    Ok(Some((frame, capture)))
}

/// Commit accounting only. It neither resets owners nor authenticates native synchronization.
pub struct Transcript {
    bootstrap: Bootstrap,
    profile_hash: [u8; 32],
    chain: [u8; 32],
    completed: u32,
    previous: Option<u32>,
    pages: Option<Vec<u32>>,
    frontiers: [(u64, u64); 2],
    pending: Option<Request>,
    outputs: Vec<u32>,
    terminal: bool,
    closed: bool,
}
impl Transcript {
    pub fn new(bootstrap: Bootstrap) -> io::Result<Self> {
        let profile_hash = bootstrap.sha256()?;
        let mut hash = Sha256::new();
        hash.update(b"ferric-guarded-mlp-long-transcript-v2\0");
        hash.update(profile_hash);
        Ok(Self {
            bootstrap,
            profile_hash,
            chain: hash.finalize().into(),
            completed: 0,
            previous: None,
            pages: None,
            frontiers: [(0, 0); 2],
            pending: None,
            outputs: Vec::with_capacity(OUTPUT_TOKENS),
            terminal: false,
            closed: false,
        })
    }
    pub fn profile(&self) -> Profile {
        self.bootstrap.profile
    }
    pub fn profile_sha256(&self) -> [u8; 32] {
        self.profile_hash
    }
    pub fn digest(&self) -> [u8; 32] {
        self.chain
    }
    pub fn completed(&self) -> u32 {
        self.completed
    }
    pub fn output_tokens(&self) -> &[u32] {
        &self.outputs
    }
    pub fn is_closed(&self) -> bool {
        self.closed && !self.terminal
    }
    pub fn cancel(&mut self) {
        self.terminal = true;
        self.pending = None;
    }
    fn checked<T>(&mut self, value: io::Result<T>) -> io::Result<T> {
        if value.is_err() {
            self.cancel();
        }
        value
    }
    pub fn begin(&mut self, request: &Request) -> io::Result<BankStep> {
        let result = (|| {
            request.validate(self.profile())?;
            check(
                !self.terminal
                    && !self.closed
                    && self.pending.is_none()
                    && self.completed < self.profile().forwards()
                    && request.id == u64::from(self.completed) + 1
                    && request.device_ids == self.bootstrap.device_ids
                    && request.session == self.bootstrap.scope.session
                    && request.registration == self.bootstrap.registration
                    && request.profile_sha256 == self.profile_hash,
                "guarded long ordered request custody",
            )?;
            let Command::Forward {
                token,
                cache_metadata,
                ..
            } = &request.command
            else {
                return Err(io::Error::other("guarded long premature Close"));
            };
            let expected = if (self.completed as usize) < PROMPT_TOKENS {
                self.bootstrap.prompt_tokens[self.completed as usize]
            } else {
                self.previous
                    .ok_or_else(|| io::Error::other("guarded long missing predecessor"))?
            };
            check(
                *token == expected
                    && self
                        .pages
                        .as_ref()
                        .is_none_or(|p| p.as_slice() == &cache_metadata[1..]),
                "guarded long prompt/own-output/KV mapping",
            )?;
            let step = BankStep::at(self.profile(), self.completed)?;
            self.pending = Some(request.clone());
            Ok(step)
        })();
        self.checked(result)
    }
    pub fn next_chain(&self, request: &Request, completion: &Completion) -> io::Result<[u8; 32]> {
        check(
            !self.terminal && !self.closed && self.pending.as_ref() == Some(request),
            "guarded long chain custody",
        )?;
        let mut value = completion.clone();
        value.chain = [0; 32];
        let mut hash = Sha256::new();
        hash.update(self.chain);
        hash.update(header_bytes(request)?);
        hash.update(header_bytes(&value)?);
        Ok(hash.finalize().into())
    }
    /// The receiver calls this only after selected-body checks and durable evidence append.
    pub fn advance(&mut self, frame: &Frame) -> io::Result<[u8; 32]> {
        let result = (|| {
            frame.validate(self.profile())?;
            check(
                !self.terminal && !self.closed && self.pending.as_ref() == Some(&frame.request),
                "guarded long pending response",
            )?;
            for rank in 0..2 {
                check(
                    frame.completion.first_frontiers[rank].0 > self.frontiers[rank].0
                        && frame.completion.first_frontiers[rank].1 >= self.frontiers[rank].1,
                    "guarded long cross-forward frontiers",
                )?;
            }
            let next = self.next_chain(&frame.request, &frame.completion)?;
            check(frame.completion.chain == next, "guarded long transcript")?;
            let Command::Forward { cache_metadata, .. } = &frame.request.command else {
                unreachable!()
            };
            if self.completed as usize >= PROMPT_TOKENS - 1 {
                self.outputs.push(frame.completion.output_token);
            }
            self.previous = Some(frame.completion.output_token);
            self.pages = Some(cache_metadata[1..].to_vec());
            self.frontiers = frame.completion.final_frontiers;
            self.chain = next;
            self.pending = None;
            self.completed += 1;
            Ok(next)
        })();
        self.checked(result)
    }
    pub fn close(&mut self, request: &Request, digest: [u8; 32]) -> io::Result<()> {
        let result = (|| {
            request.validate(self.profile())?;
            check(
                !self.terminal
                    && !self.closed
                    && self.pending.is_none()
                    && self.completed == self.profile().forwards()
                    && self.outputs.len() == self.profile().outputs()
                    && request.command == Command::Close
                    && request.device_ids == self.bootstrap.device_ids
                    && request.session == self.bootstrap.scope.session
                    && request.registration == self.bootstrap.registration
                    && request.profile_sha256 == self.profile_hash
                    && digest == self.chain,
                "guarded long Close custody",
            )?;
            self.closed = true;
            Ok(())
        })();
        self.checked(result)
    }
}

#[cfg(test)]
#[path = "finite_guarded_mlp_long_wire_v2_tests.rs"]
pub(crate) mod tests;
