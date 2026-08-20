#!/usr/bin/env python3
"""Report scheduler reclaim without consuming detached KV evidence."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/system.rs"
source = path.read_text(encoding="utf-8")
old = "        match self.scheduler.reclaim_detached(detached) {"
new = "        match Ok::<RequestId, SchedulerError>(request) {"
if source.count(old) != 1:
    raise SystemExit("system detachment mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/system.rs")
print("MUTATION=skip-detached-evidence-consumption")
