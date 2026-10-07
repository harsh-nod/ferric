# Selected Position-Five Down Exact-Error Diagnostic

Source-only proposal. No new test, diagnostic, model, GPU, or helper execution
has been performed. Future source qualification and the one bounded CPU-only
diagnostic belong to root on MI350. Original captures, prior results, runtime,
Ferric sources, arithmetic, policies, and acceptance criteria remain unchanged.

## Question and Closed Scope

At layer zero, prompt position 5 (token 271), native and independent framework
first residual and post-normalized inputs are byte-identical. Seven captured
MLP product words differ. The already executed R2 replay exactly reproduces
8,192 native final words and all 4,096 framework boundary words from each
side's own projection inputs; 14 cross-side final-hidden rows differ.

This proposal selects precisely those original rows:

```
219 955 1352 1408 1953 2040 2070 2208 2210 2328 2706 2850 3704 4017
```

Row 0 is one fixed matching control, chosen before any weight arithmetic. It
is not selected by an expected dot result. Both ranks' 6,144-column checkpoint
halves are read for each of the 15 rows. Sixty exact rank dots are evaluated:
15 rows times two ranks times two independently captured product vectors.
No model state is fed a reference tensor or an idealized intermediate.

## Arithmetic

The unchanged `exact_bf16.dot_units` evaluates BF16 products and sums as
integers in units of 2^-266. It supports the required 6,144 terms; the inherited
`head.exact_dot` is fixed at 4,096 and is not used for Down. The unchanged
`fp32_replay` finite decoding, FP32 addition, and BF16 rounding primitives join
the selected original R2 sum/narrowing boundaries. Its O-specific 2,048-term
`replay_rank` is not used. No new accumulation-tree emulator is claimed.

For each selected row let N0,N1 be exact native-own-input rank dots, F0,F1
the exact framework-own-input column-half dots, A0,A1 the observed native
FP32 partials, S the original R2-derived ordered FP32 sum, and BN/BF the
native-derived/framework-observed BF16 projections. All equations below are
exact real dyadic equalities, not tolerance checks:

```
input_delta = (N0 + N1) - (F0 + F1)
rank_error_r = Ar - Nr
tp_rounding = S - (A0 + A1)
native_bf16_rounding = BN - S
framework_projection_error = BF - (F0 + F1)

BN - BF = input_delta + rank_error_0 + rank_error_1
          + tp_rounding + native_bf16_rounding - framework_projection_error
```

The seven sparse terms `weight * (native_product - framework_product)` are
reported individually and must sum to the independently computed full-dot
delta, both per rank and jointly. Framework half dots are a diagnostic
partition, not fictitious observed framework partial accumulators. Its
projection error is not split into unknown framework implementation stages.

The report also accounts for the common residual R1 using the prior observed
final words: each side's residual-boundary error is `final - projection - R1`.
Their difference plus `BN - BF` must equal the observed final-word difference.
This does not rerun the full-vector R2 analysis; its original terminal, row
ledger, derived sums, and derived projections are retained inputs and joined
word-for-word to the original captures. Only the 15 selected sum/narrowing
boundaries are recalculated with the unchanged primitives as a consistency
check. New synthetic tests may exercise the inherited R2 implementation.

Own-input exact-dot BF16-RNE projections and their common-R1 boundaries are
reported as explicitly counterfactual values, never model outputs. Signed
zero encodings remain visible even where their exact numerical difference is
zero. Exact integers are serialized as decimal strings, avoiding JSON numeric
precision loss; no floating threshold is introduced.

## Checkpoint and Capture Custody

The runner derives from the actually executed Gate/Up exact controller. Its
prior full native/reference comparison admission is retained and rechecked,
not represented as a new full-capsule validation. The unchanged Gate/Up
extractor closes the common postnorm and seven earlier Gate/Up differences;
no actual Gate/Up dots are repeated. The unchanged position-five extractor
closes six-position sidecars, actual selected roles, common R1, two native
final vectors, original comparison pins, and repeated independent reference.
The new adapter adds the original native `activation` and reference `product`
roles with strict scope, dtype, shape, extent, part hash, and seven-word census.

Original native Begin commits the complete 178,103-byte upload manifest
`d5e66dbf7e3abfec463424addb6735f9a3a5b2d50653d56689ee79d01404da47`.
The original registration is reconstructed only at its already allowed
session/child-identity fields and must byte-hash to current Begin. Actual
Down source roles resolve to rank 0/id 188 and rank 1/id 189. Each upload is
50,331,648 bytes, respectively:

```
a3271db8c733981786d42ccc445072caa768d14e262a4275501ca9efa63ecd43
e0064427c369013147cfcecd2921e12fa350963dcf04044f81ab750b80f54e34
```

The original index pins `model.layers.0.mlp.down_proj.weight` to shard 1,
BF16 shape [4096,12288]. Each native upload is the row-major concatenation of
one 6,144-column half from all 4,096 rows, NOT a contiguous half of the
checkpoint tensor. The original full shard header/index roster is checked
using unchanged validation, with explicit Down shape and row-offset checks.
Both complete strided partitions must hash to their actual uploaded bytes.
The full 3,996,250,744-byte shard SHA is
`31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f`;
the 32,878-byte index SHA is
`f9fdbcb91c23971c13ec5d5f2573d2349e8f61f2f049371ec699281748fdb1bc`.

