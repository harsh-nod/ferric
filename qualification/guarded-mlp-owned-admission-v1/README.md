# Immutable Kernel Admission: CPU Opportunity

The fixed safe-loader experiment passed on `ssh mi350`. It measures repeated
HSACO validation and binding versus access to a retained owned loader closure.
It does not execute kernels, model tensors, dispatch fixups or currentness
checks. No production runtime admission policy changed.

## Result

Each sample performs 360 preparations: ten rank/role entries across 36 layers,
using the four original Prefix, Projection, MLP and Guarded HSACO files.
Two warmup pairs precede twelve measured pairs, alternating arm order six
times each. Cold creation of the ten owned closures took 0.680630 ms.

| Statistic | Fresh validation (ms) | Retained access (ms) |
| --- | ---: | ---: |
| Minimum | 16.795031 | 0.040950 |
| Median | 16.8091955 | 0.041175 |
| Maximum | 16.834860 | 0.041300 |

Median paired difference is 16.7679205 ms. These are twelve pairs inside one
invocation, not twelve independent process or model runs. Root calculated the
summaries on MI350 using integer nanoseconds and Decimal arithmetic.

| Pair | First arm | Fresh (ns) | Retained (ns) |
| --- | --- | ---: | ---: |
| 0 | Fresh | 16834860 | 41110 |
| 1 | Retained | 16799950 | 41110 |
| 2 | Fresh | 16795031 | 41190 |
| 3 | Retained | 16814240 | 41140 |
| 4 | Fresh | 16808191 | 40960 |
| 5 | Retained | 16813760 | 40950 |
| 6 | Fresh | 16814471 | 41190 |
| 7 | Retained | 16819410 | 41160 |
| 8 | Fresh | 16809111 | 41280 |
| 9 | Retained | 16809280 | 41270 |
| 10 | Fresh | 16801210 | 41300 |
| 11 | Retained | 16804681 | 41240 |

The [raw report](cpu-v2/evidence/benchmark.stdout) records every warmup,
sample, input identity and nonclaim. Before and after timing, both paths
compare exact plans, selected metadata, resources, bindings, descriptor/entry
bytes, identity and relocation evidence. Both arms consume the same results
and produce the same 1,406,664 digest-byte sum per sample. The retained arm
includes the loader's owned-access/Arc costs; it is not Context's existing
direct inspected-metadata branch.

## Engineering Decision

The roughly 16.8 ms admission batch is small beside the seconds-scale warm
host waits in the [earlier native Tail experiment](../guarded-mlp-scoped-tail-v1/matched-timing-report-v2/README.md).
Those are different experiments: subtracting these CPU timings from old
forward waits would not establish an end-to-end saving or bottleneck share.
The retained-admission runtime change is deferred while the larger currentness
costs are measured. A bounded own-process perf probe was denied on MI350;
shared host settings were not changed. Private duration instrumentation is the
next diagnostic, not an implemented or measured optimization here.

Admission must eventually leave the decode hot path: the 700 tokens/s target
allows about 1.43 ms per generated token. This microbenchmark does not show
that removing admission achieves that budget, resolves numerical differences,
admits Full2303 or improves GPU utilization.

## Qualification And History

The [successful retry](cpu-v2/README.md) passed ten phases and ten tests, with
all source, tool, private-cache, dependency and product postchecks clean.
Independent data-only review authenticated both original capsules, all raw
phase/test records, source joins and the integer timing summaries. The three
Rust formatter diffs were inspected and contain no semantic changes.
The [original failed attempt](cpu-failed-v1/README.md) remains intact and
contains no benchmark samples. Only the harness's incorrect registry checksum
assumption and fresh namespace changed between attempts; Rust benchmark code,
images, phase order and bounds did not.

This report is CPU opportunity evidence only. Native Full2303 has not run;
exact independent 256 IDs/raw decoded bytes, sustained BF16 target-only
Qwen3-8B 2048/256 decode, GPU overlap and 700 tokens/s remain open.
