//! Explicit shared-full ordered residual/MLP route; no legacy control layout.
use crate::finite_long_wire_v1::{read_header, write_header};
use crate::finite_prefix_decode_wire_v1 as base;
use crate::finite_setup_wire_v1::{self as setup, Part};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

pub use crate::finite_forward_wire_v1::part;
use crate::finite_forward_wire_v1::{OBSERVATION_BYTES, Payload};
pub use base::{
    Chain, Command, Completion, Event, FrameBudget, InputMode, Profile, Request, Response,
};
pub use base::{FORWARDS, HEADER_BYTES, MAX_IMAGE_BYTES, PROTOCOL, STREAM_BYTES};
pub use base::{read_request, write_request};
pub const CONTROL_BYTES: usize = 241_096;

pub const SCHEMA: &str = "FerricProjectionResidualMlpOrderedBootstrapV1";
pub const ARITHMETIC: &str = "ordered-fp32-tp2-bf16-projection-then-bf16-residual-v1";

pub fn profile_sha256(base: [u8; 32]) -> io::Result<[u8; 32]> {
    if base == [0; 32] {
        return Err(io::Error::other("ordered profile absent"));
    }
    let mut h = Sha256::new();
    h.update(b"ferric-projection-residual-mlp-ordered-ar4-shared-v1\0");
    h.update(base);
    Ok(h.finalize().into())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub schema: String,
    pub decode: base::Bootstrap,
    pub projection_residual_image: Part,
}
impl Bootstrap {
    pub fn validate(
        &self,
        devices: [u64; 2],
        timeout: u32,
        pid: u32,
        mode: InputMode,
    ) -> io::Result<()> {
        self.decode.validate(devices, timeout, pid, mode)?;
        let p = self.projection_residual_image;
        if mode != InputMode::Autoregressive
            || self.schema != SCHEMA
            || p.bytes == 0
            || p.bytes as usize > MAX_IMAGE_BYTES
            || p.sha256 == [0; 32]
        {
            return Err(io::Error::other(
                "separate authenticated projection bootstrap",
            ));
        }
        self.total_input_bytes().map(|_| ())
    }
    pub fn total_input_bytes(&self) -> io::Result<usize> {
        self.decode
            .total_input_bytes()?
            .checked_add(self.projection_residual_image.bytes as usize)
            .filter(|n| *n <= STREAM_BYTES - 7 * (HEADER_BYTES + 4))
            .ok_or_else(|| io::Error::other("projection decode combined input bound"))
    }
    pub fn sha256(&self) -> io::Result<[u8; 32]> {
        self.validate(
            self.decode.device_ids,
            self.decode.timeout_ms,
            self.decode.scope.child_identity,
            self.decode.mode,
        )?;
        profile_sha256(
            crate::finite_projection_residual_decode_wire_v1::profile_sha256(
                self.decode.sha256()?,
                self.projection_residual_image.sha256,
            )?,
        )
    }
    pub fn input(&self, position: u32, previous: Option<u32>) -> io::Result<u32> {
        self.decode.input(position, previous)
    }
}
fn image(expected: Part, bytes: &[u8]) -> io::Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES || part(bytes) != expected {
        return Err(io::Error::other("projection decode image body/hash"));
    }
    Ok(())
}
pub fn write_bootstrap(
    w: &mut impl Write,
    budget: &mut FrameBudget,
    b: &Bootstrap,
    mlp: &[u8],
    prefix: &[u8],
    projection: &[u8],
) -> io::Result<()> {
    b.validate(
        b.decode.device_ids,
        b.decode.timeout_ms,
        b.decode.scope.child_identity,
        b.decode.mode,
    )?;
    for (pin, body) in [
        (b.decode.tiles_image, mlp),
        (b.decode.prefix_image, prefix),
        (b.projection_residual_image, projection),
    ] {
        image(pin, body)?;
    }
    budget.charge(mlp.len() + prefix.len() + projection.len())?;
    write_header(w, budget, b)?;
    w.write_all(mlp)?;
    w.write_all(prefix)?;
    w.write_all(projection)?;
    w.flush()
}
pub fn read_bootstrap(
    r: &mut impl Read,
    budget: &mut FrameBudget,
) -> io::Result<Option<(Bootstrap, Vec<u8>, Vec<u8>, Vec<u8>)>> {
    let Some(b) = read_header::<Bootstrap>(r, budget)? else {
        return Ok(None);
    };
    b.validate(
        b.decode.device_ids,
        b.decode.timeout_ms,
        b.decode.scope.child_identity,
        b.decode.mode,
    )?;
    let pins = [
        b.decode.tiles_image,
        b.decode.prefix_image,
        b.projection_residual_image,
    ];
    budget.charge(pins.iter().map(|p| p.bytes as usize).sum())?;
    let mut bodies = Vec::with_capacity(3);
    for pin in pins {
        let mut body = vec![0; pin.bytes as usize];
        r.read_exact(&mut body)?;
        image(pin, &body)?;
        bodies.push(body);
    }
    let [mlp, prefix, projection]: [Vec<u8>; 3] = bodies
        .try_into()
        .map_err(|_| io::Error::other("projection image roster"))?;
    Ok(Some((b, mlp, prefix, projection)))
}
pub fn account_begin(budget: &mut FrameBudget, b: &Bootstrap) -> io::Result<()> {
    b.sha256()?;
    base::account_begin(budget, &b.decode)
}
pub fn read_begin(
    r: &mut impl Read,
    budget: &mut FrameBudget,
    b: &Bootstrap,
) -> io::Result<(setup::Request, Vec<u8>)> {
    b.sha256()?;
    base::read_begin(r, budget, &b.decode)
}

