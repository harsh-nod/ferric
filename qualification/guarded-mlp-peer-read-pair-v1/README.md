# Paired Hidden-State Reads

The narrow two-rank runtime read API and its opt-in Ferric worker caller have
passed CPU qualification on MI350. The API is integrated in fe2o3 commit
`ee63881af1`; the tested worker postimages are integrated here. The separate
parent selector and native comparison remain pending. There is no new native
performance result here.

## Change

The guarded model currently retains both ranks' 8,192-byte hidden outputs at
each of 36 layers. Two ordinary reads repeat currentness checks. The new
`Group::read_pair_v1` validates both rank-specific public-VRAM tokens and ranges,
makes a fresh full-group check, copies both outputs with actual idle-queue
checks, then makes another fresh full-group check before returning either copy.
Failure or unwind quarantines the Group. Typed state and private dependency
arenas cannot use this API. Ordinary reads, global currentness policy, limits,
dispatches, and buffer-reuse behavior are unchanged.

This is a method-local check cadence, not a cached currentness authorization or
an atomic cross-rank GPU snapshot. Both producers must already have completed
under the existing checked runtime contract.

## Actual CPU Results

[The MI350 receipt](runtime-cpu-v2/evidence/complete.json) records 22 naturally
completed phases, reaped children, absent process groups, and clean source,
tool, dependency, and cache postchecks.

| Check | Result |
| --- | --- |
| Full KFD ordinary test selection | 1,108 passed; 8 unchanged ignored |
| New paired-read tests, included above | 7 passed |
| Unchanged Ferric worker tests | 615 passed; 4 unchanged ignored |
| Existing selected facade doctests | 10 passed |
| Rustdoc parser regression cases | 8 passed |
| Runtime and worker test/executable products | 10 built and pinned |

The run used CPUs 8/9, two Cargo jobs, private offline caches, hidden GPU
visibility, and the existing storage/process bounds. Its 135.76 seconds is
CPU qualification wall time, not model latency or throughput. Ignored native
tests were not executed. Only the selected facade doctests were run.

The [retention map](runtime-cpu-v2/retention.json) authenticates 145 files:
all 117 raw files, receipts/source maps, the three tested runtime postimages,
helpers, lineage, and five workspace manifests. The unchanged compiled source
bodies match the canonical repositories. The qualified reduced Cargo workspace
and lock are retained explicitly; their saved original manifest and input lock
match canonical fe2o3. They are not substituted into the canonical workspace.

The terminal is 2,198,619 bytes, SHA-256
`61f255e100b486418723dc51d36dc400a66945c9412c04f8e8024eb93065014d`.

## Worker Caller

The [worker receipt](worker-cpu-v2/evidence/complete.json) records nine naturally
completed phases, 621 passing tests, four unchanged ignored native tests, and
625 inventoried names. Six new tests cover the explicit route, report schema,
paired hidden-state validation, ordinary read order, and healthy Close gate.
All source, cache, dependency and owned-process postchecks passed. Four Cargo
products were built and pinned. The runtime suite was not rerun in this worker
qualification; it uses the exact runtime sources qualified above.

The opt-in `--engineering-native-guarded-mlp-host-paired-read-v1` route uses fresh
AR4 arenas with shared-full currentness and rejects teacher-forced or reusable
arena modes before setup. The ordinary route is unchanged. Both paths require
equal, complete, finite BF16 hidden outputs from the two ranks.

The [retention map](worker-cpu-v2/retention.json) covers 71 original bodies,
including all 50 raw files, the nine tested worker postimages, lineage and
staging receipts. The terminal SHA-256 is
`1078bf7fc8a1359d8144f786dd32e270a5a736ea995e3f52b97d7e905581d863`;
the worker executable is 5,946,240 bytes with SHA-256
`4bbf99460ad3599248fabcbbc889139269a352e5801d434ba76d5f60b17bb8b3`.
Its 72.66-second qualification wall time is not model throughput.

## Measurement Gate

The [data-only analyzer](analysis-cpu-v2/analyze_hidden_reads.py) and its
[eight synthetic tests](analysis-cpu-v2/evidence/complete.json) ran on MI350.
It requires complete payload equality, identical model inputs and executables,
four independent sessions, and an authenticated serial control/paired/paired/
control run manifest. The expected per-layer read counters are four versus two
group checks and two versus zero individual currentness checks per rank.
Both modes must still read all 8,192 bytes from each rank.

The planned comparison uses fresh AR4 arenas and the same shared-full policy
in both modes. Its primary measurement is the outer hidden-read wall interval.
The runtime's nested `read_ns` scope changes, so comparing that counter alone
would overstate the benefit. The original five tests cover parsed-case analysis.
Three additional tests exercise complete command-line manifest admission and
reject incorrect pins, missing/reordered/overlapping runs, and corrupted,
missing, or symlinked payloads. All eight passed without GPU execution.

Next: qualify the separate parent selector, execute the owned serial
comparison, and retain exact outputs and
cleanup evidence. A paired-read reduction would not establish sustained
2,048/256 decoding, GPU overlap, full-model numerical acceptance, or 700 tok/s.
All issue #42 milestones remain open.
