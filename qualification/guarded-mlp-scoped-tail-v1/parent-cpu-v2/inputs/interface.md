# Closed Scoped Tail V1: Interface and Test Plan

Status: source-only proposal. No implementation, build, test, native execution,
numerical acceptance, performance result, or Full2303 launch admission is supplied
by this document. Existing qualified source and bound qualification inputs are
unchanged.

## Decision and staging

Implement an extent-independent, fixed-model runtime tail operation. First expose
it through a distinct opt-in Readiness40Position5 V4 route: existing bank rearm,
layer currentness and capacity census plus the new tail window. The control is
the unchanged Census V3 route on the same newly qualified worker and parent ELFs.
Both cases must be fresh and retain the original forty records, captures
0/5/16/39, and 124 parent timing spans.

The first two forwards use the existing ordinary tail. Only forwards 2..39 use
the new scoped tail. The existing Default, SharedFull, scoped-layer, bank V2,
Census V3, and all existing Full2303 selectors must remain unchanged.

A later Full2303 tail selector needs its own policy, implementation qualification,
independent exact 256-output comparison, and launch admission. Readiness V4 must
reject Full2303. Runtime extent independence is not worker-profile authority.

## Why tail before prologue or whole forward

The following is a static successful-path tally for a warm forward using bank,
layer, and census scopes, with the default Group policy and no operational,
kernel-cache, raw-timestamp or observer flags. It excludes setup, the first two
forwards, Close, failures and retries. P is the variable number of periodic
currentness polls in the five ordinary embedding/tail dispatches; it is not a
measured time or constant.

| Portion | Full topology discoveries |
| --- | ---: |
| Metadata zero-add preflight and four writes | 8 + 4 * 10 = 48 |
| Token write and two serial embedding dispatches | 10 + 2 * 11 + P_embedding = 32 + P_embedding |
| One scoped bank rearm | 2 |
| Thirty-six scoped census layers | 36 * 2 = 72 |
| Three serial tail dispatches and three reads | 3 * 11 + 3 * 10 + P_tail = 63 + P_tail |
| Final fence and bank commit zero-add preflights | 8 + 8 = 16 |
| Total | 233 + P_embedding + P_tail |

Derivation from the source bodies pinned below:

- device_gfx950.rs:233-245 performs one complete topology discovery per full
  currentness check, not two.
- engineering_gfx950_peer.rs:296-334 runs each of two ranks through full
  lifecycle currentness and idle currentness: four discoveries per Group fence.
- Ordinary Group write/read at engineering_gfx950_peer.rs:745-783 have two
  such Group fences plus two Context checks: ten. Context copy boundaries
  are engineering_gfx950.rs:677-725.
- Ordinary singleton dispatch at engineering_gfx950_peer.rs:850-870 has the
  two Group fences plus prepare, publication and completed-idle checks:
  eleven plus periodic polls. The Context paths are
  engineering_gfx950.rs:834,981,1083; polling retains its existing 100 ms
  currentness cadence.
- capacity_v1::preflight_snapshot has two Group fences, with the complete
  Counts snapshot and equality check between them
  (engineering_gfx950_peer_capacity_v1.rs:74-110).
- Metadata ordering is native_forward.rs:429-511. The tail dispatch/read
  ordering is tail_bindings.rs:311-330 and native_forward.rs:199-217.
- The final fence and commit stay outside the proposed scope
  (guarded_mlp_long_sequence_v2.rs:121-128 and
  state_roster/guarded_mlp_decode_v1.rs:626).

For 2,301 warm Full forwards the fixed existing total is 536,133 discoveries.
The tail contributes 144,963 of them; its proposed two-per-tail boundary would
contribute 4,602. This is static arithmetic, not a measured saving or a prediction
that Full2303 fits one hour. Polls become local checks, not omitted checks.

A prologue window could instead replace 80 + P_embedding discoveries with two,
but spans metadata admission, four metadata writes, token feedback write, two
dependent kernels and capacity-count authority. It has a larger mutation and
forward-admission boundary. A whole-forward window additionally spans all
thirty-six Prefix/pair owners and their completed/reusable proof transitions.
Neither broader scope is needed for the first tail experiment.

