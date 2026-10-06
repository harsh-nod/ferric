# Memory-Bounds DAG Scheduling

Status: source proposal only. The new compiler code and nine regression tests
have not yet passed MI350 qualification. No guarded HSACO, GPU execution,
model numerical result or performance acceptance is claimed. All issue #42
milestones and the 700 tokens/s target remain open.

The [last actual guarded compile](../guarded-mlp-ranked-cfg-linear-fusion-lowering-v1/README.md)
fails at a static memory-bounds work estimate. The proposed change first tries
the existing bound and solver unchanged. Only its exact work-hard-limit refusal
can select a whole-function DAG schedule, with no ownership contracts. Runtime
must validate the exact function, block roster, successor multigraph and
topological order before processing each reachable block once. A mismatched
schedule fails closed; it cannot fall back to the iterative solver under the
smaller bound. Every access predicate and resource ceiling remains unchanged.

The [source and resource derivation](proposal-v1/README.md) explain construction,
validation, fallible storage, unreachable blocks, duplicate edges and the
ownership restriction. The [test plan](proposal-v1/TEST-PLAN.md) includes an
independent set-based fixed-point reference and a constructed graph matching
the observed 567-block / 1,122-edge / 552-guard census. This fixture is not a
replay of the real guarded kernel's semantic payload.

Qualification must preserve all 44 previous phase recipes and add the new
focused Pliron cohort. Conditional on success, the full run will have 45
phases, 32 scopes, 2,990 passing executions and 25 unchanged ignores. These are
acceptance conditions, not measured results. Actual binary admission, guarded
lowering, GPU correctness and the sustained model benchmark remain separate
gates.
