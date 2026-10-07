# Checked-Load Grouping Follow-up

Status: source inspection only, 2026-10-06. No implementation, emission or
performance gain is claimed. Complete the split-K4 gate/up full-model comparison
before starting another measured optimization.

At published fe2o3 `da6b561c5a3f12acc5b0e6da74c808273e728710`,
`lower_strided_read_view_load_or` in `production_semantic_kir_v1.rs` still emits
logical bounds, overflow checks and physical bounds for each scalar read.
`GuardedLoad` lowering in `fe2o3-amdgcn-model/src/lowering.rs` emits a separate
branch/load/phi for each read. Ferric V20 already expresses eight operand loads
before arithmetic, but retained ISA does not establish multiple operand pairs
outstanding. Adding accumulators or endpoint facts alone is not evidence of
load overlap.

The next narrow compiler candidate is an all-valid fast path for a bounded
group of independent, nonvolatile, global read-only guarded loads. Conjoin the
original predicates, emit the loads in one straight-line block when all hold,
and retain the original individually guarded sequence and fallbacks otherwise.
Reconverge before arithmetic or collectives. Do not cross side effects, semantic
markers, barriers, unsafe-to-move address calculations, or dependencies on an
earlier loaded value. Preserve scalar lowering for every ineligible region.

Core implementation and compiler tests belong in fe2o3. Kernel source,
component fixtures and model integration belong in Ferric. Required checks:

- Mixed masks, invalid pointers behind false guards, empty inputs, distinct
  fallbacks, volatile/dependent-load rejection, loop/phi exits and collective
  convergence in compiler tests.
- Unchanged product/add/finite-check order and inactive tails in Ferric.
- Actual ISA showing multiple loads before the first wait/use, with no spills.
- Guarded component parity, raw ticks and host timing for O (`K4096`) and down
  (`K12288`), then same-worker full-model token parity and TTFT/TPOT.

Increased live-register pressure remains a risk. This does not reduce weight
volume or dispatch count. Ordered64 raw ticks must not be presented as native32
wall-time shares, and no numerical full-model gain is predicted.

The source-only audit at `42acca0a37bc2b422b1d52a4d6a6bc7ad87f292d`
narrows implementation to AMDGCN `emit_body` and the scalar guarded-load emitter.
Adjacent-load-only grouping would miss V20: generated bounds/address operations
separate its loads. A bounded planner must whitelist only independent total
operations and preserve the final merge label used by successor PHIs. Initially
refuse active semantic-anchor and ordered-debug routes rather than moving their
markers. MFMA grouping is a separate extension because each guarded U16 load is
immediately followed by a BF16 bitcast; a scalar-only implementation must not be
presented as a prefill optimization.

A fresh read-only audit at `4b55998ed4ee8aefa38511607db745957bc7fccd`
finds no subsequent guarded-load grouping change. AMDGCN lowering is identical
to `42acca0a`; the guarded-load arm is identical to `da6b561c`. New private-scalar
uniformity and expanded-source accounting work does not establish global-load
overlap. The nine runtime crates and their 43 transitive lock records still
match the qualified da6b runtime exactly; whole-compiler equivalence is not claimed.
