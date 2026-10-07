#![forbid(unsafe_code)]

//! M1 generated target-graph structural composition theorem.
//!
//! This theorem is parametric over the complete finite target roster in the
//! checked-in generated table. It joins every operation of one selected target
//! plan to validated generated-runner custody, validated structural-kernel
//! custody, the verified logical Qwen3 plan, target K7 dispatch expansion, the
//! row-for-row target-only addressless physical recipe, and the closed
//! target-only fixed-batch cardinality.
//!
//! This is a source-level, addressless structural theorem. It does not prove
//! physical queue execution, device or KV refinement, scheduler or
//! multi-member refinement, machine semantics, numerical or hardware
//! correctness, performance qualification, or M1 closure.

#[allow(unused_imports)]
use ferric_generated_runner::{
    GeneratedRunnerInput, ValidatedGeneratedRunnerInput, GENERATED_PLAN_TEMPLATES,
};
#[allow(unused_imports)]
use ferric_kernels::{KernelCatalogInput, ValidatedKernelCatalogInput};
#[allow(unused_imports)]
use ferric_spec::{
    Qwen3GeneratedPlan, Qwen3ModelRole, Qwen3Operator, Qwen3PlanAuthority, Qwen3PlanSelection,
    QWEN3_TARGET_PLAN_STEPS,
};
#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

/// Complete source-level composition fact for any one canonical target plan.
pub open spec fn m1_generated_target_graph_composed(
    plan_index: u16,
    selection: Qwen3PlanSelection,
    operation_start: u32,
    generated: Seq<ValidatedGeneratedRunnerInput>,
    generated_expected: Seq<GeneratedRunnerInput>,
    kernels: Seq<ValidatedKernelCatalogInput>,
    kernel_expected: Seq<KernelCatalogInput>,
    logical_plan: &Qwen3GeneratedPlan,
    logical_authority: Qwen3PlanAuthority,
) -> bool {
    &&& (plan_index as int) < 11
    &&& selection.role == Qwen3ModelRole::Target8B
    &&& GENERATED_PLAN_TEMPLATES@[plan_index as int].plan_index == plan_index
    &&& GENERATED_PLAN_TEMPLATES@[plan_index as int].selection == selection
    &&& GENERATED_PLAN_TEMPLATES@[plan_index as int].operation_start == operation_start
    &&& GENERATED_PLAN_TEMPLATES@[plan_index as int].operation_count
        == QWEN3_TARGET_PLAN_STEPS
    &&& generated.len() == QWEN3_TARGET_PLAN_STEPS as nat
    &&& generated_expected.len() == generated.len()
    &&& kernels.len() == generated.len()
    &&& kernel_expected.len() == generated.len()
    &&& logical_plan.valid_for(logical_authority, selection)
    &&& logical_plan.selection == selection
    &&& logical_plan.steps@.len() == generated.len()
    &&& forall|index: int| 0 <= index < generated.len() ==> {
        &&& generated[index].matches_expected_spec(generated_expected[index])
        &&& kernels[index].matches_expected_spec(kernel_expected[index])
        &&& generated[index].input_spec().plan_index == plan_index
        &&& generated[index].input_spec().selection == selection
        &&& generated[index].input_spec().operation_start == operation_start
        &&& generated[index].input_spec().operation_count == QWEN3_TARGET_PLAN_STEPS
        &&& generated[index].input_spec().operation_index as int
            == operation_start as int + index
        &&& kernels[index].input_spec().plan_index
            == generated[index].input_spec().plan_index
        &&& kernels[index].input_spec().operation_index
            == generated[index].input_spec().operation_index
        &&& kernels[index].input_spec().profile.selection == selection
        &&& kernels[index].input_spec().profile.step == logical_plan.steps@[index]
        &&& kernels[index].input_spec().profile.step.ordinal as int == index
        &&& generated[index].input_spec().identities.plan_id.bytes_spec()
            == kernels[index].input_spec().profile.plan_id.bytes_spec()
        &&& kernels[index].input_spec().profile.plan_id.bytes_spec()
            == logical_authority.plan_id.bytes_spec()
    }
    &&& logical_plan.steps@[543].operator == Qwen3Operator::ArgmaxCompactCompletion
    &&& forall|index: int| 0 <= index < 543
        ==> logical_plan.steps@[index].operator != Qwen3Operator::ArgmaxCompactCompletion
    &&& ferric_engine::m1_operation_dispatch_kinds_spec(
        Qwen3ModelRole::Target8B,
        logical_plan.steps@[543].operator,
    ) == seq![2nat, 3nat]
    &&& ferric_engine::m1_target_operation_dispatch_count_spec(
        logical_plan.steps@.len(),
        1,
    ) == 545
    &&& ferric_engine::m1_target_only_physical_recipe_count_spec(545) == 545
    &&& ferric_engine::m1_target_only_fixed_batch_packet_count_spec() == 545
}

