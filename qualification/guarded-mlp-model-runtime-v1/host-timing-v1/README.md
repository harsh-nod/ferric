# Guarded Host Dispatch Counters

The [MI350 retained-data analysis](complete.json) validates the existing AR4
observation and decodes its four binary controls. All four parser tests pass;
19 input files are hashed before and after analysis. No GPU work is launched.
Receipt SHA-256:
`aa33880786a660f11d3b5da0f8e81e92781c4fb996f7bd362258f1ce1c9a5360`.

These are **host wall-clock dispatch spans**, not GPU timestamps, token latency,
or an overlap trace. Each entry below sums the corresponding counter across
36 layers. Rank intervals can overlap and must not be added together.

| Position | Prefix Rank 0 (ms) | Prefix Rank 1 (ms) | Guarded MLP Segments (ms) |
| ---: | ---: | ---: | ---: |
| 0 | 589.967424 | 584.030199 | 6263.102003 |
| 1 | 588.967116 | 585.145773 | 6249.552985 |
| 2 | 588.224755 | 587.222544 | 6256.530819 |
| 3 | 589.458625 | 587.737416 | 6307.226986 |

The guarded segment timer encloses preflight, kernel/buffer admission,
packet preparation, currentness checks, publication, completion polling,
retirement and terminal-state checks. It does not isolate device execution.
Model setup, metadata uploads, state-bank rearming, intermediate readbacks and
other work outside the timer prevent using these sums as full-forward latency.

The next profiling experiment should enable the runtime's existing host
observer before allocation and record checked counter deltas around forwards
and layers. This can separate currentness, admission, reads and writes without
removing any safety check or changing the GPU arithmetic. Only measured
attribution should guide which host costs to reduce.

The exact analysis sources and test output are retained here. Reproduction
requires the original MI350 evidence paths pinned in `run.py` and a fresh output
directory; do not overwrite the completed attempt. This result neither changes
[numerical acceptance](../numerical-v1/README.md) nor establishes sustained
2048/256 decoding, vLLM-relative speedup, GPU overlap or 700 tokens/s.
