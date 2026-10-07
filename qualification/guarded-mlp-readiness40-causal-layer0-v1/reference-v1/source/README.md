# Causal Layer-Zero Reference Source

This source-only derivative runs the same two genuine 40-position Qwen3-8B
reference passes as the retained position-5 reference. It authenticates all
2,048 prompt IDs, consumes only positions 0 through 39, generates zero tokens,
and executes all 36 layers with its own fresh DynamicCache in each pass.

The unchanged BF16 model policy uses math-only SDPA and preserves FP32 rotary
evaluation. The observer wraps the genuine rotary function and returns its
original result; module hooks only copy actual values. It captures 33 stages
at positions 0 through 5 and emits exactly two extra binary sidecars. No
native inputs, native intermediate values, or substituted operation outputs
are used.

The owner runs the original 20 pure tests and seven observer tests in separate
owned CPU leaves before creating the offline container. Existing container
ownership, timeout, cleanup, idle, topology, package/source/model hashing, and
writable limits are unchanged. Both CPU suites must pass by exact named census.

The historical position-5 reference remains a host-only comparison input,
never an additional container mount. Its actual owner and inner receipt pins
are fixed in launch.py. Both complete 40-record pass histories and all eight
selected payloads (positions 0, 5, 16, 39 in each pass) must remain byte/hash
identical. The owner also revalidates every sidecar extent, stage hash,
producer/consumer join, current cache append and completed cache prefix, and
requires all causal stages to repeat across the two fresh passes.

This is a proposed diagnostic run, not an observed outcome. Numerical
acceptance, full-workload acceptance and performance claims remain false.
