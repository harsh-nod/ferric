import { access, readFile } from "node:fs/promises";
import { constants } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";
import { validatePerformance, testPerformanceRejections } from "./validate-performance.mjs";
import { validateCompetitiveness, testCompetitivenessRejections } from "./validate-competitiveness.mjs";
import { validateFollowup, testFollowupRejections } from "./validate-competitiveness-followup.mjs";
import { validateRecovery, testRecoveryRejections } from "./validate-competitiveness-recovery.mjs";

const siteRoot = dirname(fileURLToPath(import.meta.url));
const dataSource = await readFile(join(siteRoot, "data/project.js"), "utf8");
const context = { window: {} };
vm.runInNewContext(dataSource, context, { filename: "site/data/project.js" });
const project = context.window.FERRIC_PROJECT;
validateCompetitiveness(project.competitivenessSprint);
testCompetitivenessRejections(project.competitivenessSprint);
validateFollowup(project.competitivenessFollowup);
testFollowupRejections(project.competitivenessFollowup);
validateRecovery(project.competitivenessRecovery);
testRecoveryRejections(project.competitivenessRecovery);
const performanceSource = await readFile(join(siteRoot, "data/performance.js"), "utf8");
vm.runInNewContext(performanceSource, context, { filename: "site/data/performance.js" });
validatePerformance(context.window.FERRIC_PERFORMANCE);
testPerformanceRejections(context.window.FERRIC_PERFORMANCE);

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
    "competitivenessSprint",
    "competitivenessFollowup",
    "competitivenessRecovery",
    "milestone",
    "readiness",
    "envelope",
    "capabilities",
    "validation",
    "teams",
    "boundaries",
    "batchEngineeringObservations",
    "engineeringObservations",
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
  siteRefreshBase: "4a3cbd4efe1df08ccdf7e3e59e19036f9f14469d",
  previousSiteRefreshBase: "0a8df6e3595c1ed3ae7fa15dd0edcd9e00986e76",
  performanceSprintV2Source: "0780becbe5ee3e4e92f53d42311866e79d3f03af",
  performanceSprintV2CoreCommit: "79706b43a177a2fd3fa43ec328221fa3e5041af5",
  performanceSprintV2CoreHostTests: 469,
  performanceSprintV2CoreDoctests: 31,
  performanceSprintV2NativeWorlds: [2, 8],
  performanceSprintV2NativeRows: [1, 3, 16],
  performanceSprintV2RoundsPerProbe: 12,
  performanceSprintV2NativeDispatches: [24, 96],
  performanceSprintV2AllOutputsExact: true,
  performanceSprintV2HostTimingOptIn: true,
  performanceSprintV2OverlapMeasured: false,
  performanceSprintV2Bf16TieObserved: true,
  performanceSprintV2Fp32FixedWorkloadPassed: true,
  performanceSprintV2Fp32BroadNumericalQualification: false,
  performanceSprintV2Fp32ModelObservations: 3,
  performanceSprintV2Fp32ModelRepetitions: 1,
  performanceSprintV2Fp32ModelReportSha256: "38ba909a53c3b6d4ae491b22ffbe56927ff50b484dd445fc048eb8184f55b97a",
  performanceSprintV2RoundVsSerialGainObserved: true,
  performanceSprintV2PeerBeatsHost: false,
  performanceSprintV2MatrixRepetitions: 1,
  performanceSprintV2IntegrationSource: "00fa58d6a1a9394433083b0f37b4f6365d5edb63",
  performanceSprintV2AdapterRustTests: 273,
  performanceSprintV2MetadataGraphs: 27,
  performanceSprintV2ImageBoundTests: 2,
  performanceSprintV2Fp32NativeCases: 9,
  performanceSprintV2Fp32NativeReportSha256: "fb2c23697b2fe144534df214225f44d14359f3f4fe6810ae766f11b07721b69b",
  tensorParallelBatchImplementationPrivate: true,
  tensorParallelBatchRows: 16,
  tensorParallelBatchMaxSequences: 32,
  tensorParallelBatchMaxBatches: 240,
  tensorParallelBatchMaxRequestedOutputs: 256,
  tensorParallelBatchPageTokens: 16,
  tensorParallelBatchMaxPhysicalPages: 512,
  tensorParallelBatchMaxContextTokens: 8192,
  tensorParallelBatchPoolTestsPassed: 18,
  tensorParallelBatchCoordinatorClosureTestsPassed: 46,
  tensorParallelBatchDriverTestsPassed: 6,
  tensorParallelBatchIntegratedLibraryTestsPassed: 139,
  tensorParallelBatchIntegratedLibraryTestsIgnored: 1,
  tensorParallelBatchIntegratedNewCliTestsPassed: 8,
  tensorParallelBatchIntegratedOldCliTestsPassed: 9,
  tensorParallelBatchIntegratedSourcePoliciesPassed: 22,
  tensorParallelBatchHostCheckScope: "private-integration-kernel-2048c10",
  tensorParallelBatchIntegratedStrictClippyPassed: true,
  tensorParallelBatchKernelTargetsEmitted: ["gfx942:xnack-", "gfx950:xnack-"],
  tensorParallelBatchRadixImplemented: true,
  tensorParallelBatchGpuObserved: true,
  tensorParallelBatchMetricsObserved: true,
  tensorParallelBatchBenchmarkComparable: false,
  tensorParallelBatchCacheSpeedupClaimed: false,
  tensorParallelBatchUncachedObserved: true,
  tensorParallelBatchAssurance: "Contracted",
  fe2o3A8RepinValidated: true,
  fe2o3A8LockedMetadataGraphsPassed: 15,
  fe2o3A8DependencyRecordsChecked: 7,
  mi350MemoryQueueLifecycleObserved: true,
  mi350MemoryQueueLifecycleGpuCount: 8,
  mi350KernelDispatchObserved: true,
  mi350SingleRmsNormObserved: true,
  mi350EngineeringRuntimeTestsPassed: 433,
  mi350ExecutableIdentity: "live_proc_exe_sha256",
  mi350DeviceObservationGpuCount: 8,
  mi350DeviceObservationProfileSha256: "6f859b0a67f8ee2497393206930ff35bf1a9ae69d33f172106a790a0c9226667",
  mi350DeviceObservationDescriptorCleanup: true,
  mi350KfdUnitTestsPassed: 420,
  mi350KfdIntegrationTestsPassed: 20,
  mi350KfdDoctestsPassed: 27,
  mi350QueueImplemented: true,
  mi350QwenExecuted: true,
  tensorParallelWorldSizes: [1, 2, 8],
  tensorParallelSourceCommit: "3674f36ba2a12165298e6085469887152424262b",
  tensorParallelRustTestsPassed: 9,
  tensorParallelVerusQueries: 36,
  tensorParallelVerusErrors: 0,
  tensorParallelBodyMutantsRejected: 8,
  tensorParallelExecutionSourceCommit: "3924efcbc93ae3c25ba649e7dc7ee8fc4dba543d",
  tensorParallelExecutionDriverTestsPassed: 8,
  tensorParallelExecutionCursorTestsPassed: 3,
  tensorParallelExecutionVerusQueries: 8,
  tensorParallelExecutionVerusErrors: 0,
  tensorParallelExecutionBodyMutantsRejected: 8,
  tensorParallelCursorSourceSha256: "86c2cb263cd87027bfd8ef66d4b66da640301a6637c9b6ca8a5f7794503af66e",
  tensorParallelCursorReceiptSha256: "a80aaed9bd9c7811c75b9935ac29214f33132da96c87de521263f8ed6fde9470",
  tensorParallelIntegratedHostTestsPassed: 648,
  tensorParallelIntegratedHostTestsIgnored: 9,
  tensorParallelIntegratedStrictClippyPassed: true,
  tensorParallelHostReductionImplemented: true,
  tensorParallelControllerIntegrated: true,
  tensorParallelExecutionRadixCacheEnabled: false,
  tensorParallelKernelSourceCommit: "8cdde149643446b20730bc60206669c5bab1ca8d",
  tensorParallelKernelCount: 13,
  tensorParallelKernelTestsPassedPerTarget: 31,
  tensorParallelKernelEmissionComplete: true,
  tensorParallelGfx950HsacoSha256: "7c0b1934a27569a97cf535c96a8a56dd57babb63a6d1becad1c7be3a3d26edec",
  tensorParallelGfx950ContentId: "994c81cde99c63de824ef53d29b2878e97d9a02d71901815039048da58c3c83c",
  tensorParallelSyntheticGpuProbesPassed: 6,
  tensorParallelSyntheticProbeResultSha256: "609e2b2afa40228fb112f7b4eb5afd35e5ff0523d15e293a434c5cc0459a09e7",
  tensorParallelProbeHostTestsPassed: 8,
  tensorParallelCollectiveTransportImplemented: true,
  tensorParallelMultiRankCollectiveObserved: true,
  tensorParallelQwenExecuted: true,
  tensorParallelFull32QwenExecuted: true,
  dualTargetKernelSourceCommit: "296bb98e62c3ce41c085c67aefd61ca667df8fee",
  gfx950EngineeringContentId: "431f1e294d5018e0f057d490495921a1983bac0c25e4e900c3f72a05350b3f84",
  gfx950HsacoSha256: "2679e59626eee9939412aaf7af6a542c8aeccbe4dd13fb7e6ea3bdcf4f3b8222",
  gfx950HsacoBytes: 103616,
  gfx950KernelCount: 12,
  gfx950GuardedStoreCount: 26,
  gfx950ExactReplayMatch: true,
  gfx950LoaderKernelsValidated: 12,
  gfx950ArtifactAuthority: "none",
  integrationCommit: "eb219f0f850dfa139d49c1cd303fd292539725fc",
  integrationTree: "046029d7be3043469b41ef8614ddd26beb8d4bbf",
  sha256FastPathCommit: "63e0f7fc4ed84f6c4717afc2a960e8334118c5b9",
  sha256FastPathVerusQueries: 296,
  sha256FastPathVerusErrors: 0,
  sha256FastPathReleaseTestsPassed: 143,
  sha256FastPathGeneratedRecordsExact: 7,
  sha256FastPathOneMiBSpeedupMinimum: 1.115,
  sha256FastPathOneMiBSpeedupMaximum: 1.119,
  sha256FastPathSmallChunkRegressionMinimumPercent: 0.49,
  sha256FastPathSmallChunkRegressionMaximumPercent: 1.53,
  sha256FastPathEndToEndGainClaimed: false,
  startupDiagnosticsCommit: "6ef78dc4534317384f7275115c5e77a8b1acb702",
  startupDiagnosticsDefaultEnabled: false,
  startupDiagnosticsStderrOnly: true,
  startupDiagnosticsChangesJson: false,
  startupDiagnosticsChangesControllerTiming: false,
  startupDiagnosticsSpeculativeTestsPassed: 10,
  startupDiagnosticsTargetTestsPassed: 17,
  startupDiagnosticsTargetHardwareIgnored: 1,
  startupDiagnosticsPolicyTestsPassed: 20,
  startupDiagnosticsStrictClippyGreen: true,
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
  combinedAttempt7SourceCommit: "a689418a737bf0b1cf71ec3742bca5702f083187",
  combinedAttempt7SourceTree: "ce9e0bc76238e569756473cf93ab8a4439062362",
  combinedAttempt7State: "stopped-at-cold-kv-negative-timeout",
  combinedAttempt7StrictProofPackagesPassed: 10,
  combinedAttempt7TimeoutSeconds: 600,
  combinedAttempt7ReceiptEmitted: false,
  combinedAttempt7LogSha256: "7200d867b34491373922bfe5399bccb66d1e6b2ee9bb99d00e1ad29175688fa4",
  combinedQualificationSourceCommit: "a689418a737bf0b1cf71ec3742bca5702f083187",
  combinedQualificationSourceTree: "ce9e0bc76238e569756473cf93ab8a4439062362",
  combinedQualificationAttempt: 8,
  combinedQualificationState: "stopped-at-nested-tmpdir-sun-len",
  combinedQualificationRunning: false,
  combinedQualificationStrictProofPackagesPassed: 10,
  combinedQualificationSameSourceNegativeProofsGreen: true,
  combinedQualificationSameSourceMutationsPassed: 37,
  combinedQualificationNegativePropertyPolicyGreen: true,
  combinedQualificationFmtGreen: true,
  combinedQualificationClippyGreen: true,
  combinedQualificationReceiptEmitted: false,
  combinedQualificationLogSha256: "92886d7b80a20fa3d7d451fc9f212cc313f5852213d6e3328eaa2923a2cc5027",
  combinedQualificationAppliesToCurrentIntegration: false,
  nextCombinedQualificationAttempt: 9,
  nextCombinedQualificationLaunched: false,
  combinedPreflightGreen: true,
  combinedPreflightSourceCommit: "a689418a737bf0b1cf71ec3742bca5702f083187",
  combinedPreflightSourceTree: "ce9e0bc76238e569756473cf93ab8a4439062362",
  combinedPreflightLogSha256: "6a78aca1ed6c7fc5822cec74f636591631f8d129f0c123e760634cc3a1b510b4",
  combinedPreflightCuratedAdmissions: 7229,
  workerSignerNarrowPreflightGreen: true,
  workerSignerNarrowPreflightLogSha256: "65805dc943fe8c1390e3aadbe5f1144d54af9b42756631795f18d59860e567be",
  workerSignerNarrowSourceGateTestsPassed: 33,
  r33RejectedCandidateCommit: "a305034b0e7f5ae8b204f066254601fc835592d6",
  r33RejectedCandidateTree: "42390274000b3a6674fc0043ced3fdd8f8fe7c59",
  r33RejectedCandidateReviewDisposition: "hold-steady-state-allocation",
  r33TwentyWindowCommit: "1e44a64b673f8ede149686979ef759757e71369b",
  r33TwentyWindowTree: "d9e9a381e2a23a3e948943a93abe281f7fd5a0c7",
  r33TwentyWindowImplemented: true,
  r33TwentyWindowReviewState: "hold-independent-review",
  r33TwentyWindowReviewHasProbableHoldItems: true,
  r33TwentyWindowActive: true,
  r33TwentyWindowIntegrated: false,
  r33TwentyWindowValidationReported: true,
  r33TwentyWindowValidationGreen: true,
  r33TwentyWindowHardwareQualified: false,
  r33TwentyWindowHardwareState: "blocked-independent-review-hold",
  r33MeasuredAllocationRounds: [1, 16],
  r33MeasuredAllocations: 0,
  r33MeasuredReallocations: 0,
  r33AllocatorTestCoversResidentPrepareCore: false,
  r33ResidentPrepareCoreStillAllocatesVec: true,
  r33FaultedResidentExplicitCloseProved: false,
  r33OutputBudgetBoundedToFixedBuffer: false,
  r33HostileMutationsMeaningful: false,
  r33BroaderRepairMeasuresFullWarmedRoute: true,
  r33BroaderRepairCoversSettlementAndRelease: true,
  r33BroaderRepairAddsExplicitFaultedClose: true,
  r33BroaderRepairTestsOutputBudgets: [128, 129],
  r33PriorCandidateAbbrev: "2ba02a8",
  r33PriorCandidateBaseAbbrev: "2958b69",
  r33PriorCandidateReviewState: "hold-allocation-free-claim-unaccepted",
  r33PriorCandidateMatrixGreen: true,
  r33PriorCandidateEngineAllTargetsPassed: 724,
  r33PriorCandidateEngineDoctestsPassed: 171,
  r33PriorCandidateAdapterAllTargetsPassed: 104,
  r33PriorCandidateSourceGateTestsPassed: 33,
  r33PriorCandidateAdmissionModules: 171,
  r33PriorCandidateAdmissionBodies: 8059,
  r33PriorCandidateNarrowZeroAllocationTestsPassed: 4,
  r33PriorCandidateNarrowMeasuredAllocations: 0,
  r33PriorCandidateNarrowMeasuredReallocations: 0,
  r33PriorCandidatePublicNextWindowAllocationTested: false,
  r33PriorCandidatePublicWarmedPathAllocationFree: false,
  r33CurrentRepairRequired: false,
  r33CurrentTwentyWindowAttempted: false,
  r33CurrentTwentyWindowBlocker: "absent-authenticated-authority-bundle",
  r33CurrentHardwareQualified: false,
  residentAllocationRepairCommit: "90a2ff6777252db6b134a8200a2e60c12547d7a7",
  residentAllocationRepairTree: "a1f68e3eea20222190c5ffe1046420bad39c6c41",
  residentAllocationTestCommit: "684d2f64c96e549a88193e9774489e5e2ec95cac",
  residentAllocationTestTree: "176daf27bc0c2e04444e5c6f44d2eb8076a6df44",
  residentSameShapeCoreTested: true,
  residentSameShapeOneRoundAllocations: 0,
  residentSameShapeOneRoundReallocations: 0,
  residentSameShapeRepeatedRounds: 16,
  residentSameShapeRepeatedAllocations: 0,
  residentSameShapeRepeatedReallocations: 0,
  residentAllocationPositiveControlObserved: true,
  residentWholePublicEntryAllocationFreeClaimed: false,
  currentHostEngineTestsPassed: 636,
  currentHostEngineHardwareIgnored: 9,
  currentHostEngineDoctestsPassed: 171,
  integratedEngineCheckGreen: true,
  integratedEngineStrictClippyGreen: true,
  integratedFocusedAllocationTestGreen: true,
  headStoreCommit: "fc5f86939cc826480aa008ddf69b044f3d63c585",
  headStoreFollowupCommit: "4d0486476c907ae681bf48d4ba57efb1394e78f5",
  headStoreReplayRepairCommit: "d05b2daffcc32a061f497e23db8292fb208f0e75",
  headStoreTree: "c46263cc73b02d4f41b7c57137954ba08bda9f83",
  headStoreReviewState: "accepted",
  headStoreReviewDisposition: "integrate",
  headStoreIntegrated: true,
  headStoreSessionReplayRepairCandidateExists: true,
  headStoreValidationReported: true,
  headStoreValidationGreen: true,
  headStoreValidationLogSha256: "d7d68e4b4a9c9ec2951a1f849d65573f16c00a893979eb2e00d79f732a10aa27",
  headStoreValidationManifestSha256: "21bf7ae6a2df53c7e5c18985d1352274b224d6655d7ccc17bba98d51582fac84",
  headStoreDaemonAvailable: false,
  headStoreDurabilityImplemented: false,
  headStoreSessionGeneratorStoreImplemented: false,
  headStoreLauncherImplemented: false,
  headStoreDistinctUidExercised: false,
  currentRecordIpcCommit: "5df8c5e463cca191b142321a9b5040ed66955f06",
  currentRecordIpcTree: "7c0c4a203903d1515cbe1451754db8d9b1ddd591",
  currentRecordIpcAuthorMatrixGreen: true,
  currentRecordIpcReviewState: "accepted",
  currentRecordIpcIntegrated: true,
  currentRecordIpcValidationLogSha256: "be88974e99748fccb5fd4e6f0c3122b3e9a1822db038fcbd3da8707b92d9d830",
  currentRecordIpcDaemonAvailable: false,
  currentRecordProviderCommit: "2958b6915cfbbd9bbb81bdb14471fe044b203e0f",
  currentRecordProviderTree: "48de4c1fe214b9d9df2576cde735f58b0dc63b25",
  currentRecordProviderReviewState: "accepted",
  currentRecordProviderIntegrated: true,
  currentRecordProviderValidationLogSha256: "3d9533b2482e3f8424059333495fe1384f9cd66287471755c5fce0e1528eaee0",
  currentRecordProviderValidationManifestSha256: "100ecefebf2496006c1c0603c274bdff399d63eae23a35ef44f72eede25688c1",
  currentRecordProviderDaemonAvailable: false,
  r29PlanSha256: "61fca5d4441acea3a4f5ca548b5fde142294153027b0b052b8ac32f27bd7af9c",
  r29InputFilesGenerated: 20,
  r29Buckets: 7,
  r29InputsValidated: true,
  r29FirstCaptureState: "completed-failed-closed-at-semantic-join",
  r29FirstCaptureElapsedSeconds: 1608,
  r29FirstCaptureStatus: 134,
  r29FirstCapturePhysicalCompletionObserved: true,
  r29FirstCaptureReadbackObserved: true,
  r29FirstCaptureTypedCustodyRetained: true,
  r29FirstCaptureVramReleased: true,
  r29FirstCaptureErrorBranchKnown: true,
  r29FirstCaptureTypedError: "QualificationFinalLogits(NonFinite { lane: 0, token: 0 })",
  r29ObservabilityFixActive: false,
  r29SameCaseRerunActive: false,
  r29StaleHsacoDate: "2026-08-26",
  r29StaleHsacoRepresentsCurrentAggregate: false,
  r29EngineeringAggregateArtifactExists: true,
  r29TargetSmokeCanBindAggregate: true,
  r29SevenCaseCaptureAcceptsAggregate: false,
  r29HonestAggregateIdentityConversionExists: false,
  r29AggregateTechnicalCaptureWiringActive: true,
  r29TechnicalPrequalificationOnly: true,
  r29FormalQualificationBlocked: true,
  r29ColdStartupHbmCostObserved: true,
  r29ColdHbmTargetInitApproxMinutes: 24,
  r29PriorCandidateAbbrev: "d7d2",
  r29PriorSharedTestsPassed: 75,
  r29PriorHardwareTestsIgnored: 2,
  r29PriorSourcePolicyTestsPassed: 17,
  r29PriorNativeTranscriptTestsPassed: 1,
  r29PriorProducerPolicyGreen: true,
  r29PriorVendorOverlayGreen: true,
  r29CurrentAggregateBuildState: "0ea-aggregate-emitted-target-smoke-observed",
  r29CurrentAggregateMissingPackage: null,
  r29CurrentAggregateHsaco: true,
  r29CurrentTargetSmokeCompleted: true,
  r29CurrentS1T128Captured: false,
  r29CurrentTtftMeasured: true,
  r29CurrentTpotMeasured: true,
  qwenInputBundlePrepared: true,
  freshQwenRerunActive: false,
  freshQwenRerunResultAvailable: true,
  gemmLayoutFixCommit: "a3c941cc5853c8d9d49aaa80456df82ab0672ef4",
  gemmLayoutFixTree: "3fe917fb4c6601453c74a97deaad529bc83d92ae",
  gemmLayoutFixIntegrated: true,
  invalidLayoutRunTtftSeconds: 14.36,
  invalidLayoutRunTpotSecondsApprox: 3.05,
  invalidLayoutRunComparable: false,
  invalidLayoutRunOutputValid: false,
  correctedTargetOnlyObservation: true,
  correctedOutputText: " Paris. The capital of Italy is Rome",
  correctedOutputTokenCount: 8,
  correctedHardwareCompletion: true,
  correctedHsacoSha256: "068e15991b97a829af6e77f2d105262e3ffc5fc69f1fa41a5ed5efdcae0da5e3",
  correctedModelBundleId: "6dfba0ac",
  correctedTtftSeconds: 13.649661699,
  correctedPostFirstSecondsPerToken: 2.7656050044,
  correctedControllerDurationSeconds: 34.094,
  correctedBenchmarkComparable: false,
  correctedAuthority: "none",
  residentAllocationRepairActive: false,
  productionOwnerRepairActive: false,
  productionOwnerCandidateComplete: true,
  productionOwnerCandidateIntegrated: true,
  productionOwnerSourceCommit: "c0b6c3676fbabce896a55874ef53864cb0898cfb",
  productionOwnerRepinCommit: "9cb42304bb4b647206d0d0e0e5d8a038aa2514f8",
  productionOwnerFocusedGatesGreen: true,
  k3WorkProportionalKvWriteState: "artifact-emitted-target-smoke-observed",
  k3WorkProportionalKvWriteIntegrated: true,
  k3WorkProportionalSpeedupMeasured: false,
  signerIpcCommit: "dcc7c07a3c21f8a3a8a6ac678987ba172c5809f4",
  signerIpcTree: "b524a9ccc981466ecde2aa31afa59e2dff672fc8",
  signerIpcIntegrated: true,
  signerIpcReviewDisposition: "integrate",
  signerIpcServiceTestsPassed: 58,
  signerIpcServiceTestsIgnored: 3,
  signerIpcAdapterTestsPassed: 88,
  signerIpcSourceGateTestsPassed: 28,
  signerIpcValidationLogSha256: "0a4283731228a40a46233ed00a2bb846fcbe47ce01f764a4cfd721add1d9ac73",
  signerIpcHardwareQualified: false,
  workerV3LintFixCommit: "b7a8545ed00ec948942690f89b2cbe891d46f836",
  workerV3LintFixTree: "42f408c0c602166edcc703563df7574324ad4ad2",
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
  fe2o3FerricPin: "cf6faec0ee3c026d3a1fc5090ab606a3b425225c",
  fe2o3FerricPinTree: "6d115af5cd5285b84b7629834393d6eee6a37045",
  fe2o3LatestMain: "21682228486f7186cc3c37ddf165fffc438d8b6a",
  fe2o3LatestTree: "e87e1433b7a519d3acd78859520549edd58bcfc0",
  fe2o3LatestHostGateCore: "6f6a67bb2f6de70a1c5533bbcde1b09449c37359",
  fe2o3PriorE535MigrationCommit: "aba3f86ef14136fa73a385834d4f33f7c9416a32",
  fe2o3PriorE535MigrationTree: "ea47a88d49ea67384299d1bc3054b09ba4567dd3",
  fe2o3LegacyRepinScope: "Historical a8b016e1 only: fe2o3CurrentRepin counters and ELF rebuild flags are retained receipts, not latest-source qualification.",
  fe2o3LatestHostGateSource: "29d756a97c3559442c46d9f1d5d1d673a2b7e838",
  fe2o3LatestHostGateReceiptSha256: "b2e5d5b107f3b1fc51bb9d472a0c98259f03fc904ffd315583d3b4db67c4a852",
  fe2o3LatestHostGateControllerSha256: "80fab5d4907d02e0e516e2801ee9604f47171f0f0832f466ac51f3ca777b573c",
  fe2o3LatestHostGateMetadataConfigurations: 27,
  fe2o3LatestHostGatePassed: true,
  fe2o3LatestHostGateGpuExecution: false,
  fe2o3LatestHostGateModelQualification: false,
  fe2o3CurrentRepinActive: false,
  fe2o3CurrentRepinIntegrated: true,
  fe2o3CurrentRepinFormalAcceptance: false,
  fe2o3CurrentRepinLockfiles: 15,
  fe2o3CurrentRepinUnchangedRecordsExact: true,
  fe2o3CurrentRepinChangedTcbRecords: 3,
  fe2o3CurrentRepinElfRebuiltAndBound: true,
  fe2o3CurrentRepinFinalHandoffPending: false,
  fe2o3PreexistingDcoAncestryConcern: true,
  fe2o3UnsignedUpstreamCommit1: "f405dfccb5b7021c417df37c0a692aead02fd071",
  fe2o3UnsignedUpstreamCommit2: "e5351640e3df3868205bc68eac8d5ff5556352ea",
  fe2o3LatestQualificationClaimed: false,
  currentAggregateContentId: "d33933adf7f5dfe0a9aa4aba0cc4cb3909b5aa35f9bfb65db7af9af4a5c5bb40",
  currentAggregateHsacoSha256: "33d754aaa10292fa37e974eb004b6c52067a141dcd5b58ed080023c6bf315c2d",
  currentAggregateHandoffSha256: "5d40961ec2fd365835867251330072674bdf16dc48a18c4c2bc446bcd60f28b8",
  currentAggregateGuardedStores: 26,
  currentAggregateAuthority: "none",
  currentAggregateHardwareRunPending: false,
  currentAggregateCompilerCommit: "0ea54ed921cbef5fb2171f4ed7a25b9c1c5e2b71",
  latestFe2o3AggregateEmitted: true,
  latestFe2o3AggregateBuildActive: false,
  latestFe2o3HostCombinedChecksActive: false,
  currentAggregateSourceCommit: "939275509bffda3fa09cf6995f8762837d86966c",
  currentAggregateSourceTree: "01bcdc62a2dba522c3c195390687629d7adfb3b2",
  currentAggregateManifestSha256: "dd741a81befc48e19243b18f624977e5fe57f857c9ff7466d295ae11a0245814",
  currentAggregateKernelCount: 12,
  currentAggregateExactReplay: true,
  currentAggregatePublicationGrant: false,
  currentAggregateLoadGrant: false,
  currentAggregateLaunchGrant: false,
  latestFe2o3HostCombinedChecksPassed: true,
  latestFe2o3HostCombinedSourceCommit: "a6a3f2e570bf5b6dfbbbec1dc004764918ffeb2d",
  latestFe2o3HostCombinedSourceTree: "f460d6f854ea8eab4d020a386f9fe273d1e64631",
  latestFe2o3HostCombinedSourceClosureRecords: 712,
  latestFe2o3HostCombinedSourceGateTestsPassed: 36,
  latestFe2o3HostCombinedGeneratedRecordsExact: 7,
  latestFe2o3HostCombinedAdmissionModules: 171,
  latestFe2o3HostCombinedAdmissionBodies: 8207,
  plannerTopologySourceCommit: "eb219f0f850dfa139d49c1cd303fd292539725fc",
  plannerTopologySourceTree: "046029d7be3043469b41ef8614ddd26beb8d4bbf",
  plannerTopologyPolicyPassed: true,
  plannerTopologyDeterministicSlots: 354,
  plannerTopologyProtectedReceiptEmitted: false,
  ferricQwen32RunActive: false,
  currentQualificationSourceCommit: "bb50d0e0c71e44271b892120c208650bd444f684",
  currentQualificationSourceTree: "7f3f23579b0adfac8ad7357fbf1246ab4b05dc95",
  currentQualificationState: "developer-qualification-passed-frozen-bb50",
  currentQualificationComponentMatrixPassed: true,
  currentQualificationFullRunStarted: true,
  currentQualificationRunning: false,
  currentQualificationReceiptEmitted: true,
  currentQualificationReceiptSha256: "c09f212e82eace9326a6d0a0e47ff7a898a8901e5e531fa01ea52213b064b54b",
  currentQualificationDeveloperOnly: true,
  currentQualificationAppliesToLatestIntegration: false,
  currentQualificationClosesProtectedM1Gates: false,
  currentQualificationPositivePackagesPassed: 10,
  currentQualificationPositiveQueries: 1586,
  currentQualificationPositiveErrors: 0,
  currentQualificationPositiveBodies: 694,
  currentQualificationNegativeActualBodyMutationsPassed: true,
  currentQualificationNegativeAndQualityComplete: true,
  prior428HostArtifactPreDeviceProbeSeconds: 98.23,
  prior428HostArtifactPreDeviceProbeIncludesKfdAdmission: true,
  prior428HostArtifactPreDeviceProbeIncludesTopologyEnumeration: true,
  prior428HostArtifactPreDeviceProbeIncludesInitialize: false,
  prior428HostArtifactPreDeviceProbeIncludesHbm: false,
  latestFe2o3KfdDescriptorProbeGib: 97.313,
  latestFe2o3KfdDescriptorProbeSeconds: 58.133,
  latestFe2o3KfdDescriptorProbeIncludesGpu: false,
  mi300xGpuCount: 8,
  mi300xQwen32GpuUseObserved: true,
  mi300xQwen32GpuIndex: 3,
  qwenReference32ResultSha256: "1ed868663df52a146dd7921f9fbb2cf1e1d0c0ceed8a7bfda030d65f8ec5b094",
  qwenReference32Available: true,
  ferricQwen32ComparisonComplete: true,
  ferricQwen32Status: 0,
  ferricQwen32PromptTokenCount: 5,
  ferricQwen32GeneratedTokenCount: 32,
  ferricQwen32GeneratedTokenIds: [12095, 13, 576, 6722, 315, 15344, 374, 21718, 13, 576, 6722, 315, 17689, 374, 24081, 13, 576, 6722, 315, 9856, 374, 19846, 13, 576, 6722, 315, 279, 25662, 374, 37741, 13, 576],
  ferricQwen32MatchesBothReferencePasses: true,
  ferricQwen32HardwareCompletion: true,
  ferricQwen32ObservationSha256: "d894caf042156abf21436c98fa3de7d40af124ba7374baa0b879bf7df582af44",
  ferricQwen32ModelBundleSha256: "6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b",
  ferricQwen32TtftSeconds: 13.442289487,
  ferricQwen32PostFirstSecondsPerToken: 2.769104887645161,
  ferricQwen32PostFirstTokenGaps: 31,
  ferricQwen32LastTokenSeconds: 99.284541004,
  ferricQwen32ControllerSeconds: 100.370762336,
  ferricQwen32SetupSeconds: 246.065540159,
  ferricQwen32CpuKfdSetupSeconds: 82.938930765,
  ferricQwen32InitializationSeconds: 163.126609394,
  ferricQwen32WallSeconds: 346.46,
  ferricQwen32Authority: "none",
  ferricQwen32BenchmarkComparable: false,
  ferricQwen32CompilerOriginAuthenticated: false,
  ferricQwen32CurrentPublicationSelected: false,
  ferricQwen32WorkerV3Authenticated: false,
  ferricQwen32R33TpotCardinalityEligible: true,
  ferricQwen32AuthenticatedR33: false,
  ferricQwen32NumericallyQualified: false,
  fe2o3KfdInitializationGatesGreen: true,
  fe2o3KfdOneGibBeforeSeconds: 76.512,
  fe2o3KfdOneGibAfterSeconds: 6.15,
  fe2o3KfdOneGibSpeedup: 12.44,
  fe2o3KfdLargeBytes: 16384000000,
  fe2o3KfdLargeBeforeSeconds: 1206.846,
  fe2o3KfdLargeAfterSeconds: 93.074,
  fe2o3KfdLargeSpeedup: 12.97,
  fe2o3KfdLargeSecondsSaved: 1113.772,
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
  engineeringSmokeBinaryStaged: false,
  canonicalQwenSnapshotVerified: true,
  radixPrefixIntegrated: true,
  currentAggregateHsaco: true,
  qwenTokenObserved: true,
  servingEndpointAvailable: false,
  baselineRunsAvailable: false,
  dockerAccessible: false,
  nativeBaselineInstallsAvailable: false,
  protectedInfrastructureDeployed: false,
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
  "performanceSprintV2Source",
  "performanceSprintV2IntegrationSource",
  "performanceSprintV2CoreCommit",
  "siteRefreshBase",
  "integrationCommit",
  "integrationTree",
  "sha256FastPathCommit",
  "startupDiagnosticsCommit",
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
  "combinedAttempt7SourceCommit",
  "combinedAttempt7SourceTree",
  "combinedQualificationSourceCommit",
  "combinedQualificationSourceTree",
  "combinedPreflightSourceCommit",
  "combinedPreflightSourceTree",
  "r33RejectedCandidateCommit",
  "r33RejectedCandidateTree",
  "r33TwentyWindowCommit",
  "r33TwentyWindowTree",
  "residentAllocationRepairCommit",
  "residentAllocationRepairTree",
  "residentAllocationTestCommit",
  "residentAllocationTestTree",
  "headStoreCommit",
  "headStoreFollowupCommit",
  "headStoreReplayRepairCommit",
  "headStoreTree",
  "currentRecordIpcCommit",
  "currentRecordIpcTree",
  "currentRecordProviderCommit",
  "currentRecordProviderTree",
  "signerIpcCommit",
  "signerIpcTree",
  "workerV3LintFixCommit",
  "workerV3LintFixTree",
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
  "fe2o3FerricPin",
  "fe2o3FerricPinTree",
  "fe2o3LatestMain",
  "fe2o3LatestTree",
  "gemmLayoutFixCommit",
  "gemmLayoutFixTree",
  "productionOwnerSourceCommit",
  "productionOwnerRepinCommit",
  "fe2o3PriorE535MigrationCommit",
  "fe2o3PriorE535MigrationTree",
  "fe2o3UnsignedUpstreamCommit1",
  "fe2o3UnsignedUpstreamCommit2",
  "speculativeAttempt2BaseCommit",
  "speculativeAttempt3Commit",
  "speculativeAttempt3Tree",
  "currentQualificationSourceCommit",
  "currentQualificationSourceTree",
  "currentAggregateCompilerCommit",
]) {
  assertCommit(project.current[key], `current.${key}`);
}
assertSha256(project.current.correctedHsacoSha256, "current.correctedHsacoSha256");
assertSha256(project.current.currentAggregateContentId, "current.currentAggregateContentId");
assertSha256(project.current.currentAggregateHsacoSha256, "current.currentAggregateHsacoSha256");
assertSha256(project.current.currentAggregateHandoffSha256, "current.currentAggregateHandoffSha256");
assertSha256(project.current.currentQualificationReceiptSha256, "current.currentQualificationReceiptSha256");
assertSha256(project.current.qwenReference32ResultSha256, "current.qwenReference32ResultSha256");
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
assert(project.current.engineeringSmokeBinaryStaged === false, "current engineering smoke must remain absent");
assert(project.current.canonicalQwenSnapshotVerified === true, "canonical Qwen snapshot status drifted");
assert(project.current.radixPrefixIntegrated === true, "integrated radix status drifted");
assert(project.current.currentAggregateHsaco === true, "current authority-free aggregate HSACO observation must remain explicit");
assert(project.current.qwenTokenObserved === true, "site must retain the diagnostic Qwen token observation");
assert(project.current.servingEndpointAvailable === false, "site must not claim serving");
assert(project.current.baselineRunsAvailable === false, "site must not claim baseline runs");
assert(project.current.dockerAccessible === false, "Docker must remain inaccessible for this checkpoint");
assert(project.current.nativeBaselineInstallsAvailable === false, "native baseline installs must remain absent");
assert(project.current.protectedInfrastructureDeployed === false, "protected infrastructure must remain absent");
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
assert(project.current.combinedAttempt7State === "stopped-at-cold-kv-negative-timeout", "combined #7 state drifted");
assert(project.current.combinedAttempt7StrictProofPackagesPassed === 10, "combined #7 strict proof count drifted");
assert(project.current.combinedAttempt7TimeoutSeconds === 600, "combined #7 timeout drifted");
assert(project.current.combinedAttempt7ReceiptEmitted === false, "combined #7 must not claim a receipt");
assertSha256(project.current.combinedAttempt7LogSha256, "current.combinedAttempt7LogSha256");
assert(project.current.combinedQualificationAttempt === 8, "combined qualification attempt drifted");
assert(project.current.combinedQualificationState === "stopped-at-nested-tmpdir-sun-len", "combined qualification state drifted");
assert(project.current.combinedQualificationRunning === false, "combined qualification #8 must not remain running");
assert(project.current.combinedQualificationStrictProofPackagesPassed === 10, "combined #8 strict proof count drifted");
assert(project.current.combinedQualificationSameSourceNegativeProofsGreen === true, "combined #8 same-source negative proofs must remain green");
assert(project.current.combinedQualificationSameSourceMutationsPassed === 37, "combined #8 mutation count drifted");
assert(project.current.combinedQualificationNegativePropertyPolicyGreen === true, "combined #8 negative/property policy must remain green");
assert(project.current.combinedQualificationFmtGreen === true, "combined #8 formatting must remain green");
assert(project.current.combinedQualificationClippyGreen === true, "combined #8 strict Clippy must remain green");
assert(project.current.combinedQualificationReceiptEmitted === false, "combined #8 must not claim a receipt");
assertSha256(project.current.combinedQualificationLogSha256, "current.combinedQualificationLogSha256");
assert(project.current.combinedQualificationAppliesToCurrentIntegration === false, "prior qualification must not be attributed to current integration");
assert(project.current.nextCombinedQualificationAttempt === 9, "next combined qualification attempt drifted");
assert(project.current.nextCombinedQualificationLaunched === false, "combined qualification #9 must remain unlaunched");
assert(project.current.combinedPreflightGreen === true, "combined qualification preflight must remain green");
assertSha256(project.current.combinedPreflightLogSha256, "current.combinedPreflightLogSha256");
assert(project.current.combinedPreflightCuratedAdmissions === 7229, "combined preflight admission count drifted");
assert(project.current.workerSignerNarrowPreflightGreen === true, "Worker and signer narrow preflight must remain green");
assertSha256(project.current.workerSignerNarrowPreflightLogSha256, "current.workerSignerNarrowPreflightLogSha256");
assert(project.current.workerSignerNarrowSourceGateTestsPassed === 33, "Worker and signer narrow source-gate count drifted");
assert(project.current.r33TwentyWindowImplemented === true, "20-window R33 candidate must remain implemented");
assert(project.current.r33RejectedCandidateReviewDisposition === "hold-steady-state-allocation", "rejected R33 disposition drifted");
assert(project.current.r33TwentyWindowReviewState === "hold-independent-review", "20-window R33 review state drifted");
assert(project.current.r33TwentyWindowReviewHasProbableHoldItems === true, "R33 HOLD findings must remain explicit");
assert(project.current.r33TwentyWindowActive === true, "20-window R33 fix work must remain active");
assert(project.current.r33TwentyWindowIntegrated === false, "20-window R33 work must remain unintegrated");
assert(project.current.r33TwentyWindowValidationReported === true, "20-window R33 validation result must remain explicit");
assert(project.current.r33TwentyWindowValidationGreen === true, "20-window R33 author matrix must remain green");
assert(project.current.r33TwentyWindowHardwareQualified === false, "20-window R33 work must remain hardware-unqualified");
assert(project.current.r33TwentyWindowHardwareState === "blocked-independent-review-hold", "R33 hardware blocker drifted");
assert(JSON.stringify(project.current.r33MeasuredAllocationRounds) === "[1,16]", "R33 measured allocation rounds drifted");
assert(project.current.r33MeasuredAllocations === 0, "R33 warmed path must retain zero measured allocations");
assert(project.current.r33MeasuredReallocations === 0, "R33 warmed path must retain zero measured reallocations");
assert(project.current.r33AllocatorTestCoversResidentPrepareCore === false, "narrow allocator test must not claim resident prepare coverage");
assert(project.current.r33ResidentPrepareCoreStillAllocatesVec === true, "real resident prepare allocation must remain explicit");
assert(project.current.r33FaultedResidentExplicitCloseProved === false, "faulted resident close must remain unproved");
assert(project.current.r33OutputBudgetBoundedToFixedBuffer === false, "R33 output-budget bound must remain open");
assert(project.current.r33HostileMutationsMeaningful === false, "textual hostile mutations must not claim semantic coverage");
assert(project.current.r33BroaderRepairMeasuresFullWarmedRoute === true, "R33 repair must measure the full warmed route");
assert(project.current.r33BroaderRepairCoversSettlementAndRelease === true, "R33 repair must include settlement and release");
assert(project.current.r33BroaderRepairAddsExplicitFaultedClose === true, "R33 repair must include explicit faulted close");
assert(JSON.stringify(project.current.r33BroaderRepairTestsOutputBudgets) === "[128,129]", "R33 repair budget tests drifted");
assert(/^[0-9a-f]{7}$/.test(project.current.r33PriorCandidateAbbrev), "prior R33 candidate abbreviation drifted");
assert(/^[0-9a-f]{7}$/.test(project.current.r33PriorCandidateBaseAbbrev), "prior R33 base abbreviation drifted");
assert(project.current.r33PriorCandidateReviewState === "hold-allocation-free-claim-unaccepted", "prior R33 review state drifted");
assert(project.current.r33PriorCandidateMatrixGreen === true, "prior R33 matrix must remain green");
assert(project.current.r33PriorCandidateEngineAllTargetsPassed === 724, "prior R33 engine matrix count drifted");
assert(project.current.r33PriorCandidateEngineDoctestsPassed === 171, "prior R33 doctest count drifted");
assert(project.current.r33PriorCandidateAdapterAllTargetsPassed === 104, "prior R33 adapter matrix count drifted");
assert(project.current.r33PriorCandidateSourceGateTestsPassed === 33, "prior R33 source-gate count drifted");
assert(project.current.r33PriorCandidateAdmissionModules === 171, "prior R33 admission module count drifted");
assert(project.current.r33PriorCandidateAdmissionBodies === 8059, "prior R33 admission body count drifted");
assert(project.current.r33PriorCandidateNarrowZeroAllocationTestsPassed === 4, "prior R33 narrow allocator test count drifted");
assert(project.current.r33PriorCandidateNarrowMeasuredAllocations === 0, "prior R33 narrow allocation count drifted");
assert(project.current.r33PriorCandidateNarrowMeasuredReallocations === 0, "prior R33 narrow reallocation count drifted");
assert(project.current.r33PriorCandidatePublicNextWindowAllocationTested === false, "prior R33 narrow tests must not claim public next-window coverage");
assert(project.current.r33PriorCandidatePublicWarmedPathAllocationFree === false, "prior R33 public warmed path must not claim allocation freedom");
assert(project.current.r33CurrentRepairRequired === false, "integrated R33 allocation repair must remain recorded");
assert(project.current.r33CurrentTwentyWindowAttempted === false, "R33 20-window hardware run must remain unattempted");
assert(project.current.r33CurrentTwentyWindowBlocker === "absent-authenticated-authority-bundle", "R33 20-window blocker drifted");
assert(project.current.r33CurrentHardwareQualified === false, "current R33 candidate must remain hardware-unqualified");
assert(project.current.residentSameShapeCoreTested === true, "production-used resident same-shape core coverage must remain explicit");
assert(project.current.residentSameShapeOneRoundAllocations === 0, "one-round allocation count drifted");
assert(project.current.residentSameShapeOneRoundReallocations === 0, "one-round reallocation count drifted");
assert(project.current.residentSameShapeRepeatedRounds === 16, "repeated same-shape round count drifted");
assert(project.current.residentSameShapeRepeatedAllocations === 0, "repeated-round allocation count drifted");
assert(project.current.residentSameShapeRepeatedReallocations === 0, "repeated-round reallocation count drifted");
assert(project.current.residentAllocationPositiveControlObserved === true, "allocator positive control must remain explicit");
assert(project.current.residentWholePublicEntryAllocationFreeClaimed === false, "scoped allocation evidence must not become a whole-entry claim");
assert(project.current.currentHostEngineTestsPassed === 636, "current host engine test count drifted");
assert(project.current.currentHostEngineHardwareIgnored === 9, "current host hardware-ignore count drifted");
assert(project.current.currentHostEngineDoctestsPassed === 171, "current host doctest count drifted");
assert(project.current.integratedEngineCheckGreen === true, "integrated engine check must remain green");
assert(project.current.integratedEngineStrictClippyGreen === true, "integrated strict Clippy must remain green");
assert(project.current.integratedFocusedAllocationTestGreen === true, "integrated focused allocation test must remain green");
assert(project.current.headStoreReviewState === "accepted", "head-store review state drifted");
assert(project.current.headStoreReviewDisposition === "integrate", "head-store review disposition drifted");
assert(project.current.headStoreIntegrated === true, "accepted head-store work must remain integrated");
assert(project.current.headStoreSessionReplayRepairCandidateExists === true, "head-store session-replay repair candidate must remain explicit");
assert(project.current.headStoreValidationReported === true, "head-store validation result must remain explicit");
assert(project.current.headStoreValidationGreen === true, "head-store validation must remain green");
assertSha256(project.current.headStoreValidationLogSha256, "current.headStoreValidationLogSha256");
assertSha256(project.current.headStoreValidationManifestSha256, "current.headStoreValidationManifestSha256");
assert(project.current.headStoreDaemonAvailable === false, "head-store must not claim a daemon");
assert(project.current.headStoreDurabilityImplemented === false, "head-store must not claim durable storage");
assert(project.current.headStoreSessionGeneratorStoreImplemented === false, "head-store must not claim a session generator store");
assert(project.current.headStoreLauncherImplemented === false, "head-store must not claim a launcher");
assert(project.current.headStoreDistinctUidExercised === false, "head-store must not claim distinct-UID exercise");
assert(project.current.currentRecordIpcAuthorMatrixGreen === true, "current-record IPC author matrix must remain green");
assert(project.current.currentRecordIpcReviewState === "accepted", "current-record IPC review state drifted");
assert(project.current.currentRecordIpcIntegrated === true, "accepted current-record IPC must remain integrated");
assertSha256(project.current.currentRecordIpcValidationLogSha256, "current.currentRecordIpcValidationLogSha256");
assert(project.current.currentRecordIpcDaemonAvailable === false, "current-record IPC must not claim a daemon");
assert(project.current.currentRecordProviderReviewState === "accepted", "current-record provider review state drifted");
assert(project.current.currentRecordProviderIntegrated === true, "current-record provider must remain integrated");
assertSha256(project.current.currentRecordProviderValidationLogSha256, "current.currentRecordProviderValidationLogSha256");
assertSha256(project.current.currentRecordProviderValidationManifestSha256, "current.currentRecordProviderValidationManifestSha256");
assert(project.current.currentRecordProviderDaemonAvailable === false, "current-record provider must not claim production daemon deployment");
assertSha256(project.current.r29PlanSha256, "current.r29PlanSha256");
assert(project.current.r29InputFilesGenerated === 20, "R29 input count drifted");
assert(project.current.r29Buckets === 7, "R29 bucket count drifted");
assert(project.current.r29InputsValidated === true, "R29 inputs must remain validated");
assert(project.current.r29FirstCaptureState === "completed-failed-closed-at-semantic-join", "R29 first capture state drifted");
assert(project.current.r29FirstCaptureElapsedSeconds === 1608, "R29 diagnostic rerun elapsed time drifted");
assert(project.current.r29FirstCaptureStatus === 134, "R29 first capture status drifted");
assert(project.current.r29FirstCapturePhysicalCompletionObserved === true, "R29 physical completion must remain explicit");
assert(project.current.r29FirstCaptureReadbackObserved === true, "R29 readback must remain explicit");
assert(project.current.r29FirstCaptureTypedCustodyRetained === true, "R29 typed custody must remain retained");
assert(project.current.r29FirstCaptureVramReleased === true, "R29 VRAM release must remain explicit");
assert(project.current.r29FirstCaptureErrorBranchKnown === true, "R29 typed error branch must remain known");
assert(project.current.r29FirstCaptureTypedError === "QualificationFinalLogits(NonFinite { lane: 0, token: 0 })", "R29 typed error drifted");
assert(project.current.r29ObservabilityFixActive === false, "R29 observability fix must not remain active after the typed rerun");
assert(project.current.r29SameCaseRerunActive === false, "R29 stale-artifact rerun must not remain active");
assert(project.current.r29StaleHsacoDate === "2026-08-26", "R29 stale HSACO date drifted");
assert(project.current.r29StaleHsacoRepresentsCurrentAggregate === false, "stale HSACO must not represent current aggregate kernels");
assert(project.current.r29EngineeringAggregateArtifactExists === true, "engineering aggregate artifact must remain explicit");
assert(project.current.r29TargetSmokeCanBindAggregate === true, "target smoke aggregate binding must remain explicit");
assert(project.current.r29SevenCaseCaptureAcceptsAggregate === false, "seven-case capture must not claim aggregate acceptance");
assert(project.current.r29HonestAggregateIdentityConversionExists === false, "aggregate identity conversion must remain absent");
assert(project.current.r29AggregateTechnicalCaptureWiringActive === true, "aggregate technical capture wiring must remain active");
assert(project.current.r29TechnicalPrequalificationOnly === true, "R29 must remain technical prequalification");
assert(project.current.r29FormalQualificationBlocked === true, "R29 formal qualification blocker must remain explicit");
assert(project.current.r29ColdStartupHbmCostObserved === true, "R29 cold HBM cost observation must remain explicit");
assert(project.current.r29ColdHbmTargetInitApproxMinutes === 24, "R29 cold HBM target initialization estimate drifted");
assert(project.current.r29PriorCandidateAbbrev === "d7d2", "prior R29 candidate abbreviation drifted");
assert(project.current.r29PriorSharedTestsPassed === 75, "prior R29 shared-test count drifted");
assert(project.current.r29PriorHardwareTestsIgnored === 2, "prior R29 ignored hardware-test count drifted");
assert(project.current.r29PriorSourcePolicyTestsPassed === 17, "prior R29 source-policy count drifted");
assert(project.current.r29PriorNativeTranscriptTestsPassed === 1, "prior R29 transcript-test count drifted");
assert(project.current.r29PriorProducerPolicyGreen === true, "prior R29 producer policy must remain green");
assert(project.current.r29PriorVendorOverlayGreen === true, "prior R29 vendor overlay must remain green");
assert(project.current.r29CurrentAggregateBuildState === "0ea-aggregate-emitted-target-smoke-observed", "current R29 aggregate state drifted");
assert(project.current.r29CurrentAggregateMissingPackage === null, "cleared R29 vendor blocker must remain cleared");
for (const key of [
  "r29CurrentS1T128Captured",
]) {
  assert(project.current[key] === false, `${key} must remain false`);
}
for (const key of [
  "r29CurrentAggregateHsaco",
  "r29CurrentTargetSmokeCompleted",
  "r29CurrentTtftMeasured",
  "r29CurrentTpotMeasured",
  "qwenInputBundlePrepared",
  "freshQwenRerunResultAvailable",
  "correctedTargetOnlyObservation",
  "correctedHardwareCompletion",
]) {
  assert(project.current[key] === true, `${key} must remain true`);
}
assert(project.current.freshQwenRerunActive === false, "completed corrected-layout rerun must not remain active");
assert(project.current.gemmLayoutFixIntegrated === true, "GEMM layout repair must remain integrated");
assert(project.current.invalidLayoutRunTtftSeconds === 14.36, "invalid-layout TTFT observation drifted");
assert(project.current.invalidLayoutRunTpotSecondsApprox === 3.05, "invalid-layout TPOT observation drifted");
assert(project.current.invalidLayoutRunComparable === false, "invalid-layout timing must remain noncomparable");
assert(project.current.invalidLayoutRunOutputValid === false, "invalid-layout output must remain invalid");
assert(project.current.correctedOutputText === " Paris. The capital of Italy is Rome", "corrected Qwen output drifted");
assert(project.current.correctedOutputTokenCount === 8, "corrected Qwen token count drifted");
assert(project.current.correctedModelBundleId === "6dfba0ac", "corrected model bundle identity drifted");
assert(project.current.correctedTtftSeconds === 13.649661699, "corrected diagnostic TTFT drifted");
assert(project.current.correctedPostFirstSecondsPerToken === 2.7656050044, "corrected diagnostic post-first-token timing drifted");
assert(project.current.correctedControllerDurationSeconds === 34.094, "corrected controller duration drifted");
assert(project.current.correctedBenchmarkComparable === false, "corrected observation must remain nonbenchmark");
assert(project.current.correctedAuthority === "none", "corrected observation authority must remain none");
assert(project.current.residentAllocationRepairActive === false, "integrated resident allocation repair must not remain active");
assert(project.current.productionOwnerRepairActive === false, "completed production-owner repair must not remain active");
assert(project.current.productionOwnerCandidateComplete === true, "production-owner source candidate must remain complete");
assert(project.current.productionOwnerCandidateIntegrated === true, "production owner must remain integrated");
assert(project.current.productionOwnerFocusedGatesGreen === true, "production-owner focused gates must remain green");
assert(project.current.k3WorkProportionalKvWriteState === "artifact-emitted-target-smoke-observed", "K3 validation state drifted");
assert(project.current.k3WorkProportionalKvWriteIntegrated === true, "K3 source must remain integrated");
assert(project.current.k3WorkProportionalSpeedupMeasured === false, "K3 must not claim a measured speedup");
assert(project.current.fe2o3CurrentRepinActive === false, "integrated fe2o3 repin must not remain active");
assert(project.current.fe2o3CurrentRepinIntegrated === true, "latest fe2o3 repin must remain integrated");
assert(project.current.fe2o3CurrentRepinFormalAcceptance === false, "current fe2o3 repin must not claim formal acceptance");
assert(project.current.fe2o3CurrentRepinLockfiles === 15, "active fe2o3 lockfile pin count drifted");
assert(project.current.fe2o3CurrentRepinUnchangedRecordsExact === true, "unchanged repin records must remain byte-exact");
assert(project.current.fe2o3CurrentRepinChangedTcbRecords === 3, "changed repin TCB record count drifted");
assert(project.current.fe2o3CurrentRepinElfRebuiltAndBound === true, "source-pinned ELF rebuild and binding check must remain explicit");
assert(project.current.fe2o3CurrentRepinFinalHandoffPending === false, "integrated repin final handoff must not remain pending");
assert(project.current.fe2o3PreexistingDcoAncestryConcern === true, "pre-existing fe2o3 DCO ancestry concern must remain explicit");
assert(project.current.fe2o3LatestQualificationClaimed === false, "latest-fe2 qualification must remain unclaimed");
assertSha256(project.current.performanceSprintV2Fp32NativeReportSha256, "current.performanceSprintV2Fp32NativeReportSha256");
assertSha256(project.current.performanceSprintV2Fp32ModelReportSha256, "current.performanceSprintV2Fp32ModelReportSha256");
assert(project.current.currentAggregateGuardedStores === 26, "current aggregate GuardedStore count drifted");
assert(project.current.currentAggregateAuthority === "none", "current aggregate must remain authority-free");
assert(project.current.currentAggregateHardwareRunPending === false, "completed aggregate hardware run must not remain pending");
assert(project.current.currentQualificationState === "developer-qualification-passed-frozen-bb50", "current-source qualification state drifted");
assert(project.current.currentQualificationComponentMatrixPassed === true, "current-source component matrix must remain passed");
assert(project.current.currentQualificationFullRunStarted === true, "full current-source qualifier must remain started");
assert(project.current.currentQualificationRunning === false, "completed frozen-source developer qualifier must not remain in progress");
assert(project.current.currentQualificationReceiptEmitted === true, "frozen-source developer qualifier receipt must remain explicit");
assert(project.current.currentQualificationDeveloperOnly === true, "bb50 qualification must remain developer-only");
assert(project.current.currentQualificationAppliesToLatestIntegration === false, "bb50 qualification must not apply to later integration");
assert(project.current.currentQualificationClosesProtectedM1Gates === false, "developer qualification must not close protected M1 gates");
assert(project.current.currentQualificationPositivePackagesPassed === 10, "positive proof package count drifted");
assert(project.current.currentQualificationPositiveQueries === 1586, "positive proof query count drifted");
assert(project.current.currentQualificationPositiveErrors === 0, "positive proof errors must remain zero");
assert(project.current.currentQualificationPositiveBodies === 694, "positive proof body count drifted");
assert(project.current.currentQualificationNegativeActualBodyMutationsPassed === true, "negative actual-body semantic mutations must remain passed");
assert(project.current.currentQualificationNegativeAndQualityComplete === true, "completed developer qualifier gates must remain complete");
assert(project.current.latestFe2o3AggregateEmitted === true, "the emitted public-0ea aggregate must remain recorded");
assert(project.current.latestFe2o3AggregateBuildActive === false, "completed public-0ea aggregate build must not remain active");
assert(project.current.latestFe2o3HostCombinedChecksActive === false, "completed host checks must not remain active");
assert(project.current.prior428HostArtifactPreDeviceProbeIncludesKfdAdmission === true, "pre-device probe must retain KFD admission");
assert(project.current.prior428HostArtifactPreDeviceProbeIncludesTopologyEnumeration === true, "pre-device probe must retain topology enumeration");
assert(project.current.prior428HostArtifactPreDeviceProbeIncludesInitialize === false, "pre-device probe must exclude initialization");
assert(project.current.prior428HostArtifactPreDeviceProbeIncludesHbm === false, "pre-device probe must exclude HBM upload");
assert(project.current.latestFe2o3KfdDescriptorProbeIncludesGpu === false, "descriptor CPU probe must exclude GPU work");
assert(project.current.mi300xGpuCount === 8, "shared MI300X count drifted");
assert(project.current.mi300xQwen32GpuUseObserved === true, "the exact GPU 3 observation must remain scoped to the completed run");
assert(project.current.sha256FastPathEndToEndGainClaimed === false, "SHA microbenchmark must not become an end-to-end gain claim");
assert(project.current.startupDiagnosticsDefaultEnabled === false, "engineering diagnostics must remain opt-in");
assert(project.current.startupDiagnosticsStderrOnly === true, "engineering diagnostics must remain stderr-only");
assert(project.current.startupDiagnosticsChangesJson === false, "engineering diagnostics must not change JSON");
assert(project.current.startupDiagnosticsChangesControllerTiming === false, "engineering diagnostics must not change controller timing");
assert(project.current.qwenReference32Available === true, "32-token Qwen reference must remain available");
assert(project.current.ferricQwen32ComparisonComplete === true, "Ferric 32-token comparison must remain complete");
assert(project.current.ferricQwen32RunActive === false, "completed 32-token run must not remain active");
assert(project.current.latestFe2o3HostCombinedChecksPassed === true, "combined host PASS must remain explicit");
assert(project.current.plannerTopologyPolicyPassed === true, "integrated planner PASS must remain explicit");
assert(project.current.plannerTopologyProtectedReceiptEmitted === false, "planner policy must not become a protected receipt");
assert(project.current.currentAggregateCompilerCommit === "0ea54ed921cbef5fb2171f4ed7a25b9c1c5e2b71", "historical MI300X Qwen32 aggregate must retain its actual public-0ea compiler, not latest main");
assert(project.current.currentAggregateSourceCommit !== project.current.integrationCommit, "artifact emission source must remain distinct from later integration");
assert(project.current.latestFe2o3HostCombinedSourceCommit !== project.current.integrationCommit, "runtime host source must remain distinct from later integration");
assert(project.current.ferricQwen32GeneratedTokenIds.length === 32, "hardware observation must retain exactly 32 IDs");
assert(project.current.ferricQwen32PostFirstTokenGaps === project.current.ferricQwen32GeneratedTokenCount - 1, "mean gap denominator must be 31");
assert(Math.abs((project.current.ferricQwen32LastTokenSeconds - project.current.ferricQwen32TtftSeconds) / 31 - project.current.ferricQwen32PostFirstSecondsPerToken) < 1e-12, "diagnostic post-first timing arithmetic drifted");
assert(Math.abs(project.current.ferricQwen32CpuKfdSetupSeconds + project.current.ferricQwen32InitializationSeconds - project.current.ferricQwen32SetupSeconds) < 1e-12, "setup boundary arithmetic drifted");
assert(project.current.ferricQwen32R33TpotCardinalityEligible === true && project.current.ferricQwen32AuthenticatedR33 === false, "arithmetic cardinality must not imply R33 authority");
assert(project.current.ferricQwen32BenchmarkComparable === false && project.current.ferricQwen32NumericallyQualified === false, "single-prompt match must not imply a benchmark or numerical qualification");
for (const key of ["currentAggregatePublicationGrant", "currentAggregateLoadGrant", "currentAggregateLaunchGrant", "ferricQwen32CompilerOriginAuthenticated", "ferricQwen32CurrentPublicationSelected", "ferricQwen32WorkerV3Authenticated"]) {
  assert(project.current[key] === false, `${key} must remain false`);
}
assert(project.current.fe2o3KfdInitializationGatesGreen === true, "KFD initialization gates must remain green");
assert(project.current.fe2o3KfdOneGibBeforeSeconds === 76.512, "1 GiB prior timing drifted");
assert(project.current.fe2o3KfdOneGibAfterSeconds === 6.15, "1 GiB current timing drifted");
assert(project.current.fe2o3KfdOneGibSpeedup === 12.44, "1 GiB speedup drifted");
assert(project.current.fe2o3KfdLargeBytes === 16384000000, "large KFD initialization byte count drifted");
assert(project.current.fe2o3KfdLargeBeforeSeconds === 1206.846, "large prior timing drifted");
assert(project.current.fe2o3KfdLargeAfterSeconds === 93.074, "large current timing drifted");
assert(project.current.fe2o3KfdLargeSpeedup === 12.97, "large speedup drifted");
assert(project.current.fe2o3KfdLargeSecondsSaved === 1113.772, "large saved-time total drifted");
assert(project.current.signerIpcIntegrated === true, "accepted signer IPC slice must remain integrated");
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
assert(project.current.tensorParallelBatchGpuObserved === true,
  "bounded GPU observation must remain recorded");
