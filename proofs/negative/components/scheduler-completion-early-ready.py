#!/usr/bin/env python3
"""Make a completed member dispatchable before KV finalization."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/scheduler.rs"
source = path.read_text(encoding="utf-8")
old = """            permits[processed] = Some(KvQuiescencePermit {
                request: handle,
                origin: KvQuiescenceOrigin::CompletedExact { epoch: observed },
            });
"""
new = """            self.slots[handle.slot() as usize].state = RequestState::Ready;
            permits[processed] = Some(KvQuiescencePermit {
                request: handle,
                origin: KvQuiescenceOrigin::CompletedExact { epoch: observed },
            });
"""
if source.count(old) != 1:
    raise SystemExit("scheduler completion early-ready mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/scheduler.rs")
print("MUTATION=complete-member-before-kv-finalization")
