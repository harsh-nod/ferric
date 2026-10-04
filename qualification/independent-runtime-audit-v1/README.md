# Independent Runtime Audit

This namespaced collector prepares for the new independent-profile deployment.
The [actual CPU result](cpu-result.json) records 18 passing synthetic tests on
`mi350-2`, with no skips. It is not an observation of a deployed binary or GPU.

The only behavior change from the preceding collector is CLI namespace handling.
The deployed executable and output must belong to the new independent-profile
directories; historical deployment paths are refused. The 14 existing tests
remain, with four new parser, routing and unchanged-helper checks.

The collector retains the existing bounded `readelf` and `ldd` inspections,
owned-process cleanup, canonical library hashes and alias checks, boot/topology
checks and resource limits. It records observations but never writes a reviewed
artifact, qualifies an image or grants launch authority.

[Published source](source/audit.py), [tests](source/test_audit.py),
[manifest](source-manifest.json) and [test log](tests.log) match the executed
package. The source README preserves authoring-time draft status; the CPU result
records the subsequent test run. Tests require the two pinned historical schema
helpers listed in the result at their original evidence paths. They mock child
processes and platform probes.

An actual MI350 audit of the new deployed executable remains pending. Its result
must then be reviewed against the current boot and resolved libraries; the reused
observation schema does not transfer historical binary provenance.
