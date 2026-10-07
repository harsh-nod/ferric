# Position-5 Exact Head Diagnostic

Source-only proposal. No tests, shard reads, or arithmetic diagnostic have been
executed by the author. Root owns the bounded MI350 invocation below.

The retained native result selects token 9112 at 19.75, while token 2 is 19.625.
The independent framework result ties both at 19.625 and selects the lower ID.
This diagnostic asks a narrower question: what are the exact real dot products
for those two weight rows on each of the two already observed final-normalized
vectors? It computes four 4096-term dots, not another model forward.

## Arithmetic and Meaning

Every finite BF16 value is represented as an integer in units of `2^-133`.
Products and sums therefore use unbounded signed integers in units of `2^-266`.
The report includes exact decimal strings, exact token-2-minus-token-9112 margins,
round-to-nearest-even BF16, observed-minus-exact error, and signed distances to
the 19.6875 midpoint between 19.625 and 19.75. Input-driven dot and margin changes
are separately checked by exact dot products against the vector difference.

This is **not** an MFMA tree emulator or a reconstruction of the framework GEMM
tree. The actual native head visits 256 K16 MFMA blocks, accumulates FP32, and
narrows to BF16. Legal intermediate FP32 association may differ from the exact
real sum. Agreement with exact RNE can distinguish an upstream-input crossing
from a head-rounding discrepancy; neither outcome establishes full-model
correctness, an acceptance tolerance, or a performance result.

The runtime route uses the full rank-0 head with a separately authenticated
`[4096,151936]` transpose of original `[151936,4096]` storage. This helper uses
original checkpoint rows, not a new transpose or new native upload. It relies
on the already authenticated native model/source admission; it explicitly does
not revalidate GPU-resident weights or the uploaded transpose.

## Input Custody

Eight small original bodies are copied unchanged into `inputs/`, with exact
observed pins in `run.py`. They join native terminal `5b9617ab`, selected capture
`f0fbe98f`, reference inner receipt `d256b149`, both repeated reference position-5
payloads `64dcde3a`, and authenticated comparison `e6dbc471`. The native raw
capture is split at its checked 242824-byte control boundary; the following
606976-byte payload has finalnorm at byte 294912 and logits at byte 303104.
The reference uses the identical payload layout. All payload BF16 values must
be finite. No unselected native intermediate is invented.

The helper hashes original index `f9fdbcb9` and full last shard `20c2d636` on
MI350. The latter is 1244659840 bytes, containing only the 1244659712-byte
`lm_head.weight` tensor and its safetensors header. A duplicate-key-rejecting
JSON parser validates exact BF16 dtype, dimensions, offsets, complete extent,
and absence of holes/trailing data. Rows 2 and 9112 are read with positional
reads from the same open descriptor used for the full-shard hash. Only 16384
weight bytes are kept in memory. Full shard, both rows, header, inode metadata,
and all small inputs/sources are checked again before success.

The other checkpoint shards and old complete capsules are not reread. Their
prior authenticated admission remains ancestry, not a fresh qualification.

## Bounded Invocation

Fresh root:
`/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-position5-head-exact-v228-v1`

Stage `head.py`, `test_head.py`, `run.py`, `README.md`, `source-manifest.json`,
and the following eight unchanged bodies under that root:

| Destination in `inputs/` | Existing original body |
| --- | --- |
| `native-terminal.json` | Native GPU root `readiness/complete.json` |
| `native-summary.json` | Native GPU root `readiness/native/complete.json` |
| `native-capture-5.bin` | Native GPU root `readiness/native/capture-5.bin` |
| `native-request.json` | Native GPU root `readiness-request.json` |
| `reference-terminal.json` | Reference root `output/complete.json` |
| `reference-pass1-pos5.bf16` | Reference root `output/pass1-pos5.bf16` |
| `reference-pass2-pos5.bf16` | Reference root `output/pass2-pos5.bf16` |
| `comparison-terminal.json` | Authenticated position-5 comparison `output/complete.json` |

Native root ends `guarded-mlp-readiness40-position5-gpu-v228-v1`; reference
root ends `guarded-mlp-readiness40-position5-reference-v228-v1`. All destination
bytes are identical to the retained canonical bodies, not reconstructed JSON.
The original model root is fixed to
`/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target`.

Run only after independent source peer and exact manifest binding:

```sh
/usr/bin/python3 -I -B /home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-position5-head-exact-v228-v1/run.py SOURCE_MANIFEST_SHA256
```

The controller requires the observed MI350 hostname/UID, affinity 8/9, nice at
least 10, empty GPU visibility, 180 seconds whole wall, 120 seconds CPU, 512 MiB
address space, 2 MiB per output, and no core dump. It launches no child, opens
no GPU/device, imports no project/framework module, and uses no network.
Twelve exact synthetic tests run in-process before model reads. Their expected
values use independent `Fraction` arithmetic and closed-form boundaries.
The timer remains catchable through posthash and terminal retention. Expiry
may leave an incomplete failed prefix; it cannot fabricate success or retry.

`output/tests.stderr` preserves named test output. `output/complete.json` or
`output/failed.json` preserves source/input pins, full-shard/row pins, tests,
exact arithmetic, limits and explicit false execution/acceptance/performance
claims. Existing output directories are refused. No native timing is measured.
