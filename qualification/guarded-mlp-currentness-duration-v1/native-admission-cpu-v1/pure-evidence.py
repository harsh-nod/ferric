"""Original-only data export/retention of the bounded171-test admission CPU gate."""
import ast
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
Q = Path('/home/harsh/ferric-p227-integration/qualification/guarded-mlp-currentness-duration-v1')
MODES = {'admission': dict(count=171, modules={
  "test_readiness": [
    "ReadinessTests"
  ],
  "test_shared": [
    "SharedTests"
  ],
  "test_timing": [
    "TimingTests"
  ],
  "test_matched": [
    "MatchedTests"
  ],
  "test_scoped": [
    "ScopedTests"
  ],
  "test_scoped_pair": [
    "ScopedPairTests"
  ],
  "test_bank_scoped": [
    "BankScopedTests"
  ],
  "test_census": [
    "CensusTests"
  ],
  "test_tail": [
    "TailTests"
  ],
  "test_duration": [
    "DurationTests"
  ],
  "test_admission": [
    "AdmissionTests",
    "CpuAdmissionWorkflowTests",
    "PreparerWorkflowTests"
  ],
  "test_retention_workflows": [
    "RetentionWorkflowTests"
  ]
}, sources={
  "fixtures/tail-matched-v2/baseline/readiness-input.json": [
    1639,
    "710de3495afe1a03e8f9e1fbb06f25a62f74e6a9953301c212a426060e1c5f7a"
  ],
  "fixtures/tail-matched-v2/baseline/readiness-request.json": [
    9173,
    "d0ad15ae0ccb90109fe92da6fab8281745da479cf77e68af5a10abeb0988eee4"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-0-topology.json": [
    2872,
    "63b2899b5d3d93df727ae82f0d98a750db686d30ac4a9844bd4f1be2ed7dd632"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-0/command.json": [
    695,
    "b0a6b5b327184f44f4f77797153cc68669d12483fd95b0cea0f4528d2e146ff2"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-0/result.json": [
    1662,
    "a4fefac1685f7034ae7839c7d415e4867831a27d0a6c8e843f456bedeccfee57"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-0/started.json": [
    282,
    "c0267551b4b00958f2cecd684272b8972d1360867def223dae4cf4f6b016c521"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-0/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-0/stdout": [
    1275,
    "e70f2d7235dbd50455554f2a277613971e2fcdf117ec1bdcdf47644bcf49c422"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-1-topology.json": [
    2872,
    "63b2899b5d3d93df727ae82f0d98a750db686d30ac4a9844bd4f1be2ed7dd632"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-1/command.json": [
    695,
    "b0a6b5b327184f44f4f77797153cc68669d12483fd95b0cea0f4528d2e146ff2"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-1/result.json": [
    1661,
    "5b02de52c18e2aeb59de46d7914fa194404678e1d67685ed344e740c2a8afe5c"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-1/started.json": [
    282,
    "99352bdce864e8d9aae81439773881087c7516059554b5f66282eeb1a44775a4"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-1/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-1/stdout": [
    1275,
    "e70f2d7235dbd50455554f2a277613971e2fcdf117ec1bdcdf47644bcf49c422"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-2-topology.json": [
    2872,
    "63b2899b5d3d93df727ae82f0d98a750db686d30ac4a9844bd4f1be2ed7dd632"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-2/command.json": [
    695,
    "b0a6b5b327184f44f4f77797153cc68669d12483fd95b0cea0f4528d2e146ff2"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-2/result.json": [
    1662,
    "4d360aa59401ef7f542b4e27095937d4ca169ef071628db56c1e52af0f87b894"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-2/started.json": [
    282,
    "5184542fc6a8f60bc2e3cb5a901685ce1a42aa898aa2dbb410cb626bc48a354b"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-2/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/after-2/stdout": [
    1275,
    "e70f2d7235dbd50455554f2a277613971e2fcdf117ec1bdcdf47644bcf49c422"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-0-topology.json": [
    2872,
    "63b2899b5d3d93df727ae82f0d98a750db686d30ac4a9844bd4f1be2ed7dd632"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-0/command.json": [
    695,
    "b0a6b5b327184f44f4f77797153cc68669d12483fd95b0cea0f4528d2e146ff2"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-0/result.json": [
    1666,
    "1e68767e7cf27adb03d9cc3a81c810a78faa0129077203bd301bde1f7e8f9b78"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-0/started.json": [
    282,
    "a6dd9dfeaf8d33f0e3cb1483e116e085db4b988d4510bed1f2460d34cc1442c1"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-0/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-0/stdout": [
    1275,
    "e70f2d7235dbd50455554f2a277613971e2fcdf117ec1bdcdf47644bcf49c422"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-1-topology.json": [
    2872,
    "63b2899b5d3d93df727ae82f0d98a750db686d30ac4a9844bd4f1be2ed7dd632"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-1/command.json": [
    695,
    "b0a6b5b327184f44f4f77797153cc68669d12483fd95b0cea0f4528d2e146ff2"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-1/result.json": [
    1665,
    "69c2ac442033a5f660585cccf83316b9641288016040777918cc5e3b62255166"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-1/started.json": [
    282,
    "a7efb72174275b204bc84168bcea772233a89d04825318a970243514c1705fcc"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-1/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-1/stdout": [
    1275,
    "e70f2d7235dbd50455554f2a277613971e2fcdf117ec1bdcdf47644bcf49c422"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-2-topology.json": [
    2872,
    "63b2899b5d3d93df727ae82f0d98a750db686d30ac4a9844bd4f1be2ed7dd632"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-2/command.json": [
    695,
    "b0a6b5b327184f44f4f77797153cc68669d12483fd95b0cea0f4528d2e146ff2"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-2/result.json": [
    1666,
    "94afb494fcfcec6da98c47ff6d420ea90d18268e329a51558a3465e4fa33df49"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-2/started.json": [
    282,
    "51f68e81c94a369d6b953b73bf977d3f6d000fd8a5f08f82d232879a9e45a99e"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-2/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/before-2/stdout": [
    1275,
    "e70f2d7235dbd50455554f2a277613971e2fcdf117ec1bdcdf47644bcf49c422"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/complete.json": [
    171456,
    "5b9617aba0b588c33923c5cc458b013d0929828be5635ea75f3fd702a12c2cd9"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/initial-topology.json": [
    2872,
    "63b2899b5d3d93df727ae82f0d98a750db686d30ac4a9844bd4f1be2ed7dd632"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/native/capture-0.bin": [
    849800,
    "950709f4b1b3a8114ba3fdafc8431cb27b3ac954b9dc276567ca4ba78733f748"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/native/capture-16.bin": [
    849800,
    "1c53a1bcb33fce8667badbcdbff341f4a7a5bf2ba445ccdec40d4b47c187f541"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/native/capture-39.bin": [
    849800,
    "44cba4c5c203e2a17aa804257aa571611357da7d27a147df5cf356a6f08aa0a6"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/native/capture-5.bin": [
    849800,
    "f0fbe98f9b1c01fafd0f70fef346408d108cf2616e73690b7b3b697a6ebead3b"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/native/child-stderr.bin": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/native/complete.json": [
    21394,
    "1ac8dda60bc2234fa640b5cb1c851868bebb1e44898ae4a76ea8cf45138f6078"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/native/frames.ndjson": [
    134227,
    "24842304b8f67b82e46cf135cae1841c14765ec3482c0368bb77153e6666f9f2"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/observation.json": [
    93706,
    "2e466f2c0bdcaee6eaad16838d28aa91410dccb0ad4596442871f7fdd9690405"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/parent-ldd/command.json": [
    851,
    "2a52e881e5843d8138e9dff2a518150c6f9cf61d09120c4b6cb03953bc8e1f88"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/parent-ldd/result.json": [
    1673,
    "f4d8eace1d4f8a1049a9ee6e161b533aaa0bc625ae1042afe9b3382e7bed464e"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/parent-ldd/started.json": [
    282,
    "3e8cc81377595b8d2462c459cae50ce75654022c0dc52c2230120a6d904f2731"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/parent-ldd/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/parent-ldd/stdout": [
    297,
    "c1d8bb44b0bcc2496ab9c0d167368463ace1fb5fa65514b3ca8aa3a38568ccff"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/parent-readelf/command.json": [
    875,
    "c1a846b709d5c5b8097b013d6857bd68c7f9044f7f6e9bf827d37c6275ed2cb8"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/parent-readelf/result.json": [
    1691,
    "24aceb9eb05204b8884156c0c34392bc70fef8be289688e8c7cc46e1f083080f"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/parent-readelf/started.json": [
    282,
    "879361d36de845956547fc925f9b5abba2f26a7a25fb2628c77073b34f30616d"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/parent-readelf/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/parent-readelf/stdout": [
    4713,
    "9239a9a1edf00888cb204798280e181dde837c005f6db9ff9d08157647ea5595"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/parent/command.json": [
    1090,
    "39ecf3e4f113dbe725de5ab51ef7787e9d663558a92f882862d95930ad3c1927"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/parent/result.json": [
    1938,
    "20f84d5d6dea8e14d892f9ae9d9ed76f2b8c999507819b8b53bcde11070dbd97"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/parent/started.json": [
    282,
    "c9d15fd384b4b0549bc6d5c47c7467fb2a756fb1151fadfca2a91b63730818d7"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/parent/stderr": [
    2427,
    "cf8749b00a09201306db2ded1fdfd04ae246ac3a009915469c85d041852b9a6a"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/parent/stdout": [
    21394,
    "1ac8dda60bc2234fa640b5cb1c851868bebb1e44898ae4a76ea8cf45138f6078"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/worker-ldd/command.json": [
    848,
    "9bb46cd65241f55c050c1c7cd91a3dfe32f38958f620a00998cc9ffe0471a411"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/worker-ldd/result.json": [
    1673,
    "6f91d718a2f49c1f5eae943f180d71514eaf8f28da6a68fba9a6c5107d82befd"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/worker-ldd/started.json": [
    282,
    "a8ba5a2938c0d16f32965836a2b45e219206bd668eb4acf41aec6cd7d3f0945a"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/worker-ldd/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/worker-ldd/stdout": [
    230,
    "9b0e8c8207feb3d18f5cee77e5a5b3518f278bcacd3215bcdaea8f45c6c06331"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/worker-readelf/command.json": [
    872,
    "2795997edb832a7994162f53cf3ad9e9bcf7ad259f114744de816123db92a8b6"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/worker-readelf/result.json": [
    1690,
    "4e5fa3789a3f8cb3d27ff709dd44224ac766fee72efc983ae5e7740632f52846"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/worker-readelf/started.json": [
    282,
    "a3c39ffcfb4c3d842c498521299e14533265ee2fb87649f4a758cc2bc400fbf0"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/worker-readelf/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/baseline/readiness/worker-readelf/stdout": [
    4310,
    "d5ca8d1fc63a0f112c4fd62c224fe53426b963b37b21855251a8ee4c3239a23f"
  ],
  "fixtures/tail-matched-v2/manifest.json": [
    616896,
    "70dbc23f2fddd83bb5406e111954e544033a2c9168877fc976895f1f74d7ea65"
  ],
  "fixtures/tail-matched-v2/prepared-inputs.json": [
    15746,
    "d024042be1fc79f3fa630751a06d692542484bf365d361c1d6479d75720aa5ec"
  ],
  "fixtures/tail-matched-v2/tail-input.json": [
    1681,
    "9c6eeb4259021879c82aae72e2c3f26359bbefc648f62829f3f24a739c49eefe"
  ],
  "fixtures/tail-matched-v2/tail-request.json": [
    9171,
    "430c00268bd29dbd0861e0e75b5e4470f8041bc19849ec1d616f71f3864f0b11"
  ],
  "fixtures/tail-matched-v2/tail/after-0-topology.json": [
    2872,
    "63b2899b5d3d93df727ae82f0d98a750db686d30ac4a9844bd4f1be2ed7dd632"
  ],
  "fixtures/tail-matched-v2/tail/after-0/command.json": [
    695,
    "b0a6b5b327184f44f4f77797153cc68669d12483fd95b0cea0f4528d2e146ff2"
  ],
  "fixtures/tail-matched-v2/tail/after-0/result.json": [
    1758,
    "1205a5c7809a055f1c7b98b6e5876fb4802b638b45d22a3457203cff492d0784"
  ],
  "fixtures/tail-matched-v2/tail/after-0/started.json": [
    282,
    "269a51d12d1d97fe46a18991a7a22562828209bd4425bea35aae4d3646b10b40"
  ],
  "fixtures/tail-matched-v2/tail/after-0/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/tail/after-0/stdout": [
    1275,
    "e70f2d7235dbd50455554f2a277613971e2fcdf117ec1bdcdf47644bcf49c422"
  ],
  "fixtures/tail-matched-v2/tail/after-1-topology.json": [
    2872,
    "63b2899b5d3d93df727ae82f0d98a750db686d30ac4a9844bd4f1be2ed7dd632"
  ],
  "fixtures/tail-matched-v2/tail/after-1/command.json": [
    695,
    "b0a6b5b327184f44f4f77797153cc68669d12483fd95b0cea0f4528d2e146ff2"
  ],
  "fixtures/tail-matched-v2/tail/after-1/result.json": [
    1758,
    "66b0c3038b39a5b161651931b65f7106a6081cc40c617137f37e8fd755312a52"
  ],
  "fixtures/tail-matched-v2/tail/after-1/started.json": [
    282,
    "7bd08e17051279ff3a519035710fb30dd875b17ab715ebaf36bb0370b2a86b37"
  ],
  "fixtures/tail-matched-v2/tail/after-1/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/tail/after-1/stdout": [
    1275,
    "e70f2d7235dbd50455554f2a277613971e2fcdf117ec1bdcdf47644bcf49c422"
  ],
  "fixtures/tail-matched-v2/tail/after-2-topology.json": [
    2872,
    "63b2899b5d3d93df727ae82f0d98a750db686d30ac4a9844bd4f1be2ed7dd632"
  ],
  "fixtures/tail-matched-v2/tail/after-2/command.json": [
    695,
    "b0a6b5b327184f44f4f77797153cc68669d12483fd95b0cea0f4528d2e146ff2"
  ],
  "fixtures/tail-matched-v2/tail/after-2/result.json": [
    1758,
    "0354adbef0d539ee4c30194c7bbde02f28de3aec3311a96cd81a53598d00f418"
  ],
  "fixtures/tail-matched-v2/tail/after-2/started.json": [
    282,
    "cf4756da37eff0e4036f7417e34348a47f8b6966f7ddf84b9efb57d4f1e6f4e6"
  ],
  "fixtures/tail-matched-v2/tail/after-2/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/tail/after-2/stdout": [
    1275,
    "e70f2d7235dbd50455554f2a277613971e2fcdf117ec1bdcdf47644bcf49c422"
  ],
  "fixtures/tail-matched-v2/tail/before-0-topology.json": [
    2872,
    "63b2899b5d3d93df727ae82f0d98a750db686d30ac4a9844bd4f1be2ed7dd632"
  ],
  "fixtures/tail-matched-v2/tail/before-0/command.json": [
    695,
    "b0a6b5b327184f44f4f77797153cc68669d12483fd95b0cea0f4528d2e146ff2"
  ],
  "fixtures/tail-matched-v2/tail/before-0/result.json": [
    1762,
    "f9d4120692901254bf8a6254d0a2fbca119d9b338ac93ea926cbb6b37ddc9a2e"
  ],
  "fixtures/tail-matched-v2/tail/before-0/started.json": [
    282,
    "ccc0be6ebfc627b5dc5409bc3dd8fc7b3642c456ae9564561d7495d0045efd90"
  ],
  "fixtures/tail-matched-v2/tail/before-0/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/tail/before-0/stdout": [
    1275,
    "e70f2d7235dbd50455554f2a277613971e2fcdf117ec1bdcdf47644bcf49c422"
  ],
  "fixtures/tail-matched-v2/tail/before-1-topology.json": [
    2872,
    "63b2899b5d3d93df727ae82f0d98a750db686d30ac4a9844bd4f1be2ed7dd632"
  ],
  "fixtures/tail-matched-v2/tail/before-1/command.json": [
    695,
    "b0a6b5b327184f44f4f77797153cc68669d12483fd95b0cea0f4528d2e146ff2"
  ],
  "fixtures/tail-matched-v2/tail/before-1/result.json": [
    1762,
    "4b15014c828066e6755d974c812a9c4399957a3b22a61a456913b8f7cf823b1c"
  ],
  "fixtures/tail-matched-v2/tail/before-1/started.json": [
    282,
    "d50c5c2a58636970957cfbe253ed1006ed731e99cf56b57de390ee600da6a768"
  ],
  "fixtures/tail-matched-v2/tail/before-1/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/tail/before-1/stdout": [
    1275,
    "e70f2d7235dbd50455554f2a277613971e2fcdf117ec1bdcdf47644bcf49c422"
  ],
  "fixtures/tail-matched-v2/tail/before-2-topology.json": [
    2872,
    "63b2899b5d3d93df727ae82f0d98a750db686d30ac4a9844bd4f1be2ed7dd632"
  ],
  "fixtures/tail-matched-v2/tail/before-2/command.json": [
    695,
    "b0a6b5b327184f44f4f77797153cc68669d12483fd95b0cea0f4528d2e146ff2"
  ],
  "fixtures/tail-matched-v2/tail/before-2/result.json": [
    1762,
    "1ea25b76369bdec5f76c642cbbba7d6f64e8b2fe7e44ae60ce63fb617d0d2fc6"
  ],
  "fixtures/tail-matched-v2/tail/before-2/started.json": [
    282,
    "8b747cbf6ddb06989bf1cca365951a1734d00e8c11bbb8df5529d6f60377f45c"
  ],
  "fixtures/tail-matched-v2/tail/before-2/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/tail/before-2/stdout": [
    1275,
    "e70f2d7235dbd50455554f2a277613971e2fcdf117ec1bdcdf47644bcf49c422"
  ],
  "fixtures/tail-matched-v2/tail/complete.json": [
    665487,
    "d43ed2c61c07c0714bf31cb258ca683a114458fd01f87ef32b398b43045ba9d6"
  ],
  "fixtures/tail-matched-v2/tail/initial-topology.json": [
    2872,
    "63b2899b5d3d93df727ae82f0d98a750db686d30ac4a9844bd4f1be2ed7dd632"
  ],
  "fixtures/tail-matched-v2/tail/matched.json": [
    144200,
    "eb7ef215eb516ed21de73da75d302ed3946bdcda0c53fc9cf48daeba26cdedb1"
  ],
  "fixtures/tail-matched-v2/tail/native/capture-0.bin": [
    849800,
    "a3f0ffa92a0d956eba96c549ef8685f1ad87f7cb204c13ac9650a4ed4eb0437d"
  ],
  "fixtures/tail-matched-v2/tail/native/capture-16.bin": [
    849800,
    "266080da1741b01ade48988f6f45b8ea9285a38a3ac770bd25780f1896e17ada"
  ],
  "fixtures/tail-matched-v2/tail/native/capture-39.bin": [
    849800,
    "78c838e0689ab55bb469cd047c60747b2f8399dcee2b49a48c9ca3fb70e31f71"
  ],
  "fixtures/tail-matched-v2/tail/native/capture-5.bin": [
    849800,
    "25685368677b843d7ad4994be25aceb130830e964a3cfce13df93d871e022612"
  ],
  "fixtures/tail-matched-v2/tail/native/child-stderr.bin": [
    2739,
    "f0a378424dedde87302386da9fc1b807baad863506ee2ba3a162e731078fc51f"
  ],
  "fixtures/tail-matched-v2/tail/native/complete.json": [
    21522,
    "8c44922659b3e0a0642f346da8c779d39ec5803584a14bff37dcebc006403743"
  ],
  "fixtures/tail-matched-v2/tail/native/frames.ndjson": [
    133961,
    "63fe7545c991d98291e9acb14b84f6d190cd7e82eb5b54dc560f2178db7a7b74"
  ],
  "fixtures/tail-matched-v2/tail/native/host-timing.json": [
    14517,
    "54805736bd6658dfad5d189207b26ada066e1929a0b11850531717e439c905b1"
  ],
  "fixtures/tail-matched-v2/tail/observation.json": [
    99248,
    "460a24eaa35ec6c90ec97218200db6b4a0fbeeb81cecb7cd0f3ba85c31f0c89d"
  ],
  "fixtures/tail-matched-v2/tail/pair.json": [
    144906,
    "65f90d68193308b9703960c387a78042a4d778e31e47ed79706456f2b45a0ef8"
  ],
  "fixtures/tail-matched-v2/tail/parent-ldd/command.json": [
    865,
    "b18f6d9827aee0ecd153d1379a8d3b9d9d514f249c6ebbe96516e52beef4dc90"
  ],
  "fixtures/tail-matched-v2/tail/parent-ldd/result.json": [
    1770,
    "fde52606f0b44a63e2a41d5ebe0b5e6e694e9f57eccddd2b338dba34797b8163"
  ],
  "fixtures/tail-matched-v2/tail/parent-ldd/started.json": [
    282,
    "e50db1eb1a05dac08f37c7487c04fd2706b92b3e177a94db1757e3aa04df5e61"
  ],
  "fixtures/tail-matched-v2/tail/parent-ldd/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/tail/parent-ldd/stdout": [
    297,
    "006967a260df0d794825a8f5b28cd902845500ac78f5184185e19b2b6dadb98d"
  ],
  "fixtures/tail-matched-v2/tail/parent-readelf/command.json": [
    889,
    "ccddc2326d215a9f80ad33b4ae299fadf33a5990777bbccff91af12195e7df93"
  ],
  "fixtures/tail-matched-v2/tail/parent-readelf/result.json": [
    1786,
    "c17d0b832ac9839817ed7c7e990777c64fab0deb53c3df597585741c4ffabe43"
  ],
  "fixtures/tail-matched-v2/tail/parent-readelf/started.json": [
    282,
    "efdcb36c948b7b50a77832f9c1c288eea94b5785d9d74770479bde15f55e30de"
  ],
  "fixtures/tail-matched-v2/tail/parent-readelf/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/tail/parent-readelf/stdout": [
    4713,
    "714aedd27317b3e8c08e66edc5afe8b1bea8e828d8b64c9f0015e35b855b1814"
  ],
  "fixtures/tail-matched-v2/tail/parent/command.json": [
    1167,
    "d36d3351ce290493e94ba94a829dbf77c963f7375313f1af2b1a557bf09cebb6"
  ],
  "fixtures/tail-matched-v2/tail/parent/result.json": [
    2033,
    "63f8a16a188ef2fa2cbe07992920273063998c974735fcba271b3334d22b31c4"
  ],
  "fixtures/tail-matched-v2/tail/parent/started.json": [
    282,
    "169bb22f17851dcf3213cab510ab1f9b549cfc0852ef15381a3f937bdb1a3535"
  ],
  "fixtures/tail-matched-v2/tail/parent/stderr": [
    2427,
    "b1e6e62704351dd2e8963ff49ae11511e98ee296a82b0c44a6f848d7ade7e678"
  ],
  "fixtures/tail-matched-v2/tail/parent/stdout": [
    24954,
    "bc3879e1504645d4671312c3928b6c280c433e53ce951a8b94b50e7c2be7f9e4"
  ],
  "fixtures/tail-matched-v2/tail/parity.json": [
    71501,
    "ddbfd5d64812f5ad8122fc374e5ff9dd4c5e33bac277a93e8b4fe8218aa405fe"
  ],
  "fixtures/tail-matched-v2/tail/worker-ldd/command.json": [
    831,
    "438733307e77d980f6b98bdf14bb7edf45ade4895af2360113634759f9d515c7"
  ],
  "fixtures/tail-matched-v2/tail/worker-ldd/result.json": [
    1770,
    "572b82ab0062557bbde1ca290f7df69b8b91058197669eded232a6ac517681b2"
  ],
  "fixtures/tail-matched-v2/tail/worker-ldd/started.json": [
    282,
    "a8bdba8deedb4e79bb91149ddfb31b2fb1b8d0d0cc8703a1eca59a51307abc2e"
  ],
  "fixtures/tail-matched-v2/tail/worker-ldd/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/tail/worker-ldd/stdout": [
    230,
    "65ab3d709cc43365d7caa65dbc8f87cf6d8faf6b785276f71ee48ddd86163ffa"
  ],
  "fixtures/tail-matched-v2/tail/worker-readelf/command.json": [
    855,
    "bde434578032549c882410bb343874f3c5c1e485e611e24d6469361927fd215d"
  ],
  "fixtures/tail-matched-v2/tail/worker-readelf/result.json": [
    1787,
    "ef7e00c2c226b44e87a8b33989d7ad0f82cd73fbdb1da26b81344227d9301eba"
  ],
  "fixtures/tail-matched-v2/tail/worker-readelf/started.json": [
    282,
    "78f96ae3125e443e7241895d3b9e0bb6e621366f6be4fe70792510060e2d959d"
  ],
  "fixtures/tail-matched-v2/tail/worker-readelf/stderr": [
    0,
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  ],
  "fixtures/tail-matched-v2/tail/worker-readelf/stdout": [
    4311,
    "decd75a20afb4bd9eb1e7e6de34956b0615b3750b0d69f9b453900e34471cb04"
  ],
  "frozen_owned.py": [
    30433,
    "ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583"
  ],
  "library_audit.py": [
    25872,
    "b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d"
  ],
  "prepare_model_inputs.py": [
    25356,
    "eb3369b70dec617372e4571038e3ad31a59c3a1dea309ce90d2d23030653ad77"
  ],
  "readiness_announcement.py": [
    3246,
    "b974ac6b6e936d8239639ac0c595c7db36700e3b1e2cff101224699fd789a9b1"
  ],
  "retention_tool.py": [
    45933,
    "d6761696a3cad5104baac7449e4af1292336d32e22c8ba1bb84b7bf8dfa751a2"
  ],
  "run_model_gpu.py": [
    54215,
    "2803e5783d9afaf2b58c704b2bf7dddc053ee44aca29a5c1fbf1a845c887daba"
  ],
  "supervisor.py": [
    41485,
    "8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc"
  ],
  "test_admission.py": [
    38896,
    "98d505bb5bf8b42cf955c76e47916094755f5687a0f9fa3a7a7d5071cb08bb4e"
  ],
  "test_bank_scoped.py": [
    14853,
    "3ecc622e660cb6e0790a18ff8bb022a84535c91466d02d6fdbb8625ae080a99b"
  ],
  "test_census.py": [
    19688,
    "aca29934c68523dbc91b30439a2c7a5ae552b64a9faf077ddb99b97c861cb5b0"
  ],
  "test_duration.py": [
    13609,
    "3f88f035e3f08644dadf393620557b84d7827876fb3e992e498fd6e2950d3fb5"
  ],
  "test_matched.py": [
    15400,
    "d5a6b522a1effe29e1ee08430754163cc217cf34c3713ce234926b7c860f9647"
  ],
  "test_readiness.py": [
    17802,
    "4b644462a71120ee45b3351757ecb0d5867c9453daadb6a10c07bdbcd38e8bb5"
  ],
  "test_retention_workflows.py": [
    27102,
    "281c5c50b077fcd702dbc763fb98cdc24eec9b5de069b6a64e82123ea59615b2"
  ],
  "test_scoped.py": [
    15936,
    "4c015cdfa072943960e0ffa1f25e0e604790e8dbe222875b4bed207be6563fdc"
  ],
  "test_scoped_pair.py": [
    3629,
    "aa95ff79fd3c72ee3658d66e4117738a06f7984a2a77dcc6712b28a272e57f8f"
  ],
  "test_shared.py": [
    10737,
    "a122f42f8a47a0213010f1812ef1f95fa7ff582db6a9ffa190356a35874f6b96"
  ],
  "test_tail.py": [
    19524,
    "2235e1c4b97b396094df484ccaedb25619f70c757ed1656e5eaf6b92f8140c4a"
  ],
  "test_timing.py": [
    10433,
    "630febf4d840081887692144cc570d70c1d67f9ad3bb5bab1fd6d7d1f61cedb1"
  ],
  "validate_bank_pair.py": [
    4428,
    "9d3f1470f6e9d7a17f3c91d1deb29a54c6fa7e8c766dc358e9d7d73e15c4e004"
  ],
  "validate_bank_scoped.py": [
    9534,
    "2432fc14c0ba40fc934abcd7886459c12fc31ca6167ba4cf908a8c7974d26772"
  ],
  "validate_census.py": [
    10392,
    "090431e59a481d5b681311d5a015ddfd35705cc828c920118d4f0043b862ec9f"
  ],
  "validate_census_pair.py": [
    4730,
    "d4f50afe412b68c4066e934a80b96b5a5c0f83ef8ea33c59c53380aa1ab819c4"
  ],
  "validate_duration.py": [
    9343,
    "d7ed85d14191e735490170a606a06cefb331e17b07c36a4a4d34ef38a5cf19bd"
  ],
  "validate_matched.py": [
    9362,
    "d31c94a4dd1f254d166be185fe1cca10bc4eed26e500a0cb1fc3c5a74a574437"
  ],
  "validate_readiness.py": [
    19800,
    "0c319e99142b19909350d9a81404948b494c2e3ea0defbbc165c2e5529be032a"
  ],
  "validate_scoped.py": [
    9184,
    "c2541e11e534cf1e5a4faade31fe4a734726552171fed9617d6ea938f680b6fd"
  ],
  "validate_scoped_pair.py": [
    3233,
    "495fc476454e4052467810b55e177f9d2b91d86d3496a6b90876963f7f60ac62"
  ],
  "validate_shared.py": [
    8015,
    "6557fe5c082b2c92bae15274dd0d19bba3da8c3c36b9fc74757b4c1b74a87eca"
  ],
  "validate_tail.py": [
    10447,
    "696e55d4bebea97ab82e5b613a76978c5e191087789d299551c304527e598d49"
  ],
  "validate_tail_pair.py": [
    4883,
    "ab69bcafcc245b8eca05d7e625c42eec7aaec2de4084e719090de1894f4d6995"
  ],
  "validate_timing.py": [
    10104,
    "27c652f5452676b4cc08a634241aff89b2653e85ee111133c1f1c4ab13efd93f"
  ],
  "run_cpu.py": [
    51485,
    "7f1c71302dd3e794887a3c0c44333ed34aa6e4a2c2c1b422d5e17410ee0f41dc"
  ]
})}
LEAVES = (
    ('readiness-tests',120,tuple(name for name in MODES['admission']['modules']
        if name not in ('test_admission','test_retention_workflows')),126),
    ('admission-tests',180,('test_admission','test_retention_workflows'),45),
)
RAW = {'sources-before.json','sources-after.json'} | {
    label + '.' + suffix for label,_,_,_ in LEAVES
    for suffix in ('command.json','started.json','result.json','stdout','stderr')}
FILE_LIMIT, TOTAL_LIMIT = 4 << 20, 32 << 20

def require(ok, why):
    if not ok:
        raise ValueError(why)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def parse(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def read(path, cap=FILE_LIMIT):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= cap, 'bounded regular input')
        raw = stream.read(cap + 1)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size, 'input drift')
    return raw


