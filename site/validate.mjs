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
  siteRefreshBase: "37ad8bd28e28eae1883bc8c74a19e6b2a466a29d",
  integrationCommit: "1addeb33664bce3f8e634c47e2fec09bb3d7cf42",
  integrationTree: "82826452040f8cb587df1bd88b9fb0f18668fd8e",
  integrationFormattingFixCommit: "1addeb33664bce3f8e634c47e2fec09bb3d7cf42",
  integrationFormattingFixTree: "82826452040f8cb587df1bd88b9fb0f18668fd8e",
  priorValidatedIntegrationCommit: "e5d23e19973a01000129aaf2ec705b8993bd21c3",
  priorValidatedIntegrationTree: "f331a34c08394d4fc599880ac342baa51140eca3",
  residentTailCommit: "886a0d38480887e05cc4c4dde858ff56ad155ca4",
  residentReconcileCommit: "c94c69f3a5cfd763c45747e943caf57e215d1a79",
  residentFirstRoundCommit: "de490d6ca68bb6ebab993774c008fd8553a1895b",
  residentFirstRoundTree: "66bed06aeb97987327846567c690a6a22cbd3db3",
  residentRosterCommit: "68091859ce0710dc26cfb839d580e40ea0839253",
  residentRosterTree: "4a4ea1e7db1d651085b74180d38a5bdab30456f7",
  residentCandidateIntegrated: true,
  residentCandidateReviewComplete: true,
  residentCandidateReviewDisposition: "integrate",
  residentHardwareQualified: false,
  residentCandidateAdmissionModules: 168,
  residentCandidateAdmissionBodies: 7869,
  genericProviderCandidateCommit: "b30e7a6a20e4ea8a14a41e7dcb2a42927c1d68b9",
  genericProviderCandidateTree: "57bb4f66a396f1426b10726273f8c9c5f56cc0e1",
  genericProviderCandidateIntegrated: true,
  genericProviderReviewDisposition: "integrate",
  genericProviderExactAllocationCount: 11,
  genericProviderHardwareQualified: false,
  workerV3SourcePinCommit: "181cc2ab2d1fc781513e0ef557c5838c71e1a83b",
  workerV3CollectorCommit: "2c778951bd09ae3d3c96cdc5a0258e57fcc18e9b",
  workerV3ProofCommit: "caf83e690d47b131f26a31111b4978d2ba13d133",
  workerV3CandidateCommit: "c492c3113f6d3d4f5a56366978ccdd0c3b06e6f5",
  workerV3CandidateTree: "9cc283baae4026257f18d0d4b6e9779abf98382d",
  workerV3CandidateIntegrated: true,
  workerV3ReviewDisposition: "integrate",
  workerV3IntegratedSourcePinCommit: "61a40cad79b31124044fca198155492e6b151ef4",
  workerV3IntegratedCollectorCommit: "2abcc47f2f5faad4a8ebd91c1eb8497adab20b1c",
  workerV3IntegratedProofCommit: "57973da0c3d362446361d1db026c19040bd20bdb",
  workerV3IntegratedDependencyCommit: "7747409db8b4f3013456d99b5e418bc12bc72494",
  workerV3IntegratedTailCommit: "e3dc8d68bde6efdd9ec0f2df47daaad4db4037b0",
  workerV3StrictProofPackagesPassed: 10,
  workerV3SameSourceNegativeProofsGreen: true,
  workerV3FullQualificationAttempt: 4,
  workerV3GeneratedSha2RuntimeTcbRowCommitted: true,
  workerV3FocusedSourceGateTestsPassed: 33,
  workerV3ExactDecoyRejectionGreen: true,
  workerV3FocusedValidationLogSha256: "ac38d31da43810046ad57079582d1ba1ca78b318112ab8eceb8a7a407fbaba49",
  workerV3FullNegativeReleaseGateComplete: true,
  workerV3BothNegativeSuitesGreen: true,
  workerV3FullQualificationReceiptEmitted: false,
  workerV3FullQualificationBaseCommit: "ea6ef07c7b13d31c84b14d2ad06f19f8d1220665",
  workerV3CurrentIntegrationFmtGreen: true,
  combinedAttempt5SourceCommit: "e3dc8d68bde6efdd9ec0f2df47daaad4db4037b0",
  combinedAttempt5SourceTree: "4db7406eec1c1b53a2872bf711835a4ef74fa0b5",
  combinedAttempt5StrictProofPackagesPassed: 10,
  combinedAttempt5SameSourceNegativeProofsGreen: true,
  combinedAttempt5FullNegativePropertyPolicyGreen: true,
  combinedAttempt5ReceiptEmitted: false,
  combinedAttempt5NestedFmtDiffs: 4,
  combinedQualificationAttempt: 6,
  combinedQualificationState: "running",
  combinedQualificationRunning: true,
  combinedPreflightGreen: true,
  combinedPreflightLogSha256: "ec50af0f4a3ed5231046db9f82b2e639edf8a84b5e7cb43dd2b190818c306188",
  combinedPreflightCuratedAdmissions: 7229,
  r33TwentyWindowCommit: "a9966134c396c9ba8b4cd4eb8e4eb2bff1833271",
  r33TwentyWindowTree: "9f5b2fa6284b029ab216466fa9bd91b001f7c792",
  r33TwentyWindowImplemented: true,
  r33TwentyWindowReviewState: "under-review",
  r33TwentyWindowReviewHasProbableHoldItems: true,
  r33TwentyWindowActive: false,
  r33TwentyWindowIntegrated: false,
  r33TwentyWindowValidationReported: false,
  signerIpcCommit: "f657c5f1ae4cdbf8039ed55933b4f0e3385b47de",
  signerIpcTree: "969611f506286f250d027d619e25d04728312419",
  signerIpcIntegrated: false,
  signerIpcReviewDisposition: "integrate",
  signerIpcServiceTestsPassed: 58,
  signerIpcServiceTestsIgnored: 3,
  signerIpcAdapterTestsPassed: 88,
  signerIpcSourceGateTestsPassed: 28,
  signerIpcValidationLogSha256: "0a4283731228a40a46233ed00a2bb846fcbe47ce01f764a4cfd721add1d9ac73",
  signerIpcHardwareQualified: false,
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
  sourceGateTestsPassed: 33,
  successorKvPendingVerusBodies: 16,
  admissionBodiesTotal: 7922,
  admissionModules: 170,
  admissionSourceGatesGreen: true,
  admissionTcbGatesGreen: true,
  combinedInventoryCurrent: true,
  integrationEngineTestsPassed: 711,
  integrationEngineHardwareIgnored: 7,
  integrationEngineDoctestsPassed: 171,
  integrationAdapterTestsPassed: 93,
  integrationAdapterHardwareIgnored: 2,
  integrationEvidenceAggregateSha256: "b10a6e0ef1b40c1e05d5320e5391f1c946bea1f4b8badcffef8cbe58124233ee",
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
  "integrationFormattingFixCommit",
  "integrationFormattingFixTree",
  "priorValidatedIntegrationCommit",
  "priorValidatedIntegrationTree",
  "residentTailCommit",
  "residentReconcileCommit",
  "residentFirstRoundCommit",
  "residentFirstRoundTree",
  "residentRosterCommit",
  "residentRosterTree",
  "genericProviderCandidateCommit",
  "genericProviderCandidateTree",
  "workerV3SourcePinCommit",
  "workerV3CollectorCommit",
  "workerV3ProofCommit",
  "workerV3CandidateCommit",
  "workerV3CandidateTree",
  "workerV3IntegratedSourcePinCommit",
  "workerV3IntegratedCollectorCommit",
  "workerV3IntegratedProofCommit",
  "workerV3IntegratedDependencyCommit",
  "workerV3IntegratedTailCommit",
  "workerV3FullQualificationBaseCommit",
  "combinedAttempt5SourceCommit",
  "combinedAttempt5SourceTree",
  "r33TwentyWindowCommit",
  "r33TwentyWindowTree",
  "signerIpcCommit",
  "signerIpcTree",
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
assert(project.current.admissionBodiesTotal === 7922, "admission body total drifted");
assert(project.current.admissionModules === 170, "admission module count drifted");
assert(project.current.admissionSourceGatesGreen === true, "source admission gates must remain green");
assert(project.current.admissionTcbGatesGreen === true, "TCB admission gates must remain green");
assert(project.current.combinedInventoryCurrent === true, "current-head combined inventory must remain explicit");
assert(project.current.integrationEngineTestsPassed === 711, "integration engine total drifted");
assert(project.current.integrationEngineHardwareIgnored === 7, "integration engine hardware-ignore count drifted");
assert(project.current.integrationEngineDoctestsPassed === 171, "integration doctest total drifted");
assert(project.current.integrationAdapterTestsPassed === 93, "integration adapter total drifted");
assert(project.current.integrationAdapterHardwareIgnored === 2, "integration adapter hardware-ignore count drifted");
assertSha256(project.current.integrationEvidenceAggregateSha256, "current.integrationEvidenceAggregateSha256");
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
assert(project.current.residentCandidateIntegrated === true, "reviewed resident series must remain integrated");
assert(project.current.residentCandidateReviewComplete === true, "resident review must remain complete");
assert(project.current.residentCandidateReviewDisposition === "integrate", "resident review disposition drifted");
assert(project.current.residentHardwareQualified === false, "resident path must remain hardware-unqualified");
assert(project.current.residentCandidateAdmissionModules === 168, "resident admission module count drifted");
assert(project.current.residentCandidateAdmissionBodies === 7869, "resident admission body count drifted");
assert(project.current.genericProviderCandidateIntegrated === true, "accepted generic provider must remain integrated");
assert(project.current.genericProviderReviewDisposition === "integrate", "generic provider review disposition drifted");
assert(project.current.genericProviderExactAllocationCount === 11, "generic provider allocation roster drifted");
assert(project.current.genericProviderHardwareQualified === false, "generic provider must remain hardware-unqualified");
assert(project.current.workerV3CandidateIntegrated === true, "accepted Worker V3 candidate must remain integrated");
assert(project.current.workerV3ReviewDisposition === "integrate", "Worker V3 review disposition drifted");
assert(project.current.workerV3StrictProofPackagesPassed === 10, "Worker V3 strict proof package count drifted");
assert(project.current.workerV3SameSourceNegativeProofsGreen === true, "Worker V3 same-source negative proofs must remain green");
assert(project.current.workerV3FullQualificationAttempt === 4, "Worker V3 qualification attempt drifted");
assert(project.current.workerV3GeneratedSha2RuntimeTcbRowCommitted === true, "Worker V3 generated sha2 runtime TCB row must remain committed");
assert(project.current.workerV3FocusedSourceGateTestsPassed === 33, "Worker V3 focused source-gate count drifted");
assert(project.current.workerV3ExactDecoyRejectionGreen === true, "Worker V3 exact decoy rejection must remain green");
assertSha256(project.current.workerV3FocusedValidationLogSha256, "current.workerV3FocusedValidationLogSha256");
assert(project.current.workerV3FullNegativeReleaseGateComplete === true, "Worker V3 full negative/release gate must remain complete");
assert(project.current.workerV3BothNegativeSuitesGreen === true, "Worker V3 negative suites must remain green");
assert(project.current.workerV3FullQualificationReceiptEmitted === false, "Worker V3 qualification #4 must not claim a receipt");
assert(project.current.workerV3CurrentIntegrationFmtGreen === true, "current integration formatting must remain green");
assert(project.current.combinedAttempt5StrictProofPackagesPassed === 10, "combined #5 strict proof count drifted");
assert(project.current.combinedAttempt5SameSourceNegativeProofsGreen === true, "combined #5 same-source negative proofs must remain green");
assert(project.current.combinedAttempt5FullNegativePropertyPolicyGreen === true, "combined #5 negative/property policy must remain green");
assert(project.current.combinedAttempt5ReceiptEmitted === false, "combined #5 must not claim a receipt");
assert(project.current.combinedAttempt5NestedFmtDiffs === 4, "combined #5 nested formatting diff count drifted");
assert(project.current.combinedQualificationAttempt === 6, "combined qualification attempt drifted");
assert(project.current.combinedQualificationState === "running", "combined qualification state drifted");
assert(project.current.combinedQualificationRunning === true, "combined qualification #6 must remain running");
assert(project.current.combinedPreflightGreen === true, "combined qualification preflight must remain green");
assertSha256(project.current.combinedPreflightLogSha256, "current.combinedPreflightLogSha256");
assert(project.current.combinedPreflightCuratedAdmissions === 7229, "combined preflight admission count drifted");
assert(project.current.r33TwentyWindowImplemented === true, "20-window R33 candidate must remain implemented");
assert(project.current.r33TwentyWindowReviewState === "under-review", "20-window R33 review state drifted");
assert(project.current.r33TwentyWindowReviewHasProbableHoldItems === true, "20-window R33 probable HOLD items must remain explicit");
assert(project.current.r33TwentyWindowActive === false, "20-window R33 implementation work must not remain active");
assert(project.current.r33TwentyWindowIntegrated === false, "20-window R33 work must remain unintegrated");
assert(project.current.r33TwentyWindowValidationReported === false, "20-window R33 work must not claim validation");
assert(project.current.signerIpcIntegrated === false, "signer IPC slice must remain unintegrated");
assert(project.current.signerIpcReviewDisposition === "integrate", "signer IPC review disposition drifted");
assert(project.current.signerIpcServiceTestsPassed === 58, "signer IPC service test count drifted");
assert(project.current.signerIpcServiceTestsIgnored === 3, "signer IPC ignored test count drifted");
assert(project.current.signerIpcAdapterTestsPassed === 88, "signer IPC adapter test count drifted");
assert(project.current.signerIpcSourceGateTestsPassed === 28, "signer IPC source-gate count drifted");
assertSha256(project.current.signerIpcValidationLogSha256, "current.signerIpcValidationLogSha256");
assert(project.current.signerIpcHardwareQualified === false, "signer IPC must remain hardware-unqualified");
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
  "37ad8bd28e28eae1883bc8c74a19e6b2a466a29d",
  "1addeb33664bce3f8e634c47e2fec09bb3d7cf42",
  "82826452040f8cb587df1bd88b9fb0f18668fd8e",
  "e3dc8d68bde6efdd9ec0f2df47daaad4db4037b0",
  "4db7406eec1c1b53a2872bf711835a4ef74fa0b5",
  "e5d23e19973a01000129aaf2ec705b8993bd21c3",
  "f331a34c08394d4fc599880ac342baa51140eca3",
  "b10a6e0ef1b40c1e05d5320e5391f1c946bea1f4b8badcffef8cbe58124233ee",
  "68091859ce0710dc26cfb839d580e40ea0839253",
  "4a4ea1e7db1d651085b74180d38a5bdab30456f7",
  "886a0d38480887e05cc4c4dde858ff56ad155ca4",
  "c94c69f3a5cfd763c45747e943caf57e215d1a79",
  "de490d6ca68bb6ebab993774c008fd8553a1895b",
  "66bed06aeb97987327846567c690a6a22cbd3db3",
  "b30e7a6a20e4ea8a14a41e7dcb2a42927c1d68b9",
  "57bb4f66a396f1426b10726273f8c9c5f56cc0e1",
  "181cc2ab2d1fc781513e0ef557c5838c71e1a83b",
  "2c778951bd09ae3d3c96cdc5a0258e57fcc18e9b",
  "caf83e690d47b131f26a31111b4978d2ba13d133",
  "c492c3113f6d3d4f5a56366978ccdd0c3b06e6f5",
  "9cc283baae4026257f18d0d4b6e9779abf98382d",
  "61a40cad79b31124044fca198155492e6b151ef4",
  "2abcc47f2f5faad4a8ebd91c1eb8497adab20b1c",
  "57973da0c3d362446361d1db026c19040bd20bdb",
  "7747409db8b4f3013456d99b5e418bc12bc72494",
  "ea6ef07c7b13d31c84b14d2ad06f19f8d1220665",
  "a9966134c396c9ba8b4cd4eb8e4eb2bff1833271",
  "9f5b2fa6284b029ab216466fa9bd91b001f7c792",
  "f657c5f1ae4cdbf8039ed55933b4f0e3385b47de",
  "969611f506286f250d027d619e25d04728312419",
  "0a4283731228a40a46233ed00a2bb846fcbe47ce01f764a4cfd721add1d9ac73",
  "ac38d31da43810046ad57079582d1ba1ca78b318112ab8eceb8a7a407fbaba49",
  "1d8bf9a5a6391bf817eb05d3288f956f691cf5b8",
  "44eaa3fef9e671f8b44ca76b3ac1f5dbde4704b5",
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
  "7,874 bodies",
  "690 verified",
  "168 modules",
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
  "independently reviewed",
  "proof-inventory ordering",
  "711 engine target",
  "171 doctests",
  "93 adapter",
  "all 10 strict proof packages",
  "every same-source negative proof",
  "full negative/property policy",
  "four nested formatting diffs",
  "no qualification receipt",
  "Combined qualification #5",
  "combined qualification #6",
  "running from clean committed",
  "root plus all six standalone formatting manifests",
  "five exact TCB",
  "7,229",
  "7,922",
  "170 modules",
  "ec50af0f4a3ed5231046db9f82b2e639edf8a84b5e7cb43dd2b190818c306188",
  "probable lifecycle/custody HOLD items",
  "independently ACCEPTED",
  "58 service tests",
  "88 adapter tests",
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
  "unintegrated engineering harness",
  "Current integration ea6ef07",
  "Current integration 6809185",
  "Unintegrated resident commits",
  "resident candidate has green",
  "resident candidate 2880334",
  "do-not-integrate",
  "generated sha2 runtime TCB row remains uncommitted",
  "expected-diagnostic fix and generated sha2 runtime TCB row are uncommitted",
  "Worker fa80651",
  "generic provider b30e7a6 remains unintegrated",
  "unintegrated pending independent review",
  "7,869 bodies",
  "708 engine target",
  "92 adapter",
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
