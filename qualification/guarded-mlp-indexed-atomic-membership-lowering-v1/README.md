# Membership Guarded Lowering

The membership generation's lowering controller passes all 19 synthetic
fixtures on `mi350` in 3.962865 seconds: 15 admission/output-contract tests
and four normalization tests. The one bounded child exits naturally with
status zero, is reaped, and leaves no process group. Sources stay unchanged,
with clean postchecks and no timeout or forced cleanup.

The tests require the current qualified producer and loader receipts, retain
the original driver and vendor contracts, and reject stale generations,
changed limits, tool substitutions and missing bindings before effects.
A source-AST test separately checks the actual emitted result flags and
membership test roster. Baseline failure attribution is not promoted to a
claim about a new compilation.

[controller-tests-v1/attempt-v1](controller-tests-v1/attempt-v1) retains the
tested sources, fixed runner, input, actual receipt and seven raw files.
The receipt is `775742900d6bb0b7b0fb41b0d091939515467be37de0aebd478874cca9df60ac`.
The 16-member archive is
`13cc05ede8bc75da7451163d273ab6e2f88b41d74bdf8ffbd3aef06cfe890d91`.

These tests do not compile or run the guarded kernel. Actual lowering must use
the [qualified membership CPU build](../guarded-mlp-indexed-atomic-membership-v1/README.md)
after successful fresh loader inspection. No HSACO, GPU, numerical, full-model
or performance result is established here. All issue #42 milestones remain open.
