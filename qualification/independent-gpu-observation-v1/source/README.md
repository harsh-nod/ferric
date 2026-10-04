# Independent Native Profile GPU Observation

Unfrozen isolated successor to `p227-prefix-parity-observation-v4`. No tests,
native execution, deployment, or GPU qualification are claimed by this draft.
The authored test inventory is 81: prepare 18, run 15, validation 15, child
custody 10, and the byte-identical independent adapter's 23 policy tests.
Root must freeze the package and retain actual passing test evidence before
preparation. Root has bound the deployment V2 package, reader and controller
identities after 30 passing MI350 policy tests and successful actual archive
export and receiver verification. A separate hermetic in-process test driver
checks this package's 18 files before and after its 81 policy tests. A synthetic
test pass alone does not authorize deployment or replace scoped runtime reviews.

## Scope

This controller runs the new independent parent selectors:

```
--inspect-independent-profiles-v1
--execute-reviewed-engineering-prefix-independent-profiles-v1
```

The native parent still executes baseline-v5 first, then tiles-v6, using the
unchanged profile-child mode and ten sidecars. `validation.py` and
`child_evidence.py` remain byte-identical to V4. `observe.py` and
`test_observe.py` remain byte-identical to the frozen independent adapter
manifest `4c49bffced2b40b52547000f0d1b1efeabd569fc59abc1a6a1cd94b7d462952e`.
The original V4 controllers are retained as preimages, not imported.

Only the new native independent inspection/observation schemas are accepted.
A successful GPU case means that both profiles report native Close, both
rank pairs have complete authenticated captures, immutable input readbacks
match, every typed terminal state word passes the existing checks, and all
owned leaf/audit scopes terminate naturally. It does **not** require or claim
paired bitwise parity, numerical acceptance, full-prefix correctness,
production authority, performance, or 700 tokens/s.

Finite checks cover the existing computed stages and current KV slots. The
entire capture, including historical/untouched KV, is hashed and retained.
Native `validate_run` remains responsible for immutable/untouched memory
checks; the observer does not manufacture an independent full-history proof.

## Inputs and Provenance

`prepare.py` consumes a pinned JSON with the exact fields:

```
schema deployment deployment_tests deployment_test_source artifact_review
platform_review runtime_review observer_manifest observer_tests
observer_test_sources matrix_label cases topology_helper
```

The schema is `ferric-p228-independent-gpu-inputs-v1`. Every artifact/review
field is a FilePin (`path`, `bytes`, `sha256`). The matrix label is a fresh
`prefix-independent-profile-gpu-v228-vN`. `cases` retains the exact ordered
six cases and original baseline-request pins:

1. `genuine-pos0`
2. `genuine-pos4`
3. `patterned-pos15`
4. `patterned-pos16`
5. `patterned-pos2047`
6. `patterned-pos2048`

Each case contains `case`, `baseline_request`, and six ordered actual review
pins. The original 334-file, 88,974,677-byte baseline closure is unchanged.
New source-lineage, ISA, and remaining scoped reviews must bind the actual
new image, not merely reuse the historical SourceV5 review. The request ABI,
symbol, geometry, timeout, capture format and review-kind order are unchanged.

Deployment is a separate verified prerequisite. Its reader API is
`verify(D, pins, value, directory) -> (value, verified)`. The reader opens
transported archives and files only; original build-host paths are provenance,
not filesystem accesses. It must qualify the actual checked-probe V7 image
and the actual independent-native 98-test binary, with exact compiler
generation, both overlays, source rosters, products and owner receipts.
There is no fallback to a previous image or binary.

The deployment pure receipt has schema
`ferric-p228-independent-deployment-pure-v1` and closed fields `schema`,
`passed`, `tests`, `package_manifest`, `controller`, `test_source`,
`transcript`, `gpu_execution`, `numerical_acceptance`. `test_source` is the
exact deployed `test_portable.py` pin, not a source-snapshot alias. The
observer authenticates the package/test/controller/transcript pins and
requires the actual passing count and false GPU/numerical claims.

