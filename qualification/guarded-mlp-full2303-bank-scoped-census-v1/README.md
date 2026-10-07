# Full2303 Bank And Scoped Allocation Census

The explicit full-workload worker now passes [MI350 CPU qualification](cpu-v1/README.md).
Its fifteen tested Rust files are integrated, with a complete
[source postcheck](worker-integration-v1/postcheck.json): 1,283 Ferric bodies,
including all 224 worker bodies, and 823 unchanged canonical runtime bodies.
No runtime change is needed; this reuses the existing closed runtime facades.

The new worker selector is
`--engineering-native-guarded-mlp-full2303-bank-scoped-census-v1`.
It preserves the existing 2,048-token authentic prompt plus 255 own-feedback
forwards, all 256 own-generated outputs, four captures and healthy Close.
The final output is not fed back. Neither reference tokens nor an altered
deadline are introduced. Ordinary Full, scoped Full and Readiness selectors
retain their existing behavior.

After two ordinary first uses, the selected route combines scoped bank rearm,
scoped layer execution and allocation census. The compact policy checks the
full bank/layer order, retired generations, complete output-history hash and
rank-ordered owner assertions. Census counters must be a checked subset of
layer totals, not additional work. Errors and unwind are terminal, without
retry or fallback. Full allocation accounting and local fences remain;
scoped discovery is not temporally equivalent to repeated full discovery.

The prior [40-forward same-binary experiment](../guarded-mlp-scoped-capacity-census-v1/matched-timing-report-v1/README.md)
is measured host-wall evidence only. It does not qualify this longer route.
CPU tests exercise the full schedule synthetically; no native Full2303 run
has been completed. The [parent qualification](parent-cpu-v1/README.md) now
passes all 68 phases and 549 selected tests across 58 scopes in 415.377
seconds. Its six tested parent files are integrated with a complete
1,284-body postcheck, preserving all 224 worker bodies. The parent selector
is `--observe-guarded-full2303-bank-scoped-census-v1`. Strict data admission
now has a [62-test MI350 CPU qualification](admission-cpu-v1/README.md),
preserving all 42 prior comparison cases and adding 20 bank/census cases.
This qualifies the checker, not an actual native full-workload result.

Launch feasibility under the unchanged one-hour bound, exact independent
256 IDs/raw decoded bytes, repeated equal-work performance, GPU overlap and
700 tokens/s remain open. All issue #42 M0-M7 gates remain open.