## Runtime facade: fixed operation, not a callback

Frozen public data and method shape (Kernel and Buffer below are the existing
Gfx950EngineeringPeerKernelV1 and Gfx950EngineeringPeerBufferV1 types):

```rust
pub struct Gfx950EngineeringPeerScopedTailInputsV1<'kernel> {
    pub final_norm_kernel: &'kernel Kernel,
    pub head_kernel: &'kernel Kernel,
    pub argmax_kernel: &'kernel Kernel,
    pub hidden: Buffer,
    pub final_norm: Buffer,
    pub empty: Buffer,
    pub normalized: Buffer,
    pub head: Buffer,
    pub logits: Buffer,
    pub choice: Buffer,
}

#[derive(Debug)]
pub struct Gfx950EngineeringPeerScopedTailObservationV1 {
    pub choice: [u8; 4],
    pub normalized: Vec<u8>,
    pub logits: Vec<u8>,
    pub host_ns: [u64; 3],
    pub currentness: Gfx950EngineeringPeerScopedCurrentnessCountsV1,
}

impl Group {
    pub unsafe fn dispatch_tail_scoped_currentness_unchecked_v1(
        &mut self,
        inputs: &Gfx950EngineeringPeerScopedTailInputsV1<'_>,
        timeout_ms: u32,
        deadline: std::time::Instant,
    ) -> std::result::Result<Gfx950EngineeringPeerScopedTailObservationV1, String>;
}
```

Inputs borrow the three existing non-Clone kernel owners and copy only opaque
buffer tokens; they neither own nor release kernel/allocation backing. The
observation has no kernel/Group lifetime. Result is the existing runtime String
error alias, not a new error framework.

The unsafe method accepts one mutable Group borrow, fixed named tail inputs,
the unchanged per-dispatch timeout_ms, and the owning worker's existing absolute
deadline as std::time::Instant. It returns only the observation. It accepts no
callback, dispatch vector, extent/profile selector, caller counter, currentness
token, Window, ownership claim, permissive mode, or ambient configuration flag.

Inputs name three kernel tokens, final_norm_kernel/head_kernel/argmax_kernel,
and seven buffer tokens, hidden/final_norm/empty/normalized/head/logits/choice.
All are retained Group-owned tokens. This is fixed Qwen geometry, independent
only of forward position and outer workload length; it is not a generic tail
executor. Private runtime construction uses the existing exact three plans,
scalar words, access modes, pointer fixups, workgroups and grids. Equivalence to
the old tail_bindings plan is a required byte-level test, not an assumed property.

The exact runtime input contract is:

| Buffer role | Owner rank | Full allocated extent |
| --- | ---: | ---: |
| Hidden final residual | 0 | 8,192 bytes |
| Final norm weights | 0 | 8,192 bytes |
| Empty placeholder | 0 | 2 bytes |
| Normalized output | 0 | 8,192 bytes |
| Transposed head | 0 | 1,244,659,712 bytes |
| Logits output | 0 | 4,861,952 bytes |
| Choice output | 0 | 64 bytes |

All seven tokens must be distinct, current, retained by this Group and of the
required actual storage/access kind. Empty is the same root in the two existing
zero-length final-norm operands; this deliberate zero-length reuse does not
permit two semantic root tokens to alias. No fresh GPU allocation, source upload,
model transpose, rearm, Prefix/pair transition, or capacity preflight occurs in
the tail window. Existing allocation ledgers and final census checks remain.

The caller's unsafe premises remain explicit: authenticated source images and
kernel identities; exact sealed semantic bindings and transposed-head type;
exclusive admitted forward; all thirty-six layer completions accepted; and
Hidden0 is this same forward's final residual. Group token/range validation is
not a substitute for these source and semantic premises. The runtime validates
the actual tokens, kernels, ranks, dimensions, pointer/access ranges and queue
state independently; no expected owner counts are supplied.

The observation contains:

