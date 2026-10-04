# Conditional Final Norm and Output Head

This CPU-only replay on `mi350` checks the retained four-forward V7 TF4
captures against the original Qwen3-8B weights. It completed eight stage
diagnostics after **22 policy/layout tests passed**. No new GPU run was needed.

All **16,384 final-normalization BF16 words match exactly**. The output head
has 40 differing words among 607,744 logits, with matching argmax tokens at all
four positions. This is diagnostic evidence, not an arithmetic acceptance bound
or independent full-model correctness qualification.

## Method

Each stage receives its own captured immediate input: final normalization
uses the layer-35 hidden vector, and the head uses the captured normalized
vector, not the reference-predicted normalization. This separates local stage
differences from errors accumulated upstream. It is deliberately not a chained
end-to-end reference.

The replay uses unchanged independent P222 NumPy reference functions: FP64
normalization with two BF16 boundaries, and FP64 head projection followed by
FP32 then BF16 rounding. It
authenticates the original norm/head tensors in safetensors shards four/five,
checks their join to actual uploads, and streams the head's NxK-to-KxN transpose
hash in bounded chunks. It also replays the original GPU observation's lifecycle
and six audit records. Original model shards and all loaded sources are checked
again afterward.

## Results

| Forward | Norm exact / total | Head exact / total | Head max absolute error | Head max BF16 steps | Head relative L2 | Matching token |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 4,096 / 4,096 | 151,922 / 151,936 | 0.03125 | 7 | 0.00003821 | 67 |
| 1 | 4,096 / 4,096 | 151,921 / 151,936 | 0.0625 | 1 | 0.00004385 | 198 |
| 2 | 4,096 / 4,096 | 151,932 / 151,936 | 0.0625 | 1 | 0.00003153 | 25 |
| 3 | 4,096 / 4,096 | 151,929 / 151,936 | 0.0625 | 1 | 0.00003148 | 16 |

The numerical acceptance threshold remains `null`. In particular, the
seven-step result is neither automatically a bug nor proved acceptable.
Cancellation can make step counts or relative error misleading; a justified
head bound must account for absolute product sums, the actual MFMA rounding
and denormal contract, and the reference's own rounding error. No threshold
was fitted to these observed differences.

## Evidence

- [Actual diagnostic](complete.json) and [summary](result.json)
- [Bounded CPU receipt](cpu/complete.json), [22-test receipt](pure/complete.json)
  and [test log](pure/tests.log)
- [Replay source](source/run.py), [tests](source/test_run.py) and [runner](run_cpu.py)
- [Unchanged reference source](reference/model_reference.py) and
  [original source manifest](reference/manifest.json)

The replay used the existing NumPy 2.2.6 environment, CPUs 8/9, nice 10,
single-threaded numerical libraries, no visible GPU, a 2 GiB address-space
limit and 120 CPU-second limit. The wrapper measured 16.236 seconds elapsed;
that is reference-work timing, not kernel performance.

All four pure-test files and all 14 replay files were retained and rehashed
locally, including eight predicted BF16 payloads. Payload bodies and model
weights are retained in the task-owned evidence store, not published in Git.
The reference manifest and source README preserve their original draft-state
wording; the receipts here record subsequent execution.

The [new resident-state worker](../resident-state-decode-observation-v1/tf4/README.md)
produces the same 152 captured tensors as this retained V7 run. That invariance
does not convert these conditional diagnostics into full-model acceptance.
The sustained 2,048/256 benchmark, production admission and 700 tokens/s target
remain open.
