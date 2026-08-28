# Qwen3 SwiGLU Worker V3 Qualification

This harness loads one exact published HSACO through HIP and compares 3,072
BF16 SwiGLU outputs against the stable FP32 reference. It is deliberately
outside the standalone device package and the production verifier/KFD path.

Build and run on a `gfx942` host:

```sh
hipcc -std=c++17 -O2 -Wall -Wextra -Werror hip_numeric.cpp \
  -o ferric-swiglu-hip-numeric
./ferric-swiglu-hip-numeric /path/to/exact.hsaco
```

For HSACO SHA-256
`57ecb86b40db136237e65a5fae04c955f2c92fe3347c085ec5c806984fc6afa7`,
the MI300X observation was:

```text
architecture=gfx942:sramecc+:xnack- elements=3072 exact=3072 max_ulp=0 mismatches_gt_1ulp=0
```

This is numerical qualification evidence only. It grants no production
verifier, load, KFD dispatch, full-model, or Qwen inference authority.
