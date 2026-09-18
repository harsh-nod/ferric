# Native Two-GPU Dependency Canary

A bounded native canary passed on two `gfx950` GPUs after two retained rejected
attempts. This validates a specific cross-device AQL dependency and numerical
result, **not tensor-parallel model execution, dependency latency, GPU overlap,
or the 700 tokens/s target**. It is separate from the
[TP1 model packet profile](GFX950_TARGET8B_GPU_PROFILE_V1.md) and
[single-request performance observations](GFX950_TARGET8B_FEATURE_ABLATIONS_V1.md).

## Accepted Primitive

The producer dispatch was staged with an invalid header. On the consumer queue,
a witness completed while the producer remained unpublished; the dependent
barrier and consumer completion signals remained incomplete through a deliberate
hold of at least 25 ms. The producer was then published. Completion required
all four signals to acquire-read zero **and both actual queue read/write
frontiers to drain**, not signals alone. A second generation repeated the
protocol with different data to detect stale results.

| Generation | BF16 words checked | Producer read/write | Consumer read/write | Completion signals |
| --- | ---: | --- | --- | --- |
| 1 | 8,192 | 1 / 1 | 3 / 3 | All four zero |
| 2 | 8,192 | 2 / 2 | 6 / 6 | All four zero |

Each generation compares every word of a 4,096-word producer output and a
4,096-word consumer output in the native process. An independent CPU reference
checks the expected full-vector hashes in the receipt. **Actual output vectors
were not retained externally; this is not raw-output replay.** Both generations,
native peer-signal unmapping, and all seven postflight statuses passed:
`native`, `settlement`, `idle`, `mapping`, `inputs`, `tools`, and `stderr`.

The kernel is the unchanged `ferric_qwen3_tp_batch_residual_bf16_v3`: Wave64,
312-byte kernarg (56 explicit plus 256 hidden), 8-byte alignment, no LDS or
private storage. The [actual ABI receipt](assets/native-tp2-dependency-v1/residual-abi.json)
is retained unchanged. The entry point explicitly permits unauthenticated machine
code and performs its closed native ABI checks; it is an expert engineering
canary, not production or proof-authorized admission.

## Both Rejected Attempts

| Attempt | Retained outcome |
| --- | --- |
| First | Dependency drain deadline; no native JSON receipt. Generation reached, signals, frontiers, and output verification are unknown. |
| Second | Generation 1 drain deadline. Last observed signals were all zero, but producer write/read was 1/0 and consumer write/read was 3/0. Output verification was not reached. |
| Third | Both generations passed the unchanged drain and numerical acceptance rules. |

The second attempt's diagnostic is a last observation, not an independently
captured terminal snapshot. Neither failure is discarded, upgraded to success,
or assigned an accepted performance result. The observations do not establish
the root cause of the earlier frontier failure.
The sanitized [first audit](assets/native-tp2-dependency-v1/rejected-attempt-1.json)
and [second audit](assets/native-tp2-dependency-v1/rejected-attempt-2.json) remain
separate from the accepted result.

## Isolated Allocation Experiment

Only the dedicated canary selects the new queue-control allocation policy:

| Policy | Backing | UNCACHED | Flags |
| --- | --- | --- | --- |
| Earlier attempts | USERPTR | No | `0x84000004` |
| Accepted third attempt | GTT | Yes | `0x86000002` |

This changes **both backing and UNCACHED**, not one cache bit. Coherent/writable
properties, the 4,096-byte control allocation, pointer offsets `0x38`/`0x80`,
queue layout, packet headers, kernel bytes, arithmetic, and drain predicate
remain unchanged. Production and timestamp constructors retain their original
policy. Allocation-policy disclosure is bound to frozen source and binary;
there is no independent runtime allocation receipt or cache-bit causal claim.

The source-v6 to source-v7 change only fixes a whitespace-sensitive test
assertion. The production allocation-policy implementation is unchanged
between those snapshots. Fourteen CPU harness tests independently passed.

The published source-v8 differs from measured source-v7 only by merging two
adjacent `lib.rs` re-export lines. All other ten files are byte-identical.
The native binary and test results below retain their source-v7 identities;
formatting is not a rebuilt-binary equivalence claim.

## Timing Limits

The unchanged bounds are a 15-second outer native timeout, a 2-second deadline
per generation, at least 25 ms of intentional pre-producer hold, and one fixed
5-second postflight settlement followed by one unchanged idle gate. There are
no automatic native or idle-gate retries.

Observed host witness timings were about 6.05 ms, including host work before
observing witness completion. Holds were about 31.03 ms, including the deliberate
25 ms hold and surrounding validation. Subtracting the hold from protocol elapsed
time would not isolate dependency latency.
These are host-monotonic protocol observations, not GPU timestamps or isolated
dependency latency. They include deliberate holding, host polling, and
validation. No throughput, stable speedup, model TP2 correctness, overlap plot,
or bandwidth conclusion is derived from this canary.

## Frozen Evidence

The [sanitized summary](assets/native-tp2-dependency-v1/summary.json) retains the
closed scalar acceptance facts and provenance hashes, with no private paths,
process/device identifiers, or output vectors. Its
[SHA-256 manifest](assets/native-tp2-dependency-v1/SHA256SUMS) binds the public
copy. The independent audit checked the original run and all retained files;
sanitization does not turn native in-process comparisons into raw-output replay.
The [audit provenance](assets/native-tp2-dependency-v1/provenance.json) also pins
the two earlier rejected captures and their separate reviews. The independent
audit passed seven mutation tests; the frozen original validator separately
reproduced the exact accepted receipt.

| Input or receipt | SHA-256 |
| --- | --- |
| Native binary | `c718411d348c99827f0634eee5f3bfe8fbf835c136e0ac686ee6a281db48ba97` |
| Source-v7 manifest | `cf31cf0390576cd3dc018c0dbcd98585b0445144de96b58ba2a7cc7f8368e525` |
| Native HSACO | `6b0889e2834bda81313b3cbda5a60207dd9fa41c40ea80251bbec89d7befef2c` |
| Actual residual ABI/admission receipt | `e1a4587222f87afc2b4400109bbd1bc7713e6bf2e08fa47a934cb4acb27d807d` |
| Closed ABI/receipt validator | `80de7ff29bd68fc8246a6b30b87f8e7b23c377b22d2f1d8b8349f47a65862cb5` |
| Predeclared plan | `cea5bfab1c898d991a5582ddba7e62bb0efc798049795df912ace3bb23ed301a` |
| Native wrapper | `9ca02d3e1b54f67da85815d4d1b817740bccd04638f8a7c596244e95d4208cd5` |
| Accepted run manifest | `d190a52daa0c3b7c40925bf8c0fa6c59175744756a9157d006b8f133cd59685b` |
| Accepted native report | `f4623ff1467fd629a1dfaa96b842b17662448e47c7f70914db1787fc4710a462` |
| Independent audit / public summary | `6429ee279c2d7edaa1931af756bc0809448a02f8e6c8bff9c1ff98d9775edccf` |
| Independent audit provenance | `812b502748df3b657174765474c814237c2ce79fc3a70e0374ebc51a4d9a61fb` |
