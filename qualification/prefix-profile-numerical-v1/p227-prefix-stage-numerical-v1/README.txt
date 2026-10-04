P227 conditional prefix-stage numerical extension V1
==================================================

Isolated CPU-only proposal. No compiler, subprocess, GPU or host-owner action.
Author status: source/tests authored and statically inspected only;25 pure
test declarations, tests and
retained GPU-capture numerical evaluation are unrun. Root owns execution.

Fixed policy.json was derived from the source before evaluating retained
captures. Existing helpers/reference.py and extract.py are byte-identical P218;
validation.py and compare_numerical.py are exact frozen P227 sidecar helpers.
Only norm4096 and packed QKV K4096 reuse P218. Serial128 headnorm has a separately
counted envelope and split-half RoPE is separately rounded. See AUDIT.txt.

This is a separate conditional result, not a change to any old sidecar receipt.
It requires a new explicit root-reviewed arithmetic-prerequisites JSON binding
the actual baseline/V6 image, source-lineage/ISA reviews, four source digests,
policy SHA and every listed prerequisite. No default reviewed=true template is
emitted. sqrt/divide assumptions are prerequisites, not CPU-test conclusions.

Plan keys, all closed:
  schema: ferric-p227-prefix-stage-inputs-v1
  case: one of the existing six standalone case labels
  request: actual paired-prefix request FilePin
  inspection: actual inspection stdout FilePin
  observation: actual native stdout FilePin
  numerical_review: root-reviewed prefix-stage prerequisites FilePin

Use the exact actual request/inspection/observation pins from the successful
standalone case and existing numerical sidecar plan; do not invent replacements.
The checker derives six original input FilePins per rank from frozen authentic
metadata and the request, then joins all six to the actual native input hashes.
It checks complete capture parity/state with the unchanged validation helper.
Input data are read by FilePin; API callers may provide an authenticated retained
body reader. Only CLI Reader opens original paths. No original model is loaded.

Review schema is ferric-p227-prefix-stage-prerequisites-v1. Exact fields are in
compare_prefix.numerical_review, fixed prerequisite names in PREREQUISITES.
This new result explicitly leaves full_prefix_acceptance/full_model_acceptance,
gpu_execution_verified/runtime_premises_discharged/performance/production false.
It does not independently validate historical KV, attention, O, finalnorm/head,
all36 error propagation or full2048/256. The existing attention/O result remains
an independent prerequisite with its unchanged policy.

Root-only test command, NumPy2.2.6 and single-thread numerical environment:
  python3 -B -m unittest -v test_compare_prefix

Root-only numerical command (same existing bounded CPU-sidecar owner pattern):
  python3 -B compare_prefix.py PLAN_PATH PLAN_SHA NEW_RESULT_PATH

Suggested existing owner envelope remains120s/2GiB/single-thread/64MiB file limit.
This package does not run that owner itself or increase any cap. Output is
exclusive, <=64KiB, after final fresh rehash of every consumed input; a failure
does not publish completion. Retain owner/stdout/stderr separately as before.

The exact source-derived policy/math hashes are closed in compare_prefix.py.
No actual runtime review, result or acceptance is fabricated.
