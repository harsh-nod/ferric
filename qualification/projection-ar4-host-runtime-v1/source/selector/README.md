# Observer AR4 Deployment Recipe

Source-only selector and deployment recipe. No deployment, imported code,
tests, audit, request assembly or GPU execution was performed by the author.
Root owns execution and review. The selector and fourteen synthetic tests
are now authored here. The assembler successor remains to be authored;
its proposed command below is not an existing tool.

## Actual Inputs

Let `E=/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`,
`C=$E/projection-ar4-host-observation-cpu-v228-v1`, and
`P=$E/p228-projection-ar4-host-observation-gpu-v1` on MI350.

The locally rehashed actual CPU completion is `$C/complete.json`, 488734 B,
`7d8c08eeffab9cbebbf6abc973c0ba93d80616ad81063b35587e74d04df1d59c`.
It records 61 natural successful phases, scoped 855 passes/four ignores,
and three production products. Historical runtime208 was not rerun.
Deploy only these two actual selected ELF bodies at their original paths:

| Role | Path Relative To C | Bytes | SHA256 |
| --- | --- | ---: | --- |
| Parent | target/parent/debug/ferric-qwen3-finite-projection-residual-decode-host-engineering | 13938936 | 1fad2d8aafe9ce4e799322d42d1015756ead228bf2f237c302ee260fecb656fd |
| Worker | target/worker/debug/ferric-tp-peer-finite-engineering-worker-v1 | 5159512 | 14135b08c276ba9d38fcbae635fe14c3db8bd11b934ccf5ac8d51e7c2876bd75 |

The plain parent is not selected and need not be transferred. Preserve
original identities; do not deploy old CPU1037 executable bodies under new
names. Fresh regular executable copies with one link avoid Cargo hard-link
reader differences. Check exact bytes/SHA on MI350 after transfer.

New CPU admission reads nine non-ELF bodies, all at original E paths:

- C/complete.json.
- E/p228-projection-ar4-host-observation-cpu-v1/run.py, 12898f6b9aa1a1af122cf61d0393089fa38cdb898097114dbb67a36ea4fc780a.
- E/p228-projection-ar4-host-observation-v1/source-manifest.json, ab5d055a09c320780a0d0da7d235bb0fe023469c9132b37a877fbc7841738074.
- C/sources-base.json, C/sources-unformatted.json, C/sources-before.json, C/sources-after.json.
- C/worker-build-stdout and C/parent-builds-stdout.

The completion carries exact pins for all nine; new source before/after
digest is 1d173129c684afe5bcdc009ec939f8e4f0371bad8a4edecdf62b4548cb04f920.
No target tree, third ELF, Rust source tree or full 312-body raw collection
is required by observer_cpu.evidence. The existing authentic CPU1037 and
older baseline/image prerequisite trees remain necessary, unchanged.

Frozen GPU manifest: 8ce9861812a27d115e68edb7f2fb0a3f6fc5aaf3ee045bd70313356aa9d333b1.
Actual retained pure91 completion is
E/projection-ar4-host-observation-gpu-pure-v228-v1/complete.json,
14852452f3a5111a1982575090ce4a3c96c4d6b1cfddda0539f64c317bbd8a80.
It reports passed=true/tests=91 and identical source-before/after pins.
Keep its four receipt bodies and the wrapper's original external path;
no repeat pure run is required solely to reuse this unchanged package.

## Fresh ELF Audits

Reuse unchanged `p227-prefix-runtime-audit-v2/audit.py::execute`:

- Package manifest 9b80913aa0dac2da2a52bfe6e053a4db09a5e4c5647731fd269f39719e8bb062.
- audit.py def16c2f69c082fa273596bef0caf7a2af195d07d2ac6f96d944706d5f44a514.
- run_row_facts_v2.py 244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820.
- frozen_owned.py ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583.
- E's sibling resident-output-tp2-v217/run_p217_mi350.py topology helper 6016c30f46aa32abf9d175f329a0110e86cbf79d1363f1803dba3c86c1bad50a.

