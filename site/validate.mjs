import { access, readFile } from "node:fs/promises";
import { constants } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const siteRoot = dirname(fileURLToPath(import.meta.url));
const dataSource = await readFile(join(siteRoot, "data/project.js"), "utf8");
const context = { window: {} };
vm.runInNewContext(dataSource, context, { filename: "site/data/project.js" });
const project = context.window.FERRIC_PROJECT;

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertExactKeys(value, keys, location) {
  assert(value && typeof value === "object", `${location} must be an object`);
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  assert(
    JSON.stringify(actual) === JSON.stringify(expected),
    `${location} keys drifted: ${actual.join(", ")}`,
  );
}

function assertCommit(value, location) {
  assert(
    typeof value === "string" && /^[0-9a-f]{40}$/.test(value),
    `${location} must be an exact lowercase 40-character commit`,
  );
}

function assertSha256(value, location) {
  assert(
    typeof value === "string" && /^[0-9a-f]{64}$/.test(value),
    `${location} must be an exact lowercase SHA-256 digest`,
  );
}

const states = new Set([
  "implemented",
  "integration",
  "observed",
  "verified",
  "qualified",
  "open",
]);

function assertState(value, location) {
  assert(states.has(value), `${location} has unknown state ${value}`);
}

assertExactKeys(
  project,
  [
    "updated",
    "repository",
    "fe2o3Repository",
    "current",
    "milestone",
    "readiness",
    "envelope",
    "capabilities",
    "validation",
    "teams",
    "boundaries",
    "latestObservation",
    "recentProgress",
    "evidence",
  ],
  "project",
);
assert(/^\d{4}-\d{2}-\d{2}$/.test(project.updated), "updated must use YYYY-MM-DD");
assert(project.repository === "https://github.com/harsh-nod/ferric", "Ferric repository drifted");
assert(project.fe2o3Repository === "https://github.com/harsh-nod/fe2o3", "fe2o3 repository drifted");

