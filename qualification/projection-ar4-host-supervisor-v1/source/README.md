# Projection AR4 Host Observation GPU Proposal

Source-only runnable supervisor proposal with 91 authored synthetic tests;
none were executed by the author. No CPU qualification, runtime audit or GPU
result is asserted here. Root owns actual pins, preparation, tests and execution.
The Rust proposal remains frozen at source-manifest
`ab5d055a09c320780a0d0da7d235bb0fe023469c9132b37a877fbc7841738074`.

The 21-member package adapts the existing 74-test RoPE AR4 package without
changing its arithmetic validator or six frozen helper bodies. It adds eight
host-report tests, eight scoped-CPU tests and one supervisor sidecar-failure
test; the original supervisor fixtures now exercise the distinct argv and
wrapper path. `observer_cpu.py` admits a caller-pinned actual completion and
its actual Cargo products, not guessed future digests. The old CPU1037 gate
remains as `prior_cpu_evidence`, explicitly a predecessor check.

Root freezes `manifest.json` with schema
`ferric-p228-projection-ar4-host-observation-gpu-package-v1`, `pure_tests:91`,
the exact closed source roster and census recorded in the pure wrapper.
Install the wrapper both inside the package and at its required external
`E/run_projection_ar4_host_observation_gpu_pure_p228_v1.py` path.
Its current body SHA is
`f9e38e0793bb4f2f5d285d29ef8df817d83cf67f66207153c353c161a2dfa6e2`.
Root runs it CPU-only on MI350 with hidden GPUs, CPU8/9, nice10, 2 GiB AS,
120 CPU seconds and an external wall deadline. It takes `MANIFEST_SHA` and
fresh `projection-ar4-host-observation-gpu-pure-v228-vN`, retains exact named
before/after inventory, source pins and log, and makes no native claim.
After actual CPU and runtime admission, `run.py PLAN_PATH PLAN_SHA` uses
the unchanged nice0 UID9661/CPU8/9 controller convention on MI350.

## Exact Workload

Reuse the successful RoPE AR4 request, originally
`E/prefix-rope-indexed-ar4-inputs-v228-v1/request.json` (8780 bytes,
`504360572346a4ea8a859324f1dd40303b0c73c69ba67b93efe719912ece1fb4`).
Its native outer completion is `30119e94...`, not a new observation result.
Keep `FerricFiniteProjectionResidualDecodeRequestV1`, autoregressive mode,
seed 9112, TP2, all 36 layers, four forwards and own-output recurrence.
The pinned prompt contains 2048 tokens, but this route consumes only its
first token as the seed; it does not prefill 2048 tokens or generate 256.
Do not install the historical outputs as forced inputs.

Retain all model/tokenizer/prompt pins, device IDs as exact u64 JSON integers,
dispatch timeout 10000 ms and child deadline 3600000 ms. Retain these images:

- RoPE prefix, 54344 bytes,
  `29fd58e7b09fee003ed31660e6201c20a7eb5da4b873d0b4d954993f7018f2f8`.
- SiLU MLP, 33320 bytes,
  `b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589`.
- Projection residual, 10864 bytes,
  `25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25`.
- Every original Begin/bootstrap image and residual-copy pin, unchanged.

Only the worker FilePin, fresh session and evidence directory change in the
request. Select the distinct host-observation parent outside the request.
This is not a new arithmetic candidate or a sustained-performance run.

## Actual Qualification Gate

Wait for a genuine successful receipt from
`p228-projection-ar4-host-observation-cpu-v1/run.py`; no future digest or
executable extent is predeclared. Its result schema is
`ferric-p228-projection-ar4-host-observation-cpu-result-v1`.
Authenticate source maps, reviewed overlay, actual named outcomes, 61 natural
phases and clean postchecks. The intended scoped cohort is 855 passes and
four historical ignores; the old runtime208 is historical, not rerun.

Select only the actual Cargo `binaries` entries for:

- `ferric-qwen3-finite-projection-residual-decode-host-engineering`.
- `ferric-tp-peer-finite-engineering-worker-v1`.

Their original paths are under the fresh CPU result's
`target/{parent,worker}/debug/`. The qualified plain projection parent is
retained for regression/provenance, not substituted as the observer.
Transfer the two selected ELF bodies and the small qualification/source
receipts needed by admission. Do not copy Cargo targets or substitute the
old CPU1037 ELF hashes. Keep the CPU1037 predecessor explicitly linked.

