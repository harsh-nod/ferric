/// Custody class of one actual retained-worker memory access.
/// No class establishes bounds, synchronization, or physical separation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionWaveTaskMemoryClassV1 {
    /// One of the exact entry components of the selected source profile.
    Global(ProductionWaveTaskAccessRootV1),
    /// A source-attributed shared-memory access; lifecycle checks remain required.
    Workgroup,
    /// A replay-authenticated compiler-owned retained-local or enum slot access.
    CompilerPrivate,
}

/// An exact KIR memory access, including effects outside the global-root view.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionWaveTaskMemoryCensusV1 {
    class: ProductionWaveTaskMemoryClassV1,
    location: FunctionOperationLocation,
    operation_access_ordinal: u32,
    pointer: ValueId,
    source: Option<SemanticAccessSiteV1>,
    access: dialect_kernel::AccessKindAttr,
    memory_space: dialect_kernel::MemorySpaceAttr,
    atomic: Option<NormalizedAtomicContractV1>,
}

impl ProductionWaveTaskMemoryCensusV1 {
    /// Custody classification, not a memory proof.
    pub const fn class(self) -> ProductionWaveTaskMemoryClassV1 {
        self.class
    }
    /// Operation location in this owner's selected KIR function.
    pub const fn location(self) -> FunctionOperationLocation {
        self.location
    }
    /// Original ordinal for a compound operation's memory access.
    pub const fn operation_access_ordinal(self) -> u32 {
        self.operation_access_ordinal
    }
    /// Actual pointer SSA value; callers must not replace its index with zero.
    pub const fn pointer(self) -> ValueId {
        self.pointer
    }
    /// Original semantic block, statement/terminator, and unfiltered ordinal.
    /// Authenticated synthetic private slot traffic has no source access ordinal.
    pub fn source_site(self) -> Option<(u32, Option<u32>, u32)> {
        self.source
            .map(|site| (site.block, site.statement, site.ordinal))
    }
    /// Exact modeled access kind.
    pub const fn access(self) -> dialect_kernel::AccessKindAttr {
        self.access
    }
    /// Exact actual memory space.
    pub const fn memory_space(self) -> dialect_kernel::MemorySpaceAttr {
        self.memory_space
    }
    /// Existing normalized ordering, scope, and failure-ordering ranks.
    pub fn atomic_contract(self) -> Option<(u8, u8, Option<u8>)> {
        self.atomic
            .map(|contract| (contract.ordering, contract.scope, contract.failure_ordering))
    }
}

/// Complete retained-worker custody census borrowing an immutable live owner.
///
/// This retains the actual numerical body, every memory effect, allocation,
/// synchronization operation, and CFG terminator. It does not prove arithmetic,
/// task scheduling, pipeline lifecycle, bounds, publication, progress, lifetime,
/// noalias, or target/runtime premises. Only the separate conditional route may
/// consume it; it grants no ordinary clean report or artifact/launch authority.
///
/// ```compile_fail
/// use fe2o3_lower_mir_kernel::ProductionPreRankedWaveTaskRecipeV1;
/// fn duplicate(view: ProductionPreRankedWaveTaskRecipeV1<'_>) {
///     let _copy = view.clone();
/// }
/// ```
pub struct ProductionPreRankedWaveTaskRecipeV1<'owner> {
    access_roots: ProductionPreRankedWaveTaskAccessRootsV1<'owner>,
    function_index: usize,
    memory: Vec<ProductionWaveTaskMemoryCensusV1>,
    effect_operations: Vec<FunctionOperationLocation>,
    effect_sources: Vec<(FunctionOperationLocation, (u32, Option<u32>))>,
    retained_storage: usize,
}

