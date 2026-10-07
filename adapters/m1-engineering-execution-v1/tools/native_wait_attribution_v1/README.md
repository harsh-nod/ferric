# Native Wait Attribution

Diagnostic-only replay for fe2o3 `engineering-native-wait-diagnostics`.
It does not change native polling, authorize execution or admit latency samples.

`wait_observation.py` validates CPU signal-read brackets and reports conservative
completion bounds. `capture_binding.py` preserves the three original controller
stderr records for their independent existing replay, then binds the native
observations to controller execution frontiers and enclosing durations.

`run_wait.py` is a pinned, private-stage, one-request MI350 launcher. It requires
the retained qualified controller/parent inputs and fresh worker/capture CPU
receipts; it is not a general-purpose server or standalone public benchmark.
It reuses the existing resource, identity, token, isolation and lifecycle gates.
Keep restoration, launch and durable evidence retention in one SSH session on
hosts where logind removes user-owned shared-memory files at logout.

The exact supported geometry is four prefill32 programs of 649 dispatches,
three separately executed head commands, then 127 decode programs of 652
dispatches. Other workloads fail closed. A standalone parser result is not
native qualification: token replay, legacy counters, executable identities and
clean lifecycle must also pass.

Run the 28 CPU fixtures on the authorized build host:

```sh
python3 -I -B test_capture_binding.py
```

The bounded remote qualifier is `qualify_capture.py`. Builds and tests are not
run locally. See the October 7 performance report for results and limitations.
