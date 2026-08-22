//! Deterministic initialized byte images for one exact M1 step workspace.
//!
//! This Ferric-owned bridge joins structurally validated runner inputs and an
//! owned physical KV page table to one exact addressless workspace plan. It
//! allocates only ordinary host bytes. It does not allocate device memory,
//! authenticate scheduler or KV ownership, construct addresses, publish a
//! queue, launch kernels, or grant runtime authority.

use core::fmt;

use ferric_build::{AddresslessM1StepWorkspacePlan, M1StepWorkspaceRangeRole};
use ferric_qwen_kernels::rope_kv::{
    QWEN3_KV_PAGE_TABLE_ENTRIES_V1, QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1,
    QWEN3_ROPE_KV_MAX_CONTEXT_TOKENS_V1, QWEN3_ROPE_PAIR_COUNT_V1, QWEN3_ROPE_THETA_V1,
    QWEN3_ROPE_TRIG_TABLE_ELEMENTS_V1,
};
use ferric_spec::{
    Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanSelection, TokenId, ValidatedM1StepInputs,
};

/// Exact physical page-table entries supplied for every selected sequence.
pub const M1_KV_PAGE_TABLE_ENTRIES_PER_SEQUENCE_V1: u32 = QWEN3_KV_PAGE_TABLE_ENTRIES_V1;
/// Exclusive upper bound for every physical KV page index.
pub const M1_KV_PHYSICAL_PAGE_SLOTS_V1: u32 = QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1;

const U32_BYTES: usize = 4;
const U64_BYTES: usize = 8;
const IDENTITY_BYTES: usize = 32;

/// A complete initialized host image retained with its exact addressless plan.
///
/// The image starts from canonical zero bytes. Only declared runner inputs,
/// target metadata, the physical page table, and deterministic Qwen3 `RoPE`
/// tables are populated. All numerical tensors and kernel outputs remain zero.
/// This type intentionally does not implement `Clone`.
///
/// ```compile_fail
/// use ferric_engine::ComposedM1StepWorkspaceImageV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<ComposedM1StepWorkspaceImageV1>();
/// ```
#[must_use = "the exact plan and initialized image remain retained"]
#[derive(Debug, Eq, PartialEq)]
pub struct ComposedM1StepWorkspaceImageV1 {
    plan: AddresslessM1StepWorkspacePlan,
    image: Box<[u8]>,
}

impl ComposedM1StepWorkspaceImageV1 {
    /// Returns the exact finite selection shared by the plan and image.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.plan.selection()
    }

    /// Borrows the retained addressless workspace plan.
    #[must_use]
    pub const fn plan(&self) -> &AddresslessM1StepWorkspacePlan {
        &self.plan
    }

    /// Borrows the complete initialized allocation image.
    #[must_use]
    pub fn image(&self) -> &[u8] {
        &self.image
    }

    /// Recovers the exact plan and complete initialized image.
    #[must_use]
    pub fn into_parts(self) -> (AddresslessM1StepWorkspacePlan, Box<[u8]>) {
        (self.plan, self.image)
    }

    /// This host-only value grants no allocation, address, or launch authority.
    #[must_use]
    pub const fn grants_runtime_authority(&self) -> bool {
        false
    }
}

/// Stable fail-closed reason for rejecting workspace image composition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1StepWorkspaceImageCompositionErrorV1 {
    /// The plan and structurally validated inputs selected different shapes.
    Selection {
        /// Selection retained by the exact plan.
        plan: Qwen3PlanSelection,
        /// Selection retained by the validated inputs.
        inputs: Qwen3PlanSelection,
    },
    /// The complete allocation extent cannot be represented on this host.
    HostImageExtent {
        /// Exact declared workspace byte extent.
        byte_len: u64,
    },
    /// Host reservation for the complete zero image failed.
    HostImageReservation {
        /// Exact requested host byte extent.
        byte_len: usize,
    },
    /// The owned physical page table has the wrong exact `[S,512]` length.
    KvPageIndexCount {
        /// Required entry count.
        expected: usize,
        /// Supplied entry count.
        actual: usize,
    },
    /// One physical page index names no slot in the fixed cache pool.
    KvPageIndexOutOfRange {
        /// Sequence lane containing the rejected entry.
        lane: usize,
        /// Entry within the sequence's 512-entry row.
        entry: usize,
        /// Rejected physical page index.
        page: u32,
    },
    /// An inactive sequence's page-table row was not canonical zero padding.
    InactiveKvPagePaddingNonZero {
        /// Inactive sequence lane containing the rejected entry.
        lane: usize,
        /// Entry within the inactive 512-entry row.
        entry: usize,
        /// Rejected nonzero page index.
        page: u32,
    },
    /// A speculative target future-token slot was not a zero assembly placeholder.
    SpeculativeFutureTokenNonZero {
        /// Live target sequence lane.
        lane: usize,
        /// Future token column in `[1,K]`.
        column: usize,
        /// Rejected external token value.
        token: TokenId,
    },
    /// An exact plan omitted or changed one range required by its selection.
    Layout {
        /// Required semantic range.
        role: M1StepWorkspaceRangeRole,
        /// Required byte extent.
        expected: u64,
        /// Actual byte extent, or `None` when the range was absent.
        actual: Option<u64>,
    },
    /// A required declared range could not be indexed within the host image.
    LayoutBounds {
        /// Required semantic range.
        role: M1StepWorkspaceRangeRole,
    },
    /// A finite shape calculation overflowed host arithmetic.
    ArithmeticOverflow,
}

