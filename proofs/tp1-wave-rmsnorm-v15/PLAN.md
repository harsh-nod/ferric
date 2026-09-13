# Analytical V15 Fixtures

The original fixture-only profile and a9e702e7 core helper remain unchanged.
Separate `native_probe.py` and `run_native.py` now bind the accepted c9 V15
image and a root-controlled native launch. The fixture-only API itself cannot
launch a worker. See the distinct accepted checkpoints below.

Exactly eight cases: uniform and signed families at rows 1, 16, 17 and 32.
All use width 4096, epsilon bits 0x358637bd (1e-6_f32), behavior 0, WG64 and
groups equal to rows. Five buffers follow source order: input, empty residual,
weight, empty writable fused residual, normalized output. Only active extents
are allocated; no inactive 32-row capacity-tail claim. The unchanged core can
eventually check 40 complete buffers and 80 guards, including 16 empty buffers.
The maximum guarded footprint is 533120 bytes.

Uniform input/weights are +1, expected output +1. Signed input magnitude is 1;
its sign is the parity of popcount((row+1)&lane)+popcount(component), with
lane=column%64 and component=column//64. Positive column-weight BF16 bits are
0x3780+column, spanning 2^-16 through 65280 with 4096 distinct normal values.
Expected output is exactly the weight bits with the input sign. Output starts
with finite sentinel 0x55aa. No fixture expected value uses a tolerance.

Every square, local sum 64 and subgroup sum 4096 is exact. Under strict FP32
round-to-nearest-even, stabilized mean is 1+2^-20, rounded sqrt is 1+2^-21,
and rounded reciprocal is 1-2^-21. The final multiplication's displacement is
bounded by 2^-21+2^-24 < 2^-20 relative to the exact BF16 weight, whereas the
nearest BF16 midpoint is at least 2^-9 away. Rational CPU tests check these
midpoint inequalities for every weight without executing sqrt. This is a
conditional arithmetic argument, not device-math/capability emulation: actual
emitted sqrt/divide policy and native outputs remain separately unqualified.
Invalid/trap, underflow, overflow and rounding-boundary native cases are excluded.

Focused CPU gate forecast: 10 unittest methods plus a separate eight-case
self-test, using the exact pinned core file through FERRIC_RMSNORM_HELPER.
The tests cover case/ABI expectations, every signed output, distinct rows and
columns, exact striped reductions, rational rounding margins, active extents,
both empty auxiliaries, malformed specs and semantic mutations, regeneration,
source pinning and a CLI that cannot construct a worker. Host cases do not prove
transactional device traps, buffer guards, ISA semantics, native parity or gain.

Run only on an explicitly authorized bounded fresh mi300x CPU stage. No local
test/build/formatter, installation, compiler/emission or GPU work is included.
Retain exact source/support hashes, raw test/self-test output, actual statuses,
process/resource/source closure and two independent archive streams. Root owns
integration and later image binding/native authorization.

## Accepted CPU Checkpoint

The fresh mi300x CPU gate passed all 10 methods in 0.947 seconds and the separate
eight-case self-test. Both commands and outer runner exited 0; source/support
hashes remained unchanged, both owned groups exited normally, and TMP was empty.
Root accepted the complete archive
7b71c9c98b5673bd47fb3ab19592d708eec5ebfb36e25fb0683d538dd1378905,
matching independent second stream, all 39 file hashes and 42 metadata entries.
Evidence is retained in ferric-wave-rmsnorm-v15-fixture-cpu-v1/RESULT-R1.md.
This is only a fixture/conditional arithmetic CPU checkpoint. No V15 image,
native wrapper, device sqrt/divide behavior, actual guards, model parity or
performance has been admitted by this gate. The generator and tests are unchanged;
this result section is a documentation-only addition after the accepted run.

## Accepted Native Checkpoint

Source `d293b7c` adds the native probe/wrapper and their tests without changing
the generator or core helper. All 30 CPU methods pass on mi300x, plus the
separate eight-case self-test; archive `ba94c2aa` retains the run.

The actual c9-emitted image `68607fc1` from source `f393390` passes all eight
cases on mi350 GPU0. All 40 complete-buffer comparisons and 80 guards pass,
including the empty auxiliary buffers. Normal close and all-eight-idle checks
pass before and after. Result `e6b2919c` and archive `c96ea879` retain the native
run, separately from the initial install-only tar-option failure. Independent
CPU replay `816d5c29`, archive `eddabe7e`, matches all 357 protocol records and
reconstructs the expected payload hashes from the unchanged fixtures.

This qualifies only these finite normal fixture cases. It does not establish
general FP32 equivalence, invalid/trap behavior, underflow, rounding-boundary
behavior, model parity or performance. Raw host elapsed fields are retained but
not reduced into a performance result. The completed CPU, native wrapper/run
and replay stages are removed; the image and worker remain for model checks.
