#!/usr/bin/env python3
"""Allow KV prefix sharing from tentative resident tokens."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/cache.rs"
source = path.read_text(encoding="utf-8")
old = "        if token_count > self.requests[source_index].committed_tokens {"
new = "        if token_count > self.requests[source_index].resident_tokens {"
if source.count(old) != 1:
    raise SystemExit("KV committed-share mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/cache.rs")
print("MUTATION=share-resident-instead-of-committed-prefix")
