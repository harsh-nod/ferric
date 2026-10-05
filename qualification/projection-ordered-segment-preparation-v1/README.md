# Ordered Segment Qualification Preparation

The next engineering experiment combines projection residual and MLP into two
ordered packets on each of two GPU queues. Both rank pairs must be published
before polling. This directory publishes the CPU qualification driver and its
actual synthetic policy-test observation, not an implemented or GPU-qualified
optimization.

## Actual Result

On the ASROCK host reached through `ssh mi350-2`, all 15 controller policy tests
passed with exit code zero. The controller, tests and scope-document SHA-256
values matched before and after execution. The primary observation records the
tool result and execution bounds: CPU affinity 8/9, nice 10, 512 MiB address
space, 60 CPU seconds, 120 wall seconds, disabled core dumps and hidden GPUs.

- [Primary observation](primary-observation.json)
- [Policy tests](test_run.py)
- [Qualification driver](run.py)
- [Controller scope and required future coverage](controller-scope.md)

The tests exercise closed input schemas, source ownership, content pins,
additive test inventories and Cargo-target restrictions. They do not invoke the
driver's compilation path. No Rust build, GPU execution, numerical acceptance,
throughput gain or milestone completion is established by this checkpoint.

## Next Gate

The driver requires separately reviewed runtime and Ferric source manifests,
exact source preimages and a root-selected input plan. It then builds a fresh
paired source copy on ASROCK, selects the complete runtime library suite and
retains the prior 883 selected worker/parent tests plus new tests. Seven
historically ignored tests must remain explicitly ignored. Five production
executables and all source, dependency and tool postchecks are required.

The planned GPU comparison needs distinct Down-projection scratch on each rank:
the existing output-projection scratch cannot be overwritten while the peer
still reads it. New combined-stage host timing must not be represented as
individual GPU-kernel timing. Actual GPU validation and the sustained
2,048-token-prompt/256-generated-token, BF16 target-only 700 tokens/s target
remain open.
