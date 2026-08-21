#!/usr/bin/env python3
"""Discard a validated delta at the executable publication transition."""

from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/step_plan_publication.rs"
source = path.read_text(encoding="utf-8")
old = "    publication.set_phase(PublicationPhase::Published);"
new = "    publication.set_phase(PublicationPhase::Discarded);"
if source.count(old) != 1:
    raise SystemExit("publication phase-transition mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/step_plan_publication.rs")
print("MUTATION=publication-phase-transition")
print("CLAUSE=validated-to-published-transition")