const expectedCurrent = {
  siteRefreshBase: "11e408970ddce28cacb491086d81852ddf07713f",
  integrationCommit: "ea6ef07c7b13d31c84b14d2ad06f19f8d1220665",
  integrationTree: "34a2bf774ecd9072d4116423a7d512fdda882345",
  residentTailCommit: "6fcf568014818b4c5664bea72206f5d07dca050b",
  residentReconcileCommit: "55285c95446f40984892e4857d417b6412c89534",
  residentFirstRoundCommit: "2880334d3a359b37394cec5528609cec90248a40",
  residentFirstRoundTree: "6609ce9375f4611a17fd7ff07c52adf18f33eb4c",
  residentCandidateIntegrated: false,
  residentCandidateReviewComplete: true,
  residentCandidateReviewDisposition: "hold",
  residentCandidatePhysicalRunnable: false,
  residentCandidateEngineTestsPassed: 878,
  residentCandidateHardwareIgnored: 7,
  residentCandidateEngineLibTestsPassed: 617,
  residentCandidateEngineHarnessTestsPassed: 11,
  residentCandidateEnginePacketTestsPassed: 2,
  residentCandidateEngineQualificationTestsPassed: 75,
  residentCandidateEnginePreflightTestsPassed: 2,
  residentCandidateEngineDoctestsPassed: 171,
  residentCandidateAdmissionModules: 168,
  residentCandidateAdmissionBodies: 7868,
  residentCandidateNewPendingVerusBodies: 29,
  aggregateSourcePinAdapterCommit: "c8e0a18c4561e6e9f470321276dbc5142834ce61",
  workerV3CandidateCommit: "fa80651bec5c96004aaff921a8433ac04979d1a5",
  workerV3CandidateTree: "e399f2b82966a34c63ddd358a646db522e2af4a3",
  workerV3CandidateIntegrated: false,
  workerV3ReviewDisposition: "do-not-integrate",
  successorKvBridgeCommit: "136a6d2ff92597c91caad3d0e33baede74cd4c9a",
  successorKvBridgeTree: "c9684e25b4fbd6737ce73307c3cdc21bde1b221e",
  engineeringSmokeRuntimeCommit: "254b89aa3a6e4e751c3ad81db84073a5fb26b52d",
  engineeringSmokeRuntimeTree: "e31a988bb8b74557381a4a04f0cb765cafb7cf72",
  rmsnormUniformFoldCommit: "7521cdcdfebf76dce5f5499aa25f2d90fb81033a",
  r33ExecutorCommit: "c1b9590b548acea2450136d52ad37a586d03bfed",
  intermediateIntegrationCommit: "23f326a3133ef4b5e0da19b9a170bf05f8cb7a6b",
  r33LifecycleCommit: "eebdb38dab764143b023b33311363a452a1238ee",
  prefillProofSourceCommit: "240bb3d1ce394436cc62244f51888d7737ea6b9c",
  prefillProofIntegratedCommit: "f709a5da2f0130087bcfc0605a4c84ebd966ea0c",
  directReadbackSourceCommit: "18eed253d30b40a23f3984d7249d24b4db7318d2",
  pairedPrefillExecutorCommit: "e8b9908e48313ee43cbeec8092e4ba62006a3dfb",
  engineeringIdentityCommit: "6df1f2fa99a409adbafd8c3e138f8eae2a728260",
  radixPrefixCommit: "1dc659beda81d37d746cb05a16d35fd7788e29ef",
  kernelRepinSourceCommit: "5ecad80658f27909fa2477d6f73e47e9f95407ae",
  compactCompletionSourceCommit: "c4f63ca071a4012c09bdff349c11d69af0f18b18",
  hoistedWitnessSourceCommit: "17b282f77ec16f598054d9983287094457be723c",
  immediateBranchSourceCommit: "fb991b83e2a0f9d1cf4f11058c51b372ee96acff",
  flattenedLoopSourceCommit: "aad3b3aae05a261ac011b3318b786790c1f08320",
  canonicalBoundSourceCommit: "96df9a33eaf0ef0d97c176e5aaa870ff336747c5",
  kernelCandidateSourceCommit: "72c78c6fb3766ae3de1a4a393f6a8fa358498ca4",
  kernelCandidateSourceTree: "6b3a5ab219216fe2d7adf797d7ea890f94c76a60",
  fe2o3V71Main: "6492c8fa85a00d93aa6ca2a4a77675fefad2fee6",
  fe2o3V71Tree: "68573bf31789625ecc2489491711ad9153eb1cac",
  fe2o3LatestMain: "cf6faec0ee3c026d3a1fc5090ab606a3b425225c",
  fe2o3LatestTree: "6d115af5cd5285b84b7629834393d6eee6a37045",
  formalVerified: 81,
  formalErrors: 0,
  proofTestsPassed: 26,
  sourceGateTestsPassed: 28,
  successorKvPendingVerusBodies: 16,
  admissionRows: 7127,
  admissionBodiesTotal: 7817,
  admissionVerifiedBodies: 690,
  admissionUnverifiedBodies: 7127,
  admissionModules: 167,
  admissionSourceGatesGreen: true,
  admissionTcbGatesGreen: true,
  combinedInventoryCurrent: true,
  verifiedInventorySha256: "55178ef806598eab581c2adde510b5d966a051c196dbe377b35cb54964da0beb",
  unverifiedInventorySha256: "86f0d6a2f7589460c94ac015881c4159697b352c7f46233dbeeb7d4c506ae6c7",
  integrationFormatGreen: true,
  integrationWorkspaceAllTargetsCheckGreen: true,
  integrationEngineTestsPassed: 698,
  integrationEngineHardwareIgnored: 7,
  integrationEngineLibTestsPassed: 608,
  integrationEngineHarnessTestsPassed: 11,
  integrationEnginePacketTestsPassed: 2,
  integrationEngineQualificationTestsPassed: 75,
  integrationEnginePreflightTestsPassed: 2,
  integrationAdapterTestsPassed: 95,
  integrationAdapterHardwareIgnored: 2,
  integrationSpeculativeClippyGreen: true,
  integrationResidentLintAllowances: 5,
  integrationStaticValidationLogSha256: "79e2f087e4cc8a316e9dc1c92dedea76b23682d99c1013247398bf27e5c8561b",
  integrationSourceGateLogSha256: "6d2904ab368fdec3a845ee04189e84fe20f1b2446115903198102441bd9e1bb1",
  focusedRmsnormTestsPassed: 21,
  focusedRmsnormLogShaPrefix: "4d2bcd1",
  exactCompilerAttempt: "v77",
  exactCompilerLogSha256: "85073cf74d25ad06854b1874c0199b309c0636615a2e2f89b7d9b04d08b5dee6",
  exactArtifactGateLogSha256: "13fbfd7eab77b5eafd36967f642080fcd507059a34a2a7242754a883b66dee49",
  exactAdapterLogSha256: "1bfbdf9ecd30b03fcc59f3a172f1a3549dbb2dc57652bcccf0b978be97b08b83",
  exactCompilerExitStatus: 0,
  exactCompilerOutputs: 2,
  exactCompilerHandoffBytes: 415541,
  exactCompilerHandoffSha256: "3f1a68ed30f243e640f38e90fedc483bfaf8d07f3a11578e21a8569369781f84",
  exactCompilerGuardedStores: 26,
  exactCompilerHsacoBytes: 103616,
  exactCompilerHsacoSha256: "3ce9820a870379d8e0d9e76bb98178776617a1ab132c7154d38929b1243a6d4d",
  exactCompilerManifestSha256: "1dc443f1c4a22570e5a997c24518c0c9dd7cc3ef11312229bb24bad6031df120",
  exactCompilerDescriptorSha256: "f3522e568e787ea808e47ce56e82553c3f3214b7a7b9d1a20f82889cc67fa8c2",
  exactCompilerKernelCount: 12,
  exactCompilerReplayExact: true,
  exactCompilerAdmissionGreen: true,
  exactCompilerPublicationGrant: false,
  exactCompilerLoadGrant: false,
  exactCompilerLaunchGrant: false,
  engineeringSmokeStop: "Completed",
  engineeringSmokeReachedKfd: true,
  engineeringSmokeGpuStateChanged: true,
  engineeringSmokeVramStateChanged: true,
  engineeringSmokeProcessStatus: 0,
  engineeringSmokePromptTokenId: 9707,
  engineeringSmokeGeneratedTokenIds: [94364, 43619, 101691, 33159],
  engineeringSmokeGeneratedText: "spent_cm谊tered",
  engineeringSmokeExecutionDurationSeconds: 17.076646692,
  engineeringSmokeInternalTtftSeconds: 4.227489904,
  engineeringSmokeTerminalOffsetSeconds: 15.429140005,
  engineeringSmokeDerivedGapAverageSecondsPerToken: 3.733883367,
  engineeringSmokeColdWallSeconds: 1445.15,
  engineeringSmokeStdoutSha256: "efc5cc2c738ac37dca0893f5c8fc79c882ec043a7f4e646ef2d4b1c747418402",
  engineeringSmokeStderrSha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  engineeringSmokeTimeSha256: "13b365651f5ac1c9b4081fb0a0f6f731f64184d6952f8c5557ae91d572494944",
  engineeringSmokeStatusSha256: "9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa",
  engineeringSmokeSummarySha256: "3cc352bece27748d35d275fe59a04895fd72162aa95225319147b500d66c06fe",
  engineeringSmokeTargetOnly: true,
  engineeringSmokeSpeculative: false,
  speculativeAttempt2BaseCommit: "1d8bf9a5a6391bf817eb05d3288f956f691cf5b8",
  speculativeAttempt2SourceSha256: "03135d7bc7d0a72c3011094358843e31ec036e0b7caabc97719bc234aa995580",
  speculativeAttempt2BinarySha256: "6269e836cc6cc94bafe74c2f826bbe419c0c79f6374d63f6531ce6188cfe0437",
  speculativeAttempt2StderrSha256: "73902eec259f77cc422e277a44d6ffe5d8c0c1155bb596f244193b3b85e68eee",
  speculativeAttempt2ElapsedSeconds: 1424.71,
  speculativeAttempt2Status: 134,
  speculativeAttempt2AllocationCount: 20,
  speculativeQueueAllocationMaximum: 16,
  speculativeAttempt2ReachedPublication: false,
  speculativeAttempt3SourceSha256: "94a3d9396f3ba83e909e41e691939c35ed38d6697f43bc1dfc0a34dc812dbeef",
  speculativeAttempt3Commit: "f18ffe2566112cb8b9518562afe2c8919577c907",
  speculativeAttempt3Tree: "04d09e03af9a8257d211382a6f7c91909dda7df6",
  speculativeAttempt3BinarySha256: "262edc61c9f1d4c9a474aac56f8749c34a74c557ab131eb06b0fbd263673362a",
  speculativeAttempt3Integrated: true,
  speculativeAttempt3ReviewComplete: true,
  speculativeAttempt3ReviewDisposition: "integrate",
  speculativeAttempt3Status: 0,
  speculativeAttempt3WallSeconds: 1478.66,
  speculativeAttempt3AllocationCount: 11,
  speculativeAttempt3PrefillNanoseconds: 4509687305,
  speculativeAttempt3RoundNanoseconds: 9557095343,
  speculativeAttempt3TotalNanoseconds: 14066782648,
  speculativeAttempt3FirstTokenId: 69761,
  speculativeAttempt3DraftChoices: [77903, 77903, 148549, 148549],
  speculativeAttempt3TargetChoices: [3681, 149508, 101547, 123907, 115413],
  speculativeAttempt3AcceptedDraftTokens: 0,
  speculativeAttempt3PublishedTokenIds: [3681],
  speculativeAttempt3PublishedText: " previous",
  speculativeAttempt3TargetCommitEnd: 129,
  speculativeAttempt3TargetRollback: 4,
  speculativeAttempt3DraftCommitEnd: 129,
  speculativeAttempt3DraftRollback: 3,
  speculativeAttempt3StdoutSha256: "0814fa033dd72587503324fc95efee7b529d171556b43015d0de66e5eacd0ce6",
  speculativeAttempt3StderrSha256: "f0403470b2eeac6ae04793da1f9bee359cfbcb5e74c75870ec4aeffab0710ef2",
  speculativeAttempt3StatusSha256: "9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa",
  speculativeAttempt3TimeSha256: "116698fd6c2f05f6f88b46cbb55de2d04ebaa8c9817abf42851f0f3a52c52319",
  speculativeHardwareCompleted: true,
  speculativeAuthenticatedAb: false,
  speculativeBenchmarkComparable: false,
  speculativeServingAvailable: false,
  hardwareCompletionObserved: true,
  benchmarkComparable: false,
  r33TpotEligible: false,
  r33AdapterTestsPassed: 68,
  r33AdapterHardwareIgnored: 2,
  previousIntegratedEngineTestsPassed: 857,
  previousIntegratedEngineHardwareIgnored: 7,
  previousEngineValidationLogSha256: "1d48f1a140d9f51dc7363dffa3bfbf2741e81ea97f9e9f125d9122ed247c356a",
  integratedEngineTestsPassed: 862,
  integratedEngineHardwareIgnored: 7,
  engineLibTestsPassed: 608,
  engineHarnessTestsPassed: 11,
  enginePacketTestsPassed: 2,
  engineQualificationTestsPassed: 75,
  enginePreflightTestsPassed: 2,
  engineDoctestsPassed: 164,
  engineValidationLogSha256: "7fdee7b4c554418e94b28a3d9d35b140fe983c88a0f9b68ec89d87cd00ea803c",
  engineValidationStatusSha256: "9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa",
  adapterValidationLogSha256: "f712b18cd848291f524c86ba92065134adfc89e595d27d7dfa5b4ff4fdcde910",
  remoteTestFailures: 0,
  r33ExecutableWindows: 1,
  r33RequiredWindows: 20,
  r33OutputTokensPerWindow: 128,
  radixVerusVerified: 608,
  radixVerusErrors: 0,
  openM1Gates: 33,
  engineeringSmokeBinaryStaged: true,
  canonicalQwenSnapshotVerified: true,
  radixPrefixIntegrated: true,
  currentAggregateHsaco: true,
  qwenTokenObserved: true,
  servingEndpointAvailable: false,
  baselineRunsAvailable: false,
  dockerAccessible: false,
  authority: "none",
};
assertExactKeys(project.current, Object.keys(expectedCurrent), "current");
for (const [key, value] of Object.entries(expectedCurrent)) {
  const matches = Array.isArray(value)
    ? JSON.stringify(project.current[key]) === JSON.stringify(value)
    : project.current[key] === value;
  assert(matches, `current.${key} drifted`);
}
for (const key of [
  "siteRefreshBase",
  "integrationCommit",
  "integrationTree",
  "residentTailCommit",
  "residentReconcileCommit",
  "residentFirstRoundCommit",
  "residentFirstRoundTree",
  "aggregateSourcePinAdapterCommit",
  "workerV3CandidateCommit",
  "workerV3CandidateTree",
  "successorKvBridgeCommit",
  "successorKvBridgeTree",
  "engineeringSmokeRuntimeCommit",
  "engineeringSmokeRuntimeTree",
  "rmsnormUniformFoldCommit",
  "r33ExecutorCommit",
  "intermediateIntegrationCommit",
  "r33LifecycleCommit",
  "prefillProofSourceCommit",
  "prefillProofIntegratedCommit",
  "directReadbackSourceCommit",
  "pairedPrefillExecutorCommit",
  "engineeringIdentityCommit",
  "radixPrefixCommit",
  "kernelCandidateSourceCommit",
  "kernelCandidateSourceTree",
  "fe2o3V71Main",
  "fe2o3V71Tree",
  "fe2o3LatestMain",
  "fe2o3LatestTree",
  "speculativeAttempt2BaseCommit",
  "speculativeAttempt3Commit",
  "speculativeAttempt3Tree",
]) {
  assertCommit(project.current[key], `current.${key}`);
}
assert(project.current.successorKvPendingVerusBodies === 16, "successor-KV pending-verus body count drifted");
assert(project.current.admissionRows === 7127, "unverified admission row count drifted");
assert(project.current.admissionBodiesTotal === 7817, "admission body total drifted");
assert(project.current.admissionVerifiedBodies === 690, "verified admission body count drifted");
assert(project.current.admissionUnverifiedBodies === 7127, "unverified admission body count drifted");
assert(project.current.admissionModules === 167, "admission module count drifted");
assert(
  project.current.admissionVerifiedBodies + project.current.admissionUnverifiedBodies ===
    project.current.admissionBodiesTotal,
  "verified and unverified admission bodies must equal the current total",
);
assert(project.current.admissionSourceGatesGreen === true, "source admission gates must remain green");
assert(project.current.admissionTcbGatesGreen === true, "TCB admission gates must remain green");
assert(project.current.combinedInventoryCurrent === true, "current admission inventory must remain explicit");
assertSha256(project.current.verifiedInventorySha256, "current.verifiedInventorySha256");
assertSha256(project.current.unverifiedInventorySha256, "current.unverifiedInventorySha256");
assert(project.current.integrationFormatGreen === true, "integration format gate must remain green");
assert(project.current.integrationWorkspaceAllTargetsCheckGreen === true, "workspace all-target check must remain green");
assert(project.current.integrationEngineTestsPassed === 698, "integration engine total drifted");
assert(project.current.integrationEngineHardwareIgnored === 7, "integration engine hardware-ignore count drifted");
assert(
  project.current.integrationEngineLibTestsPassed +
    project.current.integrationEngineHarnessTestsPassed +
    project.current.integrationEnginePacketTestsPassed +
    project.current.integrationEngineQualificationTestsPassed +
    project.current.integrationEnginePreflightTestsPassed ===
    project.current.integrationEngineTestsPassed,
  "integration engine categories must equal the current total",
);
assert(project.current.integrationAdapterTestsPassed === 95, "integration adapter total drifted");
assert(project.current.integrationAdapterHardwareIgnored === 2, "integration adapter hardware-ignore count drifted");
assert(project.current.integrationSpeculativeClippyGreen === true, "speculative clippy gate must remain green");
assert(project.current.integrationResidentLintAllowances === 5, "resident lint allowance count drifted");
assertSha256(project.current.integrationStaticValidationLogSha256, "current.integrationStaticValidationLogSha256");
assertSha256(project.current.integrationSourceGateLogSha256, "current.integrationSourceGateLogSha256");
assert(project.current.formalErrors === 0, "strict Verus errors must remain zero");
assert(project.current.openM1Gates === 33, "all 33 M1 exit gates remain open");
assert(project.current.engineeringSmokeBinaryStaged === true, "engineering smoke binary must remain explicit");
assert(project.current.canonicalQwenSnapshotVerified === true, "canonical Qwen snapshot status drifted");
assert(project.current.radixPrefixIntegrated === true, "integrated radix status drifted");
assert(project.current.currentAggregateHsaco === true, "authority-free exact HSACO must remain explicit");
assert(project.current.qwenTokenObserved === true, "site must retain the diagnostic Qwen token observation");
assert(project.current.servingEndpointAvailable === false, "site must not claim serving");
assert(project.current.baselineRunsAvailable === false, "site must not claim baseline runs");
assert(project.current.dockerAccessible === false, "Docker must remain inaccessible for this checkpoint");
assert(project.current.authority === "none", "checkpoint authority must remain none");
assert(project.current.r33ExecutableWindows === 1, "R33 must remain limited to one executable window");
assert(project.current.r33RequiredWindows === 20, "R33 qualification must still require 20 windows");
assert(project.current.r33OutputTokensPerWindow === 128, "R33 window must retain 128 output tokens");
assertSha256(project.current.engineValidationLogSha256, "current.engineValidationLogSha256");
assertSha256(project.current.engineValidationStatusSha256, "current.engineValidationStatusSha256");
assertSha256(project.current.previousEngineValidationLogSha256, "current.previousEngineValidationLogSha256");
assertSha256(project.current.adapterValidationLogSha256, "current.adapterValidationLogSha256");
assert(project.current.remoteTestFailures === 0, "repinned remote tests must retain zero failures");
assert(
  project.current.engineLibTestsPassed +
    project.current.engineHarnessTestsPassed +
    project.current.enginePacketTestsPassed +
    project.current.engineQualificationTestsPassed +
    project.current.enginePreflightTestsPassed +
    project.current.engineDoctestsPassed ===
    project.current.integratedEngineTestsPassed,
  "engine suite category counts must equal the integrated total",
);
assert(/^[0-9a-f]{7}$/.test(project.current.focusedRmsnormLogShaPrefix), "RMSNorm log prefix drifted");
assertSha256(project.current.exactCompilerLogSha256, "current.exactCompilerLogSha256");
assertSha256(project.current.exactArtifactGateLogSha256, "current.exactArtifactGateLogSha256");
assertSha256(project.current.exactAdapterLogSha256, "current.exactAdapterLogSha256");
assertSha256(project.current.exactCompilerHandoffSha256, "current.exactCompilerHandoffSha256");
assertSha256(project.current.exactCompilerHsacoSha256, "current.exactCompilerHsacoSha256");
assertSha256(project.current.exactCompilerManifestSha256, "current.exactCompilerManifestSha256");
assertSha256(project.current.exactCompilerDescriptorSha256, "current.exactCompilerDescriptorSha256");
assert(project.current.exactCompilerExitStatus === 0, "exact compiler status must remain 0");
assert(project.current.exactCompilerOutputs === 2, "exact compiler output count drifted");
assert(project.current.exactCompilerHandoffBytes === 415541, "exact compiler handoff byte count drifted");
assert(project.current.exactCompilerGuardedStores === 26, "exact compiler GuardedStore count drifted");
assert(project.current.exactCompilerHsacoBytes === 103616, "exact compiler HSACO byte count drifted");
assert(project.current.exactCompilerKernelCount === 12, "exact compiler kernel count drifted");
assert(project.current.exactCompilerReplayExact === true, "exact compiler replay must remain exact");
assert(project.current.exactCompilerAdmissionGreen === true, "exact engineering admission must remain green");
for (const grant of [
  "exactCompilerPublicationGrant",
  "exactCompilerLoadGrant",
  "exactCompilerLaunchGrant",
]) {
  assert(project.current[grant] === false, `${grant} must remain false`);
}
assert(project.current.engineeringSmokeStop === "Completed", "engineering smoke completion drifted");
assert(project.current.engineeringSmokeReachedKfd === true, "engineering smoke must retain KFD execution");
assert(project.current.engineeringSmokeGpuStateChanged === true, "hardware smoke GPU state must remain observed");
assert(project.current.engineeringSmokeVramStateChanged === true, "hardware smoke VRAM state must remain observed");
assert(project.current.engineeringSmokeProcessStatus === 0, "hardware smoke process must retain status 0");
assert(project.current.hardwareCompletionObserved === true, "hardware completion must remain observed");
assert(project.current.engineeringSmokePromptTokenId === 9707, "prompt token drifted");
assert(
  JSON.stringify(project.current.engineeringSmokeGeneratedTokenIds) ===
    "[94364,43619,101691,33159]",
  "generated token sequence drifted",
);
assert(project.current.engineeringSmokeGeneratedText === "spent_cm谊tered", "generated text drifted");
assert(project.current.engineeringSmokeExecutionDurationSeconds === 17.076646692, "execution duration drifted");
assert(project.current.engineeringSmokeInternalTtftSeconds === 4.227489904, "internal TTFT drifted");
assert(project.current.engineeringSmokeTerminalOffsetSeconds === 15.429140005, "terminal offset drifted");
assert(
  project.current.engineeringSmokeDerivedGapAverageSecondsPerToken === 3.733883367,
  "derived three-gap average drifted",
);
assert(project.current.engineeringSmokeColdWallSeconds === 1445.15, "cold wall drifted");
assert(
  Math.abs(
    (project.current.engineeringSmokeTerminalOffsetSeconds -
      project.current.engineeringSmokeInternalTtftSeconds) /
      3 -
      project.current.engineeringSmokeDerivedGapAverageSecondsPerToken,
  ) < 1e-9,
  "three-gap average must derive from terminal minus internal TTFT",
);
assertSha256(project.current.engineeringSmokeStdoutSha256, "current.engineeringSmokeStdoutSha256");
assertSha256(project.current.engineeringSmokeStderrSha256, "current.engineeringSmokeStderrSha256");
assertSha256(project.current.engineeringSmokeTimeSha256, "current.engineeringSmokeTimeSha256");
assertSha256(project.current.engineeringSmokeStatusSha256, "current.engineeringSmokeStatusSha256");
assertSha256(project.current.engineeringSmokeSummarySha256, "current.engineeringSmokeSummarySha256");
assert(project.current.engineeringSmokeTargetOnly === true, "four-token smoke must remain target-only");
assert(project.current.engineeringSmokeSpeculative === false, "site must not claim speculative execution");
assert(project.current.residentCandidateIntegrated === false, "resident candidate must remain unintegrated");
assert(project.current.residentCandidateReviewComplete === true, "resident review must remain complete");
assert(project.current.residentCandidateReviewDisposition === "hold", "resident review disposition drifted");
assert(project.current.residentCandidatePhysicalRunnable === false, "held resident candidate must not be marked runnable");
assert(project.current.residentCandidateEngineTestsPassed === 878, "resident host total drifted");
assert(project.current.residentCandidateHardwareIgnored === 7, "resident hardware-ignore count drifted");
assert(
  project.current.residentCandidateEngineLibTestsPassed +
    project.current.residentCandidateEngineHarnessTestsPassed +
    project.current.residentCandidateEnginePacketTestsPassed +
    project.current.residentCandidateEngineQualificationTestsPassed +
    project.current.residentCandidateEnginePreflightTestsPassed +
    project.current.residentCandidateEngineDoctestsPassed ===
    project.current.residentCandidateEngineTestsPassed,
  "resident suite category counts must equal the candidate total",
);
assert(project.current.residentCandidateAdmissionModules === 168, "resident admission module count drifted");
assert(project.current.residentCandidateAdmissionBodies === 7868, "resident admission body count drifted");
assert(project.current.residentCandidateNewPendingVerusBodies === 29, "resident pending-Verus count drifted");
assert(project.current.workerV3CandidateIntegrated === false, "Worker V3 candidate must remain unintegrated");
assert(project.current.workerV3ReviewDisposition === "do-not-integrate", "Worker V3 review disposition drifted");
assertSha256(project.current.speculativeAttempt2SourceSha256, "current.speculativeAttempt2SourceSha256");
assertSha256(project.current.speculativeAttempt2BinarySha256, "current.speculativeAttempt2BinarySha256");
assertSha256(project.current.speculativeAttempt2StderrSha256, "current.speculativeAttempt2StderrSha256");
assert(project.current.speculativeAttempt2ElapsedSeconds === 1424.71, "attempt 2 elapsed time drifted");
assert(project.current.speculativeAttempt2Status === 134, "attempt 2 status drifted");
assert(project.current.speculativeAttempt2AllocationCount === 20, "attempt 2 allocation count drifted");
assert(project.current.speculativeQueueAllocationMaximum === 16, "service allocation maximum drifted");
assert(project.current.speculativeAttempt2ReachedPublication === false, "attempt 2 must remain pre-publication");
assertSha256(project.current.speculativeAttempt3SourceSha256, "current.speculativeAttempt3SourceSha256");
assertSha256(project.current.speculativeAttempt3BinarySha256, "current.speculativeAttempt3BinarySha256");
assertSha256(project.current.speculativeAttempt3StdoutSha256, "current.speculativeAttempt3StdoutSha256");
assertSha256(project.current.speculativeAttempt3StderrSha256, "current.speculativeAttempt3StderrSha256");
assertSha256(project.current.speculativeAttempt3StatusSha256, "current.speculativeAttempt3StatusSha256");
assertSha256(project.current.speculativeAttempt3TimeSha256, "current.speculativeAttempt3TimeSha256");
assert(project.current.speculativeAttempt3Status === 0, "attempt 3 status drifted");
assert(project.current.speculativeAttempt3WallSeconds === 1478.66, "attempt 3 wall time drifted");
assert(project.current.speculativeAttempt3AllocationCount === 11, "attempt 3 allocation count drifted");
assert(project.current.speculativeAttempt3AllocationCount <= project.current.speculativeQueueAllocationMaximum, "attempt 3 roster must fit the service maximum");
assert(project.current.speculativeAttempt3PrefillNanoseconds === 4509687305, "attempt 3 prefill duration drifted");
assert(project.current.speculativeAttempt3RoundNanoseconds === 9557095343, "attempt 3 round duration drifted");
assert(project.current.speculativeAttempt3TotalNanoseconds === 14066782648, "attempt 3 total duration drifted");
assert(
  project.current.speculativeAttempt3PrefillNanoseconds +
    project.current.speculativeAttempt3RoundNanoseconds ===
    project.current.speculativeAttempt3TotalNanoseconds,
  "attempt 3 total must equal prefill plus speculative round",
);
assert(project.current.speculativeAttempt3FirstTokenId === 69761, "attempt 3 first token drifted");
assert(JSON.stringify(project.current.speculativeAttempt3DraftChoices) === "[77903,77903,148549,148549]", "attempt 3 draft choices drifted");
assert(JSON.stringify(project.current.speculativeAttempt3TargetChoices) === "[3681,149508,101547,123907,115413]", "attempt 3 target choices drifted");
assert(project.current.speculativeAttempt3AcceptedDraftTokens === 0, "attempt 3 accepted count drifted");
assert(JSON.stringify(project.current.speculativeAttempt3PublishedTokenIds) === "[3681]", "attempt 3 published tokens drifted");
assert(project.current.speculativeAttempt3PublishedText === " previous", "attempt 3 text drifted");
assert(project.current.speculativeAttempt3TargetCommitEnd === 129, "attempt 3 target commit drifted");
assert(project.current.speculativeAttempt3TargetRollback === 4, "attempt 3 target rollback drifted");
assert(project.current.speculativeAttempt3DraftCommitEnd === 129, "attempt 3 draft commit drifted");
assert(project.current.speculativeAttempt3DraftRollback === 3, "attempt 3 draft rollback drifted");
assert(project.current.speculativeHardwareCompleted === true, "attempt 3 hardware completion must remain observed");
assert(project.current.speculativeAttempt3Integrated === true, "attempt 3 must remain integrated");
assert(project.current.speculativeAttempt3ReviewComplete === true, "attempt 3 independent review must remain complete");
assert(project.current.speculativeAttempt3ReviewDisposition === "integrate", "attempt 3 review disposition drifted");
assert(project.current.speculativeAuthenticatedAb === false, "site must not claim authenticated A/B");
assert(project.current.speculativeBenchmarkComparable === false, "attempt 3 must remain non-comparable");
assert(project.current.speculativeServingAvailable === false, "attempt 3 must not become serving");
assert(
  project.current.integrationTree !== project.current.engineeringSmokeRuntimeTree,
  "current source-only integration must remain distinct from the exact v77 hardware runtime tree",
);
assert(project.current.benchmarkComparable === false, "diagnostic smoke must not become benchmark-comparable");
assert(project.current.r33TpotEligible === false, "four-token smoke must not become R33 TPOT-eligible");

