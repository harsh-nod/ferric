# Ferric aggregate Worker V3 source-pin extractor

This standalone crate decodes one exact, canonical Worker V3 load envelope V2
and projects the six source coordinates needed by Ferric's aggregate M1 intake:

- compiler module SHA-256 and byte length;
- nested compiler handoff SHA-256 and byte length;
- compiler symbol-manifest SHA-256 and byte length.

The decoder uses `fe2o3-runtime-protocol` and `fe2o3-compiler-ffi` typed APIs at
revision `4413b086482f2f4ad218f28e4485dc089d6cc020`. It additionally requires LLVM
text IR, `gfx942:xnack-`, code-object V6, exactly 12 aggregate kernel-entry
symbols, and their 12 matching `.kd` descriptor symbols. Symbol-manifest
matching proves exact sets; it does not claim compiler descriptor-table order.

## Use

```text
ferric-qwen3-all-kernels-worker-v3-source-pin-v1 ENVELOPE
```

Use `-` to read the envelope from standard input. Successful output is one
deterministic, pretty-printed ASCII JSON document with a final newline.

Protected-verifier adapters that already retain a typed
`CompilerModuleHandoffV2` can call
`project_m1_aggregate_module_handoff_v1` directly. That API repeats the same
module-kind, target, code-object, entry-symbol, and descriptor-symbol policy;
it does not weaken the authority boundary below.

## Authority boundary

The output is an `identity-observation-only` projection. Decoding and matching
content identities do not authenticate compiler origin or grant protected
verifier, publication, GPU load, or GPU launch authority. Ferric must still
recover current publication custody, admit the compiler-generated roster, and
obtain independent protected verification evidence before runtime admission.

## Verification

```text
cargo fmt --manifest-path Cargo.toml -- --check
cargo clippy --manifest-path Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path Cargo.toml --all-targets --locked
```
