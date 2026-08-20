#!/usr/bin/env python3
"""Report Engine retirement without invoking the scheduler."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/system.rs"
source = path.read_text(encoding="utf-8")
old = "self.scheduler.retire(request)"
new = "Ok::<(), SchedulerError>(())"
if source.count(old) != 1:
    raise SystemExit("system retire routing mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/system.rs")
print("MUTATION=skip-engine-scheduler-retirement")
