# Synthetic Worker V3 Collector Harness

This committed harness overlays test-only code onto the exact pinned fe2o3
`e527d230c05dfeefec6cc6c91de0b6f16310f677` source tree. It reuses fe2o3's
adversarial durable-publication fixture and invokes Ferric's real aggregate
collector against a hand-authored synthetic 12-entry HSACO.

The matrix covers exact success, all typed binding fields, malformed and
signature-corrupted external records, artifact mutation, a stale first
publication after a second publication, cooperative lock retention, and
canonical-current mutation between collection and sealed handoff.

The fixture is non-production. It is not compiler-produced evidence and does
not close artifact, promotion, `CURRENT`, load, launch, hardware, numerical, or
performance gates. The separate V77 strict rejection remains the negative
evidence for the preserved compiler observation. Caller challenge replay and
atomic challenge/current-ledger consumption remain responsibilities of the
external promotion service.
