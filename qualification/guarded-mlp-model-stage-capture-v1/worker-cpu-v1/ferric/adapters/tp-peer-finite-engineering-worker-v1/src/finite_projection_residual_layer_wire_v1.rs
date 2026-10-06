//! Separately bound layer0 arithmetic candidate; the original Begin remains unchanged.
use crate::finite_prefix_layer_wire_v1 as base;
use crate::finite_setup_wire_v1::Part;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

pub use base::{Budget, Command, Control, Request, Response};
pub use base::{CAPTURE_BYTES, HEADER_LIMIT, LIMIT, PROTOCOL, STAGES};
pub use base::{capture_rows, part, read_request, read_response, write_request, write_response};

pub const SCHEMA: &str = "FerricProjectionResidualLayerBootstrapV1";
pub const ARITHMETIC: &str = "ordered-fp32-tp2-bf16-projection-then-bf16-residual-v1";

pub fn profile_sha256(base: [u8; 32], projection: [u8; 32]) -> io::Result<[u8; 32]> {
    if base == [0; 32] || projection == [0; 32] {
        return Err(io::Error::other(
            "candidate profile requires both actual image/profile identities",
        ));
    }
    let mut hash = Sha256::new();
    hash.update(b"ferric-projection-residual-layer-closed-v1\0");
    hash.update(ARITHMETIC.as_bytes());
    hash.update(base);
    hash.update(projection);
    Ok(hash.finalize().into())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub schema: String,
    pub layer: base::Bootstrap,
    pub projection_residual_image: Part,
}
impl Bootstrap {
    pub fn validate(&self) -> io::Result<()> {
        self.layer.validate()?;
        let image = self.projection_residual_image;
        if self.schema != SCHEMA
            || self.layer.profile != base::Profile::Prefix284Mlp548
            || image.bytes == 0
            || image.bytes as usize > 32 << 20
            || image.sha256 == [0; 32]
            || !self
                .layer
                .total_input_bytes()?
                .checked_add(image.bytes as usize)
                .is_some_and(|n| n <= LIMIT - 2 * HEADER_LIMIT)
        {
            return Err(io::Error::other(
                "closed projection-residual layer bootstrap",
            ));
        }
        Ok(())
    }
    pub fn sha256(&self) -> io::Result<[u8; 32]> {
        self.validate()?;
        profile_sha256(self.layer.sha256()?, self.projection_residual_image.sha256)
    }
}

pub fn write_bootstrap(
    w: &mut impl Write,
    budget: &mut Budget,
    b: &Bootstrap,
    mlp: &[u8],
    prefix: &[u8],
    projection: &[u8],
) -> io::Result<()> {
    b.validate()?;
    if part(mlp) != b.layer.mlp_image
        || Some(part(prefix)) != b.layer.prefix_image
        || part(projection) != b.projection_residual_image
    {
        return Err(io::Error::other("candidate bootstrap actual image bodies"));
    }
    base::write_header(w, budget, b, mlp.len() + prefix.len() + projection.len())?;
    w.write_all(mlp)?;
    w.write_all(prefix)?;
    w.write_all(projection)?;
    w.flush()
}

pub fn read_bootstrap(
    r: &mut impl Read,
    budget: &mut Budget,
) -> io::Result<(Bootstrap, Vec<u8>, Vec<u8>, Vec<u8>)> {
    let b: Bootstrap = base::read_header(r, budget)?;
    b.validate()?;
    let mlp = base::read_part(r, budget, b.layer.mlp_image)?;
    let prefix = base::read_part(
        r,
        budget,
        b.layer
            .prefix_image
            .ok_or_else(|| io::Error::other("candidate prefix"))?,
    )?;
    let projection = base::read_part(r, budget, b.projection_residual_image)?;
    Ok((b, mlp, prefix, projection))
}

pub fn read_begin(
    r: &mut impl Read,
    budget: &mut Budget,
    b: &Bootstrap,
) -> io::Result<(crate::finite_setup_wire_v1::Request, Vec<u8>)> {
    b.validate()?;
    base::read_begin(r, budget, &b.layer)
}

#[cfg(test)]
#[path = "finite_projection_residual_layer_wire_v1_tests.rs"]
mod tests;
