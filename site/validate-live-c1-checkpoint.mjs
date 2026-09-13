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
  ownArtifactsFresh: true, emissionAdmitted: true, nativeAdmitted: false,
  modelParityQualified: false, performanceGainClaimed: false,
  emissionAttempt: {
    state: "offline-dependency-stop-before-compilation", metadataExit: 101, gateExit: 1, emissionInvoked: false,
    noteSha256: "910a724ac83975a8acb9ff36a4aaa7168d369fa09743e1bd9d4871dba059b8be",
    diagnosticSha256: "6600808929240a65a9778baeb8d98a4abb45715654d83ad9e00f7edfa2d3794b",
    archiveSha256: "b52ecb0567d3395502372450cdf3fa394e0caf54a4ad71a4ef6fe95f3888c47a",
  },
};
const v15Emission = {
  source: "39be94db2f057f2d51c188d75c0a7dedd7faeba8", tree: "f298d3e0a070df1f6b903a8bbfbdc61da12835ff",
  compilerSource: "ae26717922b1fb7ad62fdd5ad70814d83eb01177", compilerTree: "2ef28f830df9832d53a62525e20aa33815c164fa",
  llvmWorkerActualBuild: "21682228486f7186cc3c37ddf165fffc438d8b6a",
  noteSha256: "3bb64917df571bfecc2b18cad23c6cab6c406e370c48cf6ccdefc8fcd1f400f0",
  archiveSha256: "c4f3be27d412250fe44fe28ff12a26e53e4e486230cab45a7d429f2be059c7a9",
  testsLogSha256: "ff4abd592cb78d271a8c2554e46a01e0df50ba76b7d6de8b384b19012296e866",
  clippyLogSha256: "d7aca83d92bc3d50cbe9e975f2b701795e069c61432cf3cf0c0e0d7730cc113a",
  artifactReceiptSha256: "1d42ee489b9ee9d558e4863e03150d8f6cadddc257e076049e4a063dbbedf69a",
  inspectionSha256: "0120efd7ac515a44648338574fde59df4d032ea5c6a1f50864a74ae219cae3af",
  staticReviewSha256: "c08d00f099cf1248232bacc41a41239cfc9aeff1f92d804f5ae6ec04672af250",
  imageSha256: "a9da1d7944f2072388e8a7727987692362ab0183dfec7a87aa0883a3af2d784c", imageBytes: 11240,
  hostTestsPassed: 23, contractTestsPassed: 7, hostModelTestsPassed: 16, strictClippyExit: 0, phasesPassed: 12,
  explicitArguments: 14, explicitBytes: 96, hiddenStart: 96, kernargBytes: 352, kernargAlign: 8, wavefrontSize: 64,
  sgprs: 70, vgprs: 22, sgprSpills: 0, vgprSpills: 0, privateBytes: 0, ldsBytes: 0, agprs: 0,
  dynamicStack: false, abiResourcesPassed: true, staticReviewBlocker: false, detailedTypedReviewAdmitted: false,
  nativeAdmitted: false, modelParityQualified: false, performanceGainClaimed: false,
};
const v15Current = {
  source: "f393390350b15a0a050b1025eda6476d738ac98a", tree: "9a22571657d6479d0d6a9898655e4561ab5fc4b4",
  compilerSource: "c9c7036cf98034b638886f54a745701e70ac3571", compilerTree: "27142fd1003494aac211bc8250584228ff09525c",
  llvmWorkerActualBuild: "21682228486f7186cc3c37ddf165fffc438d8b6a",
  emissionNoteSha256: "075f008e9128872bc2cbe9c0514c65dc0ab9c7b4ce562675fc4fe5358beccaaf",
  emissionArchiveSha256: "1e21a6ff97f2af54efbdf138abefe08cc12e4a610281b6aea37ffd7785119ec6",
  testsLogSha256: "546b917e8ebf8746fd9c650225fb80db044403eda9998803ba726a5d30cb04d5",
  verifierLogSha256: "4c2e69d79bf9790a8e18efa6cd79d732d0a6cb208d014b85b87a29f8b06224a5",
  clippyLogSha256: "d7aca83d92bc3d50cbe9e975f2b701795e069c61432cf3cf0c0e0d7730cc113a",
  artifactReceiptSha256: "39885d09d2d8a5e6af286b1aee3aafc35fd7da3ba0c3907ee2379f423366948c",
  verifierArtifactSha256: "a8ec6756b1947c7c3a249605c95000d6825e173f6bc96b3ce019f4bcb16c8e03",
  inspectionSha256: "46dc816dc09a1e90deaa839bf6cc2b1835edc5cb28617377a481140961276f66",
  imageSha256: "68607fc1d12eb6151dd55e7b53dcb88b00d484cfa0c087c002666b224093b45c", imageBytes: 11240,
  hostTestsPassed: 23, verifierTestsPassed: 31, strictClippyExit: 0, emissionPhasesPassed: 15,
  nativeSource: "d293b7c2f03f911c5b140d91ff746a1d4b3ab6f3", nativeTree: "72b94d328052cbbe49d56408f16f87cbfd328974",
  cpuTestsPassed: 30, cpuNoteSha256: "20f7321a21e9e8694ab94d280099bc2d59482682300b4c4c595cc4ea9030f9fe",
  cpuTestsLogSha256: "9c5ab0642e19067b99b1f21aba28501e2ca7d1f9b689a58da24248e7fe541ba5",
  cpuArchiveSha256: "ba94c2aa668c06bd12c7199427351742b798c7a23e5231b04a8d665e5c44d60a",
  nativeNoteSha256: "e6b2919cb238279c372264a110ecc0bcc0a1553c3343227faafdde1ddac1bf9c",
  nativeArchiveSha256: "c96ea879cadd7bcbc8acedcf7730bffcea2df6737b9b5a517361bc11022f5e51",
  nativeReportSha256: "2c9073ff344eb1783e2abd14d8c02af598b60a5e1760fe8587f2c8cc0c497cca",
  nativeWrapperSha256: "e0d61ae442b864dfe2d42cfdd7f30a23478c5cfcc094776e51e697cd2cc12ab1",
  protocolSha256: "46654a637a64bc497772ffdd682a1b80003ebe13af94ebb99b7109e72c60f095",
  replayNoteSha256: "816d5c29fb04adeaa7c65a4469ebb7af7b7993360331b5f99b0efdabc31a4941",
  replayReportSha256: "23fd2937e96b7873d83aac8469e6eafdfa7925276095a9af6825091e6a45f4b8",
  replayArchiveSha256: "eddabe7e909324fb2f1b216aa26cdef5295437e6e7a2c331ad93d616a9a6b206",
  workerSource: "ae26717922b1fb7ad62fdd5ad70814d83eb01177",
  workerSha256: "526cc6bf8902128767a5318c90d1c0206a9e435957612c398eeb28ff2ae71032",
  scope: "eight-analytical-finite-kernel-cases", cases: 8, rows: [1, 16, 17, 32], width: 4096,
  bufferRecords: 40, guards: 80, protocolRows: 357, finiteNativeChecksPassed: true,
  normalUnforcedClose: true, allEightIdleBeforeAfter: true, cpuReplayPassed: true,
  invalidCasesQualified: false, generalFp32Proof: false, modelParityQualified: false,
  performanceGainClaimed: false, defaultPromotion: false, sameCompilerAblation: false,
};
const v15Model = {
  source: "87fdcead7418e8f64db29fdb2703f5a4c8c9b742", tree: "561c2a1da66691d6da9c9a213ecfcd19621dbc3a",
  controllerSha256: "21168d30802925ad8aad61ba9f87a5d2f406d2413e512234b9db3690e8fc18f2", controllerBytes: 9445768,
  hostNoteSha256: "2170b8e09e15c9652aea817d63e03839cac676b452db4e620fde7a3483baedc2",
  hostReceiptSha256: "5f32bf1b0863fedf4f751f6b97d76523e26330e0e71158662fa3f6373986394b",
  hostArchiveSha256: "ca6dd01968a29ffc33cecece890ab3169267b23404eafb4e4d8840db6bfb4533",
  hostPhasesPassed: 20, strictClippyExit: 0, hostFormatExit: 0,
  hostTests: {
    focused_library: { passed: 9, ignored: 1 }, source_policy: { passed: 35, ignored: 0 },
    batched_regressions: { passed: 101, ignored: 0 }, v15_canary: { passed: 67, ignored: 4 },
    legacy_v14_canary: { passed: 67, ignored: 4 }, actual_v15_image_admission: { passed: 1, ignored: 0 },
  },
  scope: "four-fixed-prompt-correctness-cases", correctnessPassed: true, cases: 4,
  promptTokens: 128, contextTokens: 256, pages: 16, rows: 32, prefillChunkTokens: 16, tensorParallel: 1,
  layerProjection: "c1-wave", attention: "wave", submission: "ordered", argmaxMode: "wave-v11",
  bindingsSha256: "48944261c326523886d33119178fd48c4511513c35127b0704f7023d19a50959",
  noteSha256: "011e3bbd17b3ee711f4ad62507086d7f4ed7c3fa40dc04c1547b1500c49f59de",
  archiveSha256: "1e2ff5f1ced1ea4b7104ff79d1df7a45706219109ed28817ff9a366c6ccf0662",
  cleanupSha256: "adb2976ce274a8f114429d295b7d633629ab1696c84dc650b88dc44219c662c0",
  launcherCorrectionSha256: "610c727f827d9683d384c9f79382a3a1d1d0bf121f66a466acd87aa6c87dc65d",
  replayNoteSha256: "9c5323082a5bdd5f88609d3c9595c9445f248eed33c0075cdaffb494c2663006",
  replayReportSha256: "dd2f37f1289e3748bfec7151786cc3b4d11ba3ef9f86475440f00e557378d3d2",
  replayArchiveSha256: "5ad101563ddea47bf9f9e98296c064943a0e17487148a8df35649b16195f6df9",
  rowsChecked: [
    { name: "baseline8", rmsnormMode: "baseline", outputs: 8, packets: 9219, batches: 15, committedInputs: 135,
      wrapperSha256: "a41f7d79651b4db3c8ea4deed0c14027b859a58d4e1d6880ba565dfe2d0c6098" },
    { name: "candidate8", rmsnormMode: "wave-v15", outputs: 8, packets: 9219, batches: 15, committedInputs: 135,
      wrapperSha256: "9afb674c6733db8a26a0170ef64962476bd9e3a5ebf5c7a7220ce13a8cd4af0e" },
    { name: "baseline128", rmsnormMode: "baseline", outputs: 128, packets: 83139, batches: 135, committedInputs: 255,
      wrapperSha256: "3de932b405b53db61e54eb83c968839a8432f5d046951cc870404aadbb91d1c4" },
    { name: "candidate128", rmsnormMode: "wave-v15", outputs: 128, packets: 83139, batches: 135, committedInputs: 255,
      wrapperSha256: "7df69ac5e16f07c7bbca5e39120a368601dfa4779c8742ddb292e3648a49e29d" },
  ],
  normalUnforcedClose: true, allEightIdleBeforeAfter: true, cpuReplayPassed: true,
  sameControllerAndPreloads: true, correctnessTimingsExcluded: true, numericalRetry: false,
  generalizedParityQualified: false, context8192Qualified: false, httpMeasurement: false,
  performanceQualified: false, defaultPromotion: false, historicalCohortsMixed: false,
};
const v15Abba = {
  state: "complete-independent-replay", contextTokens: 256, promptTokens: 128, outputTokens: 128,
  repetitionsPerMode: 2, acquisitionOrder: ["baseline-a1", "wave-v15-b1", "wave-v15-b2", "baseline-a2"],
  experimentPlanSha256: "d4cd6df496057bf920f0451e1ad4b04f3595ca7dcf8e4dfda9f4033c207673f9",
  reducerSha256: "88858aa8d43b7c9393dfd43e739ba70a53fb6f197a48a2927efbc9d35673c21d",
  summarySha256: "5ee062de5682d8b1474e9dc9ed0362f6dd5cb0ed9de5419f4007ac24dc17807a",
  manifestSha256: "337398866cf355516c4d82c4cc1ede63d04a4db2615a6c17ff1e1cd039b6462d",
  nativeArchiveSha256: "4af4c05996f882bc5d176cac4969bf8635d1efb33df2a9202b71ee7841c31596",
  replayArchiveSha256: "1615f275f8c33c3ec075dff59b23dc0ce584f62efd1d9b2a8165f15170fea0ab",
  replayNoteSha256: "832caed175b12b7136b8caf862359f99b783bba25f5e78ab41406d0452e2a0ff",
  coreFiles: 40, fullRawFiles: 52, correctnessTimingsExcluded: true, historicalCohortsMixed: false,
  rows: [
    { profile: "baseline", ttftMeanSeconds: 2.8292537170000003, tpotMeanSeconds: 0.1668542422007874,
      workloadMeanSeconds: 24.0197425565, meanPerRunOutputTokensPerSecond: 5.415717612777184,
      tpotRangeSeconds: [0.14353043494488188, 0.19017804945669292], tpotRangePercentOfMean: 27.95710429446361 },
    { profile: "wave-v15", ttftMeanSeconds: 2.4754419739999998, tpotMeanSeconds: 0.11323067961023622,
      workloadMeanSeconds: 16.855738369500003, meanPerRunOutputTokensPerSecond: 7.8546680779958375,
      tpotRangeSeconds: [0.09006585533070867, 0.13639550388976376], tpotRangePercentOfMean: 40.916162226113514 },
  ],
  ttftReductionPercent: 12.505479479414273, tpotReductionPercent: 32.137967775504436,
  rateIncreasePercent: 45.034668341356834,
  pairedTpotReductionPercent: [28.280101578797844, 37.24964648417913],
  pairedRateIncreasePercent: [35.79438148277132, 52.1985768531815],
  descriptiveComparisonAdmitted: true, performanceQualified: false, stableGainClaimed: false,
  httpMeasurement: false, gpuDurationMeasurement: false, defaultPromotion: false, m1Completion: false,
};
const development = {
  source: "7c9283675f1f7f40e781f2d774d4c0fcd9e31d69", tree: "73518159bdb1f3b9c688509eaf25e4b10ace7d80",
  fe2o3Source: "ae26717922b1fb7ad62fdd5ad70814d83eb01177", fe2o3Tree: "2ef28f830df9832d53a62525e20aa33815c164fa",
  activeDependencySource: "c9c7036cf98034b638886f54a745701e70ac3571",
  activeDependencyTree: "27142fd1003494aac211bc8250584228ff09525c", activeDependencyValidationPending: false,
  activeDependencyValidationScope: "V15 kernel/verifier/emission and finite native fixtures",
  v15CanarySource: "9dfedb5d1f9b80bc3f80a859e8740a69b9e5d793",
  v15ModelSource: "87fdcead7418e8f64db29fdb2703f5a4c8c9b742",
  v15AdapterGatePending: false, v15ModelParityPending: false, v15PerformancePending: false,
  canaryIntegrated: true, hostState: "composite-passed", hostAdmitted: true, workerHostAdmitted: true,
  hostNoteSha256: "05bc395159aaa06d8f5b2f3908a160a35718247f78a931cb71ce5ec4345ea422",
  hostFinalArchiveSha256: "e0a7ed83af27957a33544dd7acb37dd12e774b9d4f162b6ee92b1317ba1c67b6",
  controllerSha256: "5b7fd71c289ac7efa95b8f0d2ea1ac9a5decdeef545bb5bef6066690934df9c5", controllerBytes: 9429152,
  controllerReceiptSha256: "35bba7be5b3327a6dc239511e5dabcb8bb24bd4baca23a6e837f6e1932230019",
  controllerBuildLogSha256: "83e7dfd68a96dc7f5bc948a7efe03ca693acbd7187161526ae865ee37c1026de",
  hostPolicyLogSha256: "3b8d91150a7c8f86364f9690ee67554a09d5b8a0f834191d1ceba0dc1cc3ef58",
  hostBinaryMapSha256: "cf528074a0033e2ed23d664c0364b813c9e561d83370cc2e6a16eab15135dbae",
  hostVerifierSource: "4f215146ba1033b721148372138133611ee23a8b", hostPolicyTestsPassed: 31,
  hostFormatSource: "0793070acbe27a3d516c6172196119786b69a677", hostFormatExit: 0,
  hostFormatNoteSha256: "103c7c6988fcb7fad2434587131785a79160b7cf38f81cdad504b62eec0ee8cb",
  fullCurrentAllTargetRerun: false, priorHostFailuresPreserved: true,
  workerTestsPassed: 484, workerTestsIgnored: 1, workerDoctestsPassed: 31, workerStrictClippyExit: 0,
  workerSha256: "526cc6bf8902128767a5318c90d1c0206a9e435957612c398eeb28ff2ae71032", workerBytes: 1591144,
  workerNoteSha256: "aaed3d109764d09f2701983e5cba85089358e51fb64aae26c3b7e4335305a238",
  workerArchiveSha256: "006295ae51234b789c8e054328a6ce22b84485cabdbe186f7e28d45a26dbd87d",
  workerReceiptSha256: "2a82fdfdf148ab68f60981550c6ae5fc8aea38251edd99760171543abebf2b32",
  workerTestsLogSha256: "cdc5935cebacddb1ff21f9060ab155d0e49a6fbf9aad40ead930d08880fb0f9c",
  workerDocsLogSha256: "e6e9e9e5a16c0c2a68bcf696aa0df3aba50573c45475fbf40df45a10ec79d4c9",
  workerClippyLogSha256: "7d3f20709fe76ad4c2c435857d91a9fc0f4e389fbc022d34508c1e183594ad36",
  workerBuildLogSha256: "58fcf627fda086459d9f56f836818fb84ed66a91c1e347e871fae90ce330d8e0",
  finiteNativeChecksPassed: true,
  checkerMethodsPassed: 32, currentWorkerWrapperMethodsPassed: 8,
  checkerArchiveSha256: "60a0753844cf780704fe8d4078a21e48a8c29c85a6a615b2486078d50c6a15b1",
  wrapperArchiveSha256: "89f771db8322b60aa9d852e292e0843e2bc3ba3d523eac07226db35e7e9d4bc1",
  model: {
    scope: "four-fixed-prompt-correctness-cases", correctnessPassed: true, cases: 4, contextTokens: 256, prefillChunkTokens: 16,
    promptTokens: 128, tensorParallel: 1, layerProjection: "c1-wave", submission: "ordered", argmaxMode: "wave-v11",
    bindingsSha256: "a55c27aece7070935e970b03bebe1c232837562598b8b332f53874badb5a4c0b",
    archiveSha256: "8d4fe9046241c80c7692980d6cbbfaeacb9d10032c5cdd50f3bad9b2893ef3a4",
    noteSha256: "14a0b50e2923b72778e08d2d6fca06a0415ddaa0fc99ddba5c7c68ba410de488",
    replayNoteSha256: "a38f1d6cac44fc9117c6f97c958a2d12d6d8d5c73f18e677bfb4f30e11284e87",
    replayReportSha256: "8e920204fcd7f2db285b1b19e5bf9c7251fbf7d327e6f9a9c21e2c8591502308",
    replayArchiveSha256: "26727569f6f9a537eeefbc66481a9df3bbc59e52ffed6f2375e0f3a6760fd911",
    cleanupSha256: "adb2976ce274a8f114429d295b7d633629ab1696c84dc650b88dc44219c662c0",
    failedArchiveSha256: "41cf92f0ea2ca9463809cd0673fc78c488b067b420faed52cd9f5af2c2bb28e9",
    failedWrapperSha256: "fae6d748b56b166937efede6940fd56419f5e26d21137cb9cf4f9ba051d3564c",
    failedTimingSha256: "840a52118144145a1b5a3cbdaf269b5dc8c0430284e687cd5ebadf17dc251943",
    rows: [
      { name: "resident8", attentionMode: "resident-wave", outputs: 8, packets: 9219, batches: 15, committedInputs: 135,
        wrapperSha256: "ff6ebb9c8e85239db36bfddaa8b3d6f65fdf98a26d554286fef3535cc9538df2" },
      { name: "candidate8", attentionMode: "query-hoist-v14", outputs: 8, packets: 9219, batches: 15, committedInputs: 135,
        wrapperSha256: "6743a99ac1bd6b3cc1335b090d3ff9b06d7037a741b77c003c8f4f041dc3a870" },
      { name: "resident128", attentionMode: "resident-wave", outputs: 128, packets: 83139, batches: 135, committedInputs: 255,
        wrapperSha256: "6da7b69d28464720c21d09de3e80851497f3318e681194ce16312b53a0df595e" },
      { name: "candidate128", attentionMode: "query-hoist-v14", outputs: 128, packets: 83139, batches: 135, committedInputs: 255,
        wrapperSha256: "b2c489548d97cadbead1d0528c4b1bca8b790b2e26a6fbf5df27a08628cab325" },
    ],
    normalUnforcedClose: true, allEightIdleBeforeAfter: true, cpuReplayPassed: true,
    layoutOnlyCorrection: true, originalFailurePreserved: true, sameCompilerAblation: false,
    httpMeasurement: false, performanceQualified: false, defaultPromotion: false,
  },
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
  exact(value.v15, v15, ["detail", "emission", "current", "model", "abba"]);
  exact(value.v15.emission, v15Emission);
  exact(value.v15.current, v15Current);
  exact(value.v15.model, v15Model, ["detail"]);
  exact(value.v15.abba, v15Abba, ["detail"]);
  assert.equal(value.development.v15ModelSource, value.v15.model.source);
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
  phrases(value.v15.detail, ["eight finite GPU cases", "rows 1/16/17/32, width4096",
    "40 complete buffer records and 80 guards", "normal unforced close", "all eight GPUs idle before and after",
    "30 CPU methods", "all 357 protocol rows", "guarded write/read payload hashes", "complete ABI",
    "source f393390 uses actual c9c7036 compiler tools", "23 kernel tests, 31 verifier tests",
    "strict Clippy exit zero", "all fifteen emission phases", "14 explicit arguments", "96 explicit bytes",
    "hidden start 96 and 352 total kernarg bytes", "Wave64, 70 SGPR / 22 VGPR and zero spills/private/LDS/AGPR",
    "Historical 39be/ae267 R3 emission", "earlier c534/61e CPU checkpoint", "prior failures",
    "LLVM worker remains an actual 216822 build", "production runtime worker remains ae267",
    "FP32 association differs from the old serial fold", "not general FP32 proof, invalid-case qualification or Qwen model parity",
    "No performance gain or default promotion is claimed", "historical C1 measurements are unchanged"]);
  phrases(value.v15.model.detail, ["four full-Qwen correctness cases pass", "exact token IDs and UTF8 for 8 and 128 outputs",
    "same 128-token prompt", "all twenty host phases", "test invocations overlap", "same V5/V8/V11/V15 images",
    "actual ae267 worker", "Only three pure target-width4096 norm sites change", "normal unforced close",
    "Independent CPU replay reproduces all four complete checked traces and wrappers", "no numerical retry occurred",
    "not generalized or context8192 parity", "Correctness timings are excluded from the separate ABBA cohort"]);
  phrases(value.v15.abba.detail, ["predeclared baseline / V15 / V15 / baseline", "independent replay of all forty core files",
    "n=2 per mode", "2.829254 to 2.475442 seconds", "166.854 to 113.231 ms", "5.415718 to 7.854668 tokens/s",
    "TPOT is 32.138% lower and rate 45.035% higher", "27.96% for baseline and 40.92% for V15",
    "paired TPOT reductions are 28.280% and 37.250%", "arithmetic mean of per-run 128/workload seconds, not pooled duration",
    "overlapping parent/IPC/wait scopes", "not HTTP latency, GPU duration, a stable gain or competitive result",
    "frozen c9 V15 image and actual ae267 worker", "all historical HTTP measurements and all 33 M1 gates remain unchanged",
    "No default promotion or HTTP qualification follows"]);
  phrases(value.development.detail, ["opt-in V14 full-Qwen canary is integrated in private source",
    "Scoped current controller host checks pass", "11 focused tests with one ignore",
    "all 31 verifier-policy tests", "failures remain retained, not relabeled",
    "full 1,025-test suite belongs to the earlier 61e source and was not rerun at ae267",
    "484 library tests with one unchanged ignore, 31 doctests",
    "strict release Clippy exit zero", "All twelve worker host phases pass", "current worker record is fresh:false",
    "32 synthetic methods", "eight focused wrapper methods", "rejection of superseded 61e provenance",
    "These are not model runs", "four real Qwen3-8B correctness cases pass",
    "same 128-token prompt", "same controller, current worker and V14 preload",
    "independent CPU replay exactly reproduces all four wrapper objects",
    "ContentDirectoryIdentity failure occurred before setup and remains archived",
    "R2 changed only image path/layout and wrapper/case tags",
    "do not establish generalized or context8192 parity", "Raw host timings are retained without reduction",
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
    (x) => { x.v15.emissionAdmitted = false; }, (x) => { x.v15.nativeAdmitted = true; },
    (x) => { x.v15.modelParityQualified = true; }, (x) => { x.v15.performanceGainClaimed = true; },
    (x) => { x.development.hostAdmitted = false; }, (x) => { x.development.workerHostAdmitted = false; },
    (x) => { x.development.fe2o3Source = x.v15.dependencySource; },
    (x) => { x.development.modelParityQualified = true; },
    (x) => { x.development.workerTestsPassed = 485; }, (x) => { x.development.workerTestsIgnored = 0; },
    (x) => { x.development.finiteNativeChecksPassed = false; }, (x) => { x.development.workerSha256 = "0".repeat(64); },
    (x) => { x.development.fullCurrentAllTargetRerun = true; }, (x) => { x.development.priorHostFailuresPreserved = false; },
    (x) => { x.development.hostPolicyTestsPassed = 30; }, (x) => { x.development.controllerSha256 = "0".repeat(64); },
    (x) => { x.development.hostFormatExit = 1; }, (x) => { x.development.model.cases = 3; },
    (x) => { x.development.model.contextTokens = 8192; }, (x) => { x.development.model.cpuReplayPassed = false; },
    (x) => { x.development.model.rows[1].attentionMode = "resident-wave"; },
    (x) => { x.development.model.rows[3].packets = 83138; },
    (x) => { x.development.model.sameCompilerAblation = true; }, (x) => { x.development.model.httpMeasurement = true; },
    (x) => { x.development.model.originalFailurePreserved = false; }, (x) => { x.development.model.normalUnforcedClose = false; },
    (x) => { x.v15.emissionAttempt.emissionInvoked = true; }, (x) => { x.v15.emissionAttempt.metadataExit = 0; },
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
  const rmsEmissionNote = (await pinned("v15-r3-note.md", v15Emission.noteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(rmsEmissionNote, [v15Emission.source, v15Emission.tree, v15Emission.archiveSha256,
    "all twelve phases with actual command, wrapper and outer status 0", "Strict all-target Clippy: 0",
    "0/7/16 passed", "No GPU was used", "The later upstream 18946 revision is not attributed to these bytes"]);
  const rmsEmissionTests = (await pinned("v15-r3-tests.log", v15Emission.testsLogSha256)).toString("utf8");
  assert.deepEqual([...rmsEmissionTests.matchAll(/^test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;/gm)]
    .map((row) => Number(row[1])), [0, 7, 16]);
  await pinned("v15-r3-clippy.log", v15Emission.clippyLogSha256);
  const rmsEmissionArtifacts = JSON.parse(await pinned("v15-r3-artifacts.json", v15Emission.artifactReceiptSha256));
  for (const [key, expected] of Object.entries({ source: v15Emission.source, tree: v15Emission.tree, passed: 23, ignored: 0, rows: 3 }))
    assert.deepEqual(rmsEmissionArtifacts[key], expected);
  assert.equal(rmsEmissionArtifacts.records.length, 5);
  assert.equal(Object.keys(rmsEmissionArtifacts.files).length, 6);
  assert(rmsEmissionArtifacts.records.every((row) => row.fresh === false));
  const rmsImage = JSON.parse(await pinned("v15-r3-inspection.json", v15Emission.inspectionSha256));
  assert.equal(rmsImage.schema, "FerricWaveRmsNormV15EmissionAbiResourceReviewV1");
  assert.equal(rmsImage.abi_resources_passed, true);
  for (const key of ["gpu_used", "numerical_parity", "performance_claim", "same_compiler_ablation"])
    assert.equal(rmsImage[key], false);
  for (const [key, expected] of Object.entries({ source: v15Emission.source, source_tree: v15Emission.tree,
    compiler: v15Emission.compilerSource, compiler_tree: v15Emission.compilerTree,
    worker_actual_build: v15Emission.llvmWorkerActualBuild, image_sha256: v15Emission.imageSha256, image_bytes: v15Emission.imageBytes }))
    assert.deepEqual(rmsImage.identities[key], expected);
  assert.deepEqual(Object.keys(rmsImage.image.roots), ["ferric_qwen3_tp_batch32_wave_rmsnorm_bf16_v15"]);
  const rmsImageRoot = Object.values(rmsImage.image.roots)[0];
  assert.equal(rmsImageRoot.explicit_bytes, 96);
  const rmsImageMeta = rmsImageRoot.metadata;
  assert.equal(rmsImageMeta[".args"].filter((arg) => !arg[".value_kind"].startsWith("hidden_")).length, 14);
  assert.equal(rmsImageMeta[".args"].find((arg) => arg[".value_kind"].startsWith("hidden_"))[".offset"], 96);
  assert.deepEqual(rmsImageMeta[".reqd_workgroup_size"], [64, 1, 1]);
  for (const [key, expected] of Object.entries({ ".kernarg_segment_size": 352, ".kernarg_segment_align": 8,
    ".wavefront_size": 64, ".sgpr_count": 70, ".vgpr_count": 22, ".sgpr_spill_count": 0, ".vgpr_spill_count": 0,
    ".private_segment_fixed_size": 0, ".group_segment_fixed_size": 0, ".agpr_count": 0, ".uses_dynamic_stack": false }))
    assert.deepEqual(rmsImageMeta[key], expected);
  const rmsStatic = (await pinned("v15-r3-static-review.md", v15Emission.staticReviewSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(rmsStatic, ["No concrete static blocker found", "not a proof of all FP32 behavior or a native result",
    "no detailed typed-node/LLVM payload was retained"]);
  const currentEmissionNote = (await pinned("v15-r4-note.md", v15Current.emissionNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(currentEmissionNote, [v15Current.source, v15Current.compilerSource, v15Current.emissionArchiveSha256,
    "All15 frozen phases", "LLVM worker remains actual216822", "Historical failures and source39be/ae R3 remain intact"]);
  for (const [name, logSha, receiptSha, counts, records, files] of [
    ["host", v15Current.testsLogSha256, v15Current.artifactReceiptSha256, [0, 7, 16], 5, 6],
    ["verifier", v15Current.verifierLogSha256, v15Current.verifierArtifactSha256, [31], 2, 3],
  ]) {
    const log = (await pinned(`v15-r4-${name}-tests.log`, logSha)).toString("utf8");
    assert.deepEqual([...log.matchAll(/^test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;/gm)]
      .map((row) => Number(row[1])), counts);
    const receipt = JSON.parse(await pinned(`v15-r4-${name}-artifacts.json`, receiptSha));
    for (const [key, expected] of Object.entries({ source: v15Current.source, tree: v15Current.tree,
      passed: counts.reduce((sum, count) => sum + count, 0), ignored: 0, rows: counts.length }))
      assert.deepEqual(receipt[key], expected);
    assert.equal(receipt.records.length, records);
    assert.equal(Object.keys(receipt.files).length, files);
    assert(receipt.records.every((row) => row.fresh === false));
  }
  await pinned("v15-r4-clippy.log", v15Current.clippyLogSha256);
  const currentImage = JSON.parse(await pinned("v15-r4-inspection.json", v15Current.inspectionSha256));
  assert.equal(currentImage.schema, rmsImage.schema);
  assert.equal(currentImage.abi_resources_passed, true);
  assert.deepEqual(currentImage.image, rmsImage.image);
  for (const key of ["gpu_used", "numerical_parity", "performance_claim", "same_compiler_ablation"])
    assert.equal(currentImage[key], false);
  for (const [key, expected] of Object.entries({ source: v15Current.source, source_tree: v15Current.tree,
    compiler: v15Current.compilerSource, compiler_tree: v15Current.compilerTree,
    worker_actual_build: v15Current.llvmWorkerActualBuild, image_sha256: v15Current.imageSha256, image_bytes: v15Current.imageBytes }))
    assert.deepEqual(currentImage.identities[key], expected);
  const nativeCpuNote = (await pinned("v15-native-cpu-note.md", v15Current.cpuNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(nativeCpuNote, [v15Current.nativeSource, v15Current.nativeTree, v15Current.cpuArchiveSha256,
    "30 tests passed", "10 fixture, 10 native-probe, 10 wrapper methods", "synthetic CPU validation"]);
  const nativeCpuLog = (await pinned("v15-native-cpu-tests.log", v15Current.cpuTestsLogSha256)).toString("utf8");
  assert.match(nativeCpuLog, /\nRan 30 tests in [\d.]+s\n\nOK\n$/);
  const nativeNote = (await pinned("v15-native-note.md", v15Current.nativeNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(nativeNote, [v15Current.source, v15Current.compilerSource, v15Current.workerSource, v15Current.nativeArchiveSha256,
    "40 buffer records and80 guards", "All eight GPUs were idle before and after", "group cleanup was unforced and complete",
    "This admits these eight finite cases only", development.v15CanarySource]);
  const nativeReport = JSON.parse(await pinned("v15-native-report.json", v15Current.nativeReportSha256));
  assert.equal(nativeReport.schema, "FerricTp1WaveRmsnormProbeV15");
  assert.equal(nativeReport.artifact_sha256, v15Current.imageSha256);
  assert.equal(nativeReport.worker_sha256, v15Current.workerSha256);
  for (const key of ["checks_pass", "clean_teardown", "runtime_operational", "active_inputs_finite"])
    assert.equal(nativeReport[key], true);
  const expectedNames = ["uniform", "signed"].flatMap((mode) => v15Current.rows.map((rows) => `wave_rmsnorm_v15_${mode}_rows${rows}`));
  assert.deepEqual(nativeReport.results.map((row) => row.name), expectedNames);
  const nativeBuffers = nativeReport.results.flatMap((row) => row.checks);
  assert.equal(nativeBuffers.length, v15Current.bufferRecords);
  assert.equal(nativeBuffers.filter((row) => row.bytes === 0).length, 16);
  assert(nativeBuffers.every((row) => row.guard_bytes_each_side === 64 && row.guards_unchanged === true));
  const nativeWrapper = JSON.parse(await pinned("v15-native-wrapper.json", v15Current.nativeWrapperSha256));
  assert.equal(nativeWrapper.schema, "FerricTp1WaveRmsnormV15WrapperV1");
  for (const key of ["passed", "all_eight_idle_before", "all_eight_idle_after"])
    assert.equal(nativeWrapper[key], true);
  assert.deepEqual(nativeWrapper.errors, []);
  const nativeProtocol = await pinned("v15-native-protocol.jsonl", v15Current.protocolSha256);
  assert.equal(nativeProtocol.toString("utf8").trim().split("\n").length, v15Current.protocolRows);
  const nativeReplayNote = (await pinned("v15-native-replay-note.md", v15Current.replayNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(nativeReplayNote, [v15Current.nativeArchiveSha256, v15Current.replayArchiveSha256,
    "357 protocol rows", "no metrics were calculated", "does not recover raw device payloads from hashes"]);
  const nativeReplay = JSON.parse(await pinned("v15-native-replay.json", v15Current.replayReportSha256));
  for (const [key, expected] of Object.entries({ schema: "FerricTp1WaveRmsnormV15NativeCpuReplayV1", passed: true,
    cpu_replay: true, gpu_used: false, cases: 8, complete_buffer_records: 40, guards: 80, protocol_rows: 357,
    read_write_payload_hashes_reconstructed: true, exact_abi_validated: true,
    native_archive_sha256: v15Current.nativeArchiveSha256, report_sha256: v15Current.nativeReportSha256,
    protocol_sha256: v15Current.protocolSha256, artifact_sha256: v15Current.imageSha256, worker_sha256: v15Current.workerSha256,
    elapsed_fields: "associated only; no metric reduction" }))
    assert.deepEqual(nativeReplay[key], expected);
  for (const record of [nativeReport, nativeWrapper, nativeReplay]) {
    for (const key of ["benchmark", "model_inference", "model_parity_qualified", "performance_qualified"])
      assert.equal(record[key], false);
    assert.equal(record.authority, "none");
  }
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
  const hostNote = (await pinned("current-controller-note.md", development.hostNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(hostNote, [development.source, development.tree, development.hostVerifierSource,
    development.controllerSha256, development.hostFinalArchiveSha256, "Verifier source-policy31 passed/0 ignored",
    "The full1025-test suite was not rerun at ae267", "R3 formatting is not relabeled as passing",
    "No production rebuild", "Native numerical qualification remains a separate root-owned gate"]);
  await pinned("current-controller-build.log", development.controllerBuildLogSha256);
  const policyLog = (await pinned("current-controller-policy.log", development.hostPolicyLogSha256)).toString("utf8");
  assert.equal([...policyLog.matchAll(/^test result: ok\. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;/gm)].length, 1);
  const binaryMap = (await pinned("current-controller-binaries.sha256", development.hostBinaryMapSha256)).toString("utf8");
  assert.equal(binaryMap.trim().split("\n").length, 7);
  phrases(binaryMap, [development.controllerSha256]);
  const controllerReceipt = JSON.parse(await pinned("current-controller-artifacts.json", development.controllerReceiptSha256));
  assert.equal(controllerReceipt.commit, development.source);
  assert.equal(controllerReceipt.tree, development.tree);
  assert.equal(controllerReceipt.passed, true);
  assert.equal(controllerReceipt.warm_target, true);
  assert.equal(controllerReceipt.full_r1_suite_repeated, false);
  const controllerRecords = controllerReceipt.own_compiler_artifacts.filter((row) => row.target.name === "ferric-qwen3-query-hoist-v14-canary");
  assert.equal(controllerRecords.length, 1);
  assert.equal(controllerRecords[0].fresh, false);
  assert.equal(controllerRecords[0].profile.test, false);
  const retainedController = controllerReceipt.retained.filter((row) => row.path === controllerRecords[0].executable);
  assert.equal(retainedController.length, 1);
  assert.equal(retainedController[0].sha256, development.controllerSha256);
  assert.equal(retainedController[0].bytes, development.controllerBytes);
  const formatNote = (await pinned("current-controller-format.md", development.hostFormatNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(formatNote, [development.hostFormatSource, "Actual command/outer status0", "Input and tool hashes were unchanged afterward",
    "No formatter application, full source snapshot, build, test or GPU work occurred",
    "separate from preserved R3 formatting1", "production binaries remain exact source7c"]);
  const model = development.model;
  const modelNote = (await pinned("v14-model-native-note.md", model.noteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(modelNote, [development.source, development.workerSha256, model.archiveSha256, model.failedArchiveSha256,
    model.replayArchiveSha256, "four sequential real Qwen3-8B BF16 correctness cases",
    "All eight GPUs were idle before and after each case", "No controller, worker, kernel, checker, workload or reference bytes changed",
    "not generalized numerical parity", "no new TTFT/TPOT/throughput result"]);
  const bindings = JSON.parse(await pinned("v14-model-bindings.json", model.bindingsSha256));
  assert.equal(bindings.schema, "FerricQueryHoistV14CanaryBindingsV1");
  for (const [key, source, tree, sha, bytes, receipt] of [
    ["controller", development.source, development.tree, development.controllerSha256, development.controllerBytes, development.hostNoteSha256],
    ["worker", development.fe2o3Source, development.fe2o3Tree, development.workerSha256, development.workerBytes, development.workerNoteSha256],
  ]) {
    assert.equal(bindings[key].source, source);
    assert.equal(bindings[key].tree, tree);
    assert.equal(bindings[key].sha256, sha);
    assert.equal(bindings[key].byte_len, bytes);
    assert.equal(bindings[key].host_receipt_sha256, receipt);
  }
  const query = bindings.query_hoist_artifact;
  assert.equal(query.hsaco, v14Emission.imageSha256);
  assert.equal(query.manifest, v14Emission.observationSha256);
  assert.equal(query.handoff, "dacfab37c50f78828c329322d34f14dea67a383baf8402b3f7d1bbb843947798");
  assert.equal(query.path, "/tmp/ferric-compete-gpu.VabkOGCx/fe2o3-engineering-v1/2c1332e73f150935046c803f446716b5f73f7dd33e80145cc6404c509004779a");
  const modelTraces = [];
  for (const row of model.rows) {
    const report = JSON.parse(await pinned(`v14-model-${row.name}.json`, row.wrapperSha256));
    assert.equal(report.schema, "FerricQueryHoistV14ModelWrapperV1");
    assert.equal(report.passed, true);
    assert.deepEqual(report.errors, []);
    assert.equal(report.bindings_sha256, model.bindingsSha256);
    assert.equal(report.max_new_tokens, row.outputs);
    assert.equal(report.all_eight_idle_before, true);
    assert.equal(report.all_eight_idle_after, true);
    assert.equal(report.measurement, "correctness-only-raw-host-timing-not-reduced");
    const trace = report.checked_trace;
    assert.equal(trace.schema, "FerricQueryHoistV14CanaryCheckedV1");
    for (const record of [report, trace]) {
      assert.equal(record.attention_mode, row.attentionMode);
      assert.equal(record.layer_projection, model.layerProjection);
      assert.equal(record.submission, model.submission);
      assert.equal(record.correctness_only, true);
      assert.equal(record.performance_qualified, false);
      assert.equal(record.benchmark_admitted, false);
      assert.equal(record.serving_admitted, false);
      assert.equal(record.authority, "none");
    }
    assert.equal(trace.argmax_mode, model.argmaxMode);
    assert.equal(trace.controller_sha256, development.controllerSha256);
    assert.equal(trace.worker_sha256, development.workerSha256);
    assert.equal(trace.reference_passed, true);
    assert.equal(trace.http_measurement, false);
    assert.equal(trace.gpu_clock_measurement, false);
    assert.equal(trace.completed_packets, row.packets);
    assert.equal(trace.completed_batches, row.batches);
    assert.equal(trace.committed_inputs_before_retirement, row.committedInputs);
    assert.equal(trace.generated_token_ids.length, row.outputs);
    assert.equal(trace.query_hoist_artifact_path, query.path);
    assert.deepEqual(trace.query_hoist_artifact, { hsaco: query.hsaco, manifest: query.manifest, handoff: query.handoff });
    const cleanup = JSON.parse(await pinned(`v14-model-${row.name}-cleanup.json`, model.cleanupSha256));
    assert.deepEqual(cleanup, { absent: true, cleanup_error: null, controller_returncode: 0, forced: false, reason: null });
    modelTraces.push(trace);
  }
  for (const [left, right] of [[0, 1], [2, 3]]) {
    assert.deepEqual(modelTraces[left].generated_token_ids, modelTraces[right].generated_token_ids);
    assert.equal(modelTraces[left].generated_utf8_hex, modelTraces[right].generated_utf8_hex);
  }
  const failedModel = JSON.parse(await pinned("v14-model-failed-r1.json", model.failedWrapperSha256));
  assert.equal(failedModel.passed, false);
  assert.equal(failedModel.checked_trace, null);
  assert.deepEqual(failedModel.errors, ["ValueError: execution or owned cleanup failed"]);
  assert.equal(failedModel.all_eight_idle_before, true);
  assert.equal(failedModel.all_eight_idle_after, true);
  const failedTiming = JSON.parse(await pinned("v14-model-failed-r1-timing.json", model.failedTimingSha256));
  assert.equal(failedTiming.schema, "FerricHostTimingV1");
  assert.equal(failedTiming.failure, "non-authoritative M1 engineering aggregate rejected: ContentDirectoryIdentity");
  assert.equal(failedTiming.setup, null);
  assert.equal(failedTiming.closed, null);
  assert.equal(failedTiming.active_records, 0);
  assert.deepEqual(failedTiming.records, []);
  assert.equal(failedTiming.run_status, "failed");
  const modelReplayNote = (await pinned("v14-model-replay-note.md", model.replayNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(modelReplayNote, [model.archiveSha256, model.replayArchiveSha256, model.replayReportSha256,
    "matched each complete recorded wrapper object exactly", "all twelve actual native files",
    "not a timing reduction or performance admission", "All 52 extracted regular-file hashes verified"]);
  const modelReplay = JSON.parse(await pinned("v14-model-replay.json", model.replayReportSha256));
  assert.equal(modelReplay.schema, "FerricQueryHoistV14NativeReplayV1");
  assert.equal(modelReplay.passed, true);
  assert.equal(modelReplay.bindings_sha256, model.bindingsSha256);
  assert.equal(modelReplay.externally_authenticated_native_archive_sha256, model.archiveSha256);
  assert.equal(modelReplay.raw_files_checked, 12);
  assert.equal(modelReplay.gpu_execution, false);
  assert.equal(modelReplay.timing_reduced, false);
  assert.equal(modelReplay.performance_qualified, false);
  assert.equal(modelReplay.authority, "none");
  assert.equal(modelReplay.cases.length, model.cases);
  for (const [index, row] of model.rows.entries()) {
    const replayed = modelReplay.cases[index];
    assert.equal(replayed.exact_wrapper_match, true);
    assert.equal(replayed.case, `v14-model-7c92836-${row.name}-r2`);
    assert.equal(replayed.attention_mode, row.attentionMode);
    assert.equal(replayed.max_new_tokens, row.outputs);
    assert.deepEqual(replayed.checked_trace.generated_token_ids, modelTraces[index].generated_token_ids);
    assert.equal(replayed.checked_trace.generated_utf8_hex, modelTraces[index].generated_utf8_hex);
  }
  const v15ModelHostNote = (await pinned("v15-model-host-note.md", v15Model.hostNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(v15ModelHostNote, [v15Model.source, v15Model.tree, v15Model.controllerSha256, v15Model.hostArchiveSha256,
    "all 20 commands, wrappers and outer status 0", "overlapping invocations are not pooled as unique coverage",
    "non-test policy artifact fresh=false", "fresh=true with identical retained bytes", "All earlier failures remain retained"]);
  const v15ModelHost = JSON.parse(await pinned("v15-model-host-receipt.json", v15Model.hostReceiptSha256));
  for (const [key, expected] of Object.entries({ schema: "FerricWaveRmsNormV15ControllerHostReceiptV1", authority: "none",
    source: v15Model.source, tree: v15Model.tree, fe2o3_source: v15Current.compilerSource,
    executable_sha256: v15Model.controllerSha256, executable_bytes: v15Model.controllerBytes,
    complete_archive_sha256: v15Model.hostArchiveSha256, test_results: v15Model.hostTests,
    phases_passed: v15Model.hostPhasesPassed, strict_clippy_exit: 0, format_exit: 0, release_exit: 0, outer_exit: 0,
    normal_closed_groups: 20, forced_cleanup: false, gpu_used: false, model_parity_admitted: false, performance_admitted: false }))
    assert.deepEqual(v15ModelHost[key], expected);
  const v15ModelNote = (await pinned("v15-model-note.md", v15Model.noteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(v15ModelNote, [v15Model.source, v15Model.tree, v15Model.controllerSha256, v15Model.archiveSha256,
    "same executable and V5/V8/V11/V15 preloads", "normal unforced close", "135/255 committed-input accounting passed",
    "No numerical retry, oracle change, or discarded model failure occurred", "No timing from these correctness cases is reduced"]);
  await pinned("v15-model-launcher-correction.md", v15Model.launcherCorrectionSha256);
  const v15ModelBindings = JSON.parse(await pinned("v15-model-bindings.json", v15Model.bindingsSha256));
  assert.equal(v15ModelBindings.schema, "FerricWaveRmsNormV15CanaryBindingsV1");
  for (const [key, source, tree, sha, bytes, receipt] of [
    ["controller", v15Model.source, v15Model.tree, v15Model.controllerSha256, v15Model.controllerBytes, v15Model.hostReceiptSha256],
    ["worker", development.fe2o3Source, development.fe2o3Tree, development.workerSha256, development.workerBytes, development.workerNoteSha256],
  ]) {
    assert.deepEqual({ source: v15ModelBindings[key].source, tree: v15ModelBindings[key].tree,
      sha256: v15ModelBindings[key].sha256, byte_len: v15ModelBindings[key].byte_len,
      host_receipt_sha256: v15ModelBindings[key].host_receipt_sha256 },
    { source, tree, sha256: sha, byte_len: bytes, host_receipt_sha256: receipt });
  }
  const v15ModelArtifact = v15ModelBindings.rmsnorm_artifact;
  assert.equal(v15ModelArtifact.hsaco, v15Current.imageSha256);
  const v15ModelTraces = [];
  for (const row of v15Model.rowsChecked) {
    const report = JSON.parse(await pinned(`v15-model-${row.name}.json`, row.wrapperSha256));
    assert.equal(report.schema, "FerricWaveRmsNormV15ModelWrapperV1");
    for (const key of ["passed", "all_eight_idle_before", "all_eight_idle_after"])
      assert.equal(report[key], true);
    assert.deepEqual(report.errors, []);
    assert.equal(report.bindings_sha256, v15Model.bindingsSha256);
    assert.equal(report.max_new_tokens, row.outputs);
    assert.equal(report.measurement, "correctness-only-raw-host-timing-not-reduced");
    const trace = report.checked_trace;
    assert.equal(trace.schema, "FerricWaveRmsNormV15CanaryCheckedV1");
    for (const record of [report, trace]) {
      for (const [key, expected] of Object.entries({ rmsnorm_mode: row.rmsnormMode, attention: v15Model.attention,
        layer_projection: v15Model.layerProjection, submission: v15Model.submission, correctness_only: true,
        performance_qualified: false, benchmark_admitted: false, serving_admitted: false, authority: "none" }))
        assert.deepEqual(record[key], expected);
    }
    for (const [key, expected] of Object.entries({ argmax_mode: v15Model.argmaxMode,
      controller_sha256: v15Model.controllerSha256, worker_sha256: v15Current.workerSha256,
      reference_passed: true, http_measurement: false, gpu_clock_measurement: false,
      completed_packets: row.packets, completed_batches: row.batches, committed_inputs_before_retirement: row.committedInputs }))
      assert.deepEqual(trace[key], expected);
    assert.equal(trace.generated_token_ids.length, row.outputs);
    assert.equal(trace.rmsnorm_artifact_path, v15ModelArtifact.path);
    assert.deepEqual(trace.rmsnorm_artifact, { hsaco: v15ModelArtifact.hsaco, manifest: v15ModelArtifact.manifest, handoff: v15ModelArtifact.handoff });
    const cleanup = JSON.parse(await pinned(`v15-model-${row.name}-cleanup.json`, v15Model.cleanupSha256));
    assert.deepEqual(cleanup, { absent: true, cleanup_error: null, controller_returncode: 0, forced: false, reason: null });
    v15ModelTraces.push(trace);
  }
  for (const [left, right] of [[0, 1], [2, 3]]) {
    assert.deepEqual(v15ModelTraces[left].generated_token_ids, v15ModelTraces[right].generated_token_ids);
    assert.equal(v15ModelTraces[left].generated_utf8_hex, v15ModelTraces[right].generated_utf8_hex);
  }
  const v15ModelReplayNote = (await pinned("v15-model-replay-note.md", v15Model.replayNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(v15ModelReplayNote, [v15Model.archiveSha256, v15Model.replayArchiveSha256, v15Model.replayReportSha256,
    "reproduced every complete checked_trace and wrapper exactly", "No synthetic suite, model/GPU launch, timing reduction or performance claim"]);
  const v15ModelReplay = JSON.parse(await pinned("v15-model-replay.json", v15Model.replayReportSha256));
  for (const [key, expected] of Object.entries({ schema: "FerricWaveRmsNormV15NativeReplayV1", authority: "none", passed: true,
    bindings_sha256: v15Model.bindingsSha256, externally_authenticated_native_archive_sha256: v15Model.archiveSha256,
    raw_files_checked: 12, gpu_execution: false, timing_reduced: false, performance_qualified: false }))
    assert.deepEqual(v15ModelReplay[key], expected);
  assert.equal(v15ModelReplay.cases.length, v15Model.cases);
  for (const [index, row] of v15Model.rowsChecked.entries()) {
    assert.deepEqual(v15ModelReplay.cases[index], { case: `rmsnorm-v15-87fdcea-${row.name}-r1`,
      rmsnorm_mode: row.rmsnormMode, max_new_tokens: row.outputs, exact_wrapper_match: true, checked_trace: v15ModelTraces[index] });
  }
  const v15AbbaNote = (await pinned("v15-abba-note.md", v15Abba.replayNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(v15AbbaNote, [v15Abba.summarySha256, v15Abba.manifestSha256, v15Abba.nativeArchiveSha256,
    v15Abba.replayArchiveSha256, "unchanged reducer 0", "wrong external manifest pin rejection 1 with empty stdout",
    "descriptive n=2, not a confidence interval or stability claim", "Parent/IPC/wait intervals overlap"]);
  const v15AbbaManifest = JSON.parse(await pinned("v15-abba-manifest.json", v15Abba.manifestSha256));
  const v15AbbaSummary = JSON.parse(await pinned("v15-abba-summary.json", v15Abba.summarySha256));
  assert.equal(v15AbbaManifest.schema, "FerricWaveRmsNormV15AbbaEvidenceV1");
  assert.equal(v15AbbaSummary.schema, "FerricWaveRmsNormV15AbbaSummaryV1");
  for (const record of [v15AbbaManifest, v15AbbaSummary]) {
    assert.equal(record.authority, "none");
    assert.equal(record.performance_qualified, false);
    assert.equal(record.experiment_plan_sha256, v15Abba.experimentPlanSha256);
    assert.equal(record.reducer_sha256, v15Abba.reducerSha256);
    assert.deepEqual(record.runs.map((row) => row.id), v15Abba.acquisitionOrder);
    for (const mode of ["baseline", "wave-v15"]) {
      assert.equal(record.profiles[mode].rmsnorm_mode, mode);
      assert.equal(record.profiles[mode].source, v15Model.source);
      assert.equal(record.profiles[mode].tree, v15Model.tree);
      assert.equal(record.profiles[mode].sha256, v15Model.controllerSha256);
      assert.equal(record.profiles[mode].bindings_sha256, v15Model.bindingsSha256);
      assert.deepEqual(record.profiles[mode].identity_set, v15ModelBindings);
    }
  }
  assert.equal(v15AbbaSummary.evidence_manifest_sha256, v15Abba.manifestSha256);
  assert.equal(v15AbbaManifest.runs.flatMap((row) => Object.keys(row.files)).length, v15Abba.coreFiles);
  assert.equal(v15AbbaSummary.cohort.repetitions_per_mode, v15Abba.repetitionsPerMode);
  for (const key of ["stable_performance_gain_claimed", "confidence_qualified", "http_measurement", "gpu_clock_measurement",
    "default_promotion", "serving_qualified", "competitive_ranking", "m1_completion", "old_cohort_mixed", "additive_gain_claim"])
    assert.equal(v15AbbaSummary[key], false);
  for (const row of v15Abba.rows) {
    assert.deepEqual(v15AbbaSummary.cohort.arithmetic_metric_means[row.profile], {
      diagnostic_ttft_seconds: row.ttftMeanSeconds, diagnostic_tpot_seconds: row.tpotMeanSeconds,
      diagnostic_workload_seconds: row.workloadMeanSeconds, diagnostic_output_tokens_per_second: row.meanPerRunOutputTokensPerSecond,
    });
    const variability = v15AbbaSummary.cohort.within_mode_variability[row.profile].diagnostic_tpot_seconds;
    assert.deepEqual([variability.minimum, variability.maximum], row.tpotRangeSeconds);
    assert.equal(variability.range_percent_of_mean, row.tpotRangePercentOfMean);
  }
  for (const [key, expected] of [["diagnostic_ttft_seconds", v15Abba.ttftReductionPercent],
    ["diagnostic_tpot_seconds", v15Abba.tpotReductionPercent], ["diagnostic_output_tokens_per_second", v15Abba.rateIncreasePercent]])
    assert.equal(v15AbbaSummary.cohort.ratio_of_arithmetic_means[key].improvement_percent, expected);
  assert.deepEqual(v15AbbaSummary.cohort.repetition_pairs.map((row) => row.relative_changes.diagnostic_tpot_seconds.improvement_percent), v15Abba.pairedTpotReductionPercent);
  assert.deepEqual(v15AbbaSummary.cohort.repetition_pairs.map((row) => row.relative_changes.diagnostic_output_tokens_per_second.improvement_percent), v15Abba.pairedRateIncreasePercent);
  const emissionNote = (await pinned("v15-emission-failed-note.md", v15.emissionAttempt.noteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(emissionNote, [v15.emissionAttempt.archiveSha256, "emission command was never invoked",
    "nightly Cargo metadata returned 101", "smallvec v1.16.0", "No repair or retry was performed"]);
  const emissionDiagnostic = (await pinned("v15-emission-failed.stderr", v15.emissionAttempt.diagnosticSha256)).toString("utf8");
  phrases(emissionDiagnostic, ["failed to download `smallvec v1.16.0`", "--offline was specified"]);
  console.log("PASS: historical results unchanged; V15 c9 emission, eight finite native cases, twenty host phases, four fixed-prompt model cases and descriptive n=2 ABBA with independent replay are separately bound; no HTTP or stable-gain qualification.");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  assert.equal(process.argv.length, 3);
  await validateLiveC1CheckpointEvidence(process.argv[2], projectFrom(await readFile(join(siteRoot, "data/project.js"))));
}