- choice: [u8; 4], original little-endian device output;
- normalized: Vec<u8>, exactly 8,192 copied bytes;
- logits: Vec<u8>, exactly 303,872 copied bytes;
- host_ns: [u64; 3], existing serial dispatch host durations in
  FinalNorm/Head/Argmax order, not GPU-only time;
- currentness: the existing data-only scoped count type.

No borrowed device memory, live queue object, prepared dispatch, mutable owner,
reusable proof, Window or callback escapes. Do not return a runtime-computed
replacement argmax or normalized/logit values.

## Custody, order and deadlines

A private TailOperation guard is armed before admission. It exclusively retains
the Group borrow, fixed input tokens and private Window until successful full
exit. The Group owns the actual buffers, kernels, both Contexts and queues.
The thirty-six completed Prefix/pair owners remain with the outer worker;
the tail does not borrow, rearm, rebind or export their proofs.

Successful effect order is closed:

1. Check the inherited absolute deadline, unchanged per-dispatch timeout,
   active Group, exact inputs, flags, and both queue identities/frontiers.
2. Enter the private scoped Window with full discovery and both-rank idle
   predicates. Preserve the same incarnation/device/epoch/root/generation
   checks as the existing layer window.
3. Prepare, publish, poll and retire FinalNorm completely.
4. Prepare, publish, poll and retire Head completely.
5. Prepare, publish, poll and retire Argmax completely.
6. Read Choice[0..4], Normalized[0..8192], then Logits[0..303872],
   in that original order, with the existing read-copy bounds.
7. Perform full scoped exit and both-rank idle checks. Construct and validate
   the return observation/counts while TailOperation is still armed, without
   further device effects. Perform a final inherited-deadline check, then
   disarm and return. Any refusal/unwind after exit but before disarm still
   quarantines the runtime owners; no fallible operation follows disarm.

All three kernels run on rank 0. Rank 1 is nevertheless retained in Group entry,
exit and every original Group-fence boundary. Each dependent dispatch must see
the prior AQL completion and exact queue-frontier/signal retirement; the three
dispatches must not be combined into a concurrently published round.

Every original Group-before/after and Context prepare/publication/poll/idle/read
boundary stays present, routed through private Currentness and rank-local
checks. Preserve queue predicates, pointer fixups, image admission, signal
checks, exception checks and periodic polling. RankCurrentness(true) must still
refuse the scoped route. Operational currentness, kernel-admission cache,
raw timestamp mode and host observers remain refused. Do not configure the
Group's default policy or remove checks by calling unchecked inner methods.

Use the actual worker absolute deadline, never a newly computed one-hour
deadline. Each of the three dispatches keeps its existing timeout_ms budget;
do not silently replace their three budgets with one per-tail budget or multiply
the outer deadline. Internal scoped checks can detect expiry earlier, but cannot
extend any inherited budget. No timeout or cleanup limit is raised.

The new facade admits only the existing active-forward timeout range
1..=10,000 ms. The old ordinary tail's separate API remains untouched. Any
private Window documentation currently naming only LayerOperation must be
updated to name this second closed owner precisely, not relaxed to arbitrary
callers or a longer-lived public scope.

Any runtime refusal or unwind before successful full exit quarantines Group,
both Contexts and affected device scope, including failures before a Window is
fully constructed. Drop performs only infallible poisoning; it must not launch,
reset, allocate, retry or free potentially active GPU work. No observation is
returned on failure.

## Worker validation and capture placement

The worker keeps the complete TailBindings admission, including unused
embedding/copy roots and kernel roles, before selecting the new facade. The
new runtime call does not weaken that existing catalog contract.

After successful runtime exit, the worker installs the returned bytes and uses
the existing choice extent/range check, finite_bf16 and checked_argmax once.
The existing checked_argmax scans all 151,936 BF16 logits and keeps the first
index on ties. Do not introduce another numerical implementation in the runtime,
reread the buffers, fit a tolerance, or treat currentness evidence as numerical
acceptance. See native_forward.rs:146-217.

