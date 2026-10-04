//! Additive four-forward arithmetic binding; ordinary frame bodies are unchanged.
use crate::finite_long_wire_v1::{read_header, write_header};
use crate::finite_prefix_decode_wire_v1 as base;
use crate::finite_setup_wire_v1::{self as setup, Part};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

pub use crate::finite_forward_wire_v1::part;
pub use base::{CONTROL_BYTES, FORWARDS, HEADER_BYTES, MAX_IMAGE_BYTES, PROTOCOL, STREAM_BYTES};
pub use base::{
    Chain, Command, Completion, Control, Event, FrameBudget, InputMode, LayerObservation, Profile,
    Request, Response,
};
pub use base::{read_request, read_response, write_request, write_response};

pub const SCHEMA: &str = "FerricProjectionResidualDecodeBootstrapV1";
pub const ARITHMETIC: &str = "ordered-fp32-tp2-bf16-projection-then-bf16-residual-v1";

pub fn profile_sha256(base: [u8; 32], projection: [u8; 32]) -> io::Result<[u8; 32]> {
    if base == [0; 32] || projection == [0; 32] {
        return Err(io::Error::other(
            "projection decode requires actual profile/image identities",
        ));
    }
    let mut h = Sha256::new();
    h.update(b"ferric-projection-residual-four-decode-v1\0");
    h.update(ARITHMETIC.as_bytes());
    h.update(base);
    h.update(projection);
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
        if self.schema != SCHEMA
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
        profile_sha256(self.decode.sha256()?, self.projection_residual_image.sha256)
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

#[cfg(test)]
#[path = "finite_projection_residual_decode_wire_v1_tests.rs"]
pub(crate) mod tests;
