# RoPE Eager-Validity Lowering Refusal

The V2 materialized-RoPE candidate again reached the compiler's existing
semantic SSA partial-move storage limit. No candidate HSACO was emitted or
launched. This records a failed checked-lowering attempt, not GPU qualification.

## Actual Result

The [retained compiler stderr](raw/checked-lowering-stderr) reports:

```text
production semantic SSA partial-move validation for function 0 requires 2097153 storage words, limit is 2097152
```

This is the first over-budget accounting point, not the total storage required
or evidence that raising the limit by one would complete compilation.

Fixture metadata completed with exit 0. Checked lowering completed with Cargo
exit 101; the enclosing owned controller returned exit 1. Both phase process
groups were absent afterward. The full owned tree exited naturally and was
reaped without forced cleanup. Source and input postchecks reported no drift.

See [failed.json](failed.json), [owner/failed.json](owner/failed.json), and the
[publication record](result.json). Checked replay, image emission, inspection,
and native execution did not follow this refusal. Compiler and resource limits
were not changed.

## Candidate and Prior Evidence

The [V2 CPU qualification](../rope-materialized-cpu-v2/README.md) passed 33 Rust
tests: twenty RoPE cases, twelve ordinary reciprocal cases, and one separately
selected exhaustive reciprocal case. The ordinary run's one ignored test is
that same exhaustive case, not an additional pass.

V2 changes only seventeen pure finite-check conjunctions from short-circuit
`&&` to eager boolean `&`. All eighteen predicates and BF16 arithmetic
boundaries remain unchanged. This source experiment did not remove the
compiler refusal seen in the [V1 attempt](../rope-materialized-lowering-attempt-v1/README.md).
The repeated message alone does not identify the underlying state-growth
cause or prove that any other source change would compile.

The [controller sources](controller) and [recipe](recipe.json) retain the
actual V2 compilation recipe. The fourteen pure controller tests passed; their
[record](observations/rope-materialized-lowering-pure-v228-v2.json) is a
primary-agent observation of tool output, explicitly not a remote supervisor
receipt. No V1 malformed-manifest observation is attributed to this attempt.

## Scope

The local publication rehashes the retained failure records, raw streams,
fixture sources and supporting metadata. Large before/after maps and the
courier archive remain outside Git. It does not replay compilation or claim
that all transitive compiler/library bodies were independently rehashed.

Inner failure SHA-256:
`e057e77261c6264aaf290f17616616811361b7df34cf7ef57be0ed87437c3784`.

Owner failure SHA-256:
`77d6e1d012a3b8ff85f8270e67da01c5ce826259cf8f8d39e3f273e381a84199`.

No image selection, production-route change, numerical acceptance, performance
claim, 2,048/256 result, or 700 tokens/s result follows from this checkpoint.
