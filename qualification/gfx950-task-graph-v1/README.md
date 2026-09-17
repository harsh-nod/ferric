# Finite Task-Graph Qualification Harness

This directory provides an independent exact-integer DAG reference and an
engineering-only dispatch harness for the [device source](../../device/gfx950-task-graph-v1/README.md).
The ordinary Rust source now compiles through semantic MIR, ranked/formal
checks, translation validation, target convergence and gfx950 LLVM lowering.
Production extraction 23 produced an inert native HSACO, and the bounded
engineering GPU suite passed on the original `mi350-2` host. These are
historical observations, not results for the replacement asrock host.

## Native Evidence

[evidence-native-v1.json](evidence-native-v1.json) records the exact identities,
sanitized ownership observations and hashes of the retained raw evidence.
The measured compiler is `af4f14601df828cf0a6ce5b8229d8048c189d1e6`, even if
the development branch subsequently advances. Its immutable snapshot manifest
has SHA-256 `93a3598fad6eb7ec248f08ef89c844bdf61f6546ac5d347af71ee57b4992bf63`.

| Check | Observed Result |
| --- | --- |
| Cases | Zero, ramp, maximum and seeded random |
| Dispatches | 20: 16 valid epochs and 4 stale-epoch rejections |
| Exact independent payload comparisons | 112, included in the state-word counts below |
| Atomic state words validated | 260: 244 deterministic words and 16 validated ownership encodings |
| Cross-workgroup dependencies | Both owners and 4 crossing dependency edges in every valid epoch |
| Lifecycle | All completions, immutable inputs, allocation guards, frees, queue closes and worker exits passed |
| Launch | 2 workgroups, 128 lanes each, wave64; 2 waves per workgroup |
| Native resources | 76 SGPRs, 18 VGPRs, 1,024 LDS bytes, 0 private bytes, 0 SGPR/VGPR spills |

Each valid epoch checks seven task payloads against `reference.py`, which sums
each 128-element input tile and its parent results in topological order. Inputs
are exact `u32` values in `0..=1024`; no floating-point tolerance is involved.
Each case reuses one worker queue and fifteen guarded allocations for epochs
1 through 4, then requests epoch 5 with epoch 4 initialized. The stale dispatch
must preserve the exact initial graph state except for its stale-error flag.
Ownership words are checked structurally, not against a predetermined schedule.
For example, ramp epochs 2 and 4 have different valid ownership assignments.
Crossing edges are derived from the actual parent/child owner fields, not
inferred from the two-workgroup launch.

Retained remote evidence is under `/home/harmenon/ferric-gfx950-42/`:
`evidence/task-graph-extraction-23/` contains native build inputs, outputs and
metadata; `evidence/compiler-contextual-product-bin/` contains the matched
compiler snapshot; `evidence-gpu/task-graph-native-v1/` contains four case
directories with inputs, independent references, raw states, reports and
numerical checks. The public JSON binds raw report and numerical hashes without
publishing the private device selector or worker stderr.

## Reproduction

The historical artifact used ROCm 7.2.0 and the compiler recorded above. Replay
of those identities requires their original source revisions and build script;
advancing either dependency pin or compiler produces new evidence.

For a new build, both source checkouts must be committed and clean. The device
and host dependency revisions, including Cargo's resolved providers, must equal
the compiler checkout's HEAD. Use a frozen matching compiler snapshot with
`compiler-source-head.txt`, empty `compiler-source-status.txt`, `SHA256SUMS`,
`loader-providers.sha256`, and the adjacent `SNAPSHOT.provenance.sha256` manifest.
The build verifies exact selected manifest members and rechecks sources and
hashes afterward. Cargo metadata and extraction run offline with an isolated
environment; populate the reviewed dependency cache before the retained build.

Use fresh output directories, offline artifact inspection and shared-host
device allocation checks before dispatch. `ROCM_PATH` selects the reviewed
ROCm installation explicitly; the asrock rebuild uses its ROCm 7.3 installation,
while the script's historical default remains `/opt/rocm-7.2.0`. A new build
does not establish an asrock GPU result.

```sh
cd qualification/gfx950-task-graph-v1
python3 -B -m unittest -v test_reference
FE2O3_COMPILER_BIN=MATCHED_CLEAN_COMPILER_SNAPSHOT \
  ROCM_PATH=REVIEWED_ROCM_INSTALLATION bash build.sh WORK_ROOT FRESH_BUILD_DIRECTORY
bash verify-suite.sh PROBE WORKER HSACO SOURCE PRIVATE_DEVICE_ID NEW_EVIDENCE_DIRECTORY
```

The suite freezes references and executable/source hashes before dispatch and
checks them again afterward. Never substitute handwritten machine code or IR
to bypass a compiler rejection. The engineering runner explicitly acknowledges
`--allow-unauthenticated-machine-code`; these observations do not confer
production launch authority.

## Limits

The worker requests `DEVICE_LOCAL_PUBLIC` allocations
(`VRAM | WRITABLE | PUBLIC`), not the `COHERENT` allocation flag. HBM residency
and bandwidth were not independently measured. The cross-workgroup atomic
observations establish tested behavior for this artifact and allocation path,
not ordinary tensor visibility or a protected-runtime guarantee.

This is a seven-task integer scheduler micrograph, not tensor operators,
ordinary non-atomic tensor publication, full-model inference, an unbounded
persistent runtime or a production qualification. Host dispatch intervals are
not GPU event timing. No GPU-only latency, model-speedup, throughput, benchmark
comparison or state-of-the-art claim is made. Further scheduling stress and
operator/model integration remain separate milestones.
