# Census V3 / Tail V4 Native Pair

Both Qwen3-8B BF16 TP2 cases passed on MI350 using the same qualified worker
and parent binaries. Each consumed 40 authentic prompt positions through all
36 layers, generated zero tokens, and completed one native attempt. All 40
semantic records and four complete 606,976-byte payloads match each other
and the retained historical baseline. No independent model acceptance is implied.

Each case completed all 11 supervised phases naturally, with no cleanup
signals, owned processes reaped and owned groups absent. Six all-eight-GPU
idle observations and exact topology checks were retained per case. The cases
used fresh sessions and processes on the same boot, with Census completed
before Tail. Both retain healthy Close and the unchanged resource/deadline gates.

| Original terminal | Bytes | SHA-256 |
| --- | ---: | --- |
| Census | 467223 | `10241786d223ff64803074856728cb85eefe5f18fc0925b3e854e717ecafe404` |
| Tail | 665487 | `d43ed2c61c07c0714bf31cb258ca683a114458fd01f87ef32b398b43045ba9d6` |

The unchanged [original archive](../matched-timing-gpu-v2.tar.gz) is 6,798,966
bytes, SHA-256 `2e95fedc277038899bd4be7fb7421d828cbbda74b55ec235abf1f25fe323e294`.
It contains 244 originals and expands to 14,104,414 bytes; its manifest pins
the other 243 bodies. These include 20 root sources/inputs, 147 current raw
bodies, both case terminals, 73 historical baseline bodies and the exact
collector. This README is separate commentary, not an original archive member.

Independent data-only review rehashed all originals and checked both owned
lifecycles, policy counters, transcript/capture parity and historical joins.
It did not rerun a model or execute the evidence helpers.

The [measured report](../matched-timing-report-v2/README.md) keeps all timing
categories and regressions. This pair does not execute Full2303, generate
accepted tokens, establish GPU overlap or demonstrate 700 tokens/s.
