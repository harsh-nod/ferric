# Ordered Wave Attention Candidate

This explicitly selected engineering combination uses the existing v5 wave
attention kernel inside the existing ordered attention/FFN packet groups.
Select `--attention wave --runtime-ordered-batches` with TP1, `v5-mfma32`, MFMA
projections, `fp32-v8`, device TP1 residuals and output-head pruning. The legacy
physical pool remains required. Defaults, kernel source and fe2o3 are unchanged.

Configure attention before the FP32 head and configure ordered batches last.
Afterward neither attention mode nor other frozen execution policies can change.
Each group still uses its original packet dependencies, per-packet completion
checks and bounded aggregate wait. The TP1 residual is now the final packet in
each attention/FFN group (11/6 packets), with collective advancement and handle
swaps only after completion. Head and embedding calls remain synchronous.
Large KV, peers, legacy sequences, numerical capture and replica
control remain excluded from this combination.

Host recording tests compare every command, argument, write, read and synthetic
choice against serial execution separately for baseline and wave attention,
including 1/16/17/32 rows and empty/selected/all output-head rows. These are not
GPU arithmetic checks. The earlier independently tested wave and ordered paths
do not qualify their combination; native model/reference and performance
comparisons are required before any adoption or speedup claim.
