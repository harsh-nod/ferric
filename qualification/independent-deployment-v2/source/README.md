# Independent Deployment V2 (Unfrozen Draft)

This is a data-only exporter and verifier for the actual checked compiler V7
image and the actual independent-native CPU generation. It does not run Cargo,
LLVM, a metadata helper, the native executable, a GPU, or any archived Python.
It does not use the old SourceV5 supplement reader to qualify the new image.

Status: 30 pure policy tests are authored, not imported or executed. No manifest,
pure receipt, deployment, or remote result has been created by this draft. Root
owns review, freezing, execution, transport, runtime reviews, and launch.

V1 is preserved frozen. Its actual exporter rejected the exact original CPU
command before creating a deployment because it substituted integer byte counts
into the recipe argv, whereas the retained compiler controller stringifies them.
V2 copies that controller's argv/environment-only string substitution semantics.
The evidence pins, seven-file payload format, limits, and no-authority boundary
are unchanged. The ten added fixture files are byte-identical copies of the
actual V7 recipe and all nine actual command records. Three new regressions
verify their exact hashes and joins, string byte counts with unchanged numeric
limits, and rejection of missing bindings or nonstring argv/environment values.
Fixtures are data; tests do not execute their recorded commands.

## Pinned Inputs

`portable.py` fixes these actual identities, not a future interchangeable schema:

- Compiler archive: 8,938,596 bytes, SHA256
  `cfe073463cf3512ba7a6fb7239d0b9cdebbd0346ae0b177d776e6d94412b23a7`,
  exactly 92 regular members.
- Native archive: 33,725,202 bytes, SHA256
  `e097001bf737d7b85e752ee4e3580bc702a2df9521abbb366d1ab355045d16a2`,
  exactly 5,856 regular members including all 5,783 actual source files.
- Root-verified compiler qualification: 32,235 bytes, SHA256
  `128cddec9501f363eab39a00da06752cd43e84ab0680cd99321a8f70addb2366`.
- Root-verified native qualification: 8,569 bytes, SHA256
  `edc59f8559a2c1f9a7fa6bab91c1474be91d6fb82b956f00a93c8a7742f976bb`.
- Actual image: 53,560 bytes, SHA256
  `4885204c8d510122588549107f42d2bc6f180f48fbc4eddd3bb1260e8d6629c5`.
- Actual native executable: 11,759,152 bytes, SHA256
  `017e0d3a5c79b1c64a4dccdb0251e53c8e7f8c17ab47e92bbe1309b40760c607`.

The public qualifications bind the already-reviewed candidate CPU and compiler/
finalizer prerequisites. Their immutable pins remain explicitly recorded
build-host evidence. This transport does not ship the whole compiler/toolchain
or re-execute those qualifications on MI350. Every raw phase command, result,
started record, stdout, stderr, and selected artifact in the two new archives is
rehashed and joined to the exact original completion. Nine compiler phases and
six native phases must have their actual natural successes and recorded cleanup.

The full native source roster is streamed and rehashed, joined to its before/after
snapshots, and compared with the 5,781-source qualified generation plus exactly
the recorded three-file native overlay. Both original and transported paths are
retained in the deployment manifest. Original absolute paths are never used for
IO by the MI350 reader; no archived program or module is imported.

## Portable Contract

`portable.verify(D, pins, value, directory) -> (value, verified)` accepts a parsed
`deployment.json`, a canonical final deployment directory, and the existing
safe driver/Pins API. The retained `run_row_facts_v2.py` helper with SHA256
`244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820`
provides the required `parse`, `Pins.read`, and `Pins.recheck` APIs. The runtime
controller must authenticate its own helper and this frozen reader package before
calling them. The reader itself is standard-library-only.

The directory name must be `prefix-independent-deployment-v228-vN`, N >= 1.
The directory contains exactly seven files: `deployment.json`, the two original
compressed archives, two qualification summaries, the image, and the executable.
The executable is flat at `gfx950-qwen-prefix-tiles-comparison-v6` immediately
inside the deployment directory, matching the frozen runtime auditor. Archives,
qualification summaries, and the image retain their grouped subdirectories.
Symlink members, archive links, duplicate names, path traversal, unknown package
files, altered pins, and authority claims are rejected. Hashing is streamed in
1 MiB chunks. Each archive is capped at 64 MiB compressed, 256 MiB expanded,
32 MiB per member, and 128 MiB retained evidence text. Source bodies are hashed
without retaining them; no tar paths are extracted into the target filesystem.

`verified` contains:

- `image` and `binary`: transported FilePins; `original_image` and
  `original_binary`: exact build-host FilePins.
- `metadata`: actual V7 descriptor/canonical/symbol/ABI metadata.
- `compiler_receipt`, `compiler_owner`, `native_receipt`, `native_owner`.
- `compiler_generation`, `candidate_cpu_receipt`, `candidate_cpu_sources`,
  `native_compiler_generation`, `native_overlay`, `native_sources`.
- `compiler_artifacts`, `compiler_phase_records`, `candidate_sources`.
- `arithmetic_evidence`: original `source_entry`, `reciprocal_source`, `llvm`,
  `isa`, and `elf_notes` FilePins from the authenticated compiler archive.
- `qualifications` and `archive_pins`: explicit original/transported pairs.
- `authority: "none"` and all six runtime/numerical/production/launch/performance/
  full-model flags false.

The old 334-file baseline manifest remains a separate input. This package does
not incorporate, modify, or confer new authority on it.

## Root-Owned Execution

First freeze and run the authored policy tests under the established bounded
pure-test envelope, then execute the actual exporter under a bounded root-owned
CPU row. Example arguments below describe intended commands, not executed work:

```sh
python3 export.py \
  --driver /exact/retained/run_row_facts_v2.py \
  --evidence /home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216 \
  --qualification-root /home/harsh/ferric-p227-integration/qualification \
  --output /fresh/local/prefix-independent-deployment-v228-v1 \
  --directory /actual/mi350/evidence/prefix-independent-deployment-v228-v1
```

The local output must be fresh and have the same basename as the final directory.
The exporter only creates this output, uses exclusive writes, preserves failed
partial output for root inspection, and prints the final manifest FilePin. It
imports only its sibling reader and the exact retained safe driver, never code
from the evidence archives. Root should retain the exporter outcome and transport
the seven files without changing their final manifest paths. On MI350, run the
reader and retain its successful data-only result before performing any runtime
audit. Both entry points reject `-O`/`PYTHONOPTIMIZE`.

Proposed pure receipt for the consuming controller:
`ferric-p228-independent-deployment-pure-v1`, with `passed`, `tests` (30 only after
actual execution), `package_manifest`, `controller`, `test_source`, `transcript`
(all four exact FilePins), `gpu_execution: false`, and `numerical_acceptance:
false`. Root creates this receipt and supplies actual frozen package/reader/
controller/pure pins; none are invented here.

## Remaining Gates

Finite differing native captures are lifecycle/capture evidence, not numerical
acceptance. The V7 compiler receipt still has eight undischarged runtime
requirements. This package does not issue artifact-bound ISA/arithmetic reviews,
runtime/platform/current-boot acceptance, GPU observations, reference comparisons,
full-model acceptance, or a throughput result. Fresh reviews and independent
numerical evidence remain necessary before any stronger claim.
