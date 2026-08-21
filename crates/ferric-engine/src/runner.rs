//! Logical runtime custody for one published generated-runner declaration.
//!
//! This module does not own a physical runner. It retains build-authenticated
//! declaration custody and binds request-local [`StepPlan`] values to exact
//! plan identities without creating device, artifact, load, queue, launch,
//! completion, proof, hardware, performance, or qualification authority.

use ferric_build::{
    GeneratedOperationDeclaration, GeneratedPlanDeclaration, PublishedRunnerDeclaration,
};
use ferric_spec::completion::CompletionEpoch;
use ferric_spec::{Identity, Qwen3PlanSelection, RequestId, StepPlan};

/// Fail-closed logical declaration lookup error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicalRunnerError {
    /// The requested role, mode, and bucket are absent from the publication.
    PlanNotPublished(Qwen3PlanSelection),
    /// A retained plan range no longer fits the exact operation roster.
    OperationRangeDrift,
}

/// Engine-owned custody of one linearly published runner declaration.
///
/// The private, non-clone publication remains the sole authority source. This
/// logical catalog performs no physical runner action and does not close the
/// M1 generated-runner, physical-runner, graph-proof, or qualification gates.
#[derive(Debug, Eq, PartialEq)]
pub struct LogicalRunnerDeclaration {
    publication: PublishedRunnerDeclaration,
}

impl LogicalRunnerDeclaration {
    /// Consumes the build-owned publication into engine custody.
    #[must_use]
    pub const fn from_published(publication: PublishedRunnerDeclaration) -> Self {
        Self { publication }
    }

    /// Returns the exact checked-in generated source identity.
    #[must_use]
    pub const fn source_id(&self) -> Identity {
        self.publication.source_id()
    }

    /// Returns the retained authenticated admission-record identity.
    #[must_use]
    pub const fn admission_record_id(&self) -> Identity {
        self.publication.admission_record_id()
    }

    /// Returns the exact admitted deployment identity.
    #[must_use]
    pub const fn bundle_id(&self) -> Identity {
        self.publication.bundle_id()
    }

    /// Returns the authenticated target prepacked-manifest identity.
    #[must_use]
    pub const fn target_prepacked_id(&self) -> Identity {
        self.publication.target_prepacked_id()
    }

    /// Returns the authenticated draft prepacked-manifest identity.
    #[must_use]
    pub const fn draft_prepacked_id(&self) -> Identity {
        self.publication.draft_prepacked_id()
    }

    /// Returns the exact sequential plan-catalog identity.
    #[must_use]
    pub const fn plan_catalog_id(&self) -> Identity {
        self.publication.plan_catalog_id()
    }

    /// Returns the exact structural K1-K7 catalog identity.
    #[must_use]
    pub const fn kernel_catalog_id(&self) -> Identity {
        self.publication.kernel_catalog_id()
    }

    /// Returns the retained preliminary closure identity.
    #[must_use]
    pub const fn closure_id(&self) -> Identity {
        self.publication.closure_id()
    }

    /// Returns the complete canonical declaration identity.
    #[must_use]
    pub const fn declaration_id(&self) -> Identity {
        self.publication.declaration_id()
    }

    /// Returns the exact canonical generated-declaration format version.
    #[must_use]
    pub const fn declaration_version(&self) -> u32 {
        self.publication.declaration_version()
    }

    /// Returns the exact checked-in generated-template format version.
    #[must_use]
    pub const fn template_version(&self) -> u32 {
        self.publication.template_version()
    }

    /// Returns the exact number of published target/draft plans.
    #[must_use]
    pub fn plan_count(&self) -> usize {
        self.publication.plans().len()
    }