assert(project.current.tensorParallelBatchBenchmarkComparable === false
  && project.current.tensorParallelBatchCacheSpeedupClaimed === false,
  "single logical-tick observations must not become a benchmark or cache speedup");
assert(project.current.tensorParallelBatchAssurance === "Contracted",
  "new host tests do not extend prior Verus or protected authority");
for (const [title, state] of [["Continuous batching and paged TP attention", "observed"],
  ["Persistent resident radix prefixes", "observed"], ["Batched host checks", "integration"]]) {
  assert(project.readiness.some((item) => item.label === title && item.state === state),
    `new private batch status drifted: ${title}`);
}

assert(Array.isArray(project.readiness) && project.readiness.length >= 5, "readiness roster is incomplete");
assert(project.readiness[0].label === "Ordered batches: recovery with a fresh serial control"
  && project.readiness[0].state === "observed"
  && project.readiness[1].label === "Standalone draft reference and fast-kernel fixtures"
  && project.readiness[1].state === "observed",
  "latest scoped sprint observations must precede historical checkpoints");
assert(project.readiness[project.competitivenessRecovery.currentReadinessCount].label === "Checked concurrent-rank rounds",
  "history separator must follow all current sprint readiness entries");
assert(project.current.fe2o3LatestHostGateCore !== project.current.fe2o3LatestMain,
  "the historical 6f6 host receipt must not be relabeled as current ordered-batch source");
