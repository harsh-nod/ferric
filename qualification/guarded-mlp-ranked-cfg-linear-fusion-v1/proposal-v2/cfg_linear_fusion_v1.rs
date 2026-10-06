//! Bounded, ownership-preserving fusion of unconditional single-predecessor chains.

use super::*;
use fe2o3_kernel_ir::{
    CanonicalKernelIrVerificationResourceBudgetV1 as Budget,
    CanonicalKernelIrVerificationResourceErrorV1 as Resource,
};
use ranked_projection_source_v1::resource;

type Error = ProductionRankedProjectionErrorV1;
pub(super) type FusionOutput = (
    Vec<ProductionRankedBlockV1>,
    Vec<ProjectedAccessSourceV1>,
    Vec<ProductionRankedExecutableEffectSourceV1>,
    Vec<ProjectedWaveSyncSiteV1>,
);

#[derive(Clone, Copy)]
struct Plan {
    predecessors: usize,
    parent: Option<usize>,
    next: Option<usize>,
    block: usize,
    offset: usize,
    operations: usize,
}

fn malformed() -> Error {
    Error::Unsupported("ranked CFG linear fusion encountered an invalid coordinate")
}

fn checked_add(left: usize, right: usize) -> Result<usize, Error> {
    left.checked_add(right)
        .ok_or_else(|| resource(Resource::Arithmetic))
}

fn bytes<T>(count: usize) -> Result<usize, Error> {
    count.checked_mul(std::mem::size_of::<T>())
        .ok_or_else(|| resource(Resource::Arithmetic))
}

fn reserve<T>(values: &mut Vec<T>, count: usize) -> Result<(), Error> {
    values
        .try_reserve_exact(count)
        .map_err(|_| resource(Resource::Allocation))
}

fn targets(terminator: &ProductionRankedTerminatorV1) -> [Option<u32>; 2] {
    use ProductionRankedTerminatorV1 as T;
    match terminator {
        T::IndexLessThan { true_block, false_block, .. }
        | T::IndexLessThanArgs { true_block, false_block, .. }
        | T::IndexEqual { true_block, false_block, .. }
        | T::IndexEqualArgs { true_block, false_block, .. } => {
            [Some(*true_block), Some(*false_block)]
        }
        T::AnalysisSplit { first_block, second_block, .. }
        | T::AnalysisSplitArgs { first_block, second_block, .. } => {
            [Some(*first_block), Some(*second_block)]
        }
        T::Branch { target }
        | T::BranchArgs { target, .. }
        | T::BranchArgsAdd { target, .. }
        | T::BranchArgsAddAt { target, .. } => [Some(*target), None],
        T::Return | T::Trap => [None, None],
    }
}

fn plain_value(
    value: ProductionRankedValueV1,
    blocks: &[ProductionRankedBlockV1],
    budget: &mut Budget<'_>,
) -> Result<bool, Error> {
    budget.charge_work(1).map_err(resource)?;
    match value {
        ProductionRankedValueV1::BlockArgument { block, argument } => {
            let owner = blocks.get(block as usize).ok_or_else(malformed)?;
            if argument >= owner.index_argument_count() {
                return Err(malformed());
            }
            Ok(false)
        }
        ProductionRankedValueV1::Local(_) | ProductionRankedValueV1::Argument(_) => Ok(true),
    }
}

fn plain_values(
    values: &[ProductionRankedValueV1],
    blocks: &[ProductionRankedBlockV1],
    budget: &mut Budget<'_>,
) -> Result<bool, Error> {
    let mut plain = true;
    for value in values {
        plain &= plain_value(*value, blocks, budget)?;
    }
    Ok(plain)
}

