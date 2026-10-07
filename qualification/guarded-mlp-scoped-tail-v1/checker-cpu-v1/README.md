# Tail Policy Checker Qualification

The owned CPU run on `mi350` passed all 114 synthetic tests without skips,
errors or failures in 70.690 seconds. All 98 prior cases remain unchanged;
sixteen new cases exercise Tail V4 policy records, checked integer arithmetic,
original stderr, same-binary pairing and all forty records/four payloads.

The single test process exited naturally and was reaped with its process
group absent. All 24 source bodies, tool identities and seven original raw
bodies passed postchecks. CPU affinity 8/9, nice 10, hidden GPU variables,
512 MiB address space and 180/120/50-second bounds remain unchanged.

- [Terminal](evidence/complete.json): 33,263 bytes,
  `ecb5ea55c489d756614c91c7d394f08578c38342212c27c3be42c1bc2839472d`.
- Original archive: 72,526 bytes,
  `6e7d3589567ebcb14e5ffb8dae36ff1068d3ece5e385468676804ec321a23a49`.

The [manifest](manifest.json) pins 33 bodies; it is the 34th original archive
member. Retention metadata and this README are additional commentary. These
tests qualify the data checker only, not a GPU result, independent model
correctness, overlap, throughput or Full2303 launch feasibility.
