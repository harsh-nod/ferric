# V7 All-Layer Autoregressive Observation

On 2026-10-04, the V7 prefix image completed four autoregressive forwards
through all 36 Qwen3-8B layers on `mi350`, using the unchanged CPU633 parent,
CPU475 worker and shared-full-currentness policy. One native attempt completed
Close5 and clean owned-process reaping; all six surrounding device audits passed.
The publisher rehashed all 58 retained case files (4,594,564 bytes).

The separate independent framework comparison found matching input histories
and argmax tokens at all four positions. However, **all 152 complete tensor rows
differ from the framework**. Matching tokens do not establish tensor acceptance.

| Position | Input token | Ferric/reference output | Logit relative L2 | Maximum absolute error |
| --- | ---: | ---: | ---: | ---: |
| 0 | 9112 | 67 | 0.00471959 | 0.07421875 |
| 1 | 67 | 25 | 0.00660156 | 0.125 |
| 2 | 25 | 576 | 0.00598495 | 0.125 |
| 3 | 576 | 2701 | 0.00387545 | 0.1875 |

These positions are 0 through 3, not a 2,048-token prefill followed by 256
generated tokens. Forward host intervals were 7.2213, 7.2704, 7.7708 and
7.7597 seconds. They include host validation and transport, are not GPU timing,
and do not constitute a sustained throughput result or a speedup claim.
Numerical, full-model, production and performance acceptance remain false.
The comparison uses no newly chosen acceptance tolerance.

## Evidence

- [Machine-readable result](result.json) and [actual GPU completion](gpu-complete.json).
- [Independent tensor diagnostics](framework-diagnostic.json), including all 152 rows.
- [Structural observation](observation.json) and [host counters](host-observation.json).
- [Exact plan](plan.json), [request](request.json) and [engineering review](decode-review.json).
- [Publisher](../publish_ar4.py), which verifies the retained bytes before publication.

The unchanged observation controller previously passed 84 policy tests and the
diagnostic reader passed 12. The request changes only mode, session and output
directory from the successful TF4 request; its native and framework comparison
inputs are the mode-specific AR4 records. Full binary captures remain in the
recorded evidence directory rather than Git. Review records describe explicit
engineering assumptions, not production authorization or discharged GPU proofs.
