import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const liveHttpExpected = {
    schema: "FerricPagesLiveHttpCheckpointV1", date: "2026-09-12",
    source: "433e21354ca7e9b2f97b3e96caf39070d79c711a",
    tree: "0a0717414eb5fa143081132f816c18dc8058f0c6",
    controllerSha256: "9fd94a978358f0533c096ec9b5dd51d46833b4ca5426b1eaabb0337b4409c656",
    compilerSource: "8efd4fd416d1ffae7a718144e4d299fe3c8f7590",
    workerSource: "c110ac55c655579e0969b310402800b0c2666694",
    workerSha256: "b91ddef78135829f607d1b83f5cf898d745b36013327f0d2d12771aba1b5150b",
    contractSha256: "645e2bee141670be68c9a47c516099204502b2f3198e16f9de30e8394dbf2d31",
    driverSha256: "726b39888d01e9a0fa2c79352b36b146ee44cc18f965bc8f84bfaf07c696d0ee",
    clientSha256: "979136caea4f134f33f19c62b82a8ac9537205eaa11d11a43b7a3af466af0a2d",
    referenceSha256: "cd7f512537e44d677244c606cba919e57747e8c26caf3c0df4a0aca7acc0eb8b",
    model: "Qwen3-8B", hardware: "1 x MI350X, physical GPU 0",
    tensorParallel: 1, contextTokens: 8192, physicalPages: 512, rowCapacity: 32, prefillChunk: 16,
    promptTokens: 128, outputTokens: 128, concurrency: 1,
    freshStartsPerMode: 1, warmups: 10, measuredRequests: 30, diagnostics: 2,
    measuredOutputTokens: 3840, weightsAndDecoder: "BF16", head: "fp32-v8",
    projection: "mfma", attention: "wave", argmax: "wave-v11",
    speculation: false, prefixCache: false, hostTiming: false, runtimeProfiling: false,
    qualification: [
      { mode: "synchronous", requests: 2,
        receiptSha256: "697dc47d0137de1a1e644f4f6474c7729ea7a034bdef59f6cfe32e251fe9a565",
        archiveSha256: "0076b69c498a8df0378c753046a4cdb57beb3b65707a7eb1f139bb8ac94cf588" },
      { mode: "ordered", requests: 2,
        receiptSha256: "469ba83b882ddb4e81e3dbfa0fa215ae0bfb4a93770eaef419f6bc94c30cf4f2",
        archiveSha256: "2d357980ea3b1952b9961fae7571bbd9db3d69cea2efc3d9f47e84cfd6c8aea3" },
    ],
    cohorts: [
      { mode: "synchronous", ttftMeanMs: 3482.379512666667,
        ttftP50Ms: 3589.147967, ttftP99Ms: 3670.73661252,
        tpotMeanMs: 276.2553326648294, tpotP50Ms: 287.41766190944884,
        tpotP99Ms: 292.6767426588976, e2eMeanMs: 38567.20054086667,
        outputTokensPerSecond: 3.3188001312875013, windowSeconds: 1157.044669186,
        rawFiles: 20, diagnosticRequestsChecked: 2, timedRequestIdentitiesChecked: 40,
        timedFinalTokenIdsChecked: 40, exactOutput: true, normalUnforcedClose: true,
        allEightIdleBeforeAndAfter: true,
        planSha256: "43ac4e4d78e6f1ba06ca4086f639ff48272274d6566d6b38977d061d18fc60f1",
        receiptSha256: "55eae5a1ca6730ab142c13bfd946314f91371979d975cb5605b15339e7aa6c61",
        replaySummarySha256: "c930106517682ee5425199169f3c3a85632bd226e075e2ae820c5b78e0223969",
        archiveSha256: "35143839a6567b3afb7fac988b875275e760e6280dff737b6e2f5c08fcb0a251" },
      { mode: "ordered", ttftMeanMs: 2862.0793933,
        ttftP50Ms: 2827.064237, ttftP99Ms: 3128.58817749,
        tpotMeanMs: 192.12655409396325, tpotP50Ms: 189.08512703543306,
        tpotP99Ms: 212.69070149283465, e2eMeanMs: 27262.319697,
        outputTokensPerSecond: 4.694997388229715, windowSeconds: 817.891828785,
        rawFiles: 20, diagnosticRequestsChecked: 2, timedRequestIdentitiesChecked: 40,
        timedFinalTokenIdsChecked: 40, exactOutput: true, normalUnforcedClose: true,
        allEightIdleBeforeAndAfter: true,
        planSha256: "65d9bcc5dd3143960fce1262403f92bc8bb9680d7cfab9fdd736fbbb3bbf892b",
        receiptSha256: "f4f44c5a8070d7f7f54cfad25626f0e9b354b022723b74d790ee309f89d71dec",
        replaySummarySha256: "684727bd8a8f85bc819f5bdeabd4d2fa2d536f7fbe4525915155a8832f829995",
        archiveSha256: "46547a5c48d3783d2c061cbbed20b30127f64766d56ad2cc92afd11996adcbca" },
    ],
    orderedMeasurementState: "independent-replay-passed",
    teams: [
      { name: "Live HTTP", source: "433e21354ca7e9b2f97b3e96caf39070d79c711a",
        state: "both-cohorts-independently-replayed",
        detail: "Both context8192 correctness qualifications and both 42-request HTTP runs pass independent replay. Each mode has thirty measured requests after ten warmups; this is finite C1 evidence, not sustained serving or a competitive win." },
      { name: "Visible-token attention", source: "4e287b1ed0058cca95908b0a977748d9ef8b56ee",
        tree: "c5c5a59c35a55b2a997937e73b8ce50ae05cd4f7", state: "stopped-after-active-bound-audit", label: "Stopped",
        hostAggregateSha256: "5ee934f406d1ab9091d086692507aa676f8ceb3e2f81b2bda7a4bcd59ab810bd",
        hostArchiveSha256: "036b0a70ab709bafae8c5ebdd50c3c7ee0db2967a00af540c0143b92eb135fce",
        failedEmissionArchiveSha256: "7110f5f6335bf10a25fd64c44d6e2e0d5a7b4e4d979feb171927f08cacbcf286",
        failureNoteSha256: "be0475c99e473d18e3a688e4261cf9c68210aad4ce4d7d557dc5e917c674f81f",
        stopAuditSha256: "22c88d1e3ac07a0213531d6344faad29007e7a9e3b13e06d26a05ee3473f9c33",
        detail: "Stopped for the current C1 workload. Source-fresh R3 completed 28 scoped host steps with 13/13 control and 19/19 candidate tests; Clippy commands exited 101 and matched known debt, not a strict Clippy pass. Emission then failed the unique-header-exit check before producing a candidate image. The driver already bounds attention by the maximum active position plus one: single-row decode has zero masked tail iterations. No GPU run or gain is admitted; prior failures remain retained." },
      { name: "Cooperative KV append", source: "8b576464c2dcd5d5dd18d4f07e64c9fc5c30e7b2",
        tree: "4a148bb9953ce172a31881a794f8244fb3556eb7", state: "guarded-compiler-diagnostic-failed", label: "Compiler rejected",
        diagnosticPlanSha256: "54791c01a6d3f800cee26d2b013565946f415d197342ee9696e8c88922522548",
        failureNoteSha256: "fdfa34218bd846dfd686a89710921578a341f53e18bf5ffe8cae9e9fb11dec4d",
        failedArchiveSha256: "d4303bf63950a265cbcabcfa190e9045b684508d8ef9d7d045b75481e2bcf9a4",
        failureCustodyAdmitted: true, diagnosticPassed: false, imageAdmitted: false,
        previousSource: "d03ccd5436fe99d9dc7f761964b0fd978db3cd0f",
        previousHostArchiveSha256: "a8a1e2eed30d709aaae634aa0f8d8b03b4e129d4caea323183027401be935c35",
        previousEmissionArchiveSha256: "73b5094d3cf174ff9ca3aa92f614b7f66b5bf4f95005f57e4f989c99f34f10ea",
        rejectedSource: "27163716949cd596523ef769d0b9673c2298f963",
        rejectedArchiveSha256: "f55e3ae0ad157d58e98821fed0f1eabd0b979ff92c3b8031672cbf682e0a584d",
        rejectedSgprSpills: 278, baselineSgprSpills: 0,
        detail: "Guarded source 8b57646 failed FE2O3-PROGRESS-002 before target IR or an image; its diagnostic reporter did not run. Failure custody is accepted, not compiler or image admission. Earlier d03ccd5 passed corrected source-fresh host checks but failed compiler proof before producing an image. Its earlier host failure with 18 tests where 19 were expected after stale binary reuse remains retained. Candidate 2716371 also remains rejected: 278 SGPR spills versus zero in its baseline. No candidate GPU launch or performance gain is admitted." },
      { name: "Ordered residual tail", source: "2396a82ff42c94654261a2ee8313f881a77aee1d",
        tree: "b9c74a4fda441f0df381d7f1e04525ddcd7a5da4", state: "native-correctness-admitted", label: "Native correctness accepted",
        integratedSource: "212ec854f06082241f3caa0c06b58a44959cc303", integratedTree: "bd20771b280760a318d0308c086dd7d9cf133896",
        hostNoteSha256: "b39923a4b6a39fa20bd81751f1af8e9506f9bf9a5f1e1443db2ffa7b94bd43bd",
        hostArchiveSha256: "1bb70bbd294d740237c22fc13d256bc239b2dca1898e01f957b966e07bc1c70f",
        commandsPassed: 31, adapterPasses: 780, adapterIgnored: 41, adapterResultRows: 19,
        doctestsPassed: 8, httpFixturePasses: 74, sourceGatePasses: 38, protectedPolicyPasses: 31,
        strictClippyExit: 0, archiveAdmitted: true, nativeAdmitted: true, performanceGainClaimed: false,
        nativeNoteSha256: "f84e23c23c19b27c25e1290f28f5a502229d7477a55edbe94b7d4941fdac7af2",
        nativeArchiveSha256: "b53c4d08c3044aed9d72ed0340941b21d2b64dcc30bcb303e42929b77c707756",
        nativeArchiveBytes: 268845, nativeCasesPassed: 4, nativeContext: 256,
        nativeOutputLengths: [8, 128], nativeModes: ["synchronous", "ordered"], nativeTimingsExcluded: true,
        detail: "All 31 CPU commands passed: 780 adapter test invocations, 41 ignored across 19 result rows, 8 doctests, 74 HTTP fixtures, 38 source-gate and 31 protected-policy tests; strict Clippy exited zero. Private integration 212ec85 preserves the tested code. Four native TP1/context256 cases now pass: synchronous and ordered modes with 8 or 128 outputs, exact token IDs/UTF-8, packet counts, retirement and unforced close. All eight GPUs were idle before and after each case. These are fixed-reference correctness checks, not HTTP or performance qualification; their timings are excluded. Existing measurements remain unchanged; no GPU gain or default change is claimed." },
    ],
    overview: "New opt-in wave-attention / wave-v11 HTTP measurements use the actual 8,192-token context. Synchronous / ordered mean TPOT is 276.255 / 192.127 ms, with output rates of 3.318800 / 4.694997 tokens/s. Each is one independently replayed finite concurrency-one cohort, not stable or production-serving performance. The earlier vLLM reference remains much faster; SGLang numerical failures stay excluded. Speculative HTTP serving, default promotion and M1 completion remain open.",
    scope: "Separate live HTTP remeasurement: Qwen3-8B, one MI350X GPU, TP1, context 8192, 128 input / 128 output tokens, concurrency 1. Fixed wave attention, MFMA projection, FP32-v8 head and wave-v11 argmax; speculation and prefix caching off. Each admitted mode has one fresh start, ten excluded warmups, thirty measured requests and two excluded diagnostics.",
    measurement: "The unchanged client measures send-to-first-text TTFT and first-to-last-text TPOT divided by 127. Output rate is 3,840 measured tokens divided by the first sample-window start through the last sample-window end, including gaps and drain. These are HTTP host clocks, not GPU duration; host and runtime profiling are off.",
    correctness: "Both modes first pass two full128 correctness requests at context8192, not a context256 proxy. Every admitted measured cohort then matches both diagnostic token-ID/UTF-8 oracles, all forty timed HTTP request IDs and final token arrays, exact usage, normal draining/stopped/Closed, no cleanup signals, and all-eight-GPU idle checks. Qualification and warmup timings are excluded.",
    imageScope: "The controller dependency and v11 argmax image use compiler source 8efd4fd. The v5 and FP32-v8 images remain frozen artifacts with their original provenance; they are not rebuilt or relabeled. The actual runtime worker remains c110ac55 / b91ddef7. No kernel or default changes are introduced by this HTTP profile.",
    interpretation: "These are descriptive single-start finite C1 cohorts, not repeated independent starts, stable tails, confidence intervals or sustained-load qualification. The earlier Ferric/vLLM cohort below keeps its original source and numbers; vLLM was not rerun alongside this new profile. No SGLang timing is admitted. No historical/native gains are added or pooled, and no competitive win is claimed.",
    oldCohortsPreserved: true, context256Proxy: false, warmupsPooled: false,
    gpuDurationClaim: false, confidenceIntervalClaimed: false, stableGainClaimed: false,
    servingQualified: false, frameworkWinClaimed: false, defaultPromotion: false,
    newVerusProof: false, m1Complete: false, authority: "none",
  };

