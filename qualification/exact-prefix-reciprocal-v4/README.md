# Exact Prefix Reciprocal V4

This is a CPU-tested, **unapplied candidate**. It changes only the loop-counter
update in the [V3 reciprocal](../exact-prefix-reciprocal-v3/README.md) and its
exact macro contract. It does not replace a runtime image or establish GPU
correctness. The [source patch](candidate.patch) is relative to Ferric
`9eca257697069f10c87ee9f624016391f6a2e479`; [source pins](source-pins.json)
identify all preimages and resulting bodies. Patch check and round trip passed.

## Canonical Counter Update

The retained V3 semantic capture shows both counter latches as two operations:
`temporary = counter + 1`, then `counter = temporary`. The compiler's induction
recognizer rejects that plain temporary copy. It accepts direct addition or
field zero of an authenticated checked-add result.

V4 replaces `step = step.wrapping_add(1_u32)` with `step += 1_u32`. The loop
advances from zero to 24, so the counter cannot overflow. All other wrapping
arithmetic remains unchanged, including the intentional domain/mask operations
and rejected-input exponent packing. The domain, rounding, kernel body, shared
helpers, rejection/write order and independent references are unchanged.

The V3 capture contains 1,013 blocks against the unchanged 1,024-block limit.
The checked addition may add blocks; a fresh capture must establish its size
and whether the induction form is accepted. This is not a performance claim.

## Executed CPU Checks

The [actual `mi350-2` result](cpu-result.json) records 12 ordinary arithmetic
and refusal tests, six source contracts, the explicitly executed exhaustive
test over 8,388,608 significands, and all 1,007 independent Fraction vectors.
All passed. The CPU controller's 11 policy tests also passed.

The exhaustive test remains ignored in the ordinary invocation and was run
separately with overflow checks enabled. All nine owned commands exited
naturally; source and dependency checks passed. Fifty raw records and both
test binaries were retained and rehashed locally. Tests use the retained CPU
fixture and local provider, not the device crate's unchanged Git dependency.

## Remaining Gates

The [fresh checked probe](checked-probe-result.json) passed metadata but failed
semantic-SSA partial-move validation at cumulative charge 2,097,153 against the
unchanged 2,097,152-word budget. It emitted no semantic capture or HSACO. The
checked counter changes the MIR, so V3's storage and CFG successes do not
transfer to V4. The latch fix is not qualified. All source/tool/dependency and
prior-target postchecks passed; the owned tree exited naturally and was reaped.

Checked lowering, actual replay, inert join, emission and metadata/ISA review
must pass before independent MI350 numerical validation. Compiler limits,
numerical tolerances and runtime admission requirements are unchanged. No GPU,
full-model or 700 tokens/s acceptance is claimed.
