# M1 Performance Report Evidence

`validate-performance-report.py` implements
`ferric.m1-validator.performance-report.v1`. The production evidence-index
checker owns that path, protocol, and validator source SHA-256. An evidence
index cannot select another executable.

## Canonical artifacts

For a performance artifact named `<artifact-id>`, the two files have these
fixed locations relative to the evidence-index directory:

```text
artifacts/<artifact-id>.performance-report.json
measurements/<artifact-id>.measurements.json
```

Both files are canonical, pretty-printed ASCII JSON with one trailing newline
and no duplicate, missing, or extra fields. The report authenticates the raw
measurement roster by path, size, and SHA-256. The evidence-index context in
turn authenticates the report. Every file and parent component must be a
non-symlink object with stable identity and metadata across the bounded read.

One report is bound to one exact still-`Open` M1 obligation/profile/path
binding. It repeats the requirements identity and binds the resolved path,
statement, ordered Ferric and fe2o3 commit/tree/source-closure roster, and all
three outer compiler/hardware/runtime TCB identities. It also binds the exact
`docs/PERFORMANCE.md` bytes and fixed target `gfx942:xnack-`.

## Qualification identity

The report and raw roster must agree on the named executable, generated plan,
schedule, dispatch graph, benchmark protocol, and baseline protocol, plus
immutable SHA-256 identities for those inputs, the Ferric artifact, model,
tokenizer, config, weights, and workload roster. The fixed baseline roster covers the vendor
kernel baseline, vLLM, SGLang, Ferric target-only, and the reviewed Ferric
regression reference. It authenticates each baseline identity, configuration,
and tuning budget. Ferric, the vendor, vLLM, and SGLang tuning budgets must
match.

The environment declaration binds the single MI300X device UUID, `gfx942` and
`xnack-`, ROCm, LLVM, driver, firmware, topology, clocks, power, thermal policy,
CPU, NUMA placement, affinity, and cache policy. The raw roster repeats its
canonical digest. These are checked declarations, not independent device
attestation.

The workload matrix is the exact M1 matrix in `docs/PERFORMANCE.md`: batch,
prefill, decode-KV, ISL/OSL, arrival, no prefix sharing, draft length, and
acceptance vocabulary. Every cell is inside that matrix and has a unique
workload digest. Each cell fixes its permitted primary metric and an equal p99
SLO that Ferric and every compared engine must meet, and binds its prompt
order, arrival trace, sampling seed, and output limits. A report contains at
least one cell for every core-kernel,
primary-serving, eligible-speculation, and low-acceptance gate class. The last
class additionally requires an already admitted deterministic plan.

## Recomputed gates

The validator trusts no report arithmetic. It recomputes from positive integer
measurements:

- at least ten ordered warmups and thirty ordered recorded samples;
- the exact cyclic engine order, with no missing, failed, or faulted sample;
- exactly three fresh serving starts with ten ordered windows each;
- exact rational medians and integer-parts-per-million ratios;
- a deterministic 2,048-resample paired-bootstrap 95% lower bound;
- primary-metric variance and thermal/clock drift from raw values;
- the faster of the pinned vLLM and SGLang baselines;
- the 95% core weighted-geometric-mean and 80% per-shape floors;
- the 5% general regression limit and serving lower bound of 0.95;
- the strict 1.05 lower bound for an explicit public-faster claim;
- the 10% speculation gain and 5% p99-latency limit; and
- the 5% low-acceptance regression limit.

The exact integer and bootstrap semantics are part of the versioned report and
are checker-owned constants. NaN, infinity, JSON floating-point arithmetic,
threshold changes, baseline swaps, summary substitutions, workload or
environment drift, sample loss, and cross-binding replay fail closed.

## Ferric intake producer

The planner exposes one intake command for each of the exact 36
`performance-gate` bindings:

```text
python3 -I proofs/m1-qualification/produce-performance-report.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR PERFORMANCE_INTAKE binding.NNNNN
```

`PERFORMANCE_INTAKE` is a canonical owner-private single-link file under an
owner-private `0700` parent outside the source repositories and plan root. It
contains the environment declaration and complete raw measurement roster.
Those values belong to the external harness; the Ferric producer does not
synthesize, default, repair, reorder, filter, or discard them. It recomputes
the fixed V1 summaries and release arithmetic, reconstructs the complete
current TCB declarations, and publishes exactly:

```text
measurements/<artifact-id>.measurements.json
artifacts/<artifact-id>.performance-report.json
```

The measurement roster publishes first and the report publishes last as the
completion marker. Both are held through post-sync revalidation, and a failed
transaction removes only exact producer-owned files while preserving and
reporting a rebound inode. The producer does not invoke or modify the trusted
validator; validate the published pair separately.

## Authority boundary

Acceptance authenticates checked performance only. It does not establish
semantic correctness, theorem truth, machine refinement, artifact loading,
queue publication, kernel launch, hardware correctness, or M1 qualification.
The validator creates neither an evidence index nor a qualification receipt,
changes no `Open` status, and closes no roadmap requirement, assurance
property, or path obligation. Real external measurement artifacts are still
required before M1 closure. Producer and validator acceptance authenticate the
declared samples and recomputed arithmetic; neither independently attests that
the samples were physically observed on the declared system.
