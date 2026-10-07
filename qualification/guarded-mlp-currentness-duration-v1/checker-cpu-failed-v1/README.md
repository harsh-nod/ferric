# Original Synthetic Checker Failure

The first MI350 timing-record checker run completed all 126 synthetic tests
in 71.505 seconds and reported three errors. Its original 114 inherited tests
passed. The positive complete-case, large-integer and same-side cases failed
the fixed per-forward bank census because the new fixture reused generic
Tail checkpoint counts.

The single child exited naturally with code 1, was reaped, and left no process
group. The controller completed in 71.690 seconds with no postcheck errors.
Its admitted test census is null; this is not a partial qualification.

The [original failed receipt](evidence/failed.json) is 23,385 bytes, SHA-256
`b5419c099ed61c134517629c2d4ca1d29e96985fab07f5f3a3c32498516538e1`.
The original archive is 77,118 bytes, SHA-256
`0339edfc9033c409ae9f064bda78bd4603c1b9aaa71668ba7c17f955485d84e5`:
36 members, 35 manifest pins and 419,690 expanded bytes. It preserves all 26
deployed source bodies, seven raw files, terminal, exporter and manifest.

The proposed retry corrects only the new fixture and verifies its positive
case before mutation tests. The production validator, prior test modules and
acceptance conditions are unchanged. No GPU or model ran. Local retention
checked original bytes as data; this README is added commentary.
