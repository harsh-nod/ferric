# Combined-State Candidate Proposal

Source-only additive candidate for Ferric's guarded TP2 MLP segment. Root owns
integration, CPU qualification, actual checked compilation and GPU testing.
No compiler source, compiler predicate, resource limit or existing crate is
changed by this proposal.

## Exact Base And Additions

The base is Ferric's `device/qwen3-tp-guarded-mlp-segment-kernels-v1`.
All five Rust source bytes were independently joined to the candidate-v5
rows in the retained DAG lowering source-before map. Cargo.toml and README.md
are also pinned from the exact current Ferric files. The base is immutable.

The patch adds eight files only under
`device/qwen3-tp-guarded-mlp-segment-kernels-v2`. The new package, library,
entry symbols and expectation-roster function are explicitly v2.
`arithmetic.rs` and `guard_tests.rs` are copied byte-for-byte. The R2
kernel body is byte-for-byte unchanged except its v2 entry symbol. The
548-word predicate is extracted once, with only indentation changed, into a
private shared helper. Its old wrapper remains exact-length-548 for the
inherited tests; the new combined wrapper is exact-length-552.

`source-manifest.json` records the seven exact base pins, eight additive
postimages, complete 39-test roster, roots, layout contract, failed-compile
evidence and integration patch pin. There are no guessed execution results.
The crate README carries the operational contract and remaining gates.

## Soundness Boundary

The observed failure was one natural, reaped code-1 compile, unchanged sources,
no postcheck errors and no artifact. Its 123593-byte stderr
`b7c4435210a46414e49ed798624eff224c0436935a51d6715eccd8ee61c72f0a`
shows 548 Acquire/System reads from origin1/class1 and three Relaxed plus one
Release/System writes to origin2/class1. The distinct origins share a writable
alias class, so the qualified provenance analysis correctly lacks relative
offset evidence. The new API uses one actual allocation and source argument.
It does not merely forge equal origins or distinct no-alias classes.

Natural-aligned same-width atomics could support a separately authenticated
compiler theorem even with unknown base offsets. However, width alone is not
alignment evidence for generic ranked IR. Such an extension would require
new primitive/alignment custody and a complete nonmixed-effect proof. That
larger compiler change is deliberately outside this candidate.

The new validator reads indices0..547 and writes548/549/551 Relaxed then550
Release. It never creates two slices. R2 retains two read-only four-word
guard parameters. Its actual host suffix ranges must use offset2192 and
extent16 within independently leased2208-byte owners. Atomics do not, by
themselves, validate dispatch ordering, generation freshness or buffer reuse.

## Required Qualification

1. Run all 39 candidate CPU tests, preserving the exact old27 and new12
   named rosters. Compare the independent range oracle against every tested
   mutation and verify original source pins before/after.
2. Build the host expectation roster with tests disabled, then compile both
   v2 roots using the already qualified DAG compiler/tool closure. Recheck
   nominal AtomicU32 identity, actual one-origin validator IR, 548 atomic
   reads, four atomic writes, ordering, geometry and all mandatory proof
   reports. A future failure is retained as a new failure, not relabelled.
3. Independently inspect descriptors, LLVM/ISA and HSACO argument metadata.
   The 24-explicit/280-total byte guard ABI is a prediction pending this
   measurement. Preserve R2 argument/access contracts and guard-before-payload
   dominance.
4. Integrate the additive atomic owner and completion leases. Run native
   valid-state checks, every-state-field corruption coverage, stale/zero/
   mismatched generation checks, poisoned suffixes, wrong lengths and no
   payload access on rejection; check exact state-prefix preservation.
5. Require separately retained end-to-end numerical and lifecycle evidence
   before connecting the candidate to model inference. No SoTA, throughput
   or 700-token/s claim follows from source or CPU acceptance.

No imports of project/proposal modules, tests, builds, SSH or canonical edits
were performed by this proposal author.
