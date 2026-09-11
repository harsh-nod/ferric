# Opt-In TP Draft Model Intake

`EngineeringQwenModelV1::open_with_draft` retains the canonical Qwen3-0.6B draft
payload alongside the existing Qwen3-8B target. It is an addressless engineering
intake API, not speculative execution or a new model/device admission authority.

## API and Ownership

- `open(source)` keeps its target-only behavior. Draft authentication still runs,
  but its output goes to a sink: no draft payload buffer is allocated or retained.
- `open_with_draft(source)` retains exactly 1,503,264,768 bytes of header-free,
  row-major BF16 draft payload in one owned buffer. It does not clone the target,
  draft payload, tokenizer, or authenticated layout.
- `draft()` returns `None` for target-only intake. Opt-in intake returns a borrowed
  `EngineeringQwenDraftModelV1` with `weights()`, `config()` and `layout()` access.
  Its fields and constructor are private; callers cannot substitute an unbound
  payload or model role. Its configuration comes from the layout's `draft_model`.
- Existing `target_weights()`, `config()`, `layout()`, `bundle_id()`, `encode()` and
  `decode()` semantics remain unchanged. Both roles share the original canonical
  tokenizer identity and authenticated target/draft layout; no second tokenizer
  or new bundle identity is introduced.

Both entry points use the same source reopening, exact source prepacking,
canonical bundle sealing and model-layout construction. Opting in changes only
the draft output destination. Source path/roster, role, schema, exact length, EOF,
digest, manifest and tokenizer checks are not relaxed. Allocation, incomplete
output or authentication failure returns no model or draft view.

## Bounds and Verification Scope

Focused CPU tests use small buffers and malformed source fixtures to check the
allocation-free discard path, exact retained bytes, ownership transfer, short and
oversized output, allocation failure and preserved authentication errors. They do
not substitute small fixtures for successful canonical model authentication.
Successful full-model loading and numerical draft execution require separate
model-backed gates; neither is established by these bounded unit tests.

No CLI, runtime, scheduler, paged-KV or kernel behavior changes here. Resident draft
execution needs a separately qualified draft-compatible image. End-to-end fast
speculation still needs draft/target orchestration, target verification rows,
accepted-prefix KV settlement, rejected-tail rollback and correct publication.
This intake API establishes none of those execution or performance claims.
