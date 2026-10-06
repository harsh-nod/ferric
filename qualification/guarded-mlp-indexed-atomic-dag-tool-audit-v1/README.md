# Indexed-Atomic DAG Compiler Loader

The loader controller is prepared for the
[DAG analysis experiment](../guarded-mlp-indexed-atomic-dag-v1/README.md).
It requires that exact successful compiler generation, its complete source
lineage and final build products. Only the extractor and compiler backend
can replace the previously qualified seven-tool deployment's corresponding
entries. The other five entries must remain unchanged.

## Controller Tests

All 26 synthetic admission tests passed on `mi350` in 12.364962 seconds.
The bounded child exited naturally with status zero, was reaped and left
no process group. Sources and tool postchecks were unchanged, with no timeout
or forced cleanup. These tests cover incomplete qualification, stale products,
changed source bodies, lost tests, changed independent limits and altered
compiler-generation provenance.

[controller-tests-v1/attempt-v1](controller-tests-v1/attempt-v1) retains the
executed sources, input, terminal receipt and all seven raw records. The
archive contains 15 members with 191,501 body bytes. Receipt SHA-256:
`18eea15f50260639f92f4cd9e6e065b49c1159557a4c39fb13a7da0ac9cade86`.
Archive SHA-256:
`85cc3708b2467345e8b1568f304c77a14496fe78703ef7cc7d68b22a08c2953f`.

An actual binary-loader inspection has not run for this generation yet.
Synthetic fixtures do not establish successful guarded lowering, GPU
execution, model correctness or performance. All issue #42 milestones remain open.