The existing selector `p228-projection-ar4-decode-runtime-v1/audit_projection_ar4_decode_runtime.py`
(72898b3024ec502a1a2cc7dbcbfd6b133fb6d37f9beeb170c6f2faa366db308a)
is the minimal template. Its CLI hardcodes CPU1037 and the old parent, and
the generic audit.py CLI hardcodes an older deployment basename. Neither
CLI accepts the new artifacts unchanged. The new selector here adapts only
selection/namespace: it authenticates actual CPU7d8c, calls frozen
observer_cpu.contract, binds its actual Cargo products plus controller,
proposal, four source maps and both build streams, and keeps the original
bootstrap/execute/ownership/limits code. It verifies the complete frozen
GPU package before loading only layer_validation and observer_cpu, restoring
the previous module alias on success or failure. Do not call inherited
intake.qualified_cpu for this new receipt: that API intentionally means
the CPU1037 predecessor.

Selector CLI: `LABEL ROLE CPU_PATH CPU_SHA BINARY_PATH BINARY_SHA`.
Run each role separately as UID9661, CPU8/9, nice10, Python -B, hidden GPUs,
2 GiB AS/120 CPU seconds/64 MiB file cap, with the existing 40/38 GiB floors.
Use fresh labels `projection-ar4-host-observation-runtime-{parent,worker}-v228-v1`.
The unchanged execute() runs only bounded `readelf -d ELF` and `ldd ELF`
(30 seconds each), retains owned records/topology/software, rehashes tools
and resolved libraries and checks host/boot stability. No native launch.

Root then authors two actual compatibility reviews using unchanged schema
`ferric-p227-prefix-parity-runtime-review-v1`: current host/boot, selected
binary pin, actual readelf/ldd audit pins, exact resolved library rows,
substantive notes, reviewed=true, authority=none, gpu_execution=false and
production_authority=false. Preserve all 17 files of each actual audit
tree. Old ELF review records cannot be relabeled.

This selector reads no target product other than the selected role's ELF.
It does not replace the later full intake.evidence predecessor/test-delta
and image admission replay. It grants no review or native authority.

## Selector Transfer And Tests

Install this proposal's selector, test file and README into fresh
E/p228-projection-ar4-host-observation-deployment-v1, alongside the already
installed frozen GPU package. No other new helper body is needed. Reuse
the entire existing pinned auditor package and topology helper unchanged.
Copy the nine CPU metadata bodies and two ELF bodies listed above to their
original E paths. The pure tests load and hash-check exactly the frozen
layer_validation.py and observer_cpu.py from the sibling GPU package; they
never launch audit.py, read an actual ELF or run a subprocess.

Fourteen authored methods cover both real-shaped selected products through
the real frozen contract, wrong generation/role/plain parent, observed CPU
and ELF pins, scoped counts/worker split, source transition and formatting,
Cargo streams, binary headers/mode, strict FilePins, and alias restoration.
The synthetic fixture substitutes its own known pins explicitly and is not
presented as a qualification result. Root may run the bounded test command:

```sh
DEPLOY="$E/p228-projection-ar4-host-observation-deployment-v1"
env -u PYTHONPATH -u PYTHONHOME -u PYTHONOPTIMIZE \
  PYTHONDONTWRITEBYTECODE=1 HIP_VISIBLE_DEVICES= ROCR_VISIBLE_DEVICES= CUDA_VISIBLE_DEVICES= \
  OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1 \
  timeout --signal=TERM --kill-after=5s 150s taskset -c 8,9 nice -n 10 \
  prlimit --as=2147483648 --cpu=120 --fsize=67108864 --core=0 -- \
  /usr/bin/python3 -B "$DEPLOY/test_audit_projection_ar4_host_observation_runtime.py"
```

## Lossless Assembly

Use the corrected `p228-rope-indexed-ar4-inputs-v1/prepare.py`
(15eeadc7dc143d62ed3fd01099ead3c886225792c231a540990cb58f2030ea1d)
as a four-output data-assembly template, not unchanged executable input.
Its package74/old native/pure constants and prefix-selection delta must be
replaced with actual package91/CPU7d8c/pure1485/new runtime reviews.
Load the frozen observer_cpu alias as well as layer_validation,
prefix_contracts and intake when using its manual module-loader pattern;
restore prior aliases afterward. D.Pins.read defaults to retain=False, so
config/notes JSON reads must explicitly pass retain=True.