def locations(mode):
    require(mode in MODES, 'closed CPU gate mode')
    return (E / 'guarded-mlp-currentness-duration-native-admission-cpu-v228-v1',
            'guarded-mlp-currentness-duration-native-admission-cpu-evidence-v228-v1.tar.gz',
            Q / 'native-admission-cpu-v1')

def terminal(bodies):
    names = [n for n in ('complete.json', 'failed.json') if 'evidence/' + n in bodies]
    require(len(names) == 1, 'one original outcome, never both or invented')
    return names[0]


def validate(mode, bodies, terminal_sha):
    contract = MODES[mode]
    root, _, _ = locations(mode)
    name = terminal(bodies)
    raw_terminal = bodies['evidence/' + name]
    require(pin(raw_terminal)['sha256'] == terminal_sha, 'observed original terminal SHA')
    c = parse(raw_terminal)
    require(type(c['passed']) is bool and c['schema'] == 'ferric-currentness-duration-native-admission-cpu-v1'
            and (name == 'complete.json') == c['passed']
            and (c['failure'] is None) == c['passed']
            and (c['passed'] or type(c['failure']) is str)
            and type(c['postcheck_errors']) is list
            and all(type(x) is str for x in c['postcheck_errors']), 'honest original CPU outcome')
    require(type(c['raw']) is dict and set(c['raw']) <= RAW, 'closed original raw prefix')
    original = set(contract['sources']) | {'evidence/' + n for n in set(c['raw']) | {name}}
    require(set(bodies) == original | {'pure-evidence.py'}, 'exact original prefix and helper')
    require(set(c['sources_before']) == set(contract['sources']), 'original source/fixture roster')
    for source, expected in contract['sources'].items():
        require([len(bodies[source]), pin(bodies[source])['sha256']] == expected
                and compact(c['sources_before'][source]) == pin(bodies[source])
                and c['sources_before'][source]['path'] == str(root / source), 'fixed source-before join')
    after = c['sources_after']
    require(after is None or after == c['sources_before'], 'no changed input relabeled as qualified')
    require(c['source_unchanged'] is (after == c['sources_before']), 'original source postcheck metadata')
    require(c['controller'] == c['sources_before']['run_cpu.py']
            and c['supervisor'] == c['sources_before']['supervisor.py'], 'harness source joins')
    for raw_name, row in c['raw'].items():
        require(pin(bodies['evidence/' + raw_name]) == compact(row)
                and row['path'] == str(root / 'evidence' / raw_name), 'raw pin/path join')
    for label in ('before','after'):
        raw_name = 'sources-' + label + '.json'
        if raw_name in c['raw']:
            require(parse(bodies['evidence/' + raw_name]) == c['sources_' + label], 'original source map body')
    require(type(c['phases']) is list and len(c['phases']) <= 2, 'at most two original owned children')
    require(type(c['leaf_tests']) is dict and set(c['leaf_tests']) <= {row[0] for row in LEAVES},
            'closed original per-leaf census')
    require(set(c['tool_pins']) == {'python','prlimit'}, 'closed original tool metadata')
    expected_environment = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8',
        LC_ALL='C.UTF-8', TZ='UTC', PYTHONPATH='', PYTHONDONTWRITEBYTECODE='1', PYTHONNOUSERSITE='1',
        TMPDIR=str(root / 'tmp'), HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='',
        CARGO_BUILD_JOBS='2', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1')
    require(c['environment'] == expected_environment, 'fixed GPU-hidden CPU environment')
    names_by_module = {}
    for module, expected_classes in contract['modules'].items():
        tree = ast.parse(bodies[module + '.py'])
        classes = [node for node in tree.body if isinstance(node, ast.ClassDef)
                   and any(isinstance(item, ast.FunctionDef) and item.name.startswith('test_') for item in node.body)]
        require({node.name for node in classes} == set(expected_classes)
                and len(classes) == len(expected_classes), 'exact synthetic test classes')
        names_by_module[module] = [module + '.' + cls.name + '.' + node.name
            for cls in classes for node in cls.body
            if isinstance(node, ast.FunctionDef) and node.name.startswith('test_')]
    all_names = sorted(name for names in names_by_module.values() for name in names)
    require(len(all_names) == len(set(all_names)) == 171, 'fixed171 source-declared test names')
    for index,(label,seconds,modules,count) in enumerate(LEAVES):
        phase = c['phases'][index] if index < len(c['phases']) else None
        phase_names = {label + '.' + suffix for suffix in
                       ('command.json','started.json','result.json','stdout','stderr')}
        if index > len(c['phases']):
            require(not (phase_names & set(c['raw'])), 'no raw body after an unattempted leaf')
        if index > 0 and (phase is not None or phase_names & set(c['raw'])):
            require(LEAVES[index-1][0] in c['leaf_tests'], 'second leaf follows admitted first leaf')
        script = ('import sys,unittest;sys.path.insert(0,' + repr(str(root)) + ');'
            'suite=unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromName(name) '
            'for name in ' + repr(modules) + ');'
            'result=unittest.TextTestRunner(verbosity=2).run(suite);sys.exit(not result.wasSuccessful())')
        argv = ['/usr/bin/prlimit','--as=' + str(512 << 20),'--cpu=' + str(seconds),
                '--fsize=' + str(16 << 20),'--core=0','--',
                c['tool_pins']['python']['path'],'-I','-B','-c',script]
        if phase is not None:
            require(phase['label'] == label and phase['argv'] == argv
                    and type(phase['exit_code']) in (int,type(None))
                    and all(type(phase[k]) is bool for k in ('natural_exit','reaped','process_group_absent',
                        'forced_cleanup','timed_out')), 'original ordered phase and owned lifecycle')
            for key,suffix in (('command','.command.json'),('stdout','.stdout'),('stderr','.stderr')):
                if label + suffix in c['raw']:
                    require(phase[key] == c['raw'][label + suffix], 'original phase stream/command pin')
            if label + '.result.json' in c['raw']:
                require(parse(bodies['evidence/' + label + '.result.json']) == phase, 'original result join')
            if label + '.started.json' in c['raw']:
                require(parse(bodies['evidence/' + label + '.started.json']) ==
                        dict(pid=phase['pid'],pgid=phase['pgid'],argv=phase['argv']), 'original owned registration')
        else:
            require(label + '.result.json' not in c['raw'], 'no result body without recorded phase')
        if label + '.command.json' in c['raw']:
            command = parse(bodies['evidence/' + label + '.command.json'])
            require(command['cwd'] == str(root) and command['env'] == expected_environment
                    and command['argv'] == argv and type(command['wall_timeout_seconds']) in (int,float)
                    and 0 < command['wall_timeout_seconds'] <= seconds, 'original bounded command')
        expected = sorted(name for module in modules for name in names_by_module[module])
        require(len(expected) == count, 'exact per-leaf source roster')
        if label in c['leaf_tests']:
            require(phase is not None and phase['exit_code'] == 0 and phase['natural_exit'] is True
                    and phase['reaped'] is True and phase['process_group_absent'] is True
                    and phase['forced_cleanup'] is False and phase['timed_out'] is False
                    and phase['exception'] is None and phase['storage_failure'] is None
                    and phase['observed_signals'] == [] and phase_names <= set(c['raw']),
                    'admitted census requires clean original owned child')
            require(bodies['evidence/' + label + '.stdout'] == b'', 'empty test stdout')
            prefixes = '|'.join(re.escape(module + '.' + cls)
                               for module in modules for cls in contract['modules'][module])
            expression = r'^(test_[A-Za-z0-9_]+) \((' + prefixes + r')\.\1\) \.\.\. ok$'
            stderr = bodies['evidence/' + label + '.stderr'].decode()
            observed = [prefix + '.' + test for test,prefix in re.findall(expression,stderr,re.M)]
            require(len(observed) == len(set(observed)) == count and sorted(observed) == expected,
                    'all exact named tests passed')
            tail = '\n'.join(line for line in re.sub(expression,'',stderr,flags=re.M).splitlines() if line)
            require(re.fullmatch(r'-{70}\nRan ' + str(count) + r' tests in [0-9]+\.[0-9]+s\nOK',tail),
                    'original complete unittest summary')
            require(c['leaf_tests'][label] == dict(names=expected,passed=count,failed=0,errors=0,skipped=0),
                    'exact per-leaf admitted census')
    if c['tests'] is not None:
        require(set(c['leaf_tests']) == {row[0] for row in LEAVES}
                and c['tests'] == dict(names=all_names,passed=171,failed=0,errors=0,skipped=0),
                'aggregate census only after both admitted leaves')
    if c['passed']:
        require(c['postcheck_errors'] == [] and c['tests'] is not None and after == c['sources_before']
                and set(c['raw']) == RAW and len(bodies) == 199, 'successful198 run originals plus helper')
    require(c['limits'] == dict(whole_seconds=420,leaf_seconds={'readiness-tests':120,'admission-tests':180},
        cleanup_reserve_seconds=50,address_space_bytes=512 << 20,file_bytes=16 << 20,stream_bytes=4 << 20,
        temporary_bytes=64 << 20,affinity=[8,9],nice=10,cargo_jobs=2,
        initial_free_bytes=40 << 30,live_free_bytes=38 << 30), 'qualification bounds unchanged within each leaf')
    require(c['synthetic_data_tests_only'] is True and c['inherited_checker_tests'] == 126
            and c['admission_workflow_tests'] == 45 and c['original_fixture_files'] == 152
            and all(c[k] is False for k in ('live_admission_included','gpu_execution','native_parent_execution',
                'model_execution','numerical_acceptance','full_model_acceptance','performance_claim','production_authority')),
            'pure CPU workflow scope, no execution authority')
    return c,{name:pin(body) for name,body in sorted(bodies.items())}

