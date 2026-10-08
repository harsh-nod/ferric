# Matched Native HTTP Benchmark

This is the retained down-only HTTP adapter successor, including the existing
native-width and gate/up selections. The explicit `Down1736` successor passed
316 remote CPU tests on mi300x-2. Its source-bound G56 receipt preserves the
four-core, 8 GiB RSS, nice 19, timeout and reserve checks. The README is not part
of the frozen Python source identity.

Run CPU fixtures on the assigned remote build host:

```sh
python3 -I -B -m unittest discover -s adapters/m1-engineering-execution-v1/tools/matched_native_http_v1 -p 'test_*.py' -v
```

`prepare_native_http.py`, `selected_native.py`, and `http_cpu.py` enforce exact
source, executable, image, CPU-evidence, and completed native-campaign bindings.
The historical native down-only a002 run was interrupted, and the gate/up a004
result was inconclusive. Neither admits a new HTTP comparison. `Down1736`
instead admits only the exact completed current-worker down campaign, replays
all fourteen cells and validates its worker-refresh CPU evidence. Historical
controller/kernel provenance is retained explicitly; the new worker is not
relabeled as the historical runtime. Do not manufacture a selection record or
rewrite historical hashes to bypass these checks.

The current comparison uses identical client/workload bytes, TP1/C1, 128 input
and 128 output tokens, context 8192, BF16 decoder and configured FP32 head,
greedy generation, and no prefix caching or speculation. Each configuration
has ten excluded warmups, thirty measured requests and two untimed numerical
diagnostics. Native-ingress changes do not establish an HTTP or vLLM gain.

The frozen HTTP client/server, scanner, active attribution, request shape, and
vendor settings are retained. Startup-only reconciliation now permits a fresh
complete scan after non-GPU helper membership changes while every GPU-owning
lifetime and both external GPU endpoint sets remain unchanged. Positive scans
must be complete and entirely owned; lost helpers need the existing dual
absence/pidfd proof. Refused samples remain refused, the three-sample shared
15-second bound is unchanged, and no active-run failure is retried. The old
pre-GPU-only observer attempts remain excluded; a revised comparison must rerun
all configurations with newly source-bound plans. This monitor repair is not a
performance optimization. No server starts as part of CPU fixture discovery.
See `docs/performance/MAIN_INTEGRATION.md` for publication and evidence scope.