impl<'owner> ProductionPreRankedWaveTaskRecipeV1<'owner> {
    /// Exact admitted source owner, not a supplied source-facts table.
    pub fn source(&self) -> &'owner AdmittedInertSemanticMirV1 {
        self.access_roots.owner.semantic_ssa.source_semantic()
    }
    /// Selected root in that admitted source.
    pub const fn selected_root(&self) -> SemanticFunctionIdV1 {
        self.access_roots.selected_root
    }
    /// Selected actual semantic body, including transparent-wrapper selection.
    pub const fn selected_function(&self) -> SemanticFunctionIdV1 {
        self.access_roots.selected_function
    }
    /// Actual verified canonical KIR; numerical operations are not regenerated.
    pub fn executable(&self) -> &'owner fe2o3_kernel_ir::VerifiedCanonicalKernelIrModuleV12 {
        &self.access_roots.owner.executable
    }
    /// The selected actual KIR entry, including all terminators and value types.
    pub fn function(&self) -> &'owner Function {
        &self.executable().module().functions[self.function_index]
    }
    /// Exact selected function ordinal within `executable()`.
    pub const fn function_ordinal(&self) -> usize {
        self.function_index
    }
    /// Complete original entry-component roster, not the total allocation roster.
    pub fn roots(&self) -> &[ProductionWaveTaskRootV1] {
        self.access_roots.roots()
    }
    /// Exact global accesses with their original unfiltered source ordinals.
    pub fn accesses(&self) -> &[ProductionWaveTaskAccessRootV1] {
        self.access_roots.accesses()
    }
    /// Looks up an original source ordinal without per-statement coalescing.
    pub fn access_at(
        &self,
        block: u32,
        statement: Option<u32>,
        ordinal: u32,
    ) -> Option<&ProductionWaveTaskAccessRootV1> {
        self.access_roots.access_at(block, statement, ordinal)
    }
    /// All modeled memory accesses, including private and shared-memory traffic.
    pub fn memory_census(&self) -> &[ProductionWaveTaskMemoryCensusV1] {
        &self.memory
    }
    /// Every operation with an allocation, memory, fence, or synchronization effect.
    /// Pure numerical operations and CFG terminators remain in `function()`.
    pub fn effect_operations(&self) -> &[FunctionOperationLocation] {
        &self.effect_operations
    }
    /// Exact original source span for an effect, when source-authored. Synthetic
    /// private allocations have no fabricated statement or terminator identity.
    pub fn effect_source_site(
        &self,
        location: FunctionOperationLocation,
    ) -> Option<(u32, Option<u32>)> {
        self.effect_sources
            .binary_search_by_key(&location, |row| row.0)
            .ok()
            .map(|index| self.effect_sources[index].1)
    }
    /// Reads one actual operation by location in the selected function.
    pub fn operation(&self, location: FunctionOperationLocation) -> Option<&'owner Operation> {
        self.function()
            .body
            .as_ref()?
            .blocks
            .iter()
            .find(|block| block.id == location.block)?
            .operations
            .get(location.operation_index)
    }
    /// Checks exact semantic owner and selected root/body identities.
    pub fn validate_source_selection(
        &self,
        source: &AdmittedInertSemanticMirV1,
        root: SemanticFunctionIdV1,
        function: SemanticFunctionIdV1,
    ) -> Result<(), ProductionSemanticKirErrorV1> {
        self.access_roots
            .validate_source_selection(source, root, function)
    }
    /// Logical live payload retained in the caller's canonical storage ledger.
    pub const fn retained_storage(&self) -> usize {
        self.retained_storage
    }
    /// This census never grants artifact or launch authority.
    pub const fn grants_artifact_or_launch_authority(&self) -> bool {
        false
    }
}

fn wave_recipe_mismatch_v1() -> ProductionSemanticKirErrorV1 {
    ProductionSemanticKirErrorV1::CorrespondenceMismatch
}

