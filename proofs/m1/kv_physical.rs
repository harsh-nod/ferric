//! M1 initialized physical-KV refinement theorems.
//!
//! These theorems join the verified source-level initialized target/draft KV
//! prestate, accepted-prefix speculative settlement, rejected-suffix
//! retirement, exact scheduler quiescence, and generation-safe retired-page
//! return. Target and draft roles remain distinct, the exact generational
//! request remains bound throughout, and another request is exactly framed at
//! the source-state level.
//!
//! This is deliberately not a device-memory or device-ledger refinement
//! theorem. The ordinary-Rust engine release path separately checks the gfx942
//! device, allocation, arena, lease, page, request, and queue identities and
//! implements device-ledger `Leased(g) -> Free(g + 1)`. Those checks and that
//! transition are not yet exposed as Verus postconditions, so the theorem below
//! proves only the source physical-metadata `Retired(g) -> Free(g + 1)` relation.
//! Workspace custody grants no initialization or runtime authority. Kernel
//! execution/readback semantics, scheduler-to-multi-member-batch refinement,
//! hardware execution, numerical accuracy, and performance qualification
//! remain separate M1 obligations.

#[allow(unused_imports)]
use ferric_spec::{
    ContinuousBatch, IsolatedRequestKv, IsolatedSpeculativeKvExpectation,
    IsolatedSpeculativeKvSettlement, PhysicalPageId, Qwen3ModelRole, RequestId,
    RequestIsolationError, SpeculativeKvRoundIndex,
};
#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

/// Strongest current verified initialized speculative-KV success boundary.
pub open spec fn m1_initialized_speculative_kv_success(
    before: &IsolatedRequestKv,
    after: &IsolatedRequestKv,
    index: &SpeculativeKvRoundIndex,
    outcome: IsolatedSpeculativeKvSettlement,
) -> bool {
    &&& ferric_spec::request_isolation::isolated_initialized_speculative_prestate(
        before,
        index,
        outcome,
    )
    &&& ferric_spec::request_isolation::isolated_speculative_settlement_transition(
        before,
        after,
        index,
        outcome,
    )
    &&& ferric_spec::request_isolation::request_identity_matches(
        after.projection_spec().request,
        before.projection_spec().request,
    )
    &&& ferric_spec::request_isolation::request_identity_matches(
        after.projection_spec().request,
        index.request,
    )
    &&& after.projection_spec().target.role == Qwen3ModelRole::Target8B
    &&& after.projection_spec().draft.role == Qwen3ModelRole::Draft06B
    &&& after.projection_spec().target.resident_tokens == outcome.target_commit_end
    &&& after.projection_spec().target.committed_tokens == outcome.target_commit_end
    &&& after.projection_spec().draft.resident_tokens == outcome.draft_commit_end
    &&& after.projection_spec().draft.committed_tokens == outcome.draft_commit_end
    &&& outcome.accepted_draft_tokens <= index.draft_token_count
    &&& outcome.target_commit_end as int
        == index.target_pre_committed as int + outcome.accepted_draft_tokens as int + 1
    &&& outcome.draft_commit_end as int
        == index.draft_pre_committed as int
            + if outcome.accepted_draft_tokens < index.draft_token_count {
                outcome.accepted_draft_tokens as int + 1
            } else {
                index.draft_token_count as int
            }
    &&& after.projection_spec().target_retired_pages as int
        == before.projection_spec().target_retired_pages as int
            + outcome.target_retired_pages as int
    &&& after.projection_spec().draft_retired_pages as int
        == before.projection_spec().draft_retired_pages as int
            + outcome.draft_retired_pages as int
    &&& index.target_tentative.end as int - outcome.target_commit_end as int
        == index.draft_token_count as int - outcome.accepted_draft_tokens as int
    &&& index.draft_tentative.end as int - outcome.draft_commit_end as int
        == if outcome.accepted_draft_tokens < index.draft_token_count {
            index.draft_token_count as int - outcome.accepted_draft_tokens as int - 1
        } else {
            0
        }
}

/// Exact failure frame for speculative KV settlement.
pub open spec fn m1_initialized_speculative_kv_failure(
    before_selected: &IsolatedRequestKv,
    after_selected: &IsolatedRequestKv,
    before_other: &IsolatedRequestKv,
    after_other: &IsolatedRequestKv,
) -> bool {
    &&& *after_selected == *before_selected
    &&& *after_other == *before_other
}

