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
not evidence of slower GPU arithmetic.

Source inspection localizes the additional work to the pre-layer bank reuse
path. The [bank ledger](../../../adapters/tp-peer-finite-engineering-worker-v1/src/state_roster/guarded_mlp_decode_v1.rs)
uses two banks; forwards 2 and 3 rearm an already completed 36-layer bank.
The retained runtime's reset path adds ten context fences per entry and one
final bank fence. Conservative fences perform two full checks per rank,
giving `36 * 10 * 2 + 2 = 722` additional full checks per rank. This matches
the raw forward-begin to layer-zero-begin count increase exactly: rank 0
goes from 46 to 768, rank 1 from 44 to 766. That interval rises from
0.154-0.155 seconds to 2.572-2.597 seconds; the post-layer tail remains about
0.170 seconds. The source accounting identifies validation associated with
bank rearm, not queue rollover, and does not justify removing its checks.

These are host diagnostics with setup, process gaps and nested counters.
Neither these tables nor their reciprocal establish tokens/s, device overlap,
a speedup, or the requested sustained 2,048/256 result. The qualification
source README is retained unchanged and describes the proposal before root
execution; this page records the actual result.

The tested [runner](source/run.py), [arithmetic](source/analyze.py), and
[ten tests](source/test_analysis.py) are retained. Original inputs are the
adjacent [GPU evidence capsule](../gpu-attempt-v1/). The runner accepts that
input root and a fresh output directory; the same pinned bodies are required.
