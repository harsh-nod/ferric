# Native Worker Raw-Tick Route

This checkpoint qualifies the worker-side four-forward diagnostic route. It is
not a GPU timing result, a parent-runtime qualification, a numerical comparison,
or evidence of 700 tokens/s.

## Implementation

The new worker selector routes prefix and MLP through typed raw-timestamp APIs,
both residual pairs through generic paired raw dispatch, and each embedding,
copy, final-norm, head and argmax operation through a separate raw singleton.
Ordinary selectors and legacy host-observation schemas remain separate.

The private recorder expects exactly 293 packets per forward, 1,172 across four
forwards, with 592 rank-0 and 580 rank-1 packets. Records bind selected image/entry,
forward/layer/rank, group and queue identity, packet/signal generation and raw
completion-signal ticks. Actual host intervals join the ordinary wire Control
and its Completion hash. Only successful consuming Owner Close can produce a
closed report; publication follows the successful Closed response write.

All existing state finish checks, paired scratch-consumer barriers, source
sealing, failure poisoning, layer readback, finite checks and tail argmax checks
remain. Reports are bounded to 2 MiB. No queue is toggled into raw mode after
allocation, and no missing observation falls back to a host-only timestamp.

Tail provenance correction: in this actual setup path,
`ReviewedTailImages::new(parts[6], parts[5])` consumes the original tail and
residual code objects. The tail digest therefore joins
`bootstrap.begin.tail_image.sha256`, and copy joins
`bootstrap.begin.residual_image.sha256`. The previous proposal's description of
this field as an encoded tail bundle was incorrect; the integrated validator
uses these direct joins.

## Qualification Evidence

The first fresh CPU build compiled, but two new protocol tests failed: their
synthetic backend returned all-zero logits while declaring token 100 the
winner. The production argmax guard correctly rejected that inconsistency.
The successor corrects only those test fixtures to use a one-hot winner and
strengthens the error assertions; it does not change production runtime code.
The failed attempt remains separate evidence and is not counted as a pass.

This publisher requires the frozen CPU-v2 package
`dbe0c89910178f1c83039a320a629351428c360df21ea269e2541b4f62b29c3b`
and the corresponding pure-wrapper-v2 controller. Its source-v3 overlay must
match the actual fresh build and live source; earlier package results cannot
satisfy these gates.

Publication is allowed only after the retained evidence passes `publish.py`:

- Actual CPU receipt: 609 passed, 4 existing ignored tests; 25 natural-zero phases
  with owned groups absent, 128 raw phase/source records and one worker artifact.
- Actual policy-controller receipt: 13 passed, no skipped/failed/error tests,
  unchanged frozen four-file package and retained unittest transcript.
- Runtime inventory stays unchanged; all prior 413 worker tests plus 31 named
  additions remain present. Worker outcomes are 440 passed and 4 ignored;
  selected runtime outcomes are 169 passed.
- Exact source archives at Ferric `82b0fe5850f38c3ff8d2cbe3640a9880d722467e`
  and fe2o3 `9a321f3f98e597a75e8ebeafdda169ec10e12e9e`, extended by the
  authenticated 18-file worker overlay, match the actual source maps.
- All 18 compiled overlay files match the live published worker source. The
  retained ELF matches the actual Cargo compiler-artifact record.

The `result.json` receipt supplies actual measured test counts, artifact hashes
and source pins after these checks. This README alone is not a passing receipt.
The verifier rehashes and replays retained data; it does not rerun CPU tests,
import test controllers, rebuild code or execute GPU work.

Local replay covers the new raw/package/source/archive/artifact records and the
prior command templates used for their join. It does not reopen every one of
the 161 historical/helper input bodies or the 29 build-host registry manifests.
Their original postchecks remain claims of the pinned CPU controller receipt;
the public result explicitly marks those two local rehash scopes false.

`cpu/` retains the CPU completion plus 128 raw records; `pure/` retains its
completion, two source maps, transcript and actual controller. `controller/`
retains the frozen CPU controller package, and `source-inputs.json` identifies
the two original source archives. Source files remain in the worker instead of
being duplicated here. Archives, worker binaries and large runtime source
copies are not added to Git by this publisher.

The separate parent parser/driver integration is not included in this
qualification. Actual GPU routing, all 1,172 signal records, four complete
payloads and 152 tensor comparisons, six device audits, natural Close/reap and
source/ISA/runtime reviews remain required. Raw ticks have no calibrated
nanosecond or cross-device overlap interpretation yet.

All issue #42 M0-M7 milestones remain open. Full-model independent numerical
validation and sustained single-request Qwen3-8B BF16 target-only decoding at
2,048 prompt / 256 generated tokens, including the 700 tokens/s target, remain
unmet. This checkpoint changes neither `main` nor the tutorial website.

## Replaying Publication

`publish.py` requires explicitly supplied actual CPU and pure completion
SHA-256 values; it has no default success pins. Pass `--cpu` and `--pure` retained
directories, `--cpu-sha`, `--pure-sha`, authenticated prior CPU578 directory via
`--prior`, retained worker ELF via `--worker`, the live checkout via `--repo`,
frozen four-file CPU package via `--package`, original source manifest via
`--source-inputs`, local source archive directory via `--archives`, actual pure
runner via `--pure-controller`, and a fresh `--publish-to` directory directly
under the checkout's `qualification/`. Existing destinations are refused.

Run only after every CPU/retention process has completed. On refusal, do not
commit partial output as a qualification result. There is no data repair,
automatic source mutation, compatibility fallback or implicit GPU admission.
