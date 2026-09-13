import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const siteRoot = dirname(fileURLToPath(import.meta.url));
const clone = (value) => JSON.parse(JSON.stringify(value));
const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");
const fixed = {
  schema: "FerricLiveC1PagesCheckpointV1", authority: "none", m1OpenGates: 33,
  source: "87f38de73cf7604497ac838c30e06efe84d4102d", tree: "e2ba291cb4abff7ebde5fd313cf3ad3238c00513",
  controllerSha256: "8ae69215cf93524ae3438f84bad6d4ce146a92d6636d41a8fc854b897233d450",
  workerSource: "c110ac55c655579e0969b310402800b0c2666694",
  workerSha256: "b91ddef78135829f607d1b83f5cf898d745b36013327f0d2d12771aba1b5150b",
  clientSha256: "979136caea4f134f33f19c62b82a8ac9537205eaa11d11a43b7a3af466af0a2d",
  serveSha256: "347754cb8d88da4639beb0163f54c0ca091288e9c11d6e47e29e47db1dddaf7d",
  referenceSha256: "cd7f512537e44d677244c606cba919e57747e8c26caf3c0df4a0aca7acc0eb8b",
  qualificationReviewSha256: "2c4a5b9e55b783482c801b49b2112df1c0242f78ee7f35d77658ca2b92f5a82c",
  matchedNoteSha256: "e96aae1db3734c477f09a8fbeca645f2a02909c40354c27607b63d5da5efdf9b",
  model: "Qwen3-8B", hardware: "1 x MI350X, physical GPU 0", concurrency: 1, speculation: false,
  context: 8192, physicalPages: 512, tensorParallel: 1, rowCapacity: 32, prefillChunk: 16,
  inputTokens: 128, outputTokens: 128, submission: "ordered", attention: "wave", headPrecision: "fp32-v8",
  argmax: "wave-v11", prefixCache: false, runtimeProfiling: false, hostTiming: false,
  matchedState: "independent-pair-replay-passed", httpTimingAdmitted: true,
  freshStartsPerMode: 1, diagnosticRequests: 2, warmups: 10, measuredRequests: 30,
  servingQualified: false, stablePerformanceGainClaimed: false, competitiveRanking: false,
  defaultPromotion: false, gpuDurationMeasurement: false, historicalCohortsMixed: false,
};
const qualifications = [
  { layerProjection: "mfma", requests: 2, exactOutput: true, normalUnforcedClose: true,
    allEightIdleBeforeAfter: true, batches: 270, dispatches: 166278,
    receiptSha256: "e46506a887373002be3345e5de147c41c8f2def26850201261d1e402d1815a6c",
    planSha256: "64f465fb1e0a7c0bc35dd8290d059b6dc18f9de48bc4a601e4bbec91a308f6dc",
    archiveSha256: "b71f038b8c6a3f6436d634026381edd0b7652751fdbd23f80cbb432232bc258f" },
  { layerProjection: "c1-wave", requests: 2, exactOutput: true, normalUnforcedClose: true,
    allEightIdleBeforeAfter: true, batches: 270, dispatches: 166278,
    receiptSha256: "573411cb13fe89a56d9fe65244f61729b3d1a5bdaa4f7c77c20f9d6a473d2639",
    planSha256: "f0d09b4ec71f500abc071e2887ad2c420bec5e6e54df486dc7fd8bb4bc3c7ec5",
    archiveSha256: "1af0b1eec88b5f410e330ed85f3ab876489dcc64a208158a4efdbe9f172a0016" },
];
const cohorts = [
  { layerProjection: "mfma", case: "layer-live-http-matched-mfma-r1",
    planSha256: "ae42a92f3dd88268172e6a52a3e6dc9c66fdbcf3ae249a88fcaad77ce8f8fbfe",
    receiptSha256: "9a5e0b80ec86e719861663148699bbdfeddf68b96e3112c4fe07f919cf4140dc",
    replaySha256: "783f0dcc71b5335667d093b0fe0249558363b4755ad8e37ea1c895b0e067af09",
    archiveSha256: "8e1a8fd6659564c5b2731ff71be60e45ee6c4ea367c4895ae6ff6f67703319fd",
    ttftMeanMs: 2873.6324273, ttftP50Ms: 2807.9037015000004, ttftP90Ms: 3112.8933716, ttftP99Ms: 3131.14292134,
    tpotMeanMs: 199.71890523333334, tpotP50Ms: 199.19604226771654, tpotP90Ms: 218.9746053488189, tpotP99Ms: 221.8084090627559,
    outputTokensPerSecond: 4.532744999020145, windowSeconds: 847.168768777 },
  { layerProjection: "c1-wave", case: "layer-live-http-matched-c1-wave-r2",
    planSha256: "60e130468d1b07ca3813ff6059cfbe3040da4e7d7f96b82e17ada6d9a969d9e9",
    receiptSha256: "3c2e4731a75127ff012d35a64aad5c4ee89824656daaf82a222c2a7d225a2c5c",
    replaySha256: "0814bd8f79ad98003d54cfbcf1ca48e9dff9880653328dc445927e2f88959576",
    archiveSha256: "260d126f37b964e8733a56c61305b6f50147288b0712ca8eeba3172145d5f5ea",
    ttftMeanMs: 2827.804126, ttftP50Ms: 2806.7118325, ttftP90Ms: 2989.3104293, ttftP99Ms: 3082.9398334499997,
    tpotMeanMs: 154.30964321233594, tpotP50Ms: 150.06444650787404, tpotP90Ms: 180.39347754409448, tpotP99Ms: 184.06441449952754,
    outputTokensPerSecond: 5.707624290259268, windowSeconds: 672.78429776 },
];
const excluded = { case: "layer-live-http-matched-c1-wave-r1", requests: 0, timingAdmitted: false,
  receiptSha256: "d989d4e96f0bad92ceb3832c8c97bb07651c47268fd47f648ee65af62068f0f4",
  archiveSha256: "8b61ba88d70ad416c1b7b238ad5fc01fa8f5793e6b303fdfb45799ff4ba267a1" };
