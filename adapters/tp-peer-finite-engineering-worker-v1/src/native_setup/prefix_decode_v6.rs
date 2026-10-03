//! Distinct all-layer profile constructed before the audited Group opener.
use super::{Group, NativeSetup, PrefixExecution, PreparedSetup, Read, Result, Write, hash};
use crate::native_catalog::forward::prefix_tiles_decode_v6::{Mode, Owner, Profile};
use crate::resident_layer::mlp_tiles_v2::Image as MlpImage;
use crate::resident_layer::prefix_tiles_v6::artifacts::Image as PrefixImage;

pub(crate) struct PreparedPrefixDecodeSetup {
    setup: PreparedSetup,
    prefix: PrefixImage,
    mlp: MlpImage,
    profile: Profile,
}
fn require_tail(tail: bool, images: bool) -> Result<()> {
    if !tail || !images {
        return Err("prefix decode requires full authenticated tail before open".into());
    }
    Ok(())
}
impl PreparedPrefixDecodeSetup {
    pub(crate) fn new(
        setup: PreparedSetup,
        prefix: PrefixImage,
        mlp: MlpImage,
        mode: Mode,
        timeout_ms: u32,
    ) -> Result<Self> {
        require_tail(setup.tail.is_some(), setup.tail_images.is_some())?;
        let profile = Profile::new(
            &setup.scope,
            hash(&setup.registration_bytes),
            prefix.sha256(),
            mlp.sha256(),
            mode,
            timeout_ms,
            setup.devices,
        )?;
        Ok(Self {
            setup,
            prefix,
            mlp,
            profile,
        })
    }
    pub(crate) fn device_ids(&self) -> [u64; 2] {
        self.setup.devices
    }
    pub(crate) fn profile_sha256(&self) -> [u8; 32] {
        self.profile.sha256()
    }
    /// The caller opens exactly device_ids in rank order. This profile enables
    /// no caching/performance override and does not select the old layer0 owner.
    pub(crate) fn into_processor(self, group: Group) -> Result<PrefixDecodeSetup> {
        Ok(PrefixDecodeSetup {
            inner: self.setup.into_processor_execution(
                group,
                Some(self.mlp),
                Some(self.prefix),
                PrefixExecution::DecodeFour,
            )?,
            profile: self.profile,
        })
    }
}
pub(crate) struct PrefixDecodeSetup {
    inner: NativeSetup,
    profile: Profile,
}
impl PrefixDecodeSetup {
    pub(crate) fn serve(&mut self, r: &mut impl Read, w: &mut impl Write) -> Result<()> {
        self.inner.serve(r, w)
    }
    pub(crate) fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
    /// # Safety
    /// Parent authenticates actual source/model/images and independently reviews
    /// the exact devices, shared roots, coherence and retired-command premises.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn into_decode(self) -> Result<Owner> {
        unsafe { Owner::from_sealed(self.inner.into_forward_owner()?, self.profile) }
    }
}

#[cfg(test)]
mod tests;
