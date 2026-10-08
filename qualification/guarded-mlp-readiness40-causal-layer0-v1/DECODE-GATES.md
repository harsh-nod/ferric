# Current Decode Gates

Policy inventory against Ferric source commit
`b61588712af31738d3a744fae81b6afcfdd9ea9a`. This note changes no numerical
policy, tolerance, original receipt or execution limit.

## Prompt Diagnostics

Readiness40 executes the authentic prompt's first 40 tokens with its own KV
state and zero generated tokens. Its prediction at position 5 is not fed to
position 6. Full2303 instead emits generated token 0 after prompt position
2047 and feeds only the preceding committed native output at positions 2048
through 2302. These are distinct closed profiles in the
[long wire](../../adapters/tp-peer-finite-engineering-worker-v1/src/finite_guarded_mlp_long_wire_v2.rs#L13).

The observed 39/40 prompt-position argmax agreement is a diagnostic result,
not a failed predeclared generated-token gate. The independent
[reference](../guarded-mlp-readiness40-v1/reference-v1/source/reference.py#L162)
and [comparison](../guarded-mlp-readiness40-v1/comparison-v2/source/compare.py#L79)
explicitly report no acceptance threshold and no numerical acceptance.
Their applicable finite-value, own lowest-ID BF16 argmax, repeated reference,
own-KV, custody and lifecycle checks passed. No additional full-model tensor
threshold failure follows from those observations.

At position 5 the reference ties tokens 2 and 9112 at BF16 19.625; native logits
are 19.625 and 19.75. Both decisions obey their own logits and tie rule. This
does not permit ignoring a future generated-token disagreement. Nor is bitwise
framework equality an accuracy oracle: exact dots select native at three of
five inspected Q/K elements and both inspected first-residual differences.
Those local findings do not establish full-model numerical acceptance.

## Existing Requirements

The existing generated-output behavioral requirement is exact generated token
IDs and decoded bytes, with each implementation following its own choices:
the [argmax canary](../../docs/TP1_ARGMAX_CANARY_V11.md#L38),
[attention canary](../../docs/TP1_ATTENTION_ARGMAX_CANARY_V11.md#L43) and
[comparison tool](../../adapters/m1-engineering-execution-v1/tools/compare_tp_batch.py#L416)
record this distinction. These establish the requirement, not an executed
guarded 2048/256 result. A generated mismatch must fail this scoped gate even
at a reference tie. Tensor comparisons after different choices must be labeled
different-input trajectories.

The [existing attention policy](../prefix-profile-numerical-v1/p227-prefix-numerical-sidecar-v1/attention-policy.json)
requires exact position-zero output; later outputs may differ by at most one
BF16 step OR absolute error at most `5e-5 * per_KV_head_max_abs_V`. It requires
finite causal inputs and uses the frozen FP64/FP32/BF16 reference. Historical
norm, GEMV, SiLU and Down bounds are conditional operator contracts, not an
aggregate full-model tolerance. Older residual materialization semantics must
not replace the current boundary.

The generic [M1 acceptance command](../../benches/m1/differential.rs#L1433)
requires a separately reviewed, plan-hash-bound policy with per-case logit ULP
and token-mismatch limits; it has no default tolerance. Its
[seven fixed cases](../../benches/m1/README.md#L226) are not Full2303. The
current causal diagnostic does not supply an admitted aggregate tensor policy
for that full workload.

## Next End-To-End Gate

The explicit guarded Full2303 transport and source are now CPU-qualified.
Complete its actual full-run preparer, launcher, worker ELF and retainer
bindings; establish measured feasibility within the unchanged abort bound;
then run the authentic 2,048-token prompt and 256 own generated tokens with
finite/custody/queue/KV checks, compact records, selected original payloads
and healthy Close. Compare all 256 generated IDs and raw decoded bytes with
the existing independent two-pass framework reference on the same checkpoint
and prompt, each using fresh own KV and greedy recurrence. The strict
comparator already exists. This is a scoped behavioral gate, not a substitute
for separate full-model tensor acceptance.

The older finite-long route has different image, owner and framing contracts
and cannot silently substitute. The existing Readiness40 one-hour cap remains
unchanged. New Full2303 source also starts with a separately explicit one-hour
cap; that is an abort bound, not admission that this workload can finish.

The observed 944.9 seconds is the whole 40-forward causal diagnostic elapsed,
including setup, admission and postflight, not isolated forward time. Naively
multiplying it by `2303 / 40` gives about 15.1 hours, but this is neither a
reliable forecast nor launch admission. A reviewed bounded execution budget
and measured feasibility are still required. No longer timeout, fast-prefill
claim, runtime fallback or 700 tokens/s claim follows from this note.
