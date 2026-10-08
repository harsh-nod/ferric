"""Two bounded GPU-hidden leaves: 126 unchanged checker tests and 45 admission workflows."""
import ast
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import time
import types

ROOT = Path(__file__).resolve().parent
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-currentness-duration-native-admission-cpu-v228-v1')
OUT = ROOT / 'evidence'
TMP = ROOT / 'tmp'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
SOURCES = {
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
  ]
}
ROSTERS = {
  "test_readiness": {
    "ReadinessTests": [
      "test_all144_pages_and_page_boundary_are_closed",
      "test_authentic_prompt_only_not_argmax_feedback",
      "test_bank_retirement_local_generation_and_bool_are_checked",
      "test_capture_path_digest_order_and_truncation",
      "test_close_generated_tokens_and_native_retirement",
      "test_complete40_original_chain_and_four_selected_payloads",
      "test_extra_fields_authority_and_observer_stderr_refused",
      "test_frame_order_missing_and_extra_are_refused",
      "test_full2303_and_legacy_profiles_are_refused",
      "test_marker_alone_never_substitutes_owned_lineage",
      "test_original_readiness_profile_and_schema_are_refused",
      "test_payload_finite_lowest_tie_and_signed_zero",
      "test_readiness_marker_is_distinct_and_unique",
      "test_selected_guard_and_queue_words_are_independently_checked",
      "test_stream_and_retention_accounting_are_exact",
      "test_strict_json_scalars_and_boolean_identity",
      "test_unselected_hashes_and_chains_remain_authenticated"
    ]
  },
  "test_shared": {
    "SharedTests": [
      "test_canonical_single_record_no_suffix_duplicates_or_reordering",
      "test_complete_policy_keeps_original_stderr_and_ordinary_contract",
      "test_default_empty_stderr_and_shared_opt_in_are_not_interchangeable",
      "test_every_policy_flag_is_closed_and_bool_is_not_integer",
      "test_exact_extent_captures_schema_and_no_extra_fields",
      "test_identity_scope_hash_and_device_order_are_joined",
      "test_parity_compares_all40_records_and_all_four_payloads",
      "test_parity_ignores_only_independently_admitted_control_timer_bytes",
      "test_parity_refuses_same_session_model_drift_or_numerical_authority",
      "test_pinned_original_bytes_and_all40_chain_still_required",
      "test_real_close_and_position5_only_precede_policy_admission",
      "test_wrapper_cannot_relabel_ordinary_timing_or_different_policy"
    ]
  },
  "test_timing": {
    "TimingTests": [
      "test_close_and_false_authority_are_required",
      "test_complete_timeline_zero_spans_and_large_integer_precision",
      "test_exact_retention_accounting_and_unchanged_bound",
      "test_gaps_overlap_regression_and_duration_are_refused",
      "test_order_extent_and_forward_sum_are_closed",
      "test_parity_accepts_distinct_sessions_and_rejects_any_record_change",
      "test_parity_compares_payload_not_control_timer_bytes",
      "test_parity_rejects_same_session_model_drift_and_authority",
      "test_report_hashes_scope_and_capture_set_are_joined",
      "test_sidecar_actual_body_path_hash_and_size_are_joined",
      "test_strict_u64_refuses_bool_float_negative_and_overflow",
      "test_wrapper_and_ordinary_admission_are_not_interchangeable"
    ]
  },
  "test_matched": {
    "MatchedTests": [
      "test_both_original_modes_join_policy_timeline_and_false_authority",
      "test_combined_policy_joins_original_worker_transcript_and_flags",
      "test_default_empty_stderr_and_shared_original_record_remain_separate",
      "test_explicit_mode_and_exact_wrapper_cannot_be_inferred_or_relabelled",
      "test_full_ordinary_frames_captures_and_close_precede_timing",
      "test_matched_pair_requires_same_all_four_cpu_and_product_pins",
      "test_original_policy_missing_suffix_changed_bytes_and_budgets_refuse",
      "test_pair_checks_every_semantic_record_and_complete_selected_payload",
      "test_pair_refuses_same_worker_identity_or_same_mode",
      "test_shared_sidecar_actual_path_hash_extent_and_ordinary_pin",
      "test_shared_timeline_strict_scalars_order_and_exact_disjoint_sum",
      "test_shared_timing_retention_close_and_false_authority_are_not_relaxed"
    ]
  },
  "test_scoped": {
    "ScopedTests": [
      "test_counter_census_and_checked_relations_refuse_missing_warm_calls",
      "test_counter_scalars_are_exact_unsigned_integers_not_bool_float_or_overflow",
      "test_default_shared_and_scoped_policy_routes_are_not_interchangeable",
      "test_every_policy_flag_and_execution_identity_is_closed",
      "test_explicit_wrapper_and_mode_cannot_relabel_ordinary_or_shared",
      "test_large_valid_poll_dependent_counters_preserve_all_integer_bits",
      "test_ordinary_and_timed_scoped_keep_original_policy_bytes_without_shared_label",
      "test_original_pinned_stderr_all40_frames_four_captures_and_retention_stay_required",
      "test_parity_checks_every_semantic_record_and_all_four_complete_payloads",
      "test_parity_rejects_relabelled_policy_or_another_admitted_summary",
      "test_policy_requires_single_canonical_ordered_bounded_record",
      "test_real_close_and_position5_are_required_not_full_or_old_readiness",
      "test_record_joins_bootstrap_session_devices_worker_profile_and_transcript",
      "test_timed_route_preserves_all124_spans_exact_pins_caps_and_false_authority"
    ]
  },
  "test_scoped_pair": {
    "ScopedPairTests": [
      "test_pair_rejects_wrong_mode_summary_timeline_and_original_policy_body",
      "test_pair_requires_each_same_actual_cpu_and_product_pin",
      "test_same_binary_pair_preserves_original_policy_and_host_timing_without_authority"
    ]
  },
  "test_bank_scoped": {
    "BankScopedTests": [
      "test_all_policy_flags_identity_close_and_profile_remain_closed",
      "test_bank_policy_is_one_canonical_bounded_record_not_v1_or_shared",
      "test_bank_scalar_types_generations_and_every_counter_relation_are_strict",
      "test_large_integer_counters_remain_exact_and_overflow_is_refused",
      "test_layer_counts_cannot_be_replaced_by_bank_counts_or_weakened",
      "test_original40_chain_payloads_policy_accounting_and_timing_are_required",
      "test_original_bank_record_retains_separate_layers_banks_and124_spans",
      "test_pair_checks_every_semantic_record_and_complete_selected_payload",
      "test_pair_rechecks_both_original_policy_files_and_no_relabelled_authority",
      "test_pair_requires_same_all_cpu_product_pins_and_distinct_cases",
      "test_same_elf_scoped_control_and_bank_candidate_preserve_both_original_policies",
      "test_timing_byte_cap_and_native_close_cannot_be_upgraded_by_wrapper"
    ]
  },
  "test_census": {
    "CensusTests": [
      "test_census_all_policy_flags_identity_close_and_profile_remain_closed",
      "test_census_bank_policy_is_one_canonical_bounded_record_not_v1_or_shared",
      "test_census_bank_scalar_types_generations_and_every_counter_relation_are_strict",
      "test_census_every_subset_scalar_and_nested_field_is_strict",
      "test_census_exact_owner_assertion_is_ranked_non_authoritative_and_not_equalized",
      "test_census_large_integer_counters_remain_exact_and_overflow_is_refused",
      "test_census_layer_counts_cannot_be_replaced_by_bank_counts_or_weakened",
      "test_census_original40_chain_payloads_policy_accounting_and_timing_are_required",
      "test_census_original_bank_record_retains_separate_layers_banks_and124_spans",
      "test_census_pair_checks_every_semantic_record_and_complete_selected_payload",
      "test_census_pair_rechecks_both_original_policy_files_and_no_relabelled_authority",
      "test_census_pair_refuses_subset_relabel_and_each_forward_timeline_gap",
      "test_census_pair_requires_same_all_cpu_product_pins_and_distinct_cases",
      "test_census_same_elf_bank_control_and_census_candidate_preserve_both_original_policies",
      "test_census_subset_is_not_added_twice_and_remaining_layer_work_is_required",
      "test_census_timing_byte_cap_and_native_close_cannot_be_upgraded_by_wrapper"
    ]
  },
  "test_tail": {
    "TailTests": [
      "test_tail_all_counter_fields_and_fixed_extent_relations_are_closed",
      "test_tail_and_old_policy_routes_cannot_admit_each_other",
      "test_tail_case_and_pair_nonclaims_cannot_be_promoted_to_native_acceptance",
      "test_tail_counts_never_replace_or_relax_inherited_layer_bank_census",
      "test_tail_one_original_canonical_bounded_record_is_required",
      "test_tail_original40_chain_capture_accounting_and_timing_are_required",
      "test_tail_original_record_keeps_separate_counters_bytes_and124_spans",
      "test_tail_pair_checks_all40_semantic_records_and_four_complete_payloads",
      "test_tail_pair_preserves_original_v3_v4_policies_and_exact_payload_parity",
      "test_tail_pair_rechecks_both_original_files_and_policy_authority",
      "test_tail_pair_requires_all_four_same_cpu_product_identities",
      "test_tail_same_side_requires_independent_scope_flags_and_rehashes_policy",
      "test_tail_scalar_types_widths_and_checked_overflow_are_strict",
      "test_tail_scope_flags_identities_close_and_profile_cannot_be_relabelled",
      "test_tail_variable_periodic_checks_and_large_integers_are_exact",
      "test_tail_wrapper_cannot_replace_policy_or_timing_and_bounds"
    ]
  },
  "test_duration": {
    "DurationTests": [
      "test_call_and_duration_types_widths_bounds_and_bank_containment",
      "test_complete_case_retains_both_original_records_and124_spans",
      "test_every_category_count_joins_each_original_scope",
      "test_every_identity_and_claim_flag_is_bound_to_healthy_policy",
      "test_exact_two_canonical_bounded_original_lines",
      "test_fixed_order_presence_and_unmeasured_first_two_forwards",
      "test_full_case_rechecks_original_files_capture_and_timing",
      "test_large_integer_variable_poll_counts_remain_exact",
      "test_original_policy_and_instrumented_routes_cannot_admit_each_other",
      "test_per_forward_bank_census_cannot_cancel_between_rows",
      "test_same_side_comparison_rechecks_both_diagnostic_copies_and_original_file",
      "test_unknown_fields_and_noncanonical_nested_key_order_are_rejected"
    ]
  },
  "test_admission": {
    "AdmissionTests": [
      "test_both_diagnostic_build_and_no_prior_native_authority_are_required",
      "test_default_or_mixed_modes_are_not_diagnostic_admission",
      "test_exact_repaired_generations_are_required",
      "test_matching_diagnostic_extension_is_pure_and_shared",
      "test_only_explicit_diagnostic_case_is_selected_before_io",
      "test_original_runtime_and_repaired_worker_lineage_cannot_be_substituted",
      "test_outer_runner_refuses_pending_cpu_and_product_bindings_before_reads",
      "test_parent_feature_selection_and_artifact_are_both_bound",
      "test_parent_joins_exact_worker_receipt_and_final_source_map",
      "test_retainer_refuses_unbound_sources_instead_of_inventing_success",
      "test_selected_runtime_feature_vector_is_exact",
      "test_single_case_raw_roster_and_all_inherited_bounds_remain_fixed",
      "test_worker_feature_request_and_cargo_product_must_agree",
      "test_worker_mode_source_and_feature_flags_are_exact_booleans"
    ],
    "CpuAdmissionWorkflowTests": [
      "test_all_cpu_phase_ownership_deadline_and_retirement_checks_are_active",
      "test_cross_mode_generation_and_lineage_are_rejected_after_hash_admission",
      "test_every_bound_product_receipt_and_map_pin_is_checked",
      "test_fully_bound_success_reads_both_receipts_maps_controllers_inputs_and_elfs",
      "test_named_inventory_selected_counts_and_source_extents_are_not_relaxed",
      "test_original_source_map_bytes_and_all236_worker_identities_are_required",
      "test_product_features_cargo_target_and_pretest_worker_identity_are_checked",
      "test_rebound_non_elf_product_bytes_still_fail_elf_admission",
      "test_source_unchanged_and_postcheck_status_cannot_be_forged",
      "test_wrong_cpu_root_and_controller_identity_are_rejected"
    ],
    "PreparerWorkflowTests": [
      "test_full_main_reads_real_fixture_files_and_emits_exact_original_three_body_roster",
      "test_main_checks_natural_reaped_absent_qualification_ownership",
      "test_main_keeps_fixed_image_and_all2048_prompt_hash_joins_active",
      "test_main_preserves_original_device_and_deadline_rejections",
      "test_main_refuses_default_parent_even_when_original_receipt_hash_is_rebound",
      "test_main_refuses_original_receipt_map_and_product_byte_substitution",
      "test_main_refuses_pending_parent_tuple_before_reading_qualification",
      "test_main_rejects_reused_session_and_nonfresh_output_namespace"
    ]
  },
  "test_retention_workflows": {
    "RetentionWorkflowTests": [
      "test_absent_case_verify_and_retain_without_invented_terminal",
      "test_capture_and_transcript_original_body_drift_are_refused",
      "test_changed_first_policy_cannot_hide_behind_consistent_second_sha",
      "test_complete_success_verify_and_retain_original_byte_closure",
      "test_failed_prefix_verify_and_retain_without_success_promotion",
      "test_healthy_close_and_owned_worker_ancestry_are_not_metadata_only",
      "test_late_failed_case_preserves_completed_raw_without_relabelling",
      "test_missing_extra_and_wrong_observed_outcomes_are_refused",
      "test_original_stderr_drift_fails_even_with_outer_raw_pin_refreshed",
      "test_retain_rejects_wrong_archive_hash_duplicate_escape_and_manifest_observation",
      "test_source_map_binding_and_readset_custody_cannot_be_substituted",
      "test_success_retention_refuses_an_existing_destination",
      "test_wrong_policy_sha_and_relabelled_policy_refused_after_full_repin"
    ]
  }
}
LEAVES = (
    ('readiness-tests', 120, tuple(name for name in ROSTERS if name not in
        ('test_admission', 'test_retention_workflows')), 126),
    ('admission-tests', 180, ('test_admission', 'test_retention_workflows'), 45),
)
NAMES = tuple(sorted(module + '.' + cls + '.' + name
    for module, classes in ROSTERS.items() for cls, names in classes.items() for name in names))
