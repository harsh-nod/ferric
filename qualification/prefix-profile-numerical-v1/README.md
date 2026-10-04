# Independent Prefix-Profile Checks

This CPU-only comparator checks each prefix implementation against independent
operator references. It no longer requires the baseline and candidate to have
identical bits before their numerical correctness can be examined. It does
not change Ferric's runtime admission or launch a GPU kernel.

## Executed Checks

On `mi350-2`, Python 3.10.12 and NumPy 2.2.6:

| Check | Result |
| --- | --- |
| Synthetic reference, routing and refusal tests | 20 passed; none skipped |
| Retained historical positions 15 and 16 | Four independent profile checks; eight rank rows passed |
| Norm and QKV checks | Zero bound violations |
| Current KV value append | Exact in every row |
| Attention | All 16,384 BF16 words equal the rounded independent reference |
| O projection | Largest observed error/bound ratio: 0.011039 or less |

The [CPU result](cpu-result.json), [test log](cpu-tests.log) and
[historical replay result](historical-replay-result.json) retain measured
outcomes. Source pins cover all 30 comparator/reference files, unchanged before
and after the tests. Replay rechecked 55 input identities. A first replay
stopped on missing cross-host requests; their original hash-verified bytes
were copied to isolated replicas before a fresh successful run.

The synthetic history test crosses from logical token 15, physical slot 127,
to token 16, slot 192. It uses nonzero historical KV and an analytically known
attention result. A finite prior-value corruption, with an updated capture
hash, must fail the unchanged attention bound. Other tests reject wrong input
identities, nonfinite outputs, aliasing/truncated captures and non-integer
request positions. Mocked tests establish routing/refusal behavior only.

## Reproduce Synthetic Tests

With NumPy `2.2.6` installed, run from the Ferric repository root:

```sh
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 -B -m unittest discover \
  -s qualification/prefix-profile-numerical-v1/p228-prefix-profile-numerical-v1 \
  -p test_compare_profile.py -v
```

The [comparator API](p228-prefix-profile-numerical-v1/README.md) documents the
closed input schema and required host checks. Its unchanged reference packages
are included here. Their historical source metadata is not a review of a new
compiled image. The retained-host drivers record the executed commands' logic;
historical replay also requires model inputs and captured buffers not included
in this source package.

## Qualification Boundary

Each operator is checked conditionally on its captured preceding stage. The
historical captures are from older images, not the exact-reciprocal candidate,
whose checked lowering remains incomplete. Replay is not a new GPU run.

Full model correctness, historical KV origin, untouched KV, GPU provenance,
owned process lifecycle and artifact arithmetic/ISA prerequisites require
separate checks. Numerical tolerances were not increased. No runtime admission,
production readiness, performance improvement or 700 tokens/s result is claimed.