Runtime success followed by malformed bytes, nonfinite values, argmax mismatch,
counter mismatch, expiry, frame validation or publication failure is still
fatal: the existing Sequence Attempt cancels the transcript, poisons worker
sequence/roster/catalog, and permits no retry or healthy Close. This is a
post-runtime worker rejection, not a claim that a completed runtime window was
retroactively poisoned.

Only after those worker checks may Driver::tail append the already accepted
thirty-six 8,192-byte hidden rows, then the exact returned normalized and logits
bytes. Completion, frame hash, final idle/capacity fence, bank commit, transcript
advance and publication retain their existing order. The guard at
guarded_mlp_long_sequence_v2.rs:36-50 stays armed until all of that succeeds.

Readiness captures remain 0/5/16/39. Full captures, if later authorized through a
separate selector, remain 0/2047/2048/2302. Position 0 therefore remains on the
ordinary tail in both workloads. No capture is substituted with a reference
tensor, synthetic token, later reread, or selected-only execution.

## Readiness V4 identity and counts

Proposed worker pure module:
finite_guarded_mlp_readiness_bank_scoped_census_tail_v4

Proposed explicit worker flag:
--engineering-native-guarded-mlp-readiness40-position5-bank-scoped-census-tail-v4

Proposed record schema:
FerricReadiness40Position5BankScopedWarmCensusTailPolicyV4

Proposed execution profile:
Readiness40Position5BankScopedWarmCensusTailCurrentnessV4

The record remains an exact bounded canonical JSON line on original child stderr,
published only after real forty-forward Close and original executable/deadline
checks. It retains V3 session/devices/registration/transcript/worker identity,
empty generated_tokens, capture positions, all unchanged false authority flags
and the existing new/validate/decode argument shape. No parser may strip the
policy bytes to satisfy an ordinary empty-stderr validator.

Counts retain the existing layers, banks and census fields and V3 validation,
adding tails as an independent scope, not a layer or census subset. TailCounts
records ordinary_tails=2, scoped_tails=38, dispatches=114, readbacks=114,
readback_bytes=11,858,584, and observed aggregate full_discoveries,
local_checkpoints, before_calls, after_calls and generation_probes.

readback_bytes counts all returned copies, not selected-capture or capsule
retention bytes. The state must establish these counts from genuine completed calls in positions
2..39 and checked arithmetic, not fill constants only at Close. Full discoveries
must total 76. The source audit now closes the stronger mandatory census below,
replacing the earlier general participant envelope in reviewed interface
067c0162058c7010c3fefa55703dff763ca6ef72762b5258c7d6c7fa62a038c0.
For each successful tail let P be the number of periodic currentness checkpoints
actually taken by noncompleted polls, not the number of raw polling iterations.
The closed route has exactly 12 two-rank Group checkpoints and 15 mandatory
rank-local checkpoints:

- NativeTail::prepare and ::retired preserve the before/after Group idle fences
  for each of the three serial dispatches: six Group checkpoints.
- Group::read_currentness preserves before/after Group idle fences for each of
  the three original readbacks: six more Group checkpoints.
- Context::prepare_dispatch_with_peer_bindings_currentness, publication, and
  completed polling each perform one rank-local idle/check per dispatch: nine.
- Context::read_currentness performs one rank idle before copying and one rank
  check after copying per readback: six more.
- A noncompleted poll adds a rank checkpoint only when its original periodic
  currentness deadline is reached. The original timeout, 100 ms cadence and
  completed-frontier checks are unchanged; P may be zero and is not fixed.

Thus L=27+P. The device Window full entry and full exit contribute two full
discoveries, four participant before calls, four participant after calls, and
three generation probes. Every Group checkpoint contributes two before and
two after calls; a rank checkpoint contributes one of each; every checkpoint
contributes two generation probes. A successful result must therefore satisfy
full=2, L>=27, before=after=L+16 and probes=2*L+3. For n=38 completed scoped tails,
the worker requires full=76, L>=27*n=1026, before=after=L+16*n=L+608 and
probes=2*L+3*n=2*L+114, all with checked arithmetic. These are structural
relations, not fixed poll counts, measured work savings or temporal equivalence.

