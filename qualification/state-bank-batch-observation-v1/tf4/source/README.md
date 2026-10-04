# State-Bank TF4 Comparison Draft

Data-only successor to the actually tested
`p228-resident-state-runtime-comparison-v2`. It launches no native process,
compiler or GPU. The baseline is the retained CPU522 worker observation
`prefix-resident-state-decode-tf4-shared-full-currentness-gpu-v228-v1`, whose
complete receipt SHA256 is
`f8e36cf2ed13435e7fb52b3a691f15f4135565d444038ee0d91292d94225fcba`.

The candidate is a separately completed state-bank TF4 observation. Its frozen
observer package SHA is bound to the actual frozen 29-file generation
`91fbcfa425e005342e2b7373b2a1ea1f9ee44aa253d96aa89b681d87e85dca9a`.
The worker tuple
is bound to actual CPU553 completion
`14dab6e776cdc2184457b2865cf7d48b91a71008d49bea040deca68bba75838f`:
4797496 bytes, SHA256
`6901a1319352d0401e452d616efa50d2de08c40943f50cb054d89f3f26d7fd52`.
The observer name/schema and private import
order follow `p228-state-bank-batch-observation-v1`. No candidate GPU result is
invented by this draft. There are 25 authored synthetic tests, not executed here.

## Checks

- Rehash both exact observer packages, completed one-attempt/no-retry receipts,
  retained native leaves, six recorded audit leaves and full observations.
- Reuse the frozen structural/native-close reader and each actual observer's
  input and observation checks. This does not replay current platform audits,
  reestablish historical runtime authority, or launch anything.
- Require unchanged CPU633 parent, V7 image, request fields apart from worker,
  fresh session and evidence directory, and unchanged numerical prerequisites.
  The candidate's prior deployment must be the exact CPU522 deployment.
- Rebind each forward's profile, registration and session to its own run.
  Cross-run profile hashes must not be compared because fresh sessions,
  registrations and child PIDs contribute to them. Commands, device mappings,
  protocol and IDs still must agree across runs.
- Compare all four full 606976-byte captures, four token records and all
  152 structural tensor slices byte-for-byte. Any differing tensor, record or
  complete payload is retained as a failed invariance result.
- Require exactly 572 fewer full group checks for every forward, exactly 288
  publication checks on both sides, and unchanged rank operation counts.
  Completion polls and timing counters may vary.

The expected group decrement follows the source change: two scans of 144 state
objects previously made two full checks per object (576 checks). Two bank scans
now each retain a fresh entry and exit check (four checks), removing 572. The
outer ledger fences and all 144 individual rearm calls are unchanged. Actual
GPU counters must demonstrate that decrement, including reused banks on the
third and fourth forwards; source arithmetic is not an observed result.

Host durations and their ratios are descriptive inclusive host measurements
from one run per worker. They are not GPU time, a qualified speedup, sustained
throughput, independent numerical acceptance or full-model correctness.

## Root-Owned Execution

Review and freeze all three source files, and run the
synthetic tests under the existing bounded CPU runner. No manifest is authored
by this draft.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest -v test_run
python3 run.py \
  --candidate-complete /absolute/prefix-state-bank-batch-tf4-shared-full-currentness-gpu-v228-vN/complete.json \
  --candidate-sha ACTUAL_COMPLETION_SHA256 \
  --output /absolute/fresh-state-bank-comparison-output
```

The default frozen diagnostic reader is
`E/p228-independent-decode-diagnostic-v1/run.py`, SHA256
`259f6f233be23da36eac213bfb5e0c905afa43461461fbb0305efcc45c08d930`.
Both original observer packages and their already-retained input/artifact files
must remain available at their authenticated paths. The comparator reuses
their closed readers rather than relaxing receipt schemas or treating a prior
failed comparison as success.

Successful replay writes source snapshots, a host-only Markdown table and a
`ferric-p228-state-bank-runtime-comparison-v1` result. A completed comparison
with any invariance mismatch writes that result with `passed=false` and returns
exit one. Authentication or structural failures raise and must be retained by
the root-owned bounded wrapper; they cannot produce a successful result.
