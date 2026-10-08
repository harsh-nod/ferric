# Forward-Phase Native Admission: CPU Qualification

The new three-record native-admission workflow passes **227 tests** on MI350
in **112.153889 seconds**, with four natural exit-zero processes, all reaped
and with absent process groups. This is a CPU-only qualification of admission,
preparation and retention. It does not run a native parent, model or GPU.

| Supervised Leaf | Passed | Scope |
| --- | ---: | --- |
| Legacy tests | 171 | Original two-record tests, unchanged and isolated |
| Forward CPU admission | 29 | Real qualified receipt/map/ELF admission and mutations |
| Forward retention | 12 | Synthetic three-record success, failure and absence |
| Forward preparation | 15 | Real preparation/runner entry points and refusal paths |

The separate [146-test checker](../checker-cpu-v1/README.md) comprises 126
unchanged cases plus 20 new three-record cases. This workflow includes those
126 legacy cases inside its unchanged 171-test suite, followed by 56 new
workflow tests. **146 and 227 are overlapping suites, not 373 distinct tests.**

## What Was Checked

The CPU-admission tests authenticate the actual diagnostic worker and parent
receipts, source maps and ELF bytes on MI350, then exercise isolated mutations
without replacing acceptance predicates. Preparation tests exercise the actual
reader, hash checks, writes and runner admission wrapper. They preserve the
historical 2,048-token input, selected image identities, devices and deadlines.
Process-global resource/signal effects are isolated in fixtures; the supervised
qualification itself enforces its real limits.

Retention tests derive a synthetic three-record case from authenticated
historical originals. Genuine legacy and new successful admission are required
before testing tampering, failed prefixes, absent cases, closed source rosters
and archive custody. The complete original stderr is retained, not replaced by
a two-record prefix. The policy and currentness records remain independently
authenticated; the third record binds the exact LF-inclusive prior records.

The direct live-tree native exporter path is not exercised by these retention
unit tests. Its real invocation and any native GPU result require their own
original evidence. Data-only request preparation has separately passed; no
new native timing, numerical result or speedup is claimed by this CPU packet.

## Bounds And Ownership

All four leaves have a 120-second wall cap within a 600-second whole-run bound
and 50-second cleanup reserve. The owned supervisor records original commands,
stdout, stderr, child registration and results. The run uses CPUs 8-9, nice 10,
hidden GPUs, 512 MiB address space, 16 MiB files, 4 MiB streams, a 64 MiB private
temporary budget and unchanged 40/38 GiB storage floors. No temporary files
remain. All 364 staged source/fixture identities and both tool pins are
unchanged after the run.

Actual-artifact-backed reads are CPU data checks, not execution of the retained
ELFs. No model shard is read. Device/process ownership, numerical tolerances,
the existing Tail V4 selector and native deadlines are not relaxed.

## Original Evidence

- [Complete original archive](../native-admission-cpu-v1.tar.gz)
- [Original successful terminal](complete.json)
- [Checkpoint manifest](../manifest.json)

Archive: 8,952,922 bytes,
SHA256 `31da38260d164196ef984af1bb708ba84780c08abe7916cdbbaa77c419945090`.

Terminal: 355,146 bytes,
SHA256 `e5ecec31ead47b13b6fb7eaab5fbc17ed46f2744bd47eb9fd7be6267fc210ac0`.

The archive contains 389 members: 364 staged source/fixture originals, the
input manifest, 22 original raw evidence files, the original terminal and an
archive manifest binding the preceding 388 bodies. The staged closure includes
two byte-identical copies of 152 immutable fixture bodies, needed by the
separate legacy and new source-relative fixture paths. The archive retains
both copies; they are not additional observations.

Only this README and the exact original terminal are expanded here. The full
source, fixture and raw-stream closure stays in the archive. ELF bodies,
model weights and private Cargo caches are not distributed. The recorded
controller is a fixed-host historical MI350 recipe, not a portable install
script or public native launch entry point.

The [phase checkpoint](../README.md) and
[community demo](../../../docs/GFX950_COMMUNITY_DEMO_V1.md) retain the scope:
40 prompt forwards and zero generated tokens for the diagnostic route.
Full2303 native feasibility, exact independent 256 generated IDs/raw decoded
bytes, the 700 tokens/s target and issue #42 M0-M7 remain open.