impl fmt::Display for M1StepWorkspaceImageCompositionErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 step workspace image composition rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1StepWorkspaceImageCompositionErrorV1 {}

/// Retry-safe rejection retaining every exact input unchanged.
///
/// This type intentionally does not implement `Clone`; the addressless plan
/// and structurally validated input custody remain linear.
///
/// ```compile_fail
/// use ferric_engine::M1StepWorkspaceImageCompositionFailureV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1StepWorkspaceImageCompositionFailureV1>();
/// ```
#[must_use = "rejection retains all exact inputs for retry or explicit release"]
#[derive(Debug)]
pub struct M1StepWorkspaceImageCompositionFailureV1 {
    error: M1StepWorkspaceImageCompositionErrorV1,
    plan: AddresslessM1StepWorkspacePlan,
    inputs: ValidatedM1StepInputs,
    kv_page_indices: Box<[u32]>,
}

impl M1StepWorkspaceImageCompositionFailureV1 {
    /// Returns the stable rejection reason.
    #[must_use]
    pub const fn error(&self) -> M1StepWorkspaceImageCompositionErrorV1 {
        self.error
    }

    /// Borrows the exact unchanged plan.
    #[must_use]
    pub const fn plan(&self) -> &AddresslessM1StepWorkspacePlan {
        &self.plan
    }

    /// Borrows the exact unchanged structurally validated inputs.
    #[must_use]
    pub const fn inputs(&self) -> &ValidatedM1StepInputs {
        &self.inputs
    }

    /// Borrows the exact unchanged physical page-index table.
    #[must_use]
    pub fn kv_page_indices(&self) -> &[u32] {
        &self.kv_page_indices
    }

    /// Recovers the diagnostic and every exact unchanged input.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        M1StepWorkspaceImageCompositionErrorV1,
        AddresslessM1StepWorkspacePlan,
        ValidatedM1StepInputs,
        Box<[u32]>,
    ) {
        (self.error, self.plan, self.inputs, self.kv_page_indices)
    }
}

/// Linear result of one host-only workspace image composition attempt.
#[must_use]
#[derive(Debug)]
pub enum M1StepWorkspaceImageCompositionOutcomeV1 {
    /// The exact plan is retained with a complete initialized host image.
    Composed(ComposedM1StepWorkspaceImageV1),
    /// Pure preflight failed and retained every exact unchanged input.
    Rejected(M1StepWorkspaceImageCompositionFailureV1),
}

struct CheckedRange {
    start: usize,
    end: usize,
}

struct CheckedImageLayout {
    token_ids: CheckedRange,
    position_ids: CheckedRange,
    active_lengths: CheckedRange,
    context_lengths: CheckedRange,
    kv_page_indices: CheckedRange,
    rope_cos: CheckedRange,
    rope_sin: CheckedRange,
    request_slots: Option<CheckedRange>,
    request_generations: Option<CheckedRange>,
    completion_epochs: Option<CheckedRange>,
    plan_identities: Option<CheckedRange>,
    draft_position_ids: Option<CheckedRange>,
    draft_context_lengths: Option<CheckedRange>,
}

/// Composes the complete initialized host byte image for one exact workspace.
///
/// `kv_page_indices` must be the exact sequence-major `[S,512]` physical table.
/// Every page must be below 16,384 and every inactive sequence row must be all
/// zero. Speculative target inputs must expose only the external anchor token
/// in column zero; columns `[1,K]` must be canonical zero placeholders for the
/// assembly dispatch.
///
/// All checks, including fallible host reservation, occur without allocating
/// device memory or publishing work. Rejection retains the unchanged plan,
/// validated inputs, and owned page table.
pub fn compose_m1_step_workspace_image_v1(
    plan: AddresslessM1StepWorkspacePlan,
    inputs: ValidatedM1StepInputs,
    kv_page_indices: Box<[u32]>,
) -> M1StepWorkspaceImageCompositionOutcomeV1 {
    match compose_preflight(&plan, &inputs, &kv_page_indices) {
        Ok((layout, image_len)) => {
            let mut image = Vec::new();
            if image.try_reserve_exact(image_len).is_err() {
                return rejected(
                    M1StepWorkspaceImageCompositionErrorV1::HostImageReservation {
                        byte_len: image_len,
                    },
                    plan,
                    inputs,
                    kv_page_indices,
                );
            }
            image.resize(image_len, 0);
            populate_image(&mut image, &layout, &inputs, &kv_page_indices);
            M1StepWorkspaceImageCompositionOutcomeV1::Composed(ComposedM1StepWorkspaceImageV1 {
                plan,
                image: image.into_boxed_slice(),
            })
        }
        Err(error) => rejected(error, plan, inputs, kv_page_indices),
    }
}

fn rejected(
    error: M1StepWorkspaceImageCompositionErrorV1,
    plan: AddresslessM1StepWorkspacePlan,
    inputs: ValidatedM1StepInputs,
    kv_page_indices: Box<[u32]>,
) -> M1StepWorkspaceImageCompositionOutcomeV1 {
    M1StepWorkspaceImageCompositionOutcomeV1::Rejected(M1StepWorkspaceImageCompositionFailureV1 {
        error,
        plan,
        inputs,
        kv_page_indices,
    })
}

