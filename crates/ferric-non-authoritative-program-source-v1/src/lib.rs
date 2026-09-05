#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Internal, move-only custody for a non-authoritative engineering program source.
//!
//! This crate is an auditable dependency boundary between the excluded
//! engineering adapter and the qualified engine. The source carries bytes and
//! lineage only. It grants no authentication, allocation, load, publication,
//! dispatch, or completion authority.

use ferric_spec::Identity;
#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

/// Classifies whether the observed compiler handoff has a nonempty byte image.
#[must_use]
pub fn compiler_handoff_is_nonempty_v1(byte_len: u64) -> (nonempty: bool)
    ensures nonempty == (byte_len > 0),
{
    byte_len > 0
}

} // verus!

/// Move-only bytes and complete lineage from one admitted engineering observation.
///
/// Ferric Engine deliberately does not reexport this type or its constructor.
/// A caller must declare a direct dependency on this internal crate before it
/// can construct a source for the engine's non-authoritative admission seam.
///
/// ```compile_fail
/// use ferric_non_authoritative_program_source_v1::M1NonAuthoritativeProgramSourceCapabilityV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1NonAuthoritativeProgramSourceCapabilityV1>();
/// ```
#[must_use = "program source bytes and lineage must remain in explicit custody"]
pub struct M1NonAuthoritativeProgramSourceCapabilityV1 {
    observation_manifest_id: Identity,
    canonical_descriptor_id: Identity,
    compiler_handoff_id: Identity,
    compiler_handoff_len: u64,
    hsaco_bytes: Box<[u8]>,
}

impl core::fmt::Debug for M1NonAuthoritativeProgramSourceCapabilityV1 {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("M1NonAuthoritativeProgramSourceCapabilityV1")
            .field("authority", &"none")
            .field("observation_manifest_id", &self.observation_manifest_id)
            .field("canonical_descriptor_id", &self.canonical_descriptor_id)
            .field("compiler_handoff_id", &self.compiler_handoff_id)
            .field("compiler_handoff_len", &self.compiler_handoff_len)
            .field("hsaco_len", &self.hsaco_bytes.len())
            .finish_non_exhaustive()
    }
}

impl M1NonAuthoritativeProgramSourceCapabilityV1 {
    /// Constructs custody after the excluded engineering adapter has validated
    /// the canonical manifest, directory, finalized descriptor, and roster.
    ///
    /// This is not an admission or authority boundary. Ferric Engine performs
    /// its own loader and program-catalog validation before retaining the source.
    #[doc(hidden)]
    pub fn from_observed_engineering_parts_v1(
        observation_manifest_id: Identity,
        canonical_descriptor_id: Identity,
        compiler_handoff_id: Identity,
        compiler_handoff_len: u64,
        hsaco_bytes: Box<[u8]>,
    ) -> Self {
        Self {
            observation_manifest_id,
            canonical_descriptor_id,
            compiler_handoff_id,
            compiler_handoff_len,
            hsaco_bytes,
        }
    }

    /// SHA-256 identity of the adapter-validated canonical observation manifest.
    #[must_use]
    pub const fn observation_manifest_id(&self) -> Identity {
        self.observation_manifest_id
    }

    /// Independently checked canonical whole-HSACO descriptor identity.
    #[must_use]
    pub const fn canonical_descriptor_id(&self) -> Identity {
        self.canonical_descriptor_id
    }

    /// SHA-256 identity of the compiler handoff observed by fe2o3.
    #[must_use]
    pub const fn compiler_handoff_id(&self) -> Identity {
        self.compiler_handoff_id
    }

    /// Exact byte length of the compiler handoff observed by fe2o3.
    #[must_use]
    pub const fn compiler_handoff_len(&self) -> u64 {
        self.compiler_handoff_len
    }

    /// Borrows the exact retained aggregate HSACO bytes.
    #[must_use]
    pub fn hsaco_bytes(&self) -> &[u8] {
        &self.hsaco_bytes
    }

    /// Decomposes the move-only source without changing any identity or bytes.
    pub fn into_parts(self) -> M1NonAuthoritativeProgramSourcePartsV1 {
        M1NonAuthoritativeProgramSourcePartsV1 {
            observation_manifest_id: self.observation_manifest_id,
            canonical_descriptor_id: self.canonical_descriptor_id,
            compiler_handoff_id: self.compiler_handoff_id,
            compiler_handoff_len: self.compiler_handoff_len,
            hsaco_bytes: self.hsaco_bytes,
        }
    }
}

/// Exact decomposition of one move-only non-authoritative program source.
#[must_use = "decomposed program source custody must remain retained"]
pub struct M1NonAuthoritativeProgramSourcePartsV1 {
    /// SHA-256 identity of the canonical observation manifest.
    pub observation_manifest_id: Identity,
    /// Independently checked canonical whole-HSACO descriptor identity.
    pub canonical_descriptor_id: Identity,
    /// SHA-256 identity of the compiler handoff observed by fe2o3.
    pub compiler_handoff_id: Identity,
    /// Exact byte length of the compiler handoff observed by fe2o3.
    pub compiler_handoff_len: u64,
    /// Exact aggregate HSACO bytes.
    pub hsaco_bytes: Box<[u8]>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_parts_round_trip_retains_byte_allocation_and_lineage() {
        let bytes = vec![7_u8, 8, 9].into_boxed_slice();
        let pointer = bytes.as_ptr();
        let source =
            M1NonAuthoritativeProgramSourceCapabilityV1::from_observed_engineering_parts_v1(
                Identity::new([1; 32]),
                Identity::new([2; 32]),
                Identity::new([3; 32]),
                4,
                bytes,
            );

        let parts = source.into_parts();
        assert_eq!(parts.observation_manifest_id, Identity::new([1; 32]));
        assert_eq!(parts.canonical_descriptor_id, Identity::new([2; 32]));
        assert_eq!(parts.compiler_handoff_id, Identity::new([3; 32]));
        assert_eq!(parts.compiler_handoff_len, 4);
        assert_eq!(parts.hsaco_bytes.as_ptr(), pointer);
        assert_eq!(&*parts.hsaco_bytes, &[7, 8, 9]);
    }
}