def export_manifest(mode, c, bodies, pins):
    return dict(schema='ferric-currentness-duration-native-admission-cpu-export-v1', mode=mode, files=pins,
        original_files=len(bodies) - 1, selected_files=len(bodies), terminal_name=terminal(bodies),
        terminal=pin(bodies['evidence/' + terminal(bodies)]), passed=c['passed'],
        admitted_test_count=c['tests']['passed'] if c['tests'] is not None else None,
        original_failure=c['failure'], original_postcheck_errors=c['postcheck_errors'],
        admitted_leaf_counts={label:row['passed'] for label,row in c['leaf_tests'].items()},
        synthetic_only=True, new_project_execution=False, native_execution=False,
        numerical_acceptance=False, performance_claim=False,
        original_tools_rehashed_at_export=True, external_tools_rehashed_locally=False)


def export(mode, terminal_sha):
    root, archive_name, _ = locations(mode)
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'actual CPU host')
    output = E / archive_name
    require(not os.path.lexists(output) and root.resolve(strict=True) == root, 'fresh output and canonical root')
    require({p.relative_to(root).as_posix() for p in root.rglob('*') if not p.is_dir()
             and p.relative_to(root).parts[0] not in ('evidence','tmp')} == set(MODES[mode]['sources']),
            'closed live source/fixture root')
    observed = {p.name for p in (root / 'evidence').iterdir()}
    outcomes = observed & {'complete.json', 'failed.json'}
    require(len(outcomes) == 1 and observed - outcomes <= RAW, 'one original outcome and closed raw prefix')
    paths = {name: root / name for name in MODES[mode]['sources']}
    paths.update({'evidence/' + name: root / 'evidence' / name for name in observed})
    paths['pure-evidence.py'] = Path(__file__).resolve()
    bodies = {name: read(path) for name, path in paths.items()}
    c, pins = validate(mode, bodies, terminal_sha)
    for row in c['tool_pins'].values():
        require(pin(read(Path(row['path']), 16 << 20)) == compact(row), 'original Python/prlimit live tool pin')
    manifest = export_manifest(mode, c, bodies, pins)
    packaged = dict(bodies, **{'manifest.json': encoded(manifest)})
    require(len(packaged) <= 200 and sum(map(len, packaged.values())) <= TOTAL_LIMIT, 'bounded two-hundred-member prefix')
    with output.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, raw in sorted(packaged.items()):
                member = tarfile.TarInfo(name)
                member.size, member.mode, member.mtime = len(raw), 0o644, 0
                tar.addfile(member, io.BytesIO(raw))
        stream.flush(); os.fsync(stream.fileno())
    require(all(read(path) == bodies[name] for name, path in paths.items()), 'all selected body posthashes')
    for row in c['tool_pins'].values():
        require(pin(read(Path(row['path']), 16 << 20)) == compact(row), 'original tool posthash')
    print(json.dumps(dict(archive=dict(path=str(output), **pin(read(output, TOTAL_LIMIT))),
        members=len(packaged), original_files=len(bodies) - 1, passed=c['passed'],
        admitted_tests=manifest['admitted_test_count']), sort_keys=True))


