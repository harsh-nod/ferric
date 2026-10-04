# AR4 Supervisor And Framework Policy Tests

Two separate CPU-only policy suites passed. Neither suite loaded the model,
ran a native executable or launched a GPU kernel.

| Suite | Host | Passed | Failures / Errors / Skips |
| --- | --- | ---: | ---: |
| AR4 supervisor, structural validator and admission | MI350 | 52 | 0 / 0 / 0 |
| Independent AR4 reference and owned launcher policy | ASROCK | 38 | 0 / 0 / 0 |

The MI350 suite has an actual bounded wrapper receipt, exact named test
inventories and unchanged before/after source snapshots. It ran with hidden
GPUs, CPUs 8 and 9, nice 10, and memory, CPU-time and file-size limits.

The ASROCK suite is retained as a primary-agent observation of actual SSH tool
output, not a remote supervisor receipt. The observed repeat used a 120-second
timeout, CPUs 8 and 9, nice 10 and hidden GPUs; all 38 tests passed in 0.017
seconds. Full test output and the five before/after source hashes are retained.
Its evidence type remains distinct in [result.json](result.json).

## Coverage

The native policy checks seed 9112 and each next input against the previous
independently checked lowest-index argmax. It retains the four-forward,
36-layer format: 152 tensor slices, 576 terminal states, profile-bound request
and completion chains, Close, one attempt, owned reaping and six audits.
These are tested validation rules, not actual captured AR4 tensors or tokens.

The independent reference policy separates genuine own-output autoregression
from conditional replay of native input tokens. Genuine tensor comparisons
stop after input-history divergence and cannot resume merely because later
tokens match. Any conditional replay uses a fresh framework KV cache and
borrows only tokens, never native tensors. Tests cover this separation, source
and module-alias checks, fresh-PID review binding and the inherited launcher
failure/audit behavior.

## Retained Evidence

- [Native policy receipt](tests/supervisor/complete.json) and [named transcript](tests/supervisor/tests.log).
- [ASROCK primary-tool observation](tests/framework/primary-observation.json).
- [Frozen supervisor package](supervisor/manifest.json), including its executed wrapper.
- [Framework reference](framework/run.py), [launcher](framework/launch.py) and both tested policy modules.
- [Data-only publisher](tools/publish.py) and [copied-file ledger](result.json).

Frozen source READMEs retain their original draft status; this page describes
the subsequent actual CPU test observations without changing tested bytes.
The related [AR4 Rust qualification](../projection-ar4-cpu-v1/README.md) is a
separate checkpoint. Runtime audits, native AR4 execution and genuine framework
numerical comparison require their own evidence. Numerical acceptance,
2,048/256 qualification, production authority and the 700-token/s target remain
open. No threshold or performance result is introduced here.
