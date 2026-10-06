# Indexed-Atomic DAG Compiler Loader

The loader controller is prepared for the
[DAG analysis experiment](../guarded-mlp-indexed-atomic-dag-v1/README.md).
It requires that exact successful compiler generation, its complete source
lineage and final build products. Only the extractor and compiler backend
can replace the previously qualified seven-tool deployment's corresponding
entries. The other five entries must remain unchanged.

## Actual Loader Inspection

The deployed seven-tool generation passed all 14 `readelf` and `ldd`
inspections on `mi350` in 2.410962 seconds. Every child exited naturally with
status zero, was reaped and left no process group. Input and tool postchecks
were clean. The extractor's body is unchanged, but its provenance points to
the new qualified build; the compiler backend is new. The other five
deployment entries retain their exact previous provenance and bytes.

The [actual evidence](attempt-v1) includes the receipt, deployed controller,
input and tool manifests, producer/driver lineage and all 72 raw records.
Its archive has 93 members, 92 manifest pins and 18,052,513 body bytes.
Receipt SHA-256:
`1d9b96c32dfbeb2bc562867b3a6eb96c65da615df64313c39269f5beaad08089`.
Archive SHA-256:
`d59e479fb3a89bac9b33a63b6598eeb01c0ed4df806e81515dcf943d82e16b9e`.

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

Neither synthetic fixtures nor successful library inspection establish
guarded lowering, GPU execution, model correctness or performance. The
[actual guarded lowering result](../guarded-mlp-indexed-atomic-dag-lowering-v1/README.md)
is recorded separately. All issue #42 milestones remain open.