assertExactKeys(project.milestone, ["name", "label", "state", "summary"], "milestone");
assert(project.milestone.name === "M1", "milestone must remain M1");
assertState(project.milestone.state, "milestone.state");

assert(Array.isArray(project.readiness) && project.readiness.length >= 5, "readiness roster is incomplete");
project.readiness.forEach((item, index) => {
  assertExactKeys(item, ["label", "state", "detail"], `readiness[${index}]`);
  assertState(item.state, `readiness[${index}].state`);
});

assert(Array.isArray(project.envelope) && project.envelope.length >= 8, "M1 envelope is incomplete");
const envelopeNames = new Set();
project.envelope.forEach((entry, index) => {
  assert(Array.isArray(entry) && entry.length === 2, `envelope[${index}] must be a pair`);
  assert(!envelopeNames.has(entry[0]), `duplicate envelope label ${entry[0]}`);
  envelopeNames.add(entry[0]);
});

assertExactKeys(project.capabilities, ["runnable", "experimental", "roadmap"], "capabilities");
for (const group of ["runnable", "experimental", "roadmap"]) {
  assert(Array.isArray(project.capabilities[group]) && project.capabilities[group].length > 0, `${group} is empty`);
  project.capabilities[group].forEach((item, index) => {
    assertExactKeys(item, ["name", "detail"], `capabilities.${group}[${index}]`);
  });
}

