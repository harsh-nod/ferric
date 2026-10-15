# Exact Selected-Invocation Transport Component

This bounded CPU checkpoint advances Ferric [issue #42](https://github.com/harsh-nod/ferric/issues/42).
It implements a real canonical V3 transport producer on the existing
protected-wrapper capture path and an independent selected-input verifier.
**It does not yet connect the engineering extractor to the semantic backend.**
No worker compilation, HSACO, GPU execution, numerical or performance result,
or M0-M7 completion is claimed.

## Implementation

The new [transport component](source/crates/fe2o3-rustc-invocation/src/selected_transport_v3.rs)
uses the existing V2/V3 structured constructors and canonical codec. It:

- Produces canonical V3 bytes from the final selected process descriptor and
  a complete six-role compiler closure; no closure means refusal, not V2 fallback.
- Retains V3's rustc/backend digest-to-role equality checks.
- Verifies received canonical bytes against **independently supplied** selected
  cwd, every argv element, complete environment, and every compiler role.
- Rejects altered process inputs, target, normalization flags, wrong closure
  roles, V2 bytes, truncated input and trailing data.

The existing [protected capture upgrade](source/crates/cargo-fe2o3/src/inert_rustc_invocation_capture.rs)
now uses this producer. Its selected argv/cwd/environment and role preimages
still come from the existing [binding wrapper](source/crates/cargo-fe2o3/src/binding_wrapper.rs),
not an inert test helper or manually fabricated descriptor. Production
engineering transport is not wired, and no descriptor plaintext was published.
This CPU campaign does not execute that protected compiler path: it tests its
compiled capture logic. Schema-only unit fixture pins are not executable custody.

The backend qualification snapshot supplies the candidate sourcebase.
Exactly three frontend files are copied from the separately qualified
semantic-capture driver, preserving its explicit diagnostic directory and
`optimized-inline-hint-16384-v1` normalization. The receipt records exactly six
source deltas: those three preserved frontend files, the capture upgrade,
the invocation export, and the new transport module. No backend predicates
or runtime gates change.

## Actual CPU Evidence

SSH mi350, CPU8/9, nice10, jobs2/offline, service memory maximum12 GiB,
swap disabled, owned subreaper and process groups:

| Cohort | Actual Result |
| --- | --- |
| Invocation library, all active tests | 103 passed, no ignores |
| Preserved engineering frontend | 25 passed |
| Exact capture/closure upgrade | 8 passed |
| Existing ordinary V2/protected-publication gates | 2 passed on prior qualified compiler ELF |
| Owned phases | 12 natural status0 exits, leaders reaped, groups absent |
| Elapsed | 76.564605 seconds |
| Source inputs before/after | 5,812 unchanged |
| Tool/dependency/artifact postchecks | Passed |

Six new invocation tests cover canonical roundtrip, missing closure,
all six role mismatches, altered cwd/target/normalization, incompatible
wire versions/malformed bytes, and the in-process extractor incompatibility.
The existing V2/V3 library tests additionally cover canonical golden bytes,
environment mutation, missing/zero role pins and schema bounds. The driver
ignored inventory is unchanged at five; those ignored tests were not run.

See [terminal receipt](cpu/complete.json), [invocation test output](cpu/invocation-tests.stdout),
[frontend output](cpu/engineering-tests.stdout), [capture output](cpu/capture-tests.stdout),
and [data audit](publication-audit.json). The full compiler/device/lowerer
suite was **not** rebuilt or rerun. Its two gate tests use the previously
qualified compiler ELF `b6b03b519e08c0ffb842508263fb24aadde2e450bd0d0d93e19171a80928ecf3`.

## Integration Blocker

The [engineering command](source/crates/cargo-fe2o3/src/engineering_hsaco/execution.rs)
uses Cargo's workspace extraction wrapper. That
[extractor](source/crates/rustc-codegen-fe2o3/src/bin/fe2o3-rustc-extract.rs)
calls `rustc_driver::run_compiler(args, callbacks)` **in-process**; its exact
selected argv does not include a backend argument. It also has no Cargo
binding trampoline/binding-wrapper transition providing those two closure roles.

Frozen [V2](source/crates/fe2o3-rustc-invocation/src/model_v2.rs), embedded
unchanged in [V3](source/crates/fe2o3-rustc-invocation/src/model_v3.rs),
requires a real final `-Zcodegen-backend=...` selector. The
[closure schema](source/crates/fe2o3-build-authority/src/compiler_closure.rs)
requires six real role pins. Adding an argument that was never executed,
reusing extractor bytes as absent wrappers, or injecting an inert test
descriptor would manufacture custody. The new regression refuses that route.

Therefore this checkpoint implements the reusable exact-input component,
not the whole assignment's requested engineering selector/transport integration.
There is no opt-in that silently treats the current in-process extractor
as a genuine V3 child invocation. Incompatible **extraction modes** still
need dedicated selector/integration tests after a compatible route exists.
The new component tests incompatible **wire versions**, not those modes.

## Next Bounded Step

Trace the existing real protected-wrapper child-rustc route and implement
one explicit default-off inert extraction selector there, only if it can
join actual retained tool roles before rustc threads and enter extraction-only
custody without adopting protected publication authority. Otherwise retain
the refusal and design a separate versioned in-process extraction schema;
do not alter frozen V2/V3 semantics. Coordinate this source boundary with
the compiler owner before integration.

No fresh worker compile until the matching V3 consumer retains the outer
conditional capsule and all premises losslessly and is qualified. The eight
runtime requirements, early-stop CFG issue and unresolved GPU completion
hold remain closed. No signed-Verus, compiler-refinement, numerical,
artifact or launch authority is granted by this transport.

## Provenance

[Runner](run_cpu.py), [input preparer](prepare_inputs.py), [exporter](export.py),
[input manifest](input-manifest.json), [originals ledger](originals-ledger.json),
and [originals archive](originals.tar.gz) retain the tested sources and raw phases.
The data audit rehashes all candidate sources, tools, parent receipts and
selected artifacts before exporting originals. The archive embeds relevant
source/log/controller originals, not full compiled/toolchain/source trees;
complete input rosters and artifact hashes remain in the terminal receipt.

The first export attempt failed its archive-member assertion because its
expected list was not fully sorted. Its original exporter/archive are retained
on mi350 and pinned in the manifest. This fresh export fixes only that list
comparison. The successful CPU qualification was not rerun or modified.
