# Strict Libtest Progress Parsing

Status: all 17 standalone parser regression tests pass on MI350. This is a
qualification-harness result, not compiler, guarded-kernel, GPU or model acceptance.

The [second fusion qualification](../guarded-mlp-ranked-cfg-linear-fusion-v1/attempt-v2/evidence/failed.json)
failed because the harness treated a valid libtest progress notice as a final
result. Its compiler process nevertheless exited zero with 1,323 passing tests
and 24 ignored tests. The failed qualification remains unchanged.

## Correction

The [helper](attempt-v1/qualification_helpers.py) accepts only the exact form
`test NAME has been running for over 60 seconds`. The name must belong to the
independently authenticated, non-ignored inventory, must not have completed,
and may report progress only once before the suite summary. Progress never
counts as a final result. Every final name and status must still appear exactly
once, and the unfiltered summary must match the full inventory.

Current helper identity is recorded separately from the historical helper in
the next CPU qualification and loader. Compiler code, source proposal, test
rosters, structural limits and resource limits are unchanged.

## Verification

The [tests](attempt-v1/test_qualification_helpers.py) cover accepted progress,
unknown and ignored names, duplicate notices, notices after completion or the
summary, malformed durations, missing and duplicate final results, failed or
changed ignored statuses, summary drift and invalid inventories. An exact-stream
test replays the captured compiler stdout against the previous qualified roster
plus nine fusion tests. It explicitly keeps the original receipt's failed status.

The [receipt](attempt-v1/evidence/complete.json) reports 17 passes. Its only child
exits naturally with code zero, is reaped and leaves no process group. Source,
input and tool postchecks are clean. The complete gate takes 0.124052 seconds.
Its capsule has 17 members, 16 pins and seven raw files: 387,975 compressed bytes,
SHA-256 `5883770fc65a81b0ede167425440cc40dc539b342f8fed02e5d3d55cde465589`.

The fresh 44-phase compiler qualification is a separate gate. This passing parser
test does not complete any issue #42 milestone or establish decode throughput.
