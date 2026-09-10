(function renderFerricStatus() {
  "use strict";

  const project = window.FERRIC_PROJECT;
  if (!project) {
    return;
  }

  const stateLabels = {
    implemented: "Implemented",
    integration: "In integration",
    observed: "Hardware observed",
    verified: "Source verified",
    qualified: "Qualified",
    open: "Open",
  };

  const capabilityGroups = [
    ["runnable", "Implemented surface", "Available for the exact admitted path"],
    [
      "experimental",
      "Integration surface",
      "Scoped source or hardware evidence exists",
    ],
    ["roadmap", "Still blocked", "Required before authenticated R33 serving and M1"],
  ];

  function element(tag, className, text) {
    const node = document.createElement(tag);
    if (className) {
      node.className = className;
    }
    if (text !== undefined) {
      node.textContent = text;
    }
    return node;
  }

  function stateTag(state) {
    return element(
      "span",
      `state-tag state-${state}`,
      stateLabels[state] || state,
    );
  }

  function commitLink(commit, repository = project.repository) {
    const link = element("a", "commit-link", commit);
    link.href = `${repository}/commit/${commit}`;
    link.setAttribute("aria-label", `View source commit ${commit}`);
    return link;
  }

  document.querySelectorAll("[data-repository]").forEach((link) => {
    link.href = project.repository;
  });

  const roadmapLink = document.querySelector("[data-roadmap-link]");
  roadmapLink.href = `${project.repository}/blob/main/docs/ROADMAP.md`;

  document.querySelector("[data-milestone-name]").textContent = project.milestone.name;
  document.querySelector("[data-milestone-label]").textContent = project.milestone.label;
  document.querySelector("[data-milestone-summary]").textContent = project.milestone.summary;
  document.querySelector("[data-milestone-dot]").classList.add(
    `dot-${project.milestone.state}`,
  );

  const updated = document.querySelector("[data-updated]");
  updated.dateTime = project.updated;
  updated.textContent = `Updated ${project.updated}`;

  const performance = window.FERRIC_PERFORMANCE;
  const measured = document.querySelector("[data-performance]");
  function range(values, digits = 3) {
    return values === null ? "n/a" : values.map((value) => value.toFixed(digits)).join(" to ");
  }
  function performanceTable(caption, headings, rows, parent = measured) {
    const wrap = element("div", "transition-table-wrap");
    wrap.tabIndex = 0;
    wrap.setAttribute("role", "region");
    wrap.setAttribute("aria-label", caption);
    const table = element("table", "transition-table performance-table");
    table.append(element("caption", "visually-hidden", caption));
    const head = element("thead", "");
    const heading = element("tr", "");
    headings.forEach((label) => {
      const cell = element("th", "", label);
      cell.scope = "col";
      heading.append(cell);
    });
    head.append(heading);
    const body = element("tbody", "");
    rows.forEach((values) => {
      const row = element("tr", "");
      values.forEach((value) => row.append(element("td", "", value)));
      body.append(row);
    });
    table.append(head, body);
    wrap.append(table);
    parent.append(wrap);
  }
  measured.append(
    element("p", "performance-scope", performance.scope),
    element("p", "", performance.interpretation),
  );
  function singleRunTables(section, title, caption) {
    measured.append(element("h3", "", title), element("p", "", section.scope));
    performanceTable(`${caption}: process windows`,
      ["Profile / Reps", "Output tok/s", "Workload Window (s)", "Setup (s)", "Whole Process (s)"],
      section.profiles.map((profile) => [
        `${profile.name} / n=1`, profile.outputTokensPerSecond.toFixed(6),
        profile.workloadSeconds.toFixed(3), profile.setupSeconds.toFixed(3), profile.wholeSeconds.toFixed(3),
      ]));
    measured.append(element("p", "", section.interpretation));
    performanceTable(`${caption}: request latencies`,
      ["Profile", "Request / Decode Gaps", "TTFT (s)", "TPOT (s)"],
      section.profiles.flatMap((profile) => profile.requestLatencies.map((values, index) => [
        `${profile.name} / n=1`, `${performance.requests[index].name} / ${performance.requests[index].gapsPerRepetition}`,
        values[0].toFixed(3), values[1] === null ? "n/a" : values[1].toFixed(3),
      ])));
  }
  const current = performance.currentCompatibility;
  measured.append(element("h3", "", "Current-controller combinations: mixed correctness"),
    element("p", "", current.scope), element("p", "", current.interpretation));
  performanceTable("Current-controller compatibility: rejected profiles",
    ["Rejected Profile / Reps", "Request", "Expected Token IDs", "Observed Token IDs"],
    current.rejected.map((profile) => [`${profile.name} / n=1`, profile.requestName,
      profile.expectedTokens.join(", "), profile.observedTokens.join(", ")]));
  performanceTable("Current-controller compatibility: accepted process windows",
    ["Accepted Profile / Reps", "Output tok/s", "Workload Window (s)", "Setup (s)", "Whole Process (s)"],
    current.accepted.map((profile) => [`${profile.name} / n=1`, profile.outputTokensPerSecond.toFixed(6),
      profile.workloadSeconds.toFixed(3), profile.setupSeconds.toFixed(3), profile.wholeSeconds.toFixed(3)]));
  performanceTable("Current-controller compatibility: observed rows and profiles",
    ["Profile", "Image / Projection / Pruning / Collective", "Row Capacity / Actual Batch Rows"],
    current.accepted.map((profile) => [profile.name,
      `${profile.imageProfile} / ${profile.projection} / ${profile.outputHeadPruning ? "on" : "off"} / ${profile.collective ?? "host-staged"}`,
      `${profile.rowCapacity} / ${profile.actualBatchRows.join(", ")}`]));
  const currentLatencies = element("details", "performance-identities");
  currentLatencies.append(element("summary", "", "All accepted current-controller request latencies"));
  performanceTable("Current-controller compatibility: accepted named request latencies",
    ["Profile / Request / Gaps", "TTFT (s)", "TPOT (s)"],
    current.accepted.flatMap((profile) => profile.requestLatencies.map((values, index) => [
      `${profile.name} / ${performance.requests[index].name} / ${performance.requests[index].gapsPerRepetition}`,
      values[0].toFixed(3), values[1] === null ? "n/a" : values[1].toFixed(3),
    ])), currentLatencies);
  measured.append(currentLatencies);
  const cohorts = performance.replicaCohorts;
  measured.append(element("h3", "", "Eight-GPU allocation cohorts: 64 outputs"), element("p", "", cohorts.scope));
  performanceTable("Replica cohorts: common-release throughput and process windows",
    ["Layout / Reps", "Output tok/s", "Release to Last Output (s)", "Barrier Setup (s)", "Spawn to Reap (s)"],
    cohorts.profiles.map((profile) => [`${profile.layout} / n=1`, profile.outputTokensPerSecond.toFixed(6),
      (profile.releaseToLastOutputNs / 1e9).toFixed(3), (profile.barrierSetupNs / 1e9).toFixed(3),
      (profile.spawnToReapNs / 1e9).toFixed(3)]));
  measured.append(element("p", "", cohorts.interpretation), element("p", "", cohorts.clockScope));
  performanceTable("Replica cohorts: row policy and loaded weight payloads",
    ["Layout", "Rows / Instance / Total", "Physical Token Rows", "Host Weight Bytes", "GPU Base Weight Bytes", "GPU Transposed Bytes"],
    cohorts.profiles.map((profile) => [profile.layout, `${profile.perInstanceRows} / ${profile.totalRowBudget}`,
      profile.physicalTokenRows, profile.hostWeightBytes, profile.gpuBaseWeightBytes, profile.gpuTransposedWeightBytes]));
  const replicaLatencies = element("details", "performance-identities");
  replicaLatencies.append(element("summary", "", "All replica-cohort request latencies"));
  performanceTable("Replica cohorts: eight named requests per layout",
    ["Layout / Request / Instance", "Admission TTFT (s)", "Release to First Token (s)", "TPOT (s) / 7 Gaps"],
    cohorts.profiles.flatMap((profile) => profile.requestLatencies.map((values, index) => [
      `${profile.layout} / replica-request-${String(index).padStart(2, "0")} / replica-${String(values[0]).padStart(2, "0")}`,
      (values[1] / 1e9).toFixed(3), (values[2] / 1e9).toFixed(3), values[3].toFixed(3),
    ])), replicaLatencies);
  measured.append(replicaLatencies, element("p", "", cohorts.limits));
  const wide = performance.wideRowPair;
  measured.append(element("h3", "", "True 32-row execution: a latency tradeoff"), element("p", "", wide.scope));
  performanceTable("Wide row-policy pair: common-release process windows",
    ["Row / Chunk Policy / Reps", "Output tok/s", "Release to Last Output (s)", "Barrier Setup (s)", "Spawn to Reap (s)"],
    wide.profiles.map((profile) => [`${profile.rows} / ${profile.prefillChunk} / n=1`, profile.outputTokensPerSecond.toFixed(6),
      (profile.releaseToLastOutputNs / 1e9).toFixed(3), (profile.barrierSetupNs / 1e9).toFixed(3),
      (profile.spawnToReapNs / 1e9).toFixed(3)]));
  measured.append(element("p", "", wide.interpretation));
  performanceTable("Wide row-policy pair: observed batch rows",
    ["Policy", "Actual Batch Rows / 96 Total"],
    wide.profiles.map((profile) => [profile.name, profile.actualBatchRows.join(", ")]));
  const wideLatencies = element("details", "performance-identities");
  wideLatencies.append(element("summary", "", "All wide-policy request latencies"));
  performanceTable("Wide row-policy pair: eight named requests per policy",
    ["Policy / Request", "Admission TTFT (s)", "Release to First Token (s)", "TPOT (s) / 7 Gaps"],
    wide.profiles.flatMap((profile) => profile.requestLatencies.map((values, index) => [
      `${profile.rows} rows / replica-request-${String(index).padStart(2, "0")}`,
      (values[0] / 1e9).toFixed(3), (values[1] / 1e9).toFixed(3), values[2].toFixed(3),
    ])), wideLatencies);
  measured.append(wideLatencies, element("p", "", wide.timing));
  const repeated = performance.mfmaRepeated;
  const summaryRange = (values, digits = 3) => values[0] === null ? "n/a"
    : `${((values[0] + values[1]) / 2).toFixed(digits)} [${Math.min(...values).toFixed(digits)}, ${Math.max(...values).toFixed(digits)}]`;
  function repeatedTables(section, firstProfiles, title, label) {
    const groups = firstProfiles.map((first, index) => [first, section.secondProfiles[index]]);
    measured.append(element("h3", "", title), element("p", "", section.scope));
    performanceTable(`Repeated ${label} pair: mean and observed range`,
      ["Profile / Reps", "Output tok/s", "Workload Window (s)", "Setup (s)", "Whole Process (s)"],
      groups.map((runs) => [`${runs[0].name} / n=2`,
        ...["outputTokensPerSecond", "workloadSeconds", "setupSeconds", "wholeSeconds"].map((key, index) =>
          summaryRange(runs.map((run) => run[key]), index === 0 ? 6 : 3))]));
    measured.append(element("p", "", section.interpretation));
    const latencies = element("details", "performance-identities");
    latencies.append(element("summary", "", `All repeated ${label} request latencies`));
    performanceTable(`Repeated ${label} pair: named request means and ranges`,
      ["Profile / Request / Gaps Per Run", "TTFT (s)", "TPOT (s)"],
      groups.flatMap((runs) => performance.requests.map((request, index) => [
        `${runs[0].name} / ${request.name} / ${request.gapsPerRepetition}`,
        summaryRange(runs.map((run) => run.requestLatencies[index][0])),
        summaryRange(runs.map((run) => run.requestLatencies[index][1])),
      ])), latencies);
    measured.append(latencies);
  }
  repeatedTables(repeated, performance.mfmaPair.profiles, "MFMA repeated: request gains, startup cost", "MFMA");
  singleRunTables(performance.mfmaPruning, "MFMA plus pruning: no additive gain observed", "Cumulative MFMA and pruning");
  singleRunTables(performance.mfmaPair, "MFMA R1 checkpoint: faster requests, slower startup", "Matched MFMA pair");
  repeatedTables(performance.deviceTp1Repeated, performance.deviceTp1Pair.profiles, "TP1 residual repeated: small decode change", "TP1 residual");
  singleRunTables(performance.deviceTp1Pair, "TP1 device residual pair", "Matched TP1 residual pair");
  const peerControls = performance.peerSourceControls;
  measured.append(element("h3", "", "Source-matched peer controls: a regression"),
    element("p", "", peerControls.scope), element("p", "", peerControls.interpretation));
  performanceTable("Source-matched peer controls: workload and process windows",
    ["World / Path / Reps", "Output tok/s", "Workload Window (s)", "Setup (s)", "Whole Process (s)"],
    peerControls.controls.flatMap((control, index) => [control, performance.peerObservations.profiles[index]].map((profile) => [
      `${profile.name} / n=1`, profile.outputTokensPerSecond.toFixed(6), profile.workloadSeconds.toFixed(3),
      profile.setupSeconds.toFixed(3), profile.wholeSeconds.toFixed(3),
    ])));
  const peerControlLatencies = element("details", "performance-identities");
  peerControlLatencies.append(element("summary", "", "All source-matched peer request latencies"));
  performanceTable("Source-matched peer controls: per-request latency",
    ["World / Path / Request", "TTFT (s)", "TPOT (s)"],
    peerControls.controls.flatMap((control, index) => [control, performance.peerObservations.profiles[index]].flatMap((profile) =>
      profile.requestLatencies.map((values, requestIndex) => [
        `${profile.name} / ${performance.requests[requestIndex].name}`,
        values[0].toFixed(3), values[1] === null ? "n/a" : values[1].toFixed(3),
      ]))), peerControlLatencies);
  measured.append(peerControlLatencies);
  singleRunTables(performance.peerObservations, "Original peer checkpoint: correctness, not a speedup", "Original peer checkpoint");
  measured.append(element("h3", "", "CPU transpose helper only"), element("p", "", performance.hostTranspose.scope));
  performanceTable("CPU-only transpose helper, sums of per-case medians",
    ["Shard World / Cases", "Baseline Helper (s)", "Tiled Helper (s)", "Helper Speedup"],
    performance.hostTranspose.groups.map((group) => [
      `TP${group.world} / ${group.cases}`, group.baselineSeconds.toFixed(6),
      group.tiledSeconds.toFixed(6), `${group.helperSpeedup.toFixed(3)}x`,
    ]));
  measured.append(element("p", "", performance.hostTranspose.interpretation));
  singleRunTables(performance.transposeModelPair, "Setup transpose: full-model observation", "Setup transpose model pair");
  measured.append(element("h3", "", "Historical repeated profiles"));
  performanceTable("Standalone profile ranges, two repetitions each",
    ["Profile / Reps", "Output tok/s", "Workload Window (s)", "Setup (s)", "Whole Process (s)"],
    performance.variants.map((variant) => [
      `${variant.name} / n=${variant.repetitions}`,
      range(variant.outputTokensPerSecond, 6), range(variant.workloadSeconds),
      range(variant.setupSeconds), range(variant.wholeSeconds),
    ]));
  measured.append(
    element("p", "", performance.statistics),
    element("h3", "", "Latency by request identity"),
  );
  performanceTable("Baseline and operational-only request latency ranges, two repetitions each",
    ["Request / Decode Gaps per Rep", "Baseline TTFT (s)", "Operational TTFT (s)", "Baseline TPOT (s)", "Operational TPOT (s)"],
    performance.requests.map((request) => [
      `${request.name} / ${request.gapsPerRepetition}`,
      range(request.baselineTtft), range(request.operationalTtft),
      range(request.baselineTpot), range(request.operationalTpot),
    ]));
  measured.append(
    element("p", "", `Pruning-only reuse-prefix: TTFT ${range(performance.pruningReuse.ttftSeconds)} s; TPOT ${range(performance.pruningReuse.tpotSeconds)} s, one decode gap per repetition. Large variation prevents an isolated speedup claim.`),
    element("h3", "", "Single-repetition ablations"),
    element("p", "", performance.ablations.scope),
  );
  performanceTable("Single-repetition ablations, throughput and process windows",
    ["Profile / Kind / Reps", "Output tok/s", "Workload Window (s)", "Setup (s)", "Whole Process (s)"],
    performance.ablations.profiles.map((profile) => [
      `${profile.name} / ${profile.kind} / n=1`, profile.outputTokensPerSecond.toFixed(6),
      profile.workloadSeconds.toFixed(3), profile.setupSeconds.toFixed(3), profile.wholeSeconds.toFixed(3),
    ]));
  performanceTable("Single-repetition reuse-prefix latency, one decode gap per run",
    ["Profile / Reps", "Reuse TTFT (s)", "Reuse TPOT (s)", "Workload Rate / Control"],
    performance.ablations.profiles.map((profile) => [
      `${profile.name} / n=1`, ...profile.requestLatencies[3].map((value) => value.toFixed(3)),
      `${(profile.outputTokensPerSecond / performance.ablations.profiles[0].outputTokensPerSecond).toFixed(3)}x`,
    ]));
  measured.append(element("p", "", performance.ablations.interpretation));
  const allLatencies = element("details", "performance-identities");
  allLatencies.append(element("summary", "", "All single-run request latencies"));
  performanceTable("Single-run ablation latencies by request identity",
    ["Profile", "Request / Decode Gaps", "TTFT (s)", "TPOT (s)"],
    performance.ablations.profiles.flatMap((profile) => profile.requestLatencies.map((values, index) => [
      `${profile.name} / n=1`, `${performance.requests[index].name} / ${performance.requests[index].gapsPerRepetition}`,
      values[0].toFixed(3), values[1] === null ? "n/a" : values[1].toFixed(3),
    ])), allLatencies);
  measured.append(allLatencies, element("p", "", performance.correctness));
  const definitions = element("dl", "observation-facts");
  performance.definitions.forEach(([label, detail]) => {
    definitions.append(element("dt", "", label), element("dd", "", detail));
  });
  measured.append(definitions, element("h3", "", "Fixtures and remaining ablations"));
  const fixtures = element("dl", "observation-facts");
  performance.fixtures.forEach(([label, detail]) => {
    fixtures.append(element("dt", "", label), element("dd", "", detail));
  });
  measured.append(fixtures, element("p", "", performance.publication));
  const provenance = element("details", "performance-identities");
  provenance.append(element("summary", "", "Exact identities and archived evidence"));
  provenance.append(element("p", "", performance.provenance));
  const pins = element("dl", "observation-facts");
  Object.entries(performance.identities).forEach(([label, digest]) => {
    pins.append(element("dt", "", label), element("dd", "", digest));
  });
  performance.variants.forEach((variant) => {
    pins.append(element("dt", "", `${variant.name} controller`), element("dd", "", variant.controllerSha256),
      element("dt", "", `${variant.name} worker`), element("dd", "", variant.workerSha256));
  });
  pins.append(element("dt", "", "Operational-only profile"),
    element("dd", "", JSON.stringify(performance.runtimeProfile)));
  for (const key of ["controllerSha256", "workerSha256", "comparatorSha256", "baselineLedgerSha256", "controlLedgerSha256"]) {
    pins.append(element("dt", "", `Single-run ablations ${key}`), element("dd", "", performance.ablations[key]));
  }
  performance.ablations.profiles.forEach((profile) => {
    const flags = { ...performance.runtimeProfile, runtime_operational: profile.operationalCurrentness,
      runtime_cache_admission: profile.admissionCache, dispatch_sequences: profile.dispatchSequences };
    pins.append(element("dt", "", `${profile.name} profile`), element("dd", "", JSON.stringify(flags)),
      element("dt", "", `${profile.name} collective`), element("dd", "", profile.hostWorkspaceReuse
        ? "host-staged-reuse-v3" : "host_staged_fp32_rank_order_reduce_bf16_residual"));
  });
  for (const [label, section] of [["Matched MFMA", performance.mfmaPair],
    ["Matched TP1 residual", performance.deviceTp1Pair], ["Serial peer", performance.peerObservations]]) {
    Object.entries(section.pins).forEach(([key, value]) => {
      const caption = key === "ledgerCanonicalId" ? "ledger generator source SHA-256" : key;
      pins.append(element("dt", "", `${label} ${caption}`), element("dd", "", value));
    });
    section.profiles.forEach((profile) => {
      pins.append(element("dt", "", `${profile.name} comparison SHA-256`), element("dd", "", profile.comparisonSha256));
      if (profile.metricsFileSha256) {
        pins.append(element("dt", "", `${profile.name} metrics file SHA-256`), element("dd", "", profile.metricsFileSha256));
      }
    });
  }
  for (const key of ["source", "summarySha256", "rawLogSha256"]) {
    pins.append(element("dt", "", `CPU transpose ${key}`), element("dd", "", performance.hostTranspose[key]));
  }
  Object.entries(cohorts.pins).forEach(([key, value]) => {
    pins.append(element("dt", "", `Replica cohorts ${key}`), element("dd", "", value));
  });
  cohorts.profiles.forEach((profile) => {
    for (const key of ["comparisonSha256", "expectationSha256", "releaseEpochNs", "maximumLatenessNs"]) {
      const label = key === "expectationSha256" ? "canonical expectation SHA-256" : key;
      pins.append(element("dt", "", `${profile.layout} ${label}`), element("dd", "", profile[key]));
    }
  });
  Object.entries(wide.pins).forEach(([key, value]) => {
    pins.append(element("dt", "", `Wide row policy ${key}`), element("dd", "", value));
  });
  wide.profiles.forEach((profile) => {
    for (const key of ["comparisonSha256", "expectationSha256", "releaseEpochNs", "maximumLatenessNs"]) {
      const label = key === "expectationSha256" ? "canonical expectation SHA-256" : key;
      pins.append(element("dt", "", `${profile.name} ${label}`), element("dd", "", profile[key]));
    }
  });
  for (const key of ["sourceRevision", "controllerSha256", "hostWorkerSha256", "peerWorkerSha256"]) {
    pins.append(element("dt", "", `Source-matched peer ${key}`), element("dd", "", peerControls[key]));
  }
  peerControls.controls.forEach((control) => {
    for (const key of ["comparisonSha256", "ledgerFileSha256"]) {
      pins.append(element("dt", "", `${control.name} ${key}`), element("dd", "", control[key]));
    }
  });
  Object.entries(performance.transposeModelPair.pins).forEach(([key, value]) => {
    pins.append(element("dt", "", `Transpose model pair ${key}`), element("dd", "", value));
  });
  pins.append(element("dt", "", "Repeated MFMA ledger SHA-256"), element("dd", "", repeated.ledgerFileSha256));
  pins.append(element("dt", "", "Cumulative MFMA/pruning ledger SHA-256"), element("dd", "", performance.mfmaPruning.ledgerFileSha256),
    element("dt", "", "Cumulative MFMA/pruning comparison SHA-256"), element("dd", "", performance.mfmaPruning.profiles[0].comparisonSha256));
  repeated.secondProfiles.forEach((profile) => {
    pins.append(element("dt", "", `${profile.name} comparison SHA-256`), element("dd", "", profile.comparisonSha256));
  });
  pins.append(element("dt", "", "Repeated TP1 residual ledger SHA-256"), element("dd", "", performance.deviceTp1Repeated.ledgerFileSha256));
  performance.deviceTp1Repeated.secondProfiles.forEach((profile) => {
    pins.append(element("dt", "", `${profile.name} comparison SHA-256`), element("dd", "", profile.comparisonSha256));
  });
  Object.entries(current.pins).forEach(([key, value]) => {
    pins.append(element("dt", "", `Current-controller ${key}`), element("dd", "", value));
  });
  current.accepted.forEach((profile) => {
    for (const key of ["comparisonSha256", "metricsFileSha256"]) {
      pins.append(element("dt", "", `${profile.name} ${key}`), element("dd", "", profile[key]));
    }
  });
  current.rejected.forEach((profile) => {
    pins.append(element("dt", "", `${profile.name} rejection SHA-256`), element("dd", "", profile.rejectionSha256));
  });
  performance.transposeModelPair.profiles.forEach((profile) => {
    for (const key of ["controllerSha256", "comparisonSha256"]) {
      pins.append(element("dt", "", `${profile.name} ${key}`), element("dd", "", profile[key]));
    }
  });
  provenance.append(pins);
  measured.append(provenance);

  const readiness = document.querySelector("[data-readiness]");
  project.readiness.forEach((item, index) => {
    if (index === 2) {
      readiness.append(element("h3", "", "Earlier validated checkpoints"));
    }
    const row = element("div", "readiness-row");
    const heading = element("div", "readiness-row-heading");
    heading.append(element("strong", "", item.label), stateTag(item.state));
    row.append(heading, element("p", "", item.detail));
    readiness.append(row);
  });

  const envelope = document.querySelector("[data-envelope]");
  project.envelope.forEach(([term, definition]) => {
    const item = element("div", "envelope-item");
    item.append(element("dt", "", term), element("dd", "", definition));
    envelope.append(item);
  });

  const capabilities = document.querySelector("[data-capabilities]");
  capabilityGroups.forEach(([key, title, description]) => {
    const group = element("section", `capability-group capability-${key}`);
    const heading = element("div", "capability-group-heading");
    heading.append(element("h3", "", title), element("p", "", description));
    const list = element("ul", "capability-list");
    project.capabilities[key].forEach((item) => {
      const entry = element("li", "");
      entry.append(element("strong", "", item.name), element("p", "", item.detail));
      list.append(entry);
    });
    group.append(heading, list);
    capabilities.append(group);
  });

  const validation = document.querySelector("[data-validation]");
  [
    ["host", "Host validation"],
    ["proof", "Proof policy"],
    ["hardware", "Hardware validation"],
  ].forEach(([key, label]) => {
    const item = project.validation[key];
    const article = element("article", `validation-item validation-${key}`);
    const heading = element("div", "validation-item-heading");
    const title = element("div", "");
    title.append(
      element("div", "validation-label", label),
      element("h3", "", item.title),
    );
    heading.append(title, stateTag(item.state));

    const facts = element("dl", "validation-facts");
    const sourceValue = element("dd", "");
    if (item.source) {
      sourceValue.append(commitLink(item.source, item.repository));
    } else {
      sourceValue.textContent = item.sourceStatus || "No current-source observation";
    }
    facts.append(
      element("dt", "", "Source"),
      sourceValue,
    );
    if (item.closureSha256) {
      const closureValue = element("dd", "");
      closureValue.append(
        element("code", "closure-digest", item.closureSha256),
      );
      facts.append(element("dt", "", "Source closure"), closureValue);
    }
    facts.append(
      element("dt", "", "Result"),
      element("dd", "", item.result),
    );
    article.append(
      heading,
      facts,
      element("p", "validation-detail", item.detail),
    );
    validation.append(article);
  });

  const transitions = document.querySelector("[data-transitions]");
  project.validation.transitions.forEach(([prior, next, state]) => {
    const row = element("tr", "");
    const priorCell = element("td", "", prior);
    const nextCell = element("td", "", next);
    const stateCell = element("td", "");
    stateCell.append(stateTag(state));
    row.append(priorCell, nextCell, stateCell);
    transitions.append(row);
  });
  document.querySelector("[data-transition-limitation]").textContent =
    project.validation.limitation;

  const teams = document.querySelector("[data-teams]");
  project.teams.forEach((team) => {
    const article = element("article", "team-item");
    const heading = element("div", "team-heading");
    const identity = element("div", "team-identity");
    identity.append(
      element("div", "team-scope-label", team.scope),
      element("h3", "", team.name),
    );
    const state = element("div", "team-state");
    state.append(stateTag(team.state), element("strong", "", team.status));
    heading.append(identity, state);

    const facts = element("dl", "team-facts");
    [
      ["Completed", team.completed],
      ["In progress", team.current],
      ["Dependencies", team.blockedBy],
      ["Next", team.next],
      ["Validation", team.validation],
    ].forEach(([label, value]) => {
      facts.append(element("dt", "", label), element("dd", "", value));
    });
    article.append(heading, facts);
    teams.append(article);
  });

  const boundaries = document.querySelector("[data-boundaries]");
  [
    ["ferric", "Ferric owns"],
    ["fe2o3", "fe2o3 owns"],
  ].forEach(([key, title]) => {
    const section = element("section", `boundary boundary-${key}`);
    const heading = element("h3", "", title);
    const list = element("ul", "");
    project.boundaries[key].forEach((item) => list.append(element("li", "", item)));
    section.append(heading, list);
    boundaries.append(section);
  });

  const tpObservations = document.querySelector("[data-tp-observations]");
  const batch = project.batchEngineeringObservations;
  tpObservations.append(
    element("div", "observation-label", "Batched engineering observation"),
    element("h3", "", batch.title),
    element("p", "", batch.scope),
  );
  for (const [headings, rows] of [
    [["Profile", "Rows / Batches", "Cached Tokens", "Seed TTFT", "Reuse TTFT", "Reuse Decode Gap", "Setup"],
      batch.runs.map((run) => [
        `TP${run.worldSize}, cache ${run.prefixCache ? "on" : "off"}`,
        `${run.physicalTokenRows} / ${run.batches}`, String(run.cachedTokens),
        `${(run.seedTtftNs / 1e9).toFixed(3)} s`, `${(run.reuseTtftNs / 1e9).toFixed(3)} s`,
        `${(run.reuseDecodeIntervalNs / 1e9).toFixed(3)} s`, `${run.setupSeconds.toFixed(3)} s`,
      ])],
    [["Request", "Terminal State", "Exact Output IDs", "Decoded Text"],
      batch.outputs.map((output) => [output.name, output.state, output.tokenIds.join(", "), output.text])],
  ]) {
    const wrap = element("div", "transition-table-wrap");
    const table = element("table", "transition-table");
    const head = element("thead", "");
    const heading = element("tr", "");
    headings.forEach((label) => {
      const cell = element("th", "", label);
      cell.scope = "col";
      heading.append(cell);
    });
    head.append(heading);
    const body = element("tbody", "");
    rows.forEach((values) => {
      const row = element("tr", "");
      values.forEach((value) => row.append(element("td", "", value)));
      body.append(row);
    });
    table.append(head, body);
    wrap.append(table);
    tpObservations.append(wrap);
  }
  const batchFacts = element("dl", "observation-facts tp-facts");
  const batchIdentities = [
    ["Authority", `${batch.authority}; Contracted engineering observation only`],
    ["Private implementation", `${batch.implementationSource}; comparator ${batch.comparatorSource}`],
    ["Private kernel source", batch.kernelSource],
    ["Controller SHA-256", batch.controllerSha256],
    ["KFD worker SHA-256", batch.workerSha256],
    ["Nine-root HSACO SHA-256", batch.hsacoSha256],
    ["Frozen reference SHA-256", batch.referenceSha256],
    ["Fixed workload SHA-256", batch.workloadSha256],
  ];
  batch.runs.forEach((run) => {
    const name = `TP${run.worldSize} cache ${run.prefixCache ? "on" : "off"}`;
    batchIdentities.push(
      [`${name} dispatches`, `${run.rankDispatchCounts.join(", ")}; workers closed and reaped`],
      [`${name} whole run`, `${run.wholeSeconds.toFixed(3)} s, including setup and teardown`],
      [`${name} JSONL SHA-256`, run.resultSha256],
      [`${name} comparison SHA-256`, run.comparisonSha256],
    );
  });
  batchIdentities.forEach(([name, value]) => {
    batchFacts.append(element("dt", "", name), element("dd", "", value));
  });
  tpObservations.append(batchFacts);
  const tp = project.engineeringObservations;
  tpObservations.append(
    element("div", "observation-label", "Engineering Qwen observations"),
    element("h3", "", tp.title),
    element("p", "", tp.scope),
  );
  if (tp.single32) {
    const sequence = tp.single32;
    const facts = element("dl", "observation-facts tp-facts");
    [["TTFT", `${sequence.ttftSeconds.toFixed(3)} s`],
      ["Single-sequence TPOT", `${sequence.tpotSeconds.toFixed(3)} s (mean of 31 intervals)`],
      ["Setup", `${sequence.setupSeconds.toFixed(3)} s, excluded from TTFT`],
      ["Dispatches by rank", sequence.rankDispatchCounts.join(", ")],
      ["KV positions", `${sequence.kvTokensProcessed}; all eight workers closed and reaped`],
      ["Generated IDs", sequence.generatedTokenIds.join(", ")],
      ["Generated text", sequence.generatedText],
      ["JSONL SHA-256", sequence.resultSha256],
      ["Comparison SHA-256", sequence.comparisonSha256]]
      .forEach(([name, value]) => facts.append(element("dt", "", name), element("dd", "", value)));
    tpObservations.append(
      element("h4", "", "TP8: 32-token single-sequence result"),
      element("p", "", "32/32 IDs and decoded text match both frozen reference passes. One unwarmed sequence, no repeated-run statistics or controlled speed comparison."),
      facts,
    );
  }
  tpObservations.append(element("h4", "", "Separate two-token smokes"));
  const smokeWrap = element("div", "transition-table-wrap");
  const smokeTable = element("table", "transition-table");
  const smokeHead = element("thead", "");
  const smokeHeading = element("tr", "");
  ["Mode", "Output", "TTFT", "Single Decode Interval", "Setup"].forEach((label) => {
    const cell = element("th", "", label);
    cell.scope = "col";
    smokeHeading.append(cell);
  });
  smokeHead.append(smokeHeading);
  const smokeBody = element("tbody", "");
  tp.smokes.forEach((smoke) => {
    const row = element("tr", "");
    [`TP${smoke.worldSize}`, "2/2 IDs match", `${smoke.ttftSeconds.toFixed(3)} s`,
      `${smoke.singleDecodeIntervalSeconds.toFixed(3)} s`, `${smoke.setupSeconds.toFixed(3)} s`]
      .forEach((value) => row.append(element("td", "", value)));
    smokeBody.append(row);
  });
  smokeTable.append(smokeHead, smokeBody);
  smokeWrap.append(smokeTable);
  tpObservations.append(smokeWrap, element("p", "", tp.timing));
  const tpFacts = element("dl", "observation-facts tp-facts");
  [["Shared prompt", tp.prompt], ["Smoke token IDs", tp.generatedTokenIds.join(", ")],
    ["Smoke text", tp.generatedText], ["Controller SHA-256", tp.controllerSha256],
    ["Worker SHA-256", tp.workerSha256], ["HSACO SHA-256", tp.hsacoSha256],
    ...tp.smokes.map((smoke) => [`TP${smoke.worldSize} JSONL SHA-256`, smoke.resultSha256])]
    .forEach(([name, value]) => tpFacts.append(element("dt", "", name), element("dd", "", value)));
  tpObservations.append(tpFacts, element("p", "authority-note", tp.authority));

  const observation = document.querySelector("[data-observation]");
  const observationHeader = element("div", "observation-heading");
  const observationTitle = element("div", "");
  observationTitle.append(
    element("div", "observation-label", "Historical MI300X Qwen observation"),
    element("h3", "", project.latestObservation.title),
  );
  observationHeader.append(observationTitle, stateTag(project.latestObservation.state));
  const observationFacts = element("dl", "observation-facts");
  const observationEntries = [
    [
      "Source",
      project.latestObservation.commit
        ? commitLink(project.latestObservation.commit)
        : project.latestObservation.sourceStatus,
    ],
    ["Environment", project.latestObservation.environment],
    ["Result", project.latestObservation.result],
    ["Artifact identity", project.latestObservation.buildId],
  ];
  if (project.latestObservation.generatedTokenIds.length > 0) {
    observationEntries.splice(3, 0, [
      "Token IDs",
      project.latestObservation.generatedTokenIds.join(", "),
    ]);
  }
  observationEntries.forEach(([term, value]) => {
    const dd = element("dd", "");
    if (value instanceof Node) {
      dd.append(value);
    } else {
      dd.textContent = value;
    }
    observationFacts.append(element("dt", "", term), dd);
  });
  observation.append(
    observationHeader,
    observationFacts,
    element("p", "authority-note", project.latestObservation.authority),
  );

  const progress = document.querySelector("[data-progress]");
  project.recentProgress.forEach((item) => {
    const entry = element("li", "timeline-entry");
    const marker = element("span", `timeline-marker marker-${item.state}`);
    marker.setAttribute("aria-hidden", "true");
    const body = element("div", "timeline-body");
    const heading = element("div", "timeline-heading");
    const title = element("h3", "");
    const source = item.commit
      ? commitLink(item.commit, item.repository)
      : element("code", "commit-link", item.sourceStatus);
    title.append(
      source,
      document.createTextNode(` ${item.title}`),
    );
    heading.append(title, stateTag(item.state));
    body.append(heading, element("p", "", item.detail));
    entry.append(marker, body);
    progress.append(entry);
  });

  document.querySelector("[data-evidence-summary]").textContent = project.evidence.summary;
  const legend = document.querySelector("[data-authority-legend]");
  project.evidence.legend.forEach(([state, detail]) => {
    const item = element("div", "authority-item");
    item.append(stateTag(state), element("p", "", detail));
    legend.append(item);
  });

  const gates = document.querySelector("[data-gates]");
  project.evidence.gates.forEach(([label, count, state]) => {
    const row = element("div", "gate-row");
    row.append(
      element("span", "gate-count", count),
      element("span", "gate-label", label),
      stateTag(state),
    );
    gates.append(row);
  });
})();
