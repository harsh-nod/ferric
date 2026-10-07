# Matched K16 Prefill Experiment

Default-off two-root engineering image for comparing original K16 MFMA loads
with paired adjacent K16 loads. Both BF16-output kernels use the same compiler,
SDK, guards, layouts, grid, ordered accumulator updates and final BF16 rounding.
The candidate issues both fragment loads before its two MFMA updates; it does
not implement a cross-iteration asynchronous pipeline or reduce weight bytes.

Enable only with `--no-default-features --features paired-prefill-k16-r1`.
The original 15-root V5 crate and its full `prefill-mfma-k2` feature remain
unchanged. That full feature's current emission failed race-analysis storage
bounds in its FP32 partial root. This independently declared two-root image
does not replace, qualify or remove any root from that rejected V5 artifact.
All normal verification checks remain enabled.

Source fixtures retain the original BF16 functions. Tests check exact source
equivalence apart from export names and feature placement, paired load/update
order, bounds, rejection mutations and a host arithmetic-order model. These
checks do not emulate MFMA or establish device parity or performance.
Native component measurement and full-model token/TTFT/TPOT validation are
required before any serving integration or default change.
