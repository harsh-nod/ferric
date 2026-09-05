//! Authority-free custody of one structurally valid aggregate program object.
//!
//! The excluded engineering adapter constructs an internal move-only source
//! capability only after stronger provenance and manifest checks. This module
//! independently validates the exact object profile and Ferric symbol/ABI
//! catalog, retains full input custody on rejection, and never constructs
//! authenticated Worker V3 custody or production runner authority.

use core::fmt;

use fe2o3_amdhsa_loader::{AdmittedProfile, LoadPlan, PlanError};
use ferric_non_authoritative_program_source_v1::{
    compiler_handoff_is_nonempty_v1, M1NonAuthoritativeProgramSourceCapabilityV1,
};
use ferric_spec::Identity;
use sha2::{Digest, Sha256};

use crate::physical_program_catalog::{
    bind_content_bound_m1_program_catalog_from_uniform_artifact_v1, ContentBoundM1ProgramCatalogV1,
    M1PhysicalProgramCatalogErrorV1, M1PhysicalProgramSourceContractV1,
};

/// Structural rejection of one authority-free aggregate program object.
#[derive(Debug)]
pub enum M1NonAuthoritativeProgramArtifactErrorV1 {
    /// The compiler handoff observation has no bytes.
    EmptyCompilerHandoff,
    /// The generic allocation-free gfx942/COV6 loader rejected the object.
    Loader(PlanError),
    /// Current Ferric symbols or dispatch ABIs do not close over the object.
    ProgramCatalog(Box<M1PhysicalProgramCatalogErrorV1>),
}

impl fmt::Display for M1NonAuthoritativeProgramArtifactErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "non-authoritative M1 program artifact rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1NonAuthoritativeProgramArtifactErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ProgramCatalog(source) => Some(source.as_ref()),
            Self::EmptyCompilerHandoff | Self::Loader(_) => None,
        }
    }
}

/// Move-only structural ownership of one authority-free aggregate object.
///
/// The observation identity and compiler-handoff identity are inert lineage
/// facts supplied by the caller. Ferric derives the HSACO and program-catalog
/// identities from the retained bytes and independently revalidates the load
/// plan whenever the catalog is borrowed.
///
/// ```compile_fail
/// use ferric_engine::M1NonAuthoritativeProgramArtifactV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1NonAuthoritativeProgramArtifactV1>();
/// ```
///
/// ```compile_fail
/// use ferric_engine::{
///     M1AuthenticatedPhysicalRunnerV1, M1NonAuthoritativeProgramArtifactV1,
/// };
/// fn cannot_authenticate(
///     value: M1NonAuthoritativeProgramArtifactV1,
/// ) -> M1AuthenticatedPhysicalRunnerV1 {
///     value.into()
/// }
/// ```
#[must_use = "non-authoritative program bytes must remain in explicit custody"]
pub struct M1NonAuthoritativeProgramArtifactV1 {
    source_capability: M1NonAuthoritativeProgramSourceCapabilityV1,
    hsaco_id: Identity,
    plan: LoadPlan,
    source: M1PhysicalProgramSourceContractV1,
    program_catalog_id: Identity,
}

impl fmt::Debug for M1NonAuthoritativeProgramArtifactV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1NonAuthoritativeProgramArtifactV1")
            .field("authority", &"none")
            .field("observation_id", &self.observation_id())
            .field("canonical_descriptor_id", &self.canonical_descriptor_id())
            .field("hsaco_id", &self.hsaco_id)
            .field("program_catalog_id", &self.program_catalog_id)
            .finish_non_exhaustive()
    }
}

