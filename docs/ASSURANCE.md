# Assurance Policy

Ferric reports property-level evidence. It never collapses distinct claims
into one `verified` boolean.

## System Claim

For an admitted bundle, concurrent request set, and completed execution trace:

```text
project_requests(run_ferric(bundle, requests))
  refines
map(run_sequential_target_model, requests)
```

subject to the bundle's numerical policy, while the trace satisfies:

```text
memory_safe
race_free
lifetime_safe
request_isolated
resource_bounded
artifact_authenticated
target_conforming
```

The refinement is decomposed into independently stated properties:

- `model_bundle_well_formed`
- `operator_refined`
- `graph_refined`
- `kv_refined`
- `scheduler_refined`
- `request_isolated`
- `rollback_refined`
- `sampler_refined`
- `distribution_preserved`
- `resource_bounded`
- `multi_device_refined`
- `machine_refined`

## Evidence Statuses

Ferric reuses fe2o3's independent statuses:

| Status | Meaning |
| --- | --- |
| `Proved` | An exact theorem, proof input, model, tool, TCB, and correspondence are bound. |
| `Validated` | An exact independent validator result and its TCB are bound. |
| `Contracted` | Correctness is assumed under a reviewed external contract. |
| `Checked` | A bounded structural or dynamic check ran. |
| `Unsupported` | The property is explicitly unavailable with a recorded rationale. |

There is no status ordering. A memory-safety proof cannot satisfy a functional
refinement obligation, and `Proved` source semantics cannot substitute for a
required `Validated` machine artifact.

## Initial Compiler Boundary

fe2o3 does not currently provide general reviewed source-to-machine
refinement. The honest initial boundary is:

```text
Rust/Verus -> MIR -> algorithm/schedule/tile -> gpu.* : proved or validated
gpu.* -> LLVM -> object -> HSACO                     : contracted compiler TCB
HSACO ABI, resources, and expected ISA shape         : validated or checked
driver, firmware, and hardware                       : contracted
```

Artifact hashes authenticate identity. Disassembly checks constrain machine
shape. Hardware differential tests provide observations. None independently
establishes machine semantic refinement.

A `machine_refined` status requires separate validators for:

1. authenticated MIR to structured-algorithm correspondence;
2. algorithm to schedule/tile refinement;
3. the admitted `gpu.*` subset to LLVM;
4. supported LLVM optimization certificates or equivalent validation; and
5. object/AMDGPU ISA correspondence, including MFMA and math contracts.

## Numerical Policy

Each operator declares one of:

- exact integer or bit-level semantics;
- a concrete IEEE/BF16/FP32 evaluation policy;
- a checked error bound;
- an exact finite probability table; or
- a reviewed external numerical contract.

Token equality under an error bound is conditional on the corresponding logit
margin. Stochastic speculation claims exact distribution preservation only
under a fully specified finite sampler and RNG transition. Otherwise Ferric
reports a bound rather than equality.

## Proof Admission

A proof-required bundle fails closed on:

- missing, stale, timed-out, or weaker-than-required evidence;
- unknown premises or unrecorded trusted functions;
- mismatched source, model, feature, target, schedule, or tool identities;
- an open required property obligation; or
- a proof that ends before the boundary required by policy.

## Non-Claims

Unless separately modeled and qualified, Ferric does not claim protection
against:

- a compromised OS, driver, firmware, or GPU;
- physical, DMA, timing, power, or resource-contention side channels;
- prompt injection or unsafe model behavior;
- malicious but correctly authenticated weights;
- denial of service outside declared resource bounds; or
- arbitrary custom model code, runtime plugins, or dynamically loaded kernels.

