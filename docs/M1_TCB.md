# M1 Trusted Computing Base

This document declares the trusted computing base for the single-device M1
configuration. It is a boundary declaration, not evidence that a component is
present or correct. The canonical evidence protocol is defined by
`proofs/m1/evidence/TCB_REPORT.md`; only reports accepted by its pinned
validator may satisfy an M1 evidence binding.

## Compiler Boundary

M1 trusts the exact identity-bound instances of:

- Rust and Cargo used to compile Ferric and fe2o3;
- Verus, its bundled solver dependencies, and the proof runner;
- the reviewed fe2o3 Rust-to-MIR, generic structured-IR/compiler,
  LLVM/AMDGPU, object, and HSACO pipeline components admitted by the evidence
  index;
- LLVM's AMDGPU backend and the in-process linker/finalizer; and
- the independent artifact, ABI, target, and machine-shape validators named by
  the evidence index.

Source proofs end at their declared Rust/MIR, schedule, effect, or `gpu.*`
boundary. The transformation from `gpu.*` through LLVM, object, and HSACO is a
contracted compiler boundary unless every separate machine-refinement
validator required by `ASSURANCE.md` exists and passes. A source digest,
successful compilation, disassembly check, or hardware differential test does
not promote that boundary to `Proved`.

## Cryptographic Boundary

The directly verified `ferric-build` SHA-256 implementation establishes
correspondence to its closed functional SHA-256 computation for admitted
messages shorter than 2^64 bits. The proof covers initialization, the
big-endian word schedule, 64 wrapping `u32` rounds, block chaining, streaming
updates, standard padding, encoded bit length, and digest byte order.
The verified build identity bodies additionally refine their exact ordered,
big-endian length-prefixed model and deployment-bundle records to that
functional digest computation.

That computation theorem does not prove collision resistance, preimage or
second-preimage resistance, provenance, signer identity, or authenticity.
M1 therefore trusts the standard cryptographic security assumptions for
SHA-256 wherever a digest is used as an identity commitment. Signature
algorithm choice, release keys, key identifiers, rotation, revocation, and
compromise response remain separate pending admission inputs; this declaration
does not provide or authorize any of them.

The directly verified pure weight-manifest finalizer binds the exact retained
production fields into the versioned, domain-separated canonical record. Its
acceptance theorem establishes ordered gap-free destination coverage, exact
section framing, and equality between the returned aggregate identity and the
directly verified SHA-256 computation over those canonical bytes. The theorem
is fail-closed: a successful finalization implies the relation; it does not
assert that every arbitrary input must be accepted.

External `Read` and `Write` behavior, source EOF observations, partial-write
effects, final flush behavior, filesystem staging, synchronization, rename or
publication durability, and recovery after I/O failure remain `Contracted` and
pending. Unit tests exercise hostile short reads, writes, flushes, gaps,
overlaps, reordering, overflow, and field mutation, but those tests do not
promote the external I/O boundary to `Proved`.

## Runtime Boundary

M1 trusts the exact identity-bound instances of:

- `onig` 6.5.3, `onig_sys` 69.9.3, the bundled Oniguruma C build and Unicode
  16.0.0 property/regex tables, including the unsafe Rust/C boundary;
- `unicode-normalization-alignments` 0.1.12 and its generated Unicode 9.0.0
  normalization and alignment tables;
- the reviewed fe2o3 allocation, code-object, kernarg, AQL queue, completion,
  and teardown adapters used by the generated runner;
- the HSA runtime and KFD user/kernel ABI used by those adapters;
- the host operating system, process isolation, virtual-memory, file, clock,
  and synchronization services used during admission and execution; and
- the ROCm runtime libraries and AMDGPU kernel driver admitted by the
  qualification environment.

Ferric source and model proofs cover only the explicit queue, generation,
lease, completion, cancellation, quiescence, and resource models named in
their contracts. Native atomics, CPU/GPU coherence, MMIO ordering, ioctl side
effects, signal delivery, reset behavior, and driver/runtime implementation
are contracted. Any ambiguous native side effect poisons the owning authority
and requires the teardown policy declared by the runtime evidence.

The two tokenizer libraries and their complete registry closure are pinned in
`Cargo.lock` and `proofs/RUNTIME_DEPENDENCY_TCB`. Their build scripts, native C
compiler behavior, unsafe code, regex engine, and Unicode tables are
`Contracted`. The 640-case exact-tokenizers differential is regression
evidence only; it is not exhaustive equivalence evidence and is not a Verus
proof of those external implementations.

Within that boundary, Verus directly verifies the production numeric tokenizer
execution bodies: the ByteLevel byte/codepoint bijection, earliest/longest
added-token matching and special-token policy, bounded lowest-rank BPE
selection with simultaneous nonoverlapping application and strict progress,
and exact bounded byte decode. Construction of the numeric execution program
from the authenticated string vocabulary and merge list, Rust `char`
conversion, fallible allocation, Oniguruma Split behavior, and Unicode
normalization tables and runtime behavior remain `Contracted`. These theorems
therefore do not claim full Hugging Face tokenizer equivalence.

The resolved transitive packages are `onig_sys` 69.9.3, `bitflags` 2.13.1,
`libc` 0.2.189, `once_cell` 1.21.4, `cc` 1.4.5, `find-msvc-tools` 0.1.12,
`shlex` 2.0.1, `pkg-config` 0.3.34, and `smallvec` 1.16.0. The canonical
manifest, rather than this prose list, is authoritative for checksums,
features, target predicates, target kinds, build scripts, proc macros, and
dependency edges.

## Hardware Boundary

M1 trusts one exact physical `gfx942:xnack-` device and the identity-bound
firmware, microcode, memory system, command processor, compute units, and
device clocks observed by the qualification harness. Hardware transcript
validation establishes that the declared tests ran on that device; it does
not prove that the device implements the abstract machine or source semantics.

The M1 claim excludes protection against a compromised host, driver, firmware,
or GPU and excludes physical, DMA, timing, power, and resource-contention side
channels. Multi-device execution and machine refinement remain explicit
unsupported properties for M1.

## Identity And Closure

Every qualifying TCB report must bind all of the following without omission or
substitution:

1. the canonical M1 requirements and complete still-open-or-closed obligation
   roster used for that qualification;
2. exact Ferric and fe2o3 commit, tree, base, and source-closure identities;
3. exact compiler, runtime, driver, firmware, target, and device identities;
4. the complete checker-owned validator path, protocol, and source-digest
   registry; and
5. the compiler, runtime, and hardware TCB reports as one ordered set.

An unpinned validator, missing report, identity drift, source mutation,
reordered component, or weaker authority classification fails closed. The
qualification receipt must reference the accepted report artifacts and their
digests; it cannot replace them with a self-reported summary.

## Authority

This declaration grants no theorem, artifact, load, launch, hardware,
performance, machine-refinement, or qualification authority. Those authorities
remain separate evidence kinds with separate validators. M1 can close only
when the evidence index binds every required artifact at its declared status
and the final qualification receipt validates the resulting closed set.
