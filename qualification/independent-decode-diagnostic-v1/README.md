# Retained All-Layer Tensor Diagnostics

This data-only reader passed [12 CPU tests on MI350](pure/complete.json), with
no skips or failures, then compared the actual V7 four-forward captures to the
independent framework reference. It launches no GPU work and defines no model
acceptance tolerance. [Tests](pure/tests.log), [source](source/run.py).

The reader authenticates the completed observation, all seven owned leaves,
six retained audit records, complete native buffers and typed Close evidence.
It replays the existing capture validator and calls the unchanged, mode-specific
framework comparator. It does not reconstruct deployment authority or claim
that historical audit bytes are new idle-device checks. Before/after source
snapshots agree.

For autoregressive comparisons, a diverged input history makes every subsequent
position incomparable. Such rows retain null tensor diagnostics; a coincidentally
matching later token cannot restore comparability.

The first actual [TF4 result](../independent-decode-observation-v1/tf4/README.md)
contains all 152 tensor comparisons. All four argmax tokens match, but all 152
complete tensor rows differ in at least one BF16 word. This is a diagnostic
result, not a numerical acceptance or performance claim.

Root ran the tests and the separate real-capture invocation only after the GPU
controller was terminal and reaped. Both used CPU affinity 8,9, nice10, Python
`-B`, empty GPU visibility, 2 GiB address space, 120 CPU seconds, 16 MiB file
limits and a 150-second outer timeout. The [exact test runner](run_tests.py)
pins the three source members. Source author-stage notes are preserved; the
actual receipt supersedes their prior unrun status.