assertExactKeys(project.validation, ["host", "proof", "hardware", "transitions", "limitation"], "validation");
for (const kind of ["host", "proof", "hardware"]) {
  const item = project.validation[kind];
  const sourceKey = kind === "hardware" ? "sourceStatus" : "source";
  assertExactKeys(item, ["title", "state", sourceKey, "result", "detail"], `validation.${kind}`);
  assertState(item.state, `validation.${kind}.state`);
  if (item.source) assertCommit(item.source, `validation.${kind}.source`);
}
const transitionKeys = new Set();
project.validation.transitions.forEach((entry, index) => {
  assert(Array.isArray(entry) && entry.length === 3, `transition[${index}] must have three fields`);
  assertState(entry[2], `transition[${index}].state`);
  const key = `${entry[0]} -> ${entry[1]}`;
  assert(!transitionKeys.has(key), `duplicate transition ${key}`);
  transitionKeys.add(key);
});

assert(Array.isArray(project.teams) && project.teams.length === 4, "team ledger must contain four teams");
const teamNames = new Set();
project.teams.forEach((team, index) => {
  assertExactKeys(
    team,
    ["name", "scope", "state", "status", "completed", "current", "blockedBy", "next", "validation"],
    `teams[${index}]`,
  );
  assertState(team.state, `teams[${index}].state`);
  assert(!teamNames.has(team.name), `duplicate team ${team.name}`);
  assert(team.status.includes("no team blocker"), `${team.name} must report current blocker state`);
  assert(team.blockedBy.startsWith("No team-local blocker."), `${team.name} dependencies must remain explicit`);
  teamNames.add(team.name);
});

