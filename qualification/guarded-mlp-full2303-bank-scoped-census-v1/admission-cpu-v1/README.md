# Full Bank/Census Comparison CPU Qualification

Executed on `mi350` on 2026-10-07 UTC. The single owned CPU phase passed
all 62 tests with no failures, errors or skips in a 50.382-second controller
run. It exited naturally, was reaped, and left no owned process group.
All 18 source bodies were unchanged; original tool and source postchecks
passed. GPUs were hidden and no model or native worker was executed.

The test set preserves all 42 earlier full/scoped comparison tests and adds
12 bank/census policy tests plus eight comparison tests. It checks original
records, bank generations, checked census subsets, full output-history hashes,
original deadlines, process retirement, and the strict 256-ID/raw-decoded-byte
comparison. A reference-authentication fixture is mocked at the inherited
test boundary. These synthetic tests are not an actual native/reference match.

The [original terminal](evidence/complete.json) is 24,697 bytes, SHA256
`ea0209cc6dfe21497e94ace32d494cb47137f4d601b987e43fd35ef5be1afd67`.
The [manifest](manifest.json) binds 27 original bodies; with the manifest,
the retained archive contains 28 members: 18 sources, seven raw phase/source
bodies, the terminal, the collector, and the manifest. The separate
[retention receipt](retention.json) and this README are not archive members.
The original archive is 67,229 bytes, SHA256
`71c174d0c70160f58e0ffc92f90ab951c36b6205364080840d917d1f1ab81352`.
It is also retained outside this repository in the local session evidence
and on MI350; every original member is published here.

The [controller](run_cpu.py) and [collector](pure-evidence.py) retain the
existing process/resource bounds: CPU 8/9, 512 MiB test address space,
180-second whole-run/120-second leaf/50-second cleanup bounds, and unchanged
40 GiB initial/38 GiB live free-space floors. The collector rechecked remote
tools during export; local retention does not claim to rehash remote tools.

No native Full2303 run has been completed. Full-run feasibility under the
original one-hour deadline, numerical acceptance, repeated equal-work
performance, GPU overlap, and 700 tokens/s remain separate open gates.
