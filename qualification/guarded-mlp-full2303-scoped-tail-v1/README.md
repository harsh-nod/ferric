# Full-Request Bank, Census And Tail Scope

The explicit full-workload worker passes [MI350 CPU qualification](cpu-v1/README.md):
27 phases, 1,180 runtime tests and 838 worker tests, with the existing eight/four
ignores unchanged. Its twelve tested Rust postimages are integrated and
independently checked against the complete canonical source closure.

The worker selector is
`--engineering-native-guarded-mlp-full2303-bank-scoped-census-tail-v1`.
It extends the existing full bank/census route with the same scoped-tail
operation used by the [measured Readiness40 ablation](../guarded-mlp-scoped-tail-v1/matched-timing-report-v2/README.md).
There are still two ordinary first uses, then 2,301 scoped forwards. A forward
must finish bank rearm, all 36 layers and its tail before the next can begin.
An error or unwind permanently terminates the sequence; no fallback is added.

The existing 2,048 authentic prompt tokens, 255 own-output feedback forwards,
256 generated IDs, captures at 0/2047/2048/2302, 144-page layout, healthy Close
and one-hour maximum are unchanged. Tail counters are separate from layer and
census counters, with checked closed-form counts. The new policy explicitly
does not claim temporal equivalence to repeated full discovery. Default routes,
runtime APIs, arithmetic and numerical acceptance are unchanged.

The [parent CPU qualification](parent-cpu-v1/README.md) also passed: 68 phases,
584 selected tests across 58 scopes and seven built executables. Its six tested
Rust files are integrated, with all 1,295 Ferric source bodies checked against
the qualified map. The [admission checker](admission-cpu-v1/README.md) passed
84 synthetic tests, preserving all 62 predecessor cases. Original evidence
and both integration checks received independent data-only review.

Native Full2303 has not run. The short GPU ablation cannot establish
full-length feasibility, independent output agreement, sustained decode or
700 tokens/s. Exact independent 256-ID/raw-decoded-byte acceptance remains open,
as do all issue #42 M0-M7 milestones.