impl M1NonAuthoritativeProgramArtifactV1 {
    pub(crate) fn content_bound_program_catalog_v1(
        &self,
    ) -> Result<ContentBoundM1ProgramCatalogV1<'_>, M1PhysicalProgramCatalogErrorV1> {
        bind_content_bound_m1_program_catalog_from_uniform_artifact_v1(
            self.source_capability.hsaco_bytes(),
            self.plan,
            self.source,
        )
    }

    /// Identity of the external non-authoritative observation.
    #[must_use]
    pub const fn observation_id(&self) -> Identity {
        self.source_capability.observation_manifest_id()
    }

    /// Independently checked canonical whole-HSACO descriptor identity.
    #[must_use]
    pub const fn canonical_descriptor_id(&self) -> Identity {
        self.source_capability.canonical_descriptor_id()
    }

    /// SHA-256 identity derived from the retained HSACO bytes.
    #[must_use]
    pub const fn hsaco_id(&self) -> Identity {
        self.hsaco_id
    }

    /// Caller-observed compiler handoff identity.
    #[must_use]
    pub const fn compiler_handoff_id(&self) -> Identity {
        self.source_capability.compiler_handoff_id()
    }

    /// Caller-observed compiler handoff byte length.
    #[must_use]
    pub const fn compiler_handoff_len(&self) -> u64 {
        self.source_capability.compiler_handoff_len()
    }

    /// Domain-separated identity of the twelve exact selected programs.
    #[must_use]
    pub const fn program_catalog_id(&self) -> Identity {
        self.program_catalog_id
    }

    /// Revalidates and lends the structural program catalog for one lexical use.
    ///
    /// # Errors
    ///
    /// Returns a current loader, symbol, or dispatch-ABI closure failure before
    /// invoking the callback.
    pub fn with_structural_program_catalog_v1<R>(
        &self,
        use_catalog: impl for<'catalog> FnOnce(ContentBoundM1ProgramCatalogV1<'catalog>) -> R,
    ) -> Result<R, M1PhysicalProgramCatalogErrorV1> {
        let catalog = self.content_bound_program_catalog_v1()?;
        Ok(use_catalog(catalog))
    }

    /// This structural owner carries no external authority.
    #[must_use]
    pub const fn authority(&self) -> &'static str {
        "none"
    }

    /// This owner grants no Worker V3 publication authority.
    #[must_use]
    pub const fn grants_publication_authority(&self) -> bool {
        false
    }

    /// This owner grants no authenticated executable-load authority.
    #[must_use]
    pub const fn grants_load_authority(&self) -> bool {
        false
    }

    /// This owner grants no authenticated queue-launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

/// Failed engine admission retaining the exact move-only source capability.
#[must_use = "non-authoritative admission failure retains exact source custody"]
#[derive(Debug)]
pub struct M1NonAuthoritativeProgramArtifactAdmissionFailureV1 {
    error: M1NonAuthoritativeProgramArtifactErrorV1,
    source_capability: Box<M1NonAuthoritativeProgramSourceCapabilityV1>,
}

impl fmt::Display for M1NonAuthoritativeProgramArtifactAdmissionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl std::error::Error for M1NonAuthoritativeProgramArtifactAdmissionFailureV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.error.source()
    }
}

impl M1NonAuthoritativeProgramArtifactAdmissionFailureV1 {
    /// Borrows the exact admission diagnostic.
    #[must_use]
    pub const fn error(&self) -> &M1NonAuthoritativeProgramArtifactErrorV1 {
        &self.error
    }

    /// Borrows the unchanged source capability.
    #[must_use = "the retained source capability remains in explicit custody"]
    pub const fn source_capability(&self) -> &M1NonAuthoritativeProgramSourceCapabilityV1 {
        &self.source_capability
    }

    /// Returns the exact diagnostic and unchanged move-only source capability.
    #[must_use = "the diagnostic and retained source capability must be handled"]
    pub fn into_parts(
        self,
    ) -> (
        M1NonAuthoritativeProgramArtifactErrorV1,
        M1NonAuthoritativeProgramSourceCapabilityV1,
    ) {
        (self.error, *self.source_capability)
    }
}

