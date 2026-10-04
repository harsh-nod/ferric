# Per-Profile Prefix Numerical Comparator

Pure CPU comparison support, not a runtime admission. Execution results belong
in separate source-pinned records. There is no launcher, CLI, result writer,
runtime change, or synthetic success receipt in this module.

`compare_profile.py` checks either `baseline_v5` or `tiles_v6`, independently,
using the existing numerical references and unchanged bounds. It never calls
either old `compare()` wrapper or `validation.observation()`: those require
paired V5/V6 bitwise parity, which is not a numerical correctness criterion.

## API And Inputs

Call `helpers(stage_root, sidecar_root)` with canonical paths to the retained
`p227-prefix-stage-numerical-v1` and `p227-prefix-numerical-sidecar-v1` packages.
Defaults are their sibling proposal directories. The loader pins both entry
modules; their existing loaders verify reference sources, fixture metadata,
policies, and NumPy 2.2.6. No helper or bound is copied or rewritten.

Then call `compare_profile(plan, reader, frozen)`. `reader(FilePin, maximum)`
returns exact bytes; the comparator independently checks each pin. The existing
sidecar `Reader` is suitable. A future host caller must retain the plan pin and
call `reader.recheck()` before publishing its own result. This function performs
no filesystem writes and never turns its return value into an acceptance gate.

The plan has exactly these fields:

```text
schema: "ferric-p228-prefix-profile-numerical-inputs-v1"
profile: "baseline_v5" or "tiles_v6"
case: one of the six unchanged validation.CASES
baseline_request: FilePin for the original TP2 input request
ranks: exactly two ordered objects, each containing:
  rank: integer 0 or 1
  capture: full native rank capture FilePin
  input_sha256: seven original input hashes from that profile's child receipt
  output_weights: corresponding original rank-local O shard FilePin
```

Each FilePin has exactly `path`, `bytes`, and `sha256`. Capture extent/order is
the unchanged seven-stage layout, including both entire physical KV caches.
The first six inputs are obtained through the retained `expected_inputs()`
API, with authentic fixture metadata and learned Q/K weight halves. O shard
digests remain the two pinned originals. No alternate shape, case, precision,
reference, or adjustable tolerance is accepted by the production loader.

## Numerical Checks

`check_rank()` checks wave RMSNorm, QKV, serial head normalization, split-half
RoPE, and the current KV append. Attention uses that same capture's Q and
unpaged causal K/V with the independent FP64 two-pass oracle. The unchanged
policy requires exact BF16 at position zero; otherwise at most one BF16 step
OR the fixed cancellation allowance. O uses that capture's accepted attention
and the existing dense FP64 reference with the unchanged outward error bound.
Every operator is conditioned on the actual preceding stage, not a separately
evolved whole-model reference trajectory. Failure propagates; parity cannot
rescue a numerical mismatch.

Complete captures are hashed. Prior causal K/V words must be finite, but their
historical numerical origin is not checked here. Untouched KV bytes, including
future poison, are NOT compared with the initial cache: the native
`qwen_prefix_tiles_comparison_v6/execution.rs::validate_run` already performs
that check against `Prepared.initial`. The future host adapter must retain and
authenticate that validation; it cannot infer it from this numerical result.

## Required Host Join

The existing native route is `process.rs::run_child`, with child flag
`--execute-reviewed-engineering-prefix-profile-child-v1`. Its names are
`baseline-v5` / `tiles-v6`; receipts use `baseline_v5` / `tiles_v6`. Run the
baseline child first: the candidate child's capture census requires its two
files. This document does not authorize or implement either execution.

Before using this comparator on real evidence, the host adapter must validate
the closed child receipt, exact executable/request/artifact/source identities,
two states and terminal words, immutable-input readback, initial-output hashes,
untouched KV validation, completed Close, exited/reaped owned processes, full
capture census, and existing pre/post device audits. It must independently
review the actual artifact's arithmetic/ISA prerequisites, including sqrt,
exact reciprocal rounding, noncontraction, and exponential behavior. Loaded
historical source-policy metadata is NOT a review of the new compiled image.

All acceptance, GPU provenance, runtime-premise, historical-KV, untouched-KV,
model, performance, and production-authority fields remain false, even when
conditional checks pass. A later host integration needs real captured data and
these separate joins before making any stronger claim.

## Authored Tests

`test_compare_profile.py` contains CPU-only synthetic routing/refusal tests and
synthetic fixed-shape reference checks. Synthetic fixtures are explicitly
test-only; mocked routing checks do not establish real numerical or GPU success.
The real-reference synthetic cases cover position zero and `patterned-pos16`.
The latter has sixteen nonzero historical KV rows, current zero KV, and the
logical page transition from token15/physical slot127 to token16/slot192.
Zero queries make all seventeen attention weights equal; exactly representable
BF16 history values have a known mean, with sign and power-of-two variations
across ranks/heads/dimensions. A finite corruption of token15's value, with an
honestly updated capture pin, must fail the unchanged real attention oracle.
These are not authentic model histories or evidence of native execution.
Run them in the retained NumPy 2.2.6 environment, with
assertions enabled and bounded CPU ownership. The intended unittest command is
`python -B -m unittest discover -s PACKAGE -p test_compare_profile.py -v`.
Test success is not a replacement for the separate host joins above.