for (const [title, state, claim] of [
  ["Sustained ingress and loopback HTTP", "observed", "four sequential requests and nine frozen-reference outputs"],
  ["32-row FP32 head: bounded model passes", "observed", "actual maximum rows are 16 and 17, not 32"],
  ["Shared peer currentness: native only", "observed", "four native cases, not a model-speed"],
  ["Admission cache: frozen TP1 canary", "observed", "not steady-state serving"],
  ["Large KV pool: bounded model pass", "observed", "Long-context and 32-request concurrency remain unqualified"],
  ["Wave attention with FP32 head: tiny canary", "observed", "not a steady-state HTTP test"],
  ["Ordered batches: recovery with a fresh serial control", "observed", "Serial controls vary substantially"],
  ["Standalone draft reference and fast-kernel fixtures", "observed", "not paged draft model qualification"],
  ["Exact 656 CPU integration checkpoint", "integration", "adds no GPU, emission, numerical, new Verus"],
  ["Continuous V3 measurement, not a serving result", "implemented", "There is no drain between windows"],
  ["Earlier ordered-batch native checkpoint", "observed", "Legacy serial DispatchSequence behavior is unchanged"],
  ["Polling experiments: regression retained", "observed", "lowers mean fixed-canary rate 18.35%"],
  ["Earlier authenticated draft-intake checkpoint", "implemented", "no model-backed success fixture was run"],
]) {
  const row = project.readiness.find((item) => item.label === title);
  assert(row && row.state === state, `competitiveness status drifted: ${title}`);
  assert(row.detail.includes(claim), `competitiveness qualification boundary missing: ${title}`);
}
project.readiness.forEach((item, index) => {
  assertExactKeys(item, ["label", "state", "detail"], `readiness[${index}]`);
  assertState(item.state, `readiness[${index}].state`);
});