const expected = {
  schema: "FerricPagesSubmissionCheckpointV1",
  source: "7cb6522430b6990e1d3f12bc15958865358459f4",
  tree: "2bfbe7b0ce3a6912e2909f31db11d873f422c14e",
  controllerSha256: "0da9c59a9172d809dd5109ec30681a847c5f83e7da2c396a943f51d99235c075",
  planSha256: "a13346d5587c5a6308c9790511d712a8d687fddbf03720cb5ea166be698a786b",
  host: {
    noteSha256: "66001b37ea6ce8fbf739625cd43744671ae888b85bc3f57fd81bbfca0e5e13d1",
    archiveSha256: "6aadf1c013f96760d658d8130466f7694d2b918ba659e36ab35cbcf09166f92d",
    sourceLedgerSha256: "9ec12a68c85d3fbea892340ff2be4cff743a1cb82d4d8e9b42da18826f5454d8",
    commandsPassed: 51, attempts: 51, testInvocationsPassed: 745, testInvocationsIgnored: 38,
    doctests: 8, adapterPolicies: 27, sourceGateTests: 38, protectedPolicies: 31,
    unchangedInventories: 5, custodyLedgers: 104, sourceFiles: 1073, warmTarget: true,
    sourceUnchanged: true, strictClippyPassed: true,
    checkerMethods: 26, checkerArchiveSha256: "7acf112b8922a0199d37b08a73182eecc3d81421d30893700272884cee6cd5ca",
    reducerMethods: 18, reducerArchiveSha256: "9dacc9c055dc767ef0f99897483fa2fd9a984fe5323296be1899918c4cf99970",
  },
  qualification: {
    noteSha256: "a18da51f81a76e2ff90979a01badc3ef3eef2ccda238c59523d98ff2b1c4feda",
    archiveSha256: "33da7d5e241e5d23d8a4be4ea2cc279b949bb4c34a7ad8b1bf89b868727fc6c1",
    rosterSha256: "bdb463cd800c1f9075603d41c0a712dfebe2dab8508c2ea04cf7231196316cbc",
    sizesSha256: "4ca3014ebd79c39c22de2db628865781945ba733855c4e227756195027a05b67",
    checkerSha256: "a05aaf85a6474dc8adf6a2a86374ad2a56252f0df4f4eb665f754a862b42e468",
    wrapperSha256: "7f9acd31c3b821b8c5bcfa6c567d8e605dd2ea642ede77916ec04f3f2012153b",
    referenceSha256: "cd7f512537e44d677244c606cba919e57747e8c26caf3c0df4a0aca7acc0eb8b",
    prefixReferenceSha256: "e5588cefc924a8be5b77345876b6893d53436213de03108a7fc4702cc86356a0",
    cases: [
      ["wave-submission-7cb6522-synchronous-8-r1", "synchronous", 8, 9219, 15, 135],
      ["wave-submission-7cb6522-ordered-8-r1", "ordered", 8, 9219, 15, 135],
      ["wave-submission-7cb6522-synchronous-128-r1", "synchronous", 128, 83139, 135, 255],
      ["wave-submission-7cb6522-ordered-128-r1", "ordered", 128, 83139, 135, 255],
    ],
    rawFiles: 40, inputTokens: 128, tp: 1, context: 256, pages: 16, capacity: 32, prefillChunk: 16,
    attention: "wave", argmax: "wave-v11", prefixCaching: false, speculation: false,
    exactIdsAndUtf8: true, retirementPassed: true, normalWorkerCloses: true,
    allEightIdleBeforeAndAfter: true, comparisonSamples: false,
  },
  abba: {
    summarySha256: "2de0c6a0fd2520f77f53c570959f653b55caa71e8a87ab94d937f565febe2ace",
    manifestSha256: "be4495ac63eaa8b05e2321a39232f3dcd49b12966c7ba9dce01064bd6b86c505",
    replaySha256: "2dd3d91a35632adddbc50dbe21196a6c6b59f86d3d507914df1d82fbf30bc29e",
    auditSha256: "3c7d4feeb742a73505668c319c2489b47b203b61b90f3367d180d29a91021a03",
    reducerSha256: "f0f08a9f621620fe5006c7b941fbe35641f1d1c99a7dd372544b952ccca9533f",
    rawArchiveSha256: "af4751cbc88b626b7a8a9c200ae055069ea45117d2276d198bf54df4b5fca716",
    replayArchiveSha256: "b2197021e895393c8e6b5b9abe847d29cd640ad420378d70fc7ee5cd5bc8b43d",
    runs: ["synchronous-a1", "ordered-b1", "ordered-b2", "synchronous-a2"],
    repetitionsPerMode: 2, rawFiles: 40, outputs: 128, packets: 83139, batches: 135, cursor: 255,
    synchronous: { ttftSeconds: 3.1265115569999997, tpotSeconds: 0.23055936558267717,
      outputTokensPerSecond: 3.9498842931270257, workloadSeconds: 32.407551031,
      tpotRangeSeconds: [0.22896383051181105, 0.2321549006535433],
      outputRateRange: [3.9226694139363847, 3.9770991723176667] },
    ordered: { ttftSeconds: 2.848069924, tpotSeconds: 0.19492733181496064,
      outputTokensPerSecond: 4.643287102223024, workloadSeconds: 27.603841144500002,
      tpotRangeSeconds: [0.18728891182677168, 0.20256575180314962],
      outputRateRange: [4.472915377847863, 4.813658826598184] },
    ttftReductionPercent: 8.905824524351825, tpotReductionPercent: 15.454602625951065,
    meanOutputRateGainPercent: 17.555015733057044,
    pairedTpotReductionPercent: [18.20152929477199, 12.745433659637062],
    pairedOutputRateGainPercent: [21.034417751091915, 14.027334599152685],
    clock: "controller-std-instant", outputRateAggregation: "arithmetic-mean-of-per-run-rates",
    comparisonAggregation: "ratio-of-arithmetic-means", qualificationTimingsIncluded: false,
    oldCohortMixed: false, additiveGainClaim: false, operationGroupComparison: false,
  },
  state: "host-native-and-descriptive-abba-passed",
  descriptiveComparisonAdmitted: true, stablePerformanceGainClaimed: false,
  operationGroupComparison: false, authority: "none", httpMeasurement: false,
  gpuClockMeasurement: false, confidenceQualified: false, competitiveRanking: false,
  defaultPromotion: false, newVerusProof: false, m1Complete: false,
};

