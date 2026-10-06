# Combined-State Candidate: Explicit Test Imports

Fresh source-only successor of the format-stable source assertion proposal.
The v2 CPU build exposed a no_std test-module import omission. This v3 repair
adds exactly `use std::format;` and `use std::string::String;` to the actual
formatted test source from that failed build. All test logic, the 39-name
roster, production kernels, arithmetic, atomic orderings, compiler and ABI
remain unchanged relative to the observed v2 source.

## Observed V2 Failure

The actual failed CPU receipt is 111,015 bytes, SHA-256
`4d70e61800b8f82ae609583f5de08424ad79f099aad18fd9d8e400c1e67c8f47`.
Its six-phase prefix has five natural zero exits followed by build-tests
returning 101, naturally reaped with no remaining process group. Sources
remained unchanged and postchecks were empty. The elapsed time was
72.28604292904492 seconds. No Rust test execution started, and the controller's
tests field is null. This is not a successful CPU qualification.

The Cargo JSON stdout (209,795 bytes, SHA-256
`f9ec5eb8273c9e0d3235eb684601bd6d151f195eada9425e937c3322857c0300`)
contains exactly three missing-format-macro errors and one E0425 missing-String
type error. Those detailed errors are in stdout, not fabricated from the
stderr summary. The independent retained capsule has 50 members, 49 content
pins and 36 raw bodies. The earlier v1 38-pass/one-failure result and this v2
build failure both remain immutable.

## Source/API Repair

The crate is no_std, with extern crate std enabled in tests. Its test module
already imported Vec and cell types explicitly. The newly added source-spelling
regressions additionally require the std format macro and owned String type.
The two explicit imports supply those names without a new dependency, feature,
parser, production source edit or proof-rule change. Other iterator and string
operations need no new extension-trait import.

The source-spelling assertion still compares exactly four literal store calls
in order, ignores only formatting whitespace, and rejects the same index,
ordering, order and extra-store mutations. It remains supplementary to the
unchanged runtime access/order checks, not a Rust parser or production proof.
All other Rust bodies exactly match the failed attempt's formatted snapshot.

## Integration and Next Gate

Eight manifest rows authenticate the exact current canonical preimages; none
claims a new addition. Relative to those still-unmodified canonical sources,
the patch contains the observed guard formatting and the cumulative test-only
repair. Relative to the actual v2 formatted test, only the two std imports
change. All original v1 base identities and compiler-alias history are retained,
and the immediate failed CPU receipt/streams/source map are separately pinned.

Root must run a fresh full ten-phase CPU attempt with all 39 tests and then
qualify managed gfx950 lowering and device execution. This proposal claims no
successful v3 CPU outcome, HSACO, observed ABI, GPU, numerical acceptance or
performance result. No source/controller from the failed attempts is edited.
