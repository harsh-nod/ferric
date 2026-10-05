# Shared-Host Runtime Audit Adapter

Source-only successor of the qualified CPU855 selector. Fourteen synthetic tests
are authored, not executed by the author. No new CPU result, ELF, audit or native
outcome is implied. The previous deployment/GPU packages remain unchanged.

## Selection Contract

`audit_projection_ar4_shared_host_runtime.py` accepts the actual frozen shared
GPU package manifest SHA and an actual successful CPU receipt SHA. It uses the
package's `shared_cpu.contract`, `sources`, and `test_delta`: scoped 883 passed,
four historical ignores, 63 natural phases, 322 raw identities and four recorded
production artifacts. These are admission requirements, not observed results in
this proposal. The actual855 predecessor and its source-map join remain pinned.
No full GPU intake or image replay runs during this standalone ELF audit.

Three independent selections require three fresh runtime audits:

| Role | CPU target directory | Executable |
| --- | --- | --- |
| `default-parent` | `target/parent/debug` | `ferric-qwen3-finite-projection-residual-decode-host-engineering` |
| `shared-parent` | `target/parent/debug` | `ferric-qwen3-finite-projection-residual-decode-shared-host-engineering` |
| `worker` | `target/worker/debug` | `ferric-tp-peer-finite-engineering-worker-v1` |

Only original paths below `E/projection-ar4-shared-host-cpu-v228-vN` are admitted.
CPU855 binaries, earlier audits, renamed/copied products and cross-role products
are refused. The new plain non-observer parent remains in the CPU artifact ledger
but is not deployed or read by this selector. All three executable identities
come from actual Cargo build records; there are no prospective ELF constants.

## Root Transfer And Invocation

`E=/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
Use the original E paths on MI350, preserving byte hashes and executable modes:

1. This selector/test pair; the entire root-frozen shared GPU package + manifest.
2. Actual new CPU `complete.json`, the four `sources-{base,unformatted,before,after}.json`
   bodies, `worker-build-stdout`, `parent-builds-stdout`, the qualified CPU
   controller `p228-projection-ar4-shared-host-cpu-v1/run.py`, and reviewed
   `p228-projection-ar4-shared-host-v1/source-manifest.json`.
3. The actual855 `complete.json` and `sources-after.json` at original paths.
4. The three selected new ELF bodies above. No compiler products, full source
   trees, weights, old ELF bodies or plain-parent ELF are required by this adapter.
5. Existing frozen old selector (SHA `32e45386b1513457101b670f702df48b3af554d617692468b59a7d05c687c26a`),
   `p227-prefix-runtime-audit-v2` package and its already pinned topology helper.
   Retain its original manifest, helper and ownership implementations unchanged.

For each role, root supplies actual digests and chooses a fresh label:

```sh
timeout 180s taskset -c 8,9 nice -n 10 env -u PYTHONPATH -u PYTHONHOME -u PYTHONOPTIMIZE \
  PYTHONDONTWRITEBYTECODE=1 HIP_VISIBLE_DEVICES= ROCR_VISIBLE_DEVICES= CUDA_VISIBLE_DEVICES= \
  python3 -B "$E/p228-projection-ar4-shared-host-deployment-v1/audit_projection_ar4_shared_host_runtime.py" \
  "$PACKAGE_SHA" "projection-ar4-shared-host-runtime-${ROLE}-v228-v1" "$ROLE" \
  "$ACTUAL_CPU_COMPLETE" "$CPU_SHA" "$ACTUAL_BINARY" "$BINARY_SHA"
```

The existing `audit.execute` is called unchanged: same non-native readelf/ldd
ownership, parser, limits, topology and retained records. The adapter preserves
2 GiB AS, 120 CPU seconds, 64 MiB file cap, UID 9661, CPUs 8/9, nice 10 and the
existing 40 GiB initial floor. Root must independently read the actual three
audit outputs and author two parent reviews plus one worker review. Expected
historical closures were four parent libraries and three worker libraries, but
this proposal does not certify that a future rebuilt ELF has that closure.

Focused tests, for root execution only:

```sh
timeout 120s taskset -c 8,9 nice -n 10 env PYTHONDONTWRITEBYTECODE=1 \
  HIP_VISIBLE_DEVICES= ROCR_VISIBLE_DEVICES= CUDA_VISIBLE_DEVICES= \
  python3 -B "$E/p228-projection-ar4-shared-host-deployment-v1/test_audit_projection_ar4_shared_host_runtime.py" -v
```

The 14 fixtures mock the recorded CPU admission seam; they do not replace the
shared CPU package's source/record tests or any actual CPU/audit qualification.