fn compose_preflight(
    plan: &AddresslessM1StepWorkspacePlan,
    inputs: &ValidatedM1StepInputs,
    kv_page_indices: &[u32],
) -> Result<(CheckedImageLayout, usize), M1StepWorkspaceImageCompositionErrorV1> {
    let selection = plan.selection();
    if selection != inputs.selection() {
        return Err(M1StepWorkspaceImageCompositionErrorV1::Selection {
            plan: selection,
            inputs: inputs.selection(),
        });
    }
    let dimensions = inputs.dimensions();
    let sequences = usize::try_from(dimensions.sequences)
        .map_err(|_| M1StepWorkspaceImageCompositionErrorV1::ArithmeticOverflow)?;
    let active_tokens = usize::try_from(dimensions.active_tokens)
        .map_err(|_| M1StepWorkspaceImageCompositionErrorV1::ArithmeticOverflow)?;
    let rows = sequences
        .checked_mul(active_tokens)
        .ok_or(M1StepWorkspaceImageCompositionErrorV1::ArithmeticOverflow)?;
    let page_entries = usize::try_from(QWEN3_KV_PAGE_TABLE_ENTRIES_V1)
        .map_err(|_| M1StepWorkspaceImageCompositionErrorV1::ArithmeticOverflow)?;
    let expected_pages = sequences
        .checked_mul(page_entries)
        .ok_or(M1StepWorkspaceImageCompositionErrorV1::ArithmeticOverflow)?;
    if kv_page_indices.len() != expected_pages {
        return Err(M1StepWorkspaceImageCompositionErrorV1::KvPageIndexCount {
            expected: expected_pages,
            actual: kv_page_indices.len(),
        });
    }
    let live_lanes = usize::try_from(inputs.live_lane_count())
        .map_err(|_| M1StepWorkspaceImageCompositionErrorV1::ArithmeticOverflow)?;
    for (flat, page) in kv_page_indices.iter().copied().enumerate() {
        let lane = flat / page_entries;
        let entry = flat % page_entries;
        if page >= QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 {
            return Err(
                M1StepWorkspaceImageCompositionErrorV1::KvPageIndexOutOfRange { lane, entry, page },
            );
        }
        if lane >= live_lanes && page != 0 {
            return Err(
                M1StepWorkspaceImageCompositionErrorV1::InactiveKvPagePaddingNonZero {
                    lane,
                    entry,
                    page,
                },
            );
        }
    }
    if selection.role == Qwen3ModelRole::Target8B
        && selection.mode == Qwen3ExecutionMode::Speculative
    {
        for lane in 0..live_lanes {
            for column in 1..active_tokens {
                let token = inputs.token_ids()[lane * active_tokens + column];
                if token != 0 {
                    return Err(
                        M1StepWorkspaceImageCompositionErrorV1::SpeculativeFutureTokenNonZero {
                            lane,
                            column,
                            token,
                        },
                    );
                }
            }
        }
    }

    let image_byte_len = plan.allocation().byte_len();
    let image_len = usize::try_from(image_byte_len).map_err(|_| {
        M1StepWorkspaceImageCompositionErrorV1::HostImageExtent {
            byte_len: image_byte_len,
        }
    })?;
    for range in plan.ranges() {
        let end = range
            .checked_end()
            .and_then(|end| usize::try_from(end).ok())
            .ok_or(M1StepWorkspaceImageCompositionErrorV1::LayoutBounds { role: range.role() })?;
        if end > image_len {
            return Err(M1StepWorkspaceImageCompositionErrorV1::LayoutBounds {
                role: range.role(),
            });
        }
    }

    let row_u32 = byte_count(rows, U32_BYTES)?;
    let sequence_u32 = byte_count(sequences, U32_BYTES)?;
    let page_u32 = byte_count(expected_pages, U32_BYTES)?;
    let trig_bytes = QWEN3_ROPE_TRIG_TABLE_ELEMENTS_V1
        .checked_mul(4)
        .ok_or(M1StepWorkspaceImageCompositionErrorV1::ArithmeticOverflow)?;
    let target = selection.role == Qwen3ModelRole::Target8B;
    let speculative_target = target && selection.mode == Qwen3ExecutionMode::Speculative;
    let draft_count = active_tokens
        .checked_sub(1)
        .ok_or(M1StepWorkspaceImageCompositionErrorV1::ArithmeticOverflow)?;
    let draft_metadata = byte_count(
        sequences
            .checked_mul(draft_count)
            .ok_or(M1StepWorkspaceImageCompositionErrorV1::ArithmeticOverflow)?,
        U32_BYTES,
    )?;

    Ok((
        CheckedImageLayout {
            token_ids: checked_range(plan, M1StepWorkspaceRangeRole::TokenIds, row_u32, image_len)?,
            position_ids: checked_range(
                plan,
                M1StepWorkspaceRangeRole::PositionIds,
                row_u32,
                image_len,
            )?,
            active_lengths: checked_range(
                plan,
                M1StepWorkspaceRangeRole::ActiveLengths,
                sequence_u32,
                image_len,
            )?,
            context_lengths: checked_range(
                plan,
                M1StepWorkspaceRangeRole::ContextLengths,
                sequence_u32,
                image_len,
            )?,
            kv_page_indices: checked_range(
                plan,
                M1StepWorkspaceRangeRole::KvPageIndices,
                page_u32,
                image_len,
            )?,
            rope_cos: checked_range(
                plan,
                M1StepWorkspaceRangeRole::RopeCosTable,
                trig_bytes,
                image_len,
            )?,
            rope_sin: checked_range(
                plan,
                M1StepWorkspaceRangeRole::RopeSinTable,
                trig_bytes,
                image_len,
            )?,
            request_slots: optional_target_range(
                plan,
                target,
                M1StepWorkspaceRangeRole::RequestSlots,
                sequence_u32,
                image_len,
            )?,
            request_generations: optional_target_range(
                plan,
                target,
                M1StepWorkspaceRangeRole::RequestGenerations,
                sequence_u32,
                image_len,
            )?,
            completion_epochs: optional_target_range(
                plan,
                target,
                M1StepWorkspaceRangeRole::CompletionEpochs,
                byte_count(sequences, U64_BYTES)?,
                image_len,
            )?,
            plan_identities: optional_target_range(
                plan,
                target,
                M1StepWorkspaceRangeRole::PlanIdentities,
                byte_count(sequences, IDENTITY_BYTES)?,
                image_len,
            )?,
            draft_position_ids: optional_target_range(
                plan,
                speculative_target,
                M1StepWorkspaceRangeRole::DraftPositionIds,
                draft_metadata,
                image_len,
            )?,
            draft_context_lengths: optional_target_range(
                plan,
                speculative_target,
                M1StepWorkspaceRangeRole::DraftContextLengths,
                draft_metadata,
                image_len,
            )?,
        },
        image_len,
    ))
}

