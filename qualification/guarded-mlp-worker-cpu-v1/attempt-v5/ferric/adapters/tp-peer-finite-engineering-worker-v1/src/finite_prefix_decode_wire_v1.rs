//! Closed four-forward Prefix284 + MLP548 transport; no legacy wire fallback.
use crate::finite_forward_wire_v1::{OBSERVATION_BYTES, Payload, part};
pub use crate::finite_long_wire_v1::FrameBudget;
use crate::finite_long_wire_v1::{read_header, write_header};
use crate::finite_setup_wire_v1::{self as setup, Begin, Part, Scope};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

pub const PROTOCOL: u32 = 1;
pub const FORWARDS: u32 = 4;
pub const CONTROL_BYTES: usize = 241_960;
pub const MAX_IMAGE_BYTES: usize = 32 << 20;
pub const HEADER_BYTES: usize = crate::finite_composition_wire::MAX_HEADER;
pub const STREAM_BYTES: usize = 64 << 20;
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    Prefix284Mlp548FourForwardV1,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputMode {
    TeacherForced,
    Autoregressive,
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
    pub mode: InputMode,
    pub input_tokens: Vec<u32>,
    pub tiles_image: Part,
    pub prefix_image: Part,
}
fn check(ok: bool, error: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(error))
    }
}
fn devices(ids: [u64; 2]) -> bool {
    ids[0] != 0 && ids[1] != 0 && ids[0] != ids[1]
}
impl Bootstrap {
    pub fn validate(
        &self,
        ids: [u64; 2],
        timeout: u32,
        pid: u32,
        mode: InputMode,
    ) -> io::Result<()> {
        check(
            self.protocol == PROTOCOL
                && devices(ids)
                && self.device_ids == ids
                && self.timeout_ms == timeout
                && (1..=10_000).contains(&timeout)
                && self.scope.child_identity == pid
                && pid != 0
                && self.scope.bundle_id != [0; 32]
                && self.scope.model_id != [0; 32]
                && self.scope.session != [0; 32]
                && self.scope.pool_identity != 0
                && self.registration != [0; 32]
                && self.mode == mode
                && self.input_tokens.len()
                    == (if mode == InputMode::TeacherForced {
                        4
                    } else {
                        1
                    })
                && self.input_tokens.iter().all(|t| *t < 151936)
                && self.tiles_image.bytes > 0
                && self.tiles_image.bytes as usize <= MAX_IMAGE_BYTES
                && self.tiles_image.sha256 != [0; 32],
            "tiles bootstrap identity/mode/image",
        )?;
        check(
            self.begin.scope == self.scope
                && self.begin.registration.sha256 == self.registration
                && self.prefix_image.bytes > 0
                && self.prefix_image.bytes as usize <= MAX_IMAGE_BYTES
                && self.prefix_image.sha256 != [0; 32],
            "prefix decode Begin/image binding",
        )?;
        self.total_input_bytes().map(|_| ())
    }
    /// Original Begin7 plus both new images share a closed pre-open envelope.
    /// Reserve bootstrap, Begin and all five later request headers. Subsequent
    /// model uploads retain the existing independently bounded setup protocol.
    pub fn total_input_bytes(&self) -> io::Result<usize> {
        let tail = self
            .begin
            .tail_image
            .ok_or_else(|| io::Error::other("prefix decode tail absent"))?;
        let parts = [
            self.begin.registration,
            self.begin.source_program,
            self.begin.uploads,
            self.begin.prefix_image,
            self.begin.mlp_image,
            self.begin.residual_image,
            tail,
            self.prefix_image,
            self.tiles_image,
        ];
        let mut total = 0usize;
        for (i, p) in parts.iter().enumerate() {
            check(
                p.bytes > 0 && p.sha256 != [0; 32] && (i >= 3 || p.bytes as usize <= 4 << 20),
                "prefix decode bootstrap part",
            )?;
            total = total
                .checked_add(p.bytes as usize)
                .ok_or_else(|| io::Error::other("prefix decode input overflow"))?;
        }
        check(
            total <= STREAM_BYTES - 7 * (HEADER_BYTES + 4),
            "prefix decode combined input bound",
        )?;
        Ok(total)
    }
    /// Exact backend Profile domain and field encoding, checked again before open.
    pub fn sha256(&self) -> io::Result<[u8; 32]> {
        self.validate(
            self.device_ids,
            self.timeout_ms,
            self.scope.child_identity,
            self.mode,
        )?;
        let mut h = Sha256::new();
        h.update(b"ferric-prefix284-mlp548-four-decode-v6\0");
        h.update(self.scope.bundle_id);
        h.update(self.scope.model_id);
        h.update(self.scope.session);
        h.update(self.scope.pool_identity.to_le_bytes());
        h.update(self.scope.group_id.to_le_bytes());
        h.update(self.scope.child_identity.to_le_bytes());
        h.update(self.registration);
        h.update(self.prefix_image.sha256);
        h.update(self.tiles_image.sha256);
        h.update(self.timeout_ms.to_le_bytes());
        h.update([u8::from(self.mode == InputMode::Autoregressive)]);
        for id in self.device_ids {
            h.update(id.to_le_bytes());
        }
        for i in 0..4 {
            h.update(self.input_tokens.get(i).copied().unwrap_or(0).to_le_bytes());
        }
        Ok(h.finalize().into())
    }
    pub fn input(&self, position: u32, previous: Option<u32>) -> io::Result<u32> {
        check(position < 4, "tiles input position")?;
        let token = match self.mode {
            InputMode::TeacherForced => self.input_tokens.get(position as usize).copied(),
            InputMode::Autoregressive => {
                if position == 0 {
                    self.input_tokens.first().copied()
                } else {
                    previous
                }
            }
        };
        let token =
            token.ok_or_else(|| io::Error::other("tiles previous checked output missing"))?;
        check(token < 151936, "tiles input vocabulary")?;
        Ok(token)
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Forward {
        generation: u64,
        token: u32,
        cache_metadata: Vec<u32>,
        rotary_bits: Vec<u32>,
    },
    Close,
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
fn envelope(
    protocol: u32,
    id: u64,
    ids: [u64; 2],
    session: [u8; 32],
    registration: [u8; 32],
    profile: [u8; 32],
) -> io::Result<()> {
    check(
        protocol == PROTOCOL
            && (1..=5).contains(&id)
            && devices(ids)
            && session != [0; 32]
            && registration != [0; 32]
            && profile != [0; 32],
        "tiles envelope",
    )
}
impl Request {
    pub fn validate(&self) -> io::Result<()> {
        envelope(
            self.protocol,
            self.id,
            self.device_ids,
            self.session,
            self.registration,
            self.profile_sha256,
        )?;
        match &self.command {
            Command::Close => check(self.id == 5, "tiles premature Close"),
            Command::Forward {
                generation,
                token,
                cache_metadata,
                rotary_bits,
            } => {
                check(
                    self.id <= 4
                        && *generation == self.id
                        && *token < 151936
                        && cache_metadata.len() == 145
                        && rotary_bits.len() == 128,
                    "tiles forward extent/order",
                )?;
                check(
                    cache_metadata[0] == self.id as u32 - 1
                        && rotary_bits.iter().all(|b| f32::from_bits(*b).is_finite()),
                    "tiles position/rotary",
                )?;
                let mut seen = [false; 144];
                for &page in &cache_metadata[1..] {
                    let slot = seen
                        .get_mut(page as usize)
                        .ok_or_else(|| io::Error::other("tiles page bound"))?;
                    check(!*slot, "tiles page alias")?;
                    *slot = true;
                }
                Ok(())
            }
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayerObservation {
    pub prefix_states: [[u32; 284]; 2],
    pub tiles_states: [[u32; 548]; 2],
    pub paired_ns: [[u64; 2]; 4],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Control {
    pub embedding_ns: [u64; 2],
    pub layers: [LayerObservation; 36],
    pub tail_ns: [u64; 3],
}
impl Control {
    pub fn validate(&self) -> io::Result<()> {
        for layer in &self.layers {
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
                    "tiles prefix terminal",
                )?;
                crate::finite_mlp_tiles_comparison_wire_v1::validate_tiles_state(
                    &layer.tiles_states[rank],
                )?;
            }
        }
        Ok(())
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(CONTROL_BYTES);
        for v in self.embedding_ns {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        for layer in &self.layers {
            for words in &layer.prefix_states {
                for v in words {
                    bytes.extend_from_slice(&v.to_le_bytes());
                }
            }
            for words in &layer.tiles_states {
                for v in words {
                    bytes.extend_from_slice(&v.to_le_bytes());
                }
            }
            for times in layer.paired_ns {
                for v in times {
                    bytes.extend_from_slice(&v.to_le_bytes());
                }
            }
        }
        for v in self.tail_ns {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes
    }
    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        check(
            bytes.len() == CONTROL_BYTES,
            "tiles control exact548 extent",
        )?;
        fn word<const N: usize>(r: &mut io::Cursor<&[u8]>) -> io::Result<[u8; N]> {
            let mut out = [0; N];
            r.read_exact(&mut out)?;
            Ok(out)
        }
        let mut r = io::Cursor::new(bytes);
        let mut result = Self {
            embedding_ns: [0; 2],
            layers: core::array::from_fn(|_| LayerObservation {
                prefix_states: [[0; 284]; 2],
                tiles_states: [[0; 548]; 2],
                paired_ns: [[0; 2]; 4],
            }),
            tail_ns: [0; 3],
        };
        for v in &mut result.embedding_ns {
            *v = u64::from_le_bytes(word(&mut r)?);
        }
        for layer in &mut result.layers {
            for words in &mut layer.prefix_states {
                for v in words {
                    *v = u32::from_le_bytes(word(&mut r)?);
                }
            }
            for words in &mut layer.tiles_states {
                for v in words {
                    *v = u32::from_le_bytes(word(&mut r)?);
                }
            }
            for times in &mut layer.paired_ns {
                for v in times {
                    *v = u64::from_le_bytes(word(&mut r)?);
                }
            }
        }
        for v in &mut result.tail_ns {
            *v = u64::from_le_bytes(word(&mut r)?);
        }
        result.validate()?;
        Ok(result)
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Completion {
    pub generation: u64,
    pub position: u32,
    pub input_token: u32,
    pub output_token: u32,
    pub control: Part,
    pub observation: Part,
    pub capture: Payload,
    pub chain: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    Completed(Completion),
    Closed {
        completed_forwards: u32,
        transcript_sha256: [u8; 32],
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub protocol: u32,
    pub id: u64,
    pub device_ids: [u64; 2],
    pub session: [u8; 32],
    pub registration: [u8; 32],
    pub profile_sha256: [u8; 32],
    pub event: Event,
    pub native_closed: bool,
    pub gpu_execution: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
}
impl Response {
    fn body_bytes(&self) -> io::Result<usize> {
        envelope(
            self.protocol,
            self.id,
            self.device_ids,
            self.session,
            self.registration,
            self.profile_sha256,
        )?;
        check(
            self.gpu_execution
                && !self.numerical_acceptance
                && !self.performance_claim
                && !self.production_authority,
            "tiles diagnostic claim boundary",
        )?;
        match &self.event {
            Event::Closed {
                completed_forwards, ..
            } => {
                check(
                    self.native_closed && self.id == 5 && *completed_forwards == 4,
                    "tiles Close identity",
                )?;
                Ok(0)
            }
            Event::Completed(c) => {
                check(
                    !self.native_closed
                        && self.id <= 4
                        && c.generation == self.id
                        && c.position == self.id as u32 - 1
                        && c.input_token < 151936
                        && c.output_token < 151936
                        && c.control.bytes as usize == CONTROL_BYTES
                        && c.observation.bytes as usize == OBSERVATION_BYTES
                        && c.capture.total == c.observation,
                    "tiles completion shape",
                )?;
                Ok(CONTROL_BYTES + OBSERVATION_BYTES)
            }
        }
    }
}
pub struct Chain([u8; 32]);
impl Chain {
    pub fn new(registration: [u8; 32], profile: [u8; 32]) -> Self {
        let mut h = Sha256::new();
        h.update(b"ferric-prefix284-mlp548-four-transcript-v1\0");
        h.update(registration);
        h.update(profile);
        Self(h.finalize().into())
    }
    pub fn advance(&mut self, c: &Completion) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.0);
        h.update(c.generation.to_le_bytes());
        h.update(c.position.to_le_bytes());
        h.update(c.input_token.to_le_bytes());
        h.update(c.output_token.to_le_bytes());
        h.update(c.control.bytes.to_le_bytes());
        h.update(c.control.sha256);
        h.update(c.observation.bytes.to_le_bytes());
        h.update(c.observation.sha256);
        self.0 = h.finalize().into();
        self.0
    }
    pub fn digest(&self) -> [u8; 32] {
        self.0
    }
}
fn image(expected: Part, bytes: &[u8]) -> io::Result<()> {
    check(
        !bytes.is_empty() && bytes.len() <= MAX_IMAGE_BYTES && part(bytes) == expected,
        "tiles supplied image body/hash",
    )
}
pub fn write_bootstrap(
    w: &mut impl Write,
    b: &mut FrameBudget,
    value: &Bootstrap,
    bytes: &[u8],
    prefix: &[u8],
) -> io::Result<()> {
    value.validate(
        value.device_ids,
        value.timeout_ms,
        value.scope.child_identity,
        value.mode,
    )?;
    image(value.tiles_image, bytes)?;
    image(value.prefix_image, prefix)?;
    b.charge(
        bytes
            .len()
            .checked_add(prefix.len())
            .ok_or_else(|| io::Error::other("image overflow"))?,
    )?;
    write_header(w, b, value)?;
    w.write_all(bytes)?;
    w.write_all(prefix)?;
    w.flush()
}
pub fn read_bootstrap(
    r: &mut impl Read,
    b: &mut FrameBudget,
) -> io::Result<Option<(Bootstrap, Vec<u8>, Vec<u8>)>> {
    let Some(value) = read_header::<Bootstrap>(r, b)? else {
        return Ok(None);
    };
    value.validate(
        value.device_ids,
        value.timeout_ms,
        value.scope.child_identity,
        value.mode,
    )?;
    b.charge(
        (value.tiles_image.bytes as usize)
            .checked_add(value.prefix_image.bytes as usize)
            .ok_or_else(|| io::Error::other("image overflow"))?,
    )?;
    let mut bytes = vec![0; value.tiles_image.bytes as usize];
    r.read_exact(&mut bytes)?;
    image(value.tiles_image, &bytes)?;
    let mut prefix = vec![0; value.prefix_image.bytes as usize];
    r.read_exact(&mut prefix)?;
    image(value.prefix_image, &prefix)?;
    Ok(Some((value, bytes, prefix)))
}
fn begin_request(b: &Bootstrap) -> setup::Request {
    setup::Request {
        protocol: setup::PROTOCOL,
        id: 1,
        device_ids: b.device_ids,
        session: b.scope.session,
        command: setup::Command::Begin(b.begin.clone()),
    }
}
/// Account the exact Begin sent by the unchanged parent setup Stream.
pub fn account_begin(budget: &mut FrameBudget, b: &Bootstrap) -> io::Result<()> {
    let request = begin_request(b);
    let header = crate::finite_long_wire_v1::header_bytes(&request)?;
    budget.charge(4 + header.len())?;
    budget.charge(request.payload_bytes()?)
}
/// Refuse a changed header before allocating the authenticated nine-part body.
pub fn read_begin(
    r: &mut impl Read,
    budget: &mut FrameBudget,
    b: &Bootstrap,
) -> io::Result<(setup::Request, Vec<u8>)> {
    let request: setup::Request =
        read_header(r, budget)?.ok_or_else(|| io::Error::other("prefix decode Begin absent"))?;
    check(request == begin_request(b), "prefix decode exact Begin")?;
    let n = request.payload_bytes()?;
    budget.charge(n)?;
    let mut payload = vec![0; n];
    r.read_exact(&mut payload)?;
    Ok((request, payload))
}
pub fn write_request(w: &mut impl Write, b: &mut FrameBudget, v: &Request) -> io::Result<()> {
    v.validate()?;
    write_header(w, b, v)?;
    w.flush()
}
pub fn read_request(r: &mut impl Read, b: &mut FrameBudget) -> io::Result<Option<Request>> {
    let v: Option<Request> = read_header(r, b)?;
    if let Some(v) = &v {
        v.validate()?;
    }
    Ok(v)
}
fn capture(c: &Completion, bytes: &[u8]) -> io::Result<()> {
    check(
        bytes.len() == OBSERVATION_BYTES
            && part(bytes) == c.observation
            && Payload::from_bytes(bytes)? == c.capture,
        "tiles main payload shape/hash",
    )?;
    check(
        bytes
            .chunks_exact(2)
            .all(|v| u16::from_le_bytes(v.try_into().unwrap()) & 0x7f80 != 0x7f80),
        "tiles main nonfinite",
    )?;
    let mut best = f32::NEG_INFINITY;
    let mut winner = 0;
    for (i, v) in bytes[37 * 8192..].chunks_exact(2).enumerate() {
        let value = f32::from_bits(u32::from(u16::from_le_bytes(v.try_into().unwrap())) << 16);
        if value > best {
            best = value;
            winner = i as u32;
        }
    }
    check(
        winner == c.output_token,
        "tiles checked lowest-index argmax",
    )
}
pub fn write_response(
    w: &mut impl Write,
    b: &mut FrameBudget,
    v: &Response,
    c: Option<&Control>,
    main: &[u8],
) -> io::Result<()> {
    let size = v.body_bytes()?;
    match &v.event {
        Event::Closed { .. } => check(c.is_none() && main.is_empty(), "tiles Close body")?,
        Event::Completed(value) => {
            let control = c.ok_or_else(|| io::Error::other("tiles control missing"))?;
            control.validate()?;
            check(
                part(&control.encode()) == value.control,
                "tiles control hash",
            )?;
            capture(value, main)?;
        }
    }
    b.charge(size)?;
    write_header(w, b, v)?;
    if let Some(c) = c {
        w.write_all(&c.encode())?;
    }
    w.write_all(main)?;
    w.flush()
}
pub fn read_response(
    r: &mut impl Read,
    b: &mut FrameBudget,
) -> io::Result<Option<(Response, Option<Control>, Vec<u8>)>> {
    let Some(value) = read_header::<Response>(r, b)? else {
        return Ok(None);
    };
    b.charge(value.body_bytes()?)?;
    if let Event::Completed(c) = &value.event {
        let mut bytes = vec![0; CONTROL_BYTES];
        r.read_exact(&mut bytes)?;
        check(part(&bytes) == c.control, "tiles control hash")?;
        let control = Control::decode(&bytes)?;
        let mut main = vec![0; OBSERVATION_BYTES];
        r.read_exact(&mut main)?;
        capture(c, &main)?;
        Ok(Some((value, Some(control), main)))
    } else {
        Ok(Some((value, None, Vec::new())))
    }
}

#[cfg(test)]
#[path = "finite_prefix_decode_wire_v1_tests.rs"]
pub(crate) mod tests;
