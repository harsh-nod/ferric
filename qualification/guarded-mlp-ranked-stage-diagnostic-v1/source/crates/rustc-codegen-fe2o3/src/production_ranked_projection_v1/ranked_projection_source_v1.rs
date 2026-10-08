//! Private borrowed inputs for the existing source-ranked projector.

use fe2o3_kernel_ir::{
    CanonicalKernelIrVerificationResourceBudgetV1 as Budget,
    CanonicalKernelIrVerificationResourceErrorV1 as Resource, CanonicalKernelIrWorkBudgetV1,
    VerifiedCanonicalKernelIrModuleV12,
};
use fe2o3_lower_mir_kernel::{
    ProductionEmptyEffectHelpersV1, ProductionPreRankedFiniteJoinViewV1,
    ProductionPreRankedKirOwnerV1, ProductionSourceLaunchRosterV1, SemanticKirAssertOriginsV1,
};
use fe2o3_pliron::ProductionSemanticSsaOwnerV1;

use super::{
    CanonicalAssertionErrorV1, ProductionRankedProjectionErrorV1 as Error,
    RankedStructuralValidationStageV1 as Stage,
};

pub(super) struct RankedProjectionSourceV1<'s> {
    owner: &'s ProductionPreRankedKirOwnerV1,
    semantic_ssa: &'s ProductionSemanticSsaOwnerV1,
    source_launch: &'s ProductionSourceLaunchRosterV1,
    executable: &'s VerifiedCanonicalKernelIrModuleV12,
    origins: SemanticKirAssertOriginsV1<'s>,
    empty_effect_helpers: ProductionEmptyEffectHelpersV1<'s>,
    minimum_storage: usize,
}

impl<'s> RankedProjectionSourceV1<'s> {
    pub(super) fn from_legacy(owner: &'s ProductionPreRankedKirOwnerV1) -> Result<Self, Error> {
        let minimum_storage = owner
            .executable_storage()
            .retained_storage()
            .checked_add(owner.assert_origin_storage().payload_storage())
            .ok_or_else(|| resource(Resource::Arithmetic))?;
        Ok(Self {
            owner,
            semantic_ssa: owner.semantic_ssa(),
            source_launch: owner.source_launch(),
            executable: owner.executable(),
            origins: owner.assert_origins(),
            empty_effect_helpers: owner.empty_effect_helpers(),
            minimum_storage,
        })
    }

    pub(super) const fn semantic_ssa(&self) -> &'s ProductionSemanticSsaOwnerV1 {
        self.semantic_ssa
    }

    pub(super) fn finite_join_view_for_root(
        &self,
        root: fe2o3_mir_model::semantic_mir_v1::SemanticFunctionIdV1,
        body: fe2o3_mir_model::semantic_mir_v1::SemanticFunctionIdV1,
    ) -> Result<Option<ProductionPreRankedFiniteJoinViewV1<'s>>, Error> {
        self.owner
            .replay_finite_join_view_for_root_v1(root)
            .map_err(|error| Error::StructuralValidationAt {
                stage: Stage::FiniteReplay,
                root: Some(root),
                body,
                error,
            })
    }

    pub(super) const fn source_launch(&self) -> &'s ProductionSourceLaunchRosterV1 {
        self.source_launch
    }

    pub(super) fn wave_task_view_for_root(
        &self,
        root: fe2o3_mir_model::semantic_mir_v1::SemanticFunctionIdV1,
        body: fe2o3_mir_model::semantic_mir_v1::SemanticFunctionIdV1,
        budget: &mut Budget<'_>,
    ) -> Result<Option<fe2o3_lower_mir_kernel::ProductionPreRankedWaveTaskRecipeV1<'s>>, Error>
    {
        self.owner
            .replay_wave_task_recipe_for_root_v1(root, budget)
            .map_err(|error| Error::StructuralValidationAt {
                stage: Stage::WaveReplay,
                root: Some(root),
                body,
                error,
            })
    }

    pub(super) const fn executable(&self) -> &'s VerifiedCanonicalKernelIrModuleV12 {
        self.executable
    }

    pub(super) const fn origins(&self) -> SemanticKirAssertOriginsV1<'s> {
        self.origins
    }

    pub(super) const fn empty_effect_helpers(&self) -> ProductionEmptyEffectHelpersV1<'s> {
        self.empty_effect_helpers
    }

    pub(super) fn require_floor(&self, budget: &Budget<'_>) -> Result<(), Error> {
        if budget.storage() < self.minimum_storage {
            return Err(resource(Resource::Accounting));
        }
        Ok(())
    }
}

pub(super) fn resource(error: Resource) -> Error {
    Error::CanonicalAssertions(CanonicalAssertionErrorV1::Resource(error))
}

pub(super) fn with_projection_source_budget_v1<T>(
    source: &RankedProjectionSourceV1<'_>,
    body: impl FnOnce(&mut Budget<'_>) -> Result<T, Error>,
) -> Result<T, Error> {
    let work_limit = usize::try_from(crate::production_canonical_phase_policy_v1::WORK_LIMIT)
        .map_err(|_| resource(Resource::Arithmetic))?;
    let mut work = CanonicalKernelIrWorkBudgetV1::new(work_limit);
    let mut budget = Budget::new(
        &mut work,
        crate::production_canonical_phase_policy_v1::STORAGE_LIMIT,
    );
    budget
        .reserve_storage(source.minimum_storage)
        .map_err(resource)?;
    body(&mut budget)
}
