# Projection-Residual TF4 Framework Diagnostic

Draft data-only adapter, with 12 authored synthetic tests not executed by its
author. Root owns the bounded CPU invocation and the actual candidate GPU pin.
No GPU is launched, no numerical tolerance is introduced, and no mismatch is
converted into numerical acceptance.

## Existing Reference And Geometry

The genuine reference is the unchanged teacher-forced Qwen3-8B execution for
tokens `[9112, 2190, 3772, 220]`, with two independent fresh-cache repeats:

```text
E/framework-rearm-v224-v1/complete.json
8288 B, cac5d79969c2e17a19630a581b5e21c594ea65b452806630ee87bf7855396036
E/framework-rearm-v224-v1/reference/reference.json
128641 B, 2edddf40cc6195fdd622e479e8e46f2a659f1b569106f5f4e51afdb83bdc2416
```

`E` is `/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
Each framework and native payload is exactly 606,976 bytes, in the same order:
36 layer-hidden vectors of 4,096 BF16 values, final norm of 4,096 BF16 values,
then 151,936 BF16 logits. Four positions therefore give 152 comparable tensors.
Both trajectories retain their own KV state, but receive the same teacher-forced
input history. This does not represent a 2,048-token prefill or 256-token decode.

The unchanged reference reader checks both repeats, all 304 reference tensor
descriptors, all cache-hash pairs, actual reference subprocess exit and emitted
reference pin. Framework output tokens are `[67, 198, 25, 16]`; the adapter does
not require the candidate tokens or tensors to equal them.

## Reused Functions

The root loader must authenticate these exact bodies before calling the adapter:

| Role | Existing File Under E | SHA-256 |
|---|---|---|
| C | `p227-prefix-decode-gpu-qualification-v2/compare.py` | `8154580de7f5ad40fd4897ce264fc3d92c9a109808ea7ea5dab6d0486f01e622` |
| D | `p225-tiles-tf4-framework-comparison/helpers/diagnostics.py` | `38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf` |
| stage_core | `p228-projection-residual-decode-gpu-v1/stage_core.py` | `7e3d64e80e66bacbbba05ac3236189249949bb95ea3f717150822cf45740eec2` |
| smoke_validation | `p228-projection-residual-decode-gpu-v1/smoke_validation.py` | `9bf460451e722452da9c49168c730615d32e9b73b200c579f003e9e45238ea26` |
| V | `p228-projection-residual-decode-gpu-v1/decode_validation.py` | `b4e4cee33a9c82dd58189ca359e5a64d7e1dd1780e974e215c85c40e0df479e0` |

Load `stage_core` and `smoke_validation` as temporary aliases while loading V;
restore previous aliases afterwards. Do not call `C.helpers()`: its bundled
diagnostics module is AR-specific. Pass the actual TF4 D module explicitly.
`C.reference`, `C.compare_rows`, `C.lineage` and the strict FilePin readers are
unchanged. The new structural V helper has root-executed 35-test package evidence
`e3162fa0b19b7370c81acdc1a902eaf76a61fac2e33b2ccbb04b6f71ac53ac89`.

Use the existing root-owned Reader or C.Reader, never rewrite a candidate
request or receipt into an older schema:

```python
# actual_gpu_pin is supplied only after terminal GPU completion and retention.
# root verifies the complete candidate publication/admission and six idle audits.
value = adapter.compare_retained(actual_gpu_pin, framework_owner_pin, reader, C, V, D)
reader.recheck()
```

The adapter rehashes all 14 native files, replays V, binds the actual new selector
and source-profile/image-bearing request, and checks the seven owned leaves,
Close/transcript and recorded parent/worker lineage. Six topology/process leaf
records are rehashed and joined. It does not perform a fresh ASROCK GPU audit or
reinterpret an MI350 topology sample as current ASROCK state. Root retains the
original six-audit admission; `current_platform_idle_audits_verified` and
`source_binary_image_admission_replayed` explicitly remain false here.

Results retain all 152 rows, exact BF16 word counts, byte equality, first differing
element, maximum absolute error, RMSE, relative L2, and output-token comparisons.
The additional four per-layer trajectories describe error changes between
successive hidden vectors, including negative changes; no monotonicity or causal
explanation is inferred. Signed zeros remain distinct bits. Zero reference norms
retain the helper's explicit `None` relative-L2 case. All acceptance, performance,
production, conditional-residual and full-model-correctness flags stay false.

## Minimal Transport

ASROCK already retains the genuine reference; no recapture is needed. Verify all
eight `reference/pass{1,2}-pos{0,1,2,3}.bf16` files at 606,976 bytes each, plus the
reference complete/report and its execution `result.json`, `command.json`,
`started.json`, `stdout`, and `stderr` at their original pinned paths.

From the new MI350 case transport: `complete.json`, `observation.json`, all14
native files, all35 owned leaf files, and six topology JSON files. Also retain
the pinned plan, request, controller and package manifest at their original E
paths. No model weights, ELF, HSACO, compiler target or build-host toolchain is
needed by this adapter. The caller may use an authenticated original-to-retained
Reader map; the stored original paths and digests must not be rewritten.

## Synthetic Tests

With this proposal beside the listed dependency packages:

```text
python3 -B -m unittest discover -s E/p228-projection-residual-decode-comparison-v1 -p test_comparison.py -v
```

The test loader additionally pins
`p228-projection-residual-decode-gpu-v1/test_decode_validation.py` at
`37a24160ea948a8d19e9f5abac34d5407c239be7311c3e72efbe9074a026ff82`
to reuse its synthetic native records. It checks genuine parser/14-file routing,
owned-leaf refusal, full152 geometry, token divergence, signed zero, nonfinite
and extent errors, and shrinking layer errors. Synthetic receipt anchor constants
are patched only inside tests; no such override exists in the adapter API.
Run under the existing bounded CPU-only wrapper with source before/after hashes.
