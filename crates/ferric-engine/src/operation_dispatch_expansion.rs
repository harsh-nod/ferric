//! Addressless expansion from logical M1 operations to physical dispatch rows.
//!
//! This layer preserves the existing physical-plan V1 contract. It records
//! only the extra one-to-many fact that target K7 lowers to argmax followed by
//! compact completion, while draft K7 remains argmax-only and every other
//! generated operation remains one dispatch. Rows retain the already checked
//! operation/kernel declarations and no caller-provided row count is trusted.
//!
//! The declarations here contain no address, kernarg bytes, packet, artifact
//! authentication, allocation, queue, launch, completion, readback, hardware,
//! performance, inference-result, or refinement authority.

use ferric_kernels::KernelFamily;
use ferric_qwen_kernels::logits::{
    Qwen3LogitsBucketKindV1, Qwen3LogitsCompletionKindV1, Qwen3LogitsModeV1,
    Qwen3LogitsModelRoleV1, Qwen3LogitsProfileCatalogV1,
};
use ferric_spec::{
    plan_step_count, Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3Operator, Qwen3PlanBucket,
    Qwen3PlanSelection,
};
use sha2::{Digest, Sha256};
use std::fmt;

use crate::{DeclaredOperationKernelBinding, DeclaredOperationKernelPlan, LogicalRunnerError};

const EXPANSION_IDENTITY_DOMAIN: &[u8] = b"ferric.m1.operation-dispatch-expansion.v1";

/// Canonical declaration version for logical-operation dispatch expansion.
pub const M1_OPERATION_DISPATCH_EXPANSION_VERSION: u32 = 1;
/// Conservative maximum dispatch rows in one M1 operation expansion.
pub const M1_MAX_OPERATION_DISPATCHES_V1: u32 = 1_024;

/// Exact subdispatch represented by one addressless row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1OperationDispatchKind {
    /// One complete non-K7 generated operation.
    WholeOperation,
    /// K7 lowest-ID argmax over the selected finite logits profile.
    K7Argmax,
    /// Target-only K7 compact completion after its immediately preceding argmax.
    K7Compact,
}

impl M1OperationDispatchKind {
    const fn tag(self) -> u8 {
        match self {
            Self::WholeOperation => 1,
            Self::K7Argmax => 2,
            Self::K7Compact => 3,
        }
    }
}

/// One exact addressless physical-dispatch row.
///
/// The retained operation binding is inert identity data already admitted by
/// [`DeclaredOperationKernelPlan`]. The row adds no executable or packet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1OperationDispatchRow {
    dispatch_index: u32,
    logical_ordinal: u32,
    operator: Qwen3Operator,
    operation: DeclaredOperationKernelBinding,
    kind: M1OperationDispatchKind,
}

impl M1OperationDispatchRow {
    /// Constructs unvalidated inert row data.
    #[must_use]
    pub const fn new(
        dispatch_index: u32,
        logical_ordinal: u32,
        operator: Qwen3Operator,
        operation: DeclaredOperationKernelBinding,
        kind: M1OperationDispatchKind,
    ) -> Self {
        Self {
            dispatch_index,
            logical_ordinal,
            operator,
            operation,
            kind,
        }
    }

    /// Returns the zero-based physical row position.
    #[must_use]
    pub const fn dispatch_index(&self) -> u32 {
        self.dispatch_index
    }

    /// Returns the zero-based operation ordinal within the selected plan.
    #[must_use]
    pub const fn logical_ordinal(&self) -> u32 {
        self.logical_ordinal
    }

    /// Returns the exact generated graph operator.
    #[must_use]
    pub const fn operator(&self) -> Qwen3Operator {
        self.operator
    }

    /// Returns the retained exact operation/kernel identity binding.
    #[must_use]
    pub const fn operation(&self) -> &DeclaredOperationKernelBinding {
        &self.operation
    }

    /// Returns the exact whole-operation or K7 subdispatch role.
    #[must_use]
    pub const fn kind(&self) -> M1OperationDispatchKind {
        self.kind
    }

    /// A row is addressless declaration data and grants no dispatch authority.
    #[must_use]
    pub const fn grants_dispatch_authority(&self) -> bool {
        false
    }
}

/// Versioned caller declaration of one exact operation-dispatch expansion.
///
/// This owner intentionally does not implement `Clone`.
///
/// ```compile_fail
/// use ferric_engine::DeclaredM1OperationDispatchExpansion;
/// fn require_clone<T: Clone>() {}
/// require_clone::<DeclaredM1OperationDispatchExpansion>();
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct DeclaredM1OperationDispatchExpansion {
    version: u32,
    expansion_id: Identity,
    selection: Qwen3PlanSelection,
    runner_declaration_id: Identity,
    kernel_catalog_id: Identity,
    logical_operation_count: u32,
    physical_dispatch_count: u32,
    rows: Box<[M1OperationDispatchRow]>,
}

impl DeclaredM1OperationDispatchExpansion {
    /// Constructs unvalidated inert expansion data.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        version: u32,
        expansion_id: Identity,
        selection: Qwen3PlanSelection,
        runner_declaration_id: Identity,
        kernel_catalog_id: Identity,
        logical_operation_count: u32,
        physical_dispatch_count: u32,
        rows: Box<[M1OperationDispatchRow]>,
    ) -> Self {
        Self {
            version,
            expansion_id,
            selection,
            runner_declaration_id,
            kernel_catalog_id,
            logical_operation_count,
            physical_dispatch_count,
            rows,
        }
    }

    /// Returns the declared format version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Returns the declared domain-separated expansion identity.
    #[must_use]
    pub const fn expansion_id(&self) -> Identity {
        self.expansion_id
    }

    /// Returns the declared role, mode, and bucket.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Returns the retained generated-runner declaration identity.
    #[must_use]
    pub const fn runner_declaration_id(&self) -> Identity {
        self.runner_declaration_id
    }

    /// Returns the retained structural K1-K7 catalog identity.
    #[must_use]
    pub const fn kernel_catalog_id(&self) -> Identity {
        self.kernel_catalog_id
    }

    /// Returns the declared logical operation count.
    #[must_use]
    pub const fn logical_operation_count(&self) -> u32 {
        self.logical_operation_count
    }

    /// Returns the declared physical dispatch count.
    #[must_use]
    pub const fn physical_dispatch_count(&self) -> u32 {
        self.physical_dispatch_count
    }

    /// Returns all rows in declared physical order.
    #[must_use]
    pub fn rows(&self) -> &[M1OperationDispatchRow] {
        &self.rows
    }

    /// Recovers every inert declaration field without creating authority.
    #[must_use]
    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        u32,
        Identity,
        Qwen3PlanSelection,
        Identity,
        Identity,
        u32,
        u32,
        Box<[M1OperationDispatchRow]>,
    ) {
        (
            self.version,
            self.expansion_id,
            self.selection,
            self.runner_declaration_id,
            self.kernel_catalog_id,
            self.logical_operation_count,
            self.physical_dispatch_count,
            self.rows,
        )
    }

    /// This identity declaration does not authenticate executable artifacts.
    #[must_use]
    pub const fn authenticates_artifacts(&self) -> bool {
        false
    }
}

