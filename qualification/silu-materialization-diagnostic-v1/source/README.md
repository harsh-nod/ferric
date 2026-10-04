# Captured SiLU Materialization Diagnostic

Draft source only: 18 authored tests, not executed by the author. The root owns
qualification and the actual retained-data invocation. No source or runtime
kernel is changed by this package.

The actual checked Down2 source computes FP32 SiLU, multiplies it by BF16-up
expanded to FP32, and narrows only that product. Its retained source digest is
`2e0edf9efdc1caf19ea574ba9dfeffa375072584d27e21a31308539a6b8115d1`;
checked LLVM `d6001c014f8160b1355008742f290fdd58f78c0f12a039aa6d577bbb82cad6fb`
shows OCML exp, two FP32 multiplications, then one BF16 conversion. The selected
image is `65a76f917c12476f4814174ee39d515642a643a0352c6609ee514955a6a97449`.
The genuine eager framework instead captured a BF16 SiLU result before its
BF16 multiplication by up. The frozen P218 fused SwiGLU oracle is not used.

## Calculation

`diagnostic.py` multiplies exact integer significands and rounds once to IEEE
binary32, nearest/ties-even with gradual underflow and signed zero. It then
uses the unchanged independent residual helper's BF16 narrowing function.
Nonfinite inputs, FP32 overflow, and BF16 overflow are explicit failures.
The existing helper is loaded by its exact 4347-byte SHA
`551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3`.

The real-data runner checks the complete genuine 33-stage/two-repeat capture
using the existing reader, including native dtype rather than a cast. It checks
gate equals SiLU-input and requires all 12288 captured framework products to
equal `BF16(FP32(widen(captured SiLU) * widen(captured up)))` exactly.

For each native TP rank it partitions all 6144 gate/up pairs by bitwise equality
to the corresponding genuine framework inputs. Only identical-gate rows get
a prediction: genuine captured BF16 SiLU times that row's actual native up,
with the same exact two rounding operations. Mismatch indices are retained.
For identical gate and up this prediction must also join the genuine product.
Different-gate rows are excluded, not mislabeled as exponential error.

This proves an exact product control and measures conditional differences. It
does not observe native FP32 SiLU, evaluate exp, reproduce OCML, isolate the
missing boundary as the sole cause, or accept full-model numerical behavior.
Every acceptance, performance, production, and GPU-execution flag remains false.

## Inputs And Execution

Run on ASROCK, where the actual framework blobs and the comparison's 64 explicit
transport mappings already exist. No new transport map, GPU process, model
loading, or compiler execution is needed. Preserve the original E paths:

```sh
E=/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220
env -u PYTHONOPTIMIZE -u PYTHONPATH -u PYTHONHOME \
  HIP_VISIBLE_DEVICES= ROCR_VISIBLE_DEVICES= CUDA_VISIBLE_DEVICES= \
  taskset -c 8,9 nice -n 10 /usr/bin/python3 -B \
  "$E/p228-silu-materialization-diagnostic-v1/run.py" \
  "$E/projection-residual-comparison-v228-v1/complete.json" \
  affe711d0dcac8e95609396c74e0f546ca09d9d6a4abf798eab5bfc72a7e7551 \
  "$E/silu-materialization-diagnostic-v228-v1"
```

The actual comparison receipt is the immutable prior ownership checkpoint:
candidate and baseline seven owned leaves/six audits plus genuine framework
21 linked results were already authenticated there. This small runner does not
replay those ownership trees or perform new platform checks; it records that
limitation explicitly. It rehashes all data actually consumed here and pins
the original comparison, selected Down2 source/image/LLVM, source snapshots,
genuine installed implementation sources, helpers, and its own four files.
The old reader module is pinned to
`9c3db491d4f8c20890b045474bd1ab5bd8565d42916a8eb7772c0eb07c44717e`.
Limits are at most 2 GiB address space, 120 CPU seconds, and 16 MiB output;
the explicit command restricts affinity to two cores and nice to 10.

The root can run the pure suite separately, preserving its ordinary transcript:

```sh
cd "$E/p228-silu-materialization-diagnostic-v1"
env -u PYTHONOPTIMIZE -u PYTHONPATH -u PYTHONHOME \
  HIP_VISIBLE_DEVICES= ROCR_VISIBLE_DEVICES= CUDA_VISIBLE_DEVICES= \
  SILU_RESIDUAL_ORACLE="$E/p228-independent-layer-reference-v1/helpers/residual_oracle.py" \
  taskset -c 8,9 nice -n 10 /usr/bin/python3 -B -m unittest -v test_diagnostic
```

Tests include independent Fraction nearest-neighbor comparisons over 256
deterministic FP32 products, signed zero, ties at both precision boundaries,
normal/subnormal transitions, overflow and nonfinite rejection, real-width
partitioning, wrong-stage sensitivity, and explicit diagnostic nonclaims.
They are synthetic arithmetic tests, not a genuine model execution.
