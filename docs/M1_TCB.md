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
- the reviewed fe2o3 Rust-to-MIR, structured-kernel, LLVM/AMDGPU, object, and
  HSACO pipeline components admitted by the evidence index;
- LLVM's AMDGPU backend and the in-process linker/finalizer; and
- the independent artifact, ABI, target, and machine-shape validators named by
  the evidence index.

Source proofs end at their declared Rust/MIR, schedule, effect, or `gpu.*`
boundary. The transformation from `gpu.*` through LLVM, object, and HSACO is a
contracted compiler boundary unless every separate machine-refinement
validator required by `ASSURANCE.md` exists and passes. A source digest,
successful compilation, disassembly check, or hardware differential test does
not promote that boundary to `Proved`.

## Runtime Boundary

M1 trusts the exact identity-bound instances of:

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
