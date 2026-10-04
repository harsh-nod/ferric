# Clock V2 TF4 Observation Qualification Package

This separately versioned successor of `p228-device-timing-gpu-v2` adds the explicit clock
parent route. No files in the predecessor are changed. The author has not
imported these modules, run their tests, launched a native process or executed a
GPU case. Root binds the package and bounded test runner for qualification;
only actual subsequent receipts can establish passing tests or GPU execution.

## What Changes

- The parent binary is
  `ferric-qwen3-finite-prefix-decode-device-clock-engineering`, with the required
  `--observe-device-clocks` argument. It selects the worker's separately versioned
  `--engineering-native-prefix-decode-device-clock-v2` mode; there is no V1 fallback.
- The closed request is `FerricFinitePrefixDecodeDeviceClockRequestV2`. The actual
  parent diagnostic and `native-device-clock-v2.json` must carry their exact V2
  schemas, selected identities and false authority flags.
- The sidecar contains the unchanged raw report plus exactly 16 KFD samples, in
  forward, pre/post and rank order. Every sample joins the actual group, rank,
  unique ID and queue epoch; the KFD GPU ID and system-clock frequency must remain
  stable for the same device. Host sampling brackets are ordered and nonoverlapping.
- `raw_validation.py` is the prior raw validator with a narrow `report_value`
  extraction. The new validator calls that function on the parsed, authenticated
  embedded `raw` object. It neither fabricates nor serializes a legacy receipt.
  All 1,172 raw packet rows, actual Control intervals, image identities and
  completion joins are still checked.

Zero or decreasing sampled GPU/CPU/system counters are accepted as raw values.
The system frequency is not assumed to be a GPU frequency. No conversion to
nanoseconds, dispatch-clock-domain equivalence, cross-device alignment, overlap,
or speedup follows from these samples. Existing packet completion ticks retain
their original nonzero/ordered checks. Whole-model correctness, independent
numerical acceptance, production authority and the 700 tokens/s target remain open.

## What Stays Fixed

The existing CPU553/V7 TF4 baseline, six prefix cases and their conditional
numerical receipts, model/weights/images, full 152 tensor comparisons, four exact
606,976-byte captures, and four tokens are unchanged. Only the qualified parent,
qualified worker, fresh session and evidence paths change. The complete new
sidecar is charged to the existing 8 MiB native evidence budget.

The supervisor keeps the original 4,000-second native leaf, 30-second audit leaves,
4,300-second case deadline, 40/38 GiB free-space floors, address-space and stream
bounds, reserved cleanup/post-audit path, pidfd ownership and reap checks. One
attempt runs three pre-audits, one native leaf and three post-audits. Failure
retention, all post-audits, Close and owned descendant reaping remain mandatory;
there is no retry or cleanup shortcut. The five legacy import bindings and their
exception-safe restoration are unchanged.

## Qualification Gates

The worker is bound to the actual 669-pass/4-ignore CPU receipt
`41ddcf8a9cb48b970d4f9a4187de6f6de1fd0506ba098d524e82d17335030f1d`
(213,924 bytes), and its 5,018,856-byte executable
`d2af909e7fef88af5b20f4ae8f2a5779a7aee93a187169e97a939bec7d3a0fed`.
This is CPU qualification, not an actual clock-sampling result.

The actual 275-pass/0-ignore parent receipt is now locally retained and bound:
`d2a118dd2a3bfac749b18de26883a661a1078a22ebf374853a11b081f43b1484`
(316,284 bytes). Its selected 13,771,584-byte clock-parent executable is
`8d8c8786b159ad959a6fa9ec6635915dfe2fde8214ce52a9254bf463c64aa2ca`.
The retained completion, source before/after maps, build stream and selected ELF
were read and rehashed before these pins were added; no new test was run here.

`TEST_RUNNER_SHA` binds the root-authored bounded CPU-only runner. Admission
requires the exact package manifest and actual passing pure-test receipt, not
just that source pin. Root must retain those results. The parent receipt must report
that the sibling runtime was not rebuilt: it retains its locked Git runtime
dependency, even though both source archives identify the supplied runtime tip.
Shared worker source aliases must match the actual qualified worker subtree.

Fresh runtime audits and root-reviewed records for these exact transported
executables are still required. The predecessor runtime-audit wrapper is not
silently reused with new paths. Root must author a separately versioned audit
selection before a GPU attempt, and review the engineering assumptions without
granting runtime, arithmetic or clock-domain authority.

## Authored Tests

The source declares 61 unexecuted tests: 17 retained raw-validator tests, 19 intake
tests, 13 supervisor tests and 12 clock-validator tests. The new cases cover all
16 sample identities/order, strict integer types, frequency/GPU-ID stability,
host brackets, malformed/legacy/oversized reports, forbidden claims, unchanged
raw failures, exact parent wrapper/projection and whole-sidecar accounting.
Fixtures and ownership calls are synthetic; these tests cannot establish actual
GPU support or clock equivalence. Root must execute the real discovered inventory
under the existing bounded CPU harness before freezing qualification results.

The five copied helper bodies retain their predecessor bytes. The 15-file package
roster is declared in `intake.PACKAGE_FILES`; the companion manifest binds it.
