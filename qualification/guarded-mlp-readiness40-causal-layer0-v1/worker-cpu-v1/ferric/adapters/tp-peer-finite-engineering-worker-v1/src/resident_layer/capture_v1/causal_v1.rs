//! Six genuine layer-zero prefixes. No additional dispatches or arithmetic.
use super::*;
use crate::finite_guarded_mlp_readiness_wire_v1::Bootstrap;

pub(crate) const POSITIONS: usize = 6;
pub(crate) const TOTAL_BYTES: usize = 1_598_256;
pub(crate) const META_BYTES: usize = 128 << 10;
pub(crate) const MAGIC: &[u8; 8] = b"FCAP061\0";

#[derive(Debug, Serialize)]
pub(crate) struct Snapshot {
    pub(crate) generation: u64,
    pub(crate) position: u32,
    layer: u32,
    parts: Vec<Part>,
    payload_bytes: u32,
    payload_sha256: [u8; 32],
    #[serde(skip)]
    payload: Vec<u8>,
}
impl Snapshot {
    fn bytes(&self, role: Role, rank: u32) -> Result<&[u8]> {
        let part = self
            .parts
            .iter()
            .find(|p| p.role == role && p.rank == rank)
            .ok_or("causal capture part missing")?;
        self.payload
            .get(part.offset as usize..(part.offset + part.bytes) as usize)
            .ok_or_else(|| "causal capture part extent".into())
    }
    pub(crate) fn joins_hidden(&self, hidden: &[u8]) -> Result<()> {
        if hidden.len() != 8192
            || self.bytes(Role::FinalHidden, 0)? != hidden
            || self.bytes(Role::FinalHidden, 1)? != hidden
        {
            return Err("causal capture must join the actual checked layer-zero output".into());
        }
        Ok(())
    }
}

pub(crate) struct Collector {
    position: u32,
    next: usize,
    terminal: bool,
    page_offsets: [Option<u64>; 2],
    parts: Vec<Part>,
    payload: Vec<u8>,
}
impl Collector {
    pub(crate) fn new(generation: u64, position: u32, layer: usize) -> Result<Self> {
        if position >= POSITIONS as u32 || generation != u64::from(position) + 1 || layer != 0 {
            return Err("causal capture exact generation/position0..5/layer0".into());
        }
        Ok(Self {
            position,
            next: 0,
            terminal: false,
            page_offsets: [None; 2],
            parts: Vec::with_capacity(PARTS),
            payload: Vec::with_capacity(PAYLOAD_BYTES + position as usize * 4096),
        })
    }
    fn page_offset(&self, metadata: &[u8]) -> Result<u64> {
        if metadata.len() != 580 || metadata[..4] != self.position.to_le_bytes() {
            return Err("causal capture actual metadata position".into());
        }
        // All six positions are in logical page zero. Validate the complete
        // permutation, then use its actual physical page, never logical zero.
        let mut copy = metadata.to_vec();
        copy[..4].fill(0);
        cache_offset(&copy)
    }
    fn observe<B: Allocation>(
        &mut self,
        boundary: Boundary,
        reader: &mut impl Reader<B>,
        roots: &LayerBindings<B>,
    ) -> Result<()> {
        let result = (|| {
            if self.terminal || BOUNDARIES.get(self.next) != Some(&boundary) {
                return Err("causal capture boundary order or terminal collector".into());
            }
            roots.validate()?;
            for rank in 0..2 {
                for spec in specs(boundary) {
                    let root = match spec.root {
                        Root::Prefix(i) => roots.prefix[rank][i],
                        Root::Mlp(i) => roots.mlp[rank][i],
                        Root::Final => roots.final_hidden[rank],
                    };
                    let cache = matches!(spec.role, Role::CurrentKey | Role::CurrentValue);
                    let count = if cache {
                        1024 * (self.position + 1)
                    } else {
                        spec.bytes
                    };
                    let offset = if cache {
                        self.page_offsets[rank].ok_or("causal metadata missing")?
                    } else {
                        0
                    };
                    if offset
                        .checked_add(u64::from(count))
                        .is_none_or(|n| n > root.bytes())
                    {
                        return Err("causal capture source extent".into());
                    }
                    let raw = reader.read(root, offset, count)?;
                    if raw.len() != count as usize
                        || self.parts.len() >= PARTS
                        || self
                            .payload
                            .len()
                            .checked_add(raw.len())
                            .is_none_or(|n| n > PAYLOAD_BYTES + self.position as usize * 4096)
                    {
                        return Err("causal capture exact bounded read".into());
                    }
                    finite(spec.scalar, &raw)?;
                    if spec.role == Role::CacheMetadata {
                        self.page_offsets[rank] = Some(self.page_offset(&raw)?);
                    }
                    let role = match spec.role {
                        Role::CurrentKey => Role::UsedKey,
                        Role::CurrentValue => Role::UsedValue,
                        other => other,
                    };
                    self.parts.push(Part {
                        boundary,
                        rank: rank as u32,
                        role,
                        scalar: spec.scalar,
                        elements: count / spec.scalar.bytes(),
                        offset: self.payload.len() as u32,
                        bytes: count,
                        source_byte_offset: offset,
                        sha256: Sha256::digest(&raw).into(),
                    });
                    self.payload.extend_from_slice(&raw);
                }
            }
            self.next += 1;
            Ok(())
        })();
        if result.is_err() {
            self.terminal = true;
        }
        result
    }
    pub(crate) fn observe_guarded<B: Allocation>(
        &mut self,
        boundary: Boundary,
        reader: &mut impl Reader<B>,
        roots: &LayerBindings<B>,
        down: [B; 2],
    ) -> Result<()> {
        let result = (|| {
            super::super::guarded_mlp_decode_v1::selected_mlp(roots, down)?;
            if boundary == Boundary::AfterMlp {
                let mut selected = GuardedDownReader {
                    reader,
                    old: [roots.mlp[0][9], roots.mlp[1][9]],
                    actual: down,
                };
                self.observe(boundary, &mut selected, roots)
            } else {
                self.observe(boundary, reader, roots)
            }
        })();
        if result.is_err() {
            self.terminal = true;
        }
        result
    }
    pub(crate) fn finish(self) -> Result<Snapshot> {
        if self.terminal
            || self.next != BOUNDARIES.len()
            || self.parts.len() != PARTS
            || self.payload.len() != PAYLOAD_BYTES + self.position as usize * 4096
        {
            return Err("causal capture incomplete or terminal".into());
        }
        Ok(Snapshot {
            generation: u64::from(self.position) + 1,
            position: self.position,
            layer: 0,
            payload_bytes: self.payload.len() as u32,
            payload_sha256: Sha256::digest(&self.payload).into(),
            parts: self.parts,
            payload: self.payload,
        })
    }
}