assert(Array.isArray(project.envelope) && project.envelope.length >= 8, "M1 envelope is incomplete");
const publicHeadRow = project.envelope.find(([label]) => label === "fe2o3 current public main");
assert(publicHeadRow && publicHeadRow[1].startsWith(`Observed cutoff: ${project.current.fe2o3LatestMain}; tree ${project.current.fe2o3LatestTree}.`),
  "public-main evidence row must agree with the exact current commit/tree fields");
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
  if (team.name === "Integration") {
    assert(team.status === "Authority paths under review", "Integration must expose the active authority work");
    assert(
      team.blockedBy.startsWith("R33 still needs the external authenticated authority bundle"),
      "Integration blocker must remain exact",
    );
  } else {
    assert(team.status.includes("no team blocker"), `${team.name} must report current blocker state`);
    assert(team.blockedBy.startsWith("No team-local blocker."), `${team.name} dependencies must remain explicit`);
  }
  teamNames.add(team.name);
});

assertExactKeys(project.boundaries, ["ferric", "fe2o3"], "boundaries");
for (const key of ["ferric", "fe2o3"]) {
  assert(Array.isArray(project.boundaries[key]) && project.boundaries[key].length >= 5, `${key} boundary is incomplete`);
}

const batch = project.batchEngineeringObservations;
assertExactKeys(batch, ["title", "scope", "authority", "implementationSource", "comparatorSource",
  "kernelSource", "controllerSha256", "workerSha256", "hsacoSha256", "referenceSha256",
  "workloadSha256", "cacheOffPending", "outputs", "runs"], "batchEngineeringObservations");
