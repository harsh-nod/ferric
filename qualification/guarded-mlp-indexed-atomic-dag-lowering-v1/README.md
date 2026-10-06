# Indexed-Atomic DAG Guarded Lowering

This controller prepares a fresh guarded gfx950 lowering attempt using the
[DAG analysis experiment](../guarded-mlp-indexed-atomic-dag-v1/README.md).
It requires the actual successful compiler qualification and fresh
[binary-loader inspection](../guarded-mlp-indexed-atomic-dag-tool-audit-v1/README.md).
The guarded kernel sources, optimized-inlining profile, resource limits and
compiler command are unchanged.

## Controller Tests

All 19 synthetic tests passed on `mi350` in 2.328290 seconds: 15 admission
tests and four command/output-normalization tests. The bounded child exited
naturally with status zero, was reaped and left no process group. Source and
tool postchecks passed without timeout or forced cleanup.

The fixtures reject earlier producer generations, changed independent limits,
changed DAG semantics declarations, stale final products and mismatched
loader lineage. Missing actual receipt bindings refuse before helper or
output effects.

[controller-tests-v1/attempt-v1](controller-tests-v1/attempt-v1) retains all
executed sources, input, terminal receipt and seven raw records. Its archive
has 16 members with 161,405 body bytes. Receipt SHA-256:
`0871c7d6ee9df2f0f7351a0c964d7a78a02b7ad8ad60d6a4ab8154ccaacfa0ea`.
Archive SHA-256:
`95dcdc4f1fef79c943fef9abfa19d0a692a76590be82539dabd066e69d8b7461`.

The actual guarded lowering attempt has not run for this generation yet.
These fixtures do not emit an HSACO or establish GPU execution, independent
model correctness or sustained performance. All issue #42 milestones remain open.
