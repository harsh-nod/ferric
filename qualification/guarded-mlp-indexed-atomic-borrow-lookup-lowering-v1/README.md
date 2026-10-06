# Borrow Lookup Guarded Lowering

## Fixture Qualification

The bounded MI350 lowering-admission fixture run passed all 19 selected
methods: 15 audit-binding methods and four unchanged normalization methods.
The controller completed in 6.120017 seconds. Its single owned child exited
naturally with code 0, was reaped, and left no process group. Source, input and
tool postchecks were clean; there were no failed, skipped or expected-failure
methods.

The [actual receipt](controller-tests-v1/attempt-v1/evidence/complete.json) is
12,836 bytes with SHA-256
`1efe8a746ec7e269575ed74138aaca2eaa422c5bfbf5fa8a107eed6ddbd1bf49`.
The original [child observation](controller-tests-v1/attempt-v1/evidence/indexed-atomic-borrow-lookup-lowering-tests.stdout)
and [test output](controller-tests-v1/attempt-v1/evidence/indexed-atomic-borrow-lookup-lowering-tests.stderr)
retain all 19 exact identities and passing statuses.

The [retention manifest](controller-tests-v1/attempt-v1/retention-manifest.json)
authenticates 15 original bodies, including seven raw records. The archive has
16 ordinary members totaling 180,556 body bytes. The 39,270-byte archive
`guarded-mlp-indexed-atomic-borrow-lookup-lowering-tests-evidence-v228-v1.tar.gz`
has SHA-256 `fae0902924d3d8b0835bf9a900c5f64b0c628d5f485579e6232c68d0199aff37`.

The retained [adapter](controller-tests-v1/attempt-v1/lowering.py),
[admission fixtures](controller-tests-v1/attempt-v1/test_lowering.py) and
[normalization fixtures](controller-tests-v1/attempt-v1/test_normalization.py)
are the reviewed `b12f845b`, `43eb62be` and `6d4d91e2` bodies. The positive
fixture requires the current borrow audit-input schema; the negative rejects
the preceding use-lookup schema. All 19 methods remain, with unchanged resource
and lifecycle limits and the same normalized compiler command/output checks.

This section records synthetic adapter evidence, not an actual guarded
compilation. CPU qualification and real tool inspection are separate gates.
These fixtures do not establish an HSACO, GPU launch, alias-analysis verdict,
numerical correctness or performance improvement.

## Actual Guarded Attempt

The actual guarded compilation failed. The compiler child exited naturally
with code 1 after 117.666543 seconds; the complete controller run took
121.316898 seconds. The child was reaped, its process group was absent, and
there was no timeout or forced cleanup. All 209 recorded input pins remained
unchanged, the 5,307-row source map remained the qualified candidate map
`955e37e4`, and postcheck errors were empty. No artifact was produced.

The original [stderr](attempt-v1/evidence/compile.stderr) reports:

```text
semantic-to-ranked projection rejected semantic CFG projection exceeds the ranked block limit
```

This generic message does not report a function, block count or rejecting call
site. It does not establish that the currently documented block ceiling was
exceeded by any particular amount or identify the responsible guard. The
previous `permits_address` work-limit diagnostic is absent from this output;
that observation is not an alias-analysis verdict or a guarded-success claim.
The receipt retains `actual_failure_caller_identified=false`.

The [recorded command](attempt-v1/evidence/compile.command.json) and environment
match the preceding use-lookup attempt after substituting fresh generation
paths and the measured final backend hash. The qualified candidate, vendor,
extractor and other tools, resource limits and `optimized-inline-v1`
normalization remain unchanged. The actual backend pin is
`e131d122658f9f874d310126c141ae23d16606af7fd18370a2bba91c26da647f`.

The 14,686-byte [failed receipt](attempt-v1/evidence/failed.json) has SHA-256
`cb1def314b36717465478dbf88d521f094749b34d805b9f0e07ed5f3c5421778`.
The [retention manifest](attempt-v1/retention-manifest.json) authenticates 12
original bodies, including all ten raw records. The archive contains 13
ordinary members totaling 4,227,536 body bytes. The 674,271-byte archive
`guarded-mlp-indexed-atomic-borrow-lookup-lowering-evidence-v228-v1.tar.gz`
has SHA-256 `b84ede369b867459f02eb2c05765895c3cfb4b30a9a13311cfbc95b75115c66f`.
The 8,018-byte stderr pin is
`646b7e35b6dbf8cd8b00e43f52008824ccc06634d636238fbc23fc33d9ccdb83`.

CPU qualification, synthetic fixtures and the real loader audit are separate
evidence; none converts this failed attempt into a success. There is no HSACO,
load or launch authority, GPU execution, numerical result, full-model
acceptance or performance claim.
