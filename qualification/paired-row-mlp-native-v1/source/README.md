# Down2 Clock-Enabled TF4 Candidate

Source-only successor to frozen `p228-device-clock-gpu-v1`, manifest
`2f8387de7fca50496c5c1cb8f42df3bc294d0633d54a7086ef82f0abdcbbf94c`.
No imports, tests, native execution, or GPU observations were performed by the
author. Root supplied pure-runner SHA256
`5995bcb2ee48f95b9f1c11e8eb9942ecae41e49032fc2fc56d9f9c909f33123b`;
the complete seventeen-file roster must still be frozen and actually tested.

## Exact Change

Only the request's explicit `tiles_image` may change, to the 33,112-byte Down2
image `65a76f917c12476f4814174ee39d515642a643a0352c6609ee514955a6a97449`.
The bootstrap `request.images.mlp` is not this slot and stays byte-identical.
V7 prefix, all bootstrap images, model, prompt, devices, parent, worker, runtime
reviews, currentness policy, dispatch timeouts, and four teacher-forced inputs
stay unchanged. Only session and evidence directory are additionally fresh.

The worker already accepts this separate image in
`resident_layer/mlp_tiles_v2/artifacts.rs::Image::new` and checks its symbol,
11-pointer ABI, 344-byte kernarg, 88 explicit bytes, wave64, static LDS512 and
private0 during `load_decode_kernels`. No native Rust rebuild is required.
The actual Down2 descriptor/ELF reports the same geometry/resources, with
106 VGPRs, 106 SGPRs and zero reported spills. This is static evidence, not a
runtime or performance result. Fixed 512 rounds and the original provider53
remain selected; the separately CPU-tested provider54 is not part of this image.

## Existing Evidence

`down2.py::lowering` authenticates the exact successful 23,895-byte compiler
completion `0ba363b9b4106e5293e4e6152d719c795c2d58f05e062b6a67570f194e0eb68e`,
all nine command/result/stream joins and emitted artifacts. Build-host compiler
generation and CPU prerequisites remain recorded provenance; this reader never
opens compiler binaries/targets or reruns tools on MI350. The original image pin
and transported image pin are distinct fields. All eight emitted runtime
requirements remain undischarged.

The exact clock baseline is
`prefix-device-clock-tf4-shared-full-currentness-gpu-v228-v1/complete.json`,
902,162 bytes, SHA256
`e34189597dc7db7a7c325040f5381932e84390c1a2cf32055830858cb9278ddb`.
Its native/sidecar/Controls/captures and owned leaf records are replayed with the
existing validators, including six historical audits and its CPU553 comparison
lineage. New output comparison then uses these actual clock-baseline captures.

All four complete 606,976-byte payloads, output token trajectory, all 152 typed
tensor rows, and full native forward commands must match exactly. The existing
raw report binds actual selected image hashes to all 288 MLP dispatch rows;
1172 total rows, 16 clock samples, [592,580] packets, Close/reap and six fresh
audits remain mandatory. No tolerance or numerical policy is widened.

## Root Execution Prerequisites

1. Review this successor and execute its complete CPU-only policy suite. Original
   61 tests remain, with narrow new-namespace/context fixture adaptations; 14
   additional Down2 policy tests make the authored census 75. Root must bind
   the actual wrapper SHA and freeze the package before admission.
2. Transport the retained Down2 completion, its nine phase leaves and ten artifact
   files with original relative paths under `E/row-down2-checked-probe-v228-v1`.
   Also provide a separately pinned deployment image. No compiler source/target
   transport is required. Existing clock CPU275/CPU669 executables and runtime
   audit reviews are reused exactly, not relabelled.
3. Preserve the original CPU553 `baseline` plan pin and add `clock_baseline`,
   `down2_lowering`, `down2_image`, `down2_review`. Copy the real clock request and
   change only session, evidence directory and `decode.tiles_image`. Use output
   label `prefix-down2-clock-tf4-shared-full-currentness-gpu-v228-v1`.
4. Root separately authors the new `down2_review` after inspecting the retained
   source/formal/LLVM/ISA/metadata and selected device/coherence/lifecycle facts.
   It binds every supplied artifact, exact new request and unchanged native
   runtime. An updated `decode_review` binds that review pin plus all existing
   prerequisites. Both stay authority-none with all acceptance/performance/
   calibration flags false; this package does not generate a reviewed=true file.
5. Execute one owned native attempt on MI350 with the unchanged
   `--allow-unauthenticated-machine-code --observe-device-clocks` selector and
   existing time/resource bounds. Root must check MI350's independent disk and
   process/device admission conditions first. No ASROCK rebuild is needed.

The sole new result schema is `ferric-p228-down2-clock-gpu-v1`. Failure remains a
failure, with retained artifacts and normal post-audits; no retry or old-result
reinterpretation is introduced. Raw ticks stay uncalibrated and host intervals
remain inclusive. This four-forward engineering test cannot demonstrate the
2,048/256 workload, 700 tokens/s, independent full-model numerical acceptance,
or production authority.