    /// Returns the exact number of retained typed logical operations.
    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.publication.operations().len()
    }

    /// Returns the exact number of logical request-input schema entries.
    #[must_use]
    pub fn patch_slot_count(&self) -> usize {
        self.publication.patch_slots().len()
    }

    /// Selects the unique exact published plan for a role, mode, and bucket.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalRunnerError::PlanNotPublished`] for any invalid or
    /// absent role/mode/bucket combination.
    pub fn plan(
        &self,
        selection: Qwen3PlanSelection,
    ) -> Result<&GeneratedPlanDeclaration, LogicalRunnerError> {
        find_plan(self.publication.plans(), selection)
            .ok_or(LogicalRunnerError::PlanNotPublished(selection))
    }

    /// Returns the exact contiguous logical operation range for one plan.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalRunnerError`] if the selection is absent or if the
    /// retained range fails its independent bounds check.
    pub fn operations_for(
        &self,
        selection: Qwen3PlanSelection,
    ) -> Result<&[GeneratedOperationDeclaration], LogicalRunnerError> {
        let plan = self.plan(selection)?;
        let range = checked_operation_range(plan, self.publication.operations().len())?;
        Ok(&self.publication.operations()[range])
    }

    /// Binds one request-local logical step to an exact published plan.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalRunnerError::PlanNotPublished`] when the requested
    /// selection has no exact published plan. Success grants no physical
    /// execution or completion authority.
    pub fn bind_step_plan(
        &self,
        request: RequestId,
        completion_epoch: CompletionEpoch,
        selection: Qwen3PlanSelection,
    ) -> Result<StepPlan, LogicalRunnerError> {
        let plan = self.plan(selection)?;
        Ok(StepPlan::new(
            request,
            completion_epoch,
            plan.plan_id,
            plan.selection,
        ))
    }
}

fn find_plan(
    plans: &[GeneratedPlanDeclaration],
    selection: Qwen3PlanSelection,
) -> Option<&GeneratedPlanDeclaration> {
    plans.iter().find(|plan| plan.selection == selection)
}

fn checked_operation_range(
    plan: &GeneratedPlanDeclaration,
    operation_count: usize,
) -> Result<std::ops::Range<usize>, LogicalRunnerError> {
    let start = usize::try_from(plan.operation_start)
        .map_err(|_| LogicalRunnerError::OperationRangeDrift)?;
    let count = usize::try_from(plan.operation_count)
        .map_err(|_| LogicalRunnerError::OperationRangeDrift)?;
    let end = start
        .checked_add(count)
        .ok_or(LogicalRunnerError::OperationRangeDrift)?;
    if end > operation_count {
        return Err(LogicalRunnerError::OperationRangeDrift);
    }
    Ok(start..end)
}

#[cfg(test)]
mod tests {
    use super::{checked_operation_range, find_plan, LogicalRunnerError};
    use ferric_build::GeneratedPlanDeclaration;
    use ferric_spec::{
        Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection,
    };

    const fn selection(bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket,
        }
    }

    const fn plan(
        plan_index: u16,
        bucket: Qwen3PlanBucket,
        operation_start: u32,
        operation_count: u32,
    ) -> GeneratedPlanDeclaration {
        GeneratedPlanDeclaration {
            plan_index,
            plan_id: Identity::new([plan_index as u8 + 1; 32]),
            selection: selection(bucket),
            operation_start,
            operation_count,
        }
    }

    #[test]
    fn lookup_rejects_an_unpublished_or_mode_mismatched_selection() {
        let plans = [
            plan(0, Qwen3PlanBucket::PrefillS1T128, 0, 544),
            plan(1, Qwen3PlanBucket::PrefillS8T128, 544, 544),
        ];
        assert_eq!(
            find_plan(&plans, selection(Qwen3PlanBucket::PrefillS8T128)),
            Some(&plans[1])
        );
        assert_eq!(
            find_plan(&plans, selection(Qwen3PlanBucket::PrefillS1T512)),
            None
        );
        assert_eq!(
            find_plan(
                &plans,
                Qwen3PlanSelection {
                    role: Qwen3ModelRole::Target8B,
                    mode: Qwen3ExecutionMode::Decode,
                    bucket: Qwen3PlanBucket::PrefillS1T128,
                },
            ),
            None
        );
    }

    #[test]
    fn operation_ranges_reject_truncation_and_extreme_offsets() {
        let exact = plan(0, Qwen3PlanBucket::PrefillS1T128, 544, 544);
        assert_eq!(checked_operation_range(&exact, 1_088), Ok(544..1_088));
        assert_eq!(
            checked_operation_range(&exact, 1_087),
            Err(LogicalRunnerError::OperationRangeDrift)
        );

        let overflow = plan(0, Qwen3PlanBucket::PrefillS1T128, u32::MAX, u32::MAX);
        assert_eq!(
            checked_operation_range(&overflow, 10_648),
            Err(LogicalRunnerError::OperationRangeDrift)
        );
    }
}
