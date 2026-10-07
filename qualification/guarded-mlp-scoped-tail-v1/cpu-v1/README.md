# Scoped Tail CPU Qualification

The fresh run on `mi350` passed all 27 phases in 173.159 seconds. Every phase
retired naturally, was reaped and left its process group absent. Source,
dependency, cache and executable postchecks passed. No model or GPU ran.

| Check | Observed result |
| --- | ---: |
| Complete runtime suite | 1,180 passed; 8 existing ignored |
| Complete worker suite | 820 passed; 4 existing ignored |
| New tail-focused runtime tests | 15 passed |
| Selected facade doctests | 10 passed |
| Doc-parser regression cases | 8 passed |
| Cargo products | 11 |
| Source-map entries | 1,059 |
| Original raw evidence bodies | 142 |

All previous named outcomes remain, with fifteen new runtime and twenty-one
new worker tests. They cover serial execution, every checked boundary,
failure/unwind poisoning, strict counters, first-use isolation, CLI selection
and refusal without fallback. The actual worker ELF is identical before and
after CLI tests and at final qualification. Both crates use optimization
level 2 with debug assertions and overflow checks retained.

The capsule retains all nineteen actual Rust postimages: six runtime and
thirteen worker files. Twelve were changed by the remote formatter. The final
map has 827 runtime fixture files, 228 worker files and four helpers; the
canonical runtime has 825 files after excluding fixture Cargo aliases.

## Original Identities

- [Terminal](evidence/complete.json): 2,449,865 bytes,
  `a2e3d99f94b5b01223f42f3d8917c2cf6a1fc25f9227d4c341bc2b9a7cc9285c`.
- [Source map](evidence/sources-after.json): 424,416 bytes,
  `a2a74ed023e9b2aac224621d37f328d097838c317c901a361c1496064e162ec4`.
- Worker ELF: 6,505,712 bytes,
  `635f4b9a05d56a0cea84aa894304dcea2931cfabc3d5f1723927e7a207b70e71`.
- Original archive: 1,809,920 bytes,
  `a0f8a1faae972ce5c13e44bcd3a405530fa9c6227c595820a3d22ab154ca249c`.

The [manifest](manifest.json) pins 189 original bodies and is the 190th archive
member. This README is additional commentary. External executable and cache
bodies remain on MI350; local retention does not re-observe them.

The run kept the private offline cache, locked dependencies, CPUs 8/9, nice 10,
two Cargo jobs, hidden GPU variables and unchanged 3,600/1,800/50-second
whole/leaf/cleanup bounds. Initial/live free-storage floors remain 40/38 GiB.
This CPU result alone does not admit a native launch or numerical/performance
acceptance.