assertExactKeys(project.boundaries, ["ferric", "fe2o3"], "boundaries");
for (const key of ["ferric", "fe2o3"]) {
  assert(Array.isArray(project.boundaries[key]) && project.boundaries[key].length >= 5, `${key} boundary is incomplete`);
}

assertExactKeys(
  project.latestObservation,
  ["title", "state", "sourceStatus", "environment", "result", "buildId", "generatedTokenIds", "authority"],
  "latestObservation",
);
assertState(project.latestObservation.state, "latestObservation.state");
assert(project.latestObservation.state === "observed", "hardware token must remain an observed checkpoint");
assert(!("commit" in project.latestObservation), "hardware smoke uses source status rather than a publication binding");
assert(
  JSON.stringify(project.latestObservation.generatedTokenIds) === "[3681]",
  "latest observation must retain the exact speculative correction token",
);

assert(Array.isArray(project.recentProgress) && project.recentProgress.length >= 4, "progress ledger is incomplete");
project.recentProgress.forEach((item, index) => {
  const expected = item.repository
    ? ["commit", "repository", "title", "state", "detail"]
    : ["commit", "title", "state", "detail"];
  assertExactKeys(item, expected, `recentProgress[${index}]`);
  assertCommit(item.commit, `recentProgress[${index}].commit`);
  assertState(item.state, `recentProgress[${index}].state`);
});