/// Identity field that drifted within one retained operation binding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1OperationDispatchIdentityComponent {
    /// Logical plan identity.
    Plan,
    /// Generated-runner declaration identity.
    RunnerDeclaration,
    /// Structural K1-K7 catalog identity.
    KernelCatalog,
    /// Canonical Qwen profile-catalog identity.
    ProfileCatalog,
    /// Canonical Qwen operation-profile identity.
    Profile,
    /// Declared family build identity.
    FamilyBuild,
    /// Declared artifact identity.
    Artifact,
    /// Declared ABI-layout identity.
    AbiLayout,
}

/// Fail-closed derivation or declaration-validation error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1OperationDispatchExpansionError {
    /// The declaration format version drifted.
    Version {
        /// Required version.
        expected: u32,
        /// Rejected version.
        actual: u32,
    },
    /// The declaration names the wrong model role.
    SelectionRole,
    /// The declaration names the wrong execution mode.
    SelectionMode,
    /// The declaration names the wrong finite bucket.
    SelectionBucket,
    /// The selected operation range could not be recovered.
    OperationRange(LogicalRunnerError),
    /// The selected generated operation count is not role-exact.
    LogicalOperationCount {
        /// Required role count.
        expected: u32,
        /// Observed or declared count.
        actual: u32,
    },
    /// Derived physical row count exceeded the reviewed addressless ceiling.
    PhysicalCapacity {
        /// Reviewed maximum.
        maximum: u32,
        /// Rejected derived count.
        actual: u32,
    },
    /// The declaration names the wrong generated-runner identity.
    RunnerDeclarationIdentity,
    /// The declaration names the wrong structural K1-K7 catalog identity.
    KernelCatalogIdentity,
    /// The declared physical count differs from the independently derived count.
    PhysicalDispatchCount {
        /// Required derived count.
        expected: u32,
        /// Rejected declared count.
        actual: u32,
    },
    /// The row slice length differs from the independently derived count.
    RowCount {
        /// Required derived count.
        expected: usize,
        /// Rejected row length.
        actual: usize,
    },
    /// A generated operation selection drifted from the selected plan.
    GeneratedSelection {
        /// Logical ordinal of the rejected operation.
        logical_ordinal: u32,
    },
    /// Generated and bound operation positions no longer agree.
    GeneratedOperationOrder {
        /// Logical ordinal of the rejected operation.
        logical_ordinal: u32,
    },
    /// Generated and bound plan positions no longer agree.
    GeneratedPlanIndex {
        /// Logical ordinal of the rejected operation.
        logical_ordinal: u32,
    },
    /// Generated and bound K1-K7 families no longer agree.
    GeneratedFamily {
        /// Logical ordinal of the rejected operation.
        logical_ordinal: u32,
    },
    /// A generated or bound identity no longer agrees with retained plan custody.
    GeneratedIdentity {
        /// Logical ordinal of the rejected operation.
        logical_ordinal: u32,
        /// Rejected identity boundary.
        component: M1OperationDispatchIdentityComponent,
    },
    /// The canonical logits profile catalog could not be constructed.
    CanonicalLogitsCatalog,
    /// K7 did not resolve to the exact role/bucket canonical logits profile.
    CanonicalLogitsProfile,
    /// K7 retained the wrong canonical logits catalog or profile identity.
    CanonicalLogitsIdentity,
    /// Canonical logits completion behavior disagreed with the selected role.
    CanonicalLogitsCompletion,
    /// A declared row has the wrong physical position.
    DispatchOrder {
        /// Row position being checked.
        position: usize,
        /// Required dispatch index.
        expected: u32,
        /// Rejected dispatch index.
        actual: u32,
    },
    /// A declared row has the wrong logical ordinal.
    LogicalOrder {
        /// Physical row position being checked.
        position: usize,
        /// Required logical ordinal.
        expected: u32,
        /// Rejected logical ordinal.
        actual: u32,
    },
    /// A declared row has the wrong generated operation position.
    OperationOrder {
        /// Physical row position being checked.
        position: usize,
        /// Required generated operation index.
        expected: u32,
        /// Rejected generated operation index.
        actual: u32,
    },
    /// A declared row has the wrong generated plan position.
    PlanIndex {
        /// Physical row position being checked.
        position: usize,
        /// Required plan index.
        expected: u16,
        /// Rejected plan index.
        actual: u16,
    },
    /// A declared row has the wrong graph operator.
    Operator {
        /// Physical row position being checked.
        position: usize,
        /// Required operator.
        expected: Qwen3Operator,
        /// Rejected operator.
        actual: Qwen3Operator,
    },
    /// A declared row has the wrong K1-K7 family.
    Family {
        /// Physical row position being checked.
        position: usize,
        /// Required family.
        expected: KernelFamily,
        /// Rejected family.
        actual: KernelFamily,
    },
    /// A declared row changed one exact retained identity.
    OperationIdentity {
        /// Physical row position being checked.
        position: usize,
        /// Rejected identity boundary.
        component: M1OperationDispatchIdentityComponent,
    },
    /// A declared row has the wrong exact subdispatch.
    Subdispatch {
        /// Physical row position being checked.
        position: usize,
        /// Required role.
        expected: M1OperationDispatchKind,
        /// Rejected role.
        actual: M1OperationDispatchKind,
    },
    /// The declared expansion identity differs from the canonical record.
    ExpansionIdentity,
    /// Checked position or count arithmetic overflowed.
    ArithmeticOverflow,
}

impl fmt::Display for M1OperationDispatchExpansionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 operation dispatch expansion rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1OperationDispatchExpansionError {}

/// Retry-safe rejection retaining both exact linear inputs.
///
/// This owner intentionally does not implement `Clone`.
#[derive(Debug, Eq, PartialEq)]
pub struct M1OperationDispatchExpansionFailure {
    error: M1OperationDispatchExpansionError,
    operation_plan: DeclaredOperationKernelPlan,
    declaration: DeclaredM1OperationDispatchExpansion,
}

impl M1OperationDispatchExpansionFailure {
    /// Returns the fail-closed diagnostic.
    #[must_use]
    pub const fn error(&self) -> M1OperationDispatchExpansionError {
        self.error
    }

    /// Recovers the diagnostic and both unchanged inert inputs.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        M1OperationDispatchExpansionError,
        DeclaredOperationKernelPlan,
        DeclaredM1OperationDispatchExpansion,
    ) {
        (self.error, self.operation_plan, self.declaration)
    }
}

/// Linear engine custody of one exact addressless operation-dispatch plan.
///
/// This value intentionally does not implement `Clone` and grants no packet or
/// execution authority.
///
/// ```compile_fail
/// use ferric_engine::AddresslessM1OperationDispatchPlan;
/// fn require_clone<T: Clone>() {}
/// require_clone::<AddresslessM1OperationDispatchPlan>();
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct AddresslessM1OperationDispatchPlan {
    operation_plan: DeclaredOperationKernelPlan,
    declaration: DeclaredM1OperationDispatchExpansion,
}

