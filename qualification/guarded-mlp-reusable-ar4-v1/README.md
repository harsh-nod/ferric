# Reusable Guarded AR4

This opt-in engineering route connects the qualified reusable-arena runtime to
Ferric's existing four-step Qwen3-8B BF16 TP2 decode diagnostic. It is not a
sustained benchmark, production admission, or a 700 tokens/s result. All issue
[#42](https://github.com/harsh-nod/ferric/issues/42) milestones remain open.

## Current Evidence

| Gate | MI350 result | Scope |
| --- | --- | --- |
| Runtime | 1,101 passes, 8 ignored; 10 facade doctests | [Separate runtime qualification](../guarded-mlp-reusable-arena-v1/README.md) |
| Worker | 615 passes, 4 unchanged ignores; 9 clean phases | [Actual CPU receipt](worker-cpu-v1/evidence/complete.json) |
| Census validator | 8 passes | [Synthetic checks](census-cpu-v1/complete.json), not observed GPU allocations |
| Parent | Pending | Separate build and selected tests |
| Native reusable AR4 | Pending | Actual storage reuse and complete payload equality |
| Sustained 2,048/256 | Pending | Independent numerical acceptance and performance |

The worker's twelve changed files are integrated from the exact formatted
postimages compiled on `mi350`, not the pre-format proposal. Its complete
184-file source closure, all 997 runtime/worker source pins, fifty raw evidence
files, and four Cargo artifact records are retained. The CPU run took
71.363692 seconds; that is build/test wall time, not GPU or inference timing.
All nine subprocess phases exited naturally, were reaped, and left no owned
process groups. Source, dependency, cache, and artifact postchecks passed.

## Implementation

The worker adds the explicit
`--engineering-native-guarded-mlp-reusable-ar4-v1` selector. Existing selectors
and the default fresh-allocation policy are unchanged. The new route uses
fe2o3 runtime commit `00e49fd8b0` and its consuming retirement proof; it does
not raise allocation limits or fall back to fresh allocation after failure.

Two banks each retain thirty-six layer slots across the two ranks. Global
forwards `1, 2, 3, 4` correspond to bank-local generations `1, 1, 2, 2`.
Before reuse, the runtime checks completed signals, actual consumed queue
frontiers, stable mapping/queue identities, and the entire selected bank.
Partial mutation or failed checks quarantine the runtime.

The separate profile hash binds the original model, images, session,
registration, devices, and request parameters. The existing four-forward wire
format, payloads, KV validation, bounded evidence, cancellation, and healthy
Close protocol remain in use. This route retains the conservative currentness
policy; it is not the earlier shared-full observation experiment.

After setup and each completed forward, the worker reads the actual live
allocation census from the Group and checks it against its ledger. The
expected sequence is:

```text
[715, 711] -> [751, 747] -> [787, 783] -> [787, 783] -> [787, 783]
```

These numbers are an expectation until the native gate passes. The worker
emits the five observed samples only after healthy Close, in a bounded
4,096-byte record bound to the profile, registration, session and device order.
The parent and publication validator must authenticate that record. The eight
synthetic tests reject stale identity, changed profile, missing or extra
samples, continued allocation growth, invalid types and incomplete Close.

## Next Demo Gate

Build the separate parent selector, then run the same four-step autoregressive
workload on MI350 with unchanged kernel images. Acceptance requires the actual
five-sample plateau, all four complete 606,976-byte observation payloads and
input histories equal to ordinary AR4, healthy Close, and clean device/process
postchecks. Payload equality is regression evidence, not an independent
full-model numerical acceptance decision or a speedup measurement.

Long-request framing, bounded output capture, independent numerical acceptance,
and sustained throughput remain separate work. Storage reuse removes the
fresh path's forward-38 capacity obstacle only if its native gate passes; it
does not by itself remove the measured host dispatch and observation costs.

## Reproduction Records

- Worker terminal SHA-256: `24feb83a0bde60dc9c6b8db28f2ce8252379b40d8e0d40483690f58038881b2a`.
- Worker ELF SHA-256: `a5c5c8b323e2d88ec27df1065709171fb9bd46114aff99b083b63d3644743fb8`.
- Worker capsule: 255 members, 254 pinned bodies, SHA-256 `8bd4bd893188738e28664d3c64a092c6eb0de74b07c28c1116f9e28c74651fa0`.
- [Worker capsule manifest](worker-cpu-v1/manifest.json) retains exact tool, source, command and private-cache provenance without executable or package bodies.
- Census CPU terminal SHA-256: `a1f048f2a32ccb44f5c2d7f6263faa3620a4eaf130a2c55222c8e2c32461f5a8`.
- [Census test output](census-cpu-v1/stderr) retains all eight named outcomes; the checker starts no subprocess and uses no GPU.
