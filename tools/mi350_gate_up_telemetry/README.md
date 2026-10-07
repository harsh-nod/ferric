# MI350 Gate/Up Telemetry

This narrow diagnostic samples text sysfs attributes from physical GPU0 on the
qualified `mi350` host. It checks both PCI and GPU unique identities. It does
not open KFD/render devices, load a model, change clocks or set power limits.

Run CPU fixtures on the remote CPU build host, not locally:

```sh
python3 -I -B -m unittest discover -s tools/mi350_gate_up_telemetry -p 'test_*.py' -v
```

The CLI requires an existing owner-only mode0700 directory named
`/dev/shm/ferric-gate-up-telemetry-*`, with a fresh `samples.jsonl` destination.
Use the existing resource admission checks and an external process timeout:

```sh
timeout --signal=TERM --kill-after=3s 45s nice -n 19 \
  python3 -I -B gpu_telemetry.py \
  --output /dev/shm/ferric-gate-up-telemetry-a001/samples.jsonl \
  --samples 8 --interval-seconds 0.25
```

The sampler preserves raw labels and values, marks missing optional fields as
unavailable rather than zero, and caps each read, discovery, sample and output.
The requested schedule includes a two-second read budget per sample. Sleeps
occur between completed samples, so cadence is not an exact fixed frequency.
An external timeout is still required because blocking OS reads and scheduling
can exceed an internal deadline. Partial output is not a completed capture.

Samples are not atomic or per-kernel measurements. This does not decode
throttling state, prove cache residency, or establish causes from correlation.
Instrumentation has a cost; diagnostic campaigns must remain separate from
uninstrumented qualification and matched vendor comparisons.

Qualification: 20 tests passed on mi300x-2 under the existing bounded G42
profile. An eight-sample MI350 smoke capture passed without GPU execution or
settings changes. See `docs/performance/gate-up-telemetry-a001-checkpoint.json`.
