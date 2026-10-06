# Captured R2 Boundary Replay

Source-only, bounded numerical diagnostic proposal. No tests, replay, framework
or GPU work have been executed by this proposal's author. The future replay
terminal remains unknown. Existing actual source/data pins are bound; the
original capture and previous reports are never changed.

## Question

Do the two observed guarded final-hidden vectors exactly follow the specified
R2 arithmetic on their *actual captured* dedicated Down partials and first
residuals? This is the next boundary after the current O join. It is not a new
MLP dot-product replay, independent full-model reference or acceptance rule.

For each of 4096 indices, the unchanged integer `fp32_replay.combined` performs:

1. FP32 RNE addition of positive zero and rank-0 partial.
2. FP32 RNE addition of the rank-1 partial.
3. BF16 RNE materialization of the combined projection.
4. FP32 RNE addition of the current rank's BF16 residual.
5. BF16 RNE final narrowing.

Neither partial is independently narrowed. Residual operands remain separate
for each rank, even though this actual captured workload may have equal bytes.
Finite checks, signed zero, gradual underflow, intermediate overflow and both
rounding boundaries are preserved. No host floating-point operation computes
the native expected encodings.

## Input Custody

`inputs.json` declares fourteen original data bodies. They include the actual
capture terminal `324b40f7`, native summary, capture envelope `f28715ef`, checked
capture, first request and observation, and actual controller `afa4d007`.
The independent numerical report `0a8905b7` supplies the already-qualified
full-model/capture admission and original readset joins. This proposal does not
repeat all 78 raw files, library/process audits, model weights or the full-model
validator. Its receipt states that boundary explicitly.

The `capture_admission` function is copied without AST changes from the actual
controller. It is rechecked against the pinned controller before replay and is
called only as a pure data function. Its closed three-body reader allows only
the original capture stderr, first request and first observation. It verifies
all 34 parts, all finite scalars, exact metadata/rotary bytes, physical offsets,
same-run identities, healthy Close, and final-hidden bytes against the actual
first observation. The six selected R2 slices are then independently identified
by rank/role and carry original offsets and content hashes in `replay.json`.

The qualified candidate CPU receipt `ebdcb5f3` and source map `bbe2b9e0` bind
the retained `contract/arithmetic.rs` and `contract/kernels.rs` bodies. These
are source-contract evidence only, never compiled or imported here. Agreement
does not independently prove the current machine instruction sequence.

Three already captured genuine-framework tensors provide a separate residual
boundary control: materialized Down projection, first residual and final hidden.
Their original logical pins are joined to the previous numerical report and
the exact `f69fff01` alias manifest; physical paths point to the already staged
reference copies. These framework operands are **not** substituted into the
native replay. No framework call or reference dot product occurs.

## Tests And Outcomes

There are 20 exact named tests: the twelve unchanged, previously executed
Fraction/nearest-neighbor FP32 tests plus eight focused R2/custody tests.
New cases cover all rows and distinct rank residuals, a single mutated output,
nonfinite inputs/outputs, exact role/extent/type checks, malformed part geometry
and hashes, wrong identities/failed prior admission, the separate framework
control, ties, signed zero, subnormals and overflow. The runner checks the
unittest inventory exactly and retains the raw log. No skip is accepted.

A successful actual run requires all 20 tests, clean posthashes, all 8192 native
output encodings matching, and all 4096 framework boundary-control encodings
matching. A finite native mismatch is valid diagnostic evidence but is **not**
a successful gate: all row results and derived tensors are written first, then
the terminal is `failed.json`. An admission/test/nonfinite/timeout failure also
stays failed and never invents an all-row outcome. Existing receipts are not
rewritten or retried by the runner.

Outputs are `tests.stderr`, `replay.json`, `derived-ordered-sum.f32`,
`derived-down.bf16`, two `replayed-final-rank*.bf16` files and a complete/failed
terminal. The materialized Down and replayed outputs are explicitly derived,
not added to the historical capture as observations. Every row records both
original partial encodings, both residual operands, both observed outputs,
ordered sum, derived projection and both match results.

## Root-Owned Execution

Stage the ten executable/data inputs together, retaining the `contract/`
subdirectory: `run.py`, the six other Python files, `inputs.json`, and two Rust
contract bodies. The README/source manifest are publishing metadata, not
runtime inputs. The fourteen original bodies must remain at their bound paths;
there is no missing-path fallback or implicit source substitution.

Run once with `python3 -I -B run.py`. It creates the fresh directory
`E/guarded-mlp-model-current-r2-replay-v228-v1`. Optional `--self-test` uses a
separate fresh `...-v228-v1-tests` directory and makes no actual-data claim.
Root alone executes, stages and retains outcomes.

The data-only controller has a 120-second wall/CPU deadline, 256 MiB address
space, 8 MiB input-body cap, 40-input/32 MiB readset cap, 4 MiB output-body cap
and 8 MiB output aggregate cap. Inputs must be stable ordinary canonical files;
their full contents are hashed before/after. Outputs are freshly created and
posthashed. Only exact pinned pure helpers/tests are loaded, GPU visibility is
empty, and no subprocess, network, compiler, native worker or GPU API is used.

Numerical acceptance, thresholds, full-model acceptance, performance claims
and production authority remain false even on a bit-exact boundary result.
Matched-input MLP analysis and cumulative multi-layer behavior remain open.
