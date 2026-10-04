# Retained RoPE AR4 Comparison

Source-only draft. Eighteen focused synthetic tests are authored, not executed
by the author. No future native completion, tokens or numerical result is
presumed. The root owns review, source freeze, CPU tests and actual execution.

## Inputs and Scope

This CPU-only adapter compares a completed linked-RoPE AR4 observation with the
already retained genuine independent framework chain. It loads only the three
unchanged data validators from the frozen `p228-rope-indexed-ar4-gpu-v1`
package and the byte-identical AR4 diagnostic helper. It never imports the
native supervisor, intake, framework launcher, Torch or Transformers, and
never starts a process, model or GPU operation.

The actual reference is `projection-ar4-framework-reference-v228-v1/reference.json`
(201239 bytes, SHA256 `00952244362ad51d241d179f741ae5ae61ff8acfcb3fd160b9dc64ce3b5f699e`),
joined to its completed owner `94403d351120c0eb756f5660e333682ca0f47c4c6c1ac0f3cf6e9c10db896771`.
Both genuine fresh-cache passes are read, all eight 606976-byte payloads and
304 tensor slices are hashed, own lowest-index argmax is checked and repeat
bytes are compared. The reference has no conditional-input branch.

The old native baseline is the actual `15938580d218f855883a589c819d532bbf941a02c4677cb29928f7bf7106d1cb`
AR4 completion. Its four payloads, as well as the new four payloads, are
revalidated with the unchanged AR4 parser, including all Control/profile/chain/
Close fields. The outer receipts and requests are pinned separately. Only
prefix image, fresh session and output directory may differ between requests;
CPU1037 parent/worker identities and model, prompt, SiLU tiles, original Begin
images and projection residual remain identical. The new image metadata must
join actual linked emission `d45dd1a2...` and HSACO `29fd58e7...`; those image and
compiler bodies are not read or requalified by this numerical adapter.

## History and Metrics

`diagnostics.py` is an unchanged copy of
`p225-tiles-ar4-framework-comparison/helpers/diagnostics.py`, SHA256
`645f11391b2255b7b93a8e7f0372114ad9700a0037e718bb48677d2234b926e7`.
Its existing `compare_four_forwards` checks seed9112 and each independent
own-output recurrence. Tensor comparisons require identical entire input
histories through the current position. Once a history differs, comparability
never recovers even if a later input token happens to match. A different
current output does not invalidate a comparison with the same current input
history; it affects subsequent positions.

The result retains 152 tensor slots (36 layer-hidden, final norm and logits at
each of four positions). It recomputes old-native and candidate max absolute
error, relative L2, RMSE, exact BF16 words and BF16-step diagnostics against the
same genuine reference. Candidate-minus-baseline deltas exist only where both
native histories match that reference. Incomparable positions have null
candidate metrics/deltas and `needs_conditional_reference=true`. This is not a
forced-token replay and does not manufacture a conditional reference. A zero
reference norm with nonzero error keeps relative L2 null. There are no fitted
tolerances, acceptance thresholds, causal or performance claims.

## Minimal Transport

On MI350 the old native completion, request and fourteen `native/` bodies
already exist. Retain the corresponding sixteen new native bodies after the
root observes actual completion. No raw ELF, weights or compiler snapshots
are needed. The only independent-reference courier consists of ten files:

- Original framework owner `projection-ar4-framework-launch-v228-v1/complete.json`.
- Original `projection-ar4-framework-reference-v228-v1/reference.json`.
- `genuine-ar-pass{1,2}-pos{0,1,2,3}.bf16` from that reference directory.

These are retained under the same relative paths in the root's
`W/projection-ar4-framework-evidence-v228-v1`. The root is transporting them to
MI350 `E/rope-indexed-ar4-reference-inputs-v228-v1/` with flat names. An explicit
original-path-to-retained-FilePin map preserves their original identities:
owner maps to flat `complete.json`, reference to flat `reference.json`, and
the eight payload basenames are unchanged. Derive payload pins from the
authenticated reference's `genuine_passes[*].cases[*].payload`; preserve bytes
and SHA and change only each retained path. JSON is read by Python, never
floating-point tooling that could truncate GPU uint64 identities.

The root-authored input JSON has exactly `schema`, `native_complete` and
`transport`. Schema is `ferric-p228-rope-indexed-ar4-comparison-inputs-v1`.
`native_complete` is the original future completion FilePin, supplied only
after successful capture. `transport` maps original absolute paths to actual
retained `{path,bytes,sha256}` records. Native aliases may also be supplied if
necessary. Every declared alias must actually be consumed and must preserve
the original bytes/SHA. All consumed source and data files are rehashed at exit.

## Root Execution

Install this five-body package plus its root-reviewed manifest under
`E/p228-rope-indexed-ar4-comparison-v1`. The manifest schema is
`ferric-p228-rope-indexed-ar4-comparison-package-v1`; `files` contains exact
`{path,bytes,sha256}` rows for this README, run.py, comparison.py, diagnostics.py
and test_comparison.py. The three validator bodies remain in the already
frozen GPU package; do not import or copy its live supervisor.

First, root runs the eighteen synthetic tests in the package directory:

```sh
env -u PYTHONPATH -u PYTHONHOME -u PYTHONOPTIMIZE HIP_VISIBLE_DEVICES= ROCR_VISIBLE_DEVICES= CUDA_VISIBLE_DEVICES= timeout 360s taskset -c 8,9 nice -n 10 python3 -B -m unittest -v test_comparison
```

Then root runs the actual data adapter with the observed input and package
SHA256 values (replace E with the actual evidence root):

```sh
env -u PYTHONPATH -u PYTHONHOME -u PYTHONOPTIMIZE HIP_VISIBLE_DEVICES= ROCR_VISIBLE_DEVICES= CUDA_VISIBLE_DEVICES= timeout 370s taskset -c 8,9 nice -n 10 python3 -B E/p228-rope-indexed-ar4-comparison-v1/run.py INPUTS_PATH INPUTS_SHA PACKAGE_MANIFEST_SHA rope-indexed-ar4-comparison-v228-v1
```

The actual adapter enforces MI350 host/UID9661, CPU8/9, nice10, hidden devices,
2GiB address space, 300 CPU seconds, 360 wall seconds, 16MiB output-file limit,
zero core files, 4MiB per input, 128 unique inputs/64MiB aggregate and a fresh
output directory. It emits one `complete.json` or `failure.json`. Completed
means data processing and postchecks passed, not numerical acceptance.
The caller-pinned package manifest joins all five source bodies; source and
transport originals/retained identities are included in the output ledger.

Full owner process/audit trees, model/library bodies, compiler images and
transitive admission are not replayed. Their already completed pinned receipts
remain historical provenance. This report cannot establish 2048/256 behavior,
full-model correctness, throughput or production authority.
