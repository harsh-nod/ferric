# Independent Profile Numerical CLI

UNFROZEN and unexecuted. This draft adds no launcher, process supervisor, GPU
operation, retry or numerical policy. Its 17 pure wiring tests are authored,
not run. Root owns review, freezing, actual tests, serialization and execution.
No imports, syntax checks or remote commands were performed by the author.

## Prerequisites

Root first freezes and tests `p228-independent-gpu-observation-v1`, then replaces
the explicit `OBSERVER_MANIFEST_SHA = None` and `OBSERVER_TESTS = None` freeze
gates with that actual package SHA and pure-test receipt FilePin. Until then,
the CLI refuses before probing the host. Do not invent either identity. Root
then freezes this package with exactly `run.py`, `test_run.py` and `README.md`.

The new observer must finish its real native leaves and all three before and
three after audits. Root waits for the top-level SSH observer command to exit,
confirms its task-owned descendants are absent/reaped, and only then starts
this separate CPU leaf using the existing bounded harness. No synthetic owner
record or new coordinator is introduced. The observer's `complete.json` alone
does not prove top-level process reaping; the output therefore always retains
`top_level_observer_reaping_verified=false`.

The CPU CLI calls the frozen observer's `P.load` and read-only
`run_case.replay_case(c, receipt_pin)`. That API must replay actual deployment,
runtime/platform and source reviews, all six audits, inspection/native owned
leaf records, child receipts, native closure and complete capture custody.
It has no execute branch. The CLI additionally requires one attempt, zero
retries, no recorded failures and three audits on each side before math.

## Inputs

The pinned input JSON contains exactly:

```text
schema: ferric-p228-independent-profile-numerical-cli-inputs-v1
prepared: actual prepared complete.json FilePin
observation: actual terminal case complete.json FilePin
arithmetic_review: separately root-authored review FilePin
output_weights: two original rank-local O weight FilePins
```

The arithmetic review uses schema
`ferric-p228-independent-profile-arithmetic-assumptions-review-v1`,
`authority=none`, `reviewed=true`, and explicit case/request and both image
identities. It joins the original source-lineage and ISA review pins, the new
artifact review, actual V7 compiler completion and reciprocal CPU receipt, and
the deployment verifier's exact five-member `arithmetic_evidence` map:
`source_entry`, `reciprocal_source`, `llvm`, `isa`, `elf_notes`.
Those are authenticated archive provenance pins; the wrapper never opens their
original build-host paths on MI350.

It also records the exact unchanged `S.PREREQUISITES` and `N.PREREQUISITES`,
prefix/attention policy hashes and O reference hash. Substantive notes and a
nonempty `remaining_limitations` list are required. Every review flag in
`REVIEW_FALSE` must remain false. This review records the engineering scope and
remaining assumptions; it does not fabricate universal sqrt or ordinary
attention-divide guarantees. The old `S.numerical_review`, with its historical
source roster, is deliberately not called or relabelled.

Actual V7 source/LLVM inspection found both normalization sites use the
CPU-tested exact reciprocal integer/RNE path. Ordinary attention divisions are
unchanged. No inspection substitutes for independent GPU numerical checks or
discharges every arithmetic premise.

## Execution Boundary

Use the existing MI350 Python environment, NumPy 2.2.6 and the existing bounded
CPU harness. Required settings are CPU affinity 8,9, nice 10, 2 GiB address
space, 120-second CPU/wall budget, 64 MiB file cap, no core files and one thread
for each BLAS/OpenMP setting listed in `envelope`. All GPU visibility variables
are empty. `PYTHONPATH`, `PYTHONHOME` and optimization are refused. Initial and
ongoing disk floors remain 40/38 GiB. The wrapper does not create a subprocess;
the root-owned harness enforces the wall deadline and retains terminal output.

Logical invocation inside that harness:

```text
E/row-reference-env-v226-mi350-v1/venv/bin/python3 -B \
  E/p228-independent-profile-numerical-cli-v1/run.py \
  INPUT_PATH INPUT_SHA256 prefix-independent-numerical-v228-vN CLI_MANIFEST_SHA256
```

`E` abbreviates `/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`;
the actual command uses absolute paths. All output labels are fresh; there is
no overwrite or automatic retry. A pre-comparison intake failure may leave no
result directory; the bounded caller's actual stderr/exit remains the evidence.

## Unchanged Math

The wrapper authenticates the frozen 23-test adapter package and actual pure
receipt, and the numerical source manifest with its actual 20-test receipt and
transcript. Every listed helper, policy and source fixture is rehashed. It then
calls the unchanged `observe.compare_retained` with frozen `P`, `pins` and
`N.Reader`, preserving both custody ledgers, finite/current-KV checks, complete
capture hashes, original input/weight joins and all existing tolerances.

The adapter checks baseline V5 and tiles V6 independently, including when one
profile fails mathematically. There is no paired-bitwise acceptance condition.
A mathematical rejection remains explicit per-profile data and never triggers
a retry. A custody failure remains fatal. The wrapper rechecks both ledgers
and its original frozen inputs again before publishing.

`complete.json` means the conditional comparison finished, not that numerical
acceptance was granted. Exit status is 0 only when both conditional profile
checks pass; a completed mathematical rejection retains `complete.json` and
both profile outcomes, exits 1, and requires root to stop the case sequence.
Check `conditional_operator_checks_passed` and each profile outcome. All
arithmetic-discharge, independent/full-prefix/full-model
acceptance, production, timing/performance and top-level-reaping flags remain
false. The result records assumptions and remaining limitations alongside the
conditional numerical evidence. No result is evidence for 700 tokens/s.

The authored tests use synthetic pins and mocks for observer/reference calls.
They test refusal, identity joins, assumptions, exact delegation and ledger
failures. They do not claim a real observer replay, reference run or GPU result.