impl AddresslessM1OperationDispatchPlan {
    /// Returns the exact finite selection.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.declaration.selection
    }

    /// Returns the deterministic domain-separated declaration identity.
    #[must_use]
    pub const fn expansion_id(&self) -> Identity {
        self.declaration.expansion_id
    }

    /// Returns the exact logical operation count.
    #[must_use]
    pub const fn logical_operation_count(&self) -> u32 {
        self.declaration.logical_operation_count
    }

    /// Returns the exact derived physical dispatch count.
    #[must_use]
    pub const fn physical_dispatch_count(&self) -> u32 {
        self.declaration.physical_dispatch_count
    }

    /// Returns all exact addressless rows in physical order.
    #[must_use]
    pub fn rows(&self) -> &[M1OperationDispatchRow] {
        &self.declaration.rows
    }

    /// Borrows the retained operation/kernel identity plan.
    #[must_use]
    pub const fn operation_plan(&self) -> &DeclaredOperationKernelPlan {
        &self.operation_plan
    }

    /// Aborts addressless planning and recovers both exact inert inputs.
    #[must_use]
    pub fn abort(
        self,
    ) -> (
        DeclaredOperationKernelPlan,
        DeclaredM1OperationDispatchExpansion,
    ) {
        (self.operation_plan, self.declaration)
    }

    /// The addressless plan authenticates no artifact bytes.
    #[must_use]
    pub const fn authenticates_artifacts(&self) -> bool {
        false
    }

    /// The addressless plan grants no packet, queue, or launch authority.
    #[must_use]
    pub const fn grants_execution_authority(&self) -> bool {
        false
    }

    /// Structural expansion proves no machine or operator refinement.
    #[must_use]
    pub const fn proves_refinement(&self) -> bool {
        false
    }
}

/// Exact linear result of one expansion-validation attempt.
#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub enum M1OperationDispatchExpansionOutcome {
    /// The declaration exactly matches independent derivation.
    Planned(AddresslessM1OperationDispatchPlan),
    /// Both unchanged inputs remain recoverable.
    Rejected(M1OperationDispatchExpansionFailure),
}

/// Derives the exact versioned addressless expansion for one selected plan.
///
/// Rows and both counts come only from
/// [`DeclaredOperationKernelPlan::operation_declarations_for`] plus the exact
/// canonical logits profile. No caller count participates in derivation.
///
/// # Errors
///
/// Returns [`M1OperationDispatchExpansionError`] for retained operation drift,
/// canonical K7 profile drift, checked arithmetic overflow, or capacity excess.
pub fn derive_m1_operation_dispatch_expansion(
    operation_plan: &DeclaredOperationKernelPlan,
    selection: Qwen3PlanSelection,
) -> Result<DeclaredM1OperationDispatchExpansion, M1OperationDispatchExpansionError> {
    let rows = derive_rows(operation_plan, selection)?;
    let logical_operation_count = plan_step_count(selection.role);
    let physical_dispatch_count = u32::try_from(rows.len())
        .map_err(|_| M1OperationDispatchExpansionError::ArithmeticOverflow)?;
    let mut declaration = DeclaredM1OperationDispatchExpansion {
        version: M1_OPERATION_DISPATCH_EXPANSION_VERSION,
        expansion_id: Identity::new([0; 32]),
        selection,
        runner_declaration_id: operation_plan.runner_declaration_id(),
        kernel_catalog_id: operation_plan.kernel_catalog_id(),
        logical_operation_count,
        physical_dispatch_count,
        rows,
    };
    declaration.expansion_id = expansion_identity(&declaration);
    Ok(declaration)
}

/// Consumes and validates an inert caller declaration against exact derivation.
///
/// Rejection preserves both inputs. Success remains addressless and does not
/// authenticate artifacts or grant allocation, packet, queue, launch,
/// completion, readback, hardware, performance, or refinement authority.
pub fn plan_m1_operation_dispatch_expansion(
    expected_selection: Qwen3PlanSelection,
    operation_plan: DeclaredOperationKernelPlan,
    declaration: DeclaredM1OperationDispatchExpansion,
) -> M1OperationDispatchExpansionOutcome {
    match validate_declaration(expected_selection, &operation_plan, &declaration) {
        Ok(()) => {
            M1OperationDispatchExpansionOutcome::Planned(AddresslessM1OperationDispatchPlan {
                operation_plan,
                declaration,
            })
        }
        Err(error) => {
            M1OperationDispatchExpansionOutcome::Rejected(M1OperationDispatchExpansionFailure {
                error,
                operation_plan,
                declaration,
            })
        }
    }
}

fn derive_rows(
    operation_plan: &DeclaredOperationKernelPlan,
    selection: Qwen3PlanSelection,
) -> Result<Box<[M1OperationDispatchRow]>, M1OperationDispatchExpansionError> {
    let (generated, bindings) = operation_plan
        .operation_declarations_for(selection)
        .map_err(M1OperationDispatchExpansionError::OperationRange)?;
    let expected_count = plan_step_count(selection.role);
    let actual_count = u32::try_from(generated.len())
        .map_err(|_| M1OperationDispatchExpansionError::ArithmeticOverflow)?;
    if actual_count != expected_count || bindings.len() != generated.len() {
        return Err(M1OperationDispatchExpansionError::LogicalOperationCount {
            expected: expected_count,
            actual: actual_count,
        });
    }
    let logits = Qwen3LogitsProfileCatalogV1::canonical()
        .map_err(|_| M1OperationDispatchExpansionError::CanonicalLogitsCatalog)?;
    let canonical_logits = canonical_logits_profile(&logits, selection)?;
    let row_capacity = generated
        .len()
        .checked_add(1)
        .ok_or(M1OperationDispatchExpansionError::ArithmeticOverflow)?;
    let mut rows = Vec::with_capacity(row_capacity);
    for (position, (generated, binding)) in generated.iter().zip(bindings).enumerate() {
        let logical_ordinal = u32::try_from(position)
            .map_err(|_| M1OperationDispatchExpansionError::ArithmeticOverflow)?;
        validate_retained_operation(
            operation_plan,
            selection,
            logical_ordinal,
            generated,
            binding,
        )?;
        if generated.profile.step.operator == Qwen3Operator::ArgmaxCompactCompletion {
            validate_logits_binding(&logits, canonical_logits, logical_ordinal, binding)?;
            push_row(
                &mut rows,
                logical_ordinal,
                generated.profile.step.operator,
                binding,
                M1OperationDispatchKind::K7Argmax,
            )?;
            match (selection.role, canonical_logits.completion()) {
                (
                    Qwen3ModelRole::Target8B,
                    Qwen3LogitsCompletionKindV1::TargetDirect
                    | Qwen3LogitsCompletionKindV1::TargetSpeculative,
                ) => {
                    if canonical_logits.compact_grid_workitems().is_none() {
                        return Err(M1OperationDispatchExpansionError::CanonicalLogitsCompletion);
                    }
                    push_row(
                        &mut rows,
                        logical_ordinal,
                        generated.profile.step.operator,
                        binding,
                        M1OperationDispatchKind::K7Compact,
                    )?;
                }
                (Qwen3ModelRole::Draft06B, Qwen3LogitsCompletionKindV1::DraftChoices) => {
                    if canonical_logits.compact_grid_workitems().is_some() {
                        return Err(M1OperationDispatchExpansionError::CanonicalLogitsCompletion);
                    }
                }
                _ => {
                    return Err(M1OperationDispatchExpansionError::CanonicalLogitsCompletion);
                }
            }
        } else {
            push_row(
                &mut rows,
                logical_ordinal,
                generated.profile.step.operator,
                binding,
                M1OperationDispatchKind::WholeOperation,
            )?;
        }
    }
    let physical_count = u32::try_from(rows.len())
        .map_err(|_| M1OperationDispatchExpansionError::ArithmeticOverflow)?;
    if physical_count > M1_MAX_OPERATION_DISPATCHES_V1 {
        return Err(M1OperationDispatchExpansionError::PhysicalCapacity {
            maximum: M1_MAX_OPERATION_DISPATCHES_V1,
            actual: physical_count,
        });
    }
    Ok(rows.into_boxed_slice())
}

