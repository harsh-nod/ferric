//! Linear logical and physical composition for the exact M1 Qwen3 runner.
//!
//! Logical publication remains independently observable, while
//! [`M1PhysicalRunnerV1`] additionally retains the admitted persisted K1-K7
//! bytes and the exact generated operation bindings derived from that manifest.
//! Effectful methods only compose existing Ferric ownership transitions. They
//! expose no native address, add no fallback, and preserve lower recovery or
//! quarantine owners in every failure result.

use fe2o3_kfd::CheckedGfx942XnackMinusDevice;
use ferric_build::{
    AddresslessModelMemoryPlan, GeneratedOperationDeclaration, GeneratedPlanDeclaration,
    M1KernelArtifactFamilyV1, PublishedRunnerDeclaration,
};
use ferric_kernels::KernelFamily;
use ferric_spec::completion::CompletionEpoch;
use ferric_spec::{Identity, Qwen3PlanSelection, RequestId, StepPlan};

use crate::operation_kernel_plan::derive_canonical_operation_bindings;
use crate::{
    allocate_initialized_m1_model_memory_on_device_v1, allocate_m1_prepublication_workspaces_v1,
    bind_declared_operation_kernel_plan, bind_m1_partitioned_model_memory_kv_pool_v1,
    build_m1_prepublication_batch_v1, complete_m1_physical_step_v1,
    compose_addressless_m1_full_step_workspaces, derive_m1_physical_buffer_recipe_v1,
    derive_m1_physical_dispatch_recipe_v1, derive_m1_physical_kernarg_recipe_v1,
    derive_m1_step_dispatch_plan, prepare_m1_scheduled_workspace_images_v1,
    release_m1_completed_step_kv_pages_v1, submit_m1_long_lived_queue_rearm_v1,
    AddresslessM1PhysicalBufferRecipeV1, AddresslessM1PhysicalKernargRecipeV1,
    AddresslessM1StepDispatchPlan, AdmittedPersistedM1KernelArtifactsV1, BoundM1CompletionOutputV1,
    ContentBoundM1ProgramCatalogV1, DeclaredKernelFamilyArtifact, DeclaredOperationKernelPlan,
    Engine, M1CompletedReadbackJoinFailureV1, M1CompletedStepKvReleaseFailureV1,
    M1CompletedStepOutcomeV1, M1DeviceKvArenaLeaseBindingFailureV1, M1DeviceKvCompletionRosterV1,
    M1DeviceModelMemoryAllocationFailureV1, M1FullStepKvWorkspaceTablesV1,
    M1FullStepWorkspaceCompositionFailure, M1FullStepWorkspaceCompositionOutcome,
    M1FullStepWorkspacePlans, M1LongLivedQueueRearmSubmissionFailureV1,
    M1PhysicalDispatchRecipeErrorV1, M1PhysicalProgramCatalogErrorV1,
    M1PhysicalPublishedQueueSessionV1, M1PhysicalQueueCreateFailureClassV1,
    M1PhysicalQueueCreateFailureV1, M1PhysicalQueueOperationFailureV1, M1PhysicalQueueSessionV1,
    M1PrepareFailureV1, M1PreparedLongLivedQueueRearmV1, M1PrepublicationAllocationFailureV1,
    M1PrepublicationBatchBuildFailureV1, M1RearmedPublishedQueueV1, M1ReleasedCompletedStepV1,
    M1ScheduledDispatchV1, M1StepDispatchCompositionError, M1StepDispatchIntent,
    OperationKernelPlanError, OperationKernelPlanFailure, OperationKernelPlanOutcome,
};

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

/// Exact Ferric-owned physical runner authority for the admitted M1 artifacts.
///
/// The persisted bytes remain owned here while the structural operation plan
/// retains the sole published declaration. Program envelopes can only borrow
/// those bytes, so an artifact owner cannot be separated from a later rearm.
#[must_use = "physical runner artifact and declaration custody must remain retained"]
#[derive(Debug)]
pub struct M1PhysicalRunnerV1 {
    artifacts: AdmittedPersistedM1KernelArtifactsV1,
    operations: DeclaredOperationKernelPlan,
}

impl M1PhysicalRunnerV1 {
    /// Exact canonical persisted K1-K7 manifest identity.
    #[must_use]
    pub const fn kernel_artifact_manifest_id(&self) -> Identity {
        Identity::new(*self.artifacts.manifest().identity().sha256())
    }