The root-authored artifact review schema is
`ferric-p228-independent-gpu-artifact-review-v1`; see `reviewed_artifact` for
its closed fields. It joins original and relocated image/binary pins,
compiler/native completions and owners, candidate CPU evidence, native
overlay, exact metadata and all six case reviews. Explicit notes are required.
Runtime and platform reviews keep their old closed schemas and checks, but
must name the newly deployed executable and current host boot/libraries.
`p228-independent-runtime-audit-v1` supports the new deployment namespace.

Prepared and case receipts expose `compiler_complete`, `compiler_owner`,
`compiler_generation`, `candidate_cpu_receipt`, `candidate_cpu_sources`,
`original_object`, `native_complete`, `native_owner`,
`native_compiler_generation`, `native_overlay`, `native_sources`,
`original_binary`, `binary`, `object`, `qualifications`, `compiler_artifacts`,
`compiler_phase_records`, `candidate_sources`, and `arithmetic_evidence`.
These refer to the new qualification, not historical SourceV5 labels.

## Bounded Execution

Remote evidence root:

```
/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220
```

Preparation uses `INPUT_PATH SHA OUTPUT_LABEL`, where the output label is
fresh `prefix-independent-prepared-v228-vN`. The case controller uses
`PREPARED_PATH SHA INDEX` with exactly one index `0` through `5`. Both entry
points require the fixed reviewed MI350 hostname, UID 9661, CPUs 8 and 9,
and controller nice 0. The child environment remains the frozen empty GPU
visibility environment; native KFD selection uses the two exact unique IDs.

The root must retain/reap the top-level controller and serialize cases. Each
case is fresh and one-shot; failure directories cannot be retried. Prior
cases are replayed before a later case can start. No unrelated process is
stopped or granted an audit exception.

Preserved limits and lifecycle:

- Separate owned inspection, native, and audit trees; pidfd/subreaper helpers
  and emergency cleanup are unchanged.
- Native/inspection leaves: 180 seconds, nice 10, two CPUs, 12 GiB address
  space, 8 MiB stream/file cap, disabled core dumps.
- Exactly three pre-audits and three post-audits, each audit capped at 30
  seconds. All three post-audits are attempted in `finally`, including after
  native or custody failure.
- Case deadline 2400 seconds; 32 MiB evidence tree, 256 entries, 40/38 GiB
  initial/ongoing free-space floors, reserved post-audit time unchanged.
- One native attempt, zero retries. Forced cleanup, nonzero exit, missing
  reaping/group absence, changed inputs, or nonempty stderr cannot pass.

`bounded`, resource/ownership helpers, runtime reviews, baseline loader,
guard and native request construction are unchanged; authored AST regression
tests compare them against the retained preimages.

## Separate Numerical Replay

The GPU `complete.json` schema is
`ferric-p228-independent-gpu-observation-v1`. `passed` certifies only the
bounded closed-capture observation. Every numerical/production/performance
authority field remains false. GPU execution never calls `compare_retained`.

`run_case.replay_case(c, receipt_pin)` is read-only. `c` comes from
`prepare.load(prepared_pin)`. It rechecks preparation/new-generation pins,
all eight owned leaf envelopes and retained files, the six distinct ordered
device/process audits, both parent reports, ten child sidecars and all four
complete captures. It returns `receipt`, `receipt_pin`, `request` (parsed),
`baseline`, `case_directory`, `request_pin`, `binary`, `inspection_result`,
and `native_result`. It has no launch path and makes no numerical claim.

Only after the GPU controller has exited and root has reaped its owned
processes may a **separate bounded CPU leaf** add the two original output
weight pins and invoke the frozen independent `observe.compare_retained`.
That consumer must bind the actual source/arithmetic reviews, unchanged
reference policies and numerical-package test receipts. It checks baseline
and candidate independently with fixed bounds. It cannot repurpose the GPU
cleanup/post-audit time budget for reference calculations.

## Remaining Gates

The author has not imported these modules or executed their tests. Root must
review/freeze the package, run all 81 pure tests, retain exact source-snapshot
and test receipts, complete actual deployment replay, refresh new-binary
runtime and image/source/ISA reviews, then prepare and run six fresh cases.
Actual file-backed GPU replay and later numerical acceptance remain separate
required integration checks. Synthetic policy tests are not those results.
