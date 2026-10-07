# Selected Down Exact-Error Results

The fresh V2 diagnostic completed on MI350 in 5.910 seconds. All 48 named tests
passed, with no errors, failures, skips or postcheck errors. The preceding
[failed fixture attempt](../position5-down-dot-v1/RESULTS.md) remains retained.
Only its expected container type was corrected; production arithmetic and all
test names and expected values are unchanged.

## Selected Findings

At layer zero, prompt position five, the diagnostic evaluates the 14 originally
differing final-hidden rows and a predetermined matching row-zero control.
It computes 60 exact rank dots: 15 rows, two ranks and two independently
captured input vectors. Each rank dot uses 6,144 original BF16 weights and
products, accumulated as exact integers in units of `2^-266`.

| Reported check | Result |
| --- | ---: |
| Native projection matches exact own-input dot rounded to BF16 | 15 / 15 |
| Framework projection matches exact own-input dot rounded to BF16 | 15 / 15 |
| Exact own-input ideal projections agree across sides | 1 / 15, only the control |
| Ideal projection plus common residual agrees across sides | 1 / 15, only the control |
| Exact error decomposition reconciles the observed final difference | 15 / 15 |

All 14 selected differences remain in the idealized own-input Down calculation
and subsequent common-residual boundary. Thus, replacing Down with those exact
once-rounded projections would not remove these selected differences: the
inputs already differ. This is a selected-row counterfactual, not a model
execution, universal Down acceptance or an explanation of the final argmax.
It does not justify forcing framework equality or changing kernel arithmetic.

The seven original differing product coordinates are retained individually.
For every selected row, their sparse weighted deltas sum exactly to the
difference between the two independently computed own-input dot totals.
The result separates native rank accumulation error, TP sum rounding, BF16
projection rounding, framework whole-projection error and both residual-boundary
errors. It does not invent framework rank accumulators or emulate either
machine's accumulation tree. Reported integer identities reconcile exactly;
no fitted numerical threshold is introduced.

## Evidence

The original 3,996,250,744-byte checkpoint shard and both 50,331,648-byte Down
column partitions were authenticated before and after the calculation. Their
partition hashes match the original native upload commitments. All 30 selected
weight halves are retained (368,640 bytes). The original R2 row ledger and
derived arrays are reused without rerunning the complete R2 vector analysis.

The [original terminal](output/complete.json) is 124,115 bytes, SHA-256
`dd7d032030edba63b0431ce24ac7cf24d05ef51cbde19c87c6b388b841cf664d`.
The test stream is 6,811 bytes, SHA-256
`1584f2f89034956df9ae615fa27d00abc8374f082674bba257c595adc9d33d3b`.
The archive is 4,231,160 bytes, SHA-256
`6281a5041014ca87ceee0d0babf519a2c7a8e6119ab05e9cf0621d5050202aea`.
All 37 archive originals are retained; the manifest pins the other 36 bodies.
Local retention checks original custody and reported identities, not a second
execution of the dot arithmetic or a local rehash of the remote model shard.

Independent data-only review rehashed all 37 originals and checked source/test
custody, the original 4,096-row operand joins, selected weight bytes and reported
integer identities. It did not recompute dot products or rounding, execute a
model, or locally rehash the remote checkpoint shard.

The original README preserves the pre-execution proposal. No model forward or
GPU kernel ran in this CPU diagnostic; its elapsed time is not inference
throughput. It leaves full-model numerical acceptance, exact generated-token
agreement, Full2303 execution and the 700 tokens/s target open.