    /// Exact content-bound twelve-program catalog identity.
    #[must_use]
    pub const fn program_catalog_id(&self) -> Identity {
        self.artifacts.program_catalog_id()
    }

    /// Exact generated runner declaration identity.
    #[must_use]
    pub const fn declaration_id(&self) -> Identity {
        self.operations.runner_declaration_id()
    }

    /// Exact generated structural K1-K7 catalog identity.
    #[must_use]
    pub const fn kernel_catalog_id(&self) -> Identity {
        self.operations.kernel_catalog_id()
    }

    /// Exact generated operation count retained by physical runner custody.
    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.operations.operations().len()
    }

    /// Borrows the logical declaration used for request-local plan binding.
    #[must_use]
    pub const fn logical_runner(&self) -> &LogicalRunnerDeclaration {
        self.operations.runner()
    }

    /// Revalidates and lends the exact persisted physical program catalog for
    /// one lexical use. The callback result cannot retain a borrowed envelope.
    ///
    /// # Errors
    ///
    /// Returns the exact content-bound catalog diagnostic before invoking the
    /// callback.
    pub fn with_program_catalog<R>(
        &self,
        use_catalog: impl for<'catalog> FnOnce(ContentBoundM1ProgramCatalogV1<'catalog>) -> R,
    ) -> Result<R, M1PhysicalProgramCatalogErrorV1> {
        self.artifacts
            .with_content_bound_program_catalog_v1(use_catalog)
    }

    /// Derives the complete addressless physical recipe for one admitted step.
    ///
    /// Every rejection variant retains all linear structural inputs consumed up
    /// to that exact phase.
    pub fn derive_step_recipe(
        &self,
        intent: M1StepDispatchIntent,
        workspace_plans: M1FullStepWorkspacePlans,
    ) -> M1PhysicalRunnerRecipeOutcomeV1 {
        derive_physical_step_recipe(&self.operations, intent, workspace_plans)
    }

    /// Joins scheduler authority, exact generated plan identities, and pending
    /// KV reservations to complete initialized workspace images.
    ///
    /// # Errors
    ///
    /// Returns the existing join or image-composition failure with its exact
    /// scheduler, plan, table, and reservation custody.
    pub fn prepare_scheduled_workspaces(
        &self,
        scheduled: M1ScheduledDispatchV1,
        plans: M1FullStepWorkspacePlans,
        tables: M1FullStepKvWorkspaceTablesV1,
    ) -> Result<crate::M1PreparedScheduledWorkspaceImagesV1, M1PrepareFailureV1> {
        prepare_m1_scheduled_workspace_images_v1(scheduled, self.logical_runner(), plans, tables)
    }

    /// Allocates every prepared workspace on the initialized service device
    /// while retaining scheduler, KV, and model-memory custody as one owner.
    ///
    /// # Errors
    ///
    /// Returns the existing allocation failure with all recoverable physical
    /// and scheduler owners.
    pub fn allocate_scheduled_workspaces(
        &self,
        partitioned_memory: crate::M1PartitionedModelMemoryKvPoolV1,
        prepared: crate::M1PreparedScheduledWorkspaceImagesV1,
    ) -> Result<crate::M1AllocatedScheduledStepV1, M1PrepublicationAllocationFailureV1> {
        allocate_m1_prepublication_workspaces_v1(partitioned_memory, prepared)
    }

    /// Revalidates the persisted catalog, binds all physical owners into a fixed
    /// batch, creates one queue generation, and publishes it exactly once.
    ///
    /// Catalog and batch-construction rejection retain the exact unpublished
    /// allocation inputs. Queue failures retain the existing retry or terminal
    /// quarantine owners. Any terminal queue effect permanently faults `engine`.
    ///
    /// # Errors
    ///
    /// Returns phase-local catalog, batch, queue-creation, or submission
    /// failure custody. Terminal queue effects also quarantine `engine`.
    #[allow(clippy::too_many_arguments)]
    pub fn publish_first_step<'runner, const C: usize>(
        &'runner self,
        engine: &mut Engine<C>,
        ring_bytes: u32,
        allocated: crate::M1AllocatedScheduledStepV1,
        recipe: AddresslessM1PhysicalBufferRecipeV1,
        completion_output: BoundM1CompletionOutputV1,
    ) -> Result<M1PhysicalPublishedQueueSessionV1, M1PhysicalRunnerFirstPublicationFailureV1<'runner>>
    {
        let catalog = match self.artifacts.content_bound_program_catalog_v1() {
            Ok(catalog) => catalog,
            Err(error) => {
                return Err(M1PhysicalRunnerFirstPublicationFailureV1::Catalog {
                    error,
                    allocated: Box::new(allocated),
                    recipe: Box::new(recipe),
                    completion_output: Box::new(completion_output),
                })
            }
        };
        let batch =
            match build_m1_prepublication_batch_v1(allocated, recipe, completion_output, catalog) {
                Ok(batch) => batch,
                Err(failure) => {
                    return Err(M1PhysicalRunnerFirstPublicationFailureV1::Batch(Box::new(
                        failure,
                    )))
                }
            };
        let queue = match M1PhysicalQueueSessionV1::create(ring_bytes, batch) {
            Ok(queue) => queue,
            Err(failure) => {
                if failure.class() == M1PhysicalQueueCreateFailureClassV1::Terminal {
                    engine.quarantine_m1_queue_rearm_failure();
                }
                return Err(M1PhysicalRunnerFirstPublicationFailureV1::Create(Box::new(
                    failure,
                )));
            }
        };
        match queue.submit() {
            Ok(published) => Ok(published),
            Err(failure) => {
                engine.quarantine_m1_queue_rearm_failure();
                Err(M1PhysicalRunnerFirstPublicationFailureV1::Submit(Box::new(
                    failure,
                )))
            }
        }
    }

    /// Drives one published generation through wait, recycle, exact readback,
    /// Engine completion fan-out, and retired-page release.
    pub fn complete_first_step<const C: usize>(
        &self,
        engine: &mut Engine<C>,
        published: M1PhysicalPublishedQueueSessionV1,
        polls: u32,
        semantics: &[crate::CompletionWireSemanticExpectation<'_>],
        roster: M1DeviceKvCompletionRosterV1,
    ) -> M1PhysicalRunnerFirstCompletionOutcomeV1 {
        let completed = match published.wait(polls) {
            Ok(completed) => completed,
            Err(failure) => {
                engine.quarantine_m1_queue_rearm_failure();
                return M1PhysicalRunnerFirstCompletionOutcomeV1::QueueQuarantined {
                    stage: M1PhysicalRunnerQueueFailureStageV1::Wait,
                    failure: Box::new(failure),
                };
            }
        };
        let recycled = match completed.recycle() {
            Ok(recycled) => recycled,
            Err(failure) => {
                engine.quarantine_m1_queue_rearm_failure();
                return M1PhysicalRunnerFirstCompletionOutcomeV1::QueueQuarantined {
                    stage: M1PhysicalRunnerQueueFailureStageV1::Recycle,
                    failure: Box::new(failure),
                };
            }
        };
        let readback = match recycled.read_and_check_completion(semantics) {
            Ok(readback) => readback,
            Err(failure) => {
                return M1PhysicalRunnerFirstCompletionOutcomeV1::ReadbackRejected(Box::new(
                    failure,
                ))
            }
        };
        match complete_m1_physical_step_v1(engine, readback, roster) {
            M1CompletedStepOutcomeV1::Completed(completed) => {
                match release_m1_completed_step_kv_pages_v1(completed) {
                    Ok(released) => M1PhysicalRunnerFirstCompletionOutcomeV1::Released(released),
                    Err(failure) => {
                        M1PhysicalRunnerFirstCompletionOutcomeV1::PageReleaseRejected(failure)
                    }
                }
            }
            outcome => M1PhysicalRunnerFirstCompletionOutcomeV1::CompletionNotCommitted(outcome),
        }
    }

    /// Revalidates persisted K1-K7 bytes and submits one prepared same-shape
    /// target-only or greedy-speculative long-lived queue continuation.
    ///
    /// # Errors
    ///
    /// Returns either the retryable catalog inputs or the existing terminal
    /// rearm-submission failure after quarantining through the lower path.
    pub fn submit_rearm<'runner, const C: usize>(
        &'runner self,
        engine: &mut Engine<C>,
        prepared: M1PreparedLongLivedQueueRearmV1,
        recipe: AddresslessM1PhysicalBufferRecipeV1,
    ) -> Result<M1RearmedPublishedQueueV1, M1PhysicalRunnerRearmSubmissionFailureV1<'runner>> {
        let catalog = match self.artifacts.content_bound_program_catalog_v1() {
            Ok(catalog) => catalog,
            Err(error) => {
                return Err(M1PhysicalRunnerRearmSubmissionFailureV1::Catalog {
                    error,
                    prepared: Box::new(prepared),
                    recipe: Box::new(recipe),
                })
            }
        };
        submit_m1_long_lived_queue_rearm_v1(engine, prepared, recipe, catalog).map_err(|failure| {
            M1PhysicalRunnerRearmSubmissionFailureV1::Submission(Box::new(failure))
        })
    }
}

