//! Distinct one-layer protocol; captures are released only by successful Close.
use crate::finite_setup_wire_v1::{self as setup, Begin, Part};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

pub const PROTOCOL: u32 = 1;
pub const LIMIT: usize = 64 << 20;
pub const HEADER_LIMIT: usize = 65_536;
pub const CAPTURE_BYTES: usize = 9_670_656;
pub const STAGES: [(&str, usize, usize); 14] = [
    ("norm", 8192, 2),
    ("qkv", 6144, 2),
    ("query", 4096, 2),
    ("key-cache", 2359296, 2),
    ("value-cache", 2359296, 2),
    ("attention", 4096, 2),
    ("output-partial", 16384, 4),
    ("first-residual", 8192, 2),
    ("mlp-norm", 8192, 2),
    ("gate", 12288, 2),
    ("up", 12288, 2),
    ("activation", 12288, 2),
    ("down-partial", 16384, 4),
    ("final-hidden", 8192, 2),
];
fn check(ok: bool, text: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(text))
    }
}
pub fn part(bytes: &[u8]) -> Part {
    Part {
        bytes: bytes.len() as u32,
        sha256: Sha256::digest(bytes).into(),
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    Baseline22Mlp548,
    Prefix284Mlp548,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub generation: u64,
    pub token: u32,
    pub cache_metadata: Vec<u32>,
    pub rotary_bits: Vec<u32>,
}
impl Input {
    pub fn validate(&self) -> io::Result<()> {
        check(
            self.generation == 1
                && self.token < 151936
                && self.cache_metadata.len() == 145
                && self.cache_metadata[0] == 0
                && self.rotary_bits.len() == 128
                && self
                    .rotary_bits
                    .iter()
                    .all(|v| f32::from_bits(*v).is_finite()),
            "prefix layer exact first input",
        )?;
        let mut seen = [false; 144];
        for &page in &self.cache_metadata[1..] {
            let slot = seen
                .get_mut(page as usize)
                .ok_or_else(|| io::Error::other("page bound"))?;
            check(!*slot, "page alias")?;
            *slot = true;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub protocol: u32,
    pub profile: Profile,
    pub device_ids: [u64; 2],
    pub timeout_ms: u32,
    pub begin: Begin,
    pub input: Input,
    pub mlp_image: Part,
    pub prefix_image: Option<Part>,
}
impl Bootstrap {
    pub fn validate(&self) -> io::Result<()> {
        let s = &self.begin.scope;
        check(
            self.protocol == PROTOCOL
                && self.device_ids[0] != 0
                && self.device_ids[1] != 0
                && self.device_ids[0] != self.device_ids[1]
                && (1..=10000).contains(&self.timeout_ms)
                && s.bundle_id != [0; 32]
                && s.model_id != [0; 32]
                && s.session != [0; 32]
                && s.pool_identity != 0
                && s.child_identity != 0
                && self.begin.tail_image.is_some()
                && (self.profile == Profile::Prefix284Mlp548) == self.prefix_image.is_some(),
            "prefix layer bootstrap scope/profile",
        )?;
        self.input.validate()?;
        self.total_input_bytes()?;
        Ok(())
    }
    pub fn total_input_bytes(&self) -> io::Result<usize> {
        let mut total = 0usize;
        for (i, p) in self
            .begin
            .parts()
            .into_iter()
            .chain(self.begin.tail_image)
            .chain([self.mlp_image])
            .chain(self.prefix_image)
            .enumerate()
        {
            check(
                p.bytes > 0
                    && p.sha256 != [0; 32]
                    && p.bytes as usize <= if i < 3 { 4 << 20 } else { LIMIT },
                "prefix input part",
            )?;
            total = total
                .checked_add(p.bytes as usize)
                .ok_or_else(|| io::Error::other("input overflow"))?;
        }
        check(
            total <= LIMIT - 2 * HEADER_LIMIT,
            "combined bootstrap/Begin bound",
        )?;
        Ok(total)
    }
    pub fn sha256(&self) -> io::Result<[u8; 32]> {
        self.validate()?;
        let registration = self.begin.registration.sha256;
        let mut input = Sha256::new();
        input.update(b"ferric-prefix-layer-input-v6\0");
        input.update(registration);
        input.update(self.input.generation.to_le_bytes());
        input.update(self.input.token.to_le_bytes());
        for v in self
            .input
            .cache_metadata
            .iter()
            .chain(&self.input.rotary_bits)
        {
            input.update(v.to_le_bytes());
        }
        let mut h = Sha256::new();
        let s = &self.begin.scope;
        h.update(b"ferric-prefix-layer-closed-v6\0");
        h.update(s.bundle_id);
        h.update(s.model_id);
        h.update(s.session);
        h.update(s.pool_identity.to_le_bytes());
        h.update(s.group_id.to_le_bytes());
        h.update(s.child_identity.to_le_bytes());
        h.update(registration);
        h.update([u8::from(self.prefix_image.is_some())]);
        h.update(self.prefix_image.map(|p| p.sha256).unwrap_or([0; 32]));
        h.update(self.mlp_image.sha256);
        h.update(input.finalize());
        h.update(self.timeout_ms.to_le_bytes());
        for v in self.device_ids {
            h.update(v.to_le_bytes());
        }
        Ok(h.finalize().into())
    }
}
#[derive(Default)]
pub struct Budget {
    bytes: usize,
    frames: usize,
}
impl Budget {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn used(&self) -> usize {
        self.bytes
    }
    fn add(&mut self, bytes: usize) -> io::Result<()> {
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or_else(|| io::Error::other("stream overflow"))?;
        self.frames = self
            .frames
            .checked_add(1)
            .ok_or_else(|| io::Error::other("frame count overflow"))?;
        check(
            self.bytes <= LIMIT && self.frames <= 8,
            "closed finite stream bound",
        )
    }
}
fn header<T: Serialize>(value: &T) -> io::Result<Vec<u8>> {
    struct Bounded(Vec<u8>);
    impl Write for Bounded {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            check(
                self.0
                    .len()
                    .checked_add(b.len())
                    .is_some_and(|n| n <= HEADER_LIMIT),
                "header bound",
            )?;
            self.0.extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut out = Bounded(Vec::new());
    serde_json::to_writer(&mut out, value).map_err(io::Error::other)?;
    Ok(out.0)
}
pub(crate) fn write_header(
    w: &mut impl Write,
    budget: &mut Budget,
    value: &impl Serialize,
    body: usize,
) -> io::Result<()> {
    let raw = header(value)?;
    budget.add(
        4usize
            .checked_add(raw.len())
            .and_then(|v| v.checked_add(body))
            .ok_or_else(|| io::Error::other("frame overflow"))?,
    )?;
    w.write_all(&(raw.len() as u32).to_le_bytes())?;
    w.write_all(&raw)
}
pub(crate) fn read_header<T: serde::de::DeserializeOwned>(
    r: &mut impl Read,
    budget: &mut Budget,
) -> io::Result<T> {
    let mut size = [0; 4];
    r.read_exact(&mut size)?;
    let size = u32::from_le_bytes(size) as usize;
    check(size > 0 && size <= HEADER_LIMIT, "header extent")?;
    budget.add(size + 4)?;
    let mut raw = vec![0; size];
    r.read_exact(&mut raw)?;
    serde_json::from_slice(&raw).map_err(io::Error::other)
}
pub(crate) fn read_part(r: &mut impl Read, budget: &mut Budget, p: Part) -> io::Result<Vec<u8>> {
    budget.add(p.bytes as usize)?;
    let mut raw = vec![0; p.bytes as usize];
    r.read_exact(&mut raw)?;
    check(part(&raw) == p, "body digest")?;
    Ok(raw)
}
pub fn write_bootstrap(
    w: &mut impl Write,
    budget: &mut Budget,
    b: &Bootstrap,
    mlp: &[u8],
    prefix: Option<&[u8]>,
) -> io::Result<()> {
    b.validate()?;
    check(
        part(mlp) == b.mlp_image && prefix.map(part) == b.prefix_image,
        "bootstrap body pins",
    )?;
    write_header(w, budget, b, mlp.len() + prefix.map_or(0, |v| v.len()))?;
    w.write_all(mlp)?;
    if let Some(p) = prefix {
        w.write_all(p)?;
    }
    w.flush()
}
pub fn read_bootstrap(
    r: &mut impl Read,
    budget: &mut Budget,
) -> io::Result<(Bootstrap, Vec<u8>, Option<Vec<u8>>)> {
    let b: Bootstrap = read_header(r, budget)?;
    b.validate()?;
    let mlp = read_part(r, budget, b.mlp_image)?;
    let prefix = b
        .prefix_image
        .map(|p| read_part(r, budget, p))
        .transpose()?;
    Ok((b, mlp, prefix))
}
/// Compare the small Begin header before allocating its closed payload.
pub fn read_begin(
    r: &mut impl Read,
    budget: &mut Budget,
    b: &Bootstrap,
) -> io::Result<(setup::Request, Vec<u8>)> {
    let request: setup::Request = read_header(r, budget)?;
    check(
        request.protocol == setup::PROTOCOL
            && request.id == 1
            && request.device_ids == b.device_ids
            && request.session == b.begin.scope.session
            && request.command == setup::Command::Begin(b.begin.clone()),
        "exact bootstrap Begin",
    )?;
    let n = request.payload_bytes()?;
    budget.add(n)?;
    let mut payload = vec![0; n];
    r.read_exact(&mut payload)?;
    Ok((request, payload))
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Run,
    Close,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol: u32,
    pub id: u64,
    pub profile_sha256: [u8; 32],
    pub command: Command,
}
impl Request {
    pub fn validate(&self) -> io::Result<()> {
        check(
            self.protocol == PROTOCOL
                && self.profile_sha256 != [0; 32]
                && matches!(
                    (self.id, self.command),
                    (1, Command::Run) | (2, Command::Close)
                ),
            "layer request order",
        )
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Control {
    pub prefix: [Vec<u32>; 2],
    pub mlp: [Vec<u32>; 2],
    pub embedding_ns: [u64; 2],
    pub paired_ns: [[u64; 2]; 4],
}
impl Control {
    pub fn validate(&self, profile: Profile) -> io::Result<()> {
        for rank in 0..2 {
            let p = &self.prefix[rank];
            match profile {
                Profile::Baseline22Mlp548 => check(
                    p.len() == 22
                        && p[..4] == [1, 0, 65535, 65535]
                        && p[5] == 0
                        && p[6..] == [64; 16]
                        && (0..16).all(|i| matches!((p[4] >> (i * 2)) & 3, 1 | 2)),
                    "baseline22 terminal",
                )?,
                Profile::Prefix284Mlp548 => check(
                    p.len() == 284
                        && p[..4] == [1, 0, 0, 31]
                        && p[4..9] == [1, 48, 1, 16, 64]
                        && p[9..14] == [1, 48, 1, 16, 64]
                        && p[14..19] == [u32::MAX, u32::MAX, u32::MAX, u32::MAX, 3]
                        && p[19..24] == [u32::MAX, u32::MAX, u32::MAX, u32::MAX, 3]
                        && p[24..154].iter().all(|v| (1..=64).contains(v))
                        && p[154..].iter().all(|v| *v == 64),
                    "prefix284 terminal",
                )?,
            }
            let words: &[u32; 548] = self.mlp[rank]
                .as_slice()
                .try_into()
                .map_err(|_| io::Error::other("MLP548 extent"))?;
            crate::finite_mlp_tiles_comparison_wire_v1::validate_tiles_state(words)?;
        }
        Ok(())
    }
}
pub fn capture_rows(bytes: &[u8]) -> io::Result<Vec<&[u8]>> {
    check(bytes.len() == CAPTURE_BYTES, "exact layer capture extent")?;
    let mut rows = Vec::with_capacity(28);
    let mut offset = 0;
    for _ in 0..2 {
        for (_, n, width) in STAGES {
            let row = &bytes[offset..offset + n];
            check(
                row.chunks_exact(width).all(|v| {
                    if width == 2 {
                        u16::from_le_bytes(v.try_into().unwrap()) & 0x7f80 != 0x7f80
                    } else {
                        u32::from_le_bytes(v.try_into().unwrap()) & 0x7f800000 != 0x7f800000
                    }
                }),
                "finite stage capture",
            )?;
            rows.push(row);
            offset += n;
        }
    }
    check(offset == bytes.len(), "capture partition")?;
    Ok(rows)
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub protocol: u32,
    pub id: u64,
    pub profile_sha256: [u8; 32],
    pub profile: Profile,
    pub native_closed: bool,
    pub completed_layers: u32,
    pub control: Option<Control>,
    pub capture: Option<Part>,
    pub gpu_execution: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
}
impl Response {
    pub fn validate(&self) -> io::Result<()> {
        check(
            self.protocol == PROTOCOL
                && self.profile_sha256 != [0; 32]
                && self.completed_layers == 1
                && self.gpu_execution
                && !self.numerical_acceptance
                && !self.performance_claim
                && !self.production_authority,
            "layer response scope",
        )?;
        match self.id {
            1 => check(
                !self.native_closed && self.control.is_none() && self.capture.is_none(),
                "run retains capture",
            ),
            2 => {
                check(
                    self.native_closed
                        && self.capture.is_some_and(|p| {
                            p.bytes as usize == CAPTURE_BYTES && p.sha256 != [0; 32]
                        }),
                    "closed capture",
                )?;
                self.control
                    .as_ref()
                    .ok_or_else(|| io::Error::other("closed control missing"))?
                    .validate(self.profile)
            }
            _ => Err(io::Error::other("layer response id")),
        }
    }
}
pub fn write_request(w: &mut impl Write, budget: &mut Budget, v: &Request) -> io::Result<()> {
    v.validate()?;
    write_header(w, budget, v, 0)?;
    w.flush()
}
pub fn read_request(r: &mut impl Read, budget: &mut Budget) -> io::Result<Request> {
    let v: Request = read_header(r, budget)?;
    v.validate()?;
    Ok(v)
}
pub fn write_response(
    w: &mut impl Write,
    budget: &mut Budget,
    v: &Response,
    body: &[u8],
) -> io::Result<()> {
    v.validate()?;
    if v.native_closed {
        capture_rows(body)?;
        check(v.capture == Some(part(body)), "closed capture digest")?;
    } else {
        check(body.is_empty(), "no pre-close payload")?;
    }
    write_header(w, budget, v, body.len())?;
    w.write_all(body)?;
    w.flush()
}
pub fn read_response(r: &mut impl Read, budget: &mut Budget) -> io::Result<(Response, Vec<u8>)> {
    let v: Response = read_header(r, budget)?;
    v.validate()?;
    let body = v
        .capture
        .map(|p| read_part(r, budget, p))
        .transpose()?
        .unwrap_or_default();
    if v.native_closed {
        capture_rows(&body)?;
    }
    Ok((v, body))
}

#[cfg(test)]
#[path = "finite_prefix_layer_wire_v1_tests.rs"]
pub(crate) mod tests;