SIGNALS = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)
WHOLE_SECONDS, CLEANUP_SECONDS = 420, 50

def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def load_supervisor():
    path = ROOT / 'supervisor.py'; before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_size <= 1 << 20, 'ordinary bounded supervisor')
    with path.open('rb') as stream:
        raw = stream.read((1 << 20) + 1); after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size
            and hashlib.sha256(raw).hexdigest() == SUPERVISOR_SHA, 'frozen supervisor bytes')
    module = types.ModuleType('readiness40_checker_owned'); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def interrupted(number, _frame):
    raise RuntimeError('controller signal ' + str(number))


def main():
    start = time.monotonic(); deadline = start + WHOLE_SECONDS
    def arm_deadline():
        remaining = deadline - time.monotonic()
        require(remaining > 0, 'whole CPU deadline exhausted; retain failed prefix only')
        signal.setitimer(signal.ITIMER_REAL, remaining)
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -B run_cpu.py only')
    require(ROOT == EXPECTED_ROOT and ROOT.resolve(strict=True) == ROOT
            and not os.path.lexists(OUT) and not os.path.lexists(TMP), 'fresh exact output namespace')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged CPU host')
    os.umask(0o077); os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected nice level')
    if priority == 0: os.nice(10)
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        limit = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup')
    h = load_supervisor()
    h.ROOT = ROOT; h.OUT = OUT; h.TMP = TMP; h.TARGET = ROOT / 'target'
    h.AS_LIMIT = 512 << 20; h.FILE_LIMIT = 16 << 20
    h.STREAM_LIMIT = 4 << 20; h.CACHE_LIMIT = 64 << 20
    h.CLEANUP_RESERVE = CLEANUP_SECONDS
    require(h.shutil.disk_usage(ROOT).free >= h.START_FREE, 'initial40GiB free floor')
    paths = {name: ROOT / name for name in sorted({'run_cpu.py', *SOURCES})}
    def sources():
        observed = set()
        for path in ROOT.rglob('*'):
            name = path.relative_to(ROOT).as_posix()
            if path.relative_to(ROOT).parts[0] in ('evidence', 'tmp'):
                continue
            require(path.resolve(strict=True) == path, 'canonical CPU input tree')
            if not path.is_dir(): observed.add(name)
        require(observed == set(paths), 'closed185 original CPU input bodies')
        return {name: h.pin(path) for name, path in paths.items()}
    h.sources = sources
    before = sources()
    for name, (size, sha) in SOURCES.items():
        require((before[name]['bytes'], before[name]['sha256']) == (size, sha), 'exact source or original fixture')
    require(len(paths) == 185 and len(NAMES) == len(set(NAMES)) == 171, 'closed inputs and test census')
    for module, classes in ROSTERS.items():
        tree = ast.parse((ROOT / (module + '.py')).read_bytes())
        actual = {node.name: sorted(item.name for item in node.body
                  if isinstance(item, ast.FunctionDef) and item.name.startswith('test_'))
                  for node in tree.body if isinstance(node, ast.ClassDef)
                  and any(isinstance(item, ast.FunctionDef) and item.name.startswith('test_') for item in node.body)}
        require(actual == classes, 'closed source-level test rosters')
    python = Path('/usr/bin/python3').resolve(strict=True)
    prlimit = Path('/usr/bin/prlimit').resolve(strict=True)
    tools = {'python': h.pin(python), 'prlimit': h.pin(prlimit)}
    environment = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
        PYTHONPATH='', PYTHONDONTWRITEBYTECODE='1', PYTHONNOUSERSITE='1', TMPDIR=str(TMP),
        HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='', CARGO_BUILD_JOBS='2',
        OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1')
    OUT.mkdir(mode=0o700); TMP.mkdir(mode=0o700)
    phases = []; errors = []; failure = None; census = None; after = None; leaf_tests = {}
    handlers = {n: signal.getsignal(n) for n in SIGNALS}
    for number in handlers: signal.signal(number, interrupted)
    try:
        h.save('sources-before.json', before)
        for label, seconds, modules, count in LEAVES:
            h.CPU_LIMIT = seconds
            script = ('import sys,unittest;sys.path.insert(0,' + repr(str(ROOT)) + ');'
                'suite=unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromName(name) '
                'for name in ' + repr(modules) + ');'
                'result=unittest.TextTestRunner(verbosity=2).run(suite);sys.exit(not result.wasSuccessful())')
            argv = [str(python), '-I', '-B', '-c', script]
            h.run(label, argv, environment, phases, deadline, before, seconds=seconds, cwd=ROOT)
            require((OUT / (label + '.stdout')).read_bytes() == b'', 'unexpected test stdout')
            stderr = (OUT / (label + '.stderr')).read_text()
            prefixes = '|'.join(re.escape(module + '.' + cls)
                for module in modules for cls in ROSTERS[module])
            expression = r'^(test_[A-Za-z0-9_]+) \((' + prefixes + r')\.\1\) \.\.\. ok$'
            found = [prefix + '.' + name for name, prefix in re.findall(expression, stderr, re.M)]
            expected = sorted(module + '.' + cls + '.' + name
                for module in modules for cls, names in ROSTERS[module].items() for name in names)
            require(sorted(found) == expected and len(found) == len(set(found)) == count,
                    'all selected named tests passed exactly once')
            tail = '\n'.join(line for line in re.sub(expression, '', stderr, flags=re.M).splitlines() if line)
            require(re.fullmatch(r'-{70}\nRan ' + str(count) + r' tests in [0-9]+\.[0-9]+s\nOK', tail),
                    'exact unittest summary, no skips/errors')
            leaf_tests[label] = dict(names=expected, passed=count, failed=0, errors=0, skipped=0)
            require(not any(TMP.iterdir()), 'all private test temporary files retired')
        census = dict(names=list(NAMES), passed=171, failed=0, errors=0, skipped=0)
    except BaseException as error:
        failure = type(error).__name__ + ': ' + str(error)
    finally:
        for number in handlers:
            signal.signal(number, interrupted if number == signal.SIGALRM else signal.SIG_IGN)
        arm_deadline()
        try:
            after = sources(); require(after == before, 'CPU source or original fixture drift')
            h.save('sources-after.json', after)
            require(Path('/usr/bin/python3').resolve(strict=True) == python
                    and Path('/usr/bin/prlimit').resolve(strict=True) == prlimit
                    and h.pin(python) == tools['python'] and h.pin(prlimit) == tools['prlimit'], 'CPU tool drift')
            require(not any(TMP.iterdir()), 'private temporary files remain')
            require(len(phases) <= 2 and all(row['reaped'] and row['process_group_absent'] for row in phases),
                    'all recorded children completely reaped')
            require(time.monotonic() < deadline, 'whole CPU bound')
        except BaseException as error:
            if time.monotonic() >= deadline: raise
            errors.append(type(error).__name__ + ': ' + str(error))
    arm_deadline()
    raw = {p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()}
    allowed = {'sources-before.json', 'sources-after.json'} | {
        label + '.' + suffix for label, _, _, _ in LEAVES
        for suffix in ('command.json', 'started.json', 'result.json', 'stdout', 'stderr')}
    try:
        for row in phases:
            for key in ('command', 'stdout', 'stderr'):
                require(raw[Path(row[key]['path']).name] == row[key], 'final raw phase pin join')
            require(json.loads((OUT / (row['label'] + '.result.json')).read_bytes()) == row,
                    'original phase result join')
            started = json.loads((OUT / (row['label'] + '.started.json')).read_bytes())
            require(started == dict(pid=row['pid'], pgid=row['pgid'], argv=row['argv']),
                    'original child registration join')
        require(set(raw) <= allowed, 'closed original raw prefix')
    except BaseException as error:
        if time.monotonic() >= deadline: raise
        errors.append('raw reconciliation: ' + repr(error))
    failure = failure or ('postcheck failed' if errors else None)
    value = dict(schema='ferric-currentness-duration-native-admission-cpu-v1',
        passed=failure is None and census is not None,
        failure=failure, postcheck_errors=errors, controller=before['run_cpu.py'], supervisor=before['supervisor.py'],
        sources_before=before, sources_after=after, source_unchanged=after == before,
        tool_pins=tools, environment=environment, phases=phases, tests=census, leaf_tests=leaf_tests, raw=raw,
        limits=dict(whole_seconds=WHOLE_SECONDS, leaf_seconds={'readiness-tests':120, 'admission-tests':180},
            cleanup_reserve_seconds=CLEANUP_SECONDS, address_space_bytes=512 << 20, file_bytes=16 << 20,
            stream_bytes=4 << 20, temporary_bytes=64 << 20, affinity=[8,9], nice=10, cargo_jobs=2,
            initial_free_bytes=40 << 30, live_free_bytes=38 << 30),
        elapsed_seconds=time.monotonic() - start, synthetic_data_tests_only=True,
        inherited_checker_tests=126, admission_workflow_tests=45, original_fixture_files=152,
        live_admission_included=False, gpu_execution=False, native_parent_execution=False,
        model_execution=False, numerical_acceptance=False, full_model_acceptance=False,
        performance_claim=False, production_authority=False)
    arm_deadline()
    h.save('complete.json' if value['passed'] else 'failed.json', value)
    print(json.dumps({key:value[key] for key in ('passed','failure','postcheck_errors','tests')}, sort_keys=True))
    signal.setitimer(signal.ITIMER_REAL, 0)
    for number, handler in handlers.items(): signal.signal(number, handler)
    return 0 if value['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