export function validateSubmissionCheckpoint(value) {
  assert.deepEqual(JSON.parse(JSON.stringify(value)), expected);
}

export function testSubmissionCheckpointRejections(value) {
  let mutations = 0;
  function visit(node, path = []) {
    for (const [key, child] of Object.entries(node)) {
      const next = [...path, key];
      if (child !== null && typeof child === "object") visit(child, next);
      else {
        const changed = JSON.parse(JSON.stringify(value));
        let parent = changed;
        for (const part of next.slice(0, -1)) parent = parent[part];
        parent[key] = typeof child === "boolean" ? !child : "mutated";
        assert.throws(() => validateSubmissionCheckpoint(changed));
        mutations++;
      }
    }
  }
  visit(value);
  for (const path of [[], ["host"], ["qualification"]]) {
    const changed = JSON.parse(JSON.stringify(value));
    let node = changed;
    for (const key of path) node = node[key];
    node.unreviewed = true;
    assert.throws(() => validateSubmissionCheckpoint(changed));
  }
  const changed = JSON.parse(JSON.stringify(value));
  changed.qualification.cases.reverse();
  assert.throws(() => validateSubmissionCheckpoint(changed));
  console.log(`PASS: submission checkpoint rejects ${mutations} scalar mutations and closed-roster changes.`);
}

