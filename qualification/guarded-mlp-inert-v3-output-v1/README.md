# Pinned Inert Output-File Intake

This CPU checkpoint advances Ferric [issue #42](https://github.com/harsh-nod/ferric/issues/42)
after the [inert completion-content consumer](../guarded-mlp-inert-v3-consumer-v1/README.md).
It adds retained private output-file custody to that consumer and fixes a
real same-length rewrite hole found by the new negative controls.
The selector remains disabled. No worker compilation, GPU/HSACO, numerical
or performance evidence, premise discharge or M0-M7 completion is claimed.

## Implementation

[PinnedOutput](source/crates/cargo-fe2o3/src/protected_compiler_handoff_v3/inert_semantic_output_v1.rs)
borrows the original [PinnedDirectory](source/crates/cargo-fe2o3/src/project.rs)
and retains the original file object. It accepts only the fixed basename
`inert-semantic-handoff-v3.bin` under an absolute alias-free private mode0700
directory owned by the current user. The descriptor-relative open uses
`RDONLY | NOFOLLOW | NONBLOCK | CLOEXEC`.

The file must be ordinary, nonempty, bounded, private, non-executable,
current-user-owned and link-count1. Checks bind device/inode, length,
mode, uid/gid, link count, mtime/ctime and nanoseconds; atime is intentionally
excluded. Reopening only the fixed slot relative to the retained directory
checks that its entry still names the original file, and directory pathname
validation rejects substitution. Caller aliases and arbitrary output paths
are not accepted. Symlinks, multiply-linked files, directories and FIFOs
are refused; nonblocking open prevents a FIFO from hanging before inspection.

Read allocation and extent are bounded. The captured complete outer bytes
must match an independently expected V3 outer identity. A typed
`CapturedOutput` carries the original directory/file custody, bytes and
expected identity into [Completion](source/crates/cargo-fe2o3/src/protected_compiler_handoff_v3/inert_semantic_completion_v1.rs).
Its production constructor now requires that typed capture, not raw bytes,
and retains the file object through decoding and final serialization.
The complete outer capsule/formal/proof preimages are not replaced with V2.

The existing engineering reader was inspected but not reused verbatim:
it opens an ambient pathname and returns bytes without retaining the original
output objects. The new path needs descriptor-relative custody and later
revalidation; the existing project directory abstraction is reused.

## Bug Found And Fixed

The [first implementation](first-failure/source/crates/cargo-fe2o3/src/protected_compiler_handoff_v3/inert_semantic_output_v1.rs)
used metadata-only rechecks after reading. The actual
[negative-control output](first-failure/output-tests.stdout) reports seven
passes and one failure: a same-length rewrite after the first read was
accepted at `change 2`. The original run failed and remains a failure;
its source, receipt and logs are preserved. We do not infer a particular
filesystem timestamp mechanism from that observation.

The fix compares the retained file against the captured bytes using 64 KiB
positioned reads, checks EOF, and brackets comparison with metadata/slot/path
revalidation. It does not allocate a second complete outer buffer. Byte checks
now run at capture, typed transfer, post-decode and final serialization.
The transfer test also checks a same-length rewrite after capture.

These bounded repeated observations are **not atomic writer exclusion**.
They do not prove future immutability, compiler authorship, owned wait/reaping,
proof replay, runtime-premise discharge, freshness or execution authority.
Successful file tests use real temporary filesystem objects containing
explicit inert schema fixtures. No live-parent completion or real compiler
child was executed in this campaign.

## Actual CPU Qualification

SSH mi350 CPU8/9, nice10, offline/jobs2, 12 GiB service memory maximum,
swap disabled and owned subreaper/process groups:

| Cohort | Actual Result |
| --- | --- |
| Invocation library | 103 passed |
| Preserved engineering frontend | 25 passed |
| Capture/closure upgrade | 8 passed |
| Default-off selector | 10 passed |
| Binding-wrapper lifecycle | 6 passed |
| Completion content | 9 passed |
| New pinned output tests | 8 passed |
| Prior qualified compiler gate tests | 2 passed |
| Total | 171 passed |
| Owned phases | 16 natural status0 exits, reaped leaders, groups absent |
| Elapsed | 77.902929 seconds |
| Source inputs | 5,815 unchanged |
| Source/tool/dependency/artifact postchecks | Passed |

See [passing receipt](cpu/complete.json), [output outcomes](cpu/output-tests.stdout),
[consumer outcomes](cpu/consumer-tests.stdout), and [data audit](publication-audit.json).
The fresh V2 qualification uses the repaired source and does not overwrite or
replay the failed V1 outputs. Both original controllers and all leaves retired.
Five driver ignores remain unchanged/unrun. Two compiler gate tests reuse
the prior qualified compiler ELF; full compiler/device/lowerer suites are
not rebuilt or rerun.

The eight new tests cover actual fixed-slot private-file capture through the
existing content codec, name aliases/bounds, symlink/hardlink/nonregular/FIFO
refusal, identical-byte file replacement, directory substitution, truncation,
growth, same-length writes, permissions, empty/public files and directories,
independent outer identity, and revalidation before typed transfer. Tests
modify only their owned temporary fixtures; original evidence is preserved.

## Remaining Work

Implement a separate bounded **owned child lifecycle** using existing pinned
executable/process helpers where applicable: record the original owned child,
wait/reap and drain correctly, handle unsuccessful exit/timeout/partial output,
and feed the typed captured output to inert completion. CPU controls should
use explicitly labeled scripted helper children, not claim compiler execution
or GPU qualification from a fixture. Preserve all original objects and keep
the expected outer identity separate from authentication.

Do not enable the selector or compile a new worker before the complete
matching extraction-only route is qualified. The eight actual runtime
requirements, early-stop CFG issue and unresolved GPU completion hold stay
closed. No signed-Verus, compiler-refinement, numerical, artifact,
publication, load or launch authority is granted; 700 tokens/s remains unmet.

## Provenance

[Runner](run_cpu.py), [input preparer](prepare_inputs.py), [exporter](export.py),
[manifest](manifest.json), [originals ledger](originals-ledger.json) and
[archive](originals.tar.gz) retain tested sources and raw phases. The archive
also retains the first failed receipt/logs/controller and original affected
source. Complete input rosters and compiled/toolchain content hashes are
retained, but complete compiled/toolchain/source trees are not embedded.