assertExactKeys(project.evidence, ["summary", "legend", "gates"], "evidence");
assert(project.evidence.gates.length >= 6, "evidence gate roster is incomplete");
project.evidence.gates.forEach((entry, index) => {
  assert(Array.isArray(entry) && entry.length === 3, `evidence.gates[${index}] must be a triple`);
  assertState(entry[2], `evidence.gates[${index}].state`);
  if (entry[0] === "Authority-free aggregate HSACO") {
    assert(entry[1] === "1", "authority-free aggregate HSACO count drifted");
    assert(entry[2] === "integration", "authority-free aggregate HSACO state drifted");
  } else if (entry[0] === "Authority-free target-only diagnostic Qwen tokens") {
    assert(entry[1] === "4", "diagnostic Qwen token count drifted");
    assert(entry[2] === "observed", "diagnostic Qwen token state drifted");
  } else if (entry[0] === "Authority-free speculative diagnostic Qwen tokens") {
    assert(entry[1] === "1", "speculative Qwen token count drifted");
    assert(entry[2] === "observed", "speculative Qwen token state drifted");
  } else {
    assert(entry[2] === "open", `evidence gate ${entry[0]} must remain open`);
  }
});
project.evidence.legend.forEach((entry, index) => {
  assert(Array.isArray(entry) && entry.length === 2, `evidence.legend[${index}] must be a pair`);
  assertState(entry[0], `evidence.legend[${index}].state`);
});