export async function validateSubmissionEvidence(root, value) {
  validateSubmissionCheckpoint(value);
  async function pinned(file, digest, parse = true) {
    const raw = await readFile(join(root, file));
    assert(raw.length > 0 && raw.length <= 8 * 1024 * 1024, "bounded evidence file");
    assert.equal(createHash("sha256").update(raw).digest("hex"), digest, file);
    return parse ? JSON.parse(raw) : raw;
  }
  const host = (await pinned("HOST-GATE.md", value.host.noteSha256, false)).toString("utf8");
  for (const fact of [value.source, value.tree, value.controllerSha256, value.host.sourceLedgerSha256,
    value.host.archiveSha256, "All 51 actual command exits", "745 passed invocations,38 ignored,18 result rows",
    "doctests8, source-gate38", "source policy31", "All104", "contains1,073 files",
    "warm", "All five generated inventories matched", "without inline fixes or retries"])
    assert(host.includes(fact), fact);
  const log = (await pinned("host-gate-run-r1.log",
    "b4b971ac5bf2d2ff3da9674247af0a349d99b0df329fc166e5937a9cd852969a", false)).toString("utf8");
  assert.equal([...log.matchAll(/^[a-z0-9-]+: exit=0; stage=\d+ KiB$/gm)].length, value.host.commandsPassed);
  await pinned("PLAN.md", value.planSha256, false);
  const q = value.qualification;
  await pinned("check_wave_argmax_submission.py", q.checkerSha256, false);
  await pinned("run_wave_argmax_submission.py", q.wrapperSha256, false);
  const note = (await pinned("qualification/QUALIFICATION.md", q.noteSha256, false)).toString("utf8").replace(/\s+/g, " ");
  for (const fact of [value.source, value.tree, value.controllerSha256, value.planSha256,
    q.archiveSha256, q.rosterSha256, q.sizesSha256, "All four predeclared correctness cases passed",
    "normal unforced", "timings are excluded", "No comparison", "all40 raw files"])
    assert(note.includes(fact), fact);
  const rosterBytes = await pinned("qualification/qualification-local.sha256", q.rosterSha256, false);
  assert.deepEqual(await pinned("qualification/qualification-remote.sha256", q.rosterSha256, false), rosterBytes);
  const sizeBytes = await pinned("qualification/qualification-local.sizes", q.sizesSha256, false);
  assert.deepEqual(await pinned("qualification/qualification-remote.sizes", q.sizesSha256, false), sizeBytes);
  const roster = rosterBytes.toString("utf8").trim().split("\n").map((line) => {
    const match = /^([a-f0-9]{64})  (\S+)$/.exec(line);
    assert(match, "closed hash roster line");
    return [match[2], match[1]];
  });
  const sizes = sizeBytes.toString("utf8").trim().split("\n").map((line) => {
    const match = /^([1-9][0-9]*) (\S+)$/.exec(line);
    assert(match, "closed size roster line");
    return [match[2], Number(match[1])];
  });
  const names = ["results.jsonl", "host-timing.json", "wrapper-result.json", "prelaunch.json",
    "owned-process.json", "group-cleanup.json", "wall-clock.json", "resources-before.json", "gpu-before.json", "gpu-after.json"];
  const expectedNames = q.cases.flatMap(([id]) => names.map((name) => `${id}/${name}`));
  assert.equal(expectedNames.length, q.rawFiles);
  assert.deepEqual(roster.map(([name]) => name), expectedNames);
  assert.deepEqual(sizes.map(([name]) => name), expectedNames);
  const raw = new Map();
  for (const [index, [name, digest]] of roster.entries()) {
    const bytes = await pinned(`qualification/${name}`, digest, false);
    assert.equal(bytes.length, sizes[index][1]);
    raw.set(name, bytes);
  }
  const reference = await pinned("target-reference.json", q.referenceSha256);
  const prefix = await pinned("prefix-reference.json", q.prefixReferenceSha256);
  assert.equal(reference.schema, "FerricMatched128ReferenceV1");
  assert.equal(prefix.source_reference_sha256, q.referenceSha256);
  assert.deepEqual(prefix.prompt_token_ids, reference.prompt_token_ids);
  const devices = ["0xe3233d81d822f3eb", "0x966895650c2e8ae1", "0xd4e5658294b0b967", "0xf695011eb05a2497",
    "0x33231df6bb92857", "0x80a9a2ba09978a65", "0x92050148915dd40c", "0x6ad88437269ef781"];
  for (const [id, mode, outputs, packets, batches, cursor] of q.cases) {
    const json = (name) => JSON.parse(raw.get(`${id}/${name}`));
    const text = raw.get(`${id}/results.jsonl`).toString("utf8");
    assert(text.endsWith("\n"));
    const records = text.trim().split("\n").map((line) => JSON.parse(line));
    assert.equal(records.length, 4);
    const [setup, , observation, closed] = records;
    const wrapper = json("wrapper-result.json");
    const checked = wrapper.checked_trace;
    const prelaunch = json("prelaunch.json");
    const timing = json("host-timing.json");
    for (const [record, suffix] of [[setup, "Setup"], [observation, "Observation"], [closed, "Closed"], [checked, "Checked"]])
      assert.equal(record.schema, `FerricWaveArgmaxSubmissionCanary${suffix}V1`);
    assert.equal(wrapper.schema, "FerricWaveArgmaxSubmissionModelWrapperV1");
    assert.equal(prelaunch.schema, "FerricWaveArgmaxSubmissionModelPrelaunchV1");
    assert.equal(prelaunch.controller_source, value.source);
    assert.equal(prelaunch.pins[prelaunch.argv[0]], value.controllerSha256);
    for (const [file, digest] of [["check_wave_argmax_submission.py", q.checkerSha256], ["run_wave_argmax_submission.py", q.wrapperSha256]])
      assert.equal(prelaunch.pins[`/tmp/ferric-compete-gpu.VabkOGCx/wave-argmax-submission-wrapper-v1/${file}`], digest);
    for (const [flag, expectedValue] of [["--submission", mode], ["--attention", q.attention],
      ["--argmax-mode", q.argmax], ["--max-new-tokens", String(outputs)]])
      assert.equal(prelaunch.argv[prelaunch.argv.indexOf(flag) + 1], expectedValue);
    assert.deepEqual(setup.prompt_token_ids, reference.prompt_token_ids);
    assert.equal(setup.prompt_token_ids.length, q.inputTokens);
    for (const [field, expectedValue] of Object.entries({ controller_sha256: value.controllerSha256,
      context: q.context, pages: q.pages, row_capacity: q.capacity, prefill_chunk: q.prefillChunk,
      attention: q.attention, argmax_mode: q.argmax, max_new_tokens: outputs,
      expected_packets: packets, expected_batches: batches, runtime_ordered_batches: mode === "ordered",
      prefix_cache: false, source_reference_sha256: q.referenceSha256, prefix_reference_sha256: q.prefixReferenceSha256 }))
      assert.equal(setup[field], expectedValue, `${id}: ${field}`);
    for (const [role, sha] of [["target_artifact", "98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502"],
      ["target_head_artifact", "5f19b3ba59035a5f0ebc90cdf3a40466f9910908a45e082d146cb674028da6cb"],
      ["argmax_artifact", "de9db78c0ef7ad5d84fc903d41ee9db59026113d79e23f12d3909761363b9390"]])
      assert.equal(setup[role].hsaco, sha);
    for (const result of [observation, checked]) {
      assert.equal(result.reference_passed, true);
      assert.deepEqual(result.generated_token_ids, reference.generated_token_ids.slice(0, outputs));
      assert.equal(result.generated_utf8_hex, outputs === 128 ? reference.generated_utf8_hex : prefix.prefix_utf8_hex[7]);
      assert.equal(result.completed_packets, packets);
      assert.equal(result.completed_batches, batches);
      assert.equal(result.committed_inputs_before_retirement, cursor);
    }
    assert.equal(observation.pool_retired, true);
    assert.equal(checked.controller_sha256, value.controllerSha256);
    assert.equal(checked.attention, q.attention);
    assert.equal(checked.argmax_mode, q.argmax);
    assert.equal(checked.submission, mode);
    assert.equal(checked.submission_host.attention_groups_measure,
      mode === "ordered" ? "command-preparation-only" : "submit-and-wait");
    for (const record of [setup, observation, closed, checked, wrapper, prelaunch]) {
      assert.equal(record.authority, "none");
      assert.equal(record.performance_qualified, false);
    }
    for (const flag of ["speculative_serving", "http_measurement", "gpu_clock_measurement"]) assert.equal(checked[flag], false);
    assert.equal(wrapper.passed, true);
    assert.equal(wrapper.serving_qualified, false);
    assert.equal(wrapper.submission, mode);
    assert.equal(wrapper.attention, q.attention);
    assert.equal(wrapper.max_new_tokens, outputs);
    assert.deepEqual(wrapper.errors, []);
    assert.equal(wrapper.all_eight_idle_before, true);
    assert.equal(wrapper.all_eight_idle_after, true);
    assert.equal(closed.execution_completed, true);
    assert.equal(closed.reference_passed, true);
    assert.equal(closed.worker_exited, true);
    assert.equal(closed.completed_batches, batches);
    assert.deepEqual(closed.completed_packets, [packets]);
    assert(Number.isSafeInteger(setup.worker_pid) && setup.worker_pid > 0);
    assert.equal(closed.worker_pid, setup.worker_pid);
    assert.equal(checked.worker_pid, setup.worker_pid);
    assert.deepEqual(timing.setup, setup);
    assert.deepEqual(timing.closed, closed);
    const process = json("owned-process.json");
    assert.equal(process.pid, timing.controller_pid);
    assert.equal(process.pgid, process.pid);
    assert(Number.isSafeInteger(process.pid) && process.pid > 0);
    assert.deepEqual(json("group-cleanup.json"), { absent: true, cleanup_error: null,
      controller_returncode: 0, forced: false, reason: null });
    for (const name of ["gpu-before.json", "gpu-after.json"]) {
      const cards = json(name);
      assert.deepEqual(Object.keys(cards).sort(), devices.map((_, index) => `card${index}`));
      devices.forEach((device, index) => {
        const card = cards[`card${index}`];
        assert.equal(card["Unique ID"], device);
        for (const field of ["GPU use (%)", "GPU Memory Allocated (VRAM%)", "GPU Memory Read/Write Activity (%)"])
          assert.equal(card[field], "0");
      });
    }
  }
  const a = value.abba;
  const summary = await pinned("abba/summary.json", a.summarySha256);
  const manifest = await pinned("abba/manifest.json", a.manifestSha256);
  const replay = await pinned("abba/replay.json", a.replaySha256);
  const before = await pinned("abba/before.json", a.auditSha256, false);
  assert.deepEqual(await pinned("abba/after.json", a.auditSha256, false), before);
  await pinned("abba/summarize_wave_submission.py", a.reducerSha256, false);
  assert.equal(summary.schema, "FerricWaveSubmissionAbbaSummaryV1");
  assert.equal(manifest.schema, "FerricWaveSubmissionAbbaEvidenceV1");
  assert.equal(replay.schema, "FerricWaveSubmissionReplayGateV1");
  assert.equal(replay.passed, true);
  assert.deepEqual(replay.errors, []);
  assert.deepEqual(replay.commands.map((command) => command.exit_code), [0, 1]);
  assert.equal(summary.evidence_manifest_sha256, a.manifestSha256);
  assert.equal(summary.experiment_plan_sha256, value.planSha256);
  assert.equal(summary.reducer_sha256, a.reducerSha256);
  assert.equal(summary.checker_sha256, q.checkerSha256);
  assert.equal(summary.wrapper_sha256, q.wrapperSha256);
  assert.deepEqual(summary.controller, manifest.controller);
  assert.equal(summary.controller.source, value.source);
  assert.equal(summary.controller.tree, value.tree);
  assert.equal(summary.controller.sha256, value.controllerSha256);
  for (const item of [summary, manifest, replay]) {
    assert.equal(item.authority, "none");
    assert.equal(item.performance_qualified, false);
  }
  for (const flag of ["serving_qualified", "http_measurement", "gpu_clock_measurement", "confidence_qualified",
    "competitive_ranking", "default_promotion", "m1_completion", "new_verus_proof", "old_cohort_mixed",
    "additive_gain_claim", "stable_performance_gain_claimed", "operation_group_cross_mode_comparison"])
    assert.equal(summary[flag], false);
  assert.deepEqual(summary.runs.map((run) => run.id), a.runs);
  assert.deepEqual(manifest.runs.map((run) => run.id), a.runs);
  assert(summary.stage_accounting.includes("no cross-mode group speed ratios"));
  const close = (actual, expectedValue) => {
    assert(Number.isFinite(actual) && Number.isFinite(expectedValue));
    assert(Math.abs(actual - expectedValue) <= 1e-10 * Math.max(1, Math.abs(expectedValue)),
      `arithmetic mismatch: ${actual} versus ${expectedValue}`);
  };
  const metrics = { ttftSeconds: "diagnostic_ttft_seconds", tpotSeconds: "diagnostic_tpot_seconds",
    workloadSeconds: "diagnostic_workload_seconds", outputTokensPerSecond: "diagnostic_output_tokens_per_second" };
  const perRun = [], sessions = new Set();
  let previousEnd = 0;
  for (const [index, item] of summary.runs.entries()) {
    const mode = ["synchronous", "ordered", "ordered", "synchronous"][index];
    const suffix = ["a1", "b1", "b2", "a2"][index];
    const directory = `wave-submission-7cb6522-${mode}-128-${suffix}`;
    const entry = manifest.runs[index];
    assert.equal(entry.directory, directory);
    assert.deepEqual(Object.keys(entry.files).sort(), [...names].sort());
    assert.deepEqual(item.evidence_files, entry.files);
    const bytes = new Map();
    for (const name of names) {
      const pin = entry.files[name];
      assert.deepEqual(Object.keys(pin).sort(), ["bytes", "sha256"]);
      const file = await pinned(`abba/${directory}/${name}`, pin.sha256, false);
      assert.equal(file.length, pin.bytes);
      bytes.set(name, file);
    }
    const records = bytes.get("results.jsonl").toString("utf8").trim().split("\n").map((line) => JSON.parse(line));
    assert.equal(records.length, 4);
    const [setup, , observation, closed] = records;
    const wrapper = JSON.parse(bytes.get("wrapper-result.json"));
    const checked = item.checked_trace;
    assert.deepEqual(checked, wrapper.checked_trace);
    assert.equal(item.submission, mode);
    assert.equal(item.outputs, a.outputs);
    assert.equal(item.repetition, index < 2 ? 1 : 2);
    assert.equal(item.attention, q.attention);
    assert.equal(item.argmax_mode, q.argmax);
    assert.equal(setup.controller_sha256, value.controllerSha256);
    assert.equal(setup.runtime_ordered_batches, mode === "ordered");
    assert.equal(setup.attention, q.attention);
    assert.equal(setup.argmax_mode, q.argmax);
    assert.equal(setup.context, q.context);
    assert.deepEqual(setup.prompt_token_ids, reference.prompt_token_ids);
    for (const result of [observation, checked]) {
      assert.equal(result.reference_passed, true);
      assert.deepEqual(result.generated_token_ids, reference.generated_token_ids);
      assert.equal(result.generated_utf8_hex, reference.generated_utf8_hex);
      assert.equal(result.completed_packets, a.packets);
      assert.equal(result.completed_batches, a.batches);
      assert.equal(result.committed_inputs_before_retirement, a.cursor);
    }
    assert.equal(observation.pool_retired, true);
    assert.equal(closed.worker_exited, true);
    assert.equal(closed.worker_pid, checked.worker_pid);
    assert.equal(wrapper.passed, true);
    assert.deepEqual(wrapper.errors, []);
    assert.equal(wrapper.all_eight_idle_before, true);
    assert.equal(wrapper.all_eight_idle_after, true);
    assert.equal(checked.submission_host.attention_groups_measure,
      mode === "ordered" ? "command-preparation-only" : "submit-and-wait");
    assert.deepEqual(JSON.parse(bytes.get("group-cleanup.json")), { absent: true, cleanup_error: null,
      controller_returncode: 0, forced: false, reason: null });
    const offsets = observation.elapsed_seconds;
    assert.equal(offsets.length, a.outputs);
    offsets.forEach((offset, i) => assert(Number.isFinite(offset) && offset > (i ? offsets[i - 1] : 0)));
    const derived = { ttftSeconds: offsets[0], tpotSeconds: (offsets.at(-1) - offsets[0]) / (a.outputs - 1),
      workloadSeconds: observation.workload_seconds, outputTokensPerSecond: a.outputs / observation.workload_seconds };
    assert(Number.isFinite(derived.workloadSeconds) && derived.workloadSeconds >= offsets.at(-1));
    for (const [name, metric] of Object.entries(metrics)) close(checked[metric], derived[name]);
    assert.equal(item.session, setup.session);
    assert(!sessions.has(item.session));
    sessions.add(item.session);
    const wall = JSON.parse(bytes.get("wall-clock.json"));
    assert.deepEqual(item.child_wall, wall);
    // Python replay and pinned bytes retain exact u64/ns custody; JS checks coarse chronology only.
    assert(wall.started_unix_ns > previousEnd && wall.completed_unix_ns > wall.started_unix_ns);
    previousEnd = wall.completed_unix_ns;
    perRun.push({ mode, ...derived });
  }
  const cohort = summary.cohort;
  assert.equal(cohort.repetitions_per_submission, a.repetitionsPerMode);
  assert.equal(cohort.inference, "descriptive n=2 only; no confidence interval or statistical qualification");
  assert.deepEqual(Object.keys(cohort.ratio_of_arithmetic_means).sort(), Object.values(metrics).sort());
  assert(!Object.hasOwn(cohort, "stage_ratios"));
  for (const mode of ["synchronous", "ordered"]) {
    for (const [field, metric] of Object.entries(metrics)) {
      const values = perRun.filter((run) => run.mode === mode).map((run) => run[field]);
      const mean = values.reduce((total, x) => total + x, 0) / values.length;
      close(a[mode][field], mean);
      close(cohort.arithmetic_metric_means[mode][metric], mean);
      const range = cohort.within_submission_variability[mode][metric];
      assert.equal(range.n, 2);
      range.values.forEach((x, i) => close(x, values[i]));
      close(range.minimum, Math.min(...values));
      close(range.maximum, Math.max(...values));
      if (field === "tpotSeconds" || field === "outputTokensPerSecond") {
        const displayed = a[mode][field === "tpotSeconds" ? "tpotRangeSeconds" : "outputRateRange"];
        close(displayed[0], Math.min(...values));
        close(displayed[1], Math.max(...values));
      }
    }
  }
  for (const [field, metric] of Object.entries(metrics)) {
    const synchronous = a.synchronous[field], ordered = a.ordered[field];
    const higher = field === "outputTokensPerSecond";
    const ratio = higher ? ordered / synchronous : synchronous / ordered;
    const gain = 100 * (higher ? ordered / synchronous - 1 : 1 - ordered / synchronous);
    close(cohort.ratio_of_arithmetic_means[metric].speed_ratio, ratio);
    close(cohort.ratio_of_arithmetic_means[metric].improvement_percent, gain);
    const displayed = { ttftSeconds: "ttftReductionPercent", tpotSeconds: "tpotReductionPercent",
      outputTokensPerSecond: "meanOutputRateGainPercent" }[field];
    if (displayed) close(a[displayed], gain);
    for (const [pairIndex, [syncIndex, orderedIndex]] of [[0, 1], [3, 2]].entries()) {
      const sync = perRun[syncIndex][field], orderedValue = perRun[orderedIndex][field];
      const pair = cohort.repetition_pairs[pairIndex];
      assert.equal(pair.synchronous_id, a.runs[syncIndex]);
      assert.equal(pair.ordered_id, a.runs[orderedIndex]);
      close(pair.relative_changes[metric].speed_ratio, higher ? orderedValue / sync : sync / orderedValue);
      const pairedGain = 100 * (higher ? orderedValue / sync - 1 : 1 - orderedValue / sync);
      close(pair.relative_changes[metric].improvement_percent, pairedGain);
      if (field === "tpotSeconds") close(a.pairedTpotReductionPercent[pairIndex], pairedGain);
      if (higher) close(a.pairedOutputRateGainPercent[pairIndex], pairedGain);
    }
  }
  console.log("PASS: exact submission host gate, four native correctness cases and separate forty-file full128 ABBA means/ranges/pairs; no cross-mode group comparison or stable/HTTP/GPU claim.");
}

