# Checked shared-root construction

This checkpoint addresses the host-side rank-selection failure observed in the
[original interleaved fixture](../guarded-mlp-interleaved-native-v1/README.md).
The checked fixture passes its MI350 CPU suite and one bare-GPU numerical run.
It does not establish a general compiler repair or full-model correctness.

## Source Change

The [qualified fixture](cpu-attempt-v1/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_interleaved_native_v1_tests.rs)
starts from the original CPU-v1 source, not the diagnostic-only rebuild. Exactly
one of the 802 compiler/runtime source bodies changes. All allocation-role
predicates, coordinator code, private retirement proofs and three HSACOs remain
unchanged.

`checked_rank_roots` copies one checked common row before the inner root loop.
It validates all four shared tokens' group, rank, nonzero/distinct IDs and exact
extents before requesting private allocations. Each private token then receives
the same checks, including rejection of shared/private ID reuse. Tokens are
never relabeled to bypass an owner mismatch. The native fixture preserves its
existing allocation, write, poison and output order.

Three CPU regressions check:

- Both ranks' shared/private root identities through four pairs and the exact
  eight-segment schedule, including `Inputs` transfers and kernel-reference identity.
- Invalid shared groups, ranks, extents, IDs and duplicates before any private
  allocation callback, including the observed rank-0-token-for-rank-1 failure.
- Invalid private roles, duplicates and callback failures at every allocation
  position, with exact callback counts.

The identity test uses independently listed extents, observable inputs and
68 unique synthetic allocations. Its kernel metadata is a copy-test sentinel,
never admitted or dispatched.

## Actual Qualification

Builds and GPU execution ran on `ssh mi350`. The CPU build uses the same pinned
nightly 2026-04-03 toolchain, optimization level and offline locked dependencies.
The GPU attempt uses that exact executable without a debugger or rebuild.

| Gate | Actual Result |
| --- | --- |
| [CPU suite](cpu-attempt-v1/evidence/complete.json) | 1,060 passed, 0 failed, 7 ignored; all 1,064 earlier outcomes preserved |
| CPU inventory | 1,067 tests across five targets; 1,047 library tests |
| CPU lifecycle | 16 natural zero-exit phases; no forced cleanup or postcheck errors |
| [Bare GPU attempt](gpu-attempt-v1/evidence/complete.json) | One native invocation, zero retries; passed |
| [Independent reference](gpu-attempt-v1/evidence/verify.stdout) | 557,056 computed values, including 65,536 final BF16 values, bit-exact |
| Schedule | Eight paired segments and two delayed whole-bank rearms |
| Dense matrices | 48 hashes checked; 20 distinct bodies |
| Owner state | 104 compact readbacks and 16 terminal snapshots |
| Inactive payloads | 336 expected hashes checked |
| GPU lifecycle | Eight natural zero-exit phases; reaped children and absent process groups |
| Shutdown | Healthy queue-first Close; all eight GPUs idle before and after |

The unchanged integer reference also passes its 640-mutation selftest. Its only
adaptation is the libtest filtered count, from 1,043 to 1,046 because of the
three additional CPU tests. There is no numerical tolerance relaxation.

The whole GPU diagnostic takes 105.54 seconds, including dense fixture creation,
writes, observation export and independent verification. This is **not kernel
latency or model throughput**. One passing attempt is not a repeatability study.

## Retained Evidence

The [CPU retainer](retain_cpu.py) authenticates 108 members / 5,856,558 expanded
bytes, checks the single-source delta and preserves every prior named test
outcome. The [GPU retainer](retain_gpu.py) authenticates 59 members / 13,797,089
expanded bytes, joins CPU ancestry and actual observation/verifier output, and
checks the unchanged images. Both operate on evidence as data, without replaying
the retained project. Host executable metadata is retained, not its body.

| Artifact | SHA-256 |
| --- | --- |
| CPU receipt | `f59fd6413856014abe4d6c52597fc6771a6e26272d1d05b8d578c485a04ef8bb` |
| 11,262,384-byte host ELF | `8217b6bdaa7e698f6a92f8c829613de3b28772ebec92ff699a49270b89b48ded` |
| 927,876-byte CPU archive | `e5781caee2184d9db07d6cd4440d01b8657832fcb59fb2e68aa946bdeec95016` |
| GPU receipt | `de2d97bddd929cb4ee7ac2d11a68d76b3cf12d9db45c83a52decc53c15582a58` |
| 569,854-byte GPU archive | `523492f280fb2f027f2673dfa4f14853340f7f7bb79ddbb31129227b985be0f2` |

## Remaining Gates

The two synthetic layers still have independent fixed inputs, not a model layer
chain. This does not validate attention, KV updates, arbitrary weights or logits.
The selected MLP rounds SiLU to BF16 before multiplying Up; it is not a
single-narrowing full-model reference.

Ferric integration still needs an opaque engineering-only paired-owner interface,
transactional validation/reset of both Prefix284 and combined MLP bank states,
local guard generations separate from global forward order, and a narrowly
validated residual/output lifetime policy. Arena reclamation and sustained
full-model execution remain separate gates. Qwen3-8B BF16 target-only 2,048/256,
the 700 tokens/s target and issue #42 milestones M0-M7 remain open.

Follow-up: the [private exact-residual policy](../guarded-mlp-exact-residual-v1/README.md)
passes CPU qualification, and its [native exact-residual fixture](../guarded-mlp-exact-residual-native-v1/README.md)
now passes separately. Model integration remains pending; this earlier GPU
receipt does not qualify either later executable.
