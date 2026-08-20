#!/usr/bin/env python3
"""Break scheduler/KV admission composition while preserving Rust typing."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/system.rs"
source = path.read_text(encoding="utf-8")
old = "if let Err(error) = self.kv.create_request(request) {"
new = "if let Err(error) = Ok::<(), KvError>(()) {"
if source.count(old) != 1:
    raise SystemExit("system admission mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/system.rs")
print("MUTATION=skip-kv-create-after-scheduler-admit")
