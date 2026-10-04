# Canonical KIR Work Diagnostic CPU Runner

Source-only controller and 16 authored synthetic tests. No imports, tests, builds or remote commands were executed by the author. Root supplies the actual package manifest SHA and the frozen diagnostic source manifest SHA at execution. This is a diagnostic of an expected refusal, not a compiler qualification, accepted inert join or emitted image.

## Inputs And Ownership

The controller authenticates the actual RPO V2 CPU `56fc51fc...`, owner `afca99d8...`, finalizer tools `39eab92e...` and owner `e919fb52...`. It copies the complete 5,783-file, 82,883,308-byte qualified source tree into a fresh owned source directory. The original source, dependency bodies, all eight selected products, backend aliases and existing targets remain pinned and unchanged. The local Cargo dependency roster is the prior 4,836-file subset, not the entire copied source tree.

Overlay only the three existing Rust bodies in `p228-kir-join-work-diagnostic-v1/source-manifest.json`, SHA `2dc0563b1e09531aed6ced32cc0c9fa839ed7e66215d4072d7a2bd1023806c13`. Its exact preimages, CPU/source generation, unchanged work/storage limits, opt-in flag, no-added-tests declaration and retained handoff join are required. Only these three bodies may change during rustfmt; full source and dependency maps are rechecked afterward.

Fresh case: `E/kir-join-work-diagnostic-cpu-v228-v1`. Fresh owner: `E/kir-join-work-diagnostic-cpu-owner-v228-v1`. Fresh source and target are under the case. Existing owned process-tree execution/reaping and source/configuration/product postchecks are reused. Fixed ASROCK UID 9661, CPUs 8/9, nice 10 and hidden GPUs remain. Bounds stay 12 GiB address space, 1 GiB file size, 6 GiB cache, 64 MiB streams, 40 GiB initial free floor and 38 GiB running free floor. No cleanup is performed by this controller.

## Eight Phases

1. rustfmt, restricted to the three selected bodies.
2. rustfmt-check on the same bodies.
3. Offline locked full Cargo metadata, exactly relocated from the qualified generation.
4. Build only the `finite_join_engineering_hsaco_v1` example test executable via `cargo test --offline --locked --jobs 2 --manifest-path COPY/Cargo.toml -p fe2o3-hsaco-finalize --example finite_join_engineering_hsaco_v1 --no-run --message-format=json`.
5. Its complete test listing.
6. Its ignored-test listing.
7. Its unchanged default suite, expected 190 passes and 15 historical ignores, with no diagnostic flag/output.
8. The selected historical ignored test, with exact old argv tail `--exact linux::wave_qkv_attention_output_tiles_v6::tests::wave_emission_actual_retained_v6_passes_full_inert_join --ignored --show-output --test-threads=1` and diagnostic flag set to `1`.

The actual retained `prefix-tiles.handoff-v3` remains 4,078,537 bytes, SHA `ad4b31ee88efa54702dc8dff13e1e3c331c296734f7cdcfa52e8249ed9fa37fc`. Its PATH/BYTES/SHA environment is unchanged. Only the selected new test ELF, fresh target/temp/loader paths and opt-in flag differ from the original command. The last test must naturally exit 101 with one named failure, no ignores, 204 filtered tests and the unchanged attempted work 1,084,825,160 / limit 1,073,741,824. Timeout, signal, surviving process group, success, unrelated error or changed attempted work refuses the diagnostic result.

## Marker Validation And Claims

Markers must be a nonempty prefix of the frozen 21-stage order, without duplicates or unknown fields. The controller validates integer counters, monotone work/peak, `work + remaining == 1073741824`, storage <= peak <= 134217728, successful bulk before/after deltas, and the recorded shape-charge formula. If the last observed marker is a bulk-before, its accepted work plus pending charge must match the original attempted work. A nested-call marker with zero pending charge localizes only to that observed interval; no internal charge or failure phase is invented.

`passed` and `diagnostic_completed` mean that the controller observed the default suite and expected refusal with intact custody. `actual_capture_join_passed`, `fresh_hsaco_emitted`, `fresh_compiler_built`, GPU execution, numerical acceptance, production authority and performance claim remain false. The finalizer test ELF is instrumented; the compiler, finalizer CLI and metadata tools are not rebuilt or replaced. No next algorithm change or cap adjustment is admitted here.

## Root Invocation

After reviewing/freezing `manifest.json` with the three source bodies, stage them at `E/p228-kir-join-work-diagnostic-cpu-v1` and run the outer controller with the same existing root-owned CPU affinity/nice/environment wrapper:

```text
/usr/bin/python3 -B E/p228-kir-join-work-diagnostic-cpu-v1/run.py PACKAGE_MANIFEST_SHA 2dc0563b1e09531aed6ced32cc0c9fa839ed7e66215d4072d7a2bd1023806c13
```

Root runs the 16 synthetic tests before the build. Their coverage is marker prefix/order/bounds/charge/refusal, source preimages and formatter scope, dependency subset relocation, immutable environment/handoff, and source-manifest generation/scope. Tests do not launch a compiler, subprocess or native workload. No future completion or executable hash is assumed.
