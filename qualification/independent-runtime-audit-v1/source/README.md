# Independent Profile Runtime Audit Draft

Unfrozen and unexecuted. The author has not imported or syntax-checked this
package, run its tests, opened an SSH session, built software, or invoked any
runtime audit. There is no manifest or synthetic successful receipt. Root owns
review, freezing, tests, execution and publication.

## Narrow Change

This is a namespaced successor of
`p227-prefix-runtime-audit-v2/audit.py`, whose unchanged preimage is included at
`preimage/audit.py`, SHA256
`def16c2f69c082fa273596bef0caf7a2af195d07d2ac6f96d944706d5f44a514`.

Only CLI label/deployed-directory handling changes. Existing argument validation
is moved into `arguments`, which the real `main` calls. The new namespaces are:

- Audit output: `E/prefix-independent-runtime-audit-v228-vN`.
- Deployed executable:
  `E/prefix-independent-deployment-v228-vN/gfx950-qwen-prefix-tiles-comparison-v6`.

Legacy namespaces are refused; there is no fallback. The root must supply the
actual future deployed binary byte count and SHA256. This package does not
invent that binary's successful build, deployment, or GPU qualification.

All other functions are unchanged: host/boot identity, topology projection,
software snapshot, ELF dependency parsing, canonical library pinning/alias
guards, exact leaf command construction, ownership cleanup/reaping, source pin
checks, and result/failure publication. The existing observation schemas remain
byte-compatible (`ferric-p227-prefix-runtime-audit-v1` and its failure schema);
the new output namespace/controller pin distinguish this run from historical
ones. The schema describes an inert audit, not historical artifact provenance.

The frozen custody and process helpers are copied without changes:

- `run_row_facts_v2.py`: `244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820`.
- `frozen_owned.py`: `ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583`.

The reviewed topology helper remains
`R/evidence/resident-output-tp2-v217/run_p217_mi350.py`, SHA256
`6016c30f46aa32abf9d175f329a0110e86cbf79d1363f1803dba3c86c1bad50a`.
The retained readelf/ldd fixture files are unchanged parser-test inputs, not
observations of the new executable.

## Actual Invocation

After a separately verified deployment exists, root may execute the frozen
package on `mi350` with actual parameters:

```text
taskset -c 8,9 nice -n 10 /usr/bin/python3 -B audit.py \
  prefix-independent-runtime-audit-v228-vN \
  E/prefix-independent-deployment-v228-vN/gfx950-qwen-prefix-tiles-comparison-v6 \
  ACTUAL_BINARY_BYTES ACTUAL_BINARY_SHA256
```

`E` above abbreviates the exact existing absolute evidence root; it is not a
literal accepted CLI path. The host must be `smci350-rck-g03-b19-03`, uid/euid
9661, affinity 8/9, nice 10. Initial free space is at least 40 GiB; the retained
floor is 38 GiB. The binary must be canonical, hash/extent matched and ELF64
little-endian. Only root's task-owned deployed binary is audited.

The two real leaves remain `/usr/bin/readelf -d BINARY` and `/usr/bin/ldd BINARY`,
each wrapped by the unchanged owned supervisor and `prlimit`: 2 GiB address
space, 30-second CPU/deadline, 1 MiB streams, no core files. The controller has
the same 120-second total interval/CPU limit, 2 GiB address space, and 64 MiB
file cap. There is no GPU launch or compiler invocation. Actual command/start/
owner/stdout/stderr evidence is retained before interpretation. Natural exit,
empty stderr, no forced cleanup, absent groups and all owned processes reaped
remain mandatory. Library/tool identities and aliases, host/boot and topology
are rechecked before success.

## Authority Boundary

This audit emits `reviewed=false`, `authority=none`, `gpu_execution=false`,
`production_authority=false`, `runtime_premises_discharged=false` and
`numerical_acceptance=false`. Root must inspect the actual output before
authoring an explicit runtime/platform review for the new binary and boot.
The new deployment verifier, V7 image/source/ISA/arithmetic review, GPU
controller, three pre/post device audits and six numerical cases remain out of
scope. The runtime audit does not establish any of those prerequisites.

## Unexecuted Tests

All original audit tests are retained, including actual pinned runtime-review
schema replay from `E/p227-prefix-parity-observation-v2/prepare.py` and the two
retained parser fixtures. That historical module is a schema/helper input only,
never qualification of this new binary. Four additional tests cover new
namespace parsing, legacy/malformed input refusal before host probes, actual
`main` routing through the tested parser, and AST equality of every unchanged
function plus exact frozen helper hashes. Subprocesses and platform observations
are mocked by the existing tests. No test count is an observed execution result.
