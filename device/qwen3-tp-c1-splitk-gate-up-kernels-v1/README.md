# C1 Split-K Gate/Up Experiment

Default-off TP1/C1 gate/up kernels for Qwen3-8B, N12288 and K4096. Each partial
workgroup computes one M16/N16 tile over a K1024 partition. Only row zero is
stored. Four partitions create 3,072 Wave64 groups, followed by a 192-group
merge with increasing-partition FP32 addition and one final BF16 rounding.

The 192 KiB FP32 scratch can be reused sequentially for gate and up. Existing
authenticated KN weights can be reused without another model-weight allocation.
This changes reduction association relative to the current Wave kernel, so full
model token parity is a required gate. It is not a numerical-equivalence proof.

The kernels are written with fe2o3 and stay in Ferric. SDK dependencies are pinned
to 55c1a9b6. No new compiler/runtime capability, serving selector or default is
introduced. Source and geometry tests do not establish actual emission, native
correctness, bandwidth, TTFT or TPOT. All such measurements remain pending.
