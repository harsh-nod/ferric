# Engineering Build Evidence

This record identifies the first complete tiny F32 decoder layer lowered from
ordinary Rust through fe2o3's semantic, ranked-memory, and formal checks, then
linked into a gfx950 HSACO. It is not a full Qwen model or a production-qualified
backend.

`build_evidence.json` records the actual extraction environment and argv,
compiler executable **and backend shared-library** hashes, unchanged handoff and
LLVM identities, native tool/provider hashes, command argv, and final HSACO hash.
The `extract_engineering.sh` recipe preserves the actual extraction command,
with an added fresh-output preflight. The compiler source was based on fe2o3
`1989681` with the issue's uncommitted compiler fixes. A later rebuild must be
recorded separately, not substituted for
the observed tool hashes.

`unpack_handoff.rs` is the exact helper used to decode the binary handoff through
the upstream structured API. It writes the unchanged LLVM payload with
`create_new`; it neither authors IR nor creates authority.
`link_engineering.sh` preserves the exact native-link recipe. Its absolute paths
describe the isolated MI350-2 staging directory used for the run. The JSON argv
can be replayed under a new private root after adapting paths and measuring the
new tools. Build the helper with the recorded compatible compiler-ffi rlib; place
the two scripts at the evidence paths recorded in the JSON. Preserve existing
outputs: the link script rejects them instead of overwriting artifacts.

The native route used the installed **ROCm 7.2.0** LLVM tools and nine explicitly
measured device-library bitcode files. It did not use the managed link-worker
path or claim the different reviewed ROCm 7.2.1 provider closure. Inputs were
rehash-checked after linking. No HIP or manually authored LLVM was used.

The ELF reports 48-byte explicit kernargs, six pointer/length fields, wave64,
128-workitem workgroups, and no LDS, private memory, or spills. Optional ELF
maximum-grid, pointer-access, and pointee-alignment fields are absent. Those
unknown observations remain distinct from the fixed Rust source declaration;
the test client must reject any conflicting observed field.

The build record does not assert a numerical result. The independent reference
and `verify.sh` own numerical evidence and bounded engineering dispatch.