The entire shard is streamed and hashed before dot analysis and again after;
each scan uses one no-follow regular-file descriptor, stable full stat/inode
checks, and complete partition/selected-row hashes. Before/after descriptors'
identities and extracted bytes must match. The 30 selected halves are retained
as one 368,640-byte body with offsets and per-half pins. This is checkpoint to
original-upload commitment custody, not a new device-memory readback or a
cross-image ISA equivalence claim. The current selected tile image remains
the original 33,320-byte `b0d1766f...` image recorded by the admitted native run.

## Existing Evidence

All paths below are under
`/home/harsh/ferric-p227-integration/qualification/guarded-mlp-readiness40-causal-layer0-v1`.
`source-manifest.json` provides exact local original paths and complete pins.

- `gate-up-exact-v1`: actual 20-test exact Gate/Up run; terminal
  `2da4e9cdd79aef60ff3d9eb67c6062a6a32dabc8d42d3d711cea33dc97173b6b`.
  Native matches six own-input once-rounded ideals, framework one; no uniform
  framework-bit-exact accuracy assumption is made.
- `position5-r2-replay-v1`: actual 26-test boundary run; terminal
  `bf16c755ede242da389e99a2a907e2907e8cf442c2e82fd62a41c0a7e9d2528b`.
  The existing 1,008,054-byte `output/replay.json` SHA is
  `eee5940bdf309285cc68e03dd2e8889cf074024965c0466836c13d0e67409bdd`.
  Its derived Down and ordered-sum bodies are reused unchanged.
- Original causal comparison `3f94fe36...`, native terminal `00aa0447...`,
  and repeated independent reference inner receipt `5ebd12d2...` are retained
  inputs, with exact complete pins closed by the runner and manifest.

Prior head own-input exact dots, position-zero O replay, Gate/Up exact dots,
attention conditional gate, and position-five R2 replay are not repeated as
new numerical discoveries. Historical position-zero Down error envelopes are
not transferred to current position-five dots. No envelope or tolerance gate
is used by this exact-error diagnostic.

## Execution Contract and Tests

Fresh root:
`/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-position5-down-dot-v228-v2`.
Root stages the exact 18 source/document bodies plus source manifest and the
13 original input bodies named by the manifest. The model/index remain at the
same authenticated read-only model directory as the Gate/Up run. No installer,
download, launcher, subprocess framework, or model execution is added.

```
python3 -B run.py OBSERVED_SOURCE_MANIFEST_SHA256
```

Existing bounds remain 240 seconds wall, 180 CPU seconds, 512 MiB address
space, 2 MiB per output, CPU affinity 8/9, nice 10, zero core size, hidden GPU
environment, fixed MI350 host/uid, fresh output directory, and input/source
posthashes. There are three successful output bodies: `tests.stderr`,
`selected-weight-halves.bf16`, and `complete.json`. Failure writes `failed.json`
without promoting a diagnostic outcome; already written original files are
never overwritten. Root owns process retirement and evidence retention.

The 48 authored test names comprise 26 byte-identical executed R2 tests,
eight byte-identical Gate/Up mapping tests, and 14 focused Down tests. New
cases cover strided columns, full shard geometry, registration/alias/upload
joins, original products and six-position extraction, complete reused R2
rows, corrupted pins/roles/encodings, 6,144-term Fraction oracle, exact error
reconciliation, separated TP and BF16 rounding, nonfinite/overflow/signed-zero
refusals, counterfactual labeling, selected row bounds and sparse decomposition.
The tests have only been parsed as source; no pass is claimed.

## Nonclaims

Fourteen selected differences plus one control do not validate all Down rows,
all layers, or a full model. Observed accumulation error includes the native
implementation's actual association; this source does not independently prove
that machine association or emulate framework GEMM. Neither side is presumed
wrong because it differs from the other. No final position-five argmax cause
or fix, numerical acceptance, performance result, generated-token result,
temporal equivalence, full-model authority, tolerance change, or policy change
is asserted. Genuine Full correctness still requires its existing independent
own-history generated-ID and decoded-byte gates.

## V2 Fixture-Type Repair

The preserved V1 attempt ended in an honest failed terminal
`17135/f96283daae1281a78d6590b2f3706f4c81921c3bbecbe1fde00170d076fce9f1`.
It reported 47 passing tests, one failure and zero errors across the same 48 names;
no diagnostic or model-shard read occurred. The failing comparison expected a list
although `head.words()` returns the tuple from `struct.unpack()` and
`down.reuse_r2()` preserves that return type. Only the expected container in
that assertion becomes `(0x3f80,) * 4096`: all 4,096 values, the tuple contract,
all other assertions, names and production arithmetic remain unchanged.

This fresh V2 namespace is not a relabeled V1 outcome. All V1 source, helpers and
failed evidence remain untouched. V2 has not been executed, and no passing tests,
checkpoint rehash, diagnostic or new numerical result is assumed. The source
manifest records the complete predecessor, exact one-line repair and observed
failure provenance. New transport and retention helpers require fresh V2 paths
and observed outcome hashes; no old output directory is reused.
