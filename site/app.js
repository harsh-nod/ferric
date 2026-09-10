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
  function performanceTable(caption, headings, rows) {
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
    measured.append(wrap);
  }
  measured.append(
    element("p", "performance-scope", performance.scope),
    element("p", "", performance.interpretation),
  );
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
    element("p", "", performance.correctness),
  );
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