The concrete anchors are NativeTail::{prepare,publish,poll,retired,read} in the
new engineering_gfx950_peer_scoped_tail_v1.rs; existing Group::read_currentness
in engineering_gfx950_peer.rs; Context::{read_currentness,
prepare_dispatch_with_peer_bindings_currentness,
publish_prepared_dispatch_with_currentness,poll_pending_dispatch_with_currentness}
in engineering_gfx950.rs; and device Window::{enter,checkpoint_selected,finish}
in device_gfx950_scoped_currentness_v1.rs. Focused tests must check zero and
additional periodic checkpoints by accumulating the closed-route events and
bind the actual production callsites, not only accept fabricated Counts.
The runtime source manifest pins those source/test bodies. Tail counters remain
independent of V3 layer totals and are never subtracted as census checkpoints.

Add explicit scoped_tail, full_entry_exit_per_scoped_tail and
scope_includes_tail_dispatch_and_readback facts. Keep temporal_equivalent_to_full,
numerical_acceptance, performance_claim, production_authority and
full_long_workload false. Preserve the 4,096-byte stderr cap and test worst-width
serialization; do not enlarge it to accommodate a proposed schema.

The parent is a new timed-only route. Proposed flag:
--observe-guarded-readiness40-position5-bank-scoped-census-tail-host-timing-v4

Proposed wrapper:
FerricReadiness40Position5BankScopedWarmCensusTailTimedObservationV4

It must reread and authenticate the original policy FilePin before ordinary
publication and again for the timed wrapper, preserving the existing 124-span
clock boundaries and output/retention caps. Ordinary request, wire profile,
Bootstrap and Closed do not change. A matched same-ELF Census V3 versus Tail V4
data adapter must independently admit both original policies, both timelines,
all forty records, all four payloads, fresh sessions/processes and healthy
retirement before reporting parent-wall comparisons. No speedup is assumed.

## Later Full policy, not an implicit extension

The runtime facade has no position limit, but only a future distinct Full owner
may call it after separate authorization. Keep the currently qualified
Full2303BankScopedCensusCurrentnessV1 unchanged.

A future Full tail policy must derive 2 ordinary and 2,301 scoped tails,
6,903 completed tail dispatches and 6,903 readbacks, while preserving compact
Full bank/layer/census counters, 2,303 actual forwards, 2,048 prompt inputs,
255 own-output feedback inputs, 256 actual generated IDs, 144 pages, and the
four exact Full captures. It must join the parent's independently reconstructed
256 IDs and original policy bytes before publication. Do not reuse Readiness's
forty-position policy or create an output-vector authority in the runtime.

The independent full reference and exact generated-ID/raw-decoded-byte gate stay
unchanged. A teacher-forced p5 argmax comparison, a passed short pair, static
discovery savings, or a CPU suite does not satisfy Full numerical or one-hour
launch admission. No copied reference feedback, tie waiver, deadline increase
or silent fallback is permitted.

## Required tests and review partitions

Runtime tests, using the existing private fault-engine pattern:

- Exact admission, full entry, serial prepare/publish/poll/retire ordering,
  three original-order reads, full exit and return; no dependent early publish.
- Byte-exact three plan/geometry/scalar/fixup equivalence to the existing worker
  tail plans; all named role extents/ranks/access/alias refusals.
- Fault and unwind injection at every admission, entry, prepare, publication,
  poll, completion, read and exit boundary. No later effect or observation;
  guard poisoning includes Group, both Contexts and device scope.
- Stale incarnation, root/generation, participant identity, queue epoch,
  frontier/signal and exception drift; deadline expiry before and after work.
- Operational/cache/raw/observer refusal and unchanged lifecycle=true refusal;
  old public dispatch/read paths retain their prior full-currentness behavior.
- Exact copied byte extents and observed scoped counters, including checked
  overflow and variable periodic-poll counts. No caller-defined counters.

Worker and policy tests:

- Explicit V4-only routing: positions 0/1 ordinary, 2..39 scoped; wrong extent,
  missing opt-in, causal/raw combinations, stale/extra flags and Full refuse.
