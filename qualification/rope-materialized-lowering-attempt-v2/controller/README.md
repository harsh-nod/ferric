# Checked RoPE V2 Candidate Lowering

Unexecuted controller proposal. Four files: `run.py`, byte-preserved original
prefix `contracts.py`, `test_run.py` (14 authored synthetic tests), and this file.
The root agent freezes their hashes in a `manifest.json` with schema
`ferric-p228-rope-materialized-lowering-package-v2`, then owns all execution.
There is no candidate CPU result or emitted image implied by this package.

V1 failed before image emission at the unchanged partial-move storage limit.
V2 changes only the source/controller identities and fresh row/owner paths,
plus explicit V2 package/recipe/result schemas. Source V2 replaces 17 pure
pair-valid `&&` operators with `&`; all 18 finite checks and 20 tests remain.
The first rejected cumulative charge was 2,097,153 versus 2,097,152, not a known
full storage requirement. All compiler and runtime gates remain unchanged.
The original V1 failed run and its earlier manifest-parsing refusal are retained
separately. This successor does not reinterpret either failure as success.

## Inputs and Invocation

Use the original ASROCK evidence directory
`E=/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
Required inputs are the frozen source manifest `7f1f447852f01bad9b6dedb9b40a7969961d45861592465548e6401615318d23`,
an actual successful `rope-materialized-cpu-v228-vN/complete.json` and its SHA,
the original V7 nine-file fixture/recipe and compiler generation, and the exact
retained helper/tool/configuration/source files pinned in `run.py` and that recipe.
The V2 CPU runner is independently pinned to `084d135a52c4274341e2063b4e46c732db07ea2a67e69f6cae6c9e575f089dd1`.

The CPU evidence gate verifies 11 natural phases/61 raw files, actual Cargo test
artifacts and streams, the 12 ordinary reciprocal tests plus one ignored test,
20 ordinary candidate tests, and the separate actual exhaustive test. These
are 33 passes/one ignored only after the supplied real receipt and named outcomes
pass the checks. The whole 15-file CPU fixture transition, four formatted overlay
bodies, exact eight lowering Rust inputs, provider53, dependency/source postchecks
and original Cargo pair are retained. CPU macro tests explicitly do not compile
the kernel entry. The lowering fixture is ten files, not the device-only CPU
harness: original checked Cargo pair plus those eight source files.

After root-owned pure tests, invoke the bounded parent exactly as for the prior
compiler probe:

```sh
env -i HOME=/home/harmenon PATH=/usr/bin:/bin LANG=C.UTF-8 LC_ALL=C.UTF-8 \
  HIP_VISIBLE_DEVICES= ROCR_VISIBLE_DEVICES= CUDA_VISIBLE_DEVICES= \
  taskset -c 8,9 nice -n 10 /usr/bin/python3 -B \
  "$E/p228-rope-materialized-lowering-v2/run.py" PACKAGE_MANIFEST_SHA \
  "$E/rope-materialized-cpu-v228-vN/complete.json" ACTUAL_CPU_SHA
```

Fresh outputs are `row-rope-materialized-checked-probe-v228-v2` and
`rope-materialized-checked-probe-owner-v228-v2`. Neither may already exist.
The parent is the only supported launch path; do not launch `--child` directly.

## Preserved Pipeline

The nine commands come from the actual V7 recipe, changing only fresh output
paths. Prefix capture names, the exact prefix replay and inert-join tests,
`--wave-qkv-attention-output-tiles-v6` finalizer selection, extractor/backend,
qualified ordinary-induction compiler, linker worker, build-std source and
reviewed device libraries are unchanged. `contracts.py` remains byte-identical
to `p227-prefix-tiles-source-pipeline-v5/contracts.py` SHA
`694be9d199cd92b2d11ff4fe8401ad3728497bd8846b5d6924311330f3d9de1a`.
It validates the 15-root/120 explicit bytes/376 kernarg bytes, Wave64/WG64/grid64
prefix interface, not the MLP variant.

The stages are metadata; checked Rust-to-semantic/ranked/KIR/LLVM extraction;
actual semantic replay; actual inert join; checked finalizer emission; retained
archive extraction; descriptor metadata; ELF notes; disassembly. Both owner and
child recheck the original source-map contents/stamps; preserved targets,
compiler source/configuration, provider and dynamic libraries are protected.
Actual commands, owned exits, streams and artifacts are retained on failure too.
No direct LLVM/llc path, proof bypass, compiler patch or old source receipt is
substituted for fresh candidate qualification.

Bounds remain two CPUs, nice10, GPU-hidden environment, 12 GiB address space,
1 GiB per-file cap, 6 GiB fresh target, existing 40/38 GiB disk policy, 10500-second
inner and 10800-second owned deadlines. Individual original stage deadlines and
all semantic/proof/storage/CFG caps remain unchanged. Additional RoPE conversions
and finite checks may hit an existing compiler limit; that is a retained failure,
not permission to raise it. Prior similar probe targets were about 260 MiB, but
that is not a promised peak or a reason to waive the disk floor.

## Output Meaning

On success the inner schema is `ferric-p228-rope-materialized-lowering-result-v2`
and the parent schema is `ferric-p228-rope-materialized-lowering-owned-result-v2`.
The result joins the actual CPU pin, source manifest, compiler generation, all
nine command records and ten artifact pins. ABI/resource metadata and actual ISA
are retained. Independently inspect coefficient/product BF16 conversions before
the final adds, strict FP flags and emitted denormal mode; CPU subnormal tests
are not GPU denormal evidence. Native-vs-framework table generation is still a
separate unresolved comparison.

All eight runtime requirements remain open. GPU execution, launch/production
authority, numerical/full-model acceptance and performance claims remain false.
