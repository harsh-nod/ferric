# Guarded Model Observation Validator

All **nine synthetic tests pass on MI350**, with zero failures, errors or
skips. The single test process exits naturally and is fully reaped; source,
tool and output checks pass. This validates a data checker, not GPU execution
or the numerical correctness of Qwen.

The [validator](validate_observation.py) checks the four-forward TF4/AR4
observation format independently of the Rust encoder. Its tests cover:

- Complete teacher-forced and own-output autoregressive records.
- All 36 layer states, terminal guards, bank generations and queue frontiers.
- Exact payload sizes and finite BF16 values in every captured partition.
- Lowest-index argmax, including signed-zero ties.
- Duplicate JSON keys, wrong integer types and invalid claim fields.
- Raw-body hashes, transcript continuity and clean Close records.

The test fixtures do not compute Qwen logits. Synthetic token trajectories
must not be presented as native model outputs or an independent model
reference. Neither these tests nor their 2.90-second host duration support a
kernel-performance claim.

The [actual receipt](evidence/complete.json), seven original evidence files,
controller, supervisor and both tested source files are retained unchanged.
The run uses CPUs 8 and 9 at nice 10, a 512 MiB address-space cap, a 120-second
test deadline and a 180-second whole-run deadline. GPU visibility is disabled.

Receipt SHA-256:
`75701c2ef8a3b50fb5455b5702909fb7fd21b3d34cbae7942a8140767afb4c9a`.
Actual model TF4/AR4 execution and independent numerical comparison remain
pending.
