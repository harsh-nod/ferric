# M1 External Evidence Planner

`planner.py` is the first planning-only slice of the external M1 qualification
orchestrator. It authenticates clean, exact Ferric and fe2o3 source identities,
the admitted 11-package direct pin roster, 25-package resolved pin roster and
16-edge dependency topology, all 39 source paths, the requirements manifest,
and the checker-owned validator registry. It runs the existing source closure
producer once for each repository and creates a new external planning bundle:

```text
python3 -I proofs/m1-qualification/planner.py \
  FERRIC_REPO FE2O3_REPO NEW_OUTPUT_DIR
```

The bundle contains `plan.json`, `missing-work.json`, two exact source closure
records, and their preflight transcripts. The planner allocates the exact
minimum 354 realizable bindings: 168 Roadmap bindings and 186 Assurance
bindings. The extra Assurance binding beyond the unconstrained profile/path
count is required because both `graph_refined` foundation kinds can bind only
`graph-proof`; two flexible kinds must repeat on distinct remaining paths.

The work queue names every expected primary artifact, its producer role, and
whether an in-repository producer exists. The theorem and negative-mutation
runners, the three declaration-only TCB reporters, all 74 artifact-identity
bindings, all 14 canonical-structure bindings, all 15 external-contract
bindings, all 52 fe2o3-contract bindings, all 58 MI300X hardware-test bindings,
all 36 performance-gate bindings, all 44 independent-validator bindings, and
the five exact unsupported-rationale bindings and the shared qualification
receipt intake/finalizer are represented as available commands. The queue
therefore has 358 available producer items and no missing producer. Availability
means that the in-repository production or intake command exists; it does not
mean that the required hardware measurement, performance measurement,
independent-review response, or qualification-run input has been supplied.

An additive r29-specific handoff can authenticate one complete differential run
before the external evidence plan is assembled:

```text
python3 -I proofs/m1-qualification/produce-r29-differential-evidence.py \
  INTAKE-ROOT OUTPUT-BUNDLE
python3 -I proofs/m1/evidence/validate-r29-differential-evidence.py \
  validate INTAKE-ROOT OUTPUT-BUNDLE
```

The fixed intake binds the exact seven-case plan, external policy-review
declaration, Ferric and reference bundles, pairs, comparisons, acceptance,
source/toolchain/target, and TCB identities. This source-paired replay remains
`partial-non-evidence`; it is not an evidence-index artifact, independent
validation, qualification receipt, or r29 closure. The complete schema and
authority boundary are documented in
`proofs/m1/evidence/R29_DIFFERENTIAL_EVIDENCE.md`.

After producing a plan against the final clean source identities, materialize
the three global TCB declarations independently:

```text
for subject in tcb.compiler tcb.hardware tcb.runtime; do
  python3 -I proofs/m1-qualification/produce-tcb-report.py \
    FERRIC_REPO FE2O3_REPO PLAN_DIR "$subject"
done
```

Each invocation reauthenticates the plan/work-queue identity, exact clean
source trees and closures, requirements, path resolutions, and checker-owned
validator registry before publishing one canonical report without replacement.
The producer does not import the trusted TCB validator, emit an evidence index
or receipt, or change an `Open` obligation. Once all three files exist, their
SHA-256 values form the outer TCB roster supplied to every trusted evidence
validator.

After all three TCB declarations exist, materialize one identity-only payload
and report for an exact planner-selected binding:

```text
python3 -I proofs/m1-qualification/produce-artifact-identity.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR binding.NNNNN
```

The binding selects one authenticated path and source repository. The producer
copies that exact stable regular source file as an opaque payload at
`identified-artifacts/artifact.binding.NNNNN.bin`, then publishes the canonical
report without replacement. This source-file snapshot is the producer's
operational convention; the unchanged validator grants only opaque byte
identity and canonical structure, not source provenance or semantic authority.
Each invocation reauthenticates the clean source repositories, exact complete
plan and work queue, source closures, requirements and validator pins, and all
three canonical TCB reports. It leaves every obligation `Open`.

Materialize one of the planner's 14 canonical-structure reports and its typed
companion payload with:

```text
python3 -I proofs/m1-qualification/produce-canonical-structure.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR binding.NNNNN
```

The payload is an exact six-record operational projection of the selected
stable source path: normalized declared availability, source-file SHA-256 and
size, source identity, regular-file observation, and relative path. The
producer holds and revalidates the exact plan, work queue, source closures,
source file, clean repository identities, and three TCB reports while it
publishes the owner-private canonical payload and then its report without
replacement. The report is the completion marker. The trusted validator grants
only `canonical-structure-only` authority: it validates the record encoding and
binding, not the truth or semantics suggested by record names, source code,
kernel behavior, runtime behavior, hardware, or qualification.

