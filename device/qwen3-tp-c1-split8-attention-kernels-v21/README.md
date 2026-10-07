# V21 P8 Split-Context Attention Proposal

Candidate isolated outside the active Ferric repository. The two typed device
roots pass remote formatting, twelve CPU tests and strict Clippy against c4c.
Tests cover host arithmetic, indexing and source contracts; emission and native
validation remain pending. No GPU correctness, numerical qualification or
speedup is claimed. V14/V17/V19 routing, artifacts and defaults are untouched.

## Why This Candidate

The accepted operation diagnostic is `../operation-report-r1.json`, SHA256
`421e0964c3dce1eb4c4aa59f764811106590387be05eea4c70acdca5df794907`.
All 12 cells passed their synthetic correctness and cleanup checks. Worker
aggregate wall latency divided by packet count, in microseconds:

| Operation | N1 | N4 | N16 | N16 Controller |
| --- | ---: | ---: | ---: | ---: |
| Residual | 158.413 | 39.528 | 16.341 | 29.278 |
| Attention, context192 | 357.194 | 266.162 | 239.570 | 259.370 |
| Q norm, width128 | 156.723 | 64.659 | 47.626 | 63.813 |
| Down projection, k12288 | 257.709 | 189.186 | 167.854 | 180.910 |

These are cache-hot synthetic repeated inputs with ordered writes, not GPU
timestamps, independent concurrent packets, model contribution estimates or
end-to-end TTFT/TPOT results. Controller and worker scopes overlap; subtracting
or adding them cannot establish GPU execution time. Do not multiply these
synthetic latencies by model layer count to claim a model-wide gain. V19's
accepted Qwen output parity did not show an end-to-end gain, so further serial
KV-copy tuning is not the priority suggested by this diagnostic.

The actual V14 attention path dispatches `rows * query_heads` Wave64 groups.
For TP1/C1 that is 32 waves. The kernel in
`device/qwen3-tp-wave-query-hoist-kernels-v14/src/attention.rs` loops the admitted
`max_context_tokens`, and `tp_execution/batched.rs:1498-1517,1670-1693` computes
that scalar from the selected rows' positions plus one. It is 192 at position191,
not the 8192-token allocated capacity. There is no inactive-capacity scan to
remove. V21 instead parallelizes the real active context across eight waves per
head, then performs a separate merge. At active192 that is 256 partial waves
with 24 tokens each, plus 32 merge waves with eight states each. This is an
algorithmic concurrency change, not a prediction of an eightfold speedup.

## Bounded Roots And Numerical Contract

```text
ferric_qwen3_tp_c1_split8_attention_partial_f32_v21
ferric_qwen3_tp_c1_split8_attention_merge_bf16_v21
```

The initial candidate admits only rows=1, TP=1, 32 query heads, eight KV heads,
head width128, page size16, up to512 physical/logical pages, and active context
128..256 inclusive. It requires `position + 1 == max_context_tokens`. Existing
query, position, page-table and output views retain their capacity-32 upper
bounds; they are not incorrectly required to be C1-sized. Only the first row
is read or written. Any future selector must fall back to the unchanged V14/V17
path for all other row counts, TP widths and contexts. No selector exists in
this proposal, and no existing artifact identity may admit the V21 roots.

For each head, contiguous partition p starts at `p * ceil(active/8)`, and ends
at `min(start + ceil(active/8), active)`. In the admitted range each of the eight
partitions is nonempty and at most32 tokens long. The final partition masks
every table/cache access by `token < active`, including partial-page tails.
The existing host-side page ownership, prefix/COW, scalar/view and immutable
input admission remains necessary. Kernel bounds do not prove arbitrary
pointer provenance or authorize aliasing.

Each partial preserves V14's two per-lane products, their addition, the same
Wave64 reduction and scale bits `0x3db504f3`, and the online-softmax recurrence
in ascending token order *within that partition*. It writes unnormalized FP32
maximum m, denominator l, and numerator n[128]. The merge seeds partition0,
then processes partitions1..7 in increasing order:

```text
new_m = max(m, part_m)
previous_weight = exp(m - new_m)
current_weight = exp(part_m - new_m)
l = l * previous_weight + part_l * current_weight
n = n * previous_weight + part_n * current_weight
m = new_m
```

