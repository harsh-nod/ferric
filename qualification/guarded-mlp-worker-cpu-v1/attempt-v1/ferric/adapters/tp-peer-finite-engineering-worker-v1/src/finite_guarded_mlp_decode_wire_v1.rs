//! Bounded Prefix284 + combined2208 TF4/AR4 transport, distinct from legacy routes.
use crate::finite_forward_wire_v1::{OBSERVATION_BYTES, Payload, part};
pub use crate::finite_long_wire_v1::FrameBudget;
use crate::finite_long_wire_v1::{read_header, write_header};
use crate::finite_prefix_decode_wire_v1 as base;
pub use crate::finite_prefix_decode_wire_v1::{
    Command, InputMode, Request, read_request, write_request,
};
use crate::finite_setup_wire_v1::{self as setup, Part, Scope};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

pub const PROTOCOL: u32 = 1;
pub const FORWARDS: u32 = 4;
pub const CONTROL_BYTES: usize = 242_824;
pub const MAX_IMAGE_BYTES: usize = base::MAX_IMAGE_BYTES;
pub const SCHEMA: &str = "FerricGuardedMlpDecodeBootstrapV1";
pub const RESPONSE_SCHEMA: &str = "FerricGuardedMlpDecodeObservationV1";
pub const GUARDED_IMAGE: [u8; 32] = [
    0xde, 0x88, 0x0d, 0xce, 0xbf, 0x79, 0x42, 0x5c, 0xcb, 0x55, 0x5c, 0x1a, 0x2b, 0xdc, 0xa3, 0xff,
    0x78, 0xb9, 0x26, 0xf7, 0xb2, 0x06, 0x2d, 0x06, 0xb9, 0x18, 0x76, 0x3d, 0xa4, 0xde, 0x0f, 0x66,
];

/// Pure shared encoding; callers separately enforce the closed input contract.
pub(crate) fn profile_sha256(
    scope: &Scope,
    registration: [u8; 32],
    prefix: [u8; 32],
    mlp: [u8; 32],
    projection: [u8; 32],
    autoregressive: bool,
    tokens: [u32; 4],
    timeout_ms: u32,
    devices: [u64; 2],
) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"ferric-prefix284-combined2208-four-decode-v1\0");
    for value in [
        scope.bundle_id,
        scope.model_id,
        scope.session,
        registration,
        prefix,
        mlp,
        projection,
        GUARDED_IMAGE,
    ] {
        h.update(value);
    }
    h.update(scope.pool_identity.to_le_bytes());
    h.update(scope.group_id.to_le_bytes());
    h.update(scope.child_identity.to_le_bytes());
    h.update(timeout_ms.to_le_bytes());
    h.update([u8::from(autoregressive)]);
    for device in devices {
        h.update(device.to_le_bytes());
    }
    for token in tokens {
        h.update(token.to_le_bytes());
    }
    h.finalize().into()
}

fn check(ok: bool, error: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(error))
    }
}

