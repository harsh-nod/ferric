//! Typed KV-reservation bridge for one structurally validated M1 step.
//!
//! This Ferric-owned layer joins the exact live prefix of structural runner
//! inputs to one pending device-KV step reservation per live lane. Success
//! owns the canonical padded physical page-index table alongside every pending
//! reservation. It does not initialize memory, bind an allocation, construct
//! a packet, publish a queue, launch a kernel, or claim completion.

use core::fmt;

use ferric_spec::{Identity, Qwen3PlanSelection, ValidatedM1StepInputs, M1_KV_PAGE_TOKENS};

use crate::{
    PendingDeviceKvStepWrite, M1_KV_PAGE_TABLE_ENTRIES_PER_SEQUENCE_V1,
    M1_KV_PHYSICAL_PAGE_SLOTS_V1,
};

const UNSEEN_PHYSICAL_PAGE: u32 = u32::MAX;

/// Linear custody of every live lane's pending device-KV reservation.
///
/// This value is deliberately separate from the owned page-index table so a
/// caller can pass the table and validated inputs into workspace image
/// composition while continuing to retain the reservations. It proves no
/// initialization, launch, completion, commit, or rollback event.
///
/// ```compile_fail
/// use ferric_engine::M1KvWorkspaceReservationCustodyV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1KvWorkspaceReservationCustodyV1>();
/// ```
#[must_use = "pending KV reservations must remain retained until settled or aborted"]
#[derive(Debug, Eq, PartialEq)]
pub struct M1KvWorkspaceReservationCustodyV1 {
    selection: Qwen3PlanSelection,
    allocation_id: Identity,
    reservations: Vec<PendingDeviceKvStepWrite>,
}

impl M1KvWorkspaceReservationCustodyV1 {
    /// Returns the exact role, mode, and bucket shared by every reservation.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Returns the one role-scoped arena allocation identity used by all rows.
    #[must_use]
    pub const fn allocation_id(&self) -> Identity {
        self.allocation_id
    }

    /// Borrows all reservations in exact live-lane order.
    #[must_use]
    pub fn reservations(&self) -> &[PendingDeviceKvStepWrite] {
        &self.reservations
    }

    /// Recovers every reservation in exact live-lane order.
    #[must_use]
    pub fn into_reservations(self) -> Vec<PendingDeviceKvStepWrite> {
        self.reservations
    }

    /// This structural custody grants no initialization or runtime authority.
    #[must_use]
    pub const fn grants_runtime_authority(&self) -> bool {
        false
    }
}

/// Exact structural inputs and padded page table joined to live KV custody.
///
/// The page-index table is sequence-major `[S,512]`. Every inactive lane and
/// every unused live-lane tail entry is canonical zero. This type is
/// intentionally not `Clone`.
///
/// ```compile_fail
/// use ferric_engine::BoundM1KvWorkspaceTableV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<BoundM1KvWorkspaceTableV1>();
/// ```
#[must_use = "the validated inputs, page table, and pending reservations remain linear"]
#[derive(Debug, Eq, PartialEq)]
pub struct BoundM1KvWorkspaceTableV1 {
    inputs: ValidatedM1StepInputs,
    kv_page_indices: Box<[u32]>,
    reservations: M1KvWorkspaceReservationCustodyV1,
}

impl BoundM1KvWorkspaceTableV1 {
    /// Returns the exact finite selection shared by every retained input.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.inputs.selection()
    }

    /// Borrows the exact structurally validated step inputs.
    #[must_use]
    pub const fn inputs(&self) -> &ValidatedM1StepInputs {
        &self.inputs
    }

    /// Borrows the canonical sequence-major `[S,512]` physical page table.
    #[must_use]
    pub fn kv_page_indices(&self) -> &[u32] {
        &self.kv_page_indices
    }

    /// Borrows the exact live-prefix reservation custody.
    pub const fn reservations(&self) -> &M1KvWorkspaceReservationCustodyV1 {
        &self.reservations
    }

    /// Separates the two exact workspace-image inputs from reservation custody.
    ///
    /// The first two values can be passed directly to the Ferric M1 workspace
    /// byte-image composer. The third must remain retained independently until
    /// an exact completion path settles or aborts each pending reservation.
    #[must_use = "workspace-image inputs and reservation custody must all remain retained"]
    pub fn into_workspace_image_parts(
        self,
    ) -> (
        ValidatedM1StepInputs,
        Box<[u32]>,
        M1KvWorkspaceReservationCustodyV1,
    ) {
        (self.inputs, self.kv_page_indices, self.reservations)
    }

    /// This structural join grants no initialization or runtime authority.
    #[must_use]
    pub const fn grants_runtime_authority(&self) -> bool {
        false
    }
}

