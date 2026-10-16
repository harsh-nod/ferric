# Ferric aggregate Worker V3 source-pin extractor

This standalone crate decodes one exact, canonical Worker V3 load envelope V2
and projects the six source coordinates needed by Ferric's aggregate M1 intake:

- compiler module SHA-256 and byte length;
- nested compiler handoff SHA-256 and byte length;
- compiler symbol-manifest SHA-256 and byte length.

The decoder uses `fe2o3-runtime-protocol` and `fe2o3-compiler-ffi` typed APIs at
revision `5a503c04f5ae107a3b3e951ec970b36c5d6a9a79`. This active dependency pin
does not revise the provenance of any previously emitted artifact. It additionally requires LLVM
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

## CPU Byte Prebinding

`SOURCE_PIN_ADAPTER_BINDING_20261016_V1.json` is the current CPU-only byte
prebinding, from the reviewed Rust 1.97.1 release build at Ferric `e0dd5c5`.
Its binary SHA-256 is
`b3e93266133bdeebe9aa06fb95aa2f1cb2656cffd7393cafda8715076b452268`.
The undated `SOURCE_PIN_ADAPTER_BINDING_V1.json` remains unchanged as historical
evidence and is not accepted as the current record. Neither record grants
reproducible-build, compiler-origin, semantic, proof, native, or runtime approval.

## Verification

```text
cargo fmt --manifest-path Cargo.toml -- --check
cargo clippy --manifest-path Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path Cargo.toml --all-targets --locked
```
