#!/usr/bin/env python3
"""Return a detached scheduler slot to service with its stale generation."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/scheduler.rs"
source = path.read_text(encoding="utf-8")
old = "        Ok(self.reclaim_slot(&detached, request, next_generation))"
new = "        Ok(self.reclaim_slot(&detached, request, request.generation()))"
if source.count(old) != 1:
    raise SystemExit("scheduler generation-reuse mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/scheduler.rs")
print("MUTATION=reclaim-detached-with-stale-generation")
