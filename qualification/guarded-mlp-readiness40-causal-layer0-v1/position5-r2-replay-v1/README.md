# Position-Five R2 Boundary Replay

Source-only proposal. No tests, replay arithmetic, model forward, framework call,
or GPU work has been executed for this proposal. The source is outside the
canonical repository. Root owns review, staging and MI350 execution.

## Question

At layer zero, position five, the retained native/framework first residual and
post-normalization bytes match. Seven Gate/Up words differ under identical
post-normalized input, the product differs in four rank-zero and three rank-one
words, and the final hidden differs in fourteen words per rank. Earlier K and
attention differences do not survive to this captured first residual.

This diagnostic asks only whether each side's final hidden reproduces its
**own captured** projection/reduction/materialization/residual boundary.
It does not decide whether the upstream Down projection was accurate, attribute
the final token-9112/token-2 discrepancy, or establish model acceptance.

## Unchanged Arithmetic

The exact executed `r2.py`, `fp32_replay.py`, `exact_bf16.py`, `capture.py`,
`test_r2.py` and `test_fp32_replay.py` are copied from the qualified
`guarded-mlp-model-current-r2-replay-v1/attempt-v2/source`. Its successful
terminal is `12c18ae554980536fae43dbf9d09a4e306844874e3d554c6377ea46ec27f1188`.
That older position-zero result is ancestry, not a position-five outcome.

Native uses the original integer `r2.replay` on all 4,096 row indices, with
both original 4,096-word FP32 Down partials and each rank's BF16 R1/final hidden:

1. `FP32(+0 + partial0)`, then `FP32(+ partial1)`.
2. BF16 round-to-nearest-even materialization of the combined projection.
3. Widen that BF16 projection and the captured BF16 R1; add with FP32 RNE.
4. BF16 RNE to the final hidden.

Neither partial is narrowed individually. A once-rounded exact full Down dot
is not substituted for those actual partials. Signed zero, gradual underflow,
ties, finite operands and overflow retain the predecessor implementation/tests.

Framework is a separate all-4,096-row `r2.framework_control`: its captured
BF16 Down projection plus its own captured R1 must reproduce its BF16 final
hidden. The native-derived projection is not substituted for framework Down,
and the framework full projection is not relabeled as two native partials.

The retained `arithmetic.rs` and `kernels.rs` contracts are byte-exact both
to that replay's originals and the currently read canonical kernel sources.
This is a source-boundary replay, not a new proof of current machine-instruction
order or kernel-image equivalence.

## Original Data And Admission

The source manifest lists all eight unchanged input aliases and their original
SHA256/size, read-only from the already staged QKV input directory:

```text
E/guarded-mlp-causal-qkv-exact-v228-v1/inputs
```

Here `E` is
`/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
The byte-identical originals are retained canonically under
`qualification/guarded-mlp-readiness40-causal-layer0-v1/qkv-exact-v1/inputs`.
No new extraction, model shard transfer, tokenizer, or large checkpoint read
is needed.

`run.py` retains the executed attention controller's entire `admit` function:
the pinned actual comparison, native terminal/summary/sidecar, two repeated
reference sidecars/inner receipt, and original upload manifest are joined.
Prior full capsule authentication, both same-side parity gates, genuine repeated
reference, native healthy Close/40 forwards/zero outputs, model/bundle and
checkpoint metadata remain mandatory. This reuses that admission, rather than
claiming a fresh validation of every original process/capture record.

The byte-exact `head.py` and `qkv.py` are copied from the executed five-dot
diagnostic only for strict parsing, pins and the binary envelope API. No dot
function is called. `position5.py` verifies original full sidecar hashes,
all six native capture hashes and repeated framework payload/metadata before
selecting only generation 6, position 5, layer 0, prompt token 271.

Selected native roles have strict rank, scalar, extent, source offset and
boundary checks. All native parts are unique and contiguous, and the full
34-part native / 33-part framework position rosters remain present. Selected
R1/final-hidden pins also join the original 204-row comparison. Both native
R1 copies must match the framework's current R1; both native final outputs
must match each other, but need not match framework. Framework Down and its
MLP-output consumer must remain byte-identical. Nothing synthesizes a new
capture or rewrites the original part metadata.

## Qualification And Invocation

The runner is a narrow derivative of the actually executed conditional-attention
controller (`8df71f73...` terminal). Eleven inherited functions, including
stable same-FD reads, input posthashes and prior admission, are AST-identical;
only `main` and fixed namespace/source/test constants change.

Twenty unchanged integer/captured-boundary tests plus six new mapping/custody
tests run first. The latter cover all-row mapping, wrong position/generation/
layer/token and boolean rank, missing/duplicate/incorrect geometry, changed
original body/part/comparison pins, failed repeat/Close/parity, and a reported
output mismatch without numerical authority. The manifest closes all 26
qualified method names. These are synthetic tests, not original model results.

Root may stage the fifteen manifest/source files into the fresh directory

```text
E/guarded-mlp-causal-position5-r2-replay-v228-v1
python3 -B run.py OBSERVED_SOURCE_MANIFEST_SHA256
```

The same 180-second wall, 120-second CPU, 512 MiB address space, CPU 8/9,
nice-10, hidden-GPU, 2 MiB per-file, fresh-output and signal bounds apply.
No retries or source/output overwrites are implemented. All source/input bodies
are checked again afterward; an exhausted deadline cannot produce a pass.

Outputs are the raw 26-test stream, `replay.json` with all 4,096 native row
records and the independent framework control, four original replay-derived
binary arrays, and an original complete/failed terminal. Every derived output
is pinned in the diagnostic. A failed boundary comparison remains failed,
with the computed original rows retained; no expected success is prebound.

## Claims Kept Separate

A passing result would establish only exact agreement with these two captured
boundary semantics. It would not evaluate Down dots, SiLU, other layers,
uncaptured positions, BF16 aggregate error or the cause of the position-five
argmax. It supplies no tolerance and changes no arithmetic.

Readiness40's position-five prediction is not fed into position six. The
existing Full2303 behavioral gate still requires exactly 256 own-generated IDs
and decoded bytes against the independent two-pass BF16 reference, with no tie
exception. This optional local diagnostic must not replace that end-to-end
2,048-prompt/256-output target or be presented as GPU/performance acceptance.