Only after merging does each numerator divide by the denominator and narrow
once to BF16. Every input state, correction weight, accumulator and narrowed
output retains finite/positive-denominator checks. Invalid nonfinite inputs
are rejection cases, not successful-output contracts.

**This deliberately reassociates FP32 arithmetic. V14 bit parity is not
promised.** The host cancellation fixture demonstrates both FP32 and BF16
output divergence even with finite representable values. The CPU model's
`std::exp` is not an emulation of gfx950 device math. Its 1e-5 threshold applies
only to the fixed CPU fixtures compared with an independent f64 softmax; it
is not an acceptance tolerance for GPU or Qwen results. The unchanged exact
greedy Qwen reference gate remains mandatory. Do not lower that gate to admit
this candidate. If it fails, retain the failure and the baseline fallback;
parallel QK score materialization with the original serial recurrence is a
separate possible follow-up, not silently substituted here.

## Typed Scratch And ABI Expectations

There are two separate FP32 scratch allocations, 133120 bytes total (130 KiB):

| View | Elements | Shape | Partial Write Mapping |
| --- | ---: | --- | --- |
| Stats | 512 | [256][2] | `RowStriped2D<Index1D,64,1>`, lanes0/1 |
| Numerators | 32768 | [256][128] | `RowStriped2D<Index1D,64,2>`, two columns/lane |

Partial row is `head*8 + partition`. Lane0 writes the maximum and lane1 the
denominator; other lanes write only their two numerator columns. The typed
maps assign every scratch element to exactly one invocation. Merge reads both
arrays through `StridedReadView2D` and writes the first4096 BF16 output elements
with the existing two-columns-per-lane typed map. Capacity padding remains
untouched.

These kernels **depend on each other**. Any future controller must use the
current ordered `WaitForPrior` path, with the partial completed and visible
before merge reads scratch. They are not an independent-packet batch. Scratch
must not alias Q/K/V, page tables, output, other active layers or other requests;
reuse only after confirmed completion. Normal close, poison and error cleanup
remain existing worker responsibilities. Failed partial output is never an
initialized merge input or a successful timing sample.

The marker-derived `compiler_expectation_roster_v21()` is source expectation
metadata only and grants no admission or launch authority. Source ABI:

| Root | Explicit Arguments | Explicit Bytes | Implicit Offset | Total Kernarg |
| --- | --- | ---: | ---: | ---: |
| Partial | 7 slice pointer/count pairs, then 5 u32 | 132 | 136 | 392 |
| Merge | 3 slice pointer/count pairs | 48 | 48 | 304 |

Both require alignment8, COV6's existing256-byte implicit tail and Wave64.
Partial uses exactly256 workgroups of64; merge exactly32 of64. The five partial
scalars, in order, are rows, world_size, max_pages_per_sequence, physical_pages,
max_context_tokens. Its slice order is query, key_cache, value_cache, positions,
page_table, stats, numerators. Merge slice order is stats, numerators, output.
Element widths are2/2/2/4/4/4/4 bytes and4/4/2 bytes respectively. Real emitted
descriptor/metadata/ABI admission must match these expectations exactly.

## Remote Validation And Emission

The standalone crate pins fe2o3
`c4c5cdd0f69f3844386440a5addb4d4c3dce0e4b`. The required Pliron revision is
`7ebf6e6638c2a3bcec179423993b01211a9689b4`. The copied build-target helper is
unchanged, SHA256
`590e235c06df8041b8717caffaea9d1f56e40738d03d6947f4a3324cb171865a`.
Managed emission, not the host-only dummy binding, provides artifact identity.
Explicit range/length/ceil-div expressions follow existing compiler-facing
kernel patterns; the narrow Clippy allowances do not disable any device gate.

Root must stage an immutable private snapshot, generate/review its lockfile
and vendor graph, and run each phase under the existing resource-limited remote
wrapper. No commands below have been run by this source proposal:

```text
cargo metadata --offline --manifest-path <proposal>/Cargo.toml --format-version 1
cargo metadata --offline --locked --manifest-path <proposal>/Cargo.toml --format-version 1
cargo fmt --manifest-path <proposal>/Cargo.toml -- --check
cargo test --offline --locked --manifest-path <proposal>/Cargo.toml
cargo clippy --offline --locked --manifest-path <proposal>/Cargo.toml --all-targets -- -D warnings
```

