# Indexed RoPE Checked Emission

This engineering checkpoint emitted a new gfx950 prefix image on ASROCK
through `ssh mi350-2`. All eight staged phases completed naturally with exit
zero; all owned processes were reaped and source/input postchecks passed.
Sixteen controller-policy tests also passed. This is not GPU execution,
numerical acceptance, production admission or a performance result.

## What Changed

The qualified indexed inert join consumes the actual retained RoPE/RPO
compiler handoff without raising work or storage limits. The continuation
built the finalizer and metadata tools from the qualified V2 source, passed
the explicit actual-handoff join, and emitted the image below.

The original compiler attempt remains a failed aggregate: its checked
lowering and replay succeeded, then its original inert join refused the work
budget. Those successful producer leaves are authenticated here, not rerun.
The separately qualified indexed consumer clears that refusal. This record
does not claim a fresh compiler build or a fresh full compiler cohort.

## Results

| Stage | Outcome |
| --- | --- |
| Qualified Cargo metadata | Pass |
| Consumer tool build | Pass |
| Actual retained-handoff inert join | 1 passed, 0 ignored |
| Checked HSACO emission | Pass |
| Retained formal/LLVM extraction | Pass |
| Descriptor metadata | Pass |
| ELF notes | Pass |
| Disassembly | Pass |

The image is [artifact.hsaco](artifacts/emitted/artifact.hsaco), 54,344 bytes,
SHA-256 `29fd58e7b09fee003ed31660e6201c20a7eb5da4b873d0b4d954993f7018f2f8`.
Its target is `gfx950:xnack-`, code object version 6. The descriptor records
120 explicit argument bytes, a 376-byte kernarg segment aligned to 8 bytes,
64-thread workgroups and at most 64 workgroups. ELF notes report Wave64,
512 bytes of fixed LDS, zero private-segment bytes, 132 VGPRs, 106 SGPRs,
16 scalar-register spills and zero vector-register spills. These resource
counts are not a speed measurement or evidence of overlapping execution.

The owner completed in 105.300 seconds, including build and inspection work.
That is host-controller elapsed time, not GPU latency or tokens per second.
Eight runtime requirements remain unresolved. No launch authority follows
from this engineering emission.

## Evidence And Next Gate

[result.json](result.json) joins the actual terminals, raw phase records,
artifacts, immutable prior publications and the primary-agent observation of
the sixteen policy tests. [complete.json](complete.json) and
[owner-complete.json](owner-complete.json) preserve the original receipts.
The published fixture and five formatted consumer Rust bodies are exact
copies of their recorded inputs. Full source maps and tool/test executables
were rehashed remotely; their bodies are not included or replayed locally.

Next: admit this exact image into the existing four-step autoregressive
runtime on MI350, then compare its actual trajectory and tensor slices with
the independent framework reference. Token equality alone will not establish
numerical acceptance. The sustained BF16, single-request Qwen3-8B
2,048-token prompt / 256-token decode workload and 700 tokens/s target remain
unvalidated. All issue #42 M0-M7 acceptance milestones remain open.
