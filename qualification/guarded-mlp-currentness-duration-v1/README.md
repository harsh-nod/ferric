# Scoped Currentness Duration Diagnostics

This work measures host overhead in the existing Readiness40 Tail V4 route.
It is optional instrumentation, not a kernel optimization, GPU timing result,
accepted model decode or evidence for the 700 tokens/s target.

The opt-in `engineering-currentness-duration-diagnostics` feature records
call counts and elapsed host nanoseconds for the existing before, discovery,
after and root-generation callbacks. Forty ordered forward rows distinguish
the first two unmeasured forwards from 38 warm forwards. Each warm row has
separate bank, 36-layer aggregate and tail totals.

The bank guarded-body interval includes its callbacks. These nested times
must not be added together as disjoint costs. Counts must reconcile with the
original currentness policy before the armed operation can commit. The
feature does not remove checks, extend deadlines or change arithmetic.

The selected worker publishes the original canonical policy line and a
second bounded diagnostic record after healthy Close. The diagnostic binds
the first line's hash and the original session, worker and transcript. The
parent retains and authenticates the complete original stderr. Feature-off
builds retain the single-record protocol.

## Qualification In Progress

The [first default CPU attempt](default-cpu-failed-v1/README.md) passed the
runtime tests but failed the worker compilation on an attributed assignment.
Its original sources, raw compiler error and failed terminal are retained.
The repair wraps that same assignment in a conditionally compiled block.
The [fresh default retry](default-cpu-attempt-v2/README.md) passed all 27
phases, 1,180 runtime tests and 838 worker tests, with existing ignored tests
unchanged. The [feature-enabled build](diagnostic-cpu-attempt-v2/README.md)
also passed all 27 phases, 1,192 runtime tests and 853 worker tests. Both
builds used identical project sources; the feature adds only the declared
diagnostic tests and instrumentation.

The 22 actual tested runtime and worker files are now
[integrated with a complete source postcheck](worker-integration-v1/README.md):
827 runtime bodies and 1,299 Ferric bodies match the expected generation.
The runtime and worker feature is off by default.

The independent Python checker's [fresh retry](checker-cpu-v2/README.md)
passed all 126 tests. Its [first attempt](checker-cpu-failed-v1/README.md)
reported three positive-fixture errors: generic bank counts did not describe
this diagnostic's fixed workload. The fixture-only repair uses the exact
per-forward counts and validates the positive case before mutation tests.
It does not relax the production validator.

The separate parent builds also passed: [default](default-parent-cpu-v2/README.md)
has 68 phases and 584 selected passes across 58 scopes; the
[diagnostic build](diagnostic-parent-cpu-v2/README.md) has 69 phases and 595
selected passes across 59 scopes. The eleven added cases cover the shared
codec and complete original-stderr parser. Their four actual tested parent
files are [integrated](parent-integration-v1/README.md) with a complete
1,299-Ferric / 827-runtime postcheck, preserving all 236 worker bodies.

The [live CPU-admission check](live-admission-cpu-v1/README.md) also passes
against the actual diagnostic worker, parent and checker originals on MI350.
It invokes the production admission functions without a native launch and
rehashes all 37 inputs; it does not rerun the earlier qualification suites.
The [synthetic native-workflow qualification](native-admission-cpu-v1/README.md)
also passed all 171 tests in 88.796 seconds: the 126 unchanged validator cases,
14 existing admission cases, 18 additional admission/preparation workflows
and 13 evidence-retention workflows. Both supervised leaves retired cleanly;
all original evidence and source identities passed independent data review.

Actual request preparation, final observed deployment bindings, instrumented
GPU execution, original-evidence validation and a measured duration breakdown
remain pending. No result from
this work is substituted for independent numerical acceptance or full-request
feasibility. The [community demo](../../docs/GFX950_COMMUNITY_DEMO_V1.md) still
reports the previously measured short prompt-forward experiments.
