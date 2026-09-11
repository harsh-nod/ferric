import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import vm from "node:vm";

const path = process.argv[2];
assert(path, "usage: node validate-freshness-evidence.mjs AGGREGATE_RECEIPT");
const context = { window: {} };
vm.runInNewContext(await readFile(new URL("data/project.js", import.meta.url), "utf8"), context);
const current = JSON.parse(JSON.stringify(context.window.FERRIC_PROJECT.current));
const raw = await readFile(path);
assert.equal(createHash("sha256").update(raw).digest("hex"), current.fe2o3LatestHostGateReceiptSha256);
const receipt = JSON.parse(raw);
assert.equal(receipt.schema, "FerricFreshnessHostGateV1");
assert.equal(receipt.authority, "none");
assert.equal(receipt.ferric_commit, current.fe2o3LatestHostGateSource);
assert.equal(receipt.fe2o3_commit, current.fe2o3LatestMain);
assert.equal(receipt.fe2o3_tree, current.fe2o3LatestTree);
assert.equal(receipt.controller_sha256, current.fe2o3LatestHostGateControllerSha256);
assert.equal(receipt.metadata_configurations, current.fe2o3LatestHostGateMetadataConfigurations);
assert.equal(receipt.passed, current.fe2o3LatestHostGatePassed);
assert.equal(receipt.gpu_execution, current.fe2o3LatestHostGateGpuExecution);
assert.equal(receipt.new_model_qualification, current.fe2o3LatestHostGateModelQualification);
for (const flag of ["all_agreed_gates_first_attempt_passed", "clean_core_checkout_and_harness_pair",
  "format_check_passed_without_edits", "negative_policies_passed", "strict_clippy_and_release_builds_passed"]) {
  assert.equal(receipt[flag], true);
}
assert.equal(receipt.historical_runs_relabelled, false);
assert.equal(receipt.worker_byte_comparison.byte_equal, true);
assert.equal(receipt.worker_byte_comparison.historical_runs_relabelled, false);
assert.equal(receipt.worker_byte_comparison.worker_sha256, "aaa0216a77de0d5f12c2d668b31ca8c340d8975407c2b446bb5e20b5d820bd6e");
assert.equal(receipt.reused_image.source_revision, "5110577a6d8c45390dfb353386cde748efd5d76c");
assert.equal(receipt.reused_image.hsaco_sha256, "d6086650521f72a27559cc049e51d167ea27d1990f0bbbb342b32e925057ff12");
assert(receipt.reused_image.scope.includes("no fresh emission or GPU execution"));
for (const [key, passed, ignored] of [["adapter", 273, 4], ["fp32_head_kernels", 9, 0],
  ["kfd_worker_library", 469, 1], ["kfd_doctests", 31, 0], ["reused_image_cli_tests", 2, 0],
  ["source_gate_and_verifier_policy", 69, 0], ["standalone_peer_worker", 7, 0]]) {
  assert.equal(receipt.rust[key].passed, passed);
  assert.equal(receipt.rust[key].ignored, ignored);
}
assert(current.fe2o3LegacyRepinScope.startsWith("Historical a8b016e1 only:"));
assert.notEqual(current.performanceSprintV2IntegrationSource, current.fe2o3LatestHostGateSource);
console.log("PASS: latest6f6 host receipt bound separately from frozen511 model and historical a8 repin fields.");