/// Admits one internal source capability only as an authority-free structural owner.
///
/// # Errors
///
/// Rejects an empty compiler handoff, a non-gfx942/COV6 object, or any object
/// that does not close over Ferric's current twelve-program symbol and ABI
/// catalog. Every rejection returns the exact unchanged source capability.
pub fn admit_m1_non_authoritative_program_artifact_v1(
    source_capability: M1NonAuthoritativeProgramSourceCapabilityV1,
) -> Result<M1NonAuthoritativeProgramArtifactV1, M1NonAuthoritativeProgramArtifactAdmissionFailureV1>
{
    let compiler_handoff_len = source_capability.compiler_handoff_len();
    if !compiler_handoff_is_nonempty_v1(compiler_handoff_len) {
        return Err(M1NonAuthoritativeProgramArtifactAdmissionFailureV1 {
            error: M1NonAuthoritativeProgramArtifactErrorV1::EmptyCompilerHandoff,
            source_capability: Box::new(source_capability),
        });
    }
    let bytes = source_capability.hsaco_bytes();
    let plan = {
        let envelope =
            match fe2o3_amdhsa_loader::validate(bytes, AdmittedProfile::Gfx942XnackOffCov6) {
                Ok(envelope) => envelope,
                Err(error) => {
                    return Err(M1NonAuthoritativeProgramArtifactAdmissionFailureV1 {
                        error: M1NonAuthoritativeProgramArtifactErrorV1::Loader(error),
                        source_capability: Box::new(source_capability),
                    });
                }
            };
        *envelope.plan()
    };
    let source = M1PhysicalProgramSourceContractV1::new(
        *source_capability.compiler_handoff_id().as_bytes(),
        compiler_handoff_len,
    );
    let program_catalog_id = {
        let catalog = match bind_content_bound_m1_program_catalog_from_uniform_artifact_v1(
            bytes, plan, source,
        ) {
            Ok(catalog) => catalog,
            Err(error) => {
                return Err(M1NonAuthoritativeProgramArtifactAdmissionFailureV1 {
                    error: M1NonAuthoritativeProgramArtifactErrorV1::ProgramCatalog(Box::new(
                        error,
                    )),
                    source_capability: Box::new(source_capability),
                });
            }
        };
        catalog.catalog_id()
    };
    let hsaco_id = Identity::new(digest(bytes));
    Ok(M1NonAuthoritativeProgramArtifactV1 {
        source_capability,
        hsaco_id,
        plan,
        source,
        program_catalog_id,
    })
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    let value = Sha256::digest(bytes);
    let mut digest = [0_u8; 32];
    digest.copy_from_slice(&value);
    digest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(
        compiler_handoff_len: u64,
        bytes: Box<[u8]>,
    ) -> M1NonAuthoritativeProgramSourceCapabilityV1 {
        M1NonAuthoritativeProgramSourceCapabilityV1::from_observed_engineering_parts_v1(
            Identity::new([1; 32]),
            Identity::new([2; 32]),
            Identity::new([3; 32]),
            compiler_handoff_len,
            bytes,
        )
    }

    #[test]
    fn empty_handoff_rejection_retains_exact_source_allocation_and_lineage() {
        let bytes = vec![7_u8, 8, 9].into_boxed_slice();
        let pointer = bytes.as_ptr();
        let failure = admit_m1_non_authoritative_program_artifact_v1(source(0, bytes))
            .expect_err("empty handoff must be rejected");
        assert!(matches!(
            failure.error(),
            M1NonAuthoritativeProgramArtifactErrorV1::EmptyCompilerHandoff
        ));
        let retained = failure.source_capability();
        assert_eq!(retained.observation_manifest_id(), Identity::new([1; 32]));
        assert_eq!(retained.canonical_descriptor_id(), Identity::new([2; 32]));
        assert_eq!(retained.compiler_handoff_id(), Identity::new([3; 32]));
        assert_eq!(retained.hsaco_bytes().as_ptr(), pointer);
        assert_eq!(retained.hsaco_bytes(), &[7, 8, 9]);
    }

    #[test]
    fn invalid_object_rejection_retains_exact_source_allocation() {
        let bytes = vec![1_u8, 2, 3, 4].into_boxed_slice();
        let pointer = bytes.as_ptr();
        let failure = admit_m1_non_authoritative_program_artifact_v1(source(4, bytes))
            .expect_err("non-object bytes must be rejected");
        assert!(matches!(
            failure.error(),
            M1NonAuthoritativeProgramArtifactErrorV1::Loader(_)
        ));
        assert_eq!(failure.source_capability().hsaco_bytes().as_ptr(), pointer);
        assert_eq!(failure.source_capability().compiler_handoff_len(), 4);
    }
}