fn push_row(
    rows: &mut Vec<M1OperationDispatchRow>,
    logical_ordinal: u32,
    operator: Qwen3Operator,
    operation: &DeclaredOperationKernelBinding,
    kind: M1OperationDispatchKind,
) -> Result<(), M1OperationDispatchExpansionError> {
    let dispatch_index = u32::try_from(rows.len())
        .map_err(|_| M1OperationDispatchExpansionError::ArithmeticOverflow)?;
    rows.push(M1OperationDispatchRow {
        dispatch_index,
        logical_ordinal,
        operator,
        operation: *operation,
        kind,
    });
    Ok(())
}

fn canonical_logits_profile(
    catalog: &Qwen3LogitsProfileCatalogV1,
    selection: Qwen3PlanSelection,
) -> Result<ferric_qwen_kernels::logits::Qwen3LogitsProfileV1, M1OperationDispatchExpansionError> {
    let role = match selection.role {
        Qwen3ModelRole::Target8B => Qwen3LogitsModelRoleV1::Target8B,
        Qwen3ModelRole::Draft06B => Qwen3LogitsModelRoleV1::Draft06B,
    };
    let mode = match selection.mode {
        Qwen3ExecutionMode::Prefill => Qwen3LogitsModeV1::Prefill,
        Qwen3ExecutionMode::Decode => Qwen3LogitsModeV1::Decode,
        Qwen3ExecutionMode::Speculative => Qwen3LogitsModeV1::Speculative,
    };
    let bucket = match selection.bucket {
        Qwen3PlanBucket::PrefillS1T128 => Qwen3LogitsBucketKindV1::PrefillS1T128,
        Qwen3PlanBucket::PrefillS8T128 => Qwen3LogitsBucketKindV1::PrefillS8T128,
        Qwen3PlanBucket::PrefillS1T512 => Qwen3LogitsBucketKindV1::PrefillS1T512,
        Qwen3PlanBucket::PrefillS1T2048 => Qwen3LogitsBucketKindV1::PrefillS1T2048,
        Qwen3PlanBucket::DecodeS1C8192 => Qwen3LogitsBucketKindV1::DecodeS1C8192,
        Qwen3PlanBucket::DecodeS8C8192 => Qwen3LogitsBucketKindV1::DecodeS8C8192,
        Qwen3PlanBucket::DecodeS32C8192 => Qwen3LogitsBucketKindV1::DecodeS32C8192,
        Qwen3PlanBucket::SpeculativeS1K4C8192 => Qwen3LogitsBucketKindV1::SpeculativeS1K4C8192,
        Qwen3PlanBucket::SpeculativeS8K4C8192 => Qwen3LogitsBucketKindV1::SpeculativeS8K4C8192,
        Qwen3PlanBucket::SpeculativeS1K8C8192 => Qwen3LogitsBucketKindV1::SpeculativeS1K8C8192,
        Qwen3PlanBucket::SpeculativeS1K16C8192 => Qwen3LogitsBucketKindV1::SpeculativeS1K16C8192,
    };
    catalog
        .profiles()
        .iter()
        .copied()
        .find(|profile| {
            profile.bucket().role() == role
                && profile.bucket().mode() == mode
                && profile.bucket().kind() == bucket
        })
        .ok_or(M1OperationDispatchExpansionError::CanonicalLogitsProfile)
}

fn validate_logits_binding(
    catalog: &Qwen3LogitsProfileCatalogV1,
    profile: ferric_qwen_kernels::logits::Qwen3LogitsProfileV1,
    logical_ordinal: u32,
    binding: &DeclaredOperationKernelBinding,
) -> Result<(), M1OperationDispatchExpansionError> {
    if binding.family() != KernelFamily::K7LogitsCompact {
        return Err(M1OperationDispatchExpansionError::GeneratedFamily { logical_ordinal });
    }
    if binding.profile_catalog_id() != Identity::new(*catalog.identity().as_bytes())
        || binding.profile_id() != Identity::new(*profile.identity().as_bytes())
    {
        return Err(M1OperationDispatchExpansionError::CanonicalLogitsIdentity);
    }
    Ok(())
}

fn validate_retained_operation(
    operation_plan: &DeclaredOperationKernelPlan,
    selection: Qwen3PlanSelection,
    logical_ordinal: u32,
    generated: &ferric_build::GeneratedOperationDeclaration,
    binding: &DeclaredOperationKernelBinding,
) -> Result<(), M1OperationDispatchExpansionError> {
    if generated.profile.selection != selection {
        return Err(M1OperationDispatchExpansionError::GeneratedSelection { logical_ordinal });
    }
    if generated.operation_index != binding.operation_index() {
        return Err(M1OperationDispatchExpansionError::GeneratedOperationOrder { logical_ordinal });
    }
    if generated.plan_index != binding.plan_index() {
        return Err(M1OperationDispatchExpansionError::GeneratedPlanIndex { logical_ordinal });
    }
    if generated.profile.family != binding.family() {
        return Err(M1OperationDispatchExpansionError::GeneratedFamily { logical_ordinal });
    }
    for (component, exact) in [
        (
            M1OperationDispatchIdentityComponent::Plan,
            generated.profile.plan_id == binding.plan_id(),
        ),
        (
            M1OperationDispatchIdentityComponent::RunnerDeclaration,
            operation_plan.runner_declaration_id() == binding.runner_declaration_id(),
        ),
        (
            M1OperationDispatchIdentityComponent::KernelCatalog,
            operation_plan.kernel_catalog_id() == binding.kernel_catalog_id(),
        ),
    ] {
        if !exact {
            return Err(M1OperationDispatchExpansionError::GeneratedIdentity {
                logical_ordinal,
                component,
            });
        }
    }
    Ok(())
}

