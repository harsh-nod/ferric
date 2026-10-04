# Projection-Residual AR4 Runtime Audit

Source-only successor of `p228-projection-residual-decode-runtime-v1`.
Twelve synthetic selection/refusal tests are authored, not executed. The actual
CPU1037 qualification is already complete; this package has not audited either
rebuilt executable on MI350.

## Selection

The selector binds CPU completion `7a3c170ca6ffe6000517588280fcdb99cb58c0cbb22cc3f4be910232f7d51c54`
(583,779 bytes), its CPU1022 predecessor, executed controller, identical source
snapshots, all 87 natural outcomes, exact 1,037/4 named accounting, and the
selected artifact in its authentic Cargo build stream. Parent and worker are
separate opt2/debug-assertions/overflow-checks executable artifacts:

- Parent: 13,827,528 bytes, `22d2cb744117af5232e29001f49ff265c25d94f9326ee48a1eb3ddc5aeab6319`.
- Worker: 5,133,800 bytes, `a3079acbd8ff05fbc18503fb7f7a840a4732bc0ab60717b86c03c186198bee7d`.

The prior CPU1022 audit cannot qualify these newly built ELF bodies. No CPU
build, native model execution, source-image review or numerical test is rerun
by this selector.

## Transport And Commands

Let `E=/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
Keep originals and transported files explicitly distinct in any transport
ledger. The minimal route copies these authentic bodies to their original
recorded E paths on MI350; do not rewrite the ASROCK receipt or its Cargo paths:

```text
projection-ar4-cpu-v228-v1/complete.json
projection-ar4-cpu-v228-v1/sources-before.json
projection-ar4-cpu-v228-v1/sources-after.json
projection-ar4-cpu-v228-v1/parent-builds-stdout
projection-ar4-cpu-v228-v1/worker-build-stdout
projection-ar4-cpu-v228-v1/target/parent/debug/ferric-qwen3-finite-projection-residual-decode-engineering
projection-ar4-cpu-v228-v1/target/worker/debug/ferric-tp-peer-finite-engineering-worker-v1
p228-projection-ar4-cpu-v1/run.py
```

For later GPU intake also transport `projection-ar4-cpu-inputs-v228-v1.json`
and `p228-projection-ar4-runtime-v1/source-manifest.json`; the predecessor
CPU1022 receipt and old SiLU prerequisites already have recorded original
paths. This auditor does not need either source tree or all seventeen ELFs.

```text
audit_projection_ar4_decode_runtime.py LABEL ROLE CPU_PATH CPU_SHA BINARY_PATH BINARY_SHA
```

`ROLE` is `parent` or `worker`; `LABEL` is
`projection-ar4-decode-runtime-ROLE-v228-vN`. CPU_PATH must be
`E/projection-ar4-cpu-v228-v1/complete.json`; BINARY_PATH is the role's exact
path above. A deployed alias is intentionally not admitted here. The GPU
intake permits a same-content alias, but its runtime review must then audit
that exact alias path. Use the original paths for this minimal route.

Run on the reviewed MI350 host with UID9661, CPU8/9, nice10, ordinary Python,
GPU visibility variables empty, no PYTHONPATH/PYTHONHOME/PYTHONOPTIMIZE. Limits
remain 2GiB address space, 120 CPU seconds, 64MiB file cap, 40GiB initial and
38GiB ongoing free-space floors. The separate synthetic test command is:

```text
python3 -B test_audit_projection_ar4_decode_runtime.py -v
```

Root runs that command under the same CPU-only bounds and retains its actual
transcript and source hashes. No outcome is assumed by these sources.

## Unchanged Audit

`p227-prefix-runtime-audit-v2/manifest.json` remains
`9b80913aa0dac2da2a52bfe6e053a4db09a5e4c5647731fd269f39719e8bb062`;
its `audit.py` remains
`def16c2f69c082fa273596bef0caf7a2af195d07d2ac6f96d944706d5f44a514`.
Its exact custody reader, topology helper, `frozen_owned.py`, tool paths,
dynamic library closure, topology pre/post checks, process ownership and
bounded `readelf`/`ldd` commands are unchanged. The receipt stays
`ferric-p227-prefix-runtime-audit-v1`, `reviewed:false`, `authority:none` and
`gpu_execution:false`. Root must independently review its actual outputs
before constructing the existing runtime-review records for GPU intake.

No GPU launch, numerical acceptance, clock calibration, performance or
production authority is introduced. The GPU supervisor's six selected-device
audits remain separate from the external current host/process roster check.
