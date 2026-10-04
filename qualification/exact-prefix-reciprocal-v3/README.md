# Exact Prefix Reciprocal V3

This is a CPU-tested, **unapplied candidate** for the tiled gfx950 Qwen3
prefix. It does not replace a runtime image or establish GPU correctness.
The [source patch](candidate.patch) is relative to Ferric
`9eca257697069f10c87ee9f624016391f6a2e479`; [source pins](source-pins.json)
bind every preimage, new-file absence and resulting body. A temporary-tree
`git apply --check` and full patch round trip passed.

## Smaller Control Flow

The previous candidate, with the idempotent partial-move compiler, reached
semantic capture but contained 1,027 blocks against a 1,024-block limit.
V3 removes the reciprocal's domain and power-of-two branches. It retains
the same 24 restoring-division rounds, nearest-even rounding and admitted
positive-normal interval `[2^-10, 2^64]`.

Unsigned wrapping subtraction and one comparison identify the interval.
A final bit mask returns the exact NaN sentinel outside it. All intermediate
integer calculations are safe even for rejected words; exponent packing
uses wrapping arithmetic before the rejection mask. The denominator is
evaluated once.

For significand `S = 2^23`, the recurrence finishes with remainder `S` and
quotient `2^24 - 1`. Rounding adds one, and result packing carries into the
exponent to produce the exact power-of-two reciprocal. Other admitted
significands take the unchanged recurrence. The kernel body, prefix wrappers,
shared V3 helpers and independent references remain unchanged from V2.

## Executed CPU Checks

The [actual result](cpu-result.json) on `mi350-2` records:

| Check | Result |
| --- | ---: |
| Ordinary arithmetic and refusal tests | 12 passed |
| Complete source contracts | 6 passed |
| Explicit exhaustive one-binade test | 8,388,608 significands passed |
| Independent Fraction nearest-neighbor vectors | 1,007 passed |
| CPU controller policy tests | 11 passed |

The exhaustive test is ignored in the ordinary invocation and was separately
executed. Other tests cover all admitted powers of two and adjacent words,
stratified inputs at every admitted exponent, invalid signs/exponents,
single evaluation and refusal before publication. All nine owned phases
exited naturally; source/dependency checks passed. Fifty raw records and
two test binaries were retained and rehashed locally.

CPU checks use the retained numerical fixture with the local device provider,
not the device crate's unchanged Git dependency. The independent u64 and
Fraction references retain their original implementations. The AST contract
checks source structure; it is not a numerical proof.

## Checked Compiler Result

The [fresh checked probe](checked-probe-result.json) cleared the CFG-size
gate. A matching-decoder [census](semantic-census.json) measured 1,013 stored
blocks, down from 1,027: 14 blocks removed, with 11 below the unchanged cap.
This is a compiler-structure result, not a GPU performance measurement.

Lowering then stopped at a new diagnostic: `a checked induction latch does
not copy field zero of one checked result`. Cargo exited 101; owned processes
exited naturally and were reaped. Source, tool, configuration, dependency and
prior-target checks passed. No ranked/target KIR, handoff or HSACO was emitted.
The probe and supervisor passed 35 and 15 policy tests before execution.

## Remaining Gates

A successor must resolve the loop-latch rejection, complete checked lowering,
replay, inert join, emission and metadata/ISA review. Only then can
a new image undergo independent MI350 numerical validation. Compiler caps,
numerical tolerances and production requirements remain unchanged. No GPU
performance gain, full-model acceptance or 700 tokens/s result is claimed.
