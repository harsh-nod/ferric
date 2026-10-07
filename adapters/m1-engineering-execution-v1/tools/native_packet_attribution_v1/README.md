# Native Packet Attribution

This diagnostic retains Ferric's native649 prefill and native652 decode
programs, the qualified controller, and existing kernel images. Only the
default-off fe2o3 diagnostic worker changes. It runs one baseline TP1/C1
128-input/128-output request, not a latency benchmark.

`packet_observation.py` validates raw kernel-handle/timestamp records and binds
each execution to the existing wait-observation and controller counter/span
replays. It reports per-symbol and per-position tick totals, interval unions,
overlap and uncovered portions of each native program's timestamp window.
No tick-to-time conversion or shader-only attribution is made.

For a flat remote stage, include these five source files plus these unchanged
files from sibling `native_wait_attribution_v1`: `run_wait.py`,
`capture_binding.py`, `wait_observation.py`, `test_capture_binding.py`, and
`test_wait_observation.py`. `test_all.py` also resolves the sibling fixtures in
the repository layout. The capture plan binds every source byte, current
worker build receipt, CPU harness receipt, frozen controller ancestry and GPU
admission helper. Builds and CPU tests run only on mi300x-2; the supervised
native capture runs only on mi350.

Firmware profiling can perturb the device timeline. Packet intervals include
firmware-defined processing, not just shader instructions. Uncovered ticks
exclude time before the first start and after the final end; neither they nor
the overlapping CPU sleep counters establish recoverable latency. Exact token
parity, currentness, resource floors, sampled isolation and clean shutdown are
separate requirements, not implied by timestamps.