assert(batch.authority === "none" && batch.cacheOffPending === false,
  "bounded cached/uncached observation must not claim protected authority");
const batchIdentities = {
  implementationSource: "7224c33deba9dea0fcd82f94dcd2a07db0ca5319",
  comparatorSource: "65cb43532e265e9dcf02b36aebeb857779f81ceb",
  kernelSource: "2048c103ca6f62162f9d4e02b7cefa340bf0f8a8",
  controllerSha256: "098be2f8425bffcc46545ba66f313a3c63090eb01360eccafb276d887350e151",
  workerSha256: "77a53d18b56e4ee7a67a434feffa8ac18a4fe60f8c9e5daace351f502c72e1da",
  hsacoSha256: "af5019d3cfc4e860b33ebf0d97a82439f870a9e4e893c730118a97d8735c2d6a",
  referenceSha256: "1ed868663df52a146dd7921f9fbb2cf1e1d0c0ceed8a7bfda030d65f8ec5b094",
  workloadSha256: "23882e195cf578b987c72fc5703fffb51541708e5431d4166e55e0fe7e2f137f",
};
for (const [key, value] of Object.entries(batchIdentities)) {
  assert(batch[key] === value, `batch observation ${key} drifted`);
}
assert(JSON.stringify(batch.outputs) === JSON.stringify([
  { name: "seed-prefix", state: "Completed", tokenIds: [17689, 374], text: " Spain is" },
  { name: "arriving-short", state: "Completed", tokenIds: [12095, 13, 576], text: " Paris. The" },
  { name: "cancel-between-batches", state: "Cancelled after one output", tokenIds: [12095], text: " Paris" },
  { name: "reuse-prefix", state: "Completed", tokenIds: [24081, 13], text: " Madrid." },
]), "fixed batched workload output IDs, bytes or cancellation drifted");
const batchRuns = [
  [1, true, 11569919014, 5433362703, 4794532333, 156.218936794, 185.472480546,
    "d22bc4b35fe40e4f41614e760ea44083ce8f55affdf146be183c767e601c0b73",
    "e88cc980e6e7911575c0d2cf9d8a964a45526f0df961ae1b20cd3f261f412d5e"],
  [8, true, 113270822031, 47325504628, 47801794108, 189.758860743, 466.414186138,
    "562cf9d5fd8dbb737ddc92ef928913f06625f6f6992ddac66796e90b80595981",
    "340b2bd61a8b05bbdca45f4ed9151b28acd8a2f1ee4351b7310e76ccfc456460"],
  [8, false, 85731951175, 86270679460, 37697434748, 192.698331733, 461.166303364,
    "d7d7d646374b7ec07e190d8f17d06772829a30c1c6089ef566f45e9e0e57ca87",
    "a13146920a0ad686b7f477e4605fc5af29c3d1988b31d862f5448e51036b44ac"],
];
assert(batch.runs.length === batchRuns.length, "batch observation count drifted");
batch.runs.forEach((run, index) => {
  assertExactKeys(run, ["worldSize", "prefixCache", "batches", "physicalTokenRows", "cachedTokens",
    "seedTtftNs", "reuseTtftNs", "reuseDecodeIntervalNs", "setupSeconds", "wholeSeconds",
    "rankDispatchCounts", "resultSha256", "comparisonSha256"], `batch.runs[${index}]`);
  const [world, cache, seed, reuse, gap, setup, whole, result, comparison] = batchRuns[index];
  const batches = cache ? 5 : 6;
  assert(run.worldSize === world && run.prefixCache === cache && run.batches === batches
    && run.physicalTokenRows === (cache ? 34 : 50) && run.cachedTokens === (cache ? 16 : 0),
  "batch geometry or exact prefix hit drifted");
  assert(run.seedTtftNs === seed && run.reuseTtftNs === reuse && run.reuseDecodeIntervalNs === gap
    && run.setupSeconds === setup && run.wholeSeconds === whole, "batch timing receipt drifted");
  assert(run.resultSha256 === result && run.comparisonSha256 === comparison, "batch receipt hashes drifted");
  assert(JSON.stringify(run.rankDispatchCounts) === JSON.stringify([544 * batches, ...Array(world - 1).fill(540 * batches)]),
    "batch per-rank dispatch schedule drifted");
});

