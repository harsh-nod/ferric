# Five Q/K Exact-Dot Results

The bounded CPU-only diagnostic completed on MI350 in 4.07 seconds. All 18
named tests passed, with no failures, errors, skips or postcheck errors. This
directory retains all 17 original source, input and output files; this results
page is separate from those originals. The source README's pre-execution status
is deliberately preserved.

The five dots use identical captured normalized inputs on the native and
framework sides. Each contains 4,096 original BF16 checkpoint weights and is
summed exactly as an integer in units of `2^-266`, then rounded once to BF16
with round-to-nearest-even. No framework or native accumulation tree is modeled.

| Position | Rank | Projection | Checkpoint row | Native | Framework | Exact dot rounded to BF16 | Matches exact rounding |
| ---: | ---: | --- | ---: | --- | --- | --- | --- |
| 0 | 0 | Q | 168 | `b8c2` | `b8c3` | `b8c2` | Native |
| 1 | 1 | Q | 3950 | `b976` | `b977` | `b976` | Native |
| 2 | 0 | Q | 1462 | `3787` | `3788` | `3787` | Native |
| 3 | 0 | Q | 10 | `35d8` | `35d7` | `35d7` | Framework |
| 5 | 1 | K | 609 | `3663` | `3664` | `3664` | Framework |

Entries are BF16 encodings. None of the exact sums equals the midpoint between
the two observed values. These are the five differing scalars selected by the
prior comparison, not a claim that every Q/K/V output was checked against an
exact dot. In particular, the earliest observed difference does not identify
Ferric as the less accurate implementation at that scalar.

## Weight And Evidence Joins

The original 3,996,250,744-byte safetensors shard was streamed and hashed before
and after the calculation. Both rank-specific packed-QKV concatenations were
independently hashed from the same original file and matched the current native
Begin upload manifest. The full index, all selected original capture bodies,
all executed source bodies, row offsets and row hashes passed their checks.
The report does not claim a new device-memory weight readback.

An independent data audit recomputed every reported decimal, signed error,
adjacent-word midpoint and RNE classification from the stored exact integers
using rational arithmetic. It checked all 17 original retained files, the raw
18-test stream, and the source/input pins without importing the diagnostic or
recomputing model dots locally.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Original terminal | 20,880 | `0e859bb0c29d43d6f0b3018b8e9a58a912cf9c6e225242495d65cf8a0c41c881` |
| Raw test stream | 2,069 | `7cb1cd6d99951619392a5b1b3940dfeea4c8d821ff0b3bface068aca8e4fc826` |
| Original archive | 3,679,212 | `ec365c65587215ea6d15e0aa660b679bd2a95884ecbd356a7fec50bf8e9917e9` |

No kernel arithmetic changed. Matching or missing ideal once-rounded BF16 is
not itself a violation of a specified FP32 accumulation algorithm. The mixed
result does not justify forcing framework agreement, relaxing tolerances or
attributing the position-five argmax difference to these scalars. Attention,
full-model numerical acceptance, generated-token agreement and sustained
performance remain separate checks. No GPU or model forward ran in this CPU
diagnostic, and its elapsed time is not inference throughput.
