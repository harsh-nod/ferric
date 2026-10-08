# Actual Instrumented Request Preparation

The qualified, six-identity-bound preparer passed once on `mi350` in
0.07461332599632442 seconds. It read the actual diagnostic worker and parent
CPU receipts, source maps and executables, authenticated the existing prompt
and kernel images, and wrote three fresh original JSON bodies. This operation
launched no child, model computation or GPU work.

| Prepared original | Bytes | SHA-256 |
| --- | ---: | --- |
| `tail_duration-input.json` | 2316 | `2f5844ddb0c1fcd55c4645a93e9c21f5af0cae62ffce10715a0ab3518a91a440` |
| `tail_duration-request.json` | 9185 | `3f4197b0034725a572f8ed410b67ddcc206ab4c11fdb8e401f4aaad5838aa13d` |
| `prepared-inputs.json` | 13959 | `64c94a620a1efcd024be3a464f2e7929abb839812e402e8e99c70b06981c60f2` |

The request uses the authentic 2,048-token prompt but selects only the existing
40-prompt-forward diagnostic route, with zero generated tokens and captures
at positions 0, 5, 16 and 39. It has a fresh nonzero session identity. The
fixed devices, kernel images, deadlines and arithmetic are unchanged.

All 25 readset entries were rehashed on MI350 before exporting the originals.
The archive contains 19 members: the bound preparer, thirteen unchanged
helpers, three prepared bodies, the staging receipt and an original manifest
with eighteen member pins. No intermediate or unbound native runner was staged.
Independent data-only review checked all original pins, the unchanged qualified
source bodies, request reconstruction, exact prompt/device identities and
CPU/source-map joins. Local review does not claim an additional live rehash.
The 47,944-byte exported archive has SHA-256
`726fe1fed99066a03c20087398de571bf126eec047f8156eaf35c30e9a789d38`.
This README is subsequent commentary, not an original archive member.

Preparation is not a native result. Final runner/evidence-tool bindings,
fresh device-idle admission, the instrumented GPU run and original-result
validation remain separate. There is no new duration breakdown, independent
numerical acceptance or inference performance claim.