Start from actual RoPE AR4 request
E/prefix-rope-indexed-ar4-inputs-v228-v1/request.json, 8780 B,
504360572346a4ea8a859324f1dd40303b0c73c69ba67b93efe719912ece1fb4.
Bind its actual plan through native completion30119e94, retaining all old
baseline/image prerequisites. Update parent_cpu and worker_cpu to CPU7d8c,
parent/worker to the two selected products, two runtime reviews, actual
pure91/source-before pin, and new input/output/review namespaces.

In the Rust request change EXACTLY decode.worker, decode.session, and
decode.evidence_directory. Convert the new worker's hex SHA into Rust's
32-byte JSON array. Keep prefix29fd/SiLUb0d/projection25338/all original
Begin images, model/prompt, mode=autoregressive, seed9112 and both timeout
values unchanged. Keep exact integer device IDs 16366993098680759275 and
10838076764495710945; use Python JSON, not JavaScript-number conversion.
Never force historical output tokens. The prompt's 2048 entries do not
turn this four-forward route into a 2048-prefill workload.

Reuse frozen intake.supervisor_tests/baseline/cpu_evidence/prefix_evidence/
image_evidence/deployed_binary/request_check/engineering_review and the
old P.runtime_review/guard APIs. Root supplies fresh session, one-attempt
decisions and substantive topic notes; assembly does not invent approval.
Output request.json, decode-review.json, plan.json, assembly.json in fresh
E/prefix-projection-ar4-host-observation-inputs-v228-v1. Native output must
be absent at E/prefix-projection-ar4-host-observation-gpu-v228-v1.

## Root Execution Commands

After the selector has been tested/reviewed and the assembler successor
has been authored/reviewed, use these argument shapes. Run audits with the
same clean hidden-GPU environment and outer 150-second wall bound as above;
the selector itself installs the existing resource limits. Only the
assembler filename remains proposed:

```sh
taskset -c 8,9 nice -n 10 /usr/bin/python3 -B "$DEPLOY/audit_projection_ar4_host_observation_runtime.py" \
  projection-ar4-host-observation-runtime-parent-v228-v1 parent "$C/complete.json" \
  7d8c08eeffab9cbebbf6abc973c0ba93d80616ad81063b35587e74d04df1d59c \
  "$C/target/parent/debug/ferric-qwen3-finite-projection-residual-decode-host-engineering" \
  1fad2d8aafe9ce4e799322d42d1015756ead228bf2f237c302ee260fecb656fd
taskset -c 8,9 nice -n 10 /usr/bin/python3 -B "$DEPLOY/audit_projection_ar4_host_observation_runtime.py" \
  projection-ar4-host-observation-runtime-worker-v228-v1 worker "$C/complete.json" \
  7d8c08eeffab9cbebbf6abc973c0ba93d80616ad81063b35587e74d04df1d59c \
  "$C/target/worker/debug/ferric-tp-peer-finite-engineering-worker-v1" \
  14135b08c276ba9d38fcbae635fe14c3db8bd11b934ccf5ac8d51e7c2876bd75
/usr/bin/python3 -B "$E/prepare_projection_ar4_host_observation_inputs.py" \
  "$CONFIG_PATH" "$CONFIG_SHA" "$ROOT_NOTES_PATH" "$ROOT_NOTES_SHA"
taskset -c 8,9 /usr/bin/python3 -B "$P/run.py" "$PLAN_PATH" "$PLAN_SHA"
```

The native controller must start at nice0/UID9661; it sets nice10 and limits
on owned leaves itself. Retain its existing 4000-second native/4300-second
case bounds, 32 GiB native AS, 64 MiB case/8 MiB streams, one attempt/no
retry and three pre/three post idle/empty-process audits. No monitor-policy
exception is introduced. Do not run the parent directly outside run.py.
It selects --observe-projection-host and the distinct worker host selector.

Success requires ordinary native14/Control/argmax/Close/EOF/reap validation
and the exact sibling native-projection-host-observation.json plus parent
wrapper validation. The sidecar contains four forward walls, serialization
durations, inclusive nested host counters and Close duration. These are
not GPU durations, additive disjoint times, throughput or 2048/256 evidence.
