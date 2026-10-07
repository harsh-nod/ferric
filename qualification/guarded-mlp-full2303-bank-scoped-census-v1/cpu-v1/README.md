# Full2303 Bank And Census Worker Qualification

The fresh CPU qualification on `mi350` passed all 26 phases in 172.933 seconds.
Every phase exited naturally, was reaped and left its process group absent.
Source, dependency, cache and product postchecks passed. No model or GPU ran.

| Check | Observed result |
| --- | ---: |
| Complete runtime suite | 1,165 passed; 8 existing ignored |
| Complete worker suite | 799 passed; 4 existing ignored |
| Focused runtime scopes | 9 passed |
| Selected facade doctests | 10 passed |
| Doc-parser regression cases | 8 passed |
| Actual Cargo products | 11 |
| Source-map rows | 1,053 |
| Original raw evidence bodies | 137 |

All 783 prior worker named outcomes remain; 19 library tests and one actual
executable CLI test are new. They cover the full bank/layer schedule, final
generations, compact policy and own-output history, malformed identity/counts,
checked arithmetic, first-use isolation, refusal, unwind and no fallback.
The real worker executable is identical before/after CLI tests and at the
final build. These synthetic schedule tests are not a 2,303-forward GPU run.

The runtime's 825 bodies are unchanged. Fifteen worker postimages, including
four additions, are retained exactly as formatted and tested on MI350. The
complete worker source map has 224 entries. The final ordinary Full forward
loop and its one-hour child deadline remain unchanged.

## Original Identities

- [Terminal](evidence/complete.json): 2,475,794 bytes,
  `a7fcea3f09cfcda84ba556f60b5f1578ed378a45fc84bb81fbfa6ed31273cd64`.
- [Source map](evidence/sources-after.json): 438,675 bytes,
  `5372e185504d11ed89d45cfb3d3e32672f93e8d1bbf3d04e44932b280300528c`.
- Worker ELF: 6,458,376 bytes,
  `362cb2fe7543e60bc3ae7c68b53cb84f0339acd3b33d5909a5ca49e3538f2af0`.
- Original archive: 1,752,995 bytes,
  `0387d548af19d09fe0c4c2672a6b08db972d3c241995a53c4fba8fa83c36daad`.

The [manifest](manifest.json) pins 179 bodies and is the 180th original member.
The capsule includes all raw evidence, fifteen tested worker postimages,
nine lineage bodies, source/cache receipts and executed helpers. This README
is additional commentary. Executable and dependency bodies remain on MI350;
local retention does not claim to re-observe those remote bytes or processes.

The run used a private authenticated offline cache, locked dependencies, CPUs
8/9, two Cargo jobs, nice 10 and hidden GPU variables. Bounds remain 3,600
seconds overall, 1,800 per leaf, 50 for cleanup, 12 GiB address space and
40/38 GiB initial/live free storage. Existing ignored tests remain explicit.

Parent qualification, strict native data admission, launch feasibility,
actual Full2303 execution, exact independent 256 IDs/raw decoded bytes and
performance remain separate gates. This CPU result does not admit a GPU launch.
