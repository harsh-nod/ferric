# Memory-Bounds DAG Scheduling

Status: compiler CPU qualification passed on MI350. The new compiler code and
nine regression tests pass the full and focused suites. No guarded HSACO, GPU execution,
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

## Measured Qualification

The [terminal receipt](attempt-v1/evidence/complete.json) records all 45 phases
and 32 scopes passing in 844.813792 seconds: 1,323 compiler tests and 1,516
Pliron tests, with focused repeats and extraction controls totaling 2,990
passing executions and 25 unchanged ignores. The nine new DAG tests pass in
both the full Pliron suite and the focused cohort. All 44 previous phase
recipes remain. Sources and dependencies are unchanged after the run; every
child exits naturally with status zero, is reaped and leaves no process group.
There are no timeout, forced-cleanup or postcheck errors.

The [retention manifest](attempt-v1/retention-manifest.json) authenticates 254
ordinary bodies, including 229 raw evidence files, in the 255-member capsule.
The measured compressed archive is 5,412,446 bytes, SHA-256
`836bfbf1cdcfd457c89b552047e189616e7758e294803d43a4690cdaffdbf155`.
The original receipt is 1,359,871 bytes, SHA-256
`bd1fa4827483cdf6ca6a0470aacd6826ddab007e8f129611b9725d3e0cdf8445`.
Retained raw JSON is not re-encoded.

Actual binary admission, guarded lowering, GPU correctness and the sustained
model benchmark remain separate gates. The source proposal's original pending
qualification wording is preserved as historical context, not current status.
