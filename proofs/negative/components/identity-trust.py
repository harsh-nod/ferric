#!/usr/bin/env python3
"""Insert a forbidden trust primitive into the actual identity body."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/identity.rs"
source = path.read_text(encoding="utf-8")
old = """    pub fn is_present(&self) -> (present: bool)
        ensures
            present == exists|index: int|
                0 <= index < self.bytes_spec().len()
                    && self.bytes_spec()[index] != 0,
    {
        let mut index = 0;
"""
new = """    pub fn is_present(&self) -> (present: bool)
        ensures
            present == exists|index: int|
                0 <= index < self.bytes_spec().len()
                    && self.bytes_spec()[index] != 0,
    {
        assume(false);
        let mut index = 0;
"""
if source.count(old) != 1:
    raise SystemExit("identity trust mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/identity.rs")
print("MUTATION=insert-assume-false")