/// Stable fail-closed reason for rejecting a KV workspace-table join.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1KvWorkspaceTableBindingErrorV1 {
    /// The reservation vector does not match the exact nonempty live prefix.
    ReservationCount {
        /// Required number of live-lane reservations.
        expected: usize,
        /// Supplied number of reservations.
        actual: usize,
    },
    /// One reservation names a different model role, mode, or bucket.
    ReservationSelection {
        /// Live lane containing the mismatch.
        lane: usize,
        /// Selection retained by the structural inputs.
        inputs: Qwen3PlanSelection,
        /// Selection retained by the pending reservation.
        reservation: Qwen3PlanSelection,
    },
    /// One reservation names a different generational request.
    ReservationRequest {
        /// Live lane containing the mismatch.
        lane: usize,
    },
    /// One reservation names a different exact completion epoch.
    ReservationCompletionEpoch {
        /// Live lane containing the mismatch.
        lane: usize,
    },
    /// One reservation covers a different active step width.
    ReservationActiveTokens {
        /// Live lane containing the mismatch.
        lane: usize,
        /// Width retained by the structural inputs.
        expected: u32,
        /// Width retained by the pending reservation.
        actual: u32,
    },
    /// One reservation starts from a different committed context length.
    ReservationCommittedTokens {
        /// Live lane containing the mismatch.
        lane: usize,
        /// Context retained by the structural inputs.
        expected: u32,
        /// Context retained by the pending reservation.
        actual: u32,
    },
    /// One reservation's retained interval end differs from context plus width.
    ReservationEndTokens {
        /// Live lane containing the mismatch.
        lane: usize,
        /// Required exclusive interval end.
        expected: u32,
        /// Retained exclusive interval end.
        actual: u32,
    },
    /// One reservation has the wrong exact logical page-prefix length.
    LogicalPageCount {
        /// Live lane containing the mismatch.
        lane: usize,
        /// Required logical prefix length.
        expected: usize,
        /// Supplied logical prefix length.
        actual: usize,
    },
    /// A page-table identity is not in exact zero-based logical-page order.
    LogicalPageOrder {
        /// Live lane containing the mismatch.
        lane: usize,
        /// Position within the reservation page table.
        entry: usize,
        /// Required logical page number.
        expected: u32,
        /// Supplied logical page number.
        actual: u32,
    },
    /// A page identity names the other model role's physical pool.
    PhysicalPageRole {
        /// Live lane containing the mismatch.
        lane: usize,
        /// Logical page containing the mismatch.
        logical_page: u32,
    },
    /// A page identity carries the invalid zero physical generation.
    ZeroPhysicalPageGeneration {
        /// Live lane containing the mismatch.
        lane: usize,
        /// Logical page containing the mismatch.
        logical_page: u32,
    },
    /// A page index is outside the fixed global M1 physical pool.
    PhysicalPageOutOfRange {
        /// Live lane containing the mismatch.
        lane: usize,
        /// Logical page containing the mismatch.
        logical_page: u32,
        /// Rejected physical page index.
        page: u32,
    },
    /// A page identity omits its role-scoped arena allocation identity.
    MissingAllocationIdentity {
        /// Live lane containing the mismatch.
        lane: usize,
        /// Logical page containing the mismatch.
        logical_page: u32,
    },
    /// A page identity differs from the one role-scoped arena identity.
    ArenaAllocationMismatch {
        /// Live lane containing the mismatch.
        lane: usize,
        /// Logical page containing the mismatch.
        logical_page: u32,
    },
    /// Two logical page identities alias one physical page in the same arena.
    PhysicalPageAlias {
        /// Earlier live lane retaining the page.
        first_lane: usize,
        /// Earlier logical page retaining the page.
        first_logical_page: u32,
        /// Later live lane repeating the page.
        lane: usize,
        /// Later logical page repeating the page.
        logical_page: u32,
        /// Repeated physical page index.
        page: u32,
    },
    /// The exact `[S,512]` table length overflowed host arithmetic.
    TableExtent,
    /// Host reservation for the fixed physical-alias validation map failed.
    HostValidationReservation {
        /// Exact requested validation-map entry count.
        entries: usize,
    },
    /// Host reservation for the exact output table failed.
    HostTableReservation {
        /// Exact requested entry count.
        entries: usize,
    },
}

impl fmt::Display for M1KvWorkspaceTableBindingErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 KV workspace table rejected: {self:?}")
    }
}

impl std::error::Error for M1KvWorkspaceTableBindingErrorV1 {}

/// Retry-safe rejection retaining the exact structural inputs and reservations.
///
/// This type is intentionally not `Clone`.
///
/// ```compile_fail
/// use ferric_engine::M1KvWorkspaceTableBindingFailureV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1KvWorkspaceTableBindingFailureV1>();
/// ```
#[must_use = "rejection retains every exact input for retry or explicit release"]
#[derive(Debug, Eq, PartialEq)]
pub struct M1KvWorkspaceTableBindingFailureV1 {
    error: M1KvWorkspaceTableBindingErrorV1,
    inputs: ValidatedM1StepInputs,
    reservations: Vec<PendingDeviceKvStepWrite>,
}

