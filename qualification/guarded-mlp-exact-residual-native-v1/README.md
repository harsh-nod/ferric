# Native Exact-Residual Reuse

The [private exact-own-residual policy](../guarded-mlp-exact-residual-v1/README.md)
now passes a native gfx950 numerical check on `ssh mi350`. Each rank reads its
finite old residual in R1, completes the guarded MLP and paired validators, then
overwrites that exact allocation with its R2 output. This is a synthetic
component qualification, not an enabled Ferric model route or throughput result.

## Fixture and Reference

The [qualified fixture](cpu-attempt-v1/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_interleaved_native_v1_tests.rs)
adds a separately named exact-residual case while preserving the original strict
case and its schema. Exactly one of 802 compiler/runtime source bodies changes
from the preceding policy checkpoint. The runtime policy, coordinator and three
HSACOs are unchanged.

Four retained pairs have eight private residual/output allocations across two
banks and two synthetic layers. Before each run, the fixture restores and reads
back the finite residual; it does not poison data that R1 must consume. It still
poisons the six writable MLP roots and checks inactive pairs before and after
intervening work. Eight segments use scales `[-1, 1, -2, 2, -4, 4, -8, 8]`, with
two delayed whole-bank rearms and bank-local generations `1, 1, 2, 2`.

For element `i` and rank `r`, the independently specified inputs are:

```text
p0[i] = ((i % 7) - 3) / 8
p1[i] = 1/16 - p0[i]
residual[r, i] = sign[r, i] - 1/16
```

R1 therefore still produces exactly `sign[r, i]`, preserving the previous stage
and final-output anchors. The initial residual lies on an odd multiple of
`1/16`, while final outputs lie on multiples of `1/8`. Every output element must
change, so an untouched allocation cannot satisfy the overwrite witness.

Each actual observation exports complete pre-run residual bytes, final output
bytes and equal residual/output allocation tokens. The [independent verifier](gpu-attempt-v1/verify_native.py)
uses integer dyadic arithmetic and encoded BF16/f32 bits, rather than the Rust
fixture's arithmetic or a floating-point tolerance. It checks eight distinct
payload identities, their stability across reuse, all pre-run values, all final
values and every element's overwrite. Tokens are evidence only, not authority
to reset owners or retire GPU work.

## Actual MI350 Results

| Check | Result |
| --- | --- |
| [CPU suite](cpu-attempt-v1/evidence/complete.json) | 1,067 passed, 0 failed, 8 intentionally ignored |
| Inventory | 1,075 names across five targets; 1,055 library tests |
| Prior outcomes | All 1,072 preserved; two new CPU tests and one ignored native test |
| CPU lifecycle | 16 natural zero-exit phases; no forced cleanup or postcheck errors |
| [Native GPU attempt](gpu-attempt-v1/evidence/complete.json) | One invocation, zero retries; passed |
| [Independent numerical check](gpu-attempt-v1/evidence/verify.stdout) | 557,056 computed elements, including 65,536 final BF16 elements, bit-exact |
| Residual reuse | 16 alias observations across eight distinct private allocations |
| Overwrite witness | All 65,536 final elements differ from the observed initial residual |
| Matrix evidence | 48 hashes checked; 20 distinct bodies |
| Owner and isolation checks | 104 owner readbacks, 16 terminal observations, 336 inactive payload hashes |
| Schedule | Eight segments, two delayed bank rearms |
| [Reference self-tests](gpu-attempt-v1/evidence/reference-tests.stdout) | 918 mutation refusals; fixed output/matrix anchors and malformed-input refusals passed |
| GPU lifecycle | Eight natural zero-exit phases, reaped children, absent process groups |
| Shutdown | Healthy queue-first Close; all eight GPUs idle before and after |

The build used pinned nightly 2026-04-03, offline locked dependencies, two jobs,
CPUs 8/9 and nice 10. The GPU attempt ran the exact CPU-qualified executable,
without a debugger or rebuild. CPU qualification took 65.74 seconds; the entire
GPU diagnostic took 123.86 seconds including fixture setup, transfer and
verification. Neither duration is kernel latency or model throughput. One pass
is not a repeatability study.

## Retained Evidence

The [CPU retainer](retain_cpu.py) authenticates 108 members / 5,907,307 expanded
bytes. The [GPU retainer](retain_gpu.py) authenticates 59 members / 13,838,695
expanded bytes and joins the executable, CPU receipt, raw observation and
independent verifier. These scripts retain data without replaying project code.
Host executable metadata is retained, not its body.

| Artifact | SHA-256 |
| --- | --- |
| CPU receipt | `79baed8dfa3f8302ed65827092c0036ac0cbf16a019661bcf36532b8b4eada8d` |
| 11,313,216-byte host ELF | `7da8e4b727b9006e17c61b7cf1fb243afe249793844685229fc3247ec35267c4` |
| 933,207-byte CPU archive | `521bab2749182cdc905617d7de271ce5f9577f14376f6bba98e3bc1c44b71613` |
| GPU receipt | `448f0fffb63cd5cc5ad72989f37b3c7896118df29a7fc92f2318020b218bbe69` |
| 575,787-byte GPU archive | `c79b0b9b680a41925ff131e21a16e1c2e694727da769ccda35e91c7eeaa25131` |

## Next Gate

Expose an opaque engineering-only paired interface, then integrate the model's
allocation ordering and one mixed Prefix284/Combined bank transaction. All
owners and private retirement proofs must validate before any reset. The new
guarded route must omit the old duplicate R2 dispatch and separate bank-local
generations from global forward order.

These synthetic layers are not a chained model execution. Attention, KV updates,
arbitrary model weights, logits, arena reclamation and sustained execution still
need qualification. The selected MLP narrows SiLU to BF16 before multiplying Up;
full-model references must account for that arithmetic contract. Worker
integration, production authority, full-model acceptance and performance claims
remain false. Qwen3-8B BF16 target-only 2,048/256, 700 tokens/s and M0-M7 remain open.