fn derive_physical_step_recipe(
    operations: &DeclaredOperationKernelPlan,
    intent: M1StepDispatchIntent,
    workspace_plans: M1FullStepWorkspacePlans,
) -> M1PhysicalRunnerRecipeOutcomeV1 {
    let step = match derive_m1_step_dispatch_plan(operations, intent) {
        Ok(step) => step,
        Err(error) => {
            return M1PhysicalRunnerRecipeOutcomeV1::Rejected(
                M1PhysicalRunnerRecipeFailureV1::Step {
                    error,
                    workspace_plans,
                },
            )
        }
    };
    let physical = match derive_m1_physical_dispatch_recipe_v1(&step) {
        Ok(physical) => physical,
        Err(error) => {
            return M1PhysicalRunnerRecipeOutcomeV1::Rejected(
                M1PhysicalRunnerRecipeFailureV1::Dispatch {
                    error,
                    step: Box::new(step),
                    workspace_plans,
                },
            )
        }
    };
    let kernargs = match derive_m1_physical_kernarg_recipe_v1(physical) {
        Ok(kernargs) => kernargs,
        Err(failure) => {
            return M1PhysicalRunnerRecipeOutcomeV1::Rejected(
                M1PhysicalRunnerRecipeFailureV1::Kernarg {
                    failure,
                    step: Box::new(step),
                    workspace_plans,
                },
            )
        }
    };
    let workspaces = match compose_addressless_m1_full_step_workspaces(step, workspace_plans) {
        M1FullStepWorkspaceCompositionOutcome::Composed(workspaces) => workspaces,
        M1FullStepWorkspaceCompositionOutcome::Rejected(failure) => {
            return M1PhysicalRunnerRecipeOutcomeV1::Rejected(
                M1PhysicalRunnerRecipeFailureV1::Workspace {
                    failure,
                    kernargs: Box::new(kernargs),
                },
            )
        }
    };
    match derive_m1_physical_buffer_recipe_v1(kernargs, workspaces) {
        Ok(recipe) => M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe),
        Err(failure) => M1PhysicalRunnerRecipeOutcomeV1::Rejected(
            M1PhysicalRunnerRecipeFailureV1::Buffer(failure),
        ),
    }
}

