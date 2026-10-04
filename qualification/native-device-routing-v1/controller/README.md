# Device Routing CPU Qualification Draft

This separately versioned controller qualifies the corrected worker routing
source generation. The first device-routing run compiled the expected
inventory, but two new CLI gate fixtures failed: their all-zero logits did
not support the supplied winner. The existing wire argmax check correctly
refused those inputs. That failed run and its frozen package remain intact;
they are not a passing qualification and did not execute GPU work.

Source generation V3 corrects the synthetic fixture to use the actual one-hot
winner and tightens its failure assertions. Test names remain unchanged. Its
18 formatted files have been rehashed against the actual successful V4
format/check receipt and their pins are now bound; no new result or worker
hash is predicted. The 31 added Rust tests and 13 controller-policy tests must run
again under this new frozen package. Root owns tests, builds, remote processes,
integration and publication. The previous actual CPU578/4 run remains the
regression baseline; the failed device-routing attempt does not replace it.

## Inputs

Use clean paired archives from pushed Ferric
`82b0fe5850f38c3ff8d2cbe3640a9880d722467e` (tree
`3ab6b3a9bcbc521bbf3b65fa1b4615018c442ce4`) and fe2o3
`9a321f3f98e597a75e8ebeafdda169ec10e12e9e` (tree
`7d7701d5a453f2d6b5f5c336c8e83e5a836b5b84`). The root manifest
`device-routing-source-inputs-v228-v1.json` uses the existing
`ferric-p228-clean-worker-sources-v1` schema. Archive filenames are explicit in
the controller; prefixes remain `ferric/` and `fe2o3/`.

Only worker Rust files may be overlaid. Each `overlay.json` file row contains
`path`, `source`, `before` and `after`; source bodies are staged at
`E/p228-device-routing-source-v3/draft/<repo-relative path>`. Null `before`
means a genuinely new file. Existing preimages are checked before any write.
No fe2o3, parent, Cargo manifest or dependency changes are admitted by this
version. Full extracted and overlaid source maps are retained; all compiled
sources and lockfiles must remain unchanged after Cargo.

`added_worker_tests` is the final sorted, unique, expected compiled test-name
roster for the new device schema, recorder, CLI, routing and failure tests.
It is frozen after source integration and formatting, not inferred from a
successful compilation. The schema and recorder namespaces
are required, and every old worker test name must remain. The complete runtime
inventory must remain exactly the actual CPU578 inventory. Do not remove or
rename old tests to make the extension check pass.

## Pipeline

The exact previous controller and bounded/extraction/metadata/result helpers
are authenticated and reused without editing. The actual prior receipt and
all 128 raw files are rehashed; its 25 commands, environments, natural exits,
process-group reaping, inventories and named results are replayed. Retired
old sources and old target dependencies are not required.

The same 25 phases run against a fresh paired source tree and empty target:
metadata, runtime inventory, all 20 runtime selectors, worker inventory, full
worker library/shared-wire tests, and the worker build. The runtime selections
retain 169 ordinary passes, including typed prefix/MLP timestamp joins, raw
signal and queue tests, mapped atomics, currentness, bank checks and poisoning.

The worker list must equal its 413 historical names plus every frozen new
name. All additions must pass, and the same four old tests must remain ignored.
The library and shared-wire outcomes are checked separately; the latter stays
at 13 passes. Total ordinary passes are derived as `578 + new named tests`
only after raw results match actual compiled inventory. The executable is
selected from the actual Cargo artifact message. No parent is rebuilt.

Keep unchanged offline/locked Cargo, two build/test threads, CPU affinity 8/9,
nice 10, 12 GiB leaf address-space cap, 6 GiB target cap, 40 GiB setup and
38 GiB running disk floors, empty GPU visibility, 64 MiB streams, 120-second
metadata and 1,200-second remaining leaf deadlines. Disclose reuse of the
external Cargo cache; no old target is adopted or automatically deleted.
Sources, input pins, tools, external dependency manifests and the selected
binary are rechecked after all phases, including failed Cargo/test paths.

## Root Handoff

The four files `run.py`, `test_run.py`, `README.md` and the completed
`overlay.json` are frozen under manifest schema
`ferric-p228-device-routing-cpu-package-v1`, using the existing file pin format.
Run the pure tests before the root-owned bounded CPU invocation:

```text
run.py ACTUAL_MANIFEST_SHA device-routing-cpu-v228-vN
```

A real pass writes `ferric-p228-device-routing-cpu-result-v1`; a test, build or
postcheck failure writes `failed.json` and exits nonzero. Early prerequisite
refusal produces no completion. CPU routing and synthetic recorder tests do
not establish mapped GPU execution, calibrated clocks, numerical acceptance,
production authority, overlap or a throughput gain. All such flags stay false.
