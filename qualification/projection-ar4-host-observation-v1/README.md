# Projection AR4 Host Observation

An additive, opt-in host-observation route for the existing four-forward
projection-residual AR4 path. Qualified on ASROCK (`mi350-2`) on 2026-10-04,
then integrated from the exact formatted source used by that qualification.
This checkpoint adds diagnostics, not a new GPU or performance result.

## Implementation

Seventeen source bodies (eleven replacements and six additions) preserve the
plain TF4/AR4 selectors, wire serialization, own-output recurrence, image
selection and runtime safety checks. There is no compiler, provider, KFD/driver,
kernel, HSACO or runtime ABI change.

The new parent binary is
`ferric-qwen3-finite-projection-residual-decode-host-engineering`, with exact argv:

```text
--request /absolute/request.json --allow-unauthenticated-machine-code --observe-projection-host
```

The unchanged projection-decode request schema must select autoregressive mode.
The worker uses the distinct `--engineering-native-projection-residual-decode-host-v1`
selector and a fresh `--host-sidecar` path. Parent stdout uses
`FerricFiniteProjectionResidualDecodeHostDiagnosticV1`; the sidecar uses
`FerricProjectionResidualDecodeHostObservationV1`. The parent accepts the bound
sidecar only after healthy Close, EOF and child reap. Ordinary runtime admission
and currentness checks remain enabled.

## Qualification

| Scope | Actual Result |
| --- | --- |
| Controller policy tests | 13 passed, no skips; primary SSH observation |
| Worker library | 505 passed, 4 historical ignores |
| Worker `shared_wire` | 13 passed |
| Parent library selections and binary tests | 337 passed |
| Newly executed Rust total | **855 passed, 4 historical ignores** |
| Bounded build/test phases | 61 completed naturally; all exit 0 |
| Selected production executables | 3 built and pinned |
| Postchecks | Passed; no source, dependency or old-target drift |

The 26 added Rust executions include eight report tests compiled in both crates,
five worker tests, four parent-library tests and one binary test. All sixteen old
parent binary test suites remain. Full worker and parent-library inventories were
checked for exact additions without removals. CPU1037's unrelated 208 runtime
tests are historical provenance only, not rerun or included in this total.

The controller copied the authentic CPU1037 paired source, checked every overlay
preimage, formatted only sixteen Rust bodies, and checked relocated Cargo
metadata, source/dependency identities, all old targets and selected artifacts.
Execution retained CPU8/9, nice10, hidden GPUs, 12 GiB per-leaf address space,
6 GiB aggregate fresh target, 40/38 GiB free-space floors and natural process-group
completion. No old artifact was removed by the qualifier.

The tests cover report shape/binding, interval monotonicity, CLI mode and path
refusals, parent lifecycle/report acceptance, and existing plain-route regressions.
They do not inject every I/O failure directly into the new Rust observer backend
or sidecar writer. Separate [supervisor policy tests](../projection-ar4-host-supervisor-v1/README.md)
cover synthetic sidecar and wrapper refusals, not hardware execution.

## Evidence

- [Actual completion](raw/complete.json): `7d8c08eeffab9cbebbf6abc973c0ba93d80616ad81063b35587e74d04df1d59c` (488,734 bytes).
- [Controller](controller/run.py): `12898f6b9aa1a1af122cf61d0393089fa38cdb898097114dbb67a36ea4fc780a`.
- [Controller-policy observation](raw/host-observer-cpu-policy-primary-v228-v1.json): `cf846c622b579fe3ac398eaf1aeec8ac845282f53e34f5b5ca2708bd2a260548`.
- [Source proposal](proposal/source-manifest.json): `ab5d055a09c320780a0d0da7d235bb0fe023469c9132b37a877fbc7841738074`.
- Actual formatted source map: `1d173129c684afe5bcdc009ec939f8e4f0371bad8a4edecdf62b4548cb04f920`.
- Prior CPU completion: `7a3c170ca6ffe6000517588280fcdb99cb58c0cbb22cc3f4be910232f7d51c54`.

`raw/` retains the completion, all 61 five-file phase records and the separate
policy-test primary observation. `controller/` is the executed controller and its
test source. `proposal/` preserves the authored pre-format proposal and patch;
its original draft-relative paths refer to the retained external proposal tree.
`source-overlay/` contains the seventeen **formatted, tested** bodies, identical
to the integrated adapter files. Authored-status wording in copied source
documents is preserved; actual execution evidence above supersedes that wording.

The seven large source/configuration/old-target maps and executable bodies remain
outside Git at the exact paths and hashes recorded in the completion. All 312 raw
records were downloaded for review; only those seven maps are omitted here.
The complete raw-record archive is retained with SHA-256
`9af6e17385984b3f619202d0889cc1606648d85c830cf2ea3de497182dbc9e56`.
The formatted overlay archive is retained with SHA-256
`bef9dc881af41e318239c6db10e8fb6c0bee68496aa034cbc3418917f2bb2e1a`.

| Executable | Bytes | SHA-256 |
| --- | ---: | --- |
| Observer parent | 13,938,936 | `1fad2d8aafe9ce4e799322d42d1015756ead228bf2f237c302ee260fecb656fd` |
| Plain AR4 parent | 13,917,080 | `5bfb2b39ef7b0748073fd7d15949682ca799329282afe4b1215c21a006c8ce94` |
| Worker | 5,159,512 | `14135b08c276ba9d38fcbae635fe14c3db8bd11b934ccf5ac8d51e7c2876bd75` |

## Latency Scope

The route records seven snapshots and six intervals for inclusive per-rank host
currentness, admission, prepare/publication/wait, poll and guarded-I/O counters;
shared-fence counters remain separate. These categories can overlap and must not
be summed as GPU time. Each forward timer encloses `Owner.run`, including metadata,
state I/O, validation, dispatch/wait and hidden/logit readback. Serialization timers
include assembly, hashing, Control/payload encoding and response-pipe blocking.
Close timing excludes final sidecar serialization, file sync and EOF, and does
not invent a post-consumption KFD snapshot. Sidecar publication is failure-closed.

No host-latency values are measured by this CPU qualification. It does not establish
device timing, speedup, throughput, numerical acceptance, a full CPU1037 rerun,
compiler qualification, production authority or the 2048-prompt/256-decode target.
The next execution step requires fresh MI350 executable audits and a native
four-forward run before interpreting these counters.