/// Binding failure retaining the exact persisted artifacts and publication.
#[must_use = "runner binding rejection retains every exact input"]
#[derive(Debug)]
pub enum M1PhysicalRunnerBindFailureV1 {
    /// Canonical operation derivation rejected before consuming the publication.
    Canonical {
        error: OperationKernelPlanError,
        artifacts: Box<AdmittedPersistedM1KernelArtifactsV1>,
        runner: Box<LogicalRunnerDeclaration>,
    },
    /// Independent structural validation rejected the generated bindings.
    Structural {
        artifacts: Box<AdmittedPersistedM1KernelArtifactsV1>,
        failure: Box<OperationKernelPlanFailure>,
    },
}

/// Binds persisted K1-K7 bytes to the exact generated operation roster.
///
/// Family build, artifact, and ABI identities are derived only from the
/// persisted compiler-handoff, HSACO, and symbol-manifest digests. The complete
/// 10,648-operation binding roster is then independently revalidated.
///
/// # Errors
///
/// Returns canonical derivation or independent structural binding failure with
/// the exact persisted artifact and declaration owners.
pub fn bind_m1_physical_runner_v1(
    artifacts: AdmittedPersistedM1KernelArtifactsV1,
    publication: PublishedRunnerDeclaration,
) -> Result<M1PhysicalRunnerV1, M1PhysicalRunnerBindFailureV1> {
    let runner = LogicalRunnerDeclaration::from_published(publication);
    let families = physical_family_artifacts(&artifacts);
    let operations = match derive_canonical_operation_bindings(&runner, &families) {
        Ok(operations) => operations,
        Err(error) => {
            return Err(M1PhysicalRunnerBindFailureV1::Canonical {
                error,
                artifacts: Box::new(artifacts),
                runner: Box::new(runner),
            })
        }
    };
    match bind_declared_operation_kernel_plan(runner, families, operations) {
        OperationKernelPlanOutcome::Bound(operations) => Ok(M1PhysicalRunnerV1 {
            artifacts,
            operations,
        }),
        OperationKernelPlanOutcome::Rejected(failure) => {
            Err(M1PhysicalRunnerBindFailureV1::Structural {
                artifacts: Box::new(artifacts),
                failure: Box::new(failure),
            })
        }
    }
}