fn byte_count(
    elements: usize,
    element_bytes: usize,
) -> Result<u64, M1StepWorkspaceImageCompositionErrorV1> {
    u64::try_from(elements)
        .ok()
        .and_then(|elements| {
            u64::try_from(element_bytes)
                .ok()
                .and_then(|element_bytes| elements.checked_mul(element_bytes))
        })
        .ok_or(M1StepWorkspaceImageCompositionErrorV1::ArithmeticOverflow)
}

fn checked_range(
    plan: &AddresslessM1StepWorkspacePlan,
    role: M1StepWorkspaceRangeRole,
    expected: u64,
    image_len: usize,
) -> Result<CheckedRange, M1StepWorkspaceImageCompositionErrorV1> {
    let Some(range) = plan.range(role) else {
        return Err(M1StepWorkspaceImageCompositionErrorV1::Layout {
            role,
            expected,
            actual: None,
        });
    };
    if range.byte_len() != expected {
        return Err(M1StepWorkspaceImageCompositionErrorV1::Layout {
            role,
            expected,
            actual: Some(range.byte_len()),
        });
    }
    let start = usize::try_from(range.offset())
        .map_err(|_| M1StepWorkspaceImageCompositionErrorV1::LayoutBounds { role })?;
    let end = usize::try_from(
        range
            .checked_end()
            .ok_or(M1StepWorkspaceImageCompositionErrorV1::LayoutBounds { role })?,
    )
    .map_err(|_| M1StepWorkspaceImageCompositionErrorV1::LayoutBounds { role })?;
    if start > end || end > image_len {
        return Err(M1StepWorkspaceImageCompositionErrorV1::LayoutBounds { role });
    }
    Ok(CheckedRange { start, end })
}

fn optional_target_range(
    plan: &AddresslessM1StepWorkspacePlan,
    present: bool,
    role: M1StepWorkspaceRangeRole,
    expected: u64,
    image_len: usize,
) -> Result<Option<CheckedRange>, M1StepWorkspaceImageCompositionErrorV1> {
    if present {
        checked_range(plan, role, expected, image_len).map(Some)
    } else if let Some(range) = plan.range(role) {
        Err(M1StepWorkspaceImageCompositionErrorV1::Layout {
            role,
            expected: 0,
            actual: Some(range.byte_len()),
        })
    } else {
        Ok(None)
    }
}

