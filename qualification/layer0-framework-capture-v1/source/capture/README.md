# Layer 0 Framework Capture Draft

Source-only, unexecuted proposal. No tests, framework imports, or GPU operations
were performed by the author. Sixteen synthetic tests are authored. Root owns
review, freezing, environment inspection, execution and retained results.

This captures 33 actual BF16 intermediates for layer 0, token 9112, position 0
from the original Qwen3-8B framework chain, twice with independent empty KV.
It does not calculate a replacement activation/residual, consume native
intermediates, or grant numerical/full-model acceptance.

## Reuse And Scope

The unchanged retained helper is
`p224-rearm-framework/reference/framework_reference.py`, SHA
`613579c7c84ed9b6f94abf6864c9934b72331dd001bf6badb973ab4576f4db97`.
Its `checked_plan`, source authentication, original-model helper, `Capture`,
BF16 tensor reader and live bounds are reused. Its four-forward `execute`,
`run_pass`, schemas, policies and retained outputs are never changed or called.

The new executor preserves the original model load and deterministic math-SDPA
settings, BF16 parameters and FP32 RoPE buffer. One ordinary 36-layer
`model.model` invocation is made per pass, but hooks retain only layer 0 plus
its genuine embedding/rotary/KV data. No early-exit exception truncates model
execution, and the language-model head is not run. The model is not replaced
with a hand-written layer implementation.

Module pre/post hooks observe existing calls without changing their arguments
or results. The only callable observer temporarily wraps the actual
`apply_rotary_pos_emb`, returning the original result objects unchanged. Its
selection is bracketed by layer-0 attention hooks. Exactly 36 real rotary calls
and one selected call are required, and the original callable and all hooks
are restored in `finally`. A replacement or duplicate invocation is refused.
Hook synchronization makes this unsuitable for performance measurement.

## Required Inputs

`run.py --inspect PLAN SHA256` authenticates inputs without importing Torch.
`run.py --run-reviewed-layer0-capture PLAN SHA256` runs only as the direct child
of the root-owned supervisor named by the authenticated environment plan.
Both require the retained Python 3.10 environment under `-I -B`, unchanged
package versions, host/boot/device identity, visibility and resource policy.
The root must supply the actual interpreter path, not substitute system Python.

The new closed plan has these fields:

- `schema`: `ferric-p228-layer0-framework-capture-plan-v1`.
- `harness_sha256`: the actual frozen new `run.py` digest.
- `reference_helper`: exact old helper FilePin above, at its original helper
  directory with unchanged `diagnostics.py`, `policy.json` and
  `helpers/long_reference.py` available.
- `reference_plan`: a fresh root-authored P224-shaped environment plan accepted
  by the unchanged old `checked_plan`. It binds current host, boot, selected GPU,
  direct supervisor, original model and prompt, old source/policy/helper pins,
  isolated environment, resources and its separately reviewed execution input.
  It retains the original four-token provenance but is used here solely for
  validation; no old four-forward launch or report is substituted.
- `output_root`: exactly the environment plan's output, under
  `/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/`
  with fresh `layer0-framework-capture-v228-vN` basename. Existing outputs fail.
- `implementation_sources`: four exact installed FilePins, described below.
- `execution_review`: a separately root-authored new review FilePin, not a
  template treated as approved evidence.

The new review has closed schema
`ferric-p228-layer0-framework-execution-review-v1`, `reviewed=true`,
`gpu_execution_authorized=true`, the unchanged resource map, and
`plan_projection_sha256 = SHA256(encoded(plan without execution_review))`.
Here `encoded` is sorted, two-space-indented JSON plus a trailing newline, with
nonfinite constants forbidden. Numerical, performance and production flags
must all be false. No review or future source hash is invented in this draft.

Root must first inspect and pin the actual installed objects:

| Source role | Actual object whose source file must match |
| --- | --- |
| `modeling_qwen3` | `model.__class__` |
| `activation` | `type(model.model.layers[0].mlp.act_fn)` |
| `sdpa` | Qwen module `ALL_ATTENTION_FUNCTIONS['sdpa']` |
| `torch_functional` | `torch.nn.functional` |

The Qwen source must match the old reference's exact SHA
`704c914530530a1acb0b443add1f520404e3ac2c28c0ab7e16f80f86cfe8ccb2`.
All four files must be in the retained isolated Python prefix and are read
before execution, joined to the actual loaded callables, rechecked after
execution, and copied into the new evidence. Tagged web source or a version
string is not substituted for the installed bytes. These Python file pins do
not claim complete authentication of native Torch/BLAS machine code.

## Actual Data

The fixed stage roster in `SHAPES` captures embedding and norm inputs/outputs;
Q/K/V projection inputs and outputs; Q/K norm inputs/outputs; shared cosine and
sine; actual rotated Q/K; attention output and O projection; first residual;
post-attention norm; gate/up inputs and outputs; actual SiLU input/output;
actual product consumed by down projection; down output; MLP output; layer-0
hidden output; and layer-0 stored key/value.

Every observed tensor must actually be BF16, finite, and have the exact declared
shape. There is no implicit cast to make a dtype mismatch pass. Producer/input
and reshape/cache byte joins are explicit. The captured `silu` and `product`
therefore distinguish framework materialization boundaries; O/down outputs and
first/second residuals distinguish projection narrowing from residual addition.
No residual result is recomputed and substituted into the genuine chain.

Output `capture.json` reports each pass's stage FilePins, shapes and observed
dtypes, exact installed source copies, original model provenance, runtime
settings, repeated-stage equality and false acceptance/performance flags. A
successful inner report still requires the external owned supervisor's natural
exit/reaping and current platform audits. An exception or incomplete capture
does not produce a successful report. A repeat mismatch writes `status=FAIL`
and exits nonzero.

## Bounds And Follow-up

Unchanged bounds are 900 seconds, 64 GiB host RSS, 48 GiB initial available host
and GPU memory, 32 MiB capture output and 1 GiB private cache. The selected GPU
is the one authenticated by the original environment-plan checks. Root must
retain its existing outer deadline/process ownership/audits; this script does
not introduce another supervisor or permit concurrent GPU work.

Sixteen pure tests cover closed plans/reviews, wrong source and namespace,
complete shapes/extents and producer joins, observed-reader refusal, rotary
routing/object identity/restoration, repeat mismatch and incorrect history or
dtype. They use synthetic bytes/objects and do not test an installed framework.
No actual result can be inferred from these tests.

After an actual capture, first compare layer-0 output with the retained original
position-zero framework output, then compare independently captured native V7
stages. Only after locating the first differing stage should a separately
labeled conditional replay feed identical captured operands to competing
arithmetic expressions. This package deliberately does not execute such replay
or choose a new tolerance. The long-context and whole-model numerical gates
remain open.
