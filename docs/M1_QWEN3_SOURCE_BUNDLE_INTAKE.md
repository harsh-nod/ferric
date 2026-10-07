# M1 Qwen3 Source Bundle Intake

Ferric's offline prepack path now begins with one exact, descriptor-held source
bundle. The accepted root has exactly two entries:

```text
draft/
target/
```

`target/` contains only the pinned Qwen3-8B `config.json`,
`tokenizer_config.json`, `tokenizer.json`, `model.safetensors.index.json`, and
five safetensors shards. `draft/` contains only the corresponding pinned
Qwen3-0.6B metadata, tokenizer files, and single safetensors file. File lengths
come from the same compiled model and safetensors pins used by semantic
admission.

`ferric_build::open_canonical_qwen3_source_bundle` opens the root and every
child with Linux `openat2`, `NOFOLLOW`, `NO_SYMLINKS`, `NO_MAGICLINKS`, and
`BENEATH` for child lookup. It requires exact directory rosters, regular files
with one filesystem link, distinct device/inode identities, exact lengths, and
stable file and directory metadata. Metadata and tokenizer files are read to a
stable EOF while held open. Weight descriptors remain held as forward-only
readers until the existing safetensors authenticators observe EOF and recheck
their metadata snapshot. The prepack CLI no longer reopens source names after
this boundary.

This removes caller-controlled source pathname and ignored-file ambiguity from
the production prepack flow. It does not trust the directory merely because its
shape is accepted: config and tokenizer metadata still require their exact
compiled hashes and closed schemas, both tokenizer payloads must be byte equal,
and every weight file still requires its exact full-file SHA-256, EOF, and
safetensors schema checks. The output snapshot retains its existing canonical
bundle, prepacked manifests, and admission record.

## First-Five-Gate Audit

This change was selected after checking the first five model-and-build roadmap
requirements against the production prepack path. Ferric already has bounded,
exact target/draft model descriptors; strict config, tokenizer, vocabulary,
index, and safetensors authentication; a target/draft tokenizer compatibility
join; streaming prepacked weights with per-section authentication and reopen;
and a generated target/draft runner declaration whose plans retain their
complete identities. Those mechanisms remain the content and plan authorities.

The missing slice was one level outside them: the CLI previously opened model
metadata and weights by caller-controlled paths without admitting the exact
outer directory roster or retaining file descriptors across source
authentication. This boundary closes that implementation gap. It does not turn
the five roadmap requirements from `Open` into satisfied requirements; release
qualification still needs all evidence classes required by the roadmap.

## Non-Claims

This source intake is not a signature, repository-provenance check, remote
download client, immutable-filesystem proof, or defense against a compromised
kernel or storage device. Linux pathname resolution, descriptor and stat
semantics, reads, timestamps, and filesystem behavior remain contracted host
assumptions. It creates no model, runner, artifact, hardware, numerical,
performance, qualification, or `CURRENT` authority, and closes no M1 roadmap
gate by itself.
