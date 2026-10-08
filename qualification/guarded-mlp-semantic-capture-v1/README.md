# Opt-In Semantic Diagnostic Capture

This checkpoint for [Ferric issue #42](https://github.com/harsh-nod/ferric/issues/42)
was authored, built and tested on `ssh mi350`. It adds a diagnostic path to
investigate the [previous MLP lowering failure](../guarded-mlp-inline-hint-v1/README.md).
It does not change production branches, relax compiler admission, produce an
HSACO, execute a GPU kernel or establish a performance improvement.

## Implementation

The engineering frontend accepts an explicit `--diagnostic-semantic-dir`.
It defaults off. The caller must select a fresh, absolute, versioned directory
named `fe2o3-engineering-diagnostics-v1` under an existing canonical parent.
The frontend creates it with mode 0700 and pins it through inherited FD210.
After clearing the extraction environment, it passes exactly two fixed
diagnostic destinations through that descriptor. Ambient capture variables
are not forwarded, and failed or partial captures are retained.

The backend's existing pre-ranked semantic snapshot hook now also accepts
the already collected source-file map. After a successful semantic write, it
can create a bounded JSON sidecar binding the semantic byte length, semantic
digest, raw-file digest and sorted unique source-file identities. The sidecar
is limited to 1 MiB and 4,096 files; the existing semantic limit is 16 MiB.
Both outputs are create-new, mode 0600, with completion logged only after
the write, synchronization and inode checks succeed. Diagnostic failures
do not replace the compiler's actual result.

The [frontend patch](driver.patch) changes two files. The
[backend patch](backend.patch) changes three. Existing fixed optimization
profiles, source safety, borrow checks, ABI checks and materialization remain
unchanged. Source-file display paths are diagnostic labels, not authority to
open arbitrary files. A source identity is not the SHA256 of a Rust file;
`semantic_file_sha256` hashes the captured MIR bytes.

## CPU Qualification

| Component | Selected Tests | Added Tests | Clean Phases | Controller Wall Time |
| --- | ---: | ---: | ---: | ---: |
| Frontend | 25 | 4 | 7 | 107.125962007 s |
| Backend | 11 capture + 14 helper controls | 5 | 8 | 113.246095320 s |

All selected tests passed, with no selected failures or ignores. These are
focused tests, not full repository or compiler suites. The frontend inventory
contains 409 names and five unchanged ignored tests; the backend inventory
contains 1,352 names and 24 unchanged ignores. The original 21 frontend tests,
six capture tests and existing helper controls remain in scope.

Frontend tests cover strict/default-off selection, explicit environment
forwarding, refusal of aliases/existing bytes, and pinned-directory child
and grandchild writes after display-path substitution. Backend additions cover
absent/failed/mismatched semantic pairing, exact sidecar contents, sorted-unique
file and inclusive byte limits including JSON escaping, oversized refusal
before destination creation, preservation after publication failure, and
stderr writer failure.

All phases exited naturally, were reaped and left owned process groups
absent. Source, dependency, tool, configuration and artifact postchecks passed.
Source deltas are closed against 5,297 frontend and 5,808 backend project files.

- Frontend receipt: `d8d5be4c85fbcac8b963657d1b5054c8fc8e1fdd987b4a9ba1954bc1cff1895f`.
- Backend receipt: `699c8e48b8ce20f1a1f4f52a5f2f9e3e7329bc4ae23857a63398f0bb81c25337`.
- Frontend executable: 81,300,408 bytes, `26bf333c671f938c84bd30e6df6dec24638bdc8b131162ce35646fb5ca802864`.
- Backend library: 206,380,352 bytes, `efa54ef3935ea4d852338bbf042aeacc982eafe782d8ea0b29d22e7f50e6dcfc`.
- Extractor: 205,248 bytes, `1c8342c8c7964c436da49a2303d72cf6c79e4fecfbe910eb1820b1c490676456`.

Original outputs are available for the [frontend tests](cpu/driver/driver-tests.stdout),
[backend capture tests](cpu/backend/capture-tests.stdout) and
[helper controls](cpu/backend/helper-controls.stdout), with the
[frontend receipt](cpu/driver/complete.json) and [backend receipt](cpu/backend/complete.json).

## Real Extraction

Both V8 fixtures are byte-identical to their V7 counterparts. They retain the
named callback with `inline(never)`; the only difference within each pair is
`run_fixed_rounds` versus `run`. Both use the freshly qualified frontend,
backend and extractor and the explicit 16384 normalization profile.

| Arm | Compiler Leaf Wall Time | Compiler Result | Retained MIR | Source Map |
| --- | ---: | --- | ---: | ---: |
| Fixed rounds | 118.721986214 s | Exit 1, same helper refusal | 924,436 bytes | 2,692 bytes |
| Early STOP | 118.941634120 s | Exit 1, same helper refusal | 925,515 bytes | 2,692 bytes |

These are **failed compile attempts**, not GPU timings. Both attempts emitted
complete MIR and source-map receipts through the actual Cargo/extractor path.
Both exited naturally, were reaped, left their process groups absent and
passed source/tool postchecks without timeout, forced cleanup or storage
failure. Temporary and HSACO output directories contain zero bytes.

The actual refusal still reports helper
`00b15eaa5a80fc7f25e19c1c333fcde91f768f51097ae1c2c4f27ced27c66af0`:
argument 0 is a `UniqueBorrow`, mutable reference to an ordinary aggregate.
Its displayed source is `94c691d17656:828:5`. The retained sidecar contains
the full source identity and provider path; a typed canonical decode joining
that exact helper to its source span has **not yet been performed**.
No helper name or accepted ABI is inferred from the line number alone.

- Fixed MIR SHA256: `72dfdc23aabf0eaac4f2809371131a853874f51609bf1e380476ce95eb0a1667`.
- Early MIR SHA256: `e29daedb54bb4428533a48a9342e1a1964ab3756b95fd875537fcb5803597f69`.

The [fixed stderr](pair/fixed-compile.stderr), [early stderr](pair/early-compile.stderr),
[fixed source map](pair/fixed-source-map.json), [early source map](pair/early-source-map.json),
[fixed result](pair/fixed-result.json) and [early result](pair/early-result.json)
retain the observations without converting a failed compile into a pass.
The [archive](qualification.tar.gz) retains the actual binary captures,
commands, streams, inventories, source/dependency ledgers and source
preimages/postimages. The [manifest](manifest.json) pins exported files and
archive members back to their original remote files.

## Limits And Next Step

Builds use CPU8/9, nice10, two explicit Cargo jobs and hidden GPU visibility.
The extraction frontend clears the caller environment: outer
`CARGO_BUILD_JOBS=2` is not proof of an explicit inner Cargo job limit.
Both compiler attempts preserve the 600-second leaf limit and 50-second
cleanup reserve, 12 GiB address space, and 40/38 GiB initial/live free-space
floors. No deadline, admission bound or GPU access was expanded.

Next is a small diagnostic consumer of the existing canonical MIR decoder
and logical-argument API, then an evidence-driven helper correction. The
decoder, full source-span join and any correction are unrun at this checkpoint.
Subsequent HSACO, ABI and independent numerical qualification are still required.

Native model evidence remains forty prompt forwards and zero generated
tokens, compared with historical Ferric rather than an independent framework.
Full2303, independent 256-output acceptance, sustained single-request
2,048/256 Qwen3-8B BF16 target-only decode, 700 tokens/s and all M0-M7 exits
remain open.

