# Compact Full2303 Independent Reference Proposal

Source only. No test, import, container, GPU or model execution is claimed here.
This proposal does not enable a native Full2303 launch or change its one-hour
source cap. No future reference terminal, native terminal or acceptance is bound.

## Reused Qualified State

The predecessor is the retained Position5 reference under
`qualification/guarded-mlp-readiness40-position5-v1/reference-v1` in the Ferric
checkout. Its actual owner is `36365edf95b99eeac27ec91973862959f8b354d876258e29467b19753bd802ae`
and inner report is `d256b149229d4ba94a1e4f9a3aa2d6f9354c56a721bfe1dc0cb69bbb7c49674f`.
Those observations executed two forty-position teacher-forced passes, not this
2048/256 workload. Their source manifest is
`47278b4c53f95a2b47c7d594a897b9ff29e055de4d2c9e151d6c592920225d26`.

The six helper bodies `common.py`, `owned.py`, `long_reference.py`,
`framework_reference.py`, `diagnostics.py` and `reference.py` are byte-identical.
The last is used for its authenticated full-prompt/model input admission and
existing raw capture/pure diagnostic helpers. Its old forty-position validators
are not the new generated-history gate. The historical long helper supplies the
same checkpoint loader support, tokenizer/raw-byte decoder and FP32-RoPE device
move. Its obsolete ASRock/Python3.10 `checked_plan` is not invoked.

The existing immutable image remains
`sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba`.
The observed environment body remains 1,987 bytes,
`54851b5bccdf6f7d65e67ffa7ffa1e3ad1b7d591200e9ef9448ffb06ee485fae`:
Python3.12.13, Torch2.12.0+git6bbd260, Transformers4.51.0 in the private pinned
overlay, HIP7.2.53211, and the same four authenticated implementation bodies.
No package installation, download or environment substitution is proposed.

The six original data roles and nine checkpoint pins are unchanged in
`inputs.json`. The authentic prompt is 2,048 u32 tokens, no chat template or
added special tokens; the source of its bytes is the 21,318-byte manifest
`30d047aafbd7de2b94647154680b0b9d9fb75aae2fd90b081afe0de595a88600`.
The full tokenizer round trip runs before inference. Native captures, tokens,
receipts and hidden states are not inputs to reference generation.

## Genuine Generated History

Each of two passes starts a fresh `DynamicCache`. The first actual framework
call consumes the entire 2,048-token prompt with ordinary matrix prefill. The
next 255 calls consume only the immediately preceding choice from that same
pass. There are exactly 256 framework calls, 2,303 processed positions and 256
generated IDs per pass. Output0 is chosen at position2047; output255 is chosen at
position2302 and is never consumed. No speculative decoding, early EOS stop,
sampling or reference-token feedback is allowed.

Every generated logit row is BF16, finite, exactly151936 values. Greedy choice
is the lowest token ID among equal maxima, including signed zero. Top-two BF16
bit encodings and the exact nonnegative margin in integer units of 2^-133 are
recorded. This integer representation describes existing BF16 output values; it
does not change model arithmetic or claim exact-real model logits.

The full BF16 model loader, math-only SDPA, disabled low-precision reductions,
deterministic algorithms, eval/inference mode and preserved FP32 rotary buffers
are unchanged. All36 actual KV shapes/dtypes/devices and exact cache length are
checked each call. Selected calls additionally hash actual complete key/value
bytes for all36 layers. Both prefill captures bind the same cache length2048
and same post-prefill cache hashes. No fictional position0 cache state or extra
position0 model call is reported.

## Captures And Bytes

The four full captures per pass are at0,2047,2048,2302. Each606976-byte payload
contains all36 layer-hidden BF16 slices, final norm, and logits. Real hooks take
positions0 and2047 from the single matrix-prefill call; the last two come from
their genuine one-token decode calls. Final-norm input must equal layer35 output.
Hooks are removed in `finally`, including error paths. Position0 is diagnostic
only and contributes no generated output. Its additional output-head call uses
the genuine final-norm slice and never changes KV or chosen-token feedback.

