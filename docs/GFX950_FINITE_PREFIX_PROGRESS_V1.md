# Finite Qwen3 Prefix Progress

This is an engineering checkpoint for [issue #42](https://github.com/harsh-nod/ferric/issues/42),
updated on 2026-10-06 UTC. It is not a production admission, a sustained decode
benchmark, or a claim that the 700 tokens/s target has been reached. All issue #42 M0-M7
milestones remain open. The checkpoint is being published incrementally on an
engineering branch; it does not change the production execution path.

Latest: [the updated engineering driver builds and passes its 18 focused tests on MI350](../qualification/guarded-mlp-driver-cpu-v1/README.md).
All seven phases exited naturally and were reaped, with unchanged sources and
clean postchecks. The complete 402-name compiled inventory and five ignored
names are retained, but the full driver suite was not executed. The final
driver now supports explicit optimized MIR inlining, matching the passing
atomic extraction control. A fresh loader audit and guarded lowering retry
are next; no GPU, model-correctness or performance milestone is closed.

Previously, [the refreshed guarded gfx950 lowering reaches an atomic-ordering rejection](../qualification/guarded-mlp-lowering-attempt-v2/README.md).
The previous provider-source mismatch is no longer the observed failure.
Source collection now rejects a core atomic load without a concrete ordering
argument. The compiler exited naturally after 117.259 seconds and was reaped;
integrity postchecks passed, with no timeout, forced cleanup or HSACO.
Seven synthetic controller tests passed separately. The expected alias gate
was not reached. The next attempt uses the qualified newer driver's explicit
normalization mode; atomic admission rules remain unchanged.

Previously, [the final runtime compiler binaries passed all fourteen loader checks](../qualification/guarded-mlp-s-rpo-tool-audit-v1/README.md).
The extractor resolves the exact new backend; the other five deployed tools
are unchanged. All inspection leaves exited naturally and were reaped, with
clean postchecks. Ten synthetic loader-admission tests passed separately.
The [guarded candidate dependency refresh also passes its 27 CPU tests](../qualification/guarded-mlp-dependency-refresh-v1/README.md).
Its two dependency pins now select published fe2o3 `5a500d63`, with unchanged
Rust kernels and guard protocol. These checks do not close guarded gfx950
lowering/GPU, independent model numerics or performance. All milestones remain
open. Fresh offline vendoring also passed for 145 packages with unchanged
sources and lockfiles; the new guarded gfx950 lowering attempt is next.

Previously, [the full combined runtime compiler qualification passed on MI350](../qualification/guarded-mlp-s-rpo-qualification-v1/README.md).
All 32 phases exited naturally and were reaped, with clean integrity checks.
The full compiler library passed 1,239 tests and pliron passed 1,504; 43
focused repeats and twelve extraction/rejection controls bring the total to
2,798 passing executions, not unique tests. All 25 historical ignores remain
explicit. Eleven synthetic controller tests passed separately. Final build
products and the exact derived source lineage are retained. Loader inspection,
guarded gfx950 HSACO/GPU execution, model numerics and performance remain
open; this is not production admission or a 700 tokens/s result.

Previously, [445 scoped compiler tests passed, including attention extraction](../qualification/guarded-mlp-core-kernel-error-identity-qualification-v1/README.md).
The exact core `KernelError` identity check and six new tests passed on MI350,
including eight identity/signature refusals and 38 MIR mutations. Every
earlier 438 named passing test remains green, and the unchanged attention
control now reaches gfx942 LLVM. All 39 phases returned zero naturally and
were reaped, with unchanged source/dependency maps and clean postchecks.
This completes the scoped control run, not the full runtime compiler/pliron
qualification. No production dependency, gfx950 HSACO, GPU, numerical or
performance gate has changed. Full compiler qualification and loader checks
are next; all issue #42 milestones remain open.

The [sixteen S/RPO harness helper tests also pass on MI350](../qualification/guarded-mlp-s-producer-helper-tests-v1/README.md).
They validate exact full-suite outcomes/ignored identities and final backend
rlib selection. One bounded child exited naturally and was reaped, with
unchanged inputs and clean postchecks. These synthetic harness tests are
separate from the 445 compiler tests and do not qualify the full S/RPO build.

Previously, [438 tests passed, including all checked-integer and Option wrappers](../qualification/guarded-mlp-core-checked-attention-qualification-v1/README.md).
Both builds pass on MI350. The fourth attempt corrects the observed FnOnce
kind and exact direct closure resolution. All six integer and ten Option
tests pass, including 458 and 198 MIR mutations respectively, plus two actual
closure resolutions and twelve new refusal assertions. All earlier 422
passing controls remain green, including eight atomic controls, both
unsafe-source checks and matrix extraction. Attention now reaches an
independent rejection of core's identity `From<KernelError>` conversion;
it still does not pass. All 38 phases exited naturally and were reaped, with
unchanged sources and clean postchecks. All four attempts are retained.
Full compiler qualification remains incomplete; no production dependency,
gfx950 GPU or performance gate has changed.

The [remaining `KernelError` conversion now has a complete diagnostic capture](../qualification/guarded-mlp-core-kernel-error-identity-diagnostic-v1/README.md).
An isolated compiler on MI350 passed both builds and six renderer tests,
then preserved the original attention rejection. The captured core body has
two normalized `KernelError` locals and one move followed by return. All 19
phases exited naturally and were reaped; source/dependency postchecks were
clean. This is evidence for a narrow follow-up check, not a new admission
rule, additional production qualification or GPU result.

Previously, [ten checked-arithmetic and Option helper bodies were captured from attention](../qualification/guarded-mlp-core-checked-attention-diagnostic-v1/README.md).
The second isolated diagnostic compiler built on MI350 and all six renderer
tests passed. It adds the standalone `u32::overflowing_mul` body to the nine
previously captured helpers. The unchanged attention control still rejects
`usize::checked_add`; the capture grants no admission. All ten selected bodies
were fully rendered within one 64 KiB limit, with no partial bodies or omissions.
All 19 phases exited naturally and were reaped, with unchanged sources and
clean integrity postchecks. Production recognizers and their qualification
remain pending. Both attempts and their unmodified raw records are retained.

Previously, [422 tests and matrix extraction passed; attention reached a checked-add refusal](../qualification/guarded-mlp-core-u32-widening-qualification-v1/README.md).
The exact widening recognizer and six new tests built and passed on MI350,
including 49 real-MIR mutations and eight identity/signature refusals. All
415 previously passing controls remained green. The unchanged matrix control
now reaches gfx942 LLVM and checks MFMA and workgroup storage; this does not
authorize an artifact or GPU launch. Attention still rejects the genuine core
`usize::checked_add` helper. All 36 phases exited naturally and were reaped,
with unchanged sources and clean integrity postchecks. The original run
completed through a transient SSH disconnect; it was not restarted.
Full qualification remains incomplete; Ferric's production dependency is unchanged.

Previously, [the rejected core u32-to-u64 conversion's complete MIR was captured](../qualification/guarded-mlp-core-u32-widening-diagnostic-v1/README.md).
A separate diagnostic compiler built on MI350, and all six bounded-renderer
tests passed. The unchanged matrix control retained its original rejection;
the observed helper contains one integer widening cast followed by return,
with no calls, cleanup blocks or inline origins. All 19 phases exited naturally
and were reaped, with unchanged sources and clean integrity postchecks.
The subsequent recognizer/test run is linked above. The capture itself does
not admit the conversion or qualify any provider, GPU, model or performance gate.

Previously, [415 compiler tests passed before the core conversion refusal](../qualification/guarded-mlp-core-result-qualification-v1/README.md).
Both compiler and test binaries built on MI350. V7 supplies genuine AMD-target
`core` and `compiler_builtins` metadata without changing the production matcher.
All 15 Result tests passed, including 172 MIR mutations, alongside the previous
390 tests, eight atomic controls and two unsafe/inlined-origin rejection tests.
The unchanged matrix control now gets past the Result wrappers and rejects
the concrete core `From<u32> for u64` conversion in `accessed_extent`.
That helper's body is now captured above; there is no general `From`
admission. All 34 phases exited naturally and were reaped, with unchanged
sources and clean integrity postchecks. Attention was not reached.
Ferric's dependency remains unchanged; no provider, GPU, model or performance
gate is claimed passed.

Previously, [the actual core FromResidual wrapper was captured alongside branch](../qualification/guarded-mlp-core-result-residual-diagnostic-v1/README.md).
The second diagnostic compiler built on MI350 and all ten rendering/census
tests passed. The complete observation includes the residual discriminant
assumption and its real error-conversion call; neither is implicitly trusted.
All 19 phases exited naturally and were reaped, with clean integrity postchecks.
The unchanged matrix test still returned 101 at its original rejection.
Exact body matchers and mutation tests are in progress; no provider, GPU,
model or performance gate is claimed passed.

Previously, [the rejected core Result helper's actual MIR was captured](../qualification/guarded-mlp-core-result-diagnostic-v1/README.md).
The diagnostic compiler built on MI350, six rendering tests passed, and the
unchanged matrix test retained its original rejection. The complete bounded
observation includes five blocks, six locals and three source scopes, with no
calls or inlined scopes. All 19 phases exited naturally and were reaped, with
clean integrity postchecks. This enables an exact helper-body check; it grants
no admission, provider qualification, GPU result or performance claim.

Previously, [the matched baseline confirms that the matrix refusal predates provider alignment](../qualification/guarded-mlp-provider-baseline-v1/README.md).
The published previous revision failed at the same core `Result::branch`
source-safety check, using matching build selections, tools and environments.
All 18 baseline phases exited naturally and were reaped; integrity postchecks
passed. This identifies a preexisting compiler gap, not a passing pipeline
control. The provider remains unqualified, its proposed revision unpublished,
and Ferric's dependency unchanged. No new GPU or performance result is claimed.

Previously, [the aligned provider generation passed 398 CPU tests but remains unqualified](../qualification/guarded-mlp-provider-generation-v1/README.md).
The full device library, both UI targets, trusted-provider cohort, scalar-pipeline
controls and eight selected gfx950 atomic extraction tests passed on MI350.
The existing matrix-pipeline test then failed while authenticating a core
`Result::branch` helper; the attention control was not reached. Integrity
postchecks passed. Thirteen harness fixtures passed separately, and all earlier
failed attempts are retained. The proposed dependency remains unpublished;
Ferric's kernel dependency is unchanged. No new HSACO, GPU or performance
result is claimed, and all model milestones remain open.

Previously, [the first guarded MLP lowering attempt reached device extraction but was refused](../qualification/guarded-mlp-lowering-attempt-v1/README.md).
The transferred extractor's reviewed device-source closure does not match the
candidate's pinned dependency generation. The compiler exited naturally after
116.347 seconds, with clean integrity postchecks, no timeout and no emitted
HSACO. Its trust check remains intact. A coherent published compiler/device
generation and fresh qualification are next; no new GPU or performance result
is claimed.

Previously, [locked offline dependencies are prepared for guarded MLP lowering](../qualification/guarded-mlp-dependency-preparation-v1/README.md).
Vendoring passed for 145 packages with unchanged kernel sources and lockfiles.
A separate fetch command downloaded missing crates but failed its preservation
check after older extracted cache packages disappeared. All 133 affected
packages were restored from unchanged archives with exact original content
hashes and no overwrites; that failed attempt remains retained as failed.
Four cache fixtures and eight restoration fixtures passed on MI350. This is
preparation only, not compiler emission, GPU execution or model acceptance.

Previously, [all seven compiler tools passed their MI350 loader audit](../qualification/guarded-mlp-compiler-tools-v1/README.md).
All fourteen inspection commands completed naturally with clean integrity
postchecks. The earlier loader-path failure is retained, and about 1.05 GB of
owned failed-build caches was reclaimed without deleting sources or evidence.
This establishes deployment and library resolution only. Locked dependency
preparation and actual checked gfx950 lowering remain next; no new HSACO or
GPU result is claimed.

Previously, [the full-state validator and guarded final residual passed CPU qualification on MI350](../qualification/guarded-mlp-segment-cpu-v1/README.md).
All 27 kernel tests and seven harness regression tests passed. Ten bounded
phases completed naturally, including the non-test typed Marker roster and
host library build, with clean integrity postchecks. Both earlier harness
failures are retained. The CPU crate binding is explicitly a host-only fixture,
not compiler authority. These new kernels are not yet connected to inference;
checked gfx950 emission, GPU guard controls, reusable arena validation and the
matched full-worker comparison remain next. All model and performance gates remain open.

Previously, [the finite two-rank peer-dependency graph passed native GPU qualification on MI350](../qualification/peer-dependency-signal-completion-gpu-v1/README.md).
The V3 runtime passed 1,064 Rust tests with three artifact-dependent ignores;
45 controller tests then passed before one native attempt on GPUs 1 and 2.
All sixteen signals completed, all ten guarded buffers matched, the delayed
peer-publication witness passed, and queue-first Close completed. Actual
completion-sample counters were (write, read) = (8, 3) on both queues: completed
work does not fabricate ring capacity. The 94.083 ms aggregate host interval is
not GPU time or model throughput. A previous busy-GPU preflight refusal is retained.
Reusable arena reset, full MLP-state validators, guarded final residuals and a
matched full-worker comparison are next. All milestones, numerical acceptance,
sustained 2,048/256 and 700 tokens/s remain open.

Previously, [ordered peer dependency packets passed CPU qualification on MI350](../qualification/peer-dependency-aql-cpu-v1/README.md).
All 43 Rust tests passed, including ten new dependency/publication tests. A C
oracle matched the 64-byte packet encoding and system-scoped `0x1503` header
against the installed ROCm header. All seven bounded phases exited naturally.
The exact tested Rust/C sources are pushed to both fe2o3 forks; the existing
publication gate remains unchanged. This is a packet-layer prerequisite, not
GPU peer synchronization or model qualification. Native signal ownership,
mixed-packet submission and a two-rank producer/consumer test are next. All
milestones, numerical acceptance, sustained 2,048/256 and 700 tokens/s remain open.

Previously, [the shared-full / ordered-segment GPU comparison completed on MI350](../qualification/projection-ordered-segment-pair-v1/README.md).
Both one-attempt four-forward runs passed structural and lifecycle checks;
all four payloads, covering 152 captured tensor slices per arm, are byte-identical.
The four-forward host-time sum was 21.908947 seconds shared versus 21.000850
seconds ordered, a ratio of 1.043 and 4.145% less observed wall time. Plots,
tables and all 144 combined-segment timing rows are published. This single
fixed-order pair is subject to cache warming and cold ordered arena costs;
it is not GPU timing, independent numerical acceptance or sustained 2,048/256
throughput. All milestones and the 700 tokens/s target remain open.

Previously, [both ordered-comparison parents and the common worker passed fresh MI350 runtime audits](../qualification/projection-ordered-segment-runtime-v1/README.md).
All six bounded readelf/ldd commands exited naturally, with complete library
closures and unchanged topology. Both matched input assemblies completed,
each checking 1,847 recorded identities. They retain the same model, images,
prompt, worker and lossless device IDs while selecting distinct shared-full
and ordered parents. This is preparation, not a completed native comparison,
independent numerical acceptance or sustained throughput result. All milestones
and the 700 tokens/s target remain open.

Previously, [the ordered comparison harness passed 174 policy tests on MI350](../qualification/projection-ordered-segment-gpu-preparation-v1/README.md).
Sixteen deployment-selector and thirteen input-assembler tests also passed,
with unchanged sources. The preceding V2 missing-artifact error and its fix
are retained explicitly. These are synthetic CPU tests, not a new native
GPU result. Fresh runtime audits, reviewed inputs and the matched shared-full /
ordered comparison remain separate gates. All milestones and the performance
target remain open.

Previously, [the projection-residual / MLP ordered segment passed CPU qualification on ASROCK](../qualification/projection-ordered-segment-cpu-v1/README.md).
All 75 phases passed, including the corrected default-feature check: 1,848
Rust test executions passed with seven historical ignores, five executables
were built, and all integrity postchecks passed. Seventeen controller policy
tests passed separately. The exact 36 Ferric and 24 fe2o3 source bodies are
integrated. The opt-in path combines two dispatch rounds and uses distinct
per-rank Down-output scratch to avoid a cross-rank alias race, without changing
kernel images or arithmetic. This is CPU qualification only; fresh MI350
runtime audits and matched GPU comparison remain separate execution gates.
All milestones, independent numerical acceptance, sustained 2,048/256 and
700 tokens/s remain open.

Previously, [the matched default/shared-full GPU pair completed on MI350](../qualification/projection-ar4-shared-host-pair-v1/README.md).
Both one-attempt AR4 runs passed structural and lifecycle checks. All four
606,976-byte payloads are identical across routes, covering 152 captured tensor
slices. Forward host wall time changed from 10.56-12.49 seconds to 5.25-5.77
seconds; the ratio of four-forward sums is 2.092. Plots and tables include
rank/group/publication counters and separate setup, configuration and Close.
This is one fixed-order diagnostic, subject to warmed-cache/order effects,
not GPU timing, independent numerical acceptance or sustained throughput.
All milestones and the 700 tokens/s target remain open.

Previously, [both observer parents and their shared worker passed fresh MI350 runtime audits](../qualification/projection-ar4-shared-host-runtime-v1/README.md).
All six readelf/ldd commands exited naturally, with complete library closures and
unchanged topology. Both input assemblies completed with exact same-model,
same-image requests and lossless device identities. Preparation corrections are
documented without weakening the checks. This checkpoint does not establish a
completed paired GPU run, numerical acceptance or a speedup.

Previously, [the shared-full comparison harness passed 122 policy tests on MI350](../qualification/projection-ar4-shared-host-gpu-preparation-v1/README.md).
Fourteen deployment-selector and eleven input-assembler tests also passed.
The paired supervisor checks same-build inputs, honest payload-byte comparison,
separate configuration timing and rank/group/publication currentness accounting.
This is CPU-only harness qualification, not a completed GPU comparison, numerical
acceptance or performance result. All milestones and the 700 tokens/s target remain open.

Previously, [the explicit shared-full projection AR4 route passed Rust qualification on ASROCK](../qualification/projection-ar4-shared-host-v1/README.md).
All 883 selected Rust tests passed, with four historical ignores; all 63 bounded
phases exited naturally, four executables were built and integrity postchecks
passed. Fifteen controller policy tests also passed. Fourteen exact tested source
bodies are integrated, preserving the default route and introducing an explicit
shared-full selector with separately measured configuration time. No kernel or
arithmetic changed. Fresh MI350 executable audits are linked above; the
same-generation default/shared GPU comparison is a separate execution gate.
All M0-M7, numerical acceptance, sustained 2,048/256 and 700 tokens/s remain open.

Previously, [the projection AR4 host observer completed on MI350](../qualification/projection-ar4-host-native-v1/README.md).
Four own-output forwards passed structural checks in one attempt without retry;
all 152 tensor slices, 576 terminal states, seven naturally exited/reaped leaves
and six surrounding audits passed. Forward host wall times were 10.593, 10.599,
12.531 and 12.582 seconds. Thousands of full-currentness checks per forward
dominate the nested host counters; the published table is not GPU timing or an
additive cost breakdown. The next optimization is a separately qualified
shared-full group-fence route. Numerical acceptance, sustained 2,048/256,
throughput and all milestones remain open.

Previously, [fresh observer executable audits and request assembly completed on MI350](../qualification/projection-ar4-host-runtime-v1/README.md).
Four bounded readelf/ldd commands exited naturally with complete library closures;
fourteen selector tests and eight assembler tests passed with unchanged sources.
The new request preserves the kernels, model, prompt, devices and deadlines and
changes only the worker, session and output path. This is validated preparation,
not a completed native observation or a numerical/performance result.

Previously, the [opt-in projection AR4 host-observation route passed its Rust qualification on ASROCK](../qualification/projection-ar4-host-observation-v1/README.md).
All 855 scoped Rust tests passed, with four historical ignores; thirteen controller
policy tests also passed. All 61 phases completed naturally, three executables
were built, and source/dependency/old-target postchecks passed. The seventeen
tested source bodies are integrated without changing the plain decode route or
runtime safety checks. The new counters separate inclusive host costs for diagnosis;
the subsequent GPU measurement is linked above. This is not numerical, throughput
or sustained 2,048/256 acceptance, and all milestones remain open.

Previously, the [projection AR4 host-observation supervisor passed 91 policy tests on MI350](../qualification/projection-ar4-host-supervisor-v1/README.md).
There were no failures, errors or skips, and the tested source snapshots are
unchanged. The new validator binds the stdout wrapper, native summary and
host-counter sidecar while retaining the existing image, lifecycle and audit
checks. This is synthetic CPU qualification of the Python supervisor only;
it is not a new Rust, GPU, numerical or performance result.

Previously, an [exact integer QKV audit completed on MI350](../qualification/layer0-exact-dot-v1/README.md).
All 6,144 retained layer-zero native QKV words match the once-rounded exact
BF16 dot products; the genuine framework matches 6,143. The previously
flagged Q row 168 is closer to the exact value in Ferric, so disagreement
with the framework alone is not evidence of a native arithmetic bug.
The original shard and six uploaded matrix slices were authenticated, and
all 24 arithmetic/mapping tests passed. This is a position-zero operator
diagnostic, not new-image internal capture, an FP32 reduction-order contract,
full-model numerical acceptance or a throughput result.

Previously, the [new linked RoPE AR4 numerical comparison completed on MI350](../qualification/rope-indexed-ar4-comparison-v1/README.md).
All 152 tensor slices share comparable input histories, but none matches the
independent framework reference bitwise. New logit relative-L2 errors are
0.0030448043, 0.0094134774, 0.0117436782 and 0.0050785906: unchanged at
position 0, improved at position 1, and worse at positions 2 and 3. Across
all slices, 90 improve, 24 regress and 38 are unchanged on that metric.
All eighteen comparator tests passed with unchanged source hashes. The
full before/after table is published, including the regressions. This is
not numerical acceptance; localizing the remaining differences is the next
gate before the sustained 2,048/256 workload or throughput claims.

Previously, the [new linked RoPE image completed native AR4 on MI350](../qualification/rope-indexed-ar4-native-v1/README.md).
The own-output chain was 9112 -> 67 -> 25 -> 576 -> 2701. All 152 tensor
slices, 576 terminal states, seven naturally exited/reaped leaves and six
surrounding audits passed structural checks in one attempt without retry.
Only the prefix image, session and output path changed. The 391.571-second
controller duration includes setup and audits and is not GPU throughput.
Independent tensor comparison subsequently completed as above; no numerical
or sustained 2,048/256 acceptance is claimed.

Previously, the [linked RoPE AR4 supervisor passed all 74 policy tests](../qualification/rope-indexed-ar4-supervisor-v1/README.md)
on MI350, with no failures, errors or skips and unchanged source snapshots.
The new admission path binds the actual emitted image to separate retained
producer and qualified consumer generations while preserving the runtime,
own-output recurrence and lifecycle checks. This is CPU-only synthetic
qualification, not GPU execution or numerical acceptance. The subsequent
GPU observation and independent tensor comparison are recorded above.

Previously, [linked indexed RoPE emission produced a new gfx950 HSACO](../qualification/rope-indexed-checked-emission-v1/README.md).
All eight staged phases and sixteen controller-policy tests passed. The
54,344-byte image has a 376-byte kernarg segment, Wave64, 512 bytes of fixed
LDS and zero private-segment bytes; ELF notes also report sixteen scalar
register spills. The continuation authenticates the retained successful
compiler leaves and uses the separately qualified indexed consumer, without
relabeling the original failed aggregate. All owned processes were reaped
and postchecks passed. This is checked emission, not GPU numerical acceptance
or a performance result. The subsequent GPU evaluation is recorded above.

Previously, the [indexed inert join V2 passed all thirteen staged CPU phases](../qualification/kir-indexed-formal-join-v2/README.md).
The full lower library passed 785 tests without ignores; its twenty indexed
tests also passed a separate repeat. The finalizer passed 190 default tests
with fifteen historical ignores, followed by an explicit passing actual
retained-handoff join. Twenty controller-policy tests passed separately.
The original work/storage limits remain unchanged, all owned processes were
reaped, and postchecks passed. This clears the measured inert-join refusal;
checked HSACO emission and GPU numerical evaluation subsequently completed
as above, without numerical acceptance.

Previously, an [unchanged-source control](../qualification/kir-indexed-baseline-control-v1/README.md)
reproduced the existing nested-enum location assertion in both a focused run
and the full original library (764 passed, 1 failed, 0 ignored). The 5,783-file
original RPO copy had no overlay or formatting changes. All six phases
completed naturally with clean postchecks. This establishes that the failure
predates the indexed join; it does not qualify the failing library. The next
candidate changed only that expectation in addition to the indexed-join source,
retained the alias-only rejection check, and passed the staged suite above.

Previously, the [indexed inert-join attempt](../qualification/kir-indexed-formal-join-attempt-v1/README.md)
compiled and passed all twenty new tests, but the full lower-library suite
stopped at 784 passed / 1 failed / 0 ignored. An existing nested-enum test
expected diagnostic location `(7, Some(4), 5)` and observed `(8, Some(7), 5)`.
The subsequent unchanged-RPO control above reproduced the same assertion;
the failure was not suppressed.
The subsequent finalizer and actual-handoff stages were not attempted.
All process groups exited naturally and postchecks passed; no new image,
GPU execution or numerical/performance qualification follows.

Previously, the [canonical KIR work diagnostic](../qualification/kir-join-work-diagnostic-v1/README.md)
localized the actual refusal to the inert formal layout join: 318 evidence
rows, 1,019 blocks and 6,096 nodes cause a 1,044,025,344-unit pending charge
after 40,799,816 units have already been accepted. The limits are unchanged.
All 190 default finalizer tests and 16 controller-policy tests passed; the
separate actual-capture test reproduced the expected exit-101 refusal.
All owned processes exited naturally and postchecks passed. An indexed join
was subsequently qualified for the staged CPU join as recorded above. No new
image or GPU result follows from this diagnostic.

Previously, the [RoPE/RPO checked-lowering attempt](../qualification/rope-materialized-rpo-lowering-v1/README.md)
cleared the previous semantic storage refusal and passed actual compiler
replay, then failed the finalizer's canonical-kernel work budget:
1,084,825,160 at refusal against the unchanged 1,073,741,824 limit. No HSACO
was emitted. All process groups exited naturally and postchecks passed.
The indexed join subsequently cleared this refusal without weakening its checks;
the later linked continuation emitted an image as recorded above.

Separately, the [actual four-step coefficient audit](../qualification/ar4-rope-input-audit-v1/README.md)
found all 512 BF16 coefficients identical to the retained framework tables.
Eleven synthetic tests passed. Raw FP32 differences remain at positions 1-3
but disappear at the BF16 boundary. This does not validate the new arithmetic
image or remove the position-zero residual differences.

The [RoPE/RPO four-step supervisor passed all 65 policy tests](../qualification/rope-materialized-rpo-ar4-supervisor-v1/README.md)
on MI350, with unchanged source snapshots and no errors or skips. These are
CPU-only synthetic admission and lifecycle tests, not a new GPU result. The
RoPE/RPO compiler result is recorded above; no new image is admitted.

Previously, the [independent genuine AR4 framework comparison completed](../qualification/projection-ar4-framework-v1/README.md)
on ASROCK. Two fresh-KV framework runs produce the same four-token sequence
as native: 67, 25, 576, 2701. All 152 tensor slices are comparable, but none
matches bitwise. Logit relative-L2 errors are 0.0030448043, 0.0140315337,
0.0043564111 and 0.0045131941; the table retains the larger position-1 error
and layer-level differences. All 25 child/utility leaves completed naturally
and all seven surrounding audits passed. This is a four-step diagnostic,
not numerical acceptance or the sustained 2,048/256 performance target.

Previously, the [RPO finalizer passed CPU qualification](../qualification/fe2o3-partial-move-rpo-finalizer-v1/README.md)
on ASROCK: 190 tests passed, with 15 historical actual-capture tests still
ignored. All six phases and postchecks passed. Three new finalizer products
join five preserved compiler products by their recorded identities. The
actual-capture tests still require explicit runs on a fresh checked handoff;
no new HSACO, GPU accuracy or performance result follows from this suite.

Previously, the [reverse-postorder partial-move compiler passed CPU qualification](../qualification/fe2o3-partial-move-rpo-v1/README.md)
on ASROCK: 2,700 Rust tests passed, including seventeen new scheduler and
differential tests, with 25 unchanged historical ignores. All twelve phases
and source/input postchecks passed. The four tested Rust overlays and raw
evidence are published without changing compiler limits or Ferric's production
route. Finalizer qualification subsequently completed as recorded above;
actual RoPE lowering and GPU numerical comparison remain separate gates.
No new HSACO or speedup is claimed here.

Previously, the [AR4 runtime completed four full-model forwards on MI350](../qualification/projection-ar4-native-v1/README.md).
The input chain was 9112, 67, 25, 576; outputs were 67, 25, 576, 2701.
Each next input equals the preceding checked argmax. All 152 tensor slices,
576 terminal state records, seven natural/reaped process leaves and six idle
audits passed structural checks in one attempt without retry. Fresh runtime
audits bind both CPU1037 executables. This is a native execution result, not
independent numerical acceptance. Genuine framework comparison subsequently
completed as recorded above; numerical acceptance, sustained 2,048/256
qualification and the 700-token/s target remain open.

Previously, the [AR4 supervisor and independent-reference policy suites passed](../qualification/projection-ar4-supervisor-v1/README.md):
52 CPU-only policy tests on MI350 and 38 on ASROCK, with unchanged source
snapshots. The framework result is explicitly retained as a primary-agent SSH
output observation, distinct from the bounded MI350 wrapper receipt. The tests
cover own-output recurrence, nonrecovering history comparability, separately
labeled conditional replay and the existing owned lifecycle. No model or GPU
kernel was executed by these suites. The separate native observation completed
as recorded above, followed by independent framework comparison. Numerical
acceptance, the long workload and 700 tokens/s remain open.

Previously, the [projection-residual autoregressive route passed fresh CPU qualification](../qualification/projection-ar4-cpu-v1/README.md)
on ASROCK through `mi350-2`: 1,037 Rust tests passed, with four unchanged worker
ignores, across all 87 build/test phases. Nine source files add explicit AR4
support while preserving the teacher-forced format and image bindings. Each
next input must equal the previous checked output; wrong trajectories, mode
mismatches and premature Close remain rejection cases. All seventeen selected
executables were rebuilt and all source/input postchecks passed. This is CPU
qualification, not a GPU observation. The matching GPU AR4 run completed as
recorded above, followed by independent history-aware framework comparison.
The 2,048/256 workload, numerical acceptance and 700 tokens/s target remain open.

The [branch-reduced RoPE candidate passed fresh CPU qualification](../qualification/rope-materialized-cpu-v2/README.md)
on ASROCK: all 33 Rust tests and eleven build/test phases passed with clean
postchecks. Only seventeen pure finite-check conjunctions changed; arithmetic,
all eighteen predicates and compiler limits are unchanged. Its subsequent
[checked gfx950 lowering attempt](../qualification/rope-materialized-lowering-attempt-v2/README.md)
hit the same semantic SSA partial-move storage limit. Both failed process trees
exited naturally and were reaped with clean postchecks. Neither attempt emitted
a candidate HSACO. Compiler state propagation and accounting are under review;
no compiler limit or proof check has been relaxed.

The earlier [RoPE checked-lowering attempt](../qualification/rope-materialized-lowering-attempt-v1/README.md)
stopped at the existing semantic SSA partial-move storage limit. No candidate
HSACO was emitted or launched. The failed process tree exited naturally and was
reaped; all input postchecks passed. The exact refusal and earlier manifest
syntax failure are retained. The subsequent branch-reduced validity expression
is the experiment recorded above; its fresh CPU qualification passed, but its
checked lowering also failed at the same limit.

The [BF16 RoPE candidate passed CPU qualification](../qualification/rope-materialized-cpu-v1/README.md)
on ASROCK: twenty new tests and thirteen reciprocal regression tests, including
the separately selected exhaustive case. All eleven build/test phases completed
with unchanged source and provider snapshots. The full tested fixture is
published; the default device crate and production route are unchanged. Checked
gfx950 lowering was subsequently attempted as above; GPU execution remains pending.

An [independent RoPE materialization diagnostic](../qualification/rope-framework-reference-v1/README.md)
completed on ASROCK. All 38,528 conditional BF16 words match the installed
framework when coefficients and products use its BF16 boundaries; fourteen
reference tests passed. Keeping products in FP32 produces differences at all
six nonzero captured-input probes. A separate FP64 table-construction model
still differs at position 2303. These are CPU operator diagnostics, not genuine
later-position model captures or validation of a new Rust GPU image.

Previously, the [SiLU image completed four full 36-layer forwards on MI350](../qualification/silu-materialized-decode-native-v1/README.md)
and its [independent framework comparison](../qualification/silu-materialized-decode-comparison-v1/README.md)
completed on ASROCK. All 152 tensor slices were compared; all four output tokens
match, but no complete tensor slice is bitwise identical. Logit relative-L2
errors are 0.0030448043, 0.0056276866, 0.0061773318 and 0.0079468052.
This improves three positions versus the corrected-residual baseline but worsens
position 2. The full tables retain that regression without fitting an acceptance
threshold. Seventeen comparator tests passed. Full-model acceptance, the long
workload and sustained performance remain open.

The [SiLU four-step supervisor](../qualification/silu-materialized-decode-supervisor-v1/README.md)
passed all 43 CPU policy tests on MI350, with unchanged source snapshots and no
skips. It selects the new MLP image using the existing CPU1022 executables and
preserves the original bootstrap images and lifecycle. These tests do not run
the full model; the separate native attempt and comparison completed as above.

The [BF16 SiLU image executed on MI350](../qualification/silu-materialized-native-capture-v1/README.md)
and its [independent layer-zero comparison](../qualification/silu-materialized-comparison-v1/README.md)
completed on ASROCK. All 28 arrays and 22 unchanged pre-SwiGLU arrays passed
structural checks. Final-hidden exact words improved from 2,728 to 3,807/4,096,
relative-L2 error fell from 0.0017142513 to 0.00059154807, and maximum absolute
error halved to 0.00390625. All 16,384 conditional residual words match; same-gate
materialized-product controls now match all 11,869 eligible words. Twelve
comparator tests passed. QKV and later framework differences remain; no numerical
acceptance threshold, full-model acceptance or throughput claim is introduced.

The [BF16 SiLU candidate completed checked gfx950 emission](../qualification/silu-materialized-lowering-v1/README.md)
on ASROCK, including all nine stages, actual replay and natural/reaped owner
completion. The 33,320-byte image retains the same ABI, 106 SGPRs, 106 VGPRs,
512 bytes LDS and zero reported spills/private scratch. Actual LLVM contains
the new BF16 narrow/widen before the up product. Separate policy suites passed
27 lowering and 36 capture tests. Layer-zero GPU execution and diagnostic
comparison completed as recorded above; numerical acceptance remains open.

The [explicit BF16 SiLU candidate](../qualification/silu-materialized-cpu-v1/README.md)
passed all 38 Rust tests and twelve controller-policy tests on ASROCK. The
candidate adds BF16 rounding between SiLU and its up product; existing
exponential arithmetic and Down2 scheduling are unchanged. The full tested
fixture is published separately because its dependencies differ from the live
default device crate. Checked lowering, layer-zero GPU execution and diagnostic
comparison completed as recorded above. Full-model validation remains a separate
gate. No production route or acceptance threshold changed.

The [independent four-step framework comparison](../qualification/projection-residual-decode-comparison-v1/README.md)
completed on ASROCK across all 152 tensor slices. All four output tokens match;
no complete tensor slice is bitwise identical. Logit relative-L2 error is
0.332344%, 0.757311%, 0.410781% and 0.859355% at positions 0-3.
The twelve comparator tests passed separately. Full tables and per-layer
trajectories retain every difference without introducing an acceptance
threshold. Numerical acceptance and sustained performance remain open.

The [projection-residual four-step GPU run](../qualification/projection-residual-decode-native-v1/README.md)
passed on `mi350`: four full 36-layer forwards, all 152 tensor captures,
576 terminal state records, six idle-device audits and seven natural/reaped
process leaves. One attempt completed without retry or forced cleanup.
Output tokens were 67, 198, 25 and 16. This establishes execution and structural
validity. Independent framework comparison completed as recorded above;
full-model acceptance and sustained performance remain open.

The [four-step candidate supervisor](../qualification/projection-residual-decode-supervisor-v1/README.md)
passed 35 synthetic policy tests on `mi350`; the separate executable-selector
suite passed all nine tests. Both retained unchanged sources with no skips.
These suites did not run a model or GPU kernel. The selected executables also
completed separate runtime dependency audits; the new GPU attempt completed
as recorded above.

The [projection-residual four-step decode route](../qualification/projection-residual-decode-cpu-v1/README.md)
passed joint CPU qualification on ASROCK through `mi350-2`: 1,022 passed,
four unchanged ignores, all 87 commands naturally completed and reaped,
and all 17 executables built. The 21 integrated source bodies match the
remotely tested files. The opt-in route selects the corrected residual image
across all 36 layers while preserving existing routes. Its GPU run completed
as recorded above; independent numerical acceptance remains pending.

The [captured SiLU diagnostic](../qualification/silu-materialization-diagnostic-v1/README.md)
passed all 18 synthetic tests on ASROCK. Its actual retained-data run reproduced
all 12,288 framework products from captured BF16 SiLU and up. With native gate
and up both identical to the framework, 1,542/5,702 rank-0 and 1,473/5,723 rank-1
products still differ. This localizes a remaining activation-path difference;
it does not establish missing materialization as the only cause. No new GPU
candidate, tolerance or full-model acceptance is claimed by this diagnostic.

The [projection-residual native capture and comparison](../qualification/projection-residual-native-capture-v1/README.md)
completed on `mi350`, with the independent comparison on `mi350-2`. All 28 arrays,
fourteen unchanged upstream arrays, full KV checks and six audits passed. Both
residual stages matched the independent conditional oracle on both ranks:
16,384/16,384 BF16 words. Against the genuine framework, final-hidden exact words
improved from 1,834 to 2,728/4,096 and maximum absolute error fell from 0.0625 to
0.0078125. Remaining stage differences are retained; full-model numerical
acceptance and performance remain open.

The [projection-residual capture supervisor](../qualification/projection-residual-capture-supervisor-v1/README.md)
passed all 34 synthetic policy tests on `mi350`, with unchanged sources and no
skips. No model binary or GPU kernel ran in that suite. Fresh runtime checks and
the genuine candidate capture/comparison subsequently completed as recorded above.

The [additive projection-residual parent/worker route](../qualification/projection-residual-runtime-v1/README.md)
passed joint CPU qualification on `mi350-2`: 988 passed, four unchanged worker
ignores, all 84 commands naturally completed and reaped. Both executables built;
the default-feature check and all postchecks passed. All 22 integrated source
files match the actual tested bodies. The separate route selects the new image
for both residual stages while preserving existing routes. Native candidate
execution completed as recorded above; independent model numerical acceptance
remains pending.

The [projection-residual comparison helper](../qualification/projection-residual-comparison-v1/README.md)
passed all twenty synthetic tests on `mi350-2`, including exact checks for both
residual stages, logical KV indexing, and retained framework differences. This
is comparator qualification, not a candidate GPU result. Its subsequent genuine
candidate comparison is recorded above, without full-model acceptance.

The [projection-residual candidate's checked gfx950 lowering](../qualification/projection-residual-lowering-v1/README.md)
passed on `mi350-2`, including exact-output replay and all postchecks. The
10,864-byte image also passed descriptor, ELF and ISA inspection: Wave64,
22 explicit arguments, six VGPRs, 36 SGPRs, and no reported spills. Manual
data-flow review finds the projection rounding before residual addition.
The candidate now has the separately qualified opt-in runtime route above.
GPU execution completed as recorded above; independent model numerical
acceptance remains open.

The separate [BF16 projection-residual candidate](../qualification/projection-residual-cpu-v1/README.md)
passed 17 new Rust tests and all 14 unchanged V18 regression tests on
`mi350-2`, with no ignores. All 18 bounded commands and source/tool/input
postchecks passed. The new kernel materializes the combined projection in
BF16 before residual addition; it does not alter the current runtime route.
Checked lowering and the subsequent layer-zero hardware result are recorded
above. Model numerical acceptance remains open.

The [projection-rounding replay](../qualification/output-residual-boundary-v1/README.md)
completed on `mi350-2`. The current formula reproduces all 4,096 captured native
residual values on both ranks. Materializing the projection in BF16 before
residual addition improves framework agreement from 2,886 to 4,094 values;
the projection sum itself still differs at three values. All twelve diagnostic
tests passed. This is conditional captured-data arithmetic, not a new GPU
kernel result or full-model numerical acceptance.

The [current layer-zero native capture](../qualification/layer0-native-capture-v1/README.md)
passed on `mi350`: 28 typed arrays were retained, both final hidden states
match the current native baseline byte for byte, and untouched KV regions
passed checks. One attempt, six selected-device audits and seven natural/reaped
process leaves completed without retries or forced cleanup. The controller's
40 tests also passed. The [independent 24-row stage comparison](../qualification/layer0-native-capture-v1/comparison/README.md)
also completed: both input norms and attention outputs match exactly. One
rank-0 QKV word differs, while the larger difference appears at output
projection/first residual. The comparison adapter and comparator separately
passed 15 and 18 tests. Numerical acceptance and performance remain open.

The [candidate-only native capture parent](../qualification/layer0-native-capture-parent-v1/README.md)
passed 289 Rust tests, including 14 new tests, with no ignores on `mi350-2`.
All fourteen executables, the default-feature check, 46 bounded commands and
source/input/artifact postchecks passed. Twenty separate controller tests and
format checks also passed. Its genuine current-image layer-zero intermediate
capture has now run as described above, without establishing numerical
acceptance or performance.

The [independent layer-zero capture](../qualification/layer0-framework-capture-v1/README.md)
passed on `mi350-2`: 33 genuine BF16 framework intermediates repeated exactly
across two fresh-KV passes. The new layer-zero output matches the original
framework reference byte for byte. Separate capture, launcher and comparison
suites passed 16, 18 and 14 tests. The actual CPU comparison still finds a
Ferric/reference difference: 1,834/4,096 equal words, maximum absolute error
0.0625 and relative L2 0.0035785690. Current native internal-stage capture is
the next numerical gate; no new tolerance or numerical acceptance is claimed.

The [paired-row MLP GPU comparison](../qualification/paired-row-mlp-native-v1/README.md)
passed on `mi350`: four complete output buffers and all 152 tensor comparisons
were byte-identical to the unchanged clock-enabled baseline. All 288 MLP
dispatches bind the new checked image. One attempt, six audits and seven
natural/reaped process leaves completed without retries or forced cleanup.
This follows fourteen CPU tests and nine checked lowering/replay/emission
stages on `mi350-2`. The checkpoint includes baseline/candidate raw-counter
tables and four plots; the report's twelve tests passed separately on `mi350`.
Independent numerical acceptance and calibrated/repeated timing remain open.

The [native clock V2 capture](../qualification/native-device-clock-observation-v1/README.md)
passed on `mi350`: sixteen clock samples, all 1,172 dispatch rows and exact
agreement across four complete output buffers and 152 tensor comparisons.
One attempt, six audits and seven natural/reaped process leaves completed
without forced cleanup. The report includes raw clock tables, per-device tick
distributions and layer/position plots. The controller passed 61 tests; separate
audit, input-preparation and report suites passed 7, 5 and 5 tests. Clock-domain
validation, calibration, aligned overlap and sustained throughput remain open.

The [matching clock parent](../qualification/gfx950-clock-parent-v1/README.md)
passed 275 Rust tests with no ignores, twenty separate controller tests, all
thirteen executable builds and the default-feature check on `mi350-2`.
All 44 bounded commands and source/input/artifact postchecks passed. The route
requires its own request, worker selector and sixteen-sample V2 sidecar, while
preserving the existing raw Control/capture joins and evidence limits. Its
locked parent dependencies remain distinct from the worker's newer runtime.
Both binaries subsequently passed runtime audits and the native capture above.

The [versioned clock recorder](../qualification/gfx950-clock-recorder-v1/README.md)
passed 669 selected Rust tests with four existing ignores on `mi350-2`.
All 21 new tests passed, the worker was rebuilt, and all 33 commands exited
naturally with unchanged input/source postchecks. Fifteen separate controller
tests passed. The new route retains the V1 dispatch report and samples both
ranks before and after each forward; errors poison the real owner even after
a committed forward. Native clock sampling passed in the capture above;
calibrated timings remain pending. No kernel image or arithmetic changed.

The [gfx950 raw clock-sampling API](../qualification/gfx950-clock-sampling-v1/README.md)
passed 648 selected Rust tests with four existing worker ignores on `mi350-2`.
This includes 14 new device/group clock tests, a fresh worker compatibility
build, 33 naturally completed commands and unchanged source/input postchecks.
A separate 14-test controller-policy run also passed. The API retains owned
descriptors, full currentness checks, rank identity and host sampling brackets.
The later worker checkpoint above wires it into Ferric's dispatch recorder;
native sampling is now observed, while calibration remains unqualified.

The [timestamp-enabled parent and worker run on MI350](../qualification/native-device-observation-v1/README.md).
The four-forward Qwen3-8B BF16 TP2 capture contains all 1,172 raw dispatch rows,
with 592/580 per rank. All four payloads, 152 tensors and output tokens are
byte-identical to the native baseline. All six audits and natural Close/reap
passed. The page includes actual per-device tick tables, ranges and layer/position
heatmaps. Raw ticks still provide no calibrated latency, cross-device alignment,
overlap, full-model numerical acceptance or sustained 2,048/256 result.

The first controller's real admission failed on a legacy-import dependency
before native launch. The successor binds all five authenticated dependencies
and restores imports on success/failure; all 47 tests passed on MI350 before the
successful GPU run. The original controller and failure account are retained. The native binaries
remain the separate [257-test parent](../qualification/native-device-parent-v1/README.md)
and [609-test worker](../qualification/native-device-routing-v1/README.md)
cohorts, with [actual runtime audits](../qualification/native-device-runtime-v1/README.md).
Those tests are distinct runs, not a combined suite. Existing plain/host-only
selectors and the V7 kernel image are unchanged.

The [resident-state GPU comparison](../qualification/resident-state-decode-observation-v1/tf4/README.md)
passes on MI350: all 152 tensors and four tokens are bitwise unchanged, with
576 fewer full group checks per forward and unchanged publication/rank operation
counts. Six audits and clean Close/reaping passed. The four host forward times
total 26.212 seconds versus 30.030 seconds in the retained control; this is one
diagnostic pair, not a qualified speedup or GPU/sustained-throughput measurement.
The page includes measured tables and a plot. Its comparison reader passed
21 tests; deployment/controller tests remain a separate 116-test checkpoint.

The [conditional final-norm/head replay](../qualification/tail-capture-diagnostic-v1/README.md)
completed eight diagnostics and 22 policy/layout tests on MI350. All 16,384 norm
words match; 40 of 607,744 logits differ, with all four argmax tokens matching.
Each stage uses its own captured immediate input and authentic original weights.
No head acceptance bound was invented; full-model numerical acceptance remains open.

The [paired resident-state fence candidate](../qualification/resident-state-fence-consolidation-v1/README.md)
passed 522 selected CPU tests with four ignored on `mi350-2`; a fresh worker
was built and all 17 bounded commands exited naturally. It removes redundant
private-observer fences while retaining the coordinator's outer fences and
public observers. The predicted 576 fewer group checks per forward have now
been measured on GPU with the new engineering worker. The production path is unchanged.

The [V7 all-layer autoregressive run](../qualification/independent-decode-observation-v1/ar4/README.md)
completed four own-token forwards on MI350 with six audits and clean Close/reap.
Outputs 67, 25, 576 and 2701 match the independent framework; all 152 complete
tensor rows still differ. Logit relative-L2 errors range from 0.00387545 to
0.00660156. This is not the sustained 2,048/256 target or numerical acceptance.

The [V7 all-layer teacher-forced run](../qualification/independent-decode-observation-v1/tf4/README.md)
completed four forwards through all 36 layers on MI350 with clean Close/reap
and all six audits. All four tokens match the independent framework reference,
but all 152 complete tensor rows differ; logit relative-L2 errors range from
0.00438509 to 0.01326323. Numerical acceptance remains open. These four payloads
are byte-identical to the prior native route, not a regression unique to V7.

The [historical two-residual replay](../qualification/historical-residual-capture-replay-v1/README.md)
matches all 32,768 captured BF16 words exactly across both residual stages,
both ranks and both profiles. This checks historical layer-0 residual boundaries,
not V7. The separate [historical MLP replay](../qualification/historical-mlp-capture-replay-v1/README.md)
now passes all 20 conditional stage checks and 41 CPU tests, with no bound
violations. It checks captured immediate inputs and authentic weights, not
all-layer or full-model acceptance. The all-layer observation controller passed
84 CPU policy tests and the separate framework diagnostic reader passed 12.

The [six-case new-image matrix](../qualification/independent-prefix-case-matrix-v1/README.md)
passes GPU execution and separate conditional numerical checks on MI350.
It covers genuine positions 0/4 and patterned positions 15/16/2,047/2,048,
not a full-model prefill or sustained decode. The observations below preserve
earlier checkpoints and their limitations at the time they were recorded.

## Observed Results

| Check | Observed result | Boundary |
| --- | --- | --- |
| Independent layer-zero framework capture on `mi350-2` | 33 BF16 stages x two passes repeated exactly; output matches original reference; separate 16, 18 and 14-test suites passed | Current Ferric layer-zero output still differs; first internal divergence and full-model acceptance remain open |
| Gfx950 clock-sampling API on `mi350-2` | 648 Rust tests passed, four existing ignores; fresh worker; separate 14 controller tests | CPU-only raw-sampler checks; no native ioctl qualification, calibration or recorder integration |
| Timestamp-enabled parent/worker on `mi350` | 1,172 raw rows; 152 tensors and four payloads/tokens unchanged; six audits and clean Close/reap; 47 controller tests | Per-device raw-tick plots, not calibrated GPU latency, overlap or sustained throughput |
| Device-tick parent/worker runtime audits on `mi350` | Both exact CPU-qualified ELFs inspected; four natural exits/reaps; all dependencies resolved | Seven auditor tests and 44 controller tests are separate; real admission failed before native launch |
| Corrected finite parent/worker CPU suite on `mi350-2` | 605 passed, 4 ignored; 34 owned commands | Includes 24 tests for the opt-in host-observation extension; no GPU execution |
| Historical KFD host-observation suite on `mi350-2` | 1,776 selected tests passed | Overlapping selections, including 15 new tests; separate source generation |
| Fresh Git-source worker build on `mi350-2` | 387 passed, 4 ignored; binary built from an empty target | Published Ferric/fe2o3 source pair; cached registry/toolchain, no parent or GPU run |
| Fresh-source V2 host-policy parent/worker cohort on `mi350-2` | 633 passed, 4 ignored; 12 binaries built and default-library check passed | Exact 13-file overlay; CPU checks alone do not qualify GPU execution |
| V2 host-policy baseline on `mi350` | 152 retained native tensor rows bitwise equal; six device audits and clean Close/reap | Newly built CPU633 binaries; no V2 autoregressive or sustained benchmark |
| V2 admission-cache arm on `mi350` | 152 native rows bitwise equal; repeated admissions fall to zero; six audits and clean Close/reap | No reliable overall latency improvement from the A/B/A diagnostic |
| V2 baseline repeat on `mi350` | 152 native rows bitwise equal; six audits and clean Close/reap; baseline operation counts restored | Baseline drift exceeds the initial aggregate cache delta; no confidence interval |
| V2 shared-currentness arm on `mi350` | 152 native rows bitwise equal; six audits and clean Close/reap; forward host total 61.458 seconds | Shorter than both baselines in this diagnostic, not a qualified speedup or isolated benchmark |
| Consolidated group-fence candidate on `mi350-2` | 475 selected CPU tests passed, 4 ignored; new worker built | 77 runtime plus 398 worker tests; parent remains separately qualified CPU633 |
| Consolidated group-fence worker on `mi350` | 152 native rows bitwise equal; six audits and clean Close/reap; forward host total 30.394 seconds | 18,720 duplicate full checks removed versus old shared worker; unisolated diagnostic, not sustained decode |
| Four teacher-forced forwards on `mi350` | 152 retained native tensor rows bitwise equal to the prior native route | All 36 layers, TP2, finite engineering images; not an independent full-model numerical bound |
| Four autoregressive forwards on `mi350` | 152 retained native tensor rows bitwise equal to the prior native route | Own-token history; no 2,048/256 workload or sustained throughput measurement |
| Six prefix-stage numerical cases | All 24 rank/profile rows satisfy the declared conditional norm/QKV bounds | Independent conditional arithmetic check of retained captures, not a new GPU run |
| Idempotent partial-move compiler candidate on `mi350-2` | 2,675 CPU tests passed, 25 ignored; fresh compiler built | Six new regressions executed; actual prefix-tile replay remains ignored |
| Matching finalizer tools on `mi350-2` | 190 default example tests passed, 15 ignored; three tools rebuilt | Same compiler source generation; actual-capture inert join remains ignored, no new HSACO |
| Checked reciprocal probe with new compiler on `mi350-2` | Previous storage gate cleared; canonical semantic capture emitted | Ranked projection rejects 1,027 blocks against the unchanged 1,024 limit; no HSACO or GPU run |
| Reduced-CFG reciprocal V3 on `mi350-2` | Exact census measures 1,013 blocks, 14 fewer; CFG-size gate cleared | Checked lowering now rejects an unsupported induction-latch form; no HSACO or GPU run |
| Canonical-counter reciprocal V4 on `mi350-2` | CPU arithmetic passes; fresh checked probe fails cumulative partial-move storage limit | No semantic capture or HSACO; prior V3 compiler successes do not transfer |
| Independent per-profile comparator on `mi350-2` | 20 CPU tests and eight historical rank rows passed | No paired bitwise prerequisite; unchanged bounds; not a new GPU run or new-image acceptance |
| Ordinary induction temporary compiler candidate on `mi350-2` | 2,683 CPU tests passed, 25 ignored; fresh backend and extractor built | Eight new regressions executed; actual prefix replay remains pending |
| Ordinary induction matching finalizer tools on `mi350-2` | 190 default tests passed, 15 ignored; three tools rebuilt | Exact new compiler generation; actual-capture inert join remains a separate gate |
| V3 checked probe with ordinary-induction compiler on `mi350-2` | All nine phases passed; actual replay and inert join passed; fresh 53,560-byte gfx950 HSACO emitted | Eight runtime obligations remain; no fresh GPU execution, numerical acceptance or throughput result |
| Independent-profile observation adapter on `mi350-2` | 23 synthetic policy tests passed, no skips | No launcher; mathematical return values are mocked; real-capture integration pending |
| Independent native-profile adapter on `mi350-2` | 98 tests passed, one ignored; fresh test and native executables built | All 14 new tests ran; no deployment or new GPU execution yet |
| Independent runtime-audit collector on `mi350-2` | 18 synthetic policy tests passed, no skips | New deployment namespace; actual MI350 ELF/library audit still pending |
| First new independent prefix case on `mi350` | Both profiles and both ranks pass GPU execution and separate conditional numerical checks at genuine position 0 | 8,192 attention BF16 values exact; maximum output error/bound ratio 0.015053; not full-model or throughput acceptance |
| Complete new-image prefix matrix on `mi350` | Six cases and 24 profile/rank rows pass; 49,144 attention values exact and eight within one BF16 step | No norm/QKV bound violations; maximum output error/bound ratio 0.0229955; patterned long KV is not authentic model prefill |
| Historical two-residual capture replay on `mi350` | 32,768 BF16 words exact; 14 policy/layout tests passed | Original layer-0 captures and authenticated embedding row; no new GPU run, V7 or MLP acceptance |
| New V7 image through 36-layer TF4 on `mi350` | 152 captures, four matching framework argmax tokens, six audits and clean Close/reap | All 152 complete tensor rows differ from framework; no numerical or performance acceptance |
| New V7 image through 36-layer AR4 on `mi350` | Four own-token forwards, matching framework histories and outputs, 152 captures, six audits and clean Close/reap | All 152 complete tensor rows differ from framework; not the 2,048/256 workload or numerical acceptance |
| Historical MLP capture replay on `mi350` | All 20 conditional stage checks and 41 CPU tests pass; maximum down error/bound ratio 0.0148193 | Original layer-0 captures; no new GPU run, V7 or all-layer acceptance |
| Paired resident-state fence candidate on `mi350-2` | 522 CPU tests passed, four ignored; fresh worker built; 17 natural command exits | Eight exact source files and six new regressions; predicted counter reduction is not a GPU or performance result |
| Resident-state deployment/controller on `mi350` and `mi350-2` | 116 CPU policy tests pass; actual worker export verifies source/build/test evidence | No GPU execution of the new worker yet; parent, prior worker and V7 provenance remain distinct |
| Resident-state worker on `mi350` | 152 tensor rows bitwise equal; 2,304 fewer full group checks; six audits and clean Close/reap; 21 comparison tests pass | One unchanged-V7 TF4 pair; 26.212 vs 30.030 seconds host forward total, not a qualified speedup or sustained benchmark |
| Conditional final norm/head replay on `mi350` | 16,384 exact norm words; 40 differing logits of 607,744; four matching argmax tokens; 22 tests pass | Eight immediate-input diagnostics, not a chained reference, justified MFMA error bound or full-model acceptance |

Both four-forward runs completed their Close protocol, reaped their owned
processes, and passed their six surrounding device-state audits. The CPU suite
also completed with no forced cleanup. The V1 host-instrumented (CPU605)
generation has independently completed its teacher-forced run, described below. Its
autoregressive run has not yet been repeated with instrumentation.

The six numerical cases cover genuine positions 0 and 4, plus patterned
positions 15, 16, 2,047 and 2,048. They check first normalization, QKV projection,
head normalization, split-half RoPE and exact current-value append. The checks
are conditional on captured preceding-stage values and the declared floating
point premises: at most one FP32 ULP for the square-root sequence and correctly
rounded FP32 division. A universal proof of these instruction sequences is
still open. The candidate cap and numerical policy were not loosened to obtain
these results. The older attention/output-projection check remains separate.

A subsequent exact binary32 CPU diagnostic found six standalone reciprocal
counterexamples and 21 chained counterexamples over permitted synthetic raw
seeds. For example, with denominator `1 - 2^-24` and reciprocal seed `1`, the
correction can round a midpoint back to `1`, although the correctly rounded
reciprocal is the next FP32 value. These are not observations of the GPU's
actual seed selection or model failures. They show that the assumed division
rounding does not follow from the tested instruction model and seed envelope;
the prerequisite remains unresolved. No threshold was widened. The diagnostic
receipt is `9af99ee3b0b4247a0a7b504f8364fc6ad1dd6f8c825b37f05aedc330140722d8`.

The first fresh checked compiler probe for a separate exact-reciprocal candidate
passed metadata but failed semantic-SSA partial-move validation before producing
an HSACO. Its cumulative storage budget first rejected charge 2,097,153 against
the existing 2,097,152-word limit. This does not mean that one additional word
would suffice for the whole function. The limit was not raised. Source and
old-target postchecks passed; the failed owned process tree exited without
forced cleanup and was reaped. The [failure record](assets/finite-prefix-v228/reciprocal-probe-failure.json)
keeps this distinct from successful CPU arithmetic checks and from the existing
GPU images. A smaller candidate control-flow graph needs its own arithmetic and
checked compiler validation before any GPU claim.

The revised candidate now uses masked subtraction and boolean nearest-even
rounding within the same 24-step restoring algorithm. Bounded integer
operations use wrapping methods to avoid unnecessary checked-MIR branches;
only creation of the all-ones subtraction mask intentionally wraps. The
positive-normal domain, power-of-two path, two prefix wrappers, shared V3
arithmetic, kernel body and independent numerical references remain unchanged
relative to the first candidate.
No compiler storage cap or numerical tolerance was increased.

Its fresh [CPU validation](assets/finite-prefix-v228/reciprocal-cpu-branchless.json)
passed 11 ordinary arithmetic tests, six source contracts, the explicitly
selected exhaustive test over 8,388,608 significands in one binade, and all
1,007 independent Fraction nearest-neighbor vectors. The exhaustive test is
ignored by the ordinary invocation but was separately executed successfully.
All nine owned phases exited naturally, source/dependency postchecks passed,
and all 50 raw records plus both test binaries were rehashed locally. The
additional source contract pins macro-rule structure; it is not an independent
numerical reference.

The revised candidate has now undergone a fresh checked compiler probe on
`mi350-2`, but [lowering still fails](assets/finite-prefix-v228/reciprocal-probe-branchless-failure.json)
at the same first rejected cumulative partial-move storage charge:
2,097,153 against 2,097,152. The branchless rewrite did **not** unblock the
retained compiler. No HSACO was emitted and no GPU execution was attempted.
The probe driver passed 23 pure tests; its owned outer supervisor passed 14.
Before compilation, both joined the exact passing candidate CPU receipt,
seven Rust source bodies and 53 device-provider sources. Metadata passed,
Cargo then exited 101, and all source/tool/configuration/dependency/old-target
postchecks passed. The owned process tree exited naturally and was reaped.

The [next compiler candidate](../qualification/fe2o3-partial-move-idempotent-v1/README.md)
removes a redundant clear/reinsert when a local is already marked wholly moved.
Its [fresh CPU validation](../qualification/fe2o3-partial-move-idempotent-v1/cpu-result.json)
passed 1,487 Pliron tests and 1,188 compiler tests, with 25 ignored. All six new
exact-limit, cumulative-charge, moved-read and unchanged-join regressions ran;
the full default suites also exercised existing CFG, loop and replay coverage.
Actual prefix-tile replay is among the ignored tests and remains unqualified.
The fresh backend and extractor built, all ten commands exited naturally, and
source/dependency/tool/old-target checks passed. The compiler patch, regression
tests and precise source boundary are published separately from runtime code.

The first CPU controller attempt rejected libtest's passing `should panic`
annotation. Its failure was retained; the parser fix passed nine policy tests,
then both suites and the compiler build were rerun under fresh paths. The
completed run's 59 raw records, full copied compiler source and five selected
artifacts were retained and rehashed locally. This is not evidence that the
complete reciprocal function fits. The [matching finalizer build](../qualification/fe2o3-partial-move-idempotent-v1/finalizer-result.json)
subsequently passed 190 default tests with 15 ignored, rebuilt three tools,
and preserved all five qualified compiler products. Its six commands exited
naturally; source, dependency and prior-target checks passed. The 36 raw
records and three new tools were retained and rehashed locally. The actual
V6 capture join remains ignored.
Compiler limits, numerical tolerances and production admission remain unchanged.

The [fresh checked probe](../qualification/fe2o3-partial-move-idempotent-v1/checked-probe-result.json)
with those exact compiler/finalizer products cleared the partial-move storage
gate. It emitted a canonical V41 semantic capture, then rejected the unchanged
ranked CFG block limit. A matching-decoder [read-only census](../qualification/fe2o3-partial-move-idempotent-v1/semantic-census.json)
measured one kernel-root function containing 1,027 stored blocks, three above
the per-function limit of 1,024. All source/tool/dependency/prior-target
postchecks passed and the owned processes exited naturally and were reaped.
This is not ranked or target KIR acceptance, replay, HSACO emission or GPU
validation. The next source change targets avoidable reciprocal branches;
the numerical policy and compiler caps remain unchanged.

The [reduced-CFG V3 candidate](../qualification/exact-prefix-reciprocal-v3/README.md)
now removes the reciprocal domain and power-of-two branches while preserving
the 24-step recurrence and rejection sentinel. Its actual `mi350-2` CPU run
passed 12 ordinary tests, six source contracts, the explicit exhaustive
8,388,608-significand test and all 1,007 Fraction vectors. The unapplied source
patch and exact source pins are published with the result. All nine commands
exited naturally and source/dependency checks passed.

Its [fresh checked probe](../qualification/exact-prefix-reciprocal-v3/checked-probe-result.json)
has now cleared the CFG-size gate. The new exact-decoder census measures
1,013 stored blocks, 14 fewer than V2 and 11 below the unchanged cap. Lowering
then rejects an induction latch that does not copy field zero of a checked
result. The owned tree exits naturally and is reaped; all source/tool/dependency
and prior-target checks pass. There is still no new ranked/target KIR, handoff,
HSACO or GPU result. This block-count reduction is not a GPU speedup.

The [V4 source candidate](../qualification/exact-prefix-reciprocal-v4/README.md)
now changes only the bounded reciprocal counter to `step += 1_u32`, with its
matching macro contract. The actual V3 capture identifies both rejected
counter latches as plain temporary copies. V4 passed the same 12 ordinary
tests, six contracts, explicit exhaustive test and 1,007 Fraction vectors on
`mi350-2`. The source patch and exact CPU result are published separately from
runtime adoption. Its [fresh checked probe](../qualification/exact-prefix-reciprocal-v4/checked-probe-result.json)
has now failed semantic-SSA partial-move validation before emitting a capture:
the first rejected cumulative charge is 2,097,153 against the unchanged
2,097,152-word budget. The new checked counter changes the MIR; V3's successful
storage and CFG checks cannot be carried forward. All postchecks passed and
the owned tree exited naturally and was reaped. The latch fix remains
unqualified, with no new HSACO or GPU result.

The new [per-profile comparator](../qualification/prefix-profile-numerical-v1/README.md)
checks each implementation against the independent norm/QKV/head/RoPE,
attention and output references without first requiring V5/V6 bitwise parity.
All 20 CPU tests passed on `mi350-2`, including nonzero historical KV across a
page boundary and rejection of a finite historical-value corruption. Four
independent replays of retained position-15/16 profiles passed, covering eight
rank rows. All 16,384 attention BF16 words match the rounded reference; the
largest O error/bound ratio is below 0.011039. These are conditional checks of
older captures, not new GPU execution or validation of the reciprocal candidate.
The future native-child adapter still must authenticate ownership, captures,
untouched KV and actual-image arithmetic/ISA prerequisites.

The [next compiler candidate](../qualification/fe2o3-ordinary-induction-alias-v1/README.md)
recognizes only the adjacent ordinary-Add temporary seen in the V3 capture.
It retains exact type/definition custody, no-address-escape, positive-step,
unsigned no-overflow and replay checks, without granting assertion-elision
authority or changing caps. A fresh `mi350-2` run passed 1,487 Pliron and 1,196
compiler tests, with 25 unchanged ignored selections. All eight new regressions
ran; the extractor/backend built and all postchecks passed. The complete
5,780-file prior source generation was joined before applying the two-file
change. The patch, source pins and actual result are published; this CPU result
does not yet establish full checked lowering or a new GPU image.

The [matching finalizer tools](../qualification/fe2o3-ordinary-induction-alias-v1/finalizer-result.json)
have now built against that same generation. Their 190 default tests and 14
controller policy tests passed; 15 finalizer tests remain ignored, including
the actual-capture inert join. All six commands exited naturally, all five
compiler products stayed unchanged, and source/dependency/tool/configuration
and four old-target postchecks passed. The complete archive and selected
products were retained and rehashed locally.

Fresh V3 checked lowering has now [passed as a separate leaf](../qualification/fe2o3-ordinary-induction-alias-v1/checked-lowering-checkpoint.json)
in 1,771.339 seconds, naturally exiting zero before the unchanged 1,800-second
deadline. The actual outputs include semantic MIR, neutral/target KIR V11,
gfx950 LLVM and a 4,022,416-byte inert handoff. This clears the previous latch
refusal. That record remains an interim checkpoint; the subsequent
[terminal nine-phase probe](../qualification/fe2o3-ordinary-induction-alias-v1/checked-probe-result.json)
passed actual-capture replay in 1,401.55 seconds and the separately selected
inert-join test. All nine phases exited naturally; source/dependency/tool and
five prior-target postchecks passed, and the owned tree was reaped. All 92
archive members were rehashed locally. A fresh 53,560-byte gfx950 HSACO was
emitted, with SHA-256
`4885204c8d510122588549107f42d2bc6f180f48fbc4eddd3bb1260e8d6629c5`.
This is an engineering image: eight runtime requirements remain undischarged,
generic proofs remain unsupported and production authority is absent. New
actual-image reviews, deployment and MI350 numerical checks are still required;
there is no new GPU or performance result.

A separate [observation adapter](../qualification/independent-profile-observation-v1/README.md)
now validates the new independent-profile reports while preserving the existing
child-custody validator and numerical references. All 23 synthetic policy tests
passed on `mi350-2`, including first-read authentication failures that occur
before the original Reader registers a pin. Those failures abort, whereas
ordinary numerical rejections remain specific to each profile. Its tested
source is published, but it has no launcher and has not yet replayed real new
captures through the full adapter. Mock reference returns in these tests are
not numerical evidence or a new GPU result.

The matching [native adapter](../qualification/independent-native-profiles-v1/README.md)
has now passed a fresh 98-test CPU suite, with one existing test ignored. All 14
new tests ran alongside the preserved historical inventory. Its new explicit
modes retain the existing owned child, request, review, finite-output, untouched
KV and Close checks, while capturing each profile without requiring paired
bitwise equality. The old paired mode remains unchanged. Both executables built;
all six phases exited naturally and source/dependency/fixture/tool and six
prior-target postchecks passed. All 5,856 archive members were rehashed locally.
This is a tested host adapter, not deployment or GPU numerical acceptance.

The [runtime-audit successor](../qualification/independent-runtime-audit-v1/README.md)
also passed all 18 synthetic policy tests. It changes only deployment/output
namespace handling and preserves the existing ELF, library identity, process
ownership and resource checks. It does not mint reviews or launch authority;
an actual audit of the new deployed executable on MI350 remains required.

The [new deployment](../qualification/independent-deployment-v2/README.md)
has now been exported and verified on MI350. Its receiver checks both complete
evidence archives, all 5,783 native source files and the exact emitted image and
native executable. All 30 policy tests passed, including actual-command
regressions for the string-valued byte-count bug found during the first export.
The unchanged numerical reference suite also passed all 20 tests on MI350.
These are deployment and CPU-reference checks, not new GPU captures or an
end-to-end model throughput result.

The [first new GPU case](../qualification/independent-prefix-first-case-v1/README.md)
has now completed on MI350 after the actual executable/library audit. Both
profiles closed naturally, all eight owned leaves completed, and six device
audits passed. Separate CPU reference checks accepted all four profile/rank
rows: no normalization or QKV bound violations, exact current KV append,
8,192 exact attention BF16 values, and output partials within the unchanged
bound. The maximum output error uses about 1.51% of that bound. All 64 candidate
workgroups on each rank performed useful work in this observation; that is
not a scheduling guarantee or a performance measurement. This publication
covers only genuine position 0 and preserves the conditional arithmetic and
runtime limitations. It does not establish complete model decoding or the
700 tokens/s target.

The subsequent [complete matrix](../qualification/independent-prefix-case-matrix-v1/README.md)
now covers all six selected positions with the same image, executable and
fixed numerical policies. All 48 owned leaves completed naturally and all 36
surrounding device/process audits passed. The local audit rehashed all 366
case files, including 24 full rank captures. Eight of the 49,152 attention
values differ by one BF16 step; all others match exactly. The largest output
error/bound ratio is 0.0229954713, about 2.30% of the allowed bound. Every case
observed useful work from all 64 candidate workgroups on each rank. No bound
was widened, and no full-model or performance acceptance follows from this
prefix-only result. The matrix page reports each case separately.

The next [residual-capture comparator](../qualification/independent-residual-reference-v1/README.md)
has passed 18 focused CPU tests on MI350. It reuses the unchanged integer
FP32/BF16 rounding oracle and checks both residual boundaries, explicitly
feeding the post-attention residual into the post-MLP residual check. The
tests cover exact output words, cancellation, rounding, overflow, corrupt
captures and incorrect stage chaining. These are synthetic component tests,
not new full-layer GPU evidence or validation of the intervening MLP.
The subsequent [historical capture replay](../qualification/historical-residual-capture-replay-v1/README.md)
now checks the actual retained residual boundaries with all 32,768 BF16 words
exact. That result still does not validate upstream MLP partial arithmetic.

The V7 image has also completed a new finite all-layer teacher-forced run.
Its four host-inclusive forward durations were 7.2200, 7.2450, 7.8164 and
7.7490 seconds. These are engineering diagnostics, not GPU latency or sustained
throughput, and do not establish a speedup. The separate independent framework
comparison reports all 152 tensor differences without inventing an acceptance
threshold. Four matching tokens do not qualify full-model correctness.

## Source Publication

The [source index](assets/finite-prefix-v227/source-index.json) binds the 289
imported files to the build-host source snapshot. All 355 Ferric paths in the
CPU cohort's source map were checked against the published tree. Cargo path
dependencies and literal source/fixture includes were audited separately;
the 167-file finite test census alone was not a complete source closure.
That index describes the original CPU605 generation. The separately tested
[V2 overlay](assets/finite-prefix-v228/host-policy-source.json) records later
changes without rewriting the original source evidence.

This includes the finite parent/worker, shared wire imports, consumed tokenizer
fixtures, and required device source/test dependencies. Only the four shared
wire files were added under the legacy v4 worker. Its Cargo manifest, lockfile,
main program and unrelated experiment additions were left unchanged.

The matching sibling runtime is now published at
[fe2o3 `6964f6129`](https://github.com/harsh-nod/fe2o3/commit/6964f6129c4c42d343117f61cdbcfe6166392535)
in both fe2o3 repositories. The bounded runtime export retained 765 files,
checked all 479 mapped runtime inputs, and imported exactly 84 changed paths
against its pinned base. The parent uses its own locked fe2o3 Git dependency;
the two dependency generations are not interchangeable.

A fresh Git-archive build paired Ferric `9eca2576` with that fe2o3 commit on
`mi350-2`. With an empty target directory, all 387 selected worker tests passed
and four were ignored; the worker binary also built. Cargo metadata verified
the exact ten local packages and 29 registry packages, and all 6,921 extracted
source files remained unchanged. The registry cache and toolchain were reused.
The [fresh-build record](assets/finite-prefix-v228/clean-worker-build.json)
and [worker instructions](../adapters/tp-peer-finite-engineering-worker-v1/README.md)
record the exact pair. This does not reproduce the parent, compiler/HSACO
pipeline, or GPU run from clean source, and is not a workspace-wide test pass
or a newly qualified legacy-worker build. Full workspace formatting, Clippy
and regression gates remain required before merging to production.

## Measured Host Overhead

The instrumented teacher-forced run completed on `mi350`: all 152 native
tensor rows matched the prior native baseline bit-for-bit, all three pre-run
and three post-run device audits passed, and Close/EOF/owned process reap
completed without forced cleanup. Its receipt is
`5825e7859db0066b13f3f844d6a103dec5d0d6ec428973426a2d6d4529bc6304`.
The selected parent and worker are the corrected CPU605 binaries, not the
earlier uninstrumented binaries. All 57 pinned case outputs were independently
rehashed after retention on the local machine.

The [machine-readable host summary](assets/finite-prefix-v227/host-observation.json)
retains nanosecond values and counter names. These are **inclusive, nested host
scopes**, not calibrated GPU durations or additive contributions. Currentness
checks revalidate device, topology, allocation and queue state. Do not add
rank times, subtract them from wall time, infer overlap, or turn four
teacher-forced forwards into a target-workload tokens/s result.

| Position | Forward Wall (ms) | Full Currentness R0 (ms) | Full Currentness R1 (ms) | Admission R0 (ms) | Admission R1 (ms) | Full Checks R0 / R1 |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| 0 | 18,240.289 | 8,512.988 | 8,477.279 | 14.444 | 10.846 | 5,068 / 5,048 |
| 1 | 18,252.250 | 8,523.919 | 8,486.701 | 14.496 | 10.812 | 5,068 / 5,048 |
| 2 | 20,186.874 | 9,484.764 | 9,451.487 | 14.461 | 10.819 | 5,644 / 5,624 |
| 3 | 20,203.428 | 9,499.867 | 9,463.653 | 14.412 | 10.813 | 5,644 / 5,624 |

The setup snapshot interval was 167.644 seconds and Close took 26.703 seconds.
The parent diagnostic took 413.571 seconds including its setup, forwards,
readbacks and teardown; the surrounding controller took 430.597 seconds.
These boundaries differ and must not be mixed into a performance comparison.
This is one diagnostic run with all performance opt-ins off, not an ablation
or a statistically qualified latency measurement.

The observations make repeated currentness checks the first optimization
candidate to investigate. The controlled comparisons separately
enable immutable kernel-admission caching and sharing a fresh full-currentness
observation within a group fence. Mutable allocation, pointer, ABI, geometry
and queue checks must remain. Round publication already shares a fresh
observation in the baseline, so that existing behavior cannot be claimed as a
new optimization. No speedup is claimed until the separate variants pass their
own numerical, ownership and measurement checks.

The explicit V2 policy implementation has now passed a fresh-source CPU
cohort: 398 worker/shared-wire tests, 224 disjoint parent-library tests and
11 parent CLI tests, with four ignored. All 12 binaries built, the default
parent library check passed, and all 6,928 post-overlay source files stayed unchanged.
All 213 retained raw records and 12 binary bodies were rehashed after transfer.
The [policy guide](GFX950_HOST_POLICY_V2.md) documents the checks each arm
retains and the [CPU record](assets/finite-prefix-v228/host-policy-cpu.json)
identifies this new generation. Its separate V2 baseline has now completed on
`mi350`, with all 152 native tensor rows bitwise equal, six passing device audits
and clean Close/reap. The [GPU record](assets/finite-prefix-v228/host-policy-baseline.json)
retains the new identities and host counters. All 57 final output files were
independently rehashed locally. Forward host durations were 18.266, 18.304,
20.174 and 20.171 seconds. These are not GPU durations or qualified throughput.
The separate admission-cache arm also passed native parity and lifecycle checks.
Per-forward repeated admissions fell from 148 / 145 to zero, while initial
admission, dispatch, full-currentness and I/O counts remained unchanged.
Its four forward host durations sum to 76.933 seconds versus baseline's
76.916 seconds: no overall improvement is established. The
[cache record](assets/finite-prefix-v228/host-policy-cache.json) and
[comparison table](GFX950_HOST_POLICY_V2.md#admission-cache-observation) retain
this observation without a speedup claim. The
[repeated baseline](assets/finite-prefix-v228/host-policy-baseline-repeat.json)
also passed parity, six audits and clean Close/reap; all 57 final outputs were
rehashed locally. Its forward host durations sum to 77.471 seconds, so the
cache total lies between the two baseline totals. The observed baseline drift
exceeds the initial aggregate cache delta. These three runs establish no
reliable end-to-end cache improvement or confidence interval.

The separate [shared-currentness arm](assets/finite-prefix-v228/host-policy-shared.json)
has now passed the same parity, six audits and shutdown checks; its 57 final
outputs were rehashed locally. The four forward host durations total 61.458
seconds versus 76.916 and 77.471 seconds for the baselines. New shared-group
checks replace repeated per-rank full checks; admissions, dispatches and I/O
counts remain unchanged. The [plot and counter table](GFX950_HOST_POLICY_V2.md#shared-currentness-observation)
show the observed effect and limits. Another compiler job was observed during
C preflight on the shared host. This is not an isolated or statistically
qualified performance result, GPU timing, kernel overlap or sustained decode.
The [consolidation candidate](GFX950_HOST_POLICY_V2.md#consolidation-candidate)
has since passed 475 selected CPU tests with four ignored, and its new worker
built from fresh sources. It removes duplicate rank currentness checks after
a fresh group fence while retaining queue and poison validation. Its runtime
source is published at
[fe2o3 `725ecc6a5`](https://github.com/harsh-nod/fe2o3/commit/725ecc6a500ff49e7dfaefb38b027f6bcc223ebf)
in both repositories. Its own GPU comparison and the V2 autoregressive diagnostic
remain pending; no earlier GPU result is transferred to the new binary.

## Changes And Retained Failures

The finite route uses an explicit bounded parent/worker protocol, typed
generation and bank checks, and ownership-bound shutdown. Host observation is
opt-in and keeps operational optimizations off. Its nested counters measure
host activity, not GPU elapsed time, kernel overlap, or tokens/s.

A parent compile failure exposed a `u32` image-size versus `u64` request-size
comparison. The correction widens the image size with `u64::from`, and adds
regressions rejecting oversized requests with identical low 32 bits. The
corrected 605-test run is a new result; the failed run is retained.

Portable-deployment tests also exposed aliasing in a cached test fixture.
The test-only correction deep-copies the fixture and adds a mutation-isolation
regression: all 23 tests passed. The production reader and exporter were
unchanged. The earlier 22-test run remains a failure, not a passing result.

## Evidence Identities

These are SHA-256 identities of retained receipts, not links to publicly
downloadable artifacts. Large logs, captures, model weights, compiler caches
and runtime binaries are deliberately excluded from Git. A receipt digest
alone is not a self-contained reproducer or proof.

| Retained receipt | SHA-256 |
| --- | --- |
| Corrected CPU605 completion | `f9b4da95d00cb62a3eadb33d86ee942d0bdf0ecb5c559650161846f4dcd081e8` |
| Fresh-source V2 CPU633 completion | `1fc4d17534161e6e7f96e6d0eba0a2455227ba2166623823f6a74f845b93deae` |
| V2 baseline GPU completion | `8d46f69e481ea906733765dffee163a0a6ba33bf7816316741e867880a82ea5f` |
| V2 admission-cache GPU completion | `0952f130801dcf3c958f5dff8b1228a401f436640ee7591465947edcc3881cac` |
| V2 repeated-baseline GPU completion | `4f5cbd384e136d5c5a05cf0a9d87733b249ee85fec5cd421d3164df87af4110d` |
| V2 shared-currentness GPU completion | `6aad2f46f234a98ac9f792a6108e98e6bbb6269ad81a6913aa1524f8972cd097` |
| Consolidated group-fence CPU475 completion | `0231c40bf9a5ff285e041f29b2c958e26972e73f6a4485072e153a0bd178171b` |
| Consolidated group-fence GPU completion | `ebecff7a6e459cbb6fa8b4c750bc9c524adb417405e2827608175ba0a2c66a5f` |
| Exact-reciprocal checked probe failure | `3d3fd530e171a6e2977d995c78d5665d9e5a728e173eb21b445891148ba61e0d` |
| Branchless reciprocal CPU completion | `d16dd989342d6092445d6fc3d046184ddf160527b1681d713e9b1291f6b106a4` |
| Corrected CPU605 owner | `dde383021440a721bd2c41bff614ea1156a3b36c6fa48cd2ad27c95337af7cb2` |
| Historical KFD completion | `e6cdd55c6a2dda4dafdd25a4c8d5e3670312c912cdbc1f20258a6e93db48e603` |
| Teacher-forced four-forward completion | `56f72c0c8c76e7c2c3f00ca767d5a7b53c66ba4b2c17589059565c2c2532fb8b` |
| Autoregressive four-forward completion | `0553862c7e65b74020028421b3bb668ff6eaf0080ba4a1bc405587707a403d77` |
| Numerical case: genuine position 0 | `a1990e1f8153aa8df23a8db175e293bff7d17b0f3c7fc181b223158df9a41586` |
| Numerical case: genuine position 4 | `81d9b3b17d7763b5c9d40db023c6ba1cf0eb61c61d429d1f1214fbef55475adb` |
| Numerical case: patterned position 15 | `fa7795a3ff7968c212600e2ba30dca7c39b4ccf29ac2b075d42a18377b66fa9e` |
| Numerical case: patterned position 16 | `39ae8374a10ea306e0f1426c87e9c3184f04de250e9f82695973158ad941e1e4` |
| Numerical case: patterned position 2,047 | `7cb4e5ee27e423b96f2eae3fbb2518e14da4f546066cdb08bf5d251f2df32fca` |
| Numerical case: patterned position 2,048 | `676d7e878105972365b15c9aba5f5361825a7761e04f761178487271fae491de` |
| Portable test-only correction, 23 tests | `0481e077defafed91c2272984b3d5d1da5d97cb9da9431ba307760000698de8a` |

## Next Gates

1. Diagnose the remaining differences in the completed four-step own-output
   autoregressive comparison. The checked BF16 RoPE image now runs on GPU;
   its comparison is mixed, not accepted. Exact layer-zero QKV evaluation
   shows that a framework disagreement need not be a native arithmetic bug.
   Distinguish reduction-order rounding from semantic errors at the remaining
   boundaries without fitting acceptance thresholds to the observations.
2. Validate TP residuals, MLP, final normalization, logits and token selection
   independently, including cumulative layer error and authentic KV history.
   Preserve the old bitwise-comparison mode as a separate historical check;
   do not equate native parity with an independent model reference.
3. Extend the tiled runtime through a separately bounded long-request
   profile, retaining two-bank retirement, exact completion and terminal
   failure behavior. The current tiled path executes only four forwards.
   The existing backend draft needs a distinct long parent/child wire and
   bounded capture selection: retaining every full control and payload would
   exceed 1.95 GB, outside the current 64 MiB finite-case cap.
   Test page transitions, multiple requests, KV lifetime and cleanup before
   treating it as sustained decode. Production runtime and arithmetic
   obligations remain separate from these engineering observations.
4. Qualify the full target workload: single-request Qwen3-8B, BF16,
   target-only decoding, 2,048 prompt tokens and 256 generated tokens. Report
   post-first-token throughput as `255 / (last_delivery - first_delivery)`.
5. Run equal-work baselines and paired optimization ablations. Host-inclusive
   diagnostics are not GPU overlap graphs or a qualified vLLM comparison.

The selected TP2 experiments do not satisfy the issue's original single-GPU
matrix. No M0-M7 milestone is closed by this checkpoint. Assurance properties
without closed, identity-bound proof remain `Contracted` or `Unsupported`, as
required by [the proof policy](PROOF_DEVELOPMENT.md).