fn wave_recipe_effect_source_v1(
    correspondence: &SemanticKirCorrespondenceV1,
    selected_root: SemanticFunctionIdV1,
    selected_function: SemanticFunctionIdV1,
    location: FunctionOperationLocation,
    budget: &mut UnsupportedIndexCorrelationBudgetV1,
) -> Result<Option<(u32, Option<u32>)>, ProductionSemanticKirErrorV1> {
    let mut source = None;
    for span in correspondence.statement_operation_spans() {
        budget.charge().ok_or_else(wave_access_resource_v1)?;
        if span.correspondence_owner() == selected_root
            && span.semantic_function() == selected_function
            && operation_span_contains_v1(
                span.kernel_ir_block(),
                span.first_operation_ordinal(),
                span.operation_count(),
                location,
            )
            && source
                .replace((
                    span.semantic_block().index(),
                    Some(span.statement_ordinal()),
                ))
                .is_some()
        {
            return Err(wave_recipe_mismatch_v1());
        }
    }
    for span in correspondence.terminator_operation_spans() {
        budget.charge().ok_or_else(wave_access_resource_v1)?;
        if span.correspondence_owner() == selected_root
            && span.semantic_function() == selected_function
            && operation_span_contains_v1(
                span.kernel_ir_block(),
                span.first_operation_ordinal(),
                span.operation_count(),
                location,
            )
            && source
                .replace((span.semantic_block().index(), None))
                .is_some()
        {
            return Err(wave_recipe_mismatch_v1());
        }
    }
    Ok(source)
}

impl ProductionSemanticKirOwnerV1 {
    /// Borrows retained raw worker reports without discharging their premises.
    pub fn conditional_wave_task_reports(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &fe2o3_pliron::ProductionWaveTaskConditionalPipelineReportV1,
        ),
    > {
        self.generic_checks.iter().filter_map(|checks| {
            checks
                .lowering
                .conditional_wave_tasks()
                .map(|input| (checks.function_name.as_str(), input.conditional_report()))
        })
    }

    pub(crate) fn conditional_wave_task_inputs(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            SemanticFunctionIdV1,
            &fe2o3_pliron::ProductionWaveTaskConditionalRankedInputV1,
        ),
    > {
        self.generic_checks.iter().filter_map(|checks| {
            checks
                .lowering
                .conditional_wave_tasks()
                .map(|input| (checks.function_name.as_str(), checks.selected_root, input))
        })
    }
}

// A fixed logical payload allowance per indexed input covers the simultaneous
// maps, sets, vectors, and row payloads in the existing correlation helpers.
// This is not an allocator/RSS estimate. The preflight counts every CFG edge
// argument separately, including repeated incoming SSA values.
const WAVE_RECIPE_INDEX_ITEM_STORAGE_V1: usize = 1024;

fn wave_recipe_index_storage_v1(
    owner: &ProductionPreRankedKirOwnerV1,
    body: &FunctionBody,
    budget: &mut fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1<'_>,
) -> Result<usize, ProductionSemanticKirErrorV1> {
    let mut items = body.parameters.len();
    let mut add = |count: usize| -> Result<(), ProductionSemanticKirErrorV1> {
        items = items
            .checked_add(count)
            .ok_or_else(wave_access_resource_v1)?;
        Ok(())
    };
    add(owner.correspondence.parameter_component_bindings().len())?;
    add(owner.correspondence.statement_operation_spans().len())?;
    add(owner.correspondence.terminator_operation_spans().len())?;
    add(owner.correspondence.synthetic_operation_spans().len())?;
    for block in &body.blocks {
        budget
            .charge_work(1)
            .map_err(|_| wave_access_resource_v1())?;
        add(1)?;
        add(block.parameters.len())?;
        add(block.operations.len())?;
        for (operation_index, operation) in block.operations.iter().enumerate() {
            budget
                .charge_work(1)
                .map_err(|_| wave_access_resource_v1())?;
            if !operation.has_complete_effect_summary() {
                let kind = match &operation.kind {
                    OperationKind::Call { .. } => ProductionWaveRecipeIncompleteEffectKindV1::Call,
                    OperationKind::InlineAssembly(_) => {
                        ProductionWaveRecipeIncompleteEffectKindV1::InlineAssembly
                    }
                    _ => ProductionWaveRecipeIncompleteEffectKindV1::Other,
                };
                return Err(
                    ProductionSemanticKirErrorV1::WaveRecipeIncompleteOperationEffects {
                        block: block.id.0,
                        operation: operation_index,
                        kind,
                    },
                );
            }
            add(operation.results.len())?;
        }
        match block
            .terminator
            .as_ref()
            .ok_or_else(wave_recipe_mismatch_v1)?
        {
            Terminator::Branch { arguments, .. } => add(arguments.len())?,
            Terminator::ConditionalBranch {
                then_arguments,
                else_arguments,
                ..
            } => {
                add(then_arguments.len())?;
                add(else_arguments.len())?;
            }
            Terminator::Switch {
                cases,
                default_arguments,
                ..
            } => {
                add(default_arguments.len())?;
                for case in cases {
                    budget
                        .charge_work(1)
                        .map_err(|_| wave_access_resource_v1())?;
                    add(case.arguments.len())?;
                }
            }
            Terminator::IntegerSwitch {
                cases,
                default_arguments,
                ..
            } => {
                add(default_arguments.len())?;
                for case in cases {
                    budget
                        .charge_work(1)
                        .map_err(|_| wave_access_resource_v1())?;
                    add(case.arguments.len())?;
                }
            }
            Terminator::Return { .. } | Terminator::Unreachable => {}
        }
    }
    items
        .checked_mul(WAVE_RECIPE_INDEX_ITEM_STORAGE_V1)
        .ok_or_else(wave_access_resource_v1)
}

