# TP1 Attention Fixture Checks

This additive profile checks the frozen v5 baseline and Wave64 paged-attention
roots using nonzero finite QK, token-varying V, permuted physical pages,
causal masks and untouched capacity tails. Eight predeclared dispatches compare
every byte with exact BF16 expectations. See [PLAN.md](PLAN.md) for the
arithmetic premise, image identity, limitations and lifecycle contract.

CPU-only validation, on an approved bounded remote stage:

```sh
FERRIC_ATTENTION_HELPER=/absolute/pinned/core/probe.py python3 -m unittest -v test_probe
python3 probe.py --self-test --helper /absolute/pinned/core/probe.py
```

Neither command runs a GPU. Native `--run --operational` requires explicit
worker, image, SHA256, device and output arguments and the separately reviewed
shared-host wrapper. Source presence or CPU success alone is not native
qualification. No model or performance claim is made by this profile.