/// The nested recipe authenticates the unchanged setup inputs, not a legacy
/// execution profile. Only this outer schema selects the guarded route.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub schema: String,
    pub decode: base::Bootstrap,
    pub projection_image: Part,
    pub guarded_image: Part,
}
impl Bootstrap {
    pub fn validate(
        &self,
        ids: [u64; 2],
        timeout: u32,
        pid: u32,
        mode: InputMode,
    ) -> io::Result<()> {
        self.decode.validate(ids, timeout, pid, mode)?;
        check(self.schema == SCHEMA, "guarded bootstrap schema")?;
        for p in [self.projection_image, self.guarded_image] {
            check(
                p.bytes > 0 && p.bytes as usize <= MAX_IMAGE_BYTES && p.sha256 != [0; 32],
                "guarded bootstrap image",
            )?;
        }
        check(
            self.guarded_image.sha256 == GUARDED_IMAGE,
            "guarded exact candidate image",
        )?;
        self.total_input_bytes().map(|_| ())
    }
    pub fn total_input_bytes(&self) -> io::Result<usize> {
        let total = self
            .decode
            .total_input_bytes()?
            .checked_add(self.projection_image.bytes as usize)
            .and_then(|n| n.checked_add(self.guarded_image.bytes as usize))
            .ok_or_else(|| io::Error::other("guarded combined input overflow"))?;
        check(
            total <= base::STREAM_BYTES - 7 * (base::HEADER_BYTES + 4),
            "guarded combined input bound",
        )?;
        Ok(total)
    }
    pub fn sha256(&self) -> io::Result<[u8; 32]> {
        self.validate(
            self.decode.device_ids,
            self.decode.timeout_ms,
            self.decode.scope.child_identity,
            self.decode.mode,
        )?;
        let tokens = match self.decode.mode {
            InputMode::TeacherForced => self
                .decode
                .input_tokens
                .as_slice()
                .try_into()
                .map_err(io::Error::other)?,
            InputMode::Autoregressive => [self.decode.input_tokens[0], 0, 0, 0],
        };
        Ok(profile_sha256(
            &self.decode.scope,
            self.decode.registration,
            self.decode.prefix_image.sha256,
            self.decode.tiles_image.sha256,
            self.projection_image.sha256,
            self.decode.mode == InputMode::Autoregressive,
            tokens,
            self.decode.timeout_ms,
            self.decode.device_ids,
        ))
    }
    pub fn input(&self, position: u32, previous: Option<u32>) -> io::Result<u32> {
        self.decode.input(position, previous)
    }
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
            && !ids.contains(&0)
            && ids[0] != ids[1]
            && session != [0; 32]
            && registration != [0; 32]
            && profile != [0; 32],
        "guarded envelope",
    )
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayerObservation {
    pub prefix_states: [[u32; 284]; 2],
    pub mlp_prefixes: [[u32; 548]; 2],
    pub guards: [[u32; 4]; 2],
    pub prefix_host_ns: [u64; 2],
    pub segment_host_ns: u64,
    pub observed_queue_frontiers: [(u64, u64); 2],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Control {
    pub embedding_ns: [u64; 2],
    pub layers: [LayerObservation; 36],
    pub tail_ns: [u64; 3],
}
impl Control {
    pub fn validate(&self, generation: u64) -> io::Result<()> {
        check((1..=4).contains(&generation), "guarded global generation")?;
        let local = ((generation - 1) / 2 + 1) as u32;
        let mut previous = [(0, 0); 2];
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
                    "guarded prefix terminal",
                )?;
                crate::finite_mlp_tiles_comparison_wire_v1::validate_tiles_state(
                    &layer.mlp_prefixes[rank],
                )?;
                check(
                    layer.guards[rank] == [local, 0, 1, 0],
                    "guarded current local generation",
                )?;
                let (write, read) = layer.observed_queue_frontiers[rank];
                check(
                    read <= write && write > previous[rank].0 && read >= previous[rank].1,
                    "guarded queue frontier ordering",
                )?;
                previous[rank] = (write, read);
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
            for words in &layer.mlp_prefixes {
                for v in words {
                    bytes.extend_from_slice(&v.to_le_bytes());
                }
            }
            for words in &layer.guards {
                for v in words {
                    bytes.extend_from_slice(&v.to_le_bytes());
                }
            }
            for v in layer.prefix_host_ns {
                bytes.extend_from_slice(&v.to_le_bytes());
            }
            bytes.extend_from_slice(&layer.segment_host_ns.to_le_bytes());
            for (write, read) in layer.observed_queue_frontiers {
                bytes.extend_from_slice(&write.to_le_bytes());
                bytes.extend_from_slice(&read.to_le_bytes());
            }
        }
        for v in self.tail_ns {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes
    }
    pub fn decode(bytes: &[u8], generation: u64) -> io::Result<Self> {
        check(bytes.len() == CONTROL_BYTES, "guarded control exact extent")?;
        fn word<const N: usize>(r: &mut io::Cursor<&[u8]>) -> io::Result<[u8; N]> {
            let mut out = [0; N];
            r.read_exact(&mut out)?;
            Ok(out)
        }
        let mut r = io::Cursor::new(bytes);
        let mut out = Self {
            embedding_ns: [0; 2],
            layers: core::array::from_fn(|_| LayerObservation {
                prefix_states: [[0; 284]; 2],
                mlp_prefixes: [[0; 548]; 2],
                guards: [[0; 4]; 2],
                prefix_host_ns: [0; 2],
                segment_host_ns: 0,
                observed_queue_frontiers: [(0, 0); 2],
            }),
            tail_ns: [0; 3],
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
        out.validate(generation)?;
        Ok(out)
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
    pub schema: String,
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
    pub full_model_acceptance: bool,
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
            self.schema == RESPONSE_SCHEMA
                && self.gpu_execution
                && !self.full_model_acceptance
                && !self.numerical_acceptance
                && !self.performance_claim
                && !self.production_authority,
            "guarded diagnostic claim boundary",
        )?;
        match &self.event {
            Event::Closed {
                completed_forwards, ..
            } => {
                check(
                    self.native_closed && self.id == 5 && *completed_forwards == 4,
                    "guarded Close identity",
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
                    "guarded completion shape",
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
        h.update(b"ferric-prefix284-combined2208-four-transcript-v1\0");
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
        "guarded supplied image body/hash",
    )
}
pub fn write_bootstrap(
    w: &mut impl Write,
    budget: &mut FrameBudget,
    b: &Bootstrap,
    images: [&[u8]; 4],
) -> io::Result<()> {
    b.validate(
        b.decode.device_ids,
        b.decode.timeout_ms,
        b.decode.scope.child_identity,
        b.decode.mode,
    )?;
    let parts = [
        b.decode.tiles_image,
        b.decode.prefix_image,
        b.projection_image,
        b.guarded_image,
    ];
    let mut total = 0usize;
    for (p, bytes) in parts.into_iter().zip(images) {
        image(p, bytes)?;
        total += bytes.len();
    }
    budget.charge(total)?;
    write_header(w, budget, b)?;
    for bytes in images {
        w.write_all(bytes)?;
    }
    w.flush()
}
pub fn read_bootstrap(
    r: &mut impl Read,
    budget: &mut FrameBudget,
) -> io::Result<Option<(Bootstrap, [Vec<u8>; 4])>> {
    let Some(b) = read_header::<Bootstrap>(r, budget)? else {
        return Ok(None);
    };
    b.validate(
        b.decode.device_ids,
        b.decode.timeout_ms,
        b.decode.scope.child_identity,
        b.decode.mode,
    )?;
    let parts = [
        b.decode.tiles_image,
        b.decode.prefix_image,
        b.projection_image,
        b.guarded_image,
    ];
    let mut images = core::array::from_fn(|_| Vec::new());
    for (p, bytes) in parts.into_iter().zip(images.iter_mut()) {
        budget.charge(p.bytes as usize)?;
        *bytes = vec![0; p.bytes as usize];
        r.read_exact(bytes)?;
        image(p, bytes)?;
    }
    Ok(Some((b, images)))
}
pub fn account_begin(budget: &mut FrameBudget, b: &Bootstrap) -> io::Result<()> {
    base::account_begin(budget, &b.decode)
}
pub fn read_begin(
    r: &mut impl Read,
    budget: &mut FrameBudget,
    b: &Bootstrap,
) -> io::Result<(setup::Request, Vec<u8>)> {
    base::read_begin(r, budget, &b.decode)
}
fn capture(c: &Completion, bytes: &[u8]) -> io::Result<()> {
    check(
        bytes.len() == OBSERVATION_BYTES
            && part(bytes) == c.observation
            && Payload::from_bytes(bytes)? == c.capture,
        "guarded main payload shape/hash",
    )?;
    check(
        bytes
            .chunks_exact(2)
            .all(|v| u16::from_le_bytes(v.try_into().unwrap()) & 0x7f80 != 0x7f80),
        "guarded main nonfinite",
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
        "guarded checked lowest-index argmax",
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
        Event::Closed { .. } => check(c.is_none() && main.is_empty(), "guarded Close body")?,
        Event::Completed(value) => {
            let control = c.ok_or_else(|| io::Error::other("guarded control missing"))?;
            control.validate(value.generation)?;
            check(
                part(&control.encode()) == value.control,
                "guarded control hash",
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
        check(part(&bytes) == c.control, "guarded control hash")?;
        let control = Control::decode(&bytes, c.generation)?;
        let mut main = vec![0; OBSERVATION_BYTES];
        r.read_exact(&mut main)?;
        capture(c, &main)?;
        Ok(Some((value, Some(control), main)))
    } else {
        Ok(Some((value, None, Vec::new())))
    }
}

#[cfg(test)]
#[path = "finite_guarded_mlp_decode_wire_v1_tests.rs"]
pub(crate) mod tests;
