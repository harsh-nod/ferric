# Draft Wave M1 V16 Candidate

Explicit engineering candidate for Qwen3-0.6B single-token TP1 projections on
gfx950. Host compilation, 14 focused tests, formatting and Clippy pass on
mi300x-2. No adapter/default/catalog route is changed. No device emission,
native numerical result, token parity, or speedup is claimed.
This is not the existing target C1 wave improvement, and not a qualified gfx942
M1 extension. The current source priority is the draft scalar reduction chain;
no measured bottleneck is asserted.

## Integration Contract

`ProjectionRole::descriptor(1, 1)` supplies sealed inert metadata for eight
roles. It rejects other active row counts and TP widths. Getters expose the
root, grid, output carrier, and `[rows, n, k, world_size, projection]` scalar
order. The ABI is three slice descriptors followed by five u32 values, 68
explicit bytes before ABI padding/hidden arguments. Every launch is exactly
`grid=[N,1,1]`, `workgroup=[64,1,1]`. Required device target is
`gfx950:xnack-` with Wave64, checked by the existing build target contract.
COV6 is separately required at the emitted-artifact inspection gate.

| Role | N | K | Tag | Output |
| --- | ---: | ---: | ---: | --- |
| Query | 2048 | 1024 | 1 | BF16 |
| Key / Value | 1024 | 1024 | 2 / 3 | BF16 |
| Gate / Up | 3072 | 1024 | 4 / 5 | BF16 |
| AttentionOutput | 1024 | 2048 | 1 | FP32 |
| Down | 1024 | 3072 | 2 | FP32 |
| Head | 151936 | 1024 | 6 | FP32 |

Activations remain row-major and weights remain native `[N,K]`, without the
MFMA `[K,N]` transpose. `accepts_lengths` accepts activation capacity K..32K,
exact weight length NK, and output capacity N..32N, all in element counts.
Only the first activation row and first output row are used. FP32 O/Down
outputs still require the existing residual stage; FP32 Head still requires
the FP32 argmax stage. No output aliases, residual fusion, or logits narrowing
are introduced. Inputs must remain immutable throughout execution.

An explicit future adapter must first bind the two-root generated
`compiler_expectation_roster_v16()` to an emitted/inspected artifact and its
checked buffers, then select these descriptors only for the supported draft
M=1 shapes. Existing multi-row scalar/MFMA selection stays unchanged. The
descriptor and host binding fixture confer no load, launch, or completion
authority. V10 root rosters must not silently acquire V16 symbols.

## Arithmetic And Gates

One wave owns each output dot, with lane L reading K indices L+64*j. BF16 roots
perform 16 local steps; FP32 roots have a bound of 48 with inactive K steps
masked, covering K1024/2048/3072. Each wave performs sum64 and invalid-max64
before any numerical rejection or store. Guarded shared views supply NaN
outside their admitted view instead of a lane-dependent pre-collective trap.
Finite products, lane partials, the reduced sum, and BF16 narrowed outputs are
required. Only lane0 stores. Failed launches have no transactional output
promise and must retain existing runtime failure/quarantine handling.

The lane-strided partial sums and wave tree change FP32 association from the
scalar ascending-K left fold. Host tests explicitly exhibit unequal sum bits.
BF16 results and model tokens may also change; no universal parity is claimed.
FP32 Head/O/Down do not narrow. Source-level serial work reduction is not a
physical-memory-traffic or latency estimate.

This standalone candidate pins fe2o3-device/host to the observed main revision
`f7137ab8caa1fc0c9223f5fb21203b843209707c`, with Pliron
`7ebf6e6638c2a3bcec179423993b01211a9689b4`, edition 2024 and Rust 1.94 minimum.
No new compiler API is introduced: guarded views, Gfx950Subgroup sum/max,
RowStriped2D and typed kernel attributes already appear in V11/V15. The lockfile
is generated offline on mi300x-2. The host gate also passes against Ferric's
existing 46d/cc902 dependency pair and intermediate 47d/7ebf pair; those
baselines are retained separately. This
does not adopt the newer pin into the main inference adapter or rebuild its
native runtime. See the [performance tracker](../../docs/M1_PERFORMANCE_SWARM_V3.md).

Before any route integration or performance claim: emit with the admitted
compiler/Pliron pair, inspect
the resulting ABI/resources, run independent native numerical comparisons at
every role/width including failure cases, validate downstream draft/target
parity, and retain a matched end-to-end comparison. The host numerical model
and source/shape tests are not substitutes for those gates.
