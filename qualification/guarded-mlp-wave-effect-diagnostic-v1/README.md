# Wave Recipe Local-Effect Diagnostic

Engineering evidence for [Ferric issue #42](https://github.com/harsh-nod/ferric/issues/42),
2026-10-08 UTC. This checkpoint adds a precise compiler refusal diagnostic
and a genuine materialized helper reproducer. It does not relax acceptance
or establish GPU correctness, model output, performance, or milestone closure.
All M0-M7 milestones remain open.

## Actual Worker Compilations

Both V16 attempts use the newly qualified backend and the existing
qualified 25-test frontend. They identify the actual failing predicate:

```text
stage wave-replay; root 1; body 1:
wave recipe operation lacks a complete local effect summary:
block <213 fixed / 88 early>; operation 0; kind call
```

The display above condenses two observations; the exact original single-line
errors are at line 67 in [fixed stderr](pair/fixed/compile.stderr) and
[early stderr](pair/early/compile.stderr).

| Actual attempt | Exit | KIR block | Operation | Leaf seconds | Whole seconds | HSACO |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Fixed-round V16 | 1 | 213 | 0 | 120.189408924 | 121.131670768 | None |
| Early-stop V16 | 1 | 88 | 0 | 121.633027558 | 122.572116611 | None |

Both fail before ranked root projection and CFG compaction. Their owned
processes exit naturally, are reaped, and leave no process group, with no
timeout, signals, forced cleanup, storage failure or postcheck error.
These are clean retirements of failed compilations, not successful compiles.
The diagnostic identifies a call/local-effect refusal, not its callee or
the production call's purity. Original semantic/source-map captures are
retained; no typed source-identity join is claimed by these compile probes.

## Source Change

The [three-file patch](correction.patch) is relative to the
[qualified ranked-stage diagnostic](../guarded-mlp-ranked-stage-diagnostic-v1/README.md).
At the existing local-effect preflight in wave recipe replay, the same
operation is charged and the same `has_complete_effect_summary()` predicate
is checked in the same order. Its refusal now carries the numeric Kernel IR
block ID, operation ordinal, and one of three closed kind labels:
`call`, `inline-assembly`, or `other`.

There is no additional IR traversal or callee lookup in the error path, and
no source/body dump, allocation, helper allowlist, budget increase, or
acceptance bypass. The old generic correspondence error and its display
remain for other predicates. The existing outer stage wrapper preserves
this lowerer error through `Error::source()`.

Kernel IR block IDs are not semantic MIR block IDs. These locations alone
must not be interpreted as source coordinates or a decoded callee identity.
No claim that a production call is pure follows from its kind label.

## Genuine Reproducer

The new fixture is admitted semantic MIR, not a manually inserted
unresolved Kernel IR call. It retains the real wave storage owner and input
load, then calls a defined by-value U64 identity helper. Materialization
produces the corresponding executable root and helper.

The test checks the exact owner/root/helper correspondence, actual U64
parameter/result types, the owner's empty-effect helper witness, and an
independent interprocedural effect analysis that finds the helper complete
and pure. Nevertheless, the actual call's local summary is incomplete.
Both direct preflight and public wave recipe replay reject that same
operation with the new diagnostic.

This demonstrates a distinction between transitive helper purity and the
local operation contract. It is not authority to replace one with the
other. The recipe, ranked binding, custody, and inert archive consumers
retain their checks. A generic defined-helper extension would need all
four consumers and shared work/storage bounds to be addressed together.

The focused controls retain unknown-call and inline-assembly refusals,
first-failure ordering, exact and one-short work limits, resource-error
precedence, and storage-floor restoration. All three fixed labels and
maximum numeric IDs have bounded display tests. The genuine fixture is
semantic test evidence, not a Rust source-authenticated GPU artifact.

## CPU Qualification

All authoring, builds, tests and exports ran on `ssh mi350`, host
`smci350-rck-g03-b19-03`. The [complete receipt](cpu/complete.json) records
19 completed phases in 274.766487134 seconds.

| Full suite | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Compiler library | 1,339 | 0 | 24 existing |
| Device library (CPU tests) | 359 | 0 | 0 |
| Semantic lowerer integration | 125 | 0 | 0 |
| Lowerer library | 769 | 0 | 0 |

This is 2,592 passes. The compiler/device/integration rosters and existing
ignored set are unchanged; the lowerer unit roster adds exactly three
nonignored tests. Original stdout is retained for the
[compiler](cpu/compiler-tests.stdout), [device](cpu/device-tests.stdout),
[integration](cpu/lowerer-tests.stdout), and
[lowerer unit](cpu/lowerer-unit-tests.stdout) suites.

The original SSH command handle eventually returned 255 without its final
stdout. Qualification is established by the exact remote terminal receipt
and original phase records, not that local transport exit. All 19 owned
phases exited naturally with status zero, were reaped, and left no owned
process group; sources and tools remained unchanged and postchecks passed.

The final backend is built only after all suites pass. The qualification
binds 5,808 project files, 5,812 inputs, 99 raw CPU evidence files, eight
tool pins, six selected ELF products and 2,247 pinned rust-src files.
Acceptance predicates, helper ABI support, call transport, worker provider,
and runtime behavior are unchanged.

CPU work retains cores 8-9, nice 10, offline Cargo jobs 2, 40 GiB initial /
38 GiB live free-space floors, 12 GiB address-space and 6 GiB cache limits.
The suite retains 3,600-second whole, 1,800-second leaf, 1,200-second leaf CPU,
and 50-second cleanup bounds. Each worker compile retains 600-second wall/CPU
and 50-second cleanup bounds, 64 MiB streams, and 1 GiB file limits.
Outer Cargo job settings do not prove the frontend's inner Cargo concurrency.

## Retained Failed Attempt

The first run failed in the new fixture before materialization:
`InvalidFunctionAbi`. Its initialized U64 scalar used plain ABI attributes,
omitting the required `no_undef` flag. Both argument and return reused that
value. The fresh second generation corrects only this fixture declaration
to `no_undef=true`, with no integer extension or pointee attributes.
Switching admission APIs would not fix the shared ABI validation.

The [failed receipt](failures/wave-effect-v1/failed.json) and
[original stdout](failures/wave-effect-v1/lowerer-unit-tests.stdout) preserve
768 passing lowerer tests and this one failure. The failed generation has
18 completed phases, no qualified backend, and no acceptance credit. Its
production diagnostic source is identical to the passing generation.

[failures/manifest.json](failures/manifest.json) binds the
[failed-attempt archive](failures/wave-effect-v1.tar.gz): 105 members,
104 exact originals, and 25,401,157 expanded bytes. Its export checks the
one-constructor repair byte for byte and preserves all failed raw records.

## Evidence

[manifest.json](manifest.json) binds the source patch, CPU qualification,
paired compile outcomes and [originals.tar.gz](originals.tar.gz). The archive
contains 152 members: 151 exact originals plus their path/pin manifest,
totaling 58,038,856 expanded bytes. It includes source preimages/postimages,
the parent receipt, controllers and input manifest, all CPU raw evidence,
both seven-file worker fixtures, compile records and original diagnostic
captures. It was read back member by member before publication.

| Object | Bytes | SHA-256 |
| --- | ---: | --- |
| CPU receipt | 5,775,773 | `c82f6e900aea71cb5e0b409371534fc70efb6b48b42e6675e52250f04f67a7fe` |
| Backend | 206,474,784 | `c3a841b82ef0aff3e8787e29f3e5d31a1ef77ef8235f83470962169dc5b8490d` |
| Extractor | 205,248 | `1c8342c8c7964c436da49a2303d72cf6c79e4fecfbe910eb1820b1c490676456` |
| Fixed result | 6,452 | `fbb19a79bc1ffeb4bc655be38bc75d2ed04cbf0d74f30e56a1d808266dfa68d5` |
| Early result | 6,452 | `8a0e6d0e6c5f6291db35aa7b8e1b47f3ad439d15c92732961eaa1320b662db41` |
| Patch | 20,180 | `131ed54942d473f8be5149f7553c7da5915f47bcd4380ffb02049fa7ee5871c1` |
| Archive | 8,210,585 | `8afc66b72388e126dbe02742abca48dfd629bd5e369d0166905a414f636cb040` |
| Failed archive | 4,062,278 | `9d6dddf0577652f612464327aac6f96627d398f71f97194ea2511bae01f605c8` |

## Limits And Next Step

The MI350 device-completion hold remains unresolved; no GPU query, reset,
probe, launch, numerical evaluation or benchmark was performed. This worker's
acceptance target is gfx950; a new gfx942 port is not a prerequisite.
Actual source-pinned gfx950 artifact and hardware qualification is still
required. Existing dual-target lowerer LLVM tests remain regression checks,
not either-target GPU acceptance.
No token throughput or 700 tokens/s progress is inferred from CPU timings.

A worker-specific GPU-only inline annotation is a possible follow-up, not
part of this patch. It requires a fresh provider identity, unchanged
coverage/retirement behavior, full qualification and actual authenticated
lowering evidence. Unknown effects must not be accepted to make it compile.