const v13 = {
  source: "256a6e4be4c0197ba87519c9c0dde8e1b7ed58de", tree: "f6f86ae6268e7458241e5a1f69008f40081aef06",
  compilerSource: "8efd4fd416d1ffae7a718144e4d299fe3c8f7590", actualWorkerBuild: "21682228486f7186cc3c37ddf165fffc438d8b6a",
  unchangedWorkerSubtree: "613ef51b10cdb00c192b8c6292c06f051f519a6a",
  imageSha256: "72104603f91ac5037a7b106307fc7023e20860f905e912d43700a119381665db",
  observationSha256: "0bc419bd1e3d9ef90bac4b24304c65ec8aabc29cbca329258b4a8ba99241aab9",
  resultSha256: "97eb29a8acc91baa5076f1f0ba529632252ef515a8998975d04da61bcc41895e",
  noteSha256: "ecd96c053d25a83858dd731a9952a50945e35b3edc33deedfbe95ef4f30a15d9",
  staticReviewSha256: "75dde455a821a7a81e4a1b53923496435c04b733b06117e81cec42ae35eabbcb",
  archiveSha256: "0601efc2d683bf6b02100f3d36d887ba5bbce4cbc22d92753d9135fc6d4ab9b6",
  phasesPassed: 8, roots: 2, explicitBytes: 52, hiddenStart: 56, kernargBytes: 312,
  sgprSpills: 0, vgprSpills: 0, privateBytes: 0, ldsBytes: 0, dynamicStack: false,
  abiResourcesPassed: true, detailedTypedReviewAdmitted: false, nativeAdmitted: false, performanceGainClaimed: false,
};
const v14 = {
  source: "1843d8f174b20ebd6dbc0e72d6fe9044e8d1f244", tree: "5742499d0b02f9a18fcba2b04a93e32321563fde",
  hostNoteSha256: "5e44ec6eee3fe2d6bd457bf8442de48bdbb84b5e1d7689e5bf5eeefe9b5bef0d",
  hostArchiveSha256: "9d025c4646b4751681230fb46036f738a6f38ebe5edd9bc4362a1e2f7dc7f579",
  controlNoteSha256: "cd77dc0bc4776c71892417e69797b1a0f6cc743d9c3b0250a3f3f8b46ccfa9a5",
  controlArchiveSha256: "f18848ad0870c83e92c62014616a3f4624824608f554d5a612ac432c2a4afd1e",
  hostTestsPassed: 13, strictClippyExit: 0, syntheticControlTestsPassed: 10,
  detailedTypedReviewAdmitted: false, nativeAdmitted: true, performanceGainClaimed: false,
};
const v14Emission = {
  imageSha256: "8f21681fe9103b670ee5666f429a45682e77fc90eb16802a02c4b6fb93a192c8",
  observationSha256: "eb058fceb9519c4e9f9fb9d347263dcb80e957c1accd87122c4e139a9e571752",
  resultSha256: "7de9014563122dcc7126a974aabefe2a8ea99d5eed3c80f44f7de84bd1a4342f",
  noteSha256: "5a9ffe76844fa78c9449088cd9242abd6bdc61a10f2577888f09799d08d77eda",
  staticReviewSha256: "c4ed8cff08dd0378311be425a2de699b88e543ef2539931c09e83d683c58d4c4",
  archiveSha256: "cb58d401389f3b85f06c81b126b59e2b12e8cad8da8d2b1425b77989fdacc7a0",
  phasesPassed: 9, syntheticTestsPassed: 11, roots: 1,
  explicitBytes: 116, hiddenStart: 120, kernargBytes: 376, kernargAlign: 8,
  sgprs: 95, vgprs: 37, sgprSpills: 0, vgprSpills: 0, privateBytes: 0, ldsBytes: 0, agprs: 0,
  dynamicStack: false, abiResourcesPassed: true, staticHoistObserved: true,
  rawHoistFlag: "pending", sameCompilerAblation: false,
};
const v14Native = {
  scope: "finite-eight-case-parity", cases: 8, checkedBuffers: 48, guardRegions: 96, protocolRows: 421,
  fixtureSource: "fec0311b3ec84de12ac16271eccf7515ed61e758",
  reportSha256: "f5a20e0300f3c11c660f9058a41b600471f8041d20faf1c826a2513b4a51a4c3",
  protocolSha256: "1bde1eb87d097317eb6a4b5f2ab27b0d364c34433358c26dc822870554c2cb53",
  wrapperSha256: "bbd943b97fad5140bd37a6ee3e1e2f4afc7116772977d7fd40d9a007ef4d20d0",
  cleanupSha256: "adb2976ce274a8f114429d295b7d633629ab1696c84dc650b88dc44219c662c0",
  archiveSha256: "bfb4ea3909a4a9673cbbc38c0727c0fdf1eff529ad9f7375eda909b41026ca9f",
  cpuReplayLogSha256: "28477b80434165df7472ceda4166581c6d3a28b61b8d2a6d363c2b21cc2f5759",
  cpuReplayClosureSha256: "35753ed8a7f1101a633c0bf237ee8bff68c1e4bfa9235eb279e0e8b8fed82410",
  cpuReplayArchiveSha256: "7403deacb6a422fe305c5836632a8e74a71f36d68824a38c2ba9a2887736c702",
  normalUnforcedClose: true, allEightIdleBeforeAfter: true, cpuReplayPassed: true,
  sameCompilerAblation: false, modelParityQualified: false, performanceQualified: false,
};
const v15 = {
  source: "c534e35258f6379b481380f0088e777356547efb", tree: "01dec2245a5d80f5d4de262f7fdb14883525dc97",
  dependencySource: "61e014ac28690cd993761590fd2f7d75d419d340",
  hostNoteSha256: "f230c4eb94c0b07fd5f18154550743713b99267b46fd94e8efa6456ef820a656",
  hostArchiveSha256: "3c63d3e74f680f85f213a4ee2fce0a68eee576b902b9a545dd5f088afb6fe594",
  testsLogSha256: "6e3598fb47c81ca24e12cb493f8923f918841661a81c46d15b9eb719d8ff9a57",
  clippyLogSha256: "1ceb245292588f66cd2a4a98b76c3b8d057b5170b7a6bb6d787d52ab77b42184",
  artifactReceiptSha256: "20b23aa878df1043e5fd8f7da2b4493ef6b76996eb434ab55642c64005533d72",
  lockSha256: "78154f4e92d8badc4839f6ef37c889fa105e7945bdb03f85aa3e1a9cdb9cfb52",
  hostTestsPassed: 20, contractTestsPassed: 6, hostModelTestsPassed: 14, strictClippyExit: 0,
  ownArtifactsFresh: true, emissionAdmitted: false, nativeAdmitted: false,
  modelParityQualified: false, performanceGainClaimed: false,
};
const development = {
  source: "7c9283675f1f7f40e781f2d774d4c0fcd9e31d69", tree: "73518159bdb1f3b9c688509eaf25e4b10ace7d80",
  fe2o3Source: "ae26717922b1fb7ad62fdd5ad70814d83eb01177", fe2o3Tree: "2ef28f830df9832d53a62525e20aa33815c164fa",
  canaryIntegrated: true, hostState: "validation-pending", hostAdmitted: false, workerHostAdmitted: true,
  workerTestsPassed: 484, workerTestsIgnored: 1, workerDoctestsPassed: 31, workerStrictClippyExit: 0,
  workerSha256: "526cc6bf8902128767a5318c90d1c0206a9e435957612c398eeb28ff2ae71032", workerBytes: 1591144,
  workerNoteSha256: "aaed3d109764d09f2701983e5cba85089358e51fb64aae26c3b7e4335305a238",
  workerArchiveSha256: "006295ae51234b789c8e054328a6ce22b84485cabdbe186f7e28d45a26dbd87d",
  workerReceiptSha256: "2a82fdfdf148ab68f60981550c6ae5fc8aea38251edd99760171543abebf2b32",
  workerTestsLogSha256: "cdc5935cebacddb1ff21f9060ab155d0e49a6fbf9aad40ead930d08880fb0f9c",
  workerDocsLogSha256: "e6e9e9e5a16c0c2a68bcf696aa0df3aba50573c45475fbf40df45a10ec79d4c9",
  workerClippyLogSha256: "7d3f20709fe76ad4c2c435857d91a9fc0f4e389fbc022d34508c1e183594ad36",
  workerBuildLogSha256: "58fcf627fda086459d9f56f836818fb84ed66a91c1e347e871fae90ce330d8e0",
  workerNativeAdmitted: false,
  checkerMethodsPassed: 32, currentWorkerWrapperMethodsPassed: 8,
  checkerArchiveSha256: "60a0753844cf780704fe8d4078a21e48a8c29c85a6a615b2486078d50c6a15b1",
  wrapperArchiveSha256: "89f771db8322b60aa9d852e292e0843e2bc3ba3d523eac07226db35e7e9d4bc1",
  modelParityQualified: false, performanceGainClaimed: false, defaultPromotion: false, historicalCohortsMixed: false,
};
function exact(value, expected, extra = []) {
  assert.deepEqual(Object.keys(value).sort(), [...Object.keys(expected), ...extra].sort());
  for (const [key, item] of Object.entries(expected)) assert.deepEqual(value[key], item, key);
}
function phrases(text, expected) {
  assert.equal(typeof text, "string");
  for (const part of expected) assert(text.includes(part), `missing boundary: ${part}`);
}
function close(a, b) {
  assert(Number.isFinite(a) && Number.isFinite(b));
  assert(Math.abs(a - b) <= 1e-12 * Math.max(1, Math.abs(b)));
}
function projectFrom(bytes) {
  const context = { window: {} };
  vm.runInNewContext(bytes.toString("utf8"), context);
  return clone(context.window.FERRIC_PROJECT);
}
export function validateLiveC1Checkpoint(input) {
  const value = clone(input);
  exact(value, fixed, ["qualification", "matchedCohorts", "excludedAttempt", "overview", "qualificationScope",
    "correctness", "measurement", "interpretation", "exclusions", "v13", "v14", "v15", "development"]);
  assert.deepEqual(value.qualification, qualifications);
  assert.deepEqual(value.matchedCohorts, cohorts);
  assert.deepEqual(value.excludedAttempt, excluded);
  exact(value.v13, v13, ["detail"]);
  exact(value.v14, v14, ["detail", "emission", "native"]);
  exact(value.v14.emission, v14Emission);
  exact(value.v14.native, v14Native);
  exact(value.v15, v15, ["detail"]);
  exact(value.development, development, ["detail"]);
  phrases(value.overview, ["independent replay at context8192", "199.719 to 154.310 ms", "4.532745 to 5.707624",
    "not a stable or competitive result", "all 33 M1 gates remain open"]);
  phrases(value.qualificationScope, ["Two sequential 128-input / 128-output", "same source 87f38de / controller 8ae69215",
    "context8192 / 512 pages", "ordered residual tails", "not a context256 proxy", "timing instrumentation are disabled"]);
  phrases(value.correctness, ["Qualification requests in both arms", "793 decoded UTF8 bytes", "exact SSE controller/request associations",
    "generations 1 then 2", "without cleanup signals", "270 batches and 166,278 dispatches",
    "These counts do not describe the separate 42-request matched cohorts", "All eight GPUs", "no broader concurrency"]);
  phrases(value.measurement, ["One fresh start per arm", "two exact-output diagnostics", "ten excluded warmups",
    "thirty measured requests", "All 42 requests", "all forty timed SSE/request IDs", "3,840 measured output tokens",
    "first sample-window start through the last sample-window end", "not GPU durations", "profiling stays disabled"]);
  phrases(value.interpretation, ["22.737% lower", "1.595% lower", "25.920% higher", "2873.632 to 2827.804 ms",
    "does not establish stable tails", "38.643 times", "was not rerun", "no competitive win or SGLang timing"]);
  phrases(value.exclusions, ["pre-worker socket-bind failure with zero requests", "retained and excluded",
    "earlier HTTP/native cohorts are excluded", "no gains are added or pooled", "Defaults are unchanged"]);
  phrases(value.v13.detail, ["All eight CPU emission phases", "52 explicit bytes", "hidden start 56", "312 total kernarg",
    "only the typed handoff digest was retained", "detailed typed-node review and native parity remain unadmitted",
    "actually built at 216822", "not rebuilt there", "not used by the HTTP cohorts", "no gain or kernel-refinement proof"]);
  phrases(value.v14.detail, ["13 source-fresh host methods", "strict Clippy with actual exit zero", "original ten synthetic control tests",
    "all nine phases, including 11 synthetic methods", "116 explicit bytes", "hidden start 120", "376 total kernarg",
    "outside recurring token backedges", "raw hoist flag remains pending", "typed handoff is digest-only",
    "eight finite native cases at TP1/context32", "uniform rows 1/17/31 and selector rows 32", "48 complete input/output buffers",
    "inactive tails and 96 guards", "normal unforced close", "all eight GPUs idle before and after",
    "independent CPU replay passed the unchanged validator and all 421 raw protocol rows",
    "not a same-compiler ablation or full-model/context8192 parity", "detailed typed review is still unadmitted",
    "No performance gain is claimed", "neither v14 nor v13 changes the measured C1 live route"]);
  phrases(value.v15.detail, ["20 source-fresh CPU tests", "six contract and fourteen host-model tests",
    "strict Clippy exit zero", "c534e35 using fe2o3 61e014a", "five own Cargo records are fresh:false",
    "FP32 association differs from the old serial fold", "No image emission, device ABI/resource validation",
    "native/model parity or performance gain is admitted", "not a proof of GPU arithmetic"]);
  phrases(value.development.detail, ["opt-in V14 full-Qwen canary is integrated in private source",
    "combined controller host validation is pending", "484 library tests with one unchanged ignore, 31 doctests",
    "strict release Clippy exit zero", "All twelve host phases pass", "current worker record is fresh:false",
    "Worker native qualification remains pending",
    "32 synthetic methods", "eight focused wrapper methods", "rejection of superseded 61e provenance",
    "These are not model runs", "The controller artifact and full-Qwen parity remain pending",
    "compiler 8efd provenance, not ae267", "No default, HTTP measurement, competitive or M1 claim changes",
    "all historical cohorts remain unchanged"]);
}
export function testLiveC1CheckpointRejections(input) {
  const mutations = [
    (x) => { x.context = 256; }, (x) => { x.controllerSha256 = "0".repeat(64); },
    (x) => { x.matchedCohorts.reverse(); }, (x) => { x.matchedCohorts[1].tpotMeanMs = 162.99861548031497; },
    (x) => { x.matchedCohorts[1].outputTokensPerSecond = 128 / 22.425321713166668; },
    (x) => { x.matchedCohorts[1].windowSeconds = x.matchedCohorts[0].windowSeconds; },
    (x) => { x.measuredRequests = 40; }, (x) => { x.freshStartsPerMode = 2; },
    (x) => { x.qualification[1].normalUnforcedClose = false; },
    (x) => { x.excludedAttempt.requests = 1; }, (x) => { x.excludedAttempt.timingAdmitted = true; },
    (x) => { x.runtimeProfiling = true; }, (x) => { x.hostTiming = true; },
    (x) => { x.servingQualified = true; }, (x) => { x.stablePerformanceGainClaimed = true; },
    (x) => { x.competitiveRanking = true; }, (x) => { x.defaultPromotion = true; },
    (x) => { x.gpuDurationMeasurement = true; }, (x) => { x.historicalCohortsMixed = true; },
    (x) => { x.m1OpenGates = 32; }, (x) => { x.v13.detailedTypedReviewAdmitted = true; },
    (x) => { x.v13.actualWorkerBuild = x.v13.compilerSource; },
    (x) => { x.v13.kernargBytes = 308; }, (x) => { x.v13.nativeAdmitted = true; },
    (x) => { x.v14.detailedTypedReviewAdmitted = true; }, (x) => { x.v14.syntheticControlTestsPassed = 13; },
    (x) => { x.v14.emission.syntheticTestsPassed = 10; }, (x) => { x.v14.emission.rawHoistFlag = "passed"; },
    (x) => { x.v14.emission.sameCompilerAblation = true; },
    (x) => { x.v14.nativeAdmitted = false; }, (x) => { x.v14.native.cases = 7; },
    (x) => { x.v14.native.guardRegions = 48; }, (x) => { x.v14.native.protocolRows = 420; },
    (x) => { x.v14.native.cpuReplayPassed = false; }, (x) => { x.v14.native.sameCompilerAblation = true; },
    (x) => { x.v14.native.modelParityQualified = true; }, (x) => { x.v14.native.performanceQualified = true; },
    (x) => { x.v14.native.archiveSha256 = "0".repeat(64); },
    (x) => { x.interpretation = "Stable and competitive gain"; }, (x) => { x.extra = true; },
    (x) => { x.v15.hostTestsPassed = 21; }, (x) => { x.v15.strictClippyExit = 101; },
    (x) => { x.v15.dependencySource = x.development.fe2o3Source; }, (x) => { x.v15.ownArtifactsFresh = false; },
    (x) => { x.v15.emissionAdmitted = true; }, (x) => { x.v15.nativeAdmitted = true; },
    (x) => { x.v15.modelParityQualified = true; }, (x) => { x.v15.performanceGainClaimed = true; },
    (x) => { x.development.hostAdmitted = true; }, (x) => { x.development.workerHostAdmitted = false; },
    (x) => { x.development.fe2o3Source = x.v15.dependencySource; },
    (x) => { x.development.modelParityQualified = true; },
    (x) => { x.development.workerTestsPassed = 485; }, (x) => { x.development.workerTestsIgnored = 0; },
    (x) => { x.development.workerNativeAdmitted = true; }, (x) => { x.development.workerSha256 = "0".repeat(64); },
  ];
  for (const mutate of mutations) {
    const changed = clone(input);
    mutate(changed);
    assert.throws(() => validateLiveC1Checkpoint(changed), assert.AssertionError);
  }
}