const engineering = project.engineeringObservations;
assertExactKeys(engineering, ["title", "scope", "prompt", "promptTokenIds", "generatedTokenIds",
  "generatedText", "controllerSha256", "workerSha256", "hsacoSha256", "timing", "authority",
  "smokes", "single32"], "engineeringObservations");
const single32 = engineering.single32;
assertExactKeys(single32, ["ttftSeconds", "tpotSeconds", "setupSeconds", "generationSeconds",
  "wholeSeconds", "decodeIntervalCount", "measuredRuns", "warmupRuns", "kvTokensProcessed",
  "rankDispatchCounts", "generatedTokenIds", "generatedText", "resultSha256", "comparisonSha256"], "single32");
assert(single32.ttftSeconds === 163.647853495 && single32.tpotSeconds === 40.25950984674194
  && single32.setupSeconds === 193.519502061 && single32.generationSeconds === 1411.692658744
  && single32.wholeSeconds === 1619.848894496, "single32 measured timings drifted");
assert(single32.decodeIntervalCount === 31 && single32.measuredRuns === 1
  && single32.warmupRuns === 0 && single32.kvTokensProcessed === 36, "single32 profile drifted");
assert(JSON.stringify(single32.rankDispatchCounts) === "[19584,19440,19440,19440,19440,19440,19440,19440]", "single32 rank counts drifted");
assert(JSON.stringify(single32.generatedTokenIds) === "[12095,13,576,6722,315,15344,374,21718,13,576,6722,315,17689,374,24081,13,576,6722,315,9856,374,19846,13,576,6722,315,279,25662,374,37741,13,576]", "single32 reference token match drifted");
assert(single32.generatedText === " Paris. The capital of Italy is Rome. The capital of Spain is Madrid. The capital of Germany is Berlin. The capital of the Netherlands is Amsterdam. The", "single32 reference text match drifted");
assert(single32.resultSha256 === "d69dc3e61c12a8663c28f1af0451e8e382244620b158249adfbf8bc54e9ebe63"
  && single32.comparisonSha256 === "a2474d0a29355c2120977d6b34b1418f309b0236cf0a2947f192dc2db12b816c", "single32 evidence identity drifted");
