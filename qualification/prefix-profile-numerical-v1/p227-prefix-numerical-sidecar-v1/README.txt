P227 Prefix Conditional Numerical Sidecar V1
==========================================
Isolated CPU-only proposal. No tests, controller imports, compilation, live
edits or GPU work were performed by the author. No actual V6 capture/image is
invented. Root must qualify SourceV5 and the portable host/image route first.

Scope
-----
The unchanged parity validation.observation() rechecks the four complete raw
captures, all fourteen rows and typed terminal states, declared Close, input
identity and exact V5/V6 equality. This sidecar then independently checks
attention against the frozen P214 dense FP64 two-pass oracle, and checks O
partials with P215's fixed conditional error bound on the actual accepted
attention bytes. Both profiles and both ranks get their own results.

This does NOT verify the process supervisor, actual GPU execution, allocation
lifetime, input fixture loader, or truth of an operator's source/ISA review.
The enclosing observer retains its original owner/reap/pre/post audits and
immutable readback/initial-cache checks; it is not replaced by this program.
The sidecar adds conditional operator checks, not full prefix/model acceptance.
Its output keeps gpu_execution_verified, full_prefix_acceptance,
full_model_acceptance, runtime_premises_discharged, performance_claim and
production_authority false. No old observer flag or receipt is rewritten.

Inputs and callable API
-----------------------
compare(plan, read, frozen_helpers) accepts a FilePin-to-verified-bytes reader;
the enclosing portable observer may supply its existing retained-body reader.
The provided CLI reader reads exact canonical regular paths, rejects symlinks,
checks stable file identity/extent/SHA and rechecks all inputs before publication.
No arbitrary helper or tolerance is accepted by the CLI: helper source bytes
are hash-pinned, and the frozen attention-policy bytes must equal its helper's
constants. The intended existing numerical environment is NumPy2.2.6.

Closed input JSON fields:
  schema: ferric-p227-prefix-numerical-inputs-v1
  case: one of the existing six genuine/patterned cases
  request, inspection, observation, numerical_review: FilePins
  output_weights: two rank-ordered FilePins
Requests/captures/ordinary six reviews retain their existing exact schemas.
The two16MiB O shards must be the fixed authentic P215 rank0/rank1 SHA values
AND match the observed immutable O-input SHA. This intentionally supports only
the existing layer0 fixture, not arbitrary layers/model revisions.

The additional numerical_review is an explicit engineering prerequisite input,
not an issuer of proof or production authority. Its closed fields are:
  schema: ferric-p227-prefix-numerical-prerequisites-v1; authority:none
  arithmetic_class: bf16-wave64-xor-separate-f32-online-attention-o2048-v1
  baseline_image_sha256; tiles_image_sha256
  source_lineage_review; isa_review: the request's exact corresponding FilePins
  attention_policy_sha256; output_reference_sha256: fixed helper identities
  prerequisites: exact ordered PREREQUISITES list in compare_numerical.py
  reviewed:true; production_authority:false; substantive bounded notes
Root supplies this only after reviewing the actual images' prerequisites.
There is no supplied ready-to-execute review, predicted image or future pin.

Fixed arithmetic, unchanged
--------------------------
P214 accepts exact BF16 words at position0; otherwise at most1 ordered BF16
step OR absolute error<=5e-5*causal per-KV-head max|V|. Its dimensions are
16Q/4KV/head128/context2304. Logical causal history is independently gathered
from the full physical capture using the existing fixed page permutation;
every causal word is finite and future poisoned entries are not read.
P215 uses gamma39(F32)+gamma2048(F64) times an outward absolute-product sum
plus4159 half-subnormal allowances. O.reference() is reused byte-identically;
its separate CPU source-order diagnostic is not the acceptance oracle. The
actual captured FP32 partial is compared against the independent FP64 value
and bound with outward error rounding, and the attention conditioning SHA is
retained. A P214 failure never reaches conditional O acceptance.

Reassociated/split-K/MFMA/token-split attention classes are refused even when
a tiny fixture happens to agree. No P218 MLP bound is transplanted. Future
arithmetic classes need their own justified preregistration. Whole-model
criteria remain a separate implementation task; no threshold is inferred from
four old tokens, a parity PASS, or this conditional numerical result.

Root-owned execution
--------------------
Pure tests (no GPU, small synthetic arrays plus fixed capture geometry):
Exactly18 authored regular tests, none ignored; author-unrun.
  OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 /usr/bin/python3 -B -m unittest \
    discover -s PACKAGE -p test_compare_numerical.py -v
Sidecar:
  OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 /usr/bin/python3 -B \
    PACKAGE/compare_numerical.py PLAN_PATH PLAN_SHA NEW_RESULT_PATH
Use the existing bounded CPU owner, single NumPy thread, 2GiB RSS and bounded
deadline/output. No auto retry, GPU library, subprocess or device open exists
in the sidecar. The result is create-exclusive0600 and at most64KiB. Numerical
refusal publishes no success output; a file-I/O failure may retain an unaccepted
partial result, never overwrite/retry it.

Synthetic wrapper tests explicitly substitute zero O-shard identities and a
mock conditional-O arithmetic function; they do not forge authentic-weight or
GPU evidence. They exercise the real frozen capture verifier and P214 oracle.
Separate pure tests exercise O bound comparison, fixed threshold boundaries,
GQA/page/current/future semantics, identity/refusal, and failed publication.
Actual O reference arithmetic and real capture qualification remain root gates.
