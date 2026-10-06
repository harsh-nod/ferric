# Ranked CFG Diagnostic Loader Audit

On 2026-10-06 UTC, the compiler deployment with the qualified ranked CFG
diagnostic passed all fourteen loader inspections on MI350. All leaves
exited naturally with zero status and were reaped with absent process groups.
Input, tool and library postchecks were clean. Whole-run time was 2.366144
seconds; this is loader inspection time, not GPU performance.

The [diagnostic compiler qualification](../guarded-mlp-ranked-cfg-diagnostic-v1/README.md)
supplies the final backend and extractor. The extractor body is byte-identical
to its predecessor, but its source and final Cargo provenance are refreshed.
The driver and other four tool entries exactly match the
[previous deployment](../guarded-mlp-s-rpo-tool-audit-v2/README.md).
The actual extractor resolves the exact new backend. No production dependency
or GPU execution path changes in this checkpoint.

## Checks And Evidence

| Check | Observed Result |
| --- | --- |
| `readelf -l -d` for seven tools | 7 passed |
| `ldd` and resolved dependency identity for seven tools | 7 passed |
| Diagnostic source, full-suite and final Cargo joins | Passed |
| Earlier compiler and driver provenance | Preserved |
| Five unchanged tool manifest entries | Exact match |
| Separate synthetic loader-admission fixtures | 24 passed |

The new admission checks require the exact four-file diagnostic overlay,
all 33 qualification phases, the earlier named test outcomes plus eight new
diagnostic tests, and the final Cargo products. They preserve the separate
driver proof, which covers 18 focused tests rather than its full compiled
402-name inventory. Synthetic fixtures do not execute a compiler or inspect
real loader dependencies.

[attempt-v1](attempt-v1) retains 92 original bodies and its retention manifest,
including all 72 raw records and the producer readsets. Receipt: 131,725 bytes,
SHA-256 `859184e97b9479fc4070b0e4ea4b821e87f9e2dfebeb980eb2e1235a751f952a`.
The new seven-tool manifest SHA-256 is
`c4d4cf4cd61fc3e39ec2a8c52c9a8c4685b73f6c6fe29f809a434368ddc522bd`.
All 93 archive members and 18,036,974 body bytes were verified after transfer.
Binary bodies are not included in the retention.

[controller-tests-v1/attempt-v1](controller-tests-v1/attempt-v1) retains the
separate 24-fixture run: one natural zero/reaped child, unchanged sources and
clean postchecks, 6.566985 seconds whole. Receipt SHA-256:
`5180c5ab1772398a328dec1d19145ea61abd6a572bd21d7b4a86e14aa5dac6b0`.

The [actual guarded lowering retry](../guarded-mlp-ranked-cfg-lowering-v1/README.md)
now identifies the rejecting function and block count. Loader success does
not qualify an HSACO, GPU launch, model numerics or the 700 tokens/s target.
All issue #42 milestones remain open.