fn validate_declaration(
    expected_selection: Qwen3PlanSelection,
    operation_plan: &DeclaredOperationKernelPlan,
    declaration: &DeclaredM1OperationDispatchExpansion,
) -> Result<(), M1OperationDispatchExpansionError> {
    if declaration.version != M1_OPERATION_DISPATCH_EXPANSION_VERSION {
        return Err(M1OperationDispatchExpansionError::Version {
            expected: M1_OPERATION_DISPATCH_EXPANSION_VERSION,
            actual: declaration.version,
        });
    }
    if declaration.selection.role != expected_selection.role {
        return Err(M1OperationDispatchExpansionError::SelectionRole);
    }
    if declaration.selection.mode != expected_selection.mode {
        return Err(M1OperationDispatchExpansionError::SelectionMode);
    }
    if declaration.selection.bucket != expected_selection.bucket {
        return Err(M1OperationDispatchExpansionError::SelectionBucket);
    }
    let expected = derive_m1_operation_dispatch_expansion(operation_plan, expected_selection)?;
    if declaration.runner_declaration_id != expected.runner_declaration_id {
        return Err(M1OperationDispatchExpansionError::RunnerDeclarationIdentity);
    }
    if declaration.kernel_catalog_id != expected.kernel_catalog_id {
        return Err(M1OperationDispatchExpansionError::KernelCatalogIdentity);
    }
    if declaration.logical_operation_count != expected.logical_operation_count {
        return Err(M1OperationDispatchExpansionError::LogicalOperationCount {
            expected: expected.logical_operation_count,
            actual: declaration.logical_operation_count,
        });
    }
    if declaration.physical_dispatch_count != expected.physical_dispatch_count {
        return Err(M1OperationDispatchExpansionError::PhysicalDispatchCount {
            expected: expected.physical_dispatch_count,
            actual: declaration.physical_dispatch_count,
        });
    }
    if declaration.rows.len() != expected.rows.len() {
        return Err(M1OperationDispatchExpansionError::RowCount {
            expected: expected.rows.len(),
            actual: declaration.rows.len(),
        });
    }
    for (position, (actual, expected)) in declaration.rows.iter().zip(&expected.rows).enumerate() {
        validate_row(position, actual, expected)?;
    }
    if declaration.expansion_id != expected.expansion_id {
        return Err(M1OperationDispatchExpansionError::ExpansionIdentity);
    }
    Ok(())
}

fn validate_row(
    position: usize,
    actual: &M1OperationDispatchRow,
    expected: &M1OperationDispatchRow,
) -> Result<(), M1OperationDispatchExpansionError> {
    if actual.dispatch_index != expected.dispatch_index {
        return Err(M1OperationDispatchExpansionError::DispatchOrder {
            position,
            expected: expected.dispatch_index,
            actual: actual.dispatch_index,
        });
    }
    if actual.logical_ordinal != expected.logical_ordinal {
        return Err(M1OperationDispatchExpansionError::LogicalOrder {
            position,
            expected: expected.logical_ordinal,
            actual: actual.logical_ordinal,
        });
    }
    if actual.operation.operation_index() != expected.operation.operation_index() {
        return Err(M1OperationDispatchExpansionError::OperationOrder {
            position,
            expected: expected.operation.operation_index(),
            actual: actual.operation.operation_index(),
        });
    }
    if actual.operation.plan_index() != expected.operation.plan_index() {
        return Err(M1OperationDispatchExpansionError::PlanIndex {
            position,
            expected: expected.operation.plan_index(),
            actual: actual.operation.plan_index(),
        });
    }
    if actual.operator != expected.operator {
        return Err(M1OperationDispatchExpansionError::Operator {
            position,
            expected: expected.operator,
            actual: actual.operator,
        });
    }
    if actual.operation.family() != expected.operation.family() {
        return Err(M1OperationDispatchExpansionError::Family {
            position,
            expected: expected.operation.family(),
            actual: actual.operation.family(),
        });
    }
    validate_operation_identities(position, &actual.operation, &expected.operation)?;
    if actual.kind != expected.kind {
        return Err(M1OperationDispatchExpansionError::Subdispatch {
            position,
            expected: expected.kind,
            actual: actual.kind,
        });
    }
    Ok(())
}

fn validate_operation_identities(
    position: usize,
    actual: &DeclaredOperationKernelBinding,
    expected: &DeclaredOperationKernelBinding,
) -> Result<(), M1OperationDispatchExpansionError> {
    for (component, exact) in [
        (
            M1OperationDispatchIdentityComponent::Plan,
            actual.plan_id() == expected.plan_id(),
        ),
        (
            M1OperationDispatchIdentityComponent::RunnerDeclaration,
            actual.runner_declaration_id() == expected.runner_declaration_id(),
        ),
        (
            M1OperationDispatchIdentityComponent::KernelCatalog,
            actual.kernel_catalog_id() == expected.kernel_catalog_id(),
        ),
        (
            M1OperationDispatchIdentityComponent::ProfileCatalog,
            actual.profile_catalog_id() == expected.profile_catalog_id(),
        ),
        (
            M1OperationDispatchIdentityComponent::Profile,
            actual.profile_id() == expected.profile_id(),
        ),
        (
            M1OperationDispatchIdentityComponent::FamilyBuild,
            actual.family_build_id() == expected.family_build_id(),
        ),
        (
            M1OperationDispatchIdentityComponent::Artifact,
            actual.artifact_id() == expected.artifact_id(),
        ),
        (
            M1OperationDispatchIdentityComponent::AbiLayout,
            actual.abi_layout_id() == expected.abi_layout_id(),
        ),
    ] {
        if !exact {
            return Err(M1OperationDispatchExpansionError::OperationIdentity {
                position,
                component,
            });
        }
    }
    Ok(())
}

fn expansion_identity(declaration: &DeclaredM1OperationDispatchExpansion) -> Identity {
    let mut record = Vec::with_capacity(128 + declaration.rows.len() * 320);
    record.extend_from_slice(&declaration.version.to_le_bytes());
    encode_selection(&mut record, declaration.selection);
    for identity in [
        declaration.runner_declaration_id,
        declaration.kernel_catalog_id,
    ] {
        record.extend_from_slice(identity.as_bytes());
    }
    record.extend_from_slice(&declaration.logical_operation_count.to_le_bytes());
    record.extend_from_slice(&declaration.physical_dispatch_count.to_le_bytes());
    record.extend_from_slice(&(declaration.rows.len() as u64).to_le_bytes());
    for row in &declaration.rows {
        record.extend_from_slice(&row.dispatch_index.to_le_bytes());
        record.extend_from_slice(&row.logical_ordinal.to_le_bytes());
        record.push(operator_tag(row.operator));
        record.push(row.kind.tag());
        let operation = row.operation;
        record.extend_from_slice(&operation.operation_index().to_le_bytes());
        record.extend_from_slice(&operation.plan_index().to_le_bytes());
        record.push(family_tag(operation.family()));
        for identity in [
            operation.plan_id(),
            operation.runner_declaration_id(),
            operation.kernel_catalog_id(),
            operation.profile_catalog_id(),
            operation.profile_id(),
            operation.family_build_id(),
            operation.artifact_id(),
            operation.abi_layout_id(),
        ] {
            record.extend_from_slice(identity.as_bytes());
        }
    }
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, EXPANSION_IDENTITY_DOMAIN);
    hash_field(&mut hasher, &record);
    Identity::new(hasher.finalize().into())
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) {
    let length = u64::try_from(bytes.len()).expect("usize always fits in u64 on supported targets");
    hasher.update(length.to_le_bytes());
    hasher.update(bytes);
}