- Genuine returned bytes pass unchanged finite/argmax logic, including first-ID
  ties; wrong token, NaN/Inf, wrong extent and original byte drift refuse.
- A valid completed runtime result followed by worker validation/counter failure
  cancels the actual outer sequence; no retry, next forward or healthy Close.
- Bank/layer/census counters retain V3 validation; independent tail counters,
  checked accumulation, exact Close counts, canonical framing and 4,096-byte
  worst-width bound. No bool-as-integer or extra-field acceptance in Python.
- Old selectors, first-use tests, argument guards, exact old named test outcomes
  and numerical source bodies remain unchanged; real CLI EOF/refusal coverage
  uses the existing bounded harness, not a new supervisor.
- Parent retained-policy reread and prepublication/timing-wrapper checks,
  exact 124-span boundaries and failed publication behavior.
- Pure matched-admission tests refuse cross-mode policies, altered unselected
  records, altered captures, different CPU/ELF identities and stale sessions.

Only after source peers close: root-owned CPU qualification on the exact final
sources, then one fresh same-ELF V3/V4 Readiness pair with unchanged controls and
caps. Report only admitted parent-wall spans; no GPU-overlap or token-rate claim.
Full activation is a subsequent decision. Do not add new transport/supervisor
frameworks to implement this contract.

## Source readset

F = /home/harsh/ferric-p227-integration
RT = /home/harsh/fe2o3-p228-runtime

Paths below are relative to F/adapters/tp-peer-finite-engineering-worker-v1/src
or RT/crates/fe2o3-kfd/src. These pins identify the source read for this contract,
not a claim that this proposal or any future implementation has been executed.

| Tree / file | SHA256 |
| --- | --- |
| F tail_bindings.rs | 7d28a3444db92354002f1bd7024ec129f4f44fe1458b78a001e9cbfaa188447c |
| F tail_artifacts.rs | fd3eba21656a0178262a1a17a8b70e176d084081b94cbc29e07dbbf84dcece5a |
| F native_forward.rs | 4b06cfb89b3167dd645764d10f52d458bf7a64bd9aae5eebc320f773b0e650ab |
| F native_catalog.rs | f20ddb1310e25fe2dec4ebca4019312c24902c5677cace1cf6decee1a39fe5d7 |
| F native_guarded_mlp_readiness_v1.rs | 82459ad961d2958c677c95f72b369062ea91e91b34a71ea0a4737f5e4734881b |
| F guarded_mlp_long_sequence_v2.rs | eba088151e6cd5aaa684be44652db93acb578f2a7272c95263d257f1e4de1e44 |
| F state_roster/guarded_mlp_decode_v1.rs | 726047f228f5bf3ca5ddcad2273b5c5c8a3c7621e3faa7680feb7f69f544cf35 |
| F finite_guarded_mlp_readiness_bank_scoped_census_v3.rs | c7c330d53a2fdcfbf13a7688de84a4d03ec006e7b2deb432eee5323d081eb8c7 |
| RT device_gfx950.rs | 57e32f62c32a5813b8e480ca5309af84f30cafab960166eb668d6bc80b8305e6 |
| RT engineering_gfx950.rs | 9da1deae3af661d6729d5decef2ba53cfb81e404e6a868b2b00de2689ad83321 |
| RT engineering_gfx950_peer.rs | 247ded4baa6599127618c5534b4a24a8962a711620373def39755b5ed78cb038 |
| RT engineering_gfx950_peer_capacity_v1.rs | ed49f9ee9fcb6af835a49497dd2d5bd042acbc39819fff066772305bf0ccebfd |
| RT engineering_gfx950_peer_scoped_currentness_v1.rs | 20e25f6f199aee06d74fa29f2880ba2373585852cb3924c9af327a4973f16f1a |
| RT engineering_gfx950_peer_scoped_layer_v1.rs | a879e7148ba13743fa005c923eeff9858018af54fd4a33c934a1ee7fa92a3e92 |
| RT engineering_gfx950_peer_scoped_census_layer_v1.rs | c511297726e0fd16f9d2d3db4c1dac663a2f9e41593a5bf9258b61c7cb23040b |