impl M1KvWorkspaceTableBindingFailureV1 {
    /// Returns the stable rejection reason.
    #[must_use]
    pub const fn error(&self) -> M1KvWorkspaceTableBindingErrorV1 {
        self.error
    }

    /// Borrows the exact unchanged structural inputs.
    #[must_use]
    pub const fn inputs(&self) -> &ValidatedM1StepInputs {
        &self.inputs
    }

    /// Borrows every exact unchanged reservation in supplied order.
    #[must_use]
    pub fn reservations(&self) -> &[PendingDeviceKvStepWrite] {
        &self.reservations
    }

    /// Recovers the diagnostic and every exact unchanged input.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        M1KvWorkspaceTableBindingErrorV1,
        ValidatedM1StepInputs,
        Vec<PendingDeviceKvStepWrite>,
    ) {
        (self.error, self.inputs, self.reservations)
    }
}

fn reject(
    error: M1KvWorkspaceTableBindingErrorV1,
    inputs: ValidatedM1StepInputs,
    reservations: Vec<PendingDeviceKvStepWrite>,
) -> Result<BoundM1KvWorkspaceTableV1, Box<M1KvWorkspaceTableBindingFailureV1>> {
    Err(Box::new(M1KvWorkspaceTableBindingFailureV1 {
        error,
        inputs,
        reservations,
    }))
}

