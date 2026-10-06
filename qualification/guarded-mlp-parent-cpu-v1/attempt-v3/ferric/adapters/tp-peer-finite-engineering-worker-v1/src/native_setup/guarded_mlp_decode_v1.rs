//! Source-owned opt-in preparation; no wire schema or public authority change.
use super::{Group, NativeSetup, PrefixExecution, PreparedSetup, Read, Result, Write, hash};
use crate::native_catalog::forward::guarded_mlp_decode_v1::{Mode, Owner, Profile};
use crate::resident_layer::guarded_mlp_decode_v1::Images;
use crate::resident_layer::mlp_tiles_v2::Image as MlpImage;
use crate::resident_layer::prefix_tiles_v6::artifacts::Image as PrefixImage;

pub(crate) struct PreparedGuardedDecodeSetup {
    setup: PreparedSetup,
    prefix: PrefixImage,
    mlp: MlpImage,
    images: Images,
    profile: Profile,
}
impl PreparedGuardedDecodeSetup {
    pub(crate) fn new(
        setup: PreparedSetup,
        prefix: PrefixImage,
        mlp: MlpImage,
        images: Images,
        mode: Mode,
        timeout_ms: u32,
    ) -> Result<Self> {
        if setup.tail.is_none() || setup.tail_images.is_none() {
            return Err("guarded model requires authenticated complete tail before open".into());
        }
        let profile = Profile::new(
            &setup.scope,
            hash(&setup.registration_bytes),
            prefix.sha256(),
            mlp.sha256(),
            images.projection.sha256(),
            mode,
            timeout_ms,
            setup.devices,
        )?;
        Ok(Self {
            setup,
            prefix,
            mlp,
            images,
            profile,
        })
    }
    pub(crate) fn device_ids(&self) -> [u64; 2] {
        self.setup.devices
    }
    pub(crate) fn profile_sha256(&self) -> [u8; 32] {
        self.profile.sha256()
    }
    pub(crate) fn into_processor(self, group: Group) -> Result<GuardedDecodeSetup> {
        Ok(GuardedDecodeSetup {
            inner: self.setup.into_processor_with_guarded_images(
                group,
                Some(self.mlp),
                Some(self.prefix),
                PrefixExecution::GuardedDecodeFour,
                Some(self.images),
            )?,
            profile: self.profile,
        })
    }
}
pub(crate) struct GuardedDecodeSetup {
    inner: NativeSetup,
    profile: Profile,
}
impl GuardedDecodeSetup {
    pub(crate) fn serve(&mut self, r: &mut impl Read, w: &mut impl Write) -> Result<()> {
        self.inner.serve(r, w)
    }
    pub(crate) fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
    /// # Safety
    /// Same reviewed image/model, exact rank devices and coherence obligations
    /// as the private runtime facade. This is not a protected dispatch grant.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn into_decode(self) -> Result<Owner> {
        unsafe { Owner::from_sealed(self.inner.into_forward_owner()?, self.profile) }
    }
}