export function validateLiveHttpCheckpoint(value) {
  assert.deepEqual(JSON.parse(JSON.stringify(value)), liveHttpExpected);
}

export function testLiveHttpCheckpointRejections(value) {
  let mutations = 0;
  function visit(node, path = []) {
    for (const [key, child] of Object.entries(node)) {
      const next = [...path, key];
      if (child !== null && typeof child === "object") visit(child, next);
      else {
        const changed = JSON.parse(JSON.stringify(value));
        let parent = changed;
        for (const part of next.slice(0, -1)) parent = parent[part];
        parent[key] = typeof child === "boolean" ? !child : "mutated";
        assert.throws(() => validateLiveHttpCheckpoint(changed));
        mutations++;
      }
    }
  }
  visit(value);
  for (const key of ["cohorts", "qualification", "teams"]) {
    const changed = JSON.parse(JSON.stringify(value));
    changed[key].push(changed[key][0]);
    assert.throws(() => validateLiveHttpCheckpoint(changed));
  }
  const changed = JSON.parse(JSON.stringify(value));
  changed.unreviewed = true;
  assert.throws(() => validateLiveHttpCheckpoint(changed));
  console.log(`PASS: live HTTP checkpoint rejects ${mutations} scalar mutations and added cohorts/claims.`);
}

