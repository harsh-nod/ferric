# V27 Aligned Prefill KV Copy Proposal

Source-only, independent Ferric engineering candidate. Not integrated, emitted,
executed, formally proved or performance-qualified. Existing V5/V17/V19/V22
sources, images, defaults and receipts are unchanged. The kernel SDK stays the
independent c4c revision used by the current isolated kernel proposals; this is
not a request to change the adapter's 5a SDK or relabel an existing producer.

## Closed Scope

One TP1 request, sixteen prefill rows, 1024 BF16 K channels and 1024 BF16 V
channels, covering one complete aligned sixteen-token page. Both destinations
are separately admitted, writable, exclusive 32768-byte views of the same
physical page in the distinct K and V cache allocations. Inputs are the exact
first 32768 bytes of the sixteen active rows, not their padded32-row allocations.

Two source orders are supported:

- `last_row_first=0`: source positions `[p,p+1,...,p+15]`.
- `last_row_first=1`: source positions `[p+15,p,p+1,...,p+14]`. This is the
  existing output-head-pruning permutation in the final prompt chunk.

Destination rows always remain in natural page-slot order. The kernel has no
slot table, pointer construction, arithmetic conversion, reduction or barrier.
It cannot establish page/sequence/COW authority itself. The future controller
must check the sealed prepared rows and reject the candidate path unless the
closed geometry and exact source permutation hold after execution reordering.
All other work uses the existing V5 append unchanged, including decode,
partial/unaligned chunks, multiple sequences, nonexclusive pages and TP>1.

Each global index `i` in `0..16384` owns exactly one output element in each
disjoint destination. There are 256 workgroups of64 lanes. Natural order reads
`i`; rotated order reads source row `(i/1024+1)%16`, preserving its component.
The branch implementation makes bounds explicit and avoids the earlier V24
unchecked/direct-indexing experiment: only the already-used safe
`memory::volatile_load` and typed `WriteOnlyDisjointSlice::write` APIs are used.
Different lanes may take different local branches, but there is no collective
requiring convergence. No finite-value check is added: all65536 u16 encodings,
including NaNs and signed zero, must copy bit-for-bit like V5/V19.

## ABI and Admission

Distinct root: `ferric_qwen3_tp_prefill16_kv_copy_bf16_v27`.

| Explicit offset | Argument | Shape |
| --- | --- | --- |
| 0,8 | key pointer,length | read-only16384 u16 |
| 16,24 | value pointer,length | read-only16384 u16 |
| 32,40 | key_page pointer,length | disjoint writable16384 u16 |
| 48,56 | value_page pointer,length | disjoint writable16384 u16 |
| 64 | first_position | aligned16, `0..=8176` |
| 68 | physical_page | less than physical_pages |
| 72 | physical_pages | `1..=512` |
| 76 | last_row_first | exactly0 or1 |

Expected explicit bytes80, alignment8, existing implicit bytes256, total336.
Wave64, required/max block `[64,1,1]`, exact dispatched grid `[256,1,1]`.
These are source expectations only; emitted metadata and actual typed host
roster admission must confirm them before a GPU launch. Register/LDS/scratch
counts and artifact hashes are deliberately unspecified until emission.

The current adapter must not normally link this c4c device crate. Later
integration should reuse the reviewed V25 build-script isolation: optional
build-dependency produces escaped logical/export name strings from the actual
compiler roster, while the runtime uses its existing5a admission APIs and all
manifest/descriptor/ABI checks. No hand-authored name substitute, cross-SDK
object cast, or global SDK migration is part of this proposal.

## Source Tests

Five contract tests check the distinct typed ABI/root, exact launch, complete
guarded raw-copy body, eleven negative mutations and independent SDK/target.
The contract compares parsed token trees, not whitespace-dependent snippets;
no multiline tuple/call punctuation normalization is needed for this kernel.

Six host-model tests cover every u16 encoding in both source orders, final-row
rotation, boundary physical pages and prefix/suffix guards,25 invalid scalar/
extent/launch cases, all eight matched prompt chunks and closed-selector
fallbacks. These do not execute the kernel or the production pool. Pool COW,
pointer extents, physical-slot uniqueness and worker ownership are required
additional adapter/native gates, not replaced by these models.

See `prefill-rca.md` for observed evidence and `validation-plan.md` for the
bounded next steps. No local or remote execution was performed in preparing
this proposal. Cargo.lock must be generated and reviewed remotely under the
existing resource guard, preserving only the pinned c4c/7eb dependency closure.
