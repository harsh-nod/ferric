# Finite gfx950 Decoder Probe

This standalone engineering tool uses fe2o3's existing direct-KFD framed
worker. It does not add a dispatch API, substitute HIP/HSA, mint protected
authority, or participate in Ferric's production workspace. Only trusted local
machine code is appropriate: structural ABI checks do not prove native code
honors its pointer extents. The execution property remains **Unsupported** for
proof-required production deployment.

The single fixture is `ferric_gfx950_decoder_layer_f32_v1`: 256 independent
requests, two workgroups of 128 work-items, two wave64 waves per workgroup.
Its three slices contain 3072 input, 110 weight, and 10240 output `f32` elements.
Pointer/length pairs occupy explicit kernarg bytes 0/8, 16/24, and 32/40.
The wire grid is `[256,1,1]` work-items, not two work-items.

Build this tool and the worker from adjacent `ferric` and `fe2o3` checkouts:

```sh
cargo build --manifest-path tools/gfx950-finite-probe/Cargo.toml
cargo build --manifest-path ../fe2o3/Cargo.toml -p fe2o3-kfd \
  --features engineering-gfx950 --bin fe2o3-gfx950-engineering-worker
```

First inspect the exact object without opening GPU devices. `inspect` writes
an exclusive-create artifact record containing source/object SHA-256 and
admitted kernel metadata; it rejects nonmatching fixed launch/argument ABI.
The source digest is an identity observation, **not** a source-to-object proof.
Compiler command, compiler digest and build evidence must be retained alongside
this record by the caller.

The record separates `source_declared_abi` from observed ELF metadata. Native
LLVM may omit optional pointer access/alignment qualifiers and maximum grid
dimensions; omissions stay `null`, while contradictory present values reject.
Required workgroup size, explicit argument layout and kernel resources remain
mandatory. The fixed grid, slice extents, three disjoint worker allocations and
aligned pointer offsets come from this probe's declared contract, not invented
ELF fields. Neither that contract nor optional qualifiers grant production
authority.

```sh
PROBE=/path/to/ferric-gfx950-finite-probe
"$PROBE" inspect --object /path/kernel.hsaco --source-file /path/kernel.rs \
  --metadata /path/expected-artifact.json
```

Only an operator coordinating access to the selected GPU should run this step.
The private device-selection file must be a regular file with no group/other
permissions and contain the selected KFD device UID as a decimal integer. Do
not publish this file or raw worker stderr. The existing worker receives the
selector through its required command-line argument; local process viewers may
therefore observe it. No environment-based or automatic device selection occurs.

```sh
"$PROBE" run --worker /path/fe2o3-gfx950-engineering-worker \
  --object /path/kernel.hsaco --source-file /path/kernel.rs \
  --metadata /path/expected-artifact.json \
  --inputs /path/case/inputs.f32le --weights /path/case/weights.f32le \
  --device-id-file /private/selected-device --run-dir /path/new-run \
  --allow-unauthenticated-machine-code
```

Inputs are raw little-endian `f32` files with exact fixture extents. The probe
re-inspects owned HSACO bytes, verifies the saved artifact record, checks Ready's
device/protocol/target and `authority=none`, then compares worker-loaded metadata
exactly with offline metadata. Kernarg pointer slots start zero; only the worker
fixes pointers to allocations it owns. Scalar slice lengths are fixed by the
fixture ABI. The worker populates COV6 implicit geometry arguments when present.

There is exactly one dispatch with a 10-second completion deadline, 30-second
per-command pipe deadline, and 180-second session deadline. A dedicated I/O
thread also bounds stalled writes. Any protocol failure terminates only this
owned child; there is no retry, device reset, queue reuse or foreign-process
management. Success requires unchanged inputs, intact 64-byte output guards,
finite written checkpoints, explicit reverse-order Free, Close and clean worker
exit. Shutdown waits are bounded; kernel/OS failures that prevent process exit
cannot be repaired by this tool and are not reported as successful cleanup.

`output.f32le` and identifier-free `report.json` are written only after successful
cleanup. The report binds source, object, exact metadata, worker, inputs and
output by hash. Its elapsed time is the engineering worker's dispatch observation,
not a benchmark or SoTA claim. It does **not** establish numerical correctness;
the independent reference must compare all checkpoints. The finite fixture does
not implement persistent scheduling, cross-workgroup atomics, model loading,
autoregressive generation or production inference authority.

Host-only tests use a scripted in-memory transport plus a disposable shell
protocol stub for blocked-pipe/child-cleanup tests. They do not invoke a GPU:

```sh
cargo test --manifest-path tools/gfx950-finite-probe/Cargo.toml
cargo clippy --manifest-path tools/gfx950-finite-probe/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path tools/gfx950-finite-probe/Cargo.toml --check
```
