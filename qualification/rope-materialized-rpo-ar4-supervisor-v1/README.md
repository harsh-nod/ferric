# RoPE/RPO Four-Step Supervisor Qualification

All 65 CPU-only policy tests passed on `mi350`, with no failures, errors or
skips and unchanged before/after source snapshots. The bounded suite took
1.805 seconds. This qualifies the supervisor's synthetic checks, not a GPU
image or model result.

| Test Module | Passed | Coverage |
| --- | ---: | --- |
| `test_run.py` | 11 | Existing owned execution and terminal lifecycle |
| `test_decode_validation.py` | 17 | Own-output recurrence and capture structure |
| `test_intake.py` | 16 | Bound inputs and prior admission requirements |
| `test_silu_admission.py` | 8 | Preserved SiLU image admission |
| `test_rope_admission.py` | 13 | RoPE image, checked lowering and compiler-generation joins |

The additive supervisor retains the existing CPU1037 parent/worker and
bootstrap, copy, SiLU and projection images. Selecting a RoPE image requires
successful checked-lowering and owner records, exact artifact identity and
the qualified compiler/finalizer generation. The four-step input chain must
follow each preceding native argmax; independent framework comparison remains
a separate task. No older token-parity observation substitutes for that check.

The [actual receipt](pure/complete.json), [named test log](pure/tests.log),
[source manifest](source/manifest.json) and [publication result](result.json)
retain the exact tested inputs. The publisher rehashes these bodies without
importing tested modules or executing any model, compiler or GPU work.

No runtime audit, native GPU execution or numerical acceptance is established
by this suite. The subsequent RoPE/RPO lowering attempt failed at its
actual-inert-join test after producing a checked handoff; it did not qualify
a new image. Sustained 2,048/256 decoding, production readiness and the
700-token/s target remain open.