Materialize one of the planner's 15 external-contract declarations with:

```text
python3 -I proofs/m1-qualification/produce-external-contract.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR binding.NNNNN
```

The selected binding is an exact `runtime`-profile Roadmap or Assurance slot.
The producer replays the complete source-pinned plan, holds and revalidates the
plan, work queue, both source closures, clean source repository identities, and
all three TCB reports, then publishes one owner-private report without
replacement. It independently projects only the validator's four fixed
compiler, runtime, driver/firmware, and hardware assumptions. It does not invoke
the trusted validator and grants only `declared-assumptions-only` authority: the
report does not establish implementation, satisfaction, external review,
machine refinement, load, launch, hardware, performance, or qualification.

Materialize one of the planner's 52 fe2o3-contract declarations with:

```text
python3 -I proofs/m1-qualification/produce-fe2o3-contract.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR binding.NNNNN
```

The selected binding must be one of the exact 52 `fe2o3-contract` slots. The
producer publishes the Ferric-defined JSON projection at the binding-owned
locations, without replacement and in this order:

```text
contracts/<artifact-id>.fe2o3-contract-body.json
contract-sets/<artifact-id>.fe2o3-contract-set.json
artifacts/<artifact-id>.fe2o3-contract.json
```

The report publishes last as the completion marker. A failed transaction rolls
back only the exact files created by that invocation, preserves a substituted
inode, and reports the rollback failure. The producer grants only
`contract-declaration-structure-only` authority, leaves every obligation
`Open`, and neither invokes nor changes the trusted validator or its
checker-owned source pin.

The authenticated fe2o3 source establishes the Rust contract structs and
`ContractSetV1::validate_closed()`. The producer and trusted Python validator
do not instantiate those structs or execute fe2o3 Rust; they emit and validate
only the Ferric-defined JSON projection.
`ContractSetV1::validate_closed-structural-only` is a descriptive Ferric label,
not an upstream symbol. The resulting declaration makes no implementation,
semantic, proof, machine behavior or refinement, load, launch, hardware,
performance, or qualification claim.

Import one externally measured suite for any of the 36 exact performance-gate
bindings with:

```text
python3 -I proofs/m1-qualification/produce-performance-report.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR PERFORMANCE_INTAKE binding.NNNNN
```

`PERFORMANCE_INTAKE` is one owner-private canonical JSON file under an
owner-private `0700` parent outside both repositories and `PLAN_DIR`. The
external harness owns every environment, artifact, model, workload, baseline,
and sample value. The Ferric producer does not synthesize, default, repair,
reorder, or discard those values. It recomputes the fixed V1 arithmetic,
authenticates the current source/TCB/plan context, then publishes the immutable
measurement roster before the report.
The report is the completion marker and grants only `checked-performance-only`
authority. The unchanged trusted validator must still be invoked separately;
neither producer nor validator attests that declared samples were physically
observed.

Export the exact 44 independent-review requests and 440 case inputs with:

```text
python3 -I proofs/m1-qualification/produce-independent-validator.py \
  export-all FERRIC_REPO FE2O3_REPO PLAN_DIR HANDOFF_DIR
```

`HANDOFF_DIR` must be a new canonical path under an existing owner-private
`0700` parent. This handoff creates no evidence artifact. After a real outside
organization returns the exact response/checker-material package in an
owner-private `0700` response root, ingest one binding with:

```text
python3 -I proofs/m1-qualification/produce-independent-validator.py \
  intake FERRIC_REPO FE2O3_REPO PLAN_DIR \
  INDEPENDENT_REVIEW_ROOT binding.NNNNN
```

Intake never executes, imports, or builds the returned checker. It
descriptor-authenticates the response, reconstructs the canonical roster and
transcript, then publishes roster, transcript, and report in that order. The
report is the completion marker and grants only
`independent-validation-observation-only` authority. The V1 identity and
attestation checks reject declared self-validation but contain no external
signature or trust root and cannot establish social independence.

Materialize one of the planner's 58 MI300X hardware observations with the exact
Ferric harness, persisted kernel-artifact root, and a canonical hardware
environment input:

```text
python3 -I proofs/m1-qualification/produce-hardware-transcript.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR \
  HARDWARE_HARNESS KERNEL_ARTIFACTS HARDWARE_ENVIRONMENT binding.NNNNN
```

