window.FERRIC_PERFORMANCE = {
  updated: "2026-09-11",
  concurrentRounds: {
    "scope": "One instrumented Qwen3-8B observation per TP world and transport mode (n=1 per mode): four requests and eight outputs, including one from the cancelled request. All six runs pass the frozen token/byte reference, exact dispatch schedule, cleanup and physical-idle checks. Prefix cache is on; row/chunk budgets are 16; each run executes 34 physical rows in five batches.",
    "interpretation": "Concurrent rounds reach 1.870x the serial-peer workload output rate at TP2 and 6.797x at TP8. Host-staged execution is still faster: rounds reach only 0.308x and 0.202x its rate, respectively. The serial-to-round pair isolates the round mode within the same peer worker. The host-to-peer comparison also changes worker architecture and collective placement. These are separate baselines, not an overall peer speedup.",
    "sourceScope": "Frozen instrumented controller d9d378e0 was built from Ferric abecc466 with fe2o3 dependency cutoff 4fbc0a34. Both separately identified workers use public core 79706b43; the base image is 8c81d3fe and the peer image is 9d4f1b92. Later active-source repins do not relabel these runs or replace the earlier uninstrumented observations.",
    "measurementScope": "Every run enables the same new host-timing sidecar; the independent worker runtime_profiling flag remains false. These are host wall intervals, not GPU timestamps. Workload rate excludes setup and teardown. TTFT starts at each request's actual admission; TPOT is its mean adjacent output gap. Setup and whole-process time remain separate, and no setup or close-cost improvement is attributed to rounds.",
    "limits": "Fresh workers, no warmup; n=1 per mode is not steady-state serving, a stable tail, or statistical significance. There is no physical overlap measurement, protected authority, M1 closure, or vLLM/SGLang comparison. Request latencies are never pooled across identities.",
    "pins": {
      "controllerSha256": "d9d378e0378758f41b22b3112ccf113cc0d86f266f992a5626b062ad2399d800",
      "controllerSourceRevision": "abecc46662ed884162407a4825899b212fb63db0",
      "controllerFe2o3Revision": "4fbc0a34c7938474406d37a45e7972ea8b1ec277",
      "workerFe2o3Revision": "79706b43a177a2fd3fa43ec328221fa3e5041af5",
      "hostWorkerSha256": "aaa0216a77de0d5f12c2d668b31ca8c340d8975407c2b446bb5e20b5d820bd6e",
      "peerWorkerSha256": "2032e31b6d7fbc22aae6e5b7092f9a4e37047360ca5025fb10b2e0119999ea2e",
      "hsacoSha256": "8c81d3fe869d3346d95486354f366988ce99210111cd0fc712dfb2194450b125",
      "manifestSha256": "76e582363a35d48e993fbd68f15438e853a2270b4bce27ad989d96d3370f1ce8",
      "handoffSha256": "6aaa62549ee5ca71e8e1da2aed0eba6d86fed0bc0b08edcdfdbca68a1cc48bd1",
      "peerHsacoSha256": "9d4f1b922967521b05199ce7d1e0269eaae3afa4fe558eafc4ba0bda3c4a4b65",
      "peerManifestSha256": "e0242eefcffc7b278047467b118087b5336127e45dc5ee96f7991ea138e1ef2e",
      "peerHandoffSha256": "ba2a796c529e2cac510cafff3ae9da72b7d3523de1774a9f9e88ce71a4ed16cc",
      "workloadSha256": "23882e195cf578b987c72fc5703fffb51541708e5431d4166e55e0fe7e2f137f",
      "referenceSha256": "1ed868663df52a146dd7921f9fbb2cf1e1d0c0ceed8a7bfda030d65f8ec5b094",
      "prelaunchSha256": "3675ad678cfb1427d0ec84b79d7ab3141992aebdc3d1c8cd8ab399a894128115",
      "comparatorSourceSha256": "1be4f2b3b0f8d1c96ef0b5bf21388da232d6b23e5700c7bef6295117b6093b6a",
      "ledgerGeneratorSourceSha256": "6da3b95acda3332bfa27a1c51685dda316d5f188615b9b939554d5eef8c9085e",
      "hostTimingGeneratorSourceSha256": "057f4a6dfb929db44714f8192bb1e373184f4170ce3946dcc31be811637b2402"
    },
    "worlds": [
      {
        "world": 2,
        "ledgerFileSha256": "085a233fdc654e6cf75beee9c3bc1dc8dd42c6a2d9b7ac2216c2a1692702c420",
        "serialLedgerFileSha256": "2297d9c4b8067b59c3c809efc38ba424a48ce12def680fb136bf224e9106c1d0",
        "roundOverSerial": 1.869632033434311,
        "roundOverHost": 0.3076843625405642
      },
      {
        "world": 8,
        "ledgerFileSha256": "d2e86b755c7b76915888f26ec4a7049e71981cb11b7e0ef43352130539575b64",
        "serialLedgerFileSha256": "d4a52661591b145eb24dc4573c90b368eb813c147612a0981cf95b20252294cf",
        "roundOverSerial": 6.796859078764497,
        "roundOverHost": 0.2015156584758047
      }
    ],
    "profiles": [
      {
        "id": "tp2-host-profile-r1",
        "name": "Host-staged",
        "world": 2,
        "mode": "host",
        "repetitions": 1,
        "outputTokensPerSecond": 0.8715466342343043,
        "workloadSeconds": 9.179084269,
        "setupSeconds": 106.238880038,
        "wholeSeconds": 119.078554602,
        "requestLatencies": [
          [
            4.499774554,
            1.937136546
          ],
          [
            1.887106741,
            1.8173636145
          ],
          [
            1.937080417,
            null
          ],
          [
            1.697529045,
            1.044582486
          ]
        ],
        "caseFileSha256": "1686fb25cc3b2295c7fa52a9b2cc4883c4c332ce55f2782d857102b90036d9fb",
        "comparisonFileSha256": "7a30e5242967feb74342fa7ef8e8846c080a7494c21d1f5e3075825a9bcdda80"
      },
      {
        "id": "tp2-serial-profile-r1",
        "name": "Serial peer",
        "world": 2,
        "mode": "serial",
        "repetitions": 1,
        "outputTokensPerSecond": 0.14342997219948841,
        "workloadSeconds": 55.7763477,
        "setupSeconds": 221.343087054,
        "wholeSeconds": 287.399988888,
        "requestLatencies": [
          [
            22.73383172,
            11.340460339
          ],
          [
            11.287127212,
            11.3216869045
          ],
          [
            11.340415269,
            null
          ],
          [
            11.302857202,
            10.399142171
          ]
        ],
        "caseFileSha256": "bc5545574e4f0871db06c2f20653a2eb1085332f86650ed1d560312d4875954e",
        "comparisonFileSha256": "967a5eb1a909d394935d4e56e8e91bf507bbe7b174c52cc43fbf5bbea23b31e1"
      },
      {
        "id": "tp2-round-profile-r1",
        "name": "Concurrent peer round",
        "world": 2,
        "mode": "round",
        "repetitions": 1,
        "outputTokensPerSecond": 0.2681612705787562,
        "workloadSeconds": 29.832794209,
        "setupSeconds": 221.191863439,
        "wholeSeconds": 261.256555958,
        "requestLatencies": [
          [
            12.156615679,
            6.063304663
          ],
          [
            6.042474817,
            6.046504237
          ],
          [
            6.063263384,
            null
          ],
          [
            6.029657882,
            5.583170056
          ]
        ],
        "caseFileSha256": "832a298bd24deaa0efaae79f0e33ad5418b2f24dd3bc4397536b69cf641e9077",
        "comparisonFileSha256": "bfaaf550b88e5acf4863c4278451c5c88d399dceea7215f7ce9cfeb9570fcf02"
      },
      {
        "id": "tp8-host-profile-r1",
        "name": "Host-staged",
        "world": 8,
        "mode": "host",
        "repetitions": 1,
        "outputTokensPerSecond": 0.3806743805792887,
        "workloadSeconds": 21.015335962,
        "setupSeconds": 135.388216231,
        "wholeSeconds": 171.034464625,
        "requestLatencies": [
          [
            11.134335381,
            4.347102062
          ],
          [
            4.046521012,
            3.9408970745
          ],
          [
            4.346994744,
            null
          ],
          [
            3.53455321,
            1.999206432
          ]
        ],
        "caseFileSha256": "12737ebfb85590411856e9f0e789b86d06db94eff15c3c06eb8a24fc05fecf9d",
        "comparisonFileSha256": "ebc6ef9b186db9f03b6521ec8f8ace491e45a3abfcbbc2cfa1b11f9cb22a2fa9"
      },
      {
        "id": "tp8-serial-profile-r1",
        "name": "Serial peer",
        "world": 8,
        "mode": "serial",
        "repetitions": 1,
        "outputTokensPerSecond": 0.011286367361503218,
        "workloadSeconds": 708.819741885,
        "setupSeconds": 736.568781886,
        "wholeSeconds": 1565.344074538,
        "requestLatencies": [
          [
            284.792019959,
            142.393238828
          ],
          [
            142.219230283,
            142.310494096
          ],
          [
            142.393170079,
            null
          ],
          [
            142.227666546,
            139.406733734
          ]
        ],
        "caseFileSha256": "3f92c4856f8504c8cdf3dc0664fc6410fee4c284c9ee6c12900bb90faaf11655",
        "comparisonFileSha256": "e343738ca5abbfb6e70e663c9df2fce012f02bbd7e04fd572ba09c957811bce3"
      },
      {
        "id": "tp8-round-profile-r1",
        "name": "Concurrent peer round",
        "world": 8,
        "mode": "round",
        "repetitions": 1,
        "outputTokensPerSecond": 0.07671184846730444,
        "workloadSeconds": 104.286367228,
        "setupSeconds": 737.535894015,
        "wholeSeconds": 961.521453111,
        "requestLatencies": [
          [
            41.841570966,
            20.917410031
          ],
          [
            20.898608219,
            20.913519579
          ],
          [
            20.917357012,
            null
          ],
          [
            20.909555688,
            20.617757104
          ]
        ],
        "caseFileSha256": "a0d1dc5aee4bec1826c3d9b754d69ee93325d5d09b5802132cde993c0e33ba27",
        "comparisonFileSha256": "541eaf20f25a96f83521e5bd1be13bd636c9a4aafb0af3546b3ae98876f03abb"
      }
    ]
  },
  scope: "The latest six-case instrumented TP2/TP8 transport matrix has n=1 per mode and does not replace historical observations. Two separate Qwen3-8B workloads are reported on MI350X. Allocation and wide-policy cohorts use eight requests and 64 outputs with cache off. The other tables use a fixed four-request logical-tick BF16 workload with prefix cache enabled: eight output tokens including one from the cancelled request. Historical repeated TP8 profiles, MFMA and TP1 residual comparisons have n=2 per profile; other model pairs, cumulative and peer observations have n=1. Original R1 checkpoints remain separate, without double-counting their observations. Not steady-state serving throughput, a serving SLO, protected M1 qualification, or a vLLM/SGLang comparison.",
  interpretation: "Concurrent rounds improve over serial peer execution but remain slower than host-staged execution at both TP2 and TP8. Setup and teardown remain costly. Earlier separately pinned results follow: Operational-only runtime validation is consistently faster in its two repetitions. Pruning varies substantially. The new combined runtime sample is faster than its same-controller control, but slower than the earlier operational-only samples; no extra gain from caching or sequences is established. Standalone admission caching, sequences, and host workspace reuse do not establish reliable isolated gains. Both wave-projection-only and wave-attention-only model runs fail the frozen token reference and are excluded from performance results.",
  correctness: "Every included four-request run exits with status 0, matches all eight reference-subsequence token IDs and decoded bytes, completes the exact rank dispatch schedule, closes and reaps workers, and binds before/after physical-idle checks for all eight GPUs. The separate replica cohorts check 64 outputs under their own common-clock protocol. The checker binds externally pinned controller, live worker, artifact, workload, and reference identities. Numerical kernels and transport remain Contracted; authority is none.",
  statistics: "Ranges show both observations (n=2). Nearest-rank p50 is the smaller observation and p95 the larger, calculated independently for each metric, not one median run. Small-sample p95 is not a stable tail. Request identities are never pooled.",
  definitions: [
    ["TTFT", "First committed output minus that request's actual admission; model setup is excluded."],
    ["TPOT", "Mean of adjacent committed output gaps within the same request and repetition. The cancelled request has one output and no decode interval."],
    ["Workload output tok/s", "All eight output tokens divided by the interval from first actual admission to last terminal event. Terminal means last output for a completed request or cancellation time for a cancelled request. Per-request rates are not summed."],
    ["Setup / whole", "Initialization, allocation and upload are reported separately. Whole-process time includes setup and teardown; it is not interchangeable with the workload window."],
    ["Matched controls", "Same model/reference, BF16, TP world and physical roster, cache policy, fixed workload, context/page limits, chunk and batch budgets, and unwarmed policy. Controller and worker hashes may differ only as explicitly pinned for each variant. TP1 residual and TP8 MFMA pairs remain separate from replica allocation cohorts."],
  ],
  variants: [
    {
      name: "Frozen baseline",
      kind: "baseline",
      repetitions: 2,
      outputHeadPruning: false,
      operationalCurrentness: false,
      outputTokensPerSecond: [0.03354745032108289, 0.03440722113702573],
      workloadSeconds: [232.509331926, 238.468197238],
      setupSeconds: [191.147392313, 193.130256023],
      wholeSeconds: [440.27995217, 444.151925708],
      controllerSha256: "098be2f8425bffcc46545ba66f313a3c63090eb01360eccafb276d887350e151",
      workerSha256: "77a53d18b56e4ee7a67a434feffa8ac18a4fe60f8c9e5daace351f502c72e1da",
    },
    {
      name: "Output-head pruning only",
      kind: "standalone",
      repetitions: 2,
      outputHeadPruning: true,
      operationalCurrentness: false,
      outputTokensPerSecond: [0.03281134113464035, 0.0544379713675337],
      workloadSeconds: [146.956247616, 243.81813493],
      setupSeconds: [182.408603199, 194.155723776],
      wholeSeconds: [343.808796555, 452.51767042],
      controllerSha256: "f35e02e13fb2e5ed1414f5211f623596ab35982a737c1fb504e5a1efcc65d6dd",
      workerSha256: "77a53d18b56e4ee7a67a434feffa8ac18a4fe60f8c9e5daace351f502c72e1da",
    },
    {
      name: "Operational runtime only",
      kind: "standalone",
      repetitions: 2,
      outputHeadPruning: false,
      operationalCurrentness: true,
      outputTokensPerSecond: [0.4719818769493887, 0.5040455443900378],
      workloadSeconds: [15.871581624, 16.949803352],
      setupSeconds: [120.955882883, 133.314032284],
      wholeSeconds: [152.253657747, 163.649500551],
      controllerSha256: "d06b54cc59f1daf787111f873433cb6ecdc3c8205582dad78738a887d654bffe",
      workerSha256: "70572ff23ae4453ea1d7947061692c6e9e410c6980c7679fde4b467d636d6912",
    },
  ],
  requests: [
    { name: "seed-prefix", gapsPerRepetition: 1,
      baselineTtft: [97.289069891, 99.159675982], baselineTpot: [47.908883275, 48.389209414],
      operationalTtft: [9.023063215, 10.211036628], operationalTpot: [3.180318945, 3.258528149] },
    { name: "arriving-short", gapsPerRepetition: 2,
      baselineTtft: [47.158197697, 48.003594448], baselineTpot: [46.3327152215, 47.399908923],
      operationalTtft: [2.836283013, 3.3353232], operationalTpot: [2.7995498195, 2.845806289] },
    { name: "cancel-between-batches", gapsPerRepetition: 0,
      baselineTtft: [47.908806597, 48.389128996], baselineTpot: null,
      operationalTtft: [3.180280896, 3.25849394], operationalTpot: null },
    { name: "reuse-prefix", gapsPerRepetition: 1,
      baselineTtft: [44.276142601, 46.890851923], baselineTpot: [40.684225501, 46.379309501],
      operationalTtft: [2.418666626, 2.4330315], operationalTpot: [1.139667085, 1.156905831] },
  ],
  pruningReuse: {
    ttftSeconds: [26.956054, 40.504502],
    tpotSeconds: [26.230656, 39.674194],
    gapsPerRepetition: 1,
  },
  runtimeProfile: {
    runtime_cache_admission: false, runtime_operational: true,
    dispatch_sequences: false, queue_rollover: false,
    projection: "baseline", attention: "baseline", runtime_profiling: false,
  },
  ablations: {
    scope: "Five accepted profiles from a six-case serialized queue, one repetition each. Same controller, worker, nine-root artifact, workload, and row/chunk budget 16. Reuse-prefix has one decode gap per run. Output-head pruning, queue rollover, runtime profiling, wave projection, and wave attention are off in every accepted profile. These single samples do not establish statistical significance or a stable tail.",
    interpretation: "The combined operational + admission-cache + sequence profile reaches 0.297 output tok/s versus its 0.029 control, but remains below the older-controller operational-only range of 0.472-0.504. Sequence-only is slower than control. Admission-cache and host-reuse whole-workload rates vary while reuse-prefix TPOT worsens, so neither is an established gain. Shared-host conditions and tiny output counts limit attribution.",
    controllerSha256: "bc4a283bfcaa5014c11978df88062b7c6d260ba93e5651c7de6991d837454e5e",
    workerSha256: "70572ff23ae4453ea1d7947061692c6e9e410c6980c7679fde4b467d636d6912",
    comparatorSha256: "0885d2f183d7e280db1b5bda6ad16b73388965ab98c786f7fd6d8d61d3ac4416",
    baselineLedgerSha256: "1a910edc4bdff3698bffe43ddd78da7e0495967a383801bafb982e62d6c3fe65",
    controlLedgerSha256: "ebd7945a28e54f37656847b015e86172d99917088472360fc358e507bbb95a64",
    profiles: [
      {
        id: "runtime-control", name: "Matched runtime control", kind: "control", repetitions: 1,
        admissionCache: false, operationalCurrentness: false, dispatchSequences: false, hostWorkspaceReuse: false,
        outputTokensPerSecond: 0.028965528639579515, workloadSeconds: 276.190367507,
        setupSeconds: 193.359256922, wholeSeconds: 484.040835573,
        requestLatencies: [[128.635985168, 62.860476939], [62.583964335, 54.711432035],
          [62.86040731, null], [46.562291972, 38.131518269]],
      },
      {
        id: "runtime-combined", name: "Operational + cache + sequences", kind: "cumulative", repetitions: 1,
        admissionCache: true, operationalCurrentness: true, dispatchSequences: true, hostWorkspaceReuse: false,
        outputTokensPerSecond: 0.2965814717829149, workloadSeconds: 26.974038371,
        setupSeconds: 139.633135545, wholeSeconds: 181.107494759,
        requestLatencies: [[14.474728723, 5.800196878], [5.537030621, 4.956395593],
          [5.800155369, null], [4.112530929, 2.586518462]],
      },
      {
        id: "admission-cache", name: "Admission cache only", kind: "standalone", repetitions: 1,
        admissionCache: true, operationalCurrentness: false, dispatchSequences: false, hostWorkspaceReuse: false,
        outputTokensPerSecond: 0.03294983986623466, workloadSeconds: 242.793289208,
        setupSeconds: 192.067033065, wholeSeconds: 449.409000663,
        requestLatencies: [[97.034771087, 46.523330452], [46.524560309, 46.3984656725],
          [46.523245584, null], [46.273495275, 52.961586776]],
      },
      {
        id: "sequences", name: "Dispatch sequences only", kind: "standalone", repetitions: 1,
        admissionCache: false, operationalCurrentness: false, dispatchSequences: true, hostWorkspaceReuse: false,
        outputTokensPerSecond: 0.025022199336965138, workloadSeconds: 319.716100582,
        setupSeconds: 189.207977235, wholeSeconds: 523.52823998,
        requestLatencies: [[124.066778598, 66.027320976], [61.011928595, 65.335680148],
          [66.027245568, null], [64.643954392, 64.977961688]],
      },
      {
        id: "host-reuse", name: "Host collective workspace reuse", kind: "standalone", repetitions: 1,
        admissionCache: false, operationalCurrentness: false, dispatchSequences: false, hostWorkspaceReuse: true,
        outputTokensPerSecond: 0.03457091887354652, workloadSeconds: 231.408370407,
        setupSeconds: 189.483975609, wholeSeconds: 435.367588378,
        requestLatencies: [[97.412771772, 47.253851007], [46.777104911, 46.989745109],
          [47.253788238, null], [46.725553513, 40.016108417]],
      },
    ],
  },
  currentCompatibility: {
    scope: "Current-controller source compatibility, not blanket combination qualification. Ferric controller 44e4 is built on public fe2o3 7528; worker 189 was freshly rebuilt on 1b262 and is byte-identical to its public-902 build, with the worker closure unchanged through 7528. The emitted kernel images remain frozen to their original compiler/SDK provenance. These are independent n=1 four-request/eight-output cache-on observations, operational runtime on, not the 64-output allocation workload or a current-head compiler-emission/Verus receipt.",
    interpretation: "The TP1 baseline control and TP8 wide-capacity MFMA-plus-pruning smoke pass exact reference and teardown checks. The TP8 smoke is unpaired and observed at most 17 rows, not 32: its capacity is 32 and actual batches are [17, 6, 6, 4, 1]. No gain is assigned against older profiles. Current TP1 baseline projection plus pruning and device residual also passes: workload rate improves 12.06% versus its same-current control and reuse-prefix TTFT 5.56%, but TPOT changes only 0.25%. This is n=1 each with two flags changed, not an isolated pruning or residual gain. TP1 MFMA plus pruning plus device residual is rejected, and MFMA-only also fails: seed-prefix emits [9856, 374] (Germany is) instead of [17689, 374] (Spain is). The mismatch occurs without pruning or device residual, but its underlying numerical cause is not proven. Matching other request summaries does not qualify either rejected trace; no rejected-run timing is published. The outer diagnostic SSH session exited 255 after completed case receipts; per-case status/idle checks and absence of owned workers were confirmed separately, with the anomaly retained.",
    pins: {
      controllerSha256: "44e4bd4f1717a585fe955904f2b7f762f41ef59766007c1c46c3a4d7e403a0d9",
      workerSha256: "189b918dcd3f7104767404c636eb08490a3b1714a6f78123ea3a1fa06f21babb",
      controllerFe2o3Revision: "7528e7345cef0158d7034cdbae23011e3c6fb5d2",
      workerRebuildFe2o3Revision: "1b262ac3dd23ee63067e40587d62a124f40b9fc9",
    },
    accepted: [
      {
        id: "latest-tp1-control-r1", name: "Current TP1 baseline control", repetitions: 1,
        world: 1, imageProfile: "v3-mfma", projection: "baseline", outputHeadPruning: false, collective: null,
        rowCapacity: 16, actualBatchRows: [16, 6, 7, 4, 1],
        outputTokensPerSecond: 0.8369537638457045, workloadSeconds: 9.558473055,
        setupSeconds: 102.300673194, wholeSeconds: 113.70528944,
        requestLatencies: [[4.423279847, 2.034494855], [2.000398585, 1.9624907815],
          [2.034429116, null], [1.890434159, 1.210211645]],
        comparisonSha256: "f7f25c01e5b7d99350fbb15fee0c35b73d20ab8188cdfb89e350b04ffe961ce7",
        metricsFileSha256: "37122de41dabe0287976f572627bce042e9a3be4e9820b143124371a8aa88400",
      },
      {
        id: "latest-tp8-wide-cumulative-r1", name: "Current TP8 MFMA + pruning", repetitions: 1,
        world: 8, imageProfile: "v5-mfma32", projection: "mfma", outputHeadPruning: true, collective: null,
        rowCapacity: 32, actualBatchRows: [17, 6, 6, 4, 1],
        outputTokensPerSecond: 0.44486974417084935, workloadSeconds: 17.982791828,
        setupSeconds: 148.638654416, wholeSeconds: 188.583575705,
        requestLatencies: [[7.189765347, 3.386222764], [3.386040607, 2.901255456],
          [3.23708723, null], [2.565240366, 1.604292805]],
        comparisonSha256: "67ad69a2ec6d29f2cc03f6eabe65cfcde7837cf606a37363837d65ddadb37c59",
        metricsFileSha256: "8f395b5dbfa30b276efaeacec155ce6b033c1c612e1c64e4fa843752154c8c93",
      },
      {
        id: "latest-tp1-residual-pruning-r1", name: "Current TP1 baseline + pruning + residual", repetitions: 1,
        world: 1, imageProfile: "v3-mfma", projection: "baseline", outputHeadPruning: true, collective: "device-tp1-v3",
        rowCapacity: 16, actualBatchRows: [16, 6, 7, 4, 1],
        outputTokensPerSecond: 0.9378775166561548, workloadSeconds: 8.529898476,
        setupSeconds: 102.432063554, wholeSeconds: 112.809883404,
        requestLatencies: [[3.712036679, 1.825208929], [1.809598966, 1.8053320195],
          [1.82516135, null], [1.785405941, 1.207197758]],
        comparisonSha256: "66649461712577c7df2cef26cad9449beb288f7b34eb5156154925acb27eca06",
        metricsFileSha256: "89a5521453330d8726d335febcd2ea3367e71093f69a54717bb0ad93f0ea69d9",
      },
    ],
    rejected: [
      {
        id: "latest-tp1-cumulative-r1", name: "Current TP1 MFMA + pruning + device residual", repetitions: 1,
        world: 1, projection: "mfma", outputHeadPruning: true, collective: "device-tp1-v3",
        requestName: "seed-prefix", expectedTokens: [17689, 374], observedTokens: [9856, 374],
        rejectionSha256: "be835327607d5a88c4b421327f87378c57c8513947881e1747044fe9ccf7a5b9",
      },
      {
        id: "latest-tp1-mfma-only-r1", name: "Current TP1 MFMA only", repetitions: 1,
        world: 1, projection: "mfma", outputHeadPruning: false, collective: null,
        requestName: "seed-prefix", expectedTokens: [17689, 374], observedTokens: [9856, 374],
        rejectionSha256: "2402b01e3d4896d97b0546ff2da09ab23fe655d41e406e731c366d995473bd26",
      },
    ],
  },
  mfmaPair: {
    scope: "Matched TP8 MFMA projection pair, n=1 each: same controller, worker, full fifteen-root image, exact physical roster, model/reference and four-request/eight-output workload. Prefix cache on, pruning off, legacy host-staged rank-order collective, row/chunk budget 16. Only projection selection changes; operational runtime is on and all other performance flags are off. This is not the separate 64-output replica workload.",
    interpretation: "The MFMA sample improves workload output rate by 27.91%, reuse-prefix TTFT by 30.85%, and reuse-prefix TPOT by 32.21%. Startup regresses: setup rises from 119.147 to 173.302 s and whole-process time from 151.629 to 209.070 s. These are one observation per profile, not a stable tail or a repeated performance claim. The later CPU transpose change is not included in either model run.",
    pins: {
      controllerSha256: "bc4a283bfcaa5014c11978df88062b7c6d260ba93e5651c7de6991d837454e5e",
      workerSha256: "70572ff23ae4453ea1d7947061692c6e9e410c6980c7679fde4b467d636d6912",
      hsacoSha256: "8c81d3fe869d3346d95486354f366988ce99210111cd0fc712dfb2194450b125",
      manifestSha256: "76e582363a35d48e993fbd68f15438e853a2270b4bce27ad989d96d3370f1ce8",
      handoffSha256: "6aaa62549ee5ca71e8e1da2aed0eba6d86fed0bc0b08edcdfdbca68a1cc48bd1",
      comparatorSha256: "0885d2f183d7e280db1b5bda6ad16b73388965ab98c786f7fd6d8d61d3ac4416",
      ledgerFileSha256: "8660be59f4d452e3944131069c5a7824c0c72f6c3e87bfcd83f4a7d475ad3884",
      ledgerCanonicalId: "8719cb980590c41c7c8751b95117a858b04c42912964c811bebf7e2c4d69bbea",
    },
    profiles: [
      {
        id: "mfma-image-control", name: "Same-image scalar control", repetitions: 1, projection: "baseline",
        outputTokensPerSecond: 0.44956497648927296, workloadSeconds: 17.794980522,
        setupSeconds: 119.146961353, wholeSeconds: 151.628723088,
        requestLatencies: [[10.173978781, 3.729009017], [3.339219247, 3.179961523],
          [3.728939118, null], [2.63084706, 1.261078695]],
        comparisonSha256: "fb4945a4f33fbe617e3eb73dc92bf75cf3e7b718e384dd3d8fa59832585bc977",
      },
      {
        id: "mfma-projection", name: "MFMA projection", repetitions: 1, projection: "mfma",
        outputTokensPerSecond: 0.5750538887941058, workloadSeconds: 13.911739675,
        setupSeconds: 173.30152968, wholeSeconds: 209.070220967,
        requestLatencies: [[8.42434351, 2.813204963], [2.454712134, 2.316253428],
          [2.813172113, null], [1.819253254, 0.854889309]],
        comparisonSha256: "e7c54cc64da3d0ccbecb6b4f9c80ad94bae845d652d48b36c17f2e640b5f738a",
      },
    ],
  },
  mfmaRepeated: {
    scope: "Repeated same-image TP8 MFMA comparison, two repetitions per profile. R1 is the previously published pair below, not an additional run; R2 uses the same bc4 controller, 705 worker, full 8c81 image, four-request/eight-output workload and exact runtime controls. Only projection selection changes. Prefix cache on, pruning off, operational runtime on, row/chunk budget 16, legacy host-staged collective. No tiled transpose or later runtime source is included.",
    interpretation: "Mean workload output rate improves 28.63%, reuse-prefix TTFT 31.61%, and reuse-prefix TPOT 31.95%. Mean setup worsens 46.87%, from 117.808 to 173.024 s; whole-process time worsens 38.83%, from 150.352 to 208.726 s. Each table cell reports mean [minimum, maximum] over two runs of the same profile and request identity. Nearest-rank p50/p95 at n=2 are just the minimum/maximum, not a stable tail or statistical significance. Request identities are never pooled; the cancelled one-output request has no TPOT.",
    repetitions: 2,
    ledgerFileSha256: "6b14222502f670b4cd8a0d607777885c590b88874ad40190fbf56799528bcef2",
    secondProfiles: [
      {
        id: "mfma-image-control-r2", name: "Same-image scalar control R2", repetitions: 1, projection: "baseline",
        outputTokensPerSecond: 0.44349406135878866, workloadSeconds: 18.038572998,
        setupSeconds: 116.46916198, wholeSeconds: 149.075672286,
        requestLatencies: [[10.341171908, 3.754009847], [3.434148227, 3.223939024],
          [3.753967348, null], [2.693811553, 1.249523042]],
        comparisonSha256: "6e48969c746d2dff2d9cc370e9da2d669ca0c2eea30285700f9705d52c4607e0",
      },
      {
        id: "mfma-projection-r2", name: "MFMA projection R2", repetitions: 1, projection: "mfma",
        outputTokensPerSecond: 0.5737304451830695, workloadSeconds: 13.94383036,
        setupSeconds: 172.746206434, wholeSeconds: 208.382689573,
        requestLatencies: [[8.44046609, 2.827575203], [2.472448866, 2.3248421475],
          [2.827543944, null], [1.822059573, 0.853679975]],
        comparisonSha256: "c093a0468a07dba8d5996162d3513b9a4a82c36c512426e7eab7b5ad0b53f527",
      },
    ],
  },
  mfmaPruning: {
    scope: "Cumulative TP8 operational runtime + MFMA projection + output-head pruning, one repetition. Same bc4 controller, 705 worker, full 8c81 image and four-request/eight-output workload as the two-per-profile MFMA comparison. Cache on, row/chunk budget 16, host-staged rank-order collective. This adds pruning to MFMA; it is not the isolated pruning-only profile or the later current-runtime smoke.",
    interpretation: "Exact reference tokens and bytes pass, but this sample does not show an additive pruning gain. Workload rate is 0.484514 tok/s, below MFMA-only mean 0.574392 tok/s; reuse-prefix TTFT is 2.139 s versus 1.821 s, and TPOT 1.167 s versus 0.854 s. Setup and whole-process time are also worse. The cumulative sample has n=1; the reused scalar and MFMA controls have n=2 each and are not counted as new repetitions.",
    ledgerFileSha256: "01a030080f363f83b40d9a382bd2b82b1c16ebff51810924efb5091a122994ba",
    profiles: [{
      id: "mfma-and-pruning", name: "Operational + MFMA + pruning", repetitions: 1,
      projection: "mfma", outputHeadPruning: true,
      outputTokensPerSecond: 0.4845143774176782, workloadSeconds: 16.511377934,
      setupSeconds: 181.212353879, wholeSeconds: 219.455129754,
      requestLatencies: [[9.63499524, 3.569695294], [2.975040987, 2.854549337],
        [3.569656985, null], [2.139348791, 1.16728402]],
      comparisonSha256: "a11a60ee87e4245e91f3b8304815eb6366cc9d91ff558d61a639e5ec3fa17a11",
    }],
  },
  deviceTp1Pair: {
    scope: "Matched TP1 residual pair, n=1 each, with the same controller, worker, thirteen-root image, single physical device and fixed four-request/eight-output workload. Cache on, pruning off, operational runtime on, baseline projection/attention and other performance flags off. Only the legacy host-staged collective versus device-tp1-v3 residual path changes. This is not multi-device peer transport.",
    interpretation: "The device residual sample raises workload rate 14.19% and lowers reuse-prefix TTFT 6.76%. Reuse-prefix TPOT changes only 0.97%, from 1.179 to 1.167 s: too small and too few samples for a robust gain claim. Both complete exact token/byte, dispatch, close/reap and idle checks; repetitions and longer workloads are still needed.",
    pins: {
      controllerSha256: "bc4a283bfcaa5014c11978df88062b7c6d260ba93e5651c7de6991d837454e5e",
      workerSha256: "70572ff23ae4453ea1d7947061692c6e9e410c6980c7679fde4b467d636d6912",
      hsacoSha256: "306a27d8cdb2376d57a2b12d94fd2aa06bf19532175a31e15ee5390d7de87682",
      manifestSha256: "596a455c6810fc7271022582f00f99b393bef2522ef1d881a495cb40e939ec07",
      handoffSha256: "e1193656121858522c0b83c64d6ea72bd32a5ecd02b151e9acb8261ba92d775d",
      comparatorSha256: "0885d2f183d7e280db1b5bda6ad16b73388965ab98c786f7fd6d8d61d3ac4416",
      ledgerFileSha256: "07cae3b84184cf8fa806a3ce452a2a6959b7ad3f16bf27587abf06e0e4d1d385",
    },
    profiles: [
      {
        id: "device-tp1-control", name: "TP1 host control", repetitions: 1,
        collective: "host_staged_fp32_rank_order_reduce_bf16_residual",
        outputTokensPerSecond: 0.8349137138457552, workloadSeconds: 9.581828478,
        setupSeconds: 103.902004359, wholeSeconds: 115.313311131,
        requestLatencies: [[4.496918537, 2.028198634], [1.980684097, 1.953152477],
          [2.028139965, null], [1.878016961, 1.178604987]],
        comparisonSha256: "7b6aac970fea7d108ad8be5a53bec90489667f25a7b70ba50b963cfcb96b9854",
      },
      {
        id: "device-tp1-residual", name: "TP1 device residual", repetitions: 1, collective: "device-tp1-v3",
        outputTokensPerSecond: 0.9533933332947702, workloadSeconds: 8.391080282,
        setupSeconds: 102.418669655, wholeSeconds: 112.654805442,
        requestLatencies: [[3.684019991, 1.788732392], [1.778435116, 1.769950624],
          [1.788662003, null], [1.751118337, 1.167159043]],
        comparisonSha256: "38663d06c56cbb99275b87551aa05dd0c843dce417ed7fb9835e4b9384460f29",
      },
    ],
  },
  deviceTp1Repeated: {
    scope: "Repeated TP1 residual comparison, two repetitions per profile, with run order reversed in R2. R1 is the previously published pair, not an additional run. Both repetitions retain the same bc4 controller, 705 worker, thirteen-root 306a image, four-request/eight-output workload and single device. Baseline projection/attention, operational runtime on, cache on, pruning off: only the collective changes. This is not multi-device peer transport.",
    interpretation: "Mean workload output rate improves 13.85% and reuse-prefix TTFT 6.30%. Mean reuse-prefix TPOT improves only 0.59%, from 1.178 to 1.171 s, too small for a robust gain claim. Process means and ranges retain both observations; nearest-rank p50/p95 at n=2 are the minimum/maximum, not a stable tail. Request identities are never pooled. The original R1 table is retained below; no current-runtime or cumulative profile is substituted.",
    repetitions: 2,
    ledgerFileSha256: "41b064209d601df9d733210825fd39bfee7665950e67c7fd77ff647ef64f6f32",
    secondProfiles: [
      {
        id: "device-tp1-control-r2", name: "TP1 host control R2", repetitions: 1,
        collective: "host_staged_fp32_rank_order_reduce_bf16_residual",
        outputTokensPerSecond: 0.8370592734569572, workloadSeconds: 9.557268229,
        setupSeconds: 104.359163366, wholeSeconds: 115.760185503,
        requestLatencies: [[4.491536604, 2.021997421], [1.967823286, 1.944313129],
          [2.021949842, null], [1.866568829, 1.177105367]],
        comparisonSha256: "7ebeddb92b75710d661cc5b250d91ad948f2dd533aec36fe816ec2a4c7ab98bf",
      },
      {
        id: "device-tp1-residual-r2", name: "TP1 device residual R2", repetitions: 1, collective: "device-tp1-v3",
        outputTokensPerSecond: 0.9501248176597648, workloadSeconds: 8.41994636,
        setupSeconds: 102.639536155, wholeSeconds: 112.896809999,
        requestLatencies: [[3.6878526, 1.799595627], [1.778896085, 1.778705063],
          [1.799523409, null], [1.757727941, 1.174683634]],
        comparisonSha256: "98ac06034589536e9ef9372615f3f9446c1657d74f7d3bc825ff65c937c9cb2c",
      },
    ],
  },
  peerObservations: {
    scope: "Original peer checkpoint: serial device-peer transport passed the fixed four-request/eight-output Qwen workload at TP2 and TP8, n=1 each. Prefix cache on, pruning off, operational runtime on, baseline projection/attention and all other performance flags off. Each run passed exact token/byte, dispatch, close/reap and all-eight-card idle checks. These are the same candidate observations reused by the later source-matched comparison above.",
    interpretation: "No matched host control was available at this original checkpoint. Later source-matched controls, using different worker executables on the same core source, now expose large peer-path regressions above. The original TP8 sample remains very slow: a 702.944 s workload window and 1557.722 s whole process. It is retained as negative performance evidence, not hidden behind correctness success. Older version-mixed host runs are not a causal transport comparison, and no peer speedup is established.",
    matchedControl: null,
    speedupClaimed: false,
    collective: "device-peer-serial-v4",
    pins: {
      controllerSha256: "d03089baca3857a2b64be5dd04c9f11c1e64d3fe256553191516ca4f8aaa1395",
      workerSha256: "6891fb584891cf4d6039989d5055219746a7c71bad3055b59fc9e41bdfb2d1ca",
      baseHsacoSha256: "af5019d3cfc4e860b33ebf0d97a82439f870a9e4e893c730118a97d8735c2d6a",
      baseManifestSha256: "99a7ed6f84ad1d863917c425fd51cd38105e3aee35824941b2c7f25f51e9641c",
      baseHandoffSha256: "4f9a1cd4e3fda57243ba264160c29117b13aebb96a03de16b3932a0d8257eaa9",
      peerHsacoSha256: "9d4f1b922967521b05199ce7d1e0269eaae3afa4fe558eafc4ba0bda3c4a4b65",
      peerManifestSha256: "e0242eefcffc7b278047467b118087b5336127e45dc5ee96f7991ea138e1ef2e",
      peerHandoffSha256: "ba2a796c529e2cac510cafff3ae9da72b7d3523de1774a9f9e88ce71a4ed16cc",
      comparatorSha256: "0885d2f183d7e280db1b5bda6ad16b73388965ab98c786f7fd6d8d61d3ac4416",
    },
    profiles: [
      {
        id: "peer-tp2-operational-r1", name: "TP2 serial device peer", world: 2, repetitions: 1,
        outputTokensPerSecond: 0.14649390488451794, workloadSeconds: 54.609780566,
        setupSeconds: 220.861978648, wholeSeconds: 285.630395408,
        requestLatencies: [[22.332074734, 11.07489371], [11.10570281, 11.057701605],
          [11.074844901, null], [11.040452221, 10.162302622]],
        comparisonSha256: "7452fafe952ead228830782e83c291a8f4d1b345b90cc50ab690114f6713e9ee",
        metricsFileSha256: "b7b5af6dea6b681b054a7bc664395c2992705df5cf39ad44c5a027d11dea4004",
      },
      {
        id: "peer-tp8-operational-r1", name: "TP8 serial device peer", world: 8, repetitions: 1,
        outputTokensPerSecond: 0.01138070145812333, workloadSeconds: 702.944368538,
        setupSeconds: 734.921241644, wholeSeconds: 1557.722091593,
        requestLatencies: [[282.561279842, 141.164665142], [140.990287001, 141.0913346555],
          [141.164599543, null], [141.017906811, 138.200419385]],
        comparisonSha256: "e1babe489df564a1240679fd6062e148a60eac27039af3b6f47822d7d722e2bd",
        metricsFileSha256: "f27a0729075dbc812735fffe96f5a7c41dd44be8b50c40aeb90c40df09fb3bb4",
      },
    ],
  },
  peerSourceControls: {
    scope: "Later source-matched TP2/TP8 controls use the same d030 controller and public fe2o3 902 core source as the serial-peer observations, with the same base image, physical roster within each world, cache-on eight-output workload, baseline arithmetic, operational runtime on and pruning off. The host path uses independent worker 189; the peer path uses distinct peer worker 689 and its additive v4 image. This is source-matched transport comparison, not byte-identical workers or a comparison with older runtime versions.",
    interpretation: "Serial peer is substantially slower in both n=1 pairs: TP2 workload rate is 82.06% below its source-matched host control, and TP8 is 97.66% below. Setup and whole-process costs also regress. The original peer observations are reused with their exact identities, not counted as new repetitions. No peer speedup or stable-tail claim is made.",
    sourceRevision: "902fef6e1478b3ac677e5456b2a2d1f917456fba",
    controllerSha256: "d03089baca3857a2b64be5dd04c9f11c1e64d3fe256553191516ca4f8aaa1395",
    hostWorkerSha256: "189b918dcd3f7104767404c636eb08490a3b1714a6f78123ea3a1fa06f21babb",
    peerWorkerSha256: "6891fb584891cf4d6039989d5055219746a7c71bad3055b59fc9e41bdfb2d1ca",
    controls: [
      {
        id: "host-source-matched-tp2", name: "TP2 source-matched host", world: 2, repetitions: 1,
        outputTokensPerSecond: 0.8167445332136524, workloadSeconds: 9.794984447,
        setupSeconds: 106.633300277, wholeSeconds: 120.083976933,
        requestLatencies: [[4.945205635, 2.07249364], [2.090021755, 1.868091539],
          [2.072412141, null], [1.66360459, 1.113595734]],
        comparisonSha256: "7f8d0e3ef8f46550c2b722f7dd821a684d77a1ddcac591af5af3d8b02013595d",
        ledgerFileSha256: "d59696cb6616994677e42db34aab16cd33396a99a90a29f29965392b135f5a85",
      },
      {
        id: "host-source-matched-tp8", name: "TP8 source-matched host", world: 8, repetitions: 1,
        outputTokensPerSecond: 0.4856413040195557, workloadSeconds: 16.473063419,
        setupSeconds: 116.510719193, wholeSeconds: 147.415542568,
        requestLatencies: [[9.375951126, 3.484686582], [3.205926934, 2.956638365],
          [3.484655443, null], [2.428528709, 1.183835563]],
        comparisonSha256: "f81641879b3092f530ecb088e8c900fcb0c7dd3421e0d21eae45536a007890cf",
        ledgerFileSha256: "886a3d29d7555cda2de47ab3ea305f0567bab235a7d1384ee2700b0259ab1ccc",
      },
    ],
  },
  hostTranspose: {
    scope: "CPU transpose helper only, shared mi300x AMD EPYC 9454 without CPU affinity or exclusivity. A 32-by-32 BF16 byte tile preserves exact NxK/KxN shard orientation with 2 KiB scratch. All 80 production TP1/TP2/TP8 rank/kind matrices pass three alternating repeats: 240 full-array equality checks. Separate tests cover every BF16 bit pattern, tile tails and invalid extents.",
    interpretation: "Times are sums of per-case three-repeat medians for one matrix per projection kind/rank, not all model layers. Allocation, source creation, full-array comparison, hashing, upload and inference are excluded. This is not a measured model setup, TTFT, TPOT or throughput improvement; the matched MFMA model pair predates this change.",
    modelTiming: false, allocationHashUploadIncluded: false, repetitions: 3, fullArrayComparisons: 240,
    source: "4049ad5",
    summarySha256: "bec975d68a204c7073204f6e5851475d249047564d7352269ebcc2fa5e47e40e",
    rawLogSha256: "33ef5e1277f3733814dbd83da4ffbce4af6a1673cd739b64329860324b27f6e6",
    groups: [
      { world: 1, cases: 8, baselineSeconds: 2.666687908, tiledSeconds: 0.790295324, helperSpeedup: 3.374292909266916 },
      { world: 2, cases: 15, baselineSeconds: 2.644065379, tiledSeconds: 0.786583584, helperSpeedup: 3.3614550732856383 },
      { world: 8, cases: 57, baselineSeconds: 1.920318993, tiledSeconds: 0.720505333, helperSpeedup: 2.665239110728414 },
    ],
  },
  transposeModelPair: {
    scope: "Separate full-model transpose observation, n=1 each: same public 902 worker 189, full MFMA image, TP8 roster, eight-output cache-on workload, operational runtime and MFMA projection enabled, pruning off and other controls unchanged. Controllers differ: bef is untiled, c5d8 contains the setup-only bit-exact tiled transpose. This pair is not the older bc4/705 MFMA projection experiment or the CPU-helper benchmark.",
    interpretation: "Observed setup falls from 173.195 to 146.643 s, a 26.552 s reduction (15.33%); whole-process time falls from 212.744 to 183.432 s (13.78%). Exact output, byte and teardown checks pass. The implementation change is setup-only: the observed request latency and output-rate differences below are not a causal decode-speed claim. Single observations on a shared host still need repetition.",
    causalDecodeSpeedupClaimed: false,
    pins: {
      workerSha256: "189b918dcd3f7104767404c636eb08490a3b1714a6f78123ea3a1fa06f21babb",
      hsacoSha256: "8c81d3fe869d3346d95486354f366988ce99210111cd0fc712dfb2194450b125",
      manifestSha256: "76e582363a35d48e993fbd68f15438e853a2270b4bce27ad989d96d3370f1ce8",
      handoffSha256: "6aaa62549ee5ca71e8e1da2aed0eba6d86fed0bc0b08edcdfdbca68a1cc48bd1",
      ledgerFileSha256: "389c7b8ee88fb443e2f3396cb0978e92509142e58c3087321de43bc097d0b191",
    },
    profiles: [
      {
        id: "transpose-untiled", name: "Untiled setup transpose", repetitions: 1,
        outputTokensPerSecond: 0.4514832997707543, workloadSeconds: 17.719370803,
        setupSeconds: 173.194680357, wholeSeconds: 212.744133297,
        requestLatencies: [[10.720460009, 3.121825927], [3.605721043, 2.836538182],
          [3.121791508, null], [2.551195878, 1.32583443]],
        controllerSha256: "bef12d573a77741cd0a3719d8b5aa65d1630085d98df5ba54b244b2e7a33893b",
        comparisonSha256: "eac7ab437578cc67e2803a3ecd08828252ca661b06deb4b2087a8b493816c1f1",
      },
      {
        id: "transpose-tiled", name: "Tiled setup transpose", repetitions: 1,
        outputTokensPerSecond: 0.5358716142710381, workloadSeconds: 14.928949,
        setupSeconds: 146.643120536, wholeSeconds: 183.431642533,
        requestLatencies: [[9.320251571, 2.88669315], [2.918486836, 2.3659460345],
          [2.886660621, null], [1.84514648, 0.87680536]],
        controllerSha256: "c5d8cda049ba2a89dceeb567f9e5aec4d97d321ce42125999442a51d65362187",
        comparisonSha256: "226fb579a7de449d532a833acd120c5931a1499b9acaaa825d8fbd243bc8d999",
      },
    ],
  },
  replicaCohorts: {
    scope: "Separate eight-GPU allocation experiment: eight globally named identical five-token prompts, eight greedy outputs each, 64 total outputs and seven decode intervals per request. All arrivals at zero, cache off, no cancellation, baseline projection/attention, operational runtime on, legacy host-staged reduction and no pruning. One cohort per layout, n=1; not the four-request/eight-output workload and not steady-state serving.",
    interpretation: "All three serialized cohorts pass exact comparison. The highest observed workload rate is 8xTP1 at 6.588 tok/s, versus 5.734 for 4xTP2 and 1.442 for 1xTP8. Relative to 1xTP8, the samples are 3.977x and 4.570x faster for 4xTP2 and 8xTP1. The 16-row budget is per instance: aggregate capacity is 16, 64 and 128 rows, and host model payloads are duplicated 1x, 4x and 8x. This is an allocation-policy comparison, not a pure kernel speedup or a rule for longer workloads. Setup dominates whole-process time; 4xTP2 has the shortest observed spawn-to-reap window. Each layout has n=1, with no stable-tail claim.",
    clockScope: "CLOCK_MONOTONIC_RAW on one verified host, boot and time namespace. Primary rate is 64 divided by common future release to last output, not a sum of instance rates. Admission TTFT excludes the small release-to-admission interval; release-to-first-token includes it. Barrier setup is first spawn to all Ready, excluding the one-second release lead; whole cohort is first spawn to last reap. Global idle snapshots bracket the cohort, not individual replicas.",
    limits: "Weight bytes are loaded BF16 payloads only, not host RSS or full GPU usage; they exclude KV, activations, scratch and allocator/page rounding. Retained control frames, domain/nonce/epoch, executable hashes, exact outputs, close/EOF and owned-group reap are checked. Expectation digests identify canonical JSON, not original file bytes. This is not execution attestation or a cgroup-wide proof against detached descendants. No serving SLO or protected qualification is granted.",
    pins: {
      controllerSha256: "bef12d573a77741cd0a3719d8b5aa65d1630085d98df5ba54b244b2e7a33893b",
      workerSha256: "189b918dcd3f7104767404c636eb08490a3b1714a6f78123ea3a1fa06f21babb",
      hsacoSha256: "af5019d3cfc4e860b33ebf0d97a82439f870a9e4e893c730118a97d8735c2d6a",
      manifestSha256: "99a7ed6f84ad1d863917c425fd51cd38105e3aee35824941b2c7f25f51e9641c",
      handoffSha256: "4f9a1cd4e3fda57243ba264160c29117b13aebb96a03de16b3932a0d8257eaa9",
      workloadSha256: "dc12dbb07cf8c28213c37ada8f2fff82778c860b82172edfb51b62efd005973b",
      referenceSha256: "1ed868663df52a146dd7921f9fbb2cf1e1d0c0ceed8a7bfda030d65f8ec5b094",
      cohortComparatorSha256: "14f046beffcdf717b7ade89204a416b86cfc1e48614c895cd69f6431bd5ec797",
      traceComparatorSha256: "f1850669bb2ff4860b99ef2b250a7a9f029602700801993afe413772c8496a80",
      batchComparatorSha256: "0885d2f183d7e280db1b5bda6ad16b73388965ab98c786f7fd6d8d61d3ac4416",
      allocationComparisonSha256: "47d98300668c31db0dd8f91846a4d6a7b361e353c419d7d17a911c7625ec7b32",
    },
    profiles: [
      {
        layout: "1xTP8", replicas: 1, world: 8, repetitions: 1, perInstanceRows: 16, totalRowBudget: 16,
        outputTokensPerSecond: 1.4415849152450675, releaseToLastOutputNs: 44395581088,
        barrierSetupNs: 120618727705, spawnToReapNs: 181017831576,
        releaseEpochNs: 694652457558551, maximumLatenessNs: 53110, physicalTokenRows: 96,
        hostWeightBytes: 16381470720, gpuBaseWeightBytes: 16385728512, gpuTransposedWeightBytes: 0,
        requestLatencies: [[0, 6365368002, 6365516662, 4.625090113428572],
          [0, 6365352772, 6365516662, 4.625090113428572], [0, 6365343922, 6365516662, 4.625090113428572],
          [0, 19221929878, 19222109708, 3.5962101971428573], [0, 12463431341, 12463621361, 4.225866988857143],
          [0, 12463423761, 12463621361, 4.225866988857143], [0, 19221905428, 19222109708, 3.5962101971428573],
          [0, 19221898428, 19222109708, 3.5962101971428573]],
        comparisonSha256: "8891e3d384f97030446d232e8ba24dac3128e192829c755f8747fd9d4ef42951",
        expectationSha256: "476c30ae94739ec57166e4594c74adc6fe9bb6c3dd96c4117bf354c94b9689a1",
      },
      {
        layout: "4xTP2", replicas: 4, world: 2, repetitions: 1, perInstanceRows: 16, totalRowBudget: 64,
        outputTokensPerSecond: 5.733502743551746, releaseToLastOutputNs: 11162460866,
        barrierSetupNs: 121101113079, spawnToReapNs: 159338601475,
        releaseEpochNs: 694906472256042, maximumLatenessNs: 51370, physicalTokenRows: 96,
        hostWeightBytes: 65525882880, gpuBaseWeightBytes: 65528315904, gpuTransposedWeightBytes: 0,
        requestLatencies: [[0, 2142079442, 2142195952, 1.212192911],
          [1, 2164080889, 2164216949, 1.1808452944285714], [2, 2116970492, 2117115622, 1.2921921777142855],
          [3, 2106447829, 2106577079, 1.2383359154285714], [0, 2142070552, 2142195952, 1.212192911],
          [1, 2164071419, 2164216949, 1.1808452944285714], [2, 2116957572, 2117115622, 1.2921921777142855],
          [3, 2106439179, 2106577079, 1.2383359154285714]],
        comparisonSha256: "67ef374b3d6419b780a9f7281a9b1696fd9967de4d3ba1efeb09b8d7547ecb7d",
        expectationSha256: "a956f090185d750fbcbafc0efa9b07ab3463d17fc620474b07425524738b40bf",
      },
      {
        layout: "8xTP1", replicas: 8, world: 1, repetitions: 1, perInstanceRows: 16, totalRowBudget: 128,
        outputTokensPerSecond: 6.587693123582852, releaseToLastOutputNs: 9715085205,
        barrierSetupNs: 121681735042, spawnToReapNs: 162828117278,
        releaseEpochNs: 695113518642993, maximumLatenessNs: 52300, physicalTokenRows: 96,
        hostWeightBytes: 131051765760, gpuBaseWeightBytes: 131051765760, gpuTransposedWeightBytes: 0,
        requestLatencies: [[0, 1877832702, 1877965702, 1.1195885004285715],
          [1, 1872439565, 1872569965, 1.1129982595714285], [2, 1866916799, 1867059449, 1.1160228717142857],
          [3, 1865435667, 1865867807, 1.109606644], [4, 1848108396, 1848269916, 1.111447492],
          [5, 1840283436, 1840427716, 1.1090357577142855], [6, 1846690034, 1846817934, 1.1014082227142856],
          [7, 1861816752, 1861957042, 1.110052346]],
        comparisonSha256: "729d282edf92ac0418f9f501afee60143382dab64dbf04537240ba8983257856",
        expectationSha256: "3959eb6cf6df17be6e42fc7fc6fac3401d1b3cad1870bc53a62b3772aa3192eb",
      },
    ],
  },
  wideRowPair: {
    scope: "Separate TP8 row-policy pair on the same wide V5 image, controller, worker and 64-output cache-off workload. Batch budget and prefill chunk change together from 16 to 32; all other controls are identical. The image profile is named v5-mfma32, but projection and attention selectors are baseline, not MFMA or wave. Only operational runtime is enabled. Both runs pass exact reference, shared-clock, idle and teardown checks, n=1 each.",
    interpretation: "The 32-row sample raises common-release output rate 12.56%, from 1.323 to 1.489 tok/s, with a genuine observed 32-row batch. Request00 admission TTFT worsens from 6.512 to 9.641 s while its TPOT improves from 5.085 to 4.415 s. Other request identities are shown separately; this is not a uniform latency gain or a repeated result. Both policies execute 96 physical rows for 64 outputs. Neither is pooled with the allocation trio.",
    timing: "The allocation section's RAW common-release, spawn-to-all-Ready barrier and spawn-to-reap definitions apply unchanged. Each policy uses one TP8 instance and the same loaded weight bytes: host 16381470720, GPU base 16385728512, transposed 0. These payloads exclude KV, activations, scratch, allocator overhead and page rounding.",
    pins: {
      controllerSha256: "bef12d573a77741cd0a3719d8b5aa65d1630085d98df5ba54b244b2e7a33893b",
      workerSha256: "189b918dcd3f7104767404c636eb08490a3b1714a6f78123ea3a1fa06f21babb",
      hsacoSha256: "98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502",
      manifestSha256: "c559d0533907323aff7dda03215adab6326423c3f151b296e22394c9baf37d00",
      handoffSha256: "94807eb1b5f78ce1b9e217b464eb5a98d6a270c6aeb932d8847b5ff4315c7756",
      pairComparisonSha256: "0bd49a1c66cb297e9eb20fd502ca5c99faefbaf722bd963de3acb96b5cc6fb6e",
    },
    profiles: [
      {
        name: "16-row wide-image control", rows: 16, prefillChunk: 16, repetitions: 1,
        outputTokensPerSecond: 1.3229060452041055, releaseToLastOutputNs: 48378341177,
        barrierSetupNs: 129109870999, spawnToReapNs: 193535131512,
        releaseEpochNs: 695317578358509, maximumLatenessNs: 52360,
        actualBatchRows: [16, 16, 16, 8, 8, 8, 8, 8, 5, 3],
        requestLatencies: [[6511720329, 6511879219, 5.084949181285714],
          [6511705349, 6511879219, 5.084949181285714], [6511696129, 6511879219, 5.084949181285714],
          [20360831777, 20361021507, 4.002474238571429], [13612313391, 13612512591, 4.571947384857142],
          [13612306161, 13612512591, 4.571947384857142], [20360809076, 20361021507, 4.002474238571429],
          [20360802976, 20361021507, 4.002474238571429]],
        comparisonSha256: "01a6e3579689a7bf9c219f5dcc00480bfd3540f237127065d495ee76c26ab5a7",
        expectationSha256: "00ed08c1fd857db93ab06d04396e058585e480d231bdd35ef5fd86e8ece9bf3f",
      },
      {
        name: "32-row wide-image policy", rows: 32, prefillChunk: 32, repetitions: 1,
        outputTokensPerSecond: 1.4891165175039356, releaseToLastOutputNs: 42978503863,
        barrierSetupNs: 124549474851, spawnToReapNs: 183584419202,
        releaseEpochNs: 695506916337773, maximumLatenessNs: 54240,
        actualBatchRows: [32, 14, 8, 8, 8, 8, 8, 8, 2],
        requestLatencies: [[9640646265, 9640800695, 4.414651980142858],
          [9640629395, 9640800695, 4.414651980142858], [9640619925, 9640800695, 4.414651980142858],
          [9640612215, 9640800695, 4.414651980142858], [9640600725, 9640800695, 4.414651980142858],
          [9640592875, 9640800695, 4.414651980142858], [15604016791, 15604231831, 3.9106102902857143],
          [15604009591, 15604231831, 3.9106102902857143]],
        comparisonSha256: "0f67a449f4d17a5ec7012d551e899c4b9303d880c7ce06f6f0a55ecec9e026b7",
        expectationSha256: "c46b18ac96a64de127ad0221d3158c5317ca47895987f689c4d7f38d699cef2b",
      },
    ],
  },
  fixtures: [
    ["Runtime controls and rollover", "20 GPU dispatches pass across four admission-cache/operational combinations. Exact outputs and retained buffers survive dispatch, rollover, and dispatch again; three invalid epoch/frontier/sequence cases fail before dispatch. Clean worker exits and all eight GPUs idle. Separate operational, cache, sequence, and combined model profiles now pass the fixed workload; rollover still has lifecycle evidence only."],
    ["Wave kernels", "26 native fixtures pass, but both wave-projection-only and wave-attention-only Qwen runs produce seed token 9856 instead of reference token 17689. Each is a correctness failure under investigation; both are excluded from timing tables and ledgers. A stronger projection differential passes 14 shapes and 28 native dispatches with exact sampled own-order checks and intact guards, inputs, and tails. It observes reduction-order differences but does not resolve Qwen parity. The same-image control with baseline arithmetic and operational runtime enabled passes all eight reference tokens, dispatch/exit and physical-idle checks. Although the original failed candidates had a different runtime flag, later operational-matched projection-only and attention-only runs both still fail with the same Germany-versus-Spain seed mismatch. Each now differs from the accepted control only in its arithmetic selector. Their rejection hashes are retained, and no timing or speedup is accepted."],
    ["Checked peer owner", "TP2 and TP8 native owner probes pass 8 and 128 all-case checks respectively against public fe2o3 d854c9a. Thirteen separate v4 arithmetic fixtures pass. GPU-producer TP2/TP8 probes also pass 6 and 24 observations, including BF16 GPU copy chains and actual FP32 GPU projection into peer reduction, with exact active outputs and guards. The later opt-in cached-sequence API is public at 902fef6 after separate native TP2/TP8 producer probes. Further public-API producer probes pass on both world sizes at 17, 31, and 32 rows, including two-dispatch cached sequences. Serial-peer TP2/TP8 Qwen passes the strict fixed-workload checker. No matched host control existed at the initial checkpoint; later source-matched controls now show large regressions with the expected distinct host and peer worker binaries. No peer speedup is established."],
    ["MFMA compiler and kernels", "The full fifteen-root image emits and 36 native fixtures pass with exact outputs, input/guard checks, and clean teardown; embedding and RMSNorm are not exercised by that synthetic suite. This image uses the frozen backend and SDK 3546, not the new 97ef compiler fixes. The separate same-image MFMA model pair passes the fixed-workload checker, with request gains and a startup regression. Wider-image V5 and V6 suites pass 30 and 19 native fixtures; V5 exercises 14 of 15 roots, omitting embedding. Independently, two generic analysis fixes are public in fe2o3 97efcdc and admit an unchanged model-free MFMA repro through formal/ranked checks and LLVM to HSACO. Real host-coordinator tests confirm wider schedules, changed slot generations and physical prefix reuse. A later separate V5 Qwen row-policy pair now passes 64 outputs at actual maxima 16 and 32 rows, with baseline arithmetic and an explicit TTFT tradeoff. Synthetic and model receipts remain distinct; repeated measurements and protected qualification remain open."],
  ],
  identities: {
    hsacoSha256: "af5019d3cfc4e860b33ebf0d97a82439f870a9e4e893c730118a97d8735c2d6a",
    manifestSha256: "99a7ed6f84ad1d863917c425fd51cd38105e3aee35824941b2c7f25f51e9641c",
    handoffSha256: "4f9a1cd4e3fda57243ba264160c29117b13aebb96a03de16b3932a0d8257eaa9",
    workloadSha256: "23882e195cf578b987c72fc5703fffb51541708e5431d4166e55e0fe7e2f137f",
    referenceSha256: "1ed868663df52a146dd7921f9fbb2cf1e1d0c0ceed8a7bfda030d65f8ec5b094",
    operationalLedgerSha256: "17549917bfa7f4b7d6745a2f1aa4de1ac799373b6568ca5134eb8a2c9e21a7b2",
    pruningLedgerSha256: "88a0de0d981eeb1a4ee42940e24aa95259892293bd33e59bc1782d66afe0e9e7",
    waveImageControlHsacoSha256: "306a27d8cdb2376d57a2b12d94fd2aa06bf19532175a31e15ee5390d7de87682",
    waveImageControlManifestSha256: "596a455c6810fc7271022582f00f99b393bef2522ef1d881a495cb40e939ec07",
    waveImageControlHandoffSha256: "e1193656121858522c0b83c64d6ea72bd32a5ecd02b151e9acb8261ba92d775d",
    waveImageControlComparisonSha256: "649caed0676c591c840690eade9fd68aa767983ffb71b47789fb819384ce9030",
    waveProjectionOperationalRejectionSha256: "ad9ecdbc5e35a7013a1e3dd68fcaba5344ed3b5b3b3dc07981b5250d930002b1",
    waveAttentionOperationalRejectionSha256: "494a2a258ed6df34a5f86c4f216b1032eed5f383757dc3ce6b83f029e171756c",
  },
  provenance: "Historical two-repetition ledger hashes remain preserved above. The six-completed queue ledgers recheck all eleven accepted runs with integrated comparator 0885d2f1 (45 host tests pass), preserving old report contents except checker hash. Both rejected wave profiles remain excluded. Exact baseline-relative and same-controller-control ledger hashes are listed separately. The later numerical image control uses the queue's bc4a controller and 70572 worker, the operational-only profile, and the separately pinned thirteen-root wave image; its accepted comparison is listed but no timing is added to these historical ledgers. Raw receipts remain local. Baseline and pruning use the frozen worker and nine-root artifact; operational and the queue use reviewed 7abce5c runtime with that same artifact. Newer public compiler fixes do not retroactively change binary identities.",
  publication: "Ferric model kernels, inference changes, raw receipts, and model data remain implementation-local. This publication changes the site only. Generic compiler/runtime changes are public in fe2o3; the observation cutoff is public 7528e7345cef0158d7034cdbae23011e3c6fb5d2 on September 10. Combined host gates now pass on 7528 as well as the preserved earlier 1b262 checkpoint; frozen measured binaries are not relabeled as newer source. No production endpoint, matched vLLM/SGLang baseline, or protected M1 qualification is available.",
};
