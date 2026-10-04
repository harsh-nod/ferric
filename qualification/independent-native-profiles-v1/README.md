# Independent Native Profiles

The [host adapter patch](host-adapter.patch) adds two explicit modes to the
existing fe2o3 prefix comparison example:

```text
--inspect-independent-profiles-v1 REQUEST SHA256
--execute-reviewed-engineering-prefix-independent-profiles-v1 REQUEST SHA256
```

This is a CPU-tested engineering adapter, not a GPU numerical result or a
production runtime change. [Source pins](source-pins.json) identify its exact
preimages and proposed bodies. Apply the patch only to that source generation;
it is not a standalone crate.

## Behavior

The adapter runs baseline V5 and candidate V6 in separate, owned child
processes. Each must independently complete, close, preserve immutable inputs
and untouched KV bytes, and produce valid terminal states and finite computed
outputs. Existing request, descriptor, six scoped reviews, executable identity,
child receipt and reaping checks remain unchanged.

The new mode does not require bitwise equality between the two implementations.
It retains both complete captures for the separate
[independent reference adapter](../independent-profile-observation-v1/README.md).
Different finite values are not automatically accepted as correct. The original
paired bitwise mode remains unchanged, and neither route retries a failed child.

## Executed Validation

The [fresh CPU qualification](cpu-result.json) on `mi350-2` records:

- 98 tests passed, including all 14 new regressions; one existing test ignored.
- All 85 historical test names preserved, plus exactly 14 new nonignored tests.
- 15 controller policy tests and the four-phase patch round trip passed.
- Both the test executable and native executable built from a fresh target.
- All six owned phases exited naturally, with no forced cleanup.

Coverage includes CLI routing, finite differences versus old parity rejection,
invalid states, NaN/infinity, both ranks' untouched KV, exact buffer extents,
baseline/candidate failures, uncertain Open/Close, and late postcheck failures.
All four historical real-fixture environment variables were set for the suite.

The copied 5,781-file source generation differs only in the three-file adapter
overlay, yielding 5,783 files. Source, dependency, fixture, configuration, tool
and six prior-target postchecks passed. The 5,856 retained archive members,
including all source files and both selected executables, were rehashed locally.
Prior-target checks use stat inventories, not proof of no transient writes.

The native executable is 11,759,152 bytes, SHA-256
`017e0d3a5c79b1c64a4dccdb0251e53c8e7f8c17ab47e92bbe1309b40760c607`.
It has not yet been deployed or invoked against the GPU. Fresh runtime and
image reviews, owned MI350 execution and independent numerical validation remain
required. No throughput, full-model acceptance or 700 tokens/s result is claimed.