export async function validateLiveHttpEvidence(root, value) {
  validateLiveHttpCheckpoint(value);
  async function pinned(file, digest, parse = true) {
    const bytes = await readFile(join(root, file));
    assert(bytes.length > 0 && bytes.length <= 16 * 1024 * 1024, "bounded live HTTP evidence");
    assert.equal(createHash("sha256").update(bytes).digest("hex"), digest, file);
    return parse ? JSON.parse(bytes) : bytes;
  }
  await pinned("source/wave_live_http_contract.py", value.contractSha256, false);
  await pinned("source/run_wave_live_http.py", value.driverSha256, false);
  await pinned("source/competitive_benchmark.py", value.clientSha256, false);
  const reference = await pinned("reference.json", value.referenceSha256);
  async function run(directory, row, count, purpose) {
    const receipt = await pinned(`${directory}/receipt.json`, row.receiptSha256);
    assert.equal(receipt.schema, "FerricWaveArgmaxLiveHttpReceiptV1");
    assert.equal(receipt.purpose, purpose);
    assert.equal(receipt.submission, row.mode);
    assert.equal(receipt.request_count, count);
    for (const key of ["passed", "cleanup_completed", "all_eight_idle_before_after"])
      assert.equal(receipt[key], true);
    for (const key of ["serving_qualified", "framework_win_claim", "gpu_duration_claim"])
      assert.equal(receipt[key], false);
    assert.equal(receipt.qualification_admitted, purpose === "qualification");
    assert.equal(receipt.timing_admitted, purpose === "matched");
    assert.deepEqual(receipt.errors, []);
    const raw = new Map();
    for (const [name, digest] of Object.entries(receipt.raw_files)) {
      assert(/^[a-z0-9][a-z0-9.-]+$/.test(name) && name !== "receipt.json", "direct raw file");
      raw.set(name, await pinned(`${directory}/${name}`, digest, false));
    }
    const record = (name) => JSON.parse(raw.get(name));
    const plan = record("frozen-plan.json");
    assert.equal(receipt.plan_sha256, receipt.raw_files["frozen-plan.json"]);
    assert.equal(plan.ferric.controller_source, value.source);
    assert.equal(plan.ferric.controller_tree, value.tree);
    assert.equal(plan.ferric.controller.sha256, value.controllerSha256);
    assert.equal(plan.ferric.worker_source, value.workerSource);
    assert.equal(plan.ferric.worker.sha256, value.workerSha256);
    assert.equal(plan.client.sha256, value.clientSha256);
    const setup = record("ferric-setup.json");
    for (const [key, expectedValue] of Object.entries({ context_tokens: 8192, physical_pages: 512,
      tensor_parallel: 1, kernel_row_capacity: 32, prefill_chunk: 16, prefix_cache: false,
      head_precision: "fp32-v8", argmax_mode: "wave-v11", submission: row.mode,
      runtime_ordered_batches: row.mode === "ordered", controller_sha256: value.controllerSha256 }))
      assert.equal(setup[key], expectedValue, key);
    assert.equal(setup.performance_profile.runtime_profiling, false);
    assert.equal(setup.performance_profile.attention, "wave");
    const events = record("ferric-final-events.json");
    assert.deepEqual(events[0], setup);
    assert(events.every((event) => event.authority === "none"));
    const finals = events.filter((event) => event.event === "request");
    assert.equal(finals.length, count);
    finals.forEach((final, index) => {
      assert.equal(final.request_id, index + 1);
      assert.deepEqual(final.generated_tokens, reference.generated_token_ids);
      assert.equal(Buffer.from(final.generated_utf8_bytes).toString("hex"), reference.generated_utf8_hex);
      assert.equal(final.state, "Completed");
    });
    assert.deepEqual(events.slice(-3).map((event) => event.event ?? null), ["draining", "stopped", null]);
    assert.equal(events.at(-3).reason, "shutdown");
    assert.equal(events.at(-2).reason, "drained");
    assert.equal(events.at(-2).batches, 135 * count);
    assert.deepEqual(events.at(-1).rank_dispatch_counts, [83139 * count]);
    const teardown = record("ferric-teardown.json");
    assert.equal(teardown.controller_status, 0);
    assert.equal(teardown.admissions, count);
    assert.equal(teardown.forced_group_cleanup, false);
    assert.deepEqual(teardown.backend_cleanup_signals, []);
    for (const key of ["closed_receipt", "group_absent", "worker_pids_absent", "threads_joined"])
      assert.equal(teardown[key], true);
    const idle = (snapshot) => {
      assert.deepEqual(Object.keys(snapshot).sort(), Array.from({ length: 8 }, (_, i) => `card${i}`));
      for (const card of Object.values(snapshot)) {
        for (const key of ["GPU use (%)", "GPU Memory Allocated (VRAM%)", "GPU Memory Read/Write Activity (%)"])
          assert.equal(card[key], "0");
      }
    };
    // Exact u64 GPU identities and clock replay are already bound by the pinned Python receipt.
    idle(record("gpu-preflight.stdout"));
    const settlement = record("gpu-postflight-settling.json");
    assert.equal(settlement.settled, true);
    assert(settlement.attempts.length >= 1 && settlement.attempts.length <= 31);
    assert.equal(settlement.attempts.at(-1).status, "idle");
    idle(record(`${settlement.attempts.at(-1).label}.stdout`));
    return { receipt, record };
  }
  for (const row of value.qualification) await run(`qualification-${row.mode}`, row, 2, "qualification");
  for (const row of value.cohorts) {
    const { receipt, record } = await run(row.mode, row, 42, "matched");
    assert.equal(Object.keys(receipt.raw_files).length, row.rawFiles);
    assert.equal(receipt.plan_sha256, row.planSha256);
    assert.equal(receipt.client_exit_status, 0);
    const summary = await pinned(`${row.mode}/summary.json`, row.replaySummarySha256);
    assert.equal(summary.receipt_sha256, row.receiptSha256);
    assert.equal(summary.plan_sha256, row.planSha256);
    assert.equal(summary.submission, row.mode);
    assert.equal(summary.raw_files_checked, row.rawFiles);
    assert.equal(summary.measured_requests, value.measuredRequests);
    assert.equal(summary.measured_output_tokens, value.measuredOutputTokens);
    assert.equal(summary.diagnostic_requests_checked, row.diagnosticRequestsChecked);
    assert.equal(summary.timed_http_request_identities_checked, row.timedRequestIdentitiesChecked);
    assert.equal(summary.timed_final_token_ids_checked, row.timedFinalTokenIdsChecked);
    assert.equal(summary.gpu_duration_claim, false);
    assert.equal(summary.framework_win_claim, false);
    assert.equal(summary.serving_qualified, false);
    const metrics = summary.metrics;
    assert.equal(metrics.requests, 30);
    assert.equal(metrics.successful_requests, 30);
    assert.equal(metrics.failed_requests, 0);
    assert.equal(metrics.all_requests_succeeded, true);
    for (const [field, expectedValue] of Object.entries({ mean: row.ttftMeanMs, p50: row.ttftP50Ms, p99: row.ttftP99Ms }))
      assert.equal(metrics.ttft_ms[field], expectedValue);
    for (const [field, expectedValue] of Object.entries({ mean: row.tpotMeanMs, p50: row.tpotP50Ms, p99: row.tpotP99Ms }))
      assert.equal(metrics.tpot_ms[field], expectedValue);
    assert.equal(metrics.e2e_ms.mean, row.e2eMeanMs);
    assert.equal(metrics.output_tokens_per_second, row.outputTokensPerSecond);
    assert.equal(metrics.window_seconds, row.windowSeconds);
    const timed = record("timed-raw.json");
    assert.equal(timed.warmups.length, 10);
    assert.equal(timed.samples.length, 30);
    assert.equal(summary.cohort_start_ns, timed.samples[0].started_ns);
    assert.equal(summary.cohort_end_ns, timed.samples.at(-1).completed_ns);
    assert.equal(value.measuredOutputTokens / row.windowSeconds, row.outputTokensPerSecond);
  }
  console.log("PASS: separate context8192 HTTP cohorts, source-bound replay metrics, full raw hashes, exact final tokens and normal closure; historical measurements unchanged.");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const context = { window: {} };
  vm.runInNewContext(await readFile(join(dirname(fileURLToPath(import.meta.url)), "data/project.js"), "utf8"), context);
  if (process.argv[2] === "--live-http") {
    const value = JSON.parse(JSON.stringify(context.window.FERRIC_PROJECT.liveHttpCheckpoint));
    validateLiveHttpCheckpoint(value);
    testLiveHttpCheckpointRejections(value);
    if (process.argv[3]) await validateLiveHttpEvidence(process.argv[3], value);
  } else {
    const value = JSON.parse(JSON.stringify(context.window.FERRIC_PROJECT.submissionCheckpoint));
    validateSubmissionCheckpoint(value);
    testSubmissionCheckpointRejections(value);
    if (process.argv[2]) await validateSubmissionEvidence(process.argv[2], value);
  }
}
