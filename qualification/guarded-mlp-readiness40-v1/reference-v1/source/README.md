# Independent Readiness40 Reference Proposal

This is a source-only independent Qwen3-8B framework capture proposal. It has
not been executed. It introduces no tolerance, numerical acceptance,
production admission, throughput claim, or native 2,303-forward result.
The framework executes the first 40 positions of the authentic 2,048-token
prompt, generates zero tokens, and never consumes native intermediate tensors.

## Existing Qualified Inputs

The loader and owned-container lifecycle come from the actually executed
`qualification/guarded-mlp-model-matched-input-v1` checkpoint. The cached image
is `sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba`.
Its private v2 package overlay manifest is 742,156 bytes,
`3630f4bdb5cd44300fa64cf76d51f937f7d652b4eb256f319c64a91f2a4ece3a`.
The retained environment record is 1,987 bytes,
`54851b5bccdf6f7d65e67ffa7ffa1e3ad1b7d591200e9ef9448ffb06ee485fae`.
It identifies Python 3.12.13, Torch 2.12.0+git6bbd260, Transformers 4.51.0,
NumPy 2.3.5, HIP 7.2.53211, and distribution `triton` 3.7.1+gitf0b55c07.
This is not the old ASRock Python environment or an assertion of the same
historical Torch version. The four implementation source files are pinned
separately and copied into every successful reference result.

The loader checkpoint's actual owner receipt is 8,303 bytes,
`2e63e5f8436edaf247249a33b40cd5d0171c0566816f2198a73ca2b6a9683cbb`;
its actual inner receipt is 45,474 bytes,
`568754e0716d165b977d0462f1c67cdb797a869ed5a8db2ae75e2c206497dc44`.
That old attempt called only layer0 MLP. It authenticates loader/environment
lineage, not an already executed forty-position full-model reference.

`inputs.json` lists six closed data roles and their local original paths/pins.
The original full prompt manifest/text/2,048 little-endian u32 IDs remain
unchanged. The selected authentic IDs are position0=9112, position15=5269,
position16=13352, position39=9104. The complete prompt is authenticated and
retokenized with the original nine-file checkpoint before any model forward.
The requested 256 generated tokens in the original workload remain part of
that workload identity; this bounded readiness job generates none.

The local retained prompt directory lacked `workload.json`. The proposal's
`prompt-workload.json` is an explicitly reconstructed copy using the original
serializer and authenticated prompt text. Its bytes exactly match the
original prompt manifest's 11,462-byte SHA-256
`2fafb955e87c60c992e65922afc938d8dc14d9e6fbcd4d0d91d68eddcd73aee8`.
Stage it as `/inputs/workload.json`; do not rewrite the original manifest.

## Execution Shape

`run.py` uses the existing complete `AutoModelForCausalLM` loader with
`local_files_only=True`, `trust_remote_code=False`, BF16 parameters, exact
Qwen3 geometry, and empty missing/unexpected/mismatched/error loading lists.
It preserves FP32 RoPE buffers, eval and inference mode, deterministic
algorithms, highest FP32 matmul precision, disabled TF32 and BF16 reduced
matmul reduction, and math-only SDPA with low-precision reduction disabled.
There is no autocast, layer-only checkpoint replacement, or custom MLP math.

Each of two passes constructs a fresh `DynamicCache`. Every one of the 40
calls uses shape `[1,1]` input IDs from the corresponding prompt position,
the original causal-mask construction, and the previous result's own cache.
All 36 K/V pairs must remain BF16 CUDA tensors with shape
`[1,8,position+1,128]`; lengths advance exactly 0 through40. Predictions are
recorded as diagnostic argmax values and are never fed back as input tokens.

Only positions0/15/16/39 attach all36 real decoder-layer output hooks and the
final-norm input prehook. They enforce exactly one capture per layer, join
the last layer output to the final-norm input, and copy finite contiguous
BF16 bytes immediately. The actual final normalized result and actual
`lm_head` output complete each 606,976-byte payload. Hook handles are removed
in reverse order on success and failure, including partial attachment.

