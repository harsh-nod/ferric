# TP1/C1 KV Copy V19

An opt-in engineering candidate, not a benchmark, proof, or serving qualification.
The resident V5 append root uses one grid leader to copy 1,024 key and value
elements at TP1/C1. V19 instead launches exactly 16 Wave64 workgroups, with one
invocation owning one element in each independently admitted destination view.
It copies raw u16 bits, including nonfinite BF16 encodings, without arithmetic.

The controller must validate the prepared row's pool/scope/freshness, logical
position, page-table mapping, writable page/token offset, and immutable-prefix
copy-on-write ownership before binding the two 1,024-element output views.
Their byte offset is `(physical_page * 16 + position % 16) * 1024 * 2`.
The device checks exact slice extents, launch extent and bounded page/position
scalars; it does not independently authenticate the host's page-table mapping.
The runtime still checks buffer ownership, access and slice bounds.

Only TP1 with one active row may select this root. Multi-row prefill and all
historical V5/V17 defaults retain their existing append kernel. V19 receives its
own compiler roster, canonical artifact admission and live-controller identity;
it must not be passed through a V5, V14, V15 or V17 image validator.

CPU models/source tests are not GPU execution or numerical qualification. Before
acceptance, emit this crate with the pinned fe2o3 toolchain, inspect its exact
one-root ABI, compare all 1,024 copied K/V words plus adjacent-slot guards, and
run the unchanged independent 128-input/128-output Qwen reference in matched
control/candidate arms. Report measurements without assuming a gain. Existing
host group scopes are overlapping wall latency, not individual GPU durations.