fn plain_operation(
    operation: &ProductionRankedOperationV1,
    blocks: &[ProductionRankedBlockV1],
    budget: &mut Budget<'_>,
) -> Result<bool, Error> {
    use ProductionRankedOperationV1 as O;
    budget.charge_work(1).map_err(resource)?;
    match operation {
        O::ExecutionLayout { .. }
        | O::IndexConstant { .. }
        | O::IndexUnknown { .. }
        | O::InvocationIndex { .. }
        | O::AllocationEffect { .. }
        | O::Barrier { .. }
        | O::Fence { .. }
        | O::SemanticSymbol { .. }
        | O::SemanticConstant { .. } => Ok(true),
        O::View { dynamic_extents, .. } | O::ViewInSpace { dynamic_extents, .. } => {
            plain_values(dynamic_extents, blocks, budget)
        }
        O::DeterministicJoin { dependencies, .. } => plain_values(dependencies, blocks, budget),
        O::IndexUnsignedCast { source, .. } => plain_value(*source, blocks, budget),
        O::IndexBinary { lhs, rhs, .. } | O::SemanticBinary { lhs, rhs, .. } => {
            plain_values(&[*lhs, *rhs], blocks, budget)
        }
        O::Access { view, indices, .. }
        | O::AtomicAccess { view, indices, .. }
        | O::AtomicCompareExchangeAccess { view, indices, .. } => {
            let plain = plain_value(*view, blocks, budget)?;
            Ok(plain_values(indices, blocks, budget)? & plain)
        }
        O::ValueAccess { view, indices, value, .. }
        | O::AtomicValueAccess { view, indices, value, .. } => {
            let plain = plain_values(&[*view, *value], blocks, budget)?;
            Ok(plain_values(indices, blocks, budget)? & plain)
        }
        O::PipelineCreate { view, .. }
        | O::Dimension { view, .. }
        | O::OwnershipContract { view, .. } => plain_value(*view, blocks, budget),
        O::PipelineEvent { pipeline, epoch, slot, .. } => {
            plain_values(&[*pipeline, *epoch, *slot], blocks, budget)
        }
        O::PublicationAtomicLoadU32 { view, index, .. }
        | O::PublicationAtomicStoreU32 { view, index, .. } => {
            plain_values(&[*view, *index], blocks, budget)
        }
        O::PublicationReadGuard { index, physical_extent, acquired, .. } => {
            plain_values(&[*index, *physical_extent, *acquired], blocks, budget)
        }
        O::RequireEquivalent { actual, expected } => {
            plain_values(&[*actual, *expected], blocks, budget)
        }
        // Keep unfamiliar nested recipes in the original graph. This pass is
        // not an alternative validator for their operands or proof contracts.
        _ => Ok(false),
    }
}

fn plain_terminator(
    terminator: &ProductionRankedTerminatorV1,
    blocks: &[ProductionRankedBlockV1],
    budget: &mut Budget<'_>,
) -> Result<bool, Error> {
    use ProductionRankedTerminatorV1 as T;
    match terminator {
        T::IndexLessThan { lhs, rhs, .. } | T::IndexEqual { lhs, rhs, .. } => {
            plain_values(&[*lhs, *rhs], blocks, budget)
        }
        T::AnalysisSplit { control_dependencies, .. } => {
            plain_values(control_dependencies, blocks, budget)
        }
        T::Branch { .. } | T::Return | T::Trap => Ok(true),
        T::IndexLessThanArgs { lhs, rhs, true_arguments, false_arguments, .. }
        | T::IndexEqualArgs { lhs, rhs, true_arguments, false_arguments, .. } => {
            plain_values(&[*lhs, *rhs], blocks, budget)?;
            plain_values(true_arguments, blocks, budget)?;
            plain_values(false_arguments, blocks, budget)?;
            Ok(false)
        }
        T::AnalysisSplitArgs {
            control_dependencies, first_arguments, second_arguments, ..
        } => {
            plain_values(control_dependencies, blocks, budget)?;
            plain_values(first_arguments, blocks, budget)?;
            plain_values(second_arguments, blocks, budget)?;
            Ok(false)
        }
        T::BranchArgs { arguments, .. } => {
            plain_values(arguments, blocks, budget)?;
            Ok(false)
        }
        T::BranchArgsAdd { value, step, .. } => {
            plain_values(&[*value, *step], blocks, budget)?;
            Ok(false)
        }
        T::BranchArgsAddAt { arguments, step, .. } => {
            plain_values(arguments, blocks, budget)?;
            plain_value(*step, blocks, budget)?;
            Ok(false)
        }
    }
}