`HARDWARE_HARNESS` must name the executable
`ferric-m1-hardware-harness` and match the reviewed byte length and SHA-256
stored in `hardware-k7-procedure.json`; a merely same-named executable is
rejected before invocation. `HARDWARE_ENVIRONMENT` is canonical JSON in
format `FERRIC-M1-HARDWARE-ENVIRONMENT-V1`; it records the exact KFD GPU unique
ID, its canonically derived AMD SMI UUID, the PCI BDF, and the
MI300X/gfx942/XNACK identity. Its ROCm, amdgpu-module, and firmware fields are
operator-declared identities cross-checked against the harness result; they are
not independent environment attestation. The checked-in
`hardware-k7-procedure.json` fixes the singleton request/result schema and one
K7 speculative-token-assembly launch.

Each invocation authenticates the exact 58-binding allowlist, complete plan and
queue, both source closures and repositories, three TCB reports, harness bytes,
kernel tree, procedure, and hardware environment input. It calls the harness
once, requires one completed and read-back-verified K7 launch for that binding,
and cross-checks the returned device and environment before publishing exactly:

```text
hardware-rosters/<artifact-id>.json
hardware-transcripts/<artifact-id>.json
artifacts/<artifact-id>.hardware-transcript.json
```

The transcript retains the semantic kernel-manifest and program-catalog
identities and the complete binding-local K7 observation. Its tool record binds
the held harness binary, harness-emitted package version, and immutable hashes
of the five named Ferric harness/runtime sources embedded when that harness was
built. The producer independently hashes the held source files and rejects a
stale harness whose embedded roster differs. These source hashes establish a
reviewable source association; they do not prove a reproducible build or that
the binary was produced from those sources.

The roster and transcript publish first and the report publishes last as the
completion marker; a failed transaction attempts every owned-file cleanup,
removes only exact files it created, and preserves a replacement inode while
reporting the rollback failure. The producer never invokes the trusted
validator, emits no index or receipt, leaves every obligation `Open`, and grants only
`hardware-observation-only` authority. The K7 observation does not establish
path-specific semantics, machine refinement, performance, or qualification.
The producer policy uses a deterministic synthetic harness once per binding;
its 58 binding-local invocations are not claims of 58 physical GPU launches.

The three M1 properties whose required closure status is `Unsupported` have
five exact path-bound nonclaim reports. Materialize one selected report with:

```text
python3 -I proofs/m1-qualification/produce-unsupported-rationale.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR binding.NNNNN
```

The selected binding must be one of the planner's five `unsupported-rationale`
slots. The producer independently replays the complete source-pinned plan,
reauthenticates the exact clean source closures and all three canonical TCB
reports, and projects only the validator's fixed rationale, reason code, and
excluded-claim roster. It publishes one owner-private canonical report without
replacement and grants only `nonclaim-only` authority. It emits no companion
payload, evidence index, receipt, positive validation result, or status closure.

After every binding artifact and all three TCB reports exist, export the exact
pre-receipt candidate into a new owner-private qualification-run root:

```text
python3 -I proofs/m1-qualification/produce-qualification-receipt.py \
  prepare-candidate FERRIC_REPO FE2O3_REPO PLAN_DIR QUALIFICATION_RUN_ROOT
```

The export creates only `candidate-index.json`; it emits no receipt or closure.
The external qualification orchestrator adds one canonical `intake.json` that
names the candidate, run UUID, MI300X environment, and absolute Cargo and Rustc
executables. It does not assert gate results or supply command/output hashes.
The finalizer executes the source-pinned pre-receipt protocol in
`proofs/check-m1-evidence-index.py` itself for `evidence-index`, `hardware`,
`performance`, `proof`, `quality`, `source-closure`, and `validators`. It hashes
the exact checker and Python executable, executes and verifies every gate's
canonical PASS output, and derives every artifact/binding roster from the
candidate instead of accepting caller-supplied rosters. Cargo and Rustc are
hashed and executed with `--version`; the running Python binary is measured
directly instead of being named by intake. This dedicated protocol passes both
the exported candidate path and the canonical plan directory as its artifact
root, so candidate custody stays external while evidence bytes remain in the
held plan tree.

Finalize only after the external run root is complete:

```text
python3 -I proofs/m1-qualification/produce-qualification-receipt.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR QUALIFICATION_RUN_ROOT
```

