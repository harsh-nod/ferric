# Indexed-Atomic DAG Guarded Lowering

This controller prepares a fresh guarded gfx950 lowering attempt using the
[DAG analysis experiment](../guarded-mlp-indexed-atomic-dag-v1/README.md).
It requires the actual successful compiler qualification and fresh
[binary-loader inspection](../guarded-mlp-indexed-atomic-dag-tool-audit-v1/README.md).
The guarded kernel sources, optimized-inlining profile, resource limits and
compiler command are unchanged.

## Actual Lowering Result

The actual guarded gfx950 compile still fails with:

```text
semantic-to-ranked projection rejected uniform induction CFG analysis exceeds its work limit
```

The compiler exited naturally with status one after 117.555086 seconds;
the whole attempt took 121.172512 seconds. The child was reaped and left no
process group. Sources remained unchanged and input postchecks were clean,
with no timeout or forced cleanup. No HSACO or GPU result was produced.

The DAG optimization passed full compiler qualification but did not remove
the observed work-limit rejection. The generic diagnostic still identifies
neither the function nor the charge site, so this does not establish whether
the optimized helper was reached or whether alias checks pass. The next
experiment adds bounded function/site/work reporting on the existing failure
path. It must preserve the work limit and charge semantics.

[attempt-v1](attempt-v1) retains the actual bound lowering controller,
failed receipt and all ten raw records. Its archive contains 13 members,
12 manifest pins and 4,203,822 body bytes. Receipt SHA-256:
`a83ebf9785687020282a65ba2b554e95590ac2405a20c06d40cbd4a6b23c3400`.
Archive SHA-256:
`0c16d26b6ad96359cf67a86b6dcd995b3d8bbce8c235c77f0da59a3cade63bf9`.

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

These fixtures do not emit an HSACO or establish GPU execution, independent
model correctness or sustained performance. The actual attempt above failed;
all issue #42 milestones remain open.