The eight selected payloads total 4,855,808 bytes. Every step records its own
finite logit hash/argmax and causal cache length; selected steps additionally
retain all36 K/V content hashes. The two passes must match all40 records and
all selected payload bytes. Nonrepeatability preserves the failed data; it
does not select one pass as a reference silently. Output is eight payloads,
two pass JSON bodies, four implementation sources, and one terminal receipt.

## Source and CPU Gate

Five helpers are byte-identical to the qualified checkpoint:
`common.py`, `owned.py`, `long_reference.py`, `framework_reference.py`, and
`diagnostics.py`. The old four-position `run_pass`, `validate_case`, and
`compare_four_forwards` are not called. Only the original finite BF16 byte
capture, full-prompt checker/loader helpers, payload splitter, argmax and
tensor metric functions are reused where their contracts apply.

Twenty declared `test_reference.ReferenceTests` methods cover full prompt
and checkpoint joins, fixed selection, all40 input history, malformed or
trailing captures, missing/duplicate layer hooks, final-norm custody,
finite/shape/argmax checks, repeat and cache drift, changed tensor metrics
without acceptance, plus existing container isolation and signal/retirement
rules. The owner requires the exact passing method roster, supporting both
Python verbose-unittest spellings. No tests were executed during authoring.

## Root-Only Staging and Launch

Use a fresh remote root
`E/guarded-mlp-readiness40-reference-v228-v1`, where E is the existing finite
resident integration evidence directory. Stage exactly the source-manifest
members plus `source-manifest.json` into `source/`. Stage the six data bodies
at `inputs.json.locations` and the admitted environment as
`inputs/environment.json`. The original model target mounts read-only as
`/model`; the pinned v2 overlay mounts read-only as `/packages`.

The fresh launch plan uses schema `ferric-readiness40-reference-launch-v1`
and the same fields as the qualified owner's plan: image, current observed
topology, source_manifest, environment, overlay_root and overlay_manifest.
Do not reuse an old boot observation without checking current topology.
The root-owned invocation is `python3 -B source/launch.py PLAN_SHA256`.
This runs the exact CPU gate before container creation or GPU access.

The inherited owner retains 900 seconds inside the container, 1,200 seconds
operational cutoff and a separate 600-second retirement reserve, with a
1,800-second whole deadline. It retains two CPUs,64GiB memory,256 PIDs,
32MiB output,2GiB scratch and64MiB raw-evidence caps. The image is immutable,
network disabled, model/source/inputs/packages read-only, with a private
writable scratch. Only one GPU is exposed. Ownership recovery, natural
exit/stop/remove/absence, three before/after all-eight-GPU idle checks,
source/input/package/model posthashes and catchable whole-owner signals are
preserved. No cap is silently raised for this workload.

## Candidate Comparison Boundary

Actual Readiness40 terminal/capture bindings remain `None`; the framework
generation job does not read a candidate. A later bounded data-only caller
must authenticate the actual readiness owner/parent receipt and reuse its
qualified independent checker over all40 compact transitions, real Close,
and the four original control/payload captures before extracting payloads.
It must join full prompt, checkpoint/model/bundle IDs and all40 input tokens.

`reference.compare_selected` supplies the pure comparison step and explicitly
reports `receipt_authentication=False`. It rechecks the exact reference
repeat gate, all40 candidate input IDs and the four payloads' finite values
and own lowest-index argmax. It then reuses unchanged `compare_tensor` for
152 rows: all36 hidden layers, final normalization and logits at each of four
positions. Token mismatches remain diagnostic, not input-history divergence,
because this is prompt-only teacher forcing. No raw rank-partial/full-output
comparison is introduced. Neither selected equality nor a passing harness
is an independent numerical acceptance threshold or a throughput result.
