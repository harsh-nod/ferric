# V7 All-Layer Teacher-Forced Observation

On 2026-10-04, the V7 prefix image completed four teacher-forced forwards through
all 36 Qwen3-8B layers on `mi350`, with BF16 weights and TP2. There was one native
attempt and no retry. All 152 tensor captures, typed Close, owned-process reaping
and three pre-/three post-device audits passed their structural checks.

This is not whole-model numerical acceptance or the sustained 2,048/256 target.
The run consumes only prompt positions 0 through 3, despite the original prompt
file containing 2,048 tokens. Main and production admission remain unchanged.

## Independent Numerical Diagnostics

The [separate CPU diagnostic](framework-diagnostic.json) compared all 152 rows
against the mode-specific independent framework reference after GPU teardown.
No new tolerance was chosen. All four argmax tokens agree, but every complete
tensor row differs in at least one BF16 word.

| Position | Reference / candidate token | Logit relative L2 | Maximum absolute logit error |
| --- | --- | ---: | ---: |
| 0 | 67 / 67 | 0.00471959 | 0.07421875 |
| 1 | 198 / 198 | 0.01326323 | 0.25 |
| 2 | 25 / 25 | 0.00438509 | 0.125 |
| 3 | 16 / 16 | 0.00491419 | 0.125 |

The range is approximately 0.44%-1.33% relative L2. Matching selected tokens
does not establish an error bound for all logits, cumulative layer correctness
or correctness on other inputs. MLP and final-normalization arithmetic still
need their own independently justified checks.

A separate post-run byte comparison also found all four complete 606,976-byte
payloads identical to the retained older native route. The source image change
therefore did not alter these four payloads; the framework differences also
exist in that historical native route. This historical equality was not an
admission gate and is not used as the numerical oracle.

## Runtime And Evidence

The selected parent is CPU633 `830f90b1...`, worker CPU475 `3a16059a...`, and
new image `4885204c...`. Host policy remains `shared-full-currentness`.
The original setup, residual, MLP and tail images were not changed. The exact
[request](request.json), [plan](plan.json) and [review](decode-review.json) bind
the two separately verified deployments and six standalone prerequisites.

Forward host durations were 7.2200, 7.2450, 7.8164 and 7.7490 seconds. These are
instrumented host-inclusive diagnostics, not GPU timings or steady-state token
throughput. They do not demonstrate a speedup or approach the 700 tokens/s target.

The [public result](result.json) joins the [actual GPU completion](gpu-complete.json),
[structural observation](observation.json), [host sidecar](host-observation.json)
and framework diagnostic. Root rehashed all 58 case files, totaling 4,594,474
bytes, including all complete native buffers. The seven owned leaves exited
naturally without forced cleanup. Full buffers remain in retained evidence,
outside Git. Source snapshots for the diagnostic are included beside its result.

The first CPU-only input preflight rejected a root serialization that rounded
64-bit device IDs. The corrected request uses integer-preserving JSON parsing;
the rejected request is retained and never launched GPU work. No validator,
resource cap, numerical threshold or compiler restriction was loosened.