pub(crate) struct Series {
    values: Vec<Snapshot>,
    terminal: bool,
}
impl Series {
    pub(crate) fn new() -> Self {
        Self {
            values: Vec::with_capacity(POSITIONS),
            terminal: false,
        }
    }
    pub(crate) fn push(&mut self, value: Snapshot, hidden: &[u8]) -> Result<()> {
        let result = (|| {
            if self.terminal
                || self.values.len() >= POSITIONS
                || value.position as usize != self.values.len()
            {
                return Err("causal capture exact six-position append order".into());
            }
            value.joins_hidden(hidden)?;
            if value.bytes(Role::CacheMetadata, 0)? != value.bytes(Role::CacheMetadata, 1)? {
                return Err("causal capture rank metadata mismatch".into());
            }
            if let Some(previous) = self.values.last() {
                for rank in 0..2 {
                    for role in [Role::UsedKey, Role::UsedValue] {
                        if !value
                            .bytes(role, rank)?
                            .starts_with(previous.bytes(role, rank)?)
                        {
                            return Err(
                                "causal capture changed a previously completed KV row".into()
                            );
                        }
                    }
                    if value.bytes(Role::CacheMetadata, rank)?[4..]
                        != previous.bytes(Role::CacheMetadata, rank)?[4..]
                    {
                        return Err("causal capture changed the physical page permutation".into());
                    }
                }
            }
            self.values.push(value);
            Ok(())
        })();
        if result.is_err() {
            self.terminal = true;
        }
        result
    }
    pub(crate) fn encode_after_close(
        self,
        bootstrap: &Bootstrap,
        transcript_sha256: [u8; 32],
        close: impl FnOnce() -> Result<()>,
    ) -> Result<Vec<u8>> {
        if self.terminal
            || self.values.len() != POSITIONS
            || self.values.iter().map(|v| v.payload.len()).sum::<usize>() != TOTAL_BYTES
            || bootstrap.sequence.profile
                != crate::finite_guarded_mlp_long_wire_v2::Profile::Readiness40Position5
        {
            return Err("causal capture incomplete series before Close".into());
        }
        close()?;
        #[derive(Serialize)]
        struct Header<'a> {
            schema: &'static str,
            bootstrap: &'a Bootstrap,
            transcript_sha256: [u8; 32],
            captures: &'a [Snapshot],
            payload_bytes: usize,
            payload_sha256: [u8; 32],
            cache_layout: &'static str,
            native_close_confirmed: bool,
            numerical_acceptance: bool,
            performance_claim: bool,
            production_authority: bool,
        }
        let mut payload = Vec::with_capacity(TOTAL_BYTES);
        for value in &self.values {
            payload.extend_from_slice(&value.payload);
        }
        let header = serde_json::to_vec(&Header {
            schema: "FerricReadiness40CausalLayerZeroV1",
            bootstrap,
            transcript_sha256,
            captures: &self.values,
            payload_bytes: TOTAL_BYTES,
            payload_sha256: Sha256::digest(&payload).into(),
            cache_layout: "used-prefix-token-head-channel-bf16",
            native_close_confirmed: true,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        })
        .map_err(|e| e.to_string())?;
        if header.len() > META_BYTES {
            return Err("causal capture metadata bound".into());
        }
        let mut output = Vec::with_capacity(16 + header.len() + payload.len());
        output.extend_from_slice(MAGIC);
        output.extend_from_slice(&(header.len() as u32).to_le_bytes());
        output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        output.extend_from_slice(&header);
        output.extend_from_slice(&payload);
        if output.len() > 2 << 20 {
            return Err("causal capture existing stderr bound".into());
        }
        Ok(output)
    }
}

#[cfg(test)]
#[path = "causal_v1_tests.rs"]
mod tests;
