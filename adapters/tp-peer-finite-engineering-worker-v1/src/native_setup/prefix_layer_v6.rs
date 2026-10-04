//! Caller-facing setup extension; old Begin and old selectors are unchanged.
use super::{Group, NativeSetup, PreparedSetup, Read, Result, Write, hash};
use crate::forward_sequence::ForwardInput;
use crate::native_catalog::forward::prefix_tiles_layer_v6::{Owner, Profile};
use crate::resident_layer::mlp_tiles_v2::Image as MlpImage;
use crate::resident_layer::prefix_tiles_v6::artifacts::Image as PrefixImage;
use crate::resident_layer::prefix_tiles_v6::projection_residual::Image as ProjectionImage;

pub(crate) struct PreparedPrefixLayerSetup {
    setup: PreparedSetup,
    prefix: Option<PrefixImage>,
    mlp: MlpImage,
    profile: Profile,
    projection: Option<ProjectionImage>,
}
fn require_tail(tail: bool, images: bool) -> Result<()> {
    if !tail || !images {
        return Err("one-layer requires authenticated embedding/tail setup before open".into());
    }
    Ok(())
}
impl PreparedPrefixLayerSetup {
    /// None selects a fresh Prefix22+MLP548 comparison baseline. Some selects
    /// the distinct candidate. Both are new layer-only routes, not old IPC modes.
    pub(crate) fn new(
        setup: PreparedSetup,
        prefix: Option<PrefixImage>,
        mlp: MlpImage,
        input: &ForwardInput,
        timeout_ms: u32,
    ) -> Result<Self> {
        require_tail(setup.tail.is_some(), setup.tail_images.is_some())?;
        let profile = Profile::new(
            &setup.scope,
            hash(&setup.registration_bytes),
            prefix.as_ref().map(PrefixImage::sha256),
            mlp.sha256(),
            input,
            timeout_ms,
            setup.devices,
        )?;
        Ok(Self {
            setup,
            prefix,
            mlp,
            profile,
            projection: None,
        })
    }
    pub(crate) fn new_projection(
        setup: PreparedSetup,
        prefix: PrefixImage,
        mlp: MlpImage,
        projection: ProjectionImage,
        input: &ForwardInput,
        timeout_ms: u32,
    ) -> Result<Self> {
        let mut prepared = Self::new(setup, Some(prefix), mlp, input, timeout_ms)?;
        prepared.profile = prepared.profile.with_projection(projection.sha256())?;
        prepared.projection = Some(projection);
        Ok(prepared)
    }
    pub(crate) fn device_ids(&self) -> [u64; 2] {
        self.setup.devices
    }
    pub(crate) fn profile_sha256(&self) -> [u8; 32] {
        self.profile.sha256()
    }
    /// The audited caller opens exactly device_ids in rank order. No performance
    /// switch is enabled by this route; ordinary Group admission remains active.
    pub(crate) fn into_processor(self, group: Group) -> Result<PrefixLayerSetup> {
        Ok(PrefixLayerSetup {
            inner: self
                .setup
                .into_processor_profiles(group, Some(self.mlp), self.prefix)?,
            profile: self.profile,
            projection: self.projection,
        })
    }
}
pub(crate) struct PrefixLayerSetup {
    inner: NativeSetup,
    profile: Profile,
    projection: Option<ProjectionImage>,
}
impl PrefixLayerSetup {
    pub(crate) fn serve(&mut self, r: &mut impl Read, w: &mut impl Write) -> Result<()> {
        self.inner.serve(r, w)
    }
    pub(crate) fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
    /// # Safety
    /// The parent independently authenticated actual source/model/objects and
    /// reviewed the exact selected-device, coherence and lifetime premises.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn into_layer(self) -> Result<Owner> {
        let owner = self.inner.into_forward_owner()?;
        match self.projection {
            Some(image) => unsafe { Owner::from_sealed_projection(owner, self.profile, image) },
            None => unsafe { Owner::from_sealed(owner, self.profile) },
        }
    }
}

#[cfg(test)]
mod tests;