fn encode_selection(record: &mut Vec<u8>, selection: Qwen3PlanSelection) {
    record.push(match selection.role {
        Qwen3ModelRole::Target8B => 1,
        Qwen3ModelRole::Draft06B => 2,
    });
    record.push(match selection.mode {
        Qwen3ExecutionMode::Prefill => 1,
        Qwen3ExecutionMode::Decode => 2,
        Qwen3ExecutionMode::Speculative => 3,
    });
    record.push(match selection.bucket {
        Qwen3PlanBucket::PrefillS1T128 => 1,
        Qwen3PlanBucket::PrefillS8T128 => 2,
        Qwen3PlanBucket::PrefillS1T512 => 3,
        Qwen3PlanBucket::PrefillS1T2048 => 4,
        Qwen3PlanBucket::DecodeS1C8192 => 5,
        Qwen3PlanBucket::DecodeS8C8192 => 6,
        Qwen3PlanBucket::DecodeS32C8192 => 7,
        Qwen3PlanBucket::SpeculativeS1K4C8192 => 8,
        Qwen3PlanBucket::SpeculativeS8K4C8192 => 9,
        Qwen3PlanBucket::SpeculativeS1K8C8192 => 10,
        Qwen3PlanBucket::SpeculativeS1K16C8192 => 11,
    });
}

const fn family_tag(family: KernelFamily) -> u8 {
    match family {
        KernelFamily::K1GemmGemv => 1,
        KernelFamily::K2RmsNormResidual => 2,
        KernelFamily::K3RopePagedKv => 3,
        KernelFamily::K4GqaPrefill => 4,
        KernelFamily::K5PagedGqaDecode => 5,
        KernelFamily::K6SwiGlu => 6,
        KernelFamily::K7LogitsCompact => 7,
    }
}