const snapshot = JSON.stringify(project);
const missingSnapshotClaims = [
  "11e408970ddce28cacb491086d81852ddf07713f",
  "1d8bf9a5a6391bf817eb05d3288f956f691cf5b8",
  "44eaa3fef9e671f8b44ca76b3ac1f5dbde4704b5",
  "ea6ef07c7b13d31c84b14d2ad06f19f8d1220665",
  "34a2bf774ecd9072d4116423a7d512fdda882345",
  "6fcf568014818b4c5664bea72206f5d07dca050b",
  "55285c95446f40984892e4857d417b6412c89534",
  "2880334d3a359b37394cec5528609cec90248a40",
  "6609ce9375f4611a17fd7ff07c52adf18f33eb4c",
  "c8e0a18c4561e6e9f470321276dbc5142834ce61",
  "fa80651bec5c96004aaff921a8433ac04979d1a5",
  "e399f2b82966a34c63ddd358a646db522e2af4a3",
  "f18ffe2566112cb8b9518562afe2c8919577c907",
  "04d09e03af9a8257d211382a6f7c91909dda7df6",
  "136a6d2ff92597c91caad3d0e33baede74cd4c9a",
  "c9684e25b4fbd6737ce73307c3cdc21bde1b221e",
  "0f22443df20fefef8875cdbd53c55fa5803ec1ce",
  "e31a988bb8b74557381a4a04f0cb765cafb7cf72",
  "254b89aa3a6e4e751c3ad81db84073a5fb26b52d",
  "cf6faec0ee3c026d3a1fc5090ab606a3b425225c",
  "6d115af5cd5285b84b7629834393d6eee6a37045",
  "1dd3411af96a22f0ed86289b874b57fa28670ef9",
  "55c283d3b32ad31d8d06a8a5f158307ed2fa9a6e",
  "6239bdb2c0c8863c21e8fff102c69a53cfb7a035",
  "6af2244bf51cb1215747ed80309c89b61e3329bc",
  "f1519652a789d9b27cc49a545072d4027d545b0b",
  "28b925a1c4de75aa4ea35175a9f76d6071f4ca86",
  "1ddcd36b8f8b758e0d75780fe27813b4cd0581b1",
  "553334d69d51c4ccffea383cd72cf9105df74130",
  "629465f1a85a1af331a4e285062991f7ab59a5ce",
  "12da211265860aeec3235e49b631238ce3e618cf",
  "e8d0c889b710d2cb97f70b043e2ff67385bb4b75",
  "b1d45a00b52fc76d74f32a61cef66ee13f6da080",
  "6f50ebdbbfef97a9aa98daeaf465513b6edd06d2",
  "c1b9590b548acea2450136d52ad37a586d03bfed",
  "7521cdcdfebf76dce5f5499aa25f2d90fb81033a",
  "be5668eaa71f8d60a0a5041891d25ce2ed9c2e6e",
  "f8b09ddeaaed81b0f9f49e5d17020fdb5a434367",
  "1dc659beda81d37d746cb05a16d35fd7788e29ef",
  "6df1f2fa99a409adbafd8c3e138f8eae2a728260",
  "e8b9908e48313ee43cbeec8092e4ba62006a3dfb",
  "eebdb38dab764143b023b33311363a452a1238ee",
  "240bb3d1ce394436cc62244f51888d7737ea6b9c",
  "f709a5da2f0130087bcfc0605a4c84ebd966ea0c",
  "18eed253d30b40a23f3984d7249d24b4db7318d2",
  "23f326a3133ef4b5e0da19b9a170bf05f8cb7a6b",
  "6492c8fa85a00d93aa6ca2a4a77675fefad2fee6",
  "68573bf31789625ecc2489491711ad9153eb1cac",
  "3d10825df93a86644cc5a5b006cadd45f71afb91",
  "72c78c6fb3766ae3de1a4a393f6a8fa358498ca4",
  "6b3a5ab219216fe2d7adf797d7ea890f94c76a60",
  "5ecad80658f27909fa2477d6f73e47e9f95407ae",
  "c4f63ca071a4012c09bdff349c11d69af0f18b18",
  "2360a11bbac4e3376ba004472c2fda7aee2d26b1",
  "96c3f077e75bd059d7e66f26d61173453a22871e",
  "963ae4d724baa2157d5e91b2046e345909e75843",
  "17b282f77ec16f598054d9983287094457be723c",
  "9cb8e0cc53f2faef3d6d6fb7ad0ebc9c2bf2c0a5",
  "fb991b83e2a0f9d1cf4f11058c51b372ee96acff",
  "75e19bab4346ab1bf1d4f1ea82e5ecd8c3cce911",
  "aad3b3aae05a261ac011b3318b786790c1f08320",
  "9bc5e0677041d8d9a4c776de50de1a7ef5cc4242",
  "96df9a33eaf0ef0d97c176e5aaa870ff336747c5",
  "42e959710d830c6394413ff861c41dcbf61fd54d",
  "2af4b8672051740b26e2d8c0c81c0238a52f2114",
  "16 pending-verus",
  "7,127 unverified",
  "7,817 bodies",
  "690 verified",
  "167 modules",
  "source and TCB gates are green",
  "strict Verus 81 verified / 0 errors",
  "proof tests 26/26",
  "source gate passes 28/28",
  "RMSNorm validation passes 21/21",
  "4d2bcd1",
  "Exact v71",
  "07a6180",
  "generic fe2 WorkgroupCount X lowering",
  "outlined device helper",
  "Exact v74",
  "WorkgroupCount helper gap",
  "generic fe2 WorkgroupSize X lowering",
  "f15 bb0 op2",
  "Status is 1",
  "no outputs",
  "6dd1e79bcc88d24fb1a42779e567985a329cf65ee6a995fc9e3511f2ed88fe42",
  "Exact v76",
  "Exact v77",
  "Process status 0",
  "415,541-byte",
  "31a15c036261f0d7d2ab7027e709ea6c01c16f9654bc3d6e248dca3994a12282",
  "both prior outlined-helper geometry gaps",
  "415,659-byte Kernel IR V9 handoff",
  "26 GuardedStore operations",
  "103,616-byte",
  "c270a528439df199cdae59b4cffda5566ad8a1d24d7029b2b58a519a4aef5b94",
  "6af8bd8292c5c108ea77e2416fb1325f11507d471254f2cc9607b6ae8be5035a",
  "fc8469775585a165c9674ff25683ee0676fd0e6c11a1885e1dde26cdcea392d5",
  "3f1a68ed30f243e640f38e90fedc483bfaf8d07f3a11578e21a8569369781f84",
  "3ce9820a870379d8e0d9e76bb98178776617a1ab132c7154d38929b1243a6d4d",
  "1dc443f1c4a22570e5a997c24518c0c9dd7cc3ef11312229bb24bad6031df120",
  "f3522e568e787ea808e47ce56e82553c3f3214b7a7b9d1a20f82889cc67fa8c2",
  "85073cf74d25ad06854b1874c0199b309c0636615a2e2f89b7d9b04d08b5dee6",
  "13fbfd7eab77b5eafd36967f642080fcd507059a34a2a7242754a883b66dee49",
  "1bfbdf9ecd30b03fcc59f3a172f1a3549dbb2dc57652bcccf0b978be97b08b83",
  "all 12 kernels",
  "exact replay",
  "Prompt token 9707",
  "94364, 43619, 101691, and 33159",
  "spent_cm谊tered",
  "hardware completion",
  "process status 0",
  "4.227489904 seconds",
  "15.429140005 seconds",
  "3.733883367 seconds/token",
  "17.076646692 seconds",
  "1445.15 seconds",
  "10.15 seconds above",
  "different output length",
  "uncontrolled cold variance",
  "neither speedup nor regression",
  "target-only",
  "not speculative serving",
  "23:55.18",
  "175.74 seconds",
  "10.91%",
  "26:50.92",
  "two independent admissions",
  "repeated serial KFD hashes/copy/readback",
  "engineering attribution",
  "not token compute",
  "benchmark_comparable=false",
  "r33_tpot_eligible=false",
  "efc5cc2c738ac37dca0893f5c8fc79c882ec043a7f4e646ef2d4b1c747418402",
  "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "13b365651f5ac1c9b4081fb0a0f6f731f64184d6952f8c5557ae91d572494944",
  "9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa",
  "3cc352bece27748d35d275fe59a04895fd72162aa95225319147b500d66c06fe",
  "native AMDGPU LLVM worker",
  "SIGABRT",
  "empty output manifest",
  "7d7fbb57a113f27ec42fcf919751466b783706242f12949950a0b2bd80db7d0e",
  "608 lib",
  "11 harness",
  "2 packet",
  "75 qualification",
  "2 preflight",
  "164 doctests",
  "7 hardware ignores",
  "7fdee7b4c554418e94b28a3d9d35b140fe983c88a0f9b68ec89d87cd00ea803c",
  "adapter 68 passed / 2 ignored",
  "zero failures",
  "cde5a597108aa90784e04d2397cdd5090378a67e98cc5c05f95da9f157bf4991",
  "6dbc57fc05b06935a24f29b68c1fc4718df4dd541a187103327e36f5c7b36215",
  "1d48f1a140d9f51dc7363dffa3bfbf2741e81ea97f9e9f125d9122ed247c356a",
  "f712b18cd848291f524c86ba92065134adfc89e595d27d7dfa5b4ff4fdcde910",
  "non-hardware validation",
  "exactly one 128-output window",
  "CLOCK_MONOTONIC_RAW",
  "checked-token causality",
  "required 20-window qualification",
  "page-less successor KV leasing",
  "exact K draft growth",
  "fail-closed custody",
  "independent no-blocker review",
  "resident daemon",
  "20 queue-visible allocations",
  "maximum of 16",
  "4 model",
  "2 paired workspaces",
  "12 all-finite successor outputs",
  "1 active compact output",
  "1 direct choice",
  "03135d7bc7d0a72c3011094358843e31ec036e0b7caabc97719bc234aa995580",
  "6269e836cc6cc94bafe74c2f826bbe419c0c79f6374d63f6531ce6188cfe0437",
  "73902eec259f77cc422e277a44d6ffe5d8c0c1155bb596f244193b3b85e68eee",
  "94a3d9396f3ba83e909e41e691939c35ed38d6697f43bc1dfc0a34dc812dbeef",
  "262edc61c9f1d4c9a474aac56f8749c34a74c557ab131eb06b0fbd263673362a",
  "0814fa033dd72587503324fc95efee7b529d171556b43015d0de66e5eacd0ce6",
  "f0403470b2eeac6ae04793da1f9bee359cfbcb5e74c75870ec4aeffab0710ef2",
  "116698fd6c2f05f6f88b46cbb55de2d04ebaa8c9817abf42851f0f3a52c52319",
  "1424.71 seconds",
  "status 134",
  "before queue publication",
  "11 queue-visible allocations",
  "paired prefill plus one S1/K4 round",
  "zero draft tokens",
  "correction ID 3681",
  "4,509,687,305 ns",
  "9,557,095,343 ns",
  "14,066,782,648 ns",
  "1478.66 seconds",
  "[77903, 77903, 148549, 148549]",
  "[3681, 149508, 101547, 123907, 115413]",
  "commit 129",
  "rollback counts 4 and 3",
  "authenticated_AB=false",
  "878 remote host checks",
  "617 lib",
  "171 doctests",
  "7,868 bodies",
  "168 modules",
  "29 new pending-Verus bodies",
  "review hold",
  "authenticated_prefill_bootstrap",
  "physically runnable",
  "independently reviewed",
  "proof-inventory ordering",
  "55178ef806598eab581c2adde510b5d966a051c196dbe377b35cb54964da0beb",
  "86f0d6a2f7589460c94ac015881c4159697b352c7f46233dbeeb7d4c506ae6c7",
  "698 engine",
  "95 adapter",
  "five existing resident lint allowances",
  "79e2f087e4cc8a316e9dc1c92dedea76b23682d99c1013247398bf27e5c8561b",
  "6d2904ab368fdec3a845ee04189e84fe20f1b2446115903198102441bd9e1bb1",
  "do-not-integrate",
  "zero directly verified proof rows",
  "incomplete exact dependency roster",
  "missing committed collector behavior harness",
  "consume-plus-locked-current-revalidate handoff",
  "Physical device-KV prefix reuse is M2",
  "Authority is none",
  "608 verified and 0 errors",
  "S1/T128",
  "NoMatch",
  "live-source",
  "logical",
  "v39",
  "27/27",
  "compiler-intrinsic borrow",
  "TCB gates pass 28/28",
  "focused Rope/KV v42 27/27",
  "v43 27/27",
  "v44 27/27",
  "local 201/type 63",
  "cargo fmt",
  "v50 27/27",
  "full ferric-qwen-kernels suite",
  "audited seven-file exact kernel/host ABI delta",
  "not final or public product integration",
  "repin",
  "retained-borrow locals 178 and 40",
  "AMDGPU LLVM lowering",
  "protected receipt/verifier service is undeployed",
  "symmetric memory",
  "MTP",
  "All 33 M1 exit gates remain open",
  "four target-only Qwen tokens",
  "TTFT",
  "TPOT",
  "vLLM and SGLang baselines are absent",
].filter((claim) => !snapshot.includes(claim));
assert(
  missingSnapshotClaims.length === 0,
  `current snapshot is missing claims: ${missingSnapshotClaims.join(", ")}`,
);
for (const staleOrForbidden of [
  "6,854 admitted",
  "97 unadmitted",
  "7,632 executable bodies",
  "681 directly verified",
  "6,951 explicitly unverified",
  "0 unadmitted",
  "0 stale",
  "focused v20",
  "Focused v26",
  "v21's single-SSA-local fix",
  "production R33 backend remain absent",
  "Blocked on RMSNorm barrier convergence",
  "The current kernel obligation is RMSNorm barrier convergence",
  "Current exact blocker: qwen3_rmsnorm_v1",
  "Blocked on native LLVM worker abort",
  "Current exact blocker: native LLVM worker abort",
  "No aggregate HSACO was produced",
  "Qwen serving is ready",
  "bootstrap remains under review",
  "bootstrap is not integrated",
  "current aggregate HSACO is ready",
  "deterministic scalar-reachability work bound exhausted",
  "production paired-prefill executor is in progress",
  "Radix prefix reuse is not integrated",
  "v36 active",
  "v39 validation is pending",
  "awaits focused and exact validation",
  "fresh frozen exact compile",
  "replacement exact compile is running",
  "remote lock regeneration is live",
  "comprehensive 54-file repin is in progress",
  "Vendor v6 generation",
  "canonical tools and exact compilation are pending",
  "exact toolchain closure is under repair",
  "exact v7 is live",
  "exact v10 is live",
  "Exact v10 is live",
  "exact v11 is live",
  "Exact v11 is live",
  "exact v13 is live",
  "Exact v13 is live",
  "Blocked on fe2o3 private-slot lowering",
  "General fe2o3 private-slot lowering is required",
  "sole current terminal blocker: function 6",
  "CurrentFerricDescriptorRoster",
  "before KFD",
  "GPU and VRAM state are unchanged",
  "No Qwen token",
  "No hardware execution",
  "provisionally-integratable",
  "final audit pending",
]) {
  assert(!snapshot.includes(staleOrForbidden), `stale or forbidden claim remains: ${staleOrForbidden}`);
}

const indexSource = await readFile(join(siteRoot, "index.html"), "utf8");
const appSource = await readFile(join(siteRoot, "app.js"), "utf8");
for (const target of [
  "data-readiness",
  "data-envelope",
  "data-capabilities",
  "data-validation",
  "data-transitions",
  "data-teams",
  "data-boundaries",
  "data-observation",
  "data-progress",
  "data-gates",
]) {
  assert(indexSource.includes(target), `index is missing ${target}`);
}
for (const asset of ["styles.css", "app.js", "data/project.js", "assets/mark.svg", "assets/architecture.svg"]) {
  await access(join(siteRoot, asset), constants.R_OK);
}
assert(!appSource.includes("innerHTML"), "renderer must not inject status through innerHTML");
assert(!appSource.includes("eval("), "renderer must not evaluate status strings");

console.log("Ferric site data is structurally valid and current claims remain fail-closed.");
