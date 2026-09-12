import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";
import { validateLiveHttpCheckpoint } from "./validate-submission-checkpoint.mjs";
import { validateResidualDiagnostic } from "./validate-residual-diagnostic.mjs";

const siteRoot = dirname(fileURLToPath(import.meta.url));
const baselineProjectSha256 = "8402465db7d75b5f4897774054ec2aef2f1c8e2d40077b2dc4a6ac510f34781a";
const performanceSha256 = "05ad1f50575547c0b0c8244b2912e1d7518258772527f7ff236fdb8d2e42102f";

function projectFrom(bytes) {
  const context = { window: {} };
  vm.runInNewContext(bytes.toString("utf8"), context);
  return JSON.parse(JSON.stringify(context.window.FERRIC_PROJECT));
}

export async function validateKernelCheckpointEvidence(root, project) {
  validateLiveHttpCheckpoint(project.liveHttpCheckpoint);
  validateResidualDiagnostic(project.residualDiagnostic);
  async function pinned(name, digest) {
    const bytes = await readFile(join(root, name));
    assert(bytes.length > 0 && bytes.length <= 2 * 1024 * 1024, `${name}: bounded checkpoint input`);
    assert.equal(createHash("sha256").update(bytes).digest("hex"), digest, name);
    return bytes;
  }
  const baseline = projectFrom(await pinned("baseline-project.js", baselineProjectSha256));
  const unchanged = JSON.parse(JSON.stringify(project));
  delete unchanged.residualDiagnostic;
  assert.deepEqual(unchanged.liveHttpCheckpoint.teams[0], baseline.liveHttpCheckpoint.teams[0]);
  delete unchanged.liveHttpCheckpoint.teams;
  delete baseline.liveHttpCheckpoint.teams;
  assert.deepEqual(unchanged, baseline, "only current team checkpoints may change; every measurement and historical object is frozen");
  assert.equal(createHash("sha256").update(await readFile(join(siteRoot, "data/performance.js"))).digest("hex"),
    performanceSha256, "historical performance bytes");

  const [, visible, append, residual] = project.liveHttpCheckpoint.teams;
  const host = JSON.parse(await pinned("visible-host-r3.json", visible.hostAggregateSha256));
  assert.equal(host.schema, "FerricVisibleAttentionFreshHostGateReceiptV2");
  assert.equal(host.candidate_source, visible.source);
  assert.equal(host.candidate_tree, visible.tree);
  assert.equal(host.archive_sha256, visible.hostArchiveSha256);
  assert.equal(host.step_count, 28);
  assert.deepEqual(host.all_target_passes_by_source_profile,
    { base_full: 13, base_wave: 13, candidate_full: 19, candidate_wave: 19 });
  assert.equal(host.all_own_artifacts_compiled_this_invocation, true);
  assert.equal(host.source_and_prior_unchanged, true);
  assert.deepEqual(host.actual_clippy_exits, [101, 101]);
  assert.equal(host.strict_clippy_pass, false);
  for (const key of ["typed_emission", "gpu_execution", "performance_claim", "verus_proof_claim"])
    assert.equal(host[key], false);
  const failure = (await pinned("visible-emission-failure.md", visible.failureNoteSha256)).toString("utf8");
  for (const claim of [visible.failedEmissionArchiveSha256,
    "a uniform induction region does not have one unique header exit",
    "no HSACO, observation or handoff exists", "candidate-emission1"])
    assert(failure.includes(claim), `visible failure binding: ${claim}`);
  const stopped = (await pinned("visible-stop-audit.md", visible.stopAuditSha256)).toString("utf8");
  for (const claim of [visible.source, visible.tree, visible.hostArchiveSha256,
    visible.failedEmissionArchiveSha256, "There are zero masked tail iterations to remove.",
    "already computes `max_context = max(row.position() + 1)`"])
    assert(stopped.replace(/\s+/g, " ").includes(claim), `stopped experiment binding: ${claim}`);

  const plan = (await pinned("append-diagnostic-plan.md", append.diagnosticPlanSha256)).toString("utf8");
  for (const claim of [append.source, append.tree, append.previousEmissionArchiveSha256,
    "This is not a host gate", "not used for candidate admission", "none have run for this source"])
    assert(plan.replace(/\s+/g, " ").includes(claim), `append diagnostic scope: ${claim}`);
  const appendFailure = (await pinned("append-diagnostic-result.md", append.failureNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  for (const claim of [append.source, append.tree, append.failedArchiveSha256,
    "did not emit target IR or an HSACO", "FE2O3-PROGRESS-002", "The new diagnostic reporter never ran"])
    assert(appendFailure.includes(claim), `append diagnostic failure: ${claim}`);
  assert.equal(append.failureCustodyAdmitted, true);
  assert.equal(append.diagnosticPassed, false);
  assert.equal(append.imageAdmitted, false);
  const note = (await pinned("residual-host-result.md", residual.hostNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  for (const claim of [residual.source, residual.tree, residual.hostArchiveSha256,
    "All 31 recorded commands", "780 passing invocations, 41 ignored, 19 result rows",
    "Strict all-target Clippy: actual zero", "Doctests: 8 passed. CPU HTTP fixtures: 74 passed.",
    "Source-gate tests: 38 passed. Protected source policy: 31 passed.",
    "All 89 own-source Cargo artifact records were `fresh=false`."])
    assert(note.includes(claim), `residual CPU evidence: ${claim}`);
  assert.equal(residual.archiveAdmitted, true);
  const native = (await pinned("residual-native-qualification.md", residual.nativeNoteSha256))
    .toString("utf8").replace(/\s+/g, " ");
  for (const claim of [residual.nativeArchiveSha256, "Exact source2396a82 and controller40fcfff3",
    "All four predeclared cases completed with exit0 in order: synchronous8, ordered8, synchronous128, ordered128.",
    "No case was retried.", "unchanged token-ID/UTF8 oracle, autoregressive inputs, packet/batch/cursor counts, retirement and normal worker close.",
    "Eight-output cases completed9,219 packets,15 batches and135 committed inputs",
    "128-output cases completed83,139 packets,135 batches and255 committed inputs",
    "Every wrapper reports no errors, all-eight idle before and after, and unforced owned-group cleanup.",
    `The${residual.nativeArchiveBytes.toLocaleString("en-US")}-byte archive`,
    "Root directly authenticated all48 nonempty raw SHA/size entries against mi350.",
    `TP1/context${residual.nativeContext}`, "not serving, default, Verus, M1 or performance qualification.",
    "All four timings are excluded", "The latest context8192 HTTP measurements remain unchanged."])
    assert(native.includes(claim), `residual native correctness evidence: ${claim}`);
  assert.equal(residual.nativeAdmitted, true);
  assert.equal(residual.nativeCasesPassed, 4);
  assert.deepEqual(residual.nativeOutputLengths, [8, 128]);
  assert.deepEqual(residual.nativeModes, ["synchronous", "ordered"]);
  assert.equal(residual.nativeTimingsExcluded, true);
  assert.equal(residual.performanceGainClaimed, false);
  console.log("PASS: current kernel/runtime checkpoints match pinned CPU/audit and accepted native-correctness evidence; every measurement and historical object remains unchanged; no performance admission.");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  assert.equal(process.argv.length, 3, "usage: node validate-kernel-checkpoint.mjs EVIDENCE_ROOT");
  await validateKernelCheckpointEvidence(process.argv[2],
    projectFrom(await readFile(join(siteRoot, "data/project.js"))));
}