/// Composes all 544 operations of any canonical generated target plan.
///
/// The move-only input sequences represent successful results from the
/// independently checked generated-runner and kernel-catalog validators. The
/// theorem proves the common finite-table selection, operation order, logical
/// graph step, plan identity, unique target K7 split, and reviewed 545-row
/// addressless target-only recipe/fixed-batch structure.
///
/// The theorem does not establish that regular-Rust lowering ran, or that any
/// artifact, address, packet, queue, device, or numerical implementation
/// satisfies these structural declarations.
pub struct M1GeneratedTargetGraphInputs<'a> {
    /// Canonical target-plan table index.
    pub plan_index: u16,
    /// Exact target role, execution mode, and finite bucket.
    pub selection: Qwen3PlanSelection,
    /// First operation index in the canonical generated operation table.
    pub operation_start: u32,
    /// Successfully validated generated-runner owners in logical order.
    pub generated: &'a [ValidatedGeneratedRunnerInput],
    /// Exact generated-runner expectations paired with those owners.
    pub generated_expected: &'a [GeneratedRunnerInput],
    /// Successfully validated structural-kernel owners in logical order.
    pub kernels: &'a [ValidatedKernelCatalogInput],
    /// Exact structural-kernel expectations paired with those owners.
    pub kernel_expected: &'a [KernelCatalogInput],
    /// Independently validated logical Qwen3 plan.
    pub logical_plan: &'a Qwen3GeneratedPlan,
    /// Authority naming the exact logical plan identity.
    pub logical_authority: Qwen3PlanAuthority,
}

