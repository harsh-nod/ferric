# Bounded CFG Capacity Lowering

Actual guarded gfx950 lowering with the
[qualified block-cap experiment](../guarded-mlp-ranked-cfg-capacity-v1/README.md)
reaches an independent CFG-analysis work-limit rejection. No HSACO was
emitted and no GPU kernel ran. The controller separately passed 19 synthetic
tests; those fixtures are not a successful lowering result.

## Actual Attempt

The previously observed block-count rejection is no longer reported. The
new compiler diagnostic is:

```text
semantic-to-ranked projection rejected uniform induction CFG analysis exceeds its work limit
```

The graph-work ceiling remains 3,145,728 units. This generic diagnostic does
not identify the function, analysis site or charged amount; it does not
establish an alias-analysis outcome. The next investigation targets repeated
graph-analysis work while preserving the independent resource ceiling.

The compiler exited naturally with status one after 117.958347 seconds,
was reaped and left no process group. The whole attempt took 121.538638
seconds. Sources and input/provider integrity checks were unchanged, with
no timeout or forced cleanup. The controller used the qualified final
compiler products after the [14 passing loader inspections](../guarded-mlp-ranked-cfg-cap-tool-audit-v2/README.md).

[attempt-v1](attempt-v1) retains the executed controller, failed receipt
and all ten raw records. Its manifest pins 12 bodies in 13 archive members
totaling 4,196,345 bytes. Receipt SHA-256:
`6761af874bf6040a8a2e5056aea66e4a9586867c5b196f1cb1bc452f4461edf0`.
The 669,603-byte archive SHA-256 is
`591ae5c4cc93a82cb46620242eacf6b6dc32e64b6ebd388456b7fe2beae56d0d`.

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
