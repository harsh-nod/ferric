# Ferric Performance Integration

This integration publishes the previously separate engineering branch and its
local kernel, inference, benchmark, proof, and site sources. It does not promote
every experiment to the default execution path or change historical evidence.
Core compiler and KFD implementations remain in fe2o3.

## Execution Paths

- `adapters/m1-engineering-execution-v1` contains the TP inference engine,
  scheduler, paged KV and prefix-cache integration, native token programs,
  prefill-width experiments, and diagnostics accumulated on the engineering
  branch. Features and dedicated binaries remain explicit opt-ins.
- The native prefill-width worker includes the separately qualified down-only
  split-K8 candidate. `--native-down control` preserves the control path;
  `--native-down splitk8` requires the exact candidate artifact bindings.
  Prefill is unchanged by this decode experiment.
- The gate/up native experiment is preserved separately under
  `experiments/native-gate-up-a009`. Its worker and the down-only worker were
  qualified separately. They are not a qualified combined optimization.
- Device kernel source remains under `device/`. No model checkpoint, generated
  worker executable, or newly generated HSACO is included in this integration.

## Measurements And Qualification

The down-only a002 run completed two of three planned ABBA blocks before an
unrelated GPU process appeared. Its descriptive medians were 50.0638 to
46.51571 ms TPOT (7.1% lower) and 415.1947125 to 426.7407945 ms TTFT (2.8%
higher). All 6,400 completed output token IDs matched. The incomplete run is
not a qualified speedup, HTTP admission, or matched vLLM win.

The gate/up a004 run completed but improved only four of six adjacent pairs,
below its declared acceptance gate. It remains inconclusive and default-off.
See the dated JSON checkpoints and independent reviews in this directory for
exact source identities, failures, counter scope, and retained evidence hashes.

The native worker retains its tested fe2o3 `da6b561c5` runtime dependency.
Publication is not a claim that this pin equals today's entire fe2o3 `main`.
Later upstream runtime changes require a separate compatibility qualification;
historical evidence must not be relabeled by silently changing the dependency.

## Benchmark Sources

The engineering adapter's `tools/` directory contains:

- `whole_token_benchmark_v1`: the earlier whole-token benchmark.
- `native_gate_up_benchmark_v1`: the qualified gate/up campaign harness.
- `native_down_benchmark_v1`: its separately qualified down-only successor.
- `matched_native_http_v1`: the 302-test HTTP adapter supporting native width,
  gate/up, and down-only campaign selection.

These harnesses deliberately retain exact artifact and CPU-evidence bindings.
Their historical machine paths are provenance and preconditions, not portable
defaults. Native CPU-binding tests require the retained evidence archives;
they are not a self-contained clean-checkout test suite. HTTP selection still
requires a complete passing native campaign. Publishing the scripts does not
open the down-only or gate/up HTTP admission gates.

Builds and tests run remotely on `mi300x-2`, not locally or in GitHub-hosted CI.
Integration commits use `[skip ci]`. Static site source is included; publishing
a new Pages artifact remains a separate remote-validated deployment.
