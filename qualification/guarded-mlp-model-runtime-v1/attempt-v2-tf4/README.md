# Failed TF4 Attempt V2 Retention

This is a data-only copy of the original failed outer TF4 attempt. The original
`tf4/failed.json` is 91,510 bytes with SHA-256
`8ced1ef7f167348fbcb68ce56cca8fc8e2238273ebe18fd737127563779871ce`.
It remains byte-for-byte unchanged and `passed: false`.

The outer controller's sole recorded error is
`RuntimeError: one actual worker announcement`. Its retained parent stderr says
`finite guarded owned child pid=2809580 pgid=2809580; no native setup acknowledged`,
followed by four completed-position messages. The parent exited zero, the native
summary records four completed frames and a closed child, and all 11 supervised
phases record clean owned-process retirement. These observations do not override
the failed outer receipt.

The recorded teacher-forced inputs are `[9112, 2190, 3772, 220]`; observed outputs
are `[67, 198, 25, 16]`. Local bank generations are `[1, 1, 2, 2]`. The retained
payloads and wire digests are joined without replaying the project's validator
or computing an independent numerical reference. Numerical acceptance,
full-model acceptance, the long benchmark workload, and performance claims
remain false.

## Closed Copy

`retain.py` accepts only the exact fetched directory
`guarded-model-gpu-result-v2`. It verifies 10 root inputs and 77 TF4 files:
76 raw files plus the original failed receipt. An earlier handoff count of 11
root inputs was incorrect; no missing file is invented. The original 87 bodies
total 3,791,177 bytes. The retained capsule adds this README, the data-only
retainer, and a manifest, for 90 files and 89 manifest pins.

All raw paths, sizes, and SHA-256 digests must match the original receipt. The
helper also joins phase commands, started identities, results, native frame
payload hashes, the two prepared request/plan pairs, and recorded before/after
idle observations. It rejects symlinks, hard links, nonregular files, extras,
unsafe names, changed inputs, existing destinations, or bodies beyond its
1 MiB per-file and 8 MiB total limits. JSON integers are parsed without floating
point conversion; original JSON and binary files are copied, not reserialized.

The two AR4 inputs are retained as prepared context only. No AR4 execution is
present in this attempt. The failed receipt preserves external source, image,
tool, and CPU qualification pins; their full bodies are not recopied here, so
this is not a complete transitive dependency archive.

## Invocation

After independent source review, the root agent runs `python3 -B retain.py`.
The fresh destination is
`qualification/guarded-mlp-model-runtime-v1/attempt-v2-tf4` in Ferric.
A zero retainer exit means that the failed evidence copy verified, not that the
GPU attempt passed. The helper performs no build, test, project import, remote
operation, GPU invocation, commit, or numerical validation.
