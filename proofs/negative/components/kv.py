#!/usr/bin/env python3
"""Publish tentative tokens early in the actual cache append body."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/cache.rs"
source = path.read_text(encoding="utf-8")
old = """        self.pages[page_index].initialized_tokens += written;
        self.requests[request_index].resident_tokens += written;
"""
new = """        self.pages[page_index].initialized_tokens += written;
        self.requests[request_index].resident_tokens += written;
        self.requests[request_index].committed_tokens =
            self.requests[request_index].resident_tokens;
"""
if source.count(old) != 1:
    raise SystemExit("cache append mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/cache.rs")
print("MUTATION=publish-tentative-append-before-finalization")