assert(engineering.prompt === "The capital of France is", "smoke prompt text drifted");
assert(JSON.stringify(engineering.promptTokenIds) === "[785,6722,315,9625,374]", "smoke prompt IDs drifted");
assert(JSON.stringify(engineering.generatedTokenIds) === "[12095,13]" && engineering.generatedText === " Paris.", "smoke output drifted");
assert(engineering.controllerSha256 === "6b49356f4abeed632ef7a5416122d16f234b535a32b43edbe45f5372d2fc6e46", "smoke controller drifted");
assert(engineering.workerSha256 === "77a53d18b56e4ee7a67a434feffa8ac18a4fe60f8c9e5daace351f502c72e1da", "smoke worker drifted");
assert(engineering.hsacoSha256 === project.current.tensorParallelGfx950HsacoSha256, "smoke image drifted");
const smokeFacts = [
  [1, 23.216413234, 4.636949036, 154.806338899, "a2339e7385630298124bc662e59c260225531135dc36b37107d1f8ea126d709c"],
  [2, 75.114170435, 14.674596102, 162.321828177, "6f403aa20ded7fd7d8253090e8fcdbad840e4efa4c9df6567169386e8b51ef47"],
  [8, 160.644607963, 24.711778123, 186.586377561, "f088c54f3be4f3cc03bee00b9920e2db9afe2744ca2aa3e52b3206c2ab95e262"],
];
assert(engineering.smokes.length === smokeFacts.length, "exact smoke roster drifted");
engineering.smokes.forEach((smoke, index) => {
  assertExactKeys(smoke, ["worldSize", "ttftSeconds", "singleDecodeIntervalSeconds", "setupSeconds",
    "rankDispatchCounts", "resultSha256"], `smoke[${index}]`);
  const [world, ttft, interval, setup, hash] = smokeFacts[index];
  assert(smoke.worldSize === world && smoke.ttftSeconds === ttft && smoke.singleDecodeIntervalSeconds === interval
    && smoke.setupSeconds === setup && smoke.resultSha256 === hash, `smoke[${index}] frozen evidence drifted`);
  assert(JSON.stringify(smoke.rankDispatchCounts) === JSON.stringify([3264, ...Array(world - 1).fill(3240)]), "smoke dispatch roster drifted");
});
assert(engineering.scope.includes("one unwarmed run") && engineering.authority.includes("benchmark_comparable=false"), "smoke nonclaims drifted");

assertExactKeys(
  project.latestObservation,
  ["title", "state", "sourceStatus", "environment", "result", "buildId", "generatedTokenIds", "authority"],
  "latestObservation",
);
assertState(project.latestObservation.state, "latestObservation.state");
assert(project.latestObservation.state === "observed", "current aggregate checkpoint must remain observed");
assert(!("commit" in project.latestObservation), "unpublished Ferric source must use source status rather than a public commit link");
assert(
  JSON.stringify(project.latestObservation.generatedTokenIds) === JSON.stringify(project.current.ferricQwen32GeneratedTokenIds),
  "latest hardware checkpoint must retain the exact 32 generated token IDs",
);

assert(Array.isArray(project.recentProgress) && project.recentProgress.length >= 4, "progress ledger is incomplete");
project.recentProgress.forEach((item, index) => {
  const expected = item.commit
    ? item.repository
      ? ["commit", "repository", "title", "state", "detail"]
      : ["commit", "title", "state", "detail"]
    : ["sourceStatus", "title", "state", "detail"];
  assertExactKeys(item, expected, `recentProgress[${index}]`);
  if (item.commit) {
    assertCommit(item.commit, `recentProgress[${index}].commit`);
  } else {
    assert(/^[0-9a-f]{4,7}$/.test(item.sourceStatus), `recentProgress[${index}].sourceStatus must be an abbreviated private identity`);
  }
  assertState(item.state, `recentProgress[${index}].state`);
});

