# MLP Coverage Helper Inlining

Engineering checkpoint for [Ferric issue #42](https://github.com/harsh-nod/ferric/issues/42),
authored and tested on `ssh mi350`, 2026-10-15 UTC. This worker targets gfx950.
The full Qwen decode, independent numerical validation and 700 tokens/s
requirements remain open.

## Actual Worker Probes

Both V17 attempts still exit with status 1 and emit no HSACO. The original
errors are retained at line 67 of [fixed stderr](pair/fixed/compile.stderr)
and [early stderr](pair/early/compile.stderr):

| Attempt | Stage | Root / body | KIR block / operation | Refused kind | Leaf seconds | Whole seconds |
| --- | --- | --- | --- | --- | ---: | ---: |
| Fixed-round V17 | wave-replay | 0 / 0 | 486 / 0 | call | 119.974367991 | 120.906550489 |
| Early-stop V17 | wave-replay | 0 / 0 | 296 / 0 | call | 120.268031583 | 121.198859610 |

Each failure says the operation lacks a complete local effect summary.
Compared with V16, the captured root/body and block coordinates differ.
Those numeric changes alone do not identify the remaining callee or prove
that the previous production call disappeared. Kernel IR block IDs must
not be passed to a semantic MIR block selector without a correspondence join.

Both compiles exited naturally, were reaped and left no owned process group,
with no signals, timeout, forced cleanup, storage failure or postcheck error.
The services retired before export. Their source and tool ledgers,
original semantic captures and source maps are retained in the archive.
The new helper policy passes CPU qualification; full worker compilation
remains incomplete. The next step is an exact source/callee diagnosis of
these retained failures before another source change or compile attempt.

## Repair

The previous [wave recipe diagnostic](../guarded-mlp-wave-effect-diagnostic-v1/README.md)
identified an incomplete local call-effect summary during both actual
worker compilations. The scalar `complete_coverage` helper was the only
unannotated helper directly on this module's round-completion path.

The [three-file patch](correction.patch) adds the module's existing
AMDGPU-only `rustc_force_inline` policy to that helper. Its body, signature,
call position, coverage rules and retirement ordering stay byte-identical.
Inlining lets the existing checked route inspect the scalar operations
rather than accepting an unknown call summary.

The compiler's reviewed provider closure is refreshed after formatting all
provider source and tests. The 53-file transcript uses the raw
`FE2O3/WORKGROUP-SYNC-PROVIDER-SOURCE-CLOSURE/V1\0` domain, then sorted
package-relative UTF-8 paths and full bodies. Each path and body is
separately prefixed with its byte length as a little-endian u64.
The new closure is
`9eea1b004f0682c3a5859ece38a9424c2dc2c0e3096ff2e039440761eb71afb7`.
A new source-authentication test checks the exact annotated body, accepts
this closure and refuses its previous `91b3c53f...` identity. All earlier
identity refusals remain.

No generic call-effect acceptance rule changes. The recipe preflight,
ranked binding, custody and inert archive checks remain in place. A
complete transitive helper summary alone is still insufficient to grant
a local operation contract.

## Regression Evidence

The independent coverage table covers idle, normalization, both projection
boundaries, activation, down projection, stop, invalid tokens, invalid lanes,
integer maxima, invalid-write flags and exact/adjacent write counts.

The retirement case acquires a real normalization claim, supplies an
incorrect per-lane write count, and checks all 548 state words. Only the
`INCOMPLETE_WRITES` error bit may change; claim ownership, arrivals and
completion remain unchanged. The worker retires, executes no task,
returns the uniform stop token, and accepts zero-write stop coverage on
all 64 lanes. The preexisting exhaustive coverage test is unchanged.

The [complete CPU receipt](cpu/complete.json) records 19 clean phases in
274.919629870 seconds:

| Full suite | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Compiler library | 1,340 | 0 | 24 existing |
| Device library, CPU tests | 360 | 0 | 0 |
| Semantic lowerer integration | 125 | 0 | 0 |
| Lowerer library | 769 | 0 | 0 |

All 2,594 passes are reconciled against named inventories and original
stdout: [compiler](cpu/compiler-tests.stdout),
[device](cpu/device-tests.stdout), [integration](cpu/lowerer-tests.stdout)
and [lowerer unit](cpu/lowerer-unit-tests.stdout). Exactly one compiler
test and one device test are added; all inherited lowerer tests remain.

The final backend is built after the suites pass. Its receipt binds
5,812 inputs, 5,808 project files, tool and dependency records, and six
selected ELF artifacts. Source preimages and postimages are retained.
The executable lowerer tests and extractor are byte-identical to the
qualified parent; the backend and affected test products have fresh pins.

## Execution Envelope

The CPU qualification ran once in the owned systemd service
`ferric-coverage-inline-cpu-v228-v1.service`, on CPU 8/9 at nice 10,
with `MemoryMax=12884901888`, `MemorySwapMax=0`, and a 3,700-second
outer runtime limit. The receipt records the boot ID, original PID and
start ticks, cgroup path, allowed CPUs and observed memory controls.
The service exited successfully and retired before the worker probes.

Internal suite bounds remain 3,600 seconds whole, 1,800 seconds per leaf,
1,200 seconds leaf CPU, 50 seconds cleanup reserve, 12 GiB address space,
6 GiB cache, 64 MiB streams, and 1 GiB files. Initial/live free-space
floors remain 40/38 GiB. Cargo is offline with two jobs.

Each fixed/early compile uses its own CPU 8/9, 12 GiB, no-swap service
with a 700-second outer runtime limit, retaining the 600-second compile
wall/CPU and 50-second cleanup limits. Outer Cargo job settings do not
establish the frontend's cleared inner Cargo environment concurrency.

## Evidence

[manifest.json](manifest.json) binds the qualified CPU receipt, selected
artifact pins, source patch and both compile results.
[originals.tar.gz](originals.tar.gz) contains 152 members, including 151 exact
originals and a path/pin ledger, totaling 55,114,457 expanded bytes. It includes
all CPU raw evidence, source preimages/postimages, parent receipt, controllers,
input manifest, both seven-file fixtures and both original diagnostic captures.
The exporter rehashed originals and read back every archive member.

| Object | Bytes | SHA-256 |
| --- | ---: | --- |
| CPU receipt | 5,693,317 | `fc68cb4460c46bb3a6a6ca338814b81817d33acaceeb46d8b7181b7ea33e3aa7` |
| Backend | 206,472,776 | `85b0cf0c661d5b27577c8dfb9061013a9ee15e8b0d719f6df2d46a817d307ad8` |
| Fixed result | 6,799 | `3bfb5217df9af267686e81928d4f2c880acc13549e4d576ab0d11561f6805325` |
| Early result | 6,800 | `a7ed70c4a3e4cc0af2b723dec128a33e620f9f7dfe250427028ec499fc96457e` |
| Patch | 5,955 | `ac9684885c3935286f0fbe240d449cf8b9a5357163a9e85bbe2c08854500140b` |
| Archive | 7,887,301 | `237cfa7c1bf92490c3f21dac00b721a35413590b20c96061dc7cc1e96b0ff7f2` |

## Recovery And Limits

Before this work, the original process IDs, receipts, locks, resources,
source inputs and native child registry were reconciled after the user
resumed the original stream. The prior runs were terminal and preserved.
The earlier diagnostic packet was published separately as
`c61b6d6354adb0194eccb32057b32bc68adc721a`.

GPU device occupancy was inspected through passive OS process metadata.
That observation does not clear the existing device-retirement hold.
No GPU launch, independent model output, token-throughput measurement or
milestone closure is inferred from this CPU qualification. Actual
source-pinned gfx950 artifact and hardware qualification remain required.
