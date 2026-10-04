# Device Runtime Audit Selector

Unfrozen source proposal with seven authored synthetic tests. No author imports,
test execution, SSH, native launch or GPU work. Root must bind `GPU_PACKAGE` to
the actual frozen device-intake manifest before this wrapper can run.

The command has exactly six arguments:

```text
audit_device_runtime.py LABEL ROLE CPU_PATH CPU_SHA BINARY_PATH BINARY_SHA
```

`ROLE` is `parent` or `worker`. Labels are
`device-timing-runtime-ROLE-v228-vN`. The CPU receipt must be directly under the
matching existing evidence namespace: `device-parent-cpu-v228-vN/complete.json`
or actual `device-routing-cpu-v228-v2/complete.json`. The transported ELF must be
in `E/device-timing-runtime-v228-v1/` with its actual role-specific executable
name. Its extent comes from the digest-checked body, not a guessed CLI size.

Root transports four records for each CPU: `complete.json`,
`sources-before.json`, `sources-after.json` and selected Cargo build stdout
(`parent-builds-stdout` or `worker-build-stdout`). Keep their original `E/...`
paths so the unchanged receipt FilePins resolve. The executable moves to the
explicit runtime directory; its size/digest must match the qualified original
Cargo artifact. No original ASROCK build target or source directory is opened.

The selector loads the frozen intake's `cpu_evidence` and `deployed_binary`
helpers without calling `context`. This checks the actual CPU generation,
unchanged source map, Cargo artifact and transported ELF without constructing
the runtime review that this audit will later inform. Parent qualification is
required to be actual, not an invented future hash. This audit does not grant
first-run GPU admission; the separate complete engineering intake does that.

All audit execution is delegated unchanged to
`p227-prefix-runtime-audit-v2/audit.py`, SHA256
`def16c2f69c082fa273596bef0caf7a2af195d07d2ac6f96d944706d5f44a514`.
Its manifest is
`9b80913aa0dac2da2a52bfe6e053a4db09a5e4c5647731fd269f39719e8bb062`.
The existing custody helper, `frozen_owned.py` and topology helper are loaded
through their original byte pins. No extra helper copies or PYTHONPATH are
required in this wrapper directory.

The original smci MI350 host/device checks, 120-second audit interval, 30-second
owned `readelf` and `ldd` leaves, 2 GiB address-space bound, stream caps, file
limits, before/after topology, library aliases, software observations and
natural ownership/reaping checks are preserved. The selected parent and worker
are never invoked. An existing output directory is refused.

The result retains the original runtime-audit schema and false authority flags.
It is observational evidence for a later root-authored review, not a review,
GPU qualification, numerical result, calibrated timer or performance claim.
Synthetic selector/loader tests do not replace the actual runtime audit.