fn check(ok: bool, message: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(message.to_owned()))
    }
}
fn devices(ids: [u64; 2]) -> bool {
    ids[0] != 0 && ids[1] != 0 && ids[0] != ids[1]
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayerObservation {
    pub prefix_states: [[u32; 284]; 2],
    pub tiles_states: [[u32; 548]; 2],
    pub prefix_ns: [u64; 2],
    pub segment_host_ns: u64,
    pub final_residual_ns: [u64; 2],
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
            for v in layer
                .prefix_ns
                .into_iter()
                .chain([layer.segment_host_ns])
                .chain(layer.final_residual_ns)
            {
                bytes.extend_from_slice(&v.to_le_bytes());
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
                prefix_ns: [0; 2],
                segment_host_ns: 0,
                final_residual_ns: [0; 2],
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
            for v in &mut layer.prefix_ns {
                *v = u64::from_le_bytes(word(&mut r)?);
            }
            layer.segment_host_ns = u64::from_le_bytes(word(&mut r)?);
            for v in &mut layer.final_residual_ns {
                *v = u64::from_le_bytes(word(&mut r)?);
            }
        }
        for v in &mut result.tail_ns {
            *v = u64::from_le_bytes(word(&mut r)?);
        }
        result.validate()?;
        Ok(result)
    }
}
fn body_bytes(value: &Response) -> io::Result<usize> {
    envelope(
        value.protocol,
        value.id,
        value.device_ids,
        value.session,
        value.registration,
        value.profile_sha256,
    )?;
    check(
        value.gpu_execution
            && !value.numerical_acceptance
            && !value.performance_claim
            && !value.production_authority,
        "tiles diagnostic claim boundary",
    )?;
    match &value.event {
        Event::Closed {
            completed_forwards, ..
        } => {
            check(
                value.native_closed && value.id == 5 && *completed_forwards == 4,
                "tiles Close identity",
            )?;
            Ok(0)
        }
        Event::Completed(c) => {
            check(
                !value.native_closed
                    && value.id <= 4
                    && c.generation == value.id
                    && c.position == value.id as u32 - 1
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
    let size = body_bytes(v)?;
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
    b.charge(body_bytes(&value)?)?;
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
#[path = "finite_projection_residual_mlp_ordered_wire_v1_tests.rs"]
pub(crate) mod tests;
