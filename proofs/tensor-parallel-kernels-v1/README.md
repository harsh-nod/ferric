# Engineering TP Kernel Checks

This directory contains engineering-only checks for the standalone
`device/qwen3-tp-kernels-v1` crate. It does not extend the protected aggregate's
admission, issue a certificate, or prove floating-point numerical equivalence.

`prepare-vendor.py` prepares the reviewed Cargo workspace/checksum overlay for
the exact pinned public SDK. It checks both TP dependency pins, clean Git
repositories, and byte-identical SDK Rust sources. It changes only the vendored
device manifest/checksum and adds the existing workspace template; no SDK Rust
body is changed.

`probe.py --self-test` and `test-probe.py` are host-only. The latter checks
malformed frame and payload rejection, mismatched responses, bounded kernarg
allocation, truncated pipe handling, and termination/reaping of an owned dummy
Python child. Neither opens a GPU.

Real execution requires explicit `--run`, a canonical worker ELF and artifact
ELF with exact SHA-256 values, a device unique ID, and a new absolute output
directory. The worker executes a held inode and its PID/start time/executable
identity are checked. Protocol operations have byte and time bounds. Each input
and output has 64-byte guards on both sides, every input is reread unchanged,
all output bytes are checked, and successful close/reaping is required.

The six fixtures exercise Qwen3-8B TP8 geometry: column GEMV, FP32 partial GEMV
retaining `1.00390625`, zero-input SwiGLU, identity-position RoPE, KV append
preserving other rows, and attention ignoring a stale NaN cache suffix. These
are numerical smoke checks, not model inference or comprehensive numerical
qualification. Reported dispatch nanoseconds are synchronous worker wall-clock
measurements, not TTFT, TPOT, or a model benchmark.

Run builds and host tests only on the designated build host. Coordinate hardware
use, check GPU availability immediately before execution, and remove owned
stages after archiving evidence. No external inference backend is invoked.