Each generated row is hashed while present; an ordered SHA256 over all256 raw
rows is computed without retaining a 77,791,232-byte logit file. The compact
records also carry the independently recomputable chain of ordered row pins.
Unselected raw rows are not retained, so offline recomputation of their argmax
or tensor-error metrics is explicitly unavailable. Their declared choices are
execution observations of the source-authenticated finite-row argmax code.
The selected payloads are re-split, rehashed and their greedy diagnostics
recomputed by both child and owner pure validation.

Each pass retains all256 IDs as exactly1,024 u32le bytes, plus raw decoded bytes
with and without special tokens, each at most128KiB. No lossy UTF-8 conversion
is used. The skip-special form matches `tp_model::EngineeringQwenModelV1::decode` and the current
Full parent `generated-text.bin` policy. The owner rereads and hashes the pinned
tokenizer, independently re-decodes both actual ID streams, and posthashes it.

The repeat gate requires identical generated IDs, both decoded byte forms,
compact per-call/capture records, all four payloads, per-row pins and cumulative
logit-stream hashes. This is reference self-consistency, not Ferric acceptance.
A failed repeat remains failed with its original output and error evidence.

## Bounds And Ownership

Fresh remote namespace:
`/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-full2303-reference-v228-v1`.
Source/inputs/packages/model mounts remain read-only; network is disabled, one
fixed gfx950 device is exposed, and the named container is owned and retired by
the unchanged supervisor. There is one attempt, zero automatic retries.

Existing bounds remain:900s child,930s attached CLI,1200s operational cutoff,
1800s whole owner with600s retirement reserve; CPU8/9 and2 CPUs;64GiB container
memory/no extra swap;48GiB initial free host/GPU memory;40GiB initial and38GiB
live disk floors. Output stays32MiB aggregate/8MiB per file, scratch2GiB,
evidence64MiB. Each compact pass is capped2MiB; selected payloads total4,855,808
bytes. The pass/capture/decoded set is capped12MiB, with separate room for the
terminal's repeated documents and four implementation files. Success closes
exactly21 output files:8 captures,2 passes,6 ID/decoded files,4 implementation
bodies and the original child terminal. Source and input trees remain closed.

No completion-time assumption follows from the historical long reference or
the native944.9s/40 observation. Timeout or resource refusal is a failure, not
permission to enlarge bounds. The historical ASRock full-reference IDs and
decoded files are useful separate observations, not injected inputs or a
replacement for new results in this different Torch environment.

## Qualification And Remaining Gaps

Twenty authored pure tests comprise12 generated-history/capture/byte contract
methods plus8 inherited owner lifecycle/environment tests. They cover complete
prefill-to-decode recurrence, final-output nonconsumption, strict lengths/types,
finite/tied/subnormal logits, exact margins, hook custody, selected positions,
raw invalid-UTF8/special-token decoding, repeat drift and retained-byte bounds.
The owner runs this exact named suite before opening the container. None has
been run by the proposal author. Source AST parsing and immutable data joins
are not test execution.

No current Full2303 native/reference comparison receipt exists. After separately
admitted native completion, a bounded comparator must authenticate both whole
own histories and healthy Close, bind the same prompt/model/tokenizer and compare
all256 generated IDs and skip-special decoded bytes exactly. Neither trajectory
may be teacher-forced after divergence. A mismatch is retained, not hidden by
the repeat gate, a historical result or a new tolerance.

Matrix-prefill and serial native prefill can differ in floating-point
association. BF16 framework logits are not an exact-real oracle; current exact
operator evidence already sometimes favors native values. Tensor metrics on
selected captures remain diagnostics, with `acceptance_threshold=None` and
`numerical_acceptance=False`. Generated-token behavioral equality is distinct
from a reviewed aggregate full-model tensor policy, which is still absent.
No throughput, full-model numerical acceptance or native feasibility is claimed.