/// Exposes the stable initialized-physical settlement relation as an
/// executable proof root for source-inventory binding.
pub fn m1_initialized_speculative_kv_relation_theorem(
    _before: &IsolatedRequestKv,
    _after: &IsolatedRequestKv,
    _index: &SpeculativeKvRoundIndex,
    _outcome: IsolatedSpeculativeKvSettlement,
)
    requires
        _index.valid(),
        ferric_spec::request_isolation::request_identity_matches(
            _before.request_spec(),
            _index.request,
        ),
        ferric_spec::request_isolation::isolated_initialized_speculative_prestate(
            _before,
            _index,
            _outcome,
        ),
        ferric_spec::request_isolation::isolated_speculative_settlement_transition(
            _before,
            _after,
            _index,
            _outcome,
        ),
    ensures m1_initialized_speculative_kv_success(_before, _after, _index, _outcome),
{
    proof {
        ferric_spec::request_isolation::isolated_initialized_speculative_properties(
            _before,
            _after,
            _index,
            _outcome,
        );
        reveal(m1_initialized_speculative_kv_success);
    }
}

/// Settles one initialized target/draft speculative round and proves its exact
/// accepted prefix and rejected suffix at the physical-metadata boundary.
///
/// # Errors
///
/// Returns the exact source-level routing, indexing, or physical rejection.
/// Both the selected and other physical owners remain unchanged on failure.
pub fn m1_initialized_speculative_kv_theorem(
    batch: &ContinuousBatch,
    selected: &mut IsolatedRequestKv,
    other: &mut IsolatedRequestKv,
    index: &SpeculativeKvRoundIndex,
    accepted_draft_tokens: u8,
    expected: &IsolatedSpeculativeKvExpectation,
) -> (result: Result<IsolatedSpeculativeKvSettlement, RequestIsolationError>)
    requires batch.valid(),
    ensures match result {
        Ok(outcome) => {
            &&& *final(other) == *old(other)
            &&& m1_initialized_speculative_kv_success(
                old(selected),
                final(selected),
                index,
                outcome,
            )
        },
        Err(_) => m1_initialized_speculative_kv_failure(
            old(selected),
            final(selected),
            old(other),
            final(other),
        ),
    },
{
    let ghost entry_selected = *selected;
    let ghost entry_other = *other;
    assert(entry_selected == *old(selected));
    assert(entry_other == *old(other));
    let result = ferric_spec::settle_isolated_speculative_kv(
        batch,
        selected,
        other,
        index,
        accepted_draft_tokens,
        expected,
    );
    match result {
        Ok(outcome) => {
            proof {
                index.valid_for_implies_valid(
                    expected.request_spec(),
                    expected.completion_epoch_spec(),
                    expected.plan_id_spec(),
                    expected.target_selection_spec(),
                    expected.draft_selection_spec(),
                );
                ferric_spec::request_isolation::isolated_initialized_speculative_properties(
                    &entry_selected,
                    selected,
                    index,
                    outcome,
                );
                reveal(m1_initialized_speculative_kv_success);
            }
            Ok(outcome)
        },
        Err(error) => {
            proof {
                reveal(m1_initialized_speculative_kv_failure);
            }
            Err(error)
        },
    }
}

