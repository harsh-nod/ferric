# State-Bank Four Controller

After actual CPU qualification, deployment replay and root review:

```sh
taskset -c 8,9 /usr/bin/python3 -B run.py PLAN_PATH PLAN_SHA256
```

The command remains the same CPU633 parent executable followed by
`--request REQUEST --allow-unauthenticated-machine-code --observe-host-policy`.
The request selects the newly qualified bank-batch worker and unchanged V7
image. There is one native attempt and no retry. All 152 structural tensor
rows, native Close/EOF, owned process-group absence and reaping are required.
Three pre-audits and three post-audits use exactly the preceding topology and
process checks. Failure retains partial evidence and prevents completion.

Limits and ownership code are unchanged: 4,000 seconds native, 30 seconds per
audit, 4,300 seconds per case; CPU affinity 8,9; child nice 10; native 32 GiB
and audit 12 GiB address space; 8 MiB streams; 64 MiB case evidence;
256 entries; 40/38 GiB disk floors; original pidfd cleanup/reaping behavior.

Result schema: `ferric-p228-state-bank-batch-observation-v1`.
The field list is unchanged from the CPU522 observer. `worker_cpu_complete`
and `worker_cpu_review` now identify the paired bank-source qualification.
`prior_deployment`, `prior_worker_cpu_complete`, `prior_worker_cpu_review` and
`historical_runtime` explicitly identify the CPU522 generation. Parent CPU633,
V7 image, six standalone cases and six numerical prerequisites remain separate.

The structural result keeps all numerical, full-model, performance and
production authority flags false. It does not imply 2,048/256 completion or
700 tokens/s. Any CPU522-versus-candidate tensor comparison, independent
diagnostic, host-counter attribution or descriptive timing comparison belongs
in a separate bounded CPU step after the GPU controller terminates and its
post-audits finish. No such result is fabricated by this draft.