fn coordinate(
    blocks: &[ProductionRankedBlockV1],
    block: usize,
    operation: usize,
    budget: &mut Budget<'_>,
) -> Result<(), Error> {
    budget.charge_work(1).map_err(resource)?;
    if blocks.get(block).is_none_or(|owner| operation >= owner.operations().len()) {
        return Err(malformed());
    }
    Ok(())
}

fn remap_terminator(terminator: &mut ProductionRankedTerminatorV1, plan: &[Plan]) {
    use ProductionRankedTerminatorV1 as T;
    let remap = |target: &mut u32| *target = plan[*target as usize].block as u32;
    match terminator {
        T::IndexLessThan { true_block, false_block, .. }
        | T::IndexEqual { true_block, false_block, .. } => {
            remap(true_block);
            remap(false_block);
        }
        T::AnalysisSplit { first_block, second_block, .. } => {
            remap(first_block);
            remap(second_block);
        }
        T::Branch { target } => remap(target),
        T::Return | T::Trap => {}
        _ => unreachable!("argument-bearing terminators retain the original graph"),
    }
}

/// Owns every mutable output until success; fallback returns the original
/// owners. Input edges are counted raw, not deduplicated or prematurely
/// subjected to the downstream output-edge limit.
pub(super) fn fuse(
    mut blocks: Vec<ProductionRankedBlockV1>,
    mut sources: Vec<ProjectedAccessSourceV1>,
    mut effects: Vec<ProductionRankedExecutableEffectSourceV1>,
    mut waves: Vec<ProjectedWaveSyncSiteV1>,
    budget: &mut Budget<'_>,
) -> Result<FusionOutput, Error> {
    let count = blocks.len();
    if count == 0 || count > MAX_RANKED_BOUNDS_BLOCKS {
        return Err(Error::Unsupported("ranked CFG linear fusion input exceeds the block limit"));
    }
    let mut operations = 0;
    for block in &blocks {
        budget.charge_work(1).map_err(resource)?;
        operations = checked_add(operations, block.operations().len())?;
    }
    if checked_add(count, operations)? > MAX_RANKED_BOUNDS_OPERATIONS {
        return Err(Error::Unsupported("ranked CFG linear fusion input exceeds the operation limit"));
    }
    // Logical extra payload includes the plan, replacement block slots, and
    // moved-operation buffers while their old buffers may still coexist.
    let storage = checked_add(
        checked_add(bytes::<Plan>(count)?, bytes::<ProductionRankedBlockV1>(count)?)?,
        bytes::<ProductionRankedOperationV1>(operations)?,
    )?;
    budget.reserve_storage(storage).map_err(resource)?;
    let result = (|| {
        budget.charge_work(count).map_err(resource)?;
        let mut plan = Vec::new();
        reserve(&mut plan, count)?;
        plan.resize(count, Plan {
            predecessors: 0,
            parent: None,
            next: None,
            block: usize::MAX,
            offset: 0,
            operations: 0,
        });
        let mut supported = true;
        for block in &blocks {
            budget.charge_work(1).map_err(resource)?;
            supported &= block.index_argument_count() == 0;
            for target in targets(block.terminator()).into_iter().flatten() {
                budget.charge_work(1).map_err(resource)?;
                let target = plan.get_mut(target as usize).ok_or_else(malformed)?;
                target.predecessors = checked_add(target.predecessors, 1)?;
            }
            for operation in block.operations() {
                supported &= plain_operation(operation, &blocks, budget)?;
            }
            supported &= plain_terminator(block.terminator(), &blocks, budget)?;
        }
        // Validate coordinates even when a later whole-function fallback is
        // necessary; removing a branch must never conceal malformed records.
        for source in &sources {
            coordinate(&blocks, source.block, source.operation, budget)?;
        }
        for effect in &effects {
            coordinate(&blocks, effect.ranked_block() as usize, effect.ranked_operation() as usize, budget)?;
        }
        for wave in &waves {
            coordinate(&blocks, wave.ranked_block, wave.ranked_operation, budget)?;
        }
        if !supported {
            return Ok((blocks, sources, effects, waves));
        }
        for (source, block) in blocks.iter().enumerate() {
            budget.charge_work(1).map_err(resource)?;
            if let ProductionRankedTerminatorV1::Branch { target } = block.terminator() {
                let target = *target as usize;
                if source != 0 && target != 0 && source != target && plan[target].predecessors == 1 {
                    plan[source].next = Some(target);
                    plan[target].parent = Some(source);
                }
            }
        }
        let mut output_count = 0;
        let mut visited = 0;
        for head in 0..count {
            budget.charge_work(1).map_err(resource)?;
            if plan[head].parent.is_some() {
                continue;
            }
            let mut current = head;
            let mut offset = 0;
            loop {
                budget.charge_work(1).map_err(resource)?;
                if plan[current].block != usize::MAX {
                    return Ok((blocks, sources, effects, waves));
                }
                plan[current].block = output_count;
                plan[current].offset = offset;
                offset = checked_add(offset, blocks[current].operations().len())?;
                visited += 1;
                match plan[current].next {
                    Some(next) => current = next,
                    None => break,
                }
            }
            plan[head].operations = offset;
            output_count += 1;
        }
        // An all-unconditional cycle has no head. It is not eliminated, even
        // when unreachable; no reachability assumption participates in fusion.
        if visited != count || output_count == count {
            return Ok((blocks, sources, effects, waves));
        }
        budget.charge_work(output_count).map_err(resource)?;
        let mut output = Vec::new();
        reserve(&mut output, output_count)?;
        for head in 0..count {
            budget.charge_work(1).map_err(resource)?;
            if plan[head].parent.is_some() {
                continue;
            }
            let mut moved = Vec::new();
            reserve(&mut moved, plan[head].operations)?;
            let mut current = head;
            let mut terminator;
            loop {
                budget.charge_work(checked_add(1, blocks[current].operations().len())?).map_err(resource)?;
                let block = std::mem::replace(
                    &mut blocks[current],
                    ProductionRankedBlockV1::new(Vec::new(), ProductionRankedTerminatorV1::Return),
                );
                let (_, operations, last) = block.into_parts();
                moved.extend(operations);
                terminator = last;
                match plan[current].next {
                    Some(next) => current = next,
                    None => break,
                }
            }
            budget.charge_work(3).map_err(resource)?;
            remap_terminator(&mut terminator, &plan);
            output.push(ProductionRankedBlockV1::new(moved, terminator));
        }
        for source in &mut sources {
            budget.charge_work(1).map_err(resource)?;
            let owner = plan[source.block];
            source.block = owner.block;
            source.operation = checked_add(owner.offset, source.operation)?;
        }
        for effect in &mut effects {
            budget.charge_work(1).map_err(resource)?;
            let owner = plan[effect.ranked_block() as usize];
            *effect = ProductionRankedExecutableEffectSourceV1::new(
                effect.semantic_block(), effect.semantic_effect_ordinal(),
                owner.block as u32,
                checked_add(owner.offset, effect.ranked_operation() as usize)? as u32,
                effect.origin(), effect.recipe_identity(),
            );
        }
        for wave in &mut waves {
            budget.charge_work(1).map_err(resource)?;
            let owner = plan[wave.ranked_block];
            wave.ranked_block = owner.block;
            wave.ranked_operation = checked_add(owner.offset, wave.ranked_operation)?;
        }
        Ok((output, sources, effects, waves))
    })();
    // Scratch is either dropped or transferred with the returned graph before
    // restoring the caller's retained-owner floor. The peak remains recorded.
    budget.release_storage(storage).map_err(resource)?;
    result
}

pub(super) fn for_projection(
    blocks: Vec<ProductionRankedBlockV1>,
    sources: Vec<ProjectedAccessSourceV1>,
    effects: Vec<ProductionRankedExecutableEffectSourceV1>,
    waves: Vec<ProjectedWaveSyncSiteV1>,
    facts: &mut impl ProjectedAssertionFactsV1,
) -> Result<FusionOutput, Error> {
    if blocks.len() <= fe2o3_pliron::MAX_PLIRON_IDENTITY_BLOCKS_V1 {
        return Ok((blocks, sources, effects, waves));
    }
    facts.with_canonical_budget_v1(|budget| fuse(blocks, sources, effects, waves, budget))
}
