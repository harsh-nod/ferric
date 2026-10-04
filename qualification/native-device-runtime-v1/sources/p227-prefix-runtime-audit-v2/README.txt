P227 MI350 runtime/platform observations V2

Status: frozen isolated successor; 14 authored pure tests not executed by author.
Root and peer execution remains separate. No source/build/GPU changes.
This is a data collector, not an engineering review, image approval, deployment
verifier, idle certificate or GPU launch authority. Root supplies substantive
runtime/platform review notes only after inspecting its records.

Runtime invocation (root-owned, fresh output only):
  /usr/bin/taskset -c 8,9 /usr/bin/nice -n 10 /usr/bin/python3 -B \
    E/p227-prefix-runtime-audit-v2/audit.py prefix-runtime-audit-v227-v2 \
    E/prefix-parity-deployment-v227-vN/gfx950-qwen-prefix-tiles-comparison-v6 \
    ACTUAL_BINARY_BYTES ACTUAL_BINARY_SHA256
E is /home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220.
Run once on smci350-rck-g03-b19-03 as UID9661. Root must supply the authenticated
deployed binary pin, not an arbitrary executable. The program is never launched
in a GPU mode. The two standard dynamic-linker tools inspect the trusted ELF.

The unchanged observer custody helper and owned-process helper are copied here
with their exact frozen hashes. The original run_p217_mi350.py topology helper
is loaded only at its checked existing MI350 path/hash; its main is never called.
No compiler, Cargo, rustc, build-host source, historical target or image is opened.

Each readelf -d / ldd leaf has 30 seconds wall time through D.run_coordinator and
30 CPU seconds, 2GiB address space, 1MiB per-file RLIMIT_FSIZE and no core via
prlimit. Logical argv in audit.json exactly matches observer runtime_review;
command.json separately records the actual prlimit argv, cwd and environment.
owner.json retains the frozen helper's actual outcome separately, before audit
success publication. Abnormal cleanup/nonzero exit/stderr/oversize refuses.
If the frozen helper itself throws before returning an outcome, existing streams
and started record remain, failed.json is attempted, and no outcome is invented.
The controller uses 2GiB AS/120 CPU seconds/64MiB files; elapsed checks cap the
accepted observation interval at120s. Existing40/38GiB disk floors are retained.

complete.json records binary, host, actual boot, ordered topology before/after,
exact observer topology-identity projection, software_audit, readelf/ldd audit
FilePins, both owner/command FilePins, canonical shared-library pins and original
alias paths, tool pins and all pinned inputs. It is explicitly reviewed=false
and all GPU/production/numerical/runtime-premise flags are false. Dynamic busy
and VRAM-use readings are retained but are not an idle/health proof.

software.json contains actual os.uname(), bounded os-release/proc-version/AMDGPU
module-version/source-version and /opt/rocm/.info/version[-*] observations.
Absent optional files are explicit; pseudo-file bytes are snapshots, not claimed
regular-file custody pins. No software release string is hardcoded or inferred.
Libraries are the exact closed ldd alias roster with each resolved canonical
regular file hashed; alias targets and all pinned bytes are rechecked before
publication. Readelf NEEDED identities must be covered by linked ldd names.

Root may use the retained fields to author the exact frozen observer's separate
runtime-review-v1 and platform-review-v1 documents, with actual substantive notes.
The collector intentionally never writes either reviewed schema. The root still
must qualify deployment/source/native artifacts and provide all other reviews.

V1 actual readelf/ldd both succeeded, but coverage refused because the actual
ld-linux-x86-64.so.2 DT_NEEDED identity was only a direct ldd path, not a named
arrow row. V2 permits only that exact direct interpreter basename to satisfy its
dependency identity, rejects duplicate names across direct/linked rows, and
preserves full path, alias and byte rechecks. V1 and its failure stay immutable.
The two actual stdout streams are retained verbatim as digest-checked fixtures.
No command, scope, owner, resource or output schema changes. Actual observer
intake must use the matching observation V3 parser successor.

Pure tests: 14 methods, no child processes/GPU or platform probes. One test reads
the deployed exact frozen observer prepare.py/validation.py, extracts the actual
runtime_review function and checks generated records using an explicitly synthetic
review. These dependencies must exist at E/p227-prefix-parity-observation-v2.
  python3 -B E/proposal_tests_p226.py p227-prefix-runtime-audit-v2 \
    ACTUAL_MANIFEST_SHA 'test_*.py' 14 prefix-runtime-audit-pure-v227-v2