impl ProductionPreRankedKirOwnerV1 {
    /// Replays a complete source/KIR custody census for one profile worker.
    ///
    /// Ordinary roots return `None`. The returned view retains its storage
    /// charge until its caller drops it and releases `retained_storage()`.
    /// All errors roll back new storage, but never erase accepted work charges.
    pub fn replay_wave_task_recipe_for_root_v1(
        &self,
        selected_root: SemanticFunctionIdV1,
        budget: &mut fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1<'_>,
    ) -> Result<Option<ProductionPreRankedWaveTaskRecipeV1<'_>>, ProductionSemanticKirErrorV1> {
        let owner_floor = self
            .executable_storage
            .retained_storage()
            .checked_add(self.assert_origin_storage().payload_storage())
            .ok_or_else(wave_access_resource_v1)?;
        if budget.storage() < owner_floor {
            return Err(wave_access_resource_v1());
        }
        let floor = budget.storage();
        let result = self.replay_wave_task_recipe_inner_v1(selected_root, budget);
        let retained = result
            .as_ref()
            .ok()
            .and_then(Option::as_ref)
            .map_or(0, |view| view.retained_storage);
        let release = budget
            .storage()
            .checked_sub(floor)
            .and_then(|value| value.checked_sub(retained))
            .ok_or_else(wave_access_resource_v1)?;
        budget
            .release_storage(release)
            .map_err(|_| wave_access_resource_v1())?;
        result
    }

    fn replay_wave_task_recipe_inner_v1(
        &self,
        selected_root: SemanticFunctionIdV1,
        budget: &mut fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1<'_>,
    ) -> Result<Option<ProductionPreRankedWaveTaskRecipeV1<'_>>, ProductionSemanticKirErrorV1> {
        let semantic = self.semantic_ssa.source_semantic();
        budget
            .charge_work(semantic.roots().len())
            .map_err(|_| wave_access_resource_v1())?;
        if !semantic.roots().contains(&selected_root) {
            return Err(wave_recipe_mismatch_v1());
        }
        let selection = semantic
            .select_kernel_body_for_root_v1(selected_root)
            .ok_or_else(wave_recipe_mismatch_v1)?;
        let source = semantic
            .functions()
            .get(selection.body().index() as usize)
            .ok_or_else(wave_recipe_mismatch_v1)?;
        budget
            .charge_work(source.abi().source_argument_ownership().len())
            .map_err(|_| wave_access_resource_v1())?;
        if !source
            .abi()
            .source_argument_ownership()
            .iter()
            .any(|ownership| wave_task_profile_v2(*ownership).is_some())
        {
            return Ok(None);
        }

        let mut function_index = None;
        for row in self.correspondence.lowered_functions() {
            budget
                .charge_work(1)
                .map_err(|_| wave_access_resource_v1())?;
            if row.correspondence_owner() != selected_root
                || row.semantic_function() != selection.body()
                || row.role() != SemanticKirFunctionRoleV1::KernelEntry
            {
                continue;
            }
            for (index, function) in self.executable.module().functions.iter().enumerate() {
                budget
                    .charge_work(1)
                    .map_err(|_| wave_access_resource_v1())?;
                if &function.id == row.kernel_ir_function()
                    && function_index.replace(index).is_some()
                {
                    return Err(wave_recipe_mismatch_v1());
                }
            }
        }
        let function_index = function_index.ok_or_else(wave_recipe_mismatch_v1)?;
        let function = &self.executable.module().functions[function_index];
        let body = function.body.as_ref().ok_or_else(wave_recipe_mismatch_v1)?;
        let ceiling = self
            .limits
            .max_operations
            .checked_mul(UNSUPPORTED_INDEX_CORRELATION_STEPS_PER_OPERATION_V1)
            .ok_or_else(wave_access_resource_v1)?;
        // The immutable pre-ranked owner already holds replay-validated SSA and
        // KIR. Re-derive all root/access facts with one correlation allowance;
        // no per-access reset can recover from an exhausted shared ledger.
        let scratch_storage = wave_recipe_index_storage_v1(self, body, budget)?;
        budget
            .reserve_storage(scratch_storage)
            .map_err(|_| wave_access_resource_v1())?;
        // Preflight is finished before this loan. Nothing inside the closure
        // charges canonical work; reconcile the accepted prefix on every Result.
        let allowance = ceiling.min(budget.remaining_work());
        let mut correlation = UnsupportedIndexCorrelationBudgetV1 {
            remaining: allowance,
        };
        let result = (|| {
            let (roots, accesses) = collect_wave_task_access_roots_v1(
                semantic.types(),
                source,
                &self.correspondence,
                selected_root,
                selection.body(),
                function,
                self.limits.max_operations,
                &mut correlation,
            )?;
            let access_roots = ProductionPreRankedWaveTaskAccessRootsV1 {
                owner: self,
                selected_root,
                selected_function: selection.body(),
                roots,
                accesses,
            };
            let kir =
                build_kir_correlation_index(body, self.limits.max_operations, &mut correlation)
                    .ok_or_else(wave_access_resource_v1)?;
            if !kir.unmodeled_memory_effects.is_empty() {
                return Err(wave_recipe_mismatch_v1());
            }
            let sites = index_semantic_access_sites(
                &self.correspondence,
                selected_root,
                selection.body(),
                &kir,
                &mut correlation,
            )
            .ok_or_else(wave_recipe_mismatch_v1)?;
            let retained_storage = access_roots
                .roots
                .len()
                .checked_mul(std::mem::size_of::<ProductionWaveTaskRootV1>())
                .and_then(|size| {
                    size.checked_add(
                        access_roots
                            .accesses
                            .len()
                            .checked_mul(std::mem::size_of::<ProductionWaveTaskAccessRootV1>())?,
                    )
                })
                .and_then(|size| {
                    size.checked_add(
                        kir.memory_consumers
                            .len()
                            .checked_mul(std::mem::size_of::<ProductionWaveTaskMemoryCensusV1>())?,
                    )
                })
                .and_then(|size| {
                    size.checked_add(
                        kir.operations
                            .len()
                            .checked_mul(std::mem::size_of::<FunctionOperationLocation>())?,
                    )
                })
                .and_then(|size| {
                    size.checked_add(kir.operations.len().checked_mul(std::mem::size_of::<(
                        FunctionOperationLocation,
                        (u32, Option<u32>),
                    )>())?)
                })
                .ok_or_else(wave_access_resource_v1)?;
            if retained_storage > scratch_storage {
                return Err(wave_access_resource_v1());
            }
            let mut memory = Vec::new();
            memory
                .try_reserve_exact(kir.memory_consumers.len())
                .map_err(|_| wave_access_resource_v1())?;
            let mut used_global = BTreeSet::new();
            for consumer in &kir.memory_consumers {
                correlation.charge().ok_or_else(wave_access_resource_v1)?;
                let site = sites
                    .get(&(consumer.location, consumer.operation_access_ordinal))
                    .copied();
                let class = match consumer.memory_space {
                    dialect_kernel::MemorySpaceAttr::Global => {
                        let site = site.ok_or_else(wave_recipe_mismatch_v1)?;
                        let row = *access_roots
                            .access_at(site.block, site.statement, site.ordinal)
                            .ok_or_else(wave_recipe_mismatch_v1)?;
                        if row.location != consumer.location
                            || row.operation_access_ordinal != consumer.operation_access_ordinal
                            || row.access != consumer.access
                            || row.atomic != consumer.atomic
                            || !used_global.insert(site)
                        {
                            return Err(wave_recipe_mismatch_v1());
                        }
                        ProductionWaveTaskMemoryClassV1::Global(row)
                    }
                    dialect_kernel::MemorySpaceAttr::Workgroup => {
                        site.ok_or_else(wave_recipe_mismatch_v1)?;
                        ProductionWaveTaskMemoryClassV1::Workgroup
                    }
                    dialect_kernel::MemorySpaceAttr::Private => {
                        if !compiler_owned_retained_local_storage_access_v1(
                            *consumer,
                            &kir,
                            &self.correspondence,
                            selected_root,
                            selection.body(),
                            &mut BTreeSet::new(),
                            &mut correlation,
                        )
                        .ok_or_else(wave_recipe_mismatch_v1)?
                        {
                            return Err(wave_recipe_mismatch_v1());
                        }
                        ProductionWaveTaskMemoryClassV1::CompilerPrivate
                    }
                    _ => return Err(wave_recipe_mismatch_v1()),
                };
                memory.push(ProductionWaveTaskMemoryCensusV1 {
                    class,
                    location: consumer.location,
                    operation_access_ordinal: consumer.operation_access_ordinal,
                    pointer: consumer.pointer,
                    source: site,
                    access: consumer.access,
                    memory_space: consumer.memory_space,
                    atomic: consumer.atomic,
                });
            }
            if used_global.len() != access_roots.accesses.len() {
                return Err(wave_recipe_mismatch_v1());
            }
            let mut effect_operations = Vec::new();
            effect_operations
                .try_reserve_exact(kir.operations.len())
                .map_err(|_| wave_access_resource_v1())?;
            let mut effect_sources = Vec::new();
            effect_sources
                .try_reserve_exact(kir.operations.len())
                .map_err(|_| wave_access_resource_v1())?;
            for (location, operation) in &kir.operations {
                correlation.charge().ok_or_else(wave_access_resource_v1)?;
                if operation.memory_effects().into_iter().next().is_some() {
                    effect_operations.push(*location);
                    let source_site = wave_recipe_effect_source_v1(
                        &self.correspondence,
                        selected_root,
                        selection.body(),
                        *location,
                        &mut correlation,
                    )?;
                    if let Some(source_site) = source_site {
                        effect_sources.push((*location, source_site));
                    } else if matches!(
                        operation.kind,
                        OperationKind::Barrier(_)
                            | OperationKind::Fence(_)
                            | OperationKind::WorkgroupBarrier(_)
                    ) {
                        return Err(wave_recipe_mismatch_v1());
                    }
                }
            }
            Ok(Some(ProductionPreRankedWaveTaskRecipeV1 {
                access_roots,
                function_index,
                memory,
                effect_operations,
                effect_sources,
                retained_storage,
            }))
        })();
        let consumed = allowance
            .checked_sub(correlation.remaining)
            .ok_or_else(wave_access_resource_v1)?;
        budget
            .charge_work(consumed)
            .map_err(|_| wave_access_resource_v1())?;
        result
    }
}

include!("wave_task_ranked_custody_v1.rs");
include!("../production_formal_memory_wave_tasks_v1.rs");
