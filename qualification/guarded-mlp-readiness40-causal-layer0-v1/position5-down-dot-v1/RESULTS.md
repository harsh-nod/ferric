# Selected Down Diagnostic: First Attempt

This MI350 CPU attempt failed before model-shard reading or selected-dot
analysis. All 48 synthetic tests ran: 47 passed, one failed, none errored or
were skipped. The runner retired after 1.764 seconds with clean source/input
postchecks. No GPU or model execution occurred.

`test_reuse_complete_original_boundary_rows_without_dot_reexecution` compared
the API's returned tuple with a list of the same 4,096 BF16 words. The failed
assertion is a fixture container-type mismatch; this does not establish that
the not-yet-run real-data diagnostic succeeds. A separate test-only repair and
fresh attempt are required. The original sources and failed result are immutable.

- Original terminal: 17,135 bytes, SHA-256
  `f96283daae1281a78d6590b2f3706f4c81921c3bbecbe1fde00170d076fce9f1`.
- Original test stream: 7,579 bytes, SHA-256
  `fcb93b4469a8f622359d7143efa8d2a726ee182fd7ed8b33b6cb028b65068964`.
- Original evidence archive: 3,909,989 bytes, SHA-256
  `0d365d7b5d1967a8895cce7175b03fde477a3d83e40390d80f80ed2f4dac9302`.

All 36 archive originals are retained, with 35 manifest-pinned bodies.
Local retention checked the original archive and every member, without another
test execution.
The original source-only README describes the proposed diagnostic; it is not
an observed outcome. There is no new numerical acceptance, arithmetic change,
argmax explanation, Full2303 admission or performance result.

The separately retained [V2 retry](../position5-down-dot-v2/RESULTS.md) has since
passed. It does not rewrite this original failed attempt.
