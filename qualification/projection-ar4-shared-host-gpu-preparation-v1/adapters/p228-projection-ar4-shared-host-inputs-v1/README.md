# Default And Shared AR4 Input Assembly

Source-only successor of the default observer assembler. Eleven synthetic tests
are authored, not executed by the author. No native launch or runtime approval
is performed. Previous input/deployment packages and receipts remain unchanged.

`prepare.py` creates one reviewed arm at a time. Its root-pinned shared GPU package
manifest must have the exact loaded source roster and actual frozen test census.
It authenticates every package body before loading only the five intake aliases,
then restores prior aliases (including absent entries and `None`) on every exit.
The new intake owns actual pure-test, CPU/source/Cargo, deployed ELF, image and
runtime-review admission. This assembler does not invent future qualification
pins or duplicate the paired execution policy.

## Inputs

Root provides two existing JSON files, each with its actual path/SHA. Configuration
has exactly these fields:

```json
{
  "schema": "ferric-p228-projection-ar4-shared-host-assembly-inputs-v1",
  "route": "default",
  "input_label": "prefix-projection-ar4-shared-host-default-inputs-v228-v1",
  "output_label": "prefix-projection-ar4-shared-host-default-gpu-v228-v1",
  "session": "ROOT_SELECTED_NONZERO_64_HEX",
  "cpu": {"path": "ACTUAL_NEW_CPU_COMPLETE", "bytes": 0, "sha256": "ACTUAL_SHA"},
  "supervisor_tests": {"path": "ACTUAL_NEW_PURE_COMPLETE", "bytes": 0, "sha256": "ACTUAL_SHA"},
  "parent_runtime_review": {"path": "FRESH_DEFAULT_PARENT_REVIEW", "bytes": 0, "sha256": "ACTUAL_SHA"},
  "worker_runtime_review": {"path": "FRESH_NEW_WORKER_REVIEW", "bytes": 0, "sha256": "ACTUAL_SHA"}
}
```

The zeros/uppercase text are deliberately invalid placeholders, not FilePins.
For the other arm use `route: "shared"`, matching `-shared-` labels, a distinct
session and the freshly reviewed shared parent. Both arms use the same actual
CPU883 receipt, new worker, package/pure receipt, model/prompt/images, worker
review and historical provenance. CPU receipts must be actual completed new
`projection-ar4-shared-host-cpu-v228-vN` records. Old855 or failed receipts refuse.

Root notes have schema `ferric-p228-projection-ar4-shared-host-root-notes-v1`, the
exact `configuration` object, `reviewed: true`, `authority: "none"`,
`gpu_attempts: 1`, `notes`, and six substantive `review_topics`: `source_lineage`,
`formal`, `isa`, `coherence`, `lifecycle`, `selected_device`. The author does not
provide review text or approval on root's behalf. Each review binds its route.

The template is actual default-observer native receipt 8341d101, not invented new
output. Only request `decode.worker`, `decode.session`, and
`decode.evidence_directory` change. Python JSON preserves GPU u64 IDs exactly.
RoPE 29fd58e7, SiLU b0d1766f, corrected residual 25338bea and original Begin copy,
all weights/prompt/AR recurrence and deadlines stay unchanged. Parent selection
is a plan/CLI distinction, not a request or arithmetic change.

## Root Execution

Place this package and all actual data at original E paths on MI350. Transfer
the CPU/audit readset described in the sibling deployment README first. Existing
intake baseline/image bodies already on MI350 remain prerequisites; the assembly
does not copy compiler trees or execute native code to reconstruct them.

```sh
timeout 600s env -u PYTHONPATH -u PYTHONHOME -u PYTHONOPTIMIZE PYTHONDONTWRITEBYTECODE=1 \
  HIP_VISIBLE_DEVICES= ROCR_VISIBLE_DEVICES= CUDA_VISIBLE_DEVICES= \
  python3 -B "$E/p228-projection-ar4-shared-host-inputs-v1/prepare.py" \
  "$PACKAGE_SHA" "$CONFIG_PATH" "$CONFIG_SHA" "$ROOT_NOTES_PATH" "$ROOT_NOTES_SHA"
```

The existing MI350 identity, CPUs 8/9, nice 10, 2 GiB AS, 300 CPU seconds,
600-second assembly wall guard, 4 MiB file bound and exclusive outputs remain.
Four outputs are `request.json`, `decode-review.json`, `plan.json`, `assembly.json`.
The assembler rechecks every consumed pin before and after writing. It never
creates a GPU case directory, emits a runtime review or grants numerical authority.

After both arms are assembled, root writes the small paired plan with exact
fields `schema`, `output_label`, `default`, `shared`:
`schema = ferric-p228-projection-ar4-shared-host-pair-inputs-v1`,
`output_label = prefix-projection-ar4-shared-host-pair-v228-vN`, and the two actual
arm `plan.json` FilePins. The new GPU package's `pair.plan_shape`, `same_inputs`
and full intake remain authoritative; no second pairing framework is introduced
here. Pair execution is root-owned, fixed default-then-shared, without retries.

Run the 11 synthetic tests separately with `python3 -B test_prepare.py -v` under
the same CPU-only timeout/affinity discipline. They check lossless request edits,
route/generation/refusal boundaries, package alias restoration and explicit
`retain=True` config/notes reads. They are not an actual native admission replay.

Shared configuration time remains outside the zero-baseline seven snapshots.
Any measured comparison must include rank + group + publication full-currentness
counters without adding nested dispatch/read/write timers, and require matching
actual own-input histories. Neither arm is a 2048/256 workload or GPU-time benchmark.

