# V14 Native Campaign Launcher

Engineering-only comparison of the default grouped token backend against the
explicit native whole-program backend. Both arms use the same newly built
worker, model files and nine frozen kernel images. No benchmark or serving
qualification is granted by this launcher.

The immutable campaign plan predeclares exactly 14 cells:

1. `counter-A`, `counter-B`: one correct 128/128 request per instrumented arm.
2. `block1-A1`, `block1-B1`, `block1-B2`, `block1-A2`.
3. `block2-A1`, `block2-B1`, `block2-B2`, `block2-A2`.
4. `block3-A1`, `block3-B1`, `block3-B2`, `block3-A2`.

Each latency cell contains two excluded warmups and four measured requests.
The controller budget is 135 model batches for counters and 810 for latency.
The token program remains 652 dispatches and 180 updates per decode token.

## Preparation

Run only on `mi350` (`smci350-rck-g03-b19-03`, UID 9661), with physical device
16366993098680759275 idle. Preparation, all cells and retention must remain in
one persistent SSH session: the host may remove user tmpfs files when its last
session closes. The root integration process supplies actual, remotely built
binary hashes and clean bounded CPU-suite receipts, not historical worker hashes.

```sh
/usr/bin/python3 -I -B /path/to/prepare_stage.py \
  --stage /dev/shm/ferric-v14-native-40695ec-a001 \
  --base-root /path/to/base \
  --v19-root /path/to/v19-native \
  --kv-image /path/to/v19-image \
  --measurement-root /path/to/perf-v14-measurement-r1 \
  --build-manifest /path/to/build-binding.json \
  --build-manifest-sha256 ACTUAL_SHA256
```

The stage must not exist. Preparation copies only the explicitly hashed inputs,
source archives, helper files, binaries and CPU receipts; model files remain
borrowed. It returns the immutable `plan.json` SHA256 and ordered cell IDs.
Failed preparation retains a refusal and removes only its newly created root.

`FerricV14NativeBuildBindingV1` contains `runtime_main`, `runtime_source`,
`controller_source`, `worker`, `controllers` (`A`, `B`, `counters`) and
`cpu_qualification`. File bindings have exactly `path` and `sha256`.
The legacy-named `runtime_main` field identifies the baseline base revision
`40695ecf5ffd7f3ec2bca8fd138a89cb675b57c6`, not today's upstream tip. The core
change was published on fe2o3 main as
`eded9474bff22aeb8a942061f3e956c234a0ce7f`; its final source archive SHA256 is
`5087f055a8368123bd62a947e27b86a0e28c3ad961e4905335193b609af84eb9`.
`FerricV14CpuQualificationV1` binds those exact sources and binaries, all eight
guarded CPU phases, the four launch sources and the four measurement sources
(`native_token_cell.py`, `abba_ledger.py`, `gpu_activity.py`,
`native_campaign_replay.py`). This is engineering custody, not source authenticity.

## Sequential Execution

Use the returned SHA256 unchanged for every invocation:

```sh
STAGE=/dev/shm/ferric-v14-native-40695ec-a001
PLAN_SHA=ACTUAL_PREPARATION_RESULT
for CELL in counter-A counter-B \
  block1-A1 block1-B1 block1-B2 block1-A2 \
  block2-A1 block2-B1 block2-B2 block2-A2 \
  block3-A1 block3-B1 block3-B2 block3-A2
do
  /usr/bin/python3 -I -B "$STAGE/run_stage.py" \
    --plan "$STAGE/plan.json" --plan-sha256 "$PLAN_SHA" \
    --cell-id "$CELL" --supervise || exit "$?"
done
/usr/bin/python3 -I -B "$STAGE/run_stage.py" \
  --plan "$STAGE/plan.json" --plan-sha256 "$PLAN_SHA" --summarize
```

Inputs and argv paths stay fixed. All output is create-only beneath
`cells/<cell-id>`. The global stage lock prevents simultaneous launches. Before
launching a cell, the driver requires every preceding cell to have clean final
supervision and independently replays its retained raw transcripts. A failed or
existing cell cannot be overwritten; start a new complete campaign instead.

The unchanged two-GiB cap includes all inputs and all 14 output directories.
The 64-GiB root-space floor, 128-GiB available-memory floor and added 32-GiB tmpfs
floor are never reduced. Tmpfs must be executable. Utilization, KFD sysfs,
all-UID fuser and complete root-owned descriptor scans all remain required.
Bounded retries retain every refusal and refresh the entire ownership sample.
Only the controller's own process group and its unreaped wrapper are cleaned up;
discovered GPU users are never signaled.

`campaign-report.json` contains the replayed ABBA decision and separate per-cell
CPU costs. TTFT/TPOT use uninstrumented request timing only; CPU totals include
excluded warmups and are not GPU time. Retain the complete stage and its hashes
before ending the persistent SSH session, then remove only the owned, retired
stage. No performance gain is claimed until the complete campaign passes.
