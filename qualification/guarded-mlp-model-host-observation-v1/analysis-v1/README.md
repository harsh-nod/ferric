# Actual Host Accounting

The bounded data analysis completed on MI350 with ten passing synthetic tests,
eight clean input posthashes and no errors. It launched no child process or GPU
work and did not repeat the native run. The original result is retained in
[complete.json](complete.json): 3,313 bytes, SHA-256
`69b0322936444bde9620abbe5cffed89d36dcb6283be6fa94e216364e60390f0`.

See [the measured tables](summary.md) and [exact integer data](analysis.json).
Each forward's 145 intervals are partitioned into 36 prefix, 36 paired,
36 hidden-read and 37 other intervals. All four brackets and the six outside
intervals reconcile exactly to the snapshot wall. Close is outside that series.
Rank and shared counters telescope independently and are never added to the
wall partition. The raw report, checked rows, native completion and source
controller are authenticated against the original native terminal.

The paired segments contain 2,016 full-currentness checks per rank per forward.
The first two full forward brackets are 12.808 and 12.870 seconds, versus
15.253 and 15.384 seconds for the next two. Most of that change is within
the other category, not the prefix, paired or hidden-read categories. This is
an observation to investigate, not evidence of slower GPU arithmetic.

These are host diagnostics with setup, process gaps and nested counters.
Neither these tables nor their reciprocal establish tokens/s, device overlap,
a speedup, or the requested sustained 2,048/256 result. The qualification
source README is retained unchanged and describes the proposal before root
execution; this page records the actual result.

The tested [runner](source/run.py), [arithmetic](source/analyze.py), and
[ten tests](source/test_analysis.py) are retained. Original inputs are the
adjacent [GPU evidence capsule](../gpu-attempt-v1/). The runner accepts that
input root and a fresh output directory; the same pinned bodies are required.
