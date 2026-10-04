# RoPE Checked Lowering With RPO

Source-only successor of `p228-rope-materialized-lowering-v2`. No imports,
tests, builds, compiler invocations or remote commands were run by the author.
Root owns qualification and execution. The four-body package is not frozen;
21 synthetic policy tests are authored, not results.

## Unchanged Candidate

Use the branch-reduced RoPE source V2 manifest
`7f1f447852f01bad9b6dedb9b40a7969961d45861592465548e6401615318d23`.
Its actual arithmetic/source CPU receipt is
`rope-materialized-cpu-v228-v3/complete.json`, SHA
`8dcb4f2b326ab909c52039273515f44eb4f88cd91a967c8816f866d5514c864a`.
The existing gate still authenticates 11 phases, 61 raw records, 33 passing
executions including the separately selected exhaustive test, one ordinary
ignored entry, two test ELFs and the complete 15-file CPU fixture.

The checked fixture uses eight retained Rust bodies and the original V7
Cargo.toml/Cargo.lock, not the host numerical harness Cargo files. Provider53,
source/configuration maps, the prefix symbol and the exact contracts.py body
are unchanged. The previous failed RoPE lowering directories are not reused.
CPU arithmetic qualification does not claim that the device entry compiled.

## Compiler Generation

The original V7 recipe and all four ordinary compiler/finalizer prerequisites
are authenticated unchanged as predecessor evidence. The new compiler is
selected only through the actual successful RPO CPU generation:

- `rpo-compiler-cpu-v228-v2/complete.json`, 375,781 bytes,
  `56fc51fc326980e00156d550d0a7052f44bb481ded7b9948c06217653fb246c1`.
- `rpo-compiler-cpu-owner-v228-v2/complete.json`, 57,128 bytes,
  `afca99d8799f910ce607873c320b8f244be66f9ffdb3df4dce4545eb1552ea66`.
- CPU controller `6cb1dc98...` and package `9fe67c55...`; RPO proposal
  `939b76eb...`, based on the qualified ordinary source, not live runtime.

Finalizer completion hashes remain mandatory actual CLI inputs. Paths are
closed to `rpo-finalizer-tools-v228-v2/complete.json` and
`rpo-finalizer-tools-owner-v228-v2/complete.json`. The expected reviewed
finalizer package is `d014c66154afefa324bb0f5a49bacfb034229e840efb012acd02c950d5ce0178`
and its runner is `cd5fb1f8...`. No future receipt, executable suffix or digest
is invented. A failed or mixed-generation receipt is refused.

The gate reuses the pinned RPO CPU/finalizer data functions without invoking
their mains. It replays all actual CPU test names/outcomes, the old-name plus
17-test extension and the exact finalizer 205-name inventory (190 default
passes, 15 ignored). All 12 CPU and six finalizer phase/stream records are
rehash-joined. Both natural owner outcomes, exact compiler products, source
snapshots and unchanged predecessor are required.

Only six compiler roles in the original recipe are replaced by actual Cargo
products: compiler-tests, rustc extractor, backend SO, finalizer,
finalizer-tests and descriptor metadata. The checked wrapper, four matching
argv[0] values and LD_LIBRARY_PATH follow those roles. The extractor's actual
`debug/deps/librustc_codegen_fe2o3.so` must equal the qualified backend SO.
The separate `extract-retained` tool, LLVM worker, reviewed bitcode, nightly
toolchain and every other recipe argument remain unchanged.

## Actual Replay Gate

The nine original stages stay in order: fixture metadata, checked lowering,
actual compiler replay, actual inert join, emit, extract retained archive/LLVM,
descriptor metadata, ELF notes and disassembly. The two actual-capture tests
remain separate `--exact --ignored` invocations using the new semantic capture
and handoff. Default tool tests never substitute for either real replay.
The 15-root proof markers and eight unresolved runtime obligations remain.
Prefix ABI is 120 explicit/376 total bytes, Wave64/WG64, LDS512 and private0.

All original 12 GiB AS, 6 GiB target, 40/38 GiB free-space floors, CPU8/9,
nice10, hidden GPUs, 10,500-second inner and 10,800-second owner deadlines
remain. The new target is fresh; both compiler generations' source bodies and
rosters, all selected binaries, dynamic libraries, source maps and old targets
are postchecked. No compiler limit is raised and no cache is retired here.

## Root Invocation

After source review, freeze exactly run.py, contracts.py, test_run.py and this
README in `manifest.json` with schema
`ferric-p228-rope-materialized-rpo-lowering-package-v1`. Test with the existing
bounded CPU-only pure wrapper; no new policy harness is required.

```text
taskset -c 8,9 nice -n 10 python3 -B E/p228-rope-materialized-rpo-lowering-v1/run.py PACKAGE_SHA E/rope-materialized-cpu-v228-v3/complete.json 8dcb4f2b326ab909c52039273515f44eb4f88cd91a967c8816f866d5514c864a ACTUAL_TOOLS_SHA ACTUAL_TOOLS_OWNER_SHA
```

`E=/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
Fresh output is `row-rope-materialized-rpo-checked-probe-v228-v1`, with owner
`rope-materialized-rpo-checked-probe-owner-v228-v1`. Result schemas are
`ferric-p228-rope-materialized-rpo-lowering-result-v1` and
`ferric-p228-rope-materialized-rpo-lowering-owned-result-v1`.

Source-only tests cover unchanged recipe bounds, prefix ABI/source transition,
the six-role substitution, mixed-generation rejection, mandatory actual tools
hashes and refusal of downstream authority. Future successful lowering would
remain CPU/compiler evidence only: no GPU, numerical, runtime, performance or
full-model acceptance follows.
