# M1 Property Contract

This document defines the evidence obligations for M1 qualification. It is a
requirements input, not evidence that M1 has passed.

No M1 evidence is recorded by this document or manifest. Every M1
implementation obligation remains `Open`.

`proofs/M1_REQUIREMENTS.json` is the canonical machine-readable roster. The
deterministic checker binds it to all 33 unchecked M1 roadmap requirements,
this assurance roster, exact Ferric implementation and reusable fe2o3
compiler/runtime path obligations, and the allowed evidence-profile vocabulary.
A required status is the status that a future qualification must close; it is
never a report of current evidence.

The roster distinguishes the inherited M0 `fe2o3-proof-contracts` authority at
commit `a6fa86b5ccf8f0438925cfec8f48a5d713874da3` from the reviewed M1 upstream
integration base at generic compiler/runtime commit
`d036920d5beffda4d7f9e2465ba4214355dc9b4a`, tree
`18d558abce9b4ad9e336cfee54afc282be2e70e1`. The older M0 pin does not own or
describe future M1 compiler, AQL, KFD, or service-host implementation.

## Target Assurance Roster

<!-- BEGIN M1 ASSURANCE ROSTER -->
| Property | fe2o3 kind | Required status at closure | Current obligation | Exact boundary |
| --- | --- | --- | --- | --- |
| `model_bundle_well_formed` | `Extension:model_bundle_well_formed` | `Proved` | `Open` | Canonical parsing, bounded validation, and authenticated identity compose for the exact target and draft bundle bytes. |
| `operator_refined` | `Extension:operator_refined` | `Proved` | `Open` | Each admitted kernel and schedule refines its declared operator through gpu.*; compiler, object, driver, firmware, and hardware boundaries remain separate. |
| `graph_refined` | `Extension:graph_refined` | `Proved` | `Open` | The exact generated plan and executable runner compose admitted operators into the sequential target-model step under the declared numerical policy. |
| `kv_refined` | `Extension:kv_refined` | `Proved` | `Open` | Initialized generation-owned physical target and draft KV pages refine separate contiguous logical caches across commit, rollback, and retirement. |
| `scheduler_refined` | `Extension:scheduler_refined` | `Proved` | `Open` | Interleaved continuous-batching transitions and exact device completions project to independent legal request transitions. |
| `request_isolated` | `Extension:request_isolated` | `Proved` | `Open` | Request data, KV, workspace, completion, and state effects are noninterfering; timing and resource-contention side channels are excluded. |
| `rollback_refined` | `Extension:rollback_refined` | `Proved` | `Open` | Only a validated accepted prefix becomes reachable and every rejected speculative suffix remains private and is retired safely. |
| `sampler_refined` | `Extension:sampler_refined` | `Proved` | `Open` | M1 deterministic argmax, tie handling, compact completion, and greedy acceptance refine target-only decoding. |
| `distribution_preserved` | `Extension:distribution_preserved` | `Unsupported` | `Open` | Stochastic sampling and stochastic speculative distribution preservation are outside the deterministic greedy M1 envelope. |
| `resource_bounded` | `ResourceBounds` | `Proved` | `Open` | Logical allocation, KV, workspace, queue, loop, and finite-schedule bounds hold for the admitted envelope; machine resources are checked separately. |
| `multi_device_refined` | `Extension:multi_device_refined` | `Unsupported` | `Open` | M1 admits exactly one physical gfx942 device and makes no collective or multi-device refinement claim. |
| `machine_refined` | `Extension:machine_refined` | `Unsupported` | `Open` | The five independent translation validators required by the assurance policy do not exist; source proofs do not establish machine semantics. |
| `memory_safe` | `MemorySafety` | `Proved` | `Open` | Source, kernel-effect, allocation, and runtime models prove admitted bounds and initialization under named compiler, runtime, and hardware contracts. |
| `race_free` | `DataRaceFreedom` | `Proved` | `Open` | Kernel effects, packet publication, and request ownership exclude conflicting admitted accesses at the modeled boundary. |
| `lifetime_safe` | `LeaseSafety` | `Proved` | `Open` | Generation-bound queue, allocation, KV, and completion leases prevent release or reuse before exact quiescence under named external contracts. |
| `artifact_authenticated` | `Extension:artifact_authenticated` | `Validated` | `Open` | An independent canonical validator binds signatures and hashes across bundle, weights, proof inputs, plan, runner, and executable artifacts. |
| `target_conforming` | `Extension:target_conforming` | `Validated` | `Open` | Independent ELF, AMDHSA ABI, resource, ISA, target-feature, and runtime-device validation binds the exact gfx942 xnack- artifact. |
<!-- END M1 ASSURANCE ROSTER -->