fn populate_image(
    image: &mut [u8],
    layout: &CheckedImageLayout,
    inputs: &ValidatedM1StepInputs,
    kv_page_indices: &[u32],
) {
    let selection = inputs.selection();
    let dimensions = inputs.dimensions();
    let sequences = dimensions.sequences as usize;
    let active_tokens = dimensions.active_tokens as usize;
    if selection.role == Qwen3ModelRole::Target8B
        && selection.mode == Qwen3ExecutionMode::Speculative
    {
        let mut anchors = vec![0; sequences * active_tokens];
        for lane in 0..inputs.live_lane_count() as usize {
            anchors[lane * active_tokens] = inputs.token_ids()[lane * active_tokens];
        }
        write_u32(
            &mut image[layout.token_ids.start..layout.token_ids.end],
            &anchors,
        );
    } else {
        write_u32(
            &mut image[layout.token_ids.start..layout.token_ids.end],
            inputs.token_ids(),
        );
    }
    write_u32(
        &mut image[layout.position_ids.start..layout.position_ids.end],
        inputs.position_ids(),
    );
    write_u32(
        &mut image[layout.active_lengths.start..layout.active_lengths.end],
        inputs.active_lengths(),
    );
    write_u32(
        &mut image[layout.context_lengths.start..layout.context_lengths.end],
        inputs.context_lengths(),
    );
    write_u32(
        &mut image[layout.kv_page_indices.start..layout.kv_page_indices.end],
        kv_page_indices,
    );
    write_rope_table(
        &mut image[layout.rope_cos.start..layout.rope_cos.end],
        false,
    );
    write_rope_table(&mut image[layout.rope_sin.start..layout.rope_sin.end], true);

    if let (
        Some(request_slots),
        Some(request_generations),
        Some(completion_epochs),
        Some(plan_identities),
    ) = (
        &layout.request_slots,
        &layout.request_generations,
        &layout.completion_epochs,
        &layout.plan_identities,
    ) {
        for lane in 0..inputs.live_lane_count() as usize {
            let plan = inputs.lanes()[lane]
                .as_ref()
                .expect("validated live-prefix lane must contain a StepPlan");
            write_u32_at(
                image,
                request_slots.start / U32_BYTES + lane,
                plan.request().slot(),
            );
            write_u32_at(
                image,
                request_generations.start / U32_BYTES + lane,
                plan.request().generation(),
            );
            write_u64_at(
                image,
                completion_epochs.start / U64_BYTES + lane,
                plan.completion_epoch().value(),
            );
            let start = plan_identities.start + lane * IDENTITY_BYTES;
            image[start..start + IDENTITY_BYTES].copy_from_slice(plan.plan_id().as_bytes());
        }
    }

    if let (Some(draft_positions), Some(draft_context)) =
        (&layout.draft_position_ids, &layout.draft_context_lengths)
    {
        let draft_iterations = active_tokens - 1;
        for iteration in 0..draft_iterations {
            for lane in 0..inputs.live_lane_count() as usize {
                let row = iteration * sequences + lane;
                write_u32_at(
                    image,
                    draft_positions.start / U32_BYTES + row,
                    inputs.position_ids()[lane * active_tokens + iteration],
                );
                write_u32_at(
                    image,
                    draft_context.start / U32_BYTES + row,
                    inputs.context_lengths()[lane]
                        + u32::try_from(iteration).expect("bounded draft iteration"),
                );
            }
        }
    }
}

fn write_u32(destination: &mut [u8], values: &[u32]) {
    debug_assert_eq!(destination.len(), values.len() * U32_BYTES);
    for (bytes, value) in destination.chunks_exact_mut(U32_BYTES).zip(values) {
        bytes.copy_from_slice(&value.to_le_bytes());
    }
}

fn write_u32_at(destination: &mut [u8], index: usize, value: u32) {
    let start = index * U32_BYTES;
    destination[start..start + U32_BYTES].copy_from_slice(&value.to_le_bytes());
}

fn write_u64_at(destination: &mut [u8], index: usize, value: u64) {
    let start = index * U64_BYTES;
    destination[start..start + U64_BYTES].copy_from_slice(&value.to_le_bytes());
}