def retain(mode, archive_sha, terminal_sha):
    _, archive_name, dest = locations(mode)
    archive = W / archive_name
    raw = read(archive, TOTAL_LIMIT)
    require(pin(raw)['sha256'] == archive_sha, 'observed archive SHA')
    allowed = set(MODES[mode]['sources']) | {'evidence/' + n for n in RAW | {'complete.json', 'failed.json'}} | {'pure-evidence.py', 'manifest.json'}
    bodies, total = {}, 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as tar:
        for member in tar:
            require(member.name in allowed and member.name not in bodies and member.isfile()
                    and not member.pax_headers and 0 <= member.size <= FILE_LIMIT, 'closed ordinary member')
            total += member.size
            require(total <= TOTAL_LIMIT and len(bodies) < 200, 'bounded original archive prefix')
            bodies[member.name] = tar.extractfile(member).read(member.size + 1)
            require(len(bodies[member.name]) == member.size, 'complete original body')
    require('manifest.json' in bodies, 'original manifest present')
    manifest_raw = bodies.pop('manifest.json')
    manifest = parse(manifest_raw)
    c, pins = validate(mode, bodies, terminal_sha)
    require(bodies['pure-evidence.py'] == read(Path(__file__).resolve())
            and manifest == export_manifest(mode, c, bodies, pins), 'exact helper/export manifest')
    require(read(archive, TOTAL_LIMIT) == raw and dest.parent.resolve(strict=True) == dest.parent
            and not os.path.lexists(dest), 'archive posthash and fresh retention')
    bodies['manifest.json'] = manifest_raw
    dest.mkdir(mode=0o755)
    for name, body in sorted(bodies.items()):
        target = dest / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(body); stream.flush(); os.fsync(stream.fileno())
        target.chmod(0o644)
        require(read(target) == body, 'retained original readback')
    require(read(archive, TOTAL_LIMIT) == raw, 'archive unchanged after publication')
    report = dict(schema='ferric-currentness-duration-native-admission-cpu-retention-v1', mode=mode,
        archive=pin(raw), files={name: pin(body) for name, body in sorted(bodies.items())},
        original_files=len(bodies) - 2, passed=c['passed'], admitted_tests=manifest['admitted_test_count'],
        original_terminal_unchanged=True, external_tools_rehashed_locally=False,
        new_project_execution=False, native_execution=False, numerical_acceptance=False, performance_claim=False)
    with (dest / 'retention.json').open('xb') as stream:
        stream.write(encoded(report)); stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(dict(destination=str(dest), original_files=len(bodies) - 2,
        passed=c['passed'], admitted_tests=manifest['admitted_test_count'])))


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) in (4, 5),
            'python3 -B pure-evidence.py export MODE TERMINAL_SHA | retain MODE ARCHIVE_SHA TERMINAL_SHA')
    require(sys.argv[2] in MODES and all(re.fullmatch('[0-9a-f]{64}', value) for value in sys.argv[3:]), 'closed mode and observed hashes')
    os.umask(0o077)
    for kind, cap in ((resource.RLIMIT_AS, 256 << 20), (resource.RLIMIT_FSIZE, TOTAL_LIMIT), (resource.RLIMIT_CORE, 0)):
        limits = resource.getrlimit(kind)
        value = min([cap] + [n for n in limits if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (value, value))
    def interrupted(number, _frame):
        raise RuntimeError('pure evidence signal ' + str(number))
    for number in (signal.SIGALRM, signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, interrupted)
    signal.alarm(30)
    try:
        if sys.argv[1] == 'export' and len(sys.argv) == 4:
            export(sys.argv[2], sys.argv[3])
        else:
            require(sys.argv[1] == 'retain' and len(sys.argv) == 5, 'closed retention invocation')
            retain(sys.argv[2], sys.argv[3], sys.argv[4])
    finally:
        signal.alarm(0)
