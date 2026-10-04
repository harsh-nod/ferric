# Independent Four-Forward Controller

Status: unfrozen source draft. The 15 controller tests are authored, not run.
No native execution, numerical acceptance, review, or GPU result is established
by these files. Root owns helper staging, package freeze, tests, intake review,
runtime reviews, and any eventual one-shot launch.

## Narrow Delta

`run.py` derives from the frozen `p228-group-fence-gpu-v3/run.py`, SHA
`c67e67d75a95deeec606e768703d73e304c9ac53ba4f1d5cb4a90af038960342`.
The functions `resources`, `inventory`, `child_limits`, `bounded`, `retained`,
and `retained_host` are unchanged. The owned native command, pidfd tracking,
cleanup/reaping, evidence retention, three pre-audits, three finally-block
post-audits, source guards, one attempt, and no retries are unchanged.

The separately versioned intake authenticates both deployments: the existing
CPU-qualified parent/worker cohort and the V7 image. It returns `selected_runtime`
and `runtime` for that explicit selection, while `historical_runtime` preserves
the original deployment identity. The selected image is not relabeled as part
of the old deployment. All six prefix GPU and conditional numerical receipts
remain separately bound prerequisites, not all-layer acceptance.

The old paired-comparison helper and controller remain unchanged. This new
controller calls the new intake's `compare_native` execution interface, which
delegates only to the structural observation helper. It accepts only the closed
`ferric-p228-prefix-decode-independent-observation-v1` result, exact request,
mode, policy, selected image, integer 152-row census, complete native structure,
and recorded Close/owner-reap checks. Every helper numerical, provenance,
full-model, production, and performance authority flag must remain false.

The result is retained as `observation.json`, never `comparison.json`. The
controller completion schema is `ferric-p228-independent-decode-observation-v1`.
`passed` means this one finite structural observation and its six surrounding
audits completed under the owned-process policy. It does not mean any tensor
matched a baseline or mathematical reference. The helper's `gpu_launched=false`
describes that data-only helper; the controller separately records its actual
`gpu_execution_requested` field. A later independent, bounded CPU numerical
diagnostic must run only after the native/controller processes are terminal.

## Preserved Envelope

The native command is exactly:

```text
<selected parent> --request <reviewed request>
  --allow-unauthenticated-machine-code --observe-host-policy
```

The controller uses UID 9661, CPU affinity 8 and 9, and initial nice 0. Children
use nice 10. Existing caps remain: 4,000 seconds native, 30 seconds per audit,
4,300 seconds case, 32 GiB native address space, 12 GiB audit address space,
8 MiB per stream, 64 MiB complete case, 256 evidence entries, and 40/38 GiB
initial/ongoing disk floors. No limit, cleanup deadline, audit reserve, or runtime
permission is widened.

## Tests And Integration

`test_run.py` retains the old controller's file-backed mocked execution tests,
updated only for the new observation result. It adds both-mode handling,
legacy-parity and authority refusal, closed-schema/image/request/count refusal,
post-audit failure retention, and pre-audit refusal without a GPU attempt.
It does not spawn native children or validate real GPU tensors. Frozen native
validator/helper tests and actual preflight remain separate integration gates.

After root stages the authenticated sibling helpers and freezes the package,
the focused module is `python3 -B -m unittest -v test_run`. The eventual
controller CLI remains `run.py PLAN_PATH PLAN_SHA`; neither a draft nor passing
synthetic tests supplies the required root reviews or actual artifact pins.