/// Joins the exact live reservation roster to a canonical physical page table.
///
/// `reservations` must contain exactly one entry for each live lane, in lane
/// order. Each entry must match that lane's request, selection, completion
/// epoch, committed context, and active width. Its page identities must form
/// the exact logical prefix `[0, ceil((context + active) / 16))`, use one
/// present arena allocation identity for the selected role, name nonaliasing
/// physical pages below 16,384, and retain nonzero generations.
///
/// All validation completes before the output table is allocated. Every
/// rejection returns the exact inputs and every reservation unchanged. Success
/// creates only ordinary host data; it does not alter any cache or device state.
///
/// # Errors
///
/// Returns [`M1KvWorkspaceTableBindingFailureV1`] for any roster, metadata,
/// logical-page, physical-page, arena, alias, extent, or host-reservation drift.
pub fn bind_m1_kv_workspace_table_v1(
    inputs: ValidatedM1StepInputs,
    reservations: Vec<PendingDeviceKvStepWrite>,
) -> Result<BoundM1KvWorkspaceTableV1, Box<M1KvWorkspaceTableBindingFailureV1>> {
    let live_lanes = usize::try_from(inputs.live_lane_count()).unwrap_or(usize::MAX);
    if reservations.len() != live_lanes {
        return reject(
            M1KvWorkspaceTableBindingErrorV1::ReservationCount {
                expected: live_lanes,
                actual: reservations.len(),
            },
            inputs,
            reservations,
        );
    }

    let selection = inputs.selection();
    let sequence_count = match usize::try_from(inputs.dimensions().sequences) {
        Ok(count) => count,
        Err(_) => {
            return reject(
                M1KvWorkspaceTableBindingErrorV1::TableExtent,
                inputs,
                reservations,
            );
        }
    };
    let entries_per_sequence =
        usize::try_from(M1_KV_PAGE_TABLE_ENTRIES_PER_SEQUENCE_V1).unwrap_or(usize::MAX);
    let Some(table_entries) = sequence_count.checked_mul(entries_per_sequence) else {
        return reject(
            M1KvWorkspaceTableBindingErrorV1::TableExtent,
            inputs,
            reservations,
        );
    };

    let physical_page_slots = usize::try_from(M1_KV_PHYSICAL_PAGE_SLOTS_V1).unwrap_or(usize::MAX);
    let mut seen_pages = Vec::new();
    if seen_pages.try_reserve_exact(physical_page_slots).is_err() {
        return reject(
            M1KvWorkspaceTableBindingErrorV1::HostValidationReservation {
                entries: physical_page_slots,
            },
            inputs,
            reservations,
        );
    }
    seen_pages.resize(physical_page_slots, UNSEEN_PHYSICAL_PAGE);
    let mut allocation_id = None;
    for (lane, reservation) in reservations.iter().enumerate() {
        if reservation.selection() != selection {
            return reject(
                M1KvWorkspaceTableBindingErrorV1::ReservationSelection {
                    lane,
                    inputs: selection,
                    reservation: reservation.selection(),
                },
                inputs,
                reservations,
            );
        }
        let Some(plan) = inputs.lanes().get(lane).and_then(Option::as_ref) else {
            return reject(
                M1KvWorkspaceTableBindingErrorV1::TableExtent,
                inputs,
                reservations,
            );
        };
        if reservation.request() != plan.request() {
            return reject(
                M1KvWorkspaceTableBindingErrorV1::ReservationRequest { lane },
                inputs,
                reservations,
            );
        }
        if reservation.epoch() != plan.completion_epoch() {
            return reject(
                M1KvWorkspaceTableBindingErrorV1::ReservationCompletionEpoch { lane },
                inputs,
                reservations,
            );
        }

        let Some(&active) = inputs.active_lengths().get(lane) else {
            return reject(
                M1KvWorkspaceTableBindingErrorV1::TableExtent,
                inputs,
                reservations,
            );
        };
        if reservation.active_tokens() != active {
            return reject(
                M1KvWorkspaceTableBindingErrorV1::ReservationActiveTokens {
                    lane,
                    expected: active,
                    actual: reservation.active_tokens(),
                },
                inputs,
                reservations,
            );
        }
        let Some(&committed) = inputs.context_lengths().get(lane) else {
            return reject(
                M1KvWorkspaceTableBindingErrorV1::TableExtent,
                inputs,
                reservations,
            );
        };
        if reservation.committed_tokens() != committed {
            return reject(
                M1KvWorkspaceTableBindingErrorV1::ReservationCommittedTokens {
                    lane,
                    expected: committed,
                    actual: reservation.committed_tokens(),
                },
                inputs,
                reservations,
            );
        }
        let Some(end_tokens) = committed.checked_add(active) else {
            return reject(
                M1KvWorkspaceTableBindingErrorV1::TableExtent,
                inputs,
                reservations,
            );
        };
        if reservation.end_tokens() != end_tokens {
            return reject(
                M1KvWorkspaceTableBindingErrorV1::ReservationEndTokens {
                    lane,
                    expected: end_tokens,
                    actual: reservation.end_tokens(),
                },
                inputs,
                reservations,
            );
        }
        let expected_pages =
            usize::try_from(end_tokens.div_ceil(M1_KV_PAGE_TOKENS)).unwrap_or(usize::MAX);
        if reservation.page_table().len() != expected_pages {
            return reject(
                M1KvWorkspaceTableBindingErrorV1::LogicalPageCount {
                    lane,
                    expected: expected_pages,
                    actual: reservation.page_table().len(),
                },
                inputs,
                reservations,
            );
        }

        for (entry, page_identity) in reservation.page_table().iter().enumerate() {
            let expected_logical_page = u32::try_from(entry).unwrap_or(u32::MAX);
            if page_identity.logical_page() != expected_logical_page {
                return reject(
                    M1KvWorkspaceTableBindingErrorV1::LogicalPageOrder {
                        lane,
                        entry,
                        expected: expected_logical_page,
                        actual: page_identity.logical_page(),
                    },
                    inputs,
                    reservations,
                );
            }
            let page = page_identity.page();
            if page.role() != selection.role {
                return reject(
                    M1KvWorkspaceTableBindingErrorV1::PhysicalPageRole {
                        lane,
                        logical_page: expected_logical_page,
                    },
                    inputs,
                    reservations,
                );
            }
            if page.generation() == 0 {
                return reject(
                    M1KvWorkspaceTableBindingErrorV1::ZeroPhysicalPageGeneration {
                        lane,
                        logical_page: expected_logical_page,
                    },
                    inputs,
                    reservations,
                );
            }
            if page.index() >= M1_KV_PHYSICAL_PAGE_SLOTS_V1 {
                return reject(
                    M1KvWorkspaceTableBindingErrorV1::PhysicalPageOutOfRange {
                        lane,
                        logical_page: expected_logical_page,
                        page: page.index(),
                    },
                    inputs,
                    reservations,
                );
            }
            let candidate_allocation = page_identity.allocation_id();
            if !candidate_allocation.is_present() {
                return reject(
                    M1KvWorkspaceTableBindingErrorV1::MissingAllocationIdentity {
                        lane,
                        logical_page: expected_logical_page,
                    },
                    inputs,
                    reservations,
                );
            }
            if allocation_id
                .is_some_and(|expected: Identity| !expected.equals(&candidate_allocation))
            {
                return reject(
                    M1KvWorkspaceTableBindingErrorV1::ArenaAllocationMismatch {
                        lane,
                        logical_page: expected_logical_page,
                    },
                    inputs,
                    reservations,
                );
            }
            allocation_id.get_or_insert(candidate_allocation);

            let physical_index = usize::try_from(page.index()).unwrap_or(usize::MAX);
            let Some(seen_page) = seen_pages.get_mut(physical_index) else {
                return reject(
                    M1KvWorkspaceTableBindingErrorV1::PhysicalPageOutOfRange {
                        lane,
                        logical_page: expected_logical_page,
                        page: page.index(),
                    },
                    inputs,
                    reservations,
                );
            };
            let prior = *seen_page;
            if prior != UNSEEN_PHYSICAL_PAGE {
                return reject(
                    M1KvWorkspaceTableBindingErrorV1::PhysicalPageAlias {
                        first_lane: usize::try_from(prior >> 16).unwrap_or(usize::MAX),
                        first_logical_page: prior & 0xffff,
                        lane,
                        logical_page: expected_logical_page,
                        page: page.index(),
                    },
                    inputs,
                    reservations,
                );
            }
            let Ok(packed_lane) = u32::try_from(lane) else {
                return reject(
                    M1KvWorkspaceTableBindingErrorV1::TableExtent,
                    inputs,
                    reservations,
                );
            };
            *seen_page = (packed_lane << 16) | expected_logical_page;
        }
    }

    let Some(allocation_id) = allocation_id else {
        return reject(
            M1KvWorkspaceTableBindingErrorV1::TableExtent,
            inputs,
            reservations,
        );
    };
    let mut page_table = Vec::new();
    if page_table.try_reserve_exact(table_entries).is_err() {
        return reject(
            M1KvWorkspaceTableBindingErrorV1::HostTableReservation {
                entries: table_entries,
            },
            inputs,
            reservations,
        );
    }
    page_table.resize(table_entries, 0);
    for (lane, reservation) in reservations.iter().enumerate() {
        let row_start = lane * entries_per_sequence;
        for page_identity in reservation.page_table() {
            let logical_page = usize::try_from(page_identity.logical_page()).unwrap_or(usize::MAX);
            let Some(destination) = row_start
                .checked_add(logical_page)
                .and_then(|index| page_table.get_mut(index))
            else {
                return reject(
                    M1KvWorkspaceTableBindingErrorV1::TableExtent,
                    inputs,
                    reservations,
                );
            };
            *destination = page_identity.page().index();
        }
    }

    Ok(BoundM1KvWorkspaceTableV1 {
        inputs,
        kv_page_indices: page_table.into_boxed_slice(),
        reservations: M1KvWorkspaceReservationCustodyV1 {
            selection,
            allocation_id,
            reservations,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bind_gfx942_device, ActiveDeviceKvCache, DeviceKvPageLease, Gfx942DeviceBinding,
        GFX942_PROCESSOR, GFX942_TARGET_FEATURES,
    };
    use ferric_spec::completion::CompletionEpoch;
    use ferric_spec::{
        validate_m1_step_inputs, M1StepInputCandidate, M1StepInputValidationOutcome,
        PhysicalPageId, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, RequestId, StepPlan,
    };

    fn identity(tag: u8) -> Identity {
        Identity::new([tag; 32])
    }

    fn device() -> Gfx942DeviceBinding {
        bind_gfx942_device(identity(1), 7, GFX942_PROCESSOR, GFX942_TARGET_FEATURES).unwrap()
    }

    const fn selection(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    fn role_pair(selected: Qwen3PlanSelection) -> (Qwen3PlanSelection, Qwen3PlanSelection) {
        (
            selection(Qwen3ModelRole::Target8B, selected.mode, selected.bucket),
            selection(Qwen3ModelRole::Draft06B, selected.mode, selected.bucket),
        )
    }

    fn validated_inputs(
        selected: Qwen3PlanSelection,
        requests: &[RequestId],
        epoch: CompletionEpoch,
        active: &[u32],
        contexts: &[u32],
    ) -> ValidatedM1StepInputs {
        assert_eq!(requests.len(), active.len());
        assert_eq!(requests.len(), contexts.len());
        let dimensions = selected
            .bucket
            .dimensions(selected.role, selected.mode)
            .unwrap();
        let sequences = usize::try_from(dimensions.sequences).unwrap();
        let width = usize::try_from(dimensions.active_tokens).unwrap();
        let mut lanes = vec![None; sequences];
        let mut token_ids = vec![0; sequences * width];
        let mut position_ids = vec![0; sequences * width];
        let mut active_lengths = vec![0; sequences];
        let mut context_lengths = vec![0; sequences];
        for lane in 0..requests.len() {
            lanes[lane] = Some(StepPlan::new(requests[lane], epoch, identity(41), selected));
            active_lengths[lane] = active[lane];
            context_lengths[lane] = contexts[lane];
            for column in 0..usize::try_from(active[lane]).unwrap() {
                let flat = lane * width + column;
                token_ids[flat] = u32::try_from(flat + 1).unwrap();
                position_ids[flat] = contexts[lane] + u32::try_from(column).unwrap();
            }
        }
        let candidate = M1StepInputCandidate::new(
            selected,
            lanes,
            token_ids,
            position_ids,
            active_lengths,
            context_lengths,
        );
        match validate_m1_step_inputs(candidate) {
            M1StepInputValidationOutcome::Validated(inputs) => inputs,
            M1StepInputValidationOutcome::Rejected(failure) => {
                panic!("test input must validate: {:?}", failure.error())
            }
        }
    }

    fn pending_reservation(
        selected: Qwen3PlanSelection,
        request: RequestId,
        committed: u32,
        active: u32,
        epoch: CompletionEpoch,
        physical_start: u32,
        allocation_tag: u8,
    ) -> (ActiveDeviceKvCache, PendingDeviceKvStepWrite) {
        assert_eq!(committed, 0, "fixtures currently reserve fresh caches");
        let (target, draft) = role_pair(selected);
        let mut cache = ActiveDeviceKvCache::new(device(), request, target, draft).unwrap();
        let page_count = usize::try_from(active.div_ceil(M1_KV_PAGE_TOKENS)).unwrap();
        let leases = (0..page_count)
            .map(|offset| {
                let index = physical_start + u32::try_from(offset).unwrap();
                DeviceKvPageLease::from_contracted_workspace_bridge_test_allocation(
                    device(),
                    identity(allocation_tag),
                    PhysicalPageId::new(selected.role, index, 1),
                )
            })
            .collect();
        let pending = cache
            .reserve_step_write(request, selected.role, committed, active, epoch, leases)
            .unwrap();
        (cache, pending)
    }

    fn bind_one(
        selected: Qwen3PlanSelection,
        active: u32,
    ) -> (ActiveDeviceKvCache, BoundM1KvWorkspaceTableV1) {
        let request = RequestId::new(0, 3);
        let epoch = CompletionEpoch::new(9);
        let inputs = validated_inputs(selected, &[request], epoch, &[active], &[0]);
        let (cache, pending) = pending_reservation(selected, request, 0, active, epoch, 7, 71);
        let bound = bind_m1_kv_workspace_table_v1(inputs, vec![pending]).unwrap();
        (cache, bound)
    }

    #[test]
    fn every_admitted_role_and_shape_builds_an_exact_padded_table() {
        let cases = [
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128),
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS8T128),
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T512),
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T2048),
            (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192),
            (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192),
            (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS32C8192),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS8K4C8192,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K8C8192,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K16C8192,
            ),
        ];
        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            for (mode, bucket) in cases {
                let selected = selection(role, mode, bucket);
                let dimensions = bucket.dimensions(role, mode).unwrap();
                let active = match mode {
                    Qwen3ExecutionMode::Prefill => 1,
                    Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => {
                        dimensions.active_tokens
                    }
                };
                let (_cache, bound) = bind_one(selected, active);
                assert_eq!(bound.selection(), selected);
                assert_eq!(bound.inputs().live_lane_count(), 1);
                assert_eq!(bound.reservations().reservations().len(), 1);
                assert_eq!(bound.reservations().allocation_id(), identity(71));
                assert!(!bound.grants_runtime_authority());
                assert!(!bound.reservations().grants_runtime_authority());
                assert_eq!(
                    bound.kv_page_indices().len(),
                    usize::try_from(dimensions.sequences).unwrap()
                        * usize::try_from(M1_KV_PAGE_TABLE_ENTRIES_PER_SEQUENCE_V1).unwrap()
                );
                let page_count = usize::try_from(active.div_ceil(M1_KV_PAGE_TOKENS)).unwrap();
                for page in 0..page_count {
                    assert_eq!(
                        bound.kv_page_indices()[page],
                        7 + u32::try_from(page).unwrap()
                    );
                }
                assert!(bound.kv_page_indices()[page_count..]
                    .iter()
                    .all(|page| *page == 0));
            }
        }
    }

    #[test]
    fn exact_live_prefix_rows_are_filled_and_inactive_rows_are_zero() {
        let selected = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let requests = [
            RequestId::new(0, 3),
            RequestId::new(1, 3),
            RequestId::new(2, 3),
        ];
        let epoch = CompletionEpoch::new(13);
        let inputs = validated_inputs(selected, &requests, epoch, &[1; 3], &[0; 3]);
        let mut caches = Vec::new();
        let mut reservations = Vec::new();
        for (lane, request) in requests.into_iter().enumerate() {
            let (cache, pending) = pending_reservation(
                selected,
                request,
                0,
                1,
                epoch,
                30 + u32::try_from(lane).unwrap(),
                72,
            );
            caches.push(cache);
            reservations.push(pending);
        }
        let bound = bind_m1_kv_workspace_table_v1(inputs, reservations).unwrap();
        let width = usize::try_from(M1_KV_PAGE_TABLE_ENTRIES_PER_SEQUENCE_V1).unwrap();
        for lane in 0..3 {
            assert_eq!(
                bound.kv_page_indices()[lane * width],
                30 + u32::try_from(lane).unwrap()
            );
            assert!(
                bound.kv_page_indices()[lane * width + 1..(lane + 1) * width]
                    .iter()
                    .all(|page| *page == 0)
            );
        }
        assert!(bound.kv_page_indices()[3 * width..]
            .iter()
            .all(|page| *page == 0));

        let (inputs, pages, custody) = bound.into_workspace_image_parts();
        assert_eq!(inputs.live_lane_count(), 3);
        assert_eq!(pages.len(), 8 * width);
        assert_eq!(custody.reservations().len(), 3);
        assert_eq!(custody.into_reservations().len(), 3);
        assert_eq!(caches.len(), 3);
    }

    #[test]
    fn metadata_rejections_recover_every_exact_input() {
        let target_decode = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let request = RequestId::new(0, 3);
        let epoch = CompletionEpoch::new(15);

        let inputs = validated_inputs(target_decode, &[request], epoch, &[1], &[0]);
        let failure = bind_m1_kv_workspace_table_v1(inputs, Vec::new()).unwrap_err();
        assert_eq!(
            failure.error(),
            M1KvWorkspaceTableBindingErrorV1::ReservationCount {
                expected: 1,
                actual: 0
            }
        );
        let (_, recovered_inputs, recovered) = failure.into_parts();
        assert_eq!(recovered_inputs.selection(), target_decode);
        assert!(recovered.is_empty());

        let inputs = validated_inputs(target_decode, &[request], epoch, &[1], &[0]);
        let wrong_request = RequestId::new(1, 3);
        let (_cache, pending) =
            pending_reservation(target_decode, wrong_request, 0, 1, epoch, 1, 73);
        let failure = bind_m1_kv_workspace_table_v1(inputs, vec![pending]).unwrap_err();
        assert_eq!(
            failure.error(),
            M1KvWorkspaceTableBindingErrorV1::ReservationRequest { lane: 0 }
        );
        assert_eq!(failure.reservations().len(), 1);

        let inputs = validated_inputs(target_decode, &[request], epoch, &[1], &[0]);
        let draft_decode = selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let (_cache, pending) = pending_reservation(draft_decode, request, 0, 1, epoch, 2, 73);
        let failure = bind_m1_kv_workspace_table_v1(inputs, vec![pending]).unwrap_err();
        assert!(matches!(
            failure.error(),
            M1KvWorkspaceTableBindingErrorV1::ReservationSelection { lane: 0, .. }
        ));

        let inputs = validated_inputs(target_decode, &[request], epoch, &[1], &[0]);
        let (_cache, pending) = pending_reservation(
            target_decode,
            request,
            0,
            1,
            CompletionEpoch::new(16),
            3,
            73,
        );
        let failure = bind_m1_kv_workspace_table_v1(inputs, vec![pending]).unwrap_err();
        assert_eq!(
            failure.error(),
            M1KvWorkspaceTableBindingErrorV1::ReservationCompletionEpoch { lane: 0 }
        );

        let target_prefill = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let inputs = validated_inputs(target_prefill, &[request], epoch, &[64], &[0]);
        let (_cache, pending) = pending_reservation(target_prefill, request, 0, 128, epoch, 8, 73);
        let failure = bind_m1_kv_workspace_table_v1(inputs, vec![pending]).unwrap_err();
        assert_eq!(
            failure.error(),
            M1KvWorkspaceTableBindingErrorV1::ReservationActiveTokens {
                lane: 0,
                expected: 64,
                actual: 128
            }
        );

        let inputs = validated_inputs(target_decode, &[request], epoch, &[1], &[1]);
        let (_cache, pending) = pending_reservation(target_decode, request, 0, 1, epoch, 4, 73);
        let failure = bind_m1_kv_workspace_table_v1(inputs, vec![pending]).unwrap_err();
        assert_eq!(
            failure.error(),
            M1KvWorkspaceTableBindingErrorV1::ReservationCommittedTokens {
                lane: 0,
                expected: 1,
                actual: 0
            }
        );
    }

    #[test]
    fn logical_order_and_physical_bounds_fail_closed() {
        let selected = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let request = RequestId::new(0, 3);
        let epoch = CompletionEpoch::new(17);

        let inputs = validated_inputs(selected, &[request], epoch, &[32], &[0]);
        let (_cache, mut pending) = pending_reservation(selected, request, 0, 32, epoch, 10, 74);
        pending.corrupt_workspace_bridge_page_for_test(
            1,
            7,
            identity(74),
            PhysicalPageId::new(selected.role, 11, 1),
        );
        let failure = bind_m1_kv_workspace_table_v1(inputs, vec![pending]).unwrap_err();
        assert_eq!(
            failure.error(),
            M1KvWorkspaceTableBindingErrorV1::LogicalPageOrder {
                lane: 0,
                entry: 1,
                expected: 1,
                actual: 7
            }
        );

        let inputs = validated_inputs(selected, &[request], epoch, &[32], &[0]);
        let (_cache, mut pending) = pending_reservation(selected, request, 0, 32, epoch, 10, 74);
        pending.corrupt_workspace_bridge_page_for_test(
            1,
            1,
            identity(74),
            PhysicalPageId::new(selected.role, M1_KV_PHYSICAL_PAGE_SLOTS_V1, 1),
        );
        let failure = bind_m1_kv_workspace_table_v1(inputs, vec![pending]).unwrap_err();
        assert_eq!(
            failure.error(),
            M1KvWorkspaceTableBindingErrorV1::PhysicalPageOutOfRange {
                lane: 0,
                logical_page: 1,
                page: M1_KV_PHYSICAL_PAGE_SLOTS_V1
            }
        );

        let inputs = validated_inputs(selected, &[request], epoch, &[32], &[0]);
        let (_cache, mut pending) = pending_reservation(selected, request, 0, 32, epoch, 10, 74);
        pending.corrupt_workspace_bridge_page_for_test(
            1,
            1,
            identity(74),
            PhysicalPageId::new(Qwen3ModelRole::Draft06B, 11, 1),
        );
        let failure = bind_m1_kv_workspace_table_v1(inputs, vec![pending]).unwrap_err();
        assert_eq!(
            failure.error(),
            M1KvWorkspaceTableBindingErrorV1::PhysicalPageRole {
                lane: 0,
                logical_page: 1
            }
        );

        let inputs = validated_inputs(selected, &[request], epoch, &[32], &[0]);
        let (_cache, mut pending) = pending_reservation(selected, request, 0, 32, epoch, 10, 74);
        pending.corrupt_workspace_bridge_page_for_test(
            1,
            1,
            identity(74),
            PhysicalPageId::new(selected.role, 11, 0),
        );
        let failure = bind_m1_kv_workspace_table_v1(inputs, vec![pending]).unwrap_err();
        assert_eq!(
            failure.error(),
            M1KvWorkspaceTableBindingErrorV1::ZeroPhysicalPageGeneration {
                lane: 0,
                logical_page: 1
            }
        );
    }

    #[test]
    fn allocation_drift_and_cross_lane_aliases_fail_closed() {
        let selected = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let requests = [RequestId::new(0, 3), RequestId::new(1, 3)];
        let epoch = CompletionEpoch::new(19);

        let inputs = validated_inputs(selected, &requests, epoch, &[1, 1], &[0, 0]);
        let (_first_cache, first) = pending_reservation(selected, requests[0], 0, 1, epoch, 20, 75);
        let (_second_cache, second) =
            pending_reservation(selected, requests[1], 0, 1, epoch, 21, 76);
        let failure = bind_m1_kv_workspace_table_v1(inputs, vec![first, second]).unwrap_err();
        assert_eq!(
            failure.error(),
            M1KvWorkspaceTableBindingErrorV1::ArenaAllocationMismatch {
                lane: 1,
                logical_page: 0
            }
        );
        assert_eq!(failure.reservations().len(), 2);

        let inputs = validated_inputs(selected, &requests, epoch, &[1, 1], &[0, 0]);
        let (_first_cache, first) = pending_reservation(selected, requests[0], 0, 1, epoch, 20, 75);
        let (_second_cache, second) =
            pending_reservation(selected, requests[1], 0, 1, epoch, 20, 75);
        let failure = bind_m1_kv_workspace_table_v1(inputs, vec![first, second]).unwrap_err();
        assert_eq!(
            failure.error(),
            M1KvWorkspaceTableBindingErrorV1::PhysicalPageAlias {
                first_lane: 0,
                first_logical_page: 0,
                lane: 1,
                logical_page: 0,
                page: 20
            }
        );
        assert_eq!(failure.reservations().len(), 2);
    }

    #[test]
    fn absent_arena_identity_is_rejected_without_losing_reservation() {
        let selected = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let request = RequestId::new(0, 3);
        let epoch = CompletionEpoch::new(21);
        let inputs = validated_inputs(selected, &[request], epoch, &[1], &[0]);
        let (_cache, pending) = pending_reservation(selected, request, 0, 1, epoch, 1, 0);
        let failure = bind_m1_kv_workspace_table_v1(inputs, vec![pending]).unwrap_err();
        assert_eq!(
            failure.error(),
            M1KvWorkspaceTableBindingErrorV1::MissingAllocationIdentity {
                lane: 0,
                logical_page: 0
            }
        );
        assert_eq!(failure.reservations().len(), 1);
    }
}
