# Bounded CFG Capacity Lowering

The lowering controller for the [bounded block-cap experiment](../guarded-mlp-ranked-cfg-capacity-v1/README.md)
has passed 19 synthetic tests on MI350. Actual guarded gfx950 lowering is
still pending. Synthetic fixtures do not emit an HSACO or execute a kernel.

## Controller Tests

Fifteen admission fixtures check authenticated producer and loader evidence,
source and tool identity, final rather than earlier Cargo products, preserved
resource limits and rejection of failed or diagnostic-only generations.
Four normalization fixtures preserve the explicit optimized MIR flags.
Unbound actual receipt hashes must reject before any helper or output effect.

The fixed runner completed in 2.671612 seconds. Its sole child exited
naturally with status zero, was reaped and left no process group. All 19
names passed, with no skips or unexpected successes. Source and tool
postchecks passed; process and resource bounds were unchanged.

[controller-tests-v1/attempt-v1](controller-tests-v1/attempt-v1) retains the
executed sources, exact inputs, seven raw records and receipt. All 16
archive members were verified after transfer. Receipt SHA-256:
`bbbdfa71065d3f941e2f77e10d4b6b954465f116d99403d8733b05dfadf16654`.

Guarded HSACO/GPU execution, independent model numerics and sustained
BF16 2,048/256 decode remain open. No issue #42 milestone is closed.
