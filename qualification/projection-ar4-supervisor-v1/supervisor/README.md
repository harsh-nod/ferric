# Projection-Residual AR4 Observation Draft

Source-only successor of the actual SiLU TF4 supervisor package
`47afc73e3333ff0705002a16b1a296780d22c119676bba142f07c2eb9fe80c31`.
No imports, tests, builds, runtime audits or GPU runs were performed by the
author. Root owns qualification, manifest freezing and execution.

## Scope

The only model-execution change is autoregressive input recurrence on the
separately qualified AR4 runtime. Position zero consumes seed 9112; each of
positions 1, 2 and 3 must consume the preceding forward's actual finite,
lowest-index argmax. The seed and mode are bound into the base profile, which
is bound with the selected residual image into the candidate profile.

All four full 606,976-byte observations, 152 tensor slices, 576 typed terminal
states, per-forward requests, Control records, profile-bound transcript and
Close 5 remain checked. A self-consistent teacher-forced transcript is refused.
No token or tensor equality to a previous native run is required.

Selected images remain the V7 prefix `4885204c`, materialized-SiLU MLP
`b0d1766f`, projection-residual `25338bea`, and original Begin images including
the original residual-copy image. Host rotary generation is unchanged. This
package neither selects the unqualified RoPE candidate nor changes kernels.

`run.py` preserves the retained one-attempt process supervision: the same
owned parent and single worker, private groups, deadlines, disk and stream
limits, three before and three finally-after idle audits, natural Close/EOF
and reap checks, source rechecks and failure retention. Its native argv and
all resource limits are unchanged. The checked worker marker is now
`mode=Autoregressive`.

## Admission

`intake.py` replays the actual SiLU layer capture `67a23528` and independent
layer comparison `828f8fdb` without treating them as AR4 correctness. It keeps
the original V7/model/prompt/image prerequisite closure. New runtime admission
requires the AR4 CPU result schema and 1,037 named passing executions with
four retained ignored tests, 87 naturally reaped phases and 17 Cargo artifacts.
The actual CPU receipt `7a3c170c` records these results. They do not establish
GPU execution or model correctness.

The new CPU generation must descend from the actual CPU1022 receipt `1f436506`,
which in turn descends from the CPU988 layer generation. Rebuilt parent and
worker ELFs are checked against their own new Cargo artifact rows. They are
not equated to CPU988 or CPU1022 executables.

`JOINT_CPU` binds the actual successful `7a3c170c` receipt; the parent and worker
bind the rebuilt `22d2cb74` and `a3079acb` ELF bodies. The source controller,
proposal and input manifest are bound by their actual hashes. Fresh runtime
reviews must bind the newly qualified ELFs. Root must write a separately
reviewed plan and engineering review; this package never creates approval.

## Tests and Execution

There are 52 authored, unexecuted policy tests: 11 supervisor, 17 structural
validator, 16 intake and eight unchanged SiLU-admission tests. The prior 43
test behaviors are preserved, with mode-dependent expectations changed to AR4;
nine new tests cover seed, predecessor recurrence, zero/repeated outputs,
bootstrap mode/seed, old-runtime refusal and the CPU1022-to-CPU988 lineage.

The four inherited data helpers are exact byte copies. The included bounded
pure wrapper uses the prior MI350 CPU8/9, nice10, hidden-GPU environment and
2 GiB/120 CPU-second limits. After root binds the actual CPU records and
freezes the 14-member manifest, it takes `MANIFEST_SHA FRESH_PURE_LABEL`.
The supervisor still takes `PLAN_PATH PLAN_SHA`. No manifest is authored here.

Success proves only a bounded structural native AR4 observation and its own
input trajectory. Independent genuine-framework AR4 comparison remains a
separate gate. Numerical acceptance, full-model correctness, sustained
2048/256 execution, calibrated GPU timing, throughput and production authority
all remain false.
