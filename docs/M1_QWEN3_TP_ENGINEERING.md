# Qwen3 Tensor-Parallel Engineering Execution

This is an explicitly opted-in engineering command, not the protected M1
server or a qualification receipt. It runs one Qwen3-8B BF16 model across
one, two, or eight physical GPUs. It does not run independent model replicas.

## Implementation

- Ferric owns the inference driver and the thirteen-root kernel crate at
  `device/qwen3-tp-kernels-v1`. All kernels are compiled with fe2o3.
- Each GPU has one isolated fe2o3 KFD worker process. The parent validates
  physical device identities, live worker executable hashes, artifact bytes,
  inspected argument layouts, transfer extents, and bounded protocol frames.
- Q/K/V and gate/up projections use rank-local output shards. O/down use
  rank-local input shards and FP32 partial outputs. The parent reduces in
  rank order, adds the residual once, and rounds to BF16 once.
- Every rank is submitted before waiting for completions. A partial failure
  poisons the sequence; reset cannot make a failed engine usable again.
- RoPE, contiguous KV storage, and causal grouped-query attention are local
  to each rank. Rank zero performs embedding, final normalization, LM head,
  and lowest-ID greedy argmax.
- Prompt priming processes one token at a time. There is no prefix-cache
  reuse, symmetric-memory collective, MTP, batching, HTTP API, or graph replay
  in this profile.

The sequence cursor has a same-source Verus proof of its bounded host state
transitions. That proof does not establish transport completion, GPU memory
correctness, floating-point equivalence, or correctness of the whole driver.
Numerics remain Contracted and are checked separately against retained data.

## Reproduce The Current Observation

Builds and host tests run only on `mi300x`. Execution runs on `mi350`, after
checking that the shared GPUs are idle. Do not start another GPU job while
this one is active, and do not terminate another user's processes.

The current private runtime bundle is `mi350:/tmp/ferric-qwen8.TRNKht`.
It contains the authenticated source-model files, release controller, release
worker, exact engineering artifact, and a bounded launch helper. The helper
uses the recorded physical unique IDs, rejects an existing output directory,
records before/after GPU observations, and imposes a 1,800-second deadline.

Run a short eight-GPU correctness check with a new output name:

```sh
ssh mi350 bash /tmp/ferric-qwen8.TRNKht/ferric-qwen8-run.sh \
  8 /tmp/ferric-qwen8.TRNKht/artifact/fe2o3-engineering-v1/994c81cde99c63de824ef53d29b2878e97d9a02d71901815039048da58c3c83c \
  tp8-user-smoke-001 2 1 0
```

The final three arguments are generated-token count, measured-run count, and
warmup-run count. Use `32 1 0` for the single-sequence extended observation.
At the current baseline speed, four 32-token runs exceed the helper deadline;
do not describe the short or single-sequence profile as a repeated benchmark.

The controller's direct interface is:

```text
ferric-qwen3-tp-engineering --source DIR --artifact DIR --worker FILE \
  --devices ID[,ID...] --allow-unauthenticated-machine-code \
  [--prompt TEXT] [--new-tokens 32] [--repetitions 3] [--warmup 1] [--capacity 128]
```

The explicit machine-code opt-in is mandatory. This interface cannot publish
or convert its observations into protected execution authority.

## Recorded Baseline

The September 9 engineering image and runtime use public fe2o3
`3546d54d2c4a913f5d079701aed557d0a378bba8`. The kernel source is `8cdde149`,
and the controller was built from the integration through `60cedea`.
The later probe-harness correction does not change either executable.

| Identity | SHA-256 |
| --- | --- |
| Release controller | `6b49356f4abeed632ef7a5416122d16f234b535a32b43edbe45f5372d2fc6e46` |
| Release worker | `77a53d18b56e4ee7a67a434feffa8ac18a4fe60f8c9e5daace351f502c72e1da` |
| Thirteen-root gfx950 HSACO | `7c0b1934a27569a97cf535c96a8a56dd57babb63a6d1becad1c7be3a3d26edec` |

All three two-token smoke runs completed the five-token prompt
`The capital of France is` and generated IDs `[12095, 13]`, decoding to
` Paris.`. These match both retained reference prefixes. Eight GPUs means
eight distinct physical IDs and eight worker PIDs, not eight replicas.

| Tensor Parallelism | TTFT (s) | Sole Decode Interval (s) | Setup (s) |
| --- | ---: | ---: | ---: |
| 1 | 23.216413234 | 4.636949036 | 154.806338899 |
| 2 | 75.114170435 | 14.674596102 | 162.321828177 |
| 8 | 160.644607963 | 24.711778123 | 186.586377561 |

Each smoke has only one measured sequence and one post-first-token interval.
These are not steady-state TPOT distributions or controlled scaling results.
Setup is excluded from TTFT. Timings include IPC, host collectives, and
per-token progress logging; they are not HTTP request timings.

The eight-rank smoke executes 3,264 dispatches on rank zero and 3,240 on every
other rank, advances the KV cursor to six, and closes/reaps all eight workers.
GPU observations are idle before and after.

The extended eight-GPU run completed on September 9 local time (September 10
UTC). All 32 generated IDs and decoded bytes match both frozen reference
passes and both reference argmax arrays. The output is:

```text
 Paris. The capital of Italy is Rome. The capital of Spain is Madrid. The capital of Germany is Berlin. The capital of the Netherlands is Amsterdam. The
```

| Single Unwarmed TP8 Sequence | Seconds |
| --- | ---: |
| TTFT | 163.647853495 |
| Mean post-first interval, 31 intervals | 40.25950984674194 |
| Setup, excluded from TTFT | 193.519502061 |
| Generation, including first token | 1411.692658744 |
| Whole process, including setup and teardown | 1619.848894496 |

This is one prompt and one sequence, not repeated-run statistics. The run
executes 19,584 dispatches on rank zero and 19,440 on each of seven peers,
advances the KV cursor to 36, and closes/reaps all eight workers. The process
status is zero; all recorded worker PIDs are absent afterward, and before/after
GPU observations are idle. The strict single-sequence comparator independently
checks all three expected image hashes, token/reference agreement, timing
arithmetic, rank counts, and the final close roster.

Raw JSONL SHA-256 is
`d69dc3e61c12a8663c28f1af0451e8e382244620b158249adfbf8bc54e9ebe63`;
comparison report SHA-256 is
`a2474d0a29355c2120977d6b34b1418f309b0236cf0a2947f192dc2db12b816c`.

The thirteen-root image separately passes exact replay and six real GPU
numerical probes with input immutability, unchanged 64-byte allocation guards,
and clean teardown. These probes are not model-inference evidence.

## Performance Limitations

The implementation currently scales poorly. A read-only five-second TP8
sample observed 31.25 worker CPU-seconds in kernel mode versus 1.17 in user
mode, and approximately 232,000 read syscalls per second across workers.
Repeated full topology/currentness checks in the engineering runtime are the
leading bottleneck hypothesis, not an isolated causal measurement.

Column GEMV also loses active waves as ranks increase without shortening each
serial dot product, and the LM head stays on rank zero. Host-staged collectives
are intentionally simple. No vLLM/SGLang performance comparison, speedup,
production readiness, or protected M1 gate closure follows from these results.

The local evidence directory is
`/home/harsh/.codex-tmp/ferric-qwen8-evidence-v1`. It retains raw JSONL, status,
progress logs, GPU observations, frozen reference, and separate strict
comparison profiles. Completed build stages are removed after archival;
the runtime bundle is retained because it is needed to reproduce execution.
