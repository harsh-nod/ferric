# Current-Worker Native Down ABBA

This adapter repeats the qualified native down control/split-K8 experiment with
the ordinary `engineering-gfx950` worker built from fe2o3 `1736eff451` code. Both
arms use worker SHA-256
`8f764849a5a1c6a23c567f8aefe253de5e6ef2320977a79db7c472ad3417a8b5`.
Wait and packet diagnostics are disabled. This is native-ingress measurement,
not HTTP serving or a new vendor comparison.

The existing down harness, controller binaries, ten loaded images, 128 KiB
scratch, source and CPU evidence remain immutable historical ancestry. Its
`da6b561...` runtime pin is not rewritten or presented as current qualification.
The new `worker_refresh` envelope separately binds the new worker, source roster,
exact build receipt and clean bounded outer build result. The prior interrupted
campaign contributes no measurements to this one.

## Staging Contract

1. Restore or prepare the complete historical stage and its hash-bound
   `plan.json` using the existing qualified import/prepare path. Do not restore
   old `cells/` outputs into the new stage.
2. Create a separate mode-0700 `/dev/shm/ferric-native-down-*` directory. Copy
   every historical `files` member byte-for-byte except `worker-candidate` and
   `run_stage.py`. Do not change embedded historical receipt paths.
3. Stage the four Python files from this directory at the fresh stage root.
   Stage the original guard and contract at `historical/run_stage.py` and
   `historical/launch_contract.py`. The root `launch_contract.py` is unchanged.
4. Stage the current worker at `worker-candidate` with mode 0700. Stage its
   receipt, outer `result.json`, build stdout/stderr, and the full hash roster at
   `worker-cpu/{receipt.json,result.json,stdout,stderr,source.json}`.
5. Stage this adapter's fresh CPU receipt, outer result and stdout/stderr at
   `harness-cpu/{receipt.json,result.json,stdout,stderr}`. All data and Python
   inputs use mode 0600, matching the historical file-identity policy.
6. Record the exact fresh file roster; use `current_binding.derive_plan` on the
   CPU/native host with the fully validated original plan and existing cell
   module. Write the new plan create-only. `build` and `cpu` intentionally
   continue to name historical ancestry; `worker_refresh` identifies the new
   executable. The externally retained new plan hash binds both.

The wrapper uses the name `run_stage.py` so the unchanged retained-cell replay
can verify the actual executed command. It calls the original admission,
execution, exact token replay, process endpoints, ABBA ledger and supervisor
functions without replacing constants or bypassing validators. Only the small
outer orchestration is repeated. Both historical and new stages are locked.

Run `--validate-only` before launching, then `--supervise --cell-id CELL` in the
declared order: two counters, followed by three complete ABBA blocks. Finally
run `--summarize`. Each invocation requires `--plan PATH --plan-sha256 SHA256`.
Every predecessor is independently replayed against the new plan hash before
the next cell; old cells cannot satisfy this chain. Admission still requires
64 GiB root free, 32 GiB tmpfs free, 128 GiB available RAM, a 2 GiB stage cap,
root-visible sampled GPU isolation and clean, unsignaled process retirement.
Retain results durably before closing the SSH session on hosts with RemoveIPC.

## CPU Qualification

Run only on the CPU build host, under the validated G48 guard and environment:

```sh
/usr/bin/python3 -I -B INPUTS/qualify_cpu.py --output /tmp/ferric-v16-emitter-b95a642-r1/native-down-current-cpu-a001
```

The qualifier checks syntax and runs exactly 24 focused tests, binding all four
source hashes before and after. It reserves 1 MiB and cannot raise the aggregate
cap or bypass cleanup rules. Full actual-plan validation is additionally required
before native execution; these fixtures alone do not qualify hardware behavior.
The historical worker build must retain its exact G45 closure; the new adapter
qualification must use G48. The validator does not accept either profile
interchangeably.