/// Verifies one complete target graph from retained validated owners.
pub fn m1_generated_target_graph_theorem(
    _inputs: M1GeneratedTargetGraphInputs<'_>,
)
    requires
        (_inputs.plan_index as int) < 11,
        _inputs.selection.role == Qwen3ModelRole::Target8B,
        GENERATED_PLAN_TEMPLATES@[_inputs.plan_index as int].plan_index == _inputs.plan_index,
        GENERATED_PLAN_TEMPLATES@[_inputs.plan_index as int].selection == _inputs.selection,
        GENERATED_PLAN_TEMPLATES@[_inputs.plan_index as int].operation_start
            == _inputs.operation_start,
        GENERATED_PLAN_TEMPLATES@[_inputs.plan_index as int].operation_count
            == QWEN3_TARGET_PLAN_STEPS,
        _inputs.generated@.len() == QWEN3_TARGET_PLAN_STEPS as nat,
        _inputs.generated_expected@.len() == _inputs.generated@.len(),
        _inputs.kernels@.len() == _inputs.generated@.len(),
        _inputs.kernel_expected@.len() == _inputs.generated@.len(),
        _inputs.logical_plan.valid_for(_inputs.logical_authority, _inputs.selection),
        forall|index: int| 0 <= index < _inputs.generated@.len() ==> {
            &&& _inputs.generated@[index].matches_expected_spec(
                _inputs.generated_expected@[index],
            )
            &&& _inputs.kernels@[index].matches_expected_spec(_inputs.kernel_expected@[index])
            &&& _inputs.generated_expected@[index].plan_index == _inputs.plan_index
            &&& _inputs.generated_expected@[index].selection == _inputs.selection
            &&& _inputs.generated_expected@[index].operation_start == _inputs.operation_start
            &&& _inputs.generated_expected@[index].operation_count == QWEN3_TARGET_PLAN_STEPS
            &&& _inputs.generated_expected@[index].operation_index as int
                == _inputs.operation_start as int + index
            &&& _inputs.kernel_expected@[index].plan_index == _inputs.plan_index
            &&& _inputs.kernel_expected@[index].operation_index as int
                == _inputs.operation_start as int + index
            &&& _inputs.kernel_expected@[index].profile.selection == _inputs.selection
            &&& _inputs.kernel_expected@[index].profile.step.ordinal as int == index
            &&& _inputs.generated_expected@[index].identities.plan_id.bytes_spec()
                == _inputs.kernel_expected@[index].profile.plan_id.bytes_spec()
            &&& _inputs.kernel_expected@[index].profile.plan_id.bytes_spec()
                == _inputs.logical_authority.plan_id.bytes_spec()
        },
    ensures m1_generated_target_graph_composed(
        _inputs.plan_index,
        _inputs.selection,
        _inputs.operation_start,
        _inputs.generated@,
        _inputs.generated_expected@,
        _inputs.kernels@,
        _inputs.kernel_expected@,
        _inputs.logical_plan,
        _inputs.logical_authority,
    ),
{
    proof {
        _inputs
            .logical_plan
            .expose_valid_steps(_inputs.logical_authority, _inputs.selection);
        _inputs
            .logical_plan
            .expose_unique_target_completion(_inputs.logical_authority, _inputs.selection);

        assert forall|index: int| 0 <= index < _inputs.generated@.len() implies {
            &&& _inputs.generated@[index].input_spec().plan_index == _inputs.plan_index
            &&& _inputs.generated@[index].input_spec().selection == _inputs.selection
            &&& _inputs.generated@[index].input_spec().operation_start == _inputs.operation_start
            &&& _inputs.generated@[index].input_spec().operation_count == QWEN3_TARGET_PLAN_STEPS
            &&& _inputs.generated@[index].input_spec().operation_index as int
                == _inputs.operation_start as int + index
            &&& _inputs.kernels@[index].input_spec().plan_index
                == _inputs.generated@[index].input_spec().plan_index
            &&& _inputs.kernels@[index].input_spec().operation_index
                == _inputs.generated@[index].input_spec().operation_index
            &&& _inputs.kernels@[index].input_spec().profile.selection == _inputs.selection
            &&& _inputs.kernels@[index].input_spec().profile.step
                == _inputs.logical_plan.steps@[index]
            &&& _inputs.kernels@[index].input_spec().profile.step.ordinal as int == index
            &&& _inputs.generated@[index].input_spec().identities.plan_id.bytes_spec()
                == _inputs.kernels@[index].input_spec().profile.plan_id.bytes_spec()
            &&& _inputs.kernels@[index].input_spec().profile.plan_id.bytes_spec()
                == _inputs.logical_authority.plan_id.bytes_spec()
        } by {
            _inputs.generated@[index]
                .expose_plan_operation(_inputs.generated_expected@[index]);
            _inputs.kernels@[index]
                .expose_plan_operation(_inputs.kernel_expected@[index]);
            assert(ferric_spec::canonical_expected_step_spec(
                _inputs.selection.role,
                _inputs.selection.mode,
                _inputs.selection.bucket,
                index as u32,
            ) == Some(_inputs.kernel_expected@[index].profile.step));
            assert(ferric_spec::canonical_expected_step_spec(
                _inputs.selection.role,
                _inputs.selection.mode,
                _inputs.selection.bucket,
                index as u32,
            ) == Some(_inputs.logical_plan.steps@[index]));
            assert(_inputs.kernel_expected@[index].profile.step
                == _inputs.logical_plan.steps@[index]);
        }

        ferric_engine::m1_target_completion_dispatch_shape();
        ferric_engine::m1_target_only_physical_recipe_shape();
        ferric_engine::m1_target_only_fixed_batch_shape();
        reveal(m1_generated_target_graph_composed);
    }
}

} // verus!
