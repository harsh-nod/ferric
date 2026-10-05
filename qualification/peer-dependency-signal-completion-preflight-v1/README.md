# Peer Signal Completion: Preflight Refusal

This checkpoint supports [issue #42](https://github.com/harsh-nod/ferric/issues/42)
and preserves the first attempted launch of the
[CPU-qualified V3 runtime](../peer-dependency-signal-completion-cpu-v1/README.md).
**No native process launched and no GPU queues were created by this probe.**

On `ssh mi350`, all 33 controller fixtures passed. The controller then refused
its all-eight-GPUs-idle precondition: an unrelated Python process was using GPU 0.
GPUs 1 through 7 were idle in both snapshots. The foreign process was neither
terminated nor modified. This is an environment preflight refusal, not a native
V3 failure or a GPU correctness result.

| Check | Result |
| --- | --- |
| Controller tests | 33 passed, 0 failed, 0 skipped |
| Native attempts | 0 |
| Owned leaves | readelf, ldd, pre-audit, post-audit |
| Leaf termination | All natural exit 0, reaped, process groups absent |
| Input postchecks | Passed |
| GPU readback / healthy Close | Not attempted |

The next controller must explicitly select an idle pair and bind its AMD-SMI
indices to actual KFD unique IDs before and after execution. It must preserve
the full eight-device roster, require the selected pair idle, and not claim
global idleness. Runtime source, CPU receipt and native result requirements
need no relaxation for that change.

[`result.json`](result.json) binds all 30 retained files. The original
[`failed.json`](retained/evidence/failed.json) SHA256 is
`f137bd377dcf9ae5c5961dc24b2ccef474fff87f51542e1d575d0191bcac1393`.
The native V3 path remains unqualified by this checkpoint. All M0-M7, model
numerical acceptance, sustained 2,048/256 and 700 tokens/s gates remain open.