export async function validateLiveC1CheckpointEvidence(root, project) {
  validateLiveC1Checkpoint(project.liveC1Checkpoint);
  async function pinned(name, expected) {
    const bytes = await readFile(join(root, name));
    assert(bytes.length > 0 && bytes.length <= 2 * 1024 * 1024, name);
    assert.equal(sha(bytes), expected, name);
    return bytes;
  }
  const baseline = projectFrom(await pinned("live-c1-baseline-project.js",
    "f7b007301cb69f43c719e3bb5388eead8d198cbbce27747f30b87d63e13cfb57"));
  const historical = clone(project);
  delete historical.liveC1Checkpoint;
  assert.deepEqual(historical, baseline, "all historical public objects remain unchanged");
  assert.equal(sha(await readFile(join(siteRoot, "data/performance.js"))),
    "05ad1f50575547c0b0c8244b2912e1d7518258772527f7ff236fdb8d2e42102f");
  const replayByMode = {};
  const identityByMode = {};
  for (const [index, row] of cohorts.entries()) {
    const q = qualifications[index];
    const qualification = JSON.parse(await pinned(`live-c1-${row.layerProjection}-qualification.json`, q.receiptSha256));
    const receipt = JSON.parse(await pinned(`live-c1-${row.layerProjection}-receipt.json`, row.receiptSha256));
    const replay = JSON.parse(await pinned(`live-c1-${row.layerProjection}-replay.json`, row.replaySha256));
    replayByMode[row.layerProjection] = replay;
    for (const [record, purpose, count, plan] of [[qualification, "qualification", 2, q.planSha256],
      [receipt, "matched", 42, row.planSha256]]) {
      assert.equal(record.schema, "FerricLayerC1WaveLiveHttpReceiptV1");
      assert.equal(record.purpose, purpose);
      assert.equal(record.layer_projection, row.layerProjection);
      assert.equal(record.submission, "ordered");
      assert.equal(record.passed, true);
      assert.equal(record.cleanup_completed, true);
      assert.equal(record.all_eight_idle_before_after, true);
      assert.equal(record.request_count, count);
      assert.equal(record.plan_sha256, plan);
      assert.deepEqual(record.errors, []);
      for (const flag of ["serving_qualified", "framework_win_claim", "gpu_duration_claim"])
        assert.equal(record[flag], false);
      assert.equal(record.timing_admitted, purpose === "matched");
      assert.equal(record.numerical_diagnostics.length, 2);
      for (const diagnostic of record.numerical_diagnostics)
        exact(diagnostic, { admitted: true, classification: "exact_match", first_token_mismatch: null });
      assert.equal(Object.keys(record.raw_files).length, purpose === "matched" ? 20 : 18);
    }
    const identity = qualification.qualification_identity;
    assert.deepEqual(receipt.qualification_identity, identity);
    assert.equal(receipt.client_exit_status, 0);
    assert.equal(receipt.qualification_admitted, false);
    identityByMode[row.layerProjection] = clone(identity);
    assert.equal(identity.ferric.controller_source, fixed.source);
    assert.equal(identity.ferric.controller_tree, fixed.tree);
    assert.equal(identity.ferric.controller.sha256, fixed.controllerSha256);
    assert.equal(identity.ferric.worker.sha256, fixed.workerSha256);
    assert.equal(identity.driver_sources["competitive_benchmark.py"], fixed.clientSha256);
    assert.equal(identity.driver_sources["serve_ferric.py"], fixed.serveSha256);
    assert.equal(identity.ferric.expected_setup.context_tokens, 8192);
    assert.equal(identity.ferric.expected_setup.performance_profile.runtime_profiling, false);
    assert.equal(identity.layer_projection, row.layerProjection);
    assert.equal(identity.ferric.expected_setup.layer_projection, row.layerProjection);
    assert.equal(identity.ferric.expected_setup.performance_profile.layer_projection, row.layerProjection);
    assert.equal(identity.ferric.argv[identity.ferric.argv.indexOf("--layer-projection") + 1], row.layerProjection);
    assert(!identity.ferric.argv.includes("--host-timing") && !identity.ferric.argv.includes("--runtime-profile"));
    for (const [name, setting] of Object.entries({ context: 8192, concurrency: 1, samples: 30, warmups: 10,
      prompt_tokens: 128, completion_tokens: 128, prefix_cache: false, speculation: false }))
      assert.equal(identity.settings[name], setting);
    assert.equal(qualification.qualification_admitted, true);
    assert.equal(replay.layer_projection, row.layerProjection);
    assert.equal(replay.receipt_sha256, row.receiptSha256);
    assert.equal(replay.plan_sha256, row.planSha256);
    assert.equal(replay.scope, "same-binary layer C1 finite closed-loop HTTP cohort; excludes two diagnostics and ten warmups");
    for (const [key, count] of Object.entries({ measured_requests: 30, measured_output_tokens: 3840,
      diagnostic_requests_checked: 2, timed_final_token_ids_checked: 40, timed_http_request_identities_checked: 40,
      raw_files_checked: 20 })) assert.equal(replay[key], count);
    for (const flag of ["serving_qualified", "framework_win_claim", "gpu_duration_claim"])
      assert.equal(replay[flag], false);
    assert.equal(replay.submission, "ordered");
    exact(replay.excluded_zero_request_c1_r1, {
      externally_authenticated_archive_sha256: excluded.archiveSha256, receipt_sha256: excluded.receiptSha256,
      reason: "pre-worker socket-bind failure; not a cohort", request_count: 0, timing_admitted: false,
    });
    for (const other of cohorts) {
      assert.equal(replay.both_completed_plan_sha256[other.layerProjection], other.planSha256);
      assert.equal(replay.both_completed_receipt_sha256[other.layerProjection], other.receiptSha256);
      assert.equal(replay.completed_case_names[other.layerProjection], other.case);
    }
    const metrics = replay.metrics;
    assert.equal(metrics.requests, 30);
    assert.equal(metrics.successful_requests, 30);
    assert.equal(metrics.failed_requests, 0);
    assert.equal(metrics.all_requests_succeeded, true);
    for (const [prefix, field] of [["ttft", "ttft_ms"], ["tpot", "tpot_ms"]])
      for (const [suffix, key] of [["Mean", "mean"], ["P50", "p50"], ["P90", "p90"], ["P99", "p99"]])
        close(row[`${prefix}${suffix}Ms`], metrics[field][key]);
    close(row.outputTokensPerSecond, metrics.output_tokens_per_second);
    close(row.windowSeconds, metrics.window_seconds);
    assert(Number.isSafeInteger(replay.cohort_start_ns) && Number.isSafeInteger(replay.cohort_end_ns));
    assert(replay.cohort_end_ns > replay.cohort_start_ns);
    close(row.windowSeconds, (replay.cohort_end_ns - replay.cohort_start_ns) / 1e9);
    close(row.outputTokensPerSecond, 3840 / row.windowSeconds);
  }
  const baseIdentity = identityByMode.mfma;
  const candidateIdentity = identityByMode["c1-wave"];
  for (const identity of [baseIdentity, candidateIdentity]) {
    delete identity.layer_projection;
    delete identity.ferric.expected_setup.layer_projection;
    delete identity.ferric.expected_setup.performance_profile.layer_projection;
    const selector = identity.ferric.argv.indexOf("--layer-projection");
    assert(selector > 0);
    identity.ferric.argv[selector + 1] = "layer-mode-only";
  }
  assert.deepEqual(baseIdentity, candidateIdentity, "same binary/source/images/settings, only layer selector differs");
  assert(replayByMode.mfma.cohort_end_ns < replayByMode["c1-wave"].cohort_start_ns);
  close((cohorts[1].tpotMeanMs / cohorts[0].tpotMeanMs - 1) * 100, -22.736586688148208);
  close((cohorts[1].ttftMeanMs / cohorts[0].ttftMeanMs - 1) * 100, -1.5947864752855367);
  close((cohorts[1].outputTokensPerSecond / cohorts[0].outputTokensPerSecond - 1) * 100, 25.9198188182459);
  assert.equal(baseline.matched128.metrics[1].engine, "vLLM 0.28.0");
  assert.equal((baseline.matched128.metrics[1].outputTokensPerSecond / cohorts[1].outputTokensPerSecond).toFixed(3), "38.643");
  const matchedNote = (await pinned("live-c1-matched-note.md", fixed.matchedNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(matchedNote, ["42 exact-output requests", "All forty timed HTTP/final associations pass",
    "close normally without cleanup signals", "eight-device rosters are idle", "Both actual replay statuses are zero",
    ...cohorts.flatMap((row) => [row.receiptSha256, row.archiveSha256, row.replaySha256])]);
  await pinned("live-c1-qualification-review.md", fixed.qualificationReviewSha256);
  const failed = JSON.parse(await pinned("live-c1-excluded-receipt.json", excluded.receiptSha256));
  assert.equal(failed.passed, false);
  assert.equal(failed.request_count, 0);
  assert.equal(failed.timing_admitted, false);
  const result = JSON.parse(await pinned("v13-emission-result.json", v13.resultSha256));
  assert.equal(result.schema, "FerricShardedArgmaxV13EmissionAbiResourceReviewV1");
  assert.equal(result.abi_resources_passed, true);
  assert.equal(result.manual_typed_progress_effect_and_isa_review_pending, true);
  for (const key of ["gpu_used", "numerical_parity", "performance_claim"]) assert.equal(result[key], false);
  assert.equal(result.identities.image_sha256, v13.imageSha256);
  assert.equal(result.identities.source, v13.source);
  assert.equal(result.identities.worker_actual_build, v13.actualWorkerBuild);
  const roots = Object.entries(result.image.roots);
  assert.deepEqual(roots.map(([name]) => name).sort(), ["finalize", "produce"].map((part) =>
    `ferric_qwen3_tp_batch32_sharded_argmax_${part}_f32_v13`).sort());
  for (const [, rootRecord] of roots) {
    assert.equal(rootRecord.explicit_bytes, 52);
    const meta = rootRecord.metadata;
    assert.equal(meta[".kernarg_segment_size"], 312);
    assert.equal(meta[".args"][7][".offset"], 56);
    assert.deepEqual(meta[".reqd_workgroup_size"], [64, 1, 1]);
    for (const field of [".sgpr_spill_count", ".vgpr_spill_count", ".private_segment_fixed_size", ".group_segment_fixed_size"])
      assert.equal(meta[field], 0);
    assert.equal(meta[".uses_dynamic_stack"], false);
  }
  const observation = JSON.parse(await pinned("v13-observation.json", v13.observationSha256));
  exact(observation.grants, { publication: false, load: false, launch: false });
  assert.equal(observation.execution.exact_output_replay, true);
  assert.equal(observation.options.verify_each, true);
  await pinned("v13-emission-note.md", v13.noteSha256);
  await pinned("v13-static-review.md", v13.staticReviewSha256);
  const host = (await pinned("v14-host-note.md", v14.hostNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(host, [v14.source, v14.tree, "Exact 13 method names passed", "actual 0 with `-D warnings`", v14.hostArchiveSha256]);
  const control = (await pinned("v14-control-note.md", v14.controlNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(control, ["exactly ten synthetic checker methods", "not a compiler emission", v14.controlArchiveSha256]);
  const hoist = JSON.parse(await pinned("v14-emission-result.json", v14Emission.resultSha256));
  assert.equal(hoist.schema, "FerricQueryHoistV14EmissionAbiResourceReviewV1");
  assert.equal(hoist.abi_resources_passed, true);
  assert.equal(hoist.hoist_survival, "pending");
  assert.equal(hoist.manual_typed_progress_effect_and_isa_review_pending, true);
  for (const key of ["gpu_used", "numerical_parity", "performance_claim"]) assert.equal(hoist[key], false);
  assert.equal(hoist.identities.image_sha256, v14Emission.imageSha256);
  assert.equal(hoist.identities.source, v14.source);
  assert.equal(hoist.identities.source_tree, v14.tree);
  assert.equal(hoist.identities.compiler, v13.compilerSource);
  assert.equal(hoist.identities.worker_actual_build, v13.actualWorkerBuild);
  assert.deepEqual(Object.keys(hoist.image.roots), ["ferric_qwen3_tp_batch32_wave_paged_gqa_query_hoist_bf16_v14"]);
  const hoistRoot = Object.values(hoist.image.roots)[0];
  assert.equal(hoistRoot.explicit_bytes, 116);
  const hoistMeta = hoistRoot.metadata;
  assert.equal(hoistMeta[".kernarg_segment_size"], 376);
  assert.equal(hoistMeta[".kernarg_segment_align"], 8);
  assert.equal(hoistMeta[".args"].find((arg) => arg[".value_kind"].startsWith("hidden_"))[".offset"], 120);
  assert.deepEqual(hoistMeta[".reqd_workgroup_size"], [64, 1, 1]);
  assert.equal(hoistMeta[".sgpr_count"], 95);
  assert.equal(hoistMeta[".vgpr_count"], 37);
  for (const field of [".sgpr_spill_count", ".vgpr_spill_count", ".private_segment_fixed_size", ".group_segment_fixed_size", ".agpr_count"])
    assert.equal(hoistMeta[field], 0);
  assert.equal(hoistMeta[".uses_dynamic_stack"], false);
  const hoistObservation = JSON.parse(await pinned("v14-observation.json", v14Emission.observationSha256));
  exact(hoistObservation.grants, { publication: false, load: false, launch: false });
  assert.deepEqual(hoistObservation.compiler_handoff, hoist.identities.compiler_handoff);
  const hoistNote = (await pinned("v14-emission-note.md", v14Emission.noteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(hoistNote, ["all nine frozen phases", "11 synthetic methods", v14Emission.archiveSha256,
    "not native parity or a same-compiler performance ablation", "typed handoff is digest-only"]);
  const hoistReview = (await pinned("v14-static-review.md", v14Emission.staticReviewSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(hoistReview, ["Static query-load hoisting survives", v14Emission.imageSha256,
    "no actual handoff", "Raw `result.json` is not rewritten", "Native finite attention fixtures and numerical parity remain pending"]);
  const native = JSON.parse(await pinned("v14-native-report.json", v14Native.reportSha256));
  assert.equal(native.schema, "FerricTp1AttentionQueryHoistProbeV1");
  assert.equal(native.authority, "none");
  for (const key of ["checks_pass", "clean_teardown", "active_inputs_finite", "runtime_operational"])
    assert.equal(native[key], true);
  for (const key of ["benchmark", "performance_qualified", "model_inference", "model_parity_qualified", "same_compiler_ablation"])
    assert.equal(native[key], false);
  assert.equal(native.images[0].compiler, "3e74a9324a5acd7107e96a4a9b5319d3dd5ecde8");
  assert.equal(native.images[1].compiler, v13.compilerSource);
  assert.equal(native.images[1].sha256, v14Emission.imageSha256);
  assert.equal(native.results.length, 8);
  const specs = [["uniform", 1], ["uniform", 17], ["uniform", 31], ["selector", 32]];
  const bufferNames = ["query", "keys", "values", "positions", "table", "output"];
  for (const [index, result] of native.results.entries()) {
    const [family, rows] = specs[Math.floor(index / 2)];
    const role = index % 2 ? "candidate" : "resident";
    assert.equal(result.name, `${role}_${family}_rows${rows}_tp1_causal_pages`);
    assert.equal(result.role, role);
    assert.equal(result.symbol, native.images[index % 2].root);
    assert.deepEqual(result.checks.map((check) => check.name), bufferNames);
    for (const check of result.checks) {
      assert.match(check.sha256, /^[0-9a-f]{64}$/);
      assert.equal(check.guard_bytes_each_side, 64);
      assert.equal(check.guards_unchanged, true);
    }
    if (index % 2) assert.deepEqual(result.checks, native.results[index - 1].checks);
  }
  const protocolBytes = await pinned("v14-native-protocol.jsonl", v14Native.protocolSha256);
  assert.equal(protocolBytes.at(-1), 10);
  const protocol = protocolBytes.toString("utf8").trimEnd().split("\n").map((line) => JSON.parse(line));
  assert.equal(protocol.length, 421);
  assert.equal(protocol[0].response.op, "ready");
  for (const [op, count] of [["load_kernel", 8], ["dispatch", 8], ["allocate", 48], ["write", 48], ["read", 48], ["free", 48]])
    assert.equal(protocol.filter((row) => row.command?.op === op).length, count);
  assert.equal(protocol.at(-2).command.op, "close");
  assert.equal(protocol.at(-1).response.op, "closed");
  // Large device IDs stay bound by raw-byte pins and the accepted integer-safe CPU replay.
  exact(JSON.parse(await pinned("v14-native-wrapper.json", v14Native.wrapperSha256)), {
    schema: "FerricTp1AttentionQueryHoistWrapperV1", authority: "none", passed: true, errors: [],
    all_eight_idle_before: true, all_eight_idle_after: true, performance_qualified: false, same_compiler_ablation: false,
  });
  exact(JSON.parse(await pinned("v14-native-cleanup.json", v14Native.cleanupSha256)), {
    absent: true, cleanup_error: null, controller_returncode: 0, forced: false, reason: null,
  });
  assert.equal((await pinned("v14-cpu-replay.log", v14Native.cpuReplayLogSha256)).toString("utf8"),
    "PASS: frozen native validator and 421 protocol rows; 48 buffers/96 guards; CPU only\n");
  const cpuClosure = (await pinned("v14-cpu-closure.tsv", v14Native.cpuReplayClosureSha256)).toString("utf8");
  phrases(cpuClosure, ["owned_groups_absent\ttrue", "forced_cleanup\t0", "TMP_empty\ttrue", "raw-replay\t0", "exit_code\t0"]);
  const rmsNote = (await pinned("v15-host-note.md", v15.hostNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(rmsNote, [v15.source, v15.tree, v15.hostArchiveSha256, "Twenty host tests passed",
    "Strict all-target release Clippy returned 0", "Five actual own Cargo", "all fresh:false",
    "No local Cargo/Python, emitted image, GPU dispatch or performance run occurred"]);
  const rmsTests = (await pinned("v15-tests.log", v15.testsLogSha256)).toString("utf8");
  const rmsRows = [...rmsTests.matchAll(/^test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;/gm)];
  assert.deepEqual(rmsRows.map((row) => Number(row[1])), [0, 6, 14]);
  await pinned("v15-clippy.log", v15.clippyLogSha256);
  const rmsArtifacts = JSON.parse(await pinned("v15-artifacts.json", v15.artifactReceiptSha256));
  assert.equal(rmsArtifacts.passed, 20);
  assert.equal(rmsArtifacts.ignored, 0);
  assert.equal(rmsArtifacts.rows, 3);
  assert.equal(rmsArtifacts.records.length, 5);
  assert.equal(Object.keys(rmsArtifacts.files).length, 6);
  assert(rmsArtifacts.records.every((row) => row.fresh === false));
  await pinned("v15-Cargo.lock", v15.lockSha256);
  const checkerNote = (await pinned("v14-canary-cpu32.md", "f0392b3daca5a4157c264319ddad8d8789e905fb980ccbb2d198167d10c7ac29"))
    .toString("utf8").replace(/\s+/g, " ");
  phrases(checkerNote, [development.checkerArchiveSha256, "All 32 methods reported", "No retries",
    "does not authenticate any real host receipt or admit native execution"]);
  const wrapperNote = (await pinned("v14-current-wrapper-cpu8.md", "6fee47c94fed367085611d31a311ed91abedff7aaf5b09676628c354ffbb2ba7"))
    .toString("utf8").replace(/\s+/g, " ");
  phrases(wrapperNote, [development.wrapperArchiveSha256, development.fe2o3Source, development.fe2o3Tree,
    "passed eight wrapper methods", "No actual bindings, model/GPU reads or controller/worker execution occurred"]);
  const workerNote = (await pinned("current-worker-note.md", development.workerNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(workerNote, [development.fe2o3Source, development.fe2o3Tree, development.workerSha256,
    development.workerArchiveSha256, "all twelve actual commands", "Library tests: 484 passed, one unchanged ignored",
    "All 31 doctests passed", "Strict release Clippy and explicit worker build returned 0",
    "No live-validation, ignored test, worker or GPU execution occurred", "not native parity or a performance result"]);
  const workerTests = (await pinned("current-worker-tests.log", development.workerTestsLogSha256)).toString("utf8");
  assert.equal([...workerTests.matchAll(/^test result: ok\. 484 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out;/gm)].length, 1);
  phrases(workerTests, ["test queue::live::tests::auxiliary_destroy_requires_retired_dispatch_ledger_before_taking_custody ... ok"]);
  const workerDocs = (await pinned("current-worker-docs.log", development.workerDocsLogSha256)).toString("utf8");
  assert.equal([...workerDocs.matchAll(/^test result: ok\. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;/gm)].length, 1);
  await pinned("current-worker-clippy.log", development.workerClippyLogSha256);
  await pinned("current-worker-build.log", development.workerBuildLogSha256);
  const workerReceipt = JSON.parse(await pinned("current-worker-artifacts.json", development.workerReceiptSha256));
  assert.equal(workerReceipt.source, development.fe2o3Source);
  assert.equal(workerReceipt.tree, development.fe2o3Tree);
  const workerRecords = workerReceipt.records.filter((row) => row.target.name === "fe2o3-gfx950-engineering-worker");
  assert.equal(workerRecords.length, 1);
  assert.equal(workerRecords[0].fresh, false);
  assert.equal(workerRecords[0].profile.test, false);
  assert.deepEqual(workerRecords[0].features, ["default", "engineering-gfx950"]);
  assert.deepEqual(workerReceipt.files["release/fe2o3-gfx950-engineering-worker"],
    { bytes: development.workerBytes, sha256: development.workerSha256 });
  console.log("PASS: admitted HTTP pair and historical objects unchanged; V13/V14 finite scopes, V15 CPU evidence and current-worker host-only boundary preserved.");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  assert.equal(process.argv.length, 3);
  await validateLiveC1CheckpointEvidence(process.argv[2], projectFrom(await readFile(join(siteRoot, "data/project.js"))));
}