Seven host tests cover every admitted partition length, ownership and padded
outputs, noncontiguous pages with poisoned inactive tails, exact constant-BF16
oracles, variable-value f64 comparison, deliberate reassociation, and invalid
states/nonfinite/overflow rejection. Five source-contract tests pin the typed
root ABI, launch/guards/masked accesses, ordered arithmetic and one-time BF16
narrowing, stores, exact producer and closed build target. Optional trailing
call commas are normalized structurally; argument mutations are still rejected.
These do not execute a device kernel or prove compiled arithmetic/ownership.

For CPU emission, adapt the bounded existing
`../upstream-c4c-migration/emit-v20.sh` for a new V21 source-review and output
directory, with all input/tool/vendor hash checks retained. Required changes
to its `engineering hsaco` command are only the candidate-specific identities:

```text
--crate ferric_qwen3_tp_c1_split8_attention_kernels_device_v21
--output-root <fresh-v21-output>/fe2o3-engineering-v1
--cargo-git-source https://github.com/harsh-nod/fe2o3.git@c4c5cdd0f69f3844386440a5addb4d4c3dce0e4b
--cargo-git-source https://github.com/harsh-nod/pliron.git@7ebf6e6638c2a3bcec179423993b01211a9689b4
-- --manifest-path <frozen-v21-source>/Cargo.toml --lib --no-default-features --features gfx950
```

Keep gfx950:xnack-, COV6, Wave64, exact backend/extractor/worker/tool hashes,
private vendor closure, resource bounds, before/after source hashes and exact
observation replay. The existing LLVM worker is a separately SHA-pinned
historical binary; a c4c Rust producer does not imply it was rebuilt at c4c.

Emission must resolve these specific risks without relaxing gates: dynamic
span proof at32; first-partition/tail control-flow convergence; two independently
typed output witnesses; narrow stats rows with64-lane ownership; FP32 NaN
sentinel constants; and the extra accumulator/register lifetime. Retain actual
VGPR/SGPR/scratch/LDS/launch metadata and ISA. Check no hidden spill or accidental
BF16 intermediate state, retained dot-product/reduction ordering, exact masked
cache accesses and expected store geometry. Report any compiler rejection as
such rather than treating host compilation as successful device emission.

## Native And Integration Gates

Before any timing, independently admit the new artifact and run a guarded
two-dispatch probe using existing held-image/ownership helpers and normal
supervised cleanup. Cover contexts128,129,135,191,192,193,255,256; multiple KV
heads; noncontiguous high physical pages; last-page poisoned tails; padded
query/position/table/output views; unchanged input bytes; full scratch/output
readback; and before/after guards. Use constant power-of-two exact BF16 fixtures
plus variable signed values, varied scores, cancellation and wide score gaps.
Compare both intermediate partition states and final results with independently
computed expectations. Report absolute/relative/BF16-distance diagnostics for
variable fixtures rather than inventing an acceptance tolerance after observing
the result. Invalid bounds, aliasing and scalar cases should be rejected by
host admission before launch; deliberate trap probes need the existing isolated
failure lifecycle and are excluded from all performance data.

Then reproduce the attention192 operation fixture using both old V14 and the
two-root V21 chain, counting **one attention operation as two V21 packets**.
Warmup and full correctness/guards must pass before measurement. Keep the same
immutable data, repetitions and cleanup policy; report controller/worker chain
wall latency and extra dispatch overhead honestly, not a per-packet number that
halves the apparent candidate cost. GPU timestamps, if independently admitted,
should remain a separate metric. These synthetic results still cannot establish
model gains.

Only after review should root add a distinct opt-in adapter/controller route.
It must retain the same BF16 decoder, FP32 head, greedy policy, 128-input and
128-output Qwen workload, exact independent reference, prefix caching off,
speculation off, both-arm image loading, eligibility fallback, and failure
cleanup. Add precise scratch extent/disjointness admission and ordered dependency
tests before GPU use. Retain baseline/candidate per-layer numeric diagnostics
and unchanged exact greedy IDs for all warmup/measured/diagnostic requests.
Measure repeated matched model-level TTFT/TPOT/throughput only after those gates,
including total extra packet counts. No default switch or win label is justified
by this isolated source proposal.

All new bodies are engineering-only and unverified. There is no formal proof,
active source-coverage inventory change, vendor qualification or serving claim.
