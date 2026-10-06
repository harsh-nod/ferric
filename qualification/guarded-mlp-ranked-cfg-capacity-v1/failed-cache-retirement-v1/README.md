# Failed Build Cache Retirement

The obsolete failed CFG-cap V1 generation's `target` and empty `tmp`
directories were removed from `mi350`. The [executed controller](controller.py)
verified the exact failed receipt and archive, 15 terminal/reaped children,
ownership, ordinary-file roster and absence of owned-process references.
Elevated access was used only for process metadata inspection; the controller
irreversibly dropped to UID 9661 before deletion.

The [receipt](receipt.json) records 2,028 removed build files with
2,444,630,241 logical bytes. Available space changed from 41,614,012,416 to
43,841,245,184 bytes during cleanup; on this shared filesystem that difference
is not an isolated physical-byte measurement. All 83 preserved evidence pins
were rechecked. Source, input, raw evidence and archive remain; no successful
generation was touched. The process-reference scan is a point-in-time check,
not a lock against future processes.

Controller SHA-256:
`8e498f6e95e5f38f5f74773dd147db9410d3b676ec700cef6bbbd02cf44bf9c4`.
Receipt SHA-256:
`1bf38729ad87d37b18261700929e2c85105ee03d4b707ff3fa1c57c4016c5faf`.

This is storage maintenance, not compiler or GPU qualification.