/// Strongest current verified terminal retired-page return boundary.
pub open spec fn m1_terminal_page_release_success(
    before: &IsolatedRequestKv,
    after: &IsolatedRequestKv,
    request: RequestId,
    role: Qwen3ModelRole,
    retired: PhysicalPageId,
    released: PhysicalPageId,
    exact_epoch: ferric_spec::completion::CompletionEpoch,
) -> bool {
    &&& ferric_spec::request_isolation::isolated_exact_page_release_transition(
        before,
        after,
        request,
        role,
        retired,
        released,
        exact_epoch,
    )
    &&& ferric_spec::request_isolation::request_identity_matches(
        after.projection_spec().request,
        before.projection_spec().request,
    )
    &&& after.projection_spec().quiescent_epoch == Some(exact_epoch)
    &&& after.projection_spec().target == before.projection_spec().target
    &&& after.projection_spec().draft == before.projection_spec().draft
    &&& ferric_spec::request_isolation::isolated_role_page_is_retired_at_epoch(
        before,
        request,
        role,
        exact_epoch,
        retired,
    )
    &&& ferric_spec::request_isolation::isolated_role_page_is_free_generation(
        after,
        role,
        released,
    )
    &&& released.role_spec() == retired.role_spec()
    &&& released.index_spec() == retired.index_spec()
    &&& released.generation_spec() as int == retired.generation_spec() as int + 1
    &&& if ferric_spec::request_isolation::target_role(role) {
        &&& after.projection_spec().target_retired_pages as int + 1
            == before.projection_spec().target_retired_pages as int
        &&& after.projection_spec().draft_retired_pages
            == before.projection_spec().draft_retired_pages
    } else {
        &&& ferric_spec::request_isolation::draft_role(role)
        &&& after.projection_spec().draft_retired_pages as int + 1
            == before.projection_spec().draft_retired_pages as int
        &&& after.projection_spec().target_retired_pages
            == before.projection_spec().target_retired_pages
    }
}

/// Exposes exact quiescent source-page return as an executable proof root for
/// source-inventory binding.
pub fn m1_terminal_page_release_relation_theorem(
    _before: &IsolatedRequestKv,
    _after: &IsolatedRequestKv,
    _request: RequestId,
    _role: Qwen3ModelRole,
    _retired: PhysicalPageId,
    _released: PhysicalPageId,
    _exact_epoch: ferric_spec::completion::CompletionEpoch,
)
    requires ferric_spec::request_isolation::isolated_exact_page_release_transition(
        _before,
        _after,
        _request,
        _role,
        _retired,
        _released,
        _exact_epoch,
    ),
    ensures m1_terminal_page_release_success(
        _before,
        _after,
        _request,
        _role,
        _retired,
        _released,
        _exact_epoch,
    ),
{
    proof {
        ferric_spec::request_isolation::isolated_exact_page_release_properties(
            _before,
            _after,
            _request,
            _role,
            _retired,
            _released,
            _exact_epoch,
        );
        reveal(m1_terminal_page_release_success);
    }
}

/// Returns one exact quiescent retired page and proves generation-safe reuse.
///
/// The sealed owning relation establishes retired `(role, index, generation)`
/// custody becoming the free `(role, index, generation + 1)` source-level slot.
/// It makes no claim that device bytes were scrubbed or that a later allocation
/// has already leased the successor.
///
/// # Errors
///
/// Returns the exact source-level routing, quiescence, counter, or physical
/// release rejection. Both request owners remain unchanged on failure.
pub fn m1_terminal_page_release_theorem(
    batch: &ContinuousBatch,
    selected: &mut IsolatedRequestKv,
    other: &mut IsolatedRequestKv,
    request: RequestId,
    role: Qwen3ModelRole,
    retired: PhysicalPageId,
    exact_epoch: ferric_spec::completion::CompletionEpoch,
) -> (result: Result<PhysicalPageId, RequestIsolationError>)
    requires batch.valid(),
    ensures match result {
        Ok(released) => {
            &&& *final(other) == *old(other)
            &&& m1_terminal_page_release_success(
                old(selected),
                final(selected),
                request,
                role,
                retired,
                released,
                exact_epoch,
            )
        },
        Err(_) => m1_initialized_speculative_kv_failure(
            old(selected),
            final(selected),
            old(other),
            final(other),
        ),
    },
{
    let ghost entry_selected = *selected;
    let ghost entry_other = *other;
    assert(entry_selected == *old(selected));
    assert(entry_other == *old(other));
    let result = ferric_spec::release_isolated_page(
        batch,
        selected,
        other,
        request,
        role,
        retired,
        exact_epoch,
    );
    match result {
        Ok(released) => {
            proof {
                ferric_spec::request_isolation::isolated_exact_page_release_properties(
                    &entry_selected,
                    selected,
                    request,
                    role,
                    retired,
                    released,
                    exact_epoch,
                );
                reveal(m1_terminal_page_release_success);
            }
            Ok(released)
        },
        Err(error) => {
            proof {
                reveal(m1_initialized_speculative_kv_failure);
            }
            Err(error)
        },
    }
}

} // verus!