Run fresh bounded runtime audits for both new ELF bodies on MI350 using the
existing readelf/ldd/ownership auditor; exact actual results and resolved
libraries must be root-reviewed before admission. Old-ELF audit receipts
cannot qualify these new executables. No new monitor allowance is implied.

## Small Supervisor Successor

Reuse `p228-rope-indexed-ar4-gpu-v1/run.py`, SHA
`6e1a70ad537aa20b38907fcc4d189be605865d088fcbc034ea7ec76ed5aa575d`.
Keep its owned subprocess/pidfd cleanup, lineage, one attempt/no retry,
three pre- and three post-audits, exact empty process policy, idle samples,
post-audit finally path, CPU8/9 nice10 leaves, 4000-second native and
4300-second case deadlines, 32 GiB native AS limit, 8 MiB streams, 64 MiB
case cap, and 40/38 GiB free-space floors. No KFD or fence-policy changes.

The native leaf argv becomes:

```text
ACTUAL_OBSERVER_PARENT --request ACTUAL_REQUEST_JSON \
  --allow-unauthenticated-machine-code --observe-projection-host
```

The parent selects `--engineering-native-projection-residual-decode-host-v1`
and binds its sidecar path; the outer supervisor does not launch a second
worker. Preserve the actual one-child marker and profile/mode lines.

Retain the original 14-file native body/summary roster. Add the bounded
sibling `native-projection-host-observation.json` outside `native/` and the
parent stdout wrapper. Do not require wrapper bytes to equal native summary
bytes: the wrapper contains that exact observation as a nested JSON member.
Run unchanged `decode_validation.py` (SHA
`0eb96d4ac5e10e7f6ac10b018692c56f969286949681be336cb6040d56f01422`)
on the actual native summary plus bodies first. It still checks four
606976-byte payloads, 152 tensors, 576 terminal states, own argmax/recurrence,
Control/transcript/profile chains, Close5, EOF and the one naturally reaped
worker. No old-output or teacher-forced token equality becomes admission.

Adapt only the existing host data validator
`p227-prefix-decode-host-gpu-observation-v1/host_validation.py`
(`ca7ca657161ecd7aa00a4333971ed8fbbc8f2667c082641a97ae71674897aea6`):
use the new projection wrapper/report schemas, nested projection bootstrap
and request, exact sidecar suffix, and four serialization durations. Require
the wrapper's exact observation member plus LF to equal native summary;
join parent/worker/image/session/profile, completion payload/control digests,
Close and transcript to the independently checked native observation.
Preserve original JSON member bytes for serde projection hashing and u64s.
Require seven ordered snapshots, six monotone checked deltas, full-currentness
policy, no operational/shared/cached/raw-timestamp options, and false claims.
The report's new serialization duration i belongs within interval i+2,
not the interval that encloses forward i. Keep its 64 KiB sidecar and
8 MiB combined native bound. Reject absent, aliased, malformed or mismatched
sidecars even after a structurally valid native Close.

Focused pure regressions should cover wrapper/summary identity, new actual
artifact selection, missing/changed sidecar, both duration interval bounds,
non-full-currentness flags, profile/worker/Close substitution and unchanged
post-audit retention on failure. Reuse existing AR4 validator/supervisor
tests, not a new process-ownership framework. Freeze only actual sources.

## Interpretation

Publish enclosing `forward_host_ns[4]`, `serialization_host_ns[4]`, Close
host duration and raw setup/forward counter deltas. Distinguish setup
writes/kernel admission from forward full-currentness checks, dispatch
prepare/publish/wait/polls and guarded readback. Rank/shared counters are
inclusive nested host scopes, not disjoint categories to sum. Currentness
work can be included inside command/dispatch/read durations; serialization
also includes pipe blocking. Do not subtract them as if they were a GPU
timeline. Observer overhead and cold setup remain present.

The decisive first measurement is whether forward host wall time is
dominated by full-currentness validation, wait/poll, guarded readback or
serialization, separately from setup. No GPU-time, throughput, 700-token/s,
accuracy, 2048/256, overlap or production claim follows from this run.
