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
    ["roadmap", "Still blocked", "Required before end-to-end Qwen and M1"],
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

  const readiness = document.querySelector("[data-readiness]");
  project.readiness.forEach((item) => {
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
      sourceValue.append(commitLink(item.source));
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

  const observation = document.querySelector("[data-observation]");
  const observationHeader = element("div", "observation-heading");
  const observationTitle = element("div", "");
  observationTitle.append(
    element("div", "observation-label", "Latest hardware attempt"),
    element("h3", "", project.latestObservation.title),
  );
  observationHeader.append(observationTitle, stateTag(project.latestObservation.state));
  const observationFacts = element("dl", "observation-facts");
  const observationEntries = [
    ["Source", commitLink(project.latestObservation.commit)],
    ["Environment", project.latestObservation.environment],
    ["Result", project.latestObservation.result],
    ["ELF Build ID", project.latestObservation.buildId],
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
    title.append(
      commitLink(item.commit, item.repository),
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
