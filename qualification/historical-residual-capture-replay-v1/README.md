# Historical Residual Capture Replay

On 2026-10-04, a bounded CPU-only replay on `mi350` checked both TP2 residual
boundaries in the retained V227 layer-0, position-0, token-9112 GPU captures.
The independent reference matches all 32,768 BF16 output words exactly,
including signed zero. No GPU was launched by this replay.

| Historical profile | Post-attention words | Post-MLP words | Result |
| --- | ---: | ---: | --- |
| Baseline | 8,192 | 8,192 | Exact |
| Candidate | 8,192 | 8,192 | Exact |

Each count includes both 4,096-element rank outputs. Both captured profiles
happen to be identical; each is separately checked against the arithmetic
reference, not accepted because the two profiles match one another.

The input embedding row is extracted from the authenticated original model
shard, not inferred from the residual output. One streaming pass hashes the
whole 3,996,250,744-byte shard and the actual uploaded 1,244,659,712-byte embedding
tensor. The prompt's first token is independently pinned to 9112. Both complete
9,670,656-byte captures and all 28 slices per profile are authenticated.

The [unchanged reference](../independent-residual-reference-v1/README.md) applies
ordered, separately rounded FP32 additions followed by one BF16 nearest-even
rounding. The accepted first residual supplies the second stage's skip input.
Upstream O and MLP-down partials are conditioning inputs, not validated here.

## Evidence

- [14 passing policy/layout tests](pure/complete.json), [transcript](pure/tests.log).
- [Actual CPU replay](replay/complete.json), [full numerical result](replay/residuals.json).
- [Data-only CLI](source/run.py), [tests](source/test_run.py), and exact CPU runners.
- Before/after source maps agree. The successful replay took 3.613 host seconds;
  that is CPU validation time, not kernel latency.

Root launched both runners with `timeout --kill-after=10s 150s taskset -c 8,9
nice -n 10`, Python `-B`, unset PYTHONPATH/PYTHONHOME and empty GPU visibility.
Internal caps were 2 GiB address space, 120 CPU seconds, 16 MiB per output file
and no core dumps. The test mode was `tests residual-capture-pure-v228-v1`;
the successful replay was `replay residual-capture-replay-v228-v2`.

The first replay stopped before model arithmetic because the root runner used
the model directory instead of its `target` shard subdirectory. Its original
source and partial source snapshot remain retained; V2 changes only that path.
No threshold, capture, reference, GPU image or arithmetic policy was changed.

This is historical conditional residual evidence. It does not validate V7,
MLP computation, all-layer/model numerics, runtime proof premises, sustained
2,048/256 decoding or 700 tokens/s. Original historical failure records remain
unchanged. Captures and checkpoint weights are retained outside Git.
