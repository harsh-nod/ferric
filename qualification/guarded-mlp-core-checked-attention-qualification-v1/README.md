# Checked Attention Wrapper Qualification

The production changes implement source-origin checks for the ten helpers
observed in the [bounded attention diagnostics](../guarded-mlp-core-checked-attention-diagnostic-v1/README.md).
This checkpoint does not yet qualify attention extraction, guarded gfx950
machine code, GPU execution or model performance. Ferric's production
dependency remains unchanged.

## Implementation

The production checks were introduced in
`a5720f77e52e6212ad35f675504421b8110db9f7`. It adds two private recognizers,
their tests, and insertion-only collector hooks. Prior Result and widening
checks are unchanged.

| Family | Closed Scope | Checks |
| --- | --- | --- |
| Integer | `usize::checked_add`, `usize::checked_div`, `u32::checked_mul`, `u32::overflowing_mul`, `u32::is_multiple_of` | Genuine core identity, exact signatures and full MIR operations, locals, scopes, branches, constants and assertions |
| Option `ok_or` | `usize`, `u32`, or the observed provider tile with genuine `KernelError` | Genuine Option/Result and provider identities, exact payload movement and construction, no-drop error and complete control flow |
| Option `and_then` | `usize` to `usize`, local Fn closure capturing one shared `u32` reference | Exact wrapper body, capture/signature shape and genuine call-once resolution; independent callback and shim checks remain required |

Integer `None` constants use the compiler's actual target layout and an
initialized tag; inactive payload bytes are not read. Recognizing a wrapper
does not bypass callee, inline-origin, local unsafe-source, or ordinary MIR
lowering checks.

The six integer tests check five real positives, fourteen identity/signature
refusals and 458 cloned-MIR mutations; all pass in the second attempt below.
The ten Option tests author five real
positives, fifteen refusals and 198 mutations. These are authored assertions,
not passing Option results. The first attempt ran no tests.

## First MI350 Attempt

The [actual receipt](attempt-v1/evidence/failed.json) records a successful
production compiler build in 40.820 seconds. The subsequent test-binary build
failed in 43.226 seconds with one `E0639` error: the integer mutation fixture
attempted to construct rustc's non-exhaustive `Statement` type directly.
No test suite or extraction control ran, so this attempt reports zero tests
passed. This does not replace the earlier [422-test checkpoint](../guarded-mlp-core-u32-widening-qualification-v1/README.md).

All five executed phases exited naturally and were reaped, with no remaining
process groups. Source and dependency checks were unchanged; integrity
postchecks were clean. The [retention manifest](attempt-v1/retention-manifest.json)
pins 42 original files, including ten compiler source bodies and 29 raw
evidence records. Cargo's original structured diagnostic is retained in
[test-build output](attempt-v1/evidence/compiler-tests-build.stdout).

The reviewed repair clones an actual MIR statement before inserting the Nop
mutation, including when the current block is empty. It changes no production
check, test name or mutation count. The second attempt validates that repair.

## Second MI350 Attempt

Generation `4e8d03cd065fc51b810a20ee9af3d2beb9696438` changes only the integer
fixture's statement construction. Both production and test builds pass,
in 40.394 and 47.163 seconds respectively. The [actual receipt](attempt-v2/evidence/failed.json)
records 417 validated passing tests before the Option cohort fails:

| Scope | Passing Tests |
| --- | ---: |
| Device library and two UI targets | 358 |
| Trusted provider and scalar pipeline | 32 |
| Existing Result and widening wrappers | 21 |
| New checked-integer wrappers | 6 |
| Total | 417 |

The [integer transcript](attempt-v2/evidence/core-checked-integer.stdout)
records six passes, including the real-body mutation and identity assertions.
The [Option transcript](attempt-v2/evidence/core-attention-option.stdout)
records ten failures from one cached fixture failure at the first `ok_usize`
full-authentication assertion. Its preceding signature and MIR-body assertions
pass. Source inspection identifies a missing metadata-observation environment
value required by the independent provider authentication; V2 supplies none.

All 26 executed phases exited naturally and were reaped. Sources and
dependencies remained unchanged, with clean postchecks. The [second retention manifest](attempt-v2/retention-manifest.json)
pins 147 original files and 134 raw evidence records. Atomic, unsafe-source,
matrix and attention extraction controls were not reached. Thus 417 is a
partial-run count, not a regression comparison with the older 422-test run.

The next fixture repair gives the callback an explicit codegen metadata value
and checks the supplied observation against the public derivation from the
actual rustc session. The test runner supplies it only to the Option process,
before its threads start. It changes no production authentication rule and
grants no artifact authority; fresh execution remains pending.

## Qualification Contract

The controller preserves all 36 predecessor commands in order and adds the
two exact new compiler cohorts. Successful completion requires 38 phases and
21 fully validated test scopes, including device/UI, provider, scalar,
Result/widening, eight atomic, two unsafe-source, matrix and attention controls.
All sixteen new names must match the compiled test inventory without ignores.

Builds and tests run on MI350 with GPU visibility disabled, two CPUs and two
Cargo jobs, existing process/resource limits, and pinned sources and tools.
These CPU checks are prerequisites, not gfx950 GPU numerical or throughput
measurements. All issue #42 M0-M7, independent model numerics, sustained BF16
single-request 2,048/256 decode and the 700 tokens/s target remain open.