The exact statement boundary for each row is in the canonical manifest. Source,
operator, graph, KV, scheduler, isolation, rollback, sampler, resource,
memory-safety, race-freedom, and lifetime claims stop at their named modeled
boundary. Contracted compiler, runtime, driver, firmware, and hardware premises
must be reported separately; they are not promoted by a source theorem.

`artifact_authenticated` and `target_conforming` require independent validators.
Their target `Validated` statuses do not mean those validators exist today.
`distribution_preserved` excludes stochastic sampling from M1, and
`multi_device_refined` excludes multi-device execution. `machine_refined`
remains an `Unsupported` target status until all five independent translation
validators required by `docs/ASSURANCE.md` exist and pass.

## Roadmap Closure

The manifest contains one open record for each M1 roadmap checkbox:

- model and build: `m1.r01` through `m1.r05`;
- Ferric kernel families: `m1.r06` through `m1.r13`;
- runtime: `m1.r14` through `m1.r19`;
- proof and validation: `m1.r20` through `m1.r28`; and
- qualification: `m1.r29` through `m1.r33`.

Every roadmap record resolves to at least one assurance property, evidence
profile, and concrete path obligation. Path obligations name the repository
that owns the missing final-path implementation. A path record may therefore
name future Ferric code or required upstream fe2o3 code; resolution means that
the obligation is unambiguous, not that the file or evidence already exists.

Each path record is explicitly `ExistingFoundation` or `RequiredFuture`.
Existing AQL and KFD foundations are named at their reviewed M1 base paths.
Ferric owns every Qwen kernel-family target under
`crates/ferric-qwen-kernels/src/`. Reusable fe2o3 service-host targets remain
separate `RequiredFuture` records. Resolution does not assert that any future
path exists or implements its obligation.

Ferric's source-level K1-K7 safety foundation validates a caller-supplied
complete finite schedule, buffer roster, initialized half-open ranges, and
modeled access roster. It bounds those rosters before range or pairwise access
scans and returns one non-clone certificate only after resource, memory, and
race checks all pass. It does not derive effects from kernel source, and its
success remains conditional on the named compiler, runtime, target, ABI, TCB,
and hardware premises without granting artifact, GPU, numerical, performance,
or qualification authority.

## Evidence Profiles

Evidence profiles collect independent evidence kinds without treating any one
kind as a substitute for another:

- `admission`: canonical structure, authenticated inputs, negative mutations,
  a TCB, and a Verus theorem;
- `authentication`: canonical structure, exact artifact identity, an independent
  validator, negative mutations, and a TCB;
- `kernel`: Ferric kernel contract first, then generic fe2o3 compiler/runtime
  contracts, theorem, mutation, hardware, validator, performance, identity,
  and TCB records;
- `runtime`: Ferric runtime and generic fe2o3 compiler/runtime contracts plus
  theorem, mutation, hardware, validator, performance, identity, and TCB
  records;
- `composition`: exact identity, contract, theorem, mutation, hardware, and TCB
  records;
- `qualification`: exact identity, hardware, validator, mutation, performance,
  and TCB records; and
- `nonclaim`: an exact unsupported rationale and its TCB.

The checker rejects unknown or duplicate requirements, properties, paths,
profiles, and evidence kinds. It also rejects status weakening, boundary drift,
checked roadmap boxes, noncanonical JSON, any non-`Open` implementation state,
and any `actual_status`, `evidence`, `receipt`, or `satisfaction` field.

Run the requirements and hostile policy checks directly:

```sh
python3 -I proofs/check-m1-requirements.py .
python3 -I proofs/m1-requirements/test-policy.py .
python3 -I proofs/m1/negative/check-registry.py \
  . proofs/m1/negative/REQUIRED_FOUNDATIONS /tmp/m1-active
python3 -I proofs/m1/negative/test-policy.py .
```

## Non-Claims

This scaffold does not authenticate a model, tokenizer, weight section, kernel,
proof, generated runner, executable, benchmark, or machine. It does not run
Verus, fe2o3, HSA, MI300X hardware, vLLM, or SGLang. It creates no
`ContractSetV1`, qualification artifact, receipt, or evidence status. A passing
requirements check proves only that the open M1 obligation roster is canonical,
complete, internally resolved, and consistent with the unchecked roadmap and
this document. The paired M1 positive-theorem and actual-body negative-mutation
registries bind current direct-Verus bodies to selected-proof and hostile-test
requirements. Their speculative composition rows cover atomic failure framing
and publication-to-KV accepted-count agreement. Registry validation, a selected
theorem run, or a fresh pinned-compiler rejection does not implement its
associated future proof path and is not closure evidence.