assertExactKeys(project.evidence, ["summary", "legend", "gates"], "evidence");
assert(project.evidence.gates.length >= 6, "evidence gate roster is incomplete");
project.evidence.gates.forEach((entry, index) => {
  assert(Array.isArray(entry) && entry.length === 3, `evidence.gates[${index}] must be a triple`);
  assertState(entry[2], `evidence.gates[${index}].state`);
  assert(entry[2] === "open", `evidence gate ${entry[0]} must remain open`);
  if (entry[0] !== "M1 exit gates") {
    assert(entry[1] === "0", `evidence gate ${entry[0]} must retain zero current results`);
  }
});
project.evidence.legend.forEach((entry, index) => {
  assert(Array.isArray(entry) && entry.length === 2, `evidence.legend[${index}] must be a pair`);
  assertState(entry[0], `evidence.legend[${index}].state`);
});

const snapshot = JSON.stringify(project);
const missingSnapshotClaims = [
  "4a3cbd4efe1df08ccdf7e3e59e19036f9f14469d",
  "Prefix reuse reduces work from 50 to 34 physical token rows and six to five batched forwards",
  "no isolated cache-speedup claim follows",
  "The CLI admits at most 32 requests, 1-256 requested output tokens each, and 240 batches without ring rollover",
  "113.270822031s",
  "47.325504628s",
  "47.801794108s",
  "86.270679460s",
  "37.697434748s",
  "139 library tests with one preexisting ignore",
  "Earlier planner/cursor Verus receipts do not cover the new pool",
  "Persistence means across requests, not disk or process restart",
  "All eight MI350X gfx950 devices",
  "TP1/2/8",
  "not MI350 Qwen execution",
  "36 selected Verus queries with 0 errors",
  "8 rejected actual-body mutations",
  "a8b016e14ca8c77c9e7abe4591086f7cab11ce61",
  "3b14f1e5e1ce800e07ec05638d85d61100c1b04e",
  "2679e59626eee9939412aaf7af6a542c8aeccbe4dd13fb7e6ea3bdcf4f3b8222",
  "All 12 passed host loader closure/materialization",
  "final Ferric a8 repin checks passed",
  "strict Verus 8 verified / 0 errors",
  "8 rejected cursor-body mutations",
  "30 host/source tests per target",
  "One unwarmed TP8 sequence matches all 32 output IDs",
  "one post-first interval each",
  "not a controlled speed comparison",
  "3546d54d2c4a913f5d079701aed557d0a378bba8",
  "433 runtime tests",
  "All six synthetic GPU probes pass",
  "31 host/source tests per target",
  "648 tests with 9 ignored",
  "754-file source closure",
  "not a full-model result or TP8 latency measurement",
  "a real single RMSNorm dispatch",
  "no radix prefix cache in this execution profile",
  "bc6b3a50096854ace496b691933295930066fc04",
  "8b10db06d29d0550b8ce2cadfddc007360016a92",
  "6ef78dc4534317384f7275115c5e77a8b1acb702",
  "63e0f7fc4ed84f6c4717afc2a960e8334118c5b9",
  "bb50d0e0c71e44271b892120c208650bd444f684",
  "7f3f23579b0adfac8ad7357fbf1246ab4b05dc95",
  "c09f212e82eace9326a6d0a0e47ff7a898a8901e5e531fa01ea52213b064b54b",
  "eb219f0f850dfa139d49c1cd303fd292539725fc",
  "046029d7be3043469b41ef8614ddd26beb8d4bbf",
  "d33933adf7f5dfe0a9aa4aba0cc4cb3909b5aa35f9bfb65db7af9af4a5c5bb40",
  "33d754aaa10292fa37e974eb004b6c52067a141dcd5b58ed080023c6bf315c2d",
  "5d40961ec2fd365835867251330072674bdf16dc48a18c4c2bc446bcd60f28b8",
  "d894caf042156abf21436c98fa3de7d40af124ba7374baa0b879bf7df582af44",
  "354 deterministic slots",
  "712-record source closure",
  "arithmetic cardinality only",
  "0ea54ed921cbef5fb2171f4ed7a25b9c1c5e2b71",
  "15f49dde796fe9cbf76274f7d36c7de4c25fef68",
  "42882993b3f84d60f38e398d2018cc9302e8fe19",
  "528fa128e398b9aac5f5fa672388b44ff7b7e67932332abbb61d7e9704715d7a",
  "cf786f800b818a1771c32bd9aa3eb2fe8daf56c625177aa193d6406eab033804",
  "382afa968efda2b761919746e20a2e032b9bdef01ed57ce56dcc757af2ac69a5",
  "1ed868663df52a146dd7921f9fbb2cf1e1d0c0ceed8a7bfda030d65f8ec5b094",
  "all 15 active",
  "26 GuardedStore",
  "exact replay",
  "32/32 tokens",
  "1.115x",
  "1.119x",
  "0.49%",
  "1.53%",
  "off by default",
  "stderr-only",
  "98.23 seconds",
  "97.313 GiB",
  "58.133 seconds",
  "three TCB",
  "source-pinned ELF",
  "matching engineering aggregate",
  "negative actual-body semantic mutations",
  "complete developer qualifier",
  "197/197",
  "711-record source-closure",
  "protected promotion",
  "all eight shared MI300X GPUs",
  "all 32 token IDs exactly match both frozen Hugging Face reference passes",
  "a689418a737bf0b1cf71ec3742bca5702f083187",
  "ce9e0bc76238e569756473cf93ab8a4439062362",
  "b7a8545ed00ec948942690f89b2cbe891d46f836",
  "42f408c0c602166edcc703563df7574324ad4ad2",
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
  "94c3c88518971700498bc05b5da34fb96b23d4e1",
  "e5c6d82c2c0fd02c60bcdafffd887f46bbb35720",
  "fc5f86939cc826480aa008ddf69b044f3d63c585",
  "4d0486476c907ae681bf48d4ba57efb1394e78f5",
  "d05b2daffcc32a061f497e23db8292fb208f0e75",
  "c46263cc73b02d4f41b7c57137954ba08bda9f83",
  "5df8c5e463cca191b142321a9b5040ed66955f06",
  "7c0c4a203903d1515cbe1451754db8d9b1ddd591",
  "be88974e99748fccb5fd4e6f0c3122b3e9a1822db038fcbd3da8707b92d9d830",
  "2958b6915cfbbd9bbb81bdb14471fe044b203e0f",
  "48de4c1fe214b9d9df2576cde735f58b0dc63b25",
  "3d9533b2482e3f8424059333495fe1384f9cd66287471755c5fce0e1528eaee0",
  "100ecefebf2496006c1c0603c274bdff399d63eae23a35ef44f72eede25688c1",
  "a305034b0e7f5ae8b204f066254601fc835592d6",
  "42390274000b3a6674fc0043ced3fdd8f8fe7c59",
  "1e44a64b673f8ede149686979ef759757e71369b",
  "d9e9a381e2a23a3e948943a93abe281f7fd5a0c7",
  "61fca5d4441acea3a4f5ca548b5fde142294153027b0b052b8ac32f27bd7af9c",
  "d7d68e4b4a9c9ec2951a1f849d65573f16c00a893979eb2e00d79f732a10aa27",
  "21bf7ae6a2df53c7e5c18985d1352274b224d6655d7ccc17bba98d51582fac84",
  "dcc7c07a3c21f8a3a8a6ac678987ba172c5809f4",
  "b524a9ccc981466ecde2aa31afa59e2dff672fc8",
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
  "d211c9a0f0c4eebe98172cb30d09c6947f62e233",
  "7f59964f3a66959e3518b898224b47330a949d58",
  "a3c941cc5853c8d9d49aaa80456df82ab0672ef4",
  "3fe917fb4c6601453c74a97deaad529bc83d92ae",
  "c0b6c3676fbabce896a55874ef53864cb0898cfb",
  "9cb42304bb4b647206d0d0e0e5d8a038aa2514f8",
  "e5351640e3df3868205bc68eac8d5ff5556352ea",
  "aba3f86ef14136fa73a385834d4f33f7c9416a32",
  "ea47a88d49ea67384299d1bc3054b09ba4567dd3",
  "f405dfccb5b7021c417df37c0a692aead02fd071",
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
  "target-only",
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
  "no qualification receipt",
  "Qualification #7",
  "Clippy",
  "all 37",
  "SUN_LEN",
  "7,229",
  "7,922",
  "170 modules",
  "6a78aca1ed6c7fc5822cec74f636591631f8d129f0c123e760634cc3a1b510b4",
  "7200d867b34491373922bfe5399bccb66d1e6b2ee9bb99d00e1ad29175688fa4",
  "92886d7b80a20fa3d7d451fc9f212cc313f5852213d6e3328eaa2923a2cc5027",
  "on HOLD",
  "independently accepted",
  "58 service tests",
  "88 adapter tests",
  "The new private TP pool and batched driver implement persistent resident physical-page prefix reuse separately",
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
  "protected infrastructure",
  "symmetric memory",
  "MTP",
  "All 33 M1 exit gates remain open",
  "a3c941cc5853c8d9d49aaa80456df82ab0672ef4",
  "3fe917fb4c6601453c74a97deaad529bc83d92ae",
  "exact Qwen input bundle",
  "14.36s TTFT",
  "3.05s TPOT",
  "invalid and noncomparable",
  "wrong GEMM layout",
  "nonsense output",
  "068e15991b97a829af6e77f2d105262e3ffc5fc69f1fa41a5ed5efdcae0da5e3",
  "6dfba0ac",
  " Paris. The capital of Italy is Rome",
  "8/8",
  "13.649661699s TTFT",
  "2.7656050044 seconds per post-first token",
  "34.094s",
  "hardware completion",
  "benchmark_comparable=false",
  "9886f60d7aa78ba48115f54a67d44f448837a469",
  "02c3c0792b5fc21428ca7499d12f3552d26af533",
  "production-used same-shape core",
  "repeated 16-round",
  "positive allocator control",
  "636 engine tests",
  "9 hardware ignores",
  "integrated engine check",
  "strict Clippy",
  "K3 work-proportional KV-write",
  "no speedup",
  "caller-buffer completed readback",
  "bounded Worker diagnostic",
  "d7d2",
  "2ba02a8",
  "rustc-literal-escaper",
  "public next-window path",
  "authority-free aggregate 528",
  "No authenticated 20-window run",
  "TTFT",
  "TPOT",
  "vLLM and SGLang baselines are absent",
  "Docker permission",
  "no native",
].filter((claim) => !snapshot.includes(claim));
assert(
  missingSnapshotClaims.length === 0,
  `current snapshot is missing claims: ${missingSnapshotClaims.join(", ")}`,
);
for (const staleOrForbidden of [
  "Signed public fe2o3 main is now 0ea",
  "Signed public fe2o3 main is 0ea",
  "Signed current public main 0ea",
  "Signed fe2o3 public main is now 0ea",
  "stale authenticated S1/K4 source-policy anchor",
  "stale S1/K4 source policy",
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
  "data-performance",
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
for (const asset of ["styles.css", "app.js", "data/project.js", "data/performance.js", "assets/mark.svg", "assets/architecture.svg"]) {
  await access(join(siteRoot, asset), constants.R_OK);
}
assert(!appSource.includes("innerHTML"), "renderer must not inject status through innerHTML");
assert(!appSource.includes("eval("), "renderer must not evaluate status strings");

console.log("Ferric site data is structurally valid and current claims remain fail-closed.");