The finalizer independently replays the planner, reauthenticates the clean
sources, source closures, every artifact, TCB report, validator source, measured
tool binary, and candidate index before and after publication, and retains exact
directory custody throughout the transaction. It first publishes the
`completed-work.json` transition that binds all 358 queue items and the seven
executed gates, then the qualification transcript, receipt, and finally
`evidence-index.json` as the commit marker. Every create is no-replace and every
rollback is inode-identity checked. The finalizer invokes trusted validators only
through the source-pinned pre-receipt checker and prints no closure claim. The
authoritative final step remains a separate command:

```text
python3 -I proofs/check-m1-evidence-index.py \
  FERRIC_REPO PLAN_DIR/evidence-index.json FE2O3_REPO
```

All planning and individual evidence-production commands retain
`planning-only-no-evidence` or narrower authority and change no `Open` in-repo
M1 obligation.

Record one completed protected Worker V3 build and its inert load-readiness
publication with:

```text
python3 -I proofs/m1-qualification/produce-protected-worker-v3-build.py \
  FERRIC_SOURCE_REPO FE2O3_COMPILER_REPO PRODUCTION_CONFIG \
  ARTIFACT_ROOT CARGO_FE2O3 RUSTC_WRAPPER CODEGEN_BACKEND NEW_RECORD
```

The producer requires exact clean Git worktrees, a canonical production
configuration, the complete singleton Worker V3 output roster, valid claim and
readiness checksums and cross-links, compiler images matching the authenticated
closure, and successful descriptive inspection of the held finalized HSACO.
It publishes one canonical owner-private record without replacement. Absolute
build and worker paths are deliberately omitted from the record; the exact
configuration bytes remain bound by SHA-256, while the stable unit-relative
path and immutable worker identity fields are projected for review.

This is an observational progress record only. Its authority is limited to the
observed protected compilation, Worker V3 HSACO finalization, and inert
load-envelope publication. It does not establish verifier authority, GPU load
or dispatch, numerical correctness, performance, Qwen execution, or M1
qualification; it emits no evidence index or qualification receipt and closes
no M1 obligation. Validate the pinned record and its producer policy with:

```text
python3 -I proofs/m1/evidence/validate-protected-worker-v3-build.py \
  proofs/m1/evidence/PROTECTED_WORKER_V3_SWIGLU_BUILD.json
python3 -I proofs/m1/evidence/test-protected-worker-v3-build-policy.py
python3 -I \
  proofs/m1-qualification/test-protected-worker-v3-build-producer-policy.py
```

The aggregate 12-kernel production build is prepared and launched by one
Ferric-owned, fail-closed entrypoint. Start from exact clean Ferric and fe2o3
worktrees, an owner-private output directory, and a measured Worker V3 linker
whose directory also contains `fe2o3-llvm-build-id.txt` and
`fe2o3-worker-build-id.txt`:

```text
python3 -I -B \
  proofs/m1-qualification/produce-protected-worker-v3-all-kernels-release.py \
  prepare-config FERRIC_SOURCE_REPO WORKER NEW_PRODUCTION_CONFIG
```

This publishes compact canonical `fe2o3-production-build-config-v2` bytes for
the single aggregate compilation unit, `source-isa-summary-v1`, COV6, O2,
debug stripping, per-stage verification, no providers, fixed execution limits,
and the exact measured Worker V3 identity. It prints the config SHA-256 and the
independently computed fe2o3 transitive V2 config identity.

Run the protected build from a new owner-private target path with:

```text
python3 -I -B \
  proofs/m1-qualification/produce-protected-worker-v3-all-kernels-release.py \
  build FERRIC_SOURCE_REPO FE2O3_SOURCE_REPO PRODUCTION_CONFIG NEW_TARGET_DIR \
  CARGO_FE2O3 CARGO CARGO_BINDING_TRAMPOLINE RUSTC \
  RUSTC_RUNTIME_TREE_SHA256 CODEGEN_BACKEND
```

The command admits only public fe2o3 main revision
`16da71edd823e0d5c16529bfbbedb4f9dd8e70c6`, verifies both aggregate Cargo
pins and the lockfile, reconstructs the exact protected-release environment,
and invokes `cargo-fe2o3 authority release build --locked`. It fails before
spawning Cargo unless the fixed root-owned compiler client profile at
`/etc/fe2o3/compiler-execution/client-profile-v1` and the protected supervisor
socket at `/run/fe2o3/compiler-execution-supervisor.sock` are present. Missing
infrastructure is a deployment blocker; it must not be substituted with a
local or unprotected compiler path.

After the successful build, produce the observational receipt and noncurrent,
owner-private selection candidate with:

```text
python3 -I -B \
  proofs/m1-qualification/produce-protected-worker-v3-all-kernels-release.py \
  produce-candidate FERRIC_SOURCE_REPO FE2O3_SOURCE_REPO PRODUCTION_CONFIG \
  TARGET_DIR/fe2o3 CARGO_FE2O3 RUSTC_WRAPPER CODEGEN_BACKEND \
  SOURCE_PIN_ADAPTER NEW_OBSERVATIONAL_RECORD NEW_SELECTION_CANDIDATE
```

The orchestration delegates to the existing strict aggregate build-record
producer, validator, and publication-selection producer. It publishes neither
output by replacement and prints both byte identities on success. Validate the
observational build record independently with:

```text
python3 -I -B \
  proofs/m1-qualification/validate-protected-worker-v3-all-kernels-build.py \
  OBSERVATIONAL_BUILD_RECORD
python3 -I -B \
  proofs/m1-qualification/produce-protected-worker-v3-all-kernels-publication-selection.py \
  FERRIC_SOURCE_REPO FE2O3_SOURCE_REPO OBSERVATIONAL_BUILD_RECORD NEW_CANDIDATE
python3 -I -B \
  proofs/m1-qualification/test-protected-worker-v3-all-kernels-publication-selection-policy.py
python3 -I -B \
  proofs/m1-qualification/test-protected-worker-v3-all-kernels-release-policy.py
```

When the protected compiler service is unavailable, the same entrypoint can
produce an explicitly non-authoritative engineering HSACO without contacting
that service. First prepare a disposable, canonical `cargo vendor` tree for
the exact public fe2o3 checkout. This installs only the reviewed
`fe2o3-device` manifest and its required workspace overlay into that disposable
tree:

```text
python3 -I -B \
  proofs/m1-qualification/produce-protected-worker-v3-all-kernels-release.py \
  prepare-engineering-vendor FERRIC_SOURCE_REPO FE2O3_SOURCE_REPO CARGO_VENDOR
```

Then run the authority-free engineering build with:

```text
python3 -I -B \
  proofs/m1-qualification/produce-protected-worker-v3-all-kernels-release.py \
  engineering-hsaco FERRIC_SOURCE_REPO FE2O3_SOURCE_REPO \
  OWNER_PRIVATE_PARENT/fe2o3-engineering-v1 CARGO_FE2O3 RUSTC_EXTRACTOR \
  EXTRACTOR_BACKEND WORKER CARGO RUSTC CLANG LLD LLD_ARGV_PROXY CARGO_VENDOR
```

The fixed command selects `ferric_qwen3_all_kernels_device_v1`,
`gfx942:xnack-`, COV6, no providers, the exact public fe2o3 Git source, and the
aggregate manifest in the clean Ferric worktree. It admits only an observation
whose HSACO metadata names the exact 12-kernel set and whose publication,
load, and launch grants are all false. This path is useful for descriptive
inspection and development smoke tests only. It produces no protected Worker
V3 receipt or publication authority and cannot substitute for the protected
build above.

The candidate resolves every claimed device, adapter, and adapter-binding row
through the clean Git object bytes, then binds the record bytes and exact
Ferric, fe2o3, and manifest-selected provider source identities,
`gfx942:xnack-`, COV6, the exact ordered 12-symbol roster, all six
source-pin coordinates, the finalized HSACO identity, and the receipt-bound
Worker V3 envelope identity. It is published without replacement at mode 0600.
It is not a current publication, verifier result, runtime selection, load or
launch authority, Qwen execution record, M1 evidence, or gate closure. The
engine's private current aggregate selection remains `None` and has no public,
environment, CLI, or file override.

Run the focused hostile policy with:

```text
python3 -I proofs/m1-qualification/test-policy.py FERRIC_REPO FE2O3_OBJECT_REPO
python3 -I proofs/m1-qualification/test-tcb-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
python3 -I proofs/m1-qualification/test-artifact-identity-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
python3 -I proofs/m1-qualification/test-canonical-structure-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
python3 -I proofs/m1-qualification/test-external-contract-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
python3 -I proofs/m1-qualification/test-fe2o3-contract-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
python3 -I proofs/m1-qualification/test-performance-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
python3 -I proofs/m1-qualification/test-independent-validator-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
python3 -I proofs/m1-qualification/test-qualification-receipt-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
python3 -I proofs/m1-qualification/test-unsupported-rationale-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
```

`FE2O3_OBJECT_REPO` may have any checked-out branch, but its local Git object
store must contain the exact revision pinned by `FERRIC_REPO`. The policy uses
disposable shared clones and leaves both supplied repositories unchanged.
