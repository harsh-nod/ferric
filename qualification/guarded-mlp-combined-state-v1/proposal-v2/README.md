# Combined-State Candidate: Format-Stable Source Assertion

This is a fresh, source-only successor proposal. It keeps the same additive
`device/qwen3-tp-guarded-mlp-segment-kernels-v2` crate, two v2 entry symbols,
single 552-word atomic guard argument, generation contract and 39-test roster.
It changes no compiler, proof predicate, resource ceiling, runtime arithmetic,
atomic ordering, or device ABI. The original v1 proposal and failed CPU capsule
remain immutable.

## Observed Failure

The first actual MI350 CPU attempt produced a failed receipt of 122,686 bytes,
SHA-256 `17ae87ee9eaf720fba25df765fdee2942fb30866a073ff63c4cdedca93a75760`.
Nine phases completed naturally; the last, lib-tests, returned 101. The raw
libtest output reports 38 passes and one failure with no ignored tests. The
controller's `tests` field is null because it refused immediately on that
nonzero child result. Sources remained stable and postchecks were empty.

The failed method was
`combined_state_tests::combined_source_contract_has_one_unsplit_argument_and_literal_store_offsets`.
It searched guard.rs for a one-line Release store. The qualified formatter
correctly rewrote that long macro call to multiple lines, so the literal
substring was absent. All other 38 arithmetic, predicate, generation,
corruption, access-trace, publication-order and marker tests passed in that
failed attempt. Those observations are not a successful CPU qualification.
The pinned raw stdout is 4,741 bytes, SHA-256
`9d3bae3814213917117ba865e774fa0b41a6860c00fb1198e222e60a5dcbfff9`.

## Narrow Repair

All production Rust bodies are copied byte-for-byte from the failed attempt's
actual formatted source snapshot. Only the supplementary source-contract test
changes semantically. No Rust parser dependency is introduced for this test:
its local whitespace normalizer compares the literal spellings of exactly four
store calls from the combined publication macro, in order. The required calls
remain 548/low/Relaxed, 549/high/Relaxed, 551/zero/Relaxed, then
550/verdict/Release. The comparison rejects missing, extra, malformed,
reordered, differently indexed or differently ordered calls. It is expressly
not a Rust parser, production compiler gate, or replacement for the existing
runtime access/order tests.

Within the same named method, source-only regressions cover one-line and
multiline spellings, a changed literal index, a weakened Release ordering,
reordered publication, and an extra prefix store. The existing one-unsplit-
argument, exact extent/generation/launch checks, lane-zero gate, no-slicing,
no-pointer-cast and entry-symbol assertions remain unchanged.

## Exact Integration

The manifest uses the existing source schema, with eight closed materialized
rows. Every before pin authenticates the actual current canonical v2 crate;
no row falsely claims a new addition. Six postimages are unchanged. guard.rs
adds only the already observed rustfmt formatting, and combined_state_tests.rs
contains the observed formatting plus this repair. The patch modifies only
those two files. The original seven-file v1 base and prior compiler-alias
failure remain historical provenance, while new fields explicitly identify
the failed CPU receipt and its formatted source map.

Root must review, integrate and run a fresh complete ten-phase CPU attempt,
then validate managed gfx950 lowering and device execution. This source
proposal reports no successful successor CPU run, HSACO, observed ABI,
numerical acceptance, GPU execution or performance result.

