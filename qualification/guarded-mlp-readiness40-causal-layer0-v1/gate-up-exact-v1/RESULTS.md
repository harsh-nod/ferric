# Seven Gate/Up Exact-Dot Results

The bounded CPU-only diagnostic completed on MI350 in 4.479 seconds. All 20
named tests passed, with no failures, errors, skips or postcheck errors. This
directory retains all 18 original source, input and output files; this page is
separate from those originals. The source README's pre-execution status is
preserved.

At position 5, layer 0, both ranks' native post-normalized input is byte-exact
with the framework's post-normalized, Gate-input and Up-input captures. Each
selected dot uses that input and 4,096 original BF16 checkpoint weights. The
products are summed exactly as integers in units of `2^-266`, then rounded
once to BF16 with round-to-nearest-even. This is not an emulation of either
implementation's FP32 accumulation tree.

| Rank | Projection | Local row | Checkpoint row | Native | Framework | Exact dot rounded to BF16 |
| ---: | --- | ---: | ---: | --- | --- | --- |
| 0 | Gate | 553 | 553 | `bd86` | `bd85` | `bd86` |
| 0 | Up | 2575 | 2575 | `badd` | `badc` | `badd` |
| 0 | Up | 2603 | 2603 | `3cb2` | `3cb1` | `3cb2` |
| 0 | Up | 2840 | 2840 | `ba49` | `ba4a` | `ba49` |
| 1 | Gate | 2063 | 8207 | `3c50` | `3c51` | `3c50` |
| 1 | Up | 1719 | 7863 | `b846` | `b847` | `b846` |
| 1 | Up | 5310 | 11454 | `b8cb` | `b8cc` | `b8cc` |

Entries are BF16 encodings. Native matches exact rounding in six selected
cases; the framework matches in one. None of the exact sums is the midpoint
between the observed values. These seven differing scalars are not an exact
evaluation of every Gate/Up output and do not establish full-model correctness.

## Evidence Joins

The original 3,996,250,744-byte safetensors shard was streamed and hashed before
and after the calculation. Four rank/projection partitions, each 6,144 by
4,096 BF16 elements, were hashed from the checkpoint and matched the current
native Begin upload manifest. The original registration body is retained;
only session and child identity are rebound in memory to reconstruct the
authenticated current registration. Weight roles, ranks and geometry are not
rewritten. All source, index, capture, upload and registration postchecks pass.
No new device-memory weight readback is claimed.

A separate rational-arithmetic data check recomputed each reported decimal,
signed error, adjacent-word midpoint and nearest-BF16 classification from the
stored exact integers without importing the diagnostic or recomputing model
dots locally. The archive and all 18 original bodies passed retention readback.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Original terminal | 28,075 | `2da4e9cdd79aef60ff3d9eb67c6062a6a32dabc8d42d3d711cea33dc97173b6b` |
| Raw test stream | 2,403 | `f5b49091122ff1caedb674db7d30bc7d02ec6a6df3519765f93e2516ec3b2176` |
| Original archive | 3,689,664 | `0a72166bc911ada4b7562956056972a33b7163761e5c06cddc7331eb232d719e` |

No kernel arithmetic changed. Missing ideal once-rounded BF16 is not itself a
violation of a specified FP32 reduction algorithm. This mixed result does not
justify forcing framework equality, relaxing tolerances, changing SiLU or
attributing the position-5 argmax difference to these scalars. Attention,
down-projection, accumulated full-model error, generated-token agreement and
sustained performance remain separate checks. No GPU or model forward ran in
this diagnostic; its elapsed time is not inference throughput.
