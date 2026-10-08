# Native Admission CPU Qualification

All 171 selected synthetic tests passed on `mi350` in 88.7957819539588
seconds. This is CPU qualification of the instrumented request's admission,
preparation and evidence handling, not native GPU execution or numerical
acceptance of the model.

| Scope | Passed |
| --- | ---: |
| Unchanged validator tests | 126 |
| Existing admission tests | 14 |
| Additional admission and preparation workflows | 18 |
| Evidence retention workflows | 13 |
| Total | 171 |

The two supervised leaves ran 126 and 45 tests respectively. Both exited
naturally with status zero, were reaped, and had no remaining process group.
There were no failures, errors, skips, timeouts, cleanup signals or postcheck
errors. The raw unittest summaries report 72.406 and 16.019 seconds; these
are not model or GPU timings.

Preparation workflows call the actual preparer with ordinary temporary files.
Mocks cover only process-environment effects such as host identity, affinity,
resource limits and session generation. Retention workflows exercise real
validation and archive retention, including failed and absent outcomes and
tampering. Synthetic one-nanosecond durations are fixture data, not measurements.
The native exporter's host/live-tree entry point remains outside this suite;
its actual post-run execution must be checked separately.

## Original Evidence

The capsule retains 200 original archive members, including the original
manifest with 199 member pins. All 185 staged source/fixture inputs are
unchanged; the evidence includes twelve raw supervision records and both
source inventories. Independent data-only review checked the original archive,
all pins, raw test names, source bodies and clean child retirement.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Original terminal, `evidence/complete.json` | 184539 | `259299e98566e3d9ddd5ad45258722c32ba65034c0a3f725a9d3c19841160d9a` |
| Original exported archive | 4499937 | `a730fa396938997abbbc74109269ebd3f48c8fdb4be861d59ec6d587578f90fe` |

This README is subsequent commentary, not an original manifest member. The
separate [live admission check](../live-admission-cpu-v1/README.md) validates
the real diagnostic worker, parent and checker artifacts. This suite uses
synthetic workflows and does not substitute for that check.

Actual request preparation, final observed deployment bindings, the
instrumented GPU run and its original-evidence validation remain pending.
No new duration breakdown, generated-token result or inference speedup is
claimed.