fn physical_family_artifacts(
    artifacts: &AdmittedPersistedM1KernelArtifactsV1,
) -> Box<[DeclaredKernelFamilyArtifact]> {
    artifacts
        .manifest()
        .entries()
        .iter()
        .map(|entry| {
            DeclaredKernelFamilyArtifact::new(
                kernel_family(entry.family()),
                Identity::new(*entry.compiler_handoff().sha256()),
                Identity::new(*entry.artifact().sha256()),
                Identity::new(*entry.symbol_manifest().sha256()),
            )
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

const fn kernel_family(family: M1KernelArtifactFamilyV1) -> KernelFamily {
    match family {
        M1KernelArtifactFamilyV1::Gemm => KernelFamily::K1GemmGemv,
        M1KernelArtifactFamilyV1::RmsNorm => KernelFamily::K2RmsNormResidual,
        M1KernelArtifactFamilyV1::RopeKv => KernelFamily::K3RopePagedKv,
        M1KernelArtifactFamilyV1::Prefill => KernelFamily::K4GqaPrefill,
        M1KernelArtifactFamilyV1::PagedDecode => KernelFamily::K5PagedGqaDecode,
        M1KernelArtifactFamilyV1::SwiGlu => KernelFamily::K6SwiGlu,
        M1KernelArtifactFamilyV1::Logits => KernelFamily::K7LogitsCompact,
    }
}

/// Checked-device, initialized model-memory, or KV-partition failure.
#[must_use = "physical memory initialization failure retains lower recovery custody"]
#[derive(Debug)]
pub enum M1PhysicalRunnerMemoryFailureV1 {
    Device(crate::M1CheckedGfx942ServiceDeviceAcquireFailureV1),
    Model(M1DeviceModelMemoryAllocationFailureV1),
    Kv(M1DeviceKvArenaLeaseBindingFailureV1),
}

/// Acquires one admitted gfx942 device, initializes target/draft model memory,
/// and consumes both KV arenas into their exact physical page partitions.
///
/// # Errors
///
/// Returns the exact checked-device, initialized-model-memory, or KV partition
/// failure with the lower layer's recovery or quarantine custody.
pub fn initialize_m1_physical_runner_memory_v1(
    checked: CheckedGfx942XnackMinusDevice,
    plan: AddresslessModelMemoryPlan,
    target_prepacked_weights: Box<[u8]>,
    draft_prepacked_weights: Box<[u8]>,
) -> Result<crate::M1PartitionedModelMemoryKvPoolV1, M1PhysicalRunnerMemoryFailureV1> {
    let device = crate::acquire_m1_checked_gfx942_service_device_v1(checked)
        .map_err(M1PhysicalRunnerMemoryFailureV1::Device)?;
    let initialized = allocate_initialized_m1_model_memory_on_device_v1(
        device,
        plan,
        target_prepacked_weights,
        draft_prepacked_weights,
    )
    .map_err(M1PhysicalRunnerMemoryFailureV1::Model)?;
    bind_m1_partitioned_model_memory_kv_pool_v1(initialized)
        .map_err(M1PhysicalRunnerMemoryFailureV1::Kv)
}

/// Exact phase-local recovery from addressless physical recipe derivation.
#[must_use = "recipe rejection retains all consumed linear inputs"]
#[derive(Debug)]
pub enum M1PhysicalRunnerRecipeFailureV1 {
    Step {
        error: M1StepDispatchCompositionError,
        workspace_plans: M1FullStepWorkspacePlans,
    },
    Dispatch {
        error: M1PhysicalDispatchRecipeErrorV1,
        step: Box<AddresslessM1StepDispatchPlan>,
        workspace_plans: M1FullStepWorkspacePlans,
    },
    Kernarg {
        failure: crate::M1PhysicalKernargRecipeFailureV1,
        step: Box<AddresslessM1StepDispatchPlan>,
        workspace_plans: M1FullStepWorkspacePlans,
    },
    Workspace {
        failure: M1FullStepWorkspaceCompositionFailure,
        kernargs: Box<AddresslessM1PhysicalKernargRecipeV1>,
    },
    Buffer(crate::M1PhysicalBufferRecipeFailureV1),
}

/// Complete addressless physical recipe or exact phase-local recovery.
#[must_use]
#[derive(Debug)]
pub enum M1PhysicalRunnerRecipeOutcomeV1 {
    Prepared(AddresslessM1PhysicalBufferRecipeV1),
    Rejected(M1PhysicalRunnerRecipeFailureV1),
}

/// First-publication failure retaining exact retry or quarantine custody.
#[must_use = "publication failure retains all available physical owners"]
#[derive(Debug)]
pub enum M1PhysicalRunnerFirstPublicationFailureV1<'a> {
    Catalog {
        error: M1PhysicalProgramCatalogErrorV1,
        allocated: Box<crate::M1AllocatedScheduledStepV1>,
        recipe: Box<AddresslessM1PhysicalBufferRecipeV1>,
        completion_output: Box<BoundM1CompletionOutputV1>,
    },
    Batch(Box<M1PrepublicationBatchBuildFailureV1<'a>>),
    Create(Box<M1PhysicalQueueCreateFailureV1<'a>>),
    Submit(Box<M1PhysicalQueueOperationFailureV1>),
}

/// Terminal generic queue phase for first-generation execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalRunnerQueueFailureStageV1 {
    Wait,
    Recycle,
}

/// Exhaustive first-generation completion result.
#[must_use = "completion outcome retains exact queue, cache, and release custody"]
#[derive(Debug)]
pub enum M1PhysicalRunnerFirstCompletionOutcomeV1 {
    Released(M1ReleasedCompletedStepV1),
    QueueQuarantined {
        stage: M1PhysicalRunnerQueueFailureStageV1,
        failure: Box<M1PhysicalQueueOperationFailureV1>,
    },
    ReadbackRejected(Box<M1CompletedReadbackJoinFailureV1>),
    CompletionNotCommitted(M1CompletedStepOutcomeV1),
    PageReleaseRejected(Box<M1CompletedStepKvReleaseFailureV1>),
}

/// Catalog rejection or existing terminal rearm-submission quarantine.
#[must_use = "rearm failure retains every prepared queue and recipe owner"]
#[derive(Debug)]
pub enum M1PhysicalRunnerRearmSubmissionFailureV1<'a> {
    Catalog {
        error: M1PhysicalProgramCatalogErrorV1,
        prepared: Box<M1PreparedLongLivedQueueRearmV1>,
        recipe: Box<AddresslessM1PhysicalBufferRecipeV1>,
    },
    Submission(Box<M1LongLivedQueueRearmSubmissionFailureV1<'a>>),
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
    use super::{
        checked_operation_range, derive_physical_step_recipe, find_plan, LogicalRunnerError,
        M1PhysicalRunnerRecipeOutcomeV1,
    };
    use ferric_build::{
        m1_step_workspace_requirements, plan_addressless_m1_step_workspace,
        AddresslessM1StepWorkspacePlan, AvailableM1StepWorkspace,
        DeclaredM1StepWorkspaceAllocation, GeneratedPlanDeclaration, M1StepWorkspaceDeclaration,
        M1StepWorkspacePlanOutcome,
    };
    use ferric_spec::{
        Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection,
    };

    use crate::operation_kernel_plan::tests::public_operation_kernel_plan_fixture;
    use crate::{M1FullStepWorkspacePlans, M1StepDispatchIntent};

    const fn selection(bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket,
        }
    }

    fn plan(
        plan_index: u8,
        bucket: Qwen3PlanBucket,
        operation_start: u32,
        operation_count: u32,
    ) -> GeneratedPlanDeclaration {
        GeneratedPlanDeclaration {
            plan_index: u16::from(plan_index),
            plan_id: Identity::new([plan_index + 1; 32]),
            selection: selection(bucket),
            operation_start,
            operation_count,
        }
    }

    fn workspace_plan(
        selection: Qwen3PlanSelection,
        identity_byte: u8,
    ) -> AddresslessM1StepWorkspacePlan {
        let requirements = m1_step_workspace_requirements(selection).unwrap();
        let available = AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
            selection,
            DeclaredM1StepWorkspaceAllocation::new(
                Identity::new([identity_byte; 32]),
                requirements.allocation_byte_len(),
                requirements.allocation_alignment(),
            ),
            requirements.ranges().to_vec().into_boxed_slice(),
        ));
        match plan_addressless_m1_step_workspace(selection, available) {
            M1StepWorkspacePlanOutcome::Planned(plan) => plan,
            M1StepWorkspacePlanOutcome::Rejected(_) => panic!("exact workspace fixture rejected"),
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

    #[test]
    fn physical_recipe_composes_target_only_and_greedy_speculative_shapes() {
        let operations = public_operation_kernel_plan_fixture();
        let target_decode = Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        };
        let target = derive_physical_step_recipe(
            &operations,
            M1StepDispatchIntent::TargetOnly(target_decode),
            M1FullStepWorkspacePlans::target_only(workspace_plan(target_decode, 40)),
        );
        let M1PhysicalRunnerRecipeOutcomeV1::Prepared(target) = target else {
            panic!("canonical target-only recipe rejected");
        };
        assert_eq!(target.rows().len(), 545);
        assert!(!target.requires_future_materialization());
        assert!(!target.binds_device_memory());

        let target_speculative = Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
        };
        let draft_decode = Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        };
        let speculative = derive_physical_step_recipe(
            &operations,
            M1StepDispatchIntent::SpeculativeRound(target_speculative),
            M1FullStepWorkspacePlans::speculative_round(
                workspace_plan(draft_decode, 41),
                workspace_plan(target_speculative, 42),
            ),
        );
        let M1PhysicalRunnerRecipeOutcomeV1::Prepared(speculative) = speculative else {
            panic!("canonical greedy-speculative recipe rejected");
        };
        assert_eq!(speculative.rows().len(), 2_242);
        assert!(!speculative.requires_future_materialization());
        assert!(!speculative.binds_device_memory());
    }

    #[test]
    #[ignore = "requires admitted K1-K7 artifacts, prepacked Qwen bytes, and an exclusive MI300X"]
    fn configured_mi300x_target_only_dispatch_smoke_is_not_numerical_evidence() {
        use std::fs;

        use fe2o3_kfd::{DeviceSelector, OpenedKfd};
        use ferric_build::{
            generate_qwen3_gfx942_runner_declaration, publish_qwen3_gfx942_runner_declaration,
            qwen3_model_memory_plan_test_fixture, qwen3_runner_closure_test_fixture,
        };
        use ferric_spec::{
            validate_m1_step_inputs, M1StepInputCandidate, M1StepInputValidationOutcome,
            ValidatedM1StepInputs,
        };

        use super::{bind_m1_physical_runner_v1, initialize_m1_physical_runner_memory_v1};
        use crate::{
            bind_m1_kv_workspace_table_v1, reopen_persisted_m1_kernel_artifacts_v1,
            ActiveDeviceKvCache, Engine, M1FullStepKvWorkspaceTablesV1,
            M1PhysicalFixedBatchShapeV1,
        };

        fn required_path(name: &str) -> std::path::PathBuf {
            std::env::var_os(name).map_or_else(|| panic!("set {name}"), std::path::PathBuf::from)
        }

        fn one_decode_input(plan: ferric_spec::StepPlan) -> ValidatedM1StepInputs {
            let candidate = M1StepInputCandidate::new(
                plan.selection(),
                vec![Some(plan)],
                vec![1],
                vec![0],
                vec![1],
                vec![0],
            );
            match validate_m1_step_inputs(candidate) {
                M1StepInputValidationOutcome::Validated(inputs) => inputs,
                M1StepInputValidationOutcome::Rejected(failure) => {
                    panic!(
                        "one-token target decode input rejected: {:?}",
                        failure.error()
                    )
                }
            }
        }

        // Required environment contract for this ignored hardware smoke:
        // FERRIC_M1_KERNEL_ARTIFACT_DIRECTORY
        // FERRIC_M1_TARGET_PREPACKED_WEIGHTS
        // FERRIC_M1_DRAFT_PREPACKED_WEIGHTS
        // FERRIC_M1_GPU_UNIQUE_ID
        let artifact_directory = required_path("FERRIC_M1_KERNEL_ARTIFACT_DIRECTORY");
        let target_weights = required_path("FERRIC_M1_TARGET_PREPACKED_WEIGHTS");
        let draft_weights = required_path("FERRIC_M1_DRAFT_PREPACKED_WEIGHTS");
        let unique_id = std::env::var("FERRIC_M1_GPU_UNIQUE_ID")
            .expect("set FERRIC_M1_GPU_UNIQUE_ID to the selected MI300X unique ID")
            .parse::<u64>()
            .expect("FERRIC_M1_GPU_UNIQUE_ID must be a decimal u64");

        let artifacts = reopen_persisted_m1_kernel_artifacts_v1(artifact_directory)
            .expect("admit persisted K1-K7 artifacts");
        let declaration =
            generate_qwen3_gfx942_runner_declaration(qwen3_runner_closure_test_fixture())
                .expect("generate fixture structural publication");
        let publication = publish_qwen3_gfx942_runner_declaration(declaration)
            .expect("publish fixture structural declaration");
        let runner = bind_m1_physical_runner_v1(artifacts, publication)
            .expect("bind persisted kernels to canonical operations");

        let checked = OpenedKfd::open_default()
            .expect("open KFD")
            .admit_uapi()
            .expect("admit pinned KFD UAPI")
            .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(unique_id))
            .expect("bind checked gfx942:xnack- MI300X");
        let mut memory = initialize_m1_physical_runner_memory_v1(
            checked,
            qwen3_model_memory_plan_test_fixture(),
            fs::read(target_weights)
                .expect("read target prepacked bytes")
                .into_boxed_slice(),
            fs::read(draft_weights)
                .expect("read draft prepacked bytes")
                .into_boxed_slice(),
        )
        .expect("initialize target/draft memory and partition KV arenas");

        let selection = Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        };
        let draft_selection = Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        };
        let mut engine = Engine::<1>::new(512, 256, 8_192).expect("construct one-lane engine");
        let request = engine.admit().expect("admit one request");
        engine
            .append_tentative(request, 1)
            .expect("append one token");
        let scheduled = engine
            .dispatch_m1_ready()
            .expect("schedule one M1 batch")
            .expect("one request is ready");
        let step_plan = runner
            .logical_runner()
            .bind_step_plan(request, scheduled.epoch(), selection)
            .expect("bind target decode plan");
        let inputs = one_decode_input(step_plan);

        let mut cache =
            ActiveDeviceKvCache::new(memory.device(), request, selection, draft_selection)
                .expect("construct request-local device cache");
        let lease = memory
            .lease_page(request, Qwen3ModelRole::Target8B, 0)
            .expect("lease first target page");
        let pending = cache
            .reserve_step_write(
                request,
                Qwen3ModelRole::Target8B,
                0,
                1,
                scheduled.epoch(),
                vec![lease],
            )
            .expect("reserve one target decode write");
        let table = bind_m1_kv_workspace_table_v1(inputs, vec![pending])
            .expect("bind physical target page table");
        let tables = M1FullStepKvWorkspaceTablesV1::TargetOnly { target: table };
        let prepared = runner
            .prepare_scheduled_workspaces(
                scheduled,
                M1FullStepWorkspacePlans::target_only(workspace_plan(selection, 90)),
                tables,
            )
            .expect("prepare scheduler-bound workspace image");
        let completion = memory
            .allocate_completion_output(selection)
            .expect("allocate completion output");
        let allocated = runner
            .allocate_scheduled_workspaces(memory, prepared)
            .expect("allocate initialized target workspace");
        let recipe = match runner.derive_step_recipe(
            M1StepDispatchIntent::TargetOnly(selection),
            M1FullStepWorkspacePlans::target_only(workspace_plan(selection, 90)),
        ) {
            M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
            M1PhysicalRunnerRecipeOutcomeV1::Rejected(failure) => {
                panic!("derive exact physical recipe: {failure:?}")
            }
        };
        let published = runner
            .publish_first_step(&mut engine, 1 << 20, allocated, recipe, completion)
            .expect("create queue and publish exactly once");
        let completed = published.wait(20_000_000).expect("wait for live dispatch");
        let recycled = completed.recycle().expect("recycle live dispatch queue");
        assert_eq!(recycled.shape(), M1PhysicalFixedBatchShapeV1::TargetOnly);
        let _release = recycled
            .destroy_and_release()
            .expect("destroy queue and release model/workspace storage");

        // The fixture publication and memory plan are structural test
        // authorities. This smoke proves only that the admitted queue ran to a
        // completion signal; it does not authenticate a deployment or assert
        // token values, numerical correctness, performance, or refinement.
        drop(cache);
    }
}
