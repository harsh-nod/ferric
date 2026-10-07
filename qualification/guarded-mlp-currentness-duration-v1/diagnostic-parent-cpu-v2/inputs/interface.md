# Currentness Duration Diagnostic Wire Proposal

Source-only opt-in proposal. No model, CPU qualification, native result, or gain
is claimed. Runtime ownership is in ../runtime-v1; this proposal owns worker
collection and the narrow parent Tail parser.

Feature: `engineering-currentness-duration-diagnostics`. Worker forwards this
feature to fe2o3-kfd. Parent enables its existing readiness feature and imports
the pure codec, without changing any default feature, selector, environment
variable, facade, policy, or execution authority.

Pure codec module:
`finite_guarded_mlp_readiness_currentness_durations_v1`.
Schema: `FerricReadiness40TailCurrentnessDurationsV1`.
Public codec types: CallDuration, Durations, MeasuredForward, ForwardRow, Record.
Record::new(policy, rows), Record::validate(policy), encode(), and
decode_stderr(raw, bootstrap, transcript, worker) authenticate original bytes.
The parent retains whole stderr. Disabled builds retain the exact original
single canonical policy decoder and 4096-byte bound. Enabled builds require
exactly two canonical newline-terminated records: the original V4 policy
(max4096) and this record (max65536), without truncation or substitution.

The record binds original policy SHA-256, session, worker SHA-256 and transcript.
It explicitly marks instrumentation, host elapsed nanoseconds, bank-body overlap,
no numerical acceptance, no performance claim, and no execution authority.

Forty ordered rows cover positions0..39. The first two have measured=null.
The remaining38 contain bank/layers/tail Durations and bank_guarded_body_ns.
Each Durations has before/discover/after/root_generation; each CallDuration has
calls:u64 and elapsed_ns:u64. Aggregated call counts must equal the corresponding
original V4 policy bank/layer/tail counts. Every warm bank row must independently
have 2 discoveries, 726 before calls, 726 after calls and 725 generation probes.
The four bank callback durations are checked-summed and must not exceed that
row's bank guarded-body duration. The guarded-body interval includes its
callback times and must not be summed with them as a disjoint category.

Runtime leaves CurrentnessCounts unchanged. Feature-only root-exported types
are Gfx950EngineeringCurrentnessCallDurationV1 and
Gfx950EngineeringCurrentnessDurationsV1, Copy+Default+Eq. Existing Layer/Bank/Tail
observations acquire currentness_durations; Bank additionally acquires
bank_guarded_body_ns. The runtime owns callback count/duration validation before
its existing armed commit. Worker validates/aggregates every metric inside an
armed operation before its forward/Close commit. Failure and unwind remain
terminal. Wire identity/publication follows healthy Close and original worker
rehash; it does not retroactively poison a retired runtime owner.

The same explicit Readiness40 Tail CLI is required. Full, Default, causal and
other readiness routes do not emit this record. Stdout serialization is
unchanged, and the first line is exactly the original encoder's policy bytes.
No cross-run identity of elapsed timings, or of whole stderr, is claimed.
All 124 parent timing spans, numerical rules, capture payloads, own transcript
and deadlines remain unchanged. Runtime feature builds instrument all existing
scoped windows; only this explicit worker route collects and emits the record.
Unmodified Python/native validators still reject the additional record; a
separately qualified diagnostic admission is required before any native run.