const fn operator_tag(operator: Qwen3Operator) -> u8 {
    match operator {
        Qwen3Operator::TokenEmbedding => 1,
        Qwen3Operator::InputRmsNorm => 2,
        Qwen3Operator::QueryProjection => 3,
        Qwen3Operator::KeyProjection => 4,
        Qwen3Operator::ValueProjection => 5,
        Qwen3Operator::QueryRmsNorm => 6,
        Qwen3Operator::KeyRmsNorm => 7,
        Qwen3Operator::Rope => 8,
        Qwen3Operator::KvWrite => 9,
        Qwen3Operator::Attention => 10,
        Qwen3Operator::AttentionOutputResidual => 11,
        Qwen3Operator::PostAttentionRmsNorm => 12,
        Qwen3Operator::GateProjection => 13,
        Qwen3Operator::UpProjection => 14,
        Qwen3Operator::SwiGlu => 15,
        Qwen3Operator::DownResidual => 16,
        Qwen3Operator::FinalRmsNorm => 17,
        Qwen3Operator::LogitsProjection => 18,
        Qwen3Operator::ArgmaxCompactCompletion => 19,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use ferric_kernels::{KernelFamily, M1_B3_PLAN_BUCKETS};
    use ferric_spec::{
        Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3Operator, Qwen3PlanBucket,
        Qwen3PlanSelection,
    };

    use super::{
        derive_m1_operation_dispatch_expansion, plan_m1_operation_dispatch_expansion,
        DeclaredM1OperationDispatchExpansion, M1OperationDispatchExpansionError,
        M1OperationDispatchExpansionOutcome, M1OperationDispatchIdentityComponent,
        M1OperationDispatchKind, M1OperationDispatchRow, M1_MAX_OPERATION_DISPATCHES_V1,
        M1_OPERATION_DISPATCH_EXPANSION_VERSION,
    };
    use crate::operation_kernel_plan::tests::public_operation_kernel_plan_fixture;
    use crate::{
        DeclaredKernelFamilyArtifact, DeclaredOperationIdentity, DeclaredOperationKernelBinding,
        DeclaredOperationKernelPlan,
    };

    const TARGET_SELECTION: Qwen3PlanSelection = Qwen3PlanSelection {
        role: Qwen3ModelRole::Target8B,
        mode: Qwen3ExecutionMode::Prefill,
        bucket: Qwen3PlanBucket::PrefillS1T128,
    };

    #[derive(Debug, Eq, PartialEq)]
    struct DeclarationSnapshot {
        version: u32,
        expansion_id: Identity,
        selection: Qwen3PlanSelection,
        runner_declaration_id: Identity,
        kernel_catalog_id: Identity,
        logical_operation_count: u32,
        physical_dispatch_count: u32,
        rows: Vec<M1OperationDispatchRow>,
    }

    fn snapshot(declaration: &DeclaredM1OperationDispatchExpansion) -> DeclarationSnapshot {
        DeclarationSnapshot {
            version: declaration.version(),
            expansion_id: declaration.expansion_id(),
            selection: declaration.selection(),
            runner_declaration_id: declaration.runner_declaration_id(),
            kernel_catalog_id: declaration.kernel_catalog_id(),
            logical_operation_count: declaration.logical_operation_count(),
            physical_dispatch_count: declaration.physical_dispatch_count(),
            rows: declaration.rows().to_vec(),
        }
    }

    fn assert_snapshot(
        declaration: &DeclaredM1OperationDispatchExpansion,
        expected: &DeclarationSnapshot,
    ) {
        assert_eq!(snapshot(declaration), *expected);
    }

    fn reject_and_recover(
        operation_plan: DeclaredOperationKernelPlan,
        expected_selection: Qwen3PlanSelection,
        declaration: DeclaredM1OperationDispatchExpansion,
        expected_error: M1OperationDispatchExpansionError,
    ) -> DeclaredOperationKernelPlan {
        let runner_declaration_id = operation_plan.runner_declaration_id();
        let kernel_catalog_id = operation_plan.kernel_catalog_id();
        let operation_count = operation_plan.operations().len();
        let first_operation = operation_plan.operations().first().copied();
        let last_operation = operation_plan.operations().last().copied();
        let declaration_snapshot = snapshot(&declaration);
        let M1OperationDispatchExpansionOutcome::Rejected(failure) =
            plan_m1_operation_dispatch_expansion(expected_selection, operation_plan, declaration)
        else {
            panic!("drifted declaration must be rejected");
        };
        assert_eq!(failure.error(), expected_error);
        let (error, operation_plan, declaration) = failure.into_parts();
        assert_eq!(error, expected_error);
        assert_eq!(
            operation_plan.runner_declaration_id(),
            runner_declaration_id
        );
        assert_eq!(operation_plan.kernel_catalog_id(), kernel_catalog_id);
        assert_eq!(operation_plan.operations().len(), operation_count);
        assert_eq!(
            operation_plan.operations().first().copied(),
            first_operation
        );
        assert_eq!(operation_plan.operations().last().copied(), last_operation);
        assert_snapshot(&declaration, &declaration_snapshot);
        operation_plan
    }

    fn drift_identity(seed: u8) -> Identity {
        let mut bytes = [0_u8; 32];
        bytes[0] = seed;
        bytes[31] = 1;
        Identity::new(bytes)
    }

    fn operation_with_identity_drift(
        operation: &DeclaredOperationKernelBinding,
        component: M1OperationDispatchIdentityComponent,
    ) -> DeclaredOperationKernelBinding {
        let drifted = drift_identity(component_tag(component));
        let identity = DeclaredOperationIdentity::new(
            if component == M1OperationDispatchIdentityComponent::Plan {
                drifted
            } else {
                operation.plan_id()
            },
            if component == M1OperationDispatchIdentityComponent::RunnerDeclaration {
                drifted
            } else {
                operation.runner_declaration_id()
            },
            if component == M1OperationDispatchIdentityComponent::KernelCatalog {
                drifted
            } else {
                operation.kernel_catalog_id()
            },
            if component == M1OperationDispatchIdentityComponent::ProfileCatalog {
                drifted
            } else {
                operation.profile_catalog_id()
            },
            if component == M1OperationDispatchIdentityComponent::Profile {
                drifted
            } else {
                operation.profile_id()
            },
        );
        let family = DeclaredKernelFamilyArtifact::new(
            operation.family(),
            if component == M1OperationDispatchIdentityComponent::FamilyBuild {
                drifted
            } else {
                operation.family_build_id()
            },
            if component == M1OperationDispatchIdentityComponent::Artifact {
                drifted
            } else {
                operation.artifact_id()
            },
            if component == M1OperationDispatchIdentityComponent::AbiLayout {
                drifted
            } else {
                operation.abi_layout_id()
            },
        );
        DeclaredOperationKernelBinding::new(
            operation.operation_index(),
            operation.plan_index(),
            identity,
            family,
        )
    }

    const fn component_tag(component: M1OperationDispatchIdentityComponent) -> u8 {
        match component {
            M1OperationDispatchIdentityComponent::Plan => 1,
            M1OperationDispatchIdentityComponent::RunnerDeclaration => 2,
            M1OperationDispatchIdentityComponent::KernelCatalog => 3,
            M1OperationDispatchIdentityComponent::ProfileCatalog => 4,
            M1OperationDispatchIdentityComponent::Profile => 5,
            M1OperationDispatchIdentityComponent::FamilyBuild => 6,
            M1OperationDispatchIdentityComponent::Artifact => 7,
            M1OperationDispatchIdentityComponent::AbiLayout => 8,
        }
    }

    fn replace_operation(
        row: &M1OperationDispatchRow,
        operation: &DeclaredOperationKernelBinding,
    ) -> M1OperationDispatchRow {
        M1OperationDispatchRow::new(
            row.dispatch_index(),
            row.logical_ordinal(),
            row.operator(),
            *operation,
            row.kind(),
        )
    }

    fn operation_with_position_or_family(
        operation: &DeclaredOperationKernelBinding,
        operation_index: u32,
        plan_index: u16,
        family: KernelFamily,
    ) -> DeclaredOperationKernelBinding {
        DeclaredOperationKernelBinding::new(
            operation_index,
            plan_index,
            DeclaredOperationIdentity::new(
                operation.plan_id(),
                operation.runner_declaration_id(),
                operation.kernel_catalog_id(),
                operation.profile_catalog_id(),
                operation.profile_id(),
            ),
            DeclaredKernelFamilyArtifact::new(
                family,
                operation.family_build_id(),
                operation.artifact_id(),
                operation.abi_layout_id(),
            ),
        )
    }

    #[test]
    fn all_22_selections_have_exact_deterministic_expansions() {
        let operation_plan = public_operation_kernel_plan_fixture();
        let mut expansion_ids = HashSet::new();
        let mut selection_count = 0_usize;

        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            for (mode, bucket) in M1_B3_PLAN_BUCKETS {
                let selection = Qwen3PlanSelection { role, mode, bucket };
                let declaration =
                    derive_m1_operation_dispatch_expansion(&operation_plan, selection)
                        .expect("canonical operation plan must expand");
                let repeated = derive_m1_operation_dispatch_expansion(&operation_plan, selection)
                    .expect("repeated canonical derivation must expand");
                let expected_logical = match role {
                    Qwen3ModelRole::Target8B => 544,
                    Qwen3ModelRole::Draft06B => 424,
                };
                let expected_physical = match role {
                    Qwen3ModelRole::Target8B => 545,
                    Qwen3ModelRole::Draft06B => 424,
                };
                assert_eq!(
                    declaration.version(),
                    M1_OPERATION_DISPATCH_EXPANSION_VERSION
                );
                assert_eq!(declaration.selection(), selection);
                assert_eq!(declaration.logical_operation_count(), expected_logical);
                assert_eq!(declaration.physical_dispatch_count(), expected_physical);
                assert!(declaration.physical_dispatch_count() <= M1_MAX_OPERATION_DISPATCHES_V1);
                assert_eq!(declaration.rows(), repeated.rows());
                assert_eq!(declaration.expansion_id(), repeated.expansion_id());
                assert!(expansion_ids.insert(*declaration.expansion_id().as_bytes()));

                let (generated, bindings) = operation_plan
                    .operation_declarations_for(selection)
                    .expect("canonical plan range");
                let mut physical_position = 0_usize;
                for (logical_position, (generated, binding)) in
                    generated.iter().zip(bindings).enumerate()
                {
                    let is_k7 =
                        generated.profile.step.operator == Qwen3Operator::ArgmaxCompactCompletion;
                    let dispatches = if is_k7 && role == Qwen3ModelRole::Target8B {
                        2
                    } else {
                        1
                    };
                    for subdispatch in 0..dispatches {
                        let row = declaration.rows()[physical_position];
                        assert_eq!(
                            row.dispatch_index(),
                            u32::try_from(physical_position).expect("dispatch position fits u32")
                        );
                        assert_eq!(
                            row.logical_ordinal(),
                            u32::try_from(logical_position).expect("logical position fits u32")
                        );
                        assert_eq!(row.operator(), generated.profile.step.operator);
                        assert_eq!(row.operation(), binding);
                        assert_eq!(
                            row.kind(),
                            match (is_k7, subdispatch) {
                                (false, _) => M1OperationDispatchKind::WholeOperation,
                                (true, 0) => M1OperationDispatchKind::K7Argmax,
                                (true, _) => M1OperationDispatchKind::K7Compact,
                            }
                        );
                        assert!(!row.grants_dispatch_authority());
                        physical_position += 1;
                    }
                }
                assert_eq!(physical_position, declaration.rows().len());
                assert_eq!(
                    generated
                        .iter()
                        .filter(|operation| operation.profile.step.operator
                            == Qwen3Operator::ArgmaxCompactCompletion)
                        .count(),
                    1
                );
                selection_count += 1;
            }
        }
        assert_eq!(selection_count, 22);
        assert_eq!(expansion_ids.len(), 22);
    }

    #[test]
    fn exact_planning_is_inert_and_abort_recovers_both_inputs() {
        let operation_plan = public_operation_kernel_plan_fixture();
        let runner_declaration_id = operation_plan.runner_declaration_id();
        let declaration = derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION)
            .expect("canonical target expansion");
        let declaration_snapshot = snapshot(&declaration);
        assert!(!declaration.authenticates_artifacts());
        let M1OperationDispatchExpansionOutcome::Planned(planned) =
            plan_m1_operation_dispatch_expansion(TARGET_SELECTION, operation_plan, declaration)
        else {
            panic!("exact declaration must plan");
        };
        assert_eq!(planned.selection(), TARGET_SELECTION);
        assert_eq!(planned.logical_operation_count(), 544);
        assert_eq!(planned.physical_dispatch_count(), 545);
        assert_eq!(planned.rows().len(), 545);
        assert_eq!(
            planned.operation_plan().runner_declaration_id(),
            runner_declaration_id
        );
        assert!(!planned.authenticates_artifacts());
        assert!(!planned.grants_execution_authority());
        assert!(!planned.proves_refinement());

        let (operation_plan, declaration) = planned.abort();
        assert_eq!(
            operation_plan.runner_declaration_id(),
            runner_declaration_id
        );
        assert_snapshot(&declaration, &declaration_snapshot);
    }

    #[test]
    fn header_count_and_identity_drift_fail_closed_and_recover() {
        let mut operation_plan = public_operation_kernel_plan_fixture();

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        declaration.version += 1;
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::Version {
                expected: M1_OPERATION_DISPATCH_EXPANSION_VERSION,
                actual: M1_OPERATION_DISPATCH_EXPANSION_VERSION + 1,
            },
        );

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        declaration.selection.role = Qwen3ModelRole::Draft06B;
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::SelectionRole,
        );

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        declaration.selection.mode = Qwen3ExecutionMode::Decode;
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::SelectionMode,
        );

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        declaration.selection.bucket = Qwen3PlanBucket::PrefillS8T128;
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::SelectionBucket,
        );

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        declaration.runner_declaration_id = drift_identity(20);
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::RunnerDeclarationIdentity,
        );

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        declaration.kernel_catalog_id = drift_identity(21);
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::KernelCatalogIdentity,
        );

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        declaration.logical_operation_count -= 1;
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::LogicalOperationCount {
                expected: 544,
                actual: 543,
            },
        );

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        declaration.physical_dispatch_count -= 1;
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::PhysicalDispatchCount {
                expected: 545,
                actual: 544,
            },
        );

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        declaration.rows = declaration.rows[..544].into();
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::RowCount {
                expected: 545,
                actual: 544,
            },
        );

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        declaration.expansion_id = drift_identity(22);
        let _operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::ExpansionIdentity,
        );
    }

    #[test]
    fn row_order_family_subdispatch_and_all_identities_fail_closed() {
        let mut operation_plan = public_operation_kernel_plan_fixture();

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        let row = declaration.rows[0];
        declaration.rows[0] = M1OperationDispatchRow::new(
            1,
            row.logical_ordinal(),
            row.operator(),
            *row.operation(),
            row.kind(),
        );
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::DispatchOrder {
                position: 0,
                expected: 0,
                actual: 1,
            },
        );

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        declaration.rows.swap(0, 1);
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::DispatchOrder {
                position: 0,
                expected: 0,
                actual: 1,
            },
        );

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        let row = declaration.rows[0];
        declaration.rows[0] = M1OperationDispatchRow::new(
            row.dispatch_index(),
            1,
            row.operator(),
            *row.operation(),
            row.kind(),
        );
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::LogicalOrder {
                position: 0,
                expected: 0,
                actual: 1,
            },
        );

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        let row = declaration.rows[0];
        let operation = row.operation();
        let changed = operation_with_position_or_family(
            operation,
            operation.operation_index() + 1,
            operation.plan_index(),
            operation.family(),
        );
        declaration.rows[0] = replace_operation(&row, &changed);
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::OperationOrder {
                position: 0,
                expected: operation.operation_index(),
                actual: operation.operation_index() + 1,
            },
        );

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        let row = declaration.rows[0];
        let operation = row.operation();
        let changed = operation_with_position_or_family(
            operation,
            operation.operation_index(),
            operation.plan_index() + 1,
            operation.family(),
        );
        declaration.rows[0] = replace_operation(&row, &changed);
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::PlanIndex {
                position: 0,
                expected: operation.plan_index(),
                actual: operation.plan_index() + 1,
            },
        );

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        let row = declaration.rows[0];
        declaration.rows[0] = M1OperationDispatchRow::new(
            row.dispatch_index(),
            row.logical_ordinal(),
            Qwen3Operator::InputRmsNorm,
            *row.operation(),
            row.kind(),
        );
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::Operator {
                position: 0,
                expected: Qwen3Operator::TokenEmbedding,
                actual: Qwen3Operator::InputRmsNorm,
            },
        );

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        let row = declaration.rows[0];
        let operation = row.operation();
        let changed = operation_with_position_or_family(
            operation,
            operation.operation_index(),
            operation.plan_index(),
            KernelFamily::K2RmsNormResidual,
        );
        declaration.rows[0] = replace_operation(&row, &changed);
        operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::Family {
                position: 0,
                expected: KernelFamily::K1GemmGemv,
                actual: KernelFamily::K2RmsNormResidual,
            },
        );

        for component in [
            M1OperationDispatchIdentityComponent::Plan,
            M1OperationDispatchIdentityComponent::RunnerDeclaration,
            M1OperationDispatchIdentityComponent::KernelCatalog,
            M1OperationDispatchIdentityComponent::ProfileCatalog,
            M1OperationDispatchIdentityComponent::Profile,
            M1OperationDispatchIdentityComponent::FamilyBuild,
            M1OperationDispatchIdentityComponent::Artifact,
            M1OperationDispatchIdentityComponent::AbiLayout,
        ] {
            let mut declaration =
                derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
            let row = declaration.rows[0];
            let changed = operation_with_identity_drift(row.operation(), component);
            declaration.rows[0] = replace_operation(&row, &changed);
            operation_plan = reject_and_recover(
                operation_plan,
                TARGET_SELECTION,
                declaration,
                M1OperationDispatchExpansionError::OperationIdentity {
                    position: 0,
                    component,
                },
            );
        }

        let mut declaration =
            derive_m1_operation_dispatch_expansion(&operation_plan, TARGET_SELECTION).unwrap();
        let compact_position = declaration.rows.len() - 1;
        let row = declaration.rows[compact_position];
        assert_eq!(row.kind(), M1OperationDispatchKind::K7Compact);
        declaration.rows[compact_position] = M1OperationDispatchRow::new(
            row.dispatch_index(),
            row.logical_ordinal(),
            row.operator(),
            *row.operation(),
            M1OperationDispatchKind::K7Argmax,
        );
        let _operation_plan = reject_and_recover(
            operation_plan,
            TARGET_SELECTION,
            declaration,
            M1OperationDispatchExpansionError::Subdispatch {
                position: compact_position,
                expected: M1OperationDispatchKind::K7Compact,
                actual: M1OperationDispatchKind::K7Argmax,
            },
        );
    }
}