fn write_rope_table(table: &mut [u8], sine: bool) {
    debug_assert_eq!(
        table.len(),
        usize::try_from(QWEN3_ROPE_TRIG_TABLE_ELEMENTS_V1).expect("bounded RoPE table") * 4
    );
    let theta = f64::from(QWEN3_ROPE_THETA_V1);
    for pair in 0..QWEN3_ROPE_PAIR_COUNT_V1 {
        let inverse_frequency = theta.powf(-f64::from(pair) / f64::from(QWEN3_ROPE_PAIR_COUNT_V1));
        for position in 0..QWEN3_ROPE_KV_MAX_CONTEXT_TOKENS_V1 {
            let angle = f64::from(position) * inverse_frequency;
            let index = usize::try_from(position * QWEN3_ROPE_PAIR_COUNT_V1 + pair)
                .expect("bounded RoPE index");
            let value = if sine { angle.sin() } else { angle.cos() };
            write_u32_at(table, index, narrow_f64_to_f32(value).to_bits());
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn narrow_f64_to_f32(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::{
        compose_m1_step_workspace_image_v1, compose_preflight, narrow_f64_to_f32,
        M1StepWorkspaceImageCompositionErrorV1, M1StepWorkspaceImageCompositionOutcomeV1,
    };
    use ferric_build::{
        m1_step_workspace_requirements, plan_addressless_m1_step_workspace,
        AddresslessM1StepWorkspacePlan, AvailableM1StepWorkspace,
        DeclaredM1StepWorkspaceAllocation, M1StepWorkspaceDeclaration, M1StepWorkspacePlanOutcome,
        M1StepWorkspaceRangeRole,
    };
    use ferric_spec::completion::CompletionEpoch;
    use ferric_spec::{
        validate_m1_step_inputs, Identity, M1StepInputCandidate, M1StepInputValidationOutcome,
        Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection, RequestId,
        StepPlan, ValidatedM1StepInputs,
    };

    const ALL_SELECTIONS: [Qwen3PlanSelection; 22] = [
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS8T128,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T512,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T2048,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS32C8192,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K16C8192,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS8T128,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T512,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T2048,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS32C8192,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K16C8192,
        ),
    ];

    const fn selection(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    fn exact_plan(selection: Qwen3PlanSelection, seed: u8) -> AddresslessM1StepWorkspacePlan {
        let requirements = m1_step_workspace_requirements(selection).expect("finite selection");
        let available = AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
            selection,
            DeclaredM1StepWorkspaceAllocation::new(
                Identity::new([seed; 32]),
                requirements.allocation_byte_len(),
                requirements.allocation_alignment(),
            ),
            requirements.ranges().to_vec().into_boxed_slice(),
        ));
        let M1StepWorkspacePlanOutcome::Planned(plan) =
            plan_addressless_m1_step_workspace(selection, available)
        else {
            panic!("exact fixture must plan")
        };
        plan
    }

    fn validated_inputs(
        selection: Qwen3PlanSelection,
        live_lanes: usize,
        speculative_target_placeholders: bool,
    ) -> ValidatedM1StepInputs {
        validated_inputs_with_identity(
            selection,
            live_lanes,
            speculative_target_placeholders,
            [0x41; 32],
        )
    }

    fn validated_inputs_with_identity(
        selection: Qwen3PlanSelection,
        live_lanes: usize,
        speculative_target_placeholders: bool,
        plan_identity: [u8; 32],
    ) -> ValidatedM1StepInputs {
        let dimensions = selection
            .bucket
            .dimensions(selection.role, selection.mode)
            .expect("finite selection");
        let sequences = dimensions.sequences as usize;
        let width = dimensions.active_tokens as usize;
        assert!((1..=sequences).contains(&live_lanes));
        let mut lanes = Vec::with_capacity(sequences);
        let mut tokens = vec![0; sequences * width];
        let mut positions = vec![0; sequences * width];
        let mut active = vec![0; sequences];
        let mut context = vec![0; sequences];
        for lane in 0..live_lanes {
            let lane_u32 = u32::try_from(lane).expect("bounded lane");
            let request_slot = if sequences == 32 {
                lane_u32
            } else {
                7 + lane_u32
            };
            lanes.push(Some(StepPlan::new(
                RequestId::new(request_slot, 19 + lane_u32),
                CompletionEpoch::new(23),
                Identity::new(plan_identity),
                selection,
            )));
            active[lane] = dimensions.active_tokens;
            context[lane] = match selection.mode {
                Qwen3ExecutionMode::Prefill => 0,
                Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => 31,
            };
            for column in 0..width {
                let column_u32 = u32::try_from(column).expect("bounded active width");
                let flat = lane * width + column;
                let future_target = speculative_target_placeholders
                    && selection.role == Qwen3ModelRole::Target8B
                    && selection.mode == Qwen3ExecutionMode::Speculative
                    && column > 0;
                tokens[flat] = if future_target {
                    0
                } else {
                    100 + lane_u32 * 32 + column_u32
                };
                positions[flat] = context[lane] + column_u32;
            }
        }
        lanes.resize(sequences, None);
        let candidate =
            M1StepInputCandidate::new(selection, lanes, tokens, positions, active, context);
        let M1StepInputValidationOutcome::Validated(inputs) = validate_m1_step_inputs(candidate)
        else {
            panic!("canonical fixture must validate")
        };
        inputs
    }

    fn page_table(selection: Qwen3PlanSelection, live_lanes: usize) -> Box<[u32]> {
        let sequences = selection
            .bucket
            .dimensions(selection.role, selection.mode)
            .expect("finite selection")
            .sequences as usize;
        let mut pages = vec![0; sequences * 512];
        for lane in 0..live_lanes {
            for entry in 0..512 {
                pages[lane * 512 + entry] =
                    u32::try_from(lane * 512 + entry + 1).expect("bounded physical page fixture");
            }
        }
        pages.into_boxed_slice()
    }

    fn range_bytes<'a>(
        plan: &AddresslessM1StepWorkspacePlan,
        image: &'a [u8],
        role: M1StepWorkspaceRangeRole,
    ) -> &'a [u8] {
        let range = plan.range(role).expect("required fixture range");
        let start = usize::try_from(range.offset()).expect("bounded range start");
        let end = usize::try_from(range.checked_end().expect("bounded range end"))
            .expect("host-representable range end");
        &image[start..end]
    }

    fn read_u32(bytes: &[u8]) -> Vec<u32> {
        bytes
            .chunks_exact(4)
            .map(|word| u32::from_le_bytes(word.try_into().expect("four bytes")))
            .collect()
    }

    fn read_u64(bytes: &[u8]) -> Vec<u64> {
        bytes
            .chunks_exact(8)
            .map(|word| u64::from_le_bytes(word.try_into().expect("eight bytes")))
            .collect()
    }

    #[test]
    fn all_22_finite_shapes_pass_exact_layout_preflight_without_image_allocation() {
        for (index, selection) in ALL_SELECTIONS.iter().copied().enumerate() {
            let dimensions = selection
                .bucket
                .dimensions(selection.role, selection.mode)
                .unwrap();
            let live_lanes = if dimensions.sequences > 1 {
                dimensions.sequences as usize - 1
            } else {
                1
            };
            let plan = exact_plan(
                selection,
                u8::try_from(index + 1).expect("22 finite selections"),
            );
            let inputs = validated_inputs(selection, live_lanes, true);
            let pages = page_table(selection, live_lanes);
            let (_, image_len) =
                compose_preflight(&plan, &inputs, &pages).expect("finite exact layout");
            assert_eq!(image_len as u64, plan.allocation().byte_len());
        }
    }

    #[test]
    fn target_metadata_rope_inputs_and_all_outputs_have_exact_bytes() {
        let selection = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let plan = exact_plan(selection, 1);
        let inputs = validated_inputs(selection, 3, false);
        let expected_tokens = inputs.token_ids().to_vec();
        let expected_positions = inputs.position_ids().to_vec();
        let expected_active = inputs.active_lengths().to_vec();
        let expected_context = inputs.context_lengths().to_vec();
        let pages = page_table(selection, 3);
        let expected_pages = pages.to_vec();
        let M1StepWorkspaceImageCompositionOutcomeV1::Composed(composed) =
            compose_m1_step_workspace_image_v1(plan, inputs, pages)
        else {
            panic!("exact image must compose")
        };
        let (plan, image) = composed.into_parts();
        assert_eq!(image.len() as u64, plan.allocation().byte_len());
        assert_eq!(
            read_u32(range_bytes(
                &plan,
                &image,
                M1StepWorkspaceRangeRole::TokenIds
            )),
            expected_tokens
        );
        assert_eq!(
            read_u32(range_bytes(
                &plan,
                &image,
                M1StepWorkspaceRangeRole::PositionIds
            )),
            expected_positions
        );
        assert_eq!(
            read_u32(range_bytes(
                &plan,
                &image,
                M1StepWorkspaceRangeRole::ActiveLengths
            )),
            expected_active
        );
        assert_eq!(
            read_u32(range_bytes(
                &plan,
                &image,
                M1StepWorkspaceRangeRole::ContextLengths
            )),
            expected_context
        );
        assert_eq!(
            read_u32(range_bytes(
                &plan,
                &image,
                M1StepWorkspaceRangeRole::KvPageIndices
            )),
            expected_pages
        );
        assert_eq!(
            read_u32(range_bytes(
                &plan,
                &image,
                M1StepWorkspaceRangeRole::RequestSlots
            )),
            vec![7, 8, 9, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            read_u32(range_bytes(
                &plan,
                &image,
                M1StepWorkspaceRangeRole::RequestGenerations
            )),
            vec![19, 20, 21, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            read_u64(range_bytes(
                &plan,
                &image,
                M1StepWorkspaceRangeRole::CompletionEpochs
            )),
            vec![23, 23, 23, 0, 0, 0, 0, 0]
        );
        let identities = range_bytes(&plan, &image, M1StepWorkspaceRangeRole::PlanIdentities);
        assert!(identities[..3 * 32].iter().all(|byte| *byte == 0x41));
        assert!(identities[3 * 32..].iter().all(|byte| *byte == 0));
        for role in [
            M1StepWorkspaceRangeRole::ResidualHidden,
            M1StepWorkspaceRangeRole::Logits,
            M1StepWorkspaceRangeRole::Choices,
            M1StepWorkspaceRangeRole::CompactCompletionRecords,
        ] {
            assert!(range_bytes(&plan, &image, role)
                .iter()
                .all(|byte| *byte == 0));
        }
        let cos = read_u32(range_bytes(
            &plan,
            &image,
            M1StepWorkspaceRangeRole::RopeCosTable,
        ));
        let sin = read_u32(range_bytes(
            &plan,
            &image,
            M1StepWorkspaceRangeRole::RopeSinTable,
        ));
        assert_eq!(cos[0], 1.0_f32.to_bits());
        assert_eq!(sin[0], 0.0_f32.to_bits());
        assert_eq!(cos[64], narrow_f64_to_f32(1.0_f64.cos()).to_bits());
        assert_eq!(sin[64], narrow_f64_to_f32(1.0_f64.sin()).to_bits());
        let expected_angle = 1.0 / 1_000_000_f64.powf(1.0 / 64.0);
        assert_eq!(cos[65], narrow_f64_to_f32(expected_angle.cos()).to_bits());
        assert_eq!(sin[65], narrow_f64_to_f32(expected_angle.sin()).to_bits());
    }

    #[test]
    fn speculative_target_keeps_only_anchor_and_derives_iteration_major_metadata() {
        let selection = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
        );
        let plan = exact_plan(selection, 2);
        let inputs = validated_inputs(selection, 2, true);
        let expected_positions = inputs.position_ids().to_vec();
        let M1StepWorkspaceImageCompositionOutcomeV1::Composed(composed) =
            compose_m1_step_workspace_image_v1(plan, inputs, page_table(selection, 2))
        else {
            panic!("exact speculative image must compose")
        };
        let (plan, image) = composed.into_parts();
        assert_eq!(
            read_u32(range_bytes(
                &plan,
                &image,
                M1StepWorkspaceRangeRole::TokenIds
            )),
            vec![
                100, 0, 0, 0, 0, 132, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ]
        );
        assert_eq!(
            read_u32(range_bytes(
                &plan,
                &image,
                M1StepWorkspaceRangeRole::PositionIds
            )),
            expected_positions
        );
        assert_eq!(
            read_u32(range_bytes(
                &plan,
                &image,
                M1StepWorkspaceRangeRole::DraftPositionIds
            )),
            vec![
                31, 31, 0, 0, 0, 0, 0, 0, 32, 32, 0, 0, 0, 0, 0, 0, 33, 33, 0, 0, 0, 0, 0, 0, 34,
                34, 0, 0, 0, 0, 0, 0
            ]
        );
        assert_eq!(
            read_u32(range_bytes(
                &plan,
                &image,
                M1StepWorkspaceRangeRole::DraftContextLengths
            )),
            vec![
                31, 31, 0, 0, 0, 0, 0, 0, 32, 32, 0, 0, 0, 0, 0, 0, 33, 33, 0, 0, 0, 0, 0, 0, 34,
                34, 0, 0, 0, 0, 0, 0
            ]
        );
        for role in [
            M1StepWorkspaceRangeRole::DraftChoices,
            M1StepWorkspaceRangeRole::Choices,
            M1StepWorkspaceRangeRole::Logits,
        ] {
            assert!(range_bytes(&plan, &image, role)
                .iter()
                .all(|byte| *byte == 0));
        }
    }

    #[test]
    fn hostile_length_selection_placeholder_page_and_inactive_padding_retain_inputs() {
        let target_decode = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let short_pages = vec![0; 8 * 512 - 1].into_boxed_slice();
        let short_pointer = short_pages.as_ptr();
        let M1StepWorkspaceImageCompositionOutcomeV1::Rejected(failure) =
            compose_m1_step_workspace_image_v1(
                exact_plan(target_decode, 3),
                validated_inputs(target_decode, 2, false),
                short_pages,
            )
        else {
            panic!("short page table must reject")
        };
        assert_eq!(failure.kv_page_indices().as_ptr(), short_pointer);
        assert_eq!(
            failure.error(),
            M1StepWorkspaceImageCompositionErrorV1::KvPageIndexCount {
                expected: 8 * 512,
                actual: 8 * 512 - 1,
            }
        );

        let draft_decode = selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let inputs = validated_inputs(draft_decode, 2, false);
        let expected_tokens = inputs.token_ids().to_vec();
        let M1StepWorkspaceImageCompositionOutcomeV1::Rejected(failure) =
            compose_m1_step_workspace_image_v1(
                exact_plan(target_decode, 4),
                inputs,
                page_table(draft_decode, 2),
            )
        else {
            panic!("selection mismatch must reject")
        };
        assert!(matches!(
            failure.error(),
            M1StepWorkspaceImageCompositionErrorV1::Selection { .. }
        ));
        let (_, plan, inputs, pages) = failure.into_parts();
        assert_eq!(plan.selection(), target_decode);
        assert_eq!(inputs.selection(), draft_decode);
        assert_eq!(inputs.token_ids(), expected_tokens);
        assert_eq!(pages.len(), 8 * 512);

        let target_spec = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        let M1StepWorkspaceImageCompositionOutcomeV1::Rejected(failure) =
            compose_m1_step_workspace_image_v1(
                exact_plan(target_spec, 5),
                validated_inputs(target_spec, 1, false),
                page_table(target_spec, 1),
            )
        else {
            panic!("external future token must reject")
        };
        assert_eq!(
            failure.error(),
            M1StepWorkspaceImageCompositionErrorV1::SpeculativeFutureTokenNonZero {
                lane: 0,
                column: 1,
                token: 101,
            }
        );

        let mut pages = page_table(target_decode, 2);
        pages[17] = 16_384;
        let M1StepWorkspaceImageCompositionOutcomeV1::Rejected(failure) =
            compose_m1_step_workspace_image_v1(
                exact_plan(target_decode, 6),
                validated_inputs(target_decode, 2, false),
                pages,
            )
        else {
            panic!("out-of-range page must reject")
        };
        assert_eq!(
            failure.error(),
            M1StepWorkspaceImageCompositionErrorV1::KvPageIndexOutOfRange {
                lane: 0,
                entry: 17,
                page: 16_384,
            }
        );

        let mut pages = page_table(target_decode, 2);
        pages[2 * 512 + 9] = 7;
        let M1StepWorkspaceImageCompositionOutcomeV1::Rejected(failure) =
            compose_m1_step_workspace_image_v1(
                exact_plan(target_decode, 7),
                validated_inputs(target_decode, 2, false),
                pages,
            )
        else {
            panic!("inactive page padding must reject")
        };
        assert_eq!(
            failure.error(),
            M1StepWorkspaceImageCompositionErrorV1::InactiveKvPagePaddingNonZero {
                lane: 2,
                entry: 9,
                page: 7,
            }
        );
    }

    #[test]
    fn hostile_plan_metadata_never_reaches_composer_typestate() {
        let selection = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let dimensions = selection
            .bucket
            .dimensions(selection.role, selection.mode)
            .unwrap();
        let candidate = M1StepInputCandidate::new(
            selection,
            vec![Some(StepPlan::new(
                RequestId::new(7, 19),
                CompletionEpoch::new(23),
                Identity::new([0; 32]),
                selection,
            ))],
            vec![100],
            vec![31],
            vec![dimensions.active_tokens],
            vec![31],
        );
        assert!(matches!(
            validate_m1_step_inputs(candidate),
            M1StepInputValidationOutcome::Rejected(_)
        ));
    }
}
