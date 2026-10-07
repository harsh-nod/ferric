# Feature-Enabled CPU Qualification

The fresh MI350 build with `engineering-currentness-duration-diagnostics`
passed all 27 phases in 193.496 seconds: 1,192 runtime tests with eight
ignored, 853 worker tests with four ignored, ten focused scopes and ten
facade doctests. The twelve new runtime and fifteen new worker tests passed;
all previous named outcomes remain required.

Every phase exited naturally, was reaped and left no process group. Postcheck
errors are empty. All 1,065 project source bodies match the separately tested
default build byte-for-byte; only the four mode-specific helpers differ.

The [original receipt](evidence/complete.json) is 2,569,774 bytes, SHA-256
`5a27b9983ecd0d523226cbf2db4070108cd6deaf75d2ee6fe528274d2f8a5ac2`.
The source map is 450,252 bytes, SHA-256
`74bcd0c27c264faf5991e8b13f8dcd2b9f52d42ad1d9b3e9a30350ee2c1baa0c`.
The actual worker ELF is 6,598,384 bytes, SHA-256
`10c7432d29f8ca780ce9338c3217bd93c75b04cd3463d7cd0921c0904382606d`.
Its identity is unchanged across the worker tests.

The original archive is 1,852,863 bytes, SHA-256
`f2318284992560334cdcbece1fe6b5a00c59021cda9acc08d17f60643d90d85d`:
192 members, 191 manifest pins and 13,280,432 expanded bytes, including 142
raw evidence files. Remote export rehashed the full source composition, tools,
cache, dependencies and eleven Cargo-selected products. Local retention
checked original archive and member bytes as data; external ELF and crate
bodies were not copied or locally rehashed. This README is added commentary.

This is CPU qualification of instrumentation. It does not establish a native
timing result, numerical acceptance, faster decode or parent qualification.
