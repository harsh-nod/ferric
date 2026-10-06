//! One additional immutable allocation; never reinterprets the original head.

use fe2o3_kfd::{Gfx950EngineeringPeerBufferV1 as Buffer, Gfx950EngineeringPeerGroupV1 as Group};
use sha2::{Digest, Sha256};

type Result<T> = std::result::Result<T, String>;
pub(super) const HEAD_BYTES: u64 = 1_244_659_712;
const MAX_CHUNK: usize = 4 * 1024 * 1024;

/// Private setup description. NativeOwner must join this to the authenticated
/// source-owned upload manifest, not accept a hash as executable authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct HeadManifest {
    pub(super) source_id: u64,
    pub(super) source_sha256: [u8; 32],
    pub(super) output_sha256: [u8; 32],
}

struct Upload<B, const BYTES: u64 = HEAD_BYTES> {
    buffer: B,
    manifest: HeadManifest,
    offset: u64,
    hasher: Sha256,
    terminal: bool,
    complete: bool,
}

trait Backend {
    type Buffer: Copy + Eq;
    fn preflight(&mut self) -> Result<()>;
    fn allocate(&mut self) -> Result<Self::Buffer>;
    fn write(&mut self, buffer: Self::Buffer, offset: u64, bytes: &[u8]) -> Result<()>;
}
impl Backend for Group {
    type Buffer = Buffer;
    fn preflight(&mut self) -> Result<()> {
        self.preflight_additional_allocations_v1(&[1, 0])
            .map(|_| ())
    }
    fn allocate(&mut self) -> Result<Buffer> {
        Group::allocate(self, 0, &[], HEAD_BYTES)
    }
    fn write(&mut self, buffer: Buffer, offset: u64, bytes: &[u8]) -> Result<()> {
        Group::write(self, buffer, offset, bytes)
    }
}

fn allocate<B: Backend>(
    backend: &mut B,
    original: B::Buffer,
    source_id: u64,
    initialized_source_sha256: [u8; 32],
    manifest: HeadManifest,
) -> Result<Upload<B::Buffer>> {
    if source_id == 0
        || source_id != manifest.source_id
        || manifest.source_sha256 == [0; 32]
        || manifest.output_sha256 == [0; 32]
        || initialized_source_sha256 != manifest.source_sha256
    {
        return Err("tail transpose original source manifest mismatch".into());
    }
    backend.preflight()?;
    let buffer = backend.allocate()?;
    if buffer == original {
        return Err("tail transpose aliases original NxK source".into());
    }
    Ok(Upload {
        buffer,
        manifest,
        offset: 0,
        hasher: Sha256::new(),
        terminal: false,
        complete: false,
    })
}

impl<B: Copy, const BYTES: u64> Upload<B, BYTES> {
    fn write<T: Backend<Buffer = B>>(
        &mut self,
        backend: &mut T,
        offset: u64,
        bytes: &[u8],
    ) -> Result<()>
    where
        B: Eq,
    {
        let result = (|| {
            let end = offset
                .checked_add(bytes.len() as u64)
                .ok_or("tail transpose write overflow")?;
            if self.terminal
                || self.complete
                || offset != self.offset
                || bytes.is_empty()
                || bytes.len() > MAX_CHUNK
                || bytes.len() % 2 != 0
                || end > BYTES
            {
                return Err("tail transpose upload sequence or extent".into());
            }
            backend.write(self.buffer, offset, bytes)?;
            self.hasher.update(bytes);
            self.offset = end;
            if end == BYTES {
                if <[u8; 32]>::from(self.hasher.clone().finalize()) != self.manifest.output_sha256 {
                    return Err("tail transpose final digest mismatch".into());
                }
                self.complete = true;
            }
            Ok(())
        })();
        if result.is_err() {
            self.terminal = true;
        }
        result
    }
    fn buffer(&self) -> Result<B> {
        if self.terminal || !self.complete || self.offset != BYTES {
            return Err("tail transpose is not completely initialized".into());
        }
        Ok(self.buffer)
    }
}

/// A separate actual owner token whose accessor remains closed until full digest
/// completion. No ordinary source token can be cast/decoded into this type.
pub(super) struct HeadTranspose {
    upload: Upload<Buffer>,
}
impl HeadTranspose {
    /// Caller privately resolves the original head record and its completed SHA
    /// from this same retained owner; IPC source IDs are not native tokens.
    pub(super) fn allocate(
        group: &mut Group,
        original: Buffer,
        source_id: u64,
        initialized_source_sha256: [u8; 32],
        manifest: HeadManifest,
    ) -> Result<Self> {
        if original.owner_rank() != 0 || original.bytes() != HEAD_BYTES {
            return Err("tail original head owner or byte extent".into());
        }
        let upload = allocate(
            group,
            original,
            source_id,
            initialized_source_sha256,
            manifest,
        )?;
        if upload.buffer.owner_rank() != 0 || upload.buffer.bytes() != HEAD_BYTES {
            return Err("tail transpose native owner or byte extent".into());
        }
        Ok(Self { upload })
    }
    pub(super) fn write(&mut self, group: &mut Group, offset: u64, bytes: &[u8]) -> Result<()> {
        self.upload.write(group, offset, bytes)
    }
    pub(super) fn buffer(&self) -> Result<Buffer> {
        self.upload.buffer()
    }
}

#[cfg(test)]
#[path = "tail_head_tests.rs"]
mod tests;
