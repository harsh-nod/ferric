# Projection-Residual Four-Step GPU Decode

The new opt-in Qwen3-8B BF16 TP2 route completed four full 36-layer forwards on
`mi350`. Both residual stages selected the separately checked gfx950
projection-residual image. Original prefix, MLP, copy and tail images were
unchanged.

| Check | Actual Result |
| --- | --- |
| Teacher-forced input tokens | 9112, 2190, 3772, 220 |
| Observed output tokens | 67, 198, 25, 16 |
| Captured payloads | 4 x 606,976 bytes |
| Captured tensor slices | 152 |
| Prefix / MLP terminal state records | 288 / 288 |
| Native attempts / retries | 1 / 0 |
| Pre-run / post-run device audits | 3 / 3 |
| Owned process leaves | 7, all natural exit zero and reaped |

The parent and worker identities were observed by the outer process tracker.
All six recorded device process rosters were empty, idle samples and stable
VRAM checks passed, and no forced cleanup was required. Request/profile,
Control, transcript, finite payload, lowest-index argmax and consuming Close
checks passed.

The actual completion SHA-256 is
`5876cbde996253b77b30eb0b147710be1bc18f4c270177a4cfd28334ddbdef2f`.
[result.json](result.json) contains the source, image, executable and retained
file identities. [publish.py](publish.py) rehashed the 57-file retained case,
replayed its structural validator and recorded ownership/device checks, and
joined the actual CPU-qualified executables. Raw tensor buffers remain outside
Git.

## Scope

This establishes execution and structurally valid capture, not independent
numerical acceptance. The corrected arithmetic is intentionally not required
to equal the old native output. The independent framework comparison is a
separate CPU diagnostic.

This run used four teacher-forced positions, not a 2,048-token prompt followed
by 256 generated tokens. Its roughly 390-second outer wall time includes setup
and audits and is not a decode latency or throughput measurement. Calibrated
GPU timing, overlap, full-model acceptance and the 700 tokens/s target remain
open.
