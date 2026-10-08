# Instrumented Readiness40 GPU Result

The actual diagnostic passed on `mi350` on its first attempt. It completed
40 prompt forwards through all 36 Qwen3-8B layers across two gfx950 GPUs,
with zero generated tokens. The whole supervised case took 450.8463263739832
seconds; the native parent phase took 444.11034544801805 seconds. These are
host-wall intervals, not GPU kernel timings or decode throughput.

All eleven supervised phases exited with status zero, without cleanup signals,
with all owned processes reaped and their groups absent. The original worker
lineage was verified. Three prelaunch and three postflight idle/topology checks
passed. The GPU window was released after those postchecks; subsequent work
is data-only.

## What Passed

- All forty semantic records, logit pins and observation pins match the
  historical Ferric baseline.
- All four complete observation payloads at positions 0, 5, 16 and 39 match
  that baseline. Timing/control fields are checked separately; whole capture
  file equality is not claimed.
- The complete original worker stderr contains exactly two canonical records:
  the existing currentness policy and the new duration diagnostic. The latter
  binds the original policy bytes, session, worker and transcript.
- The first two forwards are unmeasured by the new callback instrumentation;
  the remaining thirty-eight have bank, layer-aggregate and tail measurements.
  Their callback counts reconcile with the original policy.
- The 124 disjoint parent timing spans are retained separately from the
  callback measurements. Nested bank-body time must not be added to its
  callback subtotal.

## Original Evidence

The final, independently source-reviewed evidence tool ran on MI350 and
successfully revalidated and exported the actual result. The capsule has
167 original members and 166 manifest pins: eighteen deployed/prepared root
bodies, the current terminal and 73 raw files, 73 historical baseline originals,
and the exact retention tool. Its expanded size is 9,625,092 bytes.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `tail_duration/complete.json` | 737180 | `65d7a93e4ef330452566923908ef851956a30d47b95708d6c97976678ee18cd7` |
| Original exported archive | 4555390 | `fe351dbef62f5e1ba90fe91eee99877c9c53006394054cf88fec5654fa2cf71b` |

The original native terminal authenticates 145 readset inputs. Local retention
checked every archive member pin, all 73 raw joins, the eleven original phase
records and the recorded parity outcome without importing project helpers.
Independent data-only review also reconstructed both original forty-frame
transcript chains, checked all four payloads, canonical stderr/count joins and
all 124 parent spans. It rehashed 131 locally retained readset bodies and joined
fourteen remote-only identities through metadata, without claiming a new live
rehash of those external files.
This README is subsequent commentary, not an original manifest member.

Same-side parity is not an independent numerical reference. The known
position-5 framework discrepancy remains unresolved. Full2303 has not run,
the full 2,048/256 acceptance gate remains open, and this instrumented single
case is not a matched speed comparison or evidence for 700 tokens/s. The
reproducible duration report is a separate next step.
